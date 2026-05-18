use parley::PositionedLayoutItem;
use vello::kurbo::{Affine, Rect, RoundedRect, Stroke};
use vello::peniko::{color::AlphaColor, BlendMode, Color as PenikoColor, Fill};
use vello::{Glyph, Scene};

use crate::element::style::{default_text_color, Color, CornerRadii, Edges};
use crate::layout::{LayoutMap, LayoutRect};
use crate::retained::{RetainedKind, RetainedNode, RetainedTree};

type ParleyLayout = parley::Layout<[u8; 4]>;

/// Clip bounds covering all logical content for the HiDPI root layer.
fn clip_everything() -> Rect {
    Rect::new(-1e9, -1e9, 1e9, 1e9)
}

#[derive(Clone, Copy)]
struct PaintContext {
    /// Global scale applied to all logical coordinates (HiDPI).
    base: Affine,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct PaintStats {
    pub fills: u32,
    pub strokes: u32,
    pub glyph_runs: u32,
}

/// Walk the retained tree in pre-order and emit Vello draw commands.
///
/// Drawing uses logical points; `scale_factor` is applied via a root transform on
/// all emitted geometry (and a matching root layer per the architecture spec).
pub fn paint_pass(
    tree: &RetainedTree,
    layout: &LayoutMap,
    scene: &mut Scene,
    scale_factor: f32,
) -> PaintStats {
    let mut stats = PaintStats::default();
    let base = if scale_factor == 1.0 {
        Affine::IDENTITY
    } else {
        Affine::scale(f64::from(scale_factor))
    };
    let ctx = PaintContext { base };

    scene.push_layer(
        Fill::NonZero,
        BlendMode::default(),
        1.0,
        base,
        &clip_everything(),
    );

    if let Some(root) = tree.root.as_ref() {
        paint_node(root, layout, scene, ctx, &mut stats);
    }

    scene.pop_layer();
    stats
}

fn paint_node(
    node: &RetainedNode,
    layout: &LayoutMap,
    scene: &mut Scene,
    ctx: PaintContext,
    stats: &mut PaintStats,
) {
    if matches!(node.kind, RetainedKind::Component { .. }) {
        for child in &node.children {
            paint_node(child, layout, scene, ctx, stats);
        }
        return;
    }

    let Some(taffy_id) = node.taffy_id else {
        for child in &node.children {
            paint_node(child, layout, scene, ctx, stats);
        }
        return;
    };

    let Some(rect) = layout.get(taffy_id) else {
        return;
    };

    paint_container(node, rect, scene, ctx, stats);
    paint_text(node, rect, scene, ctx, stats);

    for child in &node.children {
        paint_node(child, layout, scene, ctx, stats);
    }
}

fn paint_container(
    node: &RetainedNode,
    rect: &LayoutRect,
    scene: &mut Scene,
    ctx: PaintContext,
    stats: &mut PaintStats,
) {
    let is_container = matches!(
        node.kind,
        RetainedKind::Box | RetainedKind::Row | RetainedKind::Column | RetainedKind::Button
    );
    if !is_container {
        return;
    }

    let shape = layout_rect_to_rounded(rect, &node.paint.radius);

    if let Some(background) = node.paint.background {
        fill_rounded_rect(scene, ctx, &shape, background, stats);
    }

    if let Some(border_color) = node.paint.border_color {
        if node.paint.border_width > 0.0 {
            stroke_rounded_rect(
                scene,
                ctx,
                &shape,
                border_color,
                f64::from(node.paint.border_width),
                stats,
            );
        }
    }
}

fn paint_text(
    node: &RetainedNode,
    rect: &LayoutRect,
    scene: &mut Scene,
    ctx: PaintContext,
    stats: &mut PaintStats,
) {
    let is_text = matches!(node.kind, RetainedKind::Text | RetainedKind::Button);
    if !is_text {
        return;
    }

    let Some(text) = node.text.as_ref() else {
        return;
    };
    let Some(layout) = text.parley_layout.as_ref() else {
        return;
    };

    let color = text.style.color.unwrap_or_else(default_text_color);
    let (origin_x, origin_y) = if matches!(node.kind, RetainedKind::Button) {
        let content = node
            .style
            .padding
            .as_ref()
            .map(|pad| inset_rect(rect, pad))
            .unwrap_or(*rect);
        centered_origin(&content, layout.width(), layout.height())
    } else {
        (rect.x, rect.y)
    };
    paint_parley_layout(scene, ctx, layout, origin_x, origin_y, color, stats);
}

fn inset_rect(outer: &LayoutRect, padding: &Edges<f32>) -> LayoutRect {
    LayoutRect {
        x: outer.x + padding.left,
        y: outer.y + padding.top,
        width: (outer.width - padding.left - padding.right).max(0.0),
        height: (outer.height - padding.top - padding.bottom).max(0.0),
    }
}

fn centered_origin(content: &LayoutRect, text_width: f32, text_height: f32) -> (f32, f32) {
    (
        content.x + ((content.width - text_width) * 0.5).max(0.0),
        content.y + ((content.height - text_height) * 0.5).max(0.0),
    )
}

fn paint_parley_layout(
    scene: &mut Scene,
    ctx: PaintContext,
    layout: &ParleyLayout,
    origin_x: f32,
    origin_y: f32,
    color: Color,
    stats: &mut PaintStats,
) {
    let transform = ctx.base * Affine::translate((f64::from(origin_x), f64::from(origin_y)));
    let brush = to_peniko_color(color);

    for line in layout.lines() {
        for item in line.items() {
            let PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                continue;
            };

            let run = glyph_run.run();
            let font = run.font();
            let synthesis = run.synthesis();
            let glyph_xform = synthesis
                .skew()
                .map(|angle| Affine::skew(angle.to_radians().tan() as f64, 0.0));

            scene
                .draw_glyphs(font)
                .brush(brush)
                .hint(true)
                .transform(transform)
                .glyph_transform(glyph_xform)
                .font_size(run.font_size())
                .normalized_coords(run.normalized_coords())
                .draw(
                    Fill::NonZero,
                    glyph_run.positioned_glyphs().map(|glyph| Glyph {
                        id: glyph.id,
                        x: glyph.x,
                        y: glyph.y,
                    }),
                );
            stats.glyph_runs += 1;
        }
    }
}

fn fill_rounded_rect(
    scene: &mut Scene,
    ctx: PaintContext,
    shape: &RoundedRect,
    color: Color,
    stats: &mut PaintStats,
) {
    scene.fill(Fill::NonZero, ctx.base, to_peniko_color(color), None, shape);
    stats.fills += 1;
}

fn stroke_rounded_rect(
    scene: &mut Scene,
    ctx: PaintContext,
    shape: &RoundedRect,
    color: Color,
    width: f64,
    stats: &mut PaintStats,
) {
    scene.stroke(
        &Stroke::new(width),
        ctx.base,
        to_peniko_color(color),
        None,
        shape,
    );
    stats.strokes += 1;
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
    use crate::element::builders::{Button, Column, Text};
    use crate::layout::{layout_pass, Viewport};
    use crate::retained::RetainedTree;

    fn layout_and_paint(tree: &mut RetainedTree, scale_factor: f32) -> PaintStats {
        let layout = layout_pass(
            &mut *tree,
            Viewport {
                width: 200.0,
                height: 200.0,
            },
            scale_factor,
        )
        .unwrap();
        let mut scene = Scene::new();
        paint_pass(tree, &layout, &mut scene, scale_factor)
    }

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

        let stats = layout_and_paint(&mut tree, 1.0);

        assert_eq!(stats.fills, 1);
        assert_eq!(stats.strokes, 0);
    }

    #[test]
    fn container_border_emits_stroke() {
        let mut tree = RetainedTree::mount(
            Column::new()
                .width(100.0)
                .height(80.0)
                .border(Color::rgb8(255, 0, 0), 2.0)
                .into_element(),
        )
        .unwrap();

        let stats = layout_and_paint(&mut tree, 1.0);

        assert_eq!(stats.fills, 0);
        assert_eq!(stats.strokes, 1);
    }

    #[test]
    fn text_with_parley_layout_emits_glyph_runs() {
        let mut tree =
            RetainedTree::mount(Text::new("hello").font_size(16.0).into_element()).unwrap();

        let stats = layout_and_paint(&mut tree, 1.0);

        assert!(
            stats.glyph_runs > 0,
            "expected glyph runs for laid-out text"
        );
    }

    #[test]
    fn text_without_parley_layout_skips_glyphs() {
        let mut tree =
            RetainedTree::mount(Text::new("hello").font_size(16.0).into_element()).unwrap();
        let layout = layout_pass(
            &mut tree,
            Viewport {
                width: 200.0,
                height: 200.0,
            },
            1.0,
        )
        .unwrap();

        tree.root
            .as_mut()
            .unwrap()
            .text
            .as_mut()
            .unwrap()
            .parley_layout = None;

        let mut scene = Scene::new();
        let stats = paint_pass(&tree, &layout, &mut scene, 1.0);

        assert_eq!(stats.glyph_runs, 0);
    }

    #[test]
    fn button_paints_background_then_label() {
        let mut tree = RetainedTree::mount(
            Button::new("Press")
                .width(120.0)
                .height(48.0)
                .background(Color::rgb8(30, 60, 90))
                .into_element(),
        )
        .unwrap();

        let stats = layout_and_paint(&mut tree, 1.0);

        assert_eq!(stats.fills, 1, "button background");
        assert!(stats.glyph_runs > 0, "button label glyphs");
    }

    #[test]
    fn hidpi_scale_factor_runs_without_panic() {
        let mut tree = RetainedTree::mount(
            Column::new()
                .width(80.0)
                .height(60.0)
                .background(Color::rgb8(1, 2, 3))
                .child(Text::new("Hi").font_size(14.0))
                .into_element(),
        )
        .unwrap();

        let stats = layout_and_paint(&mut tree, 2.0);
        assert_eq!(stats.fills, 1);
        assert!(stats.glyph_runs > 0);
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

        let stats = layout_and_paint(&mut tree, 1.0);

        assert_eq!(stats.fills, 1, "only the column background is painted");
    }
}
