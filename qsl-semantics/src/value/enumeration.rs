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
//!
//! The runtime half -- the structural [`EnumDeclaration`], [`EnumValue`],
//! the member index and the FR-141 comparison -- is
//! `quire_semantic_value::enumeration` (ADR-011 §6.1 layer SV).
//! [`AdmittedEnumDeclaration`] pairs that declaration with the preimage its
//! key was verified against.

use serde::{Deserialize, Serialize};

use super::semantic_node::{
    is_qualified_name, preimage_bytes, preimage_digest, refuse, retains, CanonicalOwner,
    NodeIdDocument, NodeIdentityPreimage, NodeOwner, OwnerSelection,
};
use qsl_foundation::digest::WireNodeId;
use quire_exact::is_identifier;
use quire_exact::NodeKey;
use quire_exact::VariantId;
use quire_semantic_value::enumeration::{is_member_list, EnumDeclaration, EnumValue};
use quire_semantic_value::semantic_node::{
    CanonicalNodeId, InvalidSemanticGraph, SemanticGraphCause,
};

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
    #[qsl_attrs::string_edge]
    pub fn from_json(value: serde_json::Value) -> Result<Self, InvalidSemanticGraph> {
        let document: DeclarationDocument = serde_json::from_value(value)
            .map_err(|_| refuse(SemanticGraphCause::NonCanonicalPreimage))?;
        let well_formed = document.version == DECLARATION_VERSION
            && document.owner.is_well_formed()
            && is_qualified_name(&document.qualified_declaration)
            && is_member_list(&document.members);
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

    /// Build a preimage from its parts. It applies the schema checks
    /// [`Self::from_json`] applies and refuses with `NonCanonicalPreimage`:
    /// the members are nonempty, distinct identifiers, and the owner and
    /// the qualified name are well formed. Member order is the caller's:
    /// declaration order for an `ordered enum`, sorted otherwise (FR-141).
    pub fn new(
        owner: NodeOwner,
        qualified_declaration: Vec<String>,
        ordered: bool,
        members: Vec<String>,
    ) -> Result<Self, InvalidSemanticGraph> {
        let well_formed = owner.is_well_formed()
            && is_qualified_name(&qualified_declaration)
            && is_member_list(&members);
        if !well_formed {
            return Err(refuse(SemanticGraphCause::NonCanonicalPreimage));
        }
        Ok(Self {
            owner,
            qualified_declaration,
            ordered,
            members,
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

impl EnumDeclarationPreimage {
    fn canonical(&self) -> CanonicalDeclaration<'_> {
        canonical_declaration(
            &self.owner,
            &self.qualified_declaration,
            self.ordered,
            &self.members,
        )
    }
}

/// The JCS form of a `quire.enum-declaration-node/v1` preimage, from its
/// parts: one statement for the preimage and the admitted declaration.
fn canonical_declaration<'a>(
    owner: &'a NodeOwner,
    qualified_declaration: &'a [String],
    ordered: bool,
    members: &'a [String],
) -> CanonicalDeclaration<'a> {
    CanonicalDeclaration {
        members,
        ordered,
        owner: owner.canonical(),
        qualified_declaration,
        version: DECLARATION_VERSION,
    }
}

impl NodeIdentityPreimage for EnumDeclarationPreimage {
    fn digest(&self) -> Result<[u8; 32], InvalidSemanticGraph> {
        preimage_digest(&self.canonical())
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
    #[qsl_attrs::string_edge]
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

    /// Build a preimage from its parts: the case of the enum declaration
    /// keyed by `declaration`. It applies the schema check
    /// [`Self::from_json`] applies to the case and refuses with
    /// `NonCanonicalPreimage`.
    pub fn new(
        declaration: NodeKey,
        case: impl Into<String>,
    ) -> Result<Self, InvalidSemanticGraph> {
        let case = case.into();
        if !is_identifier(&case) {
            return Err(refuse(SemanticGraphCause::NonCanonicalPreimage));
        }
        Ok(Self {
            declaration: WireNodeId::from_digest(*declaration.as_bytes()),
            case,
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
            declaration_node_id: CanonicalNodeId::from(*self.declaration.as_bytes()),
            version: MEMBER_VERSION,
        })
    }
}

/// An admitted enum declaration node: the runtime [`EnumDeclaration`]
/// (key, ordering, members) together with the rest of the preimage its key
/// was verified against (owner and qualified name). The members are held
/// once, by the runtime declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedEnumDeclaration {
    declaration: EnumDeclaration,
    owner: NodeOwner,
    qualified_declaration: Vec<String>,
}

impl AdmittedEnumDeclaration {
    /// Admit a declaration node whose graph retains `key`.
    ///
    /// Order: semantic well-formedness, owner join, then key recomputation.
    pub fn admit(
        preimage: EnumDeclarationPreimage,
        key: NodeKey,
        owners: &OwnerSelection,
    ) -> Result<Self, InvalidSemanticGraph> {
        let EnumDeclarationPreimage {
            owner,
            qualified_declaration,
            ordered,
            members,
        } = preimage;
        let declaration = EnumDeclaration::new(key, ordered, members)?;
        if !owners.contains(&owner) {
            return Err(refuse(SemanticGraphCause::OwnerNotSelected));
        }
        let admitted = Self {
            declaration,
            owner,
            qualified_declaration,
        };
        if preimage_digest(&admitted.canonical())? != *key.as_bytes() {
            return Err(refuse(SemanticGraphCause::StaleKey));
        }
        Ok(admitted)
    }

    fn canonical(&self) -> CanonicalDeclaration<'_> {
        canonical_declaration(
            &self.owner,
            &self.qualified_declaration,
            self.declaration.is_ordered(),
            self.declaration.members(),
        )
    }

    /// The declaration node key.
    pub fn key(&self) -> NodeKey {
        self.declaration.key()
    }

    /// The admitted runtime declaration.
    pub fn declaration(&self) -> &EnumDeclaration {
        &self.declaration
    }

    /// The owner projection.
    pub fn owner(&self) -> &NodeOwner {
        &self.owner
    }

    /// Qualified declaration name segments.
    pub fn qualified_declaration(&self) -> &[String] {
        &self.qualified_declaration
    }

    /// The admitted content, as a preimage.
    pub fn preimage(&self) -> EnumDeclarationPreimage {
        EnumDeclarationPreimage {
            owner: self.owner.clone(),
            qualified_declaration: self.qualified_declaration.clone(),
            ordered: self.declaration.is_ordered(),
            members: self.declaration.members().to_vec(),
        }
    }

    /// The RFC 8785 bytes whose SHA-256 is the declaration's node key: the
    /// checked graph node's preimage (FR-092 rule 1).
    pub(crate) fn preimage_bytes(&self) -> Result<Vec<u8>, InvalidSemanticGraph> {
        preimage_bytes(&self.canonical())
    }

    /// Admit a member node of this declaration whose graph retains `key`.
    pub fn admit_member(
        &self,
        preimage: &EnumMemberPreimage,
        key: NodeKey,
    ) -> Result<EnumValue, InvalidSemanticGraph> {
        // Resolve the member's wire declaration id by lookup against this
        // declaration's own key (ADR-013 O-04).
        if preimage.declaration.as_bytes() != self.key().as_bytes() {
            return Err(refuse(SemanticGraphCause::ForeignDeclaration));
        }
        let member = self.declaration.member(&preimage.case, key)?;
        if !retains(key, preimage)? {
            return Err(refuse(SemanticGraphCause::StaleKey));
        }
        Ok(member)
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

/// The RFC 8785 bytes of the `quire.enum-member-node/v1` preimage of member
/// `case` of the enum declaration keyed by `declaration`: the checked graph
/// node's preimage (FR-092 rule 1), whose SHA-256 is [`mint_variant_id`]'s
/// digest.
pub(crate) fn member_preimage_bytes(
    declaration: NodeKey,
    case: &str,
) -> Result<Vec<u8>, InvalidSemanticGraph> {
    preimage_bytes(&CanonicalMember {
        case,
        declaration_node_id: declaration.into(),
        version: MEMBER_VERSION,
    })
}
