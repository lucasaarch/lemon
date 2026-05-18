use std::rc::Rc;

use crate::element::{
    content::TextContent,
    style::{PaintProps, StyleProps},
};

pub type ComponentFn = fn(&crate::runtime::cx::Cx) -> crate::element::Element;

fn component_identity(view: ComponentFn) -> usize {
    view as *const () as usize
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Key(pub u64);

/// Used by Box_, Row, and Column — all three are the same struct.
#[derive(Clone, Default, Debug)]
pub struct BoxElement {
    pub style: StyleProps,
    pub paint: PaintProps,
    pub children: Vec<crate::element::Element>,
    pub key: Option<Key>,
    pub handlers: crate::retained::EventHandlers,
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
    view: Rc<dyn Fn(&crate::runtime::cx::Cx) -> crate::element::Element>,
    type_id: std::any::TypeId,
    identity: usize,
    key: Option<Key>,
}

impl ComponentElement {
    fn new(
        type_id: std::any::TypeId,
        identity: usize,
        view: Rc<dyn Fn(&crate::runtime::cx::Cx) -> crate::element::Element>,
    ) -> Self {
        Self {
            view,
            type_id,
            identity,
            key: None,
        }
    }

    pub fn from_component_fn(view: ComponentFn) -> Self {
        Self::new(
            std::any::TypeId::of::<ComponentFn>(),
            component_identity(view),
            Rc::new(view),
        )
    }

    pub fn type_id(&self) -> std::any::TypeId {
        self.type_id
    }

    pub(crate) fn identity(&self) -> usize {
        self.identity
    }

    pub fn key(&self) -> Option<&Key> {
        self.key.as_ref()
    }

    pub fn view(&self) -> Rc<dyn Fn(&crate::runtime::cx::Cx) -> crate::element::Element> {
        self.view.clone()
    }

    pub(crate) fn with_key(mut self, key: Key) -> Self {
        self.key = Some(key);
        self
    }
}

impl std::fmt::Debug for ComponentElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let _ = &self.view;
        f.debug_struct("ComponentElement")
            .field("view", &"Box<dyn Fn()>")
            .field("type_id", &self.type_id)
            .field("identity", &self.identity)
            .field("key", &self.key)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::builders::Text;
    use crate::element::content::TextContent;
    use crate::element::Element;

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

    #[test]
    fn component_element_new_preserves_explicit_identity_for_wrapped_view() {
        fn child(_cx: &crate::runtime::cx::Cx) -> Element {
            Text::new("child").into_element()
        }

        let type_id = std::any::TypeId::of::<ComponentFn>();
        let component = ComponentElement::new(type_id, component_identity(child), Rc::new(child))
            .with_key(Key(3));

        assert_eq!(component.type_id(), type_id);
        assert_eq!(component.identity(), component_identity(child));
        assert_eq!(component.key(), Some(&Key(3)));
        let Element::Text(rendered) = (component.view())(&crate::runtime::cx::Cx::new()) else {
            panic!("expected text element");
        };
        assert_eq!(rendered.content.resolve(), "child");
    }

    #[test]
    fn component_fn_identity_distinguishes_different_functions_of_same_type() {
        fn first(_cx: &crate::runtime::cx::Cx) -> Element {
            Text::new("first").into_element()
        }

        fn second(_cx: &crate::runtime::cx::Cx) -> Element {
            Text::new("second").into_element()
        }

        let first_component = ComponentElement::from_component_fn(first);
        let second_component = ComponentElement::from_component_fn(second);

        assert_eq!(first_component.type_id(), second_component.type_id());
        assert_ne!(first_component.identity(), second_component.identity());
    }
}
