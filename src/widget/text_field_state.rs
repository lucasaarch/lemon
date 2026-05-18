use crate::{KeyEvent, KeyState, LemonKey, NamedKey};
use arboard::Clipboard;

/// Plain editing state for a text field.
///
/// The cursor is stored as a UTF-8 byte offset into [`Self::value`], and all
/// movement/edit operations keep it on character boundaries.
///
/// # Examples
///
/// ```
/// use lemon::{KeyEvent, KeyState, LemonKey, Modifiers};
/// use lemon::widget::TextFieldState;
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
    /// When set, the range from this anchor to [`Self::cursor`] is selected.
    selection_anchor: Option<usize>,
}

impl TextFieldState {
    /// Creates a text field editing state with an initial value.
    ///
    /// The cursor starts at byte index `0`.
    pub fn new(initial: impl Into<String>) -> Self {
        Self {
            value: initial.into(),
            cursor: 0,
            selection_anchor: None,
        }
    }

    /// Clears the value and resets the cursor and selection.
    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
        self.selection_anchor = None;
    }

    /// Applies a keyboard event to the text state.
    ///
    /// Supports character insertion, Backspace/Delete, ArrowLeft/ArrowRight,
    /// Home, End, Space, and Ctrl/Cmd+A/C/V shortcuts. Released key events are ignored.
    pub fn handle_key(&mut self, event: &KeyEvent) {
        if event.state == KeyState::Released {
            return;
        }

        self.clamp_cursor_to_boundary();

        let shortcut = event.modifiers.ctrl || event.modifiers.meta;

        if shortcut {
            if let LemonKey::Character(chars) = &event.key {
                if chars.eq_ignore_ascii_case("a") {
                    self.select_all();
                    return;
                }
                if chars.eq_ignore_ascii_case("c") {
                    self.copy_selection();
                    return;
                }
                if chars.eq_ignore_ascii_case("v") {
                    self.paste_from_clipboard();
                    return;
                }
            }
        }

        match &event.key {
            LemonKey::Character(chars)
                if !event.modifiers.ctrl && !event.modifiers.alt && !event.modifiers.meta =>
            {
                self.clear_selection();
                for ch in chars.chars() {
                    self.value.insert(self.cursor, ch);
                    self.cursor += ch.len_utf8();
                }
            }
            LemonKey::Named(NamedKey::Backspace) if self.cursor > 0 => {
                if self.selection_anchor.is_some() {
                    self.delete_selection();
                } else {
                    let prev = self.prev_char_boundary(self.cursor);
                    self.value.drain(prev..self.cursor);
                    self.cursor = prev;
                }
            }
            LemonKey::Named(NamedKey::Delete) if self.cursor < self.value.len() => {
                if self.selection_anchor.is_some() {
                    self.delete_selection();
                } else {
                    let next = self.next_char_boundary(self.cursor);
                    self.value.drain(self.cursor..next);
                }
            }
            LemonKey::Named(NamedKey::ArrowLeft) if self.cursor > 0 => {
                self.clear_selection();
                self.cursor = self.prev_char_boundary(self.cursor);
            }
            LemonKey::Named(NamedKey::ArrowRight) if self.cursor < self.value.len() => {
                self.clear_selection();
                self.cursor = self.next_char_boundary(self.cursor);
            }
            LemonKey::Named(NamedKey::Home) => {
                self.clear_selection();
                self.cursor = 0;
            }
            LemonKey::Named(NamedKey::End) => {
                self.clear_selection();
                self.cursor = self.value.len();
            }
            LemonKey::Named(NamedKey::Space) => {
                self.clear_selection();
                self.value.insert(self.cursor, ' ');
                self.cursor += 1;
            }
            _ => {}
        }
    }

    fn select_all(&mut self) {
        self.selection_anchor = Some(0);
        self.cursor = self.value.len();
    }

    fn clear_selection(&mut self) {
        self.selection_anchor = None;
    }

    fn selection_range(&self) -> Option<(usize, usize)> {
        self.selection_anchor.map(|anchor| {
            if anchor <= self.cursor {
                (anchor, self.cursor)
            } else {
                (self.cursor, anchor)
            }
        })
    }

    fn delete_selection(&mut self) {
        if let Some((start, end)) = self.selection_range() {
            self.value.drain(start..end);
            self.cursor = start;
        }
        self.clear_selection();
    }

    fn copy_selection(&mut self) {
        let text = if let Some((start, end)) = self.selection_range() {
            self.value.get(start..end).unwrap_or("")
        } else {
            self.value.as_str()
        };

        if text.is_empty() {
            return;
        }

        if let Ok(mut clipboard) = Clipboard::new() {
            let _ = clipboard.set_text(text);
        }
    }

    fn paste_from_clipboard(&mut self) {
        let Ok(mut clipboard) = Clipboard::new() else {
            return;
        };
        let Ok(text) = clipboard.get_text() else {
            return;
        };
        if text.is_empty() {
            return;
        }

        self.delete_selection();
        for ch in text.chars() {
            self.value.insert(self.cursor, ch);
            self.cursor += ch.len_utf8();
        }
        self.clamp_cursor_to_boundary();
    }

    /// Clamps the cursor inside the current text and onto a UTF-8 char boundary.
    fn clamp_cursor_to_boundary(&mut self) {
        self.cursor = self.cursor.min(self.value.len());
        while self.cursor > 0 && !self.value.is_char_boundary(self.cursor) {
            self.cursor -= 1;
        }
    }

    /// Returns the previous UTF-8 character boundary at or before `pos`.
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

    /// Returns the next UTF-8 character boundary at or after `pos`.
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
    use crate::Modifiers;

    fn key(key: LemonKey) -> KeyEvent {
        KeyEvent {
            key,
            modifiers: Modifiers::default(),
            repeat: false,
            state: KeyState::Pressed,
        }
    }

    fn key_with_modifiers(key: LemonKey, modifiers: Modifiers) -> KeyEvent {
        KeyEvent {
            key,
            modifiers,
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
            selection_anchor: None,
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
            selection_anchor: None,
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
            selection_anchor: None,
        };
        start.handle_key(&key(LemonKey::Named(NamedKey::Backspace)));
        assert_eq!(start.value, "abc");
        assert_eq!(start.cursor, 0);

        let mut end = TextFieldState {
            value: "abc".into(),
            cursor: 3,
            selection_anchor: None,
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
            selection_anchor: None,
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
            selection_anchor: None,
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

    #[test]
    fn space_named_key_inserts_space_character() {
        let mut state = TextFieldState {
            value: "hi".into(),
            cursor: 2,
            selection_anchor: None,
        };

        state.handle_key(&key(LemonKey::Named(NamedKey::Space)));

        assert_eq!(state.value, "hi ");
        assert_eq!(state.cursor, 3);
    }

    #[test]
    fn modified_character_shortcuts_are_not_inserted() {
        let mut state = TextFieldState::new("");
        state.handle_key(&KeyEvent {
            key: LemonKey::Character("x".into()),
            modifiers: Modifiers {
                ctrl: true,
                ..Default::default()
            },
            repeat: false,
            state: KeyState::Pressed,
        });
        state.handle_key(&KeyEvent {
            key: LemonKey::Character("x".into()),
            modifiers: Modifiers {
                alt: true,
                ..Default::default()
            },
            repeat: false,
            state: KeyState::Pressed,
        });
        state.handle_key(&KeyEvent {
            key: LemonKey::Character("x".into()),
            modifiers: Modifiers {
                meta: true,
                ..Default::default()
            },
            repeat: false,
            state: KeyState::Pressed,
        });

        assert_eq!(state.value, "");
        assert_eq!(state.cursor, 0);
    }

    #[test]
    fn ctrl_a_selects_all_text() {
        let mut state = TextFieldState {
            value: "hello".into(),
            cursor: 2,
            selection_anchor: None,
        };

        state.handle_key(&key_with_modifiers(
            LemonKey::Character("a".into()),
            Modifiers {
                ctrl: true,
                ..Default::default()
            },
        ));

        assert_eq!(state.cursor, 5);
        assert_eq!(state.selection_anchor, Some(0));
    }

    #[test]
    fn backspace_with_selection_deletes_range() {
        let mut state = TextFieldState {
            value: "hello".into(),
            cursor: 5,
            selection_anchor: Some(0),
        };

        state.handle_key(&key(LemonKey::Named(NamedKey::Backspace)));

        assert_eq!(state.value, "");
        assert_eq!(state.cursor, 0);
        assert_eq!(state.selection_anchor, None);
    }
}
