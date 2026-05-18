//! Scroll widget demo: scrollable list with a proportional scrollbar thumb.
//!
//! Thirty items are rendered inside a `Scroll` viewport shorter than the content.
//! The scrollbar thumb tracks scroll position using the measured content height from layout.

use lemon::prelude::*;

fn app(cx: &Cx) -> Element {
    const COUNT: usize = 30;

    let mut list = Column::new().gap(4.0);
    for i in 0..COUNT {
        list = list.child(
            Row::new()
                .key(i as u64)
                .padding(4.0)
                .child(Text::new(format!("{:02}. Item — lorem ipsum", i + 1)).font_size(14.0)),
        );
    }

    Column::new()
        .padding(40.0)
        .gap(20.0)
        .children(children![
            Text::new("Scroll").font_size(22.0),
            Text::new("Scrollable viewport with a proportional scrollbar thumb.")
                .font_size(13.0)
                .color(Color::rgb8(140, 150, 170)),
            Scroll::new(cx, list).height(220.0).width(400.0),
        ])
        .into_element()
}

fn main() {
    run(
        WindowConfig::default()
            .title("Lemon — scroll")
            .size(560.0, 420.0),
        app,
    );
}
