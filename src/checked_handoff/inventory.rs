// SPDX-License-Identifier: AGPL-3.0-or-later
//! The inventory file, `quire.checked-handoff.inventory/1` (FR-517 "The
//! inventory file"): QSL's own encoding of the `SourceInventoryOutcome` that
//! `qsl_replay::qsl_inventory` returns over the in-process unit, and its reader.

use quire_contract_model::source_obligation::{
    SourceInventoryCompleteness, SourceInventoryIdentity, SourceInventoryOutcome,
    SourcePendingObligationKind,
};
use quire_contract_model::CheckedNodeId;
use serde::{Deserialize, Serialize};

use super::{Error, INVENTORY_FORMAT};

/// The formal identity of the inventory: the `source` member, which every row
/// shares.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventorySource {
    /// The `SourceDocumentId` text (FR-517 "The inventory file").
    pub document: String,
    /// The revision, always `1` for the writer.
    pub revision: u64,
}

/// One row of the inventory file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InventoryRow {
    /// A checked requirement record.
    RequirementRecord {
        /// The wire node id of the node in `checked-package.json`.
        node: CheckedNodeId,
        /// The requirement's role text.
        role: String,
        /// The occurrence ordinal.
        ordinal: u64,
    },
    /// A pending obligation.
    PendingObligation {
        /// The ordinal of the requirement form that holds the span.
        declaration: u64,
        /// The source unit handle, `0`.
        unit: u64,
        /// The span's start byte offset.
        start: u64,
        /// The span's end byte offset.
        end: u64,
        /// The exact capability kind name.
        capability_kind: String,
    },
}

/// A parsed `inventory.json`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Inventory {
    source: InventorySource,
    rows: Vec<InventoryRow>,
}

/// An inventory file that does not parse.
#[derive(Debug, thiserror::Error)]
pub enum InventoryReadError {
    /// The bytes are not an inventory file of this shape.
    #[error("not an inventory file: {0}")]
    Malformed(String),
    /// The file is of another format.
    #[error("inventory format {0:?} is not {INVENTORY_FORMAT}")]
    Format(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
enum Owner {
    #[serde(rename = "qsl")]
    Qsl,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
enum Scope {
    #[serde(rename = "qsl_checked_requirements_and_pending_obligations")]
    CheckedRequirementsAndPendingObligations,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
enum Completeness {
    #[serde(rename = "complete")]
    Complete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Kind {
    RequirementRecord,
    PendingObligation,
}

/// A row as the file spells it: the members of both kinds, in file order.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RowJson {
    kind: Kind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    node: Option<CheckedNodeId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ordinal: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    declaration: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    unit: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    start: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    end: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    capability_kind: Option<String>,
}

impl RowJson {
    fn of(row: &InventoryRow) -> Self {
        let mut json = Self {
            kind: Kind::RequirementRecord,
            node: None,
            role: None,
            ordinal: None,
            declaration: None,
            unit: None,
            start: None,
            end: None,
            capability_kind: None,
        };
        match row {
            InventoryRow::RequirementRecord {
                node,
                role,
                ordinal,
            } => {
                json.node = Some(node.clone());
                json.role = Some(role.clone());
                json.ordinal = Some(*ordinal);
            }
            InventoryRow::PendingObligation {
                declaration,
                unit,
                start,
                end,
                capability_kind,
            } => {
                json.kind = Kind::PendingObligation;
                json.declaration = Some(*declaration);
                json.unit = Some(*unit);
                json.start = Some(*start);
                json.end = Some(*end);
                json.capability_kind = Some(capability_kind.clone());
            }
        }
        json
    }

    /// The row, when exactly the members of its kind are present.
    fn row(self) -> Option<InventoryRow> {
        match self {
            Self {
                kind: Kind::RequirementRecord,
                node: Some(node),
                role: Some(role),
                ordinal: Some(ordinal),
                declaration: None,
                unit: None,
                start: None,
                end: None,
                capability_kind: None,
            } => Some(InventoryRow::RequirementRecord {
                node,
                role,
                ordinal,
            }),
            Self {
                kind: Kind::PendingObligation,
                node: None,
                role: None,
                ordinal: None,
                declaration: Some(declaration),
                unit: Some(unit),
                start: Some(start),
                end: Some(end),
                capability_kind: Some(capability_kind),
            } => Some(InventoryRow::PendingObligation {
                declaration,
                unit,
                start,
                end,
                capability_kind,
            }),
            _ => None,
        }
    }
}

/// The `format` member alone.
#[derive(Deserialize)]
struct Head {
    format: String,
}

/// The file's members in file order.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileJson {
    format: String,
    owner: Owner,
    scope: Scope,
    source: InventorySource,
    completeness: Completeness,
    observed_count: u64,
    rows: Vec<RowJson>,
}

/// Whether `format` spells the inventory format this reader admits.
#[ix_trace_rs::string_edge]
fn is_inventory_format(format: &str) -> bool {
    format == INVENTORY_FORMAT
}

fn malformed(what: impl std::fmt::Display) -> InventoryReadError {
    InventoryReadError::Malformed(what.to_string())
}

impl Inventory {
    /// Parse an inventory file. No partial value is returned.
    pub fn read(bytes: &[u8]) -> Result<Self, InventoryReadError> {
        // The format decides first: a file of another format is `Format`
        // whatever else it holds.
        let head: Head = serde_json::from_slice(bytes).map_err(malformed)?;
        if !is_inventory_format(&head.format) {
            return Err(InventoryReadError::Format(head.format));
        }
        let file: FileJson = serde_json::from_slice(bytes).map_err(malformed)?;
        if usize::try_from(file.observed_count).ok() != Some(file.rows.len()) {
            return Err(malformed(format!(
                "observed_count {} is not the {} rows",
                file.observed_count,
                file.rows.len()
            )));
        }
        let rows = file
            .rows
            .into_iter()
            .enumerate()
            .map(|(at, row)| {
                row.row().ok_or_else(|| {
                    malformed(format!("row {at} does not have the members of its kind"))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            source: file.source,
            rows,
        })
    }

    /// The formal identity every row shares.
    pub fn source(&self) -> &InventorySource {
        &self.source
    }

    /// The rows: requirement records, then pending obligations.
    pub fn rows(&self) -> &[InventoryRow] {
        &self.rows
    }

    /// The file's bytes, deterministic.
    pub(super) fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let observed_count = u64::try_from(self.rows.len()).map_err(|_| Error::Unnameable {
            what: "inventory row count".to_owned(),
        })?;
        let file = FileJson {
            format: INVENTORY_FORMAT.to_owned(),
            owner: Owner::Qsl,
            scope: Scope::CheckedRequirementsAndPendingObligations,
            source: self.source.clone(),
            completeness: Completeness::Complete,
            observed_count,
            rows: self.rows.iter().map(RowJson::of).collect(),
        };
        let mut bytes = serde_json::to_vec_pretty(&file).map_err(|source| Error::Encode {
            what: "inventory.json",
            source,
        })?;
        bytes.push(b'\n');
        Ok(bytes)
    }

    /// The inventory of `outcome`, each requirement record's node written as
    /// the wire node id `package_bytes` spells it with.
    pub(super) fn derive(
        outcome: &SourceInventoryOutcome,
        package_bytes: &[u8],
    ) -> Result<Self, Error> {
        let unnameable = |what: String| Error::Unnameable { what };
        if outcome.completeness() != &SourceInventoryCompleteness::Complete {
            return Err(unnameable("a complete inventory".to_owned()));
        }
        let wire = qsl_semantics::check::wire_id::WireGraph::of_package_bytes(package_bytes)
            .map_err(unnameable)?;
        let count = |value: usize, what: &str| {
            u64::try_from(value).map_err(|_| unnameable(format!("inventory {what}")))
        };
        let mut rows = Vec::with_capacity(outcome.items().len());
        for item in outcome.items() {
            rows.push(match item.identity() {
                SourceInventoryIdentity::RequirementRecord(record) => {
                    let key = wire
                        .key(&record.node)
                        .ok()
                        .filter(|key| wire.index_of(key).is_some())
                        .ok_or_else(|| unnameable("requirement node in the package".to_owned()))?;
                    let node = wire
                        .wire_id(&key)
                        .map_err(|_| unnameable("requirement node id".to_owned()))?;
                    InventoryRow::RequirementRecord {
                        node,
                        role: record.role.to_string(),
                        ordinal: record.ordinal,
                    }
                }
                SourceInventoryIdentity::PendingObligation(pending) => {
                    let SourcePendingObligationKind::CapabilityKind(kind) = &pending.kind else {
                        return Err(unnameable("capability kind of an obligation".to_owned()));
                    };
                    InventoryRow::PendingObligation {
                        declaration: count(pending.site.declaration.0, "declaration")?,
                        unit: count(pending.site.unit.0, "unit")?,
                        start: count(pending.site.start, "start")?,
                        end: count(pending.site.end, "end")?,
                        capability_kind: kind.to_string(),
                    }
                }
                _ => return Err(unnameable("a requirement or obligation row".to_owned())),
            });
        }
        let document = outcome.source().document().as_str().to_owned();
        Ok(Self {
            source: InventorySource {
                document,
                revision: outcome.source().revision().get(),
            },
            rows,
        })
    }
}

#[cfg(test)]
mod tests {
    use quire_contract_model::source_obligation::{
        SourceDeclarationHandle, SourceInventoryEnumerator, SourceInventoryIncompleteReason,
        SourceInventoryOwner, SourceInventoryRow, SourceInventoryScope, SourceObligationSite,
        SourcePendingObligationId, SourceRequirementOccurrenceId, SourceUnitHandle,
    };
    use quire_contract_model::{SourceDocumentId, SourceIdentity, SourceRevision};
    use quire_exact::{NodeKey, PathSegment};

    use super::*;

    /// A package holding one node, `scalar_type/integer`.
    const PACKAGE: &str = r#"{"semantic_graph":{"nodes":[
        {"node_id":{"package":"p","path":["scalar_type","integer"],"ordinal":0}}],"operand_rows":[[]]}}"#;

    struct Rows {
        identity: SourceIdentity,
        rows: std::vec::IntoIter<SourceInventoryIdentity>,
        stop: Option<SourceInventoryIncompleteReason>,
        observed: usize,
    }

    impl SourceInventoryEnumerator for Rows {
        fn source(&self) -> &SourceIdentity {
            &self.identity
        }
        fn owner(&self) -> SourceInventoryOwner {
            SourceInventoryOwner::Qsl
        }
        fn scope(&self) -> SourceInventoryScope {
            SourceInventoryScope::QslCheckedRequirementsAndPendingObligations
        }
        fn next_row(
            &mut self,
        ) -> Result<Option<SourceInventoryRow>, SourceInventoryIncompleteReason> {
            if let Some(identity) = self.rows.next() {
                self.observed += 1;
                return Ok(Some(SourceInventoryRow::new(
                    self.identity.clone(),
                    identity,
                )));
            }
            self.stop.take().map_or(Ok(None), Err)
        }
        fn observed_count(&self) -> usize {
            self.observed
        }
    }

    fn outcome(
        rows: Vec<SourceInventoryIdentity>,
        stop: Option<SourceInventoryIncompleteReason>,
    ) -> SourceInventoryOutcome {
        let identity = SourceIdentity::new(
            SourceDocumentId::new("u.native").unwrap(),
            SourceRevision::new(1).unwrap(),
        );
        SourceInventoryOutcome::from_enumerator(Rows {
            identity,
            rows: rows.into_iter(),
            stop,
            observed: 0,
        })
        .unwrap()
    }

    fn requirement(name: &str) -> SourceInventoryIdentity {
        let key = NodeKey::new(
            "p",
            vec![
                PathSegment::Text("scalar_type".into()),
                PathSegment::Text(name.into()),
            ],
            0,
        )
        .unwrap();
        SourceInventoryIdentity::RequirementRecord(SourceRequirementOccurrenceId {
            node: qsl_semantics::check::wire_id::structural_id(&key),
            role: "expression".into(),
            ordinal: 3,
        })
    }

    fn obligation(kind: SourcePendingObligationKind) -> SourceInventoryIdentity {
        SourceInventoryIdentity::PendingObligation(SourcePendingObligationId {
            site: SourceObligationSite {
                declaration: SourceDeclarationHandle(2),
                unit: SourceUnitHandle(0),
                expression: None,
                start: 4,
                end: 9,
            },
            kind,
        })
    }

    fn unnameable(result: Result<Inventory, Error>) -> String {
        match result {
            Err(Error::Unnameable { what }) => what,
            other => panic!("{other:?}"),
        }
    }

    #[ix_trace_rs::trace("FR-517-AC-11", "TC-1312")]
    #[test]
    fn a_node_the_package_holds_is_named_by_its_wire_id() {
        let found = outcome(
            vec![
                requirement("integer"),
                obligation(SourcePendingObligationKind::CapabilityKind("k".into())),
            ],
            None,
        );
        let inventory = Inventory::derive(&found, PACKAGE.as_bytes()).unwrap();
        let wire: serde_json::Value = serde_json::from_str(PACKAGE).unwrap();
        let expected: CheckedNodeId =
            serde_json::from_value(wire["semantic_graph"]["nodes"][0]["node_id"].clone()).unwrap();
        assert_eq!(
            inventory.rows(),
            [
                InventoryRow::RequirementRecord {
                    node: expected,
                    role: "expression".into(),
                    ordinal: 3,
                },
                InventoryRow::PendingObligation {
                    declaration: 2,
                    unit: 0,
                    start: 4,
                    end: 9,
                    capability_kind: "k".into(),
                },
            ]
        );
    }

    #[ix_trace_rs::trace("FR-517-AC-11", "TC-1312")]
    #[test]
    fn a_node_the_package_does_not_hold_is_unnameable() {
        let found = outcome(vec![requirement("boolean")], None);
        assert_eq!(
            unnameable(Inventory::derive(&found, PACKAGE.as_bytes())),
            "requirement node in the package"
        );
    }

    #[ix_trace_rs::trace("FR-517-AC-7", "FR-517-AC-11", "TC-1312")]
    #[test]
    fn an_incomplete_outcome_is_never_written() {
        let stopped = outcome(
            vec![requirement("integer")],
            Some(SourceInventoryIncompleteReason::EarlyStop {
                owner: SourceInventoryOwner::Qsl,
            }),
        );
        assert!(matches!(
            stopped.completeness(),
            SourceInventoryCompleteness::Incomplete(_)
        ));
        assert_eq!(
            unnameable(Inventory::derive(&stopped, PACKAGE.as_bytes())),
            "a complete inventory"
        );
    }

    #[ix_trace_rs::trace("FR-517-AC-11", "TC-1312")]
    #[test]
    fn an_obligation_without_a_capability_kind_is_unnameable() {
        let found = outcome(
            vec![obligation(SourcePendingObligationKind::Presence)],
            None,
        );
        assert_eq!(
            unnameable(Inventory::derive(&found, PACKAGE.as_bytes())),
            "capability kind of an obligation"
        );
    }
}
