use std::{any::Any, rc::Rc};

use crate::element::{
    content::TextContent,
    style::{PaintProps, StyleProps},
};

pub type ComponentFn = fn(&crate::runtime::cx::Cx) -> crate::element::Element;

fn component_identity(view: ComponentFn) -> usize {
    view as *const () as usize
}

/// Type-erased component props stored on [`ComponentElement`].
///
/// This enables comparing and cloning typed props without exposing their concrete type.
pub trait AnyProps: 'static {
    /// Clones this props value into a boxed trait object.
    fn clone_box(&self) -> Box<dyn AnyProps>;
    /// Compares this props value against another type-erased props value.
    fn eq_box(&self, other: &dyn AnyProps) -> bool;
    /// Returns this props value as [`Any`] for downcasting.
    fn as_any(&self) -> &dyn Any;
}

impl<T: Clone + PartialEq + 'static> AnyProps for T {
    fn clone_box(&self) -> Box<dyn AnyProps> {
        Box::new(self.clone())
    }

    fn eq_box(&self, other: &dyn AnyProps) -> bool {
        other
            .as_any()
            .downcast_ref::<T>()
            .is_some_and(|other| self == other)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Key(pub u64);

/// Editing metadata for a single-line text field (caret painting, focus chrome).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextInputMeta {
    pub cursor: usize,
    pub value: String,
}

/// Shared backing struct for [`View`](crate::element::builders::View), [`Row`](crate::element::builders::Row), and [`Column`](crate::element::builders::Column).
#[derive(Clone, Default, Debug)]
pub struct BoxElement {
    pub style: StyleProps,
    pub paint: PaintProps,
    pub children: Vec<crate::element::Element>,
    pub key: Option<Key>,
    pub handlers: crate::retained::EventHandlers,
    /// When set, the paint pass draws a caret for this field when it has keyboard focus.
    pub text_input: Option<TextInputMeta>,
    /// Marks a clipped viewport that may show a vertical scrollbar when content overflows.
    pub scroll_viewport: bool,
    /// Marks a clipped viewport whose scrollbar track and thumb are painted by the paint pass.
    pub scroll_bar: bool,
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

pub struct ComponentElement {
    /// Closure that captures props and calls the component function.
    view: Rc<dyn Fn(&crate::runtime::cx::Cx) -> crate::element::Element>,
    type_id: std::any::TypeId,
    identity: usize,
    key: Option<Key>,
    props: Option<Box<dyn AnyProps>>,
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
            props: None,
        }
    }

    pub fn from_component_fn(view: ComponentFn) -> Self {
        Self::new(
            std::any::TypeId::of::<ComponentFn>(),
            component_identity(view),
            Rc::new(view),
        )
    }

    /// Creates a component element from a function pointer and typed props.
    ///
    /// `view` keeps function-pointer identity semantics while `props` are stored for
    /// typed equality checks and cloning.
    pub fn from_component_fn_with_props<P: Clone + PartialEq + 'static>(
        view: fn(&crate::runtime::cx::Cx, &P) -> crate::element::Element,
        props: P,
    ) -> Self {
        let identity = view as *const () as usize;
        let props_for_view = props.clone();
        Self {
            view: Rc::new(move |cx| view(cx, &props_for_view)),
            type_id: std::any::TypeId::of::<
                fn(&crate::runtime::cx::Cx, &P) -> crate::element::Element,
            >(),
            identity,
            key: None,
            props: Some(Box::new(props)),
        }
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

    pub(crate) fn props_eq(&self, other: &Self) -> bool {
        match (&self.props, &other.props) {
            (Some(left), Some(right)) => left.eq_box(right.as_ref()),
            (None, None) => true,
            _ => false,
        }
    }

    pub(crate) fn has_props(&self) -> bool {
        self.props.is_some()
    }

    pub(crate) fn with_key(mut self, key: Key) -> Self {
        self.key = Some(key);
        self
    }
}

impl Clone for ComponentElement {
    fn clone(&self) -> Self {
        Self {
            view: self.view.clone(),
            type_id: self.type_id,
            identity: self.identity,
            key: self.key.clone(),
            props: self.props.as_ref().map(|props| props.clone_box()),
        }
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
            .field("props", &self.props.as_ref().map(|_| "Box<dyn AnyProps>"))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::builders::{Component, Text};
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

    #[test]
    fn component_element_with_props_stores_and_compares_props() {
        #[derive(Clone, PartialEq)]
        struct MyProps {
            value: i32,
        }

        fn child(_cx: &crate::runtime::cx::Cx, props: &MyProps) -> Element {
            Text::new(props.value.to_string()).into_element()
        }
        fn child_no_props(_cx: &crate::runtime::cx::Cx) -> Element {
            Text::new("no props").into_element()
        }

        let a = ComponentElement::from_component_fn_with_props(child, MyProps { value: 1 });
        let b = ComponentElement::from_component_fn_with_props(child, MyProps { value: 1 });
        let c = ComponentElement::from_component_fn_with_props(child, MyProps { value: 2 });
        let d = ComponentElement::from_component_fn(child_no_props);

        assert_eq!(a.identity(), b.identity());
        assert!(a.props_eq(&b));
        assert!(!a.props_eq(&c));
        assert!(!a.props_eq(&d));
    }

    #[test]
    fn component_new_with_props_renders_with_props() {
        #[derive(Clone, PartialEq)]
        struct Props {
            label: String,
        }

        fn child(_cx: &crate::runtime::cx::Cx, props: &Props) -> Element {
            Text::new(props.label.clone()).into_element()
        }

        let Element::Component(component) =
            Component::new_with_props(child, Props { label: "hi".into() }).into_element()
        else {
            panic!("expected component element");
        };

        let Element::Text(rendered) = (component.view())(&crate::runtime::cx::Cx::new()) else {
            panic!("expected text element");
        };
        assert_eq!(rendered.content.resolve(), "hi");
    }
}
