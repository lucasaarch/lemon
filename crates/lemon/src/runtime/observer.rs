use std::cell::{Cell, RefCell};
use std::rc::Weak;

pub trait Subscriber {
    fn mark_dirty(&self);
}

thread_local! {
    static OBSERVER_STACK: RefCell<Vec<Weak<dyn Subscriber>>> = RefCell::new(Vec::new());
    static SUPPRESS_NOTIFY: Cell<bool> = const { Cell::new(false) };
}

/// Runs `f` without recording signal dependencies or notifying subscribers on writes.
pub fn without_observer<R>(f: impl FnOnce() -> R) -> R {
    let prev_notify = SUPPRESS_NOTIFY.with(|flag| {
        let prev = flag.get();
        flag.set(true);
        prev
    });
    let result = OBSERVER_STACK.with(|stack| {
        let stack_len = stack.borrow().len();
        let out = f();
        stack.borrow_mut().truncate(stack_len);
        out
    });
    SUPPRESS_NOTIFY.with(|flag| flag.set(prev_notify));
    result
}

pub(crate) fn notifications_suppressed() -> bool {
    SUPPRESS_NOTIFY.with(|flag| flag.get())
}

struct StackGuard;

impl Drop for StackGuard {
    fn drop(&mut self) {
        OBSERVER_STACK.with(|stack| {
            stack.borrow_mut().pop();
        });
    }
}

pub fn with_observer<R>(observer: Weak<dyn Subscriber>, f: impl FnOnce() -> R) -> R {
    OBSERVER_STACK.with(|stack| stack.borrow_mut().push(observer));
    let _guard = StackGuard;
    f()
}

pub fn current_observer() -> Option<Weak<dyn Subscriber>> {
    OBSERVER_STACK.with(|stack| stack.borrow().last().cloned())
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
