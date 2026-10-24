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
pub use crate::element::builders::{
    ColumnView, Component, Icon, Image, RowView, ScrollView, ScrollViewStyle, Text, View,
};
pub use crate::element::events::{
    Cursor, KeyEvent, KeyState, LemonKey, Modifiers, NamedKey, ScrollAxis, ScrollDelta,
    ScrollOffset,
};
pub use crate::element::style::{
    Align, Color, Distribute, Gradient, GradientStop, LinearGradient, Overflow, Placement,
    Position, RadialGradient,
};
pub use crate::element::types::TextAlignment;
pub use crate::element::Element;
pub use crate::platform::{run, AppCx};
pub use crate::runtime::cx::Cx;
pub use crate::widget::{
    Button, ButtonStyle, Checkbox, Dialog, FileTree, FileTreeNode, FileTreeStyle, Popover,
    SecureField, Select, SelectStyle, Slider, SliderStyle, Switch, TextArea, TextField,
    TextFieldStyle, ToastId, ToastKind, Toaster,
};
pub use crate::window::{WindowChrome, WindowConfig};
pub use crate::State;

pub use crate::element::decoration::Decorate;

pub use crate::transition::{Transition, TransitionProperty};

pub use crate::icons::{lucide, IconData};
