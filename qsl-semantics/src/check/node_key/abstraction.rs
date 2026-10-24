// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-451: the body of a `correspondence`/`abstraction_relation` node,
//! and the key it hashes to.
//!
//! The node's `node_id` is the SHA-256 of the canonical bytes of
//! `{version: "quire.abstraction-relation-node/v1", body}`, a preimage of its
//! own beside FR-322's application preimage and FR-092's structural one, so
//! every binding of the body is part of the key. [`abstraction_node_key`] is
//! the only function that mints it; [`node_key`](super::node_key) refuses an
//! abstraction body.
//!
//! The body's arrays are in FR-451's canonical order whatever order they were
//! built in: [`AbstractionTerm::new`] sorts them. Each Rust path is the array
//! of its segments, so a `::` of the checked spelling is a segment boundary.

use std::collections::BTreeSet;

use qsl_foundation::{ByteDigest, IdentityLimits};
use quire_canonical::FixedShape;
use quire_exact::NodeKey;
use serde::Serialize;

use super::{canonical_bytes, KeyedPreimage, NodeKeyRefusal, NodeRef};

/// FR-451's preimage version.
pub(crate) const ABSTRACTION_NODE_VERSION: &str = "quire.abstraction-relation-node/v1";

/// An abstraction relation node's body, `{term: "abstraction_relation",
/// objects, populations, frames}` (QSpec FR-451).
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, FixedShape)]
#[serde(tag = "term", rename = "abstraction_relation")]
pub struct AbstractionTerm {
    objects: Vec<ObjectEntry>,
    populations: Vec<PopulationEntry>,
    frames: Vec<FrameEntry>,
}

/// One object binding: `{type, rust_type, fields}`.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, FixedShape)]
pub struct ObjectEntry {
    #[serde(rename = "type")]
    ty: NodeRef,
    rust_type: Vec<String>,
    fields: Vec<FieldEntry>,
}

/// One field binding: `{name, rust_field}`.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, FixedShape)]
struct FieldEntry {
    name: String,
    rust_field: String,
}

/// One population binding: `{population, collection}`.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, FixedShape)]
pub struct PopulationEntry {
    population: NodeRef,
    collection: Vec<String>,
}

/// One operation binding: `{context, operation, function, receiver,
/// parameters}`.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, FixedShape)]
pub struct FrameEntry {
    context: NodeRef,
    operation: String,
    function: Vec<String>,
    receiver: String,
    parameters: Vec<ParameterEntry>,
}

/// One parameter binding: `{name, rust_parameter}`.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, FixedShape)]
struct ParameterEntry {
    name: String,
    rust_parameter: String,
}

impl ObjectEntry {
    /// The binding of the object type `ty`, with each `(field, rust_field)`
    /// pair; the pairs are sorted by field name as UTF-8 bytes.
    pub(crate) fn new(
        ty: NodeKey,
        rust_type: Vec<String>,
        mut fields: Vec<(String, String)>,
    ) -> Self {
        fields.sort();
        Self {
            ty: NodeRef(ty),
            rust_type,
            fields: fields
                .into_iter()
                .map(|(name, rust_field)| FieldEntry { name, rust_field })
                .collect(),
        }
    }
}

impl PopulationEntry {
    /// The binding of the population `population` to `collection`.
    pub(crate) fn new(population: NodeKey, collection: Vec<String>) -> Self {
        Self {
            population: NodeRef(population),
            collection,
        }
    }
}

impl FrameEntry {
    /// The binding of `operation` of the object type `context`, with each
    /// `(parameter, rust_parameter)` pair; the pairs are sorted by parameter
    /// name as UTF-8 bytes.
    pub(crate) fn new(
        context: NodeKey,
        operation: String,
        function: Vec<String>,
        receiver: String,
        mut parameters: Vec<(String, String)>,
    ) -> Self {
        parameters.sort();
        Self {
            context: NodeRef(context),
            operation,
            function,
            receiver,
            parameters: parameters
                .into_iter()
                .map(|(name, rust_parameter)| ParameterEntry {
                    name,
                    rust_parameter,
                })
                .collect(),
        }
    }
}

impl AbstractionTerm {
    /// The body holding these entries, each array in FR-451's canonical
    /// order: `objects` by `type`, `populations` by `population`, and
    /// `frames` by `context` and then `operation` as UTF-8 bytes, each digest
    /// compared as unsigned bytes.
    pub(crate) fn new(
        mut objects: Vec<ObjectEntry>,
        mut populations: Vec<PopulationEntry>,
        mut frames: Vec<FrameEntry>,
    ) -> Self {
        objects.sort_by_key(|entry| entry.ty);
        populations.sort_by_key(|entry| entry.population);
        frames.sort_by(|left, right| {
            (left.context, &left.operation).cmp(&(right.context, &right.operation))
        });
        Self {
            objects,
            populations,
            frames,
        }
    }

    /// Every `type`, `population` and `context` target: the unique,
    /// digest-ascending set a node's `dependencies` must equal.
    pub fn targets(&self) -> BTreeSet<NodeKey> {
        let objects = self.objects.iter().map(|entry| entry.ty.0);
        let populations = self.populations.iter().map(|entry| entry.population.0);
        let contexts = self.frames.iter().map(|entry| entry.context.0);
        objects.chain(populations).chain(contexts).collect()
    }

    /// This body with every node key it names replaced by `map`'s image, the
    /// arrays put back in canonical order.
    pub(crate) fn map_keys(&self, map: &mut impl FnMut(NodeKey) -> NodeKey) -> Self {
        let mut remap = |reference: NodeRef| NodeRef(map(reference.0));
        let objects = self
            .objects
            .iter()
            .map(|entry| ObjectEntry {
                ty: remap(entry.ty),
                ..entry.clone()
            })
            .collect();
        let populations = self
            .populations
            .iter()
            .map(|entry| PopulationEntry {
                population: remap(entry.population),
                ..entry.clone()
            })
            .collect();
        let frames = self
            .frames
            .iter()
            .map(|entry| FrameEntry {
                context: remap(entry.context),
                ..entry.clone()
            })
            .collect();
        Self::new(objects, populations, frames)
    }
}

// Test-only, matching `FrameTerm`'s pattern: QSpec's operation vectors carry
// no abstraction body, so the conformance decoder never needs one.
#[cfg(test)]
impl<'de> serde::Deserialize<'de> for AbstractionTerm {
    fn deserialize<D: serde::Deserializer<'de>>(_: D) -> Result<Self, D::Error> {
        Err(serde::de::Error::custom(
            "abstraction bodies are not decoded from vectors",
        ))
    }
}

/// QSpec FR-451's preimage `{version, body}`.
#[derive(Serialize, FixedShape)]
struct AbstractionPreimage<'a> {
    version: &'static str,
    body: &'a AbstractionTerm,
}

/// The key of the abstraction relation node holding `body` (QSpec FR-451
/// "node_id"): the SHA-256 of the canonical bytes of `{version, body}`.
pub(crate) fn abstraction_node_key(
    body: &AbstractionTerm,
    identity: IdentityLimits,
) -> Result<KeyedPreimage, NodeKeyRefusal> {
    let preimage = canonical_bytes(
        &AbstractionPreimage {
            version: ABSTRACTION_NODE_VERSION,
            body,
        },
        identity,
    )?;
    let key = NodeKey::from_digest(ByteDigest::of(&preimage).as_bytes());
    Ok(KeyedPreimage { preimage, key })
}

#[cfg(test)]
mod tests;
