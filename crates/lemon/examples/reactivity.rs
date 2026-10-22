//! Signals hold state; memos derive values; effects perform side effects.
//! Watch the terminal for effect output. Reset uses `set`, increment uses `update`.

use lemon::prelude::*;

fn app(cx: &Cx) -> Element {
    let count = cx.use_signal(0);
    let source = count.clone();
    let doubled = cx.use_memo(move || source.get() * 2);

    let source = count.clone();
    cx.use_effect(move || println!("Count changed: {}", source.get()));

    let label = count.clone();
    let increment = count.clone();
    Column::new()
        .padding(24.0)
        .gap(12.0)
        .children(children![
            Text::new(move || format!("Count: {}", label.get())),
            Text::new(move || format!("Doubled (memo): {}", doubled.get())),
            Button::new(cx, "Increment").on_click(move || increment.update(|n| *n += 1)),
            Button::new(cx, "Reset").on_click(move || count.set(0)),
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
