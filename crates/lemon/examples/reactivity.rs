//! Signals hold state; memos derive values; effects perform side effects.
//! Watch the terminal for effect output. Reset uses `set`, increment uses `update`.

use lemon::prelude::*;

fn app(cx: &Cx) -> Element {
    let count = cx.state(0);
    let source = count.clone();
    let doubled = cx.derive(move || source.get() * 2);

    let source = count.clone();
    cx.watch(move || println!("Count changed: {}", source.get()));

    let label = count.clone();
    let increment = count.clone();
    ColumnView::new()
        .inset(24.0)
        .spacing(12.0)
        .children(children![
            Text::new(move || format!("Count: {}", label.get())),
            Text::new(move || format!("Doubled (memo): {}", doubled.get())),
            Button::new(cx, "Increment").action(move || increment.update(|n| *n += 1)),
            Button::new(cx, "Reset").action(move || count.set(0)),
        ])
        .into_element()
}

fn main() {
    run(
        WindowConfig::default()
            .title("Lemon — reactivity")
            .size(360.0, 260.0),
        app,
    );
}

#[cfg(test)]
#[path = "../tests/examples/reactivity.rs"]
mod tests;
