//! Per-frame animation support for widget authors.
//!
//! Widgets that need to animate over time (e.g. a sliding fill, a fading transition) use two
//! primitives:
//!
//! - [`animation_frame_signal`] — a [`Signal<u32>`] that the platform increments once per frame
//!   while at least one widget has requested an animation tick.  Read it from `into_element` to
//!   subscribe the component to frame events so the runtime re-renders on each tick.
//! - [`request_animation_frame`] — call this from `into_element` when the current animation has
//!   not yet reached its target.  The platform will call `request_redraw` and advance the frame
//!   signal on the next `about_to_wait`.
//!
//! Both primitives are thread-local and single-window.  They are intentionally low-level; a
//! higher-level `use_animation` hook is planned for v0.5.

use std::cell::{Cell, RefCell};

use crate::runtime::signal::Signal;

thread_local! {
    static FRAME_SIGNAL: RefCell<Option<Signal<u32>>> = const { RefCell::new(None) };
    static PENDING: Cell<bool> = const { Cell::new(false) };
}

/// Returns the shared frame-tick [`Signal<u32>`].
///
/// The platform increments this signal once per frame while animation is active.  Call this from
/// `into_element` and read the returned signal to subscribe the current component to frame events:
///
/// ```ignore
/// if progress < 1.0 {
///     lemon::animation_frame_signal().get(); // subscribe
///     lemon::request_animation_frame();      // keep ticking
/// }
/// ```
pub fn animation_frame_signal() -> Signal<u32> {
    FRAME_SIGNAL.with(|cell| {
        let mut guard = cell.borrow_mut();
        if guard.is_none() {
            *guard = Some(Signal::new(0u32));
        }
        guard.as_ref().unwrap().clone()
    })
}

/// Requests that the platform render one more frame and advance the animation clock.
///
/// Call this from `into_element` whenever your animation has not yet converged.  The platform
/// will respond by incrementing [`animation_frame_signal`] and calling `request_redraw`, which
/// drives the reactive re-render needed to advance the interpolation.
pub fn request_animation_frame() {
    PENDING.with(|f| f.set(true));
}

/// Returns `true` if [`request_animation_frame`] has been called since the last
/// [`take_animation_pending`].
pub(crate) fn animation_pending() -> bool {
    PENDING.with(Cell::get)
}

/// Clears the pending flag and returns its previous value.
pub(crate) fn take_animation_pending() -> bool {
    PENDING.with(|f| {
        let was = f.get();
        f.set(false);
        was
    })
}

/// Increments the frame signal by one, triggering a reactive re-render in any component that
/// subscribed to it via [`animation_frame_signal`].
pub(crate) fn tick_animation_frame() {
    FRAME_SIGNAL.with(|cell| {
        if let Some(signal) = cell.borrow().as_ref() {
            signal.update(|v| *v = v.wrapping_add(1));
        }
    });
}
