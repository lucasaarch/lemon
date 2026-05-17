use crate::layout::{NodeCtx, NodeKind, NodeRect, TextRole, UiLayout};
use parley::{
    Alignment, AlignmentOptions, FontContext, GenericFamily, LayoutContext, PositionedLayoutItem,
    StyleProperty,
};
use vello::kurbo::{Affine, RoundedRect, Stroke};
use vello::peniko::color::palette;
use vello::peniko::{Brush, Color, Fill};
use vello::{Glyph, Scene};

const BG: Color = Color::from_rgb8(18, 18, 22);
const SURFACE: Color = Color::from_rgb8(32, 34, 42);
const SURFACE_ELEVATED: Color = Color::from_rgb8(44, 47, 58);
const BORDER: Color = Color::from_rgb8(68, 72, 88);
const ACCENT: Color = Color::from_rgb8(255, 196, 72);
const TEXT_PRIMARY: Brush = Brush::Solid(palette::css::WHITE);
const TEXT_MUTED: Brush = Brush::Solid(Color::from_rgb8(160, 166, 184));

/// Tamanhos em pontos lógicos (como o macOS mede fontes).
pub mod type_scale {
    pub const CAPTION: f32 = 11.0;
    pub const BODY: f32 = 14.0;
    pub const HEADLINE: f32 = 17.0;
}

pub struct UiState {
    pub font_cx: FontContext,
    pub layout_cx: LayoutContext<Brush>,
    pub tree: UiLayout,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            font_cx: FontContext::new(),
            layout_cx: LayoutContext::new(),
            tree: UiLayout::new(),
        }
    }

    pub fn draw(&mut self, scene: &mut Scene, width: f32, height: f32, scale_factor: f32) {
        self.tree.compute(
            &mut self.font_cx,
            &mut self.layout_cx,
            width,
            height,
            scale_factor,
        );

        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            BG,
            None,
            &vello::kurbo::Rect::new(0.0, 0.0, width as f64, height as f64),
        );

        let root = self.tree.root;
        let mut nodes = Vec::new();
        self.tree.visit(root, taffy::geometry::Point::ZERO, &mut |_, ctx, rect| {
            nodes.push((*ctx, rect));
        });
        for (ctx, rect) in nodes {
            self.paint_node(scene, scale_factor, &ctx, rect);
        }
    }

    fn paint_node(
        &mut self,
        scene: &mut Scene,
        scale_factor: f32,
        ctx: &NodeCtx,
        rect: NodeRect,
    ) {
        let x = rect.x as f64;
        let y = rect.y as f64;
        let w = rect.width as f64;
        let h = rect.height as f64;

        match ctx.kind {
            NodeKind::PassThrough => {}
            NodeKind::Sidebar => {
                fill_rounded(
                    scene,
                    RoundedRect::new(x, y, x + w, y + h, 14.0),
                    SURFACE,
                );
                stroke_rounded(
                    scene,
                    RoundedRect::new(x, y, x + w, y + h, 14.0),
                    BORDER,
                    1.0,
                );
            }
            NodeKind::NavActive => {
                fill_rounded(
                    scene,
                    RoundedRect::new(x, y, x + w, y + h, 4.0),
                    ACCENT,
                );
            }
            NodeKind::NavItem => {
                fill_rounded(
                    scene,
                    RoundedRect::new(x, y, x + w, y + h, 6.0),
                    SURFACE_ELEVATED,
                );
            }
            NodeKind::Card => {
                fill_rounded(
                    scene,
                    RoundedRect::new(x, y, x + w, y + h, 18.0),
                    SURFACE,
                );
                stroke_rounded(
                    scene,
                    RoundedRect::new(x, y, x + w, y + h, 18.0),
                    BORDER,
                    1.5,
                );
            }
            NodeKind::Header => {
                fill_rounded(
                    scene,
                    RoundedRect::new(x, y, x + w, y + h, (18.0, 18.0, 0.0, 0.0)),
                    SURFACE_ELEVATED,
                );
            }
            NodeKind::Chip => {
                let chip = RoundedRect::new(x, y, x + w, y + h, 10.0);
                fill_rounded(
                    scene,
                    chip,
                    Color::from_rgb8(72, 132, 255).with_alpha(0.18),
                );
                stroke_rounded(
                    scene,
                    chip,
                    Color::from_rgb8(120, 168, 255),
                    1.0,
                );
            }
            NodeKind::Panel => {
                let panel = RoundedRect::new(x, y, x + w, y + h, 12.0);
                fill_rounded(scene, panel, SURFACE_ELEVATED);
                stroke_rounded(scene, panel, BORDER, 1.0);
            }
            NodeKind::RectButton => {
                let btn = vello::kurbo::Rect::new(x, y, x + w, y + h);
                scene.fill(Fill::NonZero, Affine::IDENTITY, SURFACE_ELEVATED, None, &btn);
                scene.stroke(
                    &Stroke::new(1.0),
                    Affine::IDENTITY,
                    BORDER,
                    None,
                    &btn,
                );
            }
            NodeKind::RoundButton => {
                fill_rounded(
                    scene,
                    RoundedRect::new(x, y, x + w, y + h, 12.0),
                    ACCENT.with_alpha(0.9),
                );
            }
            NodeKind::Text(role) => {
                let spec = role.spec();
                let (brush, pad_x, pad_y) = match role {
                    TextRole::Title => (TEXT_PRIMARY, 0.0, 0.0),
                    TextRole::Pill => (Brush::Solid(ACCENT), 0.0, 0.0),
                    TextRole::Chip => (
                        Brush::Solid(Color::from_rgb8(140, 190, 255)),
                        0.0,
                        0.0,
                    ),
                    TextRole::Body => (TEXT_MUTED, 0.0, 0.0),
                    TextRole::RectButton => (TEXT_PRIMARY, 0.0, 0.0),
                    TextRole::RoundButton => (
                        Brush::Solid(Color::from_rgb8(28, 24, 12)),
                        0.0,
                        0.0,
                    ),
                };
                let max_width = if matches!(role, TextRole::Body) {
                    Some((w - pad_x * 2.0) as f32)
                } else {
                    None
                };
                self.draw_text(
                    scene,
                    scale_factor,
                    spec.text,
                    x + pad_x,
                    y + pad_y,
                    spec.font_size,
                    spec.weight,
                    brush,
                    max_width,
                );
            }
        }
    }

    fn draw_text(
        &mut self,
        scene: &mut Scene,
        scale_factor: f32,
        text: &str,
        x: f64,
        y: f64,
        font_size: f32,
        weight: parley::FontWeight,
        brush: Brush,
        max_width: Option<f32>,
    ) {
        let mut builder = self.layout_cx.ranged_builder(
            &mut self.font_cx,
            text,
            scale_factor,
            true,
        );
        builder.push_default(GenericFamily::SystemUi);
        builder.push_default(StyleProperty::FontSize(font_size));
        builder.push_default(StyleProperty::FontWeight(weight));
        builder.push_default(StyleProperty::Brush(brush));
        let mut layout = builder.build(text);
        layout.break_all_lines(max_width);
        layout.align(Alignment::Start, AlignmentOptions::default());

        let transform = Affine::translate((x, y));
        for line in layout.lines() {
            for item in line.items() {
                let PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                    continue;
                };
                let style = glyph_run.style();
                let mut gx = glyph_run.offset();
                let gy = glyph_run.baseline();
                let run = glyph_run.run();
                let font = run.font();
                let synthesis = run.synthesis();
                let glyph_xform = synthesis
                    .skew()
                    .map(|angle| Affine::skew(angle.to_radians().tan() as f64, 0.0));

                scene
                    .draw_glyphs(font)
                    .brush(&style.brush)
                    .hint(true)
                    .transform(transform)
                    .glyph_transform(glyph_xform)
                    .font_size(run.font_size())
                    .normalized_coords(run.normalized_coords())
                    .draw(
                        Fill::NonZero,
                        glyph_run.glyphs().map(|glyph| {
                            let x = gx + glyph.x;
                            let y = gy + glyph.y;
                            gx += glyph.advance;
                            Glyph {
                                id: glyph.id,
                                x,
                                y,
                            }
                        }),
                    );
            }
        }
    }
}

fn fill_rounded(scene: &mut Scene, shape: RoundedRect, color: Color) {
    scene.fill(Fill::NonZero, Affine::IDENTITY, color, None, &shape);
}

fn stroke_rounded(scene: &mut Scene, shape: RoundedRect, color: Color, width: f64) {
    scene.stroke(
        &Stroke::new(width),
        Affine::IDENTITY,
        color,
        None,
        &shape,
    );
}
