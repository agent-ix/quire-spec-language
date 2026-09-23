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
//! This module computes both preimage digests ([`NodeIdentityPreimage`]) and
//! compares a retained key's bytes with them at admission. It never
//! constructs a `NodeKey`: only `check` mints one (ADR-011 §6.1, ADR-013
//! O-04). A member preimage's `declaration_node_id` stays a
//! [`WireNodeId`] until `admit_member` resolves it against the admitted
//! declaration's key.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::outcome::{Outcome, Stop};
use super::semantic_node::{
    is_qualified_name, preimage_digest, refuse, retains, CanonicalNodeId, CanonicalOwner,
    InvalidSemanticGraph, NodeIdDocument, NodeIdentityPreimage, NodeOwner, OwnerSelection,
    SemanticGraphCause,
};
use qsl_foundation::digest::WireNodeId;
use quire_exact::NodeKey;
use quire_exact::VariantId;
use quire_exact::{is_identifier, Charge, ChargePoint, LimitKind, Meter};
use quire_exact::{ComparisonOperator, IllTyped, IllTypedCause};

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

// JCS preimages: fields are declared in ascending key order (see `semantic_node`).
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
}

impl NodeIdentityPreimage for EnumDeclarationPreimage {
    fn digest(&self) -> Result<[u8; 32], InvalidSemanticGraph> {
        preimage_digest(&CanonicalDeclaration {
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
    declaration: WireNodeId,
    case: String,
}

impl EnumMemberPreimage {
    /// Read a preimage object; any schema violation is non-canonical.
    pub fn from_json(value: serde_json::Value) -> Result<Self, InvalidSemanticGraph> {
        let document: MemberDocument = serde_json::from_value(value)
            .map_err(|_| refuse(SemanticGraphCause::NonCanonicalPreimage))?;
        let declaration = document
            .declaration_node_id
            .wire_id()
            .filter(|_| document.version == MEMBER_VERSION && is_identifier(&document.case))
            .ok_or(refuse(SemanticGraphCause::NonCanonicalPreimage))?;
        Ok(Self {
            declaration,
            case: document.case,
        })
    }

    /// The referenced declaration node id, as read.
    pub fn declaration(&self) -> WireNodeId {
        self.declaration
    }

    /// The member case name.
    pub fn case(&self) -> &str {
        &self.case
    }
}

impl NodeIdentityPreimage for EnumMemberPreimage {
    fn digest(&self) -> Result<[u8; 32], InvalidSemanticGraph> {
        preimage_digest(&CanonicalMember {
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
        if !retains(key, &preimage)? {
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
        // Resolve the member's wire declaration id by lookup against this
        // declaration's own key (ADR-013 O-04).
        if preimage.declaration.as_bytes() != self.key.as_bytes() {
            return Err(refuse(SemanticGraphCause::ForeignDeclaration));
        }
        let position = self
            .preimage
            .members
            .iter()
            .position(|case| *case == preimage.case)
            .ok_or(refuse(SemanticGraphCause::UndeclaredCase))?;
        if !retains(key, preimage)? {
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
#[derive(Clone, Debug, Eq, PartialEq)]
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

    /// Zero-based FR-141 canonical-list position of the member: the same
    /// value ADR-013 O-14/OQ-D's `EnumMember::rank` carries once a caller
    /// pairs it with this member's [`Self::variant`]. Widened from
    /// `pub(crate)` (this change): an external caller building an
    /// `EnumShape`/`Value::Enum` from an admitted [`EnumValue`] -- exactly
    /// what `check::check::Typer::name` and `check::EnumBinding::shape` do
    /// inside this crate -- needs it too.
    pub fn position(&self) -> usize {
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

    /// This member's kernel `VariantId` (ADR-013 O-14, OQ-F ruling): the same
    /// `quire.checked-semantic-node/v1` bytes as [`Self::member`], retyped.
    /// [`Self::member`]'s bytes are already the OQ-F-ruled `quire.enum-
    /// member-node/v1` preimage digest -- verified against that exact
    /// preimage at [`EnumDeclaration::admit_member`] -- so this needs no
    /// fresh computation, only the kernel's own opaque wrapper.
    pub fn variant(&self) -> VariantId {
        VariantId::from_digest(*self.member.as_bytes())
    }
}

/// Mint the FR-141/OQ-F enum-member `VariantId` of a member `case` of the
/// enum declaration keyed by `declaration`: the node-key digest over
/// `{version: quire.enum-member-node/v1, declaration_node_id, case}`
/// (ADR-013 O-14 "Sum types", OQ-F ruling). This is the exact preimage
/// [`EnumMemberPreimage::digest`] recomputes at admission over a retained
/// key; this function mints one directly for a caller with no already-
/// admitted member node to retain a key from, such as `check::identity`'s
/// checked-type-node conversion (ADR-013 C-26). Replaces the private
/// `"sum-variant-member"` preimage `check/identity.rs:570` carried before
/// this change, which did not conform to FR-141's member identity.
pub fn mint_variant_id(declaration: NodeKey, case: &str) -> VariantId {
    let digest = preimage_digest(&CanonicalMember {
        case,
        declaration_node_id: declaration.into(),
        version: MEMBER_VERSION,
    })
    .expect(
        "a declaration NodeKey and a validated case identifier are always \
         representable as canonical JSON",
    );
    VariantId::from_digest(digest)
}

/// ADR-013 T-6 (last sentence): the checked `VariantId` -> [`EnumValue`]
/// index, built once as `check` admits each enum member and consulted
/// wherever an evaluated kernel `Value::Enum` (a bare `VariantId` and rank,
/// ADR-013 O-14/OQ-D) needs its declaration, ordered flag, position or case
/// name back. The kernel is a leaf and carries none of this (`quire_exact::
/// value`'s own module doc); the FR-141 enum-specific `=`/ordering schedule
/// ([`compare_enum`], `value::declaration::CheckedEquality`'s `Enum`
/// schedule, and `value::expression::evaluate`'s `OrderedKind::Enums` arm)
/// resolves a `VariantId` back to its full [`EnumValue`] through this index
/// rather than the kernel ever holding one.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EnumMemberIndex(BTreeMap<VariantId, EnumValue>);

impl EnumMemberIndex {
    /// Record one admitted member, keyed by its own `VariantId`. The last
    /// write for a given `VariantId` wins; two structurally identical
    /// members (same declaration, same case) always share one `VariantId`
    /// (content-addressed, ADR-013 O-04), so recording either is equivalent.
    pub fn record(&mut self, member: EnumValue) {
        self.0.insert(member.variant(), member);
    }

    /// The full checked member `variant` names, or `None` when it was never
    /// recorded -- never re-derived by any other means (R-05).
    pub fn resolve(&self, variant: VariantId) -> Option<&EnumValue> {
        self.0.get(&variant)
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
