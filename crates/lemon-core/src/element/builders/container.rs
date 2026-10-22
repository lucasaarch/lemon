use crate::element::{
    style::{BoxShadow, Color, ColorSource, CornerRadii, Edges, LinearGradient, Overflow},
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

            /// Sets spacing between children in logical points (default `0.0`).
            pub fn spacing(mut self, v: f32) -> Self {
                self.0.style.gap = Some(v);
                self
            }

            /// Deprecated name for [`spacing`](Self::spacing).
            #[deprecated(note = "use .spacing(...) instead")]
            pub fn gap(self, v: f32) -> Self {
                self.spacing(v)
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

            /// Deprecated name for [`horizontal_spacing`](Self::horizontal_spacing).
            #[deprecated(note = "use .horizontal_spacing(...) instead")]
            pub fn column_gap(self, v: f32) -> Self {
                self.horizontal_spacing(v)
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

            /// Deprecated name for [`vertical_spacing`](Self::vertical_spacing).
            #[deprecated(note = "use .vertical_spacing(...) instead")]
            pub fn row_gap(self, v: f32) -> Self {
                self.vertical_spacing(v)
            }
            /// Sets a paint-only opacity multiplier for this container and its descendants.
            ///
            /// Values are clamped to `0.0..=1.0`; non-finite inputs fall back to `1.0`.
            pub fn opacity(mut self, v: f32) -> Self {
                self.0.style.opacity = if v.is_finite() {
                    v.clamp(0.0, 1.0)
                } else {
                    1.0
                };
                self
            }
            /// Uniform padding on all sides, in logical points.
            pub fn inset(mut self, v: f32) -> Self {
                self.0.style.padding = Some(Edges::all(v));
                self
            }

            /// Deprecated name for [`inset`](Self::inset).
            #[deprecated(note = "use .inset(...) instead")]
            pub fn padding(self, v: f32) -> Self {
                self.inset(v)
            }
            /// Padding on the left and right sides, in logical points.
            pub fn inset_horizontal(mut self, v: f32) -> Self {
                let mut edges = self.0.style.padding.take().unwrap_or_default();
                edges.left = v;
                edges.right = v;
                self.0.style.padding = Some(edges);
                self
            }

            /// Deprecated name for [`inset_horizontal`](Self::inset_horizontal).
            #[deprecated(note = "use .inset_horizontal(...) instead")]
            pub fn padding_x(self, v: f32) -> Self {
                self.inset_horizontal(v)
            }
            /// Padding on the top and bottom sides, in logical points.
            pub fn inset_vertical(mut self, v: f32) -> Self {
                let mut edges = self.0.style.padding.take().unwrap_or_default();
                edges.top = v;
                edges.bottom = v;
                self.0.style.padding = Some(edges);
                self
            }

            /// Deprecated name for [`inset_vertical`](Self::inset_vertical).
            #[deprecated(note = "use .inset_vertical(...) instead")]
            pub fn padding_y(self, v: f32) -> Self {
                self.inset_vertical(v)
            }
            /// Padding on the top side only, in logical points.
            pub fn inset_top(mut self, v: f32) -> Self {
                let mut edges = self.0.style.padding.take().unwrap_or_default();
                edges.top = v;
                self.0.style.padding = Some(edges);
                self
            }

            /// Deprecated name for [`inset_top`](Self::inset_top).
            #[deprecated(note = "use .inset_top(...) instead")]
            pub fn padding_top(self, v: f32) -> Self {
                self.inset_top(v)
            }
            /// Padding on the right side only, in logical points.
            pub fn inset_right(mut self, v: f32) -> Self {
                let mut edges = self.0.style.padding.take().unwrap_or_default();
                edges.right = v;
                self.0.style.padding = Some(edges);
                self
            }

            /// Deprecated name for [`inset_right`](Self::inset_right).
            #[deprecated(note = "use .inset_right(...) instead")]
            pub fn padding_right(self, v: f32) -> Self {
                self.inset_right(v)
            }
            /// Padding on the bottom side only, in logical points.
            pub fn inset_bottom(mut self, v: f32) -> Self {
                let mut edges = self.0.style.padding.take().unwrap_or_default();
                edges.bottom = v;
                self.0.style.padding = Some(edges);
                self
            }

            /// Deprecated name for [`inset_bottom`](Self::inset_bottom).
            #[deprecated(note = "use .inset_bottom(...) instead")]
            pub fn padding_bottom(self, v: f32) -> Self {
                self.inset_bottom(v)
            }
            /// Padding on the left side only, in logical points.
            pub fn inset_left(mut self, v: f32) -> Self {
                let mut edges = self.0.style.padding.take().unwrap_or_default();
                edges.left = v;
                self.0.style.padding = Some(edges);
                self
            }

            /// Deprecated name for [`inset_left`](Self::inset_left).
            #[deprecated(note = "use .inset_left(...) instead")]
            pub fn padding_left(self, v: f32) -> Self {
                self.inset_left(v)
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
            /// use lemon_core::{Placement, Text, View};
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
            /// use lemon_core::{Text, View};
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
            /// use lemon_core::{Text, View};
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

            /// Deprecated name for [`grow`](Self::grow).
            #[deprecated(note = "use .grow(...) instead")]
            pub fn flex_grow(self, v: f32) -> Self {
                self.grow(v)
            }
            /// Aligns children perpendicular to this container's stacking direction; use `.place(...)` for physical positioning.
            pub fn align(mut self, v: crate::element::style::Align) -> Self {
                self.0.style.align_items = Some(v);
                self
            }

            /// Deprecated name for [`align`](Self::align).
            #[deprecated(note = "use .align(...) instead")]
            pub fn align_items(self, v: crate::element::style::Align) -> Self {
                self.align(v)
            }
            /// Distributes spare space along the stacking direction, preserving perpendicular alignment.
            pub fn distribute(mut self, v: impl Into<crate::element::style::Distribution>) -> Self {
                self.0.style.justify_content = Some(v.into().into());
                self
            }

            /// Deprecated name for [`distribute`](Self::distribute).
            #[deprecated(note = "use .distribute(...) instead")]
            pub fn justify_content(self, v: crate::element::style::Justify) -> Self {
                self.distribute(v)
            }
            /// Deprecated overflow switch; use `.clip()` or `.unclip()`.
            #[deprecated(note = "use .clip() or .unclip() instead")]
            pub fn overflow(mut self, v: Overflow) -> Self {
                self.0.style.overflow = v;
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

            /// Deprecated name for [`layer`](Self::layer).
            #[deprecated(note = "use .layer(...) instead")]
            pub fn z_index(self, z: i32) -> Self {
                self.layer(z)
            }
            /// Solid fill painted inside this container’s rounded bounds.
            ///
            /// Calling this clears any previously configured [`gradient`](Self::gradient).
            pub fn surface(mut self, c: impl Into<ColorSource>) -> Self {
                self.0.paint.background = Some(c.into());
                self.0.paint.background_gradient = None;
                self
            }

            /// Deprecated name for [`surface`](Self::surface).
            #[deprecated(note = "use .surface(...) instead")]
            pub fn background(self, c: impl Into<ColorSource>) -> Self {
                self.surface(c)
            }
            /// Two-stop linear gradient fill painted inside this container’s rounded bounds.
            pub fn gradient(mut self, gradient: LinearGradient) -> Self {
                self.0.paint.background_gradient = Some(gradient);
                self.0.paint.background = None;
                self
            }

            /// Deprecated name for [`gradient`](Self::gradient).
            #[deprecated(note = "use .gradient(...) instead")]
            pub fn linear_gradient(self, gradient: LinearGradient) -> Self {
                self.gradient(gradient)
            }
            /// Blurred outer shadow painted before the background and borders.
            pub fn shadow(mut self, shadow: BoxShadow) -> Self {
                self.0.paint.box_shadow = Some(shadow);
                self
            }

            /// Deprecated name for [`shadow`](Self::shadow).
            #[deprecated(note = "use .shadow(...) instead")]
            pub fn box_shadow(self, shadow: BoxShadow) -> Self {
                self.shadow(shadow)
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

            /// Deprecated name for [`border_horizontal`](Self::border_horizontal).
            #[deprecated(note = "use .border_horizontal(...) instead")]
            pub fn border_x(self, color: Color, width: f32) -> Self {
                self.border_horizontal(color, width)
            }
            /// Border on the top and bottom sides.
            pub fn border_vertical(mut self, color: Color, width: f32) -> Self {
                self.0.paint.border_color.top = Some(ColorSource::Static(color));
                self.0.paint.border_color.bottom = Some(ColorSource::Static(color));
                self.0.paint.border_width.top = width;
                self.0.paint.border_width.bottom = width;
                self
            }

            /// Deprecated name for [`border_vertical`](Self::border_vertical).
            #[deprecated(note = "use .border_vertical(...) instead")]
            pub fn border_y(self, color: Color, width: f32) -> Self {
                self.border_vertical(color, width)
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

            /// Deprecated name for [`rounding`](Self::rounding).
            #[deprecated(note = "use .rounding(...) instead")]
            pub fn radius(self, r: f32) -> Self {
                self.rounding(r)
            }
            /// Top-left corner radius only.
            pub fn rounding_top_left(mut self, r: f32) -> Self {
                self.0.paint.radius.top_left = r;
                self
            }

            /// Deprecated name for [`rounding_top_left`](Self::rounding_top_left).
            #[deprecated(note = "use .rounding_top_left(...) instead")]
            pub fn radius_top_left(self, r: f32) -> Self {
                self.rounding_top_left(r)
            }
            /// Top-right corner radius only.
            pub fn rounding_top_right(mut self, r: f32) -> Self {
                self.0.paint.radius.top_right = r;
                self
            }

            /// Deprecated name for [`rounding_top_right`](Self::rounding_top_right).
            #[deprecated(note = "use .rounding_top_right(...) instead")]
            pub fn radius_top_right(self, r: f32) -> Self {
                self.rounding_top_right(r)
            }
            /// Bottom-right corner radius only.
            pub fn rounding_bottom_right(mut self, r: f32) -> Self {
                self.0.paint.radius.bottom_right = r;
                self
            }

            /// Deprecated name for [`rounding_bottom_right`](Self::rounding_bottom_right).
            #[deprecated(note = "use .rounding_bottom_right(...) instead")]
            pub fn radius_bottom_right(self, r: f32) -> Self {
                self.rounding_bottom_right(r)
            }
            /// Bottom-left corner radius only.
            pub fn rounding_bottom_left(mut self, r: f32) -> Self {
                self.0.paint.radius.bottom_left = r;
                self
            }

            /// Deprecated name for [`rounding_bottom_left`](Self::rounding_bottom_left).
            #[deprecated(note = "use .rounding_bottom_left(...) instead")]
            pub fn radius_bottom_left(self, r: f32) -> Self {
                self.rounding_bottom_left(r)
            }
            /// Radius on both top corners (left and right).
            pub fn rounding_top(mut self, r: f32) -> Self {
                self.0.paint.radius.top_left = r;
                self.0.paint.radius.top_right = r;
                self
            }

            /// Deprecated name for [`rounding_top`](Self::rounding_top).
            #[deprecated(note = "use .rounding_top(...) instead")]
            pub fn radius_top(self, r: f32) -> Self {
                self.rounding_top(r)
            }
            /// Radius on both bottom corners.
            pub fn rounding_bottom(mut self, r: f32) -> Self {
                self.0.paint.radius.bottom_left = r;
                self.0.paint.radius.bottom_right = r;
                self
            }

            /// Deprecated name for [`rounding_bottom`](Self::rounding_bottom).
            #[deprecated(note = "use .rounding_bottom(...) instead")]
            pub fn radius_bottom(self, r: f32) -> Self {
                self.rounding_bottom(r)
            }
            /// Radius on both left corners (top and bottom).
            pub fn rounding_left(mut self, r: f32) -> Self {
                self.0.paint.radius.top_left = r;
                self.0.paint.radius.bottom_left = r;
                self
            }

            /// Deprecated name for [`rounding_left`](Self::rounding_left).
            #[deprecated(note = "use .rounding_left(...) instead")]
            pub fn radius_left(self, r: f32) -> Self {
                self.rounding_left(r)
            }
            /// Radius on both right corners (top and bottom).
            pub fn rounding_right(mut self, r: f32) -> Self {
                self.0.paint.radius.top_right = r;
                self.0.paint.radius.bottom_right = r;
                self
            }

            /// Deprecated name for [`rounding_right`](Self::rounding_right).
            #[deprecated(note = "use .rounding_right(...) instead")]
            pub fn radius_right(self, r: f32) -> Self {
                self.rounding_right(r)
            }
            /// Sets an image to draw inside this container using object-fit: contain scaling.
            ///
            /// The image is scaled uniformly to fit within the container's layout rectangle while
            /// preserving its aspect ratio, then centered within the available rectangle.
            ///
            /// ```no_run
            /// use lemon_core::{ColumnView, ImageHandle};
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

            /// Deprecated name for [`action`](Self::action).
            #[deprecated(note = "use .action(...) instead")]
            pub fn on_click(self, f: impl Fn() + 'static) -> Self {
                self.action(f)
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
            pub fn on_hover_enter(mut self, f: impl Fn() + 'static) -> Self {
                self.0.handlers.on_hover_enter = Some(Rc::new(f));
                self
            }
            pub fn on_hover_leave(mut self, f: impl Fn() + 'static) -> Self {
                self.0.handlers.on_hover_leave = Some(Rc::new(f));
                self
            }
            /// Mouse wheel handler; `delta` is the vertical scroll amount in logical pixels.
            ///
            /// Used by scrollable regions (such as `lemon-ui`'s `Scroll`). The platform
            /// dispatches wheel events to the deepest node under the cursor with this handler.
            pub fn on_scroll(mut self, f: impl Fn(f64) + 'static) -> Self {
                self.0.handlers.on_scroll = Some(Rc::new(f));
                self
            }

            /// Marks this node as the offset wrapper whose children form scroll content.
            ///
            /// Use inside a clipped viewport with an `on_scroll` handler. The wrapper's
            /// negative top margin represents the current vertical offset.
            ///
            /// ```
            /// use lemon_core::View;
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
            /// # use lemon_core::{View, Color};
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
            /// Pointer release handler with normalized coordinates in this node's bounds (`0.0..=1.0`).
            pub fn on_pointer_up(mut self, f: impl Fn(f32, f32) + 'static) -> Self {
                self.0.handlers.on_pointer_up = Some(Rc::new(f));
                self
            }
            /// Binds a cell updated after layout for scroll clamping (used by `Scroll` widget).
            pub fn scroll_layout_max(mut self, cell: std::rc::Rc<std::cell::Cell<f64>>) -> Self {
                self.0.handlers.scroll_layout_max = Some(cell);
                self
            }
            /// Uniform margin on all sides, in logical points.
            pub fn outset(mut self, v: f32) -> Self {
                self.0.style.margin = Some(Edges::all(v));
                self
            }

            /// Deprecated name for [`outset`](Self::outset).
            #[deprecated(note = "use .outset(...) instead")]
            pub fn margin(self, v: f32) -> Self {
                self.outset(v)
            }
            /// Margin on the left and right sides, in logical points.
            pub fn outset_horizontal(mut self, v: f32) -> Self {
                let mut edges = self.0.style.margin.take().unwrap_or_default();
                edges.left = v;
                edges.right = v;
                self.0.style.margin = Some(edges);
                self
            }

            /// Deprecated name for [`outset_horizontal`](Self::outset_horizontal).
            #[deprecated(note = "use .outset_horizontal(...) instead")]
            pub fn margin_x(self, v: f32) -> Self {
                self.outset_horizontal(v)
            }
            /// Margin on the top and bottom sides, in logical points.
            pub fn outset_vertical(mut self, v: f32) -> Self {
                let mut edges = self.0.style.margin.take().unwrap_or_default();
                edges.top = v;
                edges.bottom = v;
                self.0.style.margin = Some(edges);
                self
            }

            /// Deprecated name for [`outset_vertical`](Self::outset_vertical).
            #[deprecated(note = "use .outset_vertical(...) instead")]
            pub fn margin_y(self, v: f32) -> Self {
                self.outset_vertical(v)
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

            /// Deprecated name for [`outset_top`](Self::outset_top).
            #[deprecated(note = "use .outset_top(...) instead")]
            pub fn margin_top(self, v: f32) -> Self {
                self.outset_top(v)
            }
            /// Sets right margin in logical points (other sides unchanged).
            pub fn outset_right(mut self, v: f32) -> Self {
                let mut edges = self.0.style.margin.take().unwrap_or_default();
                edges.right = v;
                self.0.style.margin = Some(edges);
                self
            }

            /// Deprecated name for [`outset_right`](Self::outset_right).
            #[deprecated(note = "use .outset_right(...) instead")]
            pub fn margin_right(self, v: f32) -> Self {
                self.outset_right(v)
            }
            /// Sets bottom margin in logical points (other sides unchanged).
            pub fn outset_bottom(mut self, v: f32) -> Self {
                let mut edges = self.0.style.margin.take().unwrap_or_default();
                edges.bottom = v;
                self.0.style.margin = Some(edges);
                self
            }

            /// Deprecated name for [`outset_bottom`](Self::outset_bottom).
            #[deprecated(note = "use .outset_bottom(...) instead")]
            pub fn margin_bottom(self, v: f32) -> Self {
                self.outset_bottom(v)
            }
            /// Sets left margin in logical points (other sides unchanged).
            pub fn outset_left(mut self, v: f32) -> Self {
                let mut edges = self.0.style.margin.take().unwrap_or_default();
                edges.left = v;
                self.0.style.margin = Some(edges);
                self
            }

            /// Deprecated name for [`outset_left`](Self::outset_left).
            #[deprecated(note = "use .outset_left(...) instead")]
            pub fn margin_left(self, v: f32) -> Self {
                self.outset_left(v)
            }
            /// Sets the top inset in logical points.
            ///
            /// Use with [`.floating()`](Self::floating) to position an overlay relative to its
            /// containing flex ancestor without taking up normal flex space.
            pub fn top(mut self, v: f32) -> Self {
                let mut edges = self.0.style.inset.take().unwrap_or_default();
                edges.top = v;
                self.0.style.inset = Some(edges);
                self
            }
            /// Sets the left inset in logical points.
            ///
            /// Use with [`.floating()`](Self::floating) to pin an overlay's horizontal position
            /// relative to its containing flex ancestor.
            pub fn left(mut self, v: f32) -> Self {
                let mut edges = self.0.style.inset.take().unwrap_or_default();
                edges.left = v;
                self.0.style.inset = Some(edges);
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
            /// Marks this node as a vertical scroll viewport (scrollbar when content overflows).
            pub fn scroll_viewport(mut self) -> Self {
                self.0.scroll_viewport = true;
                self
            }
            /// Paints a widget-style scrollbar track and thumb from layout measurements.
            pub fn scroll_bar(mut self) -> Self {
                self.0.scroll_bar = true;
                self
            }
            /// Removes this node from normal flex flow.
            ///
            /// The node is positioned by Taffy's absolute-position algorithm relative to its
            /// nearest flex ancestor. Pair this with inset builders such as [`.top()`](Self::top)
            /// and [`.left()`](Self::left) when the absolute node must anchor to a specific edge.
            pub fn floating(mut self) -> Self {
                self.0.style.position_absolute = true;
                self
            }

            /// Deprecated name for [`floating`](Self::floating).
            #[deprecated(note = "use .floating(...) instead")]
            pub fn absolute(self) -> Self {
                self.floating()
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
    "Vertical container: children stack from top to bottom.\n\nFormerly `Column`; `Column` remains a compatibility alias. Spacing defaults to zero and size is automatic.\n\n```\nuse lemon_core::{ColumnView, Text};\nlet column = ColumnView::new().spacing(12.0).child(Text::new(\"Hello\")).into_element();\n```"
);
container_builder!(
    RowView,
    Row,
    false,
    "Horizontal container: children stack from left to right.\n\nFormerly `Row`; `Row` remains a compatibility alias. Spacing defaults to zero and size is automatic.\n\n```\nuse lemon_core::{RowView, Text};\nlet row = RowView::new().spacing(12.0).child(Text::new(\"Hello\")).into_element();\n```"
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
