// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-141 declaration-qualified enumerations and their I04 nominal node
//! identities.
//!
//! A declaration node key is the SHA-256 of the RFC 8785 JCS encoding of its
//! `quire.enum-declaration-node/v1` preimage; a member node key hashes
//! `quire.enum-member-node/v1`. The strict reader recomputes both from the
//! admitted content and refuses a retained key that no longer matches with
//! `invalid_semantic_graph`. Display text and source loci are not part of
//! either preimage.
//!
//! `EnumDeclaration::admit`/`admit_member` mint these node ids the same way
//! `value::expression::check` mints checked-expression node ids (ADR-013
//! O-04), but this module is not itself a `check`-stage implementer: no
//! `src/` caller builds an `EnumDeclaration` today (#118 scoped this module
//! to I04/I05 identity semantics only, never a check-stage wiring), so this
//! is outside ADR-011 §6.1's "only `check` calls the kernel `NodeKey`
//! constructor" rule (ADR-011 FB-13, #211). Today it is exercised only by
//! this crate's own tests. Remaining work: #131 wires a check-stage caller.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::comparison::{ComparisonOperator, IllTyped, IllTypedCause};
use super::node::{
    is_qualified_name, node_key_of, refuse, CanonicalNodeId, CanonicalOwner, InvalidSemanticGraph,
    NodeIdDocument, NodeKey, NodeOwner, OwnerSelection, SemanticGraphCause,
};
use super::outcome::{Outcome, Stop};
use quire_exact::{is_identifier, Charge, ChargePoint, LimitKind, Meter};

const DECLARATION_VERSION: &str = "quire.enum-declaration-node/v1";
const MEMBER_VERSION: &str = "quire.enum-member-node/v1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeclarationDocument {
    version: String,
    owner: NodeOwner,
    qualified_declaration: Vec<String>,
    ordered: bool,
    members: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MemberDocument {
    version: String,
    declaration_node_id: NodeIdDocument,
    case: String,
}

// JCS preimages: fields are declared in ascending key order (see `node`).
#[derive(Serialize)]
struct CanonicalDeclaration<'a> {
    members: &'a [String],
    ordered: bool,
    owner: CanonicalOwner<'a>,
    qualified_declaration: &'a [String],
    version: &'static str,
}

#[derive(Serialize)]
struct CanonicalMember<'a> {
    case: &'a str,
    declaration_node_id: CanonicalNodeId,
    version: &'static str,
}

/// A `quire.enum-declaration-node/v1` preimage that satisfies the preimage
/// schema.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumDeclarationPreimage {
    owner: NodeOwner,
    qualified_declaration: Vec<String>,
    ordered: bool,
    members: Vec<String>,
}

impl EnumDeclarationPreimage {
    /// Read a preimage object; any schema violation is non-canonical.
    pub fn from_json(value: serde_json::Value) -> Result<Self, InvalidSemanticGraph> {
        let document: DeclarationDocument = serde_json::from_value(value)
            .map_err(|_| refuse(SemanticGraphCause::NonCanonicalPreimage))?;
        let distinct: BTreeSet<_> = document.members.iter().collect();
        let well_formed = document.version == DECLARATION_VERSION
            && document.owner.is_well_formed()
            && is_qualified_name(&document.qualified_declaration)
            && !document.members.is_empty()
            && distinct.len() == document.members.len()
            && document.members.iter().all(|case| is_identifier(case));
        if !well_formed {
            return Err(refuse(SemanticGraphCause::NonCanonicalPreimage));
        }
        Ok(Self {
            owner: document.owner,
            qualified_declaration: document.qualified_declaration,
            ordered: document.ordered,
            members: document.members,
        })
    }

    /// The owner projection.
    pub fn owner(&self) -> &NodeOwner {
        &self.owner
    }

    /// Qualified declaration name segments.
    pub fn qualified_declaration(&self) -> &[String] {
        &self.qualified_declaration
    }

    /// Whether the declaration selects ordered semantics.
    pub fn is_ordered(&self) -> bool {
        self.ordered
    }

    /// Declaration-ordered (or case-sorted, if unordered) member cases.
    pub fn members(&self) -> &[String] {
        &self.members
    }

    /// The node key this content determines.
    pub fn node_key(&self) -> Result<NodeKey, InvalidSemanticGraph> {
        node_key_of(&CanonicalDeclaration {
            members: &self.members,
            ordered: self.ordered,
            owner: self.owner.canonical(),
            qualified_declaration: &self.qualified_declaration,
            version: DECLARATION_VERSION,
        })
    }
}

/// A `quire.enum-member-node/v1` preimage that satisfies the preimage schema.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumMemberPreimage {
    declaration: NodeKey,
    case: String,
}

impl EnumMemberPreimage {
    /// Read a preimage object; any schema violation is non-canonical.
    pub fn from_json(value: serde_json::Value) -> Result<Self, InvalidSemanticGraph> {
        let document: MemberDocument = serde_json::from_value(value)
            .map_err(|_| refuse(SemanticGraphCause::NonCanonicalPreimage))?;
        let declaration = document
            .declaration_node_id
            .key()
            .filter(|_| document.version == MEMBER_VERSION && is_identifier(&document.case))
            .ok_or(refuse(SemanticGraphCause::NonCanonicalPreimage))?;
        Ok(Self {
            declaration,
            case: document.case,
        })
    }

    /// The referenced declaration node key.
    pub fn declaration(&self) -> NodeKey {
        self.declaration
    }

    /// The member case name.
    pub fn case(&self) -> &str {
        &self.case
    }

    /// The node key this content determines.
    pub fn node_key(&self) -> Result<NodeKey, InvalidSemanticGraph> {
        node_key_of(&CanonicalMember {
            case: &self.case,
            declaration_node_id: self.declaration.into(),
            version: MEMBER_VERSION,
        })
    }
}

/// An admitted enum declaration node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumDeclaration {
    key: NodeKey,
    preimage: EnumDeclarationPreimage,
}

impl EnumDeclaration {
    /// Admit a declaration node whose graph retains `key`.
    ///
    /// Order: semantic well-formedness, owner join, then key recomputation.
    pub fn admit(
        preimage: EnumDeclarationPreimage,
        key: NodeKey,
        owners: &OwnerSelection,
    ) -> Result<Self, InvalidSemanticGraph> {
        if !preimage.ordered && !preimage.members.is_sorted() {
            return Err(refuse(SemanticGraphCause::UnsortedUnorderedMembers));
        }
        if !owners.contains(&preimage.owner) {
            return Err(refuse(SemanticGraphCause::OwnerNotSelected));
        }
        if preimage.node_key()? != key {
            return Err(refuse(SemanticGraphCause::StaleKey));
        }
        Ok(Self { key, preimage })
    }

    /// The declaration node key.
    pub fn key(&self) -> NodeKey {
        self.key
    }

    /// The admitted content.
    pub fn preimage(&self) -> &EnumDeclarationPreimage {
        &self.preimage
    }

    /// Admit a member node of this declaration whose graph retains `key`.
    pub fn admit_member(
        &self,
        preimage: &EnumMemberPreimage,
        key: NodeKey,
    ) -> Result<EnumValue, InvalidSemanticGraph> {
        if preimage.declaration != self.key {
            return Err(refuse(SemanticGraphCause::ForeignDeclaration));
        }
        let position = self
            .preimage
            .members
            .iter()
            .position(|case| *case == preimage.case)
            .ok_or(refuse(SemanticGraphCause::UndeclaredCase))?;
        if preimage.node_key()? != key {
            return Err(refuse(SemanticGraphCause::StaleKey));
        }
        Ok(EnumValue {
            declaration: self.key,
            member: key,
            ordered: self.preimage.ordered,
            position,
            case: preimage.case.as_str().into(),
        })
    }
}

/// An enumeration value: (declaration identity, member identity).
#[derive(Clone, Debug)]
pub struct EnumValue {
    declaration: NodeKey,
    member: NodeKey,
    ordered: bool,
    position: usize,
    case: Box<str>,
}

impl EnumValue {
    /// Declaration node key.
    pub fn declaration(&self) -> NodeKey {
        self.declaration
    }

    /// Member node key.
    pub fn member(&self) -> NodeKey {
        self.member
    }

    /// Zero-based declaration position of the member.
    pub(crate) fn position(&self) -> usize {
        self.position
    }

    /// Whether the declaration is an `ordered enum`.
    pub fn is_ordered(&self) -> bool {
        self.ordered
    }

    /// The case identifier.
    pub fn case(&self) -> &str {
        &self.case
    }
}

/// Compare two enum values. Members of different declarations, and ordering
/// requests over an unordered declaration, are ill-typed and consume nothing.
pub fn compare_enum(
    operator: ComparisonOperator,
    left: &EnumValue,
    right: &EnumValue,
    meter: &mut Meter,
) -> Result<Outcome<bool>, IllTyped> {
    if left.declaration != right.declaration {
        return Err(IllTyped {
            cause: IllTypedCause::DistinctEnumDeclarations,
        });
    }
    if operator.is_ordering() && !left.ordered {
        return Err(IllTyped {
            cause: IllTypedCause::UnorderedEnumOrdering,
        });
    }
    Ok(Outcome::from_stop(compare(operator, left, right, meter)))
}

fn compare(
    operator: ComparisonOperator,
    left: &EnumValue,
    right: &EnumValue,
    meter: &mut Meter,
) -> Result<bool, Stop> {
    for read in 1..=2 {
        meter.charge(
            Charge::new(ChargePoint::EnumIdentityRead).size(LimitKind::ValueOccurrences, read),
        )?;
    }
    let ordering = if operator.is_ordering() {
        left.position.cmp(&right.position)
    } else {
        left.member.cmp(&right.member)
    };
    meter.charge(Charge::new(ChargePoint::EnumResultRetain).results(1))?;
    Ok(operator.holds(ordering))
}
