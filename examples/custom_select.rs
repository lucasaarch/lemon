//! A mouse-operated picker built from primitives, with keyed options and reactive paint.
//! For the built-in dropdown and overlay behavior, use `Select` (see `form`).

use lemon::prelude::*;

const COLORS: [(&str, Color); 3] = [
    ("Blue", Color::rgb8(80, 145, 255)),
    ("Green", Color::rgb8(70, 180, 130)),
    ("Rose", Color::rgb8(235, 95, 135)),
];

fn app(cx: &Cx) -> Element {
    let selected = cx.state(0_usize);
    let open = cx.state(false);
    let swatch = selected.clone();
    let label = selected.clone();
    let toggle = open.clone();
    let outside = open.clone();

    let mut picker = ColumnView::new()
        .inset(24.0)
        .spacing(8.0)
        .on_click_outside(move || outside.set(false))
        .child(
            View::new()
                .width(220.0)
                .height(48.0)
                .rounding(8.0)
                .center()
                .surface(move || COLORS[swatch.get()].1)
                .cursor(Cursor::Pointer)
                .action(move || toggle.update(|value| *value = !*value))
                .child(Text::new(move || {
                    format!("Accent: {}", COLORS[label.get()].0)
                })),
        );

    if open.get() {
        let mut options = ColumnView::new().key(100).spacing(4.0);
        for (id, (name, color)) in COLORS.into_iter().enumerate() {
            let select = selected.clone();
            let close = open.clone();
            options = options.child(
                View::new()
                    .key(id as u64)
                    .inset(8.0)
                    .surface(color)
                    .cursor(Cursor::Pointer)
                    .action(move || {
                        select.set(id);
                        close.set(false);
                    })
                    .child(Text::new(name)),
            );
        }
        picker = picker.child(options);
    }

    picker.into_element()
}

fn main() {
    run(|cx| {
        cx.open_window(
            WindowConfig::default()
                .title("Lemon — custom select")
                .size(320.0, 300.0),
            app,
        );
    });
}
