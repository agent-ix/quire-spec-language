// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-336-AC-4 (ADR-013 O-09): the request writer's temporal items over
//! model subjects, each joined to its obligation identity.
//!
//! An item's obligation identity is its subject and its kind
//! ([`ItemObligation`]), and the subject names its universes, so two
//! requests for one clause over different universes are two obligations and
//! two request indices. A result carries the [`RequestIndex`] of the item it
//! answers (QSpec FR-360), so [`ModelRequest::obligation`] joins a result to
//! its own obligation and to no other.

use qsl_foundation::RequestIndex;
use qsl_semantics::family::Requirements;

use crate::deadlock::{ItemObligation, ModelTemporalItem};
use crate::request::{RequestItem, RequestWriter};

/// The items of one request over model subjects `S`, in index order, each
/// with its obligation identity.
#[derive(Clone, Debug)]
pub struct ModelRequest<S> {
    writer: RequestWriter,
    /// `obligations[i]` is the obligation of the item at index `i`.
    obligations: Vec<ItemObligation<S>>,
}

impl<S> Default for ModelRequest<S> {
    fn default() -> Self {
        Self {
            writer: RequestWriter::new(),
            obligations: Vec::new(),
        }
    }
}

impl<S: Clone + Eq> ModelRequest<S> {
    /// An empty request.
    pub fn new() -> Self {
        Self::default()
    }

    /// Write the item for `item` with its `requirements`, and no proof
    /// bounds. The requirement record is the checked clause's alone
    /// (FR-336-AC-1), so the subject's universes reach only the item's
    /// obligation identity. An item whose obligation is already written is
    /// not written twice: the existing item's index is returned, so one
    /// obligation has one index.
    pub fn authored(
        &mut self,
        item: &ModelTemporalItem<'_, S>,
        requirements: &Requirements,
    ) -> RequestIndex {
        let obligation = item.obligation();
        if let Some(index) = self.index_of(&obligation) {
            return index;
        }
        let index = self.writer.item(item.occurrence.clone(), requirements);
        self.obligations.push(obligation);
        index
    }

    /// The obligation of the item at `index`: the join of a result that
    /// carries `index` to the obligation it answers. `None` for an index
    /// this request did not write.
    pub fn obligation(&self, index: RequestIndex) -> Option<&ItemObligation<S>> {
        self.obligations.get(index.get())
    }

    /// The index of the item with `obligation`, when this request wrote it.
    pub fn index_of(&self, obligation: &ItemObligation<S>) -> Option<RequestIndex> {
        self.obligations
            .iter()
            .position(|written| written == obligation)
            .map(RequestIndex::new)
    }

    /// The items written so far, in index order.
    pub fn items(&self) -> &[RequestItem] {
        self.writer.items()
    }
}
