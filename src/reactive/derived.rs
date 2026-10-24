//! Cached reactive computations.

use crate::reactive::observer::{current_observer, with_observer, RunObserver, Subscriber};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

struct DerivedInner<T> {
    f: Box<dyn Fn() -> T>,
    cached: RefCell<Option<T>>,
    stale: Cell<bool>,
    observer: RefCell<Option<Rc<RunObserver>>>,
    downstream: RefCell<Vec<Weak<dyn Subscriber>>>,
}

impl<T: Clone + PartialEq + 'static> DerivedInner<T> {
    fn evaluate(inner: &Rc<Self>) -> T {
        let weak = Rc::downgrade(inner);
        let observer = RunObserver::new(move || {
            if let Some(inner) = weak.upgrade() {
                Self::recompute(&inner);
            }
        });
        if let Some(previous) = inner.observer.replace(Some(observer.clone())) {
            previous.deactivate();
        }
        with_observer(Rc::downgrade(&observer) as Weak<dyn Subscriber>, || {
            (inner.f)()
        })
    }

    fn recompute(inner: &Rc<Self>) {
        let new_value = Self::evaluate(inner);

        let changed = match inner.cached.borrow().as_ref() {
            Some(old) => old != &new_value,
            None => true,
        };
        if !changed {
            return;
        }

        inner.stale.set(false);
        *inner.cached.borrow_mut() = Some(new_value);

        let subs: Vec<Rc<dyn Subscriber>> = {
            let mut downstream = inner.downstream.borrow_mut();
            let mut upgraded = Vec::with_capacity(downstream.len());
            downstream.retain(|w| {
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

/// Cached reactive computation, notifying dependents only when its value changes.
/// Each evaluation replaces its signal subscriptions, including conditional reads.
///
/// ```
/// use lemon::reactive::{Derived, State};
/// let source = State::new(2);
/// let input = source.clone();
/// let doubled = Derived::new(move || input.get() * 2);
/// assert_eq!(doubled.get(), 4);
/// ```
pub struct Derived<T>(Rc<DerivedInner<T>>);

impl<T: Clone + PartialEq + 'static> Derived<T> {
    /// Creates a computation evaluated lazily on its first read.
    pub fn new(f: impl Fn() -> T + 'static) -> Self {
        let inner = Rc::new(DerivedInner {
            f: Box::new(f),
            cached: RefCell::new(None),
            stale: Cell::new(true),
            observer: RefCell::new(None),
            downstream: RefCell::new(Vec::new()),
        });
        Derived(inner)
    }

    /// Returns the cached value, computing it if necessary and tracking the current subscriber.
    pub fn get(&self) -> T {
        if let Some(obs) = current_observer() {
            let mut downstream = self.0.downstream.borrow_mut();
            downstream.retain(|subscriber| subscriber.strong_count() > 0);
            if !downstream.iter().any(|w| w.ptr_eq(&obs)) {
                downstream.push(obs);
            }
        }
        if self.0.stale.get() {
            let value = DerivedInner::evaluate(&self.0);
            self.0.stale.set(false);
            *self.0.cached.borrow_mut() = Some(value.clone());
            value
        } else {
            self.0.cached.borrow().as_ref().unwrap().clone()
        }
    }
}

impl<T> Clone for Derived<T> {
    fn clone(&self) -> Self {
        Derived(Rc::clone(&self.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reactive::effect::Effect;
    use crate::reactive::state::State;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn computes_on_first_get() {
        let s = State::new(2i32);
        let d = Derived::new(move || s.get() * 3);
        assert_eq!(d.get(), 6);
    }

    #[test]
    fn caches_result() {
        let count = Rc::new(Cell::new(0u32));
        let c = count.clone();
        let s = State::new(1i32);
        let d = Derived::new(move || {
            c.set(c.get() + 1);
            s.get()
        });
        d.get();
        d.get();
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn recomputes_when_signal_changes() {
        let s = State::new(10i32);
        let s_clone = s.clone();
        let d = Derived::new(move || s_clone.get() + 1);
        assert_eq!(d.get(), 11);
        s.set(20);
        assert_eq!(d.get(), 21);
    }

    #[test]
    fn does_not_notify_downstream_when_computed_value_unchanged() {
        let s = State::new(10i32);
        let s_clone = s.clone();
        let d = Derived::new(move || s_clone.get() / 10 * 10);

        let count = Rc::new(Cell::new(0u32));
        let c = count.clone();
        let d2 = d.clone();
        let _e = Effect::new(move || {
            d2.get();
            c.set(c.get() + 1);
        });
        assert_eq!(count.get(), 1);

        s.set(11);
        assert_eq!(d.get(), 10);
        assert_eq!(count.get(), 1);

        s.set(20);
        assert_eq!(d.get(), 20);
        assert_eq!(count.get(), 2);
    }

    #[test]
    fn derived_stops_listening_to_states_not_read_on_latest_evaluation() {
        let choose_first = State::new(true);
        let first = State::new(0);
        let second = State::new(0);
        let evaluations = Rc::new(Cell::new(0));
        let (choice, a, b, count) = (
            choose_first.clone(),
            first.clone(),
            second.clone(),
            evaluations.clone(),
        );
        let derived = Derived::new(move || {
            count.set(count.get() + 1);
            if choice.get() {
                a.get()
            } else {
                b.get()
            }
        });
        assert_eq!(derived.get(), 0);
        choose_first.set(false);
        assert_eq!(evaluations.get(), 2);
        first.set(1);
        assert_eq!(evaluations.get(), 2);
        second.set(1);
        assert_eq!(evaluations.get(), 3);
        assert_eq!(derived.get(), 1);
        choose_first.set(true);
        assert_eq!(evaluations.get(), 4);
        second.set(2);
        assert_eq!(evaluations.get(), 4);
        first.set(2);
        assert_eq!(evaluations.get(), 5);
        assert_eq!(derived.get(), 2);
    }

    #[test]
    fn derived_stops_notifying_effects_that_no_longer_read_it() {
        let choose_derived = State::new(true);
        let source = State::new(0);
        let input = source.clone();
        let derived = Derived::new(move || input.get());
        let choice = choose_derived.clone();
        let value = derived.clone();
        let runs = Rc::new(Cell::new(0));
        let count = runs.clone();
        let _effect = Effect::new(move || {
            count.set(count.get() + 1);
            if choice.get() {
                value.get();
            }
        });
        choose_derived.set(false);
        assert_eq!(runs.get(), 2);
        source.set(1);
        assert_eq!(runs.get(), 2);
    }

    #[test]
    fn reads_prune_downstream_subscriptions_from_finished_runs() {
        let derived = Derived::new(|| 1);
        for _ in 0..8 {
            let observer = RunObserver::new(|| {});
            with_observer(Rc::downgrade(&observer) as Weak<dyn Subscriber>, || {
                derived.get();
            });
            assert_eq!(derived.0.downstream.borrow().len(), 1);
        }
    }
}
