//! Application-owned context for UI rendering and requests to the native host.

use std::cell::RefCell;
use std::rc::Rc;

pub use crate::host_requests::{OpenWindowParams, OpenWindowRequest};
use crate::theme::{current_theme, set_active_theme, Theme};
use crate::Signal;

type ClipboardWrite = Rc<dyn Fn(&str)>;

/// Application-owned window request queue, theme, animation clock and clipboard adapter.
///
/// Clone this context into every window of the same application. Separate
/// contexts have independent queues, even on the same thread.
///
/// ```
/// use lemon::{host::HostContext, Cx, OpenWindowParams, Text, Theme};
/// let host = HostContext::new(Theme::default_light());
/// {
///     let _scope = host.enter();
///     Cx::new().open_window(OpenWindowParams::default(), |_| Text::new("Child").into_element());
/// }
/// assert_eq!(host.take_window_requests().len(), 1);
/// ```
#[derive(Clone)]
pub struct HostContext {
    theme: Signal<Theme>,
    clipboard_read: Option<Rc<dyn Fn() -> Option<String>>>,
    clipboard_write: Option<ClipboardWrite>,
    animations: crate::animation::AnimRegistry,
    requests: Rc<RefCell<Vec<OpenWindowRequest>>>,
}

thread_local! {
    static ACTIVE: RefCell<Option<HostContext>> = const { RefCell::new(None) };
}

impl HostContext {
    /// Creates an independent host context with `theme` and an empty request queue.
    pub fn new(theme: Theme) -> Self {
        Self {
            theme: Signal::new(theme),
            clipboard_read: None,
            clipboard_write: None,
            animations: crate::animation::AnimRegistry::default(),
            requests: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Connects text clipboard operations supplied by the host.
    ///
    /// Headless contexts default to an unavailable clipboard. Failed reads return
    /// `None`; write failures are handled by the supplied callback. Native clipboard
    /// dependencies belong to the desktop host, not the UI engine.
    ///
    /// ```
    /// use lemon::{host::HostContext, Theme};
    /// let host = HostContext::new(Theme::default()).with_clipboard(
    ///     || Some("clipboard text".to_owned()),
    ///     |text| { /* send text to the native clipboard */ },
    /// );
    /// ```
    pub fn with_clipboard(
        mut self,
        read: impl Fn() -> Option<String> + 'static,
        write: impl Fn(&str) + 'static,
    ) -> Self {
        self.clipboard_read = Some(Rc::new(read));
        self.clipboard_write = Some(Rc::new(write));
        self
    }

    /// Returns this host's animation registry, independent of other applications.
    pub fn animations(&self) -> crate::animation::AnimRegistry {
        self.animations.clone()
    }

    pub(crate) fn clipboard_text(&self) -> Option<String> {
        self.clipboard_read.as_ref().and_then(|read| read())
    }

    pub(crate) fn set_clipboard_text(&self, text: &str) {
        if let Some(write) = &self.clipboard_write {
            write(text);
        }
    }

    pub(crate) fn current() -> Self {
        ACTIVE
            .with(|active| active.borrow().clone())
            .unwrap_or_else(|| Self::new(current_theme()))
    }

    /// Returns the current theme and subscribes the current reactive observer to changes.
    ///
    /// All clones of this context share updates made through [`set_theme`](Self::set_theme).
    pub fn theme(&self) -> Theme {
        self.theme.get()
    }

    /// Updates this host's theme, notifying its views without resetting component state.
    ///
    /// Returns `false` without notifying observers when `theme` is unchanged. Native
    /// appearance detection belongs to the desktop host; headless hosts can call this
    /// directly. Other host contexts remain independent.
    ///
    /// ```
    /// use lemon::{host::HostContext, Theme};
    /// let host = HostContext::new(Theme::default_dark());
    /// assert!(host.set_theme(Theme::default_light()));
    /// assert_eq!(host.theme(), Theme::default_light());
    /// assert!(!host.set_theme(Theme::default_light()));
    /// ```
    pub fn set_theme(&self, theme: Theme) -> bool {
        if self.theme.peek() == theme {
            return false;
        }
        self.theme.set(theme);
        // Synchronous reactive callbacks restore their scopes before returning. Refresh
        // the active context without subscribing an observer to unrelated host state.
        if let Some(active) = ACTIVE.with(|active| active.borrow().clone()) {
            set_active_theme(active.theme.peek());
        }
        true
    }

    /// Activates this context until the returned guard is dropped.
    ///
    /// Nested scopes and unwinding restore the previous context and theme.
    /// Keep guards in lexical, last-in-first-out order on the current thread.
    pub fn enter(&self) -> HostScope {
        let previous_theme = current_theme();
        let previous = ACTIVE.with(|active| active.replace(Some(self.clone())));
        set_active_theme(self.theme.get());
        HostScope {
            previous,
            previous_theme,
            _thread: std::marker::PhantomData,
        }
    }

    /// Drains only this application's pending window requests.
    pub fn take_window_requests(&self) -> Vec<OpenWindowRequest> {
        std::mem::take(&mut *self.requests.borrow_mut())
    }

    pub(crate) fn request_window(&self, request: OpenWindowRequest) {
        self.requests.borrow_mut().push(request);
    }
}

/// Restores the previous host context and theme on drop; cannot cross threads.
pub struct HostScope {
    previous: Option<HostContext>,
    previous_theme: Theme,
    _thread: std::marker::PhantomData<Rc<()>>,
}

impl Drop for HostScope {
    fn drop(&mut self) {
        let theme = self
            .previous
            .as_ref()
            .map(|previous| previous.theme.peek())
            .unwrap_or_else(|| self.previous_theme.clone());
        ACTIVE.with(|active| {
            active.replace(self.previous.take());
        });
        set_active_theme(theme);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Cx, OpenWindowParams, Text};

    #[test]
    fn requests_remain_with_originating_application_after_scope_exit() {
        let first = HostContext::new(Theme::default_light());
        let second = HostContext::new(Theme::default_dark());
        let cx = {
            let _scope = first.enter();
            Cx::new()
        };
        let _scope = second.enter();
        cx.open_window(OpenWindowParams::default(), |_| {
            Text::new("hi").into_element()
        });
        assert!(second.take_window_requests().is_empty());
        assert_eq!(first.take_window_requests().len(), 1);
        assert_eq!(cx.theme(), Theme::default_light());
    }

    #[test]
    fn unwinding_restores_previous_theme_and_context() {
        let before = current_theme();
        let host = HostContext::new(Theme::default_light());
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _scope = host.enter();
            panic!("test unwind");
        }));
        assert_eq!(current_theme(), before);
    }
}
