use lemon::{run, Color, Cx, WindowConfig};
use lemon_widgets::{Button, Column, Row, Text};

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

    Column::new()
        .padding(24.0)
        .gap(12.0)
        .child(Text::new("Keyed List").font_size(20.0))
        .child(Button::new("Add item").on_click(move || {
            let id = next_id_add.get();
            next_id_add.update(|n| *n += 1);
            items_add.update(|list| {
                list.push(Item {
                    id,
                    label: format!("Item #{id}"),
                });
            });
        }))
        .child({
            let items_snapshot = items_read.get();
            let mut col = Column::new().gap(8.0);
            for item in items_snapshot {
                let items_remove = items.clone();
                let item_id = item.id;
                let label = item.label.clone();
                col = col.child(
                    Row::new()
                        .key(item.id)
                        .gap(8.0)
                        .align_items(lemon::element::style::Align::Center)
                        .child(
                            Text::new(label)
                                .font_size(15.0)
                                .color(Color::rgb8(210, 210, 230)),
                        )
                        .child(
                            Button::new("Remove")
                                .background(Color::rgb8(180, 50, 50))
                                .on_click(move || {
                                    items_remove.update(|list| list.retain(|i| i.id != item_id));
                                }),
                        ),
                );
            }
            col
        })
        .into_element()
}

fn main() {
    run(
        WindowConfig::default()
            .title("Keyed List")
            .size(500.0, 600.0),
        app,
    );
}
