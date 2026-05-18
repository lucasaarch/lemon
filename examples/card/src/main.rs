use lemon::{run, Color, Cx, WindowConfig};
use lemon_widgets::{Box_, Button, Column, Row, Text};

fn card(cx: &Cx) -> lemon::element::Element {
    let liked = cx.use_signal(false);
    let liked_read = liked.clone();

    Column::new()
        .padding(32.0)
        .gap(0.0)
        .child(
            Box_::new()
                .padding(24.0)
                .background(Color::rgb8(30, 30, 38))
                .radius(12.0)
                .child(
                    Column::new()
                        .gap(12.0)
                        .child(Text::new("Lemon Card").font_size(20.0))
                        .child(
                            Text::new("A composable UI component built with lemon-widgets.")
                                .font_size(14.0)
                                .color(Color::rgb8(160, 160, 180)),
                        )
                        .child(
                            Box_::new()
                                .flex_grow(1.0)
                                .height(1.0)
                                .background(Color::rgb8(55, 55, 70)),
                        )
                        .child(
                            Row::new()
                                .gap(8.0)
                                .child(
                                    Button::new(move || {
                                        if liked_read.get() {
                                            "Liked".to_string()
                                        } else {
                                            "Like".to_string()
                                        }
                                    })
                                    .on_click(move || liked.update(|v| *v = !*v)),
                                )
                                .child(Button::new("Share")),
                        ),
                ),
        )
        .into_element()
}

fn main() {
    run(
        WindowConfig::default()
            .title("Lemon Card")
            .size(600.0, 400.0),
        card,
    );
}
