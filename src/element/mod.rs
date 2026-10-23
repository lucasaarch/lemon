pub mod builders;

pub mod content;

pub mod decoration;

pub mod events;

pub mod style;

pub mod types;

use types::{BoxElement, ComponentElement, TextElement};

/// Renderable tree built by views, text and component functions.
///
/// Composed widgets, including buttons and images, emit ordinary views and text;
/// their rendering and reconciliation follow the same paths as custom content.
#[derive(Clone, Debug)]
pub enum Element {
    /// Text measured and painted with the active typography.
    Text(TextElement),
    /// General container with layout, paint and input properties.
    View(BoxElement),
    /// Container that stacks children horizontally.
    Row(BoxElement),
    /// Container that stacks children vertically.
    Column(BoxElement),
    /// Component scope with its own reactive hooks and identity.
    Component(ComponentElement),
    /// Children reconciled without an additional layout container.
    Fragment(Vec<Element>),
    /// No rendered content.
    None,
}
