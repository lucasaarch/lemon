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
pub mod input;
pub mod layout;
pub mod paint;
pub mod retained;
pub mod runtime;
pub mod theme;
pub mod ui;
pub mod widget;

/// Common builders and state for headless UI construction.
pub mod prelude {
    pub use crate::animation::{AnimationConfig, AnimationHandle, Easing};
    pub use crate::children;
    pub use crate::element::builders::{Column, ColumnView, Component, Row, RowView, Text, View};
    pub use crate::element::decoration::Decorate;
    pub use crate::element::events::{Cursor, KeyEvent, KeyState, LemonKey, Modifiers, NamedKey};
    pub use crate::element::style::{Align, Color, Distribution, Justify, Overflow, Placement};
    pub use crate::element::Element;
    pub use crate::runtime::cx::{Cx, OpenWindowParams};
    pub use crate::runtime::signal::Signal;
    pub use crate::widget::{
        Button, Image, Scroll, ScrollStyle, SecureField, Select, SelectStyle, Slider, SliderStyle,
        TextField, TextFieldState, TextFieldStyle,
    };
    #[allow(deprecated)]
    pub use crate::widget::{TextInput, TextInputStyle};
    pub use crate::State;
}

pub use animation::{
    animation_frame_signal, request_animation_frame, AnimRegistry, AnimSlot, AnimationConfig,
    AnimationHandle, Easing,
};
pub use asset::ImageHandle;
pub use element::builders::{Column, ColumnView, Component, Row, RowView, Text, View};
pub use element::events::{Cursor, KeyEvent, KeyState, LemonKey, Modifiers, NamedKey};
pub use element::style::{
    BoxShadow, Color, Distribution, LinearGradient, Overflow, Placement, StyleProps,
};
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

/// Shared reactive application state, returned by [`Cx::state`].
///
/// Clones refer to the same value. Read with `.get()`, replace with `.set(...)`,
/// or mutate with `.update(...)`. Component-owned state must be declared in a stable
/// order through `cx.state(...)`; `State::new(...)` creates independent external state.
///
/// ```
/// use lemon_core::State;
/// let count = State::new(0);
/// let shared = count.clone();
/// count.update(|value| *value += 1);
/// assert_eq!(shared.get(), 1);
/// ```
pub type State<T> = Signal<T>;
pub use runtime::Runtime;
pub use theme::{
    current_theme, set_active_theme, ColorTokens, RadiusTokens, SpacingTokens, Theme, ThemePair,
    TypographyTokens, WidgetChromeTokens,
};

pub use widget::{
    Button, Image, Scroll, ScrollStyle, SecureField, Select, SelectStyle, Slider, SliderStyle,
    TextField, TextFieldState, TextFieldStyle,
};
#[allow(deprecated)]
pub use widget::{TextInput, TextInputStyle};

pub use crate::element::decoration::Decorate;
