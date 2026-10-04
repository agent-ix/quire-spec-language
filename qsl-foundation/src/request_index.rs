// SPDX-License-Identifier: AGPL-3.0-or-later
//! The one typed `request_index`: an item's position in one QSpec FR-331
//! request, shared by the request writer (`qsl-route`) and the terminal
//! record (`qsl-replay`) so the two join on the index and never on text.

/// An item's position in one request (QSpec FR-331 `request_index`). Each
/// item, bounded or not, has its own, and a backend reports exactly one
/// terminal record per index.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RequestIndex(usize);

impl RequestIndex {
    /// The index at zero-based `position`.
    pub fn new(position: usize) -> Self {
        Self(position)
    }

    /// The zero-based position.
    pub fn get(self) -> usize {
        self.0
    }
}
