//! slipfeed feed management.

mod datetime;
mod entry;
mod feed;
mod filter;
pub mod prelude;
mod tag;
mod updater;

#[cfg(test)]
mod tests;

use prelude::internal::*;
use prelude::*;
