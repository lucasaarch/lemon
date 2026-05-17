use crate::runtime::observer::{with_observer, Subscriber};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

struct EffectInner {
    f: Box<dyn Fn()>,
    self_weak: RefCell<Weak<EffectInner>>,
    is_running: Cell<bool>,
}

impl Subscriber for EffectInner {
    fn mark_dirty(&self) {
        // Prevent recursive effect invocation while already running
        if self.is_running.get() {
            return;
        }
        if let Some(strong) = self.self_weak.borrow().upgrade() {
            self.is_running.set(true);
            with_observer(Rc::downgrade(&strong) as Weak<dyn Subscriber>, || {
                (strong.f)();
            });
            self.is_running.set(false);
        }
    }
}

/// An effect that runs a closure immediately and re-runs it whenever a signal it reads changes.
/// Dropping the effect stops it from re-running.
pub struct Effect {
    _inner: Rc<EffectInner>,
}

impl Effect {
    pub fn new(f: impl Fn() + 'static) -> Self {
        let inner = Rc::new(EffectInner {
            f: Box::new(f),
            self_weak: RefCell::new(Weak::new()),
            is_running: Cell::new(true), // Set to true during initial run
        });
        *inner.self_weak.borrow_mut() = Rc::downgrade(&inner);
        with_observer(Rc::downgrade(&inner) as Weak<dyn Subscriber>, || {
            (inner.f)();
        });
        inner.is_running.set(false); // Reset to false after initial run
        Effect { _inner: inner }
    }

    /// Like [`new`](Self::new), but does not run `f` until a tracked signal marks the effect dirty.
    pub fn new_lazy(f: impl Fn() + 'static) -> Self {
        let inner = Rc::new(EffectInner {
            f: Box::new(f),
            self_weak: RefCell::new(Weak::new()),
            is_running: Cell::new(false),
        });
        *inner.self_weak.borrow_mut() = Rc::downgrade(&inner);
        Effect { _inner: inner }
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
        let _e = Effect::new(move || {
            r.set(true);
        });
        assert!(ran.get());
    }

    #[test]
    fn reruns_on_signal_change() {
        let s = Signal::new(0i32);
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
        let s = Signal::new(0i32);
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
}
