//! Select widget demo: choose a font size and see the effect applied live.
//!
//! A `Select<FontSize>` dropdown controls the size of a preview sentence.
//! Demonstrates open/close behavior, placeholder, and reactive value binding.

use lemon::prelude::*;

#[derive(Clone, PartialEq)]
enum FontSize {
    Small,
    Medium,
    Large,
    XLarge,
}

fn app(cx: &Cx) -> Element {
    let font_size = cx.use_signal(None::<FontSize>);

    let pts = match font_size.get() {
        Some(FontSize::Small) => 11.0_f32,
        Some(FontSize::Medium) => 16.0,
        Some(FontSize::Large) => 22.0,
        Some(FontSize::XLarge) => 30.0,
        None => 16.0,
    };

    Column::new()
        .padding(40.0)
        .gap(24.0)
        .children(children![
            Text::new("Select").font_size(22.0),
            Text::new("Choose a size from the dropdown to resize the preview below.")
                .font_size(13.0)
                .color(Color::rgb8(140, 150, 170)),
            Select::new(
                cx,
                font_size,
                vec![
                    (FontSize::Small, "Small — 11pt".to_string()),
                    (FontSize::Medium, "Medium — 16pt".to_string()),
                    (FontSize::Large, "Large — 22pt".to_string()),
                    (FontSize::XLarge, "X-Large — 30pt".to_string()),
                ],
            )
            .placeholder("Choose a size")
            .width(200.0),
            Text::new("The quick brown fox jumps over the lazy dog.").font_size(pts),
        ])
        .into_element()
}

fn main() {
    run(
        WindowConfig::default()
            .title("Lemon — select")
            .size(560.0, 380.0),
        app,
    );
}
