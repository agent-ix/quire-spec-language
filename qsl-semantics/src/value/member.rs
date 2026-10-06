// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-013 O-06: checked member identity.
//!
//! [`Member`] mirrors the v2 `OperationMember` union exactly
//! (`node-identity-preimage.schema.json` `$defs.OperationMember`, resolved
//! from `agent-ix/quire-specification`'s `checked-package-v2` proposal,
//! which ADR-013 O-06 names as this type's serialized authority alongside
//! QSpec FR-322). `declaration` is a [`NodeKey`], unique across packages
//! (O-04); the kernel itself carries a member only as an opaque `MemberId`
//! digest (QC-15, `quire_exact::MemberId`) -- this structured type
//! is QSL's own, not the kernel's.
//!
//! No production caller constructs a [`Member`] yet: the layer-3 check stage
//! that resolves members (O-06's own "Owner" row) is `CheckedGraph`, ADR-013
//! T-1, which is S-3's row. This type and its total wire mapping (its
//! `Serialize`, C-18) are S-2's to land regardless -- ADR-013 §7 gates S-2 on S-1 and the
//! QSpec tickets only, not on S-3 -- exactly as the kernel identity newtypes in
//! `quire-exact::identity` landed in S-1 ahead of the checker that will use
//! them, with tests as their only caller until then.

use super::counter::Counter;
use quire_canonical::FixedShape;
use serde::Serialize;

use qsl_forms::StateClauseKind;
use qsl_foundation::digest::{DigestDomain, DigestRecord};
use quire_exact::Identifier;
use quire_exact::NodeKey;

// `NODE_KEY_DOMAIN` is a test-only import now that `declaration_json` mints
// through `DigestRecord`/`DigestDomain` (O-18 fold, #260 review item 5): the
// tests below still assert the wire shape against the domain's own constant.
#[cfg(test)]
use quire_exact::NODE_KEY_DOMAIN;

/// One closed checked-member identity (ADR-013 O-06).
///
/// Equality is `derive`d, which already gives ADR-013's own "declared"
/// equality rule -- "(declaring node id, identifier or declared position)"
/// -- directly: two `Member`s are equal only when they are the same variant
/// (the wire union's own `kind` discriminant) with equal fields, so a
/// `Field` and an `Operation` sharing a declaration and name are never
/// confused, matching the schema's own discriminated-union shape.
///
/// quire:canonical
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Member {
    /// A composite type's named field.
    Field {
        /// The declaring composite type.
        declaration: NodeKey,
        /// The field's own identifier.
        name: Identifier,
    },
    /// A tuple's positional element.
    Position {
        /// The declaring tuple type.
        declaration: NodeKey,
        /// The zero-based position.
        position: u64,
    },
    /// A collection's element (position-independent).
    Element {
        /// The declaring collection type.
        declaration: NodeKey,
    },
    /// A relationship's named end.
    RelationshipEnd {
        /// The declaring relationship.
        declaration: NodeKey,
        /// The end's own identifier.
        name: Identifier,
    },
    /// A type's named operation.
    Operation {
        /// The declaring type.
        declaration: NodeKey,
        /// The operation's own identifier.
        name: Identifier,
    },
    /// A generic declaration's type argument (position-independent: a
    /// declaration has at most one type argument in this catalog).
    TypeArgument {
        /// The declaring generic type.
        declaration: NodeKey,
    },
    /// A semantic profile's operator, named directly rather than through a
    /// declaring node -- the only variant with no `declaration` (the schema's
    /// own shape: `{"kind": "profile_operator", "operator": ...}`, no
    /// `declaration` member).
    ProfileOperator {
        /// The operator's own identifier, e.g. `"add"`.
        operator: Identifier,
    },
    /// A `quire.op.state.clause` application's own clause kind (QSpec
    /// STD-111, FR-341): the only difference between an invariant's,
    /// precondition's and postcondition's spellings. Like
    /// [`Self::ProfileOperator`], this variant carries no `declaration` --
    /// the schema's shape is `{"kind": "state_clause", "clause": ...}`.
    StateClause {
        /// Invariant, precondition or postcondition -- an invalid kind is
        /// unrepresentable, since [`StateClauseKind`] is itself a closed enum.
        clause: StateClauseKind,
    },
}

impl Member {
    /// The declaring node this member names, or `None` for a
    /// [`Self::ProfileOperator`], which names none.
    pub fn declaration(&self) -> Option<NodeKey> {
        match self {
            Self::Field { declaration, .. }
            | Self::Position { declaration, .. }
            | Self::Element { declaration }
            | Self::RelationshipEnd { declaration, .. }
            | Self::Operation { declaration, .. }
            | Self::TypeArgument { declaration } => Some(*declaration),
            Self::ProfileOperator { .. } | Self::StateClause { .. } => None,
        }
    }

    /// This member's total v2 wire encoding (C-18): exactly the
    /// `node-identity-preimage.schema.json` `OperationMember` shape, over
    /// every variant with no `_` arm, so a new variant fails to compile here
    /// until this match grows an arm for it.
    fn wire(&self) -> MemberWire<'_> {
        // O-18 fold (#260 review item 5): a `NodeKey` is exactly a
        // `DigestDomain::CheckedSemanticNodeV1` digest (`NODE_KEY_DOMAIN` is
        // that domain's own label), so this member's declaration digest
        // mints through `DigestRecord` -- a real caller for the type, not
        // the bare domain-string-plus-raw-hex construction this helper used
        // before. `DigestRecord::hex()` and `NodeKey`'s own `Display` are
        // both lowercase 2-digit-per-byte hex.
        fn declaration_wire(declaration: &NodeKey) -> DeclarationWire {
            let record =
                DigestRecord::mint(DigestDomain::CheckedSemanticNodeV1, *declaration.as_bytes());
            DeclarationWire {
                domain: record.domain().to_string(),
                digest: record.hex(),
            }
        }
        match self {
            Self::Field { declaration, name } => MemberWire::Field {
                declaration: declaration_wire(declaration),
                name: name.as_str(),
            },
            Self::Position {
                declaration,
                position,
            } => MemberWire::Position {
                declaration: declaration_wire(declaration),
                position: Counter::new(*position),
            },
            Self::Element { declaration } => MemberWire::Element {
                declaration: declaration_wire(declaration),
            },
            Self::RelationshipEnd { declaration, name } => MemberWire::RelationshipEnd {
                declaration: declaration_wire(declaration),
                name: name.as_str(),
            },
            Self::Operation { declaration, name } => MemberWire::Operation {
                declaration: declaration_wire(declaration),
                name: name.as_str(),
            },
            Self::TypeArgument { declaration } => MemberWire::TypeArgument {
                declaration: declaration_wire(declaration),
            },
            Self::ProfileOperator { operator } => MemberWire::ProfileOperator {
                operator: operator.as_str(),
            },
            Self::StateClause { clause } => MemberWire::StateClause {
                clause: state_clause_spelling(*clause),
            },
        }
    }
}

/// The `OperationMember` wire union, tagged by `kind`.
#[derive(Serialize, FixedShape)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum MemberWire<'a> {
    Field {
        declaration: DeclarationWire,
        name: &'a str,
    },
    Position {
        declaration: DeclarationWire,
        position: Counter,
    },
    Element {
        declaration: DeclarationWire,
    },
    RelationshipEnd {
        declaration: DeclarationWire,
        name: &'a str,
    },
    Operation {
        declaration: DeclarationWire,
        name: &'a str,
    },
    TypeArgument {
        declaration: DeclarationWire,
    },
    ProfileOperator {
        operator: &'a str,
    },
    StateClause {
        clause: &'static str,
    },
}

/// A member's declaring node: `{domain, digest}`.
#[derive(Serialize, FixedShape)]
struct DeclarationWire {
    domain: String,
    digest: String,
}

/// FR-341's own three spellings of a [`StateClauseKind`].
fn state_clause_spelling(clause: StateClauseKind) -> &'static str {
    match clause {
        StateClauseKind::Invariant => "invariant",
        StateClauseKind::Precondition => "precondition",
        StateClauseKind::Postcondition => "postcondition",
    }
}

/// Serializes as its `OperationMember` wire form, the one encoding of a
/// member, also when it is embedded in a larger preimage (the FR-322
/// application-node key's `operation.member`).
impl Serialize for Member {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.wire().serialize(serializer)
    }
}

/// A [`Member`] serializes as a `MemberWire`, so it nests as deep.
impl FixedShape for Member {
    const DEPTH: usize = <MemberWire<'static> as FixedShape>::DEPTH;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    /// `member`'s serialized wire form.
    fn to_wire(member: &Member) -> Value {
        serde_json::to_value(member).expect("a member serializes")
    }

    fn node(fill: u8) -> NodeKey {
        NodeKey::from_digest([fill; 32])
    }

    /// (#213 S-2, C-18) one test per variant: each renders exactly the
    /// `OperationMember` shape, field-for-field.
    #[test]
    fn field_renders_the_schema_shape() {
        let member = Member::Field {
            declaration: node(1),
            name: Identifier::new("quantity").unwrap(),
        };
        let wire = to_wire(&member);
        assert_eq!(
            wire,
            json!({
                "kind": "field",
                "declaration": {"domain": NODE_KEY_DOMAIN, "digest": node(1).to_string()},
                "name": "quantity",
            })
        );
    }

    #[test]
    fn position_renders_the_schema_shape() {
        let member = Member::Position {
            declaration: node(2),
            position: 3,
        };
        let wire = to_wire(&member);
        assert_eq!(
            wire,
            json!({
                "kind": "position",
                "declaration": {"domain": NODE_KEY_DOMAIN, "digest": node(2).to_string()},
                "position": 3,
            })
        );
    }

    #[test]
    fn element_renders_the_schema_shape() {
        let member = Member::Element {
            declaration: node(3),
        };
        let wire = to_wire(&member);
        assert_eq!(
            wire,
            json!({
                "kind": "element",
                "declaration": {"domain": NODE_KEY_DOMAIN, "digest": node(3).to_string()},
            })
        );
    }

    #[test]
    fn relationship_end_renders_the_schema_shape() {
        let member = Member::RelationshipEnd {
            declaration: node(4),
            name: Identifier::new("source").unwrap(),
        };
        let wire = to_wire(&member);
        assert_eq!(
            wire,
            json!({
                "kind": "relationship_end",
                "declaration": {"domain": NODE_KEY_DOMAIN, "digest": node(4).to_string()},
                "name": "source",
            })
        );
    }

    #[test]
    fn operation_renders_the_schema_shape() {
        let member = Member::Operation {
            declaration: node(5),
            name: Identifier::new("totalPrice").unwrap(),
        };
        let wire = to_wire(&member);
        assert_eq!(
            wire,
            json!({
                "kind": "operation",
                "declaration": {"domain": NODE_KEY_DOMAIN, "digest": node(5).to_string()},
                "name": "totalPrice",
            })
        );
    }

    #[test]
    fn type_argument_renders_the_schema_shape() {
        let member = Member::TypeArgument {
            declaration: node(6),
        };
        let wire = to_wire(&member);
        assert_eq!(
            wire,
            json!({
                "kind": "type_argument",
                "declaration": {"domain": NODE_KEY_DOMAIN, "digest": node(6).to_string()},
            })
        );
    }

    /// FR-341's own catalog member shape (QSpec STD-111).
    #[test]
    fn state_clause_renders_the_schema_shape_with_no_declaration() {
        let member = Member::StateClause {
            clause: StateClauseKind::Invariant,
        };
        let wire = to_wire(&member);
        assert_eq!(
            wire,
            json!({
                "kind": "state_clause",
                "clause": "invariant",
            })
        );
        assert_eq!(member.declaration(), None);
    }

    #[test]
    fn profile_operator_renders_the_schema_shape_with_no_declaration() {
        let member = Member::ProfileOperator {
            operator: Identifier::new("add").unwrap(),
        };
        let wire = to_wire(&member);
        assert_eq!(
            wire,
            json!({
                "kind": "profile_operator",
                "operator": "add",
            })
        );
    }

    /// ADR-013 O-06's own equality rule: a `Field` and an `Operation` on the
    /// same declaration with the same name are different members, because
    /// they are different variants of the discriminated union -- not merely
    /// different `(declaration, name)` tuples.
    #[test]
    fn field_and_operation_of_the_same_name_are_unequal() {
        let field = Member::Field {
            declaration: node(7),
            name: Identifier::new("same").unwrap(),
        };
        let operation = Member::Operation {
            declaration: node(7),
            name: Identifier::new("same").unwrap(),
        };
        assert_ne!(field, operation);
    }
}
