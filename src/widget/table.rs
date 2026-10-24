use crate::element::Element;
use crate::{ColumnView, Cx, RowView, Text, View};

/// A simple data table with a themed header and equal-width columns.
///
/// Each row must contain one cell per header. Wrap the returned element in a
/// [`ScrollView`](crate::ScrollView) when the table needs a bounded viewport.
///
/// ```
/// use lemon::{Cx, widget::Table};
/// let table = Table::new(
///     &Cx::new(),
///     ["Name", "Role"],
///     [["Ada", "Engineer"], ["Linus", "Maintainer"]],
/// );
/// ```
pub struct Table {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    theme: crate::Theme,
}

impl Table {
    /// Creates a table from column headings and rows of string values.
    ///
    /// # Panics
    /// Panics if any row has a different number of cells than `headers`.
    pub fn new<H, R, C>(
        cx: &Cx,
        headers: impl IntoIterator<Item = H>,
        rows: impl IntoIterator<Item = R>,
    ) -> Self
    where
        H: Into<String>,
        R: IntoIterator<Item = C>,
        C: Into<String>,
    {
        let headers = headers.into_iter().map(Into::into).collect::<Vec<_>>();
        assert!(!headers.is_empty(), "Table requires at least one column");
        let rows = rows
            .into_iter()
            .map(|row| {
                let row = row.into_iter().map(Into::into).collect::<Vec<_>>();
                assert_eq!(
                    row.len(),
                    headers.len(),
                    "Table rows must have one cell per header"
                );
                row
            })
            .collect();
        Self {
            headers,
            rows,
            theme: cx.theme(),
        }
    }

    /// Builds a full-width header and its data rows.
    pub fn into_element(self) -> Element {
        let theme = self.theme;
        let mut header = RowView::new()
            .fill_width()
            .height(36.0)
            .surface(theme.colors.surface_pressed);
        for label in self.headers {
            header = header.child(
                View::new()
                    .grow(1.0)
                    .fill_height()
                    .inset_horizontal(10.0)
                    .place(crate::element::style::Placement::Left)
                    .child(Text::new(label).size(12.0)),
            );
        }

        let mut body = ColumnView::new().fill_width();
        for (index, row) in self.rows.into_iter().enumerate() {
            let mut line = RowView::new()
                .fill_width()
                .height(34.0)
                .surface(if index % 2 == 0 {
                    theme.colors.surface
                } else {
                    theme.colors.background
                });
            for cell in row {
                line = line.child(
                    View::new()
                        .grow(1.0)
                        .fill_height()
                        .inset_horizontal(10.0)
                        .place(crate::element::style::Placement::Left)
                        .child(Text::new(cell).size(12.0)),
                );
            }
            body = body.child(line);
        }
        ColumnView::new()
            .fill_width()
            .clip()
            .surface(theme.colors.surface)
            .child(header)
            .child(body)
            .into_element()
    }
}

impl From<Table> for Element {
    fn from(table: Table) -> Self {
        table.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_renders_header_and_equal_width_cells() {
        let table = Table::new(&Cx::new(), ["Name", "Role"], [["Ada", "Engineer"]]).into_element();
        let Element::Column(root) = table else {
            panic!("table must render a column")
        };
        let Element::Row(header) = &root.children[0] else {
            panic!("table header must render as a row")
        };
        let Element::Column(body) = &root.children[1] else {
            panic!("table body must render as a column")
        };
        let Element::Row(data) = &body.children[0] else {
            panic!("table data must render as a row")
        };
        assert_eq!(header.children.len(), 2);
        assert_eq!(data.children.len(), 2);
        assert!(header.children.iter().all(|cell| match cell {
            Element::View(view) => view.style.flex_grow == Some(1.0),
            _ => false,
        }));
    }

    #[test]
    #[should_panic(expected = "Table rows must have one cell per header")]
    fn table_rejects_rows_with_wrong_cell_count() {
        let _ = Table::new(&Cx::new(), ["Name", "Role"], [["Ada"]]);
    }
}
