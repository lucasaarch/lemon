//! A mouse-operated picker built from primitives, with keyed options and reactive paint.
//! For the built-in dropdown and overlay behavior, use `Select` (see `form`).

use lemon::prelude::*;

const COLORS: [(&str, Color); 3] = [
    ("Blue", Color::rgb8(80, 145, 255)),
    ("Green", Color::rgb8(70, 180, 130)),
    ("Rose", Color::rgb8(235, 95, 135)),
];

fn app(cx: &Cx) -> Element {
    let selected = cx.use_signal(0_usize);
    let open = cx.use_signal(false);
    let swatch = selected.clone();
    let label = selected.clone();
    let toggle = open.clone();
    let outside = open.clone();

    let mut picker = Column::new()
        .padding(24.0)
        .gap(8.0)
        .on_click_outside(move || outside.set(false))
        .child(
            View::new()
                .width(220.0)
                .height(48.0)
                .radius(8.0)
                .align_items(Align::Center)
                .justify_content(Justify::Center)
                .background(move || COLORS[swatch.get()].1)
                .cursor(Cursor::Pointer)
                .on_click(move || toggle.update(|value| *value = !*value))
                .child(Text::new(move || {
                    format!("Accent: {}", COLORS[label.get()].0)
                })),
        );

    if open.get() {
        let mut options = Column::new().key(100).gap(4.0);
        for (id, (name, color)) in COLORS.into_iter().enumerate() {
            let select = selected.clone();
            let close = open.clone();
            options = options.child(
                View::new()
                    .key(id as u64)
                    .padding(8.0)
                    .background(color)
                    .cursor(Cursor::Pointer)
                    .on_click(move || {
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
    run(
        WindowConfig::default()
            .title("Lemon — custom select")
            .size(320.0, 300.0),
        app,
    );
}
