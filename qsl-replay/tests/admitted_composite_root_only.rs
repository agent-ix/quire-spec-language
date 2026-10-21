// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1090 (FR-396-AC-1, FR-396-AC-6): a downstream generator compiles
//! authentic QSL source with a record, a tuple and a union through `compile_admitted`, looks both
//! declarations' keys up through the `qsl_replay` root, and builds the
//! kernel composite declarations, `ValueType::Composite` types and values
//! from those keys alone. This file names no `qsl_replay::spine` or
//! `qsl_semantics` item, and builds no key: it decodes no digest, hashes
//! nothing and calls no `NodeKey` constructor.

use ix_trace_rs::trace;
use qsl_replay::{
    compile_admitted, AdmittedCompositeForm, AdmittedContext, DependencyInput, Identifier,
    ReplayLimits, SourceIdentity, StageLimits, WireNodeId,
};
use quire_exact::{FieldValue, Presence, Value, ValueType};
use quire_semantic_value::declaration::{
    CompositeDeclaration, CompositeShape, FieldDeclaration, TypeEnvironment, UnionMemberDeclaration,
};

const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"quire.value.complete/v1\";\n\
     tuple Pair(Boolean, Boolean);\n\
     record Holder { flag: Boolean; pair: Pair; }\n\
     union Shape { Empty, Circle(Integer), Rect(Pair, Boolean) }\n";

fn context() -> AdmittedContext {
    compile_admitted(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        SOURCE.as_bytes(),
        [],
        &DependencyInput::default(),
        StageLimits::default(),
        ReplayLimits::default(),
    )
    .expect("the unit compiles")
    .into_parts()
    .1
}

/// The wire node id of the one admitted composite named `name`, as a
/// consumer reads it from the emitted package or a Contract IR document.
fn wire_id_of(context: &AdmittedContext, name: &str) -> WireNodeId {
    let name = Identifier::new(name).expect("an identifier");
    let found = context
        .composites()
        .find(|composite| composite.qualified_name().last() == Some(&name))
        .expect("the declaration is admitted");
    WireNodeId::from_digest(*found.node().as_bytes())
}

#[trace("TC-1090", "FR-396-AC-1")]
#[test]
fn a_consumer_builds_composite_declarations_from_admitted_keys() {
    let context = context();
    let pair = context
        .composite_declaration(wire_id_of(&context, "Pair"))
        .expect("the tuple is found");
    let holder = context
        .composite_declaration(wire_id_of(&context, "Holder"))
        .expect("the record is found");
    assert_eq!(pair.form(), AdmittedCompositeForm::Tuple);
    assert_eq!(holder.form(), AdmittedCompositeForm::Record);
    assert_ne!(pair.key(), holder.key());

    let types = TypeEnvironment::new(
        [
            CompositeDeclaration::new(
                pair.key(),
                "Pair",
                CompositeShape::Tuple(vec![ValueType::Boolean, ValueType::Boolean]),
            ),
            CompositeDeclaration::new(
                holder.key(),
                "Holder",
                CompositeShape::Record(vec![
                    FieldDeclaration::new("flag", ValueType::Boolean, Presence::Required),
                    FieldDeclaration::new(
                        "pair",
                        ValueType::Composite(pair.key()),
                        Presence::Required,
                    ),
                ]),
            ),
        ],
        [],
    )
    .expect("the admitted keys form one closed environment");

    let pair_value = types
        .tuple(
            pair.key(),
            vec![Value::Boolean(true), Value::Boolean(false)],
        )
        .expect("a Pair value");
    let holder_value = types
        .record(
            holder.key(),
            vec![
                ("flag", FieldValue::Present(Value::Boolean(true))),
                ("pair", FieldValue::Present(pair_value)),
            ],
        )
        .expect("a Holder value");
    assert!(types
        .admits(&ValueType::Composite(holder.key()), &holder_value)
        .expect("admission runs"));
}

/// FR-396-AC-6: a union's key, member identifiers, `VariantId`s and payload
/// types come from the `compile_admitted` result through the replay root
/// alone, and declare a closed `TypeEnvironment` with the record, tuple and
/// union under the admitted keys.
#[trace("TC-1090", "FR-396-AC-6")]
#[test]
fn a_consumer_gets_a_unions_key_and_member_ids_from_the_context() {
    let context = context();
    let pair = context
        .composite_declaration(wire_id_of(&context, "Pair"))
        .expect("the tuple is found");
    let shape = context
        .composite_declaration(wire_id_of(&context, "Shape"))
        .expect("the union is found");
    assert_eq!(shape.form(), AdmittedCompositeForm::Union);
    let members = shape.union_members().expect("a union has members");
    let names: Vec<_> = members.iter().map(|m| m.identifier().as_str()).collect();
    assert_eq!(names, ["Empty", "Circle", "Rect"]);
    let variants: std::collections::BTreeSet<_> = members.iter().map(|m| m.variant()).collect();
    assert_eq!(variants.len(), 3, "each member has its own id");
    assert_eq!(
        members[2].payload(),
        [ValueType::Composite(pair.key()), ValueType::Boolean]
    );

    let declared = members
        .iter()
        .map(|member| {
            UnionMemberDeclaration::resolved(member.identifier().clone(), member.payload().to_vec())
        })
        .collect();
    TypeEnvironment::new(
        [
            CompositeDeclaration::new(
                pair.key(),
                "Pair",
                CompositeShape::Tuple(vec![ValueType::Boolean, ValueType::Boolean]),
            ),
            CompositeDeclaration::new(shape.key(), "Shape", CompositeShape::Union(declared)),
        ],
        [],
    )
    .expect("the admitted keys form one closed environment");
}
