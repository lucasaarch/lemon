//! Debug-only snapshots and Lemon's in-process inspector. Native window ownership stays in platform.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::Duration;

use taffy::NodeId;

use crate::diff::NodePath;
use crate::layout::LayoutRect;
use crate::ui::UiState;
use crate::State;

mod details;
mod snapshot;
#[cfg(test)]
mod tests;
mod view;

use snapshot::collect_nodes;

const HISTORY: usize = 120;

#[derive(Clone, Default)]
pub(crate) struct RenderInfo {
    pub renders: u64,
    pub last_render: Duration,
    pub total_render: Duration,
    pub hooks: Vec<HookInfo>,
}

#[derive(Clone)]
pub(crate) struct HookInfo {
    pub kind: &'static str,
    pub value_type: &'static str,
    pub calls: u64,
}

#[derive(Clone)]
pub(crate) struct ComponentInfo {
    pub id: usize,
    pub path: NodePath,
    pub key: Option<crate::element::types::Key>,
    pub render: RenderInfo,
}

#[derive(Clone, Default)]
pub(crate) struct FrameSample {
    pub(super) number: u64,
    pub updates: Duration,
    pub layout: Duration,
    pub paint: Duration,
    pub patches: usize,
    pub layouts: usize,
    pub paints: usize,
    pub fills: u32,
    pub strokes: u32,
    pub glyph_runs: u32,
    pub effect_layers: usize,
}

#[derive(Default)]
pub(crate) struct Capture {
    pub pending: FrameSample,
    pub highlighted: Option<NodeId>,
    pub hovered: Option<NodeId>,
}

pub(super) struct NodeInfo {
    pub(super) id: Option<NodeId>,
    pub(super) tree_id: u64,
    pub(super) path: NodePath,
    pub(super) label: String,
    pub(super) kind: String,
    pub(super) debug_id: Option<String>,
    pub(super) rect: Option<LayoutRect>,
    pub(super) icon: crate::IconData,
    pub(super) icon_color: crate::Color,
    pub(super) properties: Vec<(String, String)>,
}

#[derive(Default)]
pub(super) struct Snapshot {
    pub(super) nodes: Vec<NodeInfo>,
    pub(super) components: Vec<ComponentInfo>,
    pub(super) frames: Vec<FrameSample>,
}

/// One target's inspector state. Owned by its desktop window, never globally registered.
#[derive(Clone)]
pub(crate) struct Session {
    snapshot: State<Rc<Snapshot>>,
    pub selected: State<Option<NodeId>>,
    pub hovered: State<Option<NodeId>>,
    paused: State<bool>,
    history: Rc<RefCell<VecDeque<FrameSample>>>,
}

impl Session {
    pub fn new() -> Self {
        Self {
            snapshot: State::new(Rc::new(Snapshot::default())),
            selected: State::new(None),
            hovered: State::new(None),
            paused: State::new(false),
            history: Rc::default(),
        }
    }

    pub fn publish(&self, ui: &mut UiState) {
        let Some(capture) = ui.inspection.as_mut() else {
            return;
        };
        let mut sample = std::mem::take(&mut capture.pending);
        // Pausing discards samples instead of accumulating a misleading giant frame.
        if self.paused.peek() {
            return;
        }
        let mut history = self.history.borrow_mut();
        sample.number = history.back().map_or(1, |frame| frame.number + 1);
        if history.len() == HISTORY {
            history.pop_front();
        }
        history.push_back(sample);
        let mut nodes = Vec::new();
        if let Some(root) = ui.retained().and_then(|tree| tree.root()) {
            collect_nodes(root, ui, &NodePath::root(), 0, &mut nodes);
        }
        let selected = self.selected.peek();
        if selected.is_some() && !nodes.iter().any(|node| node.id == selected) {
            self.selected.set(None);
        }
        let hovered = self.hovered.peek();
        if hovered.is_some() && !nodes.iter().any(|node| node.id == hovered) {
            self.hovered.set(None);
        }
        self.snapshot.set(Rc::new(Snapshot {
            nodes,
            components: ui.runtime().inspect_components(),
            frames: history.iter().cloned().collect(),
        }));
    }
}
