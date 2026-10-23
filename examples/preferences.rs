//! A reader's preferences form with validation, apply/revert and a shared preview window.
//! Changes are session-local. The preview changes only when Apply is pressed.
use lemon::prelude::*;
#[derive(Clone, Debug, PartialEq)]
struct Preferences {
    name: String,
    font: String,
    size: f32,
    comfortable: bool,
    guide: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            name: "Alex".into(),
            font: "sans-serif".into(),
            size: 0.4,
            comfortable: true,
            guide: true,
        }
    }
}
#[derive(Clone)]
struct Form {
    name: State<String>,
    font: State<Option<String>>,
    size: State<f32>,
    comfortable: State<bool>,
    guide: State<bool>,
    applied: State<Preferences>,
}
impl Form {
    fn new() -> Self {
        let value = Preferences::default();
        Self {
            name: State::new(value.name.clone()),
            font: State::new(Some(value.font.clone())),
            size: State::new(value.size),
            comfortable: State::new(value.comfortable),
            guide: State::new(value.guide),
            applied: State::new(value),
        }
    }
    fn draft(&self) -> Preferences {
        Preferences {
            name: self.name.get().trim().into(),
            font: self.font.get().unwrap_or_else(|| "sans-serif".into()),
            size: self.size.get(),
            comfortable: self.comfortable.get(),
            guide: self.guide.get(),
        }
    }
    fn restore(&self, value: Preferences) {
        self.name.set(value.name);
        self.font.set(Some(value.font));
        self.size.set(value.size);
        self.comfortable.set(value.comfortable);
        self.guide.set(value.guide);
    }
    fn apply(&self) -> bool {
        let value = self.draft();
        if value.name.is_empty() {
            return false;
        }
        self.applied.set(value);
        true
    }
}
fn preview(cx: &Cx, preferences: &State<Preferences>) -> Element {
    let theme = cx.theme();
    let value = preferences.get();
    let size = 14.0 + value.size * 12.0;
    let article=ColumnView::new().fill_width().spacing(if value.comfortable {20.0}else{10.0})
        .child(Text::new("The value of a quiet hour").size(size*1.6).weight(650).font(&value.font))
        .child(Text::new(format!("Reading with {} · {:.0} pt",value.name,size)).color(theme.colors.foreground_secondary))
        .child(Text::new("A good workspace gives your attention somewhere to settle. A short list, a clear page and a little room to think are often enough to begin.\n\nSmall decisions add up. Choose a typeface you enjoy reading, make the text comfortable and keep only the tools you need nearby.\n\nThere is no perfect arrangement. The useful one is the one that helps you return to your work, one quiet hour at a time.")
            .font(value.font).size(size).leading(if value.comfortable {1.7}else{1.3}))
        .decorate(|view|view.inset(24.0).border_left(if value.guide {theme.colors.accent}else{theme.colors.surface},3.0).surface(theme.colors.surface).rounding(theme.radius.lg));
    ColumnView::new()
        .fill()
        .inset(24.0)
        .spacing(16.0)
        .surface(theme.colors.background)
        .child(
            Text::new("Reader")
                .size(14.0)
                .color(theme.colors.foreground_secondary),
        )
        .child(ScrollView::new(cx, article).decorate(|view| view.fill().height(0.0).grow(1.0)))
        .into_element()
}
fn app(cx: &Cx, form: &Form) -> Element {
    let theme = cx.theme();
    let draft = form.draft();
    let applied = form.applied.get();
    let apply = form.clone();
    let revert = form.clone();
    let reset = form.clone();
    let windows = Cx::new();
    let preview_value = form.applied.clone();
    let toaster = Toaster::new(cx);
    let notice = toaster.clone();
    let panel = ColumnView::new()
        .fill_width()
        .inset(24.0)
        .spacing(16.0)
        .surface(theme.colors.surface)
        .rounding(theme.radius.lg)
        .child(
            Text::new("Personalize your reading space")
                .size(20.0)
                .weight(600),
        )
        .child(Text::new("Reader name"))
        .child(
            TextField::new(cx, form.name.clone())
                .placeholder("Your name")
                .decorate(|view| view.fill_width()),
        )
        .child(
            Text::new(if draft.name.is_empty() {
                "Enter a name before applying."
            } else {
                "Shown in the preview's reading status."
            })
            .size(12.0)
            .color(if draft.name.is_empty() {
                theme.colors.error
            } else {
                theme.colors.foreground_secondary
            }),
        )
        .child(Text::new("Typeface"))
        .child(
            Select::new(
                cx,
                form.font.clone(),
                vec![
                    ("sans-serif".into(), "Sans serif".into()),
                    ("serif".into(), "Serif".into()),
                    ("monospace".into(), "Monospace".into()),
                ],
            )
            .width(260.0),
        )
        .child(Text::new(format!(
            "Text size · {:.0} pt",
            14.0 + draft.size * 12.0
        )))
        .child(Slider::new(cx, form.size.clone()).width(280.0))
        .child(Switch::new(cx, form.comfortable.clone()).label("Comfortable spacing"))
        .child(Checkbox::new(cx, form.guide.clone()).label("Show reading guide"));
    ColumnView::new()
        .fill()
        .inset(24.0)
        .spacing(16.0)
        .surface(theme.colors.background)
        .child(Text::new("Reader preferences").size(28.0).weight(650))
        .child(
            Text::new("Apply changes to every open preview. Preferences last for this session.")
                .color(theme.colors.foreground_secondary),
        )
        .child(ScrollView::new(cx, panel).decorate(|view| view.fill_width().grow(1.0).height(0.0)))
        .child(
            RowView::new()
                .spacing(8.0)
                .child(
                    Button::new(cx, "Apply")
                        .style(ButtonStyle::Filled)
                        .disabled(draft.name.is_empty() || draft == applied)
                        .action(move || {
                            if apply.apply() {
                                notice.show("Preferences applied to all previews");
                            }
                        }),
                )
                .child(
                    Button::new(cx, "Revert")
                        .style(ButtonStyle::Bordered)
                        .disabled(draft == applied)
                        .action(move || revert.restore(revert.applied.get())),
                )
                .child(
                    Button::new(cx, "Defaults")
                        .style(ButtonStyle::Clean)
                        .action(move || reset.restore(Preferences::default())),
                )
                .child(
                    Button::new(cx, "Open preview")
                        .style(ButtonStyle::Bordered)
                        .action(move || {
                            let value = preview_value.clone();
                            windows.open_window(
                                WindowConfig::default()
                                    .title("Reader preview")
                                    .size(660.0, 620.0),
                                move |cx| preview(cx, &value),
                            );
                        }),
                ),
        )
        .child(
            Text::new(if draft == applied {
                "All changes applied"
            } else {
                "Unapplied changes"
            })
            .size(12.0)
            .color(theme.colors.foreground_secondary),
        )
        .child(toaster)
        .into_element()
}
fn main() {
    let form = Form::new();
    run(move |cx| {
        cx.open_window(
            WindowConfig::default()
                .title("Reader preferences")
                .size(640.0, 700.0),
            move |cx| app(cx, &form),
        )
    });
}
#[cfg(test)]
#[path = "../tests/examples/preferences.rs"]
mod tests;
