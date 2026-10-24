//! A working focus timer with session presets, pause/resume, reset and completion notification.
//! Time is measured by Instant; motion only schedules UI frames while the timer runs.
use lemon::prelude::*;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Default)]
struct Session {
    elapsed: Duration,
    started: Option<Instant>,
    finished: bool,
}

impl Session {
    fn elapsed_at(&self, now: Instant) -> Duration {
        self.elapsed
            + self
                .started
                .map(|start| now.saturating_duration_since(start))
                .unwrap_or_default()
    }

    fn start(&mut self, now: Instant) {
        if self.started.is_none() && !self.finished {
            self.started = Some(now);
        }
    }

    fn pause(&mut self, now: Instant) {
        self.elapsed = self.elapsed_at(now);
        self.started = None;
    }

    fn finish(&mut self, total: Duration) {
        self.elapsed = total;
        self.started = None;
        self.finished = true;
    }
}

#[derive(Clone, PartialEq)]
struct TimerProps {
    seconds: u64,
}

fn timer(cx: &Cx, props: &TimerProps) -> Element {
    let theme = cx.theme();
    let total = Duration::from_secs(props.seconds);
    let session = cx.state(Session::default());
    let intention = cx.state(String::new());
    let clock = cx.motion(AnimationConfig::new(total));
    let _frame = clock.progress();
    let toaster = Toaster::new(cx);
    let notice = toaster.clone();
    let watched = session.clone();
    let ticking = clock.clone();
    cx.watch(move || {
        let value = watched.get();
        let _frame = ticking.progress();
        if value.started.is_some() && value.elapsed_at(Instant::now()) >= total {
            watched.update(|session| session.finish(total));
            ticking.reset();
            notice.show_kind_for(
                ToastKind::Success,
                "Session complete. Time for a break.",
                Duration::from_secs(8),
            );
        }
    });
    let value = session.get();
    let elapsed = value.elapsed_at(Instant::now()).min(total);
    let remaining = total.saturating_sub(elapsed).as_secs_f64().ceil() as u64;
    let fraction = (elapsed.as_secs_f32() / total.as_secs_f32()).clamp(0.0, 1.0);
    let start = session.clone();
    let play = clock.clone();
    let pause = session.clone();
    let stop = clock.clone();
    let reset = session.clone();
    let rewind = clock.clone();
    ColumnView::new()
        .fill_width()
        .spacing(20.0)
        .child(
            TextField::new(cx, intention)
                .placeholder("What will you focus on?")
                .decorate(|view| view.fill_width()),
        )
        .child(
            ColumnView::new()
                .fill_width()
                .center()
                .inset(32.0)
                .spacing(12.0)
                .rounding(theme.radius.lg)
                .gradient(RadialGradient::new(
                    (0.5, 0.0),
                    1.3,
                    theme.colors.surface_hover,
                    theme.colors.surface,
                ))
                .child(
                    Text::new(format!("{:02}:{:02}", remaining / 60, remaining % 60))
                        .size(64.0)
                        .weight(600)
                        .font("monospace"),
                )
                .child(
                    Text::new(if value.finished {
                        "Session complete"
                    } else if value.started.is_some() {
                        "Focus in progress"
                    } else if value.elapsed.is_zero() {
                        "Ready when you are"
                    } else {
                        "Paused"
                    })
                    .color(theme.colors.foreground_secondary),
                ),
        )
        .child(
            RowView::new()
                .height(6.0)
                .fill_width()
                .surface(theme.colors.surface_hover)
                .rounding(3.0)
                .child(
                    View::new()
                        .height(6.0)
                        .width(0.0)
                        .grow(fraction)
                        .surface(theme.colors.accent)
                        .rounding(3.0),
                )
                .child(View::new().width(0.0).height(6.0).grow(1.0 - fraction)),
        )
        .child(
            RowView::new()
                .spacing(8.0)
                .child(
                    Button::new(
                        cx,
                        if value.elapsed.is_zero() {
                            "Start"
                        } else {
                            "Resume"
                        },
                    )
                    .style(ButtonStyle::Filled)
                    .disabled(value.started.is_some() || value.finished)
                    .action(move || {
                        start.update(|session| session.start(Instant::now()));
                        play.play();
                    }),
                )
                .child(
                    Button::new(cx, "Pause")
                        .style(ButtonStyle::Bordered)
                        .disabled(value.started.is_none())
                        .action(move || {
                            pause.update(|session| session.pause(Instant::now()));
                            stop.reset();
                        }),
                )
                .child(
                    Button::new(cx, "Reset")
                        .style(ButtonStyle::Clean)
                        .action(move || {
                            reset.set(Session::default());
                            rewind.reset();
                        }),
                ),
        )
        .child(toaster)
        .into_element()
}

fn app(cx: &Cx) -> Element {
    let theme = cx.theme();
    let duration = cx.state(Some(1500_u64));
    let seconds = duration.get().unwrap_or(1500);
    ColumnView::new()
        .fill()
        .inset(28.0)
        .spacing(20.0)
        .surface(theme.colors.background)
        .child(Text::new("A little room to focus").size(26.0).weight(650))
        .child(
            Text::new("Choose a session, name your intention and begin.")
                .color(theme.colors.foreground_secondary),
        )
        .child(
            Select::new(
                cx,
                duration,
                vec![
                    (300, "5 minute reset".into()),
                    (1500, "25 minute focus".into()),
                    (3000, "50 minute deep work".into()),
                ],
            )
            .width(260.0),
        )
        // Changing the preset intentionally creates a fresh timer with its own hook state.
        .child(Component::new_with_props(timer, TimerProps { seconds }).key(seconds))
        .child(
            Text::new("Changing the session resets the timer. Keep the window open while timing.")
                .size(12.0)
                .color(theme.colors.foreground_secondary),
        )
        .into_element()
}

fn main() {
    run(|cx| {
        cx.open_window(
            WindowConfig::default().title("Focus").size(520.0, 600.0),
            app,
        )
    });
}

#[cfg(test)]
#[path = "../tests/examples/focus.rs"]
mod tests;
