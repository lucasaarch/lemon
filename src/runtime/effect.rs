use std::cell::RefCell;
use std::rc::{Rc, Weak};
use crate::runtime::observer::{with_observer, Subscriber};

struct EffectInner {
    f: Box<dyn Fn()>,
    self_weak: RefCell<Weak<EffectInner>>,
}

impl Subscriber for EffectInner {
    fn mark_dirty(&self) {
        if let Some(strong) = self.self_weak.borrow().upgrade() {
            with_observer(Rc::downgrade(&strong) as Weak<dyn Subscriber>, || {
                (strong.f)();
            });
        }
    }
}

/// An effect that runs a closure immediately and re-runs it whenever a signal it reads changes.
/// Dropping the effect stops it from re-running.
#[allow(dead_code)]
pub struct Effect(Rc<EffectInner>);

impl Effect {
    pub fn new(f: impl Fn() + 'static) -> Self {
        let inner = Rc::new(EffectInner {
            f: Box::new(f),
            self_weak: RefCell::new(Weak::new()),
        });
        *inner.self_weak.borrow_mut() = Rc::downgrade(&inner);
        Effect::run(&inner);
        Effect(inner)
    }

    fn run(inner: &Rc<EffectInner>) {
        with_observer(Rc::downgrade(inner) as Weak<dyn Subscriber>, || {
            (inner.f)();
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::signal::Signal;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn runs_immediately() {
        let ran = Rc::new(Cell::new(false));
        let r = ran.clone();
        let _e = Effect::new(move || { r.set(true); });
        assert!(ran.get());
    }

    #[test]
    fn reruns_on_signal_change() {
        let s = Signal::new(0i32);
        let count = Rc::new(Cell::new(0u32));
        let c = count.clone();
        let s2 = s.clone();
        let _e = Effect::new(move || { s2.get(); c.set(c.get() + 1); });
        assert_eq!(count.get(), 1);
        s.set(1);
        assert_eq!(count.get(), 2);
        s.set(2);
        assert_eq!(count.get(), 3);
    }

    #[test]
    fn stops_rerunning_after_drop() {
        let s = Signal::new(0i32);
        let count = Rc::new(Cell::new(0u32));
        let c = count.clone();
        let s2 = s.clone();
        { let _e = Effect::new(move || { s2.get(); c.set(c.get() + 1); }); }
        s.set(99);
        assert_eq!(count.get(), 1); // ran once on mount, not again after drop
    }
}
