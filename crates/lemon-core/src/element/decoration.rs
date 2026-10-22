//! Styling shared by every element builder and composed control.
use super::{builders::View, Element};

/// Applies reusable container styling after a widget's theme and defaults.
///
/// Implemented automatically for every type convertible to [`Element`], including
/// custom widgets. Existing containers keep their kind, children, key and handlers;
/// leaves and components receive a surrounding `View`. The callback receives the
/// existing view properties, so only fields explicitly changed are overwritten.
/// Decorations run in call order. Widget-specific `.style(...)` configures internal
/// parts before decoration; decorating a control does not restyle its internal parts.
///
/// ```
/// use lemon_core::prelude::*;
/// fn card(view: View) -> View { view.inset(20.0).rounding(12.0) }
/// let element = ColumnView::new().child(Text::new("Hello")).decorate(card);
/// let Element::Column(container) = element else { panic!("column preserved") };
/// assert_eq!(container.style.padding.unwrap().top, 20.0);
/// ```
pub trait Decorate: Into<Element> + Sized {
    /// Finishes this builder and modifies its root using the complete `View` API.
    /// Returns the decorated element, ready for `.child(...)` or `.children(...)`.
    fn decorate(self, decoration: impl FnOnce(View) -> View) -> Element {
        match self.into() {
            Element::View(node) => {
                Element::View(decoration(View::from_box(node, false)).into_box())
            }
            Element::Row(node) => Element::Row(decoration(View::from_box(node, false)).into_box()),
            Element::Column(node) => {
                Element::Column(decoration(View::from_box(node, true)).into_box())
            }
            leaf => {
                let mut view = View::new();
                // Keys belong to the decorated sibling, not the new inner child.
                let mut leaf = leaf;
                let key = match &mut leaf {
                    Element::Text(node) => node.key.take(),
                    Element::Button(node) => node.key.take(),
                    Element::Image(node) => node.key.take(),
                    Element::Component(node) => node.key().cloned(),
                    _ => None,
                };
                if let Some(key) = key {
                    view = view.key(key.0);
                }
                decoration(view.child(leaf)).into_element()
            }
        }
    }
}
impl<T: Into<Element>> Decorate for T {}
