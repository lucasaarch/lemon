use crate::element::{
    types::{ComponentElement, ComponentFn, Key},
    Element,
};

// ── Component ─────────────────────────────────────────────────────────────

/// Nested view with its own [`Cx`](crate::Cx) hook state (signals, effects).
///
/// `view` must be a **function pointer** (`fn(&Cx) -> Element`), not a closure that captures
/// environment data. Identity is the function address plus an optional [`.key`](Self::key).
///
/// For list rows that need per-item callbacks, build keyed [`crate::RowView`] / [`crate::ColumnView`] children instead
/// of capturing state inside [`Component::new`].
pub struct Component(ComponentElement);

impl Component {
    /// Wraps a sub-view function; hooks inside `view` persist across parent re-renders.
    pub fn new(view: ComponentFn) -> Self {
        Self(ComponentElement::from_component_fn(view))
    }

    /// Wraps a typed sub-view function and props; hooks inside `view` persist across re-renders.
    ///
    /// `view` must be a function pointer. Use this when the component needs strongly typed,
    /// comparable props.
    ///
    /// ```
    /// use lemon_core::prelude::*;
    ///
    /// #[derive(Clone, PartialEq)]
    /// struct Props {
    ///     label: &'static str,
    /// }
    ///
    /// fn child(_cx: &Cx, props: &Props) -> Element {
    ///     Text::new(props.label).into_element()
    /// }
    ///
    /// let _ = Component::new_with_props(child, Props { label: "hello" });
    /// ```
    pub fn new_with_props<P: Clone + PartialEq + 'static>(
        view: fn(&crate::runtime::cx::Cx, &P) -> Element,
        props: P,
    ) -> Self {
        Self(ComponentElement::from_component_fn_with_props(view, props))
    }

    /// Stable key when this component is one of several keyed siblings.
    ///
    /// Keys must be unique among siblings and stable across renders. Duplicate
    /// keys in fully keyed lists cause reconciliation to panic.
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
