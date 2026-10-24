// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1089 (FR-395-AC-1, -3 to -8, -11): `run_with_requirements` returns,
//! with `run`'s package id and outcome, exactly the requirement records the
//! selected function owns, in occurrence-key order, and refuses as `run`
//! does. The calls under test reach the `qsl_replay` root; the oracle (the
//! checked graph's records, `run`) reads `qsl_replay::spine`.
//! FR-395-AC-9 (the crate root alone) is `call_requirements_root_only.rs`;
//! FR-395-AC-2 and FR-075-AC-17 (the items) are in the root crate's
//! `tests/it/call_requirements_items.rs`.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_replay::spine::{self, default_accounting, ParseRequest};
use qsl_replay::{
    compile_package, run_with_requirements, Call, CallArgument, CallOutcome, DependencyInput,
    Integer, ObjectEnvironment, OccurrenceKey, ReplayLimits, RunRefusal, RunWithRequirements,
    SourceIdentity, SpineLimits, StageLimits, SuppliedLimits, Value,
};
use quire_exact::Cancel;

/// Fixture K (FR-395): four records, in package order: `sq`'s `integer.mul`
/// (0), the `x + 1` node's ordinal 0, which is `inc`'s add (1), and its
/// ordinals 1 and 2, which are `sq`'s two adds (2, 3).
const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n";
const FUNCTIONS: &str = "function inc using v(x: Int[0, 9]): Int[0, 10] pure { x + 1 }\n\
    function sq using v(x: Int[0, 9]): Integer pure { (x + 1) * (x + 1) }\n\
    function id using v(x: Int[0, 9]): Int[0, 9] pure { x }\n\
    function two using v(): Int[0, 10] pure { inc(1) }\n";

const ADD: &str = "function add using v(x: Int[0, 9], y: Int[0, 9]): Int[0, 18] pure { x + y }\n";

fn k() -> String {
    format!("{HEADER}{FUNCTIONS}")
}

fn identity() -> SourceIdentity {
    SourceIdentity::new("a", "u", "git", "1")
}

fn integer(value: i64) -> Value {
    Value::Integer(Integer::from(value))
}

fn argument(parameter: &str, value: Value) -> CallArgument {
    CallArgument {
        parameter: parameter.to_owned(),
        value,
    }
}

fn call(function: &str, arguments: Vec<CallArgument>, work_units: u64) -> Call {
    Call {
        function: function.to_owned(),
        arguments,
        accounting: default_accounting(work_units),
        supplied: SuppliedLimits::default(),
    }
}

fn with_requirements(
    source: &str,
    call: &Call,
    cancel: &Cancel,
) -> Result<RunWithRequirements, Box<RunRefusal>> {
    run_with_requirements(
        identity(),
        "unit.native",
        source.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
        SpineLimits::default(),
        call,
        &ObjectEnvironment::default(),
        cancel,
    )
}

fn members(source: &str, call: &Call) -> Vec<(OccurrenceKey, usize)> {
    let result = with_requirements(source, call, &Cancel::new()).expect("the call runs");
    result
        .requirements()
        .members()
        .iter()
        .map(|member| (member.occurrence().clone(), member.request_index().get()))
        .collect()
}

/// The checked graph's record keys in package order, read through the spine.
fn record_keys(source: &str) -> Vec<OccurrenceKey> {
    let identity = identity();
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let parsed = spine::parse(
        &ParseRequest {
            source: &identity,
            path: "unit.native",
            bytes: source.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .expect("parses")
    .into_value();
    let models = spine::select(&parsed, &BTreeMap::new(), limits.model, &cancel)
        .expect("selects")
        .into_value();
    let checked = spine::check(
        &parsed,
        &models,
        &DependencyInput::default(),
        &spine::LockEvidence::default(),
        limits,
        &cancel,
    )
    .expect("checks")
    .into_value();
    checked
        .package()
        .graph()
        .requirements()
        .keys()
        .cloned()
        .collect()
}

fn refusal(source: &str, call: &Call, cancel: &Cancel) -> RunRefusal {
    *with_requirements(source, call, cancel).expect_err("the call refuses")
}

/// FR-395-AC-1: `inc` has one member (index 1); `sq` has three, in ascending
/// key order (indices 0, 2, 3); the sets are disjoint and every key is one of
/// K's records.
#[trace("TC-1089", "FR-395-AC-1")]
#[test]
fn the_selected_function_owns_its_records_in_key_order() {
    let source = k();
    let keys = record_keys(&source);
    assert_eq!(keys.len(), 4);
    let inc = members(
        &source,
        &call("inc", vec![argument("x", integer(3))], 1_000_000),
    );
    let sq = members(
        &source,
        &call("sq", vec![argument("x", integer(3))], 1_000_000),
    );
    assert_eq!(inc.iter().map(|m| m.1).collect::<Vec<_>>(), [1]);
    assert_eq!(sq.iter().map(|m| m.1).collect::<Vec<_>>(), [0, 2, 3]);
    for (key, index) in inc.iter().chain(&sq) {
        assert_eq!(keys.get(*index), Some(key));
    }
    let sq_keys: Vec<_> = sq.iter().map(|m| &m.0).collect();
    assert!(sq_keys.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(!sq_keys.contains(&&inc[0].0));
}

/// FR-395-AC-3: a function with no scalar application, and one that only
/// calls `inc`, return an empty list and complete.
#[trace("TC-1089", "FR-395-AC-3")]
#[test]
fn a_function_with_no_application_has_no_member_and_a_caller_does_not_own_its_callee() {
    let source = k();
    for call in [
        call("id", vec![argument("x", integer(3))], 1_000_000),
        call("two", vec![], 1_000_000),
    ] {
        let result = with_requirements(&source, &call, &Cancel::new()).expect("the call runs");
        assert!(result.requirements().members().is_empty());
        assert!(matches!(result.outcome(), CallOutcome::Completed(_)));
    }
}

/// FR-395-AC-4: arguments do not select items, and a wrong-kind argument
/// refuses with no members.
#[trace("TC-1089", "FR-395-AC-4")]
#[test]
fn arguments_do_not_select_items() {
    let source = k();
    let at = |x| {
        members(
            &source,
            &call("sq", vec![argument("x", integer(x))], 1_000_000),
        )
    };
    assert_eq!(at(0), at(9));
    let wrong = call("sq", vec![argument("x", Value::Boolean(true))], 1_000_000);
    assert!(matches!(
        refusal(&source, &wrong, &Cancel::new()),
        RunRefusal::WrongValueKind { position: 0 }
    ));
}

/// FR-395-AC-5: an incomplete outcome carries the same members.
#[trace("TC-1089", "FR-395-AC-5")]
#[test]
fn an_incomplete_call_returns_the_same_members() {
    let source = k();
    let full = call("sq", vec![argument("x", integer(3))], 1_000_000);
    let starved = call("sq", vec![argument("x", integer(3))], 0);
    let result = with_requirements(&source, &starved, &Cancel::new()).expect("the call runs");
    let CallOutcome::Incomplete(incomplete) = result.outcome() else {
        panic!("incomplete, not {:?}", result.outcome());
    };
    assert_eq!(incomplete.limits_field(), "work_units");
    assert_eq!(
        result
            .requirements()
            .members()
            .iter()
            .map(|m| (m.occurrence().clone(), m.request_index().get()))
            .collect::<Vec<_>>(),
        members(&source, &full)
    );
}

/// FR-395-AC-6: package id and outcome equal `run`'s and `compile_package`'s.
#[trace("TC-1089", "FR-395-AC-6")]
#[test]
fn package_id_and_outcome_equal_run_and_compile_package() {
    let source = k();
    let compiled = compile_package(
        identity(),
        "unit.native",
        source.as_bytes(),
        [],
        &DependencyInput::default(),
        StageLimits::default(),
        ReplayLimits::default(),
    )
    .expect("compiles");
    for call in [
        call("inc", vec![argument("x", integer(3))], 1_000_000),
        call("sq", vec![argument("x", integer(3))], 1_000_000),
        call("id", vec![argument("x", integer(3))], 1_000_000),
        call("two", vec![], 1_000_000),
        call("sq", vec![argument("x", integer(3))], 0),
    ] {
        let direct = spine::run(
            identity(),
            "unit.native",
            source.as_bytes(),
            &BTreeMap::new(),
            &DependencyInput::default(),
            SpineLimits::default(),
            &call,
            &ObjectEnvironment::default(),
            &Cancel::new(),
        )
        .expect("run runs");
        let result = with_requirements(&source, &call, &Cancel::new()).expect("the call runs");
        assert_eq!(result.package_id(), direct.0);
        assert_eq!(result.requirements().package_id(), direct.0);
        assert_eq!(result.package_id().record(), compiled.package_id());
        assert_eq!(format!("{:?}", result.outcome()), format!("{:?}", direct.1));
    }
}

/// FR-395-AC-7: a name that selects no function, and a source that does not
/// parse, refuse as `run` does, with no members.
#[trace("TC-1089", "FR-395-AC-7")]
#[test]
fn unselectable_names_and_unparsable_sources_refuse() {
    let source = k();
    for name in ["nope", "", "inc.x", "1inc"] {
        let refused = refusal(&source, &call(name, vec![], 1_000_000), &Cancel::new());
        assert!(
            matches!(&refused, RunRefusal::MissingDeclaration { .. }),
            "{name}: {refused:?}"
        );
        assert_eq!(refused.code().as_str(), "missing_declaration");
    }
    let broken = refusal(
        "this is not a program",
        &call("inc", vec![], 1_000_000),
        &Cancel::new(),
    );
    assert!(matches!(broken, RunRefusal::Compile(_)), "{broken:?}");
}

/// FR-395-AC-8: a cancelled handle refuses `Cancelled`.
#[trace("TC-1089", "FR-395-AC-8")]
#[test]
fn a_cancelled_handle_refuses() {
    let cancel = Cancel::new();
    cancel.cancel(quire_exact::CancelCause::Requested);
    let refused = refusal(
        &k(),
        &call("sq", vec![argument("x", integer(3))], 1_000_000),
        &cancel,
    );
    assert!(matches!(refused, RunRefusal::Cancelled(_)), "{refused:?}");
}

/// FR-395-AC-11: `add` called with its arguments in either order is the same
/// call and returns equal members.
#[trace("TC-1089", "FR-395-AC-11")]
#[test]
fn argument_order_does_not_change_the_call() {
    let source = format!("{HEADER}{ADD}");
    let forward = call(
        "add",
        vec![argument("x", integer(2)), argument("y", integer(5))],
        1_000_000,
    );
    let reversed = call(
        "add",
        vec![argument("y", integer(5)), argument("x", integer(2))],
        1_000_000,
    );
    let first = with_requirements(&source, &forward, &Cancel::new()).expect("runs");
    let second = with_requirements(&source, &reversed, &Cancel::new()).expect("runs");
    assert!(first.requirements().same_call(second.requirements()));
    assert_eq!(members(&source, &forward), members(&source, &reversed));
    assert_eq!(first.requirements().members().len(), 1);
    let other = call(
        "add",
        vec![argument("x", integer(5)), argument("y", integer(2))],
        1_000_000,
    );
    let third = with_requirements(&source, &other, &Cancel::new()).expect("runs");
    assert!(!first.requirements().same_call(third.requirements()));
}
