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

/// Reactive renderer: mounts a root view, diffs trees, and collects [`Patch`]es.
///
/// [`run`](crate::platform::run) owns a `Runtime` for you. Use this type directly only in tests or
/// custom hosts that call [`flush_effects`](Self::flush_effects) and apply patches themselves.
pub struct Runtime {
    slots: Vec<ComponentSlot>,
    patch_queue: Vec<Patch>,
    deferred_effects: Rc<RefCell<Vec<Effect>>>,
}

impl Runtime {
    /// Empty runtime with no mounted root.
    pub fn new() -> Self {
        Runtime {
            slots: Vec::new(),
            patch_queue: Vec::new(),
            deferred_effects: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Mounts a single root view function (same shape as the closure passed to [`run`](crate::platform::run)).
    pub fn mount(&mut self, f: impl Fn(&Cx) -> Element + 'static) {
        let mut slot = create_component_slot(
            NodePath::root(),
            Rc::new(f),
            false,
            Rc::clone(&self.deferred_effects),
        );
        let initial = slot.pending.borrow_mut().take();
        if let Some(initial) = initial {
            slot.previous = Some(initial.clone());
        }
        self.slots.push(slot);
        if let Some(initial) = self.slots.first().and_then(|s| s.previous.clone()) {
            bootstrap_initial_components(
                &mut self.slots,
                &initial,
                NodePath::root(),
                &mut self.patch_queue,
                Rc::clone(&self.deferred_effects),
            );
        }
    }

    /// Re-runs dirty views, diffs against the previous tree, and queues patches.
    ///
    /// Call after changing a [`Signal`] (or other tracked state), then [`take_patches`](Self::take_patches).
    pub fn flush_effects(&mut self) {
        crate::lemon_trace!(Runtime, "flush_effects ({} root slot(s))", self.slots.len());
        for slot in &mut self.slots {
            flush_slot_subtree(slot, &mut self.patch_queue);
        }
    }

    /// Returns and clears patches produced since the last `take_patches`.
    pub fn take_patches(&mut self) -> Vec<Patch> {
        std::mem::take(&mut self.patch_queue)
    }

    /// Run mount-time `use_effect` hooks queued during render (after first paint).
    pub fn flush_deferred_effects(&mut self) {
        cx::flush_deferred_sink(&self.deferred_effects);
    }

    /// Frozen element tree after the last successful render (for [`RetainedTree::mount`](crate::retained::RetainedTree::mount)).
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
    cx.borrow().set_deferred_sink(Rc::clone(&deferred_effects));
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

fn flush_slot_subtree(slot: &mut ComponentSlot, patches: &mut Vec<Patch>) {
    for child in &mut slot.children {
        flush_slot_subtree(child, patches);
    }
    render_slot(slot, patches);
}

fn render_slot(slot: &mut ComponentSlot, patches: &mut Vec<Patch>) {
    if let Some(new_tree) = slot.pending.borrow_mut().take() {
        let next_previous = new_tree.clone();
        if let Some(old_tree) = slot.previous.take() {
            crate::lemon_trace!(
                Runtime,
                "render_slot path={} old={} new={}",
                crate::debug::format_path(&slot.path),
                crate::debug::element_kind(&old_tree),
                crate::debug::element_kind(&new_tree)
            );
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

/// Child component slots for `path` live in the parent slot's `children` list.
fn slots_bucket_mut<'a>(
    roots: &'a mut Vec<ComponentSlot>,
    path: &NodePath,
) -> &'a mut Vec<ComponentSlot> {
    if path.0.is_empty() {
        return roots;
    }
    let parent = NodePath(path.0[..path.0.len() - 1].to_vec());
    &mut find_slot_mut(roots, &parent)
        .expect("parent component slot must exist")
        .children
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

fn bootstrap_initial_components(
    slots: &mut Vec<ComponentSlot>,
    element: &Element,
    path: NodePath,
    patches: &mut Vec<Patch>,
    deferred_effects: Rc<RefCell<Vec<Effect>>>,
) {
    match element {
        Element::Component(component) => {
            let bucket = slots_bucket_mut(slots, &path);
            mount_component_slot(
                bucket,
                path.clone(),
                component.clone(),
                Rc::clone(&deferred_effects),
            );
            if let Some(slot) = find_slot_mut(slots, &path) {
                let rendered = slot
                    .previous
                    .clone()
                    .expect("component slot must render on mount");
                patches.push(Patch::ReplaceNode {
                    node: path.clone(),
                    new_element: rendered,
                });
                patches.push(Patch::MountComponent {
                    node: path.clone(),
                    component: component.clone(),
                });
            }
        }
        Element::Column(container) | Element::Row(container) | Element::View(container) => {
            for (index, child) in container.children.iter().enumerate() {
                bootstrap_initial_components(
                    slots,
                    child,
                    path.child(index),
                    patches,
                    Rc::clone(&deferred_effects),
                );
            }
        }
        Element::Fragment(children) => {
            for (index, child) in children.iter().enumerate() {
                bootstrap_initial_components(
                    slots,
                    child,
                    path.child(index),
                    patches,
                    Rc::clone(&deferred_effects),
                );
            }
        }
        _ => {}
    }
}

fn mount_component_slot(
    parent_slots: &mut Vec<ComponentSlot>,
    path: NodePath,
    component: ComponentElement,
    deferred_effects: Rc<RefCell<Vec<Effect>>>,
) {
    crate::lemon_trace!(
        Runtime,
        "mount_component_slot path={} key={:?} identity={}",
        crate::debug::format_path(&path),
        component.key(),
        component.identity()
    );
    // Nested components must run their effect on mount so signal reads register subscribers.
    // A lazy effect never runs until dirty, but it cannot become dirty until it has run once.
    let mut slot = create_component_slot(path, component.view(), false, deferred_effects);
    if let Some(initial) = slot.pending.borrow_mut().take() {
        slot.previous = Some(initial);
    }
    parent_slots.push(slot);
}

fn render_without_subscribing(slot: &ComponentSlot) -> Element {
    crate::runtime::observer::without_observer(|| {
        slot.cx.borrow().reset_hooks();
        freeze_element(&slot.view.borrow()(&slot.cx.borrow()))
    })
}

/// Re-render a slot for a parent-driven update without re-entering the slot's effect.
fn sync_render(slot: &ComponentSlot) {
    *slot.pending.borrow_mut() = Some(render_without_subscribing(slot));
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
    let props_changed = !old.props_eq(&new);
    if same_component_identity(old, &new) {
        if let Some(slot) = find_slot_mut(parent_slots, &path) {
            *slot.view.borrow_mut() = new.view();
            // A signal-driven effect may have already rendered into `pending`. Do not clobber it.
            let had_pending = slot.pending.borrow().is_some();
            if !had_pending {
                sync_render(slot);
            }
            crate::lemon_trace!(
                Runtime,
                "handle_component_pair path={} key={:?} pending_before={had_pending} props_changed={props_changed}",
                crate::debug::format_path(&path),
                new.key()
            );
            render_slot(slot, patches);
        } else {
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
    if crate::diff::children_are_fully_keyed(old_children)
        && crate::diff::children_are_fully_keyed(new_children)
    {
        diff_children_slots_keyed(
            slots,
            old_children,
            new_children,
            parent,
            patches,
            deferred_effects,
        );
    } else {
        diff_children_slots_by_index(
            slots,
            old_children,
            new_children,
            parent,
            patches,
            deferred_effects,
        );
    }
}

fn push_insert_child_slot(
    slots: &mut Vec<ComponentSlot>,
    element: &Element,
    parent: &NodePath,
    index: usize,
    patches: &mut Vec<Patch>,
    deferred_effects: Rc<RefCell<Vec<Effect>>>,
) {
    let path = parent.child(index);
    match element {
        Element::Component(component) => {
            patches.push(Patch::MountComponent {
                node: path.clone(),
                component: component.clone(),
            });
            mount_component_slot(slots, path, component.clone(), Rc::clone(&deferred_effects));
        }
        element => patches.push(Patch::InsertChild {
            parent: parent.clone(),
            index,
            element: element.clone(),
        }),
    }
}

fn push_remove_child_slot(
    slots: &mut Vec<ComponentSlot>,
    element: &Element,
    parent: &NodePath,
    index: usize,
    patches: &mut Vec<Patch>,
) {
    let path = parent.child(index);
    match element {
        Element::Component(_) => {
            unmount_component_slot(slots, &path);
            patches.push(Patch::UnmountComponent { node: path });
        }
        _ => patches.push(Patch::RemoveChild {
            parent: parent.clone(),
            index,
        }),
    }
}

fn diff_children_slots_by_index(
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

    for (index, element) in new_children.iter().enumerate().skip(min) {
        push_insert_child_slot(
            slots,
            element,
            parent,
            index,
            patches,
            Rc::clone(&deferred_effects),
        );
    }

    for i in (min..old_children.len()).rev() {
        push_remove_child_slot(slots, &old_children[i], parent, i, patches);
    }
}

fn diff_children_slots_keyed(
    slots: &mut Vec<ComponentSlot>,
    old_children: &[Element],
    new_children: &[Element],
    parent: &NodePath,
    patches: &mut Vec<Patch>,
    deferred_effects: Rc<RefCell<Vec<Effect>>>,
) {
    use std::collections::{HashMap, HashSet};

    let mut old_by_key: HashMap<crate::element::types::Key, (usize, Element)> = HashMap::new();
    let mut old_order = Vec::with_capacity(old_children.len());
    for (index, element) in old_children.iter().cloned().enumerate() {
        let key =
            crate::diff::element_key(&element).expect("keyed diff requires keys on every child");
        old_by_key.insert(key.clone(), (index, element));
        old_order.push(key);
    }

    let new_items: Vec<(crate::element::types::Key, Element)> = new_children
        .iter()
        .cloned()
        .map(|element| {
            let key = crate::diff::element_key(&element)
                .expect("keyed diff requires keys on every child");
            (key, element)
        })
        .collect();

    let new_order: Vec<_> = new_items.iter().map(|(key, _)| key.clone()).collect();
    let new_key_set: HashSet<_> = new_order.iter().cloned().collect();
    let old_key_set: HashSet<_> = old_by_key.keys().cloned().collect();

    let mut removals: Vec<(usize, Element)> = old_by_key
        .iter()
        .filter(|(key, _)| !new_key_set.contains(*key))
        .map(|(_, (index, element))| (*index, element.clone()))
        .collect();
    removals.sort_by(|(a, _), (b, _)| b.cmp(a));
    for (index, element) in removals {
        push_remove_child_slot(slots, &element, parent, index, patches);
    }

    if old_key_set == new_key_set {
        let mut current_order: Vec<_> = old_order
            .into_iter()
            .filter(|key| new_key_set.contains(key))
            .collect();
        for (new_index, key) in new_order.iter().enumerate() {
            let current_index = current_order
                .iter()
                .position(|current| current == key)
                .expect("key present in both lists");
            if current_index != new_index {
                patches.push(Patch::MoveChild {
                    parent: parent.clone(),
                    from: current_index,
                    to: new_index,
                });
                let moved = current_order.remove(current_index);
                current_order.insert(new_index, moved);
            }
        }
    }

    for (new_index, (key, new_child)) in new_items.iter().enumerate() {
        if !old_key_set.contains(key) {
            push_insert_child_slot(
                slots,
                new_child,
                parent,
                new_index,
                patches,
                Rc::clone(&deferred_effects),
            );
        }
    }

    for (new_index, (key, new_child)) in new_items.iter().enumerate() {
        if let Some((_, old_child)) = old_by_key.get(key) {
            diff_with_nested_components(
                slots,
                old_child.clone(),
                new_child.clone(),
                parent.child(new_index),
                patches,
                Rc::clone(&deferred_effects),
            );
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
                    slots_bucket_mut(slots, node),
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
                        slots_bucket_mut(slots, node),
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
        | (Element::View(old), Element::View(new)) => {
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
        Element::View(container) => Element::View(freeze_box(container)),
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
        handlers: container.handlers.clone(),
        text_input: container.text_input.clone(),
        scroll_viewport: container.scroll_viewport,
        scroll_bar: container.scroll_bar,
    }
}

fn freeze_paint(paint: &PaintProps) -> PaintProps {
    let resolved = paint.resolve();
    PaintProps {
        background: resolved.background.map(ColorSource::Static),
        border_color: resolved.border_color.map(ColorSource::Static),
        border_width: resolved.border_width,
        radius: resolved.radius,
        image: resolved.image,
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
    fn nested_component_effect_subscribes_to_signal_on_mount() {
        use std::cell::RefCell;

        use crate::element::builders::{Column, Component};

        thread_local! {
            static SIGNAL: RefCell<Option<Signal<i32>>> = const { RefCell::new(None) };
        }

        fn child(cx: &Cx) -> Element {
            let n = cx.use_signal(0i32);
            SIGNAL.with(|cell| *cell.borrow_mut() = Some(n.clone()));
            let read = n.clone();
            Text::new(move || read.get().to_string()).into_element()
        }

        let mut runtime = Runtime::new();
        runtime.mount(|_cx| {
            Column::new()
                .child(Component::new(child).key(1))
                .into_element()
        });

        let signal = SIGNAL.with(|cell| cell.borrow_mut().take().expect("signal captured"));
        signal.set(1);

        runtime.flush_effects();
        let patches = runtime.take_patches();

        assert!(
            patches.iter().any(|patch| {
                matches!(
                    patch,
                    Patch::UpdateText { content, .. } if content == "1"
                )
            }),
            "expected nested component to react to signal, got {patches:?}"
        );
    }

    #[test]
    fn flush_effects_processes_nested_component_slot_patches() {
        use crate::element::builders::{Column, Component};

        fn child(_cx: &Cx) -> Element {
            Text::new("0").into_element()
        }

        let mut runtime = Runtime::new();
        runtime.mount(move |_cx| {
            Column::new()
                .child(Component::new(child).key(1))
                .into_element()
        });

        let child_slot =
            find_slot_mut(&mut runtime.slots, &NodePath(vec![0])).expect("nested slot");
        child_slot.previous = Some(Text::new("0").into_element());
        *child_slot.pending.borrow_mut() = Some(Text::new("1").into_element());

        runtime.flush_effects();
        let patches = runtime.take_patches();

        assert!(
            patches.iter().any(|patch| {
                matches!(
                    patch,
                    Patch::UpdateText { content, .. } if content == "1"
                )
            }),
            "expected nested slot diff to emit UpdateText, got {patches:?}"
        );
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
        runtime.take_patches();

        trigger.set(1);
        runtime.flush_effects();
        let patches = runtime.take_patches();

        assert!(patches
            .iter()
            .any(|patch| { matches!(patch, Patch::UpdateText { content, .. } if content == "2") }));
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
                0 | 2 | 3 => counting_child,
                _ => child_b,
            };
            Column::new()
                .child(Component::new(view).key(1))
                .into_element()
        });
        runtime.take_patches();

        phase.set(1);
        runtime.flush_effects();
        runtime.take_patches();

        phase.set(2);
        runtime.flush_effects();
        let remount_patches = runtime.take_patches();
        assert!(
            remount_patches
                .iter()
                .any(|patch| matches!(patch, Patch::UnmountComponent { .. })),
            "expected unmount when swapping back to counting_child, got {remount_patches:?}"
        );
        assert!(
            remount_patches
                .iter()
                .any(|patch| matches!(patch, Patch::MountComponent { .. })),
            "expected remount when swapping back to counting_child, got {remount_patches:?}"
        );

        // Fresh hook state after remount: first parent-driven sync should go 1 -> 2.
        phase.set(3);
        runtime.flush_effects();
        let patches = runtime.take_patches();
        assert!(
            patches.iter().any(|patch| {
                matches!(patch, Patch::UpdateText { content, .. } if content == "2")
            }),
            "expected UpdateText \"2\" after remount, got {patches:?}"
        );
    }

    #[test]
    fn keyed_list_add_remove_patches_apply_without_error() {
        use crate::element::builders::{Button, Column, Row};
        use crate::element::style::{Align, Color};
        use crate::retained::RetainedTree;

        #[derive(Clone)]
        struct Item {
            id: u64,
            label: String,
        }

        let items = Signal::new(Vec::<Item>::new());
        let items_for_view = items.clone();
        let mut rt = Runtime::new();
        rt.mount(move |_cx| {
            let mut col = Column::new().gap(8.0);
            for item in items_for_view.get() {
                let items_remove = items_for_view.clone();
                let item_id = item.id;
                let label = item.label.clone();
                col = col.child(
                    Row::new()
                        .key(item.id)
                        .gap(8.0)
                        .align_items(Align::Center)
                        .child(
                            Text::new(label)
                                .font_size(15.0)
                                .color(Color::rgb8(210, 210, 230)),
                        )
                        .child(
                            Button::new("Remove")
                                .background(Color::rgb8(180, 50, 50))
                                .on_click(move || {
                                    items_remove.update(|list| list.retain(|i| i.id != item_id));
                                }),
                        ),
                );
            }
            Column::new()
                .padding(24.0)
                .gap(12.0)
                .child(Text::new("Keyed List").font_size(20.0))
                .child(Button::new("Add item"))
                .child(col)
                .into_element()
        });
        let mut tree = RetainedTree::mount(rt.root_element().unwrap()).unwrap();

        for step in 0..3u64 {
            items.update(|list| {
                list.push(Item {
                    id: step,
                    label: format!("Item #{step}"),
                });
            });
            rt.flush_effects();
            tree.apply_patches(rt.take_patches())
                .expect("apply_patches after add");
            assert_eq!(
                tree.root.as_ref().unwrap().children[2].children.len(),
                (step + 1) as usize
            );
        }

        items.update(|list| list.retain(|item| item.id != 0));
        rt.flush_effects();
        tree.apply_patches(rt.take_patches())
            .expect("apply_patches after remove");
        assert_eq!(tree.root.as_ref().unwrap().children[2].children.len(), 2);
    }

    #[test]
    fn sibling_component_counters_update_independently() {
        use std::cell::RefCell;

        use crate::element::builders::{Button, Column, Component, Row};
        use crate::retained::RetainedTree;

        thread_local! {
            static FIRST: RefCell<Option<Signal<i32>>> = const { RefCell::new(None) };
            static SECOND: RefCell<Option<Signal<i32>>> = const { RefCell::new(None) };
        }

        fn first(cx: &Cx) -> Element {
            let n = cx.use_signal(0i32);
            FIRST.with(|cell| *cell.borrow_mut() = Some(n.clone()));
            let label = n.clone();
            Row::new()
                .child(Text::new(move || format!("{}", label.get())).font_size(16.0))
                .child(Button::new("+").width(44.0))
                .into_element()
        }

        fn second(cx: &Cx) -> Element {
            let n = cx.use_signal(0i32);
            SECOND.with(|cell| *cell.borrow_mut() = Some(n.clone()));
            let label = n.clone();
            Row::new()
                .child(Text::new(move || format!("{}", label.get())).font_size(16.0))
                .child(Button::new("+").width(44.0))
                .into_element()
        }

        let mut runtime = Runtime::new();
        runtime.mount(|_cx| {
            Column::new()
                .child(Text::new("a"))
                .child(Text::new("b"))
                .child(Text::new("c"))
                .child(Text::new("d"))
                .child(Text::new("e"))
                .child(Component::new(first).key(1))
                .child(Component::new(second).key(2))
                .into_element()
        });

        let element = runtime.root_element().expect("root");
        let mut tree = RetainedTree::mount(element).expect("mount");
        tree.apply_patches(runtime.take_patches())
            .expect("bootstrap patches");

        let first = FIRST.with(|cell| cell.borrow().clone().expect("first signal"));
        let second = SECOND.with(|cell| cell.borrow().clone().expect("second signal"));

        first.set(1);
        runtime.flush_effects();
        let first_patches = runtime.take_patches();
        assert!(
            first_patches.iter().any(|patch| {
                matches!(
                    patch,
                    Patch::UpdateText {
                        node,
                        content,
                    } if node.0 == [5, 0] && content == "1"
                )
            }),
            "first counter click should update text at [5, 0], got {first_patches:?}"
        );
        tree.apply_patches(first_patches)
            .expect("apply first counter patches");

        second.set(1);
        runtime.flush_effects();
        let second_patches = runtime.take_patches();
        assert!(
            second_patches.iter().any(|patch| {
                matches!(
                    patch,
                    Patch::UpdateText {
                        node,
                        content,
                    } if node.0 == [6, 0] && content == "1"
                )
            }),
            "second counter click should update text at [6, 0], got {second_patches:?}"
        );
        tree.apply_patches(second_patches)
            .expect("apply second counter patches");

        let first_text = tree
            .root
            .as_ref()
            .and_then(|root| root.children.get(5))
            .and_then(|node| node.children.first())
            .and_then(|row| row.children.first())
            .and_then(|node| node.text.as_ref())
            .expect("first counter text");
        let second_text = tree
            .root
            .as_ref()
            .and_then(|root| root.children.get(6))
            .and_then(|node| node.children.first())
            .and_then(|row| row.children.first())
            .and_then(|node| node.text.as_ref())
            .expect("second counter text");
        assert_eq!(first_text.content, "1");
        assert_eq!(second_text.content, "1");
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
