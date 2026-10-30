// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-451: the body of a `correspondence`/`abstraction_relation` node.
//!
//! The body's arrays are in FR-451's canonical order whatever order they were
//! built in: [`AbstractionTerm::new`] sorts them. Each Rust path is the array
//! of its segments, so a `::` of the checked spelling is a segment boundary.

use std::collections::BTreeSet;

use quire_exact::NodeKey;

use super::{NodeKeyRefusal, NodeRef};

/// An abstraction relation node's body, `{term: "abstraction_relation",
/// objects, populations, frames}` (QSpec FR-451).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AbstractionTerm {
    objects: Vec<ObjectEntry>,
    populations: Vec<PopulationEntry>,
    frames: Vec<FrameEntry>,
}

/// One object binding: `{type, rust_type, fields}`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ObjectEntry {
    ty: NodeRef,
    rust_type: Vec<String>,
    fields: Vec<FieldEntry>,
}

/// One field binding: `{name, rust_field}`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct FieldEntry {
    name: String,
    rust_field: String,
}

/// One population binding: `{population, collection}`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PopulationEntry {
    population: NodeRef,
    collection: Vec<String>,
}

/// One operation binding: `{context, operation, function, receiver,
/// parameters}`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FrameEntry {
    context: NodeRef,
    operation: String,
    function: Vec<String>,
    receiver: String,
    parameters: Vec<ParameterEntry>,
}

/// One parameter binding: `{name, rust_parameter}`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
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
            ty: NodeRef::Admitted(ty),
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
            population: NodeRef::Admitted(population),
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
            context: NodeRef::Admitted(context),
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
        objects.sort_by(|left, right| left.ty.cmp(&right.ty));
        populations.sort_by(|left, right| left.population.cmp(&right.population));
        frames.sort_by(|left, right| {
            (&left.context, &left.operation).cmp(&(&right.context, &right.operation))
        });
        Self {
            objects,
            populations,
            frames,
        }
    }

    /// Every admitted `type`, `population` and `context` target in structural
    /// order. A pending target is a lowering fault and cannot be emitted.
    pub fn targets(&self) -> Result<BTreeSet<NodeKey>, NodeKeyRefusal> {
        self.objects
            .iter()
            .map(|entry| &entry.ty)
            .chain(self.populations.iter().map(|entry| &entry.population))
            .chain(self.frames.iter().map(|entry| &entry.context))
            .map(|reference| {
                reference
                    .admitted()
                    .cloned()
                    .ok_or(NodeKeyRefusal::UnresolvedPlaceholder)
            })
            .collect()
    }

    /// Remap admitted node references and retain unresolved local handles.
    pub(crate) fn map_keys(&self, map: &mut impl FnMut(NodeKey) -> NodeKey) -> Self {
        let objects = self
            .objects
            .iter()
            .map(|entry| ObjectEntry {
                ty: entry.ty.map_admitted(map),
                ..entry.clone()
            })
            .collect();
        let populations = self
            .populations
            .iter()
            .map(|entry| PopulationEntry {
                population: entry.population.map_admitted(map),
                ..entry.clone()
            })
            .collect();
        let frames = self
            .frames
            .iter()
            .map(|entry| FrameEntry {
                context: entry.context.map_admitted(map),
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
