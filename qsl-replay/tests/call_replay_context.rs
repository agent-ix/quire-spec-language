// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1173 (FR-404): `run_with_replay_context` returns, with the outcome and
//! members of `run_with_requirements`, a replay context from the same
//! compile, which builds requests only for the call's own items under the
//! run's `package_id`. The calls under test reach the `qsl_replay` root; the
//! oracle (a `PluginReplayContext` built from a separate compile) reads
//! `qsl_replay::spine`.
//!
//! Every member of a run is a `value-validity` application record, which
//! `PluginReplayContext::request` builds a request for and `replay` decides
//! as the application claim at the member's occurrence (FR-098-AC-22).

use std::collections::BTreeMap;

fn fixture_node(fill: u8) -> NodeKey {
    NodeKey::new(
        "fixture",
        vec![quire_exact::PathSegment::Text(fill.to_string())],
        0,
    )
    .expect("fixture node")
}

use ix_trace_rs::trace;
use qsl_replay::spine::{self, default_accounting, ParseRequest};
use qsl_replay::{
    compile_package, replay, run_with_replay_context, run_with_requirements, BackendId, Call,
    CallArgument, CallOutcome, CallerLimits, CanonicalAssignment, DependencyInput, DomainKey,
    DomainKind, FiniteBound, InconclusiveCause, Integer, NodeKey, ObjectEnvironment,
    PluginReplayContext, PluginReplayRefusal, ProofBound, ReplayLimits, ReplayRequestWire,
    RequestIndex, RunRefusal, RunWithReplayContext, SourceIdentity, SpineLimits, TerminalRecord,
    TerminalValue, Value, WitnessValue,
};
use quire_exact::Cancel;
use quire_semantic_value::supplied_stop::SuppliedLimits;

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
            node: fixture_node(7),
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

/// The node of `function`'s first parameter in `source`, read through a
/// separate compile.
fn parameter(source: &str, function: &str) -> NodeKey {
    let identity = identity();
    let limits = CallerLimits::default().spine;
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
        limits,
        &cancel,
    )
    .expect("checks")
    .into_value();
    let graph = checked.package().graph();
    let identity = graph.callable(function).expect("declared").identity;
    let parameter = graph
        .semantic_graph()
        .node(identity)
        .and_then(|node| node.function_parameters())
        .expect("a function node")[0];
    parameter.clone()
}

fn assign(parameter: NodeKey, x: i128) -> Vec<CanonicalAssignment> {
    vec![CanonicalAssignment {
        parameter,
        value: WitnessValue::Integer(x),
    }]
}

/// The context's request for the run's one member and the oracle's for the
/// member's occurrence key, equal member by member.
fn member_request(
    source: &str,
    result: &RunWithReplayContext,
    function: &str,
    x: i128,
) -> ReplayRequestWire {
    let oracle = oracle(source, CallerLimits::default());
    let member = &result.requirements().members()[0];
    let parameter = parameter(source, function);
    let bounds = [bound_at(parameter)];
    let ours = result
        .context()
        .request(
            result.package_id(),
            member.request_index(),
            &bounds,
            &plugin(),
            assign(parameter, x),
        )
        .expect("a member is an application item");
    let theirs = oracle
        .request(
            member.occurrence(),
            &bounds,
            &plugin(),
            assign(parameter, x),
        )
        .expect("the oracle builds it too");
    assert_eq!(ours.package_id, theirs.package_id);
    assert_eq!(ours.obligation_identity, theirs.obligation_identity);
    assert_eq!(ours.replay_target, theirs.replay_target);
    assert_eq!(ours.dependencies, theirs.dependencies);
    assert_eq!(ours.backend, theirs.backend);
    assert_eq!(ours.stage_limits, theirs.stage_limits);
    assert_eq!(ours.accounting_limits, theirs.accounting_limits);
    assert_eq!(ours.byte_provision, theirs.byte_provision);
    assert_eq!(ours.declared_domains.len(), theirs.declared_domains.len());
    for (a, b) in ours.declared_domains.iter().zip(&theirs.declared_domains) {
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
    }
    assert_eq!(ours.source, theirs.source);
    assert_eq!(ours.profile_selections, theirs.profile_selections);
    assert_eq!(ours.state_environment, theirs.state_environment);
    assert_eq!(ours.supplied_limits, theirs.supplied_limits);
    assert_eq!(ours.object_environment, theirs.object_environment);
    assert_eq!(ours.package_id.1, result.package_id().hex());
    assert_eq!(
        ours.replay_target
            .members
            .function
            .as_ref()
            .map(ToString::to_string),
        Some(function.to_owned())
    );
    assert_eq!(
        ours.replay_target.occurrence_key.as_ref(),
        Some(member.occurrence())
    );
    ours
}

fn settle(wire: ReplayRequestWire) -> TerminalRecord {
    let replayed = replay(wire, ReplayLimits::default());
    TerminalRecord::from_plugin_counterexample(RequestIndex::new(0), plugin(), &replayed)
}

/// FR-404-AC-2: `q`'s member builds the oracle's request, and its replay
/// decides the application claim at the member: `x + 1` completes at
/// `x = 9`, so the item settles `inconclusive` with `replay-parity`, naming
/// `acme`. (The completed `10` is the call's value, not the claim's: the
/// replay result carries the claim's Boolean, FR-098.)
#[trace("TC-1173", "FR-404-AC-2")]
#[test]
fn a_completed_members_request_settles_replay_parity() {
    let source = unit("");
    let result = run_q(&source);
    let record = settle(member_request(&source, &result, "q", 9));
    assert!(matches!(
        record.value(),
        TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(_))
    ));
    assert_eq!(record.backend(), Some(&plugin()));
}

/// FR-404-AC-3: `r` at `x = 0` returns `undefined`, and its member's request
/// replays to the application undefined, which refutes the item; `q` under
/// no work budget returns `incomplete` and builds AC-2's request.
#[trace("TC-1173", "FR-404-AC-3")]
#[test]
fn an_undefined_calls_member_refutes_and_an_incomplete_calls_request_is_unchanged() {
    let source = unit("");
    let undefined = run(
        &source,
        &call("r", integer(0), 1_000_000),
        CallerLimits::default(),
        &Cancel::new(),
    )
    .expect("runs");
    assert!(matches!(undefined.outcome(), CallOutcome::Undefined { .. }));
    let record = settle(member_request(&source, &undefined, "r", 0));
    assert_eq!(record.value(), &TerminalValue::Refuted);
    assert_eq!(record.backend(), Some(&plugin()));
    let r_x = member_request(&source, &undefined, "r", 3);
    assert!(matches!(
        settle(r_x).value(),
        TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(_))
    ));

    let incomplete = run(
        &source,
        &call("q", integer(3), 0),
        CallerLimits::default(),
        &Cancel::new(),
    )
    .expect("runs");
    assert!(matches!(incomplete.outcome(), CallOutcome::Incomplete(_)));
    let completed = run_q(&source);
    let expected = member_request(&source, &completed, "q", 9);
    let built = member_request(&source, &incomplete, "q", 9);
    assert_eq!(built.package_id, expected.package_id);
    assert_eq!(built.obligation_identity, expected.obligation_identity);
    assert_eq!(built.replay_target, expected.replay_target);
    assert_eq!(built.byte_provision, expected.byte_provision);
}

fn bound_at(node: NodeKey) -> ProofBound {
    ProofBound::new(
        DomainKey::Node { node, path: vec![] },
        Some(DomainKind::Integer),
        FiniteBound::integer_range(Integer::from(0_i64), Integer::from(9_i64)).unwrap(),
    )
    .unwrap()
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
/// keys; one run's context refuses the other's pairing key, and each
/// context's request for its own member carries its own package and source.
#[trace("TC-1173", "FR-404-AC-5")]
#[test]
fn another_runs_package_is_a_foreign_run() {
    let (first_source, second_source) = (unit(""), unit(S));
    let first = run_q(&first_source);
    let second = run_q(&second_source);
    let own_first = member_request(&first_source, &first, "q", 9);
    let own_second = member_request(&second_source, &second, "q", 9);
    assert_eq!(own_first.package_id.1, first.package_id().hex());
    assert_eq!(own_second.package_id.1, second.package_id().hex());
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

/// FR-404-AC-6: `CallerLimits.spine` bounds the run: `s1.tokens` of 1
/// refuses at the S1 token limit while `run_with_requirements` over the same
/// inputs under `SpineLimits::default()` completes, and the roomy limits
/// reach the member's request.
#[trace("TC-1173", "FR-404-AC-6")]
#[test]
fn only_the_spine_limits_bound_the_run() {
    let source = unit("");
    let call = call("q", integer(3), 1_000_000);
    let tight = CallerLimits::from_operands(["s1.tokens=1"]).unwrap();
    let refusal = run(&source, &call, tight, &Cancel::new()).expect_err("refuses");
    let RunRefusal::Compile(compile) = &*refusal else {
        panic!("a compile refusal, not {refusal:?}");
    };
    assert!(
        format!("{compile:?}").contains("setting: S1Tokens, configured_bound: 1"),
        "{compile:?}"
    );
    run_with_requirements(
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
    .expect("the same inputs under the default SpineLimits complete");
    let roomy = CallerLimits::from_operands([
        "s1.tokens=200000",
        "library.definitions=9000",
        "accounting.work_units=5000",
    ])
    .unwrap();
    let result = run(&source, &call, roomy, &Cancel::new()).expect("completes");
    let member = &result.requirements().members()[0];
    let parameter = parameter(&source, "q");
    let wire = result
        .context()
        .request(
            result.package_id(),
            member.request_index(),
            &[bound_at(parameter)],
            &plugin(),
            assign(parameter, 9),
        )
        .expect("a member request");
    assert_eq!(wire.stage_limits["s1.tokens"], 200_000);
    assert_eq!(wire.stage_limits["library.definitions"], 9000);
    assert_eq!(wire.accounting_limits.work_units, 5000);
}

/// FR-404-AC-7: the refusal order is `ForeignRun`, `NotACallItem`, then
/// FR-290's (`DuplicateBoundKey`), with two proof bounds at one key.
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
    assert!(matches!(
        first
            .context()
            .request(first.package_id(), q_index, &twice, &plugin(), Vec::new())
            .err(),
        Some(PluginReplayRefusal::DuplicateBoundKey { .. })
    ));
    // A foreign package is refused before the index is looked at.
    assert!(matches!(
        refusal(second.package_id(), RequestIndex::new(99)),
        PluginReplayRefusal::ForeignRun { .. }
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
