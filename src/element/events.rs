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

/// The single direction in which a [`crate::ScrollView`] scrolls.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScrollAxis {
    /// Scroll from top to bottom (the default).
    #[default]
    Vertical,
    /// Scroll from left to right using horizontal wheel/trackpad movement or Shift+wheel.
    Horizontal,
}
impl ScrollAxis {
    pub(crate) fn amount(self, delta: ScrollDelta) -> f64 {
        match self {
            Self::Vertical => delta.y,
            Self::Horizontal => delta.x,
        }
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
