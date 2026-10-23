use std::rc::Rc;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Edges<T> {
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

impl Edges<f32> {
    /// Returns true if any side has a width greater than zero.
    pub fn any_positive(&self) -> bool {
        self.top > 0.0 || self.right > 0.0 || self.bottom > 0.0 || self.left > 0.0
    }

    /// Returns true when all four sides share the same width.
    pub fn is_uniform(&self) -> bool {
        self.top == self.right && self.right == self.bottom && self.bottom == self.left
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

/// How a view participates in layout and scrolling.
///
/// Edge offsets use logical points through `.top()`, `.right()`, `.bottom()` and
/// `.left()`. Fixed views escape ancestor clipping; sticky views stay inside their
/// parent and the nearest clipped scroll region (or window when none exists).
///
/// ```
/// use lemon::{Position, View};
/// let badge = View::new().position(Position::Fixed).top(16.0).right(16.0)
///     .width(80.0).height(32.0);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Position {
    /// Normal flow; offsets move the view without changing sibling placement.
    #[default]
    Relative,
    /// Outside normal flow, positioned against the immediate layout parent.
    Absolute,
    /// Outside normal flow, positioned and sized against the window viewport.
    /// Missing offsets anchor at the viewport's top-left corner.
    Fixed,
    /// In normal flow, clamped by edge offsets while its scroll region moves.
    /// The view cannot leave its parent's bounds; unset edges do not constrain it.
    Sticky,
}

/// Position of a container's children within its available space.
///
/// Positions use the visible horizontal/vertical directions, independent of whether
/// the container is a [`RowView`](crate::RowView), [`ColumnView`](crate::ColumnView), or [`View`](crate::View).
/// Placement does not change the container's size; use `.fill()` to request parent space.
///
/// ```
/// use lemon::{ColumnView, Placement, Text};
/// let panel = ColumnView::new().fill().place(Placement::BottomRight).child(Text::new("Status"));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Placement {
    /// Upper-left corner.
    TopLeft,
    /// Top edge, horizontally centered.
    Top,
    /// Upper-right corner.
    TopRight,
    /// Left edge, vertically centered.
    Left,
    /// Centered horizontally and vertically.
    #[default]
    Center,
    /// Right edge, vertically centered.
    Right,
    /// Lower-left corner.
    BottomLeft,
    /// Bottom edge, horizontally centered.
    Bottom,
    /// Lower-right corner.
    BottomRight,
}

impl Placement {
    pub(crate) fn axes(self, vertical: bool) -> (Align, Distribute) {
        let (horizontal, vertical_alignment) = match self {
            Self::TopLeft => (0, 0),
            Self::Top => (1, 0),
            Self::TopRight => (2, 0),
            Self::Left => (0, 1),
            Self::Center => (1, 1),
            Self::Right => (2, 1),
            Self::BottomLeft => (0, 2),
            Self::Bottom => (1, 2),
            Self::BottomRight => (2, 2),
        };
        let (cross, main) = if vertical {
            (horizontal, vertical_alignment)
        } else {
            (vertical_alignment, horizontal)
        };
        let align = match cross {
            0 => Align::Start,
            1 => Align::Center,
            _ => Align::End,
        };
        let justify = match main {
            0 => Distribute::Start,
            1 => Distribute::Center,
            _ => Distribute::End,
        };
        (align, justify)
    }
}

/// How children share spare space along their container's stacking direction.
///
/// Rows distribute horizontally; columns distribute vertically. This leaves the
/// perpendicular alignment unchanged. Use `.place(...)` for physical positioning.
///
/// ```
/// use lemon::{Distribute, RowView, Text};
/// let header = RowView::new().fill_width().distribute(Distribute::Between)
///     .child(Text::new("Left")).child(Text::new("Right"));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Distribute {
    /// Group children at the start of the stacking direction.
    #[default]
    Start,
    /// Group children in the center.
    Center,
    /// Group children at the end.
    End,
    /// Put spare space between children, with none at the outer edges.
    Between,
    /// Put space around each child, with half-sized gaps at the outer edges.
    Around,
    /// Put equal gaps between children and at both outer edges.
    Evenly,
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
/// App code usually sets these through builder methods ([`ColumnView::spacing`](crate::element::builders::ColumnView::spacing), etc.)
/// rather than constructing `StyleProps` directly.
#[derive(Clone, Debug, PartialEq)]
pub struct StyleProps {
    /// Whether this subtree accepts pointer, keyboard and focus input (default: true).
    /// Disabling input preserves layout and paint, allowing an overlay to fade out safely.
    pub interactive: bool,
    /// Implicit visual transition for this subtree; absent by default.
    pub transition: Option<crate::transition::Transition>,
    pub width: Option<Dimension>,
    pub height: Option<Dimension>,
    pub padding: Option<Edges<f32>>,
    pub margin: Option<Edges<f32>>,
    /// Positioning edge offsets in logical points; unset edges are automatic.
    ///
    /// Edge offsets follow [`position`](Self::position); unset edges remain automatic. Use builder methods such
    /// as [`.top()`](crate::element::builders::View::top) and
    /// [`.left()`](crate::element::builders::View::left) rather than setting this field directly.
    pub inset: Option<Edges<Option<f32>>>,
    pub gap: Option<f32>,
    /// Per-axis gap overrides. When set, take precedence over [`gap`](Self::gap) for that axis.
    ///
    /// `column_gap` is the gap between columns (Taffy `gap.width`).
    /// `row_gap` is the gap between rows (Taffy `gap.height`).
    /// Use the builder methods [`column_gap`](crate::element::builders::ColumnView::horizontal_spacing) and
    /// [`row_gap`](crate::element::builders::ColumnView::vertical_spacing) rather than setting these directly.
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
    pub justify_content: Option<Distribute>,
    pub overflow: Overflow,
    /// Marks the offset wrapper of a scroll region; defaults to `false`.
    ///
    /// Its children are the measured content. This semantic marker lets layout
    /// and paint find scroll content without depending on wrapper depth.
    pub scroll_content: bool,
    /// Paint-only z-order for this node among siblings (`0` = normal flow).
    ///
    /// Layout is unaffected. During painting, `z_index == 0` nodes are painted first in normal
    /// traversal order; non-zero nodes are deferred and then painted in ascending `z_index` order.
    pub z_index: i32,
    /// Cross-axis alignment when this node is a flex child (e.g. avoid stretch in a column).
    pub align_self: Option<Align>,
    pub focusable: bool,
    pub cursor: crate::element::events::Cursor,
    /// Positioning mode; defaults to normal-flow [`Position::Relative`].
    pub position: Position,
}

impl Default for StyleProps {
    fn default() -> Self {
        Self {
            interactive: true,
            transition: None,
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
            scroll_content: false,
            z_index: 0,
            align_self: None,
            focusable: false,
            cursor: crate::element::events::Cursor::default(),
            position: Position::Relative,
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

/// Two-stop linear gradient painted within a container's layout rect.
///
/// `start` and `end` use normalized coordinates where `(0.0, 0.0)` is the top-left corner and
/// `(1.0, 1.0)` is the bottom-right corner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinearGradient {
    pub start: (f32, f32),
    pub end: (f32, f32),
    pub start_color: Color,
    pub end_color: Color,
}

impl LinearGradient {
    /// Creates a two-stop linear gradient inside the painted rect.
    ///
    /// ```
    /// use lemon::{Color, LinearGradient, View};
    ///
    /// let _ = View::new()
    ///     .gradient(LinearGradient::new(
    ///         (0.0, 0.0),
    ///         (1.0, 1.0),
    ///         Color::rgb8(255, 128, 64),
    ///         Color::rgb8(64, 128, 255),
    ///     ))
    ///     .into_element();
    /// ```
    pub fn new(start: (f32, f32), end: (f32, f32), start_color: Color, end_color: Color) -> Self {
        Self {
            start,
            end,
            start_color,
            end_color,
        }
    }
}

/// Single outer box shadow painted behind a container.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxShadow {
    /// Shadow tint and alpha.
    pub color: Color,
    /// Horizontal shadow offset in logical points.
    pub offset_x: f32,
    /// Vertical shadow offset in logical points.
    pub offset_y: f32,
    /// Gaussian blur radius in logical points.
    pub blur_radius: f32,
}

impl BoxShadow {
    /// Creates a blurred outer shadow.
    ///
    /// ```
    /// use lemon::{BoxShadow, Color, View};
    ///
    /// let _ = View::new()
    ///     .shadow(BoxShadow::new(
    ///         Color::rgb8(0, 0, 0).with_alpha(0.25),
    ///         0.0,
    ///         8.0,
    ///         12.0,
    ///     ))
    ///     .into_element();
    /// ```
    pub fn new(color: Color, offset_x: f32, offset_y: f32, blur_radius: f32) -> Self {
        Self {
            color,
            offset_x,
            offset_y,
            blur_radius,
        }
    }
}

/// Visual decoration properties. May contain dynamic closures.
#[derive(Clone, Default)]
pub struct PaintProps {
    pub background: Option<ColorSource>,
    /// Optional two-stop linear gradient painted instead of [`background`](Self::background).
    pub background_gradient: Option<LinearGradient>,
    /// Per-side border colors. `None` skips painting for that side.
    pub border_color: Edges<Option<ColorSource>>,
    /// Per-side border widths in logical points (`0.0` = no border on that side).
    pub border_width: Edges<f32>,
    pub radius: CornerRadii,
    /// Optional outer shadow painted before the background and borders.
    pub box_shadow: Option<BoxShadow>,
    /// Optional image drawn inside the container using object-fit: contain scaling.
    pub image: Option<crate::asset::ImageHandle>,
    /// Optional vector icon drawn within this container using contain scaling.
    pub icon: Option<crate::icons::IconPaint>,
    /// Optional override for widget scrollbar track painting.
    pub scroll_track_color: Option<ColorSource>,
    /// Optional override for widget scrollbar thumb painting.
    pub scroll_thumb_color: Option<ColorSource>,
    /// Focus ring color. Setting it on a focusable view enables focus chrome;
    /// text inputs also use the current theme when no override is provided.
    pub focus_ring_color: Option<ColorSource>,
}

impl std::fmt::Debug for PaintProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PaintProps")
            .field("background", &self.background.as_ref().map(|c| c.resolve()))
            .field("background_gradient", &self.background_gradient)
            .field("border_color", &resolve_color_edges(&self.border_color))
            .field("border_width", &self.border_width)
            .field("radius", &self.radius)
            .field("box_shadow", &self.box_shadow)
            .field("image", &self.image)
            .field("icon", &self.icon)
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
    pub background_gradient: Option<LinearGradient>,
    pub border_color: Edges<Option<Color>>,
    pub border_width: Edges<f32>,
    pub radius: CornerRadii,
    pub box_shadow: Option<BoxShadow>,
    /// Optional image drawn inside the container using object-fit: contain scaling.
    pub image: Option<crate::asset::ImageHandle>,
    /// Optional vector icon drawn within this container using contain scaling.
    pub icon: Option<crate::icons::IconPaint>,
    pub scroll_track_color: Option<Color>,
    pub scroll_thumb_color: Option<Color>,
    pub focus_ring_color: Option<Color>,
}

impl PaintProps {
    pub fn resolve(&self) -> PaintData {
        PaintData {
            background: self.background.as_ref().map(|c| c.resolve()),
            background_gradient: self.background_gradient,
            border_color: resolve_color_edges(&self.border_color),
            border_width: self.border_width,
            radius: self.radius.clone(),
            box_shadow: self.box_shadow,
            image: self.image.clone(),
            icon: self.icon.clone(),
            scroll_track_color: self.scroll_track_color.as_ref().map(|c| c.resolve()),
            scroll_thumb_color: self.scroll_thumb_color.as_ref().map(|c| c.resolve()),
            focus_ring_color: self.focus_ring_color.as_ref().map(|c| c.resolve()),
        }
    }
}

fn resolve_color_edges(edges: &Edges<Option<ColorSource>>) -> Edges<Option<Color>> {
    Edges {
        top: edges.top.as_ref().map(ColorSource::resolve),
        right: edges.right.as_ref().map(ColorSource::resolve),
        bottom: edges.bottom.as_ref().map(ColorSource::resolve),
        left: edges.left.as_ref().map(ColorSource::resolve),
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

impl TextStyle {
    pub(crate) fn resolved_font_size(&self) -> f32 {
        if self.font_size > 0.0 {
            self.font_size
        } else {
            crate::theme::current_theme().typography.font_size_md
        }
    }
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
