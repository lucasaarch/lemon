pub mod builders;
pub mod content;
pub mod events;
pub mod style;
pub mod types;

use types::{BoxElement, ButtonElement, ComponentElement, ImageElement, TextElement};

#[derive(Clone, Debug)]
pub enum Element {
    Text(TextElement),
    View(BoxElement),
    Row(BoxElement),
    Column(BoxElement),
    Button(ButtonElement),
    Image(ImageElement),
    Component(ComponentElement),
    Fragment(Vec<Element>),
    None,
}
