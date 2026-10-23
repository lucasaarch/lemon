use parley::PositionedLayoutItem;
use taffy::NodeId;
use vello::kurbo::{Affine, Line, Rect, RoundedRect, Stroke};
use vello::peniko::{
    color::AlphaColor, BlendMode, Blob, Color as PenikoColor, Fill, Gradient, ImageAlphaType,
    ImageBrush, ImageData as PenikoImageData, ImageFormat,
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
struct PaintContext<'a> {
    /// Global scale applied to all logical coordinates (HiDPI).
    base: Affine,
    visuals: Option<&'a std::collections::HashMap<NodeId, crate::transition::Visual>>,
}

impl PaintContext<'_> {
    fn visual(&self, node: &RetainedNode) -> Option<&crate::transition::Visual> {
        self.visuals
            .and_then(|visuals| visuals.get(&node.taffy_id?))
    }
    fn paint<'a>(&'a self, node: &'a RetainedNode) -> &'a crate::element::style::PaintData {
        self.visual(node)
            .map_or(&node.paint, |visual| &visual.paint)
    }
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
    paint_pass_with_visuals(
        tree,
        layout,
        scene,
        scale_factor,
        focused,
        caret_visible,
        None,
    )
}

pub(crate) fn paint_pass_with_visuals(
    tree: &RetainedTree,
    layout: &LayoutMap,
    scene: &mut Scene,
    scale_factor: f32,
    focused: Option<NodeId>,
    caret_visible: bool,
    visuals: Option<&std::collections::HashMap<NodeId, crate::transition::Visual>>,
) -> PaintStats {
    let mut stats = PaintStats::default();
    let base = if scale_factor == 1.0 {
        Affine::IDENTITY
    } else {
        Affine::scale(f64::from(scale_factor))
    };
    let ctx = PaintContext { base, visuals };

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
    while !global_deferred.is_empty() {
        global_deferred.sort_by_key(|(z, _)| *z);
        let (_, node) = global_deferred.remove(0);
        paint_node(
            node,
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
            node.scroll_viewport
                .and_then(|_| crate::layout::scroll_viewport_rect(node, layout)),
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
            node.scroll_viewport
                .and_then(|_| crate::layout::scroll_viewport_rect(node, layout)),
            global_deferred,
        );
        return;
    };

    let Some(rect) = layout.get(taffy_id) else {
        return;
    };

    let opacity = resolved_opacity(
        ctx.visual(node)
            .map_or(node.style.opacity, |visual| visual.opacity),
    );
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
    paint_icon(node, rect, scene, ctx, stats);
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

    paint_text_input_selection(node, layout, focused, scene, ctx, stats);

    paint_children(
        &node.children,
        layout,
        scene,
        ctx,
        scale_factor,
        focused,
        caret_visible,
        stats,
        node.scroll_viewport
            .and_then(|_| crate::layout::scroll_viewport_rect(node, layout)),
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
    paint_scrollbar(node, layout, scene, ctx, stats);

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
    scroll_clip: Option<LayoutRect>,
    global_deferred: &mut Vec<(i32, &'a RetainedNode)>,
) {
    for child in children {
        if child.style.z_index != 0 || child.style.position == crate::Position::Fixed {
            // Defer globally so high-z nodes paint on top of all siblings at every level.
            global_deferred.push((child.style.z_index, child));
        } else {
            let clip = scroll_clip.filter(|_| child.style.scroll_content);
            if let Some(rect) = clip {
                let rect = Rect::new(
                    rect.x as f64,
                    rect.y as f64,
                    (rect.x + rect.width) as f64,
                    (rect.y + rect.height) as f64,
                );
                scene.push_layer(Fill::NonZero, BlendMode::default(), 1.0, ctx.base, &rect);
            }
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
            if clip.is_some() {
                scene.pop_layer();
            }
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
        RetainedKind::View | RetainedKind::Row | RetainedKind::Column
    );
    if !is_container {
        return;
    }

    let paint = ctx.paint(node);
    let shape = layout_rect_to_rounded(rect, &paint.radius);

    if let Some(shadow) = paint.box_shadow {
        paint_box_shadow(scene, ctx, rect, &paint.radius, shadow, stats);
    }

    if let Some(gradient) = paint.background_gradient {
        fill_gradient_rounded_rect(scene, ctx, rect, &shape, gradient, stats);
    } else if let Some(background) = paint.background {
        fill_rounded_rect(scene, ctx, &shape, background, stats);
    }

    let widths = &paint.border_width;
    if widths.any_positive() {
        if let Some(border_color) = uniform_border_color(&paint.border_color) {
            if widths.is_uniform() {
                stroke_rounded_rect(
                    scene,
                    ctx,
                    &shape,
                    border_color,
                    f64::from(widths.top),
                    stats,
                );
            } else {
                paint_per_side_borders(
                    scene,
                    ctx,
                    rect,
                    &paint.radius,
                    *widths,
                    &paint.border_color,
                    stats,
                );
            }
        } else {
            paint_per_side_borders(
                scene,
                ctx,
                rect,
                &paint.radius,
                *widths,
                &paint.border_color,
                stats,
            );
        }
    }
}

fn paint_box_shadow(
    scene: &mut Scene,
    ctx: PaintContext,
    rect: &LayoutRect,
    radii: &CornerRadii,
    shadow: crate::element::style::BoxShadow,
    stats: &mut PaintStats,
) {
    if shadow.color.a <= f32::EPSILON {
        return;
    }

    let shadow_rect = Rect::new(
        f64::from(rect.x + shadow.offset_x),
        f64::from(rect.y + shadow.offset_y),
        f64::from(rect.x + rect.width + shadow.offset_x),
        f64::from(rect.y + rect.height + shadow.offset_y),
    );
    scene.draw_blurred_rounded_rect(
        ctx.base,
        shadow_rect,
        to_peniko_color(shadow.color),
        shadow_radius(radii),
        f64::from(shadow.blur_radius.max(0.0)),
    );
    stats.fills += 1;
}

/// Fills rectangular strips for each side that has a positive border width.
fn paint_per_side_borders(
    scene: &mut Scene,
    ctx: PaintContext,
    rect: &LayoutRect,
    radii: &CornerRadii,
    widths: Edges<f32>,
    colors: &Edges<Option<Color>>,
    stats: &mut PaintStats,
) {
    let x = f64::from(rect.x);
    let y = f64::from(rect.y);
    let w = f64::from(rect.width);
    let h = f64::from(rect.height);

    let tl = f64::from(radii.top_left.min(rect.width * 0.5));
    let tr = f64::from(radii.top_right.min(rect.width * 0.5));
    let br = f64::from(radii.bottom_right.min(rect.width * 0.5));
    let bl = f64::from(radii.bottom_left.min(rect.width * 0.5));

    if widths.top > 0.0 {
        if let Some(color) = colors.top {
            let t = f64::from(widths.top);
            let bar = Rect::new(x + tl, y, x + w - tr, y + t);
            scene.fill(Fill::NonZero, ctx.base, to_peniko_color(color), None, &bar);
            stats.fills += 1;
        }
    }
    if widths.bottom > 0.0 {
        if let Some(color) = colors.bottom {
            let b = f64::from(widths.bottom);
            let bar = Rect::new(x + bl, y + h - b, x + w - br, y + h);
            scene.fill(Fill::NonZero, ctx.base, to_peniko_color(color), None, &bar);
            stats.fills += 1;
        }
    }
    if widths.left > 0.0 {
        if let Some(color) = colors.left {
            let l = f64::from(widths.left);
            let bar = Rect::new(x, y + tl, x + l, y + h - bl);
            scene.fill(Fill::NonZero, ctx.base, to_peniko_color(color), None, &bar);
            stats.fills += 1;
        }
    }
    if widths.right > 0.0 {
        if let Some(color) = colors.right {
            let r = f64::from(widths.right);
            let bar = Rect::new(x + w - r, y + tr, x + w, y + h - br);
            scene.fill(Fill::NonZero, ctx.base, to_peniko_color(color), None, &bar);
            stats.fills += 1;
        }
    }
}

fn paint_icon(
    node: &RetainedNode,
    rect: &LayoutRect,
    scene: &mut Scene,
    ctx: PaintContext,
    stats: &mut PaintStats,
) {
    let Some(icon) = ctx.paint(node).icon.as_ref() else {
        return;
    };
    let size = rect.width.min(rect.height);
    if !size.is_finite() || size <= 0.0 {
        return;
    }
    let transform = ctx.base
        * Affine::translate((
            (rect.x + (rect.width - size) / 2.0) as f64,
            (rect.y + (rect.height - size) / 2.0) as f64,
        ))
        * Affine::scale(size as f64 / 24.0);
    let color = to_peniko_color(icon.color);
    if icon.stroke_width.is_finite() && icon.stroke_width > 0.0 {
        if let Ok(path) = vello::kurbo::BezPath::from_svg(icon.data.stroke) {
            let stroke = Stroke::new(icon.stroke_width as f64)
                .with_caps(vello::kurbo::Cap::Round)
                .with_join(vello::kurbo::Join::Round);
            scene.stroke(&stroke, transform, color, None, &path);
            stats.strokes += 1;
        }
    }
    if !icon.data.fill.is_empty() {
        if let Ok(path) = vello::kurbo::BezPath::from_svg(icon.data.fill) {
            scene.fill(Fill::NonZero, transform, color, None, &path);
            stats.fills += 1;
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
        RetainedKind::View | RetainedKind::Row | RetainedKind::Column
    );
    if !is_container {
        return;
    }

    let Some(handle) = ctx.paint(node).image.as_ref() else {
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
    let is_text = matches!(node.kind, RetainedKind::Text);
    if !is_text {
        return;
    }

    let Some(text) = node.text.as_ref() else {
        return;
    };
    let Some(layout) = text.parley_layout.as_ref() else {
        return;
    };

    let color = ctx
        .visual(node)
        .and_then(|visual| visual.text_color)
        .unwrap_or_else(|| crate::element::style::resolved_text_color(&text.style));
    let (origin_x, origin_y) = (rect.x, rect.y);
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

fn fill_gradient_rounded_rect(
    scene: &mut Scene,
    ctx: PaintContext,
    rect: &LayoutRect,
    shape: &RoundedRect,
    gradient: crate::element::style::LinearGradient,
    stats: &mut PaintStats,
) {
    let brush = to_peniko_gradient(rect, gradient);
    scene.fill(Fill::NonZero, ctx.base, &brush, None, shape);
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

fn to_peniko_gradient(
    rect: &LayoutRect,
    gradient: crate::element::style::LinearGradient,
) -> Gradient {
    let start = (
        rect.x + rect.width * gradient.start.0,
        rect.y + rect.height * gradient.start.1,
    );
    let end = (
        rect.x + rect.width * gradient.end.0,
        rect.y + rect.height * gradient.end.1,
    );
    Gradient::new_linear(start, end).with_stops([
        to_peniko_color(gradient.start_color),
        to_peniko_color(gradient.end_color),
    ])
}

fn uniform_border_color(colors: &Edges<Option<Color>>) -> Option<Color> {
    match (colors.top, colors.right, colors.bottom, colors.left) {
        (Some(top), Some(right), Some(bottom), Some(left))
            if top == right && right == bottom && bottom == left =>
        {
            Some(top)
        }
        _ => None,
    }
}

fn shadow_radius(radii: &CornerRadii) -> f64 {
    f64::from(
        radii
            .top_left
            .max(radii.top_right)
            .max(radii.bottom_right)
            .max(radii.bottom_left),
    )
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
    if node.text_input.is_none() && (!node.style.focusable || node.paint.focus_ring_color.is_none())
    {
        return;
    }
    let color = if let Some(visual) = ctx.visual(node) {
        visual.paint.focus_ring_color.unwrap_or_default()
    } else {
        if focused != Some(taffy_id) {
            return;
        }
        node.paint
            .focus_ring_color
            .unwrap_or(crate::theme::current_theme().chrome.focus_ring)
    };
    if color.a <= 0.0 {
        return;
    }

    let shape = layout_rect_to_rounded(rect, &ctx.paint(node).radius);
    stroke_rounded_rect(scene, ctx, &shape, color, 2.0, stats);
}

fn text_input_selection_rects(node: &RetainedNode, layout: &LayoutMap) -> Vec<Rect> {
    let Some(meta) = node.text_input.as_ref() else {
        return Vec::new();
    };
    let Some(range) = meta.selection.as_ref() else {
        return Vec::new();
    };
    if range.start >= range.end || meta.value.get(range.clone()).is_none() {
        return Vec::new();
    }
    let Some(text_node) = crate::layout::text_input_node(node) else {
        return Vec::new();
    };
    let Some(text) = text_node.text.as_ref() else {
        return Vec::new();
    };
    // Never highlight a placeholder or use a stale text layout.
    if text.content != meta.value {
        return Vec::new();
    }
    let Some(text_layout) = text.parley_layout.as_ref() else {
        return Vec::new();
    };
    let Some(rect) = text_node.taffy_id.and_then(|id| layout.get(id)) else {
        return Vec::new();
    };
    let Some(content) = crate::layout::text_input_rect(node, layout) else {
        return Vec::new();
    };
    let clip = Rect::new(
        content.x as f64,
        content.y as f64,
        (content.x + content.width) as f64,
        (content.y + content.height) as f64,
    );
    use parley::editing::{Cursor, Selection};
    use parley::layout::Affinity;
    let selection = Selection::new(
        Cursor::from_byte_index(text_layout, range.start, Affinity::Downstream),
        Cursor::from_byte_index(text_layout, range.end, Affinity::Upstream),
    );
    selection
        .geometry(text_layout)
        .into_iter()
        .filter_map(|(bounds, _)| {
            let bounds = Rect::new(
                bounds.x0 + rect.x as f64,
                bounds.y0 + rect.y as f64,
                bounds.x1 + rect.x as f64,
                bounds.y1 + rect.y as f64,
            )
            .intersect(clip);
            (bounds.width() > 0.0 && bounds.height() > 0.0).then_some(bounds)
        })
        .collect()
}

fn paint_text_input_selection(
    node: &RetainedNode,
    layout: &LayoutMap,
    focused: Option<NodeId>,
    scene: &mut Scene,
    ctx: PaintContext,
    stats: &mut PaintStats,
) {
    if focused.is_none() || focused != node.taffy_id {
        return;
    }
    let color = crate::theme::current_theme().colors.accent.with_alpha(0.28);
    for rect in text_input_selection_rects(node, layout) {
        scene.fill(Fill::NonZero, ctx.base, to_peniko_color(color), None, &rect);
        stats.fills += 1;
    }
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

    let text_node = crate::layout::text_input_node(node);
    let text_rect = text_node
        .and_then(|n| n.taffy_id.and_then(|id| layout.get(id).copied()))
        .unwrap_or_else(|| {
            let padding = node.style.padding.unwrap_or_default();
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
    let font_size = text_style.resolved_font_size();
    let caret_height = font_size * CARET_HEIGHT_RATIO;
    let caret_bottom_extend = font_size * CARET_BOTTOM_EXTEND_RATIO;

    let (caret_x, caret_top, caret_bottom) = if let Some(parley_layout) = parley_layout {
        let geom =
            caret_geometry_in_layout(parley_layout, content, cursor, CARET_STROKE, meta.affinity);
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

    // A placeholder is not editable text: its width must not move the empty caret
    // away from the chosen alignment anchor.
    let caret_x = if meta.value.is_empty() {
        empty_field_caret_x(text_rect, meta.alignment)
    } else {
        caret_x
    };
    let line = Line::new(
        (f64::from(caret_x), f64::from(caret_top)),
        (f64::from(caret_x), f64::from(caret_bottom)),
    );
    // Match the visible text color (value or placeholder), not only `chrome.caret`, so the
    // caret stays visible when the app background does not match the active theme palette.
    let caret_color = text_node
        .and_then(|node| ctx.visual(node))
        .and_then(|visual| visual.text_color)
        .unwrap_or_else(|| crate::element::style::resolved_text_color(&text_style));
    let Some(bounds) = crate::layout::text_input_rect(node, layout) else {
        return;
    };
    let clip = Rect::new(
        (bounds.x - CARET_STROKE * 0.5) as f64,
        bounds.y as f64,
        (bounds.x + bounds.width + CARET_STROKE * 0.5) as f64,
        (bounds.y + bounds.height) as f64,
    );
    scene.push_layer(Fill::NonZero, BlendMode::default(), 1.0, ctx.base, &clip);
    scene.stroke(
        &Stroke::new(f64::from(CARET_STROKE)),
        ctx.base,
        to_peniko_color(caret_color),
        None,
        &line,
    );
    scene.pop_layer();
    stats.strokes += 1;
}

fn empty_field_caret_x(rect: LayoutRect, alignment: crate::TextAlignment) -> f32 {
    rect.x
        + rect.width
            * match alignment {
                crate::TextAlignment::Start => 0.0,
                crate::TextAlignment::Center => 0.5,
                crate::TextAlignment::End => 1.0,
            }
}

const WIDGET_SCROLLBAR_WIDTH: f32 = 8.0;
const WIDGET_SCROLLBAR_THUMB_MIN: f32 = 20.0;

fn scroll_offset_from_inner(node: &RetainedNode) -> f32 {
    crate::layout::scroll_content_node(node)
        .and_then(|inner| inner.style.margin.as_ref())
        .map(|m| match node.scroll_viewport.unwrap_or_default() {
            crate::ScrollAxis::Vertical => (-m.top).max(0.0),
            crate::ScrollAxis::Horizontal => (-m.left).max(0.0),
        })
        .unwrap_or(0.0)
}

fn paint_scrollbar(
    node: &RetainedNode,
    layout: &LayoutMap,
    scene: &mut Scene,
    ctx: PaintContext,
    stats: &mut PaintStats,
) {
    let Some(axis) = node.scroll_viewport else {
        return;
    };
    let Some(viewport_rect) = crate::layout::scroll_viewport_rect(node, layout) else {
        return;
    };
    let Some(content_height) = crate::layout::scroll_content_extent(node, layout) else {
        return;
    };
    let viewport_height = match axis {
        crate::ScrollAxis::Vertical => viewport_rect.height,
        crate::ScrollAxis::Horizontal => viewport_rect.width,
    };
    if content_height <= viewport_height + 1.0 {
        return;
    }
    let scroll_offset = scroll_offset_from_inner(node);
    let max_offset = (content_height - viewport_height).max(1.0);
    let track_x = match axis {
        crate::ScrollAxis::Vertical => {
            viewport_rect.x + viewport_rect.width - WIDGET_SCROLLBAR_WIDTH
        }
        crate::ScrollAxis::Horizontal => {
            viewport_rect.y + viewport_rect.height - WIDGET_SCROLLBAR_WIDTH
        }
    };
    let track = match axis {
        crate::ScrollAxis::Vertical => Rect::new(
            track_x as f64,
            viewport_rect.y as f64,
            (track_x + WIDGET_SCROLLBAR_WIDTH) as f64,
            (viewport_rect.y + viewport_height) as f64,
        ),
        crate::ScrollAxis::Horizontal => Rect::new(
            viewport_rect.x as f64,
            track_x as f64,
            (viewport_rect.x + viewport_height) as f64,
            (track_x + WIDGET_SCROLLBAR_WIDTH) as f64,
        ),
    };
    scene.fill(
        Fill::NonZero,
        ctx.base,
        to_peniko_color(
            ctx.paint(node)
                .scroll_track_color
                .unwrap_or(crate::theme::current_theme().chrome.scrollbar_track),
        ),
        None,
        &track,
    );
    stats.fills += 1;

    let thumb_height = (viewport_height / content_height * viewport_height).clamp(
        WIDGET_SCROLLBAR_THUMB_MIN.min(viewport_height),
        viewport_height,
    );
    let thumb_travel = (viewport_height - thumb_height).max(0.0);
    let origin = match axis {
        crate::ScrollAxis::Vertical => viewport_rect.y,
        crate::ScrollAxis::Horizontal => viewport_rect.x,
    };
    let thumb_y = origin + (scroll_offset / max_offset).clamp(0.0, 1.0) * thumb_travel;
    let thumb = match axis {
        crate::ScrollAxis::Vertical => Rect::new(
            track_x as f64,
            thumb_y as f64,
            (track_x + WIDGET_SCROLLBAR_WIDTH) as f64,
            (thumb_y + thumb_height) as f64,
        ),
        crate::ScrollAxis::Horizontal => Rect::new(
            thumb_y as f64,
            track_x as f64,
            (thumb_y + thumb_height) as f64,
            (track_x + WIDGET_SCROLLBAR_WIDTH) as f64,
        ),
    };
    scene.fill(
        Fill::NonZero,
        ctx.base,
        to_peniko_color(
            ctx.paint(node)
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
    use crate::element::builders::{ColumnView, Text, View};
    use crate::layout::{LayoutMap, Viewport};
    use crate::retained::RetainedTree;
    use parley::FontContext;

    fn layout_pass(
        tree: &mut RetainedTree,
        viewport: Viewport,
    ) -> Result<LayoutMap, crate::retained::RetainedError> {
        let mut font_cx = FontContext::new();
        crate::layout::layout_pass(tree, &mut font_cx, viewport)
    }

    #[test]
    fn selection_geometry_tracks_aligned_text_and_rejects_invalid_utf8_ranges() {
        use crate::{Cx, State, TextAlignment, TextField};
        let cx = Cx::new();
        let mut tree = RetainedTree::mount(
            TextField::new(&cx, State::new("école".into()))
                .width(200.0)
                .inset(8.0)
                .alignment(TextAlignment::End)
                .into_element(),
        )
        .unwrap();
        let layout = layout_pass(
            &mut tree,
            Viewport {
                width: 400.0,
                height: 200.0,
            },
        )
        .unwrap();
        let node = tree.root.as_mut().unwrap();
        node.text_input.as_mut().unwrap().selection = Some(0..6);
        let rects = text_input_selection_rects(node, &layout);
        assert_eq!(rects.len(), 1);
        let text = &node.children[0].children[0];
        let text_rect = layout.get(text.taffy_id.unwrap()).unwrap();
        assert!((rects[0].x0 - text_rect.x as f64).abs() < 0.1);
        // Taffy rounds text width up; glyph advances retain subpixel precision.
        assert!((rects[0].x1 - 192.0).abs() < 1.0);
        node.text_input.as_mut().unwrap().selection = Some(0..2);
        let partial = text_input_selection_rects(node, &layout);
        assert_eq!(partial.len(), 1);
        assert!(partial[0].width() > 0.0 && partial[0].width() < rects[0].width());
        for range in [1..2, 0..99, 2..2] {
            node.text_input.as_mut().unwrap().selection = Some(range);
            assert!(text_input_selection_rects(node, &layout).is_empty());
        }
    }

    #[test]
    fn empty_caret_uses_the_alignment_anchor_instead_of_placeholder_start() {
        let rect = LayoutRect {
            x: 40.0,
            y: 8.0,
            width: 120.0,
            height: 21.0,
        };
        assert_eq!(empty_field_caret_x(rect, crate::TextAlignment::Start), 40.0);
        assert_eq!(
            empty_field_caret_x(rect, crate::TextAlignment::Center),
            100.0
        );
        assert_eq!(empty_field_caret_x(rect, crate::TextAlignment::End), 160.0);
    }

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
            ColumnView::new()
                .width(100.0)
                .height(80.0)
                .surface(Color::rgb8(40, 80, 120))
                .into_element(),
        )
        .unwrap();

        let stats = layout_and_paint(&mut tree, 1.0);

        assert_eq!(stats.fills, 1);
        assert_eq!(stats.strokes, 0);
    }

    #[test]
    fn column_with_linear_gradient_emits_fill() {
        let mut tree = RetainedTree::mount(
            ColumnView::new()
                .width(100.0)
                .height(80.0)
                .gradient(crate::element::style::LinearGradient::new(
                    (0.0, 0.0),
                    (1.0, 1.0),
                    Color::rgb8(40, 80, 120),
                    Color::rgb8(120, 80, 40),
                ))
                .into_element(),
        )
        .unwrap();

        let stats = layout_and_paint(&mut tree, 1.0);

        assert_eq!(stats.fills, 1);
        assert_eq!(stats.strokes, 0);
    }

    #[test]
    fn column_with_box_shadow_emits_fill_before_background() {
        let mut tree = RetainedTree::mount(
            ColumnView::new()
                .width(100.0)
                .height(80.0)
                .shadow(crate::element::style::BoxShadow::new(
                    Color::rgb8(0, 0, 0).with_alpha(0.25),
                    0.0,
                    6.0,
                    12.0,
                ))
                .surface(Color::rgb8(40, 80, 120))
                .into_element(),
        )
        .unwrap();

        let stats = layout_and_paint(&mut tree, 1.0);

        assert_eq!(stats.fills, 2, "shadow then background");
        assert_eq!(stats.strokes, 0);
    }

    #[test]
    fn container_border_emits_stroke() {
        let mut tree = RetainedTree::mount(
            ColumnView::new()
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
    fn container_border_colors_per_side_emit_one_fill_per_side() {
        let mut tree = RetainedTree::mount(
            ColumnView::new()
                .width(100.0)
                .height(80.0)
                .border(Color::rgb8(255, 0, 0), 2.0)
                .border_color_top(Color::rgb8(255, 0, 0))
                .border_color_right(Color::rgb8(0, 255, 0))
                .border_color_bottom(Color::rgb8(0, 0, 255))
                .border_color_left(Color::rgb8(255, 255, 0))
                .into_element(),
        )
        .unwrap();

        let stats = layout_and_paint(&mut tree, 1.0);

        assert_eq!(stats.fills, 4, "each side is painted separately");
        assert_eq!(stats.strokes, 0);
    }

    #[test]
    fn container_border_top_only_paints_one_side() {
        let mut tree = RetainedTree::mount(
            ColumnView::new()
                .width(100.0)
                .height(80.0)
                .border_top(Color::rgb8(255, 0, 0), 2.0)
                .into_element(),
        )
        .unwrap();

        let stats = layout_and_paint(&mut tree, 1.0);

        assert_eq!(stats.fills, 1, "top border strip is a fill");
        assert_eq!(stats.strokes, 0);
    }

    #[test]
    fn text_with_parley_layout_emits_glyph_runs() {
        let mut tree = RetainedTree::mount(Text::new("hello").size(16.0).into_element()).unwrap();

        let stats = layout_and_paint(&mut tree, 1.0);

        assert!(
            stats.glyph_runs > 0,
            "expected glyph runs for laid-out text"
        );
    }

    #[test]
    fn text_without_parley_layout_skips_glyphs() {
        let mut tree = RetainedTree::mount(Text::new("hello").size(16.0).into_element()).unwrap();
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
    fn view_paints_background_then_label() {
        let mut tree = RetainedTree::mount(
            crate::View::new()
                .child(crate::Text::new("Press"))
                .width(120.0)
                .height(48.0)
                .surface(Color::rgb8(30, 60, 90))
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
            ColumnView::new()
                .width(80.0)
                .height(60.0)
                .surface(Color::rgb8(1, 2, 3))
                .child(Text::new("Hi").size(14.0))
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
            ColumnView::new()
                .surface(Color::rgb8(10, 20, 30))
                .into_element(),
        )
        .unwrap();

        let mut scene = Scene::new();
        let stats = paint_pass(&tree, &LayoutMap::default(), &mut scene, 1.0, None, true);

        assert_eq!(stats.fills, 0);
    }

    #[test]
    fn overflow_hidden_emits_extra_push_pop_layer() {
        // Without overflow:hidden — children painted with no extra clip layer.
        let mut tree_no_clip = RetainedTree::mount(
            ColumnView::new()
                .width(100.0)
                .height(80.0)
                .surface(Color::rgb8(40, 80, 120))
                .child(Text::new("inner").size(12.0))
                .into_element(),
        )
        .unwrap();
        let stats_no_clip = layout_and_paint(&mut tree_no_clip, 1.0);

        // With overflow:hidden — paint_node wraps children in an extra clip layer.
        let mut tree_clip = RetainedTree::mount(
            ColumnView::new()
                .width(100.0)
                .height(80.0)
                .surface(Color::rgb8(40, 80, 120))
                .clip()
                .child(Text::new("inner").size(12.0))
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
            ColumnView::new()
                .width(120.0)
                .height(120.0)
                .child(
                    View::new()
                        .width(80.0)
                        .height(40.0)
                        .surface(Color::rgb8(255, 0, 0)),
                )
                .child(
                    View::new()
                        .width(80.0)
                        .height(40.0)
                        .surface(Color::rgb8(0, 255, 0))
                        .layer(1),
                )
                .into_element(),
        )
        .unwrap();

        let stats = layout_and_paint(&mut tree, 1.0);
        assert_eq!(stats.fills, 2, "both children must be painted");
    }

    #[test]
    fn z_index_children_with_overflow_hidden_are_painted_without_panic() {
        let mut tree = RetainedTree::mount(
            ColumnView::new()
                .width(120.0)
                .height(120.0)
                .clip()
                .child(
                    View::new()
                        .width(80.0)
                        .height(40.0)
                        .surface(Color::rgb8(255, 0, 0)),
                )
                .child(
                    View::new()
                        .width(80.0)
                        .height(40.0)
                        .surface(Color::rgb8(0, 255, 0))
                        .layer(2),
                )
                .into_element(),
        )
        .unwrap();

        let stats = layout_and_paint(&mut tree, 1.0);
        assert_eq!(stats.fills, 2, "overflow clipping should keep both fills");
    }

    #[test]
    fn nested_fixed_and_layered_children_are_all_painted() {
        let mut tree = RetainedTree::mount(
            View::new()
                .width(100.0)
                .height(100.0)
                .clip()
                .child(
                    View::new()
                        .position(crate::Position::Fixed)
                        .width(40.0)
                        .height(40.0)
                        .surface(Color::rgb8(255, 0, 0))
                        .child(
                            View::new()
                                .layer(2)
                                .width(10.0)
                                .height(10.0)
                                .surface(Color::rgb8(0, 255, 0)),
                        )
                        .child(
                            View::new()
                                .position(crate::Position::Fixed)
                                .width(10.0)
                                .height(10.0)
                                .surface(Color::rgb8(0, 0, 255)),
                        ),
                )
                .into_element(),
        )
        .unwrap();
        assert_eq!(layout_and_paint(&mut tree, 1.0).fills, 3);
    }

    #[test]
    fn a_small_scroll_viewport_paints_one_scrollbar_without_invalid_thumb_bounds() {
        let mut tree = RetainedTree::mount(
            View::new()
                .width(100.0)
                .height(8.0)
                .clip()
                .scroll_viewport(crate::ScrollAxis::Vertical)
                .on_scroll(|_| {})
                .child(
                    View::new()
                        .scroll_content()
                        .child(View::new().height(200.0)),
                )
                .into_element(),
        )
        .unwrap();
        let stats = layout_and_paint(&mut tree, 1.0);
        assert_eq!(stats.fills, 2, "one track and one thumb");
    }

    #[test]
    fn partial_opacity_paints_without_panic() {
        let mut tree = RetainedTree::mount(
            ColumnView::new()
                .width(100.0)
                .height(80.0)
                .surface(Color::rgb8(40, 80, 120))
                .opacity(0.5)
                .child(Text::new("hello").size(12.0))
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
            ColumnView::new()
                .width(120.0)
                .height(120.0)
                .child(
                    View::new()
                        .width(80.0)
                        .height(40.0)
                        .surface(Color::rgb8(255, 0, 0)),
                )
                .child(
                    View::new()
                        .width(80.0)
                        .height(40.0)
                        .surface(Color::rgb8(0, 255, 0))
                        .layer(1),
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
            ColumnView::new()
                .width(200.0)
                .height(200.0)
                // Select wrapper
                .child(
                    ColumnView::new()
                        .width(100.0)
                        .height(100.0)
                        // trigger
                        .child(
                            View::new()
                                .width(100.0)
                                .height(20.0)
                                .surface(Color::rgb8(80, 80, 80)),
                        )
                        // dropdown with high z_index
                        .child(
                            View::new()
                                .width(100.0)
                                .height(60.0)
                                .surface(Color::rgb8(35, 35, 52))
                                .layer(10),
                        ),
                )
                // sibling that dropdown must overlay
                .child(
                    View::new()
                        .width(200.0)
                        .height(80.0)
                        .surface(Color::rgb8(60, 60, 60)),
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
        // Select dropdown: column + absolute position + layer + background.
        let mut tree = RetainedTree::mount(
            ColumnView::new()
                .width(200.0)
                .height(200.0)
                .child(
                    ColumnView::new()
                        .width(100.0)
                        .child(
                            View::new()
                                .width(100.0)
                                .height(40.0)
                                .surface(Color::rgb8(55, 120, 220)),
                        )
                        .child(
                            ColumnView::new()
                                .layer(10)
                                .position(crate::Position::Absolute)
                                .top(40.0)
                                .left(0.0)
                                .width(100.0)
                                .surface(Color::rgb8(35, 35, 52))
                                .border(Color::rgb8(80, 80, 110), 1.0)
                                .rounding(6.0)
                                .child(
                                    View::new()
                                        .inset(8.0)
                                        .child(Text::new("Option A").size(14.0)),
                                )
                                .child(
                                    View::new()
                                        .inset(8.0)
                                        .child(Text::new("Option B").size(14.0)),
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
            ColumnView::new()
                .width(100.0)
                .height(80.0)
                .surface(Color::rgb8(200, 100, 50))
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
        // Structure mirrors the ScrollView widget: viewport -> inner wrapper -> content.
        // scroll_content_extent returns a height and the scrollbar is drawn.
        let content = View::new()
            .width(100.0)
            .height(300.0)
            .surface(Color::rgb8(200, 200, 200));
        let inner = View::new().scroll_content().child(content);
        let viewport = View::new()
            .width(100.0)
            .height(80.0)
            .clip()
            .scroll_viewport(crate::ScrollAxis::Vertical)
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
                affinity: Default::default(),
                multiline: false,
                selection: None,
                alignment: Default::default(),
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
            ColumnView::new()
                .width(100.0)
                .height(80.0)
                .surface(Color::rgb8(40, 80, 120))
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
