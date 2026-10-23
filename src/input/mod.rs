//! Backend-independent focus, pointer capture, keyboard, hover and scrolling.
use crate::retained::TextCursorHandler;
pub mod hit_test;
mod observers;
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
    /// Pointer left the host window; releases hover without canceling drag capture.
    PointerLeave,
    /// ScrollView delta in logical points at `point` (positive moves content upward).
    Scroll {
        /// Position of the pointer.
        point: LogicalPoint,
        /// Native wheel movement on both axes.
        delta: crate::ScrollDelta,
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

type FocusObserver = (taffy::NodeId, Option<std::rc::Rc<dyn Fn(bool)>>);

pub(crate) struct InputState {
    focus_manager: FocusManager,
    observed_focus: Option<FocusObserver>,
    hovered_nodes: Vec<(taffy::NodeId, crate::retained::EventHandlers)>,
    pub(crate) pointer_focus: bool,
    hovered_node_id: Option<taffy::NodeId>,
    mouse_capture_node: Option<taffy::NodeId>,
    mouse_button_down: bool,
    last_pointer: Option<LogicalPoint>,
    reveal_caret: bool,
    text_dragging: bool,
    dragged: bool,
    drag_release: Option<LogicalPoint>,
    last_caret_activity: Instant,
    cursor: Cursor,
}
impl Default for InputState {
    fn default() -> Self {
        Self {
            focus_manager: FocusManager::new(),
            observed_focus: None,
            hovered_nodes: Vec::new(),
            pointer_focus: false,
            hovered_node_id: None,
            mouse_capture_node: None,
            mouse_button_down: false,
            last_pointer: None,
            reveal_caret: false,
            text_dragging: false,
            dragged: false,
            drag_release: None,
            last_caret_activity: Instant::now(),
            cursor: Cursor::Default,
        }
    }
}

impl UiState {
    pub(crate) fn reconcile_input(&mut self) {
        let root = self.retained.as_ref().and_then(|tree| tree.root.as_ref());
        if let Some(tree) = &self.retained {
            self.paint_dirty |= self.input.focus_manager.reconcile_modal(tree);
        }
        let exists = |id| root.and_then(|root| find_interactive_node_by_taffy_id(root, id));
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
            self.input.last_pointer = None;
        }
        if self
            .input
            .hovered_node_id
            .is_some_and(|id| exists(id).is_none())
        {
            self.input.hovered_node_id = None;
            self.input.cursor = Cursor::Default;
        }
        self.notify_focus();
        self.reconcile_hover_observers();
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
        match &event {
            InputEvent::PointerDown(_) | InputEvent::Click(_) => self.input.pointer_focus = true,
            InputEvent::Key(event) if event.state == KeyState::Pressed => {
                self.input.pointer_focus = false
            }
            _ => {}
        }
        let handled = match event {
            InputEvent::Click(point) => self.event_pass_click(point),
            InputEvent::PointerDown(point) => {
                self.input.mouse_button_down = true;
                self.event_pass_pointer_down(point)
            }
            InputEvent::PointerMove(point) => self.event_pass_pointer_move(point),
            InputEvent::PointerUp(point) => {
                if self.input.dragged {
                    self.input.drag_release = Some(point);
                }
                self.input.text_dragging = false;
                self.input.dragged = false;
                let handled = self.event_pass_pointer_up(point);
                self.input.mouse_button_down = false;
                self.input.mouse_capture_node = None;
                self.input.last_pointer = None;
                handled
            }
            InputEvent::Hover(point) => {
                self.event_pass_hover(point);
                false
            }
            InputEvent::PointerLeave => {
                self.clear_hover();
                false
            }
            InputEvent::Scroll { point, delta } => self.event_pass_scroll(point, delta),
            InputEvent::Key(event) => self.event_pass_keyboard(event),
        };
        self.notify_focus();
        self.apply_updates()?;
        Ok(InputOutcome {
            handled,
            cursor: self.input.cursor.clone(),
        })
    }

    pub(crate) fn clamp_scroll_offsets(&mut self) -> bool {
        fn collect(
            node: &crate::retained::RetainedNode,
            layout: &crate::LayoutMap,
            callbacks: &mut Vec<std::rc::Rc<dyn Fn(crate::ScrollDelta)>>,
        ) {
            if let (Some(axis), Some(inner), Some(maximum), Some(handler)) = (
                node.scroll_viewport,
                crate::layout::scroll_content_node(node),
                crate::layout::scroll_content_max_offset(node, layout),
                &node.handlers.on_scroll,
            ) {
                let margin = inner.style.margin.unwrap_or_default();
                if (axis.horizontal() && f64::from(-margin.left) > maximum.x + 0.01)
                    || (axis.vertical() && f64::from(-margin.top) > maximum.y + 0.01)
                {
                    callbacks.push(handler.clone());
                }
            }
            for child in &node.children {
                collect(child, layout, callbacks);
            }
        }
        let mut callbacks = Vec::new();
        if let Some(root) = self.retained.as_ref().and_then(|tree| tree.root.as_ref()) {
            collect(root, &self.layout_map, &mut callbacks);
        }
        for callback in &callbacks {
            callback(crate::ScrollDelta::default());
        }
        !callbacks.is_empty()
    }

    pub(crate) fn reveal_text_caret(&mut self) -> bool {
        if !std::mem::take(&mut self.input.reveal_caret) {
            return false;
        }
        let Some(id) = self.input.focus_manager.focused else {
            return false;
        };
        let Some(node) = self
            .retained
            .as_ref()
            .and_then(|tree| tree.root.as_ref())
            .and_then(|root| find_interactive_node_by_taffy_id(root, id))
        else {
            return false;
        };
        let Some(meta) = node.text_input.as_ref().filter(|meta| meta.multiline) else {
            return false;
        };
        let Some(viewport) = crate::layout::text_scroll_viewport(node) else {
            return false;
        };
        let Some(handler) = viewport.handlers.on_scroll.clone() else {
            return false;
        };
        let Some(bounds) = crate::layout::text_input_rect(node, &self.layout_map) else {
            return false;
        };
        let Some(text) = crate::layout::text_input_node(node) else {
            return false;
        };
        let Some(rect) = text.taffy_id.and_then(|id| self.layout_map.get(id)) else {
            return false;
        };
        let Some(cache) = &text.text else {
            return false;
        };
        let Some(layout) = &cache.parley_layout else {
            return false;
        };
        let caret = crate::layout::caret_geometry_in_layout(
            layout,
            &cache.content,
            meta.cursor,
            1.0,
            meta.affinity,
        );
        let top = rect.y + caret.y0 as f32;
        let bottom = rect.y + caret.y1 as f32;
        let amount = if top < bounds.y {
            bounds.y - top
        } else if bottom > bounds.y + bounds.height {
            bounds.y + bounds.height - bottom
        } else {
            0.0
        };
        if amount.abs() < 0.01 {
            return false;
        }
        handler(crate::ScrollDelta::new(0.0, amount as f64));
        true
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
        find_interactive_node_by_taffy_id(root, focused_id)
            .is_some_and(|node| node.text_input.is_some())
    }

    /// Invalidates paint for a focused editor so a host can animate the caret.
    pub fn tick_caret_blink(&mut self) {
        if self.focused_text_input() {
            self.paint_dirty = true;
        }
    }

    /// Route a pointer click through hit-test and dispatch `on_click` (logical coordinates).
    fn event_pass_click(&mut self, point: LogicalPoint) -> bool {
        if self.input.drag_release.take().is_some_and(|release| {
            (release.x - point.x).abs() < 0.1 && (release.y - point.y).abs() < 0.1
        }) {
            return true;
        }
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

        if let Some(node) = focused.and_then(|id| find_interactive_node_by_taffy_id(root, id)) {
            if let Some((handler, index, affinity)) =
                text_cursor_at_point(node, &self.layout_map, point, false)
            {
                handler(index, affinity, false);
                self.input.reveal_caret = true;
            }
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
        self.input.text_dragging = false;
        self.input.dragged = false;
        self.input.drag_release = None;
        let Some(root) = self.retained.as_ref().and_then(|t| t.root.as_ref()) else {
            return false;
        };

        dispatch_outside_clicks(root, &self.layout_map, point);

        let hit = hit_test_pointer_down(root, &self.layout_map, point);
        let Some((node, (nx, ny))) = hit else {
            self.input.mouse_capture_node = None;
            self.input.last_pointer = None;

            return false;
        };

        let capture_id = node.taffy_id;
        if node.handlers.on_drag.is_some() {
            self.input.focus_manager.focused =
                hit_test_focusable(root, &self.layout_map, point).and_then(|node| node.taffy_id);
            self.input.last_caret_activity = Instant::now();
            self.paint_dirty = true;
        }
        if let Some((handler, index, affinity)) =
            text_cursor_at_point(node, &self.layout_map, point, false)
        {
            handler(index, affinity, false);
            self.input.text_dragging = true;
            self.input.focus_manager.focused = capture_id;
            self.input.reveal_caret = true;
            self.paint_dirty = true;
        }
        if let Some(handler) = node.handlers.on_pointer_down.clone() {
            handler(nx, ny);
        }
        self.input.mouse_capture_node = capture_id;
        self.input.last_pointer = Some(point);

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
        let Some(node) = self
            .retained
            .as_ref()
            .and_then(|tree| tree.root.as_ref())
            .and_then(|root| find_interactive_node_by_taffy_id(root, capture_id))
        else {
            return false;
        };
        let drag = node.handlers.on_drag.clone();
        let movement = node.handlers.on_pointer_move.clone();
        let rect = self.layout_map.get(capture_id).copied();
        let previous = self.input.last_pointer.replace(point).unwrap_or(point);
        let mut handled = false;
        if self.input.text_dragging {
            if let Some((handler, index, affinity)) =
                text_cursor_at_point(node, &self.layout_map, point, true)
            {
                handler(index, affinity, true);
                self.input.dragged |= point != previous;
                self.input.reveal_caret = true;
                self.paint_dirty = true;
                handled = true;
            }
        }
        if let Some(drag) = drag {
            self.input.dragged |= point != previous;
            drag(point.x - previous.x, point.y - previous.y);
            handled = true;
        }
        if let (Some(movement), Some(rect)) = (movement, rect) {
            let (x, y) = normalize_coords(point, &rect);
            movement(x, y);
            handled = true;
        }
        handled
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
            .and_then(|root| find_interactive_node_by_taffy_id(root, capture_id))
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
        let state = event.state.clone();

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

        // Escape is handled by the nearest containing scope before its focused child.
        if state == KeyState::Pressed
            && !event.repeat
            && lemon_key == LemonKey::Named(NamedKey::Escape)
        {
            fn escape_handler(
                node: &crate::retained::RetainedNode,
                focused: Option<taffy::NodeId>,
            ) -> Option<std::rc::Rc<dyn Fn()>> {
                if !node.style.interactive {
                    return None;
                }
                if let Some(child) = node
                    .children
                    .iter()
                    .find_map(|child| escape_handler(child, focused))
                {
                    return Some(child);
                }
                let contains = focused.is_some_and(|id| {
                    crate::input::hit_test::find_node_by_taffy_id(node, id).is_some()
                });
                if contains || node.style.modal {
                    node.handlers.on_escape.clone()
                } else {
                    None
                }
            }
            if let Some(handler) = self
                .retained
                .as_ref()
                .and_then(|tree| tree.root.as_ref())
                .and_then(|root| {
                    escape_handler(
                        crate::retained::focus::active_modal(root).unwrap_or(root),
                        self.input.focus_manager.focused,
                    )
                })
            {
                handler();
                self.paint_dirty = true;
                return true;
            }
        }

        let Some(focused_id) = self.input.focus_manager.focused else {
            return false;
        };

        if let Some(node) = self
            .retained
            .as_ref()
            .and_then(|tree| tree.root.as_ref())
            .and_then(|root| find_interactive_node_by_taffy_id(root, focused_id))
        {
            if state == KeyState::Pressed && node.text_input.is_some() {
                self.input.reveal_caret = true;
                if let Some((index, affinity)) = text_cursor_for_key(node, &event) {
                    if let Some(handler) = node.handlers.on_text_cursor.clone() {
                        handler(index, affinity, event.modifiers.shift);
                        self.input.last_caret_activity = Instant::now();
                        self.paint_dirty = true;
                        return true;
                    }
                }
            }
        }
        let handler = self
            .retained
            .as_ref()
            .and_then(|tree| tree.root.as_ref())
            .and_then(|root| find_interactive_node_by_taffy_id(root, focused_id))
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

    /// Route a scroll event through hit-test and dispatch `on_scroll` (logical coordinates).
    fn event_pass_scroll(&mut self, point: LogicalPoint, delta: crate::ScrollDelta) -> bool {
        let Some(root) = self.retained.as_ref().and_then(|t| t.root.as_ref()) else {
            return false;
        };
        let Some(node) = hit_test_scroll(root, &self.layout_map, point, delta) else {
            return false;
        };
        if let Some(handler) = node.handlers.on_scroll.clone() {
            handler(delta);

            true
        } else {
            false
        }
    }
}

fn text_cursor_at_point(
    node: &crate::retained::RetainedNode,
    layout: &crate::LayoutMap,
    point: LogicalPoint,
    captured: bool,
) -> Option<(
    TextCursorHandler,
    usize,
    crate::element::types::CaretAffinity,
)> {
    let handler = node.handlers.on_text_cursor.clone()?;
    let meta = node.text_input.as_ref()?;
    let content_rect = crate::layout::text_input_rect(node, layout)?;
    if !captured
        && (point.x < content_rect.x
            || point.x > content_rect.x + content_rect.width
            || point.y < content_rect.y
            || point.y > content_rect.y + content_rect.height)
    {
        return None;
    }
    if meta.value.is_empty() {
        return Some((handler, 0, Default::default()));
    }
    let text = crate::layout::text_input_node(node)?;
    let rect = layout.get(text.taffy_id?)?;
    let parley = text.text.as_ref()?.parley_layout.as_ref()?;
    let cursor = parley::Cursor::from_point(parley, point.x - rect.x, point.y - rect.y);
    Some((handler, cursor.index(), cursor_affinity(cursor)))
}

fn text_cursor_for_key(
    node: &crate::retained::RetainedNode,
    event: &KeyEvent,
) -> Option<(usize, crate::element::types::CaretAffinity)> {
    let meta = node.text_input.as_ref()?;
    if !meta.multiline || event.modifiers.ctrl || event.modifiers.meta || event.modifiers.alt {
        return None;
    }
    let text = crate::layout::text_input_node(node)?.text.as_ref()?;
    if text.content != meta.value {
        return None;
    }
    let layout = text.parley_layout.as_ref()?;
    let selection = parley::Selection::from_byte_index(
        layout,
        meta.cursor,
        if meta.cursor >= meta.value.len()
            || meta.affinity == crate::element::types::CaretAffinity::Upstream
        {
            parley::Affinity::Upstream
        } else {
            parley::Affinity::Downstream
        },
    );
    let selection = if let Some(range) = &meta.selection {
        let anchor = if range.start == meta.cursor {
            range.end
        } else {
            range.start
        };
        parley::Selection::new(
            parley::Cursor::from_byte_index(layout, anchor, parley::Affinity::Downstream),
            selection.focus(),
        )
    } else {
        selection
    };
    let selection = match event.key {
        LemonKey::Named(NamedKey::ArrowLeft) => {
            selection.previous_visual(layout, event.modifiers.shift)
        }
        LemonKey::Named(NamedKey::ArrowRight) => {
            selection.next_visual(layout, event.modifiers.shift)
        }
        LemonKey::Named(NamedKey::ArrowUp) => selection.previous_line(layout, false),
        LemonKey::Named(NamedKey::ArrowDown) => selection.next_line(layout, false),
        LemonKey::Named(NamedKey::Home) => selection.line_start(layout, false),
        LemonKey::Named(NamedKey::End) => selection.line_end(layout, false),
        _ => return None,
    };
    Some((
        selection.focus().index(),
        cursor_affinity(selection.focus()),
    ))
}

fn cursor_affinity(cursor: parley::Cursor) -> crate::element::types::CaretAffinity {
    match cursor.affinity() {
        parley::Affinity::Upstream => crate::element::types::CaretAffinity::Upstream,
        parley::Affinity::Downstream => crate::element::types::CaretAffinity::Downstream,
    }
}
