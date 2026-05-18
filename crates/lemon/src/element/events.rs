#[derive(Clone, Debug, PartialEq)]
pub enum LemonKey {
    Character(String),
    Named(NamedKey),
    Other,
}

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

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum KeyState {
    Pressed,
    Released,
}

#[derive(Clone, Debug)]
pub struct KeyEvent {
    pub key: LemonKey,
    pub modifiers: Modifiers,
    pub repeat: bool,
    pub state: KeyState,
}

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
