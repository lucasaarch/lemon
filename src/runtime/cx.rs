use crate::animation::{AnimSlot, AnimationConfig, AnimationHandle};
use crate::element::Element;
use crate::reactive::derived::Derived;
use crate::reactive::effect::Effect;
use crate::reactive::state::State;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use crate::{host::OpenWindowRequest, WindowConfig};

pub(crate) type DeferredEffect = Box<dyn FnOnce()>;
type Cleanup = Box<dyn FnOnce()>;
type Mount = Box<dyn FnOnce() -> Cleanup>;

struct MountHook {
    mount: RefCell<Option<Mount>>,
    cleanup: RefCell<Option<Cleanup>>,
}

impl MountHook {
    fn run(&self) {
        if let Some(mount) = self.mount.borrow_mut().take() {
            *self.cleanup.borrow_mut() = Some(mount());
        }
    }
}

impl Drop for MountHook {
    fn drop(&mut self) {
        if let Some(cleanup) = self.cleanup.get_mut().take() {
            cleanup();
        }
    }
}

/// Reactive context passed to your root view and to [`Component`](crate::element::builders::Component) views.
///
/// Declare component resources in a **fixed order** on every render.
/// Use [`state`](Self::state) for mutable app state and [`watch`](Self::watch) for
/// side effects that should run after mount or when tracked signals change.
/// Use [`on_mount`](Self::on_mount) when a resource needs cleanup on unmount.
pub struct Cx {
    host: crate::host::HostContext,
    hooks: RefCell<Vec<Box<dyn Any>>>,
    index: Cell<usize>,
    #[cfg(debug_assertions)]
    previous_hook_count: Cell<Option<usize>>,
    #[cfg(debug_assertions)]
    diagnostics: RefCell<crate::devtools::RenderInfo>,
    #[cfg(debug_assertions)]
    render_started: Cell<Option<std::time::Instant>>,
    deferred_sink: RefCell<Option<Rc<RefCell<Vec<DeferredEffect>>>>>,
}

impl Cx {
    /// Creates a context using the active host, or an independent headless context.
    ///
    /// Without a host scope, this captures the current theme and owns its animation
    /// registry and window request queue. Enter a `HostContext` before constructing
    /// contexts when a custom host needs to share or drain those resources.
    pub fn new() -> Self {
        Cx {
            host: crate::host::HostContext::current(),
            hooks: RefCell::new(Vec::new()),
            index: Cell::new(0),
            #[cfg(debug_assertions)]
            previous_hook_count: Cell::new(None),
            #[cfg(debug_assertions)]
            diagnostics: RefCell::new(crate::devtools::RenderInfo::default()),
            #[cfg(debug_assertions)]
            render_started: Cell::new(None),
            deferred_sink: RefCell::new(None),
        }
    }

    pub(crate) fn set_deferred_sink(&self, sink: Rc<RefCell<Vec<DeferredEffect>>>) {
        *self.deferred_sink.borrow_mut() = Some(sink);
    }

    pub(crate) fn animations(&self) -> crate::AnimRegistry {
        self.host.animations()
    }

    pub(crate) fn enter_host(&self) -> crate::host::HostScope {
        self.host.enter()
    }

    /// Must be called before each re-render of this component.
    pub fn reset_hooks(&self) {
        self.index.set(0);
        #[cfg(debug_assertions)]
        self.render_started.set(Some(std::time::Instant::now()));
    }

    pub(crate) fn check_hook_count(&self) -> Option<(usize, usize)> {
        #[cfg(debug_assertions)]
        {
            if let Some(started) = self.render_started.take() {
                let mut info = self.diagnostics.borrow_mut();
                info.renders += 1;
                let elapsed = started.elapsed();
                info.last_render = elapsed;
                info.total_render += elapsed;
            }
            let current = self.index.get();
            let previous = self.previous_hook_count.replace(Some(current));
            previous
                .filter(|previous| *previous != current)
                .map(|previous| (previous, current))
        }
        #[cfg(not(debug_assertions))]
        {
            None
        }
    }

    #[cfg(debug_assertions)]
    fn record_hook(&self, index: usize, kind: &'static str, value_type: &'static str) {
        let mut info = self.diagnostics.borrow_mut();
        if index == info.hooks.len() {
            info.hooks.push(crate::devtools::HookInfo {
                kind,
                value_type,
                calls: 0,
            });
        }
        let hook = &mut info.hooks[index];
        hook.kind = kind;
        hook.value_type = value_type;
        hook.calls += 1;
    }

    #[cfg(debug_assertions)]
    pub(crate) fn inspect_hooks(&self) -> crate::devtools::RenderInfo {
        let mut info = self.diagnostics.borrow().clone();
        info.hooks.truncate(self.index.get());
        info
    }

    /// Returns persistent reactive state for this component instance.
    ///
    /// The same declaration slot on later renders returns the same [`crate::State`];
    /// changing the declaration order can panic or bind state to a different resource. Cloning the returned state shares the same value;
    /// `initial` is used only when this declaration first mounts.
    ///
    /// ```
    /// use lemon::Cx;
    /// let cx = Cx::new();
    /// let count = cx.state(0);
    /// count.set(1);
    /// assert_eq!(count.get(), 1);
    /// ```
    pub fn state<T: Clone + 'static>(&self, initial: T) -> crate::State<T> {
        let idx = self.index.get();
        self.index.set(idx + 1);
        #[cfg(debug_assertions)]
        self.record_hook(idx, "state", std::any::type_name::<T>());
        let mut hooks = self.hooks.borrow_mut();
        if idx < hooks.len() {
            hooks[idx]
                .downcast_ref::<State<T>>()
                .expect("state: hook type mismatch — called with different type on re-render")
                .clone()
        } else {
            let s = State::new(initial);
            hooks.push(Box::new(s.clone()));
            s
        }
    }

    /// Declares a cached value derived from reactive reads inside `f`.
    ///
    /// Values must implement `PartialEq`: dependents are notified only when the result
    /// changes. Declare this in a stable order, outside conditional branches or changing
    /// loops. Use it for derived data; build element trees directly during rendering.
    ///
    /// ```
    /// use lemon::Cx;
    /// let cx = Cx::new();
    /// let count = cx.state(2);
    /// let source = count.clone();
    /// let doubled = cx.derive(move || source.get() * 2);
    /// assert_eq!(doubled.get(), 4);
    /// ```
    pub fn derive<T: Clone + PartialEq + 'static>(
        &self,
        f: impl Fn() -> T + 'static,
    ) -> Derived<T> {
        let idx = self.index.get();
        self.index.set(idx + 1);
        #[cfg(debug_assertions)]
        self.record_hook(idx, "derive", std::any::type_name::<T>());
        let mut hooks = self.hooks.borrow_mut();
        if idx < hooks.len() {
            hooks[idx]
                .downcast_ref::<Derived<T>>()
                .expect("derive: hook type mismatch")
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
    /// Declare watchers in a stable order on every render. Do not declare component
    /// resources inside the callback. The first callback is retained for this declaration;
    /// capture shared state to read current values. Hosts flush the initial call after paint.
    ///
    /// ```no_run
    /// use lemon::{Cx, element::Element, Text};
    /// fn view(cx: &Cx) -> Element {
    ///     let count = cx.state(0);
    ///     let observed = count.clone();
    ///     cx.watch(move || println!("Count: {}", observed.get()));
    ///     Text::new(move || count.get().to_string()).into_element()
    /// }
    /// ```
    pub fn watch(&self, f: impl Fn() + 'static) {
        let idx = self.index.get();
        self.index.set(idx + 1);
        #[cfg(debug_assertions)]
        self.record_hook(idx, "watch", std::any::type_name::<Effect>());
        let mut hooks = self.hooks.borrow_mut();
        if idx < hooks.len() {
            assert!(hooks[idx].is::<Effect>(), "watch: hook type mismatch");
        } else {
            let effect = Effect::new_lazy(f);
            if let Some(sink) = self.deferred_sink.borrow().as_ref() {
                let weak = effect.downgrade();
                sink.borrow_mut()
                    .push(Box::new(move || weak.run_deferred_initial()));
            }
            hooks.push(Box::new(effect));
        }
        // On re-render, the effect already lives in hooks; f is dropped
    }

    /// Runs `mount` once after the first presented frame and runs its returned cleanup
    /// when this component instance leaves the tree or its window closes.
    ///
    /// Declare this hook in a fixed order on every render. The first `mount` closure is
    /// retained; later closures at the same hook slot are discarded. If the component
    /// unmounts before its first frame, neither closure runs. The cleanup must release
    /// resources it creates, such as listeners or timers. For reactive work that re-runs
    /// when a state changes, use [`watch`](Self::watch).
    ///
    /// ```no_run
    /// use lemon::{element::Element, Cx, Text};
    /// fn page(cx: &Cx) -> Element {
    ///     cx.on_mount(|| {
    ///         println!("page mounted");
    ///         || println!("page unmounted")
    ///     });
    ///     Text::new("Page").into_element()
    /// }
    /// ```
    pub fn on_mount<F, C>(&self, mount: F)
    where
        F: FnOnce() -> C + 'static,
        C: FnOnce() + 'static,
    {
        let idx = self.index.get();
        self.index.set(idx + 1);
        #[cfg(debug_assertions)]
        self.record_hook(idx, "on_mount", std::any::type_name::<MountHook>());
        let mut hooks = self.hooks.borrow_mut();
        if idx < hooks.len() {
            assert!(
                hooks[idx].is::<Rc<MountHook>>(),
                "on_mount: hook type mismatch"
            );
            return;
        }
        let hook = Rc::new(MountHook {
            mount: RefCell::new(Some(Box::new(move || Box::new(mount())))),
            cleanup: RefCell::new(None),
        });
        if let Some(sink) = self.deferred_sink.borrow().as_ref() {
            let weak = Rc::downgrade(&hook);
            sink.borrow_mut().push(Box::new(move || {
                if let Some(hook) = weak.upgrade() {
                    hook.run();
                }
            }));
        }
        hooks.push(Box::new(hook));
    }

    /// Returns the current theme from this component's originating host context.
    ///
    /// This does not consume a hook slot and can be called anywhere during render.
    /// The value belongs to the originating host, even while another host is active.
    /// Reads subscribe the current reactive observer to host theme changes.
    pub fn theme(&self) -> crate::theme::Theme {
        self.host.theme()
    }

    /// Queues a new native window using the same [`WindowConfig`] as [`crate::run`].
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
    /// use lemon::prelude::*;
    /// use lemon::WindowConfig;
    ///
    /// fn settings(_cx: &Cx) -> Element {
    ///     Text::new("Settings").into_element()
    /// }
    ///
    /// fn view(cx: &Cx) -> Element {
    ///     let windows = Cx::new();
    ///     Button::new(cx, "Open").action(move || {
    ///         windows.open_window(WindowConfig::default().title("Settings"), settings);
    ///     }).into_element()
    /// }
    /// ```
    pub fn open_window(&self, config: WindowConfig, root: impl Fn(&Cx) -> Element + 'static) {
        self.host.request_window(OpenWindowRequest {
            config,
            root: Arc::new(root),
        });
    }

    /// Returns a stable animation handle for this component instance.
    ///
    /// The same hook index on later renders returns the same [`AnimationHandle`]; changing hook
    /// order between renders will panic. The handle is registered with the originating host's
    /// [`crate::AnimRegistry`] and can be cloned into dynamic element closures.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use lemon::prelude::*;
    ///
    /// fn view(cx: &Cx) -> Element {
    ///     let anim = cx.motion(AnimationConfig::default());
    ///     anim.play();
    ///     Text::new(move || format!("{:.2}", anim.progress())).into_element()
    /// }
    /// ```
    pub fn motion(&self, config: AnimationConfig) -> AnimationHandle {
        let idx = self.index.get();
        self.index.set(idx + 1);
        #[cfg(debug_assertions)]
        self.record_hook(idx, "motion", std::any::type_name::<AnimationHandle>());
        let mut hooks = self.hooks.borrow_mut();
        if idx < hooks.len() {
            hooks[idx]
                .downcast_ref::<AnimationHandle>()
                .expect("motion: hook type mismatch")
                .clone()
        } else {
            let handle = self.host.animations().register(AnimSlot::new(config));
            hooks.push(Box::new(handle.clone()));
            handle
        }
    }
}

pub(crate) fn flush_deferred_sink(sink: &Rc<RefCell<Vec<DeferredEffect>>>) {
    let pending = std::mem::take(&mut *sink.borrow_mut());
    for effect in pending {
        effect();
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
    fn standalone_contexts_do_not_share_window_queues_or_animation_requests() {
        let first = Cx::new();
        let second = Cx::new();
        first.open_window(WindowConfig::default(), |_| {
            crate::Text::new("child").into_element()
        });
        assert!(second.host.take_window_requests().is_empty());
        assert_eq!(first.host.take_window_requests().len(), 1);
        let animation = first.motion(AnimationConfig::default());
        animation.play();
        assert!(first.host.animations().take_pending());
        assert!(!second.host.animations().take_pending());
    }

    #[test]
    fn state_returns_signal() {
        let cx = Cx::new();
        let s = cx.state(42i32);
        assert_eq!(s.get(), 42);
    }

    #[test]
    fn state_same_signal_on_second_call() {
        let cx = Cx::new();
        let s1 = cx.state(0i32);
        s1.set(7);
        cx.reset_hooks();
        let s2 = cx.state(0i32); // same hook index → same signal
        assert_eq!(s2.get(), 7);
    }

    #[test]
    fn derive_returns_derived() {
        let cx = Cx::new();
        let s = cx.state(5i32);
        let s2 = s.clone();
        let m = cx.derive(move || s2.get() * 2);
        assert_eq!(m.get(), 10);
        s.set(8);
        assert_eq!(m.get(), 16);
    }

    #[test]
    fn watch_deferred_until_flush() {
        use std::cell::Cell;
        use std::rc::Rc;

        let run_count = Rc::new(Cell::new(0u32));
        let sink = Rc::new(RefCell::new(Vec::new()));
        let cx = Cx::new();
        cx.set_deferred_sink(Rc::clone(&sink));

        cx.reset_hooks();
        let r = run_count.clone();
        cx.watch(move || {
            r.set(r.get() + 1);
        });

        assert_eq!(run_count.get(), 0);
        super::flush_deferred_sink(&sink);
        assert_eq!(run_count.get(), 1);
    }

    #[test]
    fn watch_runs_once_on_mount_not_on_rerender() {
        use crate::reactive::state::State;
        use std::cell::Cell;
        use std::rc::Rc;

        let run_count = Rc::new(Cell::new(0u32));
        let sink = Rc::new(RefCell::new(Vec::new()));
        let cx = Cx::new();
        cx.set_deferred_sink(Rc::clone(&sink));
        let trigger = State::new(0u32);

        cx.reset_hooks();
        let r = run_count.clone();
        let t = trigger.clone();
        cx.watch(move || {
            t.get();
            r.set(r.get() + 1);
        });

        assert_eq!(run_count.get(), 0);
        super::flush_deferred_sink(&sink);
        assert_eq!(run_count.get(), 1);

        cx.reset_hooks();
        let r2 = run_count.clone();
        let t2 = trigger.clone();
        cx.watch(move || {
            t2.get();
            r2.set(r2.get() + 1);
        });

        assert_eq!(run_count.get(), 1);

        trigger.set(1);
        assert_eq!(run_count.get(), 2);
    }

    #[test]
    fn theme_returns_active_theme() {
        use crate::theme::{current_theme, set_active_theme, Theme};

        let previous = current_theme();
        let dark = Theme::default_dark();
        set_active_theme(dark.clone());

        let cx = Cx::new();
        let active = cx.theme();
        assert_eq!(active, dark);

        set_active_theme(previous);
    }

    #[test]
    fn motion_returns_stable_handle_on_second_call() {
        let cx = Cx::new();
        let first = cx.motion(AnimationConfig::default());
        first.play();

        cx.reset_hooks();
        let second = cx.motion(AnimationConfig::default());

        assert!(second.is_playing());
        second.reset();
        assert_eq!(first.progress(), 0.0);
    }

    #[cfg(debug_assertions)]
    #[test]
    fn hook_count_change_is_detected_after_rerender() {
        let cx = Cx::new();
        cx.reset_hooks();
        cx.state(0);
        assert_eq!(cx.check_hook_count(), None);
        cx.reset_hooks();
        assert_eq!(cx.check_hook_count(), Some((1, 0)));
    }

    #[test]
    fn open_window_queues_request_to_originating_host() {
        use crate::element::builders::Text;

        let host = crate::host::HostContext::new(crate::Theme::default());
        let _scope = host.enter();

        let cx = Cx::new();
        let params = WindowConfig::default().title("Test").size(400.0, 300.0);
        cx.open_window(params.clone(), |_cx| Text::new("hello").into_element());

        let requests = host.take_window_requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].config, params);
        assert_eq!(requests[0].config.title, "Test");
        assert_eq!(requests[0].config.width, 400.0);
        assert_eq!(requests[0].config.height, 300.0);

        // Queue should be empty after draining.
        assert!(host.take_window_requests().is_empty());
    }
}
