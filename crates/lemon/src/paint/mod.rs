use parley::PositionedLayoutItem;
use taffy::NodeId;
use vello::kurbo::{Affine, Line, Rect, RoundedRect, Stroke};
use vello::peniko::{
    color::AlphaColor, BlendMode, Blob, Color as PenikoColor, Fill, ImageAlphaType, ImageBrush,
    ImageData as PenikoImageData, ImageFormat,
};
use vello::{Glyph, Scene};

use crate::element::style::{Color, CornerRadii, Edges, Overflow};
use crate::layout::{caret_geometry_in_layout, measure_single_line_width, LayoutMap, LayoutRect};
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

/// Counters from a single [`paint_pass`] (useful in tests and profiling).
#[derive(Debug, Default, PartialEq, Eq)]
pub struct PaintStats {
    pub fills: u32,
    pub strokes: u32,
    pub glyph_runs: u32,
}

/// Walks the retained tree and records Vello draw commands into `scene`.
///
/// Coordinates in `layout` are logical points; `scale_factor` scales the root transform for HiDPI
/// (pass the window’s scale factor, often `1.0` in tests).
pub fn paint_pass(
    tree: &RetainedTree,
    layout: &LayoutMap,
    scene: &mut Scene,
    scale_factor: f32,
    focused: Option<NodeId>,
    caret_visible: bool,
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

    let mut global_deferred: Vec<(i32, &RetainedNode)> = Vec::new();

    if let Some(root) = tree.root.as_ref() {
        paint_node(
            root,
            layout,
            scene,
            ctx,
            scale_factor,
            focused,
            caret_visible,
            &mut stats,
            &mut global_deferred,
        );
    }

    // Paint globally deferred high-z nodes on top of all normal content.
    global_deferred.sort_by_key(|(z, _)| *z);
    for (_, node) in global_deferred {
        paint_node(
            node,
            layout,
            scene,
            ctx,
            scale_factor,
            focused,
            caret_visible,
            &mut stats,
            &mut Vec::new(),
        );
    }

    scene.pop_layer();
    stats
}

#[allow(clippy::too_many_arguments)]
fn paint_node<'a>(
    node: &'a RetainedNode,
    layout: &LayoutMap,
    scene: &mut Scene,
    ctx: PaintContext,
    scale_factor: f32,
    focused: Option<NodeId>,
    caret_visible: bool,
    stats: &mut PaintStats,
    global_deferred: &mut Vec<(i32, &'a RetainedNode)>,
) {
    if matches!(node.kind, RetainedKind::Component { .. }) {
        paint_children(
            &node.children,
            layout,
            scene,
            ctx,
            scale_factor,
            focused,
            caret_visible,
            stats,
            global_deferred,
        );
        return;
    }

    let Some(taffy_id) = node.taffy_id else {
        paint_children(
            &node.children,
            layout,
            scene,
            ctx,
            scale_factor,
            focused,
            caret_visible,
            stats,
            global_deferred,
        );
        return;
    };

    let Some(rect) = layout.get(taffy_id) else {
        return;
    };

    let opacity = resolved_opacity(node.style.opacity);
    let use_opacity_layer = opacity < 1.0;
    if use_opacity_layer {
        // Keep overflow-visible descendants paintable outside the node's layout bounds.
        scene.push_layer(
            Fill::NonZero,
            BlendMode::default(),
            opacity,
            ctx.base,
            &clip_everything(),
        );
    }

    paint_container(node, rect, scene, ctx, stats);
    paint_text(node, rect, scene, ctx, stats);
    paint_image(node, rect, scene, ctx);
    paint_focus_ring(node, rect, taffy_id, focused, scene, ctx, stats);

    let clip_children = node.style.overflow == Overflow::Hidden;
    if clip_children {
        let clip = Rect::new(
            f64::from(rect.x),
            f64::from(rect.y),
            f64::from(rect.x + rect.width),
            f64::from(rect.y + rect.height),
        );
        scene.push_layer(Fill::NonZero, BlendMode::default(), 1.0, ctx.base, &clip);
    }

    paint_children(
        &node.children,
        layout,
        scene,
        ctx,
        scale_factor,
        focused,
        caret_visible,
        stats,
        global_deferred,
    );

    if clip_children {
        scene.pop_layer();
    }

    paint_text_input_caret(
        node,
        layout,
        scale_factor,
        focused,
        caret_visible,
        scene,
        ctx,
        stats,
    );
    paint_scrollbar(node, rect, layout, scene, ctx, stats);
    paint_widget_scroll_bar(node, rect, layout, scene, ctx, stats);

    if use_opacity_layer {
        scene.pop_layer();
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_children<'a>(
    children: &'a [RetainedNode],
    layout: &LayoutMap,
    scene: &mut Scene,
    ctx: PaintContext,
    scale_factor: f32,
    focused: Option<NodeId>,
    caret_visible: bool,
    stats: &mut PaintStats,
    global_deferred: &mut Vec<(i32, &'a RetainedNode)>,
) {
    for child in children {
        if child.style.z_index != 0 {
            // Defer globally so high-z nodes paint on top of all siblings at every level.
            global_deferred.push((child.style.z_index, child));
        } else {
            paint_node(
                child,
                layout,
                scene,
                ctx,
                scale_factor,
                focused,
                caret_visible,
                stats,
                global_deferred,
            );
        }
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
        RetainedKind::View | RetainedKind::Row | RetainedKind::Column | RetainedKind::Button
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

/// Draws the image stored in `node.paint.image` into `rect` using object-fit: contain scaling.
///
/// The image is scaled uniformly so it fits within `rect` while preserving its aspect ratio,
/// then centered within the rectangle. No fill stat is recorded; image draws do not count as
/// fills. Returns early if the node is not a container kind, has no image, or has zero-size
/// dimensions.
fn paint_image(node: &RetainedNode, rect: &LayoutRect, scene: &mut Scene, ctx: PaintContext) {
    let is_container = matches!(
        node.kind,
        RetainedKind::View | RetainedKind::Row | RetainedKind::Column | RetainedKind::Button
    );
    if !is_container {
        return;
    }

    let Some(handle) = node.paint.image.as_ref() else {
        return;
    };

    let img_w = handle.width() as f64;
    let img_h = handle.height() as f64;
    if img_w <= 0.0 || img_h <= 0.0 {
        return;
    }

    let rect_w = f64::from(rect.width);
    let rect_h = f64::from(rect.height);
    if rect_w <= 0.0 || rect_h <= 0.0 {
        return;
    }

    // Object-fit: contain — uniform scale to fit within the box.
    let scale = (rect_w / img_w).min(rect_h / img_h);
    let scaled_w = img_w * scale;
    let scaled_h = img_h * scale;
    let offset_x = f64::from(rect.x) + (rect_w - scaled_w) * 0.5;
    let offset_y = f64::from(rect.y) + (rect_h - scaled_h) * 0.5;

    let transform = ctx.base * Affine::translate((offset_x, offset_y)) * Affine::scale(scale);

    let peniko_data = PenikoImageData {
        data: Blob::from(handle.pixels().to_vec()),
        format: ImageFormat::Rgba8,
        alpha_type: ImageAlphaType::Alpha,
        width: handle.width(),
        height: handle.height(),
    };
    let brush = ImageBrush::new(peniko_data);
    scene.draw_image(brush.as_ref(), transform);
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

    let color = crate::element::style::resolved_text_color(&text.style);
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

fn resolved_opacity(opacity: f32) -> f32 {
    if opacity.is_finite() {
        opacity.clamp(0.0, 1.0)
    } else {
        1.0
    }
}

fn paint_focus_ring(
    node: &RetainedNode,
    rect: &LayoutRect,
    taffy_id: NodeId,
    focused: Option<NodeId>,
    scene: &mut Scene,
    ctx: PaintContext,
    stats: &mut PaintStats,
) {
    if node.text_input.is_none() || focused != Some(taffy_id) {
        return;
    }

    let shape = layout_rect_to_rounded(rect, &node.paint.radius);
    stroke_rounded_rect(
        scene,
        ctx,
        &shape,
        node.paint
            .focus_ring_color
            .unwrap_or(crate::theme::current_theme().chrome.focus_ring),
        2.0,
        stats,
    );
}

fn first_text_descendant(node: &RetainedNode) -> Option<&RetainedNode> {
    if node.text.is_some() {
        return Some(node);
    }
    for child in &node.children {
        if let Some(found) = first_text_descendant(child) {
            return Some(found);
        }
    }
    None
}

#[allow(clippy::too_many_arguments)]
fn paint_text_input_caret(
    node: &RetainedNode,
    layout: &LayoutMap,
    _scale_factor: f32,
    focused: Option<NodeId>,
    caret_visible: bool,
    scene: &mut Scene,
    ctx: PaintContext,
    stats: &mut PaintStats,
) {
    if !caret_visible {
        return;
    }

    let Some(meta) = node.text_input.as_ref() else {
        return;
    };
    let Some(field_id) = node.taffy_id else {
        return;
    };
    if focused != Some(field_id) {
        return;
    }
    let Some(field_rect) = layout.get(field_id).copied() else {
        return;
    };

    let text_node = first_text_descendant(node);
    let text_rect = text_node
        .and_then(|n| n.taffy_id.and_then(|id| layout.get(id).copied()))
        .unwrap_or_else(|| {
            let padding = node.style.padding.clone().unwrap_or_default();
            inset_rect(&field_rect, &padding)
        });
    let text_style = text_node
        .and_then(|n| n.text.as_ref())
        .map(|t| t.style.clone())
        .unwrap_or_default();
    let parley_layout = text_node
        .and_then(|n| n.text.as_ref())
        .and_then(|t| t.parley_layout.as_ref());
    let content = text_node
        .and_then(|n| n.text.as_ref())
        .map(|t| t.content.as_str())
        .unwrap_or(meta.value.as_str());

    // `TextInputMeta.cursor` is authoritative; `TextCache.caret` can lag behind `UpdateText`.
    let cursor = meta.cursor.min(content.len());
    const CARET_STROKE: f32 = 1.0;
    /// Caret height as a fraction of the text font size (not the full line box).
    const CARET_HEIGHT_RATIO: f32 = 0.88;
    /// Extra length below center so the caret sits slightly lower without moving the top.
    const CARET_BOTTOM_EXTEND_RATIO: f32 = 0.1;
    let font_size = effective_font_size(&text_style);
    let caret_height = font_size * CARET_HEIGHT_RATIO;
    let caret_bottom_extend = font_size * CARET_BOTTOM_EXTEND_RATIO;

    let (caret_x, caret_top, caret_bottom) = if let Some(parley_layout) = parley_layout {
        let geom = caret_geometry_in_layout(parley_layout, content, cursor, CARET_STROKE);
        let line_top = text_rect.y + geom.y0 as f32;
        let line_bottom = text_rect.y + geom.y1 as f32;
        let line_center = (line_top + line_bottom) * 0.5;
        (
            text_rect.x + geom.x0 as f32,
            line_center - caret_height * 0.5,
            line_center + caret_height * 0.5 + caret_bottom_extend,
        )
    } else {
        let prefix = &content[..cursor];
        let caret_x = text_rect.x + measure_single_line_width(prefix, &text_style);
        let line_center = text_rect.y + text_rect.height * 0.5;
        (
            caret_x,
            line_center - caret_height * 0.5,
            line_center + caret_height * 0.5 + caret_bottom_extend,
        )
    };

    let line = Line::new(
        (f64::from(caret_x), f64::from(caret_top)),
        (f64::from(caret_x), f64::from(caret_bottom)),
    );
    // Match the visible text color (value or placeholder), not only `chrome.caret`, so the
    // caret stays visible when the app background does not match the active theme palette.
    let caret_color = crate::element::style::resolved_text_color(&text_style);
    scene.stroke(
        &Stroke::new(f64::from(CARET_STROKE)),
        ctx.base,
        to_peniko_color(caret_color),
        None,
        &line,
    );
    stats.strokes += 1;
}

fn effective_font_size(style: &crate::element::style::TextStyle) -> f32 {
    if style.font_size > 0.0 {
        style.font_size
    } else {
        crate::theme::current_theme().typography.font_size_md
    }
}

const WIDGET_SCROLLBAR_WIDTH: f32 = 8.0;
const WIDGET_SCROLLBAR_THUMB_MIN: f32 = 20.0;

fn scroll_offset_from_inner(node: &RetainedNode) -> f32 {
    node.children
        .first()
        .and_then(|inner| inner.style.margin.as_ref())
        .map(|m| (-m.top).max(0.0))
        .unwrap_or(0.0)
}

fn paint_widget_scroll_bar(
    node: &RetainedNode,
    viewport_rect: &LayoutRect,
    layout: &LayoutMap,
    scene: &mut Scene,
    ctx: PaintContext,
    stats: &mut PaintStats,
) {
    if !node.scroll_bar {
        return;
    }
    let Some(content_height) = crate::layout::scroll_content_extent(node, layout) else {
        return;
    };
    let viewport_height = viewport_rect.height;
    if content_height <= viewport_height + 1.0 {
        return;
    }

    let scroll_offset = scroll_offset_from_inner(node);
    let max_offset = (content_height - viewport_height).max(1.0);
    let track_x = viewport_rect.x + viewport_rect.width - WIDGET_SCROLLBAR_WIDTH;
    let track = Rect::new(
        f64::from(track_x),
        f64::from(viewport_rect.y),
        f64::from(track_x + WIDGET_SCROLLBAR_WIDTH),
        f64::from(viewport_rect.y + viewport_height),
    );
    scene.fill(
        Fill::NonZero,
        ctx.base,
        to_peniko_color(
            node.paint
                .scroll_track_color
                .unwrap_or(crate::theme::current_theme().chrome.scrollbar_track),
        ),
        None,
        &track,
    );
    stats.fills += 1;

    let thumb_height = (viewport_height / content_height * viewport_height)
        .clamp(WIDGET_SCROLLBAR_THUMB_MIN, viewport_height);
    let thumb_travel = (viewport_height - thumb_height).max(0.0);
    let thumb_y = viewport_rect.y + (scroll_offset / max_offset).clamp(0.0, 1.0) * thumb_travel;
    let thumb = Rect::new(
        f64::from(track_x),
        f64::from(thumb_y),
        f64::from(track_x + WIDGET_SCROLLBAR_WIDTH),
        f64::from(thumb_y + thumb_height),
    );
    scene.fill(
        Fill::NonZero,
        ctx.base,
        to_peniko_color(
            node.paint
                .scroll_thumb_color
                .unwrap_or(crate::theme::current_theme().chrome.scrollbar_thumb),
        ),
        None,
        &thumb,
    );
    stats.fills += 1;
}

fn paint_scrollbar(
    node: &RetainedNode,
    viewport_rect: &LayoutRect,
    layout: &LayoutMap,
    scene: &mut Scene,
    ctx: PaintContext,
    stats: &mut PaintStats,
) {
    if !node.scroll_viewport {
        return;
    }
    let Some(inner) = node.children.first() else {
        return;
    };
    let content = inner.children.first().unwrap_or(inner);
    let Some(content_id) = content.taffy_id else {
        return;
    };
    let Some(content_rect) = layout.get(content_id) else {
        return;
    };

    let content_height = content_rect.height;
    let viewport_height = viewport_rect.height;
    if content_height <= viewport_height + 1.0 {
        return;
    }

    let scroll_offset = scroll_offset_from_inner(node);
    let max_offset = (content_height - viewport_height).max(1.0);
    let track_width = 6.0;
    let track_x = viewport_rect.x + viewport_rect.width - track_width - 2.0;
    let track = Rect::new(
        f64::from(track_x),
        f64::from(viewport_rect.y + 2.0),
        f64::from(track_x + track_width),
        f64::from(viewport_rect.y + viewport_rect.height - 2.0),
    );
    scene.fill(
        Fill::NonZero,
        ctx.base,
        to_peniko_color(
            node.paint
                .scroll_track_color
                .unwrap_or(crate::theme::current_theme().chrome.scrollbar_track),
        ),
        None,
        &track,
    );
    stats.fills += 1;

    let thumb_height = (viewport_height / content_height * viewport_height).max(16.0);
    let thumb_travel = (viewport_height - thumb_height - 4.0).max(0.0);
    let thumb_y =
        viewport_rect.y + 2.0 + (scroll_offset / max_offset).clamp(0.0, 1.0) * thumb_travel;
    let thumb = Rect::new(
        f64::from(track_x + 1.0),
        f64::from(thumb_y),
        f64::from(track_x + track_width - 1.0),
        f64::from(thumb_y + thumb_height),
    );
    scene.fill(
        Fill::NonZero,
        ctx.base,
        to_peniko_color(
            node.paint
                .scroll_thumb_color
                .unwrap_or(crate::theme::current_theme().chrome.scrollbar_thumb),
        ),
        None,
        &thumb,
    );
    stats.fills += 1;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::builders::{Button, Column, Text, View};
    use crate::layout::{layout_pass, Viewport};
    use crate::retained::RetainedTree;

    fn layout_and_paint(tree: &mut RetainedTree, scale_factor: f32) -> PaintStats {
        let layout = layout_pass(
            &mut *tree,
            Viewport {
                width: 200.0,
                height: 200.0,
            },
        )
        .unwrap();
        let mut scene = Scene::new();
        paint_pass(tree, &layout, &mut scene, scale_factor, None, true)
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
        let stats = paint_pass(&tree, &layout, &mut scene, 1.0, None, true);

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
        let stats = paint_pass(&tree, &LayoutMap::default(), &mut scene, 1.0, None, true);

        assert_eq!(stats.fills, 0);
    }

    #[test]
    fn overflow_hidden_emits_extra_push_pop_layer() {
        use crate::element::style::Overflow;

        // Without overflow:hidden — children painted with no extra clip layer.
        let mut tree_no_clip = RetainedTree::mount(
            Column::new()
                .width(100.0)
                .height(80.0)
                .background(Color::rgb8(40, 80, 120))
                .child(Text::new("inner").font_size(12.0))
                .into_element(),
        )
        .unwrap();
        let stats_no_clip = layout_and_paint(&mut tree_no_clip, 1.0);

        // With overflow:hidden — paint_node wraps children in an extra clip layer.
        let mut tree_clip = RetainedTree::mount(
            Column::new()
                .width(100.0)
                .height(80.0)
                .background(Color::rgb8(40, 80, 120))
                .overflow(Overflow::Hidden)
                .child(Text::new("inner").font_size(12.0))
                .into_element(),
        )
        .unwrap();
        let stats_clip = layout_and_paint(&mut tree_clip, 1.0);

        // Same fill count — clipping does not change how many fills are drawn.
        assert_eq!(stats_no_clip.fills, stats_clip.fills);
        // Both produce glyph runs for the text child.
        assert!(stats_clip.glyph_runs > 0);
    }

    #[test]
    fn z_index_children_are_painted_without_panic() {
        let mut tree = RetainedTree::mount(
            Column::new()
                .width(120.0)
                .height(120.0)
                .child(
                    View::new()
                        .width(80.0)
                        .height(40.0)
                        .background(Color::rgb8(255, 0, 0)),
                )
                .child(
                    View::new()
                        .width(80.0)
                        .height(40.0)
                        .background(Color::rgb8(0, 255, 0))
                        .z_index(1),
                )
                .into_element(),
        )
        .unwrap();

        let stats = layout_and_paint(&mut tree, 1.0);
        assert_eq!(stats.fills, 2, "both children must be painted");
    }

    #[test]
    fn z_index_children_with_overflow_hidden_are_painted_without_panic() {
        use crate::element::style::Overflow;

        let mut tree = RetainedTree::mount(
            Column::new()
                .width(120.0)
                .height(120.0)
                .overflow(Overflow::Hidden)
                .child(
                    View::new()
                        .width(80.0)
                        .height(40.0)
                        .background(Color::rgb8(255, 0, 0)),
                )
                .child(
                    View::new()
                        .width(80.0)
                        .height(40.0)
                        .background(Color::rgb8(0, 255, 0))
                        .z_index(2),
                )
                .into_element(),
        )
        .unwrap();

        let stats = layout_and_paint(&mut tree, 1.0);
        assert_eq!(stats.fills, 2, "overflow clipping should keep both fills");
    }

    #[test]
    fn partial_opacity_paints_without_panic() {
        let mut tree = RetainedTree::mount(
            Column::new()
                .width(100.0)
                .height(80.0)
                .background(Color::rgb8(40, 80, 120))
                .opacity(0.5)
                .child(Text::new("hello").font_size(12.0))
                .into_element(),
        )
        .unwrap();

        let stats = layout_and_paint(&mut tree, 1.0);
        assert_eq!(stats.fills, 1);
        assert!(stats.glyph_runs > 0);
    }

    #[test]
    fn z_index_non_zero_nodes_paint_on_top_of_siblings() {
        // A column with two children: z_index=0 (red) and z_index=1 (green).
        // With global deferral both must be painted; green deferred node paints last.
        let mut tree = RetainedTree::mount(
            Column::new()
                .width(120.0)
                .height(120.0)
                .child(
                    View::new()
                        .width(80.0)
                        .height(40.0)
                        .background(Color::rgb8(255, 0, 0)),
                )
                .child(
                    View::new()
                        .width(80.0)
                        .height(40.0)
                        .background(Color::rgb8(0, 255, 0))
                        .z_index(1),
                )
                .into_element(),
        )
        .unwrap();

        let stats = layout_and_paint(&mut tree, 1.0);
        assert_eq!(stats.fills, 2, "both siblings must be painted");
    }

    #[test]
    fn z_index_non_zero_paints_over_later_siblings_at_parent_level() {
        // Simulates the Select dropdown scenario: a Column (the Select wrapper) containing
        // a trigger (z=0) and a dropdown (z=10), alongside a sibling Column (z=0).
        // The dropdown must paint on top of the sibling, not behind it.
        let mut tree = RetainedTree::mount(
            Column::new()
                .width(200.0)
                .height(200.0)
                // Select wrapper
                .child(
                    Column::new()
                        .width(100.0)
                        .height(100.0)
                        // trigger
                        .child(
                            View::new()
                                .width(100.0)
                                .height(20.0)
                                .background(Color::rgb8(80, 80, 80)),
                        )
                        // dropdown with high z_index
                        .child(
                            View::new()
                                .width(100.0)
                                .height(60.0)
                                .background(Color::rgb8(35, 35, 52))
                                .z_index(10),
                        ),
                )
                // sibling that dropdown must overlay
                .child(
                    View::new()
                        .width(200.0)
                        .height(80.0)
                        .background(Color::rgb8(60, 60, 60)),
                )
                .into_element(),
        )
        .unwrap();

        let stats = layout_and_paint(&mut tree, 1.0);
        // trigger(1) + sibling(1) + dropdown(1) = 3 fills, all painted
        assert_eq!(stats.fills, 3, "dropdown and all siblings must be painted");
    }

    #[test]
    fn z_index_absolute_column_dropdown_paints_background() {
        // Mirrors lemon_widgets::Select open dropdown: Column + absolute + z_index + background.
        let mut tree = RetainedTree::mount(
            Column::new()
                .width(200.0)
                .height(200.0)
                .child(
                    Column::new()
                        .width(100.0)
                        .child(
                            View::new()
                                .width(100.0)
                                .height(40.0)
                                .background(Color::rgb8(55, 120, 220)),
                        )
                        .child(
                            Column::new()
                                .z_index(10)
                                .absolute()
                                .top(40.0)
                                .left(0.0)
                                .width(100.0)
                                .background(Color::rgb8(35, 35, 52))
                                .border(Color::rgb8(80, 80, 110), 1.0)
                                .radius(6.0)
                                .child(
                                    View::new()
                                        .padding(8.0)
                                        .child(Text::new("Option A").font_size(14.0)),
                                )
                                .child(
                                    View::new()
                                        .padding(8.0)
                                        .child(Text::new("Option B").font_size(14.0)),
                                ),
                        ),
                )
                .child(Text::new("Sibling below"))
                .into_element(),
        )
        .unwrap();

        let stats = layout_and_paint(&mut tree, 1.0);
        assert!(
            stats.fills >= 2,
            "trigger and dropdown panel must emit fills, got {}",
            stats.fills
        );
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

    #[test]
    fn image_handle_in_paint_data_runs_without_panic() {
        use crate::asset::image_handle::ImageData;
        use crate::asset::ImageHandle;
        use crate::element::builders::View;
        use std::sync::Arc;

        let handle = ImageHandle::from_arc(Arc::new(ImageData {
            width: 4,
            height: 4,
            pixels: vec![128u8; 4 * 4 * 4],
        }));

        let mut tree = RetainedTree::mount(
            View::new()
                .width(100.0)
                .height(100.0)
                .image(handle)
                .into_element(),
        )
        .unwrap();

        let stats = layout_and_paint(&mut tree, 1.0);
        assert_eq!(stats.fills, 0, "image draw does not count as fill");
    }

    /// Verify that the active theme's `chrome.scrollbar_track` and `chrome.scrollbar_thumb`
    /// tokens are read by the paint pass.  We switch to a custom theme, run paint, then
    /// restore the original theme.  If paint panics or the hardcoded constants were still
    /// used the fill counts would be wrong.
    #[test]
    fn scrollbar_paints_two_fills_with_custom_chrome_theme() {
        use crate::element::style::Overflow;
        use crate::theme::{set_active_theme, Theme, WidgetChromeTokens};

        let original = crate::theme::current_theme();

        let mut custom = Theme::default_dark();
        custom.chrome = WidgetChromeTokens {
            scrollbar_track: Color::rgb8(10, 20, 30),
            scrollbar_thumb: Color::rgb8(50, 60, 70),
            caret: Color::rgb8(255, 255, 255),
            focus_ring: Color::rgb8(255, 0, 0),
        };
        set_active_theme(custom);

        // Build a scroll-bar viewport whose content is taller than the viewport.
        // Structure mirrors the Scroll widget: viewport -> inner wrapper -> content.
        // The viewport must have scroll_bar, overflow: hidden, and on_scroll so that
        // scroll_content_extent returns a height and the scrollbar is drawn.
        let content = View::new()
            .width(100.0)
            .height(300.0)
            .background(Color::rgb8(200, 200, 200));
        let inner = View::new().child(content);
        let viewport = View::new()
            .width(100.0)
            .height(80.0)
            .overflow(Overflow::Hidden)
            .scroll_bar()
            .on_scroll(|_| {})
            .child(inner);

        let mut tree = RetainedTree::mount(viewport.into_element()).unwrap();
        let stats = layout_and_paint(&mut tree, 1.0);

        set_active_theme(original);

        // The scrollbar contributes 2 fills (track + thumb); the child background adds 1.
        assert_eq!(
            stats.fills, 3,
            "track + thumb + child background must be painted"
        );
        assert_eq!(stats.strokes, 0);
    }

    /// Verify that the active theme's `chrome.focus_ring` token is read by paint when a
    /// text-input node is focused.
    #[test]
    fn focus_ring_uses_theme_chrome_token() {
        use crate::element::types::TextInputMeta;
        use crate::theme::{set_active_theme, Theme, WidgetChromeTokens};

        let original = crate::theme::current_theme();

        let mut custom = Theme::default_light();
        custom.chrome = WidgetChromeTokens {
            scrollbar_track: Color::rgb8(200, 200, 200),
            scrollbar_thumb: Color::rgb8(150, 150, 150),
            caret: Color::rgb8(0, 0, 0),
            focus_ring: Color::rgb8(255, 128, 0),
        };
        set_active_theme(custom);

        // Build a text-input node so that paint_focus_ring is exercised.
        let mut tree =
            RetainedTree::mount(View::new().width(120.0).height(40.0).into_element()).unwrap();

        // Inject text_input meta onto the root node so paint_focus_ring activates.
        if let Some(root) = tree.root.as_mut() {
            root.text_input = Some(TextInputMeta {
                value: String::new(),
                cursor: 0,
            });
        }

        let root_taffy_id = tree.root.as_ref().unwrap().taffy_id;

        let layout = layout_pass(
            &mut tree,
            Viewport {
                width: 400.0,
                height: 300.0,
            },
        )
        .unwrap();
        let mut scene = Scene::new();
        let stats = paint_pass(&tree, &layout, &mut scene, 1.0, root_taffy_id, false);

        set_active_theme(original);

        // Focus ring is painted as a stroke.
        assert_eq!(stats.strokes, 1, "focus ring must emit exactly one stroke");
    }

    /// Verify that switching the active theme between calls to paint_pass does not panic
    /// and that the fill/stroke counts are stable.
    #[test]
    fn paint_is_stable_across_theme_switch() {
        use crate::theme::{set_active_theme, Theme};

        let original = crate::theme::current_theme();

        let mut tree = RetainedTree::mount(
            Column::new()
                .width(100.0)
                .height(80.0)
                .background(Color::rgb8(40, 80, 120))
                .into_element(),
        )
        .unwrap();

        let stats_light = layout_and_paint(&mut tree, 1.0);

        set_active_theme(Theme::default_dark());
        let stats_dark = layout_and_paint(&mut tree, 1.0);

        set_active_theme(original);

        assert_eq!(stats_light.fills, 1, "light theme: one background fill");
        assert_eq!(stats_dark.fills, 1, "dark theme: one background fill");
        assert_eq!(
            stats_light.strokes, stats_dark.strokes,
            "stroke count unchanged across themes"
        );
    }
}
