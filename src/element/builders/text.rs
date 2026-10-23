use crate::element::{
    content::TextContent,
    style::{Color, TextStyle},
    types::{Key, TextElement},
    Element,
};

// ── Text ──────────────────────────────────────────────────────────────────

/// Single-line or wrapped text label.
///
/// Pass a `&str` / `String` for static copy, or a `Fn() -> String` closure to re-read signals
/// on each render:
///
/// ```no_run
/// # use lemon::{State, element::builders::Text};
/// # let count = State::new(0);
/// # let c = count.clone();
/// Text::new(move || format!("{}", c.get()));
/// ```
pub struct Text {
    content: TextContent,
    style: TextStyle,
    key: Option<Key>,
}

impl Text {
    /// Creates text from static content or a reactive closure (see [`TextContent`]).
    ///
    /// Typography defaults (size/family/line-height/letter-spacing) are read from the active
    /// theme (`Cx::theme` / [`crate::theme::current_theme`]) at build time.
    pub fn new(content: impl Into<TextContent>) -> Self {
        Text {
            content: content.into(),
            style: TextStyle::default(),
            key: None,
        }
    }

    /// Stable key when this text node is a keyed sibling (see [`crate::ColumnView::key`]).
    pub fn key(mut self, key: u64) -> Self {
        self.key = Some(Key(key));
        self
    }

    /// Font size in logical points.
    pub fn size(mut self, size: f32) -> Self {
        self.style.font_size = size;
        self
    }

    /// Preferred font family list (for example `"Inter, system-ui"`).
    pub fn font(mut self, family: impl Into<String>) -> Self {
        self.style.font_family = family.into();
        self
    }

    /// Unitless line-height multiplier relative to font size.
    pub fn leading(mut self, line_height: f32) -> Self {
        self.style.line_height = line_height;
        self
    }

    pub fn weight(mut self, w: u16) -> Self {
        self.style.font_weight = w;
        self
    }

    pub fn color(mut self, c: Color) -> Self {
        self.style.color = Some(c);
        self
    }

    /// Finishes the builder and returns an [`Element`].
    pub fn into_element(self) -> Element {
        Element::Text(TextElement {
            content: self.content,
            style: self.style,
            key: self.key,
        })
    }
}

impl From<Text> for Element {
    fn from(b: Text) -> Self {
        b.into_element()
    }
}
