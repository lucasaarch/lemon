//! Counter demo — opens a window (UI wiring completed in platform Task 3).

use lemon::{run, Column, Cx, Text, WindowConfig};

fn root(_cx: &Cx) -> lemon::element::Element {
    Column::new()
        .child(Text::new("Counter"))
        .into_element()
}

fn main() {
    run(
        WindowConfig::default()
            .title("Lemon Counter")
            .size(900.0, 600.0),
        root,
    );
}
