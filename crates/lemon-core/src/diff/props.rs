use crate::element::types::BoxElement;

use super::{NodePath, Patch};

pub(crate) fn diff_container_props(
    old: &BoxElement,
    new: &BoxElement,
    path: &NodePath,
    patches: &mut Vec<Patch>,
) {
    if old.handlers != new.handlers {
        patches.push(Patch::UpdateHandlers {
            node: path.clone(),
            handlers: new.handlers.clone(),
        });
    }
    if old.style != new.style {
        patches.push(Patch::UpdateStyle {
            node: path.clone(),
            style: new.style.clone(),
        });
    }
    let old_paint = old.paint.resolve();
    let new_paint = new.paint.resolve();
    if old_paint != new_paint {
        patches.push(Patch::UpdatePaint {
            node: path.clone(),
            paint: new_paint,
        });
    }
}

/// Emit chrome after child updates so caret synchronization sees the new text.
pub(crate) fn diff_container_chrome(
    old: &BoxElement,
    new: &BoxElement,
    path: &NodePath,
    patches: &mut Vec<Patch>,
) {
    if old.text_input != new.text_input
        || old.scroll_viewport != new.scroll_viewport
        || old.scroll_bar != new.scroll_bar
    {
        patches.push(Patch::UpdateWidgetChrome {
            node: path.clone(),
            text_input: new.text_input.clone(),
            scroll_viewport: new.scroll_viewport,
            scroll_bar: new.scroll_bar,
        });
    }
}
