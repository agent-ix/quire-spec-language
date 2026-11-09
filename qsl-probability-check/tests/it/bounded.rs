// SPDX-License-Identifier: AGPL-3.0-or-later
//! Real bounded EN-5 documents checked by a separate qualified-core process.

use std::io::Write;
use std::process::{Command, Stdio};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ix_trace_rs::trace;
use qsl_analyze::analyze::{analyze_items, exact_probabilistic_item, Engines};
use qsl_analyze::exact_probabilistic::{ExactProbLimits, ProductRequest};
use qsl_eval::model_subject::ModelSubject;
use qsl_eval::simulation::model::{ExpansionLimits, ModelSimulationRequest, PopulationUniverse};
use qsl_foundation::{RequestIndex, SourceIdentity};
use qsl_replay::spine::{self, DependencyInput, ParseRequest, SpineLimits};
use qsl_semantics::model::intake::package_input;
use qsl_semantics::model::observation::{DocumentRef, ObservationLimits};
use quire_checked::key::DeclarationKey;
use quire_exact::Cancel;
use serde_json::{json, Value};

const DOMAIN: &[u8] = include_bytes!("../fixtures/link.semantic-ir.json");
const SNAPSHOT: &[u8] = include_bytes!("../fixtures/link.snapshot.json");
const SOURCE: &str = include_str!("../fixtures/link.native");
const PROFILE: &str = "profile bounded = \"quire.temporal.event-position.false-extension/v1\";\n";

fn claim(scheduler: &str, operator: &str, threshold: &str, horizon: u64) -> String {
    format!("{PROFILE}probabilistic Likely using bounded on M over (m: M::Msg) under {scheduler} on origin {{ probability {operator} {threshold} [ eventually[0,{horizon}] holds(m.delivered) ] }}\n")
}

fn request(
    source: &str,
    snapshot: &[u8],
    exact: ExactProbLimits,
) -> (Option<Value>, qsl_replay::TerminalValue) {
    let identity = SourceIdentity::new("test", "bounded-certificate-source");
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let packages = package_input([DOMAIN]).unwrap();
    let parsed = spine::parse(
        &ParseRequest {
            source: &identity,
            path: "bounded.native",
            bytes: source.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .unwrap()
    .into_value();
    let models = spine::select(&parsed, &packages, limits.model, &cancel)
        .unwrap()
        .into_value();
    let checked = spine::check(
        &parsed,
        "bounded-fixture".to_owned(),
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .unwrap()
    .into_value();
    let package = checked.package();
    let claim = package
        .graph()
        .probabilistic_model()
        .claim("Likely")
        .unwrap();
    let document = DocumentRef {
        authority: "test".to_owned(),
        identity: "s0".to_owned(),
    };
    let snapshots = [(document.clone(), snapshot.to_vec())]
        .into_iter()
        .collect();
    let simulation = ModelSimulationRequest {
        package,
        packages: &packages,
        normalization: limits.model,
        snapshots: &snapshots,
        initial: vec![document],
        universes: vec![PopulationUniverse {
            population: DeclarationKey {
                package: "test/link".to_owned(),
                node: "ix://test/link/msgs".to_owned(),
            },
            keys: vec!["m".to_owned()],
        }],
        observation: ObservationLimits::default(),
        expansion: ExpansionLimits {
            max_candidates: 100_000,
        },
        meter: qsl_replay::AccountingLimits::default().0,
        emit: limits.package.emit(),
    };
    let engines = Engines::qsl().unwrap();
    let mut records = analyze_items(
        &engines,
        vec![exact_probabilistic_item(
            RequestIndex::new(0),
            ProductRequest {
                subject: ModelSubject {
                    simulation,
                    proof_bounds: Vec::new(),
                },
                claim,
                limits: exact,
            },
        )],
        &cancel,
    );
    let record = records.remove(0);
    let terminal = record.terminal().expect("bounded analysis settles");
    let value = terminal.value().clone();
    let Some(bytes) = terminal.exact_certificate_document() else {
        return (None, value);
    };
    let item = qsl_replay::probabilistic_claim_identity(claim).unwrap();
    let request = json!({
        "source": {"authority":identity.authority, "identity":identity.identity, "content":source},
        "package_name":"bounded-fixture",
        "item":{"node_id":qsl_foundation::node_wire::node_key_to_json(item.node()).unwrap(), "model":item.model().map(|model|model.as_str()), "ordinal":item.ordinal()},
        "byte_provision":[["test","bounded-certificate-source",source.as_bytes()]],
        "documents":[DOMAIN,snapshot],
        "initial":[{"authority":"test","identity":"s0"}],
        "universes":[{"population":{"package":"test/link","node":"ix://test/link/msgs"},"keys":["m"]}],
        "certificate":String::from_utf8(bytes.to_vec()).unwrap(),
        "limits":{"bytes":1_000_000,"storage_bytes":8_000_000,"states":100,"edges":1000,"scalar_bits":128,"expansion":100_000,"stages":{}},
    });
    (Some(request), value)
}

fn proof(source: &str, snapshot: &[u8]) -> Value {
    let (request, value) = request(source, snapshot, ExactProbLimits::default());
    request.unwrap_or_else(|| panic!("bounded exact proof carried no document: {value:?}"))
}

fn run(request: &Value) -> (i32, Value) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_qsl-probability-check"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(request).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    let value = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|_| panic!("child output: {output:?}"));
    (output.status.code().unwrap(), value)
}

fn document(request: &Value) -> Value {
    serde_json::from_str(request["certificate"].as_str().unwrap()).unwrap()
}

fn key(text: &Value) -> Value {
    serde_json::from_slice(&URL_SAFE_NO_PAD.decode(text.as_str().unwrap()).unwrap()).unwrap()
}

fn mutate(request: &Value, change: impl FnOnce(&mut Value)) -> Value {
    let mut changed = request.clone();
    let mut doc = document(request);
    change(&mut doc);
    changed["certificate"] = json!(serde_json::to_string(&doc).unwrap());
    changed
}

fn alter_keys(document: &mut Value, change: impl Fn(&mut Value)) {
    let alter = |text: &mut Value| {
        let mut state = key(text);
        change(&mut state);
        *text = json!(URL_SAFE_NO_PAD.encode(serde_json::to_vec(&state).unwrap()));
    };
    for member in ["values", "ranking"] {
        let rows = document[member].as_array_mut().unwrap();
        for row in rows.iter_mut() {
            alter(&mut row["state"]);
        }
        rows.sort_by_key(|row| {
            URL_SAFE_NO_PAD
                .decode(row["state"].as_str().unwrap())
                .unwrap()
        });
    }
    let targets = document["objective"]["targets"].as_array_mut().unwrap();
    for target in targets.iter_mut() {
        alter(target);
    }
    targets.sort_by_key(|target| URL_SAFE_NO_PAD.decode(target.as_str().unwrap()).unwrap());
}

#[trace("TC-636", "FR-201-AC-12", "FR-282-AC-13")]
#[test]
fn bounded_deliver_exact_minimum_maximum_and_workload_cross_the_process_boundary() {
    for (scheduler, operator, threshold, beyond, numerator, denominator, loss_n, loss_d) in [
        (
            "every scheduler",
            ">=",
            "0.96",
            "0.97",
            "24",
            "25",
            "4",
            "5",
        ),
        (
            "every scheduler",
            "<=",
            "0.99",
            "0.98",
            "99",
            "100",
            "9",
            "10",
        ),
        ("Even", ">=", "0.9775", "0.98", "391", "400", "17", "20"),
    ] {
        let source = format!("{SOURCE}{}", claim(scheduler, operator, threshold, 2));
        let request = proof(&source, SNAPSHOT);
        let (code, result) = run(&request);
        assert_eq!(code, 0, "{scheduler}: {result}");
        assert_eq!(
            result["value"],
            json!({"numerator":numerator,"denominator":denominator})
        );
        let doc = document(&request);
        let loss = doc["ranking"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["rank"] == 1)
            .unwrap();
        let value = doc["values"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["state"] == loss["state"])
            .unwrap();
        assert_eq!(
            value["value"],
            json!({"numerator":loss_n,"denominator":loss_d})
        );
        let mut stronger = request.clone();
        let source = source.replace(threshold, beyond);
        stronger["source"]["content"] = json!(source);
        stronger["byte_provision"][0][2] = json!(source.as_bytes());
        let (code, result) = run(&stronger);
        assert_eq!(code, 1, "{result}");
        assert_eq!(result["rule"], "threshold-missed");
    }
}

fn cyclic_source(horizon: u64, threshold: &str) -> String {
    let model = SOURCE
        .replace(" and self.attempts < 2", "")
        .replace(
            "self.attempts - pre(self.attempts) = 1",
            "self.attempts = pre(self.attempts)",
        )
        .replace("true: 1, false: 9", "true: 1, false: 1")
        .replace("true: 1, false: 4", "true: 1, false: 1")
        .replace("self.delivered or self.attempts = 2", "self.delivered");
    format!(
        "{model}{}",
        claim("every scheduler", ">=", threshold, horizon)
    )
}

#[trace("TC-636", "FR-201-AC-13", "FR-282-AC-13")]
#[test]
fn bounded_raw_cycle_has_distinct_real_horizons_and_exact_ranks() {
    let request = proof(&cyclic_source(2, "0.75"), SNAPSHOT);
    let (code, result) = run(&request);
    assert_eq!(code, 0, "{result}");
    assert_eq!(result["value"], json!({"numerator":"3","denominator":"4"}));
    let doc = document(&request);
    let running: Vec<_> = doc["ranking"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| key(&row["state"])[3][1][2][0] == "bounded")
        .collect();
    assert_eq!(running.len(), 2);
    let first = key(&running[0]["state"]);
    let second = key(&running[1]["state"]);
    assert_eq!(
        first[2], second[2],
        "raw model state is the same on both visits"
    );
    assert_ne!(first[3], second[3]);
    let mut ranks: Vec<_> = running
        .iter()
        .map(|row| row["rank"].as_u64().unwrap())
        .collect();
    ranks.sort();
    assert_eq!(ranks, vec![1, 2]);
}

#[trace("TC-636", "FR-201-AC-13", "FR-282-AC-13")]
#[test]
fn bounded_zero_horizon_reads_position_zero_and_terminal_closes_by_false_extension() {
    for delivered in [false, true] {
        let mut snapshot: Value = serde_json::from_slice(SNAPSHOT).unwrap();
        snapshot["populations"][0]["objects"][0]["fields"]["delivered"]["boolean"] =
            json!(delivered);
        let source = cyclic_source(0, if delivered { "1" } else { "0" });
        let request = proof(&source, &serde_json::to_vec(&snapshot).unwrap());
        let (code, result) = run(&request);
        assert_eq!(code, 0, "{result}");
        assert_eq!(
            result["value"],
            json!({"numerator":if delivered {"1"} else {"0"},"denominator":"1"})
        );
    }
    let source = cyclic_source(2, "0").replace("when { self.delivered }", "when { true }");
    let request = proof(&source, SNAPSHOT);
    let (code, result) = run(&request);
    assert_eq!(code, 0, "{result}");
    assert_eq!(result["value"], json!({"numerator":"0","denominator":"1"}));
}

#[trace("TC-636", "FR-201-AC-14", "FR-282-AC-13")]
#[test]
fn bounded_altered_horizon_binding_and_rank_never_certify_the_original_claim() {
    let request = proof(&cyclic_source(2, "0.75"), SNAPSHOT);
    let changed = mutate(&request, |doc| {
        let row = doc["ranking"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|row| row["rank"] == 2)
            .unwrap();
        row["rank"] = json!(3);
    });
    let (code, result) = run(&changed);
    assert_eq!(code, 1, "{result}");
    assert_eq!(result["rule"], "rank-broken");
    for alteration in ["horizon", "binding", "local-id", "absent"] {
        let changed = mutate(&request, |doc| {
            alter_keys(doc, |state| {
                if state[3][1][2][0] == "bounded" {
                    match alteration {
                        "binding" => state[3][1][1][3] = json!([]),
                        "horizon" => state[3][1][2][1][1] = json!("9"),
                        "local-id" => state[3][1] = json!(1),
                        "absent" => state[3] = Value::Null,
                        _ => unreachable!("closed test cases"),
                    }
                }
            });
        });
        let (code, result) = run(&changed);
        assert_ne!(code, 0, "{result}");
    }
}

#[trace("TC-636", "FR-201-AC-14", "FR-282-AC-13")]
#[test]
fn bounded_wrong_source_and_nonexact_rows_never_produce_an_accepted_document() {
    let mut original = proof(&cyclic_source(2, "0.75"), SNAPSHOT);
    original["package_name"] = json!("another-source-package");
    let (code, result) = run(&original);
    assert_ne!(code, 0, "{result}");
    for (threshold, precision, maximum, incomplete) in
        [("0.5", 64, 64, false), ("0.75", 1, 1, true)]
    {
        let mut limits = ExactProbLimits::default();
        limits.max_rational_bits = 1;
        limits.precision_bits = precision;
        limits.max_precision_bits = maximum;
        let (document, terminal) = request(&cyclic_source(2, threshold), SNAPSHOT, limits);
        assert!(
            document.is_none(),
            "interval rows must never become exact evidence"
        );
        if incomplete {
            assert!(
                matches!(
                    terminal,
                    qsl_replay::TerminalValue::Incomplete(
                        qsl_replay::IncompleteCause::PrecisionBudget { .. }
                    )
                ),
                "{terminal:?}"
            );
        } else {
            assert!(
                matches!(terminal, qsl_replay::TerminalValue::Unsupported(_)),
                "{terminal:?}"
            );
        }
    }
}
