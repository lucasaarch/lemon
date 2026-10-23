use crate::{icons::lucide, prelude::*, Cursor};

/// A labeled boolean checkbox bound to app-owned state.
/// Click, Space and Enter toggle the entire labeled row. Defaults to enabled.
///
/// ```
/// use lemon::prelude::*;
/// let cx = Cx::new();
/// let accepted = cx.state(false);
/// let checkbox = Checkbox::new(&cx, accepted).label("Accept terms");
/// ```
pub struct Checkbox(Toggle);

/// A labeled boolean switch bound to app-owned state.
/// Shares checkbox keyboard and disabled behavior; uses a sliding visual indicator.
///
/// ```
/// use lemon::prelude::*;
/// let cx = Cx::new();
/// let enabled = cx.state(true);
/// let switch = Switch::new(&cx, enabled).label("Notifications");
/// ```
pub struct Switch(Toggle);

struct Toggle {
    value: State<bool>,
    label: String,
    disabled: bool,
    theme: crate::Theme,
}

macro_rules! toggle_builder {
    ($name:ident, $switch:literal) => {
        impl $name {
            /// Binds this control to a boolean state. Does not consume a hook slot.
            pub fn new(cx: &Cx, value: State<bool>) -> Self {
                Self(Toggle {
                    value,
                    label: String::new(),
                    disabled: false,
                    theme: cx.theme(),
                })
            }
            /// Sets a label; clicking it also toggles the control. Empty by default.
            pub fn label(mut self, label: impl Into<String>) -> Self {
                self.0.label = label.into();
                self
            }
            /// Disables pointer and keyboard interaction and removes the Tab stop.
            pub fn disabled(mut self, disabled: bool) -> Self {
                self.0.disabled = disabled;
                self
            }
            /// Builds a themed element. Use [`Decorate`] for custom root styling.
            pub fn into_element(self) -> Element {
                self.0.build($switch)
            }
        }
        impl From<$name> for Element {
            fn from(control: $name) -> Element {
                control.into_element()
            }
        }
    };
}
toggle_builder!(Checkbox, false);
toggle_builder!(Switch, true);

impl Toggle {
    fn build(self, switch: bool) -> Element {
        let colors = self.theme.colors;
        let checked = self.value.get();
        let accent = if self.disabled {
            colors.foreground_disabled
        } else {
            colors.accent
        };
        let mut indicator = View::new()
            .width(if switch { 38.0 } else { 20.0 })
            .height(if switch { 22.0 } else { 20.0 })
            .rounding(if switch { 11.0 } else { 4.0 })
            .surface(if checked {
                accent
            } else {
                colors.surface_hover
            })
            .border(if checked { accent } else { colors.border }, 1.0);
        if switch {
            indicator = indicator.child(
                View::new()
                    .position(Position::Absolute)
                    .top(3.0)
                    .left(if checked { 19.0 } else { 3.0 })
                    .width(16.0)
                    .height(16.0)
                    .rounding(8.0)
                    .surface(colors.on_accent),
            );
        } else if checked {
            indicator = indicator
                .center()
                .child(Icon::new(lucide::CHECK).size(14.0).color(colors.on_accent));
        }
        let mut root = RowView::new()
            .spacing(8.0)
            .align(Align::Center)
            .inset_vertical(4.0)
            .transition(super::interaction_transition())
            .cursor(if self.disabled {
                Cursor::NotAllowed
            } else {
                Cursor::Pointer
            })
            .child(indicator);
        if !self.label.is_empty() {
            root = root.child(Text::new(self.label).color(if self.disabled {
                colors.foreground_disabled
            } else {
                colors.foreground
            }));
        }
        if !self.disabled {
            let click = self.value.clone();
            let key = self.value;
            root = root
                .focusable()
                .action(move || click.update(|value| *value = !*value))
                .on_key_down(move |event| {
                    if event.state == KeyState::Pressed
                        && !event.repeat
                        && matches!(
                            event.key,
                            LemonKey::Named(NamedKey::Enter | NamedKey::Space)
                        )
                    {
                        key.update(|value| *value = !*value);
                    }
                });
        }
        let mut element = root.into_element();
        if let Element::Row(node) = &mut element {
            node.paint.focus_ring_color = Some(accent.into());
        }
        element
    }
}
