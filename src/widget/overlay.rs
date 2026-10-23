use crate::prelude::*;

/// An anchored, non-modal panel controlled by app-owned visibility state.
/// Appears below the anchor, fades on open/close, and closes on an outside press
/// or Escape while focus is inside it. Keep mounted while closed for transitions.
/// Content can extend beyond ancestor clips. This is an in-window overlay.
///
/// ```
/// use lemon::prelude::*;
/// let cx = Cx::new();
/// let open = cx.state(false);
/// let toggle = open.clone();
/// let popover = Popover::new(&cx, open,
///     Button::new(&cx, "Options").action(move || toggle.update(|v| *v = !*v)),
///     Text::new("Panel content"));
/// ```
pub struct Popover {
    open: State<bool>,
    anchor: Element,
    content: Element,
    width: f32,
    theme: crate::Theme,
}

impl Popover {
    /// Places `content` below `anchor`; `open` owns visibility. No hook slots consumed.
    pub fn new(
        cx: &Cx,
        open: State<bool>,
        anchor: impl Into<Element>,
        content: impl Into<Element>,
    ) -> Self {
        Self {
            open,
            anchor: anchor.into(),
            content: content.into(),
            width: 240.0,
            theme: cx.theme(),
        }
    }

    /// Sets panel width in logical points (default: 240). The anchor keeps its own size.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Builds the anchor and panel. Decorate the content for custom interior styling.
    pub fn into_element(self) -> Element {
        let visible = self.open.get();
        let outside = self.open.clone();
        let escape = self.open;
        let mut root = ColumnView::new()
            .align(Align::Start)
            .on_click_outside(move || {
                if outside.get() {
                    outside.set(false);
                }
            })
            .child(self.anchor)
            .child(
                View::new().height(0.0).fill_width().child(
                    ColumnView::new()
                        .position(Position::Absolute)
                        .top(4.0)
                        .left(0.0)
                        .layer(20)
                        .blocks_input(true)
                        .width(self.width)
                        .inset(12.0)
                        .spacing(8.0)
                        .surface(self.theme.colors.surface)
                        .border(self.theme.colors.border, 1.0)
                        .rounding(self.theme.radius.md)
                        .transition(super::interaction_transition())
                        .opacity(if visible { 1.0 } else { 0.0 })
                        .interactive(visible)
                        .child(self.content),
                ),
            );
        if visible {
            root = root.on_escape(move || escape.set(false));
        }
        root.into_element()
    }
}

impl From<Popover> for Element {
    fn from(value: Popover) -> Element {
        value.into_element()
    }
}

/// A modal in-window dialog bound to visibility state.
/// Blocks background input, traps Tab, closes on Escape or backdrop click, and
/// restores focus on close. It fades while remaining mounted; closed content is inert.
/// This does not create a native OS window; use [`Cx::open_window`] for that.
///
/// ```
/// use lemon::prelude::*;
/// let cx = Cx::new();
/// let open = cx.state(false);
/// let close = open.clone();
/// let dialog = Dialog::new(&cx, open,
///     ColumnView::new().child(Text::new("Settings"))
///         .child(Button::new(&cx, "Done").action(move || close.set(false))));
/// ```
pub struct Dialog {
    open: State<bool>,
    content: Element,
    width: f32,
    dismiss_on_backdrop: bool,
    theme: crate::Theme,
}

impl Dialog {
    /// Creates a centered modal with app-owned visibility. No hook slots consumed.
    pub fn new(cx: &Cx, open: State<bool>, content: impl Into<Element>) -> Self {
        Self {
            open,
            content: content.into(),
            width: 400.0,
            dismiss_on_backdrop: true,
            theme: cx.theme(),
        }
    }

    /// Sets the preferred panel width in logical points (default: 400).
    /// The panel shrinks to fit the viewport's 24-point outer inset.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Whether clicking the backdrop closes the dialog (default: true).
    /// Escape always closes it; action buttons can also set visibility to false.
    pub fn dismiss_on_backdrop(mut self, enabled: bool) -> Self {
        self.dismiss_on_backdrop = enabled;
        self
    }

    /// Builds the modal. Keep this element mounted even when its state is false.
    pub fn into_element(self) -> Element {
        let visible = self.open.get();
        let backdrop = self.open.clone();
        let escape = self.open;
        let dismiss = self.dismiss_on_backdrop;
        let panel = ColumnView::new()
            .width(self.width)
            .min_width(0.0)
            .inset(20.0)
            .spacing(12.0)
            .surface(self.theme.colors.surface)
            .border(self.theme.colors.border, 1.0)
            .rounding(self.theme.radius.lg)
            .action(|| {})
            .child(self.content);
        let mut element = View::new()
            .position(Position::Fixed)
            .top(0.0)
            .left(0.0)
            .fill()
            .layer(100)
            .inset(24.0)
            .center()
            .focusable()
            .surface(Color::rgb8(0, 0, 0).with_alpha(0.4))
            .opacity(if visible { 1.0 } else { 0.0 })
            .interactive(visible)
            .transition(super::interaction_transition())
            .on_escape(move || escape.set(false))
            .action(move || {
                if dismiss {
                    backdrop.set(false);
                }
            })
            .child(panel)
            .into_element();
        if let Element::View(node) = &mut element {
            node.style.modal = true;
        }
        element
    }
}

impl From<Dialog> for Element {
    fn from(value: Dialog) -> Element {
        value.into_element()
    }
}
