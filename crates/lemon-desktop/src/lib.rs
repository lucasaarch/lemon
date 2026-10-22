//! Native window, input, and GPU presentation integration for Lemon.
//!
//! Most applications should use the `lemon` facade.

pub use lemon_core::*;

pub mod platform;

pub use platform::{
    run, run_with_fonts, run_with_theme, run_with_theme_and_fonts, AppState, WindowConfig,
    WindowState,
};
