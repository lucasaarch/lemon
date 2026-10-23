use taffy::NodeId;

use crate::retained::{RetainedNode, RetainedTree};

pub struct FocusManager {
    pub(crate) focused: Option<NodeId>,
    scopes: Vec<(NodeId, Option<NodeId>)>,
}

impl FocusManager {
    /// Returns the current focus; use `cycle` or `focus_by_id` to change it.
    pub fn focused(&self) -> Option<NodeId> {
        self.focused
    }
    pub fn new() -> Self {
        Self {
            focused: None,
            scopes: Vec::new(),
        }
    }

    // Keep restoration history here, alongside focus traversal, rather than in a widget.
    pub(crate) fn reconcile_modal(&mut self, tree: &RetainedTree) -> bool {
        let previous = self.focused;
        let root = tree.root.as_ref();
        let modal = root.and_then(active_modal).and_then(|node| node.taffy_id);
        while let Some(&(id, saved)) = self.scopes.last() {
            if Some(id) == modal {
                break;
            }
            let opening_another =
                modal.is_some_and(|next| !self.scopes.iter().any(|(id, _)| *id == next));
            let still_open = root
                .and_then(|root| find_scope(root, id))
                .is_some_and(|node| node.style.interactive);
            if opening_another && still_open {
                break;
            }
            self.scopes.pop();
            self.focused = saved;
        }
        if let Some(id) = modal {
            if self.scopes.last().is_none_or(|(old, _)| *old != id) {
                self.scopes.push((id, self.focused));
                self.focused = None;
                self.cycle(tree, true);
            }
        }
        previous != self.focused
    }

    pub fn cycle(&mut self, tree: &RetainedTree, forward: bool) {
        let focusable = collect_focusable(tree);
        if focusable.is_empty() {
            self.focused = None;
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
        let root = active_modal(root).unwrap_or(root);
        collect_focusable_node(root, &mut out);
        if root.style.modal && out.len() > 1 && out.first().copied() == root.taffy_id {
            out.remove(0);
        }
    }
    out
}

fn find_scope(node: &RetainedNode, id: NodeId) -> Option<&RetainedNode> {
    if !node.style.interactive {
        return None;
    }
    if node.taffy_id == Some(id) {
        return Some(node);
    }
    node.children.iter().find_map(|child| find_scope(child, id))
}

// Match the deferred ordering used by painting: a later modal at the same layer wins.
pub(crate) fn active_modal(root: &RetainedNode) -> Option<&RetainedNode> {
    fn visit<'a>(
        node: &'a RetainedNode,
        best: &mut Option<&'a RetainedNode>,
        deferred: &mut Vec<&'a RetainedNode>,
    ) {
        if !node.style.interactive {
            return;
        }
        if node.style.modal {
            *best = Some(node);
        }
        for child in &node.children {
            if child.style.z_index != 0 || child.style.position == crate::Position::Fixed {
                deferred.push(child);
            } else {
                visit(child, best, deferred);
            }
        }
    }
    let mut best = None;
    let mut deferred = Vec::new();
    visit(root, &mut best, &mut deferred);
    while !deferred.is_empty() {
        deferred.sort_by_key(|node| node.style.z_index);
        let node = deferred.remove(0);
        visit(node, &mut best, &mut deferred);
    }
    best
}

fn collect_focusable_node(node: &RetainedNode, out: &mut Vec<NodeId>) {
    if !node.style.interactive {
        return;
    }
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
    use crate::element::builders::{ColumnView, View};

    #[test]
    fn foremost_modal_follows_deferred_paint_order_in_nested_scopes() {
        fn modal(view: View) -> crate::element::Element {
            let mut element = view.into_element();
            if let crate::element::Element::View(node) = &mut element {
                node.style.modal = true;
            }
            element
        }
        let nested = modal(View::new().layer(100));
        let element = View::new()
            .child(modal(View::new().layer(500).child(nested)))
            .child(modal(View::new().layer(400)))
            .into_element();
        let tree = RetainedTree::mount(element).unwrap();
        let root = tree.root.as_ref().unwrap();
        assert_eq!(
            active_modal(root).unwrap().taffy_id,
            root.children[0].children[0].taffy_id
        );
    }

    #[test]
    fn collect_focusable_finds_marked_nodes() {
        let element = ColumnView::new()
            .child(View::new().width(10.0).height(10.0).focusable())
            .child(View::new().width(10.0).height(10.0))
            .child(View::new().width(10.0).height(10.0).focusable())
            .into_element();

        let tree = RetainedTree::mount(element).unwrap();
        let focusable = collect_focusable(&tree);
        assert_eq!(focusable.len(), 2);
    }

    #[test]
    fn cycle_forward_advances_through_focusable_nodes() {
        let element = ColumnView::new()
            .child(View::new().width(10.0).height(10.0).focusable())
            .child(View::new().width(10.0).height(10.0).focusable())
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
        let element = ColumnView::new()
            .child(View::new().width(10.0).height(10.0).focusable())
            .child(View::new().width(10.0).height(10.0).focusable())
            .into_element();

        let tree = RetainedTree::mount(element).unwrap();
        let mut fm = FocusManager::new();
        let ids = collect_focusable(&tree);

        fm.cycle(&tree, false);
        assert_eq!(fm.focused, Some(ids[1]));
    }

    #[test]
    fn no_focusable_nodes_cycle_is_noop() {
        let element = ColumnView::new()
            .child(View::new().width(10.0).height(10.0))
            .into_element();
        let tree = RetainedTree::mount(element).unwrap();
        let mut fm = FocusManager::new();
        fm.cycle(&tree, true);
        assert!(fm.focused.is_none());
    }
}
