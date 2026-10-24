//! Headless processing of a window's UI, independent of native presentation.

use crate::paint::Scene;
use parley::FontContext;
use taffy::NodeId;

use crate::element::Element;
use crate::retained::RetainedError;
use crate::{layout_pass, sync_scroll_layout_max, Cx, LayoutMap, RetainedTree, Runtime, Viewport};

/// UI processing state shared by desktop hosts and headless tests.
///
/// Hosts supply viewport changes, translated input events, frame scheduling, and presentation.
/// Apply reactive updates before layout, then rebuild the scene only when dirty.
///
/// ```
/// use lemon::{Text, Viewport, ui::UiState};
/// let mut ui = UiState::default();
/// ui.mount(|_| Text::new("Hello").into_element()).unwrap();
/// ui.update(Viewport { width: 400.0, height: 300.0 }, 1.0, None, true).unwrap();
/// assert!(!ui.layout_dirty());
/// assert!(!ui.paint_dirty());
/// ```
///
/// Internals are read-only; update through this pipeline to preserve caches.
/// ```compile_fail
/// let mut ui = lemon::ui::UiState::default();
/// ui.layout_dirty = false;
/// ```
pub struct UiState {
    #[cfg(debug_assertions)]
    pub(crate) inspection: Option<crate::devtools::Capture>,
    pub(crate) transitions: crate::transition::Transitions,
    pub(crate) transitions_active: bool,
    pub(crate) input: crate::input::InputState,
    pub(crate) host: crate::host::HostContext,
    pub(crate) viewport: Option<Viewport>,
    last_paint: Option<(f32, Option<NodeId>, bool)>,
    /// Reactive component runtime.
    pub(crate) runtime: Runtime,
    /// Retained state, absent until mounted.
    pub(crate) retained: Option<RetainedTree>,
    /// Latest layout geometry in logical points.
    pub(crate) layout_map: LayoutMap,
    /// Registered fonts and text measurement context.
    pub(crate) font_cx: FontContext,
    /// Scene buffer rebuilt after visual changes.
    pub(crate) scene: Scene,
    /// Host or tree geometry changes require layout before painting.
    pub(crate) layout_dirty: bool,
    scroll_limits_dirty: bool,
    /// Visual changes require rebuilding the scene before presentation.
    pub(crate) paint_dirty: bool,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            #[cfg(debug_assertions)]
            inspection: None,
            transitions: crate::transition::Transitions::default(),
            transitions_active: false,
            input: crate::input::InputState::default(),
            host: crate::host::HostContext::current(),
            viewport: None,
            last_paint: None,
            runtime: Runtime::new(),
            retained: None,
            layout_map: LayoutMap::default(),
            font_cx: FontContext::new(),
            scene: Scene::new(),
            layout_dirty: true,
            scroll_limits_dirty: false,
            paint_dirty: true,
        }
    }
}

impl UiState {
    /// Returns the component runtime for read-only inspection.
    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    /// Returns retained state after mounting; mutations belong to this pipeline.
    pub fn retained(&self) -> Option<&RetainedTree> {
        self.retained.as_ref()
    }

    /// Returns the latest computed layout rectangles.
    pub fn layout_map(&self) -> &LayoutMap {
        &self.layout_map
    }

    /// Returns the scene most recently built by `update`; render with [`crate::Renderer`]
    /// so subtree blur and brightness layers are processed.
    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    /// Returns whether geometry needs recomputing.
    pub fn layout_dirty(&self) -> bool {
        self.layout_dirty
    }

    /// Returns whether the scene needs rebuilding.
    pub fn paint_dirty(&self) -> bool {
        self.paint_dirty
    }

    /// Invalidates geometry and paint after viewport or external layout changes.
    pub fn invalidate_layout(&mut self) {
        self.layout_dirty = true;
        self.paint_dirty = true;
    }

    /// Invalidates paint after scale, focus chrome, or host appearance changes.
    pub fn invalidate_paint(&mut self) {
        self.paint_dirty = true;
    }

    /// Edits registered fonts, invalidating text measurement and geometry afterwards.
    ///
    /// The callback can use Lemon's `register_font_bytes` or Parley's font APIs.
    /// Its result is returned unchanged; caches are invalidated even if it returns
    /// an error or unwinds after partially changing the collection.
    ///
    /// ```no_run
    /// use lemon::{ui::UiState, register_font_bytes};
    /// let mut ui = UiState::default();
    /// ui.edit_fonts(|fonts| register_font_bytes(fonts, "Body", std::fs::read("Body.ttf").unwrap())).unwrap();
    /// ```
    pub fn edit_fonts<R>(&mut self, edit: impl FnOnce(&mut FontContext) -> R) -> R {
        // Invalidate first so partial writes during unwinding cannot leave clean caches.
        self.invalidate_layout();
        if let Some(tree) = &mut self.retained {
            tree.invalidate_text();
        }
        edit(&mut self.font_cx)
    }

    /// Mounts the root view and its nested components once.
    ///
    /// Returns an error if already mounted or if retained construction fails.
    pub fn mount(&mut self, root: impl Fn(&Cx) -> Element + 'static) -> Result<(), RetainedError> {
        if self.retained.is_some() {
            return Err(RetainedError::UnsupportedElement("UI already mounted"));
        }
        self.host = crate::host::HostContext::current();
        self.runtime.mount(root);
        let element = self
            .runtime
            .root_element()
            .ok_or(RetainedError::UnsupportedElement("missing root element"))?;
        let mut tree = RetainedTree::mount(element)?;
        tree.apply_patches(self.runtime.take_patches())?;
        self.retained = Some(tree);
        self.reconcile_input();
        self.layout_dirty = true;
        self.paint_dirty = true;
        Ok(())
    }

    /// Applies queued reactive changes without eagerly running layout.
    ///
    /// Dirty flags accumulate across batches. Earlier mutations remain applied
    /// if a later patch fails; errors are propagated to the host.
    pub fn apply_updates(&mut self) -> Result<(), RetainedError> {
        crate::reactive::observer::flush_deferred_notifications();
        #[cfg(debug_assertions)]
        let started = self.inspection.as_ref().map(|_| std::time::Instant::now());
        let result = self.apply_pending_updates();
        #[cfg(debug_assertions)]
        if let (Some(capture), Some(started)) = (&mut self.inspection, started) {
            capture.pending.updates += started.elapsed();
        }
        result
    }

    fn apply_pending_updates(&mut self) -> Result<(), RetainedError> {
        loop {
            self.runtime.flush_effects();
            let patches = self.runtime.take_patches();
            if patches.is_empty() {
                return Ok(());
            }
            self.apply_patches(patches)?;
        }
    }

    /// Applies an ordered patch batch, accumulating geometry and paint invalidation.
    ///
    /// Prefer `apply_updates` for reactive views. Integrations that generate patches
    /// directly use this method instead of mutating retained state or dirty flags.
    /// Earlier patches remain applied if a later patch fails.
    pub fn apply_patches(
        &mut self,
        patches: impl IntoIterator<Item = crate::diff::Patch>,
    ) -> Result<(), RetainedError> {
        let tree = self
            .retained
            .as_mut()
            .ok_or(RetainedError::UnsupportedElement("UI not mounted"))?;
        for patch in patches {
            self.scroll_limits_dirty |= matches!(
                &patch,
                crate::diff::Patch::UpdateWidgetChrome {
                    scroll_viewport: Some(_),
                    ..
                }
            ) || matches!(&patch, crate::diff::Patch::UpdateHandlers { handlers, .. }
                    if handlers.scroll_layout_max.is_some());
            self.paint_dirty = true;
            let result = tree.apply_patch(patch);
            self.layout_dirty |= tree.layout_dirty;
            result?;
            #[cfg(debug_assertions)]
            if let Some(capture) = &mut self.inspection {
                capture.pending.patches += 1;
            }
        }
        self.reconcile_input();
        if self.scroll_limits_dirty && !self.layout_dirty {
            if let Some(root) = self.retained.as_ref().and_then(|tree| tree.root.as_ref()) {
                sync_scroll_layout_max(root, &self.layout_map);
            }
        }
        Ok(())
    }

    /// Runs deferred user effects after the host has presented this UI's scene.
    pub fn finish_frame(&mut self) {
        self.runtime.flush_deferred_effects();
    }

    /// Computes geometry and scroll limits for `viewport`, preserving dirtiness on failure.
    ///
    /// Hosts call this explicitly after a resize or before geometry-dependent input.
    pub fn layout(&mut self, viewport: Viewport) -> Result<(), RetainedError> {
        #[cfg(debug_assertions)]
        let inspection_started = self.inspection.as_ref().map(|_| std::time::Instant::now());
        let started =
            crate::debug::enabled(crate::debug::Category::Layout).then(std::time::Instant::now);
        self.layout_dirty = true;
        self.paint_dirty = true;
        let tree = self
            .retained
            .as_mut()
            .ok_or(RetainedError::UnsupportedElement("UI not mounted"))?;
        self.layout_map = layout_pass(tree, &mut self.font_cx, viewport)?;
        if let Some(root) = &tree.root {
            sync_scroll_layout_max(root, &self.layout_map);
        }
        self.layout_dirty = false;
        self.scroll_limits_dirty = true;
        self.viewport = Some(viewport);
        #[cfg(debug_assertions)]
        if let (Some(capture), Some(started)) = (&mut self.inspection, inspection_started) {
            capture.pending.layout += started.elapsed();
            capture.pending.layouts += 1;
        }
        if let Some(started) = started {
            crate::lemon_trace!(
                Layout,
                "layout viewport={:.0}x{:.0} elapsed={:?}",
                viewport.width,
                viewport.height,
                started.elapsed()
            );
        }
        Ok(())
    }

    /// Applies updates, lays out dirty geometry, and rebuilds a dirty scene.
    ///
    /// Viewport changes invalidate geometry; scale, focus and caret changes invalidate
    /// paint automatically. No GPU or native window is needed.
    pub fn update(
        &mut self,
        viewport: Viewport,
        scale: f32,
        focused: Option<NodeId>,
        caret_visible: bool,
    ) -> Result<(), RetainedError> {
        self.update_at(
            viewport,
            scale,
            focused,
            caret_visible,
            std::time::Instant::now(),
        )
    }

    /// Updates UI processing using an explicit monotonic transition timestamp.
    ///
    /// Custom hosts and tests can drive transitions deterministically. Pass increasing
    /// timestamps; an earlier timestamp is clamped to the start of each transition.
    /// Like [`Self::update`], this applies pending changes and refreshes dirty layout.
    ///
    /// ```
    /// use lemon::{View, Viewport, ui::UiState};
    /// let mut ui = UiState::default();
    /// ui.mount(|_| View::new().into_element()).unwrap();
    /// ui.update_at(Viewport { width: 400.0, height: 300.0 }, 1.0, None, true,
    ///     std::time::Instant::now()).unwrap();
    /// ```
    pub fn update_at(
        &mut self,
        viewport: Viewport,
        scale: f32,
        focused: Option<NodeId>,
        caret_visible: bool,
        now: std::time::Instant,
    ) -> Result<(), RetainedError> {
        let _scope = self.host.enter();
        self.apply_updates()?;
        if self.viewport != Some(viewport) {
            self.invalidate_layout();
        }
        // Opted-out controls keep keyboard focus without drawing an outline after clicks.
        // Editor focus is unaffected, so caret/selection still use the focused node.
        let focused = focused.filter(|id| {
            !self.input.pointer_focus
                || self
                    .retained
                    .as_ref()
                    .and_then(|tree| tree.root.as_ref())
                    .and_then(|root| crate::input::hit_test::find_node_by_taffy_id(root, *id))
                    .is_none_or(|node| {
                        node.text_input.is_some() || node.style.focus_ring_on_pointer
                    })
        });
        let paint_context = (scale, focused, caret_visible);
        if self.last_paint != Some(paint_context) {
            self.invalidate_paint();
        }
        if self.layout_dirty || self.retained.as_ref().is_some_and(|tree| tree.layout_dirty) {
            self.layout(viewport)?;
        }
        if std::mem::take(&mut self.scroll_limits_dirty) && self.clamp_scroll_offsets() {
            self.apply_updates()?;
            if self.layout_dirty {
                self.layout(viewport)?;
            }
        }
        self.scroll_limits_dirty = false;
        if self.reveal_text_caret() {
            self.apply_updates()?;
            if self.layout_dirty {
                self.layout(viewport)?;
            }
        }
        if let Some(root) = self.retained.as_ref().and_then(|tree| tree.root.as_ref()) {
            let active = self.transitions.update(root, focused, now);
            self.paint_dirty |= active || self.transitions_active;
            self.transitions_active = active;
            if active {
                self.host.animations().request_frame();
            }
        }
        if self.paint_dirty {
            #[cfg(debug_assertions)]
            let inspection_started = self.inspection.as_ref().map(|_| std::time::Instant::now());
            let started =
                crate::debug::enabled(crate::debug::Category::Paint).then(std::time::Instant::now);
            let tree = self
                .retained
                .as_ref()
                .ok_or(RetainedError::UnsupportedElement("UI not mounted"))?;
            self.scene.reset();
            let _stats = crate::paint::paint_pass_with_visuals(
                tree,
                &self.layout_map,
                &mut self.scene,
                scale,
                focused,
                caret_visible,
                Some(&self.transitions.visuals),
            );
            #[cfg(debug_assertions)]
            if let (Some(capture), Some(started)) = (&mut self.inspection, inspection_started) {
                capture.pending.paint += started.elapsed();
                capture.pending.paints += 1;
                capture.pending.fills += _stats.fills;
                capture.pending.strokes += _stats.strokes;
                capture.pending.glyph_runs += _stats.glyph_runs;
                capture.pending.effect_layers = self.scene.effect_layer_count();
                if capture.hovered != capture.highlighted {
                    if let Some(rect) = capture.hovered.and_then(|id| self.layout_map.get(id)) {
                        crate::paint::paint_inspector_highlight(
                            &mut self.scene,
                            *rect,
                            scale,
                            false,
                        );
                    }
                }
                if let Some(rect) = capture.highlighted.and_then(|id| self.layout_map.get(id)) {
                    crate::paint::paint_inspector_highlight(&mut self.scene, *rect, scale, true);
                }
            }
            self.paint_dirty = false;
            self.last_paint = Some(paint_context);
            if let Some(started) = started {
                crate::lemon_trace!(Paint, "scene build elapsed={:?}", started.elapsed());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{hit_test::LogicalPoint, InputEvent};
    use crate::{ResizableView, ResizeEdge, RowView, ScrollOffset, ScrollView, State, View};
    use std::cell::Cell;
    use std::rc::Rc;

    fn resize_handle(node: &crate::retained::RetainedNode) -> Option<taffy::NodeId> {
        if node.style.cursor == crate::Cursor::ResizeHorizontal && node.handlers.on_drag.is_some() {
            return node.taffy_id;
        }
        node.children.iter().find_map(resize_handle)
    }

    #[test]
    fn deferred_scroll_events_rerender_once_when_the_batch_is_flushed() {
        let renders = Rc::new(Cell::new(0));
        let render_count = renders.clone();
        let offset = State::new(ScrollOffset::default());
        let scroll_offset = offset.clone();
        let mut ui = UiState::default();
        ui.mount(move |_| {
            render_count.set(render_count.get() + 1);
            ScrollView::with_offset(scroll_offset.clone(), View::new().height(500.0))
                .width(200.0)
                .height(100.0)
                .into_element()
        })
        .unwrap();
        let viewport = Viewport {
            width: 300.0,
            height: 200.0,
        };
        ui.update(viewport, 1.0, None, false).unwrap();
        assert_eq!(renders.get(), 1);

        for _ in 0..4 {
            ui.dispatch_input_deferred(InputEvent::Scroll {
                point: LogicalPoint::new(20.0, 20.0),
                delta: crate::ScrollDelta::new(0.0, -15.0),
            })
            .unwrap();
        }
        assert_eq!(offset.peek().y, 60.0);
        assert_eq!(renders.get(), 1, "events in one batch should not render");

        ui.apply_updates().unwrap();
        assert_eq!(renders.get(), 2, "the batch should render once");
        assert!(ui.layout_dirty());
    }

    #[test]
    fn deferred_resize_pointer_moves_accumulate_and_render_once() {
        let renders = Rc::new(Cell::new(0));
        let render_count = renders.clone();
        let mut ui = UiState::default();
        ui.mount(move |cx| {
            render_count.set(render_count.get() + 1);
            RowView::new()
                .fill()
                .child(
                    ResizableView::new(cx, View::new().fill())
                        .width(200.0)
                        .edges([ResizeEdge::Right]),
                )
                .child(View::new().grow(1.0).fill())
                .into_element()
        })
        .unwrap();
        let viewport = Viewport {
            width: 600.0,
            height: 300.0,
        };
        ui.update(viewport, 1.0, None, false).unwrap();
        let handle = resize_handle(ui.retained().unwrap().root().unwrap()).unwrap();
        let rect = ui.layout_map().get(handle).unwrap();
        let start = LogicalPoint::new(rect.x + rect.width / 2.0, 100.0);
        ui.dispatch_input(InputEvent::PointerDown(start)).unwrap();
        assert_eq!(renders.get(), 1);

        ui.dispatch_input_deferred(InputEvent::PointerMove(LogicalPoint::new(
            start.x + 20.0,
            start.y,
        )))
        .unwrap();
        ui.dispatch_input_deferred(InputEvent::PointerMove(LogicalPoint::new(
            start.x + 50.0,
            start.y,
        )))
        .unwrap();
        assert_eq!(renders.get(), 1);

        ui.apply_updates().unwrap();
        assert_eq!(renders.get(), 2);
        ui.update(viewport, 1.0, None, false).unwrap();
        let sidebar = &ui.retained().unwrap().root().unwrap().children[0];
        assert_eq!(
            ui.layout_map()
                .get(sidebar.taffy_id.unwrap())
                .unwrap()
                .width,
            250.0
        );
    }
}
