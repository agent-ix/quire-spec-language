// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-106: the writer of a `quire.state.snapshot/v1` document that names
//! objects by their full reference (population and key) and their field
//! values. A field value is written by [`SnapshotValue::to_json`], the one
//! writer of FR-106's value forms; the document is put in RFC 8785 form by
//! the shared canonical encoder, which is also what admission digests.

use serde_json::{json, Map, Value as Json};

use crate::model::key::DeclarationKey;

use super::{fault, AdmissionFailure, SnapshotValue};

/// The four FR-001 labels of a document. Admission selects a document by
/// digest alone and never compares these with the document's own identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnvironmentLabels {
    /// The `authority` label.
    pub authority: String,
    /// The `identity` label.
    pub identity: String,
    /// The `revision_namespace` label.
    pub revision_namespace: String,
    /// The `revision` label.
    pub revision: String,
}

/// One object of a population: its `key`, its object type's declaration
/// identity and every declared field's value.
#[derive(Clone, Debug)]
pub struct EnvironmentObject {
    /// The object's declared key.
    pub key: String,
    /// The object type's declaration. The document names it by its `node`
    /// identity; the package is the model's, named once by the document.
    pub type_identity: DeclarationKey,
    /// Each declared field, by member name.
    pub fields: Vec<(String, SnapshotValue)>,
}

/// One population of the document with the objects it lists.
#[derive(Clone, Debug)]
pub struct EnvironmentPopulation {
    /// The population's declaration identity.
    pub population: String,
    /// Whether the document lists the population completely.
    pub complete: bool,
    /// The objects it lists, in document order.
    pub objects: Vec<EnvironmentObject>,
}

/// The selected model the document names: its identity and version, and the
/// `sha256-jcs` digest of its domain package.
#[derive(Clone, Debug)]
pub struct EnvironmentModel {
    /// The domain package identity.
    pub identity: String,
    /// The domain package version.
    pub version: String,
    /// The domain package's `sha256-jcs` digest.
    pub digest: [u8; 32],
}

/// The canonical bytes of a current snapshot of `populations`, labelled
/// `identity` and selecting `model`.
///
/// The snapshot's anchor is the handler `replay`: a function has no anchor,
/// and admission reads it for form and never compares it.
///
/// # Errors
///
/// A `Fault` when the document has no canonical form.
pub fn write_environment_document(
    identity: &EnvironmentLabels,
    model: &EnvironmentModel,
    populations: &[EnvironmentPopulation],
) -> Result<Vec<u8>, AdmissionFailure> {
    let populations: Vec<Json> = populations
        .iter()
        .map(|population| {
            let objects: Vec<Json> = population
                .objects
                .iter()
                .map(|object| {
                    let fields: Map<String, Json> = object
                        .fields
                        .iter()
                        .map(|(name, value)| (name.clone(), value.to_json()))
                        .collect();
                    json!({"key": object.key, "type": object.type_identity.node, "fields": fields})
                })
                .collect();
            json!({
                "population": population.population,
                "complete": population.complete,
                "objects": objects,
            })
        })
        .collect();
    let document = json!({
        "format": "quire.state.snapshot/v1",
        "identity": {
            "authority": identity.authority,
            "identity": identity.identity,
            "revision_namespace": identity.revision_namespace,
            "revision": identity.revision,
        },
        "observation": "current",
        "anchor": {"kind": "handler", "name": "replay"},
        "model": {
            "identity": model.identity,
            "version": model.version,
            "digest": format!("sha256-jcs:{}", crate::model::key::hex(&model.digest)),
        },
        "populations": populations,
    });
    let text = serde_json::to_vec(&document).map_err(|_| fault("environment-document-text"))?;
    let read = quire_canonical::read(&text, u64::MAX)
        .map_err(|_| fault("environment-document-reads-back"))?;
    quire_canonical::to_vec(&read, quire_canonical::Limits::new(u64::MAX))
        .map_err(|_| fault("environment-document-has-a-canonical-form"))
}
