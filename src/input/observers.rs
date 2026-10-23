//! Element focus and hover notifications, including lifecycle-driven exits.
use super::*;
use crate::retained::{EventHandlers, RetainedNode};

fn path_to(
    node: &RetainedNode,
    id: taffy::NodeId,
    path: &mut Vec<(taffy::NodeId, EventHandlers)>,
) -> bool {
    if !node.style.interactive {
        return false;
    }
    let length = path.len();
    if let Some(id) = node.taffy_id {
        path.push((id, node.handlers.clone()));
    }
    if node.taffy_id == Some(id) || node.children.iter().any(|child| path_to(child, id, path)) {
        return true;
    }
    path.truncate(length);
    false
}

fn leave(handlers: &EventHandlers) {
    if let Some(callback) = &handlers.on_hover_leave {
        callback();
    }
    if let Some(callback) = &handlers.on_hover {
        callback(false);
    }
}

impl UiState {
    pub(super) fn notify_focus(&mut self) {
        let next = self.input.focus_manager.focused.and_then(|id| {
            let node = self
                .retained
                .as_ref()?
                .root
                .as_ref()
                .and_then(|root| find_interactive_node_by_taffy_id(root, id))?;
            Some((id, node.handlers.on_focus.clone()))
        });
        let old = std::mem::replace(&mut self.input.observed_focus, next.clone());
        if old.as_ref().map(|(id, _)| id) == next.as_ref().map(|(id, _)| id) {
            return;
        }
        if let Some((_, Some(callback))) = old {
            callback(false);
        }
        if let Some((_, Some(callback))) = next {
            callback(true);
        }
        self.paint_dirty = true;
    }

    pub(super) fn reconcile_hover_observers(&mut self) {
        let root = self.retained.as_ref().and_then(|tree| tree.root.as_ref());
        self.input.hovered_nodes.retain_mut(|(id, handlers)| {
            if let Some(node) = root.and_then(|root| find_interactive_node_by_taffy_id(root, *id)) {
                *handlers = node.handlers.clone();
                true
            } else {
                leave(handlers);
                false
            }
        });
    }

    pub(super) fn clear_hover(&mut self) {
        for (_, handlers) in self.input.hovered_nodes.drain(..).rev() {
            leave(&handlers);
        }
        self.input.hovered_node_id = None;
        self.input.cursor = Cursor::Default;
        self.paint_dirty = true;
    }

    pub(super) fn event_pass_hover(&mut self, point: LogicalPoint) {
        let root = self.retained.as_ref().and_then(|tree| tree.root.as_ref());
        let root = root.map(|root| crate::retained::focus::active_modal(root).unwrap_or(root));
        let hit = root.and_then(|root| hit_test_hover(root, &self.layout_map, point));
        let id = hit.and_then(|node| node.taffy_id);
        let cursor = hit
            .map(|node| node.style.cursor.clone())
            .unwrap_or_default();
        let mut next = Vec::new();
        if let (Some(root), Some(id)) = (root, id) {
            path_to(root, id, &mut next);
        }
        next.retain(|(_, h)| {
            h.on_hover.is_some() || h.on_hover_enter.is_some() || h.on_hover_leave.is_some()
        });
        for (id, handlers) in self.input.hovered_nodes.iter().rev() {
            if !next.iter().any(|(next, _)| next == id) {
                leave(handlers);
            }
        }
        for (id, handlers) in &next {
            if !self.input.hovered_nodes.iter().any(|(old, _)| old == id) {
                if let Some(callback) = &handlers.on_hover_enter {
                    callback();
                }
                if let Some(callback) = &handlers.on_hover {
                    callback(true);
                }
            }
        }
        self.paint_dirty |= self.input.hovered_node_id != id || self.input.cursor != cursor;
        self.input.hovered_nodes = next;
        self.input.hovered_node_id = id;
        self.input.cursor = cursor;
    }
}
