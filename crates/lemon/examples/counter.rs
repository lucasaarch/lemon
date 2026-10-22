//! Start here: a signal, reactive text, and a button.

use lemon::prelude::*;

fn app(cx: &Cx) -> Element {
    let count = cx.use_signal(0);
    let label = count.clone();

    Column::new()
        .padding(24.0)
        .gap(12.0)
        .child(Text::new(move || format!("Count: {}", label.get())).font_size(24.0))
        .child(Button::new(cx, "Increment").on_click(move || count.update(|n| *n += 1)))
        .into_element()
}

fn main() {
    run(
        WindowConfig::default()
            .title("Lemon — counter")
            .size(320.0, 180.0),
        app,
    );
}
