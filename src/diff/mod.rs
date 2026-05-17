use crate::element::{Element, style::{PaintData, StyleProps}};

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

pub enum Patch {
    UpdateStyle { node: NodePath, style: StyleProps },
    UpdatePaint { node: NodePath, paint: PaintData },
    UpdateText { node: NodePath, content: String },
    ReplaceNode { node: NodePath, new_element: Element },
    InsertChild { parent: NodePath, index: usize, element: Element },
    RemoveChild { parent: NodePath, index: usize },
    /// Reserved for keyed diffing — not emitted by the current unkeyed diff implementation.
    MoveChild { parent: NodePath, from: usize, to: usize },
}

pub fn diff(old: Element, new: Element, path: NodePath) -> Vec<Patch> {
    use Element::*;
    let mut patches = Vec::new();

    match (old, new) {
        (Text(o), Text(n)) => {
            let os = o.content.resolve();
            let ns = n.content.resolve();
            if os != ns {
                patches.push(Patch::UpdateText { node: path, content: ns });
            }
        }
        (Column(o), Column(n)) => diff_box(o, n, path, &mut patches),
        (Row(o), Row(n)) => diff_box(o, n, path, &mut patches),
        (Box_(o), Box_(n)) => diff_box(o, n, path, &mut patches),
        (Button(o), Button(n)) => {
            // TODO: diff button style (ButtonElement::style is currently not diffed; style changes will be dropped)
            let ol = o.label.resolve();
            let nl = n.label.resolve();
            if ol != nl {
                patches.push(Patch::UpdateText { node: path.clone(), content: nl });
            }
            let op = o.paint.resolve();
            let np = n.paint.resolve();
            if op != np {
                patches.push(Patch::UpdatePaint { node: path, paint: np });
            }
        }
        (None, None) | (Image(_), Image(_)) => {}
        // Different types — replace entirely
        (_, new) => {
            patches.push(Patch::ReplaceNode { node: path, new_element: new });
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
        patches.push(Patch::UpdateStyle { node: path.clone(), style: n.style });
    }
    let op = o.paint.resolve();
    let np = n.paint.resolve();
    if op != np {
        patches.push(Patch::UpdatePaint { node: path.clone(), paint: np });
    }
    diff_children(o.children, n.children, &path, patches);
}

fn diff_children(
    old: Vec<Element>,
    new: Vec<Element>,
    parent: &NodePath,
    patches: &mut Vec<Patch>,
) {
    let min = old.len().min(new.len());
    let mut old_iter = old.into_iter();
    let mut new_iter = new.into_iter();

    for i in 0..min {
        let child_patches =
            diff(old_iter.next().unwrap(), new_iter.next().unwrap(), parent.child(i));
        patches.extend(child_patches);
    }

    // Extra new children → insert
    for (i, el) in new_iter.enumerate() {
        patches.push(Patch::InsertChild { parent: parent.clone(), index: min + i, element: el });
    }

    // Removed old children → remove in reverse order to keep indices stable
    let old_remaining: Vec<_> = old_iter.collect();
    for i in (0..old_remaining.len()).rev() {
        patches
            .push(Patch::RemoveChild { parent: parent.clone(), index: min + i });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::builders::{Button, Column, Text};

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
        assert!(
            matches!(&patches[0], Patch::UpdateText { content, .. } if content == "new")
        );
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
    fn unchanged_children_produce_no_patches() {
        let old = Column::new()
            .child(txt("a"))
            .child(txt("b"))
            .into_element();
        let new = Column::new()
            .child(txt("a"))
            .child(txt("b"))
            .into_element();
        assert!(diff(old, new, NodePath::root()).is_empty());
    }
}
