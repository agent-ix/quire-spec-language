// SPDX-License-Identifier: AGPL-3.0-or-later
//! A lowered node's body as the v2 wire carries it: the JSON tree FR-322's
//! node `body` member holds, built from the typed
//! [`SemanticTerm`](super::SemanticTerm). The
//! wire, not the key: a key hashes `quire-canonical`'s encoding of the typed
//! preimage (`node_key`).

use super::SemanticNode;

impl SemanticNode {
    /// The node's FR-322 `body` as a JSON tree, for the v2 emitter.
    pub fn wire_body(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(&self.body)
    }
}
