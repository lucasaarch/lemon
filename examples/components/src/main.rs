//! [`Component::new`]: nested views with isolated hook state (function pointer + optional `.key`).

use lemon::{run, Color, Component, Cx, WindowConfig};
use lemon_widgets::{children, Button, Column, Row, Text};

fn mini_counter(cx: &Cx) -> lemon::element::Element {
    let n = cx.use_signal(0i32);
    let label = n.clone();
    let inc = n.clone();

    Row::new()
        .gap(8.0)
        .align_items(lemon::element::style::Align::Center)
        .children(children![
            Text::new(move || format!("{}", label.get())).font_size(20.0),
            Button::new("+")
                .on_click(move || inc.update(|v| *v += 1))
                .width(44.0),
        ])
        .into_element()
}

fn app(cx: &Cx) -> lemon::element::Element {
    let parent = cx.use_signal(0i32);
    let parent_label = parent.clone();
    let parent_inc = parent.clone();

    Column::new()
        .padding(24.0)
        .gap(14.0)
        .children(children![
            Text::new("Component").font_size(22.0),
            Text::new(
                "Component::new(fn) uses a function pointer — no capturing closures. Give each instance a .key when you have siblings of the same function.",
            )
            .font_size(14.0)
            .color(Color::rgb8(140, 150, 170)),
            Text::new("Parent scope (root Cx):")
                .font_size(15.0)
                .color(Color::rgb8(160, 170, 190)),
            Row::new().gap(8.0).children(children![
                Text::new(move || format!("parent = {}", parent_label.get())).font_size(18.0),
                Button::new("+ parent")
                    .on_click(move || parent_inc.update(|v| *v += 1))
                    .width(100.0),
            ]),
            Text::new("Two child components — separate hook slots:")
                .font_size(15.0)
                .color(Color::rgb8(160, 170, 190)),
            Component::new(mini_counter).key(1),
            Component::new(mini_counter).key(2),
        ])
        .into_element()
}

fn main() {
    run(
        WindowConfig::default()
            .title("Lemon — Component")
            .size(480.0, 400.0),
        app,
    );
}
