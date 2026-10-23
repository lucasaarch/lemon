//! Keyboard and pointer types surfaced to widget event handlers.

/// Physical or logical key in a [`KeyEvent`].
#[derive(Clone, Debug, PartialEq)]
pub enum LemonKey {
    Character(String),
    Named(NamedKey),
    Other,
}

/// Non-printable keys with stable names (arrows, Enter, Tab, …).
#[derive(Clone, Debug, PartialEq)]
pub enum NamedKey {
    Tab,
    Enter,
    Escape,
    Space,
    Backspace,
    Delete,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    Home,
    End,
    PageUp,
    PageDown,
}

/// Modifier keys held during a keyboard event.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

/// Whether a key transitioned to down or up.
#[derive(Clone, Debug, PartialEq)]
pub enum KeyState {
    Pressed,
    Released,
}

/// Keyboard input delivered to [`ColumnView::on_key_down`](crate::element::builders::ColumnView::on_key_down)
/// / `on_key_up` on focusable nodes.
#[derive(Clone, Debug)]
pub struct KeyEvent {
    pub key: LemonKey,
    pub modifiers: Modifiers,
    pub repeat: bool,
    pub state: KeyState,
}

/// Mouse cursor shape for [`ColumnView::cursor`](crate::element::builders::ColumnView::cursor) and similar.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Cursor {
    #[default]
    Default,
    Pointer,
    Text,
    Grab,
    Grabbing,
    Wait,
    NotAllowed,
    Move,
    Crosshair,
    /// Cursor for a handle that adjusts height only.
    ResizeVertical,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_event_character_roundtrip() {
        let ev = KeyEvent {
            key: LemonKey::Character("a".into()),
            modifiers: Modifiers::default(),
            repeat: false,
            state: KeyState::Pressed,
        };
        assert_eq!(ev.key, LemonKey::Character("a".into()));
        assert!(!ev.repeat);
        assert_eq!(ev.state, KeyState::Pressed);
    }

    #[test]
    fn key_event_named_key_tab() {
        let ev = KeyEvent {
            key: LemonKey::Named(NamedKey::Tab),
            modifiers: Modifiers {
                shift: true,
                ..Default::default()
            },
            repeat: false,
            state: KeyState::Pressed,
        };
        assert_eq!(ev.key, LemonKey::Named(NamedKey::Tab));
        assert!(ev.modifiers.shift);
    }

    #[test]
    fn cursor_defaults_to_default_variant() {
        assert_eq!(Cursor::default(), Cursor::Default);
    }
}

/// Directions enabled in a [`crate::ScrollView`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScrollAxis {
    /// Scroll from top to bottom (the default).
    #[default]
    Vertical,
    /// Scroll from left to right using horizontal wheel/trackpad movement or Shift+wheel.
    Horizontal,
    /// Scroll independently in both directions, including diagonal trackpad gestures.
    Both,
}
impl ScrollAxis {
    pub(crate) fn horizontal(self) -> bool {
        matches!(self, Self::Horizontal | Self::Both)
    }
    pub(crate) fn vertical(self) -> bool {
        matches!(self, Self::Vertical | Self::Both)
    }
    pub(crate) fn accepts(self, delta: ScrollDelta) -> bool {
        (self.horizontal() && delta.x != 0.0) || (self.vertical() && delta.y != 0.0)
    }
}

/// Wheel movement in logical points, preserving both native axes.
///
/// Positive values move toward the start of each axis. The desktop host converts
/// line deltas to logical points and maps Shift+vertical wheel to horizontal motion.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScrollDelta {
    /// Horizontal movement.
    pub x: f64,
    /// Vertical movement.
    pub y: f64,
}
impl ScrollDelta {
    /// Creates a delta in logical points; use zero for an unchanged axis.
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Scroll position in logical points from the content origin.
///
/// Positive `x` moves the viewport right; positive `y` moves it down. Defaults to
/// the top-left. Used by [`crate::ScrollView::with_offset`] and measured maximum
/// offsets. Only enabled axes affect the content position.
///
/// ```
/// use lemon::prelude::*;
/// let offset = State::new(ScrollOffset::new(40.0, 80.0));
/// let view = ScrollView::with_offset(offset, View::new().width(800.0).height(600.0))
///     .axis(ScrollAxis::Both).width(240.0).height(160.0);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScrollOffset {
    /// Horizontal distance from the left edge, in logical points.
    pub x: f64,
    /// Vertical distance from the top edge, in logical points.
    pub y: f64,
}
impl ScrollOffset {
    /// Creates independent horizontal and vertical offsets, in logical points.
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    pub(crate) fn clamped(self, maximum: Self) -> Self {
        fn clamp(value: f64, maximum: f64) -> f64 {
            if value.is_finite() {
                value.max(0.0).min(maximum)
            } else {
                0.0
            }
        }
        Self::new(clamp(self.x, maximum.x), clamp(self.y, maximum.y))
    }
}
