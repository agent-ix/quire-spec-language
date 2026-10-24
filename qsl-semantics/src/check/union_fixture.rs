// SPDX-License-Identifier: AGPL-3.0-or-later
//! Sealed union declarations for evaluator and replay tests (FR-321, FR-322).
//!
//! No checked package can hold a union until the FR-319 lowering exists, so
//! the S6a evaluator and the replay argument conversion are tested over a
//! hand-built [`TypeEnvironment`] that declares these unions and seals each
//! one with its FR-441 member keys, minted by [`mint_union_member_key`]. A
//! union's declaration handle is the key a static type and a checked node
//! carry; its settled key is the key a union value carries.
//!
//! - `Shape { Empty, Circle(Integer), Rect(Integer, Integer) }`
//! - `Other { Only(Integer) }`
//! - `Tree { Leaf(Integer), Branch(Integer, Option<Tree>) }`
//! - `Holder { Some(Reference<T>), Nothing }`, `T` a model object type.

use quire_exact::{
    EffectiveId, Identifier, Integer, NodeKey, OptionValue, UnionMember, UnionValue, Value,
    ValueType,
};
use quire_semantic_value::declaration::{
    CompositeDeclaration, CompositeShape, ObjectTypeDeclaration, TypeEnvironment,
    UnionMemberDeclaration,
};

use super::mint_union_member_key;

fn digest(label: &str) -> [u8; 32] {
    qsl_foundation::digest::ByteDigest::of(label.as_bytes()).as_bytes()
}

/// The declaration handle of the fixture union `union`.
pub fn handle(union: &str) -> NodeKey {
    NodeKey::from_digest(digest(union))
}

/// The settled key of the fixture union `union`, the key its values carry.
pub fn settled(union: &str) -> NodeKey {
    NodeKey::from_digest(digest(&format!("{union} settled")))
}

/// The static type of the fixture union `union`.
pub fn union_type(union: &str) -> ValueType {
    ValueType::Composite(handle(union))
}

/// The members of the fixture union `union` in declared order, each with its
/// payload types.
#[ix_trace_rs::string_edge]
fn members(union: &str, object_type: EffectiveId) -> Vec<(&'static str, Vec<ValueType>)> {
    match union {
        "Shape" => vec![
            ("Empty", vec![]),
            ("Circle", vec![ValueType::Integer]),
            ("Rect", vec![ValueType::Integer, ValueType::Integer]),
        ],
        "Other" => vec![("Only", vec![ValueType::Integer])],
        "Tree" => vec![
            ("Leaf", vec![ValueType::Integer]),
            (
                "Branch",
                vec![ValueType::Integer, ValueType::option(union_type("Tree"))],
            ),
        ],
        "Holder" => vec![
            ("Some", vec![ValueType::Reference(object_type)]),
            ("Nothing", vec![]),
        ],
        other => panic!("{other} is no fixture union"),
    }
}

/// The member `name` of the fixture union `union`, as the sealed binding its
/// values carry.
pub fn member(union: &str, name: &str) -> UnionMember {
    UnionMember::from_admitted(
        settled(union),
        mint_union_member_key(settled(union), name),
        Identifier::new(name).expect("a fixture member name is an identifier"),
    )
}

/// A value of the fixture union `union` holding member `name` over `payload`.
pub fn value(union: &str, name: &str, payload: Vec<Value>) -> Value {
    UnionValue::from_admitted(member(union, name), payload)
}

/// The type environment declaring every fixture union, each sealed, and the
/// model object type `T` (`object_type`) that `Holder`'s `Some` references.
pub fn types(object_type: EffectiveId) -> TypeEnvironment {
    const UNIONS: [&str; 4] = ["Shape", "Other", "Tree", "Holder"];
    let declarations = UNIONS.iter().map(|union| {
        CompositeDeclaration::new(
            handle(union),
            *union,
            CompositeShape::Union(
                members(union, object_type)
                    .into_iter()
                    .map(|(name, payload)| {
                        UnionMemberDeclaration::resolved(
                            Identifier::new(name).expect("a fixture member name"),
                            payload,
                        )
                    })
                    .collect(),
            ),
        )
    });
    let mut types = TypeEnvironment::new(
        declarations,
        [ObjectTypeDeclaration::new(object_type, "T", Vec::new())],
    )
    .expect("the fixture unions are well-formed");
    for union in UNIONS {
        let bindings = members(union, object_type)
            .into_iter()
            .map(|(name, _)| member(union, name))
            .collect();
        types
            .seal_union_verified(handle(union), settled(union), bindings)
            .expect("the fixture union seals");
    }
    types
}

/// A `Tree` of `depth` levels: `depth - 1` branches, each holding 1 and the
/// present next level, over a leaf holding 1. At depth 10,000 it holds
/// 29,999 value occurrences (10,000 unions, 10,000 integers and 9,999
/// present options). Built from the leaf up with no recursion.
pub fn tree(depth: usize) -> Value {
    let one = || Value::Integer(Integer::one());
    let mut tree = value("Tree", "Leaf", vec![one()]);
    for _ in 1..depth {
        let child = OptionValue::from_admitted(union_type("Tree"), Some(tree));
        tree = value("Tree", "Branch", vec![one(), child]);
    }
    tree
}
