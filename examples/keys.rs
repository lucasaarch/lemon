//! Stable item IDs, rather than list positions, identify rows across removal and reordering.

use lemon::prelude::*;

fn app(cx: &Cx) -> Element {
    let items = cx.state(vec![1_u64, 2, 3]);
    let reverse = items.clone();
    let mut list = ColumnView::new().spacing(8.0);

    for id in items.get() {
        let remove = items.clone();
        list = list.child(
            RowView::new()
                .key(id)
                .spacing(12.0)
                .align(Align::Center)
                .child(Text::new(format!("Item {id}")))
                // Use primitives so changing list length does not change the hook order.
                .child(
                    View::new()
                        .inset(8.0)
                        .cursor(Cursor::Pointer)
                        .action(move || remove.update(|items| items.retain(|item| *item != id)))
                        .child(Text::new("Remove")),
                ),
        );
    }

    ColumnView::new()
        .inset(24.0)
        .spacing(12.0)
        .child(Button::new(cx, "Reverse").action(move || reverse.update(|items| items.reverse())))
        .child(Button::new(cx, "Reset").action(move || items.set(vec![1, 2, 3])))
        .child(list)
        .into_element()
}

fn main() {
    run(|cx| {
        cx.open_window(
            WindowConfig::default()
                .title("Lemon — keys")
                .size(360.0, 340.0),
            app,
        );
    });
}

#[cfg(test)]
#[path = "../tests/examples/keys.rs"]
mod tests;
