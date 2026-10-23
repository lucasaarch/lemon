mod children;
mod props;

pub(crate) use children::{child_plan, ChildOp};
pub(crate) use props::{diff_container_chrome, diff_container_props};

use crate::element::{
    style::{PaintData, StyleProps, TextStyle},
    Element,
};

#[derive(Clone, Debug, PartialEq)]
pub struct NodePath(pub Vec<usize>);

impl NodePath {
    pub fn root() -> Self {
        NodePath(vec![])
    }

    pub fn child(&self, index: usize) -> Self {
        let mut v = self.0.clone();
        v.push(index);
        NodePath(v)
    }
}

#[derive(Debug)]
pub enum Patch {
    UpdateComponent {
        node: NodePath,
        component: crate::element::types::ComponentElement,
    },
    MountComponent {
        node: NodePath,
        component: crate::element::types::ComponentElement,
    },
    UnmountComponent {
        node: NodePath,
    },
    UpdateStyle {
        node: NodePath,
        style: StyleProps,
    },
    /// Replaces input callbacks without changing geometry or visible styling.
    UpdateHandlers {
        /// Path of the node whose callbacks changed.
        node: NodePath,
        /// Complete new set of callbacks, including removals.
        handlers: crate::retained::EventHandlers,
    },
    UpdatePaint {
        node: NodePath,
        paint: PaintData,
    },
    UpdateText {
        node: NodePath,
        content: String,
    },
    /// Updates retained text styling when content is unchanged but metrics or paint attributes change.
    UpdateTextStyle {
        node: NodePath,
        style: TextStyle,
    },
    UpdateWidgetChrome {
        node: NodePath,
        text_input: Option<crate::element::types::TextInputMeta>,
        scroll_viewport: bool,
    },
    ReplaceNode {
        node: NodePath,
        new_element: Element,
    },
    InsertChild {
        parent: NodePath,
        index: usize,
        element: Element,
    },
    RemoveChild {
        parent: NodePath,
        index: usize,
    },
    MoveChild {
        parent: NodePath,
        from: usize,
        to: usize,
    },
}

impl Patch {
    /// Returns a conservative layout requirement without access to the previous tree.
    ///
    /// Paint data, component metadata, and widget chrome only require repainting.
    /// Style and text-style updates are refined against the old geometry by the retained
    /// tree. Hosts should read `RetainedTree::layout_dirty()` after applying a batch.
    pub fn invalidates_layout(&self) -> bool {
        match self {
            Self::UpdateHandlers { .. }
            | Self::UpdatePaint { .. }
            | Self::UpdateWidgetChrome { .. }
            | Self::UpdateComponent { .. } => false,
            Self::MountComponent { .. }
            | Self::UnmountComponent { .. }
            | Self::UpdateStyle { .. }
            | Self::UpdateText { .. }
            | Self::UpdateTextStyle { .. }
            | Self::ReplaceNode { .. }
            | Self::InsertChild { .. }
            | Self::RemoveChild { .. }
            | Self::MoveChild { .. } => true,
        }
    }
}

pub(crate) fn flatten_children(elements: Vec<Element>) -> Vec<Element> {
    let mut flattened = Vec::with_capacity(elements.len());
    for element in elements {
        match element {
            Element::Fragment(children) => flattened.extend(flatten_children(children)),
            Element::None => {}
            other => flattened.push(other),
        }
    }
    flattened
}

/// Compares element trees and emits ordered retained-tree mutations below `path`.
///
/// Fully keyed siblings preserve identity across insertions, removals, and moves.
/// Mixed keyed/unkeyed lists use positional reconciliation.
///
/// # Panics
///
/// Panics if either fully keyed sibling list contains duplicate keys.
pub fn diff(old: Element, new: Element, path: NodePath) -> Vec<Patch> {
    reconcile(&mut PlainReconciler, old, new, path)
}

/// Lifecycle work is supplied by runtime; traversal and patch ordering live here.
pub(crate) trait Reconciler {
    fn insert(
        &mut self,
        element: Element,
        parent: &NodePath,
        index: usize,
        patches: &mut Vec<Patch>,
    ) {
        push_insert_child(element, parent, index, patches);
    }
    fn remove(
        &mut self,
        element: &Element,
        parent: &NodePath,
        index: usize,
        patches: &mut Vec<Patch>,
    ) {
        push_remove_child(element, parent, index, patches);
    }
    fn component_transition(
        &mut self,
        _old: &Element,
        _new: &Element,
        _path: &NodePath,
        _patches: &mut Vec<Patch>,
    ) -> bool {
        false
    }
    fn replaced(&mut self, _element: &Element, _path: &NodePath, _patches: &mut Vec<Patch>) {}
    fn relocate(&mut self, _parent: &NodePath, _op: &ChildOp) {}
}
struct PlainReconciler;
impl Reconciler for PlainReconciler {}

pub(crate) fn reconcile(
    context: &mut impl Reconciler,
    old: Element,
    new: Element,
    path: NodePath,
) -> Vec<Patch> {
    use Element::*;
    let mut patches = Vec::new();

    if context.component_transition(&old, &new, &path, &mut patches) {
        return patches;
    }
    match (old, new) {
        (Text(o), Text(n)) => {
            let os = o.content.resolve();
            let ns = n.content.resolve();
            if os != ns {
                patches.push(Patch::UpdateText {
                    node: path.clone(),
                    content: ns,
                });
            }
            if o.style != n.style {
                patches.push(Patch::UpdateTextStyle {
                    node: path,
                    style: n.style,
                });
            }
        }
        (Column(o), Column(n)) | (Row(o), Row(n)) | (View(o), View(n)) => {
            diff_box(context, o, n, path, &mut patches);
            return patches;
        }
        (Fragment(old_children), Fragment(new_children)) => {
            reconcile_children(
                context,
                flatten_children(old_children),
                flatten_children(new_children),
                &path,
                &mut patches,
            );
            return patches;
        }
        (Fragment(old_children), new) => {
            reconcile_children(
                context,
                flatten_children(old_children),
                flatten_children(vec![new]),
                &path,
                &mut patches,
            );
            return patches;
        }
        (old, Fragment(new_children)) => {
            reconcile_children(
                context,
                flatten_children(vec![old]),
                flatten_children(new_children),
                &path,
                &mut patches,
            );
            return patches;
        }
        (Component(o), Component(n)) => {
            if o.identity() == n.identity() && o.key() == n.key() {
                patches.push(Patch::UpdateComponent {
                    node: path,
                    component: n,
                });
            } else {
                patches.push(Patch::UnmountComponent { node: path.clone() });
                patches.push(Patch::MountComponent {
                    node: path,
                    component: n,
                });
            }
        }
        (Component(_), new) => {
            patches.push(Patch::UnmountComponent { node: path.clone() });
            patches.push(Patch::ReplaceNode {
                node: path,
                new_element: new,
            });
        }
        (_, Component(n)) => {
            patches.push(Patch::MountComponent {
                node: path.clone(),
                component: n,
            });
            patches.push(Patch::ReplaceNode {
                node: path,
                new_element: Element::None,
            });
        }
        (None, None) => {}
        // Different types — replace entirely
        (_, new) => {
            patches.push(Patch::ReplaceNode {
                node: path.clone(),
                new_element: new.clone(),
            });
            context.replaced(&new, &path, &mut patches);
        }
    }

    patches
}

fn diff_box(
    context: &mut impl Reconciler,
    mut o: crate::element::types::BoxElement,
    mut n: crate::element::types::BoxElement,
    path: NodePath,
    patches: &mut Vec<Patch>,
) {
    diff_container_props(&o, &n, &path, patches);
    reconcile_children(
        context,
        std::mem::take(&mut o.children),
        std::mem::take(&mut n.children),
        &path,
        patches,
    );
    diff_container_chrome(&o, &n, &path, patches);
}

pub(crate) fn element_key(element: &Element) -> Option<crate::element::types::Key> {
    use Element::*;
    match element {
        Text(text) => text.key.clone(),
        Column(container) | Row(container) | View(container) => container.key.clone(),
        Component(component) => component.key().cloned(),
        Fragment(_) => Option::None,
        Element::None => Option::None,
    }
}

pub(crate) fn children_are_fully_keyed(children: &[Element]) -> bool {
    !children.is_empty() && children.iter().all(|child| element_key(child).is_some())
}

pub(crate) fn reconcile_children(
    context: &mut impl Reconciler,
    old: Vec<Element>,
    new: Vec<Element>,
    parent: &NodePath,
    patches: &mut Vec<Patch>,
) {
    reconcile_children_at(context, old, new, parent, patches, |_, index| {
        parent.child(index)
    });
}

/// Component output keeps a historical transparent path for a single root.
/// Planning and traversal stay shared; only the output boundary maps update paths.
pub(crate) fn reconcile_children_at(
    context: &mut impl Reconciler,
    old: Vec<Element>,
    new: Vec<Element>,
    parent: &NodePath,
    patches: &mut Vec<Patch>,
    update_path: impl Fn(usize, usize) -> NodePath,
) {
    let old = flatten_children(old);
    let new = flatten_children(new);
    for op in child_plan(&old, &new) {
        let mut emitted = Vec::new();
        match op {
            ChildOp::Update {
                old: previous,
                new: index,
            } => {
                patches.extend(reconcile(
                    context,
                    old[previous].clone(),
                    new[index].clone(),
                    update_path(previous, index),
                ));
                continue;
            }
            ChildOp::Remove(index) => {
                context.remove(&old[index], parent, index, &mut emitted);
                context.relocate(parent, &op);
            }
            ChildOp::Insert(index) => {
                context.relocate(parent, &op);
                context.insert(new[index].clone(), parent, index, &mut emitted);
            }
            ChildOp::Move { from, to } => {
                context.relocate(parent, &op);
                emitted.push(Patch::MoveChild {
                    parent: parent.clone(),
                    from,
                    to,
                });
            }
        }
        patches.extend(emitted);
    }
}

fn push_insert_child(element: Element, parent: &NodePath, index: usize, patches: &mut Vec<Patch>) {
    match element {
        Element::Fragment(children) => {
            for (offset, child) in flatten_children(children).into_iter().enumerate() {
                push_insert_child(child, parent, index + offset, patches);
            }
        }
        Element::Component(component) => patches.push(Patch::MountComponent {
            node: parent.child(index),
            component,
        }),
        Element::None => {}
        element => patches.push(Patch::InsertChild {
            parent: parent.clone(),
            index,
            element,
        }),
    }
}

fn push_remove_child(element: &Element, parent: &NodePath, index: usize, patches: &mut Vec<Patch>) {
    match element {
        Element::Fragment(children) => {
            let flattened = flatten_children(children.clone());
            for (offset, child) in flattened.iter().enumerate().rev() {
                push_remove_child(child, parent, index + offset, patches);
            }
        }
        Element::Component(_) => patches.push(Patch::UnmountComponent {
            node: parent.child(index),
        }),
        Element::None => {}
        _ => patches.push(Patch::RemoveChild {
            parent: parent.clone(),
            index,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::builders::{ColumnView, Component, Text};

    fn txt(s: &'static str) -> Element {
        Text::new(s).into_element()
    }

    #[test]
    fn identical_text_produces_no_patches() {
        assert!(diff(txt("hi"), txt("hi"), NodePath::root()).is_empty());
    }

    #[test]
    fn changed_text_produces_update_text() {
        let patches = diff(txt("old"), txt("new"), NodePath::root());
        assert_eq!(patches.len(), 1);
        assert!(matches!(&patches[0], Patch::UpdateText { content, .. } if content == "new"));
    }

    #[test]
    fn changed_text_style_produces_patch() {
        let old = Text::new("same").size(16.0).into_element();
        let new = Text::new("same").size(24.0).into_element();

        let patches = diff(old, new, NodePath::root());

        assert_eq!(patches.len(), 1);
        assert!(matches!(
            &patches[0],
            Patch::UpdateTextStyle { style, .. } if style.font_size == 24.0
        ));
    }

    #[test]
    fn different_types_produces_replace() {
        let patches = diff(
            Text::new("x").into_element(),
            crate::View::new()
                .child(crate::Text::new("x"))
                .into_element(),
            NodePath::root(),
        );
        assert_eq!(patches.len(), 1);
        assert!(matches!(patches[0], Patch::ReplaceNode { .. }));
    }

    #[test]
    fn text_input_typing_emits_text_patch_before_widget_chrome() {
        use crate::element::builders::{RowView, View};
        use crate::element::types::TextInputMeta;

        let old = View::new()
            .text_input(TextInputMeta {
                selection: None,
                alignment: Default::default(),
                cursor: 0,
                value: String::new(),
            })
            .child(RowView::new().child(Text::new("")))
            .into_element();
        let new = View::new()
            .text_input(TextInputMeta {
                selection: None,
                alignment: Default::default(),
                cursor: 1,
                value: "a".into(),
            })
            .child(RowView::new().child(Text::new("a")))
            .into_element();

        let patches = diff(old, new, NodePath::root());
        let text_idx = patches
            .iter()
            .position(|p| matches!(p, Patch::UpdateText { .. }));
        let chrome_idx = patches
            .iter()
            .position(|p| matches!(p, Patch::UpdateWidgetChrome { .. }));
        assert!(
            text_idx < chrome_idx,
            "UpdateText should run before UpdateWidgetChrome, got {patches:?}"
        );
    }

    #[test]
    fn text_input_cursor_change_emits_update_widget_chrome() {
        use crate::element::builders::View;
        use crate::element::types::TextInputMeta;

        let old = View::new()
            .text_input(TextInputMeta {
                selection: None,
                alignment: Default::default(),
                cursor: 0,
                value: "hi".into(),
            })
            .into_element();
        let new = View::new()
            .text_input(TextInputMeta {
                selection: None,
                alignment: Default::default(),
                cursor: 2,
                value: "hi".into(),
            })
            .into_element();

        let patches = diff(old, new, NodePath::root());
        assert!(
            patches.iter().any(|patch| matches!(
                patch,
                Patch::UpdateWidgetChrome {
                    text_input: Some(meta),
                    ..
                } if meta.cursor == 2
            )),
            "expected UpdateWidgetChrome for cursor move, got {patches:?}"
        );
    }

    #[test]
    fn style_change_produces_update_style() {
        let old = ColumnView::new().spacing(4.0).into_element();
        let new = ColumnView::new().spacing(8.0).into_element();
        let patches = diff(old, new, NodePath::root());
        assert_eq!(patches.len(), 1);
        assert!(matches!(patches[0], Patch::UpdateStyle { .. }));
    }

    #[test]
    fn child_added_produces_insert() {
        let old = ColumnView::new().into_element();
        let new = ColumnView::new().child(txt("a")).into_element();
        let patches = diff(old, new, NodePath::root());
        assert_eq!(patches.len(), 1);
        assert!(matches!(patches[0], Patch::InsertChild { index: 0, .. }));
    }

    #[test]
    fn child_removed_produces_remove() {
        let old = ColumnView::new().child(txt("a")).into_element();
        let new = ColumnView::new().into_element();
        let patches = diff(old, new, NodePath::root());
        assert_eq!(patches.len(), 1);
        assert!(matches!(patches[0], Patch::RemoveChild { index: 0, .. }));
    }

    #[test]
    fn view_style_change_produces_update_style() {
        let old = crate::View::new().child(Text::new("ok")).into_element();
        let new = crate::View::new()
            .width(120.0)
            .child(Text::new("ok"))
            .into_element();
        let patches = diff(old, new, NodePath::root());
        assert!(patches
            .iter()
            .any(|p| matches!(p, Patch::UpdateStyle { .. })));
    }

    fn image_handle(red: u8) -> crate::ImageHandle {
        crate::ImageHandle::from_arc(std::sync::Arc::new(crate::asset::image_handle::ImageData {
            width: 1,
            height: 1,
            pixels: vec![red, 0, 0, 255],
        }))
    }

    #[test]
    fn image_style_change_produces_update_style() {
        let handle = image_handle(1);
        let old = crate::View::new().image(handle.clone()).into_element();
        let new = crate::View::new().image(handle).height(64.0).into_element();
        let patches = diff(old, new, NodePath::root());
        assert!(patches
            .iter()
            .any(|p| matches!(p, Patch::UpdateStyle { .. })));
    }

    #[test]
    fn image_handle_change_updates_paint_without_replacing_the_node() {
        let old = crate::View::new().image(image_handle(1)).into_element();
        let new = crate::View::new().image(image_handle(2)).into_element();
        let patches = diff(old, new, NodePath::root());
        assert!(matches!(patches.as_slice(), [Patch::UpdatePaint { .. }]));
    }

    #[test]
    fn unchanged_children_produce_no_patches() {
        let old = ColumnView::new()
            .child(txt("a"))
            .child(txt("b"))
            .into_element();
        let new = ColumnView::new()
            .child(txt("a"))
            .child(txt("b"))
            .into_element();
        assert!(diff(old, new, NodePath::root()).is_empty());
    }

    #[test]
    fn equal_component_identity_emits_update_without_lifecycle_patches() {
        fn child(_cx: &crate::runtime::cx::Cx) -> Element {
            Text::new("child").into_element()
        }

        let old = Component::new(child).key(1).into_element();
        let new = Component::new(child).key(1).into_element();
        let patches = diff(old, new, NodePath::root());

        assert!(matches!(
            patches.first(),
            Some(Patch::UpdateComponent { .. })
        ));
        assert_eq!(patches.len(), 1);
        assert!(!patches.iter().any(|patch| matches!(
            patch,
            Patch::MountComponent { .. } | Patch::UnmountComponent { .. }
        )));
    }

    #[test]
    fn different_component_functions_with_same_function_pointer_type_remount() {
        fn first(_cx: &crate::runtime::cx::Cx) -> Element {
            Text::new("first").into_element()
        }

        fn second(_cx: &crate::runtime::cx::Cx) -> Element {
            Text::new("second").into_element()
        }

        let old = Component::new(first).key(1).into_element();
        let new = Component::new(second).key(1).into_element();

        let patches = diff(old, new, NodePath::root());

        assert!(matches!(
            patches.first(),
            Some(Patch::UnmountComponent { .. })
        ));
        assert!(matches!(patches.get(1), Some(Patch::MountComponent { .. })));
    }

    #[test]
    fn changed_component_identity_unmounts_then_mounts() {
        fn child_a(_cx: &crate::runtime::cx::Cx) -> Element {
            Text::new("a").into_element()
        }

        fn child_b(_cx: &crate::runtime::cx::Cx) -> Element {
            Text::new("b").into_element()
        }

        let old = Component::new(child_a).key(1).into_element();
        let new = Component::new(child_b).key(1).into_element();

        let patches = diff(old, new, NodePath::root());

        assert!(matches!(
            patches.first(),
            Some(Patch::UnmountComponent { .. })
        ));
        assert!(matches!(patches.get(1), Some(Patch::MountComponent { .. })));
    }

    #[test]
    fn component_to_non_component_unmounts_then_replaces() {
        fn child(_cx: &crate::runtime::cx::Cx) -> Element {
            Text::new("child").into_element()
        }

        let old = Component::new(child).key(1).into_element();
        let new = Text::new("next").into_element();

        let patches = diff(old, new, NodePath::root());

        assert!(matches!(
            patches.first(),
            Some(Patch::UnmountComponent { .. })
        ));
        assert!(matches!(
            patches.get(1),
            Some(Patch::ReplaceNode {
                new_element: Element::Text(_),
                ..
            })
        ));
    }

    #[test]
    fn non_component_to_component_replaces_then_mounts() {
        fn child(_cx: &crate::runtime::cx::Cx) -> Element {
            Text::new("child").into_element()
        }

        let old = Text::new("prev").into_element();
        let new = Component::new(child).key(1).into_element();

        let patches = diff(old, new, NodePath::root());

        assert!(matches!(
            patches.first(),
            Some(Patch::MountComponent { .. })
        ));
        assert!(matches!(
            patches.get(1),
            Some(Patch::ReplaceNode {
                new_element: Element::None,
                ..
            })
        ));
    }

    #[test]
    fn component_child_added_mounts_component_instead_of_inserting_element() {
        fn child(_cx: &crate::runtime::cx::Cx) -> Element {
            Text::new("child").into_element()
        }

        let old = ColumnView::new().into_element();
        let new = ColumnView::new()
            .child(Component::new(child).key(2))
            .into_element();

        let patches = diff(old, new, NodePath::root());

        assert!(matches!(
            patches.first(),
            Some(Patch::MountComponent { node, .. }) if *node == NodePath(vec![0])
        ));
        assert!(!patches.iter().any(|patch| matches!(
            patch,
            Patch::InsertChild {
                element: Element::Component(_),
                ..
            }
        )));
    }

    #[test]
    fn keyed_reorder_emits_move_child_without_remove_or_insert() {
        let old = ColumnView::new()
            .child(Text::new("a").key(1))
            .child(Text::new("b").key(2))
            .into_element();
        let new = ColumnView::new()
            .child(Text::new("b").key(2))
            .child(Text::new("a").key(1))
            .into_element();

        let patches = diff(old, new, NodePath::root());

        assert!(patches
            .iter()
            .any(|patch| { matches!(patch, Patch::MoveChild { from: 1, to: 0, .. }) }));
        assert!(!patches.iter().any(|patch| {
            matches!(patch, Patch::RemoveChild { .. } | Patch::InsertChild { .. })
        }));
    }

    #[test]
    fn keyed_insert_emits_insert_child_at_target_index() {
        let old = ColumnView::new()
            .child(Text::new("a").key(1))
            .into_element();
        let new = ColumnView::new()
            .child(Text::new("a").key(1))
            .child(Text::new("b").key(2))
            .into_element();

        let patches = diff(old, new, NodePath::root());

        assert!(matches!(
            patches.first(),
            Some(Patch::InsertChild { index: 1, .. })
        ));
    }

    #[test]
    fn keyed_remove_emits_remove_child_in_reverse_index_order() {
        let old = ColumnView::new()
            .child(Text::new("a").key(1))
            .child(Text::new("b").key(2))
            .into_element();
        let new = ColumnView::new()
            .child(Text::new("a").key(1))
            .into_element();

        let patches = diff(old, new, NodePath::root());

        assert!(matches!(
            patches.first(),
            Some(Patch::RemoveChild { index: 1, .. })
        ));
    }

    #[test]
    fn fragment_children_are_flattened_before_diffing() {
        let old = ColumnView::new()
            .child(Element::Fragment(vec![txt("a"), txt("b")]))
            .into_element();
        let new = ColumnView::new()
            .child(Element::Fragment(vec![txt("a"), txt("c"), txt("d")]))
            .into_element();

        let patches = diff(old, new, NodePath::root());

        assert!(patches.iter().any(|patch| matches!(
            patch,
            Patch::UpdateText { node, content } if node.0 == [1] && content == "c"
        )));
        assert!(patches
            .iter()
            .any(|patch| matches!(patch, Patch::InsertChild { index: 2, .. })));
    }

    #[test]
    fn keyed_key_change_removes_old_and_inserts_new() {
        let old = ColumnView::new()
            .child(Text::new("a").key(1))
            .into_element();
        let new = ColumnView::new()
            .child(Text::new("b").key(2))
            .into_element();

        let patches = diff(old, new, NodePath::root());

        assert!(matches!(
            patches.first(),
            Some(Patch::RemoveChild { index: 0, .. })
        ));
        assert!(matches!(
            patches.get(1),
            Some(Patch::InsertChild { index: 0, .. })
        ));
        assert!(!patches
            .iter()
            .any(|patch| matches!(patch, Patch::UpdateText { .. })));
    }

    #[test]
    fn keyed_component_insert_mounts_component() {
        fn child(_cx: &crate::runtime::cx::Cx) -> Element {
            Text::new("child").into_element()
        }

        let old = ColumnView::new()
            .child(Text::new("a").key(1))
            .into_element();
        let new = ColumnView::new()
            .child(Text::new("a").key(1))
            .child(Component::new(child).key(2))
            .into_element();

        let patches = diff(old, new, NodePath::root());

        assert!(matches!(
            patches.first(),
            Some(Patch::MountComponent { node, .. }) if *node == NodePath(vec![1])
        ));
    }

    #[test]
    fn keyed_component_remove_unmounts_component() {
        fn child(_cx: &crate::runtime::cx::Cx) -> Element {
            Text::new("child").into_element()
        }

        let old = ColumnView::new()
            .child(Text::new("a").key(1))
            .child(Component::new(child).key(2))
            .into_element();
        let new = ColumnView::new()
            .child(Text::new("a").key(1))
            .into_element();

        let patches = diff(old, new, NodePath::root());

        assert!(matches!(
            patches.first(),
            Some(Patch::UnmountComponent { node }) if *node == NodePath(vec![1])
        ));
    }

    #[test]
    fn mixed_keyed_and_unkeyed_children_fall_back_to_index_diff() {
        let old = ColumnView::new()
            .child(Text::new("a").key(1))
            .child(Text::new("b"))
            .into_element();
        let new = ColumnView::new()
            .child(Text::new("b"))
            .child(Text::new("a").key(1))
            .into_element();

        let patches = diff(old, new, NodePath::root());

        assert!(!patches
            .iter()
            .any(|patch| matches!(patch, Patch::MoveChild { .. })));
        assert!(patches
            .iter()
            .any(|patch| matches!(patch, Patch::UpdateText { .. })));
    }

    #[test]
    fn component_child_removed_unmounts_component_instead_of_generic_remove() {
        fn child(_cx: &crate::runtime::cx::Cx) -> Element {
            Text::new("child").into_element()
        }

        let old = ColumnView::new()
            .child(Component::new(child).key(2))
            .into_element();
        let new = ColumnView::new().into_element();

        let patches = diff(old, new, NodePath::root());

        assert!(matches!(
            patches.first(),
            Some(Patch::UnmountComponent { node }) if *node == NodePath(vec![0])
        ));
        assert!(!patches
            .iter()
            .any(|patch| matches!(patch, Patch::RemoveChild { parent, index } if *parent == NodePath::root() && *index == 0)));
    }
}
