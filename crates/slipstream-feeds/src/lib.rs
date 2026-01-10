//! slipfeed feed management.

use std::collections::HashMap;
use std::collections::HashSet;

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
