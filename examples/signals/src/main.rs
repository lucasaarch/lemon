//! [`Cx::use_signal`]: persistent state, `.get()` / `.set()` / `.update()`, and reactive [`Text`].

use lemon::{run, Color, Cx, WindowConfig};
use lemon_widgets::{children, Button, Column, Row, Text};

fn app(cx: &Cx) -> lemon::element::Element {
    let name = cx.use_signal(String::new());
    let clicks = cx.use_signal(0u32);
    let enabled = cx.use_signal(true);

    let name_display = name.clone();
    let name_set = name.clone();
    let clicks_display = clicks.clone();
    let clicks_inc = clicks.clone();
    let enabled_read = enabled.clone();
    let enabled_toggle = enabled.clone();

    Column::new()
        .padding(24.0)
        .gap(14.0)
        .children(children![
            Text::new("use_signal").font_size(18.0),
            Text::new("Signals survive re-renders. Cloning a Signal shares the same cell.")
                .font_size(14.0)
                .color(Color::rgb8(140, 150, 170)),
            section_label("1. Read with .get() — dynamic Text re-renders when the signal changes"),
            Text::new(move || {
                if name_display.get().is_empty() {
                    "(type a name with the buttons)".to_string()
                } else {
                    format!("Hello, {}!", name_display.get())
                }
            })
            .font_size(16.0),
            Row::new().gap(8.0).children(children![
                Button::new("Alice").on_click({
                    let s = name_set.clone();
                    move || s.set("Alice".into())
                }),
                Button::new("Bob").on_click(move || name_set.set("Bob".into())),
                Button::new("Clear").on_click({
                    let s = name.clone();
                    move || s.set(String::new())
                }),
            ]),
            section_label("2. .update() for in-place mutation"),
            Text::new(move || format!("Button clicks: {}", clicks_display.get())).font_size(16.0),
            Button::new("Click me").on_click(move || clicks_inc.update(|n| *n += 1)),
            section_label("3. Multiple independent signals"),
            Text::new(move || {
                if enabled_read.get() {
                    "Feature flag: ON".to_string()
                } else {
                    "Feature flag: OFF".to_string()
                }
            })
            .font_size(16.0)
            .color(Color::rgb8(120, 200, 140)),
            Button::new("Toggle flag").on_click(move || enabled_toggle.update(|v| *v = !*v)),
        ])
        .into_element()
}

fn section_label(text: &'static str) -> lemon::element::Element {
    Text::new(text)
        .font_size(13.0)
        .color(Color::rgb8(100, 110, 130))
        .into_element()
}

fn main() {
    run(
        WindowConfig::default()
            .title("Lemon — use_signal")
            .size(600.0, 560.0),
        app,
    );
}
