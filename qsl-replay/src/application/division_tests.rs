// SPDX-License-Identifier: AGPL-3.0-or-later
//! Public owner-admitted compilation and source-backed division execution.

use super::*;
use crate::{
    admit_integer_division, compile_package, compile_package_with_integer_division,
    AdmittedIntegerDivision, DefinitionRole, SourceIdentity,
};
use ix_trace_rs::trace;
use quire_contract_model::{
    CheckedPackageEvidence, CheckedPackageReadLimits, CheckedPackageV2, CheckedPackageV2ReadResult,
    CheckedTerm,
};
use quire_eval::FamilyOutcome;
use quire_exact::{Outcome, ScalarLimits};

const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\n";

fn unit(source: &str) -> CompileUnit<'_> {
    CompileUnit {
        source: SourceIdentity::new("test", "division-selection"),
        package_name: "division-selection".to_owned(),
        path: "division.native",
        bytes: source.as_bytes(),
    }
}
fn selection(role: DefinitionRole) -> AdmittedIntegerDivision {
    admit_integer_division(&[role.reference()], None).expect("owner admits its own closed law")
}
fn compile(source: &str, law: &AdmittedIntegerDivision) -> CompiledPackage {
    compile_package_with_integer_division(
        unit(source),
        [],
        &DependencyInput::default(),
        law,
        StageLimits::default(),
        ReplayLimits::default(),
    )
    .expect("public selected source compilation")
}
fn meter() -> Meter {
    Meter::new(ScalarLimits {
        integer_bits: 10000,
        decimal_digits: 10000,
        scale_expansion: 10000,
        text_input_bytes: 10000,
        text_scalars: 10000,
        normalized_scalars: 10000,
        unit_edges: 10000,
        value_occurrences: 10000,
        work_units: 100000,
        result_units: 10000,
    })
}

#[trace("TC-1284", "FR-495-AC-8")]
#[test]
fn division_owner_selection_reaches_public_compile_source_execution_and_strict_read() {
    let source = format!("{HEADER}function q using v(a: Int[-9,9], b: Int[-4,4]): Integer pure {{ if b != 0 then a div b else 0 }}\nfunction r using v(a: Int[-9,9], b: Int[-4,4]): Integer pure {{ if b != 0 then a rem b else 0 }}\n");
    for (role, cases) in [
        (
            DefinitionRole::IntegerDivisionEuclidean,
            [(-7, 3, -3, 2), (7, -3, -2, 1), (-7, -3, 3, 2)],
        ),
        (
            DefinitionRole::IntegerDivisionFloor,
            [(-7, 3, -3, 2), (7, -3, -3, -2), (-7, -3, 2, -1)],
        ),
        (
            DefinitionRole::IntegerDivisionTruncating,
            [(-7, 3, -2, -1), (7, -3, -2, 1), (-7, -3, 2, -1)],
        ),
    ] {
        let law = selection(role);
        let compiled = compile(&source, &law);
        let acquired = acquire_checked_closure_with_integer_division(
            unit(&source),
            [],
            &DependencyInput::default(),
            &law,
            StageLimits::default(),
            ReplayLimits::default(),
        )
        .expect("public source-backed closure compilation");
        assert_eq!(acquired.package().bytes(), compiled.bytes());
        let mut evidence = CheckedPackageEvidence::new();
        evidence.support_feature("quire.value.complete/v1");
        let read = match CheckedPackageV2::read(
            compiled.bytes(),
            CheckedPackageReadLimits::bounded(),
            &evidence,
        ) {
            CheckedPackageV2ReadResult::Admitted(read) => read,
            other => panic!("actual emitted package admits independently: {other:?}"),
        };
        let mut found = 0;
        for node in &read.graph().nodes {
            let Some(CheckedTerm::Application { operation, .. }) = read.term(&node.node_id) else {
                continue;
            };
            if matches!(
                operation.identity(),
                Some("quire.op.integer.div" | "quire.op.integer.rem")
            ) {
                let laws = operation
                    .law_definitions()
                    .expect("reader admitted law views");
                assert_eq!(laws.len(), 1);
                assert_eq!(laws[0].authority.as_ref(), role.reference().authority);
                assert_eq!(laws[0].identity.as_ref(), role.reference().identity);
                found += 1;
            }
        }
        assert_eq!(
            found, 2,
            "both actual checked division applications retain the law"
        );
        for (a, b, q, r) in cases {
            for (name, expected) in [("q", q), ("r", r)] {
                let key = acquired
                    .handle
                    .package
                    .graph()
                    .function_identity(name)
                    .expect("actual checked function");
                let callable = acquired
                    .handle()
                    .lookup_callable(acquired.handle().package_name(), key)
                    .expect("owner callable");
                let environment = ApplicationEnvironment::new(ObjectClosure::default());
                let mut meter = meter();
                let result = acquired
                    .handle()
                    .registry()
                    .begin_call(
                        &callable,
                        vec![
                            Value::Integer(Integer::from(a)),
                            Value::Integer(Integer::from(b)),
                        ],
                        &environment,
                        &mut meter,
                    )
                    .expect("public source-backed call")
                    .run()
                    .expect("source execution");
                assert!(
                    matches!(result.outcome, FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(ref actual))) if actual == &Integer::from(expected)),
                    "{role:?} {a} {name} {b}: {result:?}"
                );
            }
        }
    }
}

#[trace("TC-1284", "FR-495-AC-8")]
#[test]
fn missing_division_selection_stays_typed_and_modulo_needs_no_selection() {
    let source = format!(
        "{HEADER}function q using v(a: Int[-9,9], b: Int[1,4]): Integer pure {{ a div b }}\n"
    );
    let refusal = compile_package(
        unit(&source),
        [],
        &DependencyInput::default(),
        StageLimits::default(),
        ReplayLimits::default(),
    )
    .expect_err("a demanded selection is absent");
    assert_eq!(refusal.code(), qsl_foundation::Code::MissingDeclaration);
    assert_eq!(refusal.cause(), Some("missing-selection"));
    for expression in ["a mod b", "a + b"] {
        let source = format!("{HEADER}function q using v(a: Int[-9,9], b: Int[1,4]): Integer pure {{ {expression} }}\n");
        compile_package(
            unit(&source),
            [],
            &DependencyInput::default(),
            StageLimits::default(),
            ReplayLimits::default(),
        )
        .expect("no division-law demand");
    }
}

#[trace("TC-1284", "FR-495-AC-8")]
#[test]
fn facade_admission_retains_conflicting_unknown_and_incompatible_definition_refusals() {
    use quire_semantic_value::definition::PackageCause;
    let euclidean = DefinitionRole::IntegerDivisionEuclidean.reference();
    let floor = DefinitionRole::IntegerDivisionFloor.reference();
    let conflicting =
        admit_integer_division(&[euclidean.clone(), floor.clone()], None).unwrap_err();
    assert_eq!(conflicting.cause(), PackageCause::ConflictingDefinition);
    let mut foreign = euclidean.clone();
    foreign.authority = "foreign".to_owned();
    assert_eq!(
        admit_integer_division(&[foreign], None)
            .unwrap_err()
            .cause(),
        PackageCause::IncompatibleDefinition
    );
    assert_eq!(
        admit_integer_division(&[euclidean], Some(&floor))
            .unwrap_err()
            .cause(),
        PackageCause::IncompatibleDefinition
    );
}
