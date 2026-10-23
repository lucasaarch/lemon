//! Single-threaded reactive cells, computed values, and effects.
//!
//! These primitives depend only on the standard library and do not import UI or platform modules.
//!
//! ```
//! use lemon::reactive::{Derived, State};
//! let count = State::new(2);
//! let source = count.clone();
//! let doubled = Derived::new(move || source.get() * 2);
//! assert_eq!(doubled.get(), 4);
//! count.set(3);
//! assert_eq!(doubled.get(), 6);
//! ```

pub mod derived;

pub mod effect;

pub mod observer;

pub mod state;

pub use derived::Derived;
pub use effect::Effect;
pub use state::State;
