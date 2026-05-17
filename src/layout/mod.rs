use std::collections::HashMap;

use taffy::geometry::{Point, Size};
use taffy::{AvailableSpace, NodeId};

use crate::retained::{RetainedError, RetainedKind, RetainedNode, RetainedTree};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutMap {
    rects: HashMap<NodeId, LayoutRect>,
}

impl LayoutMap {
    pub fn get(&self, node_id: NodeId) -> Option<&LayoutRect> {
        self.rects.get(&node_id)
    }

    pub fn len(&self) -> usize {
        self.rects.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rects.is_empty()
    }
}

/// Viewport size in logical points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    pub width: f32,
    pub height: f32,
}

/// Run Taffy layout on `tree` and collect absolute logical rects keyed by `NodeId`.
pub fn layout_pass(
    tree: &mut RetainedTree,
    viewport: Viewport,
    scale_factor: f32,
) -> Result<LayoutMap, RetainedError> {
    let root_node = tree
        .root
        .as_ref()
        .ok_or(RetainedError::UnsupportedElement("empty retained tree"))?;
    let root_id = root_node
        .layout_node_id()
        .ok_or(RetainedError::UnsupportedElement("root without layout node"))?;

    let mut measure_by_id: HashMap<NodeId, MeasureInfo> = HashMap::new();
    collect_measure_info(root_node, &mut measure_by_id);

    let available_space = Size {
        width: AvailableSpace::Definite(viewport.width),
        height: AvailableSpace::Definite(viewport.height),
    };

    tree.taffy.compute_layout_with_measure(
        root_id,
        available_space,
        |known_dimensions, available_space, node_id, _, _| {
            measure_node(
                measure_by_id.get(&node_id),
                known_dimensions,
                available_space,
                scale_factor,
            )
        },
    )?;

    let mut map = LayoutMap::default();
    collect_layouts(&tree.taffy, root_node, Point::ZERO, &mut map);
    Ok(map)
}

#[derive(Clone)]
struct MeasureInfo {
    content: String,
    font_size: f32,
}

fn collect_measure_info(node: &RetainedNode, map: &mut HashMap<NodeId, MeasureInfo>) {
    if let (Some(id), Some(text)) = (node.taffy_id, node.text.as_ref()) {
        map.insert(
            id,
            MeasureInfo {
                content: text.content.clone(),
                font_size: text.style.font_size,
            },
        );
    }
    for child in &node.children {
        collect_measure_info(child, map);
    }
}

/// Placeholder text measurement until Parley integration (layout pass Task 2).
fn measure_node(
    info: Option<&MeasureInfo>,
    known_dimensions: Size<Option<f32>>,
    available_space: Size<AvailableSpace>,
    _scale_factor: f32,
) -> Size<f32> {
    if let (Some(width), Some(height)) = (known_dimensions.width, known_dimensions.height) {
        return Size { width, height };
    }

    let Some(info) = info else {
        return Size::ZERO;
    };

    let font_size = if info.font_size > 0.0 {
        info.font_size
    } else {
        16.0
    };
    let line_height = font_size * 1.2;
    let char_width = font_size * 0.55;
    let content_width = info.content.chars().count() as f32 * char_width;

    let width = known_dimensions.width.unwrap_or_else(|| {
        available_space
            .width
            .into_option()
            .map(|space| space.min(content_width))
            .unwrap_or(content_width)
    });
    let height = known_dimensions.height.unwrap_or(line_height);

    Size { width, height }
}

fn collect_layouts(
    taffy: &taffy::TaffyTree<()>,
    node: &RetainedNode,
    offset: Point<f32>,
    map: &mut LayoutMap,
) {
    if matches!(node.kind, RetainedKind::Component { .. }) {
        for child in &node.children {
            collect_layouts(taffy, child, offset, map);
        }
        return;
    }

    let Some(taffy_id) = node.taffy_id else {
        for child in &node.children {
            collect_layouts(taffy, child, offset, map);
        }
        return;
    };

    let layout = taffy.layout(taffy_id).expect("layout exists after compute_layout");
    let abs = LayoutRect {
        x: offset.x + layout.location.x,
        y: offset.y + layout.location.y,
        width: layout.size.width,
        height: layout.size.height,
    };
    map.rects.insert(taffy_id, abs);

    let child_offset = Point {
        x: abs.x,
        y: abs.y,
    };
    for child in &node.children {
        collect_layouts(taffy, child, child_offset, map);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::{NodePath, Patch};
    use crate::element::builders::{Column, Text};
    use crate::element::types::ComponentElement;

    #[test]
    fn column_children_stack_vertically() {
        let mut tree = RetainedTree::mount(
            Column::new()
                .gap(8.0)
                .child(Text::new("hi"))
                .child(Text::new("hello world"))
                .into_element(),
        )
        .unwrap();

        let map = layout_pass(&mut tree, Viewport { width: 400.0, height: 600.0 }, 1.0).unwrap();

        let root = tree.root.as_ref().unwrap();
        let short_id = root.children[0].taffy_id.unwrap();
        let long_id = root.children[1].taffy_id.unwrap();

        let short = map.get(short_id).unwrap();
        let long = map.get(long_id).unwrap();

        assert!(long.y > short.y);
        assert!(short.width > 0.0 && short.height > 0.0);
        assert!(long.width > 0.0 && long.height > 0.0);
        assert_eq!(short.width, long.width, "column stretches children to same width");
    }

    #[test]
    fn component_wrapper_is_transparent_in_layout_collection() {
        fn child(_cx: &crate::runtime::cx::Cx) -> crate::element::Element {
            Text::new("child").into_element()
        }

        let mut tree = RetainedTree::mount(Text::new("root").into_element()).unwrap();
        tree.apply_patch(Patch::MountComponent {
            node: NodePath::root(),
            component: ComponentElement::from_component_fn(child)
                .with_key(crate::element::types::Key(1)),
        })
        .unwrap();

        let map = layout_pass(&mut tree, Viewport { width: 200.0, height: 200.0 }, 1.0).unwrap();

        let root = tree.root.as_ref().unwrap();
        assert!(matches!(root.kind, RetainedKind::Component { .. }));
        assert!(root.taffy_id.is_none());

        let text_id = root.children[0].taffy_id.unwrap();
        let rect = map.get(text_id).unwrap();
        assert!(rect.width > 0.0 && rect.height > 0.0);
    }
}
