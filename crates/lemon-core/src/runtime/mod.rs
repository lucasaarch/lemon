pub mod cx;

use std::cell::RefCell;
use std::rc::Rc;

use crate::diff::{NodePath, Patch};
use crate::element::{
    content::TextContent,
    style::{ColorSource, PaintProps},
    types::{BoxElement, ButtonElement, ComponentElement, TextElement},
    Element,
};
use cx::Cx;
use lemon_reactive::Effect;

type ViewFn = Rc<dyn Fn(&Cx) -> Element>;

struct ComponentSlot {
    path: NodePath,
    previous: Option<Element>,
    pending: Rc<RefCell<Option<Element>>>,
    view: Rc<RefCell<ViewFn>>,
    cx: Rc<RefCell<Cx>>,
    component: Option<ComponentElement>,
    children: Vec<ComponentSlot>,
    deferred_effects: Rc<RefCell<Vec<Effect>>>,
    _effect: Effect,
}

/// Reactive renderer: mounts a root view, diffs trees, and collects [`Patch`]es.
///
/// The desktop host owns a `Runtime` for you. Use this type directly only in tests or
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

    /// Mounts a single root view function, using the same signature as the desktop entry point.
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
                &mut self.slots[0].children,
                &initial,
                NodePath::root(),
                &mut self.patch_queue,
                Rc::clone(&self.deferred_effects),
            );
        }
    }

    /// Re-runs dirty views, diffs against the previous tree, and queues patches.
    ///
    /// Call after changing a [`State`](crate::State) (or other tracked state), then [`take_patches`](Self::take_patches).
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

    /// Run mount-time `Cx::watch` callbacks queued during render (after first paint).
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
        let _scope = cx2.borrow().enter_host();
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
        component: None,
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
            diff_component_output_slots(
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

fn unmount_subtree_slots(slots: &mut Vec<ComponentSlot>, path: &NodePath) {
    slots.retain_mut(|slot| {
        if slot.path.0.starts_with(&path.0) {
            return false;
        }
        unmount_subtree_slots(&mut slot.children, path);
        true
    });
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
            mount_component_slot(
                slots,
                path.clone(),
                component.clone(),
                Rc::clone(&deferred_effects),
            );
            let slot = slots.last_mut().expect("component slot was just mounted");
            let rendered = slot
                .previous
                .clone()
                .expect("component slot must render on mount");
            let rendered_children = component_output_children(rendered.clone());
            if rendered_children.len() == 1 {
                let child = rendered_children.into_iter().next().expect("single child");
                patches.push(Patch::ReplaceNode {
                    node: path.clone(),
                    new_element: child.clone(),
                });
                bootstrap_initial_components(
                    &mut slot.children,
                    &child,
                    path.clone(),
                    patches,
                    Rc::clone(&deferred_effects),
                );
                patches.push(Patch::MountComponent {
                    node: path,
                    component: component.clone(),
                });
            } else {
                // Multi-root or empty output must be mounted through the wrapper first so later
                // child patches operate against the component wrapper instead of replacing it.
                patches.push(Patch::MountComponent {
                    node: path.clone(),
                    component: component.clone(),
                });
                bootstrap_component_output(
                    &mut slot.children,
                    rendered_children,
                    path,
                    patches,
                    Rc::clone(&deferred_effects),
                );
            }
        }
        Element::Column(container) | Element::Row(container) | Element::View(container) => {
            let children = crate::diff::flatten_children(container.children.clone());
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
        Element::Fragment(children) => {
            let children = crate::diff::flatten_children(children.clone());
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

/// Seeds the retained wrapper for a component that renders zero or multiple top-level children.
fn bootstrap_component_output(
    slots: &mut Vec<ComponentSlot>,
    children: Vec<Element>,
    path: NodePath,
    patches: &mut Vec<Patch>,
    deferred_effects: Rc<RefCell<Vec<Effect>>>,
) {
    match children.as_slice() {
        [] => patches.push(Patch::RemoveChild {
            parent: path,
            index: 0,
        }),
        _ => {
            patches.push(Patch::ReplaceNode {
                node: path.child(0),
                new_element: children[0].clone(),
            });
            for (index, child) in children.iter().enumerate().skip(1) {
                patches.push(Patch::InsertChild {
                    parent: path.clone(),
                    index,
                    element: child.clone(),
                });
            }
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
    slot.component = Some(component);
    if let Some(initial) = slot.pending.borrow_mut().take() {
        slot.previous = Some(initial);
    }
    parent_slots.push(slot);
}

fn render_without_subscribing(slot: &ComponentSlot) -> Element {
    let _scope = slot.cx.borrow().enter_host();
    lemon_reactive::observer::without_observer(|| {
        slot.cx.borrow().reset_hooks();
        freeze_element(&slot.view.borrow()(&slot.cx.borrow()))
    })
}

/// Re-render a slot for a parent-driven update without re-entering the slot's effect.
fn sync_render(slot: &ComponentSlot) {
    *slot.pending.borrow_mut() = Some(render_without_subscribing(slot));
}

/// Returns the flattened top-level children produced by a component render.
fn component_output_children(element: Element) -> Vec<Element> {
    crate::diff::flatten_children(vec![element])
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
            let props_changed = slot
                .component
                .as_ref()
                .is_none_or(|previous| !previous.props_eq(&new));
            let should_sync_render = if new.has_props() { props_changed } else { true };
            if should_sync_render {
                *slot.view.borrow_mut() = new.view();
                sync_render(slot);
            }
            slot.component = Some(new.clone());
            crate::lemon_trace!(
                Runtime,
                "handle_component_pair path={} key={:?} props_changed={props_changed}",
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
        unmount_subtree_slots(parent_slots, &path);
        // Replace the entire old wrapper, including every fragment root, before mounting.
        patches.push(Patch::ReplaceNode {
            node: path.clone(),
            new_element: crate::Text::new("").into_element(),
        });
        bootstrap_initial_components(
            parent_slots,
            &Element::Component(new),
            path,
            patches,
            deferred_effects,
        );
    }
}

/// Diffs a component render with fragment-aware flattening while preserving single-root paths.
fn diff_component_output_slots(
    slots: &mut Vec<ComponentSlot>,
    old: Element,
    new: Element,
    path: NodePath,
    patches: &mut Vec<Patch>,
    deferred_effects: Rc<RefCell<Vec<Effect>>>,
) {
    let old_children = component_output_children(old);
    let new_children = component_output_children(new);
    match (old_children.as_slice(), new_children.as_slice()) {
        ([old_child], [new_child]) => diff_with_nested_components(
            slots,
            old_child.clone(),
            new_child.clone(),
            path,
            patches,
            deferred_effects,
        ),
        _ => diff_component_output_children(
            slots,
            &old_children,
            &new_children,
            &path,
            patches,
            deferred_effects,
        ),
    }
}

/// Diffs flattened component output children, keeping the historical single-child path stable.
fn diff_component_output_children(
    slots: &mut Vec<ComponentSlot>,
    old_children: &[Element],
    new_children: &[Element],
    parent: &NodePath,
    patches: &mut Vec<Patch>,
    deferred_effects: Rc<RefCell<Vec<Effect>>>,
) {
    crate::diff::reconcile_children_at(
        &mut SlotReconciler {
            slots,
            effects: deferred_effects,
        },
        old_children.to_vec(),
        new_children.to_vec(),
        parent,
        patches,
        |_, index| {
            if index == 0 && old_children.len() == 1 {
                parent.clone()
            } else {
                parent.child(index)
            }
        },
    );
}

struct SlotReconciler<'a> {
    slots: &'a mut Vec<ComponentSlot>,
    effects: Rc<RefCell<Vec<Effect>>>,
}
impl crate::diff::Reconciler for SlotReconciler<'_> {
    fn insert(
        &mut self,
        element: Element,
        parent: &NodePath,
        index: usize,
        patches: &mut Vec<Patch>,
    ) {
        patches.push(Patch::InsertChild {
            parent: parent.clone(),
            index,
            element: element.clone(),
        });
        bootstrap_initial_components(
            self.slots,
            &element,
            parent.child(index),
            patches,
            self.effects.clone(),
        );
    }
    fn remove(
        &mut self,
        _element: &Element,
        parent: &NodePath,
        index: usize,
        patches: &mut Vec<Patch>,
    ) {
        unmount_subtree_slots(self.slots, &parent.child(index));
        patches.push(Patch::RemoveChild {
            parent: parent.clone(),
            index,
        });
    }
    fn component_transition(
        &mut self,
        old: &Element,
        new: &Element,
        path: &NodePath,
        patches: &mut Vec<Patch>,
    ) -> bool {
        match (old, new) {
            (Element::Component(old), Element::Component(new)) => {
                handle_component_pair(
                    self.slots,
                    old,
                    new.clone(),
                    path.clone(),
                    patches,
                    self.effects.clone(),
                );
                true
            }
            (Element::Component(_), new) => {
                unmount_subtree_slots(self.slots, path);
                patches.push(Patch::ReplaceNode {
                    node: path.clone(),
                    new_element: new.clone(),
                });
                bootstrap_initial_components(
                    self.slots,
                    new,
                    path.clone(),
                    patches,
                    self.effects.clone(),
                );
                true
            }
            (_, Element::Component(_)) => {
                unmount_subtree_slots(self.slots, path);
                patches.push(Patch::ReplaceNode {
                    node: path.clone(),
                    new_element: crate::Text::new("").into_element(),
                });
                bootstrap_initial_components(
                    self.slots,
                    new,
                    path.clone(),
                    patches,
                    self.effects.clone(),
                );
                true
            }
            _ => false,
        }
    }
    fn replaced(&mut self, element: &Element, path: &NodePath, patches: &mut Vec<Patch>) {
        unmount_subtree_slots(self.slots, path);
        bootstrap_initial_components(
            self.slots,
            element,
            path.clone(),
            patches,
            self.effects.clone(),
        );
    }
    fn relocate(&mut self, parent: &NodePath, op: &crate::diff::ChildOp) {
        reindex_slots(self.slots, parent, &|index| op.relocated_index(index));
    }
}

fn reindex_slots(
    slots: &mut Vec<ComponentSlot>,
    parent: &NodePath,
    index: &impl Fn(usize) -> Option<usize>,
) {
    slots.retain_mut(|slot| {
        if slot.path.0.starts_with(&parent.0) && slot.path.0.len() > parent.0.len() {
            let Some(next) = index(slot.path.0[parent.0.len()]) else {
                return false;
            };
            slot.path.0[parent.0.len()] = next;
        }
        reindex_slots(&mut slot.children, parent, index);
        true
    });
}

fn diff_with_nested_components(
    slots: &mut Vec<ComponentSlot>,
    old: Element,
    new: Element,
    path: NodePath,
    patches: &mut Vec<Patch>,
    deferred_effects: Rc<RefCell<Vec<Effect>>>,
) {
    patches.extend(crate::diff::reconcile(
        &mut SlotReconciler {
            slots,
            effects: deferred_effects,
        },
        old,
        new,
        path,
    ));
}

fn freeze_element(element: &Element) -> Element {
    match element {
        Element::Text(text) => {
            // Resolve the theme default while freezing so diffing sees theme changes
            // even when the app did not provide an explicit foreground color.
            let mut style = text.style.clone();
            style.color = Some(crate::element::style::resolved_text_color(&style));
            Element::Text(TextElement {
                content: TextContent::Static(text.content.resolve()),
                style,
                key: text.key.clone(),
            })
        }
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
        background_gradient: resolved.background_gradient,
        border_color: freeze_border_colors(&resolved.border_color),
        border_width: resolved.border_width,
        radius: resolved.radius,
        box_shadow: resolved.box_shadow,
        image: resolved.image,
        scroll_track_color: resolved.scroll_track_color.map(ColorSource::Static),
        scroll_thumb_color: resolved.scroll_thumb_color.map(ColorSource::Static),
        focus_ring_color: resolved.focus_ring_color.map(ColorSource::Static),
    }
}

fn freeze_border_colors(
    colors: &crate::element::style::Edges<Option<crate::element::style::Color>>,
) -> crate::element::style::Edges<Option<ColorSource>> {
    crate::element::style::Edges {
        top: colors.top.map(ColorSource::Static),
        right: colors.right.map(ColorSource::Static),
        bottom: colors.bottom.map(ColorSource::Static),
        left: colors.left.map(ColorSource::Static),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::builders::Text;
    use lemon_reactive::Signal;

    #[test]
    fn no_patches_on_first_mount() {
        let mut rt = Runtime::new();
        rt.mount(|_cx| Text::new("hello").into_element());
        assert!(rt.take_patches().is_empty());
    }

    #[test]
    fn runtime_emits_widget_chrome_when_text_input_meta_changes() {
        use crate::element::builders::{RowView, View};
        use crate::element::types::TextInputMeta;

        let state = Signal::new((String::new(), 0usize));
        let field = state.clone();
        let mut rt = Runtime::new();
        rt.mount(move |_cx| {
            let (value, cursor) = field.get();
            View::new()
                .text_input(TextInputMeta {
                    cursor,
                    value: value.clone(),
                })
                .child(RowView::new().child(Text::new(value)))
                .into_element()
        });

        state.set(("hi".into(), 2));
        rt.flush_effects();
        let patches = rt.take_patches();
        assert!(
            patches.iter().any(|patch| matches!(
                patch,
                Patch::UpdateWidgetChrome {
                    text_input: Some(meta),
                    ..
                } if meta.cursor == 2 && meta.value == "hi"
            )),
            "runtime diff must emit UpdateWidgetChrome for caret metadata, got {patches:?}"
        );
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
            cx.watch(move || {
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
    fn runtime_emits_update_paint_for_gradient_shadow_and_border_colors() {
        use crate::element::builders::View;
        use crate::element::style::{BoxShadow, Color, LinearGradient};

        let inactive_gradient = LinearGradient::new(
            (0.0, 0.0),
            (1.0, 0.0),
            Color::rgb8(10, 20, 30),
            Color::rgb8(40, 50, 60),
        );
        let active_gradient = LinearGradient::new(
            (0.0, 0.0),
            (0.0, 1.0),
            Color::rgb8(60, 50, 40),
            Color::rgb8(30, 20, 10),
        );
        let inactive_shadow = BoxShadow::new(Color::rgb8(0, 0, 0).with_alpha(0.2), 0.0, 2.0, 4.0);
        let active_shadow = BoxShadow::new(Color::rgb8(0, 0, 0).with_alpha(0.3), 1.0, 6.0, 10.0);
        let trigger = Signal::new(false);
        let watched = trigger.clone();
        let mut rt = Runtime::new();
        rt.mount(move |_cx| {
            let active = watched.get();
            let gradient = if active {
                active_gradient
            } else {
                inactive_gradient
            };
            let shadow = if active {
                active_shadow
            } else {
                inactive_shadow
            };
            let mut view = View::new()
                .width(80.0)
                .height(40.0)
                .gradient(gradient)
                .shadow(shadow)
                .border(Color::rgb8(80, 80, 80), 2.0);
            if active {
                view = view.border_color_top(Color::rgb8(255, 0, 0));
            }
            view.into_element()
        });
        rt.take_patches();

        trigger.set(true);
        rt.flush_effects();
        let patches = rt.take_patches();

        assert!(
            patches.iter().any(|patch| matches!(
                patch,
                Patch::UpdatePaint { paint, .. }
                    if paint.background_gradient == Some(active_gradient)
                        && paint.box_shadow == Some(active_shadow)
                        && paint.border_color.top == Some(Color::rgb8(255, 0, 0))
                        && paint.border_color.right == Some(Color::rgb8(80, 80, 80))
            )),
            "expected UpdatePaint with resolved gradient/shadow/border colors, got {patches:?}"
        );
    }

    #[test]
    fn signal_from_use_signal_persists_across_rerenders() {
        let trigger = Signal::new(0u32);
        let t2 = trigger.clone();
        let mut rt = Runtime::new();
        rt.mount(move |_cx| {
            let count = _cx.state(0u32);
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

        use crate::element::builders::{ColumnView, Component};

        thread_local! {
            static SIGNAL: RefCell<Option<Signal<i32>>> = const { RefCell::new(None) };
        }

        fn child(cx: &Cx) -> Element {
            let n = cx.state(0i32);
            SIGNAL.with(|cell| *cell.borrow_mut() = Some(n.clone()));
            let read = n.clone();
            Text::new(move || read.get().to_string()).into_element()
        }

        let mut runtime = Runtime::new();
        runtime.mount(|_cx| {
            ColumnView::new()
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
        use crate::element::builders::{ColumnView, Component};

        fn child(_cx: &Cx) -> Element {
            Text::new("0").into_element()
        }

        let mut runtime = Runtime::new();
        runtime.mount(move |_cx| {
            ColumnView::new()
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
        use crate::element::builders::{ColumnView, Component};

        fn child(cx: &Cx) -> Element {
            let count = cx.state(0u32);
            count.update(|value| *value += 1);
            Text::new(move || count.get().to_string()).into_element()
        }

        let trigger = Signal::new(0u32);
        let t2 = trigger.clone();
        let mut runtime = Runtime::new();
        runtime.mount(move |_cx| {
            t2.get();
            ColumnView::new()
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
    fn typed_props_component_skips_rerender_when_props_unchanged() {
        use std::cell::Cell;

        use crate::element::builders::{ColumnView, Component};

        #[derive(Clone, PartialEq)]
        struct Props {
            value: u32,
        }

        thread_local! {
            static RENDERS: Cell<u32> = const { Cell::new(0) };
        }

        fn child(_cx: &Cx, props: &Props) -> Element {
            RENDERS.with(|renders| renders.set(renders.get() + 1));
            Text::new(props.value.to_string()).into_element()
        }

        RENDERS.with(|renders| renders.set(0));

        let trigger = Signal::new(0u32);
        let t2 = trigger.clone();
        let mut runtime = Runtime::new();
        runtime.mount(move |_cx| {
            t2.get();
            ColumnView::new()
                .child(Component::new_with_props(child, Props { value: 42 }))
                .into_element()
        });
        runtime.take_patches();

        let renders_after_mount = RENDERS.with(|renders| renders.get());
        assert_eq!(
            renders_after_mount, 1,
            "child should render exactly once on mount"
        );

        trigger.set(1);
        runtime.flush_effects();
        let patches = runtime.take_patches();
        let renders_after_rerender = RENDERS.with(|renders| renders.get());

        assert_eq!(
            renders_after_rerender, renders_after_mount,
            "child should not re-render when typed props are unchanged"
        );
        assert!(
            patches
                .iter()
                .any(|patch| matches!(patch, Patch::UpdateComponent { node, .. } if node.0 == [0])),
            "parent should still reconcile component node, got {patches:?}"
        );
    }

    #[test]
    fn unmounting_component_slot_drops_cx_hooks() {
        use crate::element::builders::{ColumnView, Component};

        fn counting_child(cx: &Cx) -> Element {
            let count = cx.state(0u32);
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
            ColumnView::new()
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
                .any(|patch| matches!(patch, Patch::ReplaceNode { new_element: Element::Text(text), .. } if text.content.resolve() == "1")),
            "expected fresh hook state when swapping back to counting_child, got {remount_patches:?}"
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
        use crate::element::builders::{Button, ColumnView, RowView};
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
            let mut col = ColumnView::new().spacing(8.0);
            for item in items_for_view.get() {
                let items_remove = items_for_view.clone();
                let item_id = item.id;
                let label = item.label.clone();
                col = col.child(
                    RowView::new()
                        .key(item.id)
                        .spacing(8.0)
                        .align(Align::Center)
                        .child(
                            Text::new(label)
                                .size(15.0)
                                .color(Color::rgb8(210, 210, 230)),
                        )
                        .child(
                            Button::new("Remove")
                                .surface(Color::rgb8(180, 50, 50))
                                .action(move || {
                                    items_remove.update(|list| list.retain(|i| i.id != item_id));
                                }),
                        ),
                );
            }
            ColumnView::new()
                .inset(24.0)
                .spacing(12.0)
                .child(Text::new("Keyed List").size(20.0))
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
    fn component_fragment_output_mounts_and_updates_in_retained_tree() {
        use std::cell::RefCell;

        use crate::element::builders::{ColumnView, Component};
        use crate::retained::RetainedTree;

        thread_local! {
            static LABELS: RefCell<Option<Signal<Vec<String>>>> = const { RefCell::new(None) };
        }

        fn list(_cx: &Cx) -> Element {
            let labels = LABELS.with(|cell| cell.borrow().clone().expect("labels signal"));
            Element::Fragment(
                labels
                    .get()
                    .into_iter()
                    .map(|label| Text::new(label).into_element())
                    .collect(),
            )
        }

        let labels = Signal::new(vec!["a".to_owned(), "b".to_owned()]);
        LABELS.with(|cell| *cell.borrow_mut() = Some(labels.clone()));

        let mut runtime = Runtime::new();
        runtime.mount(|_cx| {
            ColumnView::new()
                .child(Component::new(list).key(1))
                .into_element()
        });

        let mut tree = RetainedTree::mount(runtime.root_element().expect("root")).expect("mount");
        tree.apply_patches(runtime.take_patches())
            .expect("bootstrap fragment component");

        let wrapper = &tree.root.as_ref().expect("root").children[0];
        assert_eq!(wrapper.children.len(), 2);
        assert_eq!(wrapper.children[0].text_content(), Some("a"));
        assert_eq!(wrapper.children[1].text_content(), Some("b"));

        labels.set(vec!["a".to_owned(), "c".to_owned(), "d".to_owned()]);
        runtime.flush_effects();
        tree.apply_patches(runtime.take_patches())
            .expect("update fragment component");

        let wrapper = &tree.root.as_ref().expect("root").children[0];
        assert_eq!(wrapper.children.len(), 3);
        assert_eq!(wrapper.children[0].text_content(), Some("a"));
        assert_eq!(wrapper.children[1].text_content(), Some("c"));
        assert_eq!(wrapper.children[2].text_content(), Some("d"));
    }

    #[test]
    fn sibling_component_counters_update_independently() {
        use std::cell::RefCell;

        use crate::element::builders::{Button, ColumnView, Component, RowView};
        use crate::retained::RetainedTree;

        thread_local! {
            static FIRST: RefCell<Option<Signal<i32>>> = const { RefCell::new(None) };
            static SECOND: RefCell<Option<Signal<i32>>> = const { RefCell::new(None) };
        }

        fn first(cx: &Cx) -> Element {
            let n = cx.state(0i32);
            FIRST.with(|cell| *cell.borrow_mut() = Some(n.clone()));
            let label = n.clone();
            RowView::new()
                .child(Text::new(move || format!("{}", label.get())).size(16.0))
                .child(Button::new("+").width(44.0))
                .into_element()
        }

        fn second(cx: &Cx) -> Element {
            let n = cx.state(0i32);
            SECOND.with(|cell| *cell.borrow_mut() = Some(n.clone()));
            let label = n.clone();
            RowView::new()
                .child(Text::new(move || format!("{}", label.get())).size(16.0))
                .child(Button::new("+").width(44.0))
                .into_element()
        }

        let mut runtime = Runtime::new();
        runtime.mount(|_cx| {
            ColumnView::new()
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
    fn nested_component_identity_change_replaces_output_and_mounts() {
        use crate::element::builders::{ColumnView, Component};

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
            ColumnView::new()
                .child(Component::new(child).key(1))
                .into_element()
        });

        let mut tree = crate::RetainedTree::mount(runtime.root_element().unwrap()).unwrap();
        tree.apply_patches(runtime.take_patches()).unwrap();
        swap.set(true);
        runtime.flush_effects();
        let patches = runtime.take_patches();
        assert!(patches
            .iter()
            .any(|patch| matches!(patch, Patch::MountComponent { .. })));
        tree.apply_patches(patches).unwrap();
        let child = &tree.root().unwrap().children[0];
        assert!(matches!(
            child.kind,
            crate::retained::RetainedKind::Component { .. }
        ));
        assert_eq!(child.children[0].text_content(), Some("b"));
    }
}
