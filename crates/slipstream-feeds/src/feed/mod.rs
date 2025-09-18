//! Feed management.

use std::hash::Hash;

use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

use super::*;

mod feed_attributes;
mod feed_id;
mod feed_ref;
mod feed_trait;
mod types;

pub use feed_attributes::*;
pub use feed_id::*;
pub use feed_ref::*;
pub use feed_trait::*;
pub use types::*;
