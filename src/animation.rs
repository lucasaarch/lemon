//! Per-frame animation support for widget authors and headless hosts.
//!
//! Use [`Cx::motion`](crate::Cx::motion) for component-owned animations. Custom
//! hosts own an [`AnimRegistry`] through [`HostContext::animations`](crate::host::HostContext::animations),
//! tick it once per frame and present while it has active slots or pending requests.
//! Animation handles retain the registry that created them.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::{Duration, Instant};

use crate::reactive::signal::Signal;

/// Easing curve applied to an animation's normalized progress.
///
/// Use [`Linear`](Self::Linear) for constant-speed interpolation, or one of the ease variants
/// for simple acceleration and deceleration curves.
///
/// # Examples
///
/// ```
/// use lemon::Easing;
///
/// assert_eq!(Easing::Linear.sample(0.5), 0.5);
/// assert_eq!(Easing::EaseIn.sample(2.0), 1.0);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Easing {
    /// Constant-speed interpolation.
    #[default]
    Linear,
    /// Starts slowly and accelerates toward the end.
    EaseIn,
    /// Starts quickly and decelerates toward the end.
    EaseOut,
    /// Starts and ends slowly, with faster motion in the middle.
    EaseInOut,
}

impl Easing {
    /// Returns the eased value for `progress`, clamped to the range `[0.0, 1.0]`.
    pub fn sample(self, progress: f32) -> f32 {
        let t = progress.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::EaseIn => t * t,
            Self::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            Self::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - ((-2.0 * t + 2.0).powi(2) / 2.0)
                }
            }
        }
    }
}

/// Configuration used when creating an [`AnimationHandle`].
///
/// The default animation lasts 150 ms and uses [`Easing::Linear`]. A zero duration is allowed and
/// completes immediately on the next [`AnimationHandle::progress`] read.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use lemon::{AnimationConfig, Easing};
///
/// let config = AnimationConfig::new(Duration::from_millis(250)).easing(Easing::EaseOut);
/// assert_eq!(config.duration(), Duration::from_millis(250));
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimationConfig {
    duration: Duration,
    easing: Easing,
}

impl AnimationConfig {
    /// Creates a configuration with `duration` and [`Easing::Linear`].
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            easing: Easing::Linear,
        }
    }

    /// Sets the easing curve used to transform normalized progress.
    pub fn easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    /// Returns the configured animation duration.
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// Returns the configured easing curve.
    pub fn easing_curve(&self) -> Easing {
        self.easing
    }
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self::new(Duration::from_millis(150))
    }
}

/// Stored state for a single animation registered with an [`AnimRegistry`].
///
/// Most app code should create slots through [`Cx::motion`](crate::Cx::motion) or
/// [`AnimRegistry::register`]. The slot stores raw normalized progress; callers read eased
/// progress through [`AnimationHandle::progress`].
///
/// # Examples
///
/// ```
/// use lemon::{AnimRegistry, AnimSlot, AnimationConfig};
///
/// let handle = AnimRegistry::default().register(AnimSlot::new(AnimationConfig::default()));
/// assert_eq!(handle.progress(), 0.0);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct AnimSlot {
    config: AnimationConfig,
    raw_progress: f32,
    started_at: Option<Instant>,
}

impl AnimSlot {
    /// Creates an idle animation slot at progress `0.0`.
    pub fn new(config: AnimationConfig) -> Self {
        Self {
            config,
            raw_progress: 0.0,
            started_at: None,
        }
    }

    /// Returns the configuration used by this slot.
    pub fn config(&self) -> AnimationConfig {
        self.config
    }

    /// Returns the current raw normalized progress without applying easing.
    pub fn raw_progress(&self) -> f32 {
        self.raw_progress
    }

    /// Returns `true` while the slot has an active clock.
    pub fn is_playing(&self) -> bool {
        self.started_at.is_some()
    }

    fn update(&mut self, now: Instant) {
        let Some(started_at) = self.started_at else {
            return;
        };
        let next = if self.config.duration.is_zero() {
            1.0
        } else {
            now.saturating_duration_since(started_at).as_secs_f32()
                / self.config.duration.as_secs_f32()
        };
        self.raw_progress = next.clamp(0.0, 1.0);
        if self.raw_progress >= 1.0 {
            self.started_at = None;
        }
    }

    fn eased_progress(&self) -> f32 {
        self.config.easing.sample(self.raw_progress)
    }
}

/// Cloneable controller for an animation slot.
///
/// Handles are cheap to clone and share the same underlying [`AnimSlot`]. Calling
/// [`progress`](Self::progress) while the animation is playing subscribes the current render to
/// the frame signal and requests another frame.
///
/// # Examples
///
/// ```no_run
/// use lemon::prelude::*;
///
/// fn view(cx: &Cx) -> Element {
///     let fade = cx.motion(AnimationConfig::default());
///     fade.play();
///     Text::new(move || format!("{:.2}", fade.progress())).into_element()
/// }
/// ```
#[derive(Clone)]
pub struct AnimationHandle {
    slot: Rc<RefCell<AnimSlot>>,
    active_slots: Rc<RefCell<Vec<Weak<RefCell<AnimSlot>>>>>,
    frame: Signal<u32>,
    pending: Rc<Cell<bool>>,
}

impl AnimationHandle {
    fn mark_active(&self) {
        let weak = Rc::downgrade(&self.slot);
        let mut active = self.active_slots.borrow_mut();
        if !active.iter().any(|slot| slot.ptr_eq(&weak)) {
            active.push(weak);
        }
    }

    /// Returns eased progress in the range `[0.0, 1.0]`.
    pub fn progress(&self) -> f32 {
        let now = Instant::now();
        let (progress, playing) = {
            let mut slot = self.slot.borrow_mut();
            slot.update(now);
            (slot.eased_progress(), slot.is_playing())
        };
        if playing {
            self.frame.get();
            self.pending.set(true);
        }
        progress
    }

    /// Starts or resumes the animation.
    ///
    /// If the animation is complete, playback restarts from `0.0`. If it is paused after
    /// [`reset`](Self::reset), playback starts from `0.0`.
    pub fn play(&self) {
        let now = Instant::now();
        let mut slot = self.slot.borrow_mut();
        slot.update(now);
        if slot.raw_progress >= 1.0 {
            slot.raw_progress = 0.0;
        }
        let elapsed = slot.config.duration.mul_f32(slot.raw_progress);
        slot.started_at = Some(now.checked_sub(elapsed).unwrap_or(now));
        drop(slot);
        self.mark_active();
        self.pending.set(true);
    }

    /// Stops the animation and returns progress to `0.0`.
    pub fn reset(&self) {
        let mut slot = self.slot.borrow_mut();
        slot.raw_progress = 0.0;
        slot.started_at = None;
        drop(slot);
        self.pending.set(true);
        self.active_slots.borrow_mut().retain(|active| {
            active
                .upgrade()
                .is_some_and(|slot| !Rc::ptr_eq(&slot, &self.slot))
        });
    }

    /// Returns `true` while this handle is actively playing.
    pub fn is_playing(&self) -> bool {
        let mut slot = self.slot.borrow_mut();
        slot.update(Instant::now());
        slot.is_playing()
    }

    /// Returns the current raw normalized progress without applying easing.
    pub fn raw_progress(&self) -> f32 {
        let mut slot = self.slot.borrow_mut();
        slot.update(Instant::now());
        slot.raw_progress()
    }
}

/// Registry that owns animation slots and frame requests for one host.
///
/// [`Cx::motion`](crate::Cx::motion) registers slots with its originating host.
/// Custom hosts can use [`default`](Self::default) for an independent registry,
/// or obtain an existing host's registry with [`HostContext::animations`](crate::host::HostContext::animations).
///
/// # Examples
///
/// ```
/// use lemon::{AnimRegistry, AnimSlot, AnimationConfig};
///
/// let registry = AnimRegistry::default();
/// let handle = registry.register(AnimSlot::new(AnimationConfig::default()));
/// handle.play();
/// assert!(handle.is_playing());
/// ```
#[derive(Clone)]
pub struct AnimRegistry {
    slots: Rc<RefCell<Vec<Weak<RefCell<AnimSlot>>>>>,
    active_slots: Rc<RefCell<Vec<Weak<RefCell<AnimSlot>>>>>,
    frame: Signal<u32>,
    pending: Rc<Cell<bool>>,
}

impl AnimRegistry {
    /// Returns the frame signal for this registry.
    pub fn frame_signal(&self) -> Signal<u32> {
        self.frame.clone()
    }

    /// Requests a frame from this registry's host.
    pub fn request_frame(&self) {
        self.pending.set(true);
    }

    /// Clears and returns this registry's pending frame request.
    pub fn take_pending(&self) -> bool {
        self.pending.replace(false)
    }

    /// Advances this registry's frame signal, notifying its subscribers.
    pub fn tick_frame(&self) {
        self.frame.update(|frame| *frame = frame.wrapping_add(1));
    }

    /// Registers `slot` and returns a handle that controls it.
    pub fn register(&self, slot: AnimSlot) -> AnimationHandle {
        let slot = Rc::new(RefCell::new(slot));
        self.slots.borrow_mut().push(Rc::downgrade(&slot));
        AnimationHandle {
            slot,
            active_slots: self.active_slots.clone(),
            frame: self.frame.clone(),
            pending: self.pending.clone(),
        }
    }

    /// Returns the number of live slots currently tracked by this registry.
    pub fn live_slots(&self) -> usize {
        let mut slots = self.slots.borrow_mut();
        slots.retain(|slot| slot.strong_count() > 0);
        slots.len()
    }

    /// Returns the number of live animations currently playing.
    pub fn active_slots(&self) -> usize {
        let mut active = self.active_slots.borrow_mut();
        active.retain(|slot| {
            slot.upgrade()
                .is_some_and(|slot| slot.borrow().is_playing())
        });
        active.len()
    }

    /// Advances playing animations to `now`, returning whether another tick is needed.
    ///
    /// Custom hosts call this once per frame using a monotonic clock.
    pub fn tick_active(&self, now: Instant) -> bool {
        let mut active = self.active_slots.borrow_mut();
        active.retain(|slot| {
            let Some(slot) = slot.upgrade() else {
                return false;
            };
            let mut slot = slot.borrow_mut();
            slot.update(now);
            slot.is_playing()
        });
        !active.is_empty()
    }
}

impl Default for AnimRegistry {
    fn default() -> Self {
        Self {
            slots: Rc::default(),
            active_slots: Rc::default(),
            frame: Signal::new(0),
            pending: Rc::new(Cell::new(false)),
        }
    }
}

impl std::fmt::Debug for AnimationHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnimationHandle")
            .field("slot", &self.slot)
            .finish_non_exhaustive()
    }
}
impl std::fmt::Debug for AnimRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnimRegistry")
            .field("slots", &self.live_slots())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easing_curves_clamp_and_sample_expected_points() {
        assert_eq!(Easing::Linear.sample(-1.0), 0.0);
        assert_eq!(Easing::Linear.sample(2.0), 1.0);
        assert_eq!(Easing::EaseIn.sample(0.5), 0.25);
        assert_eq!(Easing::EaseOut.sample(0.5), 0.75);
        assert_eq!(Easing::EaseInOut.sample(0.5), 0.5);
    }

    #[test]
    fn registry_registers_live_slot() {
        let registry = AnimRegistry::default();
        assert_eq!(registry.live_slots(), 0);

        let handle = registry.register(AnimSlot::new(AnimationConfig::default()));
        assert_eq!(registry.live_slots(), 1);

        drop(handle);
        assert_eq!(registry.live_slots(), 0);
    }

    #[test]
    fn handle_progress_applies_easing() {
        let registry = AnimRegistry::default();
        let handle = registry.register(AnimSlot {
            config: AnimationConfig::new(Duration::from_secs(1)).easing(Easing::EaseIn),
            raw_progress: 0.0,
            started_at: Some(Instant::now() - Duration::from_millis(500)),
        });

        let progress = handle.progress();
        assert!((0.20..=0.35).contains(&progress), "progress was {progress}");
        assert!(handle.is_playing());
    }

    #[test]
    fn handle_progress_completes_elapsed_animation() {
        let registry = AnimRegistry::default();
        let handle = registry.register(AnimSlot {
            config: AnimationConfig::new(Duration::from_millis(10)),
            raw_progress: 0.0,
            started_at: Some(Instant::now() - Duration::from_secs(1)),
        });

        assert_eq!(handle.progress(), 1.0);
        assert!(!handle.is_playing());
    }

    #[test]
    fn play_and_reset_control_slot_state() {
        let registry = AnimRegistry::default();
        let handle = registry.register(AnimSlot::new(AnimationConfig::new(Duration::from_secs(1))));

        assert_eq!(handle.progress(), 0.0);
        handle.play();
        assert!(handle.is_playing());

        handle.reset();
        assert_eq!(handle.progress(), 0.0);
        assert!(!handle.is_playing());
    }
}
