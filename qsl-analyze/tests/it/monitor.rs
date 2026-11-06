// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-283 (TC-765, TC-766): `monitor` over the `Counter` subject. Each trace
//! position is a snapshot of one counter `c1` with the stated `value`, read
//! through genuine FR-106 admission and the TemporalTrace evaluator.

use ix_trace_rs::trace;
use qsl_analyze::monitor::{
    monitor, MonitorClause, MonitorLimits, MonitorOutcome, MonitorRefusal, MonitorRequest,
    MonitorVerdict, TRACE_FORMAT,
};
use qsl_foundation::diagnostic::StageFailure;
use qsl_package::CheckedPackage;
use qsl_replay::spine::{
    run_clause, ClauseDisposition, ClauseRunReport, ClauseRunRequest, ClauseRunSelection,
    ClauseRunSource, DependencyInput, OverObject, SpineLimits, TemporalUndefinedCause,
    TemporalVerdict, TraceInput,
};
use qsl_replay::{OutcomeDocument, OutcomeItem, OutcomeStage};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::package_input;
use qsl_semantics::model::observation::{DocumentRef, ObservationLimits};
use quire_exact::{Cancel, CancelCause, ScalarLimits};
use quire_semantic_value::catalog::Category;
use quire_semantic_value::fairness::{FairnessGranularity, FairnessKind};
use serde_json::{json, Value as Json};

use crate::counter::{
    bounded_package_with_clauses, clause_text, document, package_with_clauses, unit_text,
    Documents, UNLIMITED,
};

const ALWAYS_SMALL: &str = "always holds(c.value < 2)";
const ALWAYS_EVENTUALLY_THREE: &str = "always eventually holds(c.value = 3)";
const DIVIDES: &str = "always holds(3 mod (2 - c.value) >= 0)";
/// Undefined where `c.value` is 0 (FR-283-AC-5's `10 / x` at `x` = 0).
const DIVIDES_BY_VALUE: &str = "always holds(10 mod c.value >= 0)";
const REACH_TWO: &str = "eventually[0,2] holds(c.value = 2)";
const REACH_ONE: &str = "eventually[0,1] holds(c.value = 2)";

struct World {
    package: CheckedPackage,
    documents: Documents,
}

fn world(clauses: &[(&str, &str, &str)]) -> World {
    let document = document(false);
    let (package, _) = package_with_clauses(&document, "", clauses);
    World {
        package,
        documents: Documents::new(document),
    }
}

/// A world whose clauses run under the bounded event-position profile.
fn bounded_world(clauses: &[(&str, &str, &str)]) -> World {
    let document = document(false);
    World {
        package: bounded_package_with_clauses(&document, clauses),
        documents: Documents::new(document),
    }
}

fn json_of(references: &[DocumentRef]) -> Vec<Json> {
    references
        .iter()
        .map(|reference| {
            json!({"identity": {"authority": reference.authority,
                "identity": reference.identity}})
        })
        .collect()
}

impl World {
    /// One snapshot per value, provisioned here.
    fn refs(&mut self, tag: &str, values: &[i64]) -> Vec<DocumentRef> {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                self.documents
                    .start(&format!("{tag}-{index}"), &[("c1", *value)])
            })
            .collect()
    }

    /// The `DocumentRef` JSON of one snapshot per value, provisioned here.
    fn positions(&mut self, tag: &str, values: &[i64]) -> Vec<Json> {
        json_of(&self.refs(tag, values))
    }

    fn run(
        &self,
        names: &[&str],
        trace: &[u8],
        accounting: ScalarLimits,
        cancel: &Cancel,
    ) -> Result<MonitorOutcome, StageFailure<MonitorRefusal>> {
        self.run_over(names, "c1", trace, accounting, cancel)
    }

    fn run_over(
        &self,
        names: &[&str],
        key: &str,
        trace: &[u8],
        accounting: ScalarLimits,
        cancel: &Cancel,
    ) -> Result<MonitorOutcome, StageFailure<MonitorRefusal>> {
        let request = MonitorRequest {
            clauses: names
                .iter()
                .map(|name| MonitorClause {
                    name: (*name).to_owned(),
                    over: OverObject {
                        population: "ix://test/counter/counters".to_owned(),
                        key: key.to_owned(),
                    },
                })
                .collect(),
            trace,
            snapshots: &self.documents.snapshots,
            packages: &self.documents.packages,
        };
        let limits = MonitorLimits {
            observation: ObservationLimits::default(),
            normalization: ModelNormalizationLimits::default(),
            accounting,
        };
        monitor(&self.package, &request, &limits, cancel).map(|staged| staged.into_value())
    }

    fn finite(&mut self, names: &[&str], values: &[i64]) -> MonitorOutcome {
        let prefix = self.positions("f", values);
        let trace = json!({"format": TRACE_FORMAT, "prefix": prefix}).to_string();
        self.run(names, trace.as_bytes(), UNLIMITED, &Cancel::new())
            .expect("the request completes")
    }

    fn lasso(&mut self, names: &[&str], prefix: &[i64], repeat: &[i64]) -> MonitorOutcome {
        let head = self.positions("p", prefix);
        let tail = self.positions("l", repeat);
        let trace = json!({"format": TRACE_FORMAT, "prefix": head, "loop": tail}).to_string();
        self.run(names, trace.as_bytes(), UNLIMITED, &Cancel::new())
            .expect("the request completes")
    }
}

fn refused(result: Result<MonitorOutcome, StageFailure<MonitorRefusal>>) -> MonitorRefusal {
    match result {
        Err(StageFailure::Refused(refusal)) => refusal,
        other => panic!("expected a whole-request refusal, got {other:?}"),
    }
}

/// FR-283-AC-1, AC-7: the violation is at the activation position and
/// carries its counterexample and no witness; an all-true finite trace is
/// pending.
#[trace("TC-765", "FR-283-AC-1")]
#[trace("TC-765", "FR-283-AC-7")]
#[test]
fn a_finite_trace_violates_at_its_activation_or_stays_pending() {
    let mut world = world(&[("P", "", ALWAYS_SMALL)]);
    let outcome = world.finite(&["P"], &[0, 1, 2]);
    let [record] = outcome.records.as_slice() else {
        panic!("one record per selected clause");
    };
    assert_eq!(record.clause, "P");
    assert_eq!(record.verdict, MonitorVerdict::Violated);
    assert_eq!(record.category, Category::Violation);
    assert_eq!(record.report().disposition.exit_code(), 10);
    let counterexample = record.counterexample().expect("a counterexample");
    assert_eq!(counterexample.trace_position.index(), 0);
    let identities: Vec<_> = counterexample
        .prefix
        .iter()
        .map(|step| step.document.clone())
        .collect();
    assert_eq!(identities, world.refs("f", &[0, 1, 2]));
    assert_eq!(counterexample.loop_start, None);
    assert!(record.report().witness.is_none());

    let pending = world.finite(&["P"], &[0, 1, 0]);
    let [record] = pending.records.as_slice() else {
        panic!("one record");
    };
    assert_eq!(record.verdict, MonitorVerdict::Pending);
    assert_eq!(record.category, Category::Pending);
    assert_eq!(record.report().disposition.exit_code(), 0);
    let document = pending.document().expect("a document");
    assert_eq!(document.category(), Category::Pending);
    let pending_ids: Vec<_> = document.items().iter().map(OutcomeItem::clause).collect();
    assert_eq!(pending_ids, [Some("P")]);
}

/// FR-283-AC-2: over a lasso, `always eventually q` is tested when `q` holds
/// at a loop position and violated when it holds at none.
#[trace("TC-765", "FR-283-AC-2")]
#[test]
fn a_lasso_is_tested_or_violated_by_its_loop() {
    let mut world = world(&[("Q", "", ALWAYS_EVENTUALLY_THREE)]);
    let tested = world.lasso(&["Q"], &[0, 1], &[2, 3]);
    assert_eq!(tested.records[0].verdict, MonitorVerdict::Tested);
    assert_eq!(tested.records[0].category, Category::Success);
    assert_eq!(tested.records[0].report().disposition.exit_code(), 0);

    let violated = world.lasso(&["Q"], &[0, 1], &[0, 2]);
    assert_eq!(violated.records[0].verdict, MonitorVerdict::Violated);
    assert_eq!(violated.records[0].report().disposition.exit_code(), 10);
    let counterexample = violated.records[0]
        .counterexample()
        .expect("counterexample");
    assert_eq!(counterexample.loop_start, Some(2));
}

/// FR-283-AC-3: a position whose snapshot fails admission refuses the whole
/// request with its index, and a lasso with an empty loop is a trace document
/// defect.
#[trace("TC-766", "FR-283-AC-3")]
#[test]
fn an_unadmitted_position_or_an_empty_loop_refuses_the_request() {
    let mut world = world(&[("P", "", ALWAYS_SMALL)]);
    let mut prefix = world.positions("a", &[0]);
    prefix.push(json!({"identity": {"authority": "test", "identity": "never-provisioned"}}));
    let trace = json!({"format": TRACE_FORMAT, "prefix": prefix}).to_string();
    let refusal = refused(world.run(&["P"], trace.as_bytes(), UNLIMITED, &Cancel::new()));
    let MonitorRefusal::Admission { position, .. } = refusal else {
        panic!("an admission refusal, not {refusal:?}");
    };
    assert_eq!(position, 1);

    let prefix = world.positions("b", &[0]);
    let trace = json!({"format": TRACE_FORMAT, "prefix": prefix, "loop": []}).to_string();
    let refusal = refused(world.run(&["P"], trace.as_bytes(), UNLIMITED, &Cancel::new()));
    let MonitorRefusal::TraceDocument { record, pointer } = refusal else {
        panic!("a trace document refusal");
    };
    assert_eq!(record.catalog.code(), "invalid_runtime_input");
    assert_eq!(record.catalog.cause(), "invalid-value");
    assert_eq!(pointer, "/loop");
}

/// FR-283-AC-5: an undefined evaluation is a violation at the position where
/// the divisor is zero, with cause `division-by-zero` and no `undefined`
/// label.
#[trace("TC-765", "FR-283-AC-5")]
#[test]
fn an_undefined_evaluation_is_a_violation() {
    let mut world = world(&[("D", "", DIVIDES_BY_VALUE)]);
    let outcome = world.finite(&["D"], &[1, 0, 1]);
    let record = &outcome.records[0];
    assert_eq!(record.verdict, MonitorVerdict::Undefined);
    assert_eq!(record.category, Category::Violation);
    assert_eq!(record.report().disposition.exit_code(), 10);
    let ClauseDisposition::Temporal(TemporalVerdict::Undefined { at, cause }) =
        &record.report().disposition
    else {
        panic!(
            "an undefined verdict, got {:?}",
            record.report().disposition
        );
    };
    assert_eq!(*at, 1);
    assert_eq!(
        *cause,
        TemporalUndefinedCause::Kernel(quire_exact::Undefined::DivisionByZero)
    );
    assert_eq!(
        record
            .counterexample()
            .expect("counterexample")
            .trace_position
            .index(),
        1
    );
    let item = record.item(0).expect("an item");
    assert_ne!(item.result(), qsl_replay::ItemLabel::Undefined);
}

/// FR-283-AC-6: a clause with a non-empty fairness set is unsupported over a
/// supplied trace, and its neighbour still reports.
#[trace("TC-766", "FR-283-AC-6")]
#[test]
fn a_fairness_premise_settles_unsupported_beside_a_tested_clause() {
    let mut world = world(&[
        ("F", "fair weak M::Counter::inc;", ALWAYS_EVENTUALLY_THREE),
        ("Q", "", ALWAYS_EVENTUALLY_THREE),
    ]);
    let outcome = world.lasso(&["F", "Q"], &[0, 1], &[2, 3]);
    assert_eq!(
        outcome.records[0].verdict,
        MonitorVerdict::MissingFairnessPremise
    );
    assert_eq!(outcome.records[0].category, Category::Unsupported);
    assert_eq!(outcome.records[0].report().disposition.exit_code(), 21);
    let ClauseDisposition::Temporal(TemporalVerdict::MissingFairnessPremise { constraint }) =
        &outcome.records[0].report().disposition
    else {
        panic!("a missing fairness premise");
    };
    assert_eq!(constraint.kind(), FairnessKind::Weak);
    assert_eq!(constraint.granularity(), FairnessGranularity::Whole);
    assert_eq!(constraint.operation().name(), "inc");
    assert_eq!(outcome.records[1].verdict, MonitorVerdict::Tested);
}

/// FR-283-AC-8: each defective trace document refuses the whole request at
/// its first defect.
#[trace("TC-766", "FR-283-AC-8")]
#[test]
fn a_defective_trace_document_is_refused_at_its_first_defect() {
    let mut world = world(&[("P", "", ALWAYS_SMALL)]);
    let one = world.positions("d", &[0]).remove(0);
    let blank = json!({"identity": {"authority": " ", "identity": "x"}});
    let case = |bytes: Vec<u8>, code: &str, cause: &str, at: &str, field: Option<&'static str>| {
        (
            bytes,
            code.to_owned(),
            cause.to_owned(),
            at.to_owned(),
            field,
        )
    };
    let cases = vec![
        case(
            b"[".to_vec(),
            "invalid_runtime_input",
            "invalid-value",
            "",
            None,
        ),
        case(
            b"[]".to_vec(),
            "invalid_runtime_input",
            "wrong-value-kind",
            "",
            None,
        ),
        case(
            br#"{"prefix":[]}"#.to_vec(),
            "invalid_runtime_input",
            "missing-member",
            "",
            Some("format"),
        ),
        case(
            json!({"format": "quire.monitor-trace/2", "prefix": [one.clone()]})
                .to_string()
                .into_bytes(),
            "unknown_wire",
            "unsupported-wire",
            "/format",
            None,
        ),
        case(
            json!({"format": TRACE_FORMAT}).to_string().into_bytes(),
            "invalid_runtime_input",
            "missing-member",
            "",
            Some("prefix"),
        ),
        case(
            json!({"format": TRACE_FORMAT, "prefix": [one.clone()], "fairness": []})
                .to_string()
                .into_bytes(),
            "invalid_runtime_input",
            "unknown-member",
            "",
            Some("fairness"),
        ),
        case(
            json!({"format": TRACE_FORMAT, "prefix": "x"})
                .to_string()
                .into_bytes(),
            "invalid_runtime_input",
            "wrong-value-kind",
            "/prefix",
            None,
        ),
        case(
            json!({"format": TRACE_FORMAT, "prefix": [blank]})
                .to_string()
                .into_bytes(),
            "invalid_source_identity",
            "blank-label",
            "/prefix/0/identity",
            Some("authority"),
        ),
        case(
            json!({"format": TRACE_FORMAT, "prefix": []})
                .to_string()
                .into_bytes(),
            "invalid_runtime_input",
            "invalid-value",
            "/prefix",
            None,
        ),
    ];
    for (bytes, code, cause, at, field) in cases {
        let refusal = refused(world.run(&["P"], &bytes, UNLIMITED, &Cancel::new()));
        let MonitorRefusal::TraceDocument { record, pointer } = refusal else {
            panic!(
                "a trace document refusal for {}",
                String::from_utf8_lossy(&bytes)
            );
        };
        assert_eq!(
            (
                record.catalog.code(),
                record.catalog.cause(),
                pointer.as_str()
            ),
            (code.as_str(), cause.as_str(), at.as_str()),
            "{}",
            String::from_utf8_lossy(&bytes)
        );
        let named = record.fields.get("field").or(record.fields.get("label"));
        assert_eq!(
            named.map(String::as_str),
            field,
            "{}",
            String::from_utf8_lossy(&bytes)
        );
    }

    let bytes = json!({"format": TRACE_FORMAT, "prefix": [one]})
        .to_string()
        .into_bytes();
    let mut limits = MonitorLimits {
        observation: ObservationLimits::default(),
        normalization: ModelNormalizationLimits::default(),
        accounting: UNLIMITED,
    };
    limits.observation.document_bytes = u64::try_from(bytes.len()).expect("small") - 1;
    let request = MonitorRequest {
        clauses: vec![],
        trace: &bytes,
        snapshots: &world.documents.snapshots,
        packages: &world.documents.packages,
    };
    let Err(StageFailure::Refused(MonitorRefusal::TraceDocument { record, .. })) =
        monitor(&world.package, &request, &limits, &Cancel::new())
    else {
        panic!("the oversized document is refused");
    };
    assert_eq!(record.catalog.code(), "stage_limit_exceeded");
    assert_eq!(record.catalog.cause(), "input-bytes-exceeded");
    assert_eq!(record.fields["setting"], "observation.input_bytes");
}

/// FR-283-AC-9: one record per selected clause in request order.
#[trace("TC-765", "FR-283-AC-9")]
#[test]
fn records_come_in_request_order() {
    let mut world = world(&[("P", "", ALWAYS_SMALL), ("D", "", DIVIDES)]);
    let outcome = world.finite(&["D", "P"], &[0, 1, 2]);
    let verdicts: Vec<_> = outcome
        .records
        .iter()
        .map(|record| (record.clause.as_str(), record.verdict))
        .collect();
    assert_eq!(
        verdicts,
        [
            ("D", MonitorVerdict::Undefined),
            ("P", MonitorVerdict::Violated)
        ]
    );
}

/// FR-283-AC-9: over FR-327-AC-1's `Counter` trace (0, 1, 2),
/// `eventually[0,2] holds(c.value = 2)` is tested and `eventually[0,1]` is
/// violated at position 0.
#[trace("TC-765", "FR-283-AC-9")]
#[test]
fn a_bounded_eventually_is_tested_or_violated_over_the_counter_trace() {
    let mut world = bounded_world(&[("Reach", "", REACH_TWO), ("Short", "", REACH_ONE)]);
    let outcome = world.finite(&["Reach", "Short"], &[0, 1, 2]);
    let [reach, short] = outcome.records.as_slice() else {
        panic!("two records");
    };
    assert_eq!(reach.verdict, MonitorVerdict::Tested);
    assert_eq!(reach.category, Category::Success);
    assert_eq!(reach.report().disposition.exit_code(), 0);
    assert_eq!(short.verdict, MonitorVerdict::Violated);
    assert!(matches!(
        short.report().disposition,
        ClauseDisposition::Temporal(TemporalVerdict::Violated { at: 0 })
    ));
}

/// FR-283-AC-14: each record renders through `OutcomeItem::from_clause_run`,
/// and the document is those items in request order under the most severe
/// item's category.
#[trace("TC-765", "FR-283-AC-14")]
#[test]
fn each_record_renders_as_the_item_the_document_holds() {
    let mut world = world(&[("P", "", ALWAYS_SMALL), ("D", "", DIVIDES)]);
    let outcome = world.finite(&["D", "P"], &[0, 1, 2]);
    let items: Vec<OutcomeItem> = outcome
        .records
        .iter()
        .enumerate()
        .map(|(index, record)| record.item(index).expect("each record is an item"))
        .collect();
    let document = outcome.document().expect("a document");
    assert_eq!(document.items(), items.as_slice());
    let clauses: Vec<_> = items.iter().map(OutcomeItem::clause).collect();
    assert_eq!(clauses, [Some("D"), Some("P")]);
    assert_eq!(document.category(), Category::Violation);
    let expected = OutcomeDocument::from_clause_runs(
        Some(OutcomeStage::S6a),
        outcome
            .records
            .iter()
            .map(|record| (record.clause.as_str(), record.report())),
    )
    .expect("the same runs render");
    assert_eq!(document, expected);
}

/// The layer-6 replay of `name` over `trace`: `run_clause` compiling the
/// clause from source.
fn replayed(world: &World, name: &str, formula: &str, trace: TraceInput) -> ClauseRunReport {
    run_clause(ClauseRunRequest {
        source: ClauseRunSource::Program {
            identity: qsl_foundation::SourceIdentity::new("agent-ix", "counter"),
            path: "counter.native".to_owned(),
            bytes: unit_text(&clause_text(name, "", formula)).into_bytes(),
        },
        package_name: "package".to_owned(),
        packages: package_input([world.documents.document.as_slice()]).expect("packages"),
        dependencies: DependencyInput::default(),
        snapshots: world.documents.snapshots.clone(),
        invocations: std::collections::BTreeMap::new(),
        selection: ClauseRunSelection::Temporal {
            name: name.to_owned(),
            trace,
            over: OverObject {
                population: "ix://test/counter/counters".to_owned(),
                key: "c1".to_owned(),
            },
            ledger: None,
        },
        limits: SpineLimits::default(),
        observation_limits: ObservationLimits::default(),
        accounting: UNLIMITED,
    })
    .expect("a report")
}

/// FR-283-AC-4: the verdict, category, exit and counterexample `monitor`
/// reports equal layer-6 replay's for the AC-1 and AC-2 traces.
#[trace("TC-766", "FR-283-AC-4")]
#[test]
fn monitor_agrees_with_layer_6_replay() {
    let mut world = world(&[("P", "", ALWAYS_SMALL), ("Q", "", ALWAYS_EVENTUALLY_THREE)]);
    let mut cases: Vec<(&str, &str, TraceInput, MonitorOutcome)> = Vec::new();
    // Each case provisions its own snapshot names: a shared name would let a
    // later case overwrite an earlier one's document.
    for (case, values) in [[0, 1, 2], [0, 1, 0]].into_iter().enumerate() {
        let refs = world.refs(&format!("f{case}"), &values);
        let trace = json!({"format": TRACE_FORMAT, "prefix": json_of(&refs)}).to_string();
        let outcome = world
            .run(&["P"], trace.as_bytes(), UNLIMITED, &Cancel::new())
            .expect("the request completes");
        cases.push(("P", ALWAYS_SMALL, TraceInput::Finite(refs), outcome));
    }
    for (case, repeat) in [[2, 3], [0, 2]].into_iter().enumerate() {
        let prefix = world.refs(&format!("p{case}"), &[0, 1]);
        let repeat_refs = world.refs(&format!("l{case}"), &repeat);
        let trace = json!({"format": TRACE_FORMAT, "prefix": json_of(&prefix),
            "loop": json_of(&repeat_refs)})
        .to_string();
        let outcome = world
            .run(&["Q"], trace.as_bytes(), UNLIMITED, &Cancel::new())
            .expect("the request completes");
        cases.push((
            "Q",
            ALWAYS_EVENTUALLY_THREE,
            TraceInput::Lasso {
                prefix,
                repeat: repeat_refs,
            },
            outcome,
        ));
    }
    for (name, formula, trace, outcome) in cases {
        let replay = replayed(&world, name, formula, trace);
        let record = &outcome.records[0];
        assert_eq!(
            format!("{:?}", record.report().disposition),
            format!("{:?}", replay.disposition),
            "{name}"
        );
        assert_eq!(record.category, replay.category(), "{name}");
        assert_eq!(
            record.report().disposition.exit_code(),
            replay.disposition.exit_code(),
            "{name}"
        );
        assert_eq!(
            record.report().counterexample,
            replay.counterexample,
            "{name}"
        );
    }
}

/// FR-283-AC-12: an exhausted work budget is incomplete.
#[trace("TC-766", "FR-283-AC-12")]
#[test]
fn an_exhausted_work_budget_is_incomplete() {
    let mut world = world(&[("P", "", ALWAYS_SMALL)]);
    let prefix = world.positions("w", &[0, 1, 2]);
    let trace = json!({"format": TRACE_FORMAT, "prefix": prefix}).to_string();
    let accounting = ScalarLimits {
        work_units: 0,
        ..UNLIMITED
    };
    let outcome = world
        .run(&["P"], trace.as_bytes(), accounting, &Cancel::new())
        .expect("the request completes");
    assert_eq!(outcome.records[0].verdict, MonitorVerdict::Incomplete);
    assert_eq!(outcome.records[0].category, Category::Incomplete);
    assert_eq!(outcome.records[0].report().disposition.exit_code(), 22);
    let ClauseDisposition::Temporal(TemporalVerdict::Incomplete(incomplete)) =
        &outcome.records[0].report().disposition
    else {
        panic!("an incomplete verdict");
    };
    assert_eq!(incomplete.limit_kind.as_str(), "work_units");
}

/// FR-275: a cancelled request stops with its cause and no record.
#[test]
fn a_cancelled_request_stops_with_its_cause() {
    let mut world = world(&[("P", "", ALWAYS_SMALL)]);
    let prefix = world.positions("c", &[0]);
    let trace = json!({"format": TRACE_FORMAT, "prefix": prefix}).to_string();
    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    assert!(matches!(
        world.run(&["P"], trace.as_bytes(), UNLIMITED, &cancel),
        Err(StageFailure::Cancelled(_))
    ));
}

/// FR-283 Behavior: every position is admitted before any clause runs, so an
/// unknown first clause name does not hide a later admission failure.
#[test]
fn admission_of_every_position_precedes_any_clause_run() {
    let mut world = world(&[("P", "", ALWAYS_SMALL)]);
    let mut prefix = world.positions("a", &[0]);
    prefix.push(json!({"identity": {"authority": "test", "identity": "never-provisioned"}}));
    let trace = json!({"format": TRACE_FORMAT, "prefix": prefix}).to_string();
    let refusal = refused(world.run(&["Ghost", "P"], trace.as_bytes(), UNLIMITED, &Cancel::new()));
    assert!(
        matches!(refusal, MonitorRefusal::Admission { position: 1, .. }),
        "{refusal:?}"
    );

    let prefix = world.positions("b", &[0]);
    let trace = json!({"format": TRACE_FORMAT, "prefix": prefix}).to_string();
    let refusal = refused(world.run(&["Ghost"], trace.as_bytes(), UNLIMITED, &Cancel::new()));
    assert!(
        matches!(&refusal, MonitorRefusal::UnknownClause { name } if name == "Ghost"),
        "{refusal:?}"
    );
}

/// FR-330-AC-3: an `over` that names no object and a lasso under a bounded
/// profile are the clause's own admit refusals, not a position's admission.
#[test]
fn a_clause_refused_at_admit_is_no_position_failure() {
    let mut world = world(&[("P", "", ALWAYS_SMALL)]);
    let prefix = world.positions("o", &[0, 1]);
    let trace = json!({"format": TRACE_FORMAT, "prefix": prefix}).to_string();
    let refusal =
        refused(world.run_over(&["P"], "ghost", trace.as_bytes(), UNLIMITED, &Cancel::new()));
    let MonitorRefusal::ClauseRefused { clause, failure } = refusal else {
        panic!("a clause refusal, not {refusal:?}");
    };
    assert_eq!(clause, "P");
    let qsl_semantics::model::observation::AdmissionFailure::Refused(record) = *failure else {
        panic!("a refusal");
    };
    assert_eq!(record.catalog.cause(), "wrong-role-mapping");

    let mut bounded = bounded_world(&[("Reach", "", REACH_TWO)]);
    let head = bounded.positions("p", &[0, 1]);
    let tail = bounded.positions("l", &[2]);
    let trace = json!({"format": TRACE_FORMAT, "prefix": head, "loop": tail}).to_string();
    let refusal = refused(bounded.run(&["Reach"], trace.as_bytes(), UNLIMITED, &Cancel::new()));
    let MonitorRefusal::ClauseRefused { clause, failure } = refusal else {
        panic!("a clause refusal, not {refusal:?}");
    };
    assert_eq!(clause, "Reach");
    let qsl_semantics::model::observation::AdmissionFailure::Refused(record) = *failure else {
        panic!("a refusal");
    };
    assert_eq!(record.catalog.cause(), "invalid-value");
}

/// FR-001-AC-6: U+200B is not whitespace, so a label of it is not blank; the
/// unprovisioned snapshot it names fails admission, not the document.
#[trace("TC-766", "FR-283-AC-8")]
#[test]
fn a_zero_width_label_is_not_blank() {
    let world = world(&[("P", "", ALWAYS_SMALL)]);
    let trace = json!({"format": TRACE_FORMAT, "prefix": [
        {"identity": {"authority": "\u{200B}", "identity": "x"}}]})
    .to_string();
    let refusal = refused(world.run(&["P"], trace.as_bytes(), UNLIMITED, &Cancel::new()));
    assert!(
        matches!(refusal, MonitorRefusal::Admission { position: 0, .. }),
        "{refusal:?}"
    );
}
