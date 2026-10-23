//! AppKit integration kept outside the element, layout and paint layers.

use objc2::rc::Retained;
use objc2_app_kit::{NSButton, NSView, NSWindowButton, NSWindowStyleMask};
use objc2_foundation::{MainThreadMarker, NSPoint, NSSize};
use winit::{
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::Window,
};

pub(super) fn position_traffic_lights(window: &Window, position: Option<(f32, f32)>) {
    let Some((x, y)) =
        position.filter(|(x, y)| x.is_finite() && y.is_finite() && *x >= 0.0 && *y >= 0.0)
    else {
        return;
    };
    // Native windows and their views are only accessed during main-thread host callbacks.
    let _main_thread =
        MainThreadMarker::new().expect("AppKit window updates require the main thread");
    let Ok(handle) = window.window_handle() else {
        return;
    };
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return;
    };
    // SAFETY: winit owns this NSView and the window remains borrowed for this call.
    // All AppKit access below runs on the main thread; no raw view escapes this scope.
    let view = unsafe { &*handle.ns_view.as_ptr().cast::<NSView>() };
    let Some(native) = view.window() else {
        return;
    };
    if native.styleMask().contains(NSWindowStyleMask::FullScreen) {
        return;
    }
    let buttons: Vec<Retained<NSButton>> = [
        NSWindowButton::NSWindowCloseButton,
        NSWindowButton::NSWindowMiniaturizeButton,
        NSWindowButton::NSWindowZoomButton,
    ]
    .into_iter()
    .filter_map(|kind| native.standardWindowButton(kind))
    .collect();
    let Some(close) = buttons.first() else {
        return;
    };
    // SAFETY: these retained superviews belong to the live window on the main thread.
    let Some(titlebar) = (unsafe { close.superview() }) else {
        return;
    };
    let Some(container) = (unsafe { titlebar.superview() }) else {
        return;
    };
    let frames: Vec<_> = buttons.iter().map(|button| button.frame()).collect();
    let close_frame = frames[0];
    let group_width = frames
        .iter()
        .map(|frame| frame.origin.x + frame.size.width)
        .fold(0.0_f64, f64::max)
        - close_frame.origin.x;
    let button_height = frames
        .iter()
        .map(|frame| frame.size.height)
        .fold(0.0_f64, f64::max);
    let window_size = native.frame().size;
    let position = group_position((x, y), window_size, NSSize::new(group_width, button_height));
    // Expand the native titlebar's hit-test region when the group moves farther down.
    // Keeping the original bottom inset avoids clipping native buttons or their hover area.
    let bottom = if titlebar.isFlipped() {
        titlebar.bounds().size.height - close_frame.origin.y - close_frame.size.height
    } else {
        close_frame.origin.y
    };
    let mut container_frame = container.frame();
    container_frame.size.height = container_frame
        .size
        .height
        .max(position.y + button_height + bottom.max(0.0))
        .min(window_size.height);
    container_frame.origin.y = window_size.height - container_frame.size.height;
    let mut titlebar_frame = titlebar.frame();
    titlebar_frame.size.height = container_frame.size.height;
    // SAFETY: finite coordinates and existing view sizes are used on the main thread.
    // Avoid redundant setters: AppKit invalidates layout/display when frames change.
    unsafe {
        if container.frame() != container_frame {
            container.setFrame(container_frame);
        }
        if titlebar.frame() != titlebar_frame {
            titlebar.setFrame(titlebar_frame);
        }
        for (button, original) in buttons.iter().zip(frames) {
            let point = NSPoint::new(
                position.x + original.origin.x - close_frame.origin.x,
                window_size.height - position.y - original.size.height,
            );
            let origin = titlebar.convertPoint_fromView(point, None);
            if button.frame().origin != origin {
                button.setFrameOrigin(origin);
            }
        }
    }
}

fn group_position(requested: (f32, f32), window: NSSize, group: NSSize) -> NSPoint {
    NSPoint::new(
        f64::from(requested.0).min((window.width - group.width).max(0.0)),
        f64::from(requested.1).min((window.height - group.height).max(0.0)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn positions_use_logical_points_and_clamp_to_available_window_space() {
        let group = NSSize::new(60.0, 14.0);
        assert_eq!(
            group_position((20.5, 24.0), NSSize::new(640.0, 400.0), group),
            NSPoint::new(20.5, 24.0)
        );
        assert_eq!(
            group_position((1000.0, 1000.0), NSSize::new(100.0, 80.0), group),
            NSPoint::new(40.0, 66.0)
        );
        assert_eq!(
            group_position((20.0, 20.0), NSSize::new(10.0, 10.0), group),
            NSPoint::new(0.0, 0.0)
        );
    }
}
