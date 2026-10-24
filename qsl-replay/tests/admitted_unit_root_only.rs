// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1092 (FR-398-AC-1): a downstream generator compiles authentic QSL
//! source declaring a dimension and two units through `compile_admitted`,
//! looks both units up through the `qsl_replay` root, and builds a kernel
//! quantity type, a quantity value that type admits and an exact conversion
//! `Quantity<m>` to `Quantity<km>` from what the context returns. This file
//! names no `qsl_replay::spine` or `qsl_semantics` item, and builds no key:
//! it decodes no digest, hashes nothing and calls no `NodeKey` or `UnitId`
//! constructor.

use ix_trace_rs::trace;
use qsl_replay::{
    compile_admitted, AdmittedContext, AdmittedUnit, DependencyInput, Identifier, ReplayLimits,
    SourceIdentity, StageLimits, WireNodeId,
};
use quire_exact::Outcome;
use quire_exact::{Integer, Meter, Quantity, Rational, ScalarLimits, Value, ValueType};
use quire_semantic_value::declaration::TypeEnvironment;
use quire_semantic_value::quantity::{convert_quantity, ConvertedValue, QuantityTarget};

const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"quire.value.complete/v1\";\n\
     dimension Length;\n\
     unit m : Length = rational(1, 1);\n\
     unit km : Length = rational(1000, 1) * m;\n\
     function f using v(a: Quantity<m>, b: Quantity<km>): Boolean pure { true }\n";

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

/// The admitted unit named `name`, found by the wire node id a consumer reads
/// from the emitted package or a Contract IR document.
fn unit<'a>(context: &'a AdmittedContext, name: &str) -> &'a AdmittedUnit {
    let name = Identifier::new(name).expect("an identifier");
    let found = context
        .units()
        .find(|unit| unit.qualified_name().last() == Some(&name))
        .expect("the unit is admitted");
    context
        .unit_declaration(WireNodeId::from_digest(*found.node().as_bytes()))
        .expect("the lookup finds it")
}

fn meter() -> Meter {
    Meter::new(ScalarLimits {
        integer_bits: u64::MAX,
        decimal_digits: u64::MAX,
        scale_expansion: u64::MAX,
        text_input_bytes: u64::MAX,
        text_scalars: u64::MAX,
        normalized_scalars: u64::MAX,
        unit_edges: u64::MAX,
        value_occurrences: u64::MAX,
        work_units: u64::MAX,
        result_units: u64::MAX,
    })
}

fn rational(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("nonzero")
}

#[trace("TC-1092", "FR-398-AC-1")]
#[test]
fn a_consumer_builds_a_quantity_conversion_from_admitted_units() {
    let context = context();
    let (m, km) = (unit(&context, "m"), unit(&context, "km"));
    assert_ne!(m.id(), km.id());
    assert!(m.converts_to(km));

    let types = TypeEnvironment::new([], [])
        .expect("an empty environment")
        .with_units(context.unit_table().clone());
    let metres = Quantity::new(rational(5, 1), m.id());
    assert!(types
        .admits(
            &ValueType::Quantity(m.id()),
            &Value::Quantity(metres.clone())
        )
        .expect("admission runs"));
    assert!(!types
        .admits(
            &ValueType::Quantity(km.id()),
            &Value::Quantity(metres.clone())
        )
        .expect("admission runs"));

    // `Quantity<m>` to `Quantity<km>`: the operand resolves against the
    // context's table and the target is the admitted unit's `QuantityUnit`.
    let source = context
        .unit_table()
        .resolve(&metres)
        .expect("the table holds the quantity's unit");
    let outcome = convert_quantity(
        source,
        km.quantity_unit(),
        &QuantityTarget::Exact,
        &mut meter(),
    )
    .expect("the units share a dimension");
    let Outcome::Completed(conversion) = outcome else {
        panic!("the conversion completes");
    };
    assert_eq!(
        conversion.value(),
        &ConvertedValue::Exact(rational(5, 1000))
    );
    assert_eq!(conversion.unit().id().expect("an id"), km.id());
}
