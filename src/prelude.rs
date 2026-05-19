//! Commonly used types for Lemon applications.
//!
//! Import everything you need for a typical app with:
//!
//! ```
//! use lemon::prelude::*;
//! ```
//!
//! Lower-level APIs ([`Runtime`], [`layout_pass`], [`paint_pass`], …) stay on the crate root
//! and are intentionally omitted here.

pub use crate::animation::{AnimationConfig, AnimationHandle, Easing};
pub use crate::children;
pub use crate::element::builders::{Column, Component, Row, Text, View};
pub use crate::element::events::{Cursor, KeyEvent, KeyState, LemonKey, Modifiers, NamedKey};
pub use crate::element::style::{Align, Color, Justify, Overflow};
pub use crate::element::Element;
pub use crate::platform::{run, WindowConfig};
pub use crate::runtime::cx::{Cx, OpenWindowParams};
pub use crate::runtime::signal::Signal;
pub use crate::widget::{
    Button, Image, Scroll, ScrollStyle, Select, SelectStyle, Slider, SliderStyle, TextFieldState,
    TextInput, TextInputStyle,
};
