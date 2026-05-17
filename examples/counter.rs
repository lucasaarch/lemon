//! Counter demo from the Lemon architecture spec.

use lemon::{run, Button, Column, Cx, Text, WindowConfig};

fn counter(cx: &Cx) -> lemon::element::Element {
    let count = cx.use_signal(0i32);
    let count_text = count.clone();
    let count_btn = count.clone();

    Column::new()
        .gap(12.0)
        .padding(24.0)
        .child(
            Text::new(move || count_text.get().to_string())
                .font_size(24.0)
                .color(lemon::Color::rgb8(235, 235, 240)),
        )
        .child(Button::new("Incrementar").on_click(move || {
            count_btn.update(|n| *n += 1);
        }))
        .into_element()
}

fn main() {
    run(
        WindowConfig::default()
            .title("Lemon Counter")
            .size(900.0, 600.0),
        counter,
    );
}
