use crate::element::{
    style::{PaintData, StyleProps},
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
    UpdatePaint {
        node: NodePath,
        paint: PaintData,
    },
    UpdateText {
        node: NodePath,
        content: String,
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

pub fn diff(old: Element, new: Element, path: NodePath) -> Vec<Patch> {
    use Element::*;
    let mut patches = Vec::new();

    match (old, new) {
        (Text(o), Text(n)) => {
            let os = o.content.resolve();
            let ns = n.content.resolve();
            if os != ns {
                patches.push(Patch::UpdateText {
                    node: path,
                    content: ns,
                });
            }
        }
        (Column(o), Column(n)) => diff_box(o, n, path, &mut patches),
        (Row(o), Row(n)) => diff_box(o, n, path, &mut patches),
        (Box_(o), Box_(n)) => diff_box(o, n, path, &mut patches),
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
        (Button(o), Button(n)) => {
            if o.style != n.style {
                patches.push(Patch::UpdateStyle {
                    node: path.clone(),
                    style: n.style,
                });
            }
            let ol = o.label.resolve();
            let nl = n.label.resolve();
            if ol != nl {
                patches.push(Patch::UpdateText {
                    node: path.clone(),
                    content: nl,
                });
            }
            let op = o.paint.resolve();
            let np = n.paint.resolve();
            if op != np {
                patches.push(Patch::UpdatePaint {
                    node: path,
                    paint: np,
                });
            }
        }
        (Image(o), Image(n)) => {
            if o.src != n.src {
                patches.push(Patch::ReplaceNode {
                    node: path,
                    new_element: Image(n),
                });
            } else if o.style != n.style {
                patches.push(Patch::UpdateStyle {
                    node: path,
                    style: n.style,
                });
            }
        }
        (None, None) => {}
        // Different types — replace entirely
        (_, new) => {
            patches.push(Patch::ReplaceNode {
                node: path,
                new_element: new,
            });
        }
    }

    patches
}

fn diff_box(
    o: crate::element::types::BoxElement,
    n: crate::element::types::BoxElement,
    path: NodePath,
    patches: &mut Vec<Patch>,
) {
    if o.style != n.style {
        patches.push(Patch::UpdateStyle {
            node: path.clone(),
            style: n.style,
        });
    }
    let op = o.paint.resolve();
    let np = n.paint.resolve();
    if op != np {
        patches.push(Patch::UpdatePaint {
            node: path.clone(),
            paint: np,
        });
    }
    diff_children(o.children, n.children, &path, patches);
}

pub(crate) fn element_key(element: &Element) -> Option<crate::element::types::Key> {
    use Element::*;
    match element {
        Text(text) => text.key.clone(),
        Column(container) | Row(container) | Box_(container) => container.key.clone(),
        Button(button) => button.key.clone(),
        Image(image) => image.key.clone(),
        Component(component) => component.key().cloned(),
        Fragment(_) => Option::None,
        Element::None => Option::None,
    }
}

pub(crate) fn children_are_fully_keyed(children: &[Element]) -> bool {
    !children.is_empty() && children.iter().all(|child| element_key(child).is_some())
}

fn diff_children(
    old: Vec<Element>,
    new: Vec<Element>,
    parent: &NodePath,
    patches: &mut Vec<Patch>,
) {
    if children_are_fully_keyed(&old) && children_are_fully_keyed(&new) {
        diff_children_keyed(old, new, parent, patches);
    } else {
        diff_children_by_index(old, new, parent, patches);
    }
}

fn diff_children_by_index(
    old: Vec<Element>,
    new: Vec<Element>,
    parent: &NodePath,
    patches: &mut Vec<Patch>,
) {
    let min = old.len().min(new.len());
    let mut old_iter = old.into_iter();
    let mut new_iter = new.into_iter();

    for i in 0..min {
        let child_patches = diff(
            old_iter.next().unwrap(),
            new_iter.next().unwrap(),
            parent.child(i),
        );
        patches.extend(child_patches);
    }

    for (i, el) in new_iter.enumerate() {
        push_insert_child(el, parent, min + i, patches);
    }

    let old_remaining: Vec<_> = old_iter.collect();
    for i in (0..old_remaining.len()).rev() {
        push_remove_child(&old_remaining[i], parent, min + i, patches);
    }
}

fn diff_children_keyed(
    old: Vec<Element>,
    new: Vec<Element>,
    parent: &NodePath,
    patches: &mut Vec<Patch>,
) {
    use std::collections::HashMap;

    let mut old_by_key: HashMap<crate::element::types::Key, (usize, Element)> = HashMap::new();
    let mut old_order = Vec::with_capacity(old.len());
    for (index, element) in old.into_iter().enumerate() {
        let key = element_key(&element).expect("keyed diff requires keys on every child");
        old_by_key.insert(key.clone(), (index, element));
        old_order.push(key);
    }

    let new_items: Vec<(crate::element::types::Key, Element)> = new
        .into_iter()
        .map(|element| {
            let key = element_key(&element).expect("keyed diff requires keys on every child");
            (key, element)
        })
        .collect();

    let new_order: Vec<_> = new_items.iter().map(|(key, _)| key.clone()).collect();
    let new_key_set: std::collections::HashSet<_> = new_order.iter().cloned().collect();
    let old_key_set: std::collections::HashSet<_> = old_by_key.keys().cloned().collect();

    let mut removals: Vec<(usize, Element)> = old_by_key
        .iter()
        .filter(|(key, _)| !new_key_set.contains(*key))
        .map(|(_, (index, element))| (*index, element.clone()))
        .collect();
    removals.sort_by(|(a, _), (b, _)| b.cmp(a));
    for (index, element) in removals {
        push_remove_child(&element, parent, index, patches);
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
            push_insert_child(new_child.clone(), parent, new_index, patches);
        }
    }

    for (new_index, (key, new_child)) in new_items.iter().enumerate() {
        if let Some((_, old_child)) = old_by_key.get(key) {
            patches.extend(diff(
                old_child.clone(),
                new_child.clone(),
                parent.child(new_index),
            ));
        }
    }
}

fn push_insert_child(element: Element, parent: &NodePath, index: usize, patches: &mut Vec<Patch>) {
    match element {
        Element::Component(component) => patches.push(Patch::MountComponent {
            node: parent.child(index),
            component,
        }),
        element => patches.push(Patch::InsertChild {
            parent: parent.clone(),
            index,
            element,
        }),
    }
}

fn push_remove_child(element: &Element, parent: &NodePath, index: usize, patches: &mut Vec<Patch>) {
    match element {
        Element::Component(_) => patches.push(Patch::UnmountComponent {
            node: parent.child(index),
        }),
        _ => patches.push(Patch::RemoveChild {
            parent: parent.clone(),
            index,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::{
        builders::{Button, Column, Component, Text},
        types::ImageElement,
    };

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
    fn different_types_produces_replace() {
        let patches = diff(
            Text::new("x").into_element(),
            Button::new("x").into_element(),
            NodePath::root(),
        );
        assert_eq!(patches.len(), 1);
        assert!(matches!(patches[0], Patch::ReplaceNode { .. }));
    }

    #[test]
    fn style_change_produces_update_style() {
        let old = Column::new().gap(4.0).into_element();
        let new = Column::new().gap(8.0).into_element();
        let patches = diff(old, new, NodePath::root());
        assert_eq!(patches.len(), 1);
        assert!(matches!(patches[0], Patch::UpdateStyle { .. }));
    }

    #[test]
    fn child_added_produces_insert() {
        let old = Column::new().into_element();
        let new = Column::new().child(txt("a")).into_element();
        let patches = diff(old, new, NodePath::root());
        assert_eq!(patches.len(), 1);
        assert!(matches!(patches[0], Patch::InsertChild { index: 0, .. }));
    }

    #[test]
    fn child_removed_produces_remove() {
        let old = Column::new().child(txt("a")).into_element();
        let new = Column::new().into_element();
        let patches = diff(old, new, NodePath::root());
        assert_eq!(patches.len(), 1);
        assert!(matches!(patches[0], Patch::RemoveChild { index: 0, .. }));
    }

    #[test]
    fn button_style_change_produces_update_style() {
        let old = Element::Button(crate::element::types::ButtonElement {
            label: "ok".into(),
            style: Default::default(),
            paint: Default::default(),
            on_click: None,
            key: None,
        });
        let new_style = crate::element::style::StyleProps {
            width: Some(crate::element::style::Dimension::Points(120.0)),
            ..Default::default()
        };
        let new = Element::Button(crate::element::types::ButtonElement {
            label: "ok".into(),
            style: new_style,
            paint: Default::default(),
            on_click: None,
            key: None,
        });
        let patches = diff(old, new, NodePath::root());
        assert!(patches
            .iter()
            .any(|p| matches!(p, Patch::UpdateStyle { .. })));
    }

    #[test]
    fn image_style_change_produces_update_style() {
        let old = Element::Image(ImageElement {
            src: "a.png".to_owned(),
            style: Default::default(),
            key: None,
        });
        let new_style = crate::element::style::StyleProps {
            height: Some(crate::element::style::Dimension::Points(64.0)),
            ..Default::default()
        };
        let new = Element::Image(ImageElement {
            src: "a.png".to_owned(),
            style: new_style,
            key: None,
        });
        let patches = diff(old, new, NodePath::root());
        assert!(patches
            .iter()
            .any(|p| matches!(p, Patch::UpdateStyle { .. })));
    }

    #[test]
    fn image_src_change_produces_replace() {
        let old = Element::Image(ImageElement {
            src: "a.png".to_owned(),
            style: Default::default(),
            key: None,
        });
        let new = Element::Image(ImageElement {
            src: "b.png".to_owned(),
            style: Default::default(),
            key: None,
        });
        let patches = diff(old, new, NodePath::root());
        assert!(patches
            .iter()
            .any(|p| matches!(p, Patch::ReplaceNode { .. })));
    }

    #[test]
    fn unchanged_children_produce_no_patches() {
        let old = Column::new().child(txt("a")).child(txt("b")).into_element();
        let new = Column::new().child(txt("a")).child(txt("b")).into_element();
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

        let old = Column::new().into_element();
        let new = Column::new()
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
        let old = Column::new()
            .child(Text::new("a").key(1))
            .child(Text::new("b").key(2))
            .into_element();
        let new = Column::new()
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
        let old = Column::new().child(Text::new("a").key(1)).into_element();
        let new = Column::new()
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
        let old = Column::new()
            .child(Text::new("a").key(1))
            .child(Text::new("b").key(2))
            .into_element();
        let new = Column::new().child(Text::new("a").key(1)).into_element();

        let patches = diff(old, new, NodePath::root());

        assert!(matches!(
            patches.first(),
            Some(Patch::RemoveChild { index: 1, .. })
        ));
    }

    #[test]
    fn keyed_key_change_removes_old_and_inserts_new() {
        let old = Column::new().child(Text::new("a").key(1)).into_element();
        let new = Column::new().child(Text::new("b").key(2)).into_element();

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

        let old = Column::new().child(Text::new("a").key(1)).into_element();
        let new = Column::new()
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

        let old = Column::new()
            .child(Text::new("a").key(1))
            .child(Component::new(child).key(2))
            .into_element();
        let new = Column::new().child(Text::new("a").key(1)).into_element();

        let patches = diff(old, new, NodePath::root());

        assert!(matches!(
            patches.first(),
            Some(Patch::UnmountComponent { node }) if *node == NodePath(vec![1])
        ));
    }

    #[test]
    fn mixed_keyed_and_unkeyed_children_fall_back_to_index_diff() {
        let old = Column::new()
            .child(Text::new("a").key(1))
            .child(Text::new("b"))
            .into_element();
        let new = Column::new()
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

        let old = Column::new()
            .child(Component::new(child).key(2))
            .into_element();
        let new = Column::new().into_element();

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
