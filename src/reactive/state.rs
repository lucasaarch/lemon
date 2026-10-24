//! Shared reactive mutable state.

use crate::reactive::observer::{current_observer, Subscriber};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

struct StateInner<T> {
    value: T,
    subscribers: Vec<Weak<dyn Subscriber>>,
}

/// Shared reactive application state, returned by [`Cx::state`](crate::Cx::state).
///
/// Clones refer to the same value. Read with `.get()`, replace with `.set(...)`,
/// or mutate with `.update(...)`. Component-owned state must be declared in a stable
/// order through `cx.state(...)`; `State::new(...)` creates independent external state.
///
/// ```
/// use lemon::State;
/// let count = State::new(0);
/// let shared = count.clone();
/// count.update(|value| *value += 1);
/// assert_eq!(shared.get(), 1);
/// ```
pub struct State<T>(Rc<RefCell<StateInner<T>>>);

impl<T: Clone + 'static> State<T> {
    /// Creates state with no subscribers until something reads it under an effect or render.
    pub fn new(value: T) -> Self {
        State(Rc::new(RefCell::new(StateInner {
            value,
            subscribers: Vec::new(),
        })))
    }

    /// Returns a copy of the current value and records a dependency when called during render.
    pub fn get(&self) -> T {
        let mut inner = self.0.borrow_mut();
        if let Some(obs) = current_observer() {
            inner
                .subscribers
                .retain(|subscriber| subscriber.strong_count() > 0);
            if !inner.subscribers.iter().any(|w| w.ptr_eq(&obs)) {
                inner.subscribers.push(obs);
            }
        }
        inner.value.clone()
    }

    /// Replaces the value and schedules a re-render of dependent views.
    pub fn set(&self, value: T) {
        self.0.borrow_mut().value = value;
        self.notify();
    }

    /// Returns a copy of the current value **without** recording a dependency.
    ///
    /// Use when you need to read a signal's value inside an effect or render but do not want the
    /// current scope to re-run when the signal later changes. Prefer [`get`](Self::get) for
    /// normal reactive reads.
    pub fn peek(&self) -> T {
        self.0.borrow().value.clone()
    }

    /// Mutates the value in place, then notifies subscribers (same as [`set`](Self::set) after `f`).
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        f(&mut self.0.borrow_mut().value);
        self.notify();
    }

    fn notify(&self) {
        if crate::reactive::observer::notifications_suppressed() {
            return;
        }
        let subs: Vec<Rc<dyn Subscriber>> = {
            let mut inner = self.0.borrow_mut();
            let mut upgraded = Vec::with_capacity(inner.subscribers.len());
            inner.subscribers.retain(|w| {
                if let Some(rc) = w.upgrade() {
                    upgraded.push(rc);
                    true
                } else {
                    false
                }
            });
            upgraded
        };
        for sub in subs {
            crate::reactive::observer::notify_subscriber(&sub);
        }
    }
}

impl<T> Clone for State<T> {
    fn clone(&self) -> Self {
        State(Rc::clone(&self.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reactive::observer::{with_observer, RunObserver};
    use std::cell::Cell;
    use std::rc::Rc;

    struct Counter(Cell<u32>);

    impl Subscriber for Counter {
        fn mark_dirty(&self) {
            self.0.set(self.0.get() + 1);
        }
    }

    #[test]
    fn get_returns_initial_value() {
        let s = State::new(42i32);
        assert_eq!(s.get(), 42);
    }

    #[test]
    fn set_updates_value() {
        let s = State::new(0i32);
        s.set(99);
        assert_eq!(s.get(), 99);
    }

    #[test]
    fn update_mutates_value() {
        let s = State::new(10i32);
        s.update(|v| *v += 5);
        assert_eq!(s.get(), 15);
    }

    #[test]
    fn set_notifies_subscriber() {
        let s = State::new(0i32);
        let counter = Rc::new(Counter(Cell::new(0)));
        with_observer(Rc::downgrade(&counter) as _, || {
            s.get();
        });
        s.set(1);
        assert_eq!(counter.0.get(), 1);
    }

    #[test]
    fn dead_subscriber_is_skipped() {
        let s = State::new(0i32);
        let counter = Rc::new(Counter(Cell::new(0)));
        with_observer(Rc::downgrade(&counter) as _, || {
            s.get();
        });
        drop(counter);
        s.set(1); // must not panic
    }

    #[test]
    fn clone_shares_state() {
        let s1 = State::new(5i32);
        let s2 = s1.clone();
        s1.set(10);
        assert_eq!(s2.get(), 10);
    }

    #[test]
    fn reads_prune_subscriptions_from_finished_observer_runs() {
        let state = State::new(0);
        for _ in 0..8 {
            let observer = RunObserver::new(|| {});
            with_observer(Rc::downgrade(&observer) as Weak<dyn Subscriber>, || {
                state.get();
            });
            assert_eq!(state.0.borrow().subscribers.len(), 1);
        }
    }
}
