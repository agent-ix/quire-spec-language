// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-774: FR-290-AC-5 to FR-290-AC-14, the plugin result readers and the
//! plugin replay context.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_foundation::bound::{DomainKey, DomainKind, FiniteBound, ProofBound};
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::{BackendId, Code, RequestIndex, SourceIdentity};
use quire_exact::{Cancel, Integer, Origin, Role};
use serde_json::{json, Value};

use super::*;
use crate::bounds::ReplayLimits;
use crate::execute::{process_provider_obligation, BoundEntries};
use crate::limits::CallerLimits;
use crate::result::ReplayResult;
use crate::spine::{
    check, package, parse, select, DependencyInput, LockEvidence, PackageLimits, ParseRequest,
};
use crate::witness::{CanonicalAssignment, WitnessValue};

fn wrong(pointer: &str) -> PluginResultRefusal {
    PluginResultRefusal::Malformed {
        pointer: pointer.to_owned(),
        cause: ValueFault::WrongValueKind,
    }
}

fn invalid(pointer: &str) -> PluginResultRefusal {
    PluginResultRefusal::Malformed {
        pointer: pointer.to_owned(),
        cause: ValueFault::InvalidValue,
    }
}

fn no_counterexample(pointer: &str, cause: ValueFault) -> PluginResultRefusal {
    PluginResultRefusal::MissingCounterexample {
        pointer: pointer.to_owned(),
        cause,
    }
}

fn missing(pointer: &str) -> PluginResultRefusal {
    PluginResultRefusal::MissingMember {
        pointer: pointer.to_owned(),
    }
}

fn unknown(pointer: &str) -> PluginResultRefusal {
    PluginResultRefusal::UnknownMember {
        pointer: pointer.to_owned(),
    }
}

fn plugin() -> BackendId {
    BackendId::new("acme")
}

fn settle_basis(basis: ProofBasis) -> TerminalRecord {
    TerminalRecord::from_plugin_proof(RequestIndex::new(0), plugin(), basis).unwrap()
}

/// FR-290-AC-5.
#[trace("TC-774", "FR-290-AC-5")]
#[test]
fn tc_774_basis_reader_accepts_the_plugin_spellings() {
    let horizon = json!({"denominator": "2", "numerator": "5"});
    let solver = |method: &str| json!({"type": "bounded-solver", "horizon": horizon, "jumps": 3, "method": method});
    let five_halves = SolverHorizon::new(
        quire_exact::Rational::new(Integer::from(5_i64), Integer::from(2_i64)).unwrap(),
    )
    .unwrap();
    assert_eq!(
        read_plugin_basis(&json!({"type": "exhaustive"})),
        Ok(ProofBasis::Exhaustive)
    );
    assert_eq!(
        read_plugin_basis(&json!({"type": "bounded-complete", "depth": 4})),
        Ok(ProofBasis::BoundedComplete { depth: 4 })
    );
    assert_eq!(
        read_plugin_basis(&json!({"type": "inductive", "depth": 0})),
        Ok(ProofBasis::Inductive { depth: 0 })
    );
    for (method, expected) in [
        ("delta-complete", SolverMethod::DeltaComplete),
        ("flowpipe-unrounded", SolverMethod::FlowpipeUnrounded),
    ] {
        assert_eq!(
            read_plugin_basis(&solver(method)),
            Ok(ProofBasis::BoundedSolver {
                method: expected,
                horizon: five_halves.clone(),
                jumps: 3
            })
        );
    }
    let reduced = read_plugin_basis(&json!({"type": "reduced", "reductions": [
        {"parameters": {"proviso": "breadth-first-revisit"}, "reduction": "partial-order", "row": "invariant"},
        {"parameters": {"classes": [["a", "b"]]}, "reduction": "copy-swap", "row": "hyper"}
    ]}))
    .unwrap();
    assert_eq!(
        reduced,
        ProofBasis::Reduced {
            reductions: vec![
                AppliedReduction::PartialOrder {
                    proviso: Proviso::BreadthFirstRevisit
                },
                AppliedReduction::CopySwap {
                    classes: vec![vec!["a".to_owned(), "b".to_owned()]]
                },
            ]
        }
    );
    let record = settle_basis(reduced);
    assert!(matches!(
        record.value(),
        TerminalValue::Proved {
            certification: Certification::Trusted,
            ..
        }
    ));
    assert_eq!(record.backend(), Some(&plugin()));
}

/// FR-290-AC-6.
#[trace("TC-774", "FR-290-AC-6")]
#[test]
fn tc_774_basis_reader_refuses_in_order() {
    for basis in [
        json!({"type": "bounded-proof", "checks": 3}),
        json!({"type": "closed-form", "extra": 1}),
        json!({"type": "witnessed", "sources": []}),
        json!({"type": "zone-certified"}),
        json!({"type": "exact-value", "value": "1"}),
        json!({"type": "value-bounds", "lower": "0", "upper": "1"}),
    ] {
        assert_eq!(
            read_plugin_basis(&basis),
            Err(PluginResultRefusal::CoreOnlyBasis)
        );
    }
    let spelling = |pointer: &str| PluginResultRefusal::UnknownSpelling {
        pointer: pointer.to_owned(),
    };
    let solver = |horizon: Value, method: &str| json!({"type": "bounded-solver", "horizon": horizon, "jumps": 1, "method": method});
    let one = json!({"denominator": "1", "numerator": "1"});
    let cases: Vec<(Value, PluginResultRefusal)> = vec![
        (json!({"type": "replay-parity"}), spelling("/basis/type")),
        (
            json!({"type": "bounded_complete", "depth": 4}),
            spelling("/basis/type"),
        ),
        (json!("exhaustive"), wrong("/basis")),
        (json!({"depth": 4}), missing("/basis/type")),
        (json!({"type": 7}), wrong("/basis/type")),
        (json!({"type": "inductive"}), missing("/basis/depth")),
        // A missing payload member comes before an undefined one.
        (
            json!({"type": "inductive", "extra": 1}),
            missing("/basis/depth"),
        ),
        (
            json!({"type": "exhaustive", "depth": 3}),
            unknown("/basis/depth"),
        ),
        (
            json!({"type": "inductive", "depth": "3"}),
            wrong("/basis/depth"),
        ),
        (
            json!({"type": "inductive", "depth": -1}),
            invalid("/basis/depth"),
        ),
        (
            json!({"type": "inductive", "depth": 9_007_199_254_740_992_u64}),
            invalid("/basis/depth"),
        ),
        (
            solver(one.clone(), "dreal"),
            PluginResultRefusal::Unsupported {
                pointer: "/basis/method".to_owned(),
                feature: "dreal".to_owned(),
            },
        ),
        (solver(one, ""), invalid("/basis/method")),
        (
            solver(
                json!({"denominator": "1", "numerator": "-1"}),
                "delta-complete",
            ),
            invalid("/basis/horizon"),
        ),
        (
            solver(
                json!({"denominator": "4", "numerator": "2"}),
                "delta-complete",
            ),
            invalid("/basis/horizon"),
        ),
        (
            json!({"type": "reduced", "reductions": [
                {"parameters": {"declarations": []}, "reduction": "symmetry", "row": "r"}]}),
            PluginResultRefusal::Unsupported {
                pointer: "/basis/reductions/0/reduction".to_owned(),
                feature: "symmetry".to_owned(),
            },
        ),
        (
            json!({"type": "reduced", "reductions": [
                {"parameters": {"classes": [["a", "a"]]}, "reduction": "copy-swap", "row": "r"}]}),
            invalid("/basis/reductions/0/parameters/classes/0"),
        ),
    ];
    for (basis, expected) in cases {
        assert_eq!(read_plugin_basis(&basis), Err(expected), "{basis}");
    }
}

/// FR-290-AC-7.
#[trace("TC-774", "FR-290-AC-7")]
#[test]
fn tc_774_certification_reader_accepts_only_trusted() {
    assert_eq!(read_plugin_certification(Some(&json!("trusted"))), Ok(()));
    assert_eq!(
        read_plugin_certification(None),
        Err(missing("/certification"))
    );
    assert_eq!(
        read_plugin_certification(Some(&json!("vouched"))),
        Err(invalid("/certification"))
    );
    assert_eq!(
        read_plugin_certification(Some(&json!(7))),
        Err(wrong("/certification"))
    );
    for received in ["certified", "uncertified"] {
        assert_eq!(
            read_plugin_certification(Some(&json!(received))),
            Err(PluginResultRefusal::CertificationNotTrusted {
                received: received.to_owned()
            })
        );
    }
    // The driver reads `certification` before `basis`.
    let entry =
        json!({"certification": "certified", "basis": {"type": "bounded-proof", "checks": 3}});
    let driver = read_plugin_certification(entry.get("certification"))
        .and_then(|()| read_plugin_basis(&entry["basis"]));
    assert!(matches!(
        driver,
        Err(PluginResultRefusal::CertificationNotTrusted { .. })
    ));
}

/// FR-290-AC-8.
#[trace("TC-774", "FR-290-AC-8")]
#[test]
fn tc_774_cause_reader_accepts_the_plugin_spellings() {
    for (spelling, expected) in [
        ("undecided-successor", InconclusiveCause::UndecidedSuccessor),
        ("no-initial-state", InconclusiveCause::NoInitialState),
        ("no-instance", InconclusiveCause::NoInstance),
        (
            "sufficient-test-failed",
            InconclusiveCause::SufficientTestFailed,
        ),
        ("solver-inconclusive", InconclusiveCause::SolverInconclusive),
        ("match-undetermined", InconclusiveCause::MatchUndetermined),
        ("vacuous-match", InconclusiveCause::VacuousMatch),
    ] {
        assert_eq!(
            read_plugin_inconclusive_cause(&json!({"type": spelling})),
            Ok(expected)
        );
    }
    let key = "ab".repeat(32);
    let cases: Vec<(Value, InconclusiveCause)> = vec![
        (
            json!({"type": "bound-reached", "depth": 7}),
            InconclusiveCause::BoundReached { depth: 7 },
        ),
        (
            json!({"type": "reduction-not-preserving", "form": "liveness", "reduction": "constraint"}),
            InconclusiveCause::ReductionNotPreserving {
                reduction: ReductionKind::StateConstraint,
                form: PropertyForm::Liveness,
            },
        ),
        (
            json!({"type": "constraint-reached", "boundary_states": 2}),
            InconclusiveCause::ConstraintReached { boundary_states: 2 },
        ),
        (
            json!({"type": "reduction-horizon", "max_depth": 5}),
            InconclusiveCause::ReductionHorizon { max_depth: 5 },
        ),
        (
            json!({"type": "memory-bound-reached", "limit": "max_messages", "states": 3, "value": 2}),
            InconclusiveCause::MemoryBoundReached {
                bound: MemoryBound {
                    member: MemoryBoundMember::MaxMessages,
                    value: 2,
                },
                states: 3,
            },
        ),
        (
            json!({"type": "instance-bound-reached", "limit": "max_live_instances", "states": 1, "value": 4}),
            InconclusiveCause::InstanceBoundReached {
                bound: InstanceBound { value: 4 },
                states: 1,
            },
        ),
        (
            json!({"type": "mapping-undetermined", "row": "r", "state_key": key, "prefix": []}),
            InconclusiveCause::MappingUndetermined,
        ),
    ];
    for (cause, expected) in cases {
        let read = read_plugin_inconclusive_cause(&cause).unwrap();
        assert_eq!(read, expected, "{cause}");
        let record =
            TerminalRecord::from_plugin_inconclusive(RequestIndex::new(1), plugin(), read).unwrap();
        assert_eq!(record.value(), &TerminalValue::Inconclusive(expected));
    }
}

/// FR-290-AC-9.
#[trace("TC-774", "FR-290-AC-9")]
#[test]
fn tc_774_cause_reader_refuses_in_order() {
    for (spelling, cause) in [
        ("replay-parity", QslOnlyCause::ReplayParity),
        ("replay-refused", QslOnlyCause::ReplayRefused),
        ("kani_vacuous_proof", QslOnlyCause::KaniVacuousProof),
    ] {
        for body in [
            json!({"type": spelling}),
            json!({"type": spelling, "extra": 1}),
        ] {
            assert_eq!(
                read_plugin_inconclusive_cause(&body),
                Err(PluginResultRefusal::QslOnlyCause(cause))
            );
        }
    }
    for spelling in [
        "closed-form",
        "kani-vacuous-proof",
        "scalar-agrees",
        "bound_reached",
    ] {
        assert_eq!(
            read_plugin_inconclusive_cause(&json!({"type": spelling, "analysis": "edf"})),
            Err(PluginResultRefusal::UnknownSpelling {
                pointer: "/cause/type".to_owned()
            })
        );
    }
    for spelling in [
        "certificate-rejected",
        "well-definedness-unchecked",
        "symmetry-broken",
        "lasso-not-concretized",
        "no-admitted-behaviour",
        "transfer-not-preserving",
        "premise-unproved",
        "hypotheses-not-established",
    ] {
        assert_eq!(
            read_plugin_inconclusive_cause(&json!({"type": spelling})),
            Err(PluginResultRefusal::Unsupported {
                pointer: "/cause/type".to_owned(),
                feature: spelling.to_owned()
            })
        );
    }
    let cases: Vec<(Value, PluginResultRefusal)> = vec![
        (
            json!({"type": "reduction-not-preserving", "form": "fairness", "reduction": "symmetry"}),
            PluginResultRefusal::Unsupported {
                pointer: "/cause/form".to_owned(),
                feature: "fairness".to_owned(),
            },
        ),
        (
            json!({"type": "constraint-reached", "boundary_states": 0}),
            invalid("/cause/boundary_states"),
        ),
        (
            json!({"type": "memory-bound-reached", "limit": "max_store", "states": 1, "value": 1}),
            invalid("/cause/limit"),
        ),
        (
            json!({"type": "mapping-undetermined", "row": "r", "state_key": "abc", "prefix": []}),
            invalid("/cause/state_key"),
        ),
        (
            json!({"type": "mapping-undetermined", "row": "r", "state_key": "ab".repeat(32),
                   "prefix": [{"transition": {"type": "terminal-stutter"}}]}),
            missing("/cause/prefix/0/post_state"),
        ),
        (
            json!({"type": "no-instance", "depth": 1}),
            unknown("/cause/depth"),
        ),
    ];
    for (cause, expected) in cases {
        assert_eq!(
            read_plugin_inconclusive_cause(&cause),
            Err(expected),
            "{cause}"
        );
    }
}

fn entry(values: Value) -> Value {
    json!({"backend": "acme", "request_index": 0,
           "counterexample": {"type": "assignments", "values": values}})
}

/// FR-290-AC-10.
#[trace("TC-774", "FR-290-AC-10")]
#[test]
fn tc_774_assignment_reader_reads_and_refuses_in_order() {
    let parameter = "11".repeat(32);
    let value = json!({"type": "integer", "value": "0"});
    assert_eq!(
        read_plugin_assignments(&entry(json!([{"parameter": parameter, "value": value}]))),
        Ok(vec![CanonicalAssignment {
            parameter: WireNodeId::from_digest([0x11; 32]),
            value: WitnessValue::ExactInteger(Integer::from(0_i64)),
        }])
    );
    assert_eq!(read_plugin_assignments(&entry(json!([]))), Ok(Vec::new()));
    let counterexample = |body: Value| json!({"counterexample": body});
    let mut with_extra = entry(json!([]));
    with_extra["extra"] = json!(1);
    let element = |parameter: &str, value: Value| json!({"parameter": parameter, "value": value});
    let cases: Vec<(Value, PluginResultRefusal)> = vec![
        (json!([]), wrong("")),
        (
            json!({"transcript": {}}),
            no_counterexample("/counterexample", ValueFault::MissingMember),
        ),
        (
            json!({}),
            no_counterexample("/counterexample", ValueFault::MissingMember),
        ),
        (
            json!({"transcript": {}, "counterexample": {"type": "assignments", "values": []}}),
            unknown("/transcript"),
        ),
        (
            counterexample(json!(7)),
            no_counterexample("/counterexample", ValueFault::WrongValueKind),
        ),
        (
            counterexample(json!({"values": []})),
            no_counterexample("/counterexample/type", ValueFault::MissingMember),
        ),
        (
            counterexample(json!({"type": 7, "values": []})),
            no_counterexample("/counterexample/type", ValueFault::WrongValueKind),
        ),
        (
            counterexample(json!({"type": "temporal", "values": []})),
            no_counterexample("/counterexample/type", ValueFault::InvalidValue),
        ),
        (with_extra, unknown("/extra")),
        (
            counterexample(json!({"type": "assignments"})),
            missing("/counterexample/values"),
        ),
        (
            counterexample(json!({"type": "assignments", "values": {}})),
            wrong("/counterexample/values"),
        ),
        (
            entry(json!([{"parameter": parameter}])),
            missing("/counterexample/values/0/value"),
        ),
        (
            entry(json!([element(&"1".repeat(63), value.clone())])),
            invalid("/counterexample/values/0/parameter"),
        ),
        (
            entry(json!([element(&"A1".repeat(32), value.clone())])),
            invalid("/counterexample/values/0/parameter"),
        ),
        (
            entry(json!([element(
                &parameter,
                json!({"type": "integer", "value": "07"})
            )])),
            invalid("/counterexample/values/0/value"),
        ),
        (
            entry(json!([
                element(&parameter, value.clone()),
                element(&parameter, value)
            ])),
            invalid("/counterexample/values/1/parameter"),
        ),
    ];
    for (body, expected) in cases {
        assert_eq!(read_plugin_assignments(&body), Err(expected), "{body}");
    }
}

/// FR-290-AC-11.
#[trace("TC-774", "FR-290-AC-11")]
#[test]
fn tc_774_refusal_diagnostics_carry_the_table_codes() {
    use ValueFault::{InvalidValue, MissingMember, WrongValueKind};
    let spelling = PluginResultRefusal::UnknownSpelling {
        pointer: "/cause/type".to_owned(),
    };
    // (refusal, code, cause, the document_pointer its detail names)
    let expectations = [
        (
            PluginResultRefusal::CoreOnlyBasis,
            "invalid_result",
            Some("unknown-cause"),
            "/basis/type",
        ),
        (
            PluginResultRefusal::QslOnlyCause(QslOnlyCause::ReplayParity),
            "invalid_result",
            Some("unknown-cause"),
            "/cause/type",
        ),
        (
            spelling,
            "invalid_result",
            Some("unknown-cause"),
            "/cause/type",
        ),
        (
            missing("/basis/depth"),
            "invalid_result",
            Some("missing-member"),
            "/basis/depth",
        ),
        (
            unknown("/basis/depth"),
            "invalid_result",
            Some("unknown-member"),
            "/basis/depth",
        ),
        (
            wrong("/basis/depth"),
            "invalid_result",
            Some("wrong-value-kind"),
            "/basis/depth",
        ),
        (
            invalid("/basis/depth"),
            "invalid_result",
            Some("invalid-value"),
            "/basis/depth",
        ),
        (
            no_counterexample("/counterexample", MissingMember),
            "invalid_result",
            Some("missing-member"),
            "/counterexample",
        ),
        (
            no_counterexample("/counterexample", WrongValueKind),
            "invalid_result",
            Some("wrong-value-kind"),
            "/counterexample",
        ),
        (
            no_counterexample("/counterexample/type", InvalidValue),
            "invalid_result",
            Some("invalid-value"),
            "/counterexample/type",
        ),
        (
            PluginResultRefusal::CertificationNotTrusted {
                received: "certified".to_owned(),
            },
            "invalid_result",
            Some("certification-mismatch"),
            "/certification",
        ),
        (
            PluginResultRefusal::Unsupported {
                pointer: "/basis/method".to_owned(),
                feature: "dreal".to_owned(),
            },
            "unknown_required_feature",
            Some("unsupported-feature"),
            "/basis/method",
        ),
    ];
    for (refusal, code, cause, pointer) in expectations {
        let diagnostic = refusal.diagnostic(3, "received-text");
        assert_eq!(diagnostic.code(), code, "{refusal:?}");
        assert_eq!(diagnostic.cause(), cause, "{refusal:?}");
        let message = diagnostic.message();
        assert!(message.contains("result item 3"), "{message}");
        assert!(message.contains(pointer), "{message}");
        if code == "invalid_result" {
            assert!(
                message.contains(&format!("document_pointer `{pointer}`")),
                "{message}"
            );
            assert!(message.contains("received `received-text`"), "{message}");
        }
        if cause == Some("certification-mismatch") {
            assert!(
                message.contains("expected `trusted`, actual `certified`"),
                "{message}"
            );
        }
        let record = refusal.record(RequestIndex::new(3), plugin());
        assert_eq!(record.value(), &TerminalValue::Failed);
        assert_eq!(record.backend(), Some(&plugin()));
    }
}

const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n\
    function p using v(x: Int[0, 9]): Boolean pure { x > 3 }\n\
    function q using v(x: Int[0, 9]): Integer pure { x + 1 }\n\
    function r using v(x: Int[0, 9]): Boolean pure { x + 1 > 3 }\n";

struct Fixture {
    checked: crate::spine::CheckedUnit,
    emitted: crate::spine::EmittedUnit,
    models: crate::spine::AdmittedModels,
}

fn fixture() -> Fixture {
    fixture_of(SOURCE, &BTreeMap::new(), &DependencyInput::default())
}

fn fixture_of(
    source: &str,
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    dependencies: &DependencyInput,
) -> Fixture {
    let identity = SourceIdentity::new("agent-ix", "test:plugin", "fixture", "fixture:1");
    let limits = CallerLimits::default();
    let cancel = Cancel::new();
    let parsed = parse(
        &ParseRequest {
            source: &identity,
            path: "unit.native",
            bytes: source.as_bytes(),
        },
        limits.spine.source,
        &cancel,
    )
    .unwrap_or_else(|failure| panic!("parse: {failure:?}"))
    .into_value();
    let models = select(&parsed, packages, limits.spine.model, &cancel)
        .unwrap_or_else(|failure| panic!("select: {failure:?}"))
        .into_value();
    let checked = check(
        &parsed,
        &models,
        dependencies,
        &LockEvidence::default(),
        limits.spine,
        &cancel,
    )
    .unwrap_or_else(|failure| panic!("check: {failure:?}"))
    .into_value();
    let emitted = package(&checked, PackageLimits::default(), &cancel)
        .unwrap_or_else(|failure| panic!("package: {failure:?}"))
        .into_value();
    Fixture {
        checked,
        emitted,
        models,
    }
}

fn context(fixture: &Fixture, limits: CallerLimits) -> PluginReplayContext {
    PluginReplayContext::new(&fixture.checked, &fixture.emitted, &fixture.models, limits)
}

fn declaration(fixture: &Fixture, name: &str) -> (OccurrenceKey, WireNodeId) {
    let graph = fixture.checked.package().graph();
    let callable = graph.callable(name).expect("declared");
    let parameter = crate::execute::callable_parameter_keys(fixture.checked.package(), &callable)
        .expect("a function node")[0];
    let node = WireNodeId::from_digest(*callable.identity.as_bytes());
    (
        OccurrenceKey::new(node, Origin::new(Role::new("declaration"), 0)),
        WireNodeId::from_digest(*parameter.as_bytes()),
    )
}

fn bound_at(node: WireNodeId) -> ProofBound {
    ProofBound::new(
        DomainKey::Node { node, path: vec![] },
        Some(DomainKind::Integer),
        FiniteBound::integer_range(Integer::from(0_i64), Integer::from(9_i64)).unwrap(),
    )
    .unwrap()
}

fn input(parameter: WireNodeId, x: i128) -> Vec<CanonicalAssignment> {
    vec![CanonicalAssignment {
        parameter,
        value: WitnessValue::Integer(x),
    }]
}

fn settle(
    context: &PluginReplayContext,
    key: &OccurrenceKey,
    bounds: &[ProofBound],
    assignments: Vec<CanonicalAssignment>,
) -> TerminalRecord {
    match context.request(key, bounds, &plugin(), assignments) {
        Ok(wire) => {
            let replayed = crate::replay(wire, ReplayLimits::default());
            TerminalRecord::from_plugin_counterexample(RequestIndex::new(0), plugin(), &replayed)
        }
        Err(refusal) => refusal.record(RequestIndex::new(0), plugin()),
    }
}

/// FR-290-AC-12 and the QSL-977 correctness fix: a value-validity item is not
/// replayed as the owning function's predicate.
#[trace("TC-774", "FR-290-AC-12")]
#[test]
fn tc_774_function_contract_items_replay_and_value_validity_items_do_not() {
    let fixture = fixture();
    let context = context(&fixture, CallerLimits::default());
    let (p, p_x) = declaration(&fixture, "p");
    let bounds = [bound_at(p_x)];
    let wire = context
        .request(&p, &bounds, &plugin(), input(p_x, 0))
        .expect("a function-contract item");
    assert_eq!(wire.selected_function.to_string(), "p");
    assert_eq!(wire.backend, "acme");
    assert_eq!(wire.source_digests.len(), 1);
    assert_eq!(wire.declared_domains.len(), 1);
    assert!(wire.state_environment.entries().is_empty());
    assert!(wire.object_environment.is_none());
    assert_eq!(wire.byte_provision.len(), 1);
    assert_eq!(wire.byte_provision[0].2, SOURCE.as_bytes());
    let replayed = crate::replay(wire.clone(), ReplayLimits::default());
    let Ok(ReplayResult::Input(arm)) = &replayed else {
        panic!("expected an input replay, got {replayed:?}");
    };
    assert_eq!(
        arm.category(),
        qsl_foundation::diagnostic::Category::Violation
    );

    let refuted = settle(&context, &p, &bounds, input(p_x, 0));
    assert_eq!(refuted.value(), &TerminalValue::Refuted);
    assert_eq!(refuted.backend(), Some(&plugin()));
    let parity = settle(&context, &p, &bounds, input(p_x, 5));
    assert!(matches!(
        parity.value(),
        TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(_))
    ));

    // The item's identity is the process-provider preimage of its bounds.
    let entries = BoundEntries::new(bounds.to_vec()).unwrap();
    let identity = process_provider_obligation(p.node(), p.origin(), &entries).unwrap();
    assert_eq!(wire.obligation_identity, *identity.as_bytes());

    // `r(x) = x + 1 > 3` holds for every x, yet `r(0)` is false: the item at
    // `r`'s application is a value-validity item, so replaying the owner as a
    // predicate would wrongly refute it. It is non-replayable and settles
    // inconclusive, while `r`'s own function-contract item is still replayed.
    let (r, r_x) = declaration(&fixture, "r");
    let r_application = fixture
        .checked
        .package()
        .graph()
        .requirements()
        .iter()
        .find(|(_, record)| record.owner() == Some("r"))
        .map(|(key, _)| key.clone())
        .expect("r's application has a requirement record");
    let record = settle(&context, &r_application, &[bound_at(r_x)], input(r_x, 0));
    assert_eq!(
        record.value(),
        &TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(
            Code::UnknownRequiredFeature
        ))
    );
    assert!(record.replay_result().is_none());
    assert_eq!(
        settle(&context, &r, &[bound_at(r_x)], input(r_x, 0)).value(),
        &TerminalValue::Refuted
    );

    // A value-validity item at `q`'s application `x + 1` is non-replayable.
    let (q, q_x) = declaration(&fixture, "q");
    let application = fixture
        .checked
        .package()
        .graph()
        .requirements()
        .iter()
        .find(|(_, record)| record.owner() == Some("q"))
        .map(|(key, _)| key.clone())
        .expect("q's application has a requirement record");
    for key in [&application, &q] {
        let refusal = context
            .request(key, &[bound_at(q_x)], &plugin(), input(q_x, 9))
            .err()
            .expect("a non-replayable item");
        assert_eq!(
            refusal,
            PluginReplayRefusal::NotReplayable { key: key.clone() }
        );
        let record = settle(&context, key, &[bound_at(q_x)], input(q_x, 9));
        assert_eq!(
            record.value(),
            &TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(
                Code::UnknownRequiredFeature
            ))
        );
        assert!(record.replay_result().is_none());
        assert_eq!(record.backend(), Some(&plugin()));
    }
}

/// FR-290-AC-13.
#[trace("TC-774", "FR-290-AC-13")]
#[test]
fn tc_774_process_provider_preimage_encodes_and_digests() {
    let node = WireNodeId::from_digest([0x11; 32]);
    let origin = Origin::new(Role::new("declaration"), 0);
    let bound = bound_at(WireNodeId::from_digest([0x22; 32]));
    let with = process_provider_obligation(node, &origin, &BoundEntries::new(vec![bound]).unwrap())
        .unwrap();
    let without =
        process_provider_obligation(node, &origin, &BoundEntries::new(Vec::new()).unwrap())
            .unwrap();
    let hex = |bytes: &[u8; 32]| {
        bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    };
    assert_eq!(
        hex(with.as_bytes()),
        "3df483b143782f27840dd0d42eca97ba51fdb60c01cb3ddbc48db020de6ac899"
    );
    assert_eq!(
        hex(without.as_bytes()),
        "95905de0c8797755b51edf13b26a4fcdb41908a1e55f43b60784e280fa8e3a1c"
    );
}

/// FR-290-AC-14.
#[trace("TC-774", "FR-290-AC-14")]
#[test]
fn tc_774_limits_flow_into_the_request_and_refusals_are_ordered() {
    let fixture = fixture();
    let limits = CallerLimits::from_operands([
        "s1.tokens=200000",
        "library.definitions=9000",
        "work_units=5000",
    ])
    .unwrap();
    let defaults = CallerLimits::default();
    let context = context(&fixture, limits);
    let (p, p_x) = declaration(&fixture, "p");
    let wire = context.request(&p, &[], &plugin(), input(p_x, 0)).unwrap();
    assert_eq!(wire.stage_limits["s1.tokens"], 200_000);
    assert_eq!(wire.stage_limits["library.definitions"], 9000);
    assert_eq!(wire.accounting_limits.work_units, 5000);
    assert!(!wire.stage_limits.contains_key("replay.input_bytes"));
    assert!(!wire.stage_limits.contains_key("format.output_bytes"));
    use qsl_foundation::SettingLimits;
    for (setting, bound) in defaults.bounds() {
        let name = setting.name();
        if matches!(
            name,
            "s1.tokens" | "library.definitions" | "replay.input_bytes" | "format.output_bytes"
        ) {
            continue;
        }
        assert_eq!(wire.stage_limits.get(name), Some(&bound), "{name}");
    }

    // The occurrence checks come before the bound check: a non-replayable
    // item with two bounds at one key is `NotReplayable`, not a duplicate.
    let (_, q_x) = declaration(&fixture, "q");
    let application = fixture
        .checked
        .package()
        .graph()
        .requirements()
        .iter()
        .find(|(_, record)| record.owner() == Some("q"))
        .map(|(key, _)| key.clone())
        .expect("q's application has a requirement record");
    assert_eq!(
        context
            .request(
                &application,
                &[bound_at(q_x), bound_at(q_x)],
                &plugin(),
                Vec::new()
            )
            .err(),
        Some(PluginReplayRefusal::NotReplayable {
            key: application.clone()
        })
    );
    // Only the `declaration` occurrence at ordinal 0 names a function.
    let off_ordinal = OccurrenceKey::new(p.node(), Origin::new(Role::new("declaration"), 1));
    assert_eq!(
        context
            .request(&off_ordinal, &[], &plugin(), Vec::new())
            .err(),
        Some(PluginReplayRefusal::UnknownOccurrence { key: off_ordinal })
    );

    // An unknown key comes before NotReplayable; duplicate bounds come last.
    let unknown_key = OccurrenceKey::new(
        WireNodeId::from_digest([0x99; 32]),
        Origin::new(Role::new("declaration"), 0),
    );
    assert_eq!(
        context
            .request(&unknown_key, &[], &plugin(), Vec::new())
            .err(),
        Some(PluginReplayRefusal::UnknownOccurrence {
            key: unknown_key.clone()
        })
    );
    let duplicate = [bound_at(p_x), bound_at(p_x)];
    let refusal = context
        .request(&p, &duplicate, &plugin(), input(p_x, 0))
        .err()
        .expect("two bounds at one key");
    assert!(matches!(
        refusal,
        PluginReplayRefusal::DuplicateBoundKey { .. }
    ));
    for (refusal, code, cause) in [
        (
            PluginReplayRefusal::UnknownOccurrence { key: unknown_key },
            "missing_declaration",
            "missing-name",
        ),
        (refusal, "invalid_runtime_input", "duplicate-member"),
    ] {
        let record = refusal.record(RequestIndex::new(0), plugin());
        assert_eq!(record.value(), &TerminalValue::Failed);
        assert_eq!(record.backend(), Some(&plugin()));
        let diagnostic = refusal.diagnostic(0);
        assert_eq!(diagnostic.code(), code);
        assert_eq!(diagnostic.cause(), Some(cause));
    }
}

/// FR-290-AC-14: a record no function holds (an operation frame's or a state
/// clause's `operation-contract` record) is non-replayable.
#[trace("TC-774", "FR-290-AC-14")]
#[test]
fn tc_774_a_state_clause_record_is_not_replayable() {
    let (source, packages) = crate::spine::clause::tests::config_version_unit_and_packages();
    let fixture = fixture_of(&source, &packages, &DependencyInput::default());
    let context = context(&fixture, CallerLimits::default());
    let keys: Vec<OccurrenceKey> = fixture
        .checked
        .package()
        .graph()
        .requirements()
        .iter()
        .filter(|(_, record)| record.owner().is_none())
        .map(|(key, _)| key.clone())
        .collect();
    assert!(!keys.is_empty(), "the unit has operation-contract records");
    for key in keys {
        assert_eq!(
            context.request(&key, &[], &plugin(), Vec::new()).err(),
            Some(PluginReplayRefusal::NotReplayable { key: key.clone() })
        );
        let record = settle(&context, &key, &[], Vec::new());
        assert_eq!(
            record.value(),
            &TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(
                Code::UnknownRequiredFeature
            ))
        );
    }
}

/// FR-290 "Replay context": `dependencies` carries every resolved library, so
/// a unit that imports `b`, which imports `c`, replays end to end.
#[trace("TC-774", "FR-290-AC-12")]
#[test]
fn tc_774_a_transitive_dependency_replays() {
    let header =
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\n";
    let c = format!("{header}function big using v(x: Int[0, 9]): Boolean pure {{ x > 3 }}\n");
    let b = format!(
        "{header}import \"test/c\" as c;\nfunction big using v(x: Int[0, 9]): Boolean pure {{ c::big(x) }}\n"
    );
    let a = format!(
        "{header}import \"test/b\" as b;\nfunction q using v(x: Int[0, 9]): Boolean pure {{ b::big(x) }}\n"
    );
    let library = |identity: &str, bytes: &String| crate::spine::SuppliedLibrary {
        identity: format!("test/{identity}"),
        source: SourceIdentity::new(
            "agent-ix",
            format!("test:{identity}"),
            "fixture",
            "fixture:1",
        ),
        path: format!("{identity}.native"),
        bytes: bytes.as_bytes().to_vec(),
    };
    let supplied =
        DependencyInput::new(vec![library("b", &b), library("c", &c)]).expect("supplied");
    let fixture = fixture_of(&a, &BTreeMap::new(), &supplied);
    let context = context(&fixture, CallerLimits::default());
    let (q, q_x) = declaration(&fixture, "q");
    let wire = context
        .request(&q, &[bound_at(q_x)], &plugin(), input(q_x, 0))
        .expect("a function-contract item");
    let identities: Vec<&str> = wire
        .dependencies
        .iter()
        .map(|entry| entry.identity.as_str())
        .collect();
    assert_eq!(identities, ["test/b", "test/c"]);
    assert_eq!(
        settle(&context, &q, &[bound_at(q_x)], input(q_x, 0)).value(),
        &TerminalValue::Refuted
    );
    assert!(matches!(
        settle(&context, &q, &[bound_at(q_x)], input(q_x, 7)).value(),
        TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(_))
    ));
}
