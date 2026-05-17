use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use crate::runtime::observer::{current_observer, with_observer, Subscriber};

struct DerivedInner<T> {
    f: Box<dyn Fn() -> T>,
    cached: RefCell<Option<T>>,
    stale: Cell<bool>,
    self_weak: RefCell<Weak<DerivedInner<T>>>,
    downstream: RefCell<Vec<Weak<dyn Subscriber>>>,
}

impl<T: Clone + 'static> Subscriber for DerivedInner<T> {
    fn mark_dirty(&self) {
        self.stale.set(true);
        self.cached.borrow_mut().take();
        let subs: Vec<Rc<dyn Subscriber>> = self.downstream
            .borrow()
            .iter()
            .filter_map(|w| w.upgrade())
            .collect();
        for sub in subs { sub.mark_dirty(); }
    }
}

pub struct Derived<T>(Rc<DerivedInner<T>>);

impl<T: Clone + 'static> Derived<T> {
    pub fn new(f: impl Fn() -> T + 'static) -> Self {
        let inner = Rc::new(DerivedInner {
            f: Box::new(f),
            cached: RefCell::new(None),
            stale: Cell::new(true),
            self_weak: RefCell::new(Weak::new()),
            downstream: RefCell::new(Vec::new()),
        });
        *inner.self_weak.borrow_mut() = Rc::downgrade(&inner);
        Derived(inner)
    }

    pub fn get(&self) -> T {
        if let Some(obs) = current_observer() {
            self.0.downstream.borrow_mut().push(obs);
        }
        if self.0.stale.get() {
            let weak = self.0.self_weak.borrow().clone();
            let value = with_observer(weak as Weak<dyn Subscriber>, || (self.0.f)());
            self.0.stale.set(false);
            *self.0.cached.borrow_mut() = Some(value.clone());
            value
        } else {
            self.0.cached.borrow().as_ref().unwrap().clone()
        }
    }
}

impl<T> Clone for Derived<T> {
    fn clone(&self) -> Self { Derived(Rc::clone(&self.0)) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::signal::Signal;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn computes_on_first_get() {
        let s = Signal::new(2i32);
        let d = Derived::new(move || s.get() * 3);
        assert_eq!(d.get(), 6);
    }

    #[test]
    fn caches_result() {
        let count = Rc::new(Cell::new(0u32));
        let c = count.clone();
        let s = Signal::new(1i32);
        let d = Derived::new(move || { c.set(c.get() + 1); s.get() });
        d.get();
        d.get();
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn recomputes_when_signal_changes() {
        let s = Signal::new(10i32);
        let s_clone = s.clone();
        let d = Derived::new(move || s_clone.get() + 1);
        assert_eq!(d.get(), 11);
        s.set(20);
        assert_eq!(d.get(), 21);
    }
}
