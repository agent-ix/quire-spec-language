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
//! T-1, which is S-3's row. This type and its total wire mapping ([`Member::to_wire`],
//! C-18) are S-2's to land regardless -- ADR-013 §7 gates S-2 on S-1 and the
//! QSpec tickets only, not on S-3 -- exactly as the kernel identity newtypes in
//! `quire-exact::identity` landed in S-1 ahead of the checker that will use
//! them, with tests as their only caller until then.

use serde_json::{json, Value};

use super::node::NodeKey;
use qsl_foundation::digest::{DigestDomain, DigestRecord};
use quire_exact::Identifier;

// `NODE_KEY_DOMAIN` is a test-only import now that `declaration_json` mints
// through `DigestRecord`/`DigestDomain` (O-18 fold, #260 review item 5): the
// tests below still assert the wire shape against the domain's own constant.
#[cfg(test)]
use super::node::NODE_KEY_DOMAIN;

/// One closed checked-member identity (ADR-013 O-06).
///
/// Equality is `derive`d, which already gives ADR-013's own "declared"
/// equality rule -- "(declaring node id, identifier or declared position)"
/// -- directly: two `Member`s are equal only when they are the same variant
/// (the wire union's own `kind` discriminant) with equal fields, so a
/// `Field` and an `Operation` sharing a declaration and name are never
/// confused, matching the schema's own discriminated-union shape.
#[derive(Clone, Debug, Eq, PartialEq)]
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
}

impl Member {
    /// This member's total v2 wire encoding (C-18): exactly the
    /// `node-identity-preimage.schema.json` `OperationMember` shape, over
    /// every variant with no `_` arm, so a new variant fails to compile here
    /// until this match grows an arm for it.
    pub fn to_wire(&self) -> Value {
        // O-18 fold (#260 review item 5): a `NodeKey` is exactly a
        // `DigestDomain::CheckedSemanticNodeV1` digest (`NODE_KEY_DOMAIN` is
        // that domain's own label), so this member's declaration digest
        // mints through `DigestRecord` -- a real caller for the type, not
        // the bare domain-string-plus-raw-hex construction this helper used
        // before. Byte-identical wire output: `DigestRecord::hex()` and
        // `NodeKey`'s own `Display` are both lowercase 2-digit-per-byte hex.
        fn declaration_json(declaration: &NodeKey) -> Value {
            let record =
                DigestRecord::mint(DigestDomain::CheckedSemanticNodeV1, *declaration.as_bytes());
            json!({
                "domain": record.domain().to_string(),
                "digest": record.hex(),
            })
        }
        match self {
            Self::Field { declaration, name } => json!({
                "kind": "field",
                "declaration": declaration_json(declaration),
                "name": name.as_str(),
            }),
            Self::Position {
                declaration,
                position,
            } => json!({
                "kind": "position",
                "declaration": declaration_json(declaration),
                "position": position,
            }),
            Self::Element { declaration } => json!({
                "kind": "element",
                "declaration": declaration_json(declaration),
            }),
            Self::RelationshipEnd { declaration, name } => json!({
                "kind": "relationship_end",
                "declaration": declaration_json(declaration),
                "name": name.as_str(),
            }),
            Self::Operation { declaration, name } => json!({
                "kind": "operation",
                "declaration": declaration_json(declaration),
                "name": name.as_str(),
            }),
            Self::TypeArgument { declaration } => json!({
                "kind": "type_argument",
                "declaration": declaration_json(declaration),
            }),
            Self::ProfileOperator { operator } => json!({
                "kind": "profile_operator",
                "operator": operator.as_str(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let wire = member.to_wire();
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
        let wire = member.to_wire();
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
        let wire = member.to_wire();
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
        let wire = member.to_wire();
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
        let wire = member.to_wire();
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
        let wire = member.to_wire();
        assert_eq!(
            wire,
            json!({
                "kind": "type_argument",
                "declaration": {"domain": NODE_KEY_DOMAIN, "digest": node(6).to_string()},
            })
        );
    }

    #[test]
    fn profile_operator_renders_the_schema_shape_with_no_declaration() {
        let member = Member::ProfileOperator {
            operator: Identifier::new("add").unwrap(),
        };
        let wire = member.to_wire();
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
