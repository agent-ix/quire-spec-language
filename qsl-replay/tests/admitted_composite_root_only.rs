// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1090 (FR-396-AC-1): a downstream generator compiles authentic QSL
//! source with a record and a tuple through `compile_admitted`, looks both
//! declarations' keys up through the `qsl_replay` root, and builds the
//! kernel composite declarations, `ValueType::Composite` types and values
//! from those keys alone. This file names no `qsl_replay::spine` or
//! `qsl_semantics` item, and builds no key: it decodes no digest, hashes
//! nothing and calls no `NodeKey` constructor.

use ix_trace_rs::trace;
use qsl_foundation::Setting;
use qsl_replay::{
    compile_admitted, AdmittedCompositeForm, AdmittedContext, DependencyInput, Identifier,
    ReplayLimits, SourceIdentity, StageLimits, WireNodeId,
};
use quire_exact::{FieldValue, Presence, Value, ValueType};
use quire_semantic_value::declaration::{
    CompositeDeclaration, CompositeShape, FieldDeclaration, TypeEnvironment,
};

const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"quire.value.complete/v1\";\n\
     tuple Pair(Boolean, Boolean);\n\
     record Holder { flag: Boolean; pair: Pair; }\n";

fn context() -> AdmittedContext {
    let stages: StageLimits = [(Setting::S1InputBytes, 1 << 20)].into_iter().collect();
    compile_admitted(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        SOURCE.as_bytes(),
        [],
        &DependencyInput::default(),
        stages,
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
    WireNodeId::from_digest(*found.key().as_bytes())
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
