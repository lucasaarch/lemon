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

pub use crate::element::builders::{Button, Column, Component, Row, Text, View};
pub use crate::element::events::{Cursor, KeyEvent, KeyState, LemonKey, Modifiers, NamedKey};
pub use crate::element::style::{Align, Color, Justify, Overflow};
pub use crate::element::Element;
pub use crate::platform::{run, WindowConfig};
pub use crate::runtime::cx::Cx;
pub use crate::runtime::signal::Signal;

/// Builds a [`Vec`] of [`Element`] values from heterogeneous widget builders.
pub use crate::children;
