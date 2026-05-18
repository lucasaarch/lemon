pub mod focus;

use std::cell::Cell;
use std::rc::Rc;

use taffy::{NodeId, Rect, Size, Style, TaffyTree};

use crate::diff::{NodePath, Patch};
use crate::element::style::{
    Align, Color, CornerRadii, Dimension, Edges, Justify, PaintData, StyleProps, TextStyle,
};
use crate::element::{
    types::{BoxElement, ButtonElement, ImageElement, TextElement},
    Element,
};

#[derive(Debug)]
pub enum RetainedError {
    Taffy(taffy::TaffyError),
    UnsupportedElement(&'static str),
    UnsupportedPatch(&'static str),
    InvalidNodePath(NodePath),
    MissingTextCache(NodePath),
}

impl From<taffy::TaffyError> for RetainedError {
    fn from(value: taffy::TaffyError) -> Self {
        Self::Taffy(value)
    }
}

#[derive(Clone)]
pub struct TextCache {
    pub content: String,
    pub style: TextStyle,
    /// Caret byte offset for text-input fields (synced from [`TextInputMeta`](crate::element::types::TextInputMeta)).
    pub caret: usize,
    pub needs_layout: bool,
    pub parley_layout: Option<parley::Layout<[u8; 4]>>,
    pub layout_max_width: Option<f32>,
}

impl std::fmt::Debug for TextCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextCache")
            .field("content", &self.content)
            .field("style", &self.style)
            .field("needs_layout", &self.needs_layout)
            .field(
                "parley_layout",
                &self.parley_layout.as_ref().map(|_| "Layout"),
            )
            .field("layout_max_width", &self.layout_max_width)
            .finish()
    }
}

impl PartialEq for TextCache {
    fn eq(&self, other: &Self) -> bool {
        self.content == other.content
            && self.style == other.style
            && self.needs_layout == other.needs_layout
            && self.layout_max_width == other.layout_max_width
    }
}

/// Input callbacks attached to a retained node (populated from container builders).
#[derive(Clone, Default)]
pub struct EventHandlers {
    pub on_click: Option<Rc<dyn Fn()>>,
    pub on_key_down: Option<Rc<dyn Fn(crate::element::events::KeyEvent)>>,
    pub on_key_up: Option<Rc<dyn Fn(crate::element::events::KeyEvent)>>,
    pub on_hover_enter: Option<Rc<dyn Fn()>>,
    pub on_hover_leave: Option<Rc<dyn Fn()>>,
    /// Vertical wheel delta in logical pixels; set via [`.on_scroll`](crate::element::builders::Column::on_scroll).
    pub on_scroll: Option<Rc<dyn Fn(f64)>>,
    /// Called when a click is dispatched outside this retained node's bounds.
    pub on_click_outside: Option<Rc<dyn Fn()>>,
    /// Called when a pointer press lands inside this retained node with normalized `(x, y)` coordinates.
    pub on_pointer_down: Option<Rc<dyn Fn(f32, f32)>>,
    /// Called when pointer movement is reported for this retained node with normalized `(x, y)` coordinates.
    pub on_pointer_move: Option<Rc<dyn Fn(f32, f32)>>,
    /// Updated after each [`layout_pass`](crate::layout::layout_pass) for scroll viewports; used to clamp wheel delta.
    pub scroll_layout_max: Option<Rc<Cell<f64>>>,
}

impl std::fmt::Debug for EventHandlers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventHandlers")
            .field("on_click", &self.on_click.as_ref().map(|_| "Rc<dyn Fn()>"))
            .field(
                "on_key_down",
                &self.on_key_down.as_ref().map(|_| "Rc<dyn Fn(KeyEvent)>"),
            )
            .field(
                "on_key_up",
                &self.on_key_up.as_ref().map(|_| "Rc<dyn Fn(KeyEvent)>"),
            )
            .field(
                "on_hover_enter",
                &self.on_hover_enter.as_ref().map(|_| "Rc<dyn Fn()>"),
            )
            .field(
                "on_hover_leave",
                &self.on_hover_leave.as_ref().map(|_| "Rc<dyn Fn()>"),
            )
            .field(
                "on_scroll",
                &self.on_scroll.as_ref().map(|_| "Rc<dyn Fn(f64)>"),
            )
            .field(
                "on_click_outside",
                &self.on_click_outside.as_ref().map(|_| "Rc<dyn Fn()>"),
            )
            .field(
                "on_pointer_down",
                &self
                    .on_pointer_down
                    .as_ref()
                    .map(|_| "Rc<dyn Fn(f32, f32)>"),
            )
            .field(
                "on_pointer_move",
                &self
                    .on_pointer_move
                    .as_ref()
                    .map(|_| "Rc<dyn Fn(f32, f32)>"),
            )
            .field(
                "scroll_layout_max",
                &self.scroll_layout_max.as_ref().map(|_| "Rc<Cell<f64>>"),
            )
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum RetainedKind {
    View,
    Row,
    Column,
    Text,
    Button,
    Image {
        src: String,
    },
    Component {
        type_id: std::any::TypeId,
        key: Option<crate::element::types::Key>,
    },
}

#[derive(Clone, Debug)]
pub struct RetainedNode {
    pub kind: RetainedKind,
    pub taffy_id: Option<NodeId>,
    pub style: StyleProps,
    pub paint: PaintData,
    pub children: Vec<RetainedNode>,
    pub handlers: EventHandlers,
    pub text: Option<TextCache>,
    pub text_input: Option<crate::element::types::TextInputMeta>,
    pub scroll_viewport: bool,
    pub scroll_bar: bool,
}

impl RetainedNode {
    pub fn layout_node_id(&self) -> Option<NodeId> {
        self.taffy_id.or_else(|| {
            self.children
                .iter()
                .find_map(|child| child.layout_node_id())
        })
    }

    pub fn text(taffy_id: NodeId, content: String, mut style: TextStyle, color: Color) -> Self {
        style.color = Some(color);

        Self {
            kind: RetainedKind::Text,
            taffy_id: Some(taffy_id),
            style: StyleProps::default(),
            paint: PaintData {
                background: None,
                border_color: None,
                border_width: 0.0,
                radius: CornerRadii::default(),
                image: None,
            },
            children: Vec::new(),
            handlers: EventHandlers::default(),
            text: Some(TextCache {
                content,
                style,
                caret: 0,
                needs_layout: true,
                parley_layout: None,
                layout_max_width: None,
            }),
            text_input: None,
            scroll_viewport: false,
            scroll_bar: false,
        }
    }

    pub fn text_content(&self) -> Option<&str> {
        self.text.as_ref().map(|text| text.content.as_str())
    }
}

/// Mutable UI tree synchronized with Taffy layout nodes and Vello paint data.
///
/// Built from an [`Element`] via [`mount`](Self::mount), then updated with
/// patches from [`Runtime::take_patches`](crate::Runtime::take_patches). [`run`](crate::platform::run)
/// manages this internally; use it directly in integration tests or custom hosts.
#[derive(Debug)]
pub struct RetainedTree {
    /// Shared Taffy tree backing flex layout for this UI.
    pub taffy: TaffyTree<()>,
    /// Root retained node, if [`mount`](Self::mount) succeeded.
    pub root: Option<RetainedNode>,
    /// When `true`, [`layout_pass`](crate::layout::layout_pass) should run before painting.
    pub layout_dirty: bool,
}

impl RetainedTree {
    /// Returns true if any text node still needs a layout pass (e.g. after `UpdateText`).
    pub fn text_needs_reflow(&self) -> bool {
        self.root.as_ref().is_some_and(text_node_needs_reflow)
    }

    /// Empty tree with no root node.
    pub fn new() -> Self {
        Self {
            taffy: TaffyTree::new(),
            root: None,
            layout_dirty: false,
        }
    }

    /// Builds the initial retained tree from a frozen element tree (e.g. [`Runtime::root_element`](crate::Runtime::root_element)).
    pub fn mount(element: Element) -> Result<Self, RetainedError> {
        crate::lemon_trace!(
            Retained,
            "mount root={}",
            crate::debug::element_kind(&element)
        );
        let mut tree = Self::new();
        let root = tree.build_node(element)?;
        tree.root = Some(root);
        tree.layout_dirty = true;
        Ok(tree)
    }

    /// Applies a batch of diff patches and marks layout dirty.
    pub fn apply_patches(
        &mut self,
        patches: impl IntoIterator<Item = Patch>,
    ) -> Result<(), RetainedError> {
        for patch in patches {
            self.apply_patch(patch)?;
        }
        Ok(())
    }

    fn build_node(&mut self, element: Element) -> Result<RetainedNode, RetainedError> {
        match element {
            Element::View(node) => self.build_box_node(RetainedKind::View, node),
            Element::Row(node) => self.build_box_node(RetainedKind::Row, node),
            Element::Column(node) => self.build_box_node(RetainedKind::Column, node),
            Element::Text(node) => self.build_text_node(node),
            Element::Button(node) => self.build_button_node(node),
            Element::Image(node) => self.build_image_node(node),
            Element::Component(_component) => {
                self.build_text_node(crate::element::types::TextElement {
                    content: crate::element::content::TextContent::Static(String::new()),
                    style: TextStyle::default(),
                    key: None,
                })
            }
            Element::Fragment(_) => Err(RetainedError::UnsupportedElement("Fragment")),
            Element::None => Err(RetainedError::UnsupportedElement("None")),
        }
    }

    fn build_box_node(
        &mut self,
        kind: RetainedKind,
        node: BoxElement,
    ) -> Result<RetainedNode, RetainedError> {
        let BoxElement {
            style: node_style,
            paint,
            children: node_children,
            key: _,
            handlers,
            text_input,
            scroll_viewport,
            scroll_bar,
        } = node;
        let flat_children = crate::diff::flatten_children(node_children);
        let mut children = Vec::with_capacity(flat_children.len());
        for child in flat_children {
            children.push(self.build_node(child)?);
        }

        let mut style = node_style.to_taffy_style();
        style.flex_direction = match kind {
            RetainedKind::Column => taffy::FlexDirection::Column,
            RetainedKind::Row => taffy::FlexDirection::Row,
            _ => style.flex_direction,
        };

        let mut child_ids = Vec::new();
        for child in &children {
            collect_direct_layout_ids(child, &mut child_ids);
        }
        let taffy_id = self.taffy.new_with_children(style, &child_ids)?;

        let mut retained = RetainedNode {
            kind,
            taffy_id: Some(taffy_id),
            style: node_style,
            paint: paint.resolve(),
            children,
            handlers,
            text: None,
            text_input: text_input.clone(),
            scroll_viewport,
            scroll_bar,
        };
        if let Some(meta) = &text_input {
            sync_text_input_caret(&mut retained, meta.cursor);
        }
        Ok(retained)
    }

    fn build_text_node(&mut self, node: TextElement) -> Result<RetainedNode, RetainedError> {
        let content = node.content.resolve();
        let color = node.style.color.unwrap_or_default();
        let taffy_id = self.taffy.new_leaf(Style::default())?;

        Ok(RetainedNode::text(taffy_id, content, node.style, color))
    }

    fn build_button_node(&mut self, node: ButtonElement) -> Result<RetainedNode, RetainedError> {
        let label = node.label.resolve();
        let taffy_id = self.taffy.new_leaf(node.style.to_taffy_style())?;
        let label_style = TextStyle {
            font_size: 16.0,
            font_weight: 500,
            color: Some(crate::element::style::default_text_color()),
        };

        Ok(RetainedNode {
            kind: RetainedKind::Button,
            taffy_id: Some(taffy_id),
            style: node.style,
            paint: node.paint.resolve(),
            children: Vec::new(),
            handlers: EventHandlers {
                on_click: node.on_click,
                ..Default::default()
            },
            text: Some(TextCache {
                content: label,
                style: label_style,
                caret: 0,
                needs_layout: true,
                parley_layout: None,
                layout_max_width: None,
            }),
            text_input: None,
            scroll_viewport: false,
            scroll_bar: false,
        })
    }

    fn build_image_node(&mut self, node: ImageElement) -> Result<RetainedNode, RetainedError> {
        let taffy_id = self.taffy.new_leaf(node.style.to_taffy_style())?;

        Ok(RetainedNode {
            kind: RetainedKind::Image { src: node.src },
            taffy_id: Some(taffy_id),
            style: node.style,
            paint: PaintData::default(),
            children: Vec::new(),
            handlers: EventHandlers::default(),
            text: None,
            text_input: None,
            scroll_viewport: false,
            scroll_bar: false,
        })
    }

    pub fn apply_patch(&mut self, patch: Patch) -> Result<(), RetainedError> {
        crate::lemon_trace!(Patches, "apply {}", crate::debug::format_patch(&patch));
        match patch {
            Patch::UpdateComponent { node, component } => {
                let retained = self.node_mut_exact(&node)?;
                let RetainedKind::Component { type_id, key } = &mut retained.kind else {
                    return Err(RetainedError::InvalidNodePath(node));
                };
                *type_id = component.type_id();
                *key = component.key().cloned();
            }
            Patch::MountComponent { node, component } => {
                let wrapper = RetainedNode {
                    kind: RetainedKind::Component {
                        type_id: component.type_id(),
                        key: component.key().cloned(),
                    },
                    taffy_id: None,
                    style: StyleProps::default(),
                    paint: PaintData::default(),
                    children: Vec::new(),
                    handlers: EventHandlers::default(),
                    text: None,
                    text_input: None,
                    scroll_viewport: false,
                    scroll_bar: false,
                };
                self.replace_with_wrapper(node, wrapper)?;
            }
            Patch::UnmountComponent { node } => {
                self.unwrap_component(node)?;
            }
            Patch::UpdateStyle { node, style } => {
                let taffy_id = self
                    .node_mut(&node)?
                    .layout_node_id()
                    .ok_or_else(|| RetainedError::InvalidNodePath(node.clone()))?;
                self.taffy.set_style(taffy_id, style.to_taffy_style())?;
                self.node_mut(&node)?.style = style;
            }
            Patch::UpdatePaint { node, paint } => {
                self.node_mut(&node)?.paint = paint;
            }
            Patch::UpdateWidgetChrome {
                node,
                text_input,
                scroll_viewport,
                scroll_bar,
            } => {
                let retained = self.node_mut(&node)?;
                retained.text_input = text_input.clone();
                retained.scroll_viewport = scroll_viewport;
                retained.scroll_bar = scroll_bar;
                if let Some(meta) = text_input.as_ref() {
                    sync_text_input_caret(retained, meta.cursor);
                }
            }
            Patch::UpdateText { node, content } => {
                let retained = self.node_mut(&node)?;
                let text = retained
                    .text
                    .as_mut()
                    .ok_or_else(|| RetainedError::MissingTextCache(node.clone()))?;
                text.content = content;
                text.parley_layout = None;
                text.layout_max_width = None;
                text.needs_layout = true;
                if let Some(taffy_id) = retained.taffy_id {
                    self.taffy.mark_dirty(taffy_id)?;
                }
            }
            Patch::UpdateTextStyle { node, style } => {
                let retained = self.node_mut(&node)?;
                let text = retained
                    .text
                    .as_mut()
                    .ok_or_else(|| RetainedError::MissingTextCache(node.clone()))?;
                text.style = style;
                text.parley_layout = None;
                text.layout_max_width = None;
                text.needs_layout = true;
                if let Some(taffy_id) = retained.taffy_id {
                    self.taffy.mark_dirty(taffy_id)?;
                }
            }
            Patch::InsertChild {
                parent,
                index,
                element,
            } => {
                let child = self.build_node(element)?;
                self.node_mut_exact(&parent)?.children.insert(index, child);
                self.sync_layout_children(&parent)?;
            }
            Patch::RemoveChild { parent, index } => {
                let removed = self.node_mut_exact(&parent)?.children.remove(index);
                self.remove_subtree_from_taffy(removed)?;
                self.sync_layout_children(&parent)?;
            }
            Patch::ReplaceNode { node, new_element } => {
                self.replace_node(node, new_element)?;
            }
            Patch::MoveChild { parent, from, to } => {
                self.move_child(parent, from, to)?;
            }
        }

        self.layout_dirty = true;
        Ok(())
    }

    fn node_mut(&mut self, path: &NodePath) -> Result<&mut RetainedNode, RetainedError> {
        let root = self
            .root
            .as_mut()
            .ok_or_else(|| RetainedError::InvalidNodePath(path.clone()))?;
        node_mut_from(root, &path.0).ok_or_else(|| RetainedError::InvalidNodePath(path.clone()))
    }

    fn node_mut_exact(&mut self, path: &NodePath) -> Result<&mut RetainedNode, RetainedError> {
        let root = self
            .root
            .as_mut()
            .ok_or_else(|| RetainedError::InvalidNodePath(path.clone()))?;
        node_mut_from_exact(root, &path.0)
            .ok_or_else(|| RetainedError::InvalidNodePath(path.clone()))
    }

    /// Returns the exact retained node at `path` without skipping transparent component wrappers.
    fn node_exact(&self, path: &NodePath) -> Result<&RetainedNode, RetainedError> {
        let root = self
            .root
            .as_ref()
            .ok_or_else(|| RetainedError::InvalidNodePath(path.clone()))?;
        node_from_exact(root, &path.0).ok_or_else(|| RetainedError::InvalidNodePath(path.clone()))
    }

    fn replace_node(&mut self, path: NodePath, new_element: Element) -> Result<(), RetainedError> {
        let replacement = self.build_node(new_element)?;

        if path.0.is_empty() {
            if let Some(old_root) = self.root.replace(replacement) {
                self.remove_subtree_from_taffy(old_root)?;
            }
            return Ok(());
        }

        let (parent_path, index) = split_parent_path(&path)?;
        let removed = self.node_mut_exact(&parent_path)?.children.remove(index);
        self.node_mut_exact(&parent_path)?
            .children
            .insert(index, replacement);
        self.remove_subtree_from_taffy(removed)?;
        self.sync_layout_children(&parent_path)?;
        Ok(())
    }

    fn replace_with_wrapper(
        &mut self,
        path: NodePath,
        mut wrapper: RetainedNode,
    ) -> Result<(), RetainedError> {
        if path.0.is_empty() {
            if let Some(old_root) = self.root.take() {
                wrapper.children.push(old_root);
            }
            self.root = Some(wrapper);
            return Ok(());
        }

        let (parent_path, index) = split_parent_path(&path)?;
        let removed = self.node_mut_exact(&parent_path)?.children.remove(index);
        wrapper.children.push(removed);
        self.node_mut_exact(&parent_path)?
            .children
            .insert(index, wrapper);
        self.sync_layout_children(&parent_path)?;
        Ok(())
    }

    fn unwrap_component(&mut self, path: NodePath) -> Result<(), RetainedError> {
        if path.0.is_empty() {
            let root = self
                .root
                .take()
                .ok_or_else(|| RetainedError::InvalidNodePath(path.clone()))?;
            let RetainedKind::Component { .. } = root.kind else {
                return Err(RetainedError::InvalidNodePath(path));
            };
            let mut children = root.children;
            self.root = children.pop();
            return Ok(());
        }

        let (parent_path, index) = split_parent_path(&path)?;
        let removed = self.node_mut_exact(&parent_path)?.children.remove(index);
        let RetainedKind::Component { .. } = removed.kind else {
            return Err(RetainedError::InvalidNodePath(path));
        };

        for (offset, child) in removed.children.into_iter().enumerate() {
            self.node_mut_exact(&parent_path)?
                .children
                .insert(index + offset, child);
        }
        self.sync_layout_children(&parent_path)?;
        Ok(())
    }

    fn move_child(
        &mut self,
        parent: NodePath,
        from: usize,
        to: usize,
    ) -> Result<(), RetainedError> {
        let parent_node = self.node_mut_exact(&parent)?;
        let child = parent_node.children.remove(from);
        parent_node.children.insert(to, child);
        self.sync_layout_children(&parent)?;
        Ok(())
    }

    /// Rebuilds the nearest owning Taffy child list for mutations under `path`.
    fn sync_layout_children(&mut self, path: &NodePath) -> Result<(), RetainedError> {
        let Some(owner_path) = self.layout_owner_path(path)? else {
            return Ok(());
        };
        let owner_id = self
            .node_exact(&owner_path)?
            .taffy_id
            .ok_or_else(|| RetainedError::InvalidNodePath(owner_path.clone()))?;
        let mut child_ids = Vec::new();
        for child in &self.node_exact(&owner_path)?.children {
            collect_direct_layout_ids(child, &mut child_ids);
        }
        self.taffy.set_children(owner_id, &child_ids)?;
        Ok(())
    }

    /// Finds the nearest ancestor at or above `path` that owns a concrete Taffy node.
    fn layout_owner_path(&self, path: &NodePath) -> Result<Option<NodePath>, RetainedError> {
        let mut current = Some(path.clone());
        while let Some(candidate) = current {
            if self.node_exact(&candidate)?.taffy_id.is_some() {
                return Ok(Some(candidate));
            }
            current = if candidate.0.is_empty() {
                None
            } else {
                Some(NodePath(candidate.0[..candidate.0.len() - 1].to_vec()))
            };
        }
        Ok(None)
    }

    fn remove_subtree_from_taffy(&mut self, node: RetainedNode) -> Result<(), RetainedError> {
        for child in node.children {
            self.remove_subtree_from_taffy(child)?;
        }
        if let Some(taffy_id) = node.taffy_id {
            self.taffy.remove(taffy_id)?;
        }
        Ok(())
    }
}

impl Default for RetainedTree {
    fn default() -> Self {
        Self::new()
    }
}

impl StyleProps {
    pub fn to_taffy_style(&self) -> Style {
        Style {
            size: Size {
                width: self
                    .width
                    .clone()
                    .map_or(taffy::Dimension::Auto, into_taffy_dimension),
                height: self
                    .height
                    .clone()
                    .map_or(taffy::Dimension::Auto, into_taffy_dimension),
            },
            padding: self
                .padding
                .as_ref()
                .map_or(Rect::zero(), into_taffy_padding),
            margin: self.margin.as_ref().map_or(Rect::zero(), into_taffy_margin),
            inset: self.inset.as_ref().map_or(Rect::auto(), into_taffy_margin),
            gap: Size {
                width: self
                    .column_gap
                    .or(self.gap)
                    .map_or(taffy::LengthPercentage::Length(0.0), |v| {
                        taffy::LengthPercentage::Length(v)
                    }),
                height: self
                    .row_gap
                    .or(self.gap)
                    .map_or(taffy::LengthPercentage::Length(0.0), |v| {
                        taffy::LengthPercentage::Length(v)
                    }),
            },
            align_items: self.align_items.clone().map(into_taffy_align_items),
            align_self: self.align_self.clone().map(into_taffy_align_self),
            justify_content: self.justify_content.clone().map(into_taffy_justify_content),
            flex_grow: self.flex_grow.unwrap_or(0.0),
            flex_shrink: self.flex_shrink.unwrap_or(1.0),
            position: if self.position_absolute {
                taffy::Position::Absolute
            } else {
                taffy::Position::Relative
            },
            ..Default::default()
        }
    }
}

fn into_taffy_dimension(dimension: Dimension) -> taffy::Dimension {
    match dimension {
        Dimension::Auto => taffy::Dimension::Auto,
        Dimension::Points(value) => taffy::Dimension::Length(value),
        Dimension::Percent(value) => taffy::Dimension::Percent(value),
    }
}

fn into_taffy_padding(edges: &Edges<f32>) -> Rect<taffy::LengthPercentage> {
    Rect {
        left: taffy::LengthPercentage::Length(edges.left),
        right: taffy::LengthPercentage::Length(edges.right),
        top: taffy::LengthPercentage::Length(edges.top),
        bottom: taffy::LengthPercentage::Length(edges.bottom),
    }
}

fn into_taffy_margin(edges: &Edges<f32>) -> Rect<taffy::LengthPercentageAuto> {
    Rect {
        left: taffy::LengthPercentageAuto::Length(edges.left),
        right: taffy::LengthPercentageAuto::Length(edges.right),
        top: taffy::LengthPercentageAuto::Length(edges.top),
        bottom: taffy::LengthPercentageAuto::Length(edges.bottom),
    }
}

fn into_taffy_align_self(align: Align) -> taffy::AlignSelf {
    match align {
        Align::Stretch => taffy::AlignSelf::Stretch,
        Align::Start => taffy::AlignSelf::Start,
        Align::End => taffy::AlignSelf::End,
        Align::Center => taffy::AlignSelf::Center,
        Align::Baseline => taffy::AlignSelf::Baseline,
    }
}

fn into_taffy_align_items(align: Align) -> taffy::AlignItems {
    match align {
        Align::Stretch => taffy::AlignItems::Stretch,
        Align::Start => taffy::AlignItems::Start,
        Align::End => taffy::AlignItems::End,
        Align::Center => taffy::AlignItems::Center,
        Align::Baseline => taffy::AlignItems::Baseline,
    }
}

fn into_taffy_justify_content(justify: Justify) -> taffy::JustifyContent {
    match justify {
        Justify::Start => taffy::JustifyContent::Start,
        Justify::End => taffy::JustifyContent::End,
        Justify::Center => taffy::JustifyContent::Center,
        Justify::SpaceBetween => taffy::JustifyContent::SpaceBetween,
        Justify::SpaceAround => taffy::JustifyContent::SpaceAround,
        Justify::SpaceEvenly => taffy::JustifyContent::SpaceEvenly,
    }
}

fn sync_text_input_caret(node: &mut RetainedNode, cursor: usize) {
    if let Some(text_node) = first_text_descendant_mut(node) {
        if let Some(text) = text_node.text.as_mut() {
            text.caret = cursor.min(text.content.len());
        }
    }
}

fn first_text_descendant_mut(node: &mut RetainedNode) -> Option<&mut RetainedNode> {
    if node.text.is_some() {
        return Some(node);
    }
    for child in &mut node.children {
        if let Some(found) = first_text_descendant_mut(child) {
            return Some(found);
        }
    }
    None
}

fn node_mut_from<'a>(node: &'a mut RetainedNode, path: &[usize]) -> Option<&'a mut RetainedNode> {
    let node = resolve_transparent_mut(node);
    if path.is_empty() {
        return Some(node);
    }

    let (index, rest) = path.split_first()?;
    let child = node.children.get_mut(*index)?;
    node_mut_from(child, rest)
}

fn text_node_needs_reflow(node: &RetainedNode) -> bool {
    if node.text.as_ref().is_some_and(|text| text.needs_layout) {
        return true;
    }
    node.children.iter().any(text_node_needs_reflow)
}

fn resolve_transparent_mut(node: &mut RetainedNode) -> &mut RetainedNode {
    if matches!(node.kind, RetainedKind::Component { .. }) && node.children.len() == 1 {
        &mut node.children[0]
    } else {
        node
    }
}

fn node_mut_from_exact<'a>(
    node: &'a mut RetainedNode,
    path: &[usize],
) -> Option<&'a mut RetainedNode> {
    if path.is_empty() {
        return Some(node);
    }
    let (index, rest) = path.split_first()?;
    let child = node.children.get_mut(*index)?;
    node_mut_from_exact(child, rest)
}

/// Traverses a retained subtree using exact child indices without component transparency.
fn node_from_exact<'a>(node: &'a RetainedNode, path: &[usize]) -> Option<&'a RetainedNode> {
    if path.is_empty() {
        return Some(node);
    }
    let (index, rest) = path.split_first()?;
    let child = node.children.get(*index)?;
    node_from_exact(child, rest)
}

/// Collects the layout nodes a retained child contributes directly to its Taffy parent.
fn collect_direct_layout_ids(node: &RetainedNode, child_ids: &mut Vec<NodeId>) {
    if let Some(taffy_id) = node.taffy_id {
        child_ids.push(taffy_id);
        return;
    }
    for child in &node.children {
        collect_direct_layout_ids(child, child_ids);
    }
}

fn split_parent_path(path: &NodePath) -> Result<(NodePath, usize), RetainedError> {
    match path.0.split_last() {
        Some((index, parent)) => Ok((NodePath(parent.to_vec()), *index)),
        None => Err(RetainedError::InvalidNodePath(path.clone())),
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use crate::diff::{NodePath, Patch};
    use crate::element::builders::{Button, Column, Text, View};
    use crate::element::events::{KeyEvent, KeyState, LemonKey, Modifiers, NamedKey};
    use crate::element::style::{Align, Color, Dimension, Edges, Justify, StyleProps};
    use crate::element::types::ComponentElement;

    use super::*;

    #[test]
    fn style_props_convert_to_taffy_style() {
        let style = StyleProps {
            width: Some(Dimension::Points(120.0)),
            height: Some(Dimension::Percent(0.5)),
            padding: Some(Edges::all(8.0)),
            margin: Some(Edges::all(4.0)),
            inset: Some(Edges::all(6.0)),
            gap: Some(12.0),
            flex_grow: Some(1.0),
            flex_shrink: Some(0.0),
            align_items: Some(Align::Center),
            justify_content: Some(Justify::SpaceBetween),
            align_self: Some(Align::Start),
            ..Default::default()
        };

        let taffy_style = style.to_taffy_style();

        assert_eq!(
            taffy_style.size.width,
            taffy::style::Dimension::Length(120.0)
        );
        assert_eq!(
            taffy_style.size.height,
            taffy::style::Dimension::Percent(0.5)
        );
        assert_eq!(
            taffy_style.gap.width,
            taffy::style::LengthPercentage::Length(12.0),
            "gap.width (column_gap) should default to uniform gap"
        );
        assert_eq!(
            taffy_style.gap.height,
            taffy::style::LengthPercentage::Length(12.0),
            "gap.height (row_gap) should default to uniform gap"
        );
        assert_eq!(
            taffy_style.padding.left,
            taffy::style::LengthPercentage::Length(8.0)
        );
        assert_eq!(
            taffy_style.margin.left,
            taffy::style::LengthPercentageAuto::Length(4.0)
        );
        assert_eq!(
            taffy_style.inset.top,
            taffy::style::LengthPercentageAuto::Length(6.0)
        );
        assert_eq!(
            taffy_style.inset.left,
            taffy::style::LengthPercentageAuto::Length(6.0)
        );
        assert_eq!(taffy_style.flex_grow, 1.0);
        assert_eq!(taffy_style.flex_shrink, 0.0);
        assert_eq!(
            taffy_style.align_items,
            Some(taffy::style::AlignItems::Center)
        );
        assert_eq!(
            taffy_style.justify_content,
            Some(taffy::style::JustifyContent::SpaceBetween)
        );
    }

    #[test]
    fn per_axis_gap_overrides_uniform_gap() {
        let style = StyleProps {
            gap: Some(5.0),
            row_gap: Some(10.0),
            column_gap: Some(2.0),
            ..Default::default()
        };
        let taffy_style = style.to_taffy_style();
        assert_eq!(
            taffy_style.gap.height,
            taffy::style::LengthPercentage::Length(10.0),
            "row_gap should override gap for gap.height"
        );
        assert_eq!(
            taffy_style.gap.width,
            taffy::style::LengthPercentage::Length(2.0),
            "column_gap should override gap for gap.width"
        );
    }

    #[test]
    fn retained_node_helpers_expose_text_and_paint_state() {
        let node = RetainedNode::text(
            taffy::NodeId::from(7_u64),
            "hello".to_owned(),
            Default::default(),
            Color::rgb8(255, 0, 0),
        );

        assert_eq!(node.text_content(), Some("hello"));
        assert_eq!(node.paint.background, None);
        assert_eq!(node.taffy_id, Some(taffy::NodeId::from(7_u64)));
    }

    #[test]
    fn mount_builds_retained_tree_for_container_children_and_text() {
        let element = Column::new()
            .gap(6.0)
            .child(Text::new("hello"))
            .child(Text::new("world"))
            .into_element();

        let tree = RetainedTree::mount(element).unwrap();
        let root = tree.root.as_ref().unwrap();

        assert!(matches!(root.kind, RetainedKind::Column));
        assert_eq!(root.children.len(), 2);
        assert_eq!(root.children[0].text_content(), Some("hello"));
        assert_eq!(root.children[1].text_content(), Some("world"));

        let root_id = root.layout_node_id().unwrap();
        let taffy_children = tree.taffy.children(root_id).unwrap();
        assert_eq!(taffy_children.len(), 2);
        assert_eq!(
            taffy_children[0],
            root.children[0].layout_node_id().unwrap()
        );
        assert_eq!(
            taffy_children[1],
            root.children[1].layout_node_id().unwrap()
        );
    }

    #[test]
    fn mount_resolves_paint_and_handlers_for_button_nodes() {
        let fired = Rc::new(Cell::new(false));
        let click = fired.clone();
        let element = Button::new("Press")
            .background(Color::rgb8(10, 20, 30))
            .on_click(move || click.set(true))
            .into_element();

        let tree = RetainedTree::mount(element).unwrap();
        let root = tree.root.as_ref().unwrap();

        assert!(matches!(root.kind, RetainedKind::Button));
        assert_eq!(root.paint.background, Some(Color::rgb8(10, 20, 30)));
        assert_eq!(root.text_content(), Some("Press"));

        let handler = root.handlers.on_click.as_ref().unwrap();
        handler();
        assert!(fired.get());
    }

    #[test]
    fn update_widget_chrome_syncs_caret_to_text_child() {
        use crate::element::types::TextInputMeta;

        let mut tree = RetainedTree::mount(
            View::new()
                .text_input(TextInputMeta {
                    cursor: 0,
                    value: String::new(),
                })
                .child(Text::new(""))
                .into_element(),
        )
        .unwrap();

        tree.apply_patch(Patch::UpdateWidgetChrome {
            node: NodePath::root(),
            text_input: Some(TextInputMeta {
                cursor: 0,
                value: "hi".into(),
            }),
            scroll_viewport: false,
            scroll_bar: false,
        })
        .unwrap();
        tree.apply_patch(Patch::UpdateText {
            node: NodePath(vec![0]),
            content: "hi".into(),
        })
        .unwrap();
        tree.apply_patch(Patch::UpdateWidgetChrome {
            node: NodePath::root(),
            text_input: Some(TextInputMeta {
                cursor: 2,
                value: "hi".into(),
            }),
            scroll_viewport: false,
            scroll_bar: false,
        })
        .unwrap();

        let root = tree.root.as_ref().unwrap();
        let text = &root.children[0];
        assert_eq!(text.text_content(), Some("hi"));
        assert_eq!(text.text.as_ref().unwrap().caret, 2);
    }

    #[test]
    fn update_text_on_component_wrapped_row_reflows_after_layout() {
        use crate::element::builders::{Component, Row};
        use crate::element::types::ComponentElement;
        use crate::layout::{layout_pass, Viewport};

        fn mini(_cx: &crate::runtime::cx::Cx) -> Element {
            Row::new().child(Text::new("0")).into_element()
        }

        let mut tree = RetainedTree::mount(
            Column::new()
                .child(Component::new(mini).key(1))
                .into_element(),
        )
        .unwrap();

        let row = Row::new().child(Text::new("0")).into_element();
        tree.apply_patch(Patch::ReplaceNode {
            node: NodePath(vec![0]),
            new_element: row,
        })
        .unwrap();
        tree.apply_patch(Patch::MountComponent {
            node: NodePath(vec![0]),
            component: ComponentElement::from_component_fn(mini)
                .with_key(crate::element::types::Key(1)),
        })
        .unwrap();
        layout_pass(
            &mut tree,
            Viewport {
                width: 400.0,
                height: 400.0,
            },
        )
        .unwrap();
        assert!(!tree.text_needs_reflow());

        tree.apply_patch(Patch::UpdateText {
            node: NodePath(vec![0, 0]),
            content: "1".to_owned(),
        })
        .unwrap();
        assert!(tree.text_needs_reflow());

        layout_pass(
            &mut tree,
            Viewport {
                width: 400.0,
                height: 400.0,
            },
        )
        .unwrap();
        assert!(
            !tree.text_needs_reflow(),
            "layout_pass should clear needs_layout on nested component text"
        );

        let text = tree
            .root
            .as_ref()
            .unwrap()
            .children
            .first()
            .unwrap()
            .children
            .first()
            .unwrap()
            .children
            .first()
            .unwrap();
        assert_eq!(text.text_content(), Some("1"));
        assert!(text.text.as_ref().unwrap().parley_layout.is_some());
    }

    #[test]
    fn update_text_patch_replaces_content_and_invalidates_layout() {
        let mut tree = RetainedTree::mount(Text::new("before").into_element()).unwrap();
        let root = tree.root.as_mut().unwrap();
        root.text.as_mut().unwrap().needs_layout = false;

        tree.apply_patch(Patch::UpdateText {
            node: NodePath::root(),
            content: "after".to_owned(),
        })
        .unwrap();

        let root = tree.root.as_ref().unwrap();
        assert_eq!(root.text_content(), Some("after"));
        assert!(root.text.as_ref().unwrap().needs_layout);
    }

    #[test]
    fn update_text_style_patch_replaces_style_and_reflows_after_layout() {
        use crate::layout::{layout_pass, Viewport};

        let mut tree =
            RetainedTree::mount(Text::new("preview").font_size(16.0).into_element()).unwrap();
        let first_layout = layout_pass(
            &mut tree,
            Viewport {
                width: 400.0,
                height: 400.0,
            },
        )
        .unwrap();
        let root_id = tree.root.as_ref().unwrap().taffy_id.unwrap();
        let initial_height = first_layout.get(root_id).unwrap().height;
        assert!(!tree.text_needs_reflow());
        assert!(tree
            .root
            .as_ref()
            .unwrap()
            .text
            .as_ref()
            .unwrap()
            .parley_layout
            .is_some());

        let style = TextStyle {
            font_size: 32.0,
            ..Default::default()
        };
        tree.apply_patch(Patch::UpdateTextStyle {
            node: NodePath::root(),
            style,
        })
        .unwrap();

        let root = tree.root.as_ref().unwrap();
        let text = root.text.as_ref().unwrap();
        assert_eq!(text.style.font_size, 32.0);
        assert!(text.needs_layout);
        assert!(text.parley_layout.is_none());

        let second_layout = layout_pass(
            &mut tree,
            Viewport {
                width: 400.0,
                height: 400.0,
            },
        )
        .unwrap();

        let root = tree.root.as_ref().unwrap();
        let text = root.text.as_ref().unwrap();
        assert!(!text.needs_layout);
        assert!(text.parley_layout.is_some());
        assert!(
            second_layout.get(root_id).unwrap().height > initial_height,
            "larger font size should increase measured text height"
        );
    }

    #[test]
    fn insert_and_remove_child_patches_keep_taffy_and_retained_order_in_sync() {
        let mut tree =
            RetainedTree::mount(Column::new().child(Text::new("a")).into_element()).unwrap();

        tree.apply_patch(Patch::InsertChild {
            parent: NodePath::root(),
            index: 1,
            element: Text::new("b").into_element(),
        })
        .unwrap();

        let root = tree.root.as_ref().unwrap();
        assert_eq!(root.children.len(), 2);
        assert_eq!(root.children[1].text_content(), Some("b"));
        assert_eq!(
            tree.taffy.children(root.layout_node_id().unwrap()).unwrap()[1],
            root.children[1].layout_node_id().unwrap()
        );

        tree.apply_patch(Patch::RemoveChild {
            parent: NodePath::root(),
            index: 0,
        })
        .unwrap();

        let root = tree.root.as_ref().unwrap();
        assert_eq!(root.children.len(), 1);
        assert_eq!(root.children[0].text_content(), Some("b"));
        assert_eq!(
            tree.taffy.children(root.layout_node_id().unwrap()).unwrap()[0],
            root.children[0].layout_node_id().unwrap()
        );
    }

    #[test]
    fn replace_node_patch_rebuilds_subtree_at_same_index() {
        let mut tree = RetainedTree::mount(
            Column::new()
                .child(Text::new("old"))
                .child(Text::new("keep"))
                .into_element(),
        )
        .unwrap();
        let old_child_id = tree.root.as_ref().unwrap().children[0]
            .layout_node_id()
            .unwrap();

        tree.apply_patch(Patch::ReplaceNode {
            node: NodePath(vec![0]),
            new_element: Column::new().child(Text::new("new")).into_element(),
        })
        .unwrap();

        let root = tree.root.as_ref().unwrap();
        assert_ne!(root.children[0].layout_node_id().unwrap(), old_child_id);
        assert!(matches!(root.children[0].kind, RetainedKind::Column));
        assert_eq!(root.children[0].children[0].text_content(), Some("new"));
        assert_eq!(root.children[1].text_content(), Some("keep"));
    }

    #[test]
    fn fragment_children_mount_into_retained_tree() {
        use crate::element::Element;

        let tree = RetainedTree::mount(
            Column::new()
                .child(Element::Fragment(vec![
                    Text::new("a").into_element(),
                    Text::new("b").into_element(),
                ]))
                .into_element(),
        )
        .expect("fragment children should mount");

        let root = tree.root.as_ref().expect("root");
        assert_eq!(root.children.len(), 2);
        assert_eq!(root.children[0].text_content(), Some("a"));
        assert_eq!(root.children[1].text_content(), Some("b"));
    }

    #[test]
    fn mount_component_patch_creates_transparent_wrapper_without_taffy_node() {
        fn child(_cx: &crate::runtime::cx::Cx) -> Element {
            Text::new("child").into_element()
        }

        let mut tree = RetainedTree::mount(Text::new("root").into_element()).unwrap();
        tree.apply_patch(Patch::MountComponent {
            node: NodePath::root(),
            component: ComponentElement::from_component_fn(child)
                .with_key(crate::element::types::Key(5)),
        })
        .unwrap();

        let root = tree.root.as_ref().unwrap();
        assert!(matches!(root.kind, RetainedKind::Component { .. }));
        assert!(root.taffy_id.is_none());
        assert_eq!(root.children[0].text_content(), Some("root"));
    }

    #[test]
    fn mount_component_after_replace_node_keeps_sibling_rows_in_taffy_tree() {
        use crate::element::builders::{Column, Component, Row, Text};
        use crate::runtime::Runtime;
        use crate::Cx;

        fn mini(_cx: &Cx) -> Element {
            Row::new().child(Text::new("0")).into_element()
        }

        let mut runtime = Runtime::new();
        runtime.mount(|_cx| {
            Column::new()
                .child(Component::new(mini).key(1))
                .child(Component::new(mini).key(2))
                .into_element()
        });
        let mut tree = RetainedTree::mount(runtime.root_element().expect("root")).unwrap();
        tree.apply_patches(runtime.take_patches())
            .expect("bootstrap");

        let column_id = tree.root.as_ref().unwrap().taffy_id.expect("column");
        for (index, label) in [(0usize, "first"), (1, "second")] {
            let row = tree.root.as_ref().unwrap().children[index]
                .children
                .first()
                .and_then(|node| node.layout_node_id())
                .expect("row");
            assert_eq!(
                tree.taffy.parent(row),
                Some(column_id),
                "{label} component row must remain attached in Taffy"
            );
        }
    }

    #[test]
    fn component_wrapper_is_transparent_to_child_lookup() {
        let wrapper = RetainedNode {
            kind: RetainedKind::Component {
                type_id: std::any::TypeId::of::<u32>(),
                key: Some(crate::element::types::Key(1)),
            },
            taffy_id: None,
            style: Default::default(),
            paint: Default::default(),
            children: vec![RetainedNode::text(
                taffy::NodeId::from(9_u64),
                "child".to_owned(),
                Default::default(),
                Default::default(),
            )],
            handlers: Default::default(),
            text: None,
            text_input: None,
            scroll_viewport: false,
            scroll_bar: false,
        };

        assert_eq!(wrapper.children[0].text_content(), Some("child"));
    }

    #[test]
    fn update_component_patch_updates_wrapper_metadata() {
        fn child_a(_cx: &crate::runtime::cx::Cx) -> Element {
            Text::new("a").into_element()
        }

        fn child_b(_cx: &crate::runtime::cx::Cx) -> Element {
            Text::new("b").into_element()
        }

        let mut tree = RetainedTree::mount(Text::new("root").into_element()).unwrap();
        tree.apply_patch(Patch::MountComponent {
            node: NodePath::root(),
            component: ComponentElement::from_component_fn(child_a)
                .with_key(crate::element::types::Key(1)),
        })
        .unwrap();

        tree.apply_patch(Patch::UpdateComponent {
            node: NodePath::root(),
            component: ComponentElement::from_component_fn(child_b)
                .with_key(crate::element::types::Key(2)),
        })
        .unwrap();

        let root = tree.root.as_ref().unwrap();
        let RetainedKind::Component { key, .. } = &root.kind else {
            panic!("expected component wrapper");
        };
        assert_eq!(key.as_ref(), Some(&crate::element::types::Key(2)));
    }

    #[test]
    fn unmount_component_promotes_wrapped_subtree() {
        fn child(_cx: &crate::runtime::cx::Cx) -> Element {
            Text::new("child").into_element()
        }

        let mut tree = RetainedTree::mount(Text::new("root").into_element()).unwrap();
        tree.apply_patch(Patch::MountComponent {
            node: NodePath::root(),
            component: ComponentElement::from_component_fn(child)
                .with_key(crate::element::types::Key(1)),
        })
        .unwrap();
        tree.apply_patch(Patch::UnmountComponent {
            node: NodePath::root(),
        })
        .unwrap();

        let root = tree.root.as_ref().unwrap();
        assert!(matches!(root.kind, RetainedKind::Text));
        assert_eq!(root.text_content(), Some("root"));
    }

    #[test]
    fn box_element_hover_handlers_are_applied_to_retained_node() {
        let entered = Rc::new(Cell::new(false));
        let left = Rc::new(Cell::new(false));
        let e = entered.clone();
        let l = left.clone();

        let element = View::new()
            .width(100.0)
            .height(100.0)
            .on_hover_enter(move || e.set(true))
            .on_hover_leave(move || l.set(true))
            .into_element();

        let tree = RetainedTree::mount(element).unwrap();
        let root = tree.root.as_ref().unwrap();
        assert!(root.handlers.on_hover_enter.is_some());
        assert!(root.handlers.on_hover_leave.is_some());

        root.handlers.on_hover_enter.as_ref().unwrap()();
        assert!(entered.get());
        root.handlers.on_hover_leave.as_ref().unwrap()();
        assert!(left.get());
    }

    #[test]
    fn box_element_key_down_handler_is_applied_to_retained_node() {
        use std::cell::RefCell;

        let received = Rc::new(RefCell::new(None::<LemonKey>));
        let r = received.clone();

        let element = View::new()
            .width(100.0)
            .height(100.0)
            .focusable()
            .on_key_down(move |ev: KeyEvent| {
                *r.borrow_mut() = Some(ev.key.clone());
            })
            .into_element();

        let tree = RetainedTree::mount(element).unwrap();
        let root = tree.root.as_ref().unwrap();
        assert!(root.style.focusable);
        assert!(root.handlers.on_key_down.is_some());

        root.handlers.on_key_down.as_ref().unwrap()(KeyEvent {
            key: LemonKey::Named(NamedKey::Enter),
            modifiers: Modifiers::default(),
            repeat: false,
            state: KeyState::Pressed,
        });
        assert_eq!(*received.borrow(), Some(LemonKey::Named(NamedKey::Enter)));
    }

    #[test]
    fn box_on_scroll_handler_is_stored_in_retained_node() {
        use std::cell::Cell;
        let delta_received = Rc::new(Cell::new(0.0f64));
        let d = delta_received.clone();

        let tree = RetainedTree::mount(
            View::new()
                .width(200.0)
                .height(150.0)
                .on_scroll(move |delta| d.set(delta))
                .into_element(),
        )
        .unwrap();

        let root = tree.root.as_ref().unwrap();
        assert!(root.handlers.on_scroll.is_some());
        root.handlers.on_scroll.as_ref().unwrap()(42.0);
        assert_eq!(delta_received.get(), 42.0);
    }

    #[test]
    fn box_on_click_outside_handler_is_stored_in_retained_node() {
        use std::cell::Cell;

        let fired = Rc::new(Cell::new(false));
        let f = fired.clone();

        let tree = RetainedTree::mount(
            View::new()
                .width(100.0)
                .height(100.0)
                .on_click_outside(move || f.set(true))
                .into_element(),
        )
        .unwrap();

        let root = tree.root.as_ref().unwrap();
        assert!(root.handlers.on_click_outside.is_some());
        root.handlers.on_click_outside.as_ref().unwrap()();
        assert!(fired.get());
    }

    #[test]
    fn box_on_pointer_down_and_move_handlers_are_stored_in_retained_node() {
        use std::cell::Cell;

        let down_x = Rc::new(Cell::new(0.0f32));
        let move_x = Rc::new(Cell::new(0.0f32));
        let dx = down_x.clone();
        let mx = move_x.clone();

        let tree = RetainedTree::mount(
            View::new()
                .width(200.0)
                .height(50.0)
                .on_pointer_down(move |frac_x, _| dx.set(frac_x))
                .on_pointer_move(move |frac_x, _| mx.set(frac_x))
                .into_element(),
        )
        .unwrap();

        let root = tree.root.as_ref().unwrap();
        assert!(root.handlers.on_pointer_down.is_some());
        assert!(root.handlers.on_pointer_move.is_some());

        root.handlers.on_pointer_down.as_ref().unwrap()(0.5, 0.5);
        assert_eq!(down_x.get(), 0.5);
        root.handlers.on_pointer_move.as_ref().unwrap()(0.75, 0.25);
        assert_eq!(move_x.get(), 0.75);
    }
}
