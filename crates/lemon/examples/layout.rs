//! Rows arrange children horizontally, columns vertically, and views group styled content.

use lemon::prelude::*;

fn app(_cx: &Cx) -> Element {
    Column::new()
        .padding(24.0)
        .gap(16.0)
        .child(Text::new("Layout").font_size(24.0))
        .child(
            Row::new()
                .width(360.0)
                .justify_content(Justify::SpaceBetween)
                .child(Text::new("Left"))
                .child(Text::new("Right")),
        )
        .child(
            View::new()
                .width(360.0)
                .padding(16.0)
                .radius(8.0)
                .background(Color::rgb8(35, 45, 65))
                .border(Color::rgb8(90, 110, 150), 1.0)
                .child(
                    Column::new()
                        .gap(8.0)
                        .child(Text::new("A styled container"))
                        .child(Text::new("With vertically stacked children")),
                ),
        )
        .into_element()
}

fn main() {
    run(
        WindowConfig::default()
            .title("Lemon — layout")
            .size(440.0, 280.0),
        app,
    );
}
