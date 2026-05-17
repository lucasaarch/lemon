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
    InvalidNodePath(NodePath),
    MissingTextCache(NodePath),
}

impl From<taffy::TaffyError> for RetainedError {
    fn from(value: taffy::TaffyError) -> Self {
        Self::Taffy(value)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextCache {
    pub content: String,
    pub style: TextStyle,
    pub needs_layout: bool,
}

#[derive(Clone, Default)]
pub struct EventHandlers {
    pub on_click: Option<Rc<dyn Fn()>>,
}

impl std::fmt::Debug for EventHandlers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventHandlers")
            .field("on_click", &self.on_click.as_ref().map(|_| "Rc<dyn Fn()>"))
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum RetainedKind {
    Box,
    Row,
    Column,
    Text,
    Button,
    Image { src: String },
}

#[derive(Clone, Debug)]
pub struct RetainedNode {
    pub kind: RetainedKind,
    pub taffy_id: NodeId,
    pub style: StyleProps,
    pub paint: PaintData,
    pub children: Vec<RetainedNode>,
    pub handlers: EventHandlers,
    pub text: Option<TextCache>,
}

impl RetainedNode {
    pub fn text(taffy_id: NodeId, content: String, mut style: TextStyle, color: Color) -> Self {
        style.color = Some(color);

        Self {
            kind: RetainedKind::Text,
            taffy_id,
            style: StyleProps::default(),
            paint: PaintData {
                background: None,
                border_color: None,
                border_width: 0.0,
                radius: CornerRadii::default(),
            },
            children: Vec::new(),
            handlers: EventHandlers::default(),
            text: Some(TextCache {
                content,
                style,
                needs_layout: true,
            }),
        }
    }

    pub fn text_content(&self) -> Option<&str> {
        self.text.as_ref().map(|text| text.content.as_str())
    }
}

#[derive(Debug)]
pub struct RetainedTree {
    pub taffy: TaffyTree<()>,
    pub root: Option<RetainedNode>,
}

impl RetainedTree {
    pub fn new() -> Self {
        Self {
            taffy: TaffyTree::new(),
            root: None,
        }
    }

    pub fn mount(element: Element) -> Result<Self, RetainedError> {
        let mut tree = Self::new();
        let root = tree.build_node(element)?;
        tree.root = Some(root);
        Ok(tree)
    }

    fn build_node(&mut self, element: Element) -> Result<RetainedNode, RetainedError> {
        match element {
            Element::Box_(node) => self.build_box_node(RetainedKind::Box, node),
            Element::Row(node) => self.build_box_node(RetainedKind::Row, node),
            Element::Column(node) => self.build_box_node(RetainedKind::Column, node),
            Element::Text(node) => self.build_text_node(node),
            Element::Button(node) => self.build_button_node(node),
            Element::Image(node) => self.build_image_node(node),
            Element::Component(_) => Err(RetainedError::UnsupportedElement("Component")),
            Element::Fragment(_) => Err(RetainedError::UnsupportedElement("Fragment")),
            Element::None => Err(RetainedError::UnsupportedElement("None")),
        }
    }

    fn build_box_node(
        &mut self,
        kind: RetainedKind,
        node: BoxElement,
    ) -> Result<RetainedNode, RetainedError> {
        let mut children = Vec::with_capacity(node.children.len());
        for child in node.children {
            children.push(self.build_node(child)?);
        }

        let mut style = node.style.to_taffy_style();
        style.flex_direction = match kind {
            RetainedKind::Column => taffy::FlexDirection::Column,
            RetainedKind::Row => taffy::FlexDirection::Row,
            _ => style.flex_direction,
        };

        let child_ids: Vec<_> = children.iter().map(|child| child.taffy_id).collect();
        let taffy_id = self.taffy.new_with_children(style, &child_ids)?;

        Ok(RetainedNode {
            kind,
            taffy_id,
            style: node.style,
            paint: node.paint.resolve(),
            children,
            handlers: EventHandlers::default(),
            text: None,
        })
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

        Ok(RetainedNode {
            kind: RetainedKind::Button,
            taffy_id,
            style: node.style,
            paint: node.paint.resolve(),
            children: Vec::new(),
            handlers: EventHandlers {
                on_click: node.on_click,
            },
            text: Some(TextCache {
                content: label,
                style: TextStyle::default(),
                needs_layout: true,
            }),
        })
    }

    fn build_image_node(&mut self, node: ImageElement) -> Result<RetainedNode, RetainedError> {
        let taffy_id = self.taffy.new_leaf(node.style.to_taffy_style())?;

        Ok(RetainedNode {
            kind: RetainedKind::Image { src: node.src },
            taffy_id,
            style: node.style,
            paint: PaintData::default(),
            children: Vec::new(),
            handlers: EventHandlers::default(),
            text: None,
        })
    }

    pub fn apply_patch(&mut self, patch: Patch) -> Result<(), RetainedError> {
        match patch {
            Patch::UpdateStyle { node, style } => {
                let taffy_id = self.node_mut(&node)?.taffy_id;
                self.taffy.set_style(taffy_id, style.to_taffy_style())?;
                self.node_mut(&node)?.style = style;
            }
            Patch::UpdatePaint { node, paint } => {
                self.node_mut(&node)?.paint = paint;
            }
            Patch::UpdateText { node, content } => {
                let retained = self.node_mut(&node)?;
                let text = retained
                    .text
                    .as_mut()
                    .ok_or_else(|| RetainedError::MissingTextCache(node.clone()))?;
                text.content = content;
                text.needs_layout = true;
            }
            Patch::InsertChild {
                parent,
                index,
                element,
            } => {
                let child = self.build_node(element)?;
                let parent_id = self.node_mut(&parent)?.taffy_id;
                self.taffy
                    .insert_child_at_index(parent_id, index, child.taffy_id)?;
                self.node_mut(&parent)?.children.insert(index, child);
            }
            Patch::RemoveChild { parent, index } => {
                let parent_id = self.node_mut(&parent)?.taffy_id;
                let removed = self.node_mut(&parent)?.children.remove(index);
                self.taffy.remove_child_at_index(parent_id, index)?;
                self.remove_subtree_from_taffy(removed)?;
            }
            Patch::ReplaceNode { node, new_element } => {
                self.replace_node(node, new_element)?;
            }
            Patch::MoveChild { parent, from, to } => {
                self.move_child(parent, from, to)?;
            }
        }

        Ok(())
    }

    fn node_mut(&mut self, path: &NodePath) -> Result<&mut RetainedNode, RetainedError> {
        let root = self
            .root
            .as_mut()
            .ok_or_else(|| RetainedError::InvalidNodePath(path.clone()))?;
        node_mut_from(root, &path.0).ok_or_else(|| RetainedError::InvalidNodePath(path.clone()))
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
        let parent_id = self.node_mut(&parent_path)?.taffy_id;
        let removed = self.node_mut(&parent_path)?.children.remove(index);
        self.taffy.remove_child_at_index(parent_id, index)?;
        self.remove_subtree_from_taffy(removed)?;
        self.taffy
            .insert_child_at_index(parent_id, index, replacement.taffy_id)?;
        self.node_mut(&parent_path)?
            .children
            .insert(index, replacement);
        Ok(())
    }

    fn move_child(
        &mut self,
        parent: NodePath,
        from: usize,
        to: usize,
    ) -> Result<(), RetainedError> {
        let parent_id = self.node_mut(&parent)?.taffy_id;
        let child_ids = {
            let parent_node = self.node_mut(&parent)?;
            let child = parent_node.children.remove(from);
            parent_node.children.insert(to, child);
            parent_node
                .children
                .iter()
                .map(|child| child.taffy_id)
                .collect::<Vec<_>>()
        };
        self.taffy.set_children(parent_id, &child_ids)?;
        Ok(())
    }

    fn remove_subtree_from_taffy(&mut self, node: RetainedNode) -> Result<(), RetainedError> {
        for child in node.children {
            self.remove_subtree_from_taffy(child)?;
        }
        self.taffy.remove(node.taffy_id)?;
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
            gap: Size {
                width: self
                    .gap
                    .map_or(taffy::LengthPercentage::Length(0.0), |value| {
                        taffy::LengthPercentage::Length(value)
                    }),
                height: self
                    .gap
                    .map_or(taffy::LengthPercentage::Length(0.0), |value| {
                        taffy::LengthPercentage::Length(value)
                    }),
            },
            align_items: self.align_items.clone().map(into_taffy_align_items),
            justify_content: self.justify_content.clone().map(into_taffy_justify_content),
            flex_grow: self.flex_grow.unwrap_or(0.0),
            flex_shrink: self.flex_shrink.unwrap_or(1.0),
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

fn node_mut_from<'a>(node: &'a mut RetainedNode, path: &[usize]) -> Option<&'a mut RetainedNode> {
    if path.is_empty() {
        return Some(node);
    }

    let (index, rest) = path.split_first()?;
    let child = node.children.get_mut(*index)?;
    node_mut_from(child, rest)
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
    use crate::element::builders::{Button, Column, Text};
    use crate::element::style::{Align, Color, Dimension, Edges, Justify, StyleProps};

    use super::*;

    #[test]
    fn style_props_convert_to_taffy_style() {
        let style = StyleProps {
            width: Some(Dimension::Points(120.0)),
            height: Some(Dimension::Percent(0.5)),
            padding: Some(Edges::all(8.0)),
            margin: Some(Edges::all(4.0)),
            gap: Some(12.0),
            flex_grow: Some(1.0),
            flex_shrink: Some(0.0),
            align_items: Some(Align::Center),
            justify_content: Some(Justify::SpaceBetween),
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
            taffy::style::LengthPercentage::Length(12.0)
        );
        assert_eq!(
            taffy_style.padding.left,
            taffy::style::LengthPercentage::Length(8.0)
        );
        assert_eq!(
            taffy_style.margin.left,
            taffy::style::LengthPercentageAuto::Length(4.0)
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
    fn retained_node_helpers_expose_text_and_paint_state() {
        let node = RetainedNode::text(
            taffy::NodeId::from(7_u64),
            "hello".to_owned(),
            Default::default(),
            Color::rgb8(255, 0, 0),
        );

        assert_eq!(node.text_content(), Some("hello"));
        assert_eq!(node.paint.background, None);
        assert_eq!(node.taffy_id, taffy::NodeId::from(7_u64));
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

        let taffy_children = tree.taffy.children(root.taffy_id).unwrap();
        assert_eq!(taffy_children.len(), 2);
        assert_eq!(taffy_children[0], root.children[0].taffy_id);
        assert_eq!(taffy_children[1], root.children[1].taffy_id);
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
            tree.taffy.children(root.taffy_id).unwrap()[1],
            root.children[1].taffy_id
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
            tree.taffy.children(root.taffy_id).unwrap()[0],
            root.children[0].taffy_id
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
        let old_child_id = tree.root.as_ref().unwrap().children[0].taffy_id;

        tree.apply_patch(Patch::ReplaceNode {
            node: NodePath(vec![0]),
            new_element: Column::new().child(Text::new("new")).into_element(),
        })
        .unwrap();

        let root = tree.root.as_ref().unwrap();
        assert_ne!(root.children[0].taffy_id, old_child_id);
        assert!(matches!(root.children[0].kind, RetainedKind::Column));
        assert_eq!(root.children[0].children[0].text_content(), Some("new"));
        assert_eq!(root.children[1].text_content(), Some("keep"));
    }
}
