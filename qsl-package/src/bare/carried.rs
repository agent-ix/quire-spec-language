// SPDX-License-Identifier: AGPL-3.0-or-later
//! Required final producer facts in the identities/2 carrier (FR-531).

use quire_contract_model::{CheckedNodeId, SelectedSourceRecordMemberRow, SelectedSourceRecordRow};
use quire_exact::Presence;
use serde::{Deserialize, Serialize};

use super::{IdentityFileRefusal, IdentityMember};
use crate::equality_file::{OriginalNodeKey, OriginalValueType};

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
