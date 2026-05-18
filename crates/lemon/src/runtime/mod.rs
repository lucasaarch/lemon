pub mod cx;
pub mod derived;
pub mod effect;
pub mod observer;
pub mod signal;

use std::cell::RefCell;
use std::rc::Rc;

use crate::diff::{diff, NodePath, Patch};
use crate::element::{
    content::TextContent,
    style::{ColorSource, PaintProps},
    types::{BoxElement, ButtonElement, ComponentElement, TextElement},
    Element,
};
use cx::Cx;
use effect::Effect;

pub use signal::Signal;

type ViewFn = Rc<dyn Fn(&Cx) -> Element>;

struct ComponentSlot {
    path: NodePath,
    previous: Option<Element>,
    pending: Rc<RefCell<Option<Element>>>,
    view: Rc<RefCell<ViewFn>>,
    cx: Rc<RefCell<Cx>>,
    children: Vec<ComponentSlot>,
    deferred_effects: Rc<RefCell<Vec<Effect>>>,
    _effect: Effect,
}

pub struct Runtime {
    slots: Vec<ComponentSlot>,
    patch_queue: Vec<Patch>,
    deferred_effects: Rc<RefCell<Vec<Effect>>>,
}

impl Runtime {
    pub fn new() -> Self {
        Runtime {
            slots: Vec::new(),
            patch_queue: Vec::new(),
            deferred_effects: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn mount(&mut self, f: impl Fn(&Cx) -> Element + 'static) {
        let mut slot = create_component_slot(
            NodePath::root(),
            Rc::new(f),
            false,
            Rc::clone(&self.deferred_effects),
        );
        if let Some(initial) = slot.pending.borrow_mut().take() {
            slot.previous = Some(initial);
        }
        self.slots.push(slot);
    }

    /// Apply any pending re-renders: diff old vs new tree, accumulate patches.
    pub fn flush_effects(&mut self) {
        for slot in &mut self.slots {
            render_slot(slot, &mut self.patch_queue);
        }
    }

    pub fn take_patches(&mut self) -> Vec<Patch> {
        std::mem::take(&mut self.patch_queue)
    }

    /// Run mount-time `use_effect` hooks queued during render (after first paint).
    pub fn flush_deferred_effects(&mut self) {
        cx::flush_deferred_sink(&self.deferred_effects);
    }

    /// Current root element tree after the last render (mount or flush).
    pub fn root_element(&self) -> Option<Element> {
        self.slots.first()?.previous.clone()
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

fn create_component_slot(
    path: NodePath,
    view: ViewFn,
    lazy_effect: bool,
    deferred_effects: Rc<RefCell<Vec<Effect>>>,
) -> ComponentSlot {
    let view_cell = Rc::new(RefCell::new(view));
    let cx = Rc::new(RefCell::new(Cx::new()));
    cx.borrow()
        .set_deferred_sink(Rc::clone(&deferred_effects));
    let pending = Rc::new(RefCell::new(None));

    let view_cell2 = Rc::clone(&view_cell);
    let cx2 = Rc::clone(&cx);
    let pending2 = Rc::clone(&pending);

    let effect_body = move || {
        cx2.borrow().reset_hooks();
        let tree = view_cell2.borrow()(&cx2.borrow());
        *pending2.borrow_mut() = Some(freeze_element(&tree));
    };

    let effect = if lazy_effect {
        Effect::new_lazy(effect_body)
    } else {
        Effect::new(effect_body)
    };

    ComponentSlot {
        path,
        previous: None,
        pending,
        view: view_cell,
        cx,
        children: Vec::new(),
        deferred_effects,
        _effect: effect,
    }
}

fn render_slot(slot: &mut ComponentSlot, patches: &mut Vec<Patch>) {
    if let Some(new_tree) = slot.pending.borrow_mut().take() {
        let next_previous = new_tree.clone();
        if let Some(old_tree) = slot.previous.take() {
            let deferred = Rc::clone(&slot.deferred_effects);
            diff_with_nested_components(
                &mut slot.children,
                old_tree,
                new_tree,
                slot.path.clone(),
                patches,
                deferred,
            );
        }
        slot.previous = Some(next_previous);
    }
}

fn same_component_identity(old: &ComponentElement, new: &ComponentElement) -> bool {
    old.identity() == new.identity() && old.key() == new.key()
}

fn find_slot_mut<'a>(
    slots: &'a mut [ComponentSlot],
    path: &NodePath,
) -> Option<&'a mut ComponentSlot> {
    for slot in slots.iter_mut() {
        if &slot.path == path {
            return Some(slot);
        }
        if path.0.len() > slot.path.0.len() && path.0.starts_with(&slot.path.0) {
            if let Some(found) = find_slot_mut(&mut slot.children, path) {
                return Some(found);
            }
        }
    }
    None
}

fn unmount_component_slot(slots: &mut Vec<ComponentSlot>, path: &NodePath) -> bool {
    if let Some(index) = slots.iter().position(|slot| &slot.path == path) {
        slots.remove(index);
        return true;
    }
    for slot in slots.iter_mut() {
        if unmount_component_slot(&mut slot.children, path) {
            return true;
        }
    }
    false
}

fn mount_component_slot(
    parent_slots: &mut Vec<ComponentSlot>,
    path: NodePath,
    component: ComponentElement,
    deferred_effects: Rc<RefCell<Vec<Effect>>>,
) {
    let mut slot = create_component_slot(path, component.view(), true, deferred_effects);
    bootstrap_nested_slot(&mut slot);
    parent_slots.push(slot);
}

fn render_without_subscribing(slot: &ComponentSlot) -> Element {
    slot.cx.borrow().reset_hooks();
    freeze_element(&slot.view.borrow()(&slot.cx.borrow()))
}

/// Re-render a slot for a parent-driven update without re-entering the slot's effect.
fn sync_render(slot: &ComponentSlot) {
    *slot.pending.borrow_mut() = Some(render_without_subscribing(slot));
}

/// First mount of a nested slot: simulate the parent mount render plus the current flush.
fn bootstrap_nested_slot(slot: &mut ComponentSlot) {
    let first = render_without_subscribing(slot);
    let second = render_without_subscribing(slot);
    slot.previous = Some(first);
    *slot.pending.borrow_mut() = Some(second);
}

fn handle_component_pair(
    parent_slots: &mut Vec<ComponentSlot>,
    old: &ComponentElement,
    new: ComponentElement,
    path: NodePath,
    patches: &mut Vec<Patch>,
    deferred_effects: Rc<RefCell<Vec<Effect>>>,
) {
    let new_for_patch = new.clone();
    if same_component_identity(old, &new) {
        if let Some(slot) = find_slot_mut(parent_slots, &path) {
            *slot.view.borrow_mut() = new.view();
            sync_render(slot);
            render_slot(slot, patches);
        } else {
            mount_component_slot(parent_slots, path.clone(), new, Rc::clone(&deferred_effects));
            if let Some(slot) = find_slot_mut(parent_slots, &path) {
                render_slot(slot, patches);
            }
        }
        patches.push(Patch::UpdateComponent {
            node: path,
            component: new_for_patch,
        });
    } else {
        unmount_component_slot(parent_slots, &path);
        patches.push(Patch::UnmountComponent { node: path.clone() });
        patches.push(Patch::MountComponent {
            node: path.clone(),
            component: new_for_patch.clone(),
        });
        mount_component_slot(
            parent_slots,
            path.clone(),
            new,
            Rc::clone(&deferred_effects),
        );
        if let Some(slot) = find_slot_mut(parent_slots, &path) {
            render_slot(slot, patches);
        }
    }
}

fn diff_container_props(
    old: &BoxElement,
    new: &BoxElement,
    path: &NodePath,
    patches: &mut Vec<Patch>,
) {
    if old.style != new.style {
        patches.push(Patch::UpdateStyle {
            node: path.clone(),
            style: new.style.clone(),
        });
    }
    let old_paint = old.paint.resolve();
    let new_paint = new.paint.resolve();
    if old_paint != new_paint {
        patches.push(Patch::UpdatePaint {
            node: path.clone(),
            paint: new_paint,
        });
    }
}

fn diff_children_slots(
    slots: &mut Vec<ComponentSlot>,
    old_children: &[Element],
    new_children: &[Element],
    parent: &NodePath,
    patches: &mut Vec<Patch>,
    deferred_effects: Rc<RefCell<Vec<Effect>>>,
) {
    let min = old_children.len().min(new_children.len());
    for i in 0..min {
        diff_with_nested_components(
            slots,
            old_children[i].clone(),
            new_children[i].clone(),
            parent.child(i),
            patches,
            Rc::clone(&deferred_effects),
        );
    }

    for (offset, element) in new_children.iter().enumerate().skip(min) {
        let index = min + offset;
        let path = parent.child(index);
        match element {
            Element::Component(component) => {
                patches.push(Patch::MountComponent {
                    node: path.clone(),
                    component: component.clone(),
                });
                mount_component_slot(
                    slots,
                    path,
                    component.clone(),
                    Rc::clone(&deferred_effects),
                );
            }
            element => patches.push(Patch::InsertChild {
                parent: parent.clone(),
                index,
                element: element.clone(),
            }),
        }
    }

    for i in (min..old_children.len()).rev() {
        let path = parent.child(i);
        match &old_children[i] {
            Element::Component(_) => {
                unmount_component_slot(slots, &path);
                patches.push(Patch::UnmountComponent { node: path });
            }
            _ => patches.push(Patch::RemoveChild {
                parent: parent.clone(),
                index: i,
            }),
        }
    }
}

fn sync_slots_for_emitted_patches(
    slots: &mut Vec<ComponentSlot>,
    emitted: &[Patch],
    deferred_effects: Rc<RefCell<Vec<Effect>>>,
) {
    for patch in emitted {
        match patch {
            Patch::MountComponent { node, component } if find_slot_mut(slots, node).is_none() => {
                mount_component_slot(
                    slots,
                    node.clone(),
                    component.clone(),
                    Rc::clone(&deferred_effects),
                );
            }
            Patch::UnmountComponent { node } => {
                unmount_component_slot(slots, node);
            }
            Patch::UpdateComponent { node, component } => {
                if let Some(slot) = find_slot_mut(slots, node) {
                    *slot.view.borrow_mut() = component.view();
                    sync_render(slot);
                } else {
                    mount_component_slot(
                        slots,
                        node.clone(),
                        component.clone(),
                        Rc::clone(&deferred_effects),
                    );
                }
            }
            _ => {}
        }
    }
}

fn diff_with_nested_components(
    slots: &mut Vec<ComponentSlot>,
    old: Element,
    new: Element,
    path: NodePath,
    patches: &mut Vec<Patch>,
    deferred_effects: Rc<RefCell<Vec<Effect>>>,
) {
    match (old, new) {
        (Element::Component(old_component), Element::Component(new_component)) => {
            handle_component_pair(
                slots,
                &old_component,
                new_component,
                path,
                patches,
                deferred_effects,
            );
        }
        (Element::Column(old), Element::Column(new))
        | (Element::Row(old), Element::Row(new))
        | (Element::Box_(old), Element::Box_(new)) => {
            diff_container_props(&old, &new, &path, patches);
            diff_children_slots(
                slots,
                &old.children,
                &new.children,
                &path,
                patches,
                deferred_effects,
            );
        }
        (old, new) => {
            let emitted = diff(old, new, path);
            sync_slots_for_emitted_patches(slots, &emitted, deferred_effects);
            patches.extend(emitted);
        }
    }
}

fn freeze_element(element: &Element) -> Element {
    match element {
        Element::Text(text) => Element::Text(TextElement {
            content: TextContent::Static(text.content.resolve()),
            style: text.style.clone(),
            key: text.key.clone(),
        }),
        Element::Box_(container) => Element::Box_(freeze_box(container)),
        Element::Row(container) => Element::Row(freeze_box(container)),
        Element::Column(container) => Element::Column(freeze_box(container)),
        Element::Button(button) => Element::Button(ButtonElement {
            label: TextContent::Static(button.label.resolve()),
            style: button.style.clone(),
            paint: freeze_paint(&button.paint),
            on_click: button.on_click.clone(),
            key: button.key.clone(),
        }),
        Element::Image(image) => Element::Image(image.clone()),
        Element::Component(component) => Element::Component(component.clone()),
        Element::Fragment(children) => {
            Element::Fragment(children.iter().map(freeze_element).collect())
        }
        Element::None => Element::None,
    }
}

fn freeze_box(container: &BoxElement) -> BoxElement {
    BoxElement {
        style: container.style.clone(),
        paint: freeze_paint(&container.paint),
        children: container.children.iter().map(freeze_element).collect(),
        key: container.key.clone(),
    }
}

fn freeze_paint(paint: &PaintProps) -> PaintProps {
    let resolved = paint.resolve();
    PaintProps {
        background: resolved.background.map(ColorSource::Static),
        border_color: resolved.border_color.map(ColorSource::Static),
        border_width: resolved.border_width,
        radius: resolved.radius,
    }
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
    fn deferred_use_effect_runs_on_flush() {
        use std::cell::Cell;
        use std::rc::Rc;

        let ran = Rc::new(Cell::new(false));
        let flag = ran.clone();
        let mut rt = Runtime::new();
        rt.mount(move |cx| {
            let flag = flag.clone();
            cx.use_effect(move || {
                flag.set(true);
            });
            Text::new("x").into_element()
        });
        assert!(!ran.get());
        rt.flush_deferred_effects();
        assert!(ran.get());
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
        let found = patches
            .iter()
            .any(|p| matches!(p, Patch::UpdateText { content, .. } if content == "after"));
        assert!(
            found,
            "expected UpdateText with 'after'; got {patches:?} (len={})",
            patches.len()
        );
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
        let found = patches
            .iter()
            .any(|p| matches!(p, Patch::UpdateText { content, .. } if content == "2"));
        assert!(found, "expected UpdateText with '2'");
    }

    #[test]
    fn nested_component_state_survives_parent_rerender_when_identity_matches() {
        use crate::element::builders::{Column, Component};

        fn child(cx: &Cx) -> Element {
            let count = cx.use_signal(0u32);
            count.update(|value| *value += 1);
            Text::new(move || count.get().to_string()).into_element()
        }

        let trigger = Signal::new(0u32);
        let t2 = trigger.clone();
        let mut runtime = Runtime::new();
        runtime.mount(move |_cx| {
            t2.get();
            Column::new()
                .child(Component::new(child).key(1))
                .into_element()
        });

        trigger.set(1);
        runtime.flush_effects();
        let patches = runtime.take_patches();

        assert!(patches
            .iter()
            .any(|patch| matches!(patch, Patch::UpdateText { content, .. } if content == "2")));
    }

    #[test]
    fn unmounting_component_slot_drops_cx_hooks() {
        use crate::element::builders::{Column, Component};

        fn counting_child(cx: &Cx) -> Element {
            let count = cx.use_signal(0u32);
            count.update(|value| *value += 1);
            Text::new(move || count.get().to_string()).into_element()
        }

        fn child_b(_cx: &Cx) -> Element {
            Text::new("b").into_element()
        }

        let phase = Signal::new(0u32);
        let p2 = phase.clone();
        let mut runtime = Runtime::new();
        runtime.mount(move |_cx| {
            let view = match p2.get() {
                0 | 2 => counting_child,
                _ => child_b,
            };
            Column::new()
                .child(Component::new(view).key(1))
                .into_element()
        });

        phase.set(1);
        runtime.flush_effects();
        phase.set(2);
        runtime.flush_effects();
        let patches = runtime.take_patches();

        assert!(patches.iter().any(
            |patch| matches!(patch, Patch::UpdateText { content, .. } if content == "2")
        ));
    }

    #[test]
    fn nested_component_identity_change_unmounts_and_mounts() {
        use crate::element::builders::{Column, Component};

        fn child_a(_cx: &Cx) -> Element {
            Text::new("a").into_element()
        }

        fn child_b(_cx: &Cx) -> Element {
            Text::new("b").into_element()
        }

        let swap = Signal::new(false);
        let s2 = swap.clone();
        let mut runtime = Runtime::new();
        runtime.mount(move |_cx| {
            let child = if s2.get() { child_b } else { child_a };
            Column::new()
                .child(Component::new(child).key(1))
                .into_element()
        });

        swap.set(true);
        runtime.flush_effects();
        let patches = runtime.take_patches();

        assert!(patches
            .iter()
            .any(|patch| matches!(patch, Patch::UnmountComponent { .. })));
        assert!(patches
            .iter()
            .any(|patch| matches!(patch, Patch::MountComponent { .. })));
    }
}
