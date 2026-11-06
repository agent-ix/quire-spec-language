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
use qsl_replay::spine::OverObject;
use qsl_replay::{OutcomeDocument, OutcomeStage};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::observation::ObservationLimits;
use quire_exact::{Cancel, CancelCause, ScalarLimits};
use quire_semantic_value::catalog::Category;
use serde_json::{json, Value as Json};

use crate::counter::{document, package_with_clauses, Documents, UNLIMITED};

const ALWAYS_SMALL: &str = "always holds(c.value < 2)";
const ALWAYS_EVENTUALLY_THREE: &str = "always eventually holds(c.value = 3)";
const DIVIDES: &str = "always holds(3 mod (2 - c.value) >= 0)";

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

impl World {
    /// The `DocumentRef` JSON of one snapshot per value, provisioned here.
    fn positions(&mut self, tag: &str, values: &[i64]) -> Vec<Json> {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                let reference = self
                    .documents
                    .start(&format!("{tag}-{index}"), &[("c1", *value)]);
                json!({"identity": {"authority": reference.authority,
                    "identity": reference.identity}})
            })
            .collect()
    }

    fn run(
        &self,
        names: &[&str],
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
                        key: "c1".to_owned(),
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
    assert_eq!(counterexample.prefix.len(), 3);
    assert_eq!(counterexample.loop_start, None);
    assert!(record.report().witness.is_none());

    let pending = world.finite(&["P"], &[0, 1, 0]);
    let [record] = pending.records.as_slice() else {
        panic!("one record");
    };
    assert_eq!(record.verdict, MonitorVerdict::Pending);
    assert_eq!(record.category, Category::Pending);
    assert_eq!(record.report().disposition.exit_code(), 0);
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

/// FR-283-AC-5: an undefined evaluation is a violation with no `undefined`
/// label.
#[trace("TC-765", "FR-283-AC-5")]
#[test]
fn an_undefined_evaluation_is_a_violation() {
    let mut world = world(&[("D", "", DIVIDES)]);
    let outcome = world.finite(&["D"], &[0, 1, 2]);
    let record = &outcome.records[0];
    assert_eq!(record.verdict, MonitorVerdict::Undefined);
    assert_eq!(record.category, Category::Violation);
    assert_eq!(record.report().disposition.exit_code(), 10);
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
    let cases: Vec<(Vec<u8>, &str, &str, &str)> = vec![
        (b"[".to_vec(), "invalid_runtime_input", "invalid-value", ""),
        (
            b"[]".to_vec(),
            "invalid_runtime_input",
            "wrong-value-kind",
            "",
        ),
        (
            br#"{"prefix":[]}"#.to_vec(),
            "invalid_runtime_input",
            "missing-member",
            "",
        ),
        (
            json!({"format": "quire.monitor-trace/2", "prefix": [one.clone()]})
                .to_string()
                .into_bytes(),
            "unknown_wire",
            "unsupported-wire",
            "/format",
        ),
        (
            json!({"format": TRACE_FORMAT}).to_string().into_bytes(),
            "invalid_runtime_input",
            "missing-member",
            "",
        ),
        (
            json!({"format": TRACE_FORMAT, "prefix": [one.clone()], "fairness": []})
                .to_string()
                .into_bytes(),
            "invalid_runtime_input",
            "unknown-member",
            "",
        ),
        (
            json!({"format": TRACE_FORMAT, "prefix": "x"})
                .to_string()
                .into_bytes(),
            "invalid_runtime_input",
            "wrong-value-kind",
            "/prefix",
        ),
        (
            json!({"format": TRACE_FORMAT, "prefix": [blank]})
                .to_string()
                .into_bytes(),
            "invalid_source_identity",
            "blank-label",
            "/prefix/0/identity",
        ),
        (
            json!({"format": TRACE_FORMAT, "prefix": []})
                .to_string()
                .into_bytes(),
            "invalid_runtime_input",
            "invalid-value",
            "/prefix",
        ),
    ];
    for (bytes, code, cause, at) in cases {
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
            (code, cause, at),
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

/// FR-283-AC-9, AC-14: one record per selected clause in request order, and
/// the records render to the document `OutcomeDocument::from_clause_runs`
/// builds over the same runs.
#[trace("TC-765", "FR-283-AC-9")]
#[trace("TC-765", "FR-283-AC-14")]
#[test]
fn records_come_in_request_order_and_render_through_the_clause_run_items() {
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
    let document = outcome.document().expect("a document");
    let expected = OutcomeDocument::from_clause_runs(
        Some(OutcomeStage::S6a),
        outcome
            .records
            .iter()
            .map(|record| (record.clause.as_str(), record.report())),
    )
    .expect("the same runs render");
    assert_eq!(
        document.to_bytes().expect("bytes"),
        expected.to_bytes().expect("bytes")
    );
    for (index, record) in outcome.records.iter().enumerate() {
        record.item(index).expect("each record renders as an item");
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
    assert_eq!(outcome.records[0].report().disposition.exit_code(), 22);
}

/// FR-283-AC-13: a cancelled request stops with its cause and no record.
#[trace("TC-766", "FR-283-AC-13")]
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
