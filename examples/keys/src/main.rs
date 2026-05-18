//! Stable [`.key`](lemon::Column::key) on siblings so diff preserves identity when lists change.

use lemon::{run, Color, Cx, WindowConfig};
use lemon_widgets::{children, Button, Column, Row, Text};

#[derive(Clone)]
struct Item {
    id: u64,
    label: String,
}

fn app(cx: &Cx) -> lemon::element::Element {
    let items = cx.use_signal(Vec::<Item>::new());
    let next_id = cx.use_signal(0u64);

    let items_read = items.clone();
    let items_add = items.clone();
    let next_id_add = next_id.clone();

    let mut list = Column::new().gap(8.0);
    for item in items_read.get() {
        let items_remove = items.clone();
        let item_id = item.id;
        list = list.child(
            Row::new()
                .key(item.id)
                .gap(8.0)
                .align_items(lemon::element::style::Align::Center)
                .children(children![
                    Text::new(item.label).font_size(16.0),
                    Button::new(cx, "Remove")
                        .background(Color::rgb8(180, 60, 60))
                        .on_click(move || {
                            items_remove.update(|list| list.retain(|i| i.id != item_id));
                        }),
                ]),
        );
    }

    Column::new()
        .padding(24.0)
        .gap(12.0)
        .children(children![
            Text::new("keys").font_size(18.0),
            Text::new(
                "Give each dynamic sibling a stable .key(id). Add/remove/reorder without resetting unrelated row state.",
            )
            .font_size(14.0)
            .color(Color::rgb8(140, 150, 170)),
            Button::new(cx, "Add item").on_click(move || {
                let id = next_id_add.get();
                next_id_add.update(|n| *n += 1);
                items_add.update(|list| {
                    list.push(Item {
                        id,
                        label: format!("Item #{id}"),
                    });
                });
            }),
            list,
        ])
        .into_element()
}

fn main() {
    run(
        WindowConfig::default()
            .title("Lemon — keys")
            .size(560.0, 580.0),
        app,
    );
}
