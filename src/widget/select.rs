use crate::{
    element::{
        builders::{ColumnView, RowView, Text, View},
        events::Cursor,
        style::{Align, Color, ColorSource, Distribute},
        Element,
    },
    BoxShadow, Cx, KeyState, LemonKey, NamedKey, State,
};

/// Per-widget visual overrides for [`Select`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SelectStyle {
    trigger_height: Option<f32>,
    trigger_padding: Option<f32>,
    trigger_background: Option<Color>,
    trigger_radius: Option<f32>,
    dropdown_background: Option<Color>,
    dropdown_border: Option<Color>,
    dropdown_radius: Option<f32>,
    dropdown_transition: Option<crate::Transition>,
    item_padding: Option<f32>,
    item_text_color: Option<Color>,
}

impl SelectStyle {
    /// Overrides trigger height in logical points.
    pub fn trigger_height(mut self, value: f32) -> Self {
        self.trigger_height = Some(value);
        self
    }

    /// Overrides trigger padding in logical points.
    pub fn trigger_inset(mut self, value: f32) -> Self {
        self.trigger_padding = Some(value);
        self
    }

    /// Overrides trigger background color.
    pub fn trigger_surface(mut self, color: Color) -> Self {
        self.trigger_background = Some(color);
        self
    }

    /// Overrides trigger corner radius in logical points.
    pub fn trigger_rounding(mut self, value: f32) -> Self {
        self.trigger_radius = Some(value);
        self
    }

    /// Overrides dropdown background color.
    pub fn dropdown_surface(mut self, color: Color) -> Self {
        self.dropdown_background = Some(color);
        self
    }

    /// Overrides dropdown border color.
    pub fn dropdown_border(mut self, color: Color) -> Self {
        self.dropdown_border = Some(color);
        self
    }

    /// Overrides dropdown corner radius in logical points.
    pub fn dropdown_rounding(mut self, value: f32) -> Self {
        self.dropdown_radius = Some(value);
        self
    }

    /// Configures the overlay's enter/exit transition (default: 140 ms, ease-out).
    ///
    /// Include [`crate::TransitionProperty::Opacity`] when selecting properties.
    /// A zero duration disables the fade; input still follows the logical open state.
    ///
    /// ```
    /// use lemon::{SelectStyle, Transition};
    /// let style = SelectStyle::default().dropdown_transition(
    ///     Transition::new(std::time::Duration::ZERO));
    /// ```
    pub fn dropdown_transition(mut self, transition: crate::Transition) -> Self {
        self.dropdown_transition = Some(transition);
        self
    }

    /// Overrides option-item padding in logical points.
    pub fn item_inset(mut self, value: f32) -> Self {
        self.item_padding = Some(value);
        self
    }

    /// Overrides dropdown option label color.
    pub fn item_text_color(mut self, color: Color) -> Self {
        self.item_text_color = Some(color);
        self
    }
}

/// Dropdown select widget backed by user-owned signal state.
///
/// `Select<T>` renders a trigger button that opens a dropdown list of choices. Selecting an option
/// updates the value signal and closes the dropdown; clicking outside the widget also closes it.
///
/// The dropdown list is rendered as an **overlay** — it is absolutely positioned so it floats
/// above surrounding content and does not shift sibling elements when opened.
/// It fades in and out using the shared transition pipeline. The closed overlay
/// stays mounted but rejects input immediately, including during its exit fade.
/// Arrow keys navigate, Enter/Space choose, and Escape dismisses the menu.
///
/// The open/closed state is internal by default ([`Select::new`]) or caller-supplied for advanced
/// use ([`Select::with_open`]).
///
/// # Examples
///
/// ```no_run
/// use lemon::{Color, Cx, element::Element};
/// use lemon::widget::{Select, SelectStyle};
///
/// #[derive(Clone, PartialEq)]
/// enum Size { Small, Medium, Large }
///
/// fn my_view(cx: &Cx) -> Element {
///     let size = cx.state(None::<Size>);
///     Select::new(
///         cx,
///         size,
///         vec![
///             (Size::Small, "Small".to_string()),
///             (Size::Medium, "Medium".to_string()),
///             (Size::Large, "Large".to_string()),
///         ],
///     )
///     .style(SelectStyle::default().dropdown_surface(Color::rgb8(30, 30, 40)))
///     .placeholder("Choose a size")
///     .width(180.0)
///     .into_element()
/// }
/// ```
pub struct Select<T> {
    value: State<Option<T>>,
    open: State<bool>,
    trigger_hovered: State<bool>,
    trigger_pressed: State<bool>,
    highlighted: State<Option<T>>,
    accent: Color,
    secondary_text: Color,
    options: Vec<(T, String)>,
    width: Option<f32>,
    placeholder: String,
    trigger_height: f32,
    trigger_padding: f32,
    trigger_background: Color,
    trigger_background_hover: Color,
    trigger_background_pressed: Color,
    trigger_background_disabled: Color,
    trigger_radius: f32,
    trigger_text_color: Color,
    trigger_text_color_disabled: Color,
    dropdown_background: Color,
    dropdown_border: Color,
    dropdown_radius: f32,
    item_padding: f32,
    item_text_color: Color,
    style: SelectStyle,
    disabled: bool,
}

impl<T: Clone + PartialEq + 'static> Select<T> {
    /// Creates a `Select` with internally managed open/closed state.
    ///
    /// - `value` — caller-owned signal updated with the chosen option when the user selects one.
    /// - `options` — ordered list of `(value, label)` pairs shown as choices.
    pub fn new(cx: &Cx, value: State<Option<T>>, options: Vec<(T, String)>) -> Self {
        Self::with_open(cx, value, cx.state(false), options)
    }

    /// Creates a `Select` with a caller-supplied open signal.
    ///
    /// Useful when you need to control or observe the open state externally (e.g. in tests).
    ///
    /// `cx` supplies the active theme so the widget can derive its default trigger and dropdown
    /// styling from [`Cx::theme`](crate::Cx::theme).
    pub fn with_open(
        cx: &Cx,
        value: State<Option<T>>,
        open: State<bool>,
        options: Vec<(T, String)>,
    ) -> Self {
        let theme = cx.theme();
        Self {
            value,
            open,
            trigger_hovered: cx.state(false),
            trigger_pressed: cx.state(false),
            highlighted: cx.state(None),
            accent: theme.colors.accent,
            secondary_text: theme.colors.foreground_secondary,
            options,
            width: None,
            placeholder: "Select…".to_string(),
            trigger_height: theme.typography.font_size_md * theme.typography.line_height
                + theme.spacing.sm * 2.0,
            trigger_padding: theme.spacing.sm,
            trigger_background: theme.colors.surface,
            trigger_background_hover: theme.colors.surface_hover,
            trigger_background_pressed: theme.colors.surface_pressed,
            trigger_background_disabled: theme.colors.surface,
            trigger_radius: theme.radius.md,
            trigger_text_color: theme.colors.foreground,
            trigger_text_color_disabled: theme.colors.foreground_disabled,
            dropdown_background: theme.colors.surface,
            dropdown_border: theme.colors.border,
            dropdown_radius: theme.radius.md,
            item_padding: theme.spacing.sm,
            item_text_color: theme.colors.foreground,
            style: SelectStyle::default(),
            disabled: false,
        }
    }

    /// Sets the placeholder text shown in the trigger when no option is selected.
    ///
    /// Defaults to `"Select…"`.
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }

    /// Fixed width in logical points applied to the trigger and dropdown list.
    pub fn width(mut self, v: f32) -> Self {
        self.width = Some(v);
        self
    }

    /// Replaces all select style overrides.
    pub fn style(mut self, style: SelectStyle) -> Self {
        self.style = style;
        self
    }

    /// Marks this select as disabled, preventing trigger and option interactions.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Overrides trigger background color for this widget.
    pub fn trigger_surface(mut self, color: Color) -> Self {
        self.style.trigger_background = Some(color);
        self
    }

    /// Overrides dropdown background color for this widget.
    pub fn dropdown_surface(mut self, color: Color) -> Self {
        self.style.dropdown_background = Some(color);
        self
    }

    /// Overrides dropdown border color for this widget.
    pub fn dropdown_border(mut self, color: Color) -> Self {
        self.style.dropdown_border = Some(color);
        self
    }

    /// Builds and returns the [`Element`] for this select widget.
    ///
    /// Call this at the end of the builder chain to insert the widget into a parent container.
    pub fn into_element(self) -> Element {
        let is_open = !self.disabled && !self.options.is_empty() && self.open.get();
        let highlighted = self.highlighted;
        let highlighted_value = highlighted.get();
        let accent = self.accent;
        let secondary_text = self.secondary_text;
        let current = self.value.get();
        let open = self.open;
        let trigger_hovered = self.trigger_hovered;
        let trigger_pressed = self.trigger_pressed;
        let value = self.value;
        let options = self.options;
        let width = self.width;
        let placeholder = self.placeholder;
        let disabled = self.disabled;
        let trigger_height = self.trigger_height;
        let trigger_padding = self.trigger_padding;
        let trigger_background = self.trigger_background;
        let trigger_background_hover = self.trigger_background_hover;
        let trigger_background_pressed = self.trigger_background_pressed;
        let trigger_background_disabled = self.trigger_background_disabled;
        let trigger_radius = self.trigger_radius;
        let trigger_text_color = self.trigger_text_color;
        let trigger_text_color_disabled = self.trigger_text_color_disabled;
        let dropdown_background = self.dropdown_background;
        let trigger_border = self.dropdown_border;
        let dropdown_border = self.dropdown_border;
        let dropdown_radius = self.dropdown_radius;
        let item_padding = self.item_padding;
        let item_text_color = self.item_text_color;
        let style = self.style;
        let trigger_height = style.trigger_height.unwrap_or(trigger_height);
        let trigger_padding = style.trigger_padding.unwrap_or(trigger_padding);
        let trigger_background = style.trigger_background.unwrap_or(trigger_background);
        let trigger_radius = style.trigger_radius.unwrap_or(trigger_radius);
        let dropdown_background = style.dropdown_background.unwrap_or(dropdown_background);
        let dropdown_border = style.dropdown_border.unwrap_or(dropdown_border);
        let dropdown_radius = style.dropdown_radius.unwrap_or(dropdown_radius);
        let item_padding = style.item_padding.unwrap_or(item_padding);
        let item_text_color = style.item_text_color.unwrap_or(item_text_color);
        let dropdown_transition = style
            .dropdown_transition
            .unwrap_or_else(super::interaction_transition);

        // Derive trigger label from the current selection, falling back to placeholder.
        let trigger_label = current.as_ref().and_then(|v| {
            options
                .iter()
                .find(|(opt, _)| opt == v)
                .map(|(_, l)| l.clone())
        });
        let has_selection = trigger_label.is_some();
        let trigger_label = trigger_label.unwrap_or(placeholder);

        let trigger_text_color = if disabled {
            trigger_text_color_disabled
        } else if has_selection {
            trigger_text_color
        } else {
            secondary_text
        };
        let trigger_hovered_for_bg = trigger_hovered.clone();
        let trigger_pressed_for_bg = trigger_pressed.clone();
        let trigger_background =
            style
                .trigger_background
                .map(ColorSource::Static)
                .unwrap_or(ColorSource::Dynamic(std::rc::Rc::new(move || {
                    if disabled {
                        trigger_background_disabled
                    } else if trigger_pressed_for_bg.get() {
                        trigger_background_pressed
                    } else if trigger_hovered_for_bg.get() {
                        trigger_background_hover
                    } else {
                        trigger_background
                    }
                })));

        let mut trigger = View::new()
            .inset(trigger_padding)
            .surface(trigger_background)
            .rounding(trigger_radius)
            .border(if is_open { accent } else { trigger_border }, 1.0)
            .transition(super::interaction_transition())
            .height(trigger_height)
            .align(Align::Center)
            .distribute(Distribute::Between)
            .cursor(if disabled {
                Cursor::NotAllowed
            } else {
                Cursor::Pointer
            })
            .child(
                RowView::new()
                    .grow(1.0)
                    .align(Align::Center)
                    .distribute(Distribute::Between)
                    .spacing(12.0)
                    .child(Text::new(trigger_label).color(trigger_text_color))
                    .child(Text::new(if is_open { "▴" } else { "▾" }).color(secondary_text)),
            );
        if let Some(w) = width {
            trigger = trigger.width(w);
        }
        if !disabled {
            let open_for_trigger = open.clone();
            let active_for_trigger = highlighted.clone();
            let selected_for_trigger = value.clone();
            let open_for_key = open.clone();
            let value_for_key = value.clone();
            let active_for_key = highlighted.clone();
            let key_options = options.clone();
            let has_options = !options.is_empty();
            let hovered_enter = trigger_hovered.clone();
            let hovered_leave = trigger_hovered.clone();
            let pressed_down = trigger_pressed.clone();
            let pressed_up = trigger_pressed.clone();
            let pressed_leave = trigger_pressed.clone();
            trigger = trigger
                .action(move || {
                    if has_options {
                        active_for_trigger.set(selected_for_trigger.peek());
                        open_for_trigger.update(|b| *b = !*b);
                    }
                })
                .focusable()
                .on_key_down(move |event| {
                    if event.state != KeyState::Pressed || key_options.is_empty() {
                        return;
                    }
                    match event.key {
                        LemonKey::Named(NamedKey::Escape) => open_for_key.set(false),
                        LemonKey::Named(NamedKey::ArrowDown | NamedKey::ArrowUp) => {
                            let up = event.key == LemonKey::Named(NamedKey::ArrowUp);
                            let current = active_for_key.peek().or_else(|| value_for_key.peek());
                            let index = current.as_ref().and_then(|value| {
                                key_options
                                    .iter()
                                    .position(|(candidate, _)| candidate == value)
                            });
                            let next = if !open_for_key.peek() {
                                index.unwrap_or(if up { key_options.len() - 1 } else { 0 })
                            } else {
                                match index {
                                    Some(index) if up => {
                                        (index + key_options.len() - 1) % key_options.len()
                                    }
                                    Some(index) => (index + 1) % key_options.len(),
                                    None if up => key_options.len() - 1,
                                    None => 0,
                                }
                            };
                            active_for_key.set(Some(key_options[next].0.clone()));
                            open_for_key.set(true);
                        }
                        LemonKey::Named(NamedKey::Enter | NamedKey::Space) if !event.repeat => {
                            if open_for_key.peek() {
                                let current =
                                    active_for_key.peek().or_else(|| value_for_key.peek());
                                let chosen = current
                                    .filter(|value| {
                                        key_options.iter().any(|(candidate, _)| candidate == value)
                                    })
                                    .unwrap_or_else(|| key_options[0].0.clone());
                                value_for_key.set(Some(chosen));
                                open_for_key.set(false);
                            } else {
                                active_for_key.set(value_for_key.peek());
                                open_for_key.set(true);
                            }
                        }
                        _ => {}
                    }
                })
                .on_hover_enter(move || hovered_enter.set(true))
                .on_hover_leave(move || {
                    hovered_leave.set(false);
                    pressed_leave.set(false);
                })
                .on_pointer_down(move |_, _| pressed_down.set(true))
                .on_pointer_up(move |_, _| pressed_up.set(false));
        }

        // Outer container: on_click_outside closes the dropdown when the user clicks elsewhere.
        let mut container = ColumnView::new();
        if !disabled {
            let open_for_outside = open.clone();
            container = container.on_click_outside(move || open_for_outside.set(false));
        }
        if let Some(w) = width {
            container = container.width(w);
        }
        let mut trigger = trigger.into_element();
        if let Element::View(view) = &mut trigger {
            view.paint.focus_ring_color = Some(accent.into());
        }
        container = container.child(trigger);

        // Keep the overlay mounted at opacity zero when closed. This lets the
        // shared transition pipeline interpolate both directions without timers.
        // Input is disabled immediately, independently of the presentation opacity.
        let mut dropdown = ColumnView::new()
            .layer(10)
            .position(crate::Position::Absolute)
            .top(trigger_height + 4.0)
            .left(0.0)
            .fill_width()
            .inset(4.0)
            .spacing(2.0)
            .surface(dropdown_background)
            .border(dropdown_border, 1.0)
            .rounding(dropdown_radius)
            .clip()
            .shadow(BoxShadow::new(
                Color::rgb8(0, 0, 0).with_alpha(0.18),
                0.0,
                4.0,
                12.0,
            ))
            .opacity(if is_open { 1.0 } else { 0.0 })
            .interactive(is_open)
            .transition(dropdown_transition);
        if let Some(w) = width {
            dropdown = dropdown.width(w);
        }
        for (index, (opt, label)) in options.into_iter().enumerate() {
            let selected = current.as_ref() == Some(&opt);
            let active = highlighted_value.as_ref() == Some(&opt);
            let open_c = open.clone();
            let val_c = value.clone();
            let choose = opt.clone();
            let hover_enter = highlighted.clone();
            let hover_leave = highlighted.clone();
            let hover_value = opt.clone();
            let mut item = RowView::new()
                .key(index as u64)
                .fill_width()
                .inset(item_padding)
                .spacing(12.0)
                .align(Align::Center)
                .distribute(Distribute::Between)
                .rounding((dropdown_radius - 2.0).max(0.0))
                .surface(if active {
                    accent.with_alpha(0.16)
                } else if selected {
                    accent.with_alpha(0.08)
                } else {
                    Color::default()
                })
                .cursor(Cursor::Pointer)
                .child(Text::new(label).color(item_text_color))
                .child(Text::new(if selected { "✓" } else { "" }).color(accent));
            if is_open {
                item = item
                    .action(move || {
                        val_c.set(Some(choose.clone()));
                        open_c.set(false);
                    })
                    .on_hover_enter(move || hover_enter.set(Some(opt.clone())))
                    .on_hover_leave(move || {
                        if hover_leave.peek().as_ref() == Some(&hover_value) {
                            hover_leave.set(None);
                        }
                    });
            }
            dropdown = dropdown.child(item);
        }
        container = container.child(dropdown);

        container.into_element()
    }
}

impl<T: Clone + PartialEq + 'static> From<Select<T>> for Element {
    fn from(select: Select<T>) -> Self {
        select.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::Element;
    use crate::layout::{LayoutMap, Viewport};
    use parley::FontContext;

    fn layout_pass(
        tree: &mut crate::RetainedTree,
        viewport: Viewport,
    ) -> Result<LayoutMap, crate::retained::RetainedError> {
        let mut font_cx = FontContext::new();
        crate::layout::layout_pass(tree, &mut font_cx, viewport)
    }

    #[derive(Clone, PartialEq, Debug)]
    enum Color {
        Red,
        Green,
        Blue,
    }

    fn make_options() -> Vec<(Color, String)> {
        vec![
            (Color::Red, "Red".to_string()),
            (Color::Green, "Green".to_string()),
            (Color::Blue, "Blue".to_string()),
        ]
    }

    #[test]
    fn select_closed_shows_placeholder_in_trigger() {
        let value = State::new(None::<Color>);
        let open = State::new(false);
        let cx = Cx::new();
        let el = Select::with_open(&cx, value, open, make_options()).into_element();

        let Element::Column(col) = el else {
            panic!("expected Column container");
        };
        // The overlay is retained for the exit fade, but rejects input when closed.
        assert_eq!(col.children.len(), 2);
        let Element::Column(dropdown) = &col.children[1] else {
            panic!("dropdown")
        };
        assert_eq!(dropdown.style.opacity, 0.0);
        assert!(!dropdown.style.interactive);
        let Element::View(trigger) = &col.children[0] else {
            panic!("expected View trigger");
        };
        let Element::Row(content) = &trigger.children[0] else {
            panic!("trigger content")
        };
        let Element::Text(label) = &content.children[0] else {
            panic!("expected trigger Text");
        };
        let label = label.content.resolve();
        assert!(
            label.contains("Select"),
            "trigger should contain placeholder, got: {label}"
        );
    }

    #[test]
    fn select_closed_shows_selected_label_in_trigger() {
        let value = State::new(Some(Color::Green));
        let open = State::new(false);
        let cx = Cx::new();
        let el = Select::with_open(&cx, value, open, make_options()).into_element();

        let Element::Column(col) = el else {
            panic!("expected Column container");
        };
        let Element::View(trigger) = &col.children[0] else {
            panic!("expected View trigger");
        };
        let Element::Row(content) = &trigger.children[0] else {
            panic!("trigger content")
        };
        let Element::Text(label) = &content.children[0] else {
            panic!("expected trigger Text");
        };
        let label = label.content.resolve();
        assert!(
            label.contains("Green"),
            "trigger should show selected label, got: {label}"
        );
    }

    #[test]
    fn select_open_shows_options_as_children() {
        let value = State::new(None::<Color>);
        let open = State::new(true);
        let cx = Cx::new();
        let el = Select::with_open(&cx, value, open, make_options()).into_element();

        let Element::Column(col) = el else {
            panic!("expected Column container");
        };
        // Open: trigger + dropdown column
        assert_eq!(col.children.len(), 2);
        let Element::Column(dropdown) = &col.children[1] else {
            panic!("expected Column dropdown");
        };
        assert_eq!(dropdown.children.len(), 3, "dropdown should have 3 options");
        assert_eq!(dropdown.style.z_index, 10);
        assert!(
            dropdown.style.position == crate::Position::Absolute,
            "dropdown must be absolutely positioned to overlay content"
        );
    }

    #[test]
    fn select_open_dropdown_lays_out_below_trigger() {
        use crate::RetainedTree;

        let value = State::new(None::<Color>);
        let open = State::new(true);
        let cx = Cx::new();
        let root = ColumnView::new()
            .child(
                Select::with_open(&cx, value, open, make_options())
                    .placeholder("Pick")
                    .width(180.0),
            )
            .child(Text::new("Sibling below"))
            .into_element();
        let mut tree = RetainedTree::mount(root).unwrap();
        let layout = layout_pass(
            &mut tree,
            Viewport {
                width: 400.0,
                height: 400.0,
            },
        )
        .unwrap();

        let root = tree.root.as_ref().unwrap();
        let select = &root.children[0];
        let trigger = &select.children[0];
        let dropdown = &select.children[1];
        let trigger_rect = layout.get(trigger.taffy_id.unwrap()).unwrap();
        let dropdown_rect = layout.get(dropdown.taffy_id.unwrap()).unwrap();

        assert!(
            dropdown_rect.y >= trigger_rect.y + trigger_rect.height,
            "dropdown top ({}) must be at or below trigger bottom ({})",
            dropdown_rect.y,
            trigger_rect.y + trigger_rect.height
        );
        assert!(
            dropdown_rect.height > 0.0 && dropdown_rect.width > 0.0,
            "dropdown must have non-zero size for background paint, got {:?}",
            dropdown_rect
        );
    }

    #[test]
    fn mount_open_select_root_dropdown_has_nonzero_height() {
        use crate::RetainedTree;

        let value = State::new(None::<Color>);
        let open = State::new(true);
        let cx = Cx::new();
        let el = Select::with_open(&cx, value, open, make_options())
            .width(180.0)
            .into_element();
        let mut tree = RetainedTree::mount(el).unwrap();
        let layout = layout_pass(
            &mut tree,
            Viewport {
                width: 400.0,
                height: 400.0,
            },
        )
        .unwrap();
        let dropdown = &tree.root.as_ref().unwrap().children[1];
        let rect = layout.get(dropdown.taffy_id.unwrap()).unwrap();
        assert!(rect.height > 0.0, "mount-open dropdown height: {:?}", rect);
    }

    #[test]
    fn select_runtime_open_dropdown_has_layout_and_paint_after_diff() {
        use crate::diff::{diff, NodePath};
        use crate::RetainedTree;

        let value = State::new(None::<Color>);
        let open = State::new(false);
        let cx = Cx::new();
        let closed = Select::with_open(&cx, value.clone(), open.clone(), make_options())
            .width(180.0)
            .into_element();
        let mut tree = RetainedTree::mount(closed.clone()).unwrap();

        open.set(true);
        let opened = Select::with_open(&cx, value, open, make_options())
            .width(180.0)
            .into_element();
        let patches = diff(closed, opened, NodePath::root());
        tree.apply_patches(patches).unwrap();

        let layout = layout_pass(
            &mut tree,
            Viewport {
                width: 400.0,
                height: 400.0,
            },
        )
        .unwrap();
        let dropdown = &tree.root.as_ref().unwrap().children[1];
        let dropdown_rect = layout.get(dropdown.taffy_id.unwrap()).unwrap();
        assert!(
            dropdown_rect.height > 0.0,
            "dropdown layout height after diff: {:?}",
            dropdown_rect
        );
        assert!(dropdown.paint.background.is_some());
    }

    #[test]
    fn select_open_dropdown_retains_background_paint() {
        use crate::RetainedTree;

        let value = State::new(None::<Color>);
        let open = State::new(true);
        let cx = Cx::new();
        let root = Select::with_open(&cx, value, open, make_options())
            .width(180.0)
            .into_element();
        let mut tree = RetainedTree::mount(root).unwrap();
        layout_pass(
            &mut tree,
            Viewport {
                width: 400.0,
                height: 400.0,
            },
        )
        .unwrap();

        let dropdown = &tree.root.as_ref().unwrap().children[1];
        assert!(
            dropdown.paint.background.is_some(),
            "dropdown retained node must carry background paint"
        );
    }

    #[test]
    fn select_trigger_click_toggles_open_state() {
        let value = State::new(None::<Color>);
        let open = State::new(false);
        let cx = Cx::new();
        let el = Select::with_open(&cx, value, open.clone(), make_options()).into_element();

        let Element::Column(col) = el else {
            panic!("expected Column container");
        };
        let Element::View(trigger) = &col.children[0] else {
            panic!("expected View trigger");
        };
        let on_click = trigger
            .handlers
            .on_click
            .as_ref()
            .expect("trigger must have on_click");
        on_click();
        assert!(open.get(), "open signal should be true after trigger click");
    }

    #[test]
    fn select_option_click_updates_value_and_closes() {
        let value = State::new(None::<Color>);
        let open = State::new(true);
        let cx = Cx::new();
        let el = Select::with_open(&cx, value.clone(), open.clone(), make_options()).into_element();

        let Element::Column(col) = el else {
            panic!("expected Column container");
        };
        let Element::Column(dropdown) = &col.children[1] else {
            panic!("expected Column dropdown");
        };
        // Options are now View items; on_click lives in handlers.
        let Element::Row(opt_view) = &dropdown.children[1] else {
            panic!("expected View option");
        };
        let on_click = opt_view
            .handlers
            .on_click
            .as_ref()
            .expect("option must have on_click");
        on_click();

        assert_eq!(value.get(), Some(Color::Green));
        assert!(
            !open.get(),
            "dropdown should close after selecting an option"
        );
    }

    #[test]
    fn select_click_outside_closes_dropdown() {
        let value = State::new(None::<Color>);
        let open = State::new(true);
        let cx = Cx::new();
        let el = Select::with_open(&cx, value, open.clone(), make_options()).into_element();

        let Element::Column(col) = el else {
            panic!("expected Column container");
        };
        let on_click_outside = col
            .handlers
            .on_click_outside
            .as_ref()
            .expect("container must have on_click_outside");
        on_click_outside();
        assert!(!open.get(), "dropdown should close on outside click");
    }

    #[test]
    fn select_custom_placeholder_appears_in_trigger() {
        let value = State::new(None::<Color>);
        let open = State::new(false);
        let cx = Cx::new();
        let el = Select::with_open(&cx, value, open, make_options())
            .placeholder("Pick a color")
            .into_element();

        let Element::Column(col) = el else {
            panic!("expected Column container");
        };
        let Element::View(trigger) = &col.children[0] else {
            panic!("expected View trigger");
        };
        let Element::Row(content) = &trigger.children[0] else {
            panic!("trigger content")
        };
        let Element::Text(label) = &content.children[0] else {
            panic!("expected trigger Text");
        };
        let label = label.content.resolve();
        assert!(
            label.contains("Pick a color"),
            "trigger should show custom placeholder, got: {label}"
        );
    }

    #[test]
    fn select_defaults_follow_active_theme() {
        use crate::{
            current_theme,
            element::style::{CornerRadii, Edges},
            set_active_theme, Theme,
        };

        let previous = current_theme();
        let mut custom = Theme::default_dark();
        custom.colors.accent = crate::Color::rgb8(10, 20, 30);
        custom.colors.surface = crate::Color::rgb8(40, 50, 60);
        custom.colors.border = crate::Color::rgb8(70, 80, 90);
        custom.spacing.sm = 9.0;
        custom.radius.md = 11.0;
        set_active_theme(custom.clone());

        let cx = Cx::new();
        let value = State::new(None::<Color>);
        let open = State::new(true);
        let Element::Column(root) =
            Select::with_open(&cx, value, open, make_options()).into_element()
        else {
            panic!("expected Column");
        };

        let Element::View(trigger) = &root.children[0] else {
            panic!("expected trigger View");
        };
        assert_eq!(trigger.style.padding, Some(Edges::all(custom.spacing.sm)));
        let trigger_paint = trigger.paint.resolve();
        assert_eq!(trigger_paint.background, Some(custom.colors.surface));
        assert_eq!(trigger_paint.radius, CornerRadii::all(custom.radius.md));

        let Element::Column(dropdown) = &root.children[1] else {
            panic!("expected dropdown Column");
        };
        let dropdown_paint = dropdown.paint.resolve();
        assert_eq!(dropdown_paint.background, Some(custom.colors.surface));
        assert_eq!(
            dropdown_paint.border_color,
            crate::element::style::Edges::all(Some(custom.colors.border))
        );
        assert_eq!(dropdown_paint.radius, CornerRadii::all(custom.radius.md));

        let Element::Row(option) = &dropdown.children[0] else {
            panic!("expected option View");
        };
        assert_eq!(option.style.padding, Some(Edges::all(custom.spacing.sm)));

        set_active_theme(previous);
    }

    #[test]
    fn select_style_overrides_and_theme_fallbacks() {
        use crate::{
            current_theme,
            element::style::{CornerRadii, Edges},
            set_active_theme, Theme,
        };

        let previous = current_theme();
        let mut custom = Theme::default_dark();
        custom.colors.accent = crate::Color::rgb8(10, 20, 30);
        custom.colors.surface = crate::Color::rgb8(40, 50, 60);
        custom.colors.border = crate::Color::rgb8(70, 80, 90);
        custom.spacing.sm = 9.0;
        custom.radius.md = 11.0;
        set_active_theme(custom);

        let cx = Cx::new();
        let value = State::new(None::<Color>);
        let open = State::new(true);
        let Element::Column(root) = Select::with_open(&cx, value, open, make_options())
            .style(
                SelectStyle::default()
                    .trigger_surface(crate::Color::rgb8(1, 2, 3))
                    .dropdown_surface(crate::Color::rgb8(4, 5, 6))
                    .dropdown_border(crate::Color::rgb8(7, 8, 9)),
            )
            .into_element()
        else {
            panic!("expected Column");
        };

        let Element::View(trigger) = &root.children[0] else {
            panic!("expected trigger View");
        };
        assert_eq!(trigger.style.padding, Some(Edges::all(9.0)));
        let trigger_paint = trigger.paint.resolve();
        assert_eq!(trigger_paint.background, Some(crate::Color::rgb8(1, 2, 3)));
        assert_eq!(trigger_paint.radius, CornerRadii::all(11.0));

        let Element::Column(dropdown) = &root.children[1] else {
            panic!("expected dropdown Column");
        };
        let dropdown_paint = dropdown.paint.resolve();
        assert_eq!(dropdown_paint.background, Some(crate::Color::rgb8(4, 5, 6)));
        assert_eq!(
            dropdown_paint.border_color,
            crate::element::style::Edges::all(Some(crate::Color::rgb8(7, 8, 9)))
        );
        assert_eq!(dropdown_paint.radius, CornerRadii::all(11.0));

        set_active_theme(previous);
    }

    #[test]
    fn select_trigger_hover_and_press_update_background() {
        use crate::current_theme;

        let previous = current_theme();
        let mut custom = crate::Theme::default_dark();
        custom.colors.accent = crate::Color::rgb8(1, 2, 3);
        custom.colors.accent_hover = crate::Color::rgb8(4, 5, 6);
        custom.colors.accent_pressed = crate::Color::rgb8(7, 8, 9);
        crate::set_active_theme(custom.clone());

        let value = State::new(None::<Color>);
        let open = State::new(false);
        let cx = Cx::new();
        let Element::Column(root) =
            Select::with_open(&cx, value, open, make_options()).into_element()
        else {
            panic!("expected Column");
        };
        let Element::View(trigger) = &root.children[0] else {
            panic!("expected trigger View");
        };

        assert_eq!(
            trigger.paint.resolve().background,
            Some(custom.colors.surface)
        );
        trigger
            .handlers
            .on_hover_enter
            .as_ref()
            .expect("hover enter handler")();
        assert_eq!(
            trigger.paint.resolve().background,
            Some(custom.colors.surface_hover)
        );
        trigger
            .handlers
            .on_pointer_down
            .as_ref()
            .expect("pointer down handler")(0.5, 0.5);
        assert_eq!(
            trigger.paint.resolve().background,
            Some(custom.colors.surface_pressed)
        );
        trigger
            .handlers
            .on_pointer_up
            .as_ref()
            .expect("pointer up handler")(0.5, 0.5);
        assert_eq!(
            trigger.paint.resolve().background,
            Some(custom.colors.surface_hover)
        );

        crate::set_active_theme(previous);
    }

    #[test]
    fn disabled_select_has_no_interaction_handlers() {
        let value = State::new(None::<Color>);
        let open = State::new(false);
        let cx = Cx::new();
        let Element::Column(root) = Select::with_open(&cx, value, open.clone(), make_options())
            .disabled(true)
            .into_element()
        else {
            panic!("expected Column");
        };

        let Element::View(trigger) = &root.children[0] else {
            panic!("expected trigger View");
        };
        assert!(trigger.handlers.on_click.is_none());
        assert!(trigger.handlers.on_hover_enter.is_none());
        assert!(trigger.handlers.on_pointer_down.is_none());
        assert!(root.handlers.on_click_outside.is_none());

        assert!(!open.get(), "disabled select should not toggle open state");
    }
}
