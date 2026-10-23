use crate::{
    animation::{AnimSlot, AnimationHandle},
    prelude::*,
    AnimRegistry, AnimationConfig, Cursor,
};
use std::time::Duration;

/// Identity returned by [`Toaster::show`], scoped to its originating toaster.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToastId(u64);

#[derive(Clone)]
struct Toast {
    id: ToastId,
    message: String,
    timer: AnimationHandle,
    duration: Duration,
}

#[derive(Clone, Default)]
struct Queue {
    next: u64,
    entries: Vec<Toast>,
}

/// A cloneable notification controller and themed in-window toast stack.
/// Declare once in a stable hook slot, clone into actions, and mount one copy.
/// Notifications appear at the window's bottom-right, fade in/out, expire after
/// four seconds by default, and can be dismissed with the close button.
/// Timers use the originating host's animation clock; no threads or global queue.
///
/// ```
/// use lemon::prelude::*;
/// let cx = Cx::new();
/// let toaster = Toaster::new(&cx);
/// let notifications = toaster.clone();
/// let content = View::new()
///     .child(Button::new(&cx, "Save").action(move || { notifications.show("Saved"); }))
///     .child(toaster);
/// ```
#[derive(Clone)]
pub struct Toaster {
    queue: State<Queue>,
    animations: AnimRegistry,
    theme: crate::Theme,
}

impl Toaster {
    /// Declares one persistent queue (one hook slot); queues are independent per instance.
    pub fn new(cx: &Cx) -> Self {
        Self {
            queue: cx.state(Queue::default()),
            animations: cx.animations(),
            theme: cx.theme(),
        }
    }

    /// Shows a four-second notification and returns its identity for explicit dismissal.
    pub fn show(&self, message: impl Into<String>) -> ToastId {
        self.show_for(message, Duration::from_secs(4))
    }

    /// Shows a notification for `duration`, including its fades. Zero expires immediately.
    /// Expired entries are reclaimed on the next enqueue; hidden stacks do not render them.
    pub fn show_for(&self, message: impl Into<String>, duration: Duration) -> ToastId {
        let timer = self
            .animations
            .register(AnimSlot::new(AnimationConfig::new(duration)));
        timer.play();
        let mut queue = self.queue.get();
        queue.entries.retain(|toast| toast.timer.progress() < 1.0);
        let id = ToastId(queue.next);
        queue.next = queue.next.checked_add(1).expect("toast identity exhausted");
        queue.entries.push(Toast {
            id,
            message: message.into(),
            timer,
            duration,
        });
        self.queue.set(queue);
        id
    }

    /// Removes one notification immediately. Unknown or already removed identities are ignored.
    pub fn dismiss(&self, id: ToastId) {
        self.queue
            .update(|q| q.entries.retain(|toast| toast.id != id));
    }

    /// Removes all notifications immediately.
    pub fn clear(&self) {
        self.queue.update(|q| q.entries.clear());
    }

    /// Builds the notification stack. Empty areas pass input through to the application.
    /// Mount one stack for each controller; it stays below modal dialogs.
    pub fn into_element(self) -> Element {
        let mut stack = ColumnView::new()
            .position(Position::Fixed)
            .bottom(16.0)
            .right(16.0)
            .layer(50)
            .width(300.0)
            .spacing(8.0);
        for toast in self.queue.get().entries {
            let progress = toast.timer.progress();
            if progress >= 1.0 {
                continue;
            }
            let elapsed = progress * toast.duration.as_secs_f32();
            let remaining = (1.0 - progress) * toast.duration.as_secs_f32();
            let opacity = (elapsed / 0.14).min(remaining / 0.14).clamp(0.0, 1.0);
            let dismiss = self.clone();
            let id = toast.id;
            let key_dismiss = dismiss.clone();
            let close = View::new()
                .width(24.0)
                .height(24.0)
                .center()
                .focusable()
                .cursor(Cursor::Pointer)
                .action(move || dismiss.dismiss(id))
                .on_key_down(move |event| {
                    if event.state == KeyState::Pressed
                        && !event.repeat
                        && matches!(
                            event.key,
                            LemonKey::Named(NamedKey::Enter | NamedKey::Space)
                        )
                    {
                        key_dismiss.dismiss(id);
                    }
                })
                .child(
                    Icon::new(crate::icons::lucide::X)
                        .size(14.0)
                        .color(self.theme.colors.foreground_secondary),
                );
            let mut close_element = close.into_element();
            if let Element::View(node) = &mut close_element {
                node.paint.focus_ring_color = Some(self.theme.colors.accent.into());
            }
            stack = stack.child(
                RowView::new()
                    .key(id.0)
                    .blocks_input(true)
                    .fill_width()
                    .inset(12.0)
                    .spacing(8.0)
                    .align(Align::Center)
                    .opacity(opacity)
                    .surface(self.theme.colors.surface)
                    .border(self.theme.colors.border, 1.0)
                    .rounding(self.theme.radius.md)
                    .child(
                        View::new()
                            .min_width(0.0)
                            .grow(1.0)
                            .child(Text::new(toast.message)),
                    )
                    .child(close_element),
            );
        }
        stack.into_element()
    }
}

impl From<Toaster> for Element {
    fn from(value: Toaster) -> Element {
        value.into_element()
    }
}
