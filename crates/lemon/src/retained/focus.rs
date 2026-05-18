use taffy::NodeId;

use crate::retained::{RetainedNode, RetainedTree};

pub struct FocusManager {
    pub focused: Option<NodeId>,
}

impl FocusManager {
    pub fn new() -> Self {
        Self { focused: None }
    }

    pub fn cycle(&mut self, tree: &RetainedTree, forward: bool) {
        let focusable = collect_focusable(tree);
        if focusable.is_empty() {
            return;
        }

        let current = self.focused.and_then(|id| {
            focusable
                .iter()
                .position(|&focusable_id| focusable_id == id)
        });
        let len = focusable.len();
        let next = match current {
            None if forward => 0,
            None => len - 1,
            Some(i) if forward => (i + 1) % len,
            Some(i) => (i + len - 1) % len,
        };
        self.focused = Some(focusable[next]);
    }

    pub fn focus_by_id(&mut self, id: NodeId, tree: &RetainedTree) {
        let focusable = collect_focusable(tree);
        if focusable.contains(&id) {
            self.focused = Some(id);
        }
    }
}

impl Default for FocusManager {
    fn default() -> Self {
        Self::new()
    }
}

pub fn collect_focusable(tree: &RetainedTree) -> Vec<NodeId> {
    let mut out = Vec::new();
    if let Some(root) = &tree.root {
        collect_focusable_node(root, &mut out);
    }
    out
}

fn collect_focusable_node(node: &RetainedNode, out: &mut Vec<NodeId>) {
    if node.style.focusable {
        if let Some(id) = node.taffy_id {
            out.push(id);
        }
    }
    for child in &node.children {
        collect_focusable_node(child, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::builders::{Box_, Column};

    #[test]
    fn collect_focusable_finds_marked_nodes() {
        let element = Column::new()
            .child(Box_::new().width(10.0).height(10.0).focusable())
            .child(Box_::new().width(10.0).height(10.0))
            .child(Box_::new().width(10.0).height(10.0).focusable())
            .into_element();

        let tree = RetainedTree::mount(element).unwrap();
        let focusable = collect_focusable(&tree);
        assert_eq!(focusable.len(), 2);
    }

    #[test]
    fn cycle_forward_advances_through_focusable_nodes() {
        let element = Column::new()
            .child(Box_::new().width(10.0).height(10.0).focusable())
            .child(Box_::new().width(10.0).height(10.0).focusable())
            .into_element();

        let tree = RetainedTree::mount(element).unwrap();
        let mut fm = FocusManager::new();
        let ids = collect_focusable(&tree);

        fm.cycle(&tree, true);
        assert_eq!(fm.focused, Some(ids[0]));

        fm.cycle(&tree, true);
        assert_eq!(fm.focused, Some(ids[1]));

        fm.cycle(&tree, true);
        assert_eq!(fm.focused, Some(ids[0]));
    }

    #[test]
    fn cycle_backward_wraps_correctly() {
        let element = Column::new()
            .child(Box_::new().width(10.0).height(10.0).focusable())
            .child(Box_::new().width(10.0).height(10.0).focusable())
            .into_element();

        let tree = RetainedTree::mount(element).unwrap();
        let mut fm = FocusManager::new();
        let ids = collect_focusable(&tree);

        fm.cycle(&tree, false);
        assert_eq!(fm.focused, Some(ids[1]));
    }

    #[test]
    fn no_focusable_nodes_cycle_is_noop() {
        let element = Column::new()
            .child(Box_::new().width(10.0).height(10.0))
            .into_element();
        let tree = RetainedTree::mount(element).unwrap();
        let mut fm = FocusManager::new();
        fm.cycle(&tree, true);
        assert!(fm.focused.is_none());
    }
}
