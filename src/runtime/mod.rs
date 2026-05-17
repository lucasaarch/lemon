pub mod cx;
pub mod derived;
pub mod effect;
pub mod observer;
pub mod signal;

use std::cell::RefCell;
use std::rc::Rc;

use crate::diff::{diff, NodePath, Patch};
use crate::element::Element;
use cx::Cx;
use effect::Effect;

pub use signal::Signal;

type ComponentFn = Rc<dyn Fn(&Cx) -> Element>;

struct ComponentSlot {
    cx: Rc<RefCell<Cx>>,
    view: ComponentFn,
    previous: Option<Element>,
    pending: Rc<RefCell<Option<Element>>>,
    _effect: Effect,
}

pub struct Runtime {
    slots: Vec<ComponentSlot>,
    patch_queue: Vec<Patch>,
}

impl Runtime {
    pub fn new() -> Self {
        Runtime { slots: Vec::new(), patch_queue: Vec::new() }
    }

    pub fn mount(&mut self, f: impl Fn(&Cx) -> Element + 'static) {
        let cx = Rc::new(RefCell::new(Cx::new()));
        let view: ComponentFn = Rc::new(f);
        let pending: Rc<RefCell<Option<Element>>> = Rc::new(RefCell::new(None));

        // Initial render — no diff, just store the first tree
        let first_tree = {
            let cx_ref = cx.borrow();
            cx_ref.reset_hooks();
            view(&*cx_ref)
        };

        // Reactive effect — re-runs whenever signals read inside view() change
        let view2 = Rc::clone(&view);
        let cx2 = Rc::clone(&cx);
        let pending2 = Rc::clone(&pending);

        let effect = Effect::new(move || {
            {
                let cx_ref = cx2.borrow();
                cx_ref.reset_hooks();
            }
            let tree = view2(&*cx2.borrow());
            *pending2.borrow_mut() = Some(tree);
        });

        self.slots.push(ComponentSlot {
            cx,
            view,
            previous: Some(first_tree),
            pending,
            _effect: effect,
        });
    }

    /// Apply any pending re-renders: diff old vs new tree, accumulate patches.
    pub fn flush_effects(&mut self) {
        for slot in &mut self.slots {
            if let Some(new_tree) = slot.pending.borrow_mut().take() {
                if let Some(old_tree) = slot.previous.take() {
                    let patches = diff(old_tree, new_tree, NodePath::root());
                    // Reconstruct current tree for next diff by re-running view
                    let current = {
                        let cx_ref = slot.cx.borrow();
                        cx_ref.reset_hooks();
                        (slot.view)(&*cx_ref)
                    };
                    slot.previous = Some(current);
                    self.patch_queue.extend(patches);
                }
            }
        }
    }

    pub fn take_patches(&mut self) -> Vec<Patch> {
        std::mem::take(&mut self.patch_queue)
    }
}

impl Default for Runtime {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::builders::Text;

    #[test]
    fn no_patches_on_first_mount() {
        let mut rt = Runtime::new();
        rt.mount(|_cx| Text::new("hello").into_element());
        assert!(rt.take_patches().is_empty());
    }

    #[test]
    fn signal_change_produces_update_text_patch() {
        let s = Signal::new("before".to_owned());
        let s2 = s.clone();
        let mut rt = Runtime::new();
        rt.mount(move |_cx| {
            let s3 = s2.clone();
            Text::new(move || s3.get()).into_element()
        });
        s.set("after".to_owned());
        rt.flush_effects();
        let patches = rt.take_patches();
        let found = patches.iter().any(|p| {
            matches!(p, Patch::UpdateText { content, .. } if content == "after")
        });
        assert!(found, "expected UpdateText with 'after'; got {patches:?} (len={})", patches.len());
    }

    #[test]
    fn signal_from_use_signal_persists_across_rerenders() {
        let trigger = Signal::new(0u32);
        let t2 = trigger.clone();
        let mut rt = Runtime::new();
        rt.mount(move |_cx| {
            let count = _cx.use_signal(0u32);
            count.update(|n| *n += 1);
            t2.get(); // track trigger signal to force re-render
            Text::new(move || count.get().to_string()).into_element()
        });
        // After mount: count is 1 (incremented on first render)
        trigger.set(1);
        rt.flush_effects();
        let patches = rt.take_patches();
        // count should now be 2 — same signal persisted via hook index
        let found = patches.iter().any(|p| {
            matches!(p, Patch::UpdateText { content, .. } if content == "2")
        });
        assert!(found, "expected UpdateText with '2'");
    }
}
