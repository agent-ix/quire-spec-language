// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1274 (FR-491-AC-1 to -5, -8; AC-6 is `clause_evaluation_root_only.rs`, AC-7 a unit test): `evaluate_clause` evaluates one
//! state clause at a kind-correct current or pre-call observation and returns
//! a typed `Completed`, `Undefined`, `Refused` or `Incomplete` result. The
//! calls and result types reach only the `qsl_replay` root (FR-491-AC-6);
//! this file names no `qsl_replay::spine` item. The unit is the
//! `example/config-version` model.

use std::collections::BTreeMap;

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
         invariant AllPositive using v on Config::ConfigVersion at current {{ \
         forall(o in Config::config_history : o.versionNumber > 0) }}\n\
         invariant Arith using v on Config::ConfigVersion at current {{ \
         (self.versionNumber * 3 + 7) mod 11 >= 0 and self.versionNumber + 1 > self.versionNumber }}\n\
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
        "identity": {"authority": "t", "identity": format!("ix://t/{role}")},
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
        source: SourceIdentity::new("a", "u"),
        package_name: "fixture".to_owned(),
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


#[test]
fn spike() {
    for n in [2usize, 100, 1000] {
        let mut rows: Vec<(String, i64, Option<String>)> = vec![("root".into(), 1, None)];
        for i in 1..n { rows.push((format!("k{i}"), (i as i64) + 1, Some("root".into()))); }
        let r: Vec<(&str, i64, Option<&str>)> = rows.iter().map(|(a,b,c)| (a.as_str(), *b, c.as_deref())).collect();
        let key = if n == 2 { "k1".to_string() } else { "k1".to_string() };
        let key = key.as_str();
        let rows2: Vec<(&str, i64, Option<&str>)> = if n==2 { rows_with_parent(2) } else { r };
        for clause in ["ParentOrder", "Quotient", "Arith", "AllPositive"] {
            let t = std::time::Instant::now();
            let out = evaluate(clause, current(&rows2, key), 1_000_000_000);
            eprintln!("SPIKE n={n} clause={clause} total_ms={} out={:?}", t.elapsed().as_millis(), out);
        }
    }
}
