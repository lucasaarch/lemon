//! [`Cx::use_memo`]: derived values that recompute only when tracked signals change.

use lemon::{run, Color, Cx, WindowConfig};
use lemon_widgets::{children, Button, Column, Row, Text};

fn app(cx: &Cx) -> lemon::element::Element {
    let a = cx.use_signal(4i32);
    let b = cx.use_signal(7i32);
    let noise = cx.use_signal(0i32);

    let a_read = a.clone();
    let b_read = b.clone();
    let sum = cx.use_memo(move || a_read.get() + b_read.get());

    let a_read2 = a.clone();
    let b_read2 = b.clone();
    let product = cx.use_memo(move || a_read2.get() * b_read2.get());

    let sum_display = sum.clone();
    let product_display = product.clone();
    let noise_display = noise.clone();

    Column::new()
        .padding(24.0)
        .gap(14.0)
        .children(children![
            Text::new("use_memo").font_size(18.0),
            Text::new(
                "Memos cache a computed value. They re-run only when signals read inside the closure change.",
            )
            .font_size(14.0)
            .color(Color::rgb8(140, 150, 170)),
            number_row(cx, "a", a.clone()),
            number_row(cx, "b", b.clone()),
            Text::new(move || format!("a + b = {} (memo)", sum_display.get())).font_size(16.0),
            Text::new(move || format!("a × b = {} (memo)", product_display.get())).font_size(16.0),
            section_label("Unrelated signal — memos above should not change"),
            Text::new(move || format!("noise = {}", noise_display.get())).font_size(16.0),
            Button::new(cx, "Bump noise").on_click({
                let n = noise.clone();
                move || n.update(|v| *v += 1)
            }),
        ])
        .into_element()
}

fn number_row(cx: &Cx, label: &'static str, signal: lemon::Signal<i32>) -> lemon::element::Element {
    let read = signal.clone();
    let dec = signal.clone();
    let inc = signal.clone();

    Row::new()
        .gap(8.0)
        .align_items(lemon::element::style::Align::Center)
        .children(children![
            Text::new(label).font_size(16.0),
            Button::new(cx, "−")
                .on_click(move || dec.update(|n| *n -= 1))
                .width(44.0),
            Text::new(move || read.get().to_string()).font_size(16.0),
            Button::new(cx, "+")
                .on_click(move || inc.update(|n| *n += 1))
                .width(44.0),
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
            .title("Lemon — use_memo")
            .size(520.0, 480.0),
        app,
    );
}
