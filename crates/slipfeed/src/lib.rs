//! slipfeed feed management.

mod entry;
mod feed;
mod updater;

#[cfg(test)]
mod tests;

pub use entry::*;
pub use feed::*;
pub use updater::*;
