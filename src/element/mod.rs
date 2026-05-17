pub mod builders;
pub mod content;
pub mod style;
pub mod types;

use types::{BoxElement, ButtonElement, ComponentElement, ImageElement, TextElement};

#[derive(Debug)]
pub enum Element {
    Text(TextElement),
    Box_(BoxElement),
    Row(BoxElement),
    Column(BoxElement),
    Button(ButtonElement),
    Image(ImageElement),
    Component(ComponentElement),
    Fragment(Vec<Element>),
    None,
}
