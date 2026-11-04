// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-531: the evaluation entry over a checked package read from its bytes
//! alone.
//!
//! A [`BarePackage`] borrows the [`CheckedPackageV2`] that Contract IR's
//! `read_checked_package` returned. It has no other constructor, so no value
//! of the source side (`CheckedPackage`, `Emission`, `AdmittedPackageRead`)
//! can reach the evaluation code that takes it. The identity rows a protocol
//! slot call needs come from the handoff directory's `identities.json`
//! through [`ReadIdentityRows`], and the dependency closure from the direct
//! dependencies the reader retained in the package
//! ([`BarePackage::dependencies`]).

use std::collections::{BTreeMap, BTreeSet};

use quire_contract_model::{
    CheckedNodeId, CheckedPackageV2, SelectedExactDeclarationForm, SelectedExactDeclarationRow,
    SelectedExactFieldRow, SelectedExactIdentity,
};
use quire_exact::{EffectiveId, MemberId, MemberOwner, NodeKey, OriginalDeclarationKey, UnitId};
use serde::{Deserialize, Serialize};

use crate::checked_v2::wire_graph;

/// The format of `identities.json` (FR-531 "The identity file").
pub const IDENTITIES_FORMAT: &str = "quire.checked-handoff.identities/1";

/// A checked package as a bytes-only consumer holds it (FR-531).
#[derive(Clone, Copy, Debug)]
pub struct BarePackage<'p> {
    package: &'p CheckedPackageV2,
    supply: Option<BareSupply<'p>>,
}

/// What a protocol-slot call reads beside the package (FR-531).
#[derive(Clone, Copy, Debug)]
pub struct BareSupply<'p> {
    /// The selected model identity rows of the package.
    pub identities: &'p ReadIdentityRows,
}

impl<'p> BarePackage<'p> {
    /// The bare package over `package`, with no supply.
    pub fn new(package: &'p CheckedPackageV2) -> Self {
        Self {
            package,
            supply: None,
        }
    }

    /// The bare package with `supply`.
    #[must_use]
    pub fn with_supply(mut self, supply: BareSupply<'p>) -> Self {
        self.supply = Some(supply);
        self
    }

    /// The package.
    pub fn package(&self) -> &'p CheckedPackageV2 {
        self.package
    }

    /// The supply, when one was given.
    pub fn supply(&self) -> Option<&BareSupply<'p>> {
        self.supply.as_ref()
    }

    /// The package's dependency closure, read from the direct dependencies
    /// each package retains from its read evidence (IR-1325): every
    /// dependency once, nearest first, in lock-selection order. The root
    /// package is not listed.
    pub fn dependencies(&self) -> Vec<&'p CheckedPackageV2> {
        let mut closure: Vec<&'p CheckedPackageV2> = Vec::new();
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        let mut queue: std::collections::VecDeque<&'p CheckedPackageV2> =
            self.package.direct_dependencies().collect();
        while let Some(next) = queue.pop_front() {
            if !seen.insert(next.name()) {
                continue;
            }
            closure.push(next);
            queue.extend(next.direct_dependencies());
        }
        closure
    }
}

/// The selected model identity rows of one package, read from its
/// `identities.json` (FR-390, FR-531).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadIdentityRows {
    package: String,
    declarations: Vec<SelectedExactDeclarationRow>,
    fields: Vec<SelectedExactFieldRow>,
}

/// Why an identity file was not read: a closed set, naming what it found.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum IdentityFileRefusal {
    /// Not JSON, not this shape, or an unknown or repeated member.
    #[error("the identity file is malformed at `{member}`")]
    Malformed {
        /// The member the parse stopped at, or empty for the whole file.
        member: String,
    },
    /// Another `format` string.
    #[error("the identity file has format `{0}`")]
    Format(String),
    /// A row the package does not join.
    #[error("the identity file row {index} of `{member}` is not a node of the package")]
    Unjoined {
        /// `declarations` or `fields`.
        member: &'static str,
        /// The first row that does not join.
        index: usize,
    },
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct File {
    format: String,
    declarations: Vec<DeclarationJson>,
    fields: Vec<FieldJson>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DeclarationJson {
    structural_node: CheckedNodeId,
    selected_document: String,
    form: String,
    original_identity: String,
    exact_identity: IdentityJson,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct IdentityJson {
    kind: String,
    value: serde_json::Value,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FieldJson {
    record_node: CheckedNodeId,
    selected_document: String,
    record_original_identity: String,
    field_original_identity: String,
    member_id: MemberJson,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct MemberJson {
    owner: OwnerJson,
    member: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct OwnerJson {
    kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    node: Option<CheckedNodeId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    effective: Option<EffectiveJson>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct EffectiveJson {
    chain: Vec<LinkJson>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LinkJson {
    package: String,
    node: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct UnitJson {
    kind: String,
    node: CheckedNodeId,
}

fn form_spelling(form: SelectedExactDeclarationForm) -> &'static str {
    match form {
        SelectedExactDeclarationForm::RecordValueType => "record_value_type",
        SelectedExactDeclarationForm::ObjectType => "object_type",
        SelectedExactDeclarationForm::Union => "union",
        SelectedExactDeclarationForm::DeclaredUnit => "declared_unit",
    }
}

fn form_of(spelling: &str) -> Option<SelectedExactDeclarationForm> {
    Some(match spelling {
        "record_value_type" => SelectedExactDeclarationForm::RecordValueType,
        "object_type" => SelectedExactDeclarationForm::ObjectType,
        "union" => SelectedExactDeclarationForm::Union,
        "declared_unit" => SelectedExactDeclarationForm::DeclaredUnit,
        _ => return None,
    })
}

/// The package's nodes by structural key, for writing a key as the node
/// reference the wire spells it with.
fn node_references(package: &CheckedPackageV2) -> Option<BTreeMap<NodeKey, CheckedNodeId>> {
    let graph = wire_graph(package).ok()?;
    package
        .graph()
        .nodes
        .iter()
        .map(|node| Some((graph.key(&node.node_id).ok()?, node.node_id.clone())))
        .collect()
}

impl ReadIdentityRows {
    /// The package the rows were produced for.
    pub fn package_name(&self) -> &str {
        &self.package
    }

    /// The selected declaration rows, in file order.
    pub fn declarations(&self) -> &[SelectedExactDeclarationRow] {
        &self.declarations
    }

    /// The selected record-field rows, in file order.
    pub fn fields(&self) -> &[SelectedExactFieldRow] {
        &self.fields
    }

    /// Read `identities_json`, joining its rows to `package`.
    ///
    /// The rows grant no authority: they are trusted as the package file is,
    /// as an artifact of the producer the consumer chose to read. Only what
    /// the package can answer is verified: that each node a row names is a
    /// node of the package and that each declaration's form matches its
    /// identity. A refusal returns no rows.
    ///
    /// # Errors
    ///
    /// [`IdentityFileRefusal`].
    pub fn read(
        identities_json: &[u8],
        package: &CheckedPackageV2,
    ) -> Result<Self, IdentityFileRefusal> {
        let malformed = |member: &str| IdentityFileRefusal::Malformed {
            member: member.to_owned(),
        };
        let file: File = serde_json::from_slice(identities_json).map_err(|error| {
            let text = error.to_string();
            let member = text.split('`').nth(1).unwrap_or("");
            malformed(member)
        })?;
        if file.format != IDENTITIES_FORMAT {
            return Err(IdentityFileRefusal::Format(file.format));
        }
        let graph = wire_graph(package).map_err(|_| malformed(""))?;
        let held: BTreeSet<&CheckedNodeId> = package
            .graph()
            .nodes
            .iter()
            .map(|node| &node.node_id)
            .collect();
        let key = |id: &CheckedNodeId, member: &'static str, index: usize| {
            if held.contains(id) {
                graph
                    .key(id)
                    .map_err(|_| IdentityFileRefusal::Unjoined { member, index })
            } else {
                Err(IdentityFileRefusal::Unjoined { member, index })
            }
        };
        let mut declarations = Vec::new();
        for (index, row) in file.declarations.iter().enumerate() {
            let unjoined = IdentityFileRefusal::Unjoined {
                member: "declarations",
                index,
            };
            if !held.contains(&row.structural_node) {
                return Err(unjoined);
            }
            let form = form_of(&row.form).ok_or_else(|| malformed("form"))?;
            let identity = match (row.exact_identity.kind.as_str(), form) {
                ("record", SelectedExactDeclarationForm::RecordValueType) => {
                    SelectedExactIdentity::Record(node_value(
                        &row.exact_identity.value,
                        &key,
                        "declarations",
                        index,
                    )?)
                }
                ("union", SelectedExactDeclarationForm::Union) => SelectedExactIdentity::Union(
                    node_value(&row.exact_identity.value, &key, "declarations", index)?,
                ),
                ("object", SelectedExactDeclarationForm::ObjectType) => {
                    let effective: EffectiveJson =
                        serde_json::from_value(row.exact_identity.value.clone())
                            .map_err(|_| malformed("exact_identity"))?;
                    SelectedExactIdentity::Object(effective_id(&effective, &malformed)?)
                }
                ("declared_unit", SelectedExactDeclarationForm::DeclaredUnit) => {
                    let unit: UnitJson = serde_json::from_value(row.exact_identity.value.clone())
                        .map_err(|_| malformed("exact_identity"))?;
                    if unit.kind != "declared" {
                        return Err(malformed("exact_identity"));
                    }
                    SelectedExactIdentity::DeclaredUnit(UnitId::declared(key(
                        &unit.node,
                        "declarations",
                        index,
                    )?))
                }
                ("record" | "union" | "object" | "declared_unit", _) => return Err(unjoined),
                _ => return Err(malformed("exact_identity")),
            };
            declarations.push(SelectedExactDeclarationRow::new(
                row.structural_node.clone(),
                row.selected_document.as_str(),
                form,
                row.original_identity.as_str(),
                identity,
            ));
        }
        let mut fields = Vec::new();
        for (index, row) in file.fields.iter().enumerate() {
            if !held.contains(&row.record_node) {
                return Err(IdentityFileRefusal::Unjoined {
                    member: "fields",
                    index,
                });
            }
            let owner = match (
                row.member_id.owner.kind.as_str(),
                &row.member_id.owner.node,
                &row.member_id.owner.effective,
            ) {
                ("node", Some(node), None) => MemberOwner::Node(key(node, "fields", index)?),
                ("effective", None, Some(effective)) => {
                    MemberOwner::Effective(effective_id(effective, &malformed)?)
                }
                _ => return Err(malformed("member_id")),
            };
            let member_id = MemberId::new(owner, row.member_id.member.as_str())
                .map_err(|_| malformed("member_id"))?;
            fields.push(SelectedExactFieldRow::new(
                row.record_node.clone(),
                row.selected_document.as_str(),
                row.record_original_identity.as_str(),
                row.field_original_identity.as_str(),
                member_id,
            ));
        }
        Ok(Self {
            package: package.name().to_owned(),
            declarations,
            fields,
        })
    }

    /// The rows of the in-process unit, for a package it was emitted as.
    pub fn of_rows(
        package: &CheckedPackageV2,
        declarations: Vec<SelectedExactDeclarationRow>,
        fields: Vec<SelectedExactFieldRow>,
    ) -> Self {
        Self {
            package: package.name().to_owned(),
            declarations,
            fields,
        }
    }

    /// The identity file of these rows (FR-531 "The identity file"): one JSON
    /// object, pretty-printed with two-space indent, members in the
    /// documented order, UTF-8, ending in one newline, byte-identical for
    /// equal inputs.
    ///
    /// # Errors
    ///
    /// [`IdentityFileRefusal::Malformed`] for a compound unit, which is no
    /// selected declaration, and for a key the package does not hold.
    pub fn to_bytes(&self, package: &CheckedPackageV2) -> Result<Vec<u8>, IdentityFileRefusal> {
        let malformed = |member: &str| IdentityFileRefusal::Malformed {
            member: member.to_owned(),
        };
        let references = node_references(package).ok_or_else(|| malformed(""))?;
        let reference = |key: &NodeKey| {
            references
                .get(key)
                .cloned()
                .ok_or_else(|| malformed("exact_identity"))
        };
        let effective = |id: &EffectiveId| EffectiveJson {
            chain: id
                .owner_chain()
                .iter()
                .map(|link| LinkJson {
                    package: link.package().to_owned(),
                    node: link.node().to_owned(),
                })
                .collect(),
        };
        let mut declarations = Vec::new();
        for row in &self.declarations {
            let (kind, value) = match &row.exact_identity {
                SelectedExactIdentity::Record(key) => (
                    "record",
                    serde_json::to_value(reference(key)?).map_err(|_| malformed(""))?,
                ),
                SelectedExactIdentity::Union(key) => (
                    "union",
                    serde_json::to_value(reference(key)?).map_err(|_| malformed(""))?,
                ),
                SelectedExactIdentity::Object(id) => (
                    "object",
                    serde_json::to_value(effective(id)).map_err(|_| malformed(""))?,
                ),
                SelectedExactIdentity::DeclaredUnit(unit) => {
                    let key = unit
                        .declared_key()
                        .ok_or_else(|| malformed("exact_identity"))?;
                    (
                        "declared_unit",
                        serde_json::to_value(UnitJson {
                            kind: "declared".to_owned(),
                            node: reference(key)?,
                        })
                        .map_err(|_| malformed(""))?,
                    )
                }
            };
            declarations.push(DeclarationJson {
                structural_node: row.structural_node.clone(),
                selected_document: row.selected_document.to_string(),
                form: form_spelling(row.form).to_owned(),
                original_identity: row.original_identity.to_string(),
                exact_identity: IdentityJson {
                    kind: kind.to_owned(),
                    value,
                },
            });
        }
        let mut fields = Vec::new();
        for row in &self.fields {
            let owner = match row.member_id.owner() {
                MemberOwner::Node(key) => OwnerJson {
                    kind: "node".to_owned(),
                    node: Some(reference(key)?),
                    effective: None,
                },
                MemberOwner::Effective(id) => OwnerJson {
                    kind: "effective".to_owned(),
                    node: None,
                    effective: Some(effective(id)),
                },
            };
            fields.push(FieldJson {
                record_node: row.record_node.clone(),
                selected_document: row.selected_document.to_string(),
                record_original_identity: row.record_original_identity.to_string(),
                field_original_identity: row.field_original_identity.to_string(),
                member_id: MemberJson {
                    owner,
                    member: row.member_id.member().to_owned(),
                },
            });
        }
        let file = File {
            format: IDENTITIES_FORMAT.to_owned(),
            declarations,
            fields,
        };
        let mut bytes = serde_json::to_vec_pretty(&file).map_err(|_| malformed(""))?;
        bytes.push(b'\n');
        Ok(bytes)
    }
}

fn node_value(
    value: &serde_json::Value,
    key: &impl Fn(&CheckedNodeId, &'static str, usize) -> Result<NodeKey, IdentityFileRefusal>,
    member: &'static str,
    index: usize,
) -> Result<NodeKey, IdentityFileRefusal> {
    let id: CheckedNodeId =
        serde_json::from_value(value.clone()).map_err(|_| IdentityFileRefusal::Malformed {
            member: "exact_identity".to_owned(),
        })?;
    key(&id, member, index)
}

fn effective_id(
    effective: &EffectiveJson,
    malformed: &impl Fn(&str) -> IdentityFileRefusal,
) -> Result<EffectiveId, IdentityFileRefusal> {
    let mut chain = effective.chain.iter();
    let first = chain.next().ok_or_else(|| malformed("chain"))?;
    let mut id = EffectiveId::new(
        None,
        OriginalDeclarationKey::new(&first.package, &first.node),
    );
    for link in chain {
        id = EffectiveId::new(
            Some(id),
            OriginalDeclarationKey::new(&link.package, &link.node),
        );
    }
    Ok(id)
}
