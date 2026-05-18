//! Quick start: minimal reactive UI with [`Cx::use_signal`], dynamic [`Text`], and [`Button`].

use lemon_widgets::prelude::*;

fn app(cx: &Cx) -> Element {
    let count = cx.use_signal(0i32);
    let label = count.clone();
    let inc = count.clone();
    let dec = count.clone();
    let reset = count.clone();

    Column::new()
        .padding(24.0)
        .gap(12.0)
        .children(children![
            Text::new("Counter").font_size(22.0),
            Text::new("Start here — then open `signals`, `memo`, and `effects` for deep dives.")
                .font_size(14.0)
                .color(Color::rgb8(140, 150, 170)),
            Text::new(move || format!("Count: {}", label.get())).font_size(32.0),
            Row::new().gap(8.0).children(children![
                Button::new("−")
                    .on_click(move || dec.update(|n| *n -= 1))
                    .width(52.0),
                Button::new("+")
                    .on_click(move || inc.update(|n| *n += 1))
                    .width(52.0),
                Button::new("Reset")
                    .on_click(move || reset.set(0))
                    .width(72.0),
            ]),
        ])
        .into_element()
}

fn main() {
    run(
        WindowConfig::default()
            .title("Lemon — counter")
            .size(480.0, 280.0),
        app,
    );
}
