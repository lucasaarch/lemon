//! Commonly used types for Lemon applications.
//!
//! Import everything you need for a typical app with:
//!
//! ```
//! use lemon::prelude::*;
//! ```
//!
//! Lower-level APIs ([`crate::Runtime`], [`crate::layout_pass`], [`crate::paint_pass`], …) stay on the crate root
//! and are intentionally omitted here.

pub use crate::animation::{AnimationConfig, AnimationHandle, Easing};
pub use crate::children;
pub use crate::element::builders::{ColumnView, Component, RowView, Text, View};
pub use crate::element::events::{Cursor, KeyEvent, KeyState, LemonKey, Modifiers, NamedKey};
pub use crate::element::style::{Align, Color, Distribution, Justify, Overflow, Placement};
pub use crate::element::Element;
pub use crate::host::OpenWindowParams;
pub use crate::platform::{run, WindowConfig};
pub use crate::runtime::cx::Cx;
pub use crate::widget::{
    Button, Image, Scroll, ScrollStyle, SecureField, Select, SelectStyle, Slider, SliderStyle,
    TextField, TextFieldState, TextFieldStyle,
};
pub use crate::Signal;
pub use crate::State;

pub use crate::element::decoration::Decorate;
