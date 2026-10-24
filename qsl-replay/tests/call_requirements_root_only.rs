// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1089 (FR-395-AC-9): a consumer reaches `run_with_requirements`, its
//! result types and `run`'s call types through the `qsl_replay` root alone.
//! This file names no `qsl_replay::spine`, `qsl_semantics` or `quire_exact`
//! item, so it stops compiling if the facade needs one.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_replay::{
    run_with_requirements, Call, CallArgument, CallIncomplete, CallLocus, CallOutcome, CallRefusal,
    CallValue, Cancel, CancelCause, CompileRefusal, DependencyInput, Integer, ObjectEnvironment,
    PackageId, RunRefusal, ScalarLimits, SourceIdentity, SpineLimits, SuppliedLimits, Value,
};

const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"quire.value.complete/v1\";\n\
     function inc using v(x: Int[0, 9]): Int[0, 10] pure { x + 1 }\n";

fn accounting() -> ScalarLimits {
    ScalarLimits {
        integer_bits: u64::MAX,
        decimal_digits: u64::MAX,
        scale_expansion: u64::MAX,
        text_input_bytes: u64::MAX,
        text_scalars: u64::MAX,
        normalized_scalars: u64::MAX,
        unit_edges: u64::MAX,
        value_occurrences: u64::MAX,
        work_units: 1_000_000,
        result_units: u64::MAX,
    }
}

fn run(source: &str, cancel: &Cancel) -> Result<qsl_replay::RunWithRequirements, Box<RunRefusal>> {
    let call = Call {
        function: "inc".to_owned(),
        arguments: vec![CallArgument {
            parameter: "x".to_owned(),
            value: Value::Integer(Integer::from(3_i64)),
        }],
        accounting: accounting(),
        supplied: SuppliedLimits::default(),
    };
    run_with_requirements(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        source.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
        SpineLimits::default(),
        &call,
        &ObjectEnvironment::default(),
        cancel,
    )
}

/// The whole consumer path: call, read the id, the outcome's payloads and the
/// members; read each refusal type.
#[trace("TC-1089", "FR-395-AC-9")]
#[test]
fn a_consumer_calls_and_reads_through_the_crate_root() {
    let result = run(SOURCE, &Cancel::new()).expect("the call runs");
    let package_id: PackageId = result.package_id();
    let (id, outcome, requirements) = result.into_parts();
    assert_eq!(id, package_id);
    assert_eq!(requirements.package_id(), package_id);
    let CallOutcome::Completed(CallValue::Integer(value)) = outcome else {
        panic!("a completed integer, not {outcome:?}");
    };
    assert_eq!(value, Integer::from(4_i64));
    let _: (Option<CallIncomplete>, Option<CallRefusal>) = (None, None);
    assert_eq!(requirements.function(), "inc");
    assert_eq!(requirements.arguments().len(), 1);
    assert_eq!(requirements.members().len(), 1);
    assert_eq!(requirements.members()[0].request_index().get(), 0);
    let _: Option<CallLocus> = None;

    let unparsable = run("not a program", &Cancel::new()).expect_err("refuses");
    let RunRefusal::Compile(compile) = *unparsable else {
        panic!("a compile refusal");
    };
    let _: &CompileRefusal = &compile;

    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    let cancelled = run(SOURCE, &cancel).expect_err("refuses");
    assert!(matches!(
        *cancelled,
        RunRefusal::Cancelled(CancelCause::Requested)
    ));
}
