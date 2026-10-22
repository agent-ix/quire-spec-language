// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1173 (FR-404): `run_with_replay_context` returns, with the outcome and
//! members of `run_with_requirements`, a replay context from the same
//! compile, which builds requests only for the call's own items under the
//! run's `package_id`. The calls under test reach the `qsl_replay` root; the
//! oracle (a `PluginReplayContext` built from a separate compile) reads
//! `qsl_replay::spine`.
//!
//! Every member of a run is a `value-validity` application record, which
//! `PluginReplayContext::request` does not replay until QSL-980 (FR-098
//! application claims); so these tests compare the context with the oracle
//! over each member, whatever that answer is, and the in-crate tests in
//! `call_replay_context.rs` build a request through the context.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_replay::spine::{self, default_accounting, ParseRequest};
use qsl_replay::{
    compile_package, run_with_replay_context, run_with_requirements, BackendId, Call, CallArgument,
    CallOutcome, CallerLimits, DependencyInput, DomainKey, DomainKind, FiniteBound, Integer,
    ObjectEnvironment, PluginReplayContext, PluginReplayRefusal, ProofBound, ReplayLimits,
    RequestIndex, RunRefusal, RunWithReplayContext, SourceIdentity, SuppliedLimits, TerminalValue,
    Value, WireNodeId,
};
use quire_exact::Cancel;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n";
const R: &str = "function p using v(x: Int[0, 9]): Boolean pure { x > 3 }\n\
    function q using v(x: Int[0, 9]): Integer pure { x + 1 }\n\
    function r using v(x: Int[0, 9]): Integer pure { 10 mod x }\n";
const S: &str = "function s using v(x: Int[0, 9]): Integer pure { x + 2 }\n";

fn unit(extra: &str) -> String {
    format!("{HEADER}{R}{extra}")
}

fn identity() -> SourceIdentity {
    SourceIdentity::new("a", "u", "git", "1")
}

fn plugin() -> BackendId {
    BackendId::new("acme")
}

fn call(function: &str, x: Value, work_units: u64) -> Call {
    Call {
        function: function.to_owned(),
        arguments: vec![CallArgument {
            parameter: "x".to_owned(),
            value: x,
        }],
        accounting: default_accounting(work_units),
        supplied: SuppliedLimits::default(),
    }
}

fn integer(value: i64) -> Value {
    Value::Integer(Integer::from(value))
}

fn run(
    source: &str,
    call: &Call,
    limits: CallerLimits,
    cancel: &Cancel,
) -> Result<RunWithReplayContext, Box<RunRefusal>> {
    run_with_replay_context(
        identity(),
        "unit.native",
        source.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
        limits,
        call,
        &ObjectEnvironment::default(),
        cancel,
    )
}

fn run_q(source: &str) -> RunWithReplayContext {
    run(
        source,
        &call("q", integer(3), 1_000_000),
        CallerLimits::default(),
        &Cancel::new(),
    )
    .expect("the call runs")
}

/// The oracle: a `PluginReplayContext` from a separate compile.
fn oracle(source: &str, limits: CallerLimits) -> PluginReplayContext {
    let identity = identity();
    let cancel = Cancel::new();
    let parsed = spine::parse(
        &ParseRequest {
            source: &identity,
            path: "unit.native",
            bytes: source.as_bytes(),
        },
        limits.spine.source,
        &cancel,
    )
    .expect("parses")
    .into_value();
    let models = spine::select(&parsed, &BTreeMap::new(), limits.spine.model, &cancel)
        .expect("selects")
        .into_value();
    let checked = spine::check(
        &parsed,
        &models,
        &DependencyInput::default(),
        &spine::LockEvidence::default(),
        limits.spine,
        &cancel,
    )
    .expect("checks")
    .into_value();
    let emitted = spine::package(&checked, spine::PackageLimits::default(), &cancel)
        .expect("packages")
        .into_value();
    PluginReplayContext::new(&checked, &emitted, &models, limits)
}

fn bound() -> ProofBound {
    ProofBound::new(
        DomainKey::Node {
            node: WireNodeId::from_digest([7; 32]),
            path: vec![],
        },
        Some(DomainKind::Integer),
        FiniteBound::integer_range(Integer::from(0_i64), Integer::from(9_i64)).unwrap(),
    )
    .unwrap()
}

fn outcome_text(outcome: &CallOutcome) -> String {
    format!("{outcome:?}")
}

/// FR-404-AC-1: package id, outcome and members equal `run_with_requirements`'
/// and the package id equals `compile_package`'s.
#[trace("TC-1173", "FR-404-AC-1")]
#[test]
fn the_entry_returns_what_run_with_requirements_returns() {
    let source = unit("");
    let call = call("q", integer(3), 1_000_000);
    let limits = CallerLimits::default();
    let result = run(&source, &call, limits, &Cancel::new()).expect("runs");
    let plain = run_with_requirements(
        identity(),
        "unit.native",
        source.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
        limits.spine,
        &call,
        &ObjectEnvironment::default(),
        &Cancel::new(),
    )
    .expect("runs");
    assert!(matches!(
        result.outcome(),
        CallOutcome::Completed(qsl_replay::CallValue::Integer(value)) if *value == Integer::from(4_i64)
    ));
    assert_eq!(result.requirements().members().len(), 1);
    assert_eq!(result.context().package_id(), result.package_id());
    assert_eq!(result.package_id(), plain.package_id());
    assert_eq!(
        outcome_text(result.outcome()),
        outcome_text(plain.outcome())
    );
    let keys = |members: &[qsl_replay::CallRequirement]| {
        members
            .iter()
            .map(|m| (m.occurrence().clone(), m.request_index()))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        keys(result.requirements().members()),
        keys(plain.requirements().members())
    );
    let compiled = compile_package(
        identity(),
        "unit.native",
        source.as_bytes(),
        std::iter::empty::<&[u8]>(),
        &DependencyInput::default(),
        Default::default(),
        ReplayLimits::default(),
    )
    .expect("compiles");
    assert_eq!(compiled.package_id().hex(), result.package_id().hex());
}

/// FR-404-AC-2 and AC-3: for each member, the context's answer is the
/// oracle's, whatever the outcome of the call.
#[trace("TC-1173", "FR-404-AC-2")]
#[trace("TC-1173", "FR-404-AC-3")]
#[test]
fn a_member_request_equals_the_oracles_whatever_the_outcome() {
    let source = unit("");
    let oracle = oracle(&source, CallerLimits::default());
    let runs = [
        run_q(&source),
        run(
            &source,
            &call("r", integer(0), 1_000_000),
            CallerLimits::default(),
            &Cancel::new(),
        )
        .expect("runs"),
        run(
            &source,
            &call("q", integer(3), 0),
            CallerLimits::default(),
            &Cancel::new(),
        )
        .expect("runs"),
    ];
    assert!(matches!(runs[1].outcome(), CallOutcome::Undefined { .. }));
    assert!(matches!(runs[2].outcome(), CallOutcome::Incomplete(_)));
    for result in &runs {
        let members = result.requirements().members();
        assert_eq!(members.len(), 1, "{:?}", result.outcome());
        let member = &members[0];
        let ours = result.context().request(
            result.package_id(),
            member.request_index(),
            &[bound()],
            &plugin(),
            Vec::new(),
        );
        let theirs = oracle.request(member.occurrence(), &[bound()], &plugin(), Vec::new());
        match (ours, theirs) {
            (Ok(ours), Ok(theirs)) => {
                assert_eq!(ours.package_id, theirs.package_id);
                assert_eq!(ours.obligation_identity, theirs.obligation_identity);
                assert_eq!(ours.replay_target, theirs.replay_target);
            }
            (Err(ours), Err(theirs)) => assert_eq!(ours, theirs),
            (ours, theirs) => panic!("disagree: {:?} vs {:?}", ours.err(), theirs.err()),
        }
    }
}

/// FR-404-AC-4: an index that is no member's refuses `NotACallItem` and
/// settles the item `failed`, naming the plugin.
#[trace("TC-1173", "FR-404-AC-4")]
#[test]
fn an_index_that_is_no_members_is_not_a_call_item() {
    let source = unit("");
    let result = run_q(&source);
    let other = run(
        &source,
        &call("r", integer(1), 1_000_000),
        CallerLimits::default(),
        &Cancel::new(),
    )
    .expect("runs");
    let r_index = other.requirements().members()[0].request_index();
    for index in [r_index, RequestIndex::new(99)] {
        let refusal = result
            .context()
            .request(result.package_id(), index, &[], &plugin(), Vec::new())
            .err()
            .expect("not a call item");
        assert_eq!(
            refusal,
            PluginReplayRefusal::NotACallItem {
                request_index: index
            }
        );
        let diagnostic = refusal.diagnostic(0);
        assert_eq!(diagnostic.code(), "missing_declaration");
        assert_eq!(diagnostic.cause(), Some("missing-name"));
        let record = refusal.record(index, plugin());
        assert_eq!(record.value(), &TerminalValue::Failed);
        assert_eq!(record.backend(), Some(&plugin()));
    }
}

/// FR-404-AC-5: two packages holding `q` have different ids and equal member
/// keys; one run's context refuses the other's pairing key.
#[trace("TC-1173", "FR-404-AC-5")]
#[test]
fn another_runs_package_is_a_foreign_run() {
    let first = run_q(&unit(""));
    let second = run_q(&unit(S));
    assert_ne!(first.package_id(), second.package_id());
    assert_eq!(
        first.requirements().members()[0].occurrence(),
        second.requirements().members()[0].occurrence()
    );
    let index = second.requirements().members()[0].request_index();
    let refusal = first
        .context()
        .request(second.package_id(), index, &[], &plugin(), Vec::new())
        .err()
        .expect("a foreign run");
    assert_eq!(
        refusal,
        PluginReplayRefusal::ForeignRun {
            received: second.package_id(),
            run: first.package_id(),
        }
    );
    let diagnostic = refusal.diagnostic(0);
    assert_eq!(diagnostic.code(), "stale_dependency");
    assert_eq!(diagnostic.cause(), Some("content-mismatch"));
    let record = refusal.record(index, plugin());
    assert_eq!(record.value(), &TerminalValue::Failed);
    assert_eq!(record.backend(), Some(&plugin()));
}

/// FR-404-AC-6: `CallerLimits.spine` bounds the run.
#[trace("TC-1173", "FR-404-AC-6")]
#[test]
fn only_the_spine_limits_bound_the_run() {
    let source = unit("");
    let call = call("q", integer(3), 1_000_000);
    let tight = CallerLimits::from_operands(["s1.tokens=1"]).unwrap();
    let refusal = run(&source, &call, tight, &Cancel::new()).expect_err("refuses");
    assert!(matches!(*refusal, RunRefusal::Compile(_)), "{refusal:?}");
    let roomy =
        CallerLimits::from_operands(["s1.tokens=200000", "library.definitions=9000"]).unwrap();
    run(&source, &call, roomy, &Cancel::new()).expect("completes");
}

/// FR-404-AC-7: the refusal order is `ForeignRun`, `NotACallItem`, then
/// FR-290's, whatever the proof bounds.
#[trace("TC-1173", "FR-404-AC-7")]
#[test]
fn foreign_run_comes_before_not_a_call_item() {
    let first = run_q(&unit(""));
    let second = run_q(&unit(S));
    let r = run(
        &unit(""),
        &call("r", integer(1), 1_000_000),
        CallerLimits::default(),
        &Cancel::new(),
    )
    .expect("runs");
    let twice = [bound(), bound()];
    let q_index = first.requirements().members()[0].request_index();
    let r_index = r.requirements().members()[0].request_index();
    let refusal = |package, index| {
        first
            .context()
            .request(package, index, &twice, &plugin(), Vec::new())
            .err()
            .expect("refuses")
    };
    assert!(matches!(
        refusal(second.package_id(), q_index),
        PluginReplayRefusal::ForeignRun { .. }
    ));
    assert!(matches!(
        refusal(first.package_id(), r_index),
        PluginReplayRefusal::NotACallItem { .. }
    ));
}

/// FR-404-AC-8: the entry refuses as `run_with_requirements` does and
/// returns no context.
#[trace("TC-1173", "FR-404-AC-8")]
#[test]
fn refusals_are_run_with_requirements_refusals() {
    let limits = CallerLimits::default();
    let cancelled = Cancel::new();
    cancelled.cancel(quire_exact::CancelCause::Requested);
    let cases: [(&str, Call, &Cancel); 4] = [
        (
            &unit(""),
            call("nope", integer(3), 1_000_000),
            &Cancel::new(),
        ),
        (
            &unit(""),
            call("q", Value::Boolean(true), 1_000_000),
            &Cancel::new(),
        ),
        (
            "not a program",
            call("q", integer(3), 1_000_000),
            &Cancel::new(),
        ),
        (&unit(""), call("q", integer(3), 1_000_000), &cancelled),
    ];
    for (source, call, cancel) in &cases {
        let ours = run(source, call, limits, cancel).expect_err("refuses");
        let theirs = run_with_requirements(
            identity(),
            "unit.native",
            source.as_bytes(),
            &BTreeMap::new(),
            &DependencyInput::default(),
            limits.spine,
            call,
            &ObjectEnvironment::default(),
            cancel,
        )
        .expect_err("refuses");
        assert_eq!(format!("{ours:?}"), format!("{theirs:?}"));
    }
}
