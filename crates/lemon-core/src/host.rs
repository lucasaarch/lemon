//! Application-owned context for UI rendering and requests to the native host.

use std::cell::RefCell;
use std::rc::Rc;

pub use crate::host_requests::{OpenWindowParams, OpenWindowRequest};
use crate::theme::{current_theme, set_active_theme, Theme};

/// Application-owned window request queue and render theme.
///
/// Clone this context into every window of the same application. Separate
/// contexts have independent queues, even on the same thread.
///
/// ```
/// use lemon_core::{host::HostContext, Cx, OpenWindowParams, Text, Theme};
/// let host = HostContext::new(Theme::default_light());
/// {
///     let _scope = host.enter();
///     Cx::new().open_window(OpenWindowParams::default(), |_| Text::new("Child").into_element());
/// }
/// assert_eq!(host.take_window_requests().len(), 1);
/// ```
#[derive(Clone)]
pub struct HostContext {
    theme: Theme,
    requests: Rc<RefCell<Vec<OpenWindowRequest>>>,
}

thread_local! {
    static FALLBACK: HostContext = HostContext::new(Theme::default());
    static ACTIVE: RefCell<Option<HostContext>> = const { RefCell::new(None) };
}

impl HostContext {
    /// Creates an independent host context with `theme` and an empty request queue.
    pub fn new(theme: Theme) -> Self {
        Self {
            theme,
            requests: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub(crate) fn current() -> Self {
        ACTIVE
            .with(|active| active.borrow().clone())
            .unwrap_or_else(|| {
                FALLBACK.with(|fallback| {
                    let mut context = fallback.clone();
                    context.theme = current_theme();
                    context
                })
            })
    }

    /// Returns the theme captured by this context.
    pub fn theme(&self) -> Theme {
        self.theme.clone()
    }

    /// Activates this context until the returned guard is dropped.
    ///
    /// Nested scopes and unwinding restore the previous context and theme.
    /// Keep guards in lexical, last-in-first-out order on the current thread.
    pub fn enter(&self) -> HostScope {
        let previous_theme = current_theme();
        let previous = ACTIVE.with(|active| active.replace(Some(self.clone())));
        set_active_theme(self.theme.clone());
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
        ACTIVE.with(|active| {
            active.replace(self.previous.take());
        });
        set_active_theme(self.previous_theme.clone());
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
        assert_eq!(cx.use_theme(), Theme::default_light());
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
