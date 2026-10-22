//! Single-threaded reactive cells, computed values, and effects.
//!
//! This crate has no windowing, layout, or rendering dependencies.
//!
//! ```
//! use lemon_reactive::{Derived, Signal};
//! let count = Signal::new(2);
//! let source = count.clone();
//! let doubled = Derived::new(move || source.get() * 2);
//! assert_eq!(doubled.get(), 4);
//! count.set(3);
//! assert_eq!(doubled.get(), 6);
//! ```

pub mod derived;
pub mod effect;
pub mod observer;
pub mod signal;

pub use derived::Derived;
pub use effect::Effect;
pub use signal::Signal;
