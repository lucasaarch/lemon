use vello::kurbo::RoundedRect;
use vello::peniko::{color::AlphaColor, Color as PenikoColor, Fill};
use vello::Scene;

use crate::element::style::{Color, CornerRadii};
use crate::layout::{LayoutMap, LayoutRect};
use crate::retained::{RetainedKind, RetainedNode, RetainedTree};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct PaintStats {
    pub fills: u32,
    pub strokes: u32,
}

/// Walk the retained tree in pre-order and emit Vello draw commands.
pub fn paint_pass(
    tree: &RetainedTree,
    layout: &LayoutMap,
    scene: &mut Scene,
    _scale_factor: f32,
) -> PaintStats {
    let mut stats = PaintStats::default();
    if let Some(root) = tree.root.as_ref() {
        paint_node(root, layout, scene, &mut stats);
    }
    stats
}

fn paint_node(
    node: &RetainedNode,
    layout: &LayoutMap,
    scene: &mut Scene,
    stats: &mut PaintStats,
) {
    if matches!(node.kind, RetainedKind::Component { .. }) {
        for child in &node.children {
            paint_node(child, layout, scene, stats);
        }
        return;
    }

    let Some(taffy_id) = node.taffy_id else {
        for child in &node.children {
            paint_node(child, layout, scene, stats);
        }
        return;
    };

    let Some(rect) = layout.get(taffy_id) else {
        return;
    };

    paint_container_background(node, rect, scene, stats);

    for child in &node.children {
        paint_node(child, layout, scene, stats);
    }
}

fn paint_container_background(
    node: &RetainedNode,
    rect: &LayoutRect,
    scene: &mut Scene,
    stats: &mut PaintStats,
) {
    let is_container = matches!(
        node.kind,
        RetainedKind::Box | RetainedKind::Row | RetainedKind::Column | RetainedKind::Button
    );
    if !is_container {
        return;
    }

    if let Some(background) = node.paint.background {
        fill_rounded_rect(scene, rect, &node.paint.radius, background, stats);
    }
}

fn fill_rounded_rect(
    scene: &mut Scene,
    rect: &LayoutRect,
    radii: &CornerRadii,
    color: Color,
    stats: &mut PaintStats,
) {
    let shape = layout_rect_to_rounded(rect, radii);
    scene.fill(
        Fill::NonZero,
        vello::kurbo::Affine::IDENTITY,
        to_peniko_color(color),
        None,
        &shape,
    );
    stats.fills += 1;
}

fn layout_rect_to_rounded(rect: &LayoutRect, radii: &CornerRadii) -> RoundedRect {
    RoundedRect::new(
        f64::from(rect.x),
        f64::from(rect.y),
        f64::from(rect.x + rect.width),
        f64::from(rect.y + rect.height),
        (
            f64::from(radii.top_left),
            f64::from(radii.top_right),
            f64::from(radii.bottom_right),
            f64::from(radii.bottom_left),
        ),
    )
}

fn to_peniko_color(color: Color) -> PenikoColor {
    AlphaColor::new([color.r, color.g, color.b, color.a])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::builders::Column;
    use crate::layout::{layout_pass, Viewport};
    use crate::retained::RetainedTree;

    #[test]
    fn column_with_background_emits_fill() {
        let mut tree = RetainedTree::mount(
            Column::new()
                .width(100.0)
                .height(80.0)
                .background(Color::rgb8(40, 80, 120))
                .into_element(),
        )
        .unwrap();

        let layout = layout_pass(
            &mut tree,
            Viewport {
                width: 200.0,
                height: 200.0,
            },
            1.0,
        )
        .unwrap();

        let mut scene = Scene::new();
        let stats = paint_pass(&tree, &layout, &mut scene, 1.0);

        assert_eq!(stats.fills, 1);
        assert_eq!(stats.strokes, 0);
    }

    #[test]
    fn missing_layout_entry_skips_subtree_without_panic() {
        let tree = RetainedTree::mount(
            Column::new()
                .background(Color::rgb8(10, 20, 30))
                .into_element(),
        )
        .unwrap();

        let mut scene = Scene::new();
        let stats = paint_pass(&tree, &LayoutMap::default(), &mut scene, 1.0);

        assert_eq!(stats.fills, 0);
    }

    #[test]
    fn component_wrapper_is_transparent_to_paint() {
        use crate::diff::{NodePath, Patch};
        use crate::element::builders::Text;
        use crate::element::types::ComponentElement;

        fn child(_cx: &crate::runtime::cx::Cx) -> crate::element::Element {
            Text::new("child").into_element()
        }

        let mut tree = RetainedTree::mount(
            Column::new()
                .width(100.0)
                .height(80.0)
                .background(Color::rgb8(200, 100, 50))
                .child(Text::new("label"))
                .into_element(),
        )
        .unwrap();

        tree.apply_patch(Patch::MountComponent {
            node: NodePath(vec![0]),
            component: ComponentElement::from_component_fn(child)
                .with_key(crate::element::types::Key(1)),
        })
        .unwrap();

        let layout = layout_pass(
            &mut tree,
            Viewport {
                width: 200.0,
                height: 200.0,
            },
            1.0,
        )
        .unwrap();

        let mut scene = Scene::new();
        let stats = paint_pass(&tree, &layout, &mut scene, 1.0);

        assert_eq!(stats.fills, 1, "only the column background is painted");
    }
}
