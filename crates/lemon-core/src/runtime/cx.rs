use crate::animation::{AnimRegistry, AnimSlot, AnimationConfig, AnimationHandle};
use crate::element::Element;
use crate::runtime::derived::Derived;
use crate::runtime::effect::Effect;
use crate::runtime::signal::Signal;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

/// Host request types, reexported here for compatibility with earlier releases.
pub use crate::host::{OpenWindowParams, OpenWindowRequest};

/// Drains requests for the currently active host context.
///
/// Custom hosts should prefer their own [`HostContext::take_window_requests`](crate::host::HostContext::take_window_requests).
/// Outside a host scope this drains the legacy thread-local fallback context.
pub fn take_open_window_requests() -> Vec<OpenWindowRequest> {
    crate::host::HostContext::current().take_window_requests()
}

/// Reactive context passed to your root view and to [`Component`](crate::element::builders::Component) views.
///
/// Call hook methods in a **fixed order** on every render (same rules as React hooks).
/// Use [`use_signal`](Self::use_signal) for mutable app state and [`use_effect`](Self::use_effect) for
/// side effects that should run after mount or when tracked signals change.
pub struct Cx {
    host: crate::host::HostContext,
    hooks: RefCell<Vec<Box<dyn Any>>>,
    index: Cell<usize>,
    deferred_sink: RefCell<Option<Rc<RefCell<Vec<Effect>>>>>,
}

impl Cx {
    pub fn new() -> Self {
        Cx {
            host: crate::host::HostContext::current(),
            hooks: RefCell::new(Vec::new()),
            index: Cell::new(0),
            deferred_sink: RefCell::new(None),
        }
    }

    pub(crate) fn set_deferred_sink(&self, sink: Rc<RefCell<Vec<Effect>>>) {
        *self.deferred_sink.borrow_mut() = Some(sink);
    }

    pub(crate) fn enter_host(&self) -> crate::host::HostScope {
        self.host.enter()
    }

    /// Must be called before each re-render of this component.
    pub fn reset_hooks(&self) {
        self.index.set(0);
    }

    /// Returns persistent reactive state for this component instance.
    ///
    /// The same hook index on later renders returns the same [`Signal`]; changing hook order
    /// between renders will panic.
    pub fn use_signal<T: Clone + 'static>(&self, initial: T) -> Signal<T> {
        let idx = self.index.get();
        self.index.set(idx + 1);
        let mut hooks = self.hooks.borrow_mut();
        if idx < hooks.len() {
            hooks[idx]
                .downcast_ref::<Signal<T>>()
                .expect("use_signal: hook type mismatch — called with different type on re-render")
                .clone()
        } else {
            let s = Signal::new(initial);
            hooks.push(Box::new(s.clone()));
            s
        }
    }

    /// Cached value recomputed only when signals read inside `f` change.
    pub fn use_memo<T: Clone + PartialEq + 'static>(
        &self,
        f: impl Fn() -> T + 'static,
    ) -> Derived<T> {
        let idx = self.index.get();
        self.index.set(idx + 1);
        let mut hooks = self.hooks.borrow_mut();
        if idx < hooks.len() {
            hooks[idx]
                .downcast_ref::<Derived<T>>()
                .expect("use_memo: hook type mismatch")
                .clone()
        } else {
            let d = Derived::new(f);
            hooks.push(Box::new(d.clone()));
            d
        }
    }

    /// Registers a side effect. The closure runs once after the first paint, then again when
    /// any signal read inside it changes.
    ///
    /// Do not call hooks inside the effect body.
    pub fn use_effect(&self, f: impl Fn() + 'static) {
        let idx = self.index.get();
        self.index.set(idx + 1);
        let mut hooks = self.hooks.borrow_mut();
        if idx >= hooks.len() {
            let effect = Effect::new_lazy(f);
            if let Some(sink) = self.deferred_sink.borrow().as_ref() {
                sink.borrow_mut().push(effect.clone());
            }
            hooks.push(Box::new(effect));
        }
        // On re-render, the effect already lives in hooks; f is dropped
    }

    /// Returns the theme captured from this component's host context.
    ///
    /// This does not consume a hook slot and can be called anywhere during render.
    /// The value belongs to the originating host, even while another host is active.
    pub fn use_theme(&self) -> crate::theme::Theme {
        self.host.theme()
    }

    /// Queues a request to open a new native window with the given parameters.
    ///
    /// The request is pushed to this context's host queue and drained by the platform layer after
    /// the current event batch. The new window is independent and participates in the same
    /// application lifecycle as the primary window.
    ///
    /// This method does **not** consume a hook slot and can be called outside of the fixed hook
    /// ordering (e.g. inside an event handler or effect).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use lemon_core::prelude::*;
    /// use lemon_core::OpenWindowParams;
    /// use lemon_core::element::builders::Button;
    ///
    /// fn settings(_cx: &Cx) -> Element {
    ///     Text::new("Settings").into_element()
    /// }
    ///
    /// fn view(cx: &Cx) -> Element {
    ///     let open = cx.use_signal(false);
    ///     if open.get() {
    ///         cx.open_window(OpenWindowParams::default().title("Settings"), settings);
    ///     }
    ///     let open2 = open.clone();
    ///     Button::new("Open").on_click(move || open2.set(true)).into_element()
    /// }
    /// ```
    pub fn open_window(&self, params: OpenWindowParams, root: impl Fn(&Cx) -> Element + 'static) {
        self.host.request_window(OpenWindowRequest {
            params,
            root: Arc::new(root),
        });
    }

    /// Returns a stable animation handle for this component instance.
    ///
    /// The same hook index on later renders returns the same [`AnimationHandle`]; changing hook
    /// order between renders will panic. The handle is registered with the shared
    /// [`AnimRegistry`] and can be cloned into dynamic element closures.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use lemon_core::prelude::*;
    ///
    /// fn view(cx: &Cx) -> Element {
    ///     let anim = cx.use_animation(AnimationConfig::default());
    ///     anim.play();
    ///     Text::new(move || format!("{:.2}", anim.progress())).into_element()
    /// }
    /// ```
    pub fn use_animation(&self, config: AnimationConfig) -> AnimationHandle {
        let idx = self.index.get();
        self.index.set(idx + 1);
        let mut hooks = self.hooks.borrow_mut();
        if idx < hooks.len() {
            hooks[idx]
                .downcast_ref::<AnimationHandle>()
                .expect("use_animation: hook type mismatch")
                .clone()
        } else {
            let handle = AnimRegistry::shared().register(AnimSlot::new(config));
            hooks.push(Box::new(handle.clone()));
            handle
        }
    }
}

pub(crate) fn flush_deferred_sink(sink: &Rc<RefCell<Vec<Effect>>>) {
    let pending = std::mem::take(&mut *sink.borrow_mut());
    for effect in pending {
        effect.run_deferred_initial();
    }
}

impl Default for Cx {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn use_signal_returns_signal() {
        let cx = Cx::new();
        let s = cx.use_signal(42i32);
        assert_eq!(s.get(), 42);
    }

    #[test]
    fn use_signal_same_signal_on_second_call() {
        let cx = Cx::new();
        let s1 = cx.use_signal(0i32);
        s1.set(7);
        cx.reset_hooks();
        let s2 = cx.use_signal(0i32); // same hook index → same signal
        assert_eq!(s2.get(), 7);
    }

    #[test]
    fn use_memo_returns_derived() {
        let cx = Cx::new();
        let s = cx.use_signal(5i32);
        let s2 = s.clone();
        let m = cx.use_memo(move || s2.get() * 2);
        assert_eq!(m.get(), 10);
        s.set(8);
        assert_eq!(m.get(), 16);
    }

    #[test]
    fn use_effect_deferred_until_flush() {
        use std::cell::Cell;
        use std::rc::Rc;

        let run_count = Rc::new(Cell::new(0u32));
        let sink = Rc::new(RefCell::new(Vec::new()));
        let cx = Cx::new();
        cx.set_deferred_sink(Rc::clone(&sink));

        cx.reset_hooks();
        let r = run_count.clone();
        cx.use_effect(move || {
            r.set(r.get() + 1);
        });

        assert_eq!(run_count.get(), 0);
        super::flush_deferred_sink(&sink);
        assert_eq!(run_count.get(), 1);
    }

    #[test]
    fn use_effect_runs_once_on_mount_not_on_rerender() {
        use crate::runtime::signal::Signal;
        use std::cell::Cell;
        use std::rc::Rc;

        let run_count = Rc::new(Cell::new(0u32));
        let sink = Rc::new(RefCell::new(Vec::new()));
        let cx = Cx::new();
        cx.set_deferred_sink(Rc::clone(&sink));
        let trigger = Signal::new(0u32);

        cx.reset_hooks();
        let r = run_count.clone();
        let t = trigger.clone();
        cx.use_effect(move || {
            t.get();
            r.set(r.get() + 1);
        });

        assert_eq!(run_count.get(), 0);
        super::flush_deferred_sink(&sink);
        assert_eq!(run_count.get(), 1);

        cx.reset_hooks();
        let r2 = run_count.clone();
        let t2 = trigger.clone();
        cx.use_effect(move || {
            t2.get();
            r2.set(r2.get() + 1);
        });

        assert_eq!(run_count.get(), 1);

        trigger.set(1);
        assert_eq!(run_count.get(), 2);
    }

    #[test]
    fn use_theme_returns_active_theme() {
        use crate::theme::{current_theme, set_active_theme, Theme};

        let previous = current_theme();
        let dark = Theme::default_dark();
        set_active_theme(dark.clone());

        let cx = Cx::new();
        let active = cx.use_theme();
        assert_eq!(active, dark);

        set_active_theme(previous);
    }

    #[test]
    fn use_animation_returns_stable_handle_on_second_call() {
        let cx = Cx::new();
        let first = cx.use_animation(AnimationConfig::default());
        first.play();

        cx.reset_hooks();
        let second = cx.use_animation(AnimationConfig::default());

        assert!(second.is_playing());
        second.reset();
        assert_eq!(first.progress(), 0.0);
    }

    #[test]
    fn open_window_queues_request_to_thread_local_sink() {
        use crate::element::builders::Text;

        // Drain any stale requests left by other tests before we begin.
        let _ = super::take_open_window_requests();

        let cx = Cx::new();
        let params = OpenWindowParams::default().title("Test").size(400.0, 300.0);
        cx.open_window(params.clone(), |_cx| Text::new("hello").into_element());

        let requests = super::take_open_window_requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].params.title, "Test");
        assert_eq!(requests[0].params.width, 400.0);
        assert_eq!(requests[0].params.height, 300.0);

        // Queue should be empty after draining.
        assert!(super::take_open_window_requests().is_empty());
    }
}
