use crate::{icons::lucide, prelude::*, AnimationConfig, Cursor, Easing};
use std::{cell::RefCell, rc::Rc, time::Duration};

#[derive(Clone, Copy)]
struct ThumbMotion {
    from: f32,
    to: f32,
}
impl ThumbMotion {
    fn sample(self, progress: f32) -> f32 {
        self.from + (self.to - self.from) * progress
    }
    fn retarget(&mut self, target: f32, progress: f32) {
        self.from = self.sample(progress);
        self.to = target;
    }
}
fn thumb_offset(cx: &Cx, checked: bool) -> f32 {
    let target = if checked { 19.0 } else { 3.0 };
    let animation =
        cx.motion(AnimationConfig::new(Duration::from_millis(140)).easing(Easing::EaseOut));
    let motion = cx
        .state(Rc::new(RefCell::new(ThumbMotion {
            from: target,
            to: target,
        })))
        .get();
    let mut motion = motion.borrow_mut();
    if motion.to != target {
        motion.retarget(target, animation.progress());
        animation.reset();
        animation.play();
    }
    motion.sample(animation.progress())
}

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
/// Shares checkbox keyboard and disabled behavior. The thumb and colors animate
/// between states over 140 milliseconds, including programmatic state changes.
/// Interrupted movement resumes from its current position. Pointer clicks do not
/// draw a focus outline; keyboard navigation retains its focus indicator.
/// Declare switches in a stable order: each consumes two animation hook slots.
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
    thumb_offset: f32,
}

macro_rules! toggle_builder {
    ($name:ident, $switch:literal) => {
        impl $name {
            /// Binds this control to a boolean state.
            /// Declare constructors in a stable order: Checkbox uses no hook slots;
            /// Switch uses two persistent animation slots.
            pub fn new(cx: &Cx, value: State<bool>) -> Self {
                let thumb_offset = if $switch {
                    thumb_offset(cx, value.get())
                } else {
                    3.0
                };
                Self(Toggle {
                    value,
                    label: String::new(),
                    disabled: false,
                    theme: cx.theme(),
                    thumb_offset,
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
                    .left(self.thumb_offset)
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
            node.style.focus_ring_on_pointer = !switch;
        }
        element
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        host::HostContext,
        input::{hit_test::LogicalPoint, InputEvent},
        ui::UiState,
        Viewport,
    };
    use std::time::Instant;
    const SIZE: Viewport = Viewport {
        width: 300.0,
        height: 120.0,
    };
    fn update(ui: &mut UiState, now: Instant) {
        ui.update_at(SIZE, 1.0, ui.focused(), true, now).unwrap();
    }
    fn thumb(ui: &UiState) -> f32 {
        ui.retained().unwrap().root().unwrap().children[0].children[0]
            .style
            .inset
            .as_ref()
            .unwrap()
            .left
            .unwrap()
    }
    #[test]
    fn switch_animates_both_directions_for_external_updates_without_animating_mount() {
        let host = HostContext::new(crate::Theme::default_dark());
        let _scope = host.enter();
        let value = State::new(true);
        let source = value.clone();
        let mut ui = UiState::default();
        ui.mount(move |cx| Switch::new(cx, source.clone()).into_element())
            .unwrap();
        update(&mut ui, Instant::now());
        assert_eq!(thumb(&ui), 19.0);
        assert_eq!(host.animations().active_slots(), 0);
        value.set(false);
        update(&mut ui, Instant::now());
        assert!(
            thumb(&ui) > 12.0,
            "thumb must begin near its previous position"
        );
        assert_eq!(host.animations().active_slots(), 1);
        host.animations()
            .tick_active(Instant::now() + Duration::from_secs(1));
        host.animations().tick_frame();
        update(&mut ui, Instant::now());
        assert_eq!(thumb(&ui), 3.0);
        value.set(true);
        update(&mut ui, Instant::now());
        assert!(thumb(&ui) < 10.0, "reverse movement must also animate");
        host.animations()
            .tick_active(Instant::now() + Duration::from_secs(1));
        host.animations().tick_frame();
        update(&mut ui, Instant::now());
        assert_eq!(thumb(&ui), 19.0);
        assert_eq!(host.animations().active_slots(), 0);
    }
    #[test]
    fn interrupted_thumb_movement_continues_without_a_jump() {
        let mut motion = ThumbMotion {
            from: 3.0,
            to: 19.0,
        };
        assert_eq!(motion.sample(0.5), 11.0);
        motion.retarget(3.0, 0.5);
        assert_eq!(motion.sample(0.0), 11.0);
        assert_eq!(motion.sample(0.5), 7.0);
        assert_eq!(motion.sample(1.0), 3.0);
    }
    #[test]
    fn pointer_focus_hides_switch_outline_but_tab_restores_keyboard_chrome() {
        let mut ui = UiState::default();
        ui.mount(|cx| {
            Switch::new(cx, cx.state(false))
                .label("Notifications")
                .into_element()
        })
        .unwrap();
        let now = Instant::now();
        update(&mut ui, now);
        ui.dispatch_input(InputEvent::Click(LogicalPoint::new(15.0, 10.0)))
            .unwrap();
        let focused = ui.focused().expect("click preserves logical focus");
        update(&mut ui, now);
        update(&mut ui, now + Duration::from_secs(1));
        assert_eq!(
            ui.transitions.visuals[&focused]
                .paint
                .focus_ring_color
                .unwrap()
                .a,
            0.0
        );
        ui.dispatch_input(InputEvent::Key(KeyEvent {
            key: LemonKey::Named(NamedKey::Tab),
            state: KeyState::Pressed,
            modifiers: Modifiers::default(),
            repeat: false,
        }))
        .unwrap();
        update(&mut ui, now + Duration::from_secs(1));
        update(&mut ui, now + Duration::from_secs(2));
        assert_eq!(ui.focused(), Some(focused));
        assert!(
            ui.transitions.visuals[&focused]
                .paint
                .focus_ring_color
                .unwrap()
                .a
                > 0.0
        );
        ui.dispatch_input(InputEvent::Click(LogicalPoint::new(15.0, 10.0)))
            .unwrap();
        update(&mut ui, now + Duration::from_secs(2));
        update(&mut ui, now + Duration::from_secs(3));
        assert_eq!(
            ui.transitions.visuals[&focused]
                .paint
                .focus_ring_color
                .unwrap()
                .a,
            0.0
        );
    }
}
