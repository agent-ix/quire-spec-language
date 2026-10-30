// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1274 (FR-491-AC-1 to -5, -8; AC-6 is `clause_evaluation_root_only.rs`, AC-7 a unit test): `evaluate_clause` evaluates one
//! state clause at a kind-correct current or pre-call observation and returns
//! a typed `Completed`, `Undefined`, `Refused` or `Incomplete` result. The
//! calls and result types reach only the `qsl_replay` root (FR-491-AC-6);
//! this file names no `qsl_replay::spine` item. The unit is the
//! `example/config-version` model.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_replay::{
    evaluate_clause, AnchorKind, ClauseEvaluation, ClauseEvaluationRequest, ClauseIncomplete,
    ClauseObservation, ClauseRefusal, DependencyInput, ObservationLimits, ScalarLimits,
    SelectedAnchor, SelectedObject, SourceIdentity, SpineLimits,
};
use serde_json::{json, Value};

const PACKAGE: &str = "example/config-version";
const MODEL: &str = include_str!("../../examples/config-version/model.semantic-ir.json");

fn unit() -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         model Config = {PACKAGE:?};\n\
         invariant ParentOrder using v on Config::ConfigVersion at current {{ \
         present(self.parent) implies deref(value(self.parent)).versionNumber < self.versionNumber }}\n\
         invariant Quotient using v on Config::ConfigVersion at current {{ \
         10 mod self.versionNumber >= 0 }}\n\
         pre ParentPresent using v on Config::ConfigVersion::attemptUpdate {{ \
         present(self.parent) }}\n"
    )
}

fn population() -> String {
    format!("ix://{PACKAGE}/config_history")
}

/// A snapshot of role `role` over `(key, version, parent)` rows.
fn snapshot(role: &str, rows: &[(&str, i64, Option<&str>)]) -> Vec<u8> {
    let objects: Vec<Value> = rows
        .iter()
        .map(|(key, version, parent)| {
            let parent = match parent {
                Some(parent) => {
                    json!({"present": {"reference": {"population": population(), "key": parent}}})
                }
                None => json!({"absent": {}}),
            };
            json!({
                "key": key, "type": format!("ix://{PACKAGE}/ConfigVersion"),
                "fields": {"versionNumber": {"integer": version.to_string()}, "parent": parent},
            })
        })
        .collect();
    let mut document = json!({
        "format": "quire.state.snapshot/v1",
        "identity": {"authority": "t", "identity": format!("ix://t/{role}"),
            "revision_namespace": "ns", "revision": "1"},
        "observation": role,
        "model": {"identity": PACKAGE},
        "populations": [{"population": population(), "complete": true, "objects": objects}],
    });
    if role == "current" {
        document["anchor"] = json!({"kind": "handler", "name": "validate"});
    }
    document.to_string().into_bytes()
}

fn rows_with_parent(child_version: i64) -> Vec<(&'static str, i64, Option<&'static str>)> {
    vec![("root", 1, None), ("child", child_version, Some("root"))]
}

fn object(key: &str) -> Option<SelectedObject> {
    Some(SelectedObject {
        population: population(),
        key: key.to_owned(),
    })
}

fn current(rows: &[(&str, i64, Option<&str>)], key: &str) -> ClauseObservation {
    ClauseObservation::Current {
        snapshot: snapshot("current", rows),
        anchor: SelectedAnchor {
            kind: AnchorKind::Handler,
            name: "validate".to_owned(),
        },
        self_object: object(key),
    }
}

fn pre_call(rows: &[(&str, i64, Option<&str>)], key: &str) -> ClauseObservation {
    ClauseObservation::PreCall {
        snapshot: snapshot("pre", rows),
        self_object: object(key),
        parameters: BTreeMap::new(),
    }
}

fn accounting(work_units: u64) -> ScalarLimits {
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

fn evaluate(clause: &str, observation: ClauseObservation, work_units: u64) -> ClauseEvaluation {
    evaluate_clause(ClauseEvaluationRequest {
        source: SourceIdentity::new("a", "u", "git", "1"),
        path: "unit.native".to_owned(),
        bytes: unit().into_bytes(),
        packages: vec![MODEL.as_bytes().to_vec()],
        dependencies: DependencyInput::default(),
        clause: clause.to_owned(),
        observation,
        limits: SpineLimits::default(),
        observation_limits: ObservationLimits::default(),
        accounting: accounting(work_units),
    })
}

#[trace("TC-1274", "FR-491-AC-1")]
#[test]
fn an_invariant_completes_true_or_false_at_its_current_snapshot() {
    // ParentOrder is `parent < self`: root 1 < child 2 holds, root 1 < child 0 does not.
    let holds = evaluate(
        "ParentOrder",
        current(&rows_with_parent(2), "child"),
        1_000_000,
    );
    assert!(
        matches!(holds, ClauseEvaluation::Completed(true)),
        "{holds:?}"
    );
    let violated = evaluate(
        "ParentOrder",
        current(&rows_with_parent(0), "child"),
        1_000_000,
    );
    assert!(
        matches!(violated, ClauseEvaluation::Completed(false)),
        "{violated:?}"
    );
}

#[trace("TC-1274", "FR-491-AC-2")]
#[test]
fn a_precondition_is_evaluated_at_its_pre_call_observation() {
    let with_parent = evaluate(
        "ParentPresent",
        pre_call(&rows_with_parent(2), "child"),
        1_000_000,
    );
    assert!(
        matches!(with_parent, ClauseEvaluation::Completed(true)),
        "{with_parent:?}"
    );
    let without = evaluate(
        "ParentPresent",
        pre_call(&rows_with_parent(2), "root"),
        1_000_000,
    );
    assert!(
        matches!(without, ClauseEvaluation::Completed(false)),
        "{without:?}"
    );
}

#[trace("TC-1274", "FR-491-AC-3")]
#[test]
fn an_observation_of_the_wrong_kind_is_refused() {
    let rows = rows_with_parent(2);
    for (clause, observation) in [
        ("ParentOrder", pre_call(&rows, "child")),
        ("ParentPresent", current(&rows, "child")),
    ] {
        match evaluate(clause, observation, 1_000_000) {
            ClauseEvaluation::Refused(ClauseRefusal::WrongObservation) => {}
            other => panic!("{clause}: {other:?}"),
        }
    }
}

#[trace("TC-1274", "FR-491-AC-5")]
#[test]
fn refusals_and_exhausted_limits_have_their_own_variants() {
    let rows = rows_with_parent(2);
    let missing = evaluate("Absent", current(&rows, "child"), 1_000_000);
    assert!(
        matches!(&missing, ClauseEvaluation::Refused(ClauseRefusal::MissingClause { name }) if name == "Absent"),
        "{missing:?}"
    );
    let unreadable = evaluate(
        "ParentOrder",
        ClauseObservation::Current {
            snapshot: b"not json {".to_vec(),
            anchor: SelectedAnchor {
                kind: AnchorKind::Handler,
                name: "validate".to_owned(),
            },
            self_object: object("child"),
        },
        1_000_000,
    );
    assert!(
        matches!(
            unreadable,
            ClauseEvaluation::Refused(ClauseRefusal::UnreadableObservation)
        ),
        "{unreadable:?}"
    );
    let exhausted = evaluate("ParentOrder", current(&rows, "child"), 0);
    assert!(
        matches!(
            exhausted,
            ClauseEvaluation::Incomplete(ClauseIncomplete::Accounting(_))
        ),
        "{exhausted:?}"
    );
}

#[trace("TC-1274", "FR-491-AC-4")]
#[test]
fn a_clause_with_no_value_at_the_observation_is_undefined() {
    // `10 mod self.versionNumber` is undefined at version 0.
    let zero = [("root", 0, None)];
    let result = evaluate("Quotient", current(&zero, "root"), 1_000_000);
    assert!(
        matches!(
            result,
            ClauseEvaluation::Undefined {
                reason: "division-by-zero"
            }
        ),
        "{result:?}"
    );
    let one = [("root", 1, None)];
    let defined = evaluate("Quotient", current(&one, "root"), 1_000_000);
    assert!(
        matches!(defined, ClauseEvaluation::Completed(true)),
        "{defined:?}"
    );
}

#[trace("TC-1274", "FR-491-AC-8")]
#[test]
fn a_snapshot_over_the_observation_byte_limit_is_incomplete() {
    let rows = rows_with_parent(2);
    let size = snapshot("current", &rows).len() as u64;
    let request = |bound: u64| ClauseEvaluationRequest {
        source: SourceIdentity::new("a", "u", "git", "1"),
        path: "unit.native".to_owned(),
        bytes: unit().into_bytes(),
        packages: vec![MODEL.as_bytes().to_vec()],
        dependencies: DependencyInput::default(),
        clause: "ParentOrder".to_owned(),
        observation: current(&rows, "child"),
        limits: SpineLimits::default(),
        observation_limits: ObservationLimits::default().with_document_bytes(bound),
        accounting: accounting(1_000_000),
    };
    match evaluate_clause(request(size - 1)) {
        ClauseEvaluation::Incomplete(ClauseIncomplete::Observation { code, cause }) => {
            assert_eq!(
                (code, cause),
                ("resource_exhausted", "observation-input-bytes")
            );
        }
        other => panic!("{other:?}"),
    }
    assert!(matches!(
        evaluate_clause(request(size)),
        ClauseEvaluation::Completed(true)
    ));
}
