//! Slipstream modes.

use super::*;

mod config;
mod fetch;
mod read;
mod serve;

pub use config::*;
pub use fetch::*;
pub use read::*;
pub use serve::*;
