use std::rc::Rc;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Edges<T: Clone> {
    pub top: T,
    pub right: T,
    pub bottom: T,
    pub left: T,
}

impl<T: Clone> Edges<T> {
    pub fn all(v: T) -> Self {
        Edges {
            top: v.clone(),
            right: v.clone(),
            bottom: v.clone(),
            left: v,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Dimension {
    #[default]
    Auto,
    Points(f32),
    Percent(f32),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Align {
    #[default]
    Stretch,
    Start,
    End,
    Center,
    Baseline,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Justify {
    #[default]
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

/// How children are clipped when they exceed a container’s bounds.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Overflow {
    /// Children may paint outside the box (default).
    #[default]
    Visible,
    /// Clip painting to the container’s border box.
    Hidden,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CornerRadii {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl CornerRadii {
    pub fn all(r: f32) -> Self {
        CornerRadii {
            top_left: r,
            top_right: r,
            bottom_right: r,
            bottom_left: r,
        }
    }
}

/// Low-level layout and interaction fields for a node.
///
/// App code usually sets these through builder methods ([`Column::gap`](crate::element::builders::Column::gap), etc.)
/// rather than constructing `StyleProps` directly.
#[derive(Clone, Debug, PartialEq)]
pub struct StyleProps {
    pub width: Option<Dimension>,
    pub height: Option<Dimension>,
    pub padding: Option<Edges<f32>>,
    pub margin: Option<Edges<f32>>,
    /// Absolute or relative inset offsets in logical points.
    ///
    /// For nodes created with [`.absolute()`](crate::element::builders::View::absolute), these
    /// values position the node relative to its containing flex ancestor. Use builder methods such
    /// as [`.top()`](crate::element::builders::View::top) and
    /// [`.left()`](crate::element::builders::View::left) rather than setting this field directly.
    pub inset: Option<Edges<f32>>,
    pub gap: Option<f32>,
    /// Per-axis gap overrides. When set, take precedence over [`gap`](Self::gap) for that axis.
    ///
    /// `column_gap` is the gap between columns (Taffy `gap.width`).
    /// `row_gap` is the gap between rows (Taffy `gap.height`).
    /// Use the builder methods [`column_gap`](crate::element::builders::Column::column_gap) and
    /// [`row_gap`](crate::element::builders::Column::row_gap) rather than setting these directly.
    pub column_gap: Option<f32>,
    pub row_gap: Option<f32>,
    /// Opacity multiplier applied to this node's painted output and descendants.
    ///
    /// `0.0` is fully transparent and `1.0` is fully opaque.
    /// Container builders clamp values to the valid `0.0..=1.0` range.
    pub opacity: f32,
    pub flex_grow: Option<f32>,
    pub flex_shrink: Option<f32>,
    pub align_items: Option<Align>,
    pub justify_content: Option<Justify>,
    pub overflow: Overflow,
    /// Paint-only z-order for this node among siblings (`0` = normal flow).
    ///
    /// Layout is unaffected. During painting, `z_index == 0` nodes are painted first in normal
    /// traversal order; non-zero nodes are deferred and then painted in ascending `z_index` order.
    pub z_index: i32,
    /// Cross-axis alignment when this node is a flex child (e.g. avoid stretch in a column).
    pub align_self: Option<Align>,
    pub focusable: bool,
    pub cursor: crate::element::events::Cursor,
    /// When `true` the node is removed from normal flow and positioned by Taffy's
    /// absolute-position algorithm relative to its nearest flex ancestor.
    ///
    /// Use the builder method [`.absolute()`](crate::element::builders::View::absolute) rather
    /// than setting this field directly.
    pub position_absolute: bool,
}

impl Default for StyleProps {
    fn default() -> Self {
        Self {
            width: None,
            height: None,
            padding: None,
            margin: None,
            inset: None,
            gap: None,
            column_gap: None,
            row_gap: None,
            opacity: 1.0,
            flex_grow: None,
            flex_shrink: None,
            align_items: None,
            justify_content: None,
            overflow: Overflow::Visible,
            z_index: 0,
            align_self: None,
            focusable: false,
            cursor: crate::element::events::Cursor::default(),
            position_absolute: false,
        }
    }
}

/// sRGB color with components in `0.0..=1.0`.
///
/// Prefer [`Color::rgb8`] for byte values from design tools.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    /// Builds an opaque color from 8-bit sRGB channels (`0..=255`).
    pub const fn rgb8(r: u8, g: u8, b: u8) -> Self {
        Color {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: 1.0,
        }
    }
    /// Sets alpha (`0.0` = transparent, `1.0` = opaque).
    pub fn with_alpha(mut self, a: f32) -> Self {
        self.a = a;
        self
    }

    /// Converts to 8-bit sRGB channels for GPU clear colors and similar APIs.
    pub fn to_rgb8(self) -> (u8, u8, u8) {
        (
            (self.r.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.g.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.b.clamp(0.0, 1.0) * 255.0).round() as u8,
        )
    }
}

/// A color that may be evaluated dynamically from a closure.
#[derive(Clone)]
pub enum ColorSource {
    Static(Color),
    Dynamic(Rc<dyn Fn() -> Color>),
}

impl ColorSource {
    pub fn resolve(&self) -> Color {
        match self {
            Self::Static(c) => *c,
            Self::Dynamic(f) => f(),
        }
    }
}

impl From<Color> for ColorSource {
    fn from(c: Color) -> Self {
        ColorSource::Static(c)
    }
}

impl<F: Fn() -> Color + 'static> From<F> for ColorSource {
    fn from(f: F) -> Self {
        ColorSource::Dynamic(Rc::new(f))
    }
}

/// Visual decoration properties. May contain dynamic closures.
#[derive(Clone)]
pub struct PaintProps {
    pub background: Option<ColorSource>,
    pub border_color: Option<ColorSource>,
    pub border_width: f32,
    pub radius: CornerRadii,
    /// Optional image drawn inside the container using object-fit: contain scaling.
    pub image: Option<crate::asset::ImageHandle>,
    /// Optional override for widget scrollbar track painting.
    pub scroll_track_color: Option<ColorSource>,
    /// Optional override for widget scrollbar thumb painting.
    pub scroll_thumb_color: Option<ColorSource>,
    /// Optional override for text-input focus ring painting.
    pub focus_ring_color: Option<ColorSource>,
}

impl Default for PaintProps {
    fn default() -> Self {
        PaintProps {
            background: None,
            border_color: None,
            border_width: 0.0,
            radius: CornerRadii::default(),
            image: None,
            scroll_track_color: None,
            scroll_thumb_color: None,
            focus_ring_color: None,
        }
    }
}

impl std::fmt::Debug for PaintProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PaintProps")
            .field("background", &self.background.as_ref().map(|c| c.resolve()))
            .field(
                "border_color",
                &self.border_color.as_ref().map(|c| c.resolve()),
            )
            .field("border_width", &self.border_width)
            .field("radius", &self.radius)
            .field("image", &self.image)
            .field(
                "scroll_track_color",
                &self.scroll_track_color.as_ref().map(|c| c.resolve()),
            )
            .field(
                "scroll_thumb_color",
                &self.scroll_thumb_color.as_ref().map(|c| c.resolve()),
            )
            .field(
                "focus_ring_color",
                &self.focus_ring_color.as_ref().map(|c| c.resolve()),
            )
            .finish()
    }
}

/// Resolved paint values with no closures — stored in Retained Tree and Patches.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PaintData {
    pub background: Option<Color>,
    pub border_color: Option<Color>,
    pub border_width: f32,
    pub radius: CornerRadii,
    /// Optional image drawn inside the container using object-fit: contain scaling.
    pub image: Option<crate::asset::ImageHandle>,
    pub scroll_track_color: Option<Color>,
    pub scroll_thumb_color: Option<Color>,
    pub focus_ring_color: Option<Color>,
}

impl PaintProps {
    pub fn resolve(&self) -> PaintData {
        PaintData {
            background: self.background.as_ref().map(|c| c.resolve()),
            border_color: self.border_color.as_ref().map(|c| c.resolve()),
            border_width: self.border_width,
            radius: self.radius.clone(),
            image: self.image.clone(),
            scroll_track_color: self.scroll_track_color.as_ref().map(|c| c.resolve()),
            scroll_thumb_color: self.scroll_thumb_color.as_ref().map(|c| c.resolve()),
            focus_ring_color: self.focus_ring_color.as_ref().map(|c| c.resolve()),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
    pub font_size: f32,
    pub font_weight: u16,
    pub font_family: String,
    pub line_height: f32,
    pub letter_spacing: f32,
    pub color: Option<Color>,
}

impl Default for TextStyle {
    fn default() -> Self {
        let typography = crate::theme::current_theme().typography;
        TextStyle {
            font_size: typography.font_size_md,
            font_weight: 400,
            font_family: typography.font_family,
            line_height: typography.line_height,
            letter_spacing: typography.letter_spacing,
            color: None,
        }
    }
}

/// Foreground used when a text node has no explicit color.
pub fn default_text_color() -> Color {
    crate::theme::current_theme().colors.foreground
}

/// Resolved paint color for a text node (explicit style or theme foreground).
pub fn resolved_text_color(style: &TextStyle) -> Color {
    style.color.unwrap_or_else(default_text_color)
}

#[cfg(test)]
mod tests {
    use super::{resolved_text_color, Color, Overflow, StyleProps, TextStyle};
    use crate::theme::{set_active_theme, Theme};

    #[test]
    fn resolved_text_color_prefers_explicit_style() {
        let style = TextStyle {
            color: Some(Color::rgb8(255, 128, 64)),
            ..TextStyle::default()
        };
        assert_eq!(resolved_text_color(&style), Color::rgb8(255, 128, 64));
    }

    #[test]
    fn resolved_text_color_falls_back_to_theme_foreground() {
        let original = crate::theme::current_theme();
        let mut theme = Theme::default_dark();
        theme.colors.foreground = Color::rgb8(200, 210, 220);
        set_active_theme(theme);

        let style = TextStyle::default();
        assert_eq!(resolved_text_color(&style), Color::rgb8(200, 210, 220));

        set_active_theme(original);
    }

    #[test]
    fn style_props_default_overflow_is_visible() {
        assert_eq!(StyleProps::default().overflow, Overflow::Visible);
    }

    #[test]
    fn style_props_default_z_index_is_zero() {
        assert_eq!(StyleProps::default().z_index, 0);
    }
}
