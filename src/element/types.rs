use std::rc::Rc;

use crate::element::{
    content::TextContent,
    style::{PaintProps, StyleProps},
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Key(pub u64);

/// Used by Box_, Row, and Column — all three are the same struct.
#[derive(Clone, Default, Debug)]
pub struct BoxElement {
    pub style: StyleProps,
    pub paint: PaintProps,
    pub children: Vec<crate::element::Element>,
    pub key: Option<Key>,
}

#[derive(Clone)]
pub struct TextElement {
    pub content: TextContent,
    pub style: crate::element::style::TextStyle,
    pub key: Option<Key>,
}

impl std::fmt::Debug for TextElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextElement")
            .field("content", &format!("{:?}", self.content.resolve()))
            .field("style", &self.style)
            .field("key", &self.key)
            .finish()
    }
}

#[derive(Clone)]
pub struct ButtonElement {
    pub label: TextContent,
    pub style: StyleProps,
    pub paint: PaintProps,
    pub on_click: Option<Rc<dyn Fn()>>,
    pub key: Option<Key>,
}

impl std::fmt::Debug for ButtonElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ButtonElement")
            .field("label", &format!("{:?}", self.label.resolve()))
            .field("style", &self.style)
            .field("paint", &self.paint)
            .field("on_click", &self.on_click.as_ref().map(|_| "Box<dyn Fn()>"))
            .field("key", &self.key)
            .finish()
    }
}

#[derive(Clone)]
pub struct ImageElement {
    pub src: String,
    pub style: StyleProps,
    pub key: Option<Key>,
}

impl std::fmt::Debug for ImageElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageElement")
            .field("src", &self.src)
            .field("style", &self.style)
            .field("key", &self.key)
            .finish()
    }
}

#[derive(Clone)]
pub struct ComponentElement {
    /// Closure that captures props and calls the component function.
    pub view: Rc<dyn Fn(&crate::runtime::cx::Cx) -> crate::element::Element>,
    /// Used for stable component identity across re-renders.
    pub type_id: std::any::TypeId,
    pub key: Option<Key>,
}

impl std::fmt::Debug for ComponentElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComponentElement")
            .field("view", &"Box<dyn Fn()>")
            .field("type_id", &self.type_id)
            .field("key", &self.key)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::content::TextContent;

    #[test]
    fn box_element_default_has_no_children() {
        let el = BoxElement::default();
        assert!(el.children.is_empty());
    }

    #[test]
    fn text_element_resolves_static_content() {
        let el = TextElement {
            content: TextContent::Static("hello".into()),
            style: Default::default(),
            key: None,
        };
        assert_eq!(el.content.resolve(), "hello");
    }

    #[test]
    fn text_element_resolves_dynamic_content() {
        let el = TextElement {
            content: TextContent::Dynamic(Rc::new(|| "dynamic".to_owned())),
            style: Default::default(),
            key: None,
        };
        assert_eq!(el.content.resolve(), "dynamic");
    }
}
