// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-141 declaration-qualified enumerations and their I04 nominal node
//! identities.
//!
//! A declaration's node key and a member's node key are the structural
//! [`NodeKey`]s the check stage carries; nothing here hashes or recomputes
//! one. Admission checks semantic well-formedness and the owner join, and a
//! member preimage's `declaration_node_id` must be the admitted
//! declaration's own key.
//!
//! The runtime half -- the structural [`EnumDeclaration`], [`EnumValue`],
//! the member index and the FR-141 comparison -- is
//! `quire_semantic_value::enumeration` (ADR-011 §6.1 layer SV).
//! [`AdmittedEnumDeclaration`] pairs that declaration with the owner and
//! qualified name it was admitted with.

use serde::Deserialize;

use super::semantic_node::{
    is_qualified_name, refuse, NodeIdDocument, NodeOwner, NominalRefusal, OwnerSelection,
};
use quire_exact::is_identifier;
use quire_exact::{EmptyVariantName, NodeKey, VariantId};
use quire_semantic_value::enumeration::{is_member_list, EnumDeclaration, EnumValue};
use quire_semantic_value::semantic_node::{InvalidSemanticGraph, SemanticGraphCause};

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
    #[ix_trace_rs::string_edge]
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

/// A `quire.enum-member-node/v1` preimage that satisfies the preimage schema.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumMemberPreimage {
    declaration: NodeKey,
    case: String,
}

impl EnumMemberPreimage {
    /// Read a preimage object; any schema violation is non-canonical.
    #[ix_trace_rs::string_edge]
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
            declaration,
            case,
        })
    }

    /// The referenced declaration node id, as read.
    pub fn declaration(&self) -> &NodeKey {
        &self.declaration
    }

    /// The member case name.
    pub fn case(&self) -> &str {
        &self.case
    }
}

/// An admitted enum declaration node: the runtime [`EnumDeclaration`]
/// (key, ordering, members) together with the rest of its declaration
/// (owner and qualified name). The members are held
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
    /// Order: semantic well-formedness, then the owner join.
    pub fn admit(
        preimage: EnumDeclarationPreimage,
        key: NodeKey,
        owners: &OwnerSelection,
    ) -> Result<Self, NominalRefusal> {
        let EnumDeclarationPreimage {
            owner,
            qualified_declaration,
            ordered,
            members,
        } = preimage;
        let declaration = EnumDeclaration::new(key, ordered, members)?;
        if !owners.contains(&owner) {
            return Err(refuse(SemanticGraphCause::OwnerNotSelected).into());
        }
        Ok(Self {
            declaration,
            owner,
            qualified_declaration,
        })
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

    /// Admit a member node of this declaration whose graph retains `key`.
    pub fn admit_member(
        &self,
        preimage: &EnumMemberPreimage,
        key: NodeKey,
    ) -> Result<EnumValue, NominalRefusal> {
        // The member's declaration id must be this declaration's own key.
        if preimage.declaration != self.key() {
            return Err(refuse(SemanticGraphCause::ForeignDeclaration).into());
        }
        Ok(self.declaration.member(&preimage.case, key)?)
    }
}

/// Mint the FR-141/OQ-F enum-member `VariantId` of a member `case` of the
/// enum declaration keyed by `declaration`: the declaration node and the exact
/// case name (ADR-013 O-14 "Sum types").
///
/// # Errors
///
/// [`EmptyVariantName`] for an empty case name.
pub fn mint_variant_id(declaration: NodeKey, case: &str) -> Result<VariantId, EmptyVariantName> {
    VariantId::new(declaration, case)
}

/// The `VariantId` of member `member` of the union whose declaration node is
/// `declaration` (QSpec FR-441): the union declaration and the exact member
/// name.
///
/// # Errors
///
/// [`EmptyVariantName`] for an empty member name.
pub fn mint_union_variant_id(
    declaration: NodeKey,
    member: &str,
) -> Result<VariantId, EmptyVariantName> {
    VariantId::new(declaration, member)
}
