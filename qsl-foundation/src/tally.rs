// SPDX-License-Identifier: AGPL-3.0-or-later
//! A running total one check holds in its own scope.

use core::sync::atomic::{AtomicU64, Ordering};

/// A running total of counted units that a shared reference adds to, so every
/// pass over one scope counts into the same total and the scope stays `Sync`.
/// It is a field of one check's scope, never global state: a new scope starts
/// at zero, and a clone starts from the total its source had reached.
#[derive(Debug, Default)]
pub struct Tally(AtomicU64);

impl Tally {
    /// Add `count` to the total, saturating, and return the new total.
    pub fn add(&self, count: u64) -> u64 {
        let previous = self
            .0
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |total| {
                Some(total.saturating_add(count))
            })
            .unwrap_or_else(|total| total);
        previous.saturating_add(count)
    }
}

impl Clone for Tally {
    fn clone(&self) -> Self {
        Self(AtomicU64::new(self.0.load(Ordering::Relaxed)))
    }
}
