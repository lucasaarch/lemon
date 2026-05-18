use lemon::{
    element::{
        builders::{Button, Column, Text, View},
        events::Cursor,
        style::Color,
        Element,
    },
    Cx, Signal,
};

const TRIGGER_HEIGHT: f32 = 40.0;

/// Dropdown select widget backed by user-owned signal state.
///
/// `Select<T>` renders a trigger button that opens a dropdown list of choices. Selecting an option
/// updates the value signal and closes the dropdown; clicking outside the widget also closes it.
///
/// The dropdown list is rendered as an **overlay** — it is absolutely positioned so it floats
/// above surrounding content and does not shift sibling elements when opened.
///
/// The open/closed state is internal by default ([`Select::new`]) or caller-supplied for advanced
/// use ([`Select::with_open`]).
///
/// # Examples
///
/// ```no_run
/// use lemon::{Cx, element::Element};
/// use lemon_widgets::Select;
///
/// #[derive(Clone, PartialEq)]
/// enum Size { Small, Medium, Large }
///
/// fn my_view(cx: &Cx) -> Element {
///     let size = cx.use_signal(None::<Size>);
///     Select::new(
///         cx,
///         size,
///         vec![
///             (Size::Small, "Small".to_string()),
///             (Size::Medium, "Medium".to_string()),
///             (Size::Large, "Large".to_string()),
///         ],
///     )
///     .placeholder("Choose a size")
///     .width(180.0)
///     .into_element()
/// }
/// ```
pub struct Select<T> {
    value: Signal<Option<T>>,
    open: Signal<bool>,
    options: Vec<(T, String)>,
    width: Option<f32>,
    placeholder: String,
}

impl<T: Clone + PartialEq + 'static> Select<T> {
    /// Creates a `Select` with internally managed open/closed state.
    ///
    /// - `value` — caller-owned signal updated with the chosen option when the user selects one.
    /// - `options` — ordered list of `(value, label)` pairs shown as choices.
    pub fn new(cx: &Cx, value: Signal<Option<T>>, options: Vec<(T, String)>) -> Self {
        Self::with_open(value, cx.use_signal(false), options)
    }

    /// Creates a `Select` with a caller-supplied open signal.
    ///
    /// Useful when you need to control or observe the open state externally (e.g. in tests).
    pub fn with_open(
        value: Signal<Option<T>>,
        open: Signal<bool>,
        options: Vec<(T, String)>,
    ) -> Self {
        Self {
            value,
            open,
            options,
            width: None,
            placeholder: "Select…".to_string(),
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

    /// Builds and returns the [`Element`] for this select widget.
    ///
    /// Call this at the end of the builder chain to insert the widget into a parent container.
    pub fn into_element(self) -> Element {
        let is_open = self.open.get();
        let current = self.value.get();
        let open = self.open;
        let value = self.value;
        let options = self.options;
        let width = self.width;
        let placeholder = self.placeholder;

        // Derive trigger label from the current selection, falling back to placeholder.
        let trigger_label = current
            .as_ref()
            .and_then(|v| {
                options
                    .iter()
                    .find(|(opt, _)| opt == v)
                    .map(|(_, l)| l.clone())
            })
            .unwrap_or(placeholder);

        // Trigger button: toggles the dropdown open/closed on click.
        let open_for_trigger = open.clone();
        let mut trigger = Button::new(format!("{trigger_label} ▾"))
            .height(TRIGGER_HEIGHT)
            .on_click(move || {
                open_for_trigger.update(|b| *b = !*b);
            });
        if let Some(w) = width {
            trigger = trigger.width(w);
        }

        // Outer container: on_click_outside closes the dropdown when the user clicks elsewhere.
        let open_for_outside = open.clone();
        let mut container = Column::new().on_click_outside(move || open_for_outside.set(false));
        if let Some(w) = width {
            container = container.width(w);
        }
        container = container.child(trigger);

        if is_open {
            // Dropdown list: absolutely positioned so it overlays content without shifting
            // siblings, and z_index=10 so it paints above them.
            let mut dropdown = Column::new()
                .z_index(10)
                .absolute()
                .top(TRIGGER_HEIGHT)
                .left(0.0)
                .background(Color::rgb8(35, 35, 52))
                .border(Color::rgb8(80, 80, 110), 1.0)
                .radius(6.0);
            if let Some(w) = width {
                dropdown = dropdown.width(w);
            }

            for (opt, label) in options {
                let open_c = open.clone();
                let val_c = value.clone();
                let mut item = View::new()
                    .padding(8.0)
                    .cursor(Cursor::Pointer)
                    .on_click(move || {
                        val_c.set(Some(opt.clone()));
                        open_c.set(false);
                    })
                    .child(Text::new(label).font_size(14.0));
                if let Some(w) = width {
                    item = item.width(w);
                }
                dropdown = dropdown.child(item);
            }
            container = container.child(dropdown);
        }

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
    use lemon::element::Element;

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
        let value = Signal::new(None::<Color>);
        let open = Signal::new(false);
        let el = Select::with_open(value, open, make_options()).into_element();

        let Element::Column(col) = el else {
            panic!("expected Column container");
        };
        // Closed: only the trigger child
        assert_eq!(col.children.len(), 1);
        let Element::Button(trigger) = &col.children[0] else {
            panic!("expected Button trigger");
        };
        let label = trigger.label.resolve();
        assert!(
            label.contains("Select"),
            "trigger should contain placeholder, got: {label}"
        );
    }

    #[test]
    fn select_closed_shows_selected_label_in_trigger() {
        let value = Signal::new(Some(Color::Green));
        let open = Signal::new(false);
        let el = Select::with_open(value, open, make_options()).into_element();

        let Element::Column(col) = el else {
            panic!("expected Column container");
        };
        let Element::Button(trigger) = &col.children[0] else {
            panic!("expected Button trigger");
        };
        let label = trigger.label.resolve();
        assert!(
            label.contains("Green"),
            "trigger should show selected label, got: {label}"
        );
    }

    #[test]
    fn select_open_shows_options_as_children() {
        let value = Signal::new(None::<Color>);
        let open = Signal::new(true);
        let el = Select::with_open(value, open, make_options()).into_element();

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
            dropdown.style.position_absolute,
            "dropdown must be absolutely positioned to overlay content"
        );
    }

    #[test]
    fn select_open_dropdown_lays_out_below_trigger() {
        use lemon::{layout_pass, RetainedTree, Viewport};

        let value = Signal::new(None::<Color>);
        let open = Signal::new(true);
        let root = Column::new()
            .child(
                Select::with_open(value, open, make_options())
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
    }

    #[test]
    fn select_trigger_click_toggles_open_state() {
        let value = Signal::new(None::<Color>);
        let open = Signal::new(false);
        let el = Select::with_open(value, open.clone(), make_options()).into_element();

        let Element::Column(col) = el else {
            panic!("expected Column container");
        };
        let Element::Button(trigger) = &col.children[0] else {
            panic!("expected Button trigger");
        };
        let on_click = trigger
            .on_click
            .as_ref()
            .expect("trigger must have on_click");
        on_click();
        assert!(open.get(), "open signal should be true after trigger click");
    }

    #[test]
    fn select_option_click_updates_value_and_closes() {
        let value = Signal::new(None::<Color>);
        let open = Signal::new(true);
        let el = Select::with_open(value.clone(), open.clone(), make_options()).into_element();

        let Element::Column(col) = el else {
            panic!("expected Column container");
        };
        let Element::Column(dropdown) = &col.children[1] else {
            panic!("expected Column dropdown");
        };
        // Options are now View items; on_click lives in handlers.
        let Element::View(opt_view) = &dropdown.children[1] else {
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
        let value = Signal::new(None::<Color>);
        let open = Signal::new(true);
        let el = Select::with_open(value, open.clone(), make_options()).into_element();

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
        let value = Signal::new(None::<Color>);
        let open = Signal::new(false);
        let el = Select::with_open(value, open, make_options())
            .placeholder("Pick a color")
            .into_element();

        let Element::Column(col) = el else {
            panic!("expected Column container");
        };
        let Element::Button(trigger) = &col.children[0] else {
            panic!("expected Button trigger");
        };
        let label = trigger.label.resolve();
        assert!(
            label.contains("Pick a color"),
            "trigger should show custom placeholder, got: {label}"
        );
    }
}
