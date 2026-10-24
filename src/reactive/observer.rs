//! Dependency tracking scopes shared by signals and UI render effects.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// Dependency target notified when a tracked signal changes.
pub trait Subscriber {
    /// Schedules or performs this subscriber's reactive work.
    fn mark_dirty(&self);
}

/// One evaluation's subscriptions. Replacing the token retires dependencies
/// that the next evaluation did not read.
pub(crate) struct RunObserver {
    active: Cell<bool>,
    on_dirty: Box<dyn Fn()>,
}

impl RunObserver {
    pub(crate) fn new(on_dirty: impl Fn() + 'static) -> Rc<Self> {
        Rc::new(Self {
            active: Cell::new(true),
            on_dirty: Box::new(on_dirty),
        })
    }

    pub(crate) fn deactivate(&self) {
        self.active.set(false);
    }
}

impl Subscriber for RunObserver {
    fn mark_dirty(&self) {
        if self.active.get() {
            (self.on_dirty)();
        }
    }
}

thread_local! {
    static OBSERVER_STACK: RefCell<Vec<Weak<dyn Subscriber>>> = RefCell::new(Vec::new());
    static SUPPRESS_NOTIFY: Cell<bool> = const { Cell::new(false) };
    static DEFER_NOTIFY: Cell<bool> = const { Cell::new(false) };
    static DEFERRED_SUBSCRIBERS: RefCell<Vec<Weak<dyn Subscriber>>> = RefCell::new(Vec::new());
}

/// Queues reactive invalidations until the host's current input batch is processed.
pub(crate) fn defer_notifications() {
    DEFER_NOTIFY.with(|defer| defer.set(true));
}

/// Delivers queued invalidations once per subscriber, coalescing repeated event writes.
pub(crate) fn flush_deferred_notifications() {
    DEFER_NOTIFY.with(|defer| defer.set(false));
    let subscribers =
        DEFERRED_SUBSCRIBERS.with(|pending| std::mem::take(&mut *pending.borrow_mut()));
    for subscriber in subscribers
        .into_iter()
        .filter_map(|subscriber| subscriber.upgrade())
    {
        subscriber.mark_dirty();
    }
}

pub(crate) fn notify_subscriber(subscriber: &Rc<dyn Subscriber>) {
    if DEFER_NOTIFY.with(Cell::get) {
        DEFERRED_SUBSCRIBERS.with(|pending| {
            let mut pending = pending.borrow_mut();
            let subscriber = Rc::downgrade(subscriber);
            if !pending.iter().any(|queued| queued.ptr_eq(&subscriber)) {
                pending.push(subscriber);
            }
        });
    } else {
        subscriber.mark_dirty();
    }
}

/// Runs `f` without recording signal dependencies or notifying subscribers on writes.
/// Restores normal tracking and notifications even if `f` panics.
///
/// ```
/// use lemon::reactive::{Effect, State};
/// use lemon::reactive::observer::without_observer;
/// use std::{cell::Cell, rc::Rc};
/// let value = State::new(0);
/// let reads = Rc::new(Cell::new(0));
/// let source = value.clone();
/// let count = reads.clone();
/// let _effect = Effect::new(move || {
///     count.set(count.get() + 1);
///     without_observer(|| source.get());
/// });
/// value.set(1);
/// assert_eq!(reads.get(), 1);
/// ```
pub fn without_observer<R>(f: impl FnOnce() -> R) -> R {
    let previous = SUPPRESS_NOTIFY.with(|flag| flag.replace(true));
    let _guard = SuppressGuard(previous);
    f()
}

pub(crate) fn notifications_suppressed() -> bool {
    SUPPRESS_NOTIFY.with(|flag| flag.get())
}

struct SuppressGuard(bool);

impl Drop for SuppressGuard {
    fn drop(&mut self) {
        SUPPRESS_NOTIFY.with(|flag| flag.set(self.0));
    }
}

struct StackGuard;

impl Drop for StackGuard {
    fn drop(&mut self) {
        OBSERVER_STACK.with(|stack| {
            stack.borrow_mut().pop();
        });
    }
}

/// Runs `f` while signal reads subscribe `observer`, restoring the outer scope on return or panic.
pub fn with_observer<R>(observer: Weak<dyn Subscriber>, f: impl FnOnce() -> R) -> R {
    OBSERVER_STACK.with(|stack| stack.borrow_mut().push(observer));
    let _guard = StackGuard;
    f()
}

/// Returns the innermost dependency target, or `None` outside a tracked scope.
pub fn current_observer() -> Option<Weak<dyn Subscriber>> {
    if notifications_suppressed() {
        return None;
    }
    OBSERVER_STACK.with(|stack| stack.borrow().last().cloned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reactive::state::State;
    use std::cell::Cell;
    use std::rc::Rc;

    struct MockSub(Cell<u32>);

    impl Subscriber for MockSub {
        fn mark_dirty(&self) {
            self.0.set(self.0.get() + 1);
        }
    }

    #[test]
    fn no_observer_outside_scope() {
        assert!(current_observer().is_none());
    }

    #[test]
    fn observer_present_inside_with_observer() {
        let sub = Rc::new(MockSub(Cell::new(0)));
        with_observer(Rc::downgrade(&sub) as Weak<dyn Subscriber>, || {
            assert!(current_observer().is_some());
        });
        assert!(current_observer().is_none());
    }

    #[test]
    fn with_observer_returns_closure_value() {
        let sub = Rc::new(MockSub(Cell::new(0)));
        let result = with_observer(Rc::downgrade(&sub) as Weak<dyn Subscriber>, || 42);
        assert_eq!(result, 42);
    }

    #[test]
    fn without_observer_skips_dependencies_and_restores_outer_observer() {
        let tracked = State::new(0);
        let untracked = State::new(0);
        let sub = Rc::new(MockSub(Cell::new(0)));
        with_observer(Rc::downgrade(&sub) as Weak<dyn Subscriber>, || {
            without_observer(|| {
                assert!(current_observer().is_none());
                untracked.get();
                untracked.set(1);
            });
            tracked.get();
        });
        untracked.set(2);
        assert_eq!(sub.0.get(), 0);
        tracked.set(1);
        assert_eq!(sub.0.get(), 1);
    }

    #[test]
    fn without_observer_restores_notifications_after_panic() {
        let result = std::panic::catch_unwind(|| {
            without_observer(|| panic!("render failed"));
        });
        assert!(result.is_err());
        assert!(!notifications_suppressed());
    }
}
