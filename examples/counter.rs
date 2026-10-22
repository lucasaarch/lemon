//! Start here: a signal, reactive text, and a button.

use lemon::prelude::*;

fn app(cx: &Cx) -> Element {
    let count = cx.state(0);
    let label = count.clone();

    ColumnView::new()
        .inset(24.0)
        .spacing(12.0)
        .child(Text::new(move || format!("Count: {}", label.get())).size(24.0))
        .child(Button::new(cx, "Increment").action(move || count.update(|n| *n += 1)))
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
