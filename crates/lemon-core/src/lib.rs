//! Headless Lemon UI engine: elements, reconciliation, retained state, layout, and scenes.
//!
//! Native windows and GPU presentation belong to `lemon-desktop`. Applications
//! should normally depend on the `lemon` facade instead of this integration API.

mod host_requests;
mod macros;

pub mod animation;
pub mod asset;
pub mod debug;
pub mod diff;
pub mod element;
pub mod font;
pub mod host;
pub mod layout;
pub mod paint;
pub mod retained;
pub mod runtime;
pub mod theme;
pub mod ui;

/// Common builders and state for headless UI construction.
pub mod prelude {
    pub use crate::animation::{AnimationConfig, AnimationHandle, Easing};
    pub use crate::children;
    pub use crate::element::builders::{Column, Component, Row, Text, View};
    pub use crate::element::events::{Cursor, KeyEvent, KeyState, LemonKey, Modifiers, NamedKey};
    pub use crate::element::style::{Align, Color, Justify, Overflow};
    pub use crate::element::Element;
    pub use crate::runtime::cx::{Cx, OpenWindowParams};
    pub use crate::runtime::signal::Signal;
}

pub use animation::{
    animation_frame_signal, request_animation_frame, AnimRegistry, AnimSlot, AnimationConfig,
    AnimationHandle, Easing,
};
pub use asset::ImageHandle;
pub use element::builders::{Column, Component, Row, Text, View};
pub use element::events::{Cursor, KeyEvent, KeyState, LemonKey, Modifiers, NamedKey};
pub use element::style::{BoxShadow, Color, LinearGradient, Overflow, StyleProps};
pub use font::{register_font_bytes, register_font_path, FontRegistrationError};
pub use layout::{
    layout_pass, layout_pass_if_dirty, measure_element_height,
    measure_element_height_with_font_context, scroll_content_max_offset, sync_scroll_layout_max,
    LayoutMap, LayoutRect, Viewport,
};
pub use paint::{paint_pass, PaintStats};
pub use retained::RetainedTree;
pub use runtime::cx::{Cx, OpenWindowParams};
pub use runtime::signal::Signal;
pub use runtime::Runtime;
pub use theme::{
    current_theme, set_active_theme, ColorTokens, RadiusTokens, SpacingTokens, Theme,
    TypographyTokens, WidgetChromeTokens,
};
