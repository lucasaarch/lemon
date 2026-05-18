use crate::runtime::observer::{current_observer, Subscriber};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

struct SignalInner<T> {
    value: T,
    subscribers: Vec<Weak<dyn Subscriber>>,
}

/// Mutable reactive cell shared via [`Clone`].
///
/// Prefer [`Cx::use_signal`](crate::Cx::use_signal) inside views. Standalone [`Signal::new`] is useful for state held
/// outside the UI tree (e.g. in `main`) that you pass into a view by cloning before `run`.
pub struct Signal<T>(Rc<RefCell<SignalInner<T>>>);

impl<T: Clone + 'static> Signal<T> {
    /// Creates a signal with no subscribers until something reads it under an effect or render.
    pub fn new(value: T) -> Self {
        Signal(Rc::new(RefCell::new(SignalInner {
            value,
            subscribers: Vec::new(),
        })))
    }

    /// Returns a copy of the current value and records a dependency when called during render.
    pub fn get(&self) -> T {
        let mut inner = self.0.borrow_mut();
        if let Some(obs) = current_observer() {
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

    /// Mutates the value in place, then notifies subscribers (same as [`set`](Self::set) after `f`).
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        f(&mut self.0.borrow_mut().value);
        self.notify();
    }

    fn notify(&self) {
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
            sub.mark_dirty();
        }
    }
}

impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Signal(Rc::clone(&self.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::observer::with_observer;
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
        let s = Signal::new(42i32);
        assert_eq!(s.get(), 42);
    }

    #[test]
    fn set_updates_value() {
        let s = Signal::new(0i32);
        s.set(99);
        assert_eq!(s.get(), 99);
    }

    #[test]
    fn update_mutates_value() {
        let s = Signal::new(10i32);
        s.update(|v| *v += 5);
        assert_eq!(s.get(), 15);
    }

    #[test]
    fn set_notifies_subscriber() {
        let s = Signal::new(0i32);
        let counter = Rc::new(Counter(Cell::new(0)));
        with_observer(Rc::downgrade(&counter) as _, || {
            s.get();
        });
        s.set(1);
        assert_eq!(counter.0.get(), 1);
    }

    #[test]
    fn dead_subscriber_is_skipped() {
        let s = Signal::new(0i32);
        let counter = Rc::new(Counter(Cell::new(0)));
        with_observer(Rc::downgrade(&counter) as _, || {
            s.get();
        });
        drop(counter);
        s.set(1); // must not panic
    }

    #[test]
    fn clone_shares_state() {
        let s1 = Signal::new(5i32);
        let s2 = s1.clone();
        s1.set(10);
        assert_eq!(s2.get(), 10);
    }
}
