// SPDX-License-Identifier: AGPL-3.0-or-later
//! Required final producer facts in the identities/2 carrier (FR-531).

use quire_contract_model::{CheckedNodeId, SelectedSourceRecordMemberRow, SelectedSourceRecordRow};
use quire_exact::{EffectiveId, OriginalDeclarationKey, Presence};
use serde::{Deserialize, Serialize};

use super::{effective_id, EffectiveJson, IdentityFileRefusal, IdentityMember, LinkJson};
use crate::equality_file::{OriginalNodeKey, OriginalValueType};

/// One selected model's original population catalog, bound to its final root.
/// These are producer claims; native catalog validation is a separate IR read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedPopulationCatalog {
    /// The selected model's final emitted root.
    pub model_root: CheckedNodeId,
    /// The selected domain document's identity.
    pub selected_document: Box<str>,
    /// Every population, including declarations unused by expressions.
    pub populations: Vec<SelectedPopulation>,
}

/// A genuine selected population declaration and all its members.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedPopulation {
    /// The population's original declaration identity.
    pub original: OriginalDeclarationKey,
    /// Its declared members in producer order, without dropping aliases.
    pub members: Vec<SelectedPopulationMember>,
}

/// A population member's original declaration and normalized effective type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedPopulationMember {
    /// Original member type declaration, retained separately from normalization.
    pub original: OriginalDeclarationKey,
    /// Genuine normalized effective TYPE identity, with no owner.
    pub effective_type: EffectiveId,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SourceRecordJson {
    structural_node: CheckedNodeId,
    source_authority: String,
    source_identity: String,
    qualified_declaration: Vec<String>,
    record_key: OriginalNodeKey,
    members: Vec<SourceRecordMemberJson>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SourceRecordMemberJson {
    original_member_identity: String,
    admitted_name: String,
    presence: PresenceJson,
    type_node: CheckedNodeId,
    value_type: OriginalValueType,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum PresenceJson {
    Required,
    Optional,
}

impl SourceRecordJson {
    pub(super) fn of(row: &SelectedSourceRecordRow) -> Result<Self, IdentityFileRefusal> {
        let malformed = || IdentityFileRefusal::Malformed {
            member: IdentityMember::SourceRecords,
        };
        Ok(Self {
            structural_node: row.structural_node.clone(),
            source_authority: row.source_authority.to_string(),
            source_identity: row.source_identity.to_string(),
            qualified_declaration: row
                .qualified_declaration
                .iter()
                .map(ToString::to_string)
                .collect(),
            record_key: OriginalNodeKey::of(&row.record_key).map_err(|_| malformed())?,
            members: row
                .members
                .iter()
                .map(|member| {
                    Ok(SourceRecordMemberJson {
                        original_member_identity: member.original_member_identity.to_string(),
                        admitted_name: member.admitted_name.to_string(),
                        presence: match member.presence {
                            Presence::Required => PresenceJson::Required,
                            Presence::Optional => PresenceJson::Optional,
                        },
                        type_node: member.type_node.clone(),
                        value_type: OriginalValueType::of(&member.value_type)
                            .map_err(|_| malformed())?,
                    })
                })
                .collect::<Result<_, IdentityFileRefusal>>()?,
        })
    }

    pub(super) fn row(self) -> Result<SelectedSourceRecordRow, IdentityFileRefusal> {
        let malformed = || IdentityFileRefusal::Malformed {
            member: IdentityMember::SourceRecords,
        };
        Ok(SelectedSourceRecordRow {
            structural_node: self.structural_node,
            source_authority: self.source_authority.into(),
            source_identity: self.source_identity.into(),
            qualified_declaration: self
                .qualified_declaration
                .into_iter()
                .map(Into::into)
                .collect(),
            record_key: self.record_key.key().map_err(|_| malformed())?,
            members: self
                .members
                .into_iter()
                .map(|member| {
                    Ok(SelectedSourceRecordMemberRow {
                        original_member_identity: member.original_member_identity.into(),
                        admitted_name: member.admitted_name.into(),
                        presence: match member.presence {
                            PresenceJson::Required => Presence::Required,
                            PresenceJson::Optional => Presence::Optional,
                        },
                        type_node: member.type_node,
                        value_type: member.value_type.value().map_err(|_| malformed())?,
                    })
                })
                .collect::<Result<_, IdentityFileRefusal>>()?,
        })
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PopulationCatalogJson {
    model_root: CheckedNodeId,
    selected_document: String,
    populations: Vec<PopulationJson>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PopulationJson {
    original: LinkJson,
    members: Vec<PopulationMemberJson>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PopulationMemberJson {
    original: LinkJson,
    effective_type: EffectiveJson,
}

fn original_json(original: &OriginalDeclarationKey) -> LinkJson {
    LinkJson {
        package: original.package().to_owned(),
        node: original.node().to_owned(),
    }
}

impl PopulationCatalogJson {
    pub(super) fn of(row: &SelectedPopulationCatalog) -> Self {
        Self {
            model_root: row.model_root.clone(),
            selected_document: row.selected_document.to_string(),
            populations: row
                .populations
                .iter()
                .map(|population| PopulationJson {
                    original: original_json(&population.original),
                    members: population
                        .members
                        .iter()
                        .map(|member| PopulationMemberJson {
                            original: original_json(&member.original),
                            effective_type: EffectiveJson {
                                chain: member
                                    .effective_type
                                    .owner_chain()
                                    .iter()
                                    .map(original_json)
                                    .collect(),
                            },
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    pub(super) fn row(self) -> Result<SelectedPopulationCatalog, IdentityFileRefusal> {
        let malformed = |member| IdentityFileRefusal::Malformed { member };
        Ok(SelectedPopulationCatalog {
            model_root: self.model_root,
            selected_document: self.selected_document.into(),
            populations: self
                .populations
                .into_iter()
                .map(|population| {
                    Ok(SelectedPopulation {
                        original: OriginalDeclarationKey::new(
                            population.original.package,
                            population.original.node,
                        ),
                        members: population
                            .members
                            .into_iter()
                            .map(|member| {
                                if member.effective_type.chain.len() != 1 {
                                    return Err(malformed(IdentityMember::SelectedPopulations));
                                }
                                Ok(SelectedPopulationMember {
                                    original: OriginalDeclarationKey::new(
                                        member.original.package,
                                        member.original.node,
                                    ),
                                    effective_type: effective_id(
                                        &member.effective_type,
                                        &malformed,
                                    )?,
                                })
                            })
                            .collect::<Result<_, IdentityFileRefusal>>()?,
                    })
                })
                .collect::<Result<_, IdentityFileRefusal>>()?,
        })
    }
}
