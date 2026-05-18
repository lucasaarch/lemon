use lemon::{KeyEvent, KeyState, LemonKey, NamedKey};

/// Plain editing state for a text field.
///
/// The cursor is stored as a UTF-8 byte offset into [`Self::value`], and all
/// movement/edit operations keep it on character boundaries.
///
/// # Examples
///
/// ```
/// use lemon::{KeyEvent, KeyState, LemonKey, Modifiers};
/// use lemon_widgets::TextFieldState;
///
/// let mut state = TextFieldState::new("");
/// state.handle_key(&KeyEvent {
///     key: LemonKey::Character("h".into()),
///     modifiers: Modifiers::default(),
///     repeat: false,
///     state: KeyState::Pressed,
/// });
///
/// assert_eq!(state.value, "h");
/// assert_eq!(state.cursor, 1);
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextFieldState {
    /// Current field text.
    pub value: String,
    /// Cursor position as a byte offset in [`Self::value`].
    pub cursor: usize,
}

impl TextFieldState {
    /// Creates a text field editing state with an initial value.
    ///
    /// The cursor starts at byte index `0`.
    pub fn new(initial: impl Into<String>) -> Self {
        Self {
            value: initial.into(),
            cursor: 0,
        }
    }

    /// Applies a keyboard event to the text state.
    ///
    /// Supports character insertion, Backspace/Delete, ArrowLeft/ArrowRight,
    /// Home, and End. Released key events are ignored.
    pub fn handle_key(&mut self, event: &KeyEvent) {
        if event.state == KeyState::Released {
            return;
        }

        self.clamp_cursor_to_boundary();

        match &event.key {
            LemonKey::Character(chars) if !event.modifiers.ctrl && !event.modifiers.meta => {
                for ch in chars.chars() {
                    self.value.insert(self.cursor, ch);
                    self.cursor += ch.len_utf8();
                }
            }
            LemonKey::Named(NamedKey::Backspace) => {
                if self.cursor > 0 {
                    let prev = self.prev_char_boundary(self.cursor);
                    self.value.drain(prev..self.cursor);
                    self.cursor = prev;
                }
            }
            LemonKey::Named(NamedKey::Delete) => {
                if self.cursor < self.value.len() {
                    let next = self.next_char_boundary(self.cursor);
                    self.value.drain(self.cursor..next);
                }
            }
            LemonKey::Named(NamedKey::ArrowLeft) => {
                if self.cursor > 0 {
                    self.cursor = self.prev_char_boundary(self.cursor);
                }
            }
            LemonKey::Named(NamedKey::ArrowRight) => {
                if self.cursor < self.value.len() {
                    self.cursor = self.next_char_boundary(self.cursor);
                }
            }
            LemonKey::Named(NamedKey::Home) => self.cursor = 0,
            LemonKey::Named(NamedKey::End) => self.cursor = self.value.len(),
            _ => {}
        }
    }

    fn clamp_cursor_to_boundary(&mut self) {
        self.cursor = self.cursor.min(self.value.len());
        while self.cursor > 0 && !self.value.is_char_boundary(self.cursor) {
            self.cursor -= 1;
        }
    }

    fn prev_char_boundary(&self, pos: usize) -> usize {
        if pos == 0 {
            return 0;
        }

        let mut cursor = (pos - 1).min(self.value.len());
        while cursor > 0 && !self.value.is_char_boundary(cursor) {
            cursor -= 1;
        }
        cursor
    }

    fn next_char_boundary(&self, pos: usize) -> usize {
        if pos >= self.value.len() {
            return self.value.len();
        }

        let mut cursor = pos + 1;
        while cursor < self.value.len() && !self.value.is_char_boundary(cursor) {
            cursor += 1;
        }
        cursor
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lemon::Modifiers;

    fn key(key: LemonKey) -> KeyEvent {
        KeyEvent {
            key,
            modifiers: Modifiers::default(),
            repeat: false,
            state: KeyState::Pressed,
        }
    }

    #[test]
    fn typing_ascii_inserts_and_advances_cursor() {
        let mut state = TextFieldState::new("");

        state.handle_key(&key(LemonKey::Character("h".into())));
        state.handle_key(&key(LemonKey::Character("i".into())));

        assert_eq!(state.value, "hi");
        assert_eq!(state.cursor, 2);
    }

    #[test]
    fn backspace_removes_previous_character_and_moves_left() {
        let mut state = TextFieldState {
            value: "abc".into(),
            cursor: 2,
        };

        state.handle_key(&key(LemonKey::Named(NamedKey::Backspace)));

        assert_eq!(state.value, "ac");
        assert_eq!(state.cursor, 1);
    }

    #[test]
    fn delete_removes_next_character_without_moving_cursor() {
        let mut state = TextFieldState {
            value: "abc".into(),
            cursor: 1,
        };

        state.handle_key(&key(LemonKey::Named(NamedKey::Delete)));

        assert_eq!(state.value, "ac");
        assert_eq!(state.cursor, 1);
    }

    #[test]
    fn backspace_and_delete_at_edges_do_nothing() {
        let mut start = TextFieldState {
            value: "abc".into(),
            cursor: 0,
        };
        start.handle_key(&key(LemonKey::Named(NamedKey::Backspace)));
        assert_eq!(start.value, "abc");
        assert_eq!(start.cursor, 0);

        let mut end = TextFieldState {
            value: "abc".into(),
            cursor: 3,
        };
        end.handle_key(&key(LemonKey::Named(NamedKey::Delete)));
        assert_eq!(end.value, "abc");
        assert_eq!(end.cursor, 3);
    }

    #[test]
    fn arrow_home_end_navigation_moves_cursor() {
        let mut state = TextFieldState {
            value: "abc".into(),
            cursor: 1,
        };

        state.handle_key(&key(LemonKey::Named(NamedKey::ArrowRight)));
        assert_eq!(state.cursor, 2);

        state.handle_key(&key(LemonKey::Named(NamedKey::ArrowLeft)));
        assert_eq!(state.cursor, 1);

        state.handle_key(&key(LemonKey::Named(NamedKey::Home)));
        assert_eq!(state.cursor, 0);

        state.handle_key(&key(LemonKey::Named(NamedKey::End)));
        assert_eq!(state.cursor, 3);
    }

    #[test]
    fn key_released_events_are_ignored() {
        let mut state = TextFieldState::new("");
        state.handle_key(&KeyEvent {
            key: LemonKey::Character("x".into()),
            modifiers: Modifiers::default(),
            repeat: false,
            state: KeyState::Released,
        });

        assert_eq!(state.value, "");
        assert_eq!(state.cursor, 0);
    }

    #[test]
    fn unicode_insertion_updates_cursor_as_byte_offset() {
        let mut state = TextFieldState::new("");

        state.handle_key(&key(LemonKey::Character("€".into())));

        assert_eq!(state.value, "€");
        assert_eq!(state.cursor, 3);
    }

    #[test]
    fn unicode_navigation_and_deletion_follow_char_boundaries() {
        let mut state = TextFieldState {
            value: "a€b".into(),
            cursor: "a€b".len(),
        };

        state.handle_key(&key(LemonKey::Named(NamedKey::ArrowLeft)));
        assert_eq!(state.cursor, "a€".len());

        state.handle_key(&key(LemonKey::Named(NamedKey::ArrowLeft)));
        assert_eq!(state.cursor, "a".len());

        state.handle_key(&key(LemonKey::Named(NamedKey::Delete)));
        assert_eq!(state.value, "ab");
        assert_eq!(state.cursor, "a".len());

        state.handle_key(&key(LemonKey::Named(NamedKey::Backspace)));
        assert_eq!(state.value, "b");
        assert_eq!(state.cursor, 0);
    }
}
