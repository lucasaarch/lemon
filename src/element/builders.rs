use crate::element::{
    Element,
    content::TextContent,
    style::{Color, ColorSource, CornerRadii, Edges, PaintProps, StyleProps, TextStyle},
    types::{BoxElement, TextElement},
};

// ── Macro to generate container builders (Column, Row, Box_) ──────────────

macro_rules! container_builder {
    ($name:ident, $variant:ident) => {
        pub struct $name(BoxElement);

        impl $name {
            pub fn new() -> Self { $name(BoxElement::default()) }

            pub fn gap(mut self, v: f32) -> Self { self.0.style.gap = Some(v); self }
            pub fn padding(mut self, v: f32) -> Self {
                self.0.style.padding = Some(Edges::all(v)); self
            }
            pub fn width(mut self, v: f32) -> Self {
                self.0.style.width = Some(crate::element::style::Dimension::Points(v)); self
            }
            pub fn height(mut self, v: f32) -> Self {
                self.0.style.height = Some(crate::element::style::Dimension::Points(v)); self
            }
            pub fn flex_grow(mut self, v: f32) -> Self {
                self.0.style.flex_grow = Some(v); self
            }
            pub fn align_items(mut self, v: crate::element::style::Align) -> Self {
                self.0.style.align_items = Some(v); self
            }
            pub fn justify_content(mut self, v: crate::element::style::Justify) -> Self {
                self.0.style.justify_content = Some(v); self
            }
            pub fn background(mut self, c: impl Into<ColorSource>) -> Self {
                self.0.paint.background = Some(c.into()); self
            }
            pub fn border(mut self, color: Color, width: f32) -> Self {
                self.0.paint.border_color = Some(ColorSource::Static(color));
                self.0.paint.border_width = width;
                self
            }
            pub fn radius(mut self, r: f32) -> Self {
                self.0.paint.radius = CornerRadii::all(r); self
            }
            pub fn child(mut self, el: impl Into<Element>) -> Self {
                self.0.children.push(el.into()); self
            }
            pub fn into_element(self) -> Element { Element::$variant(self.0) }
        }

        impl Default for $name { fn default() -> Self { $name::new() } }
        impl From<$name> for Element { fn from(b: $name) -> Self { b.into_element() } }
    };
}

container_builder!(Column, Column);
container_builder!(Row, Row);
container_builder!(Box_, Box_);

// ── Text ──────────────────────────────────────────────────────────────────

pub struct Text {
    content: TextContent,
    style: TextStyle,
}

impl Text {
    pub fn new(content: impl Into<TextContent>) -> Self {
        Text { content: content.into(), style: TextStyle::default() }
    }
    pub fn font_size(mut self, size: f32) -> Self { self.style.font_size = size; self }
    pub fn weight(mut self, w: u16) -> Self { self.style.font_weight = w; self }
    pub fn color(mut self, c: Color) -> Self { self.style.color = Some(c); self }
    pub fn into_element(self) -> Element {
        Element::Text(TextElement { content: self.content, style: self.style, key: None })
    }
}

impl From<Text> for Element { fn from(b: Text) -> Self { b.into_element() } }

// ── Button ────────────────────────────────────────────────────────────────

pub struct Button {
    label: TextContent,
    style: StyleProps,
    paint: PaintProps,
    on_click: Option<Box<dyn Fn()>>,
}

impl Button {
    pub fn new(label: impl Into<TextContent>) -> Self {
        Button { label: label.into(), style: Default::default(), paint: Default::default(), on_click: None }
    }
    pub fn on_click(mut self, f: impl Fn() + 'static) -> Self {
        self.on_click = Some(Box::new(f)); self
    }
    pub fn background(mut self, c: impl Into<ColorSource>) -> Self {
        self.paint.background = Some(c.into()); self
    }
    pub fn radius(mut self, r: f32) -> Self { self.paint.radius = CornerRadii::all(r); self }
    pub fn into_element(self) -> Element {
        Element::Button(crate::element::types::ButtonElement {
            label: self.label,
            style: self.style,
            paint: self.paint,
            on_click: self.on_click,
            key: None,
        })
    }
}

impl From<Button> for Element { fn from(b: Button) -> Self { b.into_element() } }

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn column_builder_sets_gap() {
        let Element::Column(el) = Column::new().gap(8.0).into_element() else { panic!() };
        assert_eq!(el.style.gap, Some(8.0));
    }

    #[test]
    fn row_with_children() {
        let Element::Row(el) = Row::new()
            .child(Text::new("a"))
            .child(Text::new("b"))
            .into_element() else { panic!() };
        assert_eq!(el.children.len(), 2);
    }

    #[test]
    fn text_static_content() {
        let Element::Text(el) = Text::new("hello").into_element() else { panic!() };
        assert_eq!(el.content.resolve(), "hello");
    }

    #[test]
    fn text_dynamic_content() {
        let value = Rc::new(Cell::new(7u32));
        let v = value.clone();
        let Element::Text(el) = Text::new(move || v.get().to_string()).into_element() else { panic!() };
        assert_eq!(el.content.resolve(), "7");
        value.set(42);
        assert_eq!(el.content.resolve(), "42");
    }

    #[test]
    fn button_on_click_fires() {
        let fired = Rc::new(Cell::new(false));
        let f = fired.clone();
        let Element::Button(el) = Button::new("OK")
            .on_click(move || f.set(true))
            .into_element() else { panic!() };
        el.on_click.unwrap()();
        assert!(fired.get());
    }
}
