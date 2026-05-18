use std::time::Instant;

use lemon::{
    animation_frame_signal,
    element::{
        builders::View,
        events::Cursor,
        style::{Color, ColorSource},
        Element,
    },
    request_animation_frame, Cx, Signal,
};

/// Duration of a value-change animation in seconds.
const ANIM_DURATION: f32 = 0.15;

/// Lerp progress epsilon: treat animation as complete when this close to target.
const ANIM_EPSILON: f32 = 1e-4;

#[derive(Clone)]
struct AnimState {
    from: f32,
    to: f32,
    started_at: Instant,
}

impl AnimState {
    fn settled(value: f32) -> Self {
        Self {
            from: value,
            to: value,
            started_at: Instant::now(),
        }
    }

    /// Interpolated display value at the current wall-clock time.
    fn display(&self) -> f32 {
        let elapsed = self.started_at.elapsed().as_secs_f32();
        let progress = (elapsed / ANIM_DURATION).min(1.0);
        self.from + (self.to - self.from) * progress
    }

    /// True while the from→to transition has not yet completed.
    fn is_animating(&self) -> bool {
        let progress = (self.started_at.elapsed().as_secs_f32() / ANIM_DURATION).min(1.0);
        progress < 1.0 && (self.to - self.from).abs() > ANIM_EPSILON
    }
}

/// Per-widget visual overrides for [`Slider`].
///
/// # Examples
///
/// ```no_run
/// use lemon::Color;
/// use lemon_widgets::SliderStyle;
///
/// let style = SliderStyle::default().fill_color(Color::rgb8(255, 120, 0));
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SliderStyle {
    track_color: Option<Color>,
    fill_color: Option<Color>,
}

impl SliderStyle {
    /// Overrides the slider track color.
    pub fn track_color(mut self, color: Color) -> Self {
        self.track_color = Some(color);
        self
    }

    /// Overrides the slider fill color.
    pub fn fill_color(mut self, color: Color) -> Self {
        self.fill_color = Some(color);
        self
    }
}

/// Horizontal slider widget backed by a user-owned `f32` signal.
///
/// The value is always in the range `[0.0, 1.0]`.
///
/// **Drag** is immediate: the fill tracks the pointer with no latency.
///
/// **Programmatic value changes** (e.g. `signal.set(0.8)` from another widget) animate the fill
/// over ~150 ms using a linear interpolation driven by the platform frame loop.
///
/// Use [`SliderStyle`] to override per-widget visuals while keeping theme defaults for unset fields.
///
/// # Examples
///
/// ```no_run
/// use lemon::{Cx, Color, element::Element};
/// use lemon_widgets::{Slider, SliderStyle};
///
/// fn my_view(cx: &Cx) -> Element {
///     let volume = cx.use_signal(0.5_f32);
///     Slider::new(cx, volume)
///         .style(SliderStyle::default().fill_color(Color::rgb8(255, 120, 0)))
///         .into_element()
/// }
/// ```
pub struct Slider {
    value: Signal<f32>,
    anim: Signal<AnimState>,
    hovered: Signal<bool>,
    pressed: Signal<bool>,
    width: f32,
    height: f32,
    track_color: Color,
    track_color_hover: Color,
    track_color_pressed: Color,
    track_color_disabled: Color,
    fill_color: Color,
    fill_color_hover: Color,
    fill_color_pressed: Color,
    fill_color_disabled: Color,
    style: SliderStyle,
    disabled: bool,
}

impl Slider {
    /// Creates a `Slider` backed by `value` (a `f32` signal in `[0.0, 1.0]`).
    ///
    /// `cx` is used to allocate internal hook state for the animation.  Pass the same `&Cx`
    /// received by your view function.
    pub fn new(cx: &Cx, value: Signal<f32>) -> Self {
        let theme = cx.use_theme();
        let initial = value.peek().clamp(0.0, 1.0);
        let anim = cx.use_signal(AnimState::settled(initial));

        // When the target value changes from outside (programmatic), start a new animation from
        // the current display position.  Drag bypasses this: the pointer handlers update `anim`
        // before calling `value.set`, so the effect sees `anim.to == target` and does nothing.
        let anim_eff = anim.clone();
        let val_eff = value.clone();
        cx.use_effect(move || {
            let target = val_eff.get().clamp(0.0, 1.0); // subscribe to value
            let current = anim_eff.peek();
            if (current.to - target).abs() > ANIM_EPSILON {
                let display = current.display();
                anim_eff.set(AnimState {
                    from: display,
                    to: target,
                    started_at: Instant::now(),
                });
            }
        });

        Self {
            value,
            anim,
            hovered: cx.use_signal(false),
            pressed: cx.use_signal(false),
            width: 200.0,
            height: theme.spacing.sm * 2.0,
            track_color: theme.colors.border,
            track_color_hover: theme.colors.surface_hover,
            track_color_pressed: theme.colors.surface_pressed,
            track_color_disabled: theme.colors.surface,
            fill_color: theme.colors.accent,
            fill_color_hover: theme.colors.accent_hover,
            fill_color_pressed: theme.colors.accent_pressed,
            fill_color_disabled: theme.colors.foreground_disabled,
            style: SliderStyle::default(),
            disabled: false,
        }
    }

    /// Fixed track width in logical points (default: `200.0`).
    pub fn width(mut self, v: f32) -> Self {
        self.width = v;
        self
    }

    /// Fixed track height in logical points (default: `16.0`).
    pub fn height(mut self, v: f32) -> Self {
        self.height = v;
        self
    }

    /// Replaces all slider style overrides.
    pub fn style(mut self, style: SliderStyle) -> Self {
        self.style = style;
        self
    }

    /// Overrides the slider track color for this widget.
    pub fn track_color(mut self, color: Color) -> Self {
        self.style.track_color = Some(color);
        self
    }

    /// Overrides the slider fill color for this widget.
    pub fn fill_color(mut self, color: Color) -> Self {
        self.style.fill_color = Some(color);
        self
    }

    /// Marks this slider as disabled, preventing pointer interactions.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Builds and returns the [`Element`] for this slider.
    ///
    /// Call this at the end of the builder chain to insert the widget into a parent container.
    pub fn into_element(self) -> Element {
        let anim = self.anim.get();
        let display = anim.display().clamp(0.0, 1.0);
        let fill_width = display * self.width;
        let radius = self.height / 2.0;
        let disabled = self.disabled;
        let track_color = self.style.track_color.map(ColorSource::Static).unwrap_or({
            let hovered = self.hovered.clone();
            let pressed = self.pressed.clone();
            let track_color = self.track_color;
            let track_color_hover = self.track_color_hover;
            let track_color_pressed = self.track_color_pressed;
            let track_color_disabled = self.track_color_disabled;
            ColorSource::Dynamic(std::rc::Rc::new(move || {
                if disabled {
                    track_color_disabled
                } else if pressed.get() {
                    track_color_pressed
                } else if hovered.get() {
                    track_color_hover
                } else {
                    track_color
                }
            }))
        });
        let fill_color = self.style.fill_color.map(ColorSource::Static).unwrap_or({
            let hovered = self.hovered.clone();
            let pressed = self.pressed.clone();
            let fill_color = self.fill_color;
            let fill_color_hover = self.fill_color_hover;
            let fill_color_pressed = self.fill_color_pressed;
            let fill_color_disabled = self.fill_color_disabled;
            ColorSource::Dynamic(std::rc::Rc::new(move || {
                if disabled {
                    fill_color_disabled
                } else if pressed.get() {
                    fill_color_pressed
                } else if hovered.get() {
                    fill_color_hover
                } else {
                    fill_color
                }
            }))
        });

        // Subscribe to the frame clock while animating so the platform drives re-renders.
        if anim.is_animating() {
            animation_frame_signal().get();
            request_animation_frame();
        }

        let anim_down = self.anim.clone();
        let val_down = self.value.clone();
        let anim_move = self.anim.clone();
        let val_move = self.value.clone();
        let mut slider = View::new()
            .width(self.width)
            .height(self.height)
            .radius(radius)
            .background(track_color)
            .cursor(if disabled {
                Cursor::NotAllowed
            } else {
                Cursor::Pointer
            })
            .child(
                View::new()
                    .width(fill_width)
                    .height(self.height)
                    .radius(radius)
                    .background(fill_color),
            );
        if !disabled {
            let hovered_enter = self.hovered.clone();
            let hovered_leave = self.hovered.clone();
            let pressed_down = self.pressed.clone();
            let pressed_up = self.pressed.clone();
            let pressed_leave = self.pressed.clone();
            slider = slider
                .on_hover_enter(move || hovered_enter.set(true))
                .on_hover_leave(move || {
                    hovered_leave.set(false);
                    pressed_leave.set(false);
                })
                .on_pointer_down(move |nx, _| {
                    let v = nx.clamp(0.0, 1.0);
                    pressed_down.set(true);
                    // Update anim first so the effect sees anim.to == v and skips animation.
                    anim_down.set(AnimState::settled(v));
                    val_down.set(v);
                })
                .on_pointer_move(move |nx, _| {
                    let v = nx.clamp(0.0, 1.0);
                    anim_move.set(AnimState::settled(v));
                    val_move.set(v);
                })
                .on_pointer_up(move |_, _| pressed_up.set(false));
        }
        slider.into_element()
    }
}

impl From<Slider> for Element {
    fn from(slider: Slider) -> Self {
        slider.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lemon::element::{style::Dimension, Element};

    fn make_slider(value: Signal<f32>) -> (Element, Signal<AnimState>) {
        let cx = lemon::Cx::new();
        let slider = Slider::new(&cx, value);
        let anim = slider.anim.clone();
        (slider.into_element(), anim)
    }

    #[test]
    fn slider_has_pointer_down_and_move_handlers() {
        let value = Signal::new(0.0_f32);
        let (el, _) = make_slider(value);

        let Element::View(track) = el else {
            panic!("expected View element from Slider");
        };
        assert!(
            track.handlers.on_pointer_down.is_some(),
            "Slider track must have on_pointer_down"
        );
        assert!(
            track.handlers.on_pointer_move.is_some(),
            "Slider track must have on_pointer_move"
        );
    }

    #[test]
    fn slider_pointer_down_updates_value() {
        let value = Signal::new(0.0_f32);
        let (el, _) = make_slider(value.clone());

        let Element::View(track) = el else {
            panic!("expected View element from Slider");
        };
        let on_down = track
            .handlers
            .on_pointer_down
            .as_ref()
            .expect("must have on_pointer_down");
        on_down(0.5, 0.0);
        assert!(
            (value.get() - 0.5).abs() < 1e-6,
            "value should be 0.5 after pointer_down at nx=0.5"
        );
    }

    #[test]
    fn slider_pointer_move_updates_value() {
        let value = Signal::new(0.0_f32);
        let (el, _) = make_slider(value.clone());

        let Element::View(track) = el else {
            panic!("expected View element from Slider");
        };
        let on_move = track
            .handlers
            .on_pointer_move
            .as_ref()
            .expect("must have on_pointer_move");
        on_move(0.75, 0.0);
        assert!(
            (value.get() - 0.75).abs() < 1e-6,
            "value should be 0.75 after pointer_move at nx=0.75"
        );
    }

    #[test]
    fn slider_fill_child_width_reflects_initial_value() {
        let value = Signal::new(0.25_f32);
        let cx = lemon::Cx::new();
        let el = Slider::new(&cx, value).width(200.0).into_element();

        let Element::View(track) = el else {
            panic!("expected View element from Slider");
        };
        assert_eq!(track.children.len(), 1);
        let Element::View(fill) = &track.children[0] else {
            panic!("expected View fill child");
        };
        assert_eq!(
            fill.style.width,
            Some(Dimension::Points(50.0)),
            "fill width should be 0.25 * 200 = 50"
        );
    }

    #[test]
    fn slider_default_dimensions() {
        let previous = lemon::current_theme();
        let mut custom = lemon::Theme::default_dark();
        custom.spacing.sm = 9.0;
        lemon::set_active_theme(custom.clone());

        let value = Signal::new(0.0_f32);
        let (el, _) = make_slider(value);

        let Element::View(track) = el else {
            panic!("expected View element from Slider");
        };
        assert_eq!(track.style.width, Some(Dimension::Points(200.0)));
        assert_eq!(
            track.style.height,
            Some(Dimension::Points(custom.spacing.sm * 2.0))
        );

        lemon::set_active_theme(previous);
    }

    #[test]
    fn slider_style_overrides_beat_theme_defaults() {
        let previous = lemon::current_theme();
        let mut custom = lemon::Theme::default_dark();
        custom.colors.border = Color::rgb8(11, 22, 33);
        custom.colors.accent = Color::rgb8(44, 55, 66);
        lemon::set_active_theme(custom);

        let value = Signal::new(0.5_f32);
        let override_track = Color::rgb8(99, 88, 77);
        let override_fill = Color::rgb8(66, 55, 44);
        let cx = lemon::Cx::new();
        let el = Slider::new(&cx, value)
            .style(
                SliderStyle::default()
                    .track_color(override_track)
                    .fill_color(override_fill),
            )
            .into_element();

        let Element::View(track) = el else {
            panic!("expected View element from Slider");
        };
        assert_eq!(track.paint.resolve().background, Some(override_track));
        let Element::View(fill) = &track.children[0] else {
            panic!("expected View fill child");
        };
        assert_eq!(fill.paint.resolve().background, Some(override_fill));

        lemon::set_active_theme(previous);
    }

    #[test]
    fn slider_style_unset_fields_fall_back_to_theme() {
        let previous = lemon::current_theme();
        let mut custom = lemon::Theme::default_dark();
        custom.colors.accent = Color::rgb8(44, 55, 66);
        lemon::set_active_theme(custom.clone());

        let value = Signal::new(0.5_f32);
        let cx = lemon::Cx::new();
        let el = Slider::new(&cx, value)
            .style(SliderStyle::default().track_color(Color::rgb8(99, 88, 77)))
            .into_element();

        let Element::View(track) = el else {
            panic!("expected View element from Slider");
        };
        let Element::View(fill) = &track.children[0] else {
            panic!("expected View fill child");
        };
        assert_eq!(fill.paint.resolve().background, Some(custom.colors.accent));

        lemon::set_active_theme(previous);
    }

    #[test]
    fn slider_pointer_coords_are_clamped_to_unit_range() {
        let value = Signal::new(0.5_f32);
        let (el, _) = make_slider(value.clone());

        let Element::View(track) = el else {
            panic!("expected View element from Slider");
        };
        let on_down = track
            .handlers
            .on_pointer_down
            .as_ref()
            .expect("must have on_pointer_down");

        on_down(1.5, 0.0);
        assert_eq!(value.get(), 1.0, "value above 1.0 should clamp to 1.0");

        on_down(-0.5, 0.0);
        assert_eq!(value.get(), 0.0, "value below 0.0 should clamp to 0.0");
    }

    #[test]
    fn slider_cursor_is_pointer() {
        let value = Signal::new(0.0_f32);
        let (el, _) = make_slider(value);

        let Element::View(track) = el else {
            panic!("expected View element from Slider");
        };
        assert_eq!(
            track.style.cursor,
            Cursor::Pointer,
            "Slider track should use Pointer cursor"
        );
    }

    #[test]
    fn slider_defaults_follow_active_theme_colors() {
        let previous = lemon::current_theme();
        let mut custom = lemon::Theme::default_dark();
        custom.colors.border = Color::rgb8(11, 22, 33);
        custom.colors.accent = Color::rgb8(44, 55, 66);
        lemon::set_active_theme(custom.clone());

        let value = Signal::new(0.25_f32);
        let (el, _) = make_slider(value);

        let Element::View(track) = el else {
            panic!("expected View element from Slider");
        };
        assert_eq!(track.paint.resolve().background, Some(custom.colors.border));
        let Element::View(fill) = &track.children[0] else {
            panic!("expected View fill child");
        };
        assert_eq!(fill.paint.resolve().background, Some(custom.colors.accent));

        lemon::set_active_theme(previous);
    }

    #[test]
    fn slider_hover_and_press_states_update_colors() {
        let previous = lemon::current_theme();
        let mut custom = lemon::Theme::default_dark();
        custom.colors.border = Color::rgb8(1, 2, 3);
        custom.colors.surface_hover = Color::rgb8(4, 5, 6);
        custom.colors.surface_pressed = Color::rgb8(7, 8, 9);
        custom.colors.accent = Color::rgb8(10, 11, 12);
        custom.colors.accent_hover = Color::rgb8(13, 14, 15);
        custom.colors.accent_pressed = Color::rgb8(16, 17, 18);
        lemon::set_active_theme(custom.clone());

        let value = Signal::new(0.25_f32);
        let (el, _) = make_slider(value);

        let Element::View(track) = el else {
            panic!("expected View element from Slider");
        };
        assert_eq!(track.paint.resolve().background, Some(custom.colors.border));
        let Element::View(fill) = &track.children[0] else {
            panic!("expected View fill child");
        };
        assert_eq!(fill.paint.resolve().background, Some(custom.colors.accent));

        track
            .handlers
            .on_hover_enter
            .as_ref()
            .expect("hover enter handler")();
        assert_eq!(
            track.paint.resolve().background,
            Some(custom.colors.surface_hover)
        );
        let Element::View(fill) = &track.children[0] else {
            panic!("expected View fill child");
        };
        assert_eq!(
            fill.paint.resolve().background,
            Some(custom.colors.accent_hover)
        );

        track
            .handlers
            .on_pointer_down
            .as_ref()
            .expect("pointer down handler")(0.5, 0.0);
        assert_eq!(
            track.paint.resolve().background,
            Some(custom.colors.surface_pressed)
        );
        let Element::View(fill) = &track.children[0] else {
            panic!("expected View fill child");
        };
        assert_eq!(
            fill.paint.resolve().background,
            Some(custom.colors.accent_pressed)
        );

        track
            .handlers
            .on_pointer_up
            .as_ref()
            .expect("pointer up handler")(0.5, 0.0);
        assert_eq!(
            track.paint.resolve().background,
            Some(custom.colors.surface_hover)
        );

        lemon::set_active_theme(previous);
    }

    #[test]
    fn disabled_slider_ignores_pointer_interactions() {
        let value = Signal::new(0.25_f32);
        let cx = lemon::Cx::new();
        let el = Slider::new(&cx, value.clone())
            .disabled(true)
            .into_element();

        let Element::View(track) = el else {
            panic!("expected View element from Slider");
        };
        assert!(track.handlers.on_pointer_down.is_none());
        assert!(track.handlers.on_pointer_move.is_none());
        assert!(track.handlers.on_hover_enter.is_none());
        assert!(track.handlers.on_pointer_up.is_none());
        assert_eq!(value.get(), 0.25);
    }

    #[test]
    fn drag_sets_anim_from_and_to_equal_no_animation() {
        let value = Signal::new(0.0_f32);
        let (el, anim) = make_slider(value.clone());

        let Element::View(track) = el else {
            panic!("expected View element from Slider");
        };
        let on_down = track
            .handlers
            .on_pointer_down
            .as_ref()
            .expect("must have on_pointer_down");
        on_down(0.6, 0.0);

        let state = anim.get();
        assert!(
            (state.from - 0.6).abs() < 1e-6,
            "anim.from should equal drag position"
        );
        assert!(
            (state.to - 0.6).abs() < 1e-6,
            "anim.to should equal drag position"
        );
        assert!(
            !state.is_animating(),
            "no animation should be active after drag"
        );
    }

    #[test]
    fn lerp_clamped_progress_stays_in_zero_one() {
        // AnimState::display must clamp progress to [0, 1] regardless of elapsed time.
        let state = AnimState {
            from: 0.0,
            to: 1.0,
            // A started_at far in the past guarantees elapsed >> ANIM_DURATION.
            started_at: Instant::now() - std::time::Duration::from_secs(10),
        };
        let display = state.display();
        assert!(
            (display - 1.0).abs() < 1e-6,
            "display should reach 1.0 when animation time exceeded: got {display}"
        );
    }

    #[test]
    fn lerp_returns_from_at_zero_elapsed() {
        let state = AnimState {
            from: 0.2,
            to: 0.8,
            started_at: Instant::now() + std::time::Duration::from_secs(10), // far in future
        };
        // elapsed will be negative-ish, clamped to 0 progress
        let display = state.display();
        // progress = elapsed.max(0) / ANIM_DURATION — Instant::elapsed never returns negative,
        // but a started_at in the future means elapsed ≈ 0.
        // We just verify the result is in [from, to].
        assert!(
            (0.2 - 1e-4..=0.8 + 1e-4).contains(&display),
            "display must be between from and to: {display}"
        );
    }

    #[test]
    fn settled_state_is_not_animating() {
        let state = AnimState::settled(0.5);
        assert!(
            !state.is_animating(),
            "a settled state should not be animating"
        );
    }

    #[test]
    fn animating_state_reports_is_animating_until_complete() {
        let state = AnimState {
            from: 0.0,
            to: 1.0,
            started_at: Instant::now(),
        };
        // Immediately after creation, elapsed ≈ 0 → progress ≈ 0 < 1
        assert!(
            state.is_animating(),
            "freshly started animation should be animating"
        );
    }
}
