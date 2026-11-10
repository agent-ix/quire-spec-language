// SPDX-License-Identifier: AGPL-3.0-or-later
//! Required final producer facts in the identities/4 carrier (FR-531).

use quire_contract_model::{
    CheckedNodeId, SelectedSourceRecordMemberRow, SelectedSourceRecordRow, SelectedSourceUnionRow,
    SelectedUnionMemberRow,
};
use quire_exact::{Presence, UnionMember};
use serde::{Deserialize, Serialize};

use super::{IdentityFileRefusal, IdentityMember};
use crate::equality_file::{OriginalNodeKey, OriginalValueType};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SourceUnionJson {
    structural_node: CheckedNodeId,
    source_authority: String,
    source_identity: String,
    qualified_declaration: Vec<String>,
    type_handle: OriginalNodeKey,
    settled_key: OriginalNodeKey,
    members: Vec<SourceUnionMemberJson>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SourceUnionMemberJson {
    declaration: OriginalNodeKey,
    identifier: String,
    payload_nodes: Vec<CheckedNodeId>,
    payload: Vec<OriginalValueType>,
}

impl SourceUnionJson {
    pub(super) fn of(row: &SelectedSourceUnionRow) -> Result<Self, IdentityFileRefusal> {
        let malformed = || IdentityFileRefusal::Malformed {
            member: IdentityMember::SourceUnions,
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
            type_handle: OriginalNodeKey::of(&row.type_handle).map_err(|_| malformed())?,
            settled_key: OriginalNodeKey::of(&row.settled_key).map_err(|_| malformed())?,
            members: row
                .members
                .iter()
                .map(|member| {
                    Ok(SourceUnionMemberJson {
                        declaration: OriginalNodeKey::of(member.member.declaration())
                            .map_err(|_| malformed())?,
                        identifier: member.member.identifier().to_owned(),
                        payload_nodes: member.payload_nodes.clone(),
                        payload: member
                            .payload
                            .iter()
                            .map(|value| OriginalValueType::of(value).map_err(|_| malformed()))
                            .collect::<Result<_, _>>()?,
                    })
                })
                .collect::<Result<_, IdentityFileRefusal>>()?,
        })
    }

    pub(super) fn row(self) -> Result<SelectedSourceUnionRow, IdentityFileRefusal> {
        let malformed = || IdentityFileRefusal::Malformed {
            member: IdentityMember::SourceUnions,
        };
        Ok(SelectedSourceUnionRow {
            structural_node: self.structural_node,
            source_authority: self.source_authority.into(),
            source_identity: self.source_identity.into(),
            qualified_declaration: self
                .qualified_declaration
                .into_iter()
                .map(Into::into)
                .collect(),
            type_handle: self.type_handle.key().map_err(|_| malformed())?,
            settled_key: self.settled_key.key().map_err(|_| malformed())?,
            members: self
                .members
                .into_iter()
                .map(|member| {
                    Ok(SelectedUnionMemberRow {
                        member: UnionMember::from_admitted(
                            member.declaration.key().map_err(|_| malformed())?,
                            member.identifier,
                        )
                        .map_err(|_| malformed())?,
                        payload_nodes: member.payload_nodes,
                        payload: member
                            .payload
                            .into_iter()
                            .map(|value| value.value().map_err(|_| malformed()))
                            .collect::<Result<_, _>>()?,
                    })
                })
                .collect::<Result<_, IdentityFileRefusal>>()?,
        })
    }
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
pub(super) struct SourceOperationJson {
    structural_node: CheckedNodeId,
    source_authority: String,
    source_identity: String,
    anchor_id: CheckedNodeId,
}

impl SourceOperationJson {
    pub(super) fn of(row: &crate::SourceOperationRow) -> Self {
        Self {
            structural_node: row.structural_node.clone(),
            source_authority: row.source_authority.clone(),
            source_identity: row.source_identity.clone(),
            anchor_id: row.anchor_id.clone(),
        }
    }

    pub(super) fn row(self) -> crate::SourceOperationRow {
        crate::SourceOperationRow {
            structural_node: self.structural_node,
            source_authority: self.source_authority,
            source_identity: self.source_identity,
            anchor_id: self.anchor_id,
        }
    }
}
