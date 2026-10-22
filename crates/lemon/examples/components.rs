//! Each component instance owns its hook state. Click either counter to see isolation.
//! `Component::new` takes a function pointer, so its view cannot capture local state.

use lemon::prelude::*;

fn counter(cx: &Cx) -> Element {
    let count = cx.state(0);
    let label = count.clone();

    RowView::new()
        .spacing(12.0)
        .align(Align::Center)
        .child(Text::new(move || format!("Count: {}", label.get())))
        .child(Button::new(cx, "+").action(move || count.update(|n| *n += 1)))
        .into_element()
}

fn app(_cx: &Cx) -> Element {
    ColumnView::new()
        .inset(24.0)
        .spacing(12.0)
        // Stable keys distinguish sibling instances of the same function.
        .child(Component::new(counter).key(1))
        .child(Component::new(counter).key(2))
        .into_element()
}

fn main() {
    run(
        WindowConfig::default()
            .title("Lemon — components")
            .size(320.0, 220.0),
        app,
    );
}
