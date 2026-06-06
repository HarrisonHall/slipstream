//! Transform.

use super::*;

/// A transform is a function that takes an entry and modified is.
pub type Transform = Arc<dyn Fn(&mut Entry) -> () + Send + Sync>;
