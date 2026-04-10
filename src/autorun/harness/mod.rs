//! Harness abstraction: feature-gated agent CLI implementations.

mod shared;
pub use shared::*;

#[cfg(feature = "claude")]
mod claude;
#[cfg(feature = "claude")]
pub use claude::*;

#[cfg(feature = "cursor")]
mod cursor;
#[cfg(feature = "cursor")]
pub use cursor::*;
