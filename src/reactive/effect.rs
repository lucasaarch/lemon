//! State-dependent side effects.

use crate::reactive::observer::{with_observer, RunObserver, Subscriber};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

struct EffectInner {
    f: Box<dyn Fn()>,
    observer: RefCell<Option<Rc<RunObserver>>>,
    is_running: Cell<bool>,
}

struct RunningGuard<'a>(&'a Cell<bool>);

impl Drop for RunningGuard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

impl EffectInner {
    fn run(inner: &Rc<Self>) {
        if inner.is_running.replace(true) {
            return;
        }
        let _guard = RunningGuard(&inner.is_running);
        let weak = Rc::downgrade(inner);
        let observer = RunObserver::new(move || {
            if let Some(inner) = weak.upgrade() {
                Self::run(&inner);
            }
        });
        if let Some(previous) = inner.observer.replace(Some(observer.clone())) {
            previous.deactivate();
        }
        with_observer(Rc::downgrade(&observer) as Weak<dyn Subscriber>, || {
            (inner.f)();
        });
    }
}

/// An effect that runs a closure immediately and re-runs it whenever a signal it reads changes.
/// Each run replaces its signal subscriptions, so changes to values only read by
/// earlier runs no longer trigger the effect.
/// Dropping the effect stops it from re-running. If a callback panics and the
/// caller catches the panic, later signal changes can run the effect again.
#[derive(Clone)]
pub struct Effect {
    _inner: Rc<EffectInner>,
}

/// Non-owning reference used by the post-paint queue.
pub(crate) struct WeakEffect(Weak<EffectInner>);

impl WeakEffect {
    pub(crate) fn run_deferred_initial(&self) {
        if let Some(inner) = self.0.upgrade() {
            EffectInner::run(&inner);
        }
    }
}

impl Effect {
    /// Runs `f` now and whenever a signal read by it notifies this effect.
    ///
    /// Keep the returned handle alive to retain the subscriptions.
    pub fn new(f: impl Fn() + 'static) -> Self {
        let inner = Rc::new(EffectInner {
            f: Box::new(f),
            observer: RefCell::new(None),
            is_running: Cell::new(false),
        });
        EffectInner::run(&inner);
        Effect { _inner: inner }
    }

    /// Like [`new`](Self::new), but does not run `f` until a tracked signal marks the effect dirty.
    pub fn new_lazy(f: impl Fn() + 'static) -> Self {
        let inner = Rc::new(EffectInner {
            f: Box::new(f),
            observer: RefCell::new(None),
            is_running: Cell::new(false),
        });
        Effect { _inner: inner }
    }

    pub(crate) fn downgrade(&self) -> WeakEffect {
        WeakEffect(Rc::downgrade(&self._inner))
    }

    /// Run the effect body once (used for deferred `Cx::watch` callbacks after first paint).
    pub fn run_deferred_initial(&self) {
        EffectInner::run(&self._inner);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reactive::state::State;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn runs_immediately() {
        let ran = Rc::new(Cell::new(false));
        let r = ran.clone();
        let _e = Effect::new(move || {
            r.set(true);
        });
        assert!(ran.get());
    }

    #[test]
    fn reruns_on_signal_change() {
        let s = State::new(0i32);
        let count = Rc::new(Cell::new(0u32));
        let c = count.clone();
        let s2 = s.clone();
        let _e = Effect::new(move || {
            s2.get();
            c.set(c.get() + 1);
        });
        assert_eq!(count.get(), 1);
        s.set(1);
        assert_eq!(count.get(), 2);
        s.set(2);
        assert_eq!(count.get(), 3);
    }

    #[test]
    fn stops_rerunning_after_drop() {
        let s = State::new(0i32);
        let count = Rc::new(Cell::new(0u32));
        let c = count.clone();
        let s2 = s.clone();
        {
            let _e = Effect::new(move || {
                s2.get();
                c.set(c.get() + 1);
            });
        }
        s.set(99);
        assert_eq!(count.get(), 1); // ran once on mount, not again after drop
    }

    #[test]
    fn effect_runs_again_after_a_caught_callback_panic() {
        let state = State::new(0);
        let source = state.clone();
        let calls = Rc::new(Cell::new(0));
        let count = calls.clone();
        let panic_next = Rc::new(Cell::new(false));
        let fail = panic_next.clone();
        let _effect = Effect::new(move || {
            source.get();
            count.set(count.get() + 1);
            if fail.replace(false) {
                panic!("effect failed");
            }
        });
        panic_next.set(true);
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| state.set(1))).is_err());
        state.set(2);
        assert_eq!(calls.get(), 3);
    }

    #[test]
    fn effect_stops_listening_to_states_not_read_on_latest_run() {
        let choose_first = State::new(true);
        let first = State::new(0);
        let second = State::new(0);
        let runs = Rc::new(Cell::new(0));
        let (choice, a, b, count) = (
            choose_first.clone(),
            first.clone(),
            second.clone(),
            runs.clone(),
        );
        let _effect = Effect::new(move || {
            count.set(count.get() + 1);
            if choice.get() {
                a.get();
            } else {
                b.get();
            }
        });
        choose_first.set(false);
        assert_eq!(runs.get(), 2);
        first.set(1);
        assert_eq!(runs.get(), 2);
        second.set(1);
        assert_eq!(runs.get(), 3);
        choose_first.set(true);
        assert_eq!(runs.get(), 4);
        second.set(2);
        assert_eq!(runs.get(), 4);
        first.set(2);
        assert_eq!(runs.get(), 5);
    }
}
