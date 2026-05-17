use crate::element::{
    content::TextContent,
    style::{PaintProps, StyleProps},
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Key(pub u64);

/// Used by Box_, Row, and Column — all three are the same struct.
pub struct BoxElement {
    pub style: StyleProps,
    pub paint: PaintProps,
    pub children: Vec<crate::element::Element>,
    pub key: Option<Key>,
}

impl Default for BoxElement {
    fn default() -> Self {
        BoxElement { style: Default::default(), paint: Default::default(), children: Vec::new(), key: None }
    }
}

pub struct TextElement {
    pub content: TextContent,
    pub style: crate::element::style::TextStyle,
    pub key: Option<Key>,
}

pub struct ButtonElement {
    pub label: TextContent,
    pub style: StyleProps,
    pub paint: PaintProps,
    pub on_click: Option<Box<dyn Fn()>>,
    pub key: Option<Key>,
}

pub struct ImageElement {
    pub src: String,
    pub style: StyleProps,
    pub key: Option<Key>,
}

pub struct ComponentElement {
    /// Closure that captures props and calls the component function.
    pub view: Box<dyn Fn(&crate::runtime::cx::Cx) -> crate::element::Element>,
    /// Used for stable component identity across re-renders.
    pub type_id: std::any::TypeId,
    pub key: Option<Key>,
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
            content: TextContent::Dynamic(Box::new(|| "dynamic".to_owned())),
            style: Default::default(),
            key: None,
        };
        assert_eq!(el.content.resolve(), "dynamic");
    }
}
