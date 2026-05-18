/// Asset types for Lemon.
///
/// This module contains core asset types used by widgets and the paint layer.
/// The primary entry point is [`ImageHandle`], which represents a decoded image
/// in RGBA8 format backed by a reference-counted allocation.
pub mod image_handle;
pub use image_handle::{ImageError, ImageHandle};
