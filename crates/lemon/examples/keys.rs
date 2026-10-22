//! Stable item IDs, rather than list positions, identify rows across removal and reordering.

use lemon::prelude::*;

fn app(cx: &Cx) -> Element {
    let items = cx.use_signal(vec![1_u64, 2, 3]);
    let reverse = items.clone();
    let mut list = Column::new().gap(8.0);

    for id in items.get() {
        let remove = items.clone();
        list = list.child(
            Row::new()
                .key(id)
                .gap(12.0)
                .align_items(Align::Center)
                .child(Text::new(format!("Item {id}")))
                // Use primitives so changing list length does not change the hook order.
                .child(
                    View::new()
                        .padding(8.0)
                        .cursor(Cursor::Pointer)
                        .on_click(move || remove.update(|items| items.retain(|item| *item != id)))
                        .child(Text::new("Remove")),
                ),
        );
    }

    Column::new()
        .padding(24.0)
        .gap(12.0)
        .child(Button::new(cx, "Reverse").on_click(move || reverse.update(|items| items.reverse())))
        .child(Button::new(cx, "Reset").on_click(move || items.set(vec![1, 2, 3])))
        .child(list)
        .into_element()
}

fn main() {
    run(
        WindowConfig::default()
            .title("Lemon — keys")
            .size(360.0, 340.0),
        app,
    );
}

#[cfg(test)]
#[path = "../tests/examples/keys.rs"]
mod tests;
