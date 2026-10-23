use crate::element::types::CaretAffinity;
use crate::host::HostContext;
use crate::{KeyEvent, KeyState, LemonKey, NamedKey};

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct FieldEditor {
    /// Current field text.
    pub value: String,
    /// Cursor position as a byte offset in [`Self::value`].
    pub cursor: usize,
    pub affinity: CaretAffinity,
    /// When set, the range from this anchor to [`Self::cursor`] is selected.
    selection_anchor: Option<usize>,
}

impl FieldEditor {
    pub fn new(initial: impl Into<String>) -> Self {
        Self {
            value: initial.into(),
            cursor: 0,
            affinity: Default::default(),
            selection_anchor: None,
        }
    }

    /// Accepts app-owned changes, clearing stale selection and keeping a valid caret.
    pub fn sync_value(&mut self, value: String) {
        if self.value != value {
            self.value = value;
            self.clear_selection();
            self.affinity = CaretAffinity::Downstream;
            self.clamp_cursor_to_boundary();
        }
    }

    /// Applies a keyboard event to the text state.
    ///
    /// Supports character insertion, Backspace/Delete, ArrowLeft/ArrowRight,
    /// Home, End, Space, and Ctrl/Cmd+A/C/V shortcuts. Released key events are ignored.
    pub fn handle_key(&mut self, event: &KeyEvent, multiline: bool) {
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
                if chars.eq_ignore_ascii_case("x") {
                    if self.selection_range().is_some_and(|(a, b)| a != b) {
                        self.copy_selection();
                        self.delete_selection();
                    }
                    return;
                }
                if chars.eq_ignore_ascii_case("v") {
                    self.paste_from_clipboard(multiline);
                    return;
                }
            }
        }

        match &event.key {
            LemonKey::Character(chars)
                if !event.modifiers.ctrl && !event.modifiers.alt && !event.modifiers.meta =>
            {
                self.insert_text(chars, multiline);
            }
            LemonKey::Named(NamedKey::Backspace) => {
                if self.selection_range().is_some_and(|(a, b)| a != b) {
                    self.delete_selection();
                } else if self.cursor > 0 {
                    let previous = self.prev_char_boundary(self.cursor);
                    self.value.drain(previous..self.cursor);
                    self.cursor = previous;
                }
            }
            LemonKey::Named(NamedKey::Delete) => {
                if self.selection_range().is_some_and(|(a, b)| a != b) {
                    self.delete_selection();
                } else if self.cursor < self.value.len() {
                    let next = self.next_char_boundary(self.cursor);
                    self.value.drain(self.cursor..next);
                }
            }
            LemonKey::Named(NamedKey::ArrowLeft) => {
                let target = if event.modifiers.shift {
                    self.prev_char_boundary(self.cursor)
                } else {
                    self.selection_range()
                        .map_or_else(|| self.prev_char_boundary(self.cursor), |(start, _)| start)
                };
                self.move_cursor(target, event.modifiers.shift);
            }
            LemonKey::Named(NamedKey::ArrowRight) => {
                let target = if event.modifiers.shift {
                    self.next_char_boundary(self.cursor)
                } else {
                    self.selection_range()
                        .map_or_else(|| self.next_char_boundary(self.cursor), |(_, end)| end)
                };
                self.move_cursor(target, event.modifiers.shift);
            }
            LemonKey::Named(NamedKey::Enter) if multiline => self.insert_text("\n", true),
            LemonKey::Named(NamedKey::ArrowUp | NamedKey::ArrowDown) if multiline => {
                let start = self.value[..self.cursor].rfind('\n').map_or(0, |i| i + 1);
                let column = self.value[start..self.cursor].chars().count();
                let target = if event.key == LemonKey::Named(NamedKey::ArrowUp) {
                    if start == 0 {
                        0
                    } else {
                        let previous = self.value[..start - 1].rfind('\n').map_or(0, |i| i + 1);
                        previous
                            + self.value[previous..start - 1]
                                .char_indices()
                                .nth(column)
                                .map_or(start - 1 - previous, |(i, _)| i)
                    }
                } else {
                    self.value[self.cursor..]
                        .find('\n')
                        .map_or(self.value.len(), |i| {
                            let next = self.cursor + i + 1;
                            let end = self.value[next..]
                                .find('\n')
                                .map_or(self.value.len(), |i| next + i);
                            next + self.value[next..end]
                                .char_indices()
                                .nth(column)
                                .map_or(end - next, |(i, _)| i)
                        })
                };
                self.move_cursor(target, event.modifiers.shift);
            }
            LemonKey::Named(NamedKey::Home) => {
                let target = if multiline && !shortcut {
                    self.value[..self.cursor].rfind('\n').map_or(0, |i| i + 1)
                } else {
                    0
                };
                self.move_cursor(target, event.modifiers.shift);
            }
            LemonKey::Named(NamedKey::End) => {
                let target = if multiline && !shortcut {
                    self.value[self.cursor..]
                        .find('\n')
                        .map_or(self.value.len(), |i| self.cursor + i)
                } else {
                    self.value.len()
                };
                self.move_cursor(target, event.modifiers.shift);
            }
            LemonKey::Named(NamedKey::Space) => self.insert_text(" ", multiline),
            _ => {}
        }
    }

    pub fn move_cursor(&mut self, cursor: usize, extend: bool) {
        if extend {
            self.selection_anchor.get_or_insert(self.cursor);
        } else {
            self.clear_selection();
        }
        self.cursor = cursor;
        self.affinity = CaretAffinity::Downstream;
        self.clamp_cursor_to_boundary();
        if self.selection_anchor == Some(self.cursor) {
            self.clear_selection();
        }
    }

    fn insert_text(&mut self, text: &str, multiline: bool) {
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        let text = if multiline {
            text
        } else {
            text.replace('\n', " ")
        };
        self.delete_selection();
        self.value.insert_str(self.cursor, &text);
        self.cursor += text.len();
        self.affinity = CaretAffinity::Upstream;
    }

    fn select_all(&mut self) {
        self.selection_anchor = Some(0);
        self.cursor = self.value.len();
        self.affinity = CaretAffinity::Upstream;
    }

    fn clear_selection(&mut self) {
        self.selection_anchor = None;
    }

    pub fn selection_range(&self) -> Option<(usize, usize)> {
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
            self.affinity = CaretAffinity::Downstream;
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

        HostContext::current().set_clipboard_text(text);
    }

    fn paste_from_clipboard(&mut self, multiline: bool) {
        let Some(text) = HostContext::current().clipboard_text() else {
            return;
        };
        if text.is_empty() {
            return;
        }

        self.insert_text(&text, multiline);
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
        let mut state = FieldEditor::new("");

        state.handle_key(&key(LemonKey::Character("h".into())), false);
        state.handle_key(&key(LemonKey::Character("i".into())), false);

        assert_eq!(state.value, "hi");
        assert_eq!(state.cursor, 2);
    }

    #[test]
    fn backspace_removes_previous_character_and_moves_left() {
        let mut state = FieldEditor {
            value: "abc".into(),
            cursor: 2,
            affinity: Default::default(),
            selection_anchor: None,
        };

        state.handle_key(&key(LemonKey::Named(NamedKey::Backspace)), false);

        assert_eq!(state.value, "ac");
        assert_eq!(state.cursor, 1);
    }

    #[test]
    fn delete_removes_next_character_without_moving_cursor() {
        let mut state = FieldEditor {
            value: "abc".into(),
            cursor: 1,
            affinity: Default::default(),
            selection_anchor: None,
        };

        state.handle_key(&key(LemonKey::Named(NamedKey::Delete)), false);

        assert_eq!(state.value, "ac");
        assert_eq!(state.cursor, 1);
    }

    #[test]
    fn backspace_and_delete_at_edges_do_nothing() {
        let mut start = FieldEditor {
            value: "abc".into(),
            cursor: 0,
            affinity: Default::default(),
            selection_anchor: None,
        };
        start.handle_key(&key(LemonKey::Named(NamedKey::Backspace)), false);
        assert_eq!(start.value, "abc");
        assert_eq!(start.cursor, 0);

        let mut end = FieldEditor {
            value: "abc".into(),
            cursor: 3,
            affinity: Default::default(),
            selection_anchor: None,
        };
        end.handle_key(&key(LemonKey::Named(NamedKey::Delete)), false);
        assert_eq!(end.value, "abc");
        assert_eq!(end.cursor, 3);
    }

    #[test]
    fn arrow_home_end_navigation_moves_cursor() {
        let mut state = FieldEditor {
            value: "abc".into(),
            cursor: 1,
            affinity: Default::default(),
            selection_anchor: None,
        };

        state.handle_key(&key(LemonKey::Named(NamedKey::ArrowRight)), false);
        assert_eq!(state.cursor, 2);

        state.handle_key(&key(LemonKey::Named(NamedKey::ArrowLeft)), false);
        assert_eq!(state.cursor, 1);

        state.handle_key(&key(LemonKey::Named(NamedKey::Home)), false);
        assert_eq!(state.cursor, 0);

        state.handle_key(&key(LemonKey::Named(NamedKey::End)), false);
        assert_eq!(state.cursor, 3);
    }

    #[test]
    fn key_released_events_are_ignored() {
        let mut state = FieldEditor::new("");
        state.handle_key(
            &KeyEvent {
                key: LemonKey::Character("x".into()),
                modifiers: Modifiers::default(),
                repeat: false,
                state: KeyState::Released,
            },
            false,
        );

        assert_eq!(state.value, "");
        assert_eq!(state.cursor, 0);
    }

    #[test]
    fn unicode_insertion_updates_cursor_as_byte_offset() {
        let mut state = FieldEditor::new("");

        state.handle_key(&key(LemonKey::Character("€".into())), false);

        assert_eq!(state.value, "€");
        assert_eq!(state.cursor, 3);
    }

    #[test]
    fn unicode_navigation_and_deletion_follow_char_boundaries() {
        let mut state = FieldEditor {
            value: "a€b".into(),
            cursor: "a€b".len(),
            affinity: Default::default(),
            selection_anchor: None,
        };

        state.handle_key(&key(LemonKey::Named(NamedKey::ArrowLeft)), false);
        assert_eq!(state.cursor, "a€".len());

        state.handle_key(&key(LemonKey::Named(NamedKey::ArrowLeft)), false);
        assert_eq!(state.cursor, "a".len());

        state.handle_key(&key(LemonKey::Named(NamedKey::Delete)), false);
        assert_eq!(state.value, "ab");
        assert_eq!(state.cursor, "a".len());

        state.handle_key(&key(LemonKey::Named(NamedKey::Backspace)), false);
        assert_eq!(state.value, "b");
        assert_eq!(state.cursor, 0);
    }

    #[test]
    fn space_named_key_inserts_space_character() {
        let mut state = FieldEditor {
            value: "hi".into(),
            cursor: 2,
            affinity: Default::default(),
            selection_anchor: None,
        };

        state.handle_key(&key(LemonKey::Named(NamedKey::Space)), false);

        assert_eq!(state.value, "hi ");
        assert_eq!(state.cursor, 3);
    }

    #[test]
    fn modified_character_shortcuts_are_not_inserted() {
        let mut state = FieldEditor::new("");
        state.handle_key(
            &KeyEvent {
                key: LemonKey::Character("x".into()),
                modifiers: Modifiers {
                    ctrl: true,
                    ..Default::default()
                },
                repeat: false,
                state: KeyState::Pressed,
            },
            false,
        );
        state.handle_key(
            &KeyEvent {
                key: LemonKey::Character("x".into()),
                modifiers: Modifiers {
                    alt: true,
                    ..Default::default()
                },
                repeat: false,
                state: KeyState::Pressed,
            },
            false,
        );
        state.handle_key(
            &KeyEvent {
                key: LemonKey::Character("x".into()),
                modifiers: Modifiers {
                    meta: true,
                    ..Default::default()
                },
                repeat: false,
                state: KeyState::Pressed,
            },
            false,
        );

        assert_eq!(state.value, "");
        assert_eq!(state.cursor, 0);
    }

    #[test]
    fn ctrl_a_selects_all_text() {
        let mut state = FieldEditor {
            value: "hello".into(),
            cursor: 2,
            affinity: Default::default(),
            selection_anchor: None,
        };

        state.handle_key(
            &key_with_modifiers(
                LemonKey::Character("a".into()),
                Modifiers {
                    ctrl: true,
                    ..Default::default()
                },
            ),
            false,
        );

        assert_eq!(state.cursor, 5);
        assert_eq!(state.selection_anchor, Some(0));
    }

    #[test]
    fn backspace_with_selection_deletes_range() {
        let mut state = FieldEditor {
            value: "hello".into(),
            cursor: 5,
            affinity: Default::default(),
            selection_anchor: Some(0),
        };

        state.handle_key(&key(LemonKey::Named(NamedKey::Backspace)), false);

        assert_eq!(state.value, "");
        assert_eq!(state.cursor, 0);
        assert_eq!(state.selection_anchor, None);
    }
}
