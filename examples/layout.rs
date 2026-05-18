//! Layout builders: [`Column`], [`Row`], [`View`], gap, padding, flex, colors, borders.

use lemon::prelude::*;

fn app(_cx: &Cx) -> Element {
    Column::new()
        .padding(24.0)
        .gap(16.0)
        .children(children![
            Text::new("Layout").font_size(18.0),
            Text::new("Containers compose with Taffy flexbox under the hood.")
                .font_size(14.0)
                .color(Color::rgb8(140, 150, 170)),
            Row::new().gap(12.0).children(children![
                panel("Column", Color::rgb8(45, 55, 72)),
                panel("Row", Color::rgb8(55, 65, 45)),
            ]),
            View::new()
                .padding(16.0)
                .background(Color::rgb8(30, 32, 42))
                .border(Color::rgb8(80, 90, 120), 1.5)
                .radius(8.0)
                .child(Column::new().gap(6.0).children(children![
                    Text::new("View — generic container").font_size(16.0),
                    Text::new(".padding .background .border .radius .overflow")
                        .font_size(13.0)
                        .color(Color::rgb8(150, 160, 180)),
                ])),
            Row::new()
                .gap(8.0)
                .justify_content(lemon::element::style::Justify::SpaceBetween)
                .width(400.0)
                .children(children![
                    Text::new("justify: SpaceBetween").font_size(13.0),
                    Text::new("←→").font_size(13.0),
                ]),
        ])
        .into_element()
}

fn panel(title: &'static str, bg: Color) -> Element {
    View::new()
        .padding(12.0)
        .background(bg)
        .radius(6.0)
        .width(160.0)
        .height(80.0)
        .child(
            Column::new()
                .gap(4.0)
                .child(Text::new(title).font_size(15.0)),
        )
        .into_element()
}

fn main() {
    run(
        WindowConfig::default()
            .title("Lemon — layout")
            .size(560.0, 460.0),
        app,
    );
}
