use crate::runtime::derived::Derived;
use crate::runtime::effect::Effect;
use crate::runtime::signal::Signal;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct Cx {
    hooks: RefCell<Vec<Box<dyn Any>>>,
    index: Cell<usize>,
    deferred_sink: RefCell<Option<Rc<RefCell<Vec<Effect>>>>>,
}

impl Cx {
    pub fn new() -> Self {
        Cx {
            hooks: RefCell::new(Vec::new()),
            index: Cell::new(0),
            deferred_sink: RefCell::new(None),
        }
    }

    pub(crate) fn set_deferred_sink(&self, sink: Rc<RefCell<Vec<Effect>>>) {
        *self.deferred_sink.borrow_mut() = Some(sink);
    }

    /// Must be called before each re-render of this component.
    pub fn reset_hooks(&self) {
        self.index.set(0);
    }

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

    pub fn use_memo<T: Clone + PartialEq + 'static>(&self, f: impl Fn() -> T + 'static) -> Derived<T> {
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
}
