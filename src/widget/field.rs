use crate::{
    element::{
        builders::{ColumnView, RowView, Text, View},
        events::Cursor,
        style::{Color, ColorSource, Distribute},
        types::{TextAlignment, TextInputMeta},
        Element,
    },
    icons::lucide,
    Cx, Decorate, Icon, LemonKey, Position, ScrollView, State,
};

mod editor;
use editor::FieldEditor;

/// Theme-aware visual preset shared by [`TextField`], [`SecureField`] and [`TextArea`].
///
/// Presets have the same typography and padding. Explicit widget overrides apply
/// after the preset; `.decorate(...)` can then customize the resulting root view.
///
/// ```
/// use lemon::prelude::*;
/// let cx = Cx::new();
/// let field = TextField::new(&cx, State::new(String::new())).style(TextFieldStyle::Clean);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextFieldStyle {
    /// Transparent background without a border or focus ring by default.
    Clean,
    /// Themed surface and border. This is the default.
    #[default]
    Bordered,
    /// Themed surface without a resting border.
    Filled,
}

#[derive(Default)]
struct FieldOverrides {
    placeholder_color: Option<Color>,
    border_color: Option<Color>,
    padding: Option<f32>,
    radius: Option<f32>,
    focus_ring_color: Option<Color>,
}

/// Single-line editable text field.
///
/// `TextField` is a **pure builder** — it reads reactive state at build time and registers
/// reactive dependencies so the parent re-renders when the field value changes.
/// Line breaks in pasted/inserted text become spaces; Enter does not insert text.
/// Use [`TextArea`] for multiline editing.
///
/// Keyboard focus is handled by the platform ([`FocusManager`](crate::retained::focus::FocusManager)):
/// click or Tab to focus, then type. Ctrl/Cmd+A selects the value; the selection
/// uses the theme accent color at 28% opacity while focused. Selection is painted
/// independently of caret blinking. The clean preset also shows this highlight.
///
/// # Examples
///
/// ```no_run
/// use lemon::{Color, Cx, element::Element};
/// use lemon::widget::{TextField, TextFieldStyle};
///
/// fn my_view(cx: &Cx) -> Element {
///     let state = cx.state("".to_owned());
///     TextField::new(cx, state)
///         .style(TextFieldStyle::Bordered)
///         .border_color(Color::rgb8(200, 80, 80))
///         .placeholder("Type here…")
///         .width(240.0)
///         .into_element()
/// }
/// ```
pub struct TextField {
    state: State<String>,
    editor: State<FieldEditor>,
    hovered: State<bool>,
    pressed: State<bool>,
    placeholder: String,
    width: Option<f32>,
    placeholder_color: Color,
    text_color: Color,
    surface_color: Color,
    surface_color_hover: Color,
    surface_color_pressed: Color,
    surface_color_disabled: Color,
    border_color: Color,
    text_color_disabled: Color,
    padding: f32,
    radius: f32,
    focus_ring_color: Color,
    style: TextFieldStyle,
    alignment: TextAlignment,
    overrides: FieldOverrides,
    disabled: bool,
    secure: bool,
}

impl TextField {
    /// Creates a new `TextField` backed by `state`.
    ///
    /// The returned builder uses theme defaults and is enabled with no placeholder.
    /// The app owns only the text. Cursor and selection persist internally across renders.
    /// App writes clear selection and clamp the cursor to a valid UTF-8 boundary.
    /// Cursor movement does not notify observers of the string.
    /// This constructor consumes hook slots; call it in a stable order on every render.
    pub fn new(cx: &Cx, state: State<String>) -> Self {
        let theme = cx.theme();
        TextField {
            editor: cx.state(FieldEditor::new(state.peek())),
            state,
            hovered: cx.state(false),
            pressed: cx.state(false),
            placeholder: String::new(),
            width: None,
            placeholder_color: theme.colors.foreground_secondary,
            text_color: theme.colors.foreground,
            surface_color: theme.colors.surface,
            surface_color_hover: theme.colors.surface_hover,
            surface_color_pressed: theme.colors.surface_pressed,
            surface_color_disabled: theme.colors.surface,
            border_color: theme.colors.border,
            text_color_disabled: theme.colors.foreground_disabled,
            padding: theme.spacing.sm,
            radius: theme.radius.sm,
            focus_ring_color: theme.chrome.focus_ring,
            style: TextFieldStyle::default(),
            alignment: TextAlignment::default(),
            overrides: FieldOverrides::default(),
            disabled: false,
            secure: false,
        }
    }

    fn new_secure(cx: &Cx, state: State<String>) -> Self {
        Self {
            secure: true,
            ..Self::new(cx, state)
        }
    }

    /// Sets the placeholder text shown when the field value is empty.
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }

    /// Fixed width in logical points (defaults to auto).
    pub fn width(mut self, v: f32) -> Self {
        self.width = Some(v);
        self
    }

    /// Aligns the value, placeholder, and caret horizontally (default: start/left).
    ///
    /// This affects content placement, not text direction or keyboard navigation.
    ///
    /// ```
    /// use lemon::prelude::*;
    /// let cx = Cx::new();
    /// let field = TextField::new(&cx, State::new(String::new()))
    ///     .width(240.0)
    ///     .alignment(TextAlignment::End);
    /// ```
    pub fn alignment(mut self, alignment: TextAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Selects a theme-aware visual preset (default: [`TextFieldStyle::Bordered`]).
    ///
    /// Explicit color, padding and radius overrides remain in effect regardless of call order.
    pub fn style(mut self, style: TextFieldStyle) -> Self {
        self.style = style;
        self
    }

    /// Overrides the placeholder text color for this widget.
    pub fn placeholder_color(mut self, color: Color) -> Self {
        self.overrides.placeholder_color = Some(color);
        self
    }

    /// Overrides the border color for this widget.
    pub fn border_color(mut self, color: Color) -> Self {
        self.overrides.border_color = Some(color);
        self
    }

    /// Overrides the container padding for this widget.
    pub fn inset(mut self, value: f32) -> Self {
        self.overrides.padding = Some(value);
        self
    }

    /// Overrides the border radius for this widget.
    pub fn rounding(mut self, value: f32) -> Self {
        self.overrides.radius = Some(value);
        self
    }

    /// Overrides the focus ring color, opting into a ring even with the clean preset.
    pub fn focus_ring_color(mut self, color: Color) -> Self {
        self.overrides.focus_ring_color = Some(color);
        self
    }

    /// Marks this text input as disabled, preventing focus and keyboard input.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Builds and returns the [`Element`] for this input field.
    ///
    /// Call this at the end of the builder chain to insert the widget into a parent container.
    pub fn into_element(self) -> Element {
        self.build(None)
    }

    fn build(self, area: Option<AreaLayout>) -> Element {
        let multiline = area.is_some();
        let mut field = self.editor.get();
        field.sync_value(self.state.get());
        let selection = if self.secure {
            None
        } else {
            field
                .selection_range()
                .and_then(|(start, end)| (start < end).then_some(start..end))
        };
        let disabled = self.disabled;
        let secure = self.secure;
        // Paint and retained metadata only receive display text and display byte offsets.
        // Editing continues to use the original UTF-8 value/cursor in the signal.
        let affinity = field.affinity;
        let (current_value, cursor) = if secure {
            let cursor_chars = field
                .value
                .char_indices()
                .take_while(|(offset, _)| *offset < field.cursor)
                .count();
            (
                "•".repeat(field.value.chars().count()),
                cursor_chars * '•'.len_utf8(),
            )
        } else {
            (field.value, field.cursor)
        };
        let placeholder = self.placeholder.clone();
        let placeholder_color = self
            .overrides
            .placeholder_color
            .unwrap_or(self.placeholder_color);
        let border_color = self.overrides.border_color.unwrap_or(self.border_color);
        let padding = self.overrides.padding.unwrap_or(self.padding);
        let radius = self.overrides.radius.unwrap_or(self.radius);
        let focus_ring_color =
            self.overrides
                .focus_ring_color
                .unwrap_or(if self.style == TextFieldStyle::Clean {
                    Color::default()
                } else {
                    self.focus_ring_color
                });
        let surface_color = {
            let clean = self.style == TextFieldStyle::Clean;
            let hovered = self.hovered.clone();
            let pressed = self.pressed.clone();
            let surface_color = self.surface_color;
            let surface_color_hover = self.surface_color_hover;
            let surface_color_pressed = self.surface_color_pressed;
            let surface_color_disabled = self.surface_color_disabled;
            ColorSource::Dynamic(std::rc::Rc::new(move || {
                if clean {
                    Color::default()
                } else if disabled {
                    surface_color_disabled
                } else if pressed.get() {
                    surface_color_pressed
                } else if hovered.get() {
                    surface_color_hover
                } else {
                    surface_color
                }
            }))
        };
        let text_color = if disabled {
            self.text_color_disabled
        } else {
            self.text_color
        };
        let placeholder_color = if disabled {
            self.text_color_disabled
        } else {
            placeholder_color
        };

        // Text shown in the field: current value or placeholder (dimmed).
        let text_child: Element = if current_value.is_empty() {
            Text::new(placeholder)
                .color(placeholder_color)
                .into_element()
        } else {
            Text::new(current_value.clone())
                .color(text_color)
                .into_element()
        };

        let content: Element = if let Some(area) = &area {
            ScrollView::with_offset(
                area.offset.clone(),
                ColumnView::new().fill_width().child(text_child),
            )
            .decorate(|view| view.fill())
        } else {
            RowView::new()
                .clip()
                .grow(1.0)
                .distribute(match self.alignment {
                    TextAlignment::Start => Distribute::Start,
                    TextAlignment::Center => Distribute::Center,
                    TextAlignment::End => Distribute::End,
                })
                .child(text_child)
                .into_element()
        };
        let mut container = View::new()
            .surface(surface_color)
            .inset(padding)
            .border(
                border_color,
                if self.style == TextFieldStyle::Bordered || self.overrides.border_color.is_some() {
                    1.5
                } else {
                    0.0
                },
            )
            .transition(super::interaction_transition())
            .rounding(radius)
            .clip()
            .cursor(if disabled {
                Cursor::NotAllowed
            } else {
                Cursor::Text
            })
            .child(content);
        if !disabled {
            container = container
                .focusable()
                .text_input(TextInputMeta {
                    affinity,
                    multiline,
                    selection,
                    alignment: self.alignment,
                    cursor,
                    value: current_value,
                })
                .on_key_down({
                    let state = self.state.clone();
                    let editor = self.editor.clone();
                    move |ev| {
                        if secure
                            && (ev.modifiers.ctrl || ev.modifiers.meta)
                            && matches!(&ev.key, LemonKey::Character(key)
                                if key.eq_ignore_ascii_case("c") || key.eq_ignore_ascii_case("x"))
                        {
                            return;
                        }
                        let value = state.peek();
                        let mut next = editor.peek();
                        next.sync_value(value.clone());
                        next.handle_key(&ev, multiline);
                        if next.value != value {
                            state.set(next.value.clone());
                        }
                        if next != editor.peek() {
                            editor.set(next);
                        }
                    }
                });
            let hovered_enter = self.hovered.clone();
            let hovered_leave = self.hovered.clone();
            let pressed_down = self.pressed.clone();
            let pressed_up = self.pressed.clone();
            let pressed_leave = self.pressed.clone();
            container = container
                .on_hover_enter(move || hovered_enter.set(true))
                .on_hover_leave(move || {
                    hovered_leave.set(false);
                    pressed_leave.set(false);
                })
                .on_pointer_down(move |_, _| pressed_down.set(true))
                .on_pointer_up(move |_, _| pressed_up.set(false));
        }

        if let Some(area) = &area {
            container = container.height(area.height);
            if let Some(resized) = &area.resize {
                let resized = resized.clone();
                let height = area.height;
                let minimum = area.minimum;
                container = container.inset_bottom(padding + 14.0).child(
                    View::new()
                        .position(Position::Absolute)
                        .bottom(2.0)
                        .right(padding)
                        .width(24.0)
                        .height(12.0)
                        .center()
                        .cursor(Cursor::ResizeVertical)
                        .child(
                            Icon::new(lucide::GRIP_HORIZONTAL)
                                .size(12.0)
                                .color(placeholder_color),
                        )
                        .on_drag(move |_dx, dy| {
                            resized.update(|value| {
                                *value =
                                    Some((value.unwrap_or(height).max(minimum) + dy).max(minimum))
                            });
                        }),
                );
            }
        }

        if let Some(w) = self.width {
            container = container.width(w);
        }

        let mut element = container.into_element();
        if let Element::View(view) = &mut element {
            view.paint.focus_ring_color = Some(ColorSource::Static(focus_ring_color));
            if multiline {
                view.style.flex_shrink = Some(0.0);
            }
            if !disabled {
                let editor = self.editor.clone();
                let state = self.state.clone();
                view.handlers.on_text_cursor =
                    Some(std::rc::Rc::new(move |index, affinity, extend| {
                        let mut next = editor.peek();
                        next.sync_value(state.peek());
                        let index = if secure {
                            next.value
                                .char_indices()
                                .nth(index / '•'.len_utf8())
                                .map_or(next.value.len(), |(i, _)| i)
                        } else {
                            index
                        };
                        next.move_cursor(index, extend);
                        next.affinity = affinity;
                        if next != editor.peek() {
                            editor.set(next);
                        }
                    }));
            }
        }
        element
    }
}

impl From<TextField> for Element {
    fn from(input: TextField) -> Self {
        input.into_element()
    }
}

struct AreaLayout {
    height: f32,
    minimum: f32,
    offset: State<crate::ScrollOffset>,
    resize: Option<State<Option<f32>>>,
}

/// Multiline editable text with wrapping, vertical scrolling and optional height resizing.
///
/// Accepts app-owned `State<String>` like [`TextField`]. Enter inserts a newline;
/// arrows navigate visual lines, and Ctrl/Cmd+A selects the text. The value,
/// internal cursor, selection and resized height persist across renders.
///
/// Defaults to 120 points high, a 60-point minimum, the bordered field preset and
/// a vertical resize handle. Width is automatic; set `.width(...)` or use root
/// decoration to provide a bounded width for wrapping. Disabled areas cannot resize.
/// This constructor consumes five hook slots and must be called in a stable order.
///
/// ```
/// use lemon::prelude::*;
/// let cx = Cx::new();
/// let notes = State::new(String::new());
/// let area = TextArea::new(&cx, notes).placeholder("Notes")
///     .width(320.0).height(140.0).min_height(80.0).resizable(true);
/// ```
pub struct TextArea {
    field: TextField,
    height: f32,
    minimum: f32,
    resizable: bool,
    resized: State<Option<f32>>,
    offset: State<crate::ScrollOffset>,
}

impl TextArea {
    /// Creates an enabled multiline field with theme defaults and app-owned text.
    pub fn new(cx: &Cx, value: State<String>) -> Self {
        Self {
            field: TextField::new(cx, value),
            height: 120.0,
            minimum: 60.0,
            resizable: true,
            resized: cx.state(None),
            offset: cx.state(crate::ScrollOffset::default()),
        }
    }

    /// Sets the placeholder shown only when the value is empty.
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.field = self.field.placeholder(text);
        self
    }

    /// Sets a fixed width in logical points; otherwise width is automatic.
    pub fn width(mut self, width: f32) -> Self {
        self.field = self.field.width(width);
        self
    }

    /// Sets initial height in logical points; user resizing takes precedence afterward.
    ///
    /// The minimum height is always enforced. Panics unless finite and positive.
    pub fn height(mut self, height: f32) -> Self {
        assert!(
            height.is_finite() && height > 0.0,
            "area height must be finite and positive"
        );
        self.height = height;
        self
    }

    /// Sets the minimum height in logical points (default: 60), including padding and handle.
    ///
    /// Applies immediately, including after a user resize. Panics unless finite and positive.
    pub fn min_height(mut self, minimum: f32) -> Self {
        assert!(
            minimum.is_finite() && minimum > 0.0,
            "minimum height must be finite and positive"
        );
        self.minimum = minimum;
        self
    }

    /// Enables or hides the vertical resize handle (default: true). Width never changes.
    /// Disabling resizing preserves the current user-chosen height.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Chooses the shared field preset, preserving explicit overrides.
    pub fn style(mut self, style: TextFieldStyle) -> Self {
        self.field = self.field.style(style);
        self
    }

    /// Overrides the placeholder color.
    pub fn placeholder_color(mut self, color: Color) -> Self {
        self.field = self.field.placeholder_color(color);
        self
    }

    /// Overrides the border color, including opting into a border with the clean preset.
    pub fn border_color(mut self, color: Color) -> Self {
        self.field = self.field.border_color(color);
        self
    }

    /// Overrides padding in logical points.
    pub fn inset(mut self, padding: f32) -> Self {
        self.field = self.field.inset(padding);
        self
    }

    /// Overrides corner radius in logical points.
    pub fn rounding(mut self, radius: f32) -> Self {
        self.field = self.field.rounding(radius);
        self
    }

    /// Overrides focus ring color, including opting into a ring with the clean preset.
    pub fn focus_ring_color(mut self, color: Color) -> Self {
        self.field = self.field.focus_ring_color(color);
        self
    }

    /// Prevents text input, focus and user resizing when `disabled` is true.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.field = self.field.disabled(disabled);
        self
    }

    /// Builds the control, keeping all edits on the runtime/diff/retained pipeline.
    pub fn into_element(self) -> Element {
        let height = self.resized.get().unwrap_or(self.height).max(self.minimum);
        let resize = (self.resizable && !self.field.disabled).then_some(self.resized);
        self.field.build(Some(AreaLayout {
            height,
            minimum: self.minimum,
            offset: self.offset,
            resize,
        }))
    }
}

impl From<TextArea> for Element {
    fn from(area: TextArea) -> Self {
        area.into_element()
    }
}

/// Single-line field that displays one bullet per Unicode scalar value.
///
/// Uses `State<String>` and the same theme, focus, and style overrides as [`TextField`].
/// The element tree, retained metadata, and caret use masked text; `state` keeps
/// the original value for application code. This masks the display; it does not
/// encrypt or zeroize the string stored in the application state.
///
/// Ctrl/Cmd+C and Ctrl/Cmd+X are ignored to avoid exporting the secret. Paste,
/// select-all, navigation, and deletion use the regular text editing behavior.
///
/// # Examples
///
/// ```no_run
/// use lemon::prelude::*;
///
/// fn login(cx: &Cx) -> Element {
///     let password = cx.state("".to_owned());
///     SecureField::new(cx, password)
///         .placeholder("Password")
///         .style(TextFieldStyle::Bordered)
///         .rounding(8.0)
///         .width(240.0)
///         .into_element()
/// }
/// ```
pub struct SecureField(TextField);

impl SecureField {
    /// Creates an enabled secure field backed by `state`, with theme defaults.
    ///
    /// No placeholder or fixed width is set. Building the element subscribes the
    /// parent to state changes. This constructor consumes hook slots and must be
    /// called in a stable order on every render.
    pub fn new(cx: &Cx, state: State<String>) -> Self {
        Self(TextField::new_secure(cx, state))
    }

    /// Sets the unmasked placeholder shown only while the value is empty.
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.0 = self.0.placeholder(text);
        self
    }

    /// Sets a fixed width in logical points; defaults to automatic sizing.
    pub fn width(mut self, value: f32) -> Self {
        self.0 = self.0.width(value);
        self
    }

    /// Aligns the masked value, placeholder, and caret (default: start/left).
    ///
    /// See [`TextField::alignment`] for placement semantics and an example.
    pub fn alignment(mut self, alignment: TextAlignment) -> Self {
        self.0 = self.0.alignment(alignment);
        self
    }

    /// Selects the same theme-aware preset as a plain field; explicit overrides are preserved.
    pub fn style(mut self, style: TextFieldStyle) -> Self {
        self.0 = self.0.style(style);
        self
    }

    /// Overrides the placeholder text color.
    pub fn placeholder_color(mut self, color: Color) -> Self {
        self.0 = self.0.placeholder_color(color);
        self
    }

    /// Overrides the border color.
    pub fn border_color(mut self, color: Color) -> Self {
        self.0 = self.0.border_color(color);
        self
    }

    /// Overrides the container padding in logical points.
    pub fn inset(mut self, value: f32) -> Self {
        self.0 = self.0.inset(value);
        self
    }

    /// Overrides the corner radius in logical points.
    pub fn rounding(mut self, value: f32) -> Self {
        self.0 = self.0.rounding(value);
        self
    }

    /// Overrides the focus ring color.
    pub fn focus_ring_color(mut self, color: Color) -> Self {
        self.0 = self.0.focus_ring_color(color);
        self
    }

    /// Prevents focus and keyboard input when `disabled` is true; text stays masked.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.0 = self.0.disabled(disabled);
        self
    }

    /// Builds the masked element, using display byte offsets for its caret metadata.
    pub fn into_element(self) -> Element {
        self.0.into_element()
    }
}

impl From<SecureField> for Element {
    fn from(field: SecureField) -> Self {
        field.into_element()
    }
}

#[cfg(test)]
mod secure_tests {
    use super::*;
    use crate::reactive::Effect;
    use crate::{
        element::types::BoxElement, ui::UiState, KeyEvent, KeyState, LemonKey, Modifiers, NamedKey,
        Viewport,
    };
    use std::{cell::Cell, rc::Rc};

    fn view(element: Element) -> BoxElement {
        let Element::View(view) = element else {
            panic!("expected field container")
        };
        view
    }

    fn displayed(view: &BoxElement) -> String {
        let Element::Row(row) = &view.children[0] else {
            panic!("expected text wrapper")
        };
        let Element::Text(text) = &row.children[0] else {
            panic!("expected display text")
        };
        text.content.resolve()
    }

    fn key(key: LemonKey) -> KeyEvent {
        KeyEvent {
            key,
            modifiers: Modifiers::default(),
            repeat: false,
            state: KeyState::Pressed,
        }
    }

    #[test]
    fn unicode_secrets_only_reach_retained_layout_and_paint_as_bullets() {
        let state = State::new("".to_owned());
        let source = state.clone();
        let mut ui = UiState::default();
        ui.mount(move |cx| {
            SecureField::new(cx, source.clone())
                .width(240.0)
                .into_element()
        })
        .unwrap();
        let viewport = Viewport {
            width: 300.0,
            height: 100.0,
        };
        let focused = ui
            .retained
            .as_ref()
            .unwrap()
            .root
            .as_ref()
            .unwrap()
            .taffy_id;
        let initial = view(ui.runtime.root_element().unwrap());
        initial.handlers.on_key_down.as_ref().unwrap()(key(LemonKey::Character("aé🔒".into())));
        ui.update(viewport, 1.0, focused, true).unwrap();

        assert_eq!(state.get(), "aé🔒");
        let root = ui.retained.as_ref().unwrap().root.as_ref().unwrap();
        let meta = root.text_input.as_ref().unwrap();
        assert_eq!(meta.value, "•••");
        assert_eq!(meta.cursor, "•••".len());
        let text = root.children[0].children[0].text.as_ref().unwrap();
        assert_eq!(text.content, "•••");
        assert_eq!(text.caret, meta.cursor);
        assert!(text.parley_layout.is_some());
        assert!(!ui.layout_dirty);
        assert!(!ui.paint_dirty);

        // Moving inside a Unicode secret changes only caret metadata, not mask text.
        let current = view(ui.runtime.root_element().unwrap());
        current.handlers.on_key_down.as_ref().unwrap()(key(LemonKey::Named(NamedKey::ArrowLeft)));
        ui.apply_updates().unwrap();
        assert!(!ui.layout_dirty);
        assert!(ui.paint_dirty);
        let root = ui.retained.as_ref().unwrap().root.as_ref().unwrap();
        assert_eq!(root.text_input.as_ref().unwrap().cursor, "••".len());
        assert_eq!(
            root.children[0].children[0].text.as_ref().unwrap().caret,
            "••".len()
        );
        ui.update(viewport, 1.0, focused, true).unwrap();

        current.handlers.on_key_down.as_ref().unwrap()(key(LemonKey::Named(NamedKey::Backspace)));
        ui.update(viewport, 1.0, focused, true).unwrap();
        assert_eq!(state.get(), "a🔒");
        let root = ui.retained.as_ref().unwrap().root.as_ref().unwrap();
        assert_eq!(root.text_input.as_ref().unwrap().value, "••");
        assert_eq!(root.text_input.as_ref().unwrap().cursor, "•".len());
    }

    #[test]
    fn mask_cursor_uses_display_byte_offsets_for_every_character_boundary() {
        for (steps, mask_cursor) in [(0, 0), (1, 3), (2, 6), (3, 9)] {
            let cx = Cx::new();
            let state = State::new("aé🔒".to_owned());
            let initial = view(SecureField::new(&cx, state.clone()).into_element());
            for _ in 0..steps {
                initial.handlers.on_key_down.as_ref().unwrap()(key(LemonKey::Named(
                    NamedKey::ArrowRight,
                )));
            }
            cx.reset_hooks();
            let field = view(SecureField::new(&cx, state).into_element());
            assert_eq!(displayed(&field), "•••");
            assert_eq!(field.text_input.as_ref().unwrap().cursor, mask_cursor);
        }
    }

    #[test]
    fn empty_placeholder_and_disabled_mask_share_plain_field_styles() {
        let cx = Cx::new();
        let style = TextFieldStyle::Bordered;
        let plain = view(
            TextField::new(&cx, State::new("".to_owned()))
                .placeholder("Password")
                .style(style)
                .placeholder_color(Color::rgb8(1, 2, 3))
                .border_color(Color::rgb8(4, 5, 6))
                .focus_ring_color(Color::rgb8(7, 8, 9))
                .inset(10.0)
                .rounding(8.0)
                .width(220.0)
                .into_element(),
        );
        let secure = view(
            SecureField::new(&cx, State::new("".to_owned()))
                .placeholder("Password")
                .style(style)
                .placeholder_color(Color::rgb8(1, 2, 3))
                .border_color(Color::rgb8(4, 5, 6))
                .focus_ring_color(Color::rgb8(7, 8, 9))
                .inset(10.0)
                .rounding(8.0)
                .width(220.0)
                .into_element(),
        );
        assert_eq!(displayed(&secure), "Password");
        assert_eq!(secure.text_input.as_ref().unwrap().value, "");
        assert_eq!(secure.text_input.as_ref().unwrap().cursor, 0);
        assert_eq!(secure.style, plain.style);
        assert_eq!(secure.paint.resolve(), plain.paint.resolve());

        let disabled = view(
            SecureField::new(&cx, State::new("secret".to_owned()))
                .disabled(true)
                .into_element(),
        );
        assert_eq!(displayed(&disabled), "••••••");
        assert!(!disabled.style.focusable);
        assert!(disabled.text_input.is_none());
        assert!(disabled.handlers.on_key_down.is_none());
        assert!(disabled.handlers.on_pointer_down.is_none());
    }

    #[test]
    fn select_all_deletion_and_typing_keep_secret_state_separate_from_display() {
        let state = State::new("old password".to_owned());
        let cx = Cx::new();
        let field = view(SecureField::new(&cx, state.clone()).into_element());
        let handler = field.handlers.on_key_down.as_ref().unwrap();
        let mut select_all = key(LemonKey::Character("a".into()));
        select_all.modifiers.meta = true;
        handler(select_all);
        handler(key(LemonKey::Named(NamedKey::Backspace)));
        assert_eq!(state.get(), "");
        handler(key(LemonKey::Character("new🔑".into())));
        assert_eq!(state.get(), "new🔑");
        cx.reset_hooks();
        let field = view(SecureField::new(&cx, state).into_element());
        assert_eq!(displayed(&field), "••••");
        assert_eq!(field.text_input.as_ref().unwrap().cursor, "••••".len());
    }

    #[test]
    fn clipboard_export_shortcuts_never_enter_the_editor() {
        let state = State::new("secret".to_owned());
        let read = state.clone();
        let updates = Rc::new(Cell::new(0));
        let count = updates.clone();
        let _effect = Effect::new(move || {
            read.get();
            count.set(count.get() + 1);
        });
        let field = view(SecureField::new(&Cx::new(), state.clone()).into_element());
        for modifiers in [
            Modifiers {
                ctrl: true,
                ..Default::default()
            },
            Modifiers {
                meta: true,
                ..Default::default()
            },
        ] {
            for shortcut in ["c", "C", "x", "X"] {
                let mut event = key(LemonKey::Character(shortcut.into()));
                event.modifiers = modifiers.clone();
                field.handlers.on_key_down.as_ref().unwrap()(event);
            }
        }
        assert_eq!(
            updates.get(),
            1,
            "blocked exports must bypass state/clipboard handling"
        );
        assert_eq!(state.get(), "secret");
        // Navigation updates only the internal caret, not the app's string.
        field.handlers.on_key_down.as_ref().unwrap()(key(LemonKey::Named(NamedKey::End)));
        assert_eq!(updates.get(), 1);
        field.handlers.on_key_down.as_ref().unwrap()(key(LemonKey::Character("!".into())));
        assert_eq!(updates.get(), 2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        current_theme,
        element::{style::CornerRadii, Element},
        set_active_theme, Theme,
    };

    #[test]
    fn text_field_defaults_follow_active_theme() {
        let previous = current_theme();
        let mut custom = Theme::default_dark();
        custom.colors.foreground_secondary = Color::rgb8(120, 121, 122);
        custom.colors.border = Color::rgb8(12, 34, 56);
        custom.spacing.sm = 10.0;
        custom.radius.sm = 7.0;
        set_active_theme(custom.clone());

        let cx = Cx::new();
        let state = State::new("".to_owned());
        let Element::View(view) = TextField::new(&cx, state)
            .placeholder("Type here")
            .into_element()
        else {
            panic!("expected View element");
        };

        assert_eq!(
            view.style.padding,
            Some(crate::element::style::Edges::all(custom.spacing.sm))
        );
        let paint = view.paint.resolve();
        assert_eq!(
            paint.border_color,
            crate::element::style::Edges::all(Some(custom.colors.border))
        );
        assert_eq!(paint.radius, CornerRadii::all(custom.radius.sm));

        let Element::Row(row) = &view.children[0] else {
            panic!("expected inner Row");
        };
        let Element::Text(text) = &row.children[0] else {
            panic!("expected placeholder Text");
        };
        assert_eq!(text.style.color, Some(custom.colors.foreground_secondary));

        let state_with_text = State::new("hello".to_owned());
        let Element::View(filled) = TextField::new(&cx, state_with_text).into_element() else {
            panic!("expected View element");
        };
        let Element::Row(filled_row) = &filled.children[0] else {
            panic!("expected inner Row");
        };
        let Element::Text(filled_text) = &filled_row.children[0] else {
            panic!("expected value Text");
        };
        assert_eq!(filled_text.style.color, Some(custom.colors.foreground));
        let filled_paint = filled.paint.resolve();
        assert_eq!(filled_paint.background, Some(custom.colors.surface));

        set_active_theme(previous);
    }

    #[test]
    fn text_field_style_overrides_and_fallbacks() {
        let previous = current_theme();
        let mut custom = Theme::default_dark();
        custom.colors.foreground_secondary = Color::rgb8(120, 121, 122);
        custom.colors.border = Color::rgb8(12, 34, 56);
        custom.chrome.focus_ring = Color::rgb8(30, 40, 50);
        custom.spacing.sm = 10.0;
        custom.radius.sm = 7.0;
        set_active_theme(custom);

        let cx = Cx::new();
        let state = State::new("".to_owned());
        let Element::View(view) = TextField::new(&cx, state)
            .style(TextFieldStyle::Bordered)
            .border_color(Color::rgb8(1, 2, 3))
            .focus_ring_color(Color::rgb8(4, 5, 6))
            .placeholder("Type here")
            .into_element()
        else {
            panic!("expected View element");
        };

        assert_eq!(
            view.style.padding,
            Some(crate::element::style::Edges::all(10.0)),
            "unset padding should fall back to theme default"
        );
        let paint = view.paint.resolve();
        assert_eq!(
            paint.border_color,
            crate::element::style::Edges::all(Some(Color::rgb8(1, 2, 3)))
        );
        assert_eq!(paint.focus_ring_color, Some(Color::rgb8(4, 5, 6)));

        let Element::Row(row) = &view.children[0] else {
            panic!("expected inner Row");
        };
        let Element::Text(text) = &row.children[0] else {
            panic!("expected placeholder Text");
        };
        assert_eq!(
            text.style.color,
            Some(Color::rgb8(120, 121, 122)),
            "unset placeholder color should fall back to theme default"
        );

        set_active_theme(previous);
    }

    #[test]
    fn text_field_hover_and_press_states_update_surface_color() {
        let previous = current_theme();
        let mut custom = Theme::default_dark();
        custom.colors.surface = Color::rgb8(1, 2, 3);
        custom.colors.surface_hover = Color::rgb8(4, 5, 6);
        custom.colors.surface_pressed = Color::rgb8(7, 8, 9);
        set_active_theme(custom.clone());

        let cx = Cx::new();
        let state = State::new("".to_owned());
        let Element::View(view) = TextField::new(&cx, state).into_element() else {
            panic!("expected View element");
        };

        assert_eq!(view.paint.resolve().background, Some(custom.colors.surface));
        view.handlers
            .on_hover_enter
            .as_ref()
            .expect("hover enter handler")();
        assert_eq!(
            view.paint.resolve().background,
            Some(custom.colors.surface_hover)
        );
        view.handlers
            .on_pointer_down
            .as_ref()
            .expect("pointer down handler")(0.5, 0.5);
        assert_eq!(
            view.paint.resolve().background,
            Some(custom.colors.surface_pressed)
        );
        view.handlers
            .on_pointer_up
            .as_ref()
            .expect("pointer up handler")(0.5, 0.5);
        assert_eq!(
            view.paint.resolve().background,
            Some(custom.colors.surface_hover)
        );

        set_active_theme(previous);
    }

    #[test]
    fn disabled_text_field_ignores_keyboard_and_uses_disabled_colors() {
        let previous = current_theme();
        let mut custom = Theme::default_dark();
        custom.colors.foreground_disabled = Color::rgb8(9, 10, 11);
        set_active_theme(custom.clone());

        let cx = Cx::new();
        let state = State::new("".to_owned());
        let Element::View(view) = TextField::new(&cx, state.clone())
            .placeholder("Type here")
            .disabled(true)
            .into_element()
        else {
            panic!("expected View element");
        };

        assert!(!view.style.focusable);
        assert!(view.text_input.is_none());
        assert!(view.handlers.on_key_down.is_none());
        assert!(view.handlers.on_pointer_down.is_none());

        let Element::Row(row) = &view.children[0] else {
            panic!("expected inner Row");
        };
        let Element::Text(text) = &row.children[0] else {
            panic!("expected placeholder Text");
        };
        assert_eq!(text.style.color, Some(custom.colors.foreground_disabled));
        assert_eq!(state.get(), "");

        set_active_theme(previous);
    }
}
