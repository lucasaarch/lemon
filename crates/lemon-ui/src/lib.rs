//! Compatibility facade for widgets now owned by `lemon-core`.
//!
//! Applications should use `lemon`; headless integrations can use `lemon-core`.
//! Existing `lemon_ui::widget` and `lemon_ui::prelude` imports remain available.
pub use lemon_core::*;
