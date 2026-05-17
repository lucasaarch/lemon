#[derive(Clone, Debug, Default, PartialEq)]
pub struct Edges<T: Clone> {
    pub top: T,
    pub right: T,
    pub bottom: T,
    pub left: T,
}

impl<T: Clone> Edges<T> {
    pub fn all(v: T) -> Self {
        Edges { top: v.clone(), right: v.clone(), bottom: v.clone(), left: v }
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
pub enum Align { #[default] Stretch, Start, End, Center, Baseline }

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Justify { #[default] Start, End, Center, SpaceBetween, SpaceAround, SpaceEvenly }

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CornerRadii {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl CornerRadii {
    pub fn all(r: f32) -> Self {
        CornerRadii { top_left: r, top_right: r, bottom_right: r, bottom_left: r }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StyleProps {
    pub width: Option<Dimension>,
    pub height: Option<Dimension>,
    pub padding: Option<Edges<f32>>,
    pub margin: Option<Edges<f32>>,
    pub gap: Option<f32>,
    pub flex_grow: Option<f32>,
    pub flex_shrink: Option<f32>,
    pub align_items: Option<Align>,
    pub justify_content: Option<Justify>,
}

/// Color as RGBA floats in 0.0–1.0.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Color { pub r: f32, pub g: f32, pub b: f32, pub a: f32 }

impl Color {
    pub const fn rgb8(r: u8, g: u8, b: u8) -> Self {
        Color { r: r as f32 / 255.0, g: g as f32 / 255.0, b: b as f32 / 255.0, a: 1.0 }
    }
    pub fn with_alpha(mut self, a: f32) -> Self { self.a = a; self }
}

/// A color that may be evaluated dynamically from a closure.
pub enum ColorSource {
    Static(Color),
    Dynamic(Box<dyn Fn() -> Color>),
}

impl ColorSource {
    pub fn resolve(&self) -> Color {
        match self { Self::Static(c) => *c, Self::Dynamic(f) => f() }
    }
}

impl From<Color> for ColorSource {
    fn from(c: Color) -> Self { ColorSource::Static(c) }
}

impl<F: Fn() -> Color + 'static> From<F> for ColorSource {
    fn from(f: F) -> Self { ColorSource::Dynamic(Box::new(f)) }
}

/// Visual decoration properties. May contain dynamic closures.
#[derive(Default)]
pub struct PaintProps {
    pub background: Option<ColorSource>,
    pub border_color: Option<ColorSource>,
    pub border_width: f32,
    pub radius: CornerRadii,
}

/// Resolved paint values with no closures — stored in Retained Tree and Patches.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PaintData {
    pub background: Option<Color>,
    pub border_color: Option<Color>,
    pub border_width: f32,
    pub radius: CornerRadii,
}

impl PaintProps {
    pub fn resolve(&self) -> PaintData {
        PaintData {
            background: self.background.as_ref().map(|c| c.resolve()),
            border_color: self.border_color.as_ref().map(|c| c.resolve()),
            border_width: self.border_width,
            radius: self.radius.clone(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextStyle {
    pub font_size: f32,
    pub font_weight: u16,
    pub color: Option<Color>,
}
