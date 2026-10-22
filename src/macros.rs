//! Declarative macros for building core element trees.

/// Builds a `Vec<Element>` from heterogeneous widget builders.
///
/// Each item must implement [`Into`]`<`[`Element`](crate::element::Element)`>`.
/// Use with [`.children`](crate::element::builders::ColumnView::children) on
/// [`Column`](crate::element::builders::ColumnView),
/// [`Row`](crate::element::builders::RowView), or [`View`](crate::element::builders::View).
///
/// # Examples
///
/// ```
/// use lemon::{children, ColumnView, RowView, Text};
/// use lemon::element::builders::Button;
///
/// let _tree = ColumnView::new()
///     .children(children![
///         Text::new("Title").size(22.0),
///         RowView::new().children(children![
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
