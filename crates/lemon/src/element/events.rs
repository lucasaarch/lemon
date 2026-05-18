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

/// Keyboard input delivered to [`Column::on_key_down`](crate::element::builders::Column::on_key_down)
/// / `on_key_up` on focusable nodes.
#[derive(Clone, Debug)]
pub struct KeyEvent {
    pub key: LemonKey,
    pub modifiers: Modifiers,
    pub repeat: bool,
    pub state: KeyState,
}

/// Mouse cursor shape for [`Column::cursor`](crate::element::builders::Column::cursor) and similar.
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
