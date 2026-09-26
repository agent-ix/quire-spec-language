// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-062-AC-6, second sentence (QSL-246): a family's `evaluate` reads no
//! display string. Renaming every declared name in a package (functions,
//! parameters, type aliases) changes no evaluation result, because a name
//! that is not part of the semantics is never consulted by the evaluator.

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_package::CheckedPackage;
use qsl_semantics::check::{CheckingLimits, PackageDeclarations};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_exact::{Meter, ScalarLimits, Value};

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\" version \"1\" digest \
    \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n";

/// The unit with its declared names supplied: alias, inner function,
/// parameter and outer function.
fn unit(alias: &str, inner: &str, parameter: &str, outer: &str) -> String {
    format!(
        "{HEADER}type {alias} = Int[0, 9];\n\
         function {inner} using v({parameter}: {alias}): Int[0, 10] pure {{ {parameter} + 1 }}\n\
         function {outer} using v({parameter}: {alias}): Int[0, 11] pure \
         {{ {inner}({parameter}) + 1 }}\n"
    )
}

fn limits(work_units: u64) -> ScalarLimits {
    ScalarLimits {
        integer_bits: u64::MAX,
        decimal_digits: u64::MAX,
        scale_expansion: u64::MAX,
        text_input_bytes: u64::MAX,
        text_scalars: u64::MAX,
        normalized_scalars: u64::MAX,
        unit_edges: u64::MAX,
        value_occurrences: u64::MAX,
        work_units,
        result_units: u64::MAX,
    }
}

/// Calls `entry` with `argument` against a meter of `work_units`, returning
/// what evaluation produced that does not embed source offsets: the outcome,
/// the loss count and how many charges the meter admitted.
fn run(source: &str, entry: &str, argument: i64, work_units: u64) -> (String, usize, u64) {
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        source.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed, qsl_forms::FormsLimits::default())
        .expect("S2 builds the unit");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the assembler builds the package declarations");
    let package = CheckedPackage::link(
        declarations
            .check(CheckingLimits::default())
            .expect("the package checks"),
    );
    let mut meter = Meter::new(limits(work_units));
    let evaluation = package
        .call(
            &QualifiedName::unqualified(entry).expect("an identifier"),
            vec![Value::Integer(argument.into())],
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .expect("the call runs");
    (
        format!("{:?}", evaluation.outcome),
        evaluation.losses.len(),
        meter.admission_count(),
    )
}

/// The same package under two complete sets of declared names evaluates to
/// the same outcome, loss count and metered work, for a completed run and
/// for a run the meter stops. If `evaluate` consulted a name (a function's
/// display name, a slot name, an alias name) to decide anything, one of the
/// renamed runs would diverge from its original.
#[trace("TC-160", "FR-062-AC-6")]
#[test]
fn renaming_declared_names_leaves_evaluation_unchanged() {
    let original = unit("Digit", "inc", "x", "twice");
    let renamed = unit("Nibble", "bump", "operand", "double_step");
    assert_ne!(original, renamed);

    for (argument, work_units) in [(3_i64, u64::MAX), (3, 1)] {
        let before = run(&original, "twice", argument, work_units);
        let after = run(&renamed, "double_step", argument, work_units);
        assert_eq!(
            before, after,
            "argument {argument}, work bound {work_units}: renaming changed evaluation"
        );
    }
    // The fixture is not vacuous: the unbounded run completes with a value
    // and the bounded run does not.
    let completed = run(&original, "twice", 3, u64::MAX).0;
    assert!(completed.contains("Completed"), "{completed}");
    let stopped = run(&original, "twice", 3, 1).0;
    assert!(!stopped.contains("Completed"), "{stopped}");
}
