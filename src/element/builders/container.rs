use crate::element::{
    style::{BoxShadow, Color, ColorSource, CornerRadii, Edges, Gradient, Overflow},
    types::{BoxElement, Key},
    Element,
};
use std::rc::Rc;

// ── Macro to generate container builders (Column, Row, View) ──────────────

macro_rules! container_builder {
    ($name:ident, $variant:ident, $vertical:literal, $doc:expr) => {
        #[doc = $doc]
        pub struct $name(BoxElement, bool);

        impl $name {
            /// Creates an empty container; add children with [`.child`](Self::child).
            pub fn new() -> Self {
                $name(BoxElement::default(), $vertical)
            }

            /// Sets a descriptive ID for debug inspection without changing reconciliation identity.
            ///
            /// ```
            /// use lemon::View;
            /// let view = View::new().id("toolbar");
            /// ```
            pub fn id(mut self, id: impl Into<String>) -> Self {
                self.0.debug_id = Some(id.into());
                self
            }

            /// Sets spacing between children in logical points (default `0.0`).
            pub fn spacing(mut self, v: f32) -> Self {
                self.0.style.gap = Some(v);
                self
            }

            /// Horizontal spacing between columns, overriding uniform [`spacing`](Self::spacing)
            /// for this axis.
            ///
            /// In a `ColumnView` container this is the cross-axis gap; in a `Row` it is the main-axis
            /// gap between children.
            pub fn horizontal_spacing(mut self, v: f32) -> Self {
                self.0.style.column_gap = Some(v);
                self
            }

            /// Vertical spacing between rows, overriding uniform [`spacing`](Self::spacing)
            /// for this axis.
            ///
            /// In a `ColumnView` container this is the main-axis gap between children; in a `Row` it
            /// is the cross-axis gap.
            pub fn vertical_spacing(mut self, v: f32) -> Self {
                self.0.style.row_gap = Some(v);
                self
            }

            /// Sets a paint-only opacity multiplier for this container and its descendants.
            ///
            /// Values are clamped to `0.0..=1.0`; non-finite inputs fall back to `1.0`.
            pub fn opacity(mut self, v: f32) -> Self {
                self.0.style.opacity_group = true;
                self.0.style.opacity = if v.is_finite() {
                    v.clamp(0.0, 1.0)
                } else {
                    1.0
                };
                self
            }

            /// Blurs this view and its children with a Gaussian standard deviation in logical points.
            /// Defaults to zero. Layout and hit testing do not move; this is content blur,
            /// not backdrop blur. Filtered subtrees form their own visual stacking context.
            /// Values above 1024 points are capped to bound intermediate rendering.
            /// Panics for negative or non-finite values.
            ///
            /// ```
            /// use lemon::{Text, View};
            /// let view = View::new().blur(4.0).child(Text::new("Soft content"));
            /// ```
            pub fn blur(mut self, radius: f32) -> Self {
                assert!(
                    radius.is_finite() && radius >= 0.0,
                    "blur must be finite and nonnegative"
                );
                self.0.style.blur = radius.min(1024.0);
                self
            }

            /// Multiplies this subtree's RGB brightness while preserving alpha.
            /// Defaults to `1`; `0` produces black, values above `1` brighten.
            /// Applied after blur. Panics for negative or non-finite values.
            ///
            /// ```
            /// use lemon::View;
            /// let dimmed = View::new().brightness(0.6).opacity(0.8);
            /// ```
            pub fn brightness(mut self, factor: f32) -> Self {
                assert!(
                    factor.is_finite() && factor >= 0.0,
                    "brightness must be finite and nonnegative"
                );
                self.0.style.brightness = factor;
                self
            }

            /// Uniform padding on all sides, in logical points.
            pub fn inset(mut self, v: f32) -> Self {
                self.0.style.padding = Some(Edges::all(v));
                self
            }

            /// Padding on the left and right sides, in logical points.
            pub fn inset_horizontal(mut self, v: f32) -> Self {
                let mut edges = self.0.style.padding.take().unwrap_or_default();
                edges.left = v;
                edges.right = v;
                self.0.style.padding = Some(edges);
                self
            }

            /// Padding on the top and bottom sides, in logical points.
            pub fn inset_vertical(mut self, v: f32) -> Self {
                let mut edges = self.0.style.padding.take().unwrap_or_default();
                edges.top = v;
                edges.bottom = v;
                self.0.style.padding = Some(edges);
                self
            }

            /// Padding on the top side only, in logical points.
            pub fn inset_top(mut self, v: f32) -> Self {
                let mut edges = self.0.style.padding.take().unwrap_or_default();
                edges.top = v;
                self.0.style.padding = Some(edges);
                self
            }

            /// Padding on the right side only, in logical points.
            pub fn inset_right(mut self, v: f32) -> Self {
                let mut edges = self.0.style.padding.take().unwrap_or_default();
                edges.right = v;
                self.0.style.padding = Some(edges);
                self
            }

            /// Padding on the bottom side only, in logical points.
            pub fn inset_bottom(mut self, v: f32) -> Self {
                let mut edges = self.0.style.padding.take().unwrap_or_default();
                edges.bottom = v;
                self.0.style.padding = Some(edges);
                self
            }

            /// Padding on the left side only, in logical points.
            pub fn inset_left(mut self, v: f32) -> Self {
                let mut edges = self.0.style.padding.take().unwrap_or_default();
                edges.left = v;
                self.0.style.padding = Some(edges);
                self
            }

            /// Fixed width in logical points.
            pub fn width(mut self, v: f32) -> Self {
                self.0.style.width = Some(crate::element::style::Dimension::Points(v));
                self
            }

            /// Fixed height in logical points.
            pub fn height(mut self, v: f32) -> Self {
                self.0.style.height = Some(crate::element::style::Dimension::Points(v));
                self
            }

            /// Places children at a visible position within this container.
            ///
            /// Unlike axis-specific alignment, placement has the same meaning for rows,
            /// columns, and generic views. It replaces both alignment and distribution;
            /// later `.align(...)` / `.distribute(...)` calls can override one axis.
            ///
            /// ```
            /// use lemon::{Placement, Text, View};
            /// let panel = View::new().fill().place(Placement::BottomRight)
            ///     .child(Text::new("Status"));
            /// ```
            pub fn place(mut self, placement: crate::element::style::Placement) -> Self {
                let (align, justify) = placement.axes(self.1);
                self.0.style.align_items = Some(align);
                self.0.style.justify_content = Some(justify);
                self
            }

            /// Centers children horizontally and vertically within this container.
            ///
            /// Equivalent to `.place(Placement::Center)`. Size the container with `.fill()`
            /// or fixed dimensions to provide room for centering.
            ///
            /// ```
            /// use lemon::{Text, View};
            /// let root = View::new().fill().center().child(Text::new("Hello"));
            /// ```
            pub fn center(self) -> Self {
                self.place(crate::element::style::Placement::Center)
            }

            /// Clips children to this container's rectangular bounds.
            ///
            /// Clipping is disabled by default. Rounded paint corners do not change
            /// the clipping shape. Call `.unclip()` to restore visible overflow.
            pub fn clip(mut self) -> Self {
                self.0.style.overflow = Overflow::Hidden;
                self
            }

            /// Allows children to paint outside this container (the default).
            pub fn unclip(mut self) -> Self {
                self.0.style.overflow = Overflow::Visible;
                self
            }

            /// Requests the full width and height of the parent content area.
            ///
            /// On a root container this follows the window viewport, including resizes.
            /// In a nested layout, the parent must provide definite dimensions; normal
            /// layout constraints and sibling space sharing still apply. Later calls to
            /// [`width`](Self::width) or [`height`](Self::height) override that axis.
            ///
            /// ```
            /// use lemon::{Text, View};
            /// let root = View::new().fill().child(Text::new("Full window"));
            /// ```
            pub fn fill(self) -> Self {
                self.fill_width().fill_height()
            }

            /// Requests 100% of the parent content width, leaving height unchanged.
            ///
            /// The parent must provide a definite width. For root containers this is
            /// the viewport width; normal layout constraints still apply.
            pub fn fill_width(mut self) -> Self {
                self.0.style.width = Some(crate::element::style::Dimension::Percent(1.0));
                self
            }

            /// Requests 100% of the parent content height, leaving width unchanged.
            ///
            /// The parent must provide a definite height. For root containers this is
            /// the viewport height; normal layout constraints still apply.
            pub fn fill_height(mut self) -> Self {
                self.0.style.height = Some(crate::element::style::Dimension::Percent(1.0));
                self
            }

            /// Sets the share of spare space claimed along the parent stacking direction (default `0.0`).
            pub fn grow(mut self, v: f32) -> Self {
                self.0.style.flex_grow = Some(v);
                self
            }

            /// Aligns children perpendicular to this container's stacking direction; use `.place(...)` for physical positioning.
            pub fn align(mut self, v: crate::element::style::Align) -> Self {
                self.0.style.align_items = Some(v);
                self
            }

            /// Distributes spare space along the stacking direction, preserving perpendicular alignment.
            pub fn distribute(mut self, v: crate::element::style::Distribute) -> Self {
                self.0.style.justify_content = Some(v);
                self
            }

            /// Paint-only z-order among siblings (`0` keeps normal traversal order).
            ///
            /// Non-zero values are deferred to a sorted paint pass (ascending `z_index`) and do
            /// not affect layout.
            pub fn layer(mut self, z: i32) -> Self {
                self.0.style.z_index = z;
                self
            }

            /// Solid fill painted inside this container’s rounded bounds.
            ///
            /// Calling this clears any previously configured [`gradient`](Self::gradient).
            pub fn surface(mut self, c: impl Into<ColorSource>) -> Self {
                self.0.paint.background = Some(c.into());
                self.0.paint.background_gradient = None;
                self
            }

            /// Linear or radial fill painted inside this container's rounded bounds.
            /// Replaces any solid surface; pass `LinearGradient` or `RadialGradient`.
            /// Both support multiple color stops through their `.stops(...)` methods.
            ///
            /// ```
            /// use lemon::{Color, RadialGradient, View};
            /// let view = View::new().gradient(RadialGradient::new((0.5, 0.5), 0.5,
            ///     Color::rgb8(255, 255, 255), Color::rgb8(30, 80, 180)));
            /// ```
            pub fn gradient(mut self, gradient: impl Into<Gradient>) -> Self {
                self.0.paint.background_gradient = Some(gradient.into());
                self.0.paint.background = None;
                self
            }

            /// Blurred outer shadow painted before the background and borders.
            pub fn shadow(mut self, shadow: BoxShadow) -> Self {
                self.0.paint.box_shadow = Some(shadow);
                self
            }

            /// Border color and uniform width on all sides, in logical points.
            pub fn border(mut self, color: Color, width: f32) -> Self {
                self.0.paint.border_color = Edges::all(Some(ColorSource::Static(color)));
                self.0.paint.border_width = Edges::all(width);
                self
            }

            /// Border on the top side only (sets color and width for that side).
            pub fn border_top(mut self, color: Color, width: f32) -> Self {
                self.0.paint.border_color.top = Some(ColorSource::Static(color));
                self.0.paint.border_width.top = width;
                self
            }

            /// Border on the right side only.
            pub fn border_right(mut self, color: Color, width: f32) -> Self {
                self.0.paint.border_color.right = Some(ColorSource::Static(color));
                self.0.paint.border_width.right = width;
                self
            }

            /// Border on the bottom side only.
            pub fn border_bottom(mut self, color: Color, width: f32) -> Self {
                self.0.paint.border_color.bottom = Some(ColorSource::Static(color));
                self.0.paint.border_width.bottom = width;
                self
            }

            /// Border on the left side only.
            pub fn border_left(mut self, color: Color, width: f32) -> Self {
                self.0.paint.border_color.left = Some(ColorSource::Static(color));
                self.0.paint.border_width.left = width;
                self
            }

            /// Border on the left and right sides.
            pub fn border_horizontal(mut self, color: Color, width: f32) -> Self {
                self.0.paint.border_color.left = Some(ColorSource::Static(color));
                self.0.paint.border_color.right = Some(ColorSource::Static(color));
                self.0.paint.border_width.left = width;
                self.0.paint.border_width.right = width;
                self
            }

            /// Border on the top and bottom sides.
            pub fn border_vertical(mut self, color: Color, width: f32) -> Self {
                self.0.paint.border_color.top = Some(ColorSource::Static(color));
                self.0.paint.border_color.bottom = Some(ColorSource::Static(color));
                self.0.paint.border_width.top = width;
                self.0.paint.border_width.bottom = width;
                self
            }

            /// Uniform border color on all sides, preserving existing widths.
            pub fn border_color(mut self, color: Color) -> Self {
                self.0.paint.border_color = Edges::all(Some(ColorSource::Static(color)));
                self
            }

            /// Top border color only.
            pub fn border_color_top(mut self, color: Color) -> Self {
                self.0.paint.border_color.top = Some(ColorSource::Static(color));
                self
            }

            /// Right border color only.
            pub fn border_color_right(mut self, color: Color) -> Self {
                self.0.paint.border_color.right = Some(ColorSource::Static(color));
                self
            }

            /// Bottom border color only.
            pub fn border_color_bottom(mut self, color: Color) -> Self {
                self.0.paint.border_color.bottom = Some(ColorSource::Static(color));
                self
            }

            /// Left border color only.
            pub fn border_color_left(mut self, color: Color) -> Self {
                self.0.paint.border_color.left = Some(ColorSource::Static(color));
                self
            }

            /// Top border width only (other sides unchanged; set [`border`](Self::border) color first).
            pub fn border_width_top(mut self, width: f32) -> Self {
                self.0.paint.border_width.top = width;
                self
            }

            /// Right border width only.
            pub fn border_width_right(mut self, width: f32) -> Self {
                self.0.paint.border_width.right = width;
                self
            }

            /// Bottom border width only.
            pub fn border_width_bottom(mut self, width: f32) -> Self {
                self.0.paint.border_width.bottom = width;
                self
            }

            /// Left border width only.
            pub fn border_width_left(mut self, width: f32) -> Self {
                self.0.paint.border_width.left = width;
                self
            }

            /// Uniform corner radius on all four corners, in logical points.
            pub fn rounding(mut self, r: f32) -> Self {
                self.0.paint.radius = CornerRadii::all(r);
                self
            }

            /// Top-left corner radius only.
            pub fn rounding_top_left(mut self, r: f32) -> Self {
                self.0.paint.radius.top_left = r;
                self
            }

            /// Top-right corner radius only.
            pub fn rounding_top_right(mut self, r: f32) -> Self {
                self.0.paint.radius.top_right = r;
                self
            }

            /// Bottom-right corner radius only.
            pub fn rounding_bottom_right(mut self, r: f32) -> Self {
                self.0.paint.radius.bottom_right = r;
                self
            }

            /// Bottom-left corner radius only.
            pub fn rounding_bottom_left(mut self, r: f32) -> Self {
                self.0.paint.radius.bottom_left = r;
                self
            }

            /// Radius on both top corners (left and right).
            pub fn rounding_top(mut self, r: f32) -> Self {
                self.0.paint.radius.top_left = r;
                self.0.paint.radius.top_right = r;
                self
            }

            /// Radius on both bottom corners.
            pub fn rounding_bottom(mut self, r: f32) -> Self {
                self.0.paint.radius.bottom_left = r;
                self.0.paint.radius.bottom_right = r;
                self
            }

            /// Radius on both left corners (top and bottom).
            pub fn rounding_left(mut self, r: f32) -> Self {
                self.0.paint.radius.top_left = r;
                self.0.paint.radius.bottom_left = r;
                self
            }

            /// Radius on both right corners (top and bottom).
            pub fn rounding_right(mut self, r: f32) -> Self {
                self.0.paint.radius.top_right = r;
                self.0.paint.radius.bottom_right = r;
                self
            }

            /// Prevents pointer input inside this rectangle from reaching content behind it.
            /// Child controls still receive input. Defaults to false; useful for overlay panels.
            ///
            /// ```
            /// use lemon::View;
            /// let panel = View::new().blocks_input(true);
            /// ```
            pub fn blocks_input(mut self, blocks: bool) -> Self {
                self.0.style.blocks_input = blocks;
                self
            }

            /// Sets the minimum width in logical points (default: automatic content minimum).
            /// Use zero to let a row child shrink below its text's natural width and wrap.
            ///
            /// ```
            /// use lemon::View;
            /// let panel = View::new().width(400.0).min_width(0.0);
            /// ```
            pub fn min_width(mut self, width: f32) -> Self {
                self.0.style.min_width = Some(crate::element::style::Dimension::Points(width));
                self
            }

            /// Handles Escape from this view or a focused descendant before ordinary key input.
            /// The nearest handler wins, allowing nested popovers to dismiss independently.
            /// Repeated key presses do not trigger dismissal.
            ///
            /// ```
            /// use lemon::View;
            /// let overlay = View::new().on_escape(|| {});
            /// ```
            pub fn on_escape(mut self, f: impl Fn() + 'static) -> Self {
                self.0.handlers.on_escape = Some(Rc::new(f));
                self
            }

            /// Sets an image to draw inside this container using object-fit: contain scaling.
            ///
            /// The image is scaled uniformly to fit within the container's layout rectangle while
            /// preserving its aspect ratio, then centered within the available rectangle.
            ///
            /// ```no_run
            /// use lemon::{ColumnView, ImageHandle};
            ///
            /// # fn example(handle: ImageHandle) {
            /// ColumnView::new()
            ///     .width(200.0)
            ///     .height(150.0)
            ///     .image(handle)
            ///     .into_element();
            /// # }
            /// ```
            pub fn image(mut self, handle: crate::asset::ImageHandle) -> Self {
                self.0.paint.image = Some(handle);
                self
            }

            /// Appends a child widget; call repeatedly to build a list of children.
            pub fn child(mut self, el: impl Into<Element>) -> Self {
                self.0.children.push(el.into());
                self
            }

            /// Appends multiple children in order.
            ///
            /// For a list of mixed widget types, use the [`children!`](crate::children) macro:
            /// `.children(children![Text::new("a"), Button::new("b")])`.
            pub fn children(mut self, items: impl IntoIterator<Item = impl Into<Element>>) -> Self {
                self.0.children.extend(items.into_iter().map(Into::into));
                self
            }

            /// Stable identity for diffing when this child is inserted, removed, or reordered.
            ///
            /// All siblings under the same parent must use keys if any sibling uses one.
            /// Keys must be unique among those siblings and stable across renders.
            /// Reconciling fully keyed lists with duplicate keys panics.
            pub fn key(mut self, key: u64) -> Self {
                self.0.key = Some(Key(key));
                self
            }

            /// Mouse click handler (logical coordinates, hit-tested against this node’s bounds).
            pub fn action(mut self, f: impl Fn() + 'static) -> Self {
                self.0.handlers.on_click = Some(Rc::new(f));
                self
            }

            pub fn on_key_down(
                mut self,
                f: impl Fn(crate::element::events::KeyEvent) + 'static,
            ) -> Self {
                self.0.handlers.on_key_down = Some(Rc::new(f));
                self
            }

            pub fn on_key_up(
                mut self,
                f: impl Fn(crate::element::events::KeyEvent) + 'static,
            ) -> Self {
                self.0.handlers.on_key_up = Some(Rc::new(f));
                self
            }

            /// Observes pointer entry/exit for this view, including its descendants.
            /// Receives `true` on entry and `false` on exit, hide, removal or window leave.
            /// Does not add a Tab stop. Existing enter/leave callbacks also remain active.
            ///
            /// ```
            /// use lemon::View;
            /// let view = View::new().on_hover(|hovered| println!("hover: {hovered}"));
            /// ```
            pub fn on_hover(mut self, f: impl Fn(bool) + 'static) -> Self {
                self.0.handlers.on_hover = Some(Rc::new(f));
                self
            }

            /// Observes this view's direct focus, including loss on hide or removal.
            /// Add `.focusable()` for a plain container; controls already opt in.
            /// This observes element focus, independently of native window activation.
            ///
            /// ```
            /// use lemon::View;
            /// let view = View::new().focusable().on_focus(|focused| println!("focus: {focused}"));
            /// ```
            pub fn on_focus(mut self, f: impl Fn(bool) + 'static) -> Self {
                self.0.handlers.on_focus = Some(Rc::new(f));
                self
            }

            pub fn on_hover_enter(mut self, f: impl Fn() + 'static) -> Self {
                self.0.handlers.on_hover_enter = Some(Rc::new(f));
                self
            }

            pub fn on_hover_leave(mut self, f: impl Fn() + 'static) -> Self {
                self.0.handlers.on_hover_leave = Some(Rc::new(f));
                self
            }

            /// Mouse wheel handler; `delta` preserves both axes in logical points.
            ///
            /// Used by scrollable regions such as [`ScrollView`](crate::ScrollView). Input
            /// routes to the deepest viewport accepting the movement's axis.
            pub fn on_scroll(mut self, f: impl Fn(crate::ScrollDelta) + 'static) -> Self {
                self.0.handlers.on_scroll = Some(Rc::new(f));
                self
            }

            /// Marks this node as the offset wrapper whose children form scroll content.
            ///
            /// Use inside a clipped viewport with an `on_scroll` handler. The wrapper's
            /// negative top margin represents the current vertical offset.
            ///
            /// ```
            /// use lemon::View;
            /// let content = View::new().scroll_content().outset_top(-20.0);
            /// ```
            pub fn scroll_content(mut self) -> Self {
                self.0.style.scroll_content = true;
                self
            }

            /// Called when a click lands outside this node's layout bounds while it is mounted.
            pub fn on_click_outside(mut self, f: impl Fn() + 'static) -> Self {
                self.0.handlers.on_click_outside = Some(Rc::new(f));
                self
            }

            /// Pointer press handler with normalized coordinates in this node's bounds (`0.0..=1.0`).
            ///
            /// ```no_run
            /// # use lemon::{View, Color};
            /// View::new()
            ///     .width(200.0)
            ///     .height(100.0)
            ///     .on_pointer_down(|x, y| {
            ///         let local_px_x = x * 200.0;
            ///         let local_px_y = y * 100.0;
            ///         let _ = (local_px_x, local_px_y, Color::rgb8(255, 0, 0));
            ///     })
            ///     .into_element();
            /// ```
            pub fn on_pointer_down(mut self, f: impl Fn(f32, f32) + 'static) -> Self {
                self.0.handlers.on_pointer_down = Some(Rc::new(f));
                self
            }

            /// Pointer move handler with normalized coordinates in this node's bounds (`0.0..=1.0`).
            pub fn on_pointer_move(mut self, f: impl Fn(f32, f32) + 'static) -> Self {
                self.0.handlers.on_pointer_move = Some(Rc::new(f));
                self
            }

            /// Drags with logical-point `(dx, dy)` deltas while the pointer is captured.
            ///
            /// A press on this view starts capture; movement continues outside its bounds
            /// until release. Deltas remain stable if the view moves or resizes.
            /// After movement, the release click is consumed to avoid activating adjacent controls.
            ///
            /// ```
            /// use lemon::View;
            /// let handle = View::new().width(20.0).height(12.0).on_drag(|_dx, dy| {
            ///     println!("vertical movement: {dy}");
            /// });
            /// ```
            pub fn on_drag(mut self, action: impl Fn(f32, f32) + 'static) -> Self {
                self.0.handlers.on_drag = Some(Rc::new(action));
                self
            }

            /// Pointer release handler with normalized coordinates in this node's bounds (`0.0..=1.0`).
            pub fn on_pointer_up(mut self, f: impl Fn(f32, f32) + 'static) -> Self {
                self.0.handlers.on_pointer_up = Some(Rc::new(f));
                self
            }

            /// Binds measured horizontal/vertical maximum offsets, in logical points.
            /// Updated after layout for scroll clamping; disabled axes receive zero.
            /// Used by [`ScrollView`](crate::ScrollView) and custom marked scroll regions.
            ///
            /// ```
            /// use lemon::{ScrollOffset, View};
            /// let limits = std::rc::Rc::new(std::cell::Cell::new(ScrollOffset::default()));
            /// let viewport = View::new().scroll_layout_max(limits);
            /// ```
            pub fn scroll_layout_max(
                mut self,
                cell: std::rc::Rc<std::cell::Cell<crate::ScrollOffset>>,
            ) -> Self {
                self.0.handlers.scroll_layout_max = Some(cell);
                self
            }

            #[allow(dead_code)] // Generated for every container; only the View viewport binds it.
            pub(crate) fn scrollbar(mut self, binding: crate::retained::ScrollbarBinding) -> Self {
                self.0.handlers.scrollbar = Some(binding);
                self
            }

            /// Uniform margin on all sides, in logical points.
            pub fn outset(mut self, v: f32) -> Self {
                self.0.style.margin = Some(Edges::all(v));
                self
            }

            /// Margin on the left and right sides, in logical points.
            pub fn outset_horizontal(mut self, v: f32) -> Self {
                let mut edges = self.0.style.margin.take().unwrap_or_default();
                edges.left = v;
                edges.right = v;
                self.0.style.margin = Some(edges);
                self
            }

            /// Margin on the top and bottom sides, in logical points.
            pub fn outset_vertical(mut self, v: f32) -> Self {
                let mut edges = self.0.style.margin.take().unwrap_or_default();
                edges.top = v;
                edges.bottom = v;
                self.0.style.margin = Some(edges);
                self
            }

            /// Sets top margin in logical points (other sides unchanged).
            ///
            /// Negative values shift content up without changing layout size — useful for
            /// scroll offsets on inner content inside a clipped viewport.
            pub fn outset_top(mut self, v: f32) -> Self {
                let mut edges = self.0.style.margin.take().unwrap_or_default();
                edges.top = v;
                self.0.style.margin = Some(edges);
                self
            }

            /// Sets right margin in logical points (other sides unchanged).
            pub fn outset_right(mut self, v: f32) -> Self {
                let mut edges = self.0.style.margin.take().unwrap_or_default();
                edges.right = v;
                self.0.style.margin = Some(edges);
                self
            }

            /// Sets bottom margin in logical points (other sides unchanged).
            pub fn outset_bottom(mut self, v: f32) -> Self {
                let mut edges = self.0.style.margin.take().unwrap_or_default();
                edges.bottom = v;
                self.0.style.margin = Some(edges);
                self
            }

            /// Sets left margin in logical points (other sides unchanged).
            pub fn outset_left(mut self, v: f32) -> Self {
                let mut edges = self.0.style.margin.take().unwrap_or_default();
                edges.left = v;
                self.0.style.margin = Some(edges);
                self
            }

            /// Sets the top edge offset in logical points; other edges stay unchanged.
            ///
            /// Its reference frame follows [`.position(...)`](Self::position).
            pub fn top(mut self, v: f32) -> Self {
                let mut edges = self.0.style.inset.take().unwrap_or_default();
                edges.top = Some(v);
                self.0.style.inset = Some(edges);
                self
            }

            /// Sets the left edge offset in logical points; other edges stay unchanged.
            ///
            /// Its reference frame follows [`.position(...)`](Self::position).
            pub fn left(mut self, v: f32) -> Self {
                let mut edges = self.0.style.inset.take().unwrap_or_default();
                edges.left = Some(v);
                self.0.style.inset = Some(edges);
                self
            }

            /// Enables or disables input for this entire subtree (default: enabled).
            ///
            /// Layout and painting are preserved. Use `false` while a closing overlay
            /// fades out so its invisible controls cannot receive focus or clicks.
            /// Existing focus and pointer capture within the subtree are cleared.
            ///
            /// ```
            /// use lemon::View;
            /// let closing = View::new().opacity(0.0).interactive(false);
            /// ```
            pub fn interactive(mut self, enabled: bool) -> Self {
                self.0.style.interactive = enabled;
                self
            }

            /// Includes this node in keyboard focus traversal (Tab / Shift+Tab).
            pub fn focusable(mut self) -> Self {
                self.0.style.focusable = true;
                self
            }

            /// Attaches text-field metadata used by the paint pass (caret, focus ring).
            pub fn text_input(mut self, meta: crate::element::types::TextInputMeta) -> Self {
                self.0.text_input = Some(meta);
                self
            }

            /// Marks this node as a scroll viewport on `axis`, drawing a scrollbar on overflow.
            pub fn scroll_viewport(mut self, axis: crate::ScrollAxis) -> Self {
                self.0.scroll_viewport = Some(axis);
                self
            }

            /// Animates visual changes in this view and its descendants.
            ///
            /// Initial mounting is immediate. Layout changes are not animated.
            /// Use through `.decorate(...)` to transition a widget's default states.
            ///
            /// ```
            /// use lemon::{View, Transition};
            /// let view = View::new().transition(Transition::default());
            /// ```
            pub fn transition(mut self, transition: crate::transition::Transition) -> Self {
                self.0.style.transition = Some(transition);
                self
            }

            /// Sets normal, absolute, fixed or sticky positioning (default: relative).
            ///
            /// Pair with edge offsets; unspecified edges remain automatic. Absolute and
            /// fixed views reserve no sibling space. Fixed views use the window viewport;
            /// sticky views remain in flow and stop at their parent boundary.
            ///
            /// ```
            /// use lemon::{Position, View};
            /// let header = View::new().position(Position::Sticky).top(0.0).height(48.0);
            /// ```
            pub fn position(mut self, position: crate::element::style::Position) -> Self {
                self.0.style.position = position;
                self
            }

            /// Sets the right edge offset in logical points; other edges stay unchanged.
            pub fn right(mut self, value: f32) -> Self {
                let mut edges = self.0.style.inset.take().unwrap_or_default();
                edges.right = Some(value);
                self.0.style.inset = Some(edges);
                self
            }

            /// Sets the bottom edge offset in logical points; other edges stay unchanged.
            pub fn bottom(mut self, value: f32) -> Self {
                let mut edges = self.0.style.inset.take().unwrap_or_default();
                edges.bottom = Some(value);
                self.0.style.inset = Some(edges);
                self
            }

            /// Cursor shown when the pointer is over this node.
            pub fn cursor(mut self, c: crate::element::events::Cursor) -> Self {
                self.0.style.cursor = c;
                self
            }

            /// Finishes the builder and returns an [`Element`] for use in [`.child`](Self::child) or the root view.
            pub fn into_element(self) -> Element {
                Element::$variant(self.0)
            }
        }

        impl Default for $name {
            fn default() -> Self {
                $name::new()
            }
        }

        impl From<$name> for Element {
            fn from(b: $name) -> Self {
                b.into_element()
            }
        }
    };
}

container_builder!(
    ColumnView,
    Column,
    true,
    "Vertical container: children stack from top to bottom.\n\nSpacing defaults to zero and size is automatic.\n\n```\nuse lemon::{ColumnView, Text};\nlet column = ColumnView::new().spacing(12.0).child(Text::new(\"Hello\")).into_element();\n```"
);
container_builder!(
    RowView,
    Row,
    false,
    "Horizontal container: children stack from left to right.\n\nSpacing defaults to zero and size is automatic.\n\n```\nuse lemon::{RowView, Text};\nlet row = RowView::new().spacing(12.0).child(Text::new(\"Hello\")).into_element();\n```"
);
container_builder!(
    View,
    View,
    false,
    "Generic flex container; use when you do not need an explicit row or column axis."
);

impl View {
    pub(crate) fn from_box(element: BoxElement, vertical: bool) -> Self {
        Self(element, vertical)
    }

    pub(crate) fn into_box(self) -> BoxElement {
        self.0
    }
}
