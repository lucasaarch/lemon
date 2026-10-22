//! A themed view animates its width and opacity. Click Play to start or restart it.

use std::time::Duration;

use lemon::prelude::*;
use lemon::{run_with_theme, Theme};

fn app(cx: &Cx) -> Element {
    let theme = cx.use_theme();
    let animation = cx
        .use_animation(AnimationConfig::new(Duration::from_millis(1200)).easing(Easing::EaseInOut));
    let progress = animation.progress();

    Column::new()
        .padding(theme.spacing.lg)
        .gap(theme.spacing.md)
        .background(theme.colors.background)
        .children(children![
            Text::new(format!("Progress: {:.0}%", progress * 100.0)).color(theme.colors.foreground),
            View::new()
                .width(40.0 + progress * 220.0)
                .height(48.0)
                .radius(theme.radius.md)
                .opacity(0.3 + progress * 0.7)
                .background(theme.colors.accent),
            Button::new(cx, "Play").on_click(move || {
                animation.reset();
                animation.play();
            }),
        ])
        .into_element()
}

fn main() {
    run_with_theme(
        WindowConfig::default()
            .title("Lemon — animation")
            .size(340.0, 240.0),
        Theme::default_light(),
        app,
    );
}
