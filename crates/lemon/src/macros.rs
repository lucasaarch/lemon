//! Declarative macros for building element trees.

/// Builds a `Vec<Element>` from heterogeneous widget builders.
///
/// Each item must implement [`Into`]`<`[`Element`](crate::element::Element)`>`.
/// Use with [`.children`](crate::element::builders::Column::children) on
/// [`Column`](crate::element::builders::Column),
/// [`Row`](crate::element::builders::Row), or [`View`](crate::element::builders::View).
///
/// # Examples
///
/// ```
/// use lemon::{children, Button, Column, Row, Text};
///
/// let _tree = Column::new()
///     .children(children![
///         Text::new("Title").font_size(22.0),
///         Row::new().children(children![
///             Button::new("OK"),
///             Button::new("Cancel"),
///         ]),
///     ])
///     .into_element();
/// ```
#[macro_export]
macro_rules! children {
    () => {
        Vec::<$crate::element::Element>::new()
    };
    ($($child:expr),+ $(,)?) => {
        {
            let __children: Vec<$crate::element::Element> = vec![$(::core::convert::Into::into($child)),+];
            __children
        }
    };
}
