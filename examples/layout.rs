//! Container layout, shared styling, and a canvas that scrolls on both axes.

use lemon::prelude::*;

fn app(cx: &Cx) -> Element {
    let theme = cx.theme();
    ColumnView::new()
        .fill()
        .inset(24.0)
        .spacing(16.0)
        .child(Text::new("Layout").size(24.0))
        .child(
            RowView::new()
                .width(360.0)
                .distribute(Distribute::Between)
                .child(Text::new("Left"))
                .child(Text::new("Right")),
        )
        .child(
            View::new()
                .width(360.0)
                .inset(16.0)
                .rounding(8.0)
                .surface(theme.colors.surface)
                .border(theme.colors.border, 1.0)
                .child(
                    ColumnView::new()
                        .spacing(8.0)
                        .child(Text::new("A styled container"))
                        .child(Text::new("With vertically stacked children")),
                ),
        )
        .child(
            ScrollView::new(
                cx,
                ColumnView::new()
                    .width(720.0)
                    .height(320.0)
                    .inset(16.0)
                    .surface(theme.colors.surface)
                    .distribute(Distribute::Between)
                    .child(Text::new(
                        "A wide canvas: scroll horizontally and vertically.",
                    ))
                    .child(Text::new(
                        "Bottom of the canvas. Shift+wheel moves horizontally.",
                    )),
            )
            .axis(ScrollAxis::Both)
            .width(360.0)
            .height(100.0),
        )
        .child(Text::new("Fixed to window").decorate(|view| {
            view.position(Position::Fixed)
                .right(16.0)
                .bottom(16.0)
                .inset(8.0)
                .surface(theme.colors.surface)
                .rounding(8.0)
        }))
        .into_element()
}

fn main() {
    run(|cx| {
        cx.open_window(
            WindowConfig::default()
                .title("Lemon — layout")
                .size(440.0, 420.0),
            app,
        );
    });
}
