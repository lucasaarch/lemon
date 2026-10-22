//! Backend-independent focus, pointer capture, keyboard, hover and scrolling.
pub mod hit_test;
use crate::{
    retained::focus::FocusManager, retained::RetainedError, ui::UiState, Cursor, KeyEvent,
    KeyState, LemonKey, NamedKey,
};
use hit_test::*;
use std::time::Instant;

/// An input event in logical layout coordinates, independent of native windowing.
#[derive(Clone, Debug)]
pub enum InputEvent {
    /// Left pointer press; begins pointer capture when a handler is found.
    PointerDown(LogicalPoint),
    /// Pointer movement, delivered to the captured node while pressed.
    PointerMove(LogicalPoint),
    /// Left pointer release; always ends capture.
    PointerUp(LogicalPoint),
    /// Activation at a point; focuses the hit control or clears focus when none is eligible.
    Click(LogicalPoint),
    /// Updates hover handlers and the requested cursor.
    Hover(LogicalPoint),
    /// Scroll delta in logical points at `point` (positive moves content upward).
    Scroll {
        /// Position of the pointer.
        point: LogicalPoint,
        /// Vertical wheel delta.
        delta_y: f64,
    },
    /// Keyboard input, including modifiers and repeat state.
    Key(KeyEvent),
}

/// Result of dispatching input through the headless UI pipeline.
#[derive(Clone, Debug)]
pub struct InputOutcome {
    /// Whether a handler or focus traversal consumed the event.
    pub handled: bool,
    /// Cursor the native host should display.
    pub cursor: Cursor,
}

pub(crate) struct InputState {
    focus_manager: FocusManager,
    hovered_node_id: Option<taffy::NodeId>,
    mouse_capture_node: Option<taffy::NodeId>,
    mouse_button_down: bool,
    last_caret_activity: Instant,
    cursor: Cursor,
}
impl Default for InputState {
    fn default() -> Self {
        Self {
            focus_manager: FocusManager::new(),
            hovered_node_id: None,
            mouse_capture_node: None,
            mouse_button_down: false,
            last_caret_activity: Instant::now(),
            cursor: Cursor::Default,
        }
    }
}

impl UiState {
    pub(crate) fn reconcile_input(&mut self) {
        let root = self.retained.as_ref().and_then(|tree| tree.root.as_ref());
        let exists = |id| root.and_then(|root| find_node_by_taffy_id(root, id));
        if self
            .input
            .focus_manager
            .focused
            .is_some_and(|id| exists(id).is_none_or(|node| !node.style.focusable))
        {
            self.input.focus_manager.focused = None;
            self.paint_dirty = true;
        }
        if self
            .input
            .mouse_capture_node
            .is_some_and(|id| exists(id).is_none())
        {
            self.input.mouse_capture_node = None;
        }
        if self
            .input
            .hovered_node_id
            .is_some_and(|id| exists(id).is_none())
        {
            self.input.hovered_node_id = None;
            self.input.cursor = Cursor::Default;
        }
    }

    /// Returns the focused node, if any; focus is managed by input dispatch.
    pub fn focused(&self) -> Option<taffy::NodeId> {
        self.input.focus_manager.focused
    }

    /// Dispatches input against current geometry and applies resulting reactive updates.
    ///
    /// Call `layout` or `update` once before sending input. Dirty geometry is refreshed
    /// using that viewport before hit-testing. Errors preserve dirty flags for retry.
    /// The desktop host only translates native events and applies the returned cursor.
    ///
    /// ```
    /// use lemon::{Text, Viewport, ui::UiState, input::InputEvent, KeyEvent, KeyState, LemonKey, NamedKey, Modifiers};
    /// let mut ui = UiState::default();
    /// ui.mount(|_| Text::new("Hello").into_element()).unwrap();
    /// ui.layout(Viewport { width: 400.0, height: 300.0 }).unwrap();
    /// ui.dispatch_input(InputEvent::Key(KeyEvent {
    ///     key: LemonKey::Named(NamedKey::Tab), modifiers: Modifiers::default(), repeat: false, state: KeyState::Pressed,
    /// })).unwrap();
    /// ```
    pub fn dispatch_input(&mut self, event: InputEvent) -> Result<InputOutcome, RetainedError> {
        let _scope = self.host.enter();
        let viewport = self.viewport.ok_or(RetainedError::UnsupportedElement(
            "input requires an initial layout",
        ))?;
        self.apply_updates()?;
        if self.layout_dirty {
            self.layout(viewport)?;
        }
        let handled = match event {
            InputEvent::Click(point) => self.event_pass_click(point),
            InputEvent::PointerDown(point) => {
                self.input.mouse_button_down = true;
                self.event_pass_pointer_down(point)
            }
            InputEvent::PointerMove(point) => self.event_pass_pointer_move(point),
            InputEvent::PointerUp(point) => {
                let handled = self.event_pass_pointer_up(point);
                self.input.mouse_button_down = false;
                self.input.mouse_capture_node = None;
                handled
            }
            InputEvent::Hover(point) => {
                self.event_pass_hover(point);
                false
            }
            InputEvent::Scroll { point, delta_y } => self.event_pass_scroll(point, delta_y),
            InputEvent::Key(event) => self.event_pass_keyboard(event),
        };
        self.apply_updates()?;
        Ok(InputOutcome {
            handled,
            cursor: self.input.cursor.clone(),
        })
    }

    /// Returns the current text caret blink phase.
    pub fn caret_visible_now(&self) -> bool {
        let elapsed = self.input.last_caret_activity.elapsed().as_millis();
        if elapsed < 150 {
            return true;
        }
        (elapsed / 530).is_multiple_of(2)
    }

    /// Returns whether the focused node is a text editor.
    pub fn focused_text_input(&self) -> bool {
        let Some(focused_id) = self.input.focus_manager.focused else {
            return false;
        };
        let Some(root) = self.retained.as_ref().and_then(|t| t.root.as_ref()) else {
            return false;
        };
        find_node_by_taffy_id(root, focused_id).is_some_and(|node| node.text_input.is_some())
    }

    /// Invalidates paint for a focused editor so a host can animate the caret.
    pub fn tick_caret_blink(&mut self) {
        if self.focused_text_input() {
            self.paint_dirty = true;
        }
    }

    /// Route a pointer click through hit-test and dispatch `on_click` (logical coordinates).
    fn event_pass_click(&mut self, point: LogicalPoint) -> bool {
        let Some(tree) = self.retained.as_ref() else {
            return false;
        };
        let Some(root) = tree.root.as_ref() else {
            return false;
        };

        let focused =
            hit_test_focusable(root, &self.layout_map, point).and_then(|node| node.taffy_id);
        let focus_changed = self.input.focus_manager.focused != focused;
        self.input.focus_manager.focused = focused;
        if focused.is_some() {
            self.input.last_caret_activity = Instant::now();
        }
        if focus_changed || focused.is_some() {
            self.paint_dirty = true;
        }

        let Some(node) = hit_test_on_click(root, &self.layout_map, point) else {
            return focus_changed;
        };
        let handled = dispatch_click(node);
        crate::lemon_trace!(
            Input,
            "click at ({:.1},{:.1}) taffy_id={:?} handled={handled}",
            point.x,
            point.y,
            node.taffy_id
        );
        handled || focus_changed
    }

    /// Dispatch `on_pointer_down` to the deepest node under `point` with that handler, set
    /// mouse capture, and dispatch `on_click_outside` to all qualifying nodes.
    fn event_pass_pointer_down(&mut self, point: LogicalPoint) -> bool {
        let Some(root) = self.retained.as_ref().and_then(|t| t.root.as_ref()) else {
            return false;
        };

        dispatch_outside_clicks(root, &self.layout_map, point);

        let hit = hit_test_pointer_down(root, &self.layout_map, point);
        let Some((node, (nx, ny))) = hit else {
            self.input.mouse_capture_node = None;

            return false;
        };

        let capture_id = node.taffy_id;
        if let Some(handler) = node.handlers.on_pointer_down.clone() {
            handler(nx, ny);
        }
        self.input.mouse_capture_node = capture_id;

        true
    }

    /// Dispatch `on_pointer_move` to the currently captured node (if any) with normalized coords.
    fn event_pass_pointer_move(&mut self, point: LogicalPoint) -> bool {
        if !self.input.mouse_button_down {
            return false;
        }
        let Some(capture_id) = self.input.mouse_capture_node else {
            return false;
        };

        let (handler, rect) = match self
            .retained
            .as_ref()
            .and_then(|t| t.root.as_ref())
            .and_then(|root| find_node_by_taffy_id(root, capture_id))
        {
            Some(node) => {
                let h = node.handlers.on_pointer_move.clone();
                let r = node
                    .taffy_id
                    .and_then(|id| self.layout_map.get(id))
                    .copied();
                (h, r)
            }
            None => return false,
        };

        if let (Some(handler), Some(rect)) = (handler, rect) {
            let (nx, ny) = normalize_coords(point, &rect);
            handler(nx, ny);

            return true;
        }
        false
    }

    /// Dispatch `on_pointer_up` to the currently captured node (if any) with normalized coords.
    fn event_pass_pointer_up(&mut self, point: LogicalPoint) -> bool {
        let Some(capture_id) = self.input.mouse_capture_node else {
            return false;
        };

        let (handler, rect) = match self
            .retained
            .as_ref()
            .and_then(|t| t.root.as_ref())
            .and_then(|root| find_node_by_taffy_id(root, capture_id))
        {
            Some(node) => {
                let h = node.handlers.on_pointer_up.clone();
                let r = node
                    .taffy_id
                    .and_then(|id| self.layout_map.get(id))
                    .copied();
                (h, r)
            }
            None => return false,
        };

        if let (Some(handler), Some(rect)) = (handler, rect) {
            let (nx, ny) = normalize_coords(point, &rect);
            handler(nx, ny);

            return true;
        }
        false
    }

    fn event_pass_keyboard(&mut self, event: KeyEvent) -> bool {
        let lemon_key = event.key.clone();
        let state = event.state;

        if state == KeyState::Pressed
            && !event.repeat
            && lemon_key == LemonKey::Named(NamedKey::Tab)
        {
            if let Some(tree) = self.retained.as_ref() {
                self.input.focus_manager.cycle(tree, !event.modifiers.shift);
            }
            self.input.last_caret_activity = Instant::now();
            self.paint_dirty = true;
            return self.input.focus_manager.focused.is_some();
        }

        let Some(focused_id) = self.input.focus_manager.focused else {
            return false;
        };

        let handler = self
            .retained
            .as_ref()
            .and_then(|tree| tree.root.as_ref())
            .and_then(|root| find_node_by_taffy_id(root, focused_id))
            .and_then(|node| match state {
                KeyState::Pressed => node.handlers.on_key_down.clone(),
                KeyState::Released => node.handlers.on_key_up.clone(),
            });

        let Some(handler) = handler else {
            return false;
        };

        handler(KeyEvent {
            key: lemon_key,
            modifiers: event.modifiers,
            repeat: event.repeat,
            state,
        });
        self.input.last_caret_activity = Instant::now();
        self.paint_dirty = true;
        true
    }

    fn event_pass_hover(&mut self, point: LogicalPoint) {
        let new_id = self
            .retained
            .as_ref()
            .and_then(|tree| tree.root.as_ref())
            .and_then(|root| hit_test_hover(root, &self.layout_map, point))
            .and_then(|node| node.taffy_id);

        if new_id == self.input.hovered_node_id {
            self.input.cursor = new_id
                .and_then(|id| {
                    self.retained
                        .as_ref()?
                        .root
                        .as_ref()
                        .and_then(|root| find_node_by_taffy_id(root, id))
                })
                .map(|node| node.style.cursor.clone())
                .unwrap_or(Cursor::Default);
            return;
        }

        let old_leave = self.input.hovered_node_id.and_then(|old_id| {
            self.retained
                .as_ref()
                .and_then(|tree| tree.root.as_ref())
                .and_then(|root| find_node_by_taffy_id(root, old_id))
                .and_then(|node| node.handlers.on_hover_leave.clone())
        });

        let (new_enter, new_cursor) = if let Some(id) = new_id {
            self.retained
                .as_ref()
                .and_then(|tree| tree.root.as_ref())
                .and_then(|root| find_node_by_taffy_id(root, id))
                .map(|node| {
                    (
                        node.handlers.on_hover_enter.clone(),
                        node.style.cursor.clone(),
                    )
                })
                .unwrap_or((None, Cursor::Default))
        } else {
            (None, Cursor::Default)
        };

        if let Some(handler) = old_leave {
            handler();
        }
        if let Some(handler) = new_enter {
            handler();
        }
        self.input.cursor = new_cursor;

        self.input.hovered_node_id = new_id;
        self.paint_dirty = true;
    }

    /// Route a scroll event through hit-test and dispatch `on_scroll` (logical coordinates).
    fn event_pass_scroll(&mut self, point: LogicalPoint, delta_y: f64) -> bool {
        let Some(root) = self.retained.as_ref().and_then(|t| t.root.as_ref()) else {
            return false;
        };
        let Some(node) = hit_test_scroll(root, &self.layout_map, point) else {
            return false;
        };
        if let Some(handler) = node.handlers.on_scroll.clone() {
            handler(delta_y);

            true
        } else {
            false
        }
    }
}
