use std::rc::Rc;

use crate::element::{
    content::TextContent,
    style::{Color, ColorSource, CornerRadii, Edges, Overflow, PaintProps, StyleProps, TextStyle},
    types::{BoxElement, ComponentElement, ComponentFn, Key, TextElement},
    Element,
};

// ── Macro to generate container builders (Column, Row, View) ──────────────

macro_rules! container_builder {
    ($name:ident, $variant:ident, $doc:expr) => {
        #[doc = $doc]
        pub struct $name(BoxElement);

        impl $name {
            /// Creates an empty container; add children with [`.child`](Self::child).
            pub fn new() -> Self {
                $name(BoxElement::default())
            }

            /// Spacing between flex children, in logical points.
            pub fn gap(mut self, v: f32) -> Self {
                self.0.style.gap = Some(v);
                self
            }
            /// Uniform padding on all sides.
            pub fn padding(mut self, v: f32) -> Self {
                self.0.style.padding = Some(Edges::all(v));
                self
            }
            /// Fixed width in logical points.
            pub fn width(mut self, v: f32) -> Self {
                self.0.style.width = Some(crate::element::style::Dimension::Points(v));
                self
            }
            /// Fixed height in logical points.
            pub fn height(mut self, v: f32) -> Self {
                self.0.style.height = Some(crate::element::style::Dimension::Points(v));
                self
            }
            pub fn flex_grow(mut self, v: f32) -> Self {
                self.0.style.flex_grow = Some(v);
                self
            }
            pub fn align_items(mut self, v: crate::element::style::Align) -> Self {
                self.0.style.align_items = Some(v);
                self
            }
            pub fn justify_content(mut self, v: crate::element::style::Justify) -> Self {
                self.0.style.justify_content = Some(v);
                self
            }
            pub fn overflow(mut self, v: Overflow) -> Self {
                self.0.style.overflow = v;
                self
            }
            /// Paint-only z-order among siblings (`0` keeps normal traversal order).
            ///
            /// Non-zero values are deferred to a sorted paint pass and do not affect layout.
            pub fn z_index(mut self, z: i32) -> Self {
                self.0.style.z_index = z;
                self
            }
            pub fn background(mut self, c: impl Into<ColorSource>) -> Self {
                self.0.paint.background = Some(c.into());
                self
            }
            pub fn border(mut self, color: Color, width: f32) -> Self {
                self.0.paint.border_color = Some(ColorSource::Static(color));
                self.0.paint.border_width = width;
                self
            }
            pub fn radius(mut self, r: f32) -> Self {
                self.0.paint.radius = CornerRadii::all(r);
                self
            }
            /// Sets an image to draw inside this container using object-fit: contain scaling.
            ///
            /// The image is scaled uniformly to fit within the container's layout rectangle while
            /// preserving its aspect ratio, then centered. This is equivalent to CSS
            /// `object-fit: contain`.
            ///
            /// ```no_run
            /// use lemon::{Column, ImageHandle};
            ///
            /// # fn example(handle: ImageHandle) {
            /// Column::new()
            ///     .width(200.0)
            ///     .height(150.0)
            ///     .image(handle)
            ///     .into_element();
            /// # }
            /// ```
            pub fn image(mut self, handle: crate::asset::ImageHandle) -> Self {
                self.0.paint.image = Some(handle);
                self
            }
            /// Appends a child widget; call repeatedly to build a list of children.
            pub fn child(mut self, el: impl Into<Element>) -> Self {
                self.0.children.push(el.into());
                self
            }
            /// Appends multiple children in order.
            ///
            /// For a list of mixed widget types, use the [`children!`](crate::children) macro:
            /// `.children(children![Text::new("a"), Button::new("b")])`.
            pub fn children(mut self, items: impl IntoIterator<Item = impl Into<Element>>) -> Self {
                self.0.children.extend(items.into_iter().map(Into::into));
                self
            }
            /// Stable identity for diffing when this child is inserted, removed, or reordered.
            ///
            /// All siblings under the same parent must use keys if any sibling uses one.
            pub fn key(mut self, key: u64) -> Self {
                self.0.key = Some(Key(key));
                self
            }
            /// Mouse click handler (logical coordinates, hit-tested against this node’s bounds).
            pub fn on_click(mut self, f: impl Fn() + 'static) -> Self {
                self.0.handlers.on_click = Some(Rc::new(f));
                self
            }
            pub fn on_key_down(
                mut self,
                f: impl Fn(crate::element::events::KeyEvent) + 'static,
            ) -> Self {
                self.0.handlers.on_key_down = Some(Rc::new(f));
                self
            }
            pub fn on_key_up(
                mut self,
                f: impl Fn(crate::element::events::KeyEvent) + 'static,
            ) -> Self {
                self.0.handlers.on_key_up = Some(Rc::new(f));
                self
            }
            pub fn on_hover_enter(mut self, f: impl Fn() + 'static) -> Self {
                self.0.handlers.on_hover_enter = Some(Rc::new(f));
                self
            }
            pub fn on_hover_leave(mut self, f: impl Fn() + 'static) -> Self {
                self.0.handlers.on_hover_leave = Some(Rc::new(f));
                self
            }
            /// Mouse wheel handler; `delta` is the vertical scroll amount in logical pixels.
            ///
            /// Used by scrollable regions (see `Scroll` in `lemon-widgets`). The platform
            /// dispatches wheel events to the deepest node under the cursor with this handler.
            pub fn on_scroll(mut self, f: impl Fn(f64) + 'static) -> Self {
                self.0.handlers.on_scroll = Some(Rc::new(f));
                self
            }
            /// Sets top margin in logical points (other sides unchanged).
            ///
            /// Negative values shift content up without changing layout size — useful for
            /// scroll offsets on inner content inside a clipped viewport.
            pub fn margin_top(mut self, v: f32) -> Self {
                let mut edges = self.0.style.margin.take().unwrap_or_default();
                edges.top = v;
                self.0.style.margin = Some(edges);
                self
            }
            /// Includes this node in keyboard focus traversal (Tab / Shift+Tab).
            pub fn focusable(mut self) -> Self {
                self.0.style.focusable = true;
                self
            }
            /// Attaches text-field metadata used by the paint pass (caret, focus ring).
            pub fn text_input(mut self, meta: crate::element::types::TextInputMeta) -> Self {
                self.0.text_input = Some(meta);
                self
            }
            /// Marks this node as a vertical scroll viewport (scrollbar when content overflows).
            pub fn scroll_viewport(mut self) -> Self {
                self.0.scroll_viewport = true;
                self
            }
            /// Cursor shown when the pointer is over this node.
            pub fn cursor(mut self, c: crate::element::events::Cursor) -> Self {
                self.0.style.cursor = c;
                self
            }
            /// Finishes the builder and returns an [`Element`] for use in [`.child`](Self::child) or the root view.
            pub fn into_element(self) -> Element {
                Element::$variant(self.0)
            }
        }

        impl Default for $name {
            fn default() -> Self {
                $name::new()
            }
        }
        impl From<$name> for Element {
            fn from(b: $name) -> Self {
                b.into_element()
            }
        }
    };
}

container_builder!(
    Column,
    Column,
    "Vertical flex container: children stack from top to bottom."
);
container_builder!(
    Row,
    Row,
    "Horizontal flex container: children stack from left to right."
);
container_builder!(
    View,
    View,
    "Generic flex container; use when you do not need an explicit row or column axis."
);

// ── Text ──────────────────────────────────────────────────────────────────

/// Single-line or wrapped text label.
///
/// Pass a `&str` / `String` for static copy, or a `Fn() -> String` closure to re-read signals
/// on each render:
///
/// ```no_run
/// # use lemon::{Signal, element::builders::Text};
/// # let count = Signal::new(0);
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
    pub fn new(content: impl Into<TextContent>) -> Self {
        Text {
            content: content.into(),
            style: TextStyle::default(),
            key: None,
        }
    }
    /// Stable key when this text node is a keyed sibling (see [`Column::key`]).
    pub fn key(mut self, key: u64) -> Self {
        self.key = Some(Key(key));
        self
    }
    /// Font size in logical points.
    pub fn font_size(mut self, size: f32) -> Self {
        self.style.font_size = size;
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

// ── Button ────────────────────────────────────────────────────────────────

/// Clickable control with a text label and default padding / background.
pub struct Button {
    label: TextContent,
    style: StyleProps,
    paint: PaintProps,
    on_click: Option<Rc<dyn Fn()>>,
}

impl Button {
    /// Creates a button; label accepts the same static or closure forms as [`Text::new`].
    pub fn new(label: impl Into<TextContent>) -> Self {
        Button {
            label: label.into(),
            style: StyleProps {
                padding: Some(Edges::all(10.0)),
                align_self: Some(crate::element::style::Align::Start),
                ..Default::default()
            },
            paint: PaintProps {
                background: Some(Color::rgb8(55, 120, 220).into()),
                radius: CornerRadii::all(6.0),
                ..Default::default()
            },
            on_click: None,
        }
    }
    /// Called when the button is clicked (after hit-testing).
    pub fn on_click(mut self, f: impl Fn() + 'static) -> Self {
        self.on_click = Some(Rc::new(f));
        self
    }
    /// Fill color behind the label ([`Color`] or reactive [`ColorSource`]).
    pub fn background(mut self, c: impl Into<ColorSource>) -> Self {
        self.paint.background = Some(c.into());
        self
    }
    pub fn radius(mut self, r: f32) -> Self {
        self.paint.radius = CornerRadii::all(r);
        self
    }
    pub fn width(mut self, v: f32) -> Self {
        self.style.width = Some(crate::element::style::Dimension::Points(v));
        self
    }
    pub fn height(mut self, v: f32) -> Self {
        self.style.height = Some(crate::element::style::Dimension::Points(v));
        self
    }
    /// Finishes the builder and returns an [`Element`].
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

impl From<Button> for Element {
    fn from(b: Button) -> Self {
        b.into_element()
    }
}

// ── Component ─────────────────────────────────────────────────────────────

/// Nested view with its own [`Cx`](crate::Cx) hook state (signals, effects).
///
/// `view` must be a **function pointer** (`fn(&Cx) -> Element`), not a closure that captures
/// environment data. Identity is the function address plus an optional [`.key`](Self::key).
///
/// For list rows that need per-item callbacks, build keyed [`Row`] / [`Column`] children instead
/// of capturing state inside [`Component::new`].
pub struct Component(ComponentElement);

impl Component {
    /// Wraps a sub-view function; hooks inside `view` persist across parent re-renders.
    pub fn new(view: ComponentFn) -> Self {
        Self(ComponentElement::from_component_fn(view))
    }

    /// Stable key when this component is one of several keyed siblings.
    pub fn key(mut self, key: u64) -> Self {
        self.0 = self.0.with_key(Key(key));
        self
    }

    /// Finishes the builder and returns an [`Element`].
    pub fn into_element(self) -> Element {
        Element::Component(self.0)
    }
}

impl From<Component> for Element {
    fn from(component: Component) -> Self {
        component.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::types::Key;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn column_builder_sets_gap() {
        let Element::Column(el) = Column::new().gap(8.0).into_element() else {
            panic!()
        };
        assert_eq!(el.style.gap, Some(8.0));
    }

    #[test]
    fn row_with_children() {
        let Element::Row(el) = Row::new()
            .child(Text::new("a"))
            .child(Text::new("b"))
            .into_element()
        else {
            panic!()
        };
        assert_eq!(el.children.len(), 2);
    }

    #[test]
    fn children_macro_accepts_mixed_builders() {
        use crate::{children, Button, Column, Row, Text};

        let Element::Column(col) = Column::new()
            .children(children![
                Text::new("title"),
                Button::new("ok"),
                Row::new().child(Text::new("nested")),
            ])
            .into_element()
        else {
            panic!("expected Column");
        };
        assert_eq!(col.children.len(), 3);
    }

    #[test]
    fn box_builder_sets_key() {
        let Element::View(el) = View::new().key(9).into_element() else {
            panic!()
        };
        assert_eq!(el.key, Some(Key(9)));
    }

    #[test]
    fn box_builder_sets_overflow() {
        let Element::View(el) = View::new().overflow(Overflow::Hidden).into_element() else {
            panic!()
        };
        assert_eq!(el.style.overflow, Overflow::Hidden);
    }

    #[test]
    fn box_builder_sets_z_index() {
        let Element::View(el) = View::new().z_index(3).into_element() else {
            panic!()
        };
        assert_eq!(el.style.z_index, 3);
    }

    #[test]
    fn box_builder_sets_focusable_cursor_and_handlers() {
        let Element::View(el) = View::new()
            .focusable()
            .cursor(crate::element::events::Cursor::Pointer)
            .on_click(|| {})
            .on_hover_enter(|| {})
            .on_hover_leave(|| {})
            .on_key_down(|_| {})
            .on_key_up(|_| {})
            .into_element()
        else {
            panic!()
        };
        assert!(el.style.focusable);
        assert_eq!(el.style.cursor, crate::element::events::Cursor::Pointer);
        assert!(el.handlers.on_click.is_some());
        assert!(el.handlers.on_hover_enter.is_some());
        assert!(el.handlers.on_hover_leave.is_some());
        assert!(el.handlers.on_key_down.is_some());
        assert!(el.handlers.on_key_up.is_some());
    }

    #[test]
    fn text_static_content() {
        let Element::Text(el) = Text::new("hello").into_element() else {
            panic!()
        };
        assert_eq!(el.content.resolve(), "hello");
    }

    #[test]
    fn text_dynamic_content() {
        let value = Rc::new(Cell::new(7u32));
        let v = value.clone();
        let Element::Text(el) = Text::new(move || v.get().to_string()).into_element() else {
            panic!()
        };
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
            .into_element()
        else {
            panic!()
        };
        el.on_click.unwrap()();
        assert!(fired.get());
    }

    #[test]
    fn component_builder_tracks_function_identity_and_key() {
        fn first(_cx: &crate::runtime::cx::Cx) -> Element {
            Text::new("first").into_element()
        }

        fn second(_cx: &crate::runtime::cx::Cx) -> Element {
            Text::new("second").into_element()
        }

        let Element::Component(first_with_key) = Component::new(first).key(7).into_element() else {
            panic!("expected component element");
        };
        let Element::Component(first_again) = Component::new(first).into_element() else {
            panic!("expected component element");
        };
        let Element::Component(second_component) = Component::new(second).into_element() else {
            panic!("expected component element");
        };

        assert_eq!(first_with_key.key(), Some(&Key(7)));
        assert_eq!(first_with_key.identity(), first_again.identity());
        assert_ne!(first_with_key.identity(), second_component.identity());
    }
}
