//! Composed, themed widgets built on `lemon-core`.
//!
//! Use the `lemon` facade in applications. This crate owns controls rather than
//! reconciliation, layout, or native window behavior.

pub use lemon_core::*;

pub mod widget;

/// Common builders, reactive hooks, and composed UI controls.
pub mod prelude {
    pub use crate::widget::{
        Button, Image, Scroll, ScrollStyle, Select, SelectStyle, Slider, SliderStyle,
        TextFieldState, TextInput, TextInputStyle,
    };
    pub use lemon_core::prelude::*;
}

pub use widget::{
    Button, Image, Scroll, ScrollStyle, Select, SelectStyle, Slider, SliderStyle, TextFieldState,
    TextInput, TextInputStyle,
};
