// SPDX-License-Identifier: AGPL-3.0-or-later
//! Real EN-5 evidence crosses an operating-system process boundary and is
//! admitted using production source/model/checker code in the child.

use base64::Engine;
use std::io::Write;
use std::process::{Command, Stdio};

use ix_trace_rs::trace;
use qsl_analyze::analyze::{analyze_items, exact_probabilistic_item, Engines};
use qsl_analyze::exact_probabilistic::{ExactProbLimits, ProductRequest};
use qsl_eval::model_subject::ModelSubject;
use qsl_eval::simulation::model::{
    ExpansionLimits, ModelSimulationRequest, ModelSystem, PopulationUniverse,
};
use qsl_foundation::{RequestIndex, SourceIdentity};
use qsl_replay::spine::{self, DependencyInput, ParseRequest, SpineLimits};
use qsl_replay::{
    probability_certificate_document, write_probability_certificate, ObligationIdentity,
    ProbabilityDocumentLimits,
};
use qsl_semantics::model::intake::package_input;
use qsl_semantics::model::observation::{DocumentRef, ObservationLimits};
use quire_checked::key::DeclarationKey;
use quire_exact::Cancel;
use serde_json::{json, Value};

const DOMAIN: &[u8] = include_bytes!("../fixtures/link.semantic-ir.json");
const SNAPSHOT: &[u8] = include_bytes!("../fixtures/link.snapshot.json");
const SOURCE: &str = include_str!("../fixtures/link.native");
const CLAIM: &str = "\nprobabilistic Likely using inf on M over (m: M::Msg) under every scheduler on origin { probability >= 0.95 [ eventually holds(m.delivered) ] }\n";

fn proof_request(claim: &str) -> Value {
    proof_request_with(claim, |_, _, _, _, _| {})
}

fn proof_request_with(claim: &str, inspect: impl FnOnce(&ModelSystem<'_>, &[qsl_semantics::check::CheckedProbabilisticClaim], &ObligationIdentity, &[u8], ProbabilityDocumentLimits)) -> Value {
    let source = format!("{SOURCE}{claim}");
    let identity = SourceIdentity::new("test", "certificate-source");
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let packages = package_input([DOMAIN]).unwrap();
    let parsed = spine::parse(
        &ParseRequest {
            source: &identity,
            path: "certificate.native",
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
        "certificate-fixture".to_owned(),
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
    let snapshots = [(
        DocumentRef {
            authority: "test".to_owned(),
            identity: "s0".to_owned(),
        },
        SNAPSHOT.to_vec(),
    )]
    .into_iter()
    .collect();
    let initial = vec![DocumentRef {
        authority: "test".to_owned(),
        identity: "s0".to_owned(),
    }];
    let universes = vec![PopulationUniverse {
        population: DeclarationKey {
            package: "test/link".to_owned(),
            node: "ix://test/link/msgs".to_owned(),
        },
        keys: vec!["m".to_owned()],
    }];
    let simulation = ModelSimulationRequest {
        package,
        packages: &packages,
        normalization: limits.model,
        snapshots: &snapshots,
        initial,
        universes,
        observation: ObservationLimits::default(),
        expansion: ExpansionLimits {
            max_candidates: 100_000,
        },
        meter: qsl_replay::AccountingLimits::default().0,
        emit: limits.package.emit(),
    };
    let system = ModelSystem::new(simulation.clone()).unwrap();
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
                limits: ExactProbLimits::default(),
            },
        )],
        &cancel,
    );
    let record = records.remove(0);
    let evidence = record
        .exact_probabilistic
        .as_ref()
        .expect("a real EN-5 proof carries its checked certificate");
    let item = ObligationIdentity::new(claim.occurrence().unwrap().node().clone(), None, 0);
    let document_limits = ProbabilityDocumentLimits {
        bytes: 1_000_000,
        storage_bytes: 8_000_000,
        states: 100,
        edges: 1000,
        scalar_bits: 128,
        accounting: qsl_replay::AccountingLimits::default().0,
    };
    let document = probability_certificate_document(
        &system,
        package.graph().probabilistic_model().claims(),
        &item,
        &evidence.certificate,
        document_limits,
        &cancel,
    )
    .unwrap();
    let certificate = write_probability_certificate(&document, document_limits, &cancel).unwrap();
    inspect(&system, package.graph().probabilistic_model().claims(), &item, &certificate, document_limits);
    json!({
        "source": {"authority": identity.authority, "identity": identity.identity, "content": source},
        "package_name": "certificate-fixture",
        "item": {"node_id": qsl_foundation::node_wire::node_key_to_json(item.node()).unwrap(), "model": null, "ordinal": 0},
        "byte_provision": [["test", "certificate-source", source.as_bytes()]],
        "documents": [DOMAIN, SNAPSHOT],
        "initial": [{"authority":"test", "identity":"s0"}],
        "universes": [{"population":{"package":"test/link", "node":"ix://test/link/msgs"}, "keys":["m"]}],
        "certificate": String::from_utf8(certificate).unwrap(),
        "limits": {"bytes":1_000_000,"storage_bytes":8_000_000, "states":100, "edges":1000, "scalar_bits":128, "expansion":100_000, "stages":{}},
    })
}

#[trace("TC-764", "FR-282-AC-12", "FR-201-AC-11")]
#[test]
fn cancelling_at_every_observed_document_graph_and_arithmetic_poll_returns_no_acceptance() {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{Arc, OnceLock};
    use qsl_replay::{check_probability_certificate_document, ExactProbabilityCertificateCheck, ProbabilityDocumentError};
    use quire_exact::CancelCause;

    proof_request_with(CLAIM, |system, claims, item, bytes, limits| {
        let check = |cancel: &Cancel| check_probability_certificate_document(system, claims, item, bytes, limits, cancel);
        let live = Cancel::new();
        let _counting = live.count_charges();
        assert!(matches!(check(&live).unwrap(), ExactProbabilityCertificateCheck::Accepted(_)));
        let total = live.charges();
        assert!(total > 0);
        for cause in [CancelCause::Requested, CancelCause::Deadline] {
            for at in 1..=total {
                let slot = Arc::new(OnceLock::<Cancel>::new());
                let weak = Arc::downgrade(&slot);
                let seen = Arc::new(AtomicU32::new(0));
                let observed = Arc::clone(&seen);
                let cancel = Cancel::observing(move || {
                    if observed.fetch_add(1, Ordering::Relaxed) + 1 == at {
                        weak.upgrade().unwrap().get().unwrap().cancel(cause);
                    }
                });
                slot.set(cancel.clone()).unwrap();
                assert_eq!(check(&cancel), Err(ProbabilityDocumentError::Cancelled(cause)), "poll {at}/{total}");
                assert_eq!(seen.load(Ordering::Relaxed), at);
            }
        }
    });
}

fn run(request: &Value, arguments: &[&str]) -> (i32, Value) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_qsl-probability-check"))
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let input = serde_json::to_vec(request).unwrap();
    if let Err(error) = child.stdin.take().unwrap().write_all(&input) {
        assert_eq!(
            error.kind(),
            std::io::ErrorKind::BrokenPipe,
            "the bounded child may close its input early"
        );
    }
    let output = child.wait_with_output().unwrap();
    let result = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|_| panic!("child output: {:?}", output));
    (output.status.code().unwrap(), result)
}

fn change_certificate(request: &mut Value, change: impl FnOnce(&mut Value)) {
    let mut document: Value =
        serde_json::from_str(request["certificate"].as_str().unwrap()).unwrap();
    change(&mut document);
    request["certificate"] = Value::String(serde_json::to_string(&document).unwrap());
}

fn alter_keys(document: &mut Value, change: impl Fn(&mut Value)) {
    let alter = |text: &mut Value| {
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(text.as_str().unwrap())
            .unwrap();
        let mut key: Value = serde_json::from_slice(&bytes).unwrap();
        change(&mut key);
        *text = json!(base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(&key).unwrap()));
    };
    for row in document["values"].as_array_mut().unwrap() {
        alter(&mut row["state"]);
    }
    for row in document["ranking"].as_array_mut().unwrap() {
        alter(&mut row["state"]);
    }
    for target in document["objective"]["targets"].as_array_mut().unwrap() {
        alter(target);
    }
}

#[trace("TC-764", "FR-282-AC-1", "FR-282-AC-11", "FR-201-AC-9", "FR-201-AC-11")]
#[test]
fn an_actual_en5_proof_is_checked_in_a_fresh_process_using_its_source() {
    let request = proof_request(CLAIM);
    let (code, result) = run(&request, &[]);
    assert_eq!(code, 0, "{result}");
    assert_eq!(result["status"], "accepted");
    assert_eq!(
        result["value"],
        json!({"numerator":"24", "denominator":"25"})
    );
}

#[trace("TC-764", "FR-282-AC-1", "FR-282-AC-2", "FR-282-AC-11", "FR-201-AC-10")]
#[test]
fn a_fresh_checker_refuses_altered_values_objectives_identities_ranks_and_provisions() {
    let original = proof_request(CLAIM);
    let mut cases = Vec::new();
    let mut changed = original.clone();
    change_certificate(&mut changed, |document| {
        document["values"][0]["value"] = json!({"numerator":"1", "denominator":"2"});
    });
    cases.push(changed);
    let mut changed = original.clone();
    change_certificate(&mut changed, |document| {
        document["objective"]["targets"] = json!([]);
    });
    cases.push(changed);
    let mut changed = original.clone();
    change_certificate(&mut changed, |document| {
        document["objective"]["optimization"] = json!("maximum");
    });
    cases.push(changed);
    let mut changed = original.clone();
    change_certificate(&mut changed, |document| {
        document["identity"]["ordinal"] = json!(1);
    });
    cases.push(changed);
    let mut changed = original.clone();
    change_certificate(&mut changed, |document| {
        for row in document["ranking"].as_array_mut().unwrap() {
            row["rank"] = json!(0);
        }
    });
    cases.push(changed);
    let mut changed = original.clone();
    changed["byte_provision"] = json!([]);
    cases.push(changed);
    let mut changed = original.clone();
    changed["documents"] = json!([DOMAIN]);
    cases.push(changed);
    let mut changed = original.clone();
    changed["source"]["content"] = json!("different admitted source");
    cases.push(changed);
    let mut changed = original.clone();
    change_certificate(&mut changed, |document| {
        alter_keys(document, |key| {
            key[1] = Value::Null;
        })
    });
    cases.push(changed);
    let mut changed = original.clone();
    change_certificate(&mut changed, |document| {
        alter_keys(document, |key| {
            key[1]["identity"] = json!("foreign-object");
        })
    });
    cases.push(changed);
    let mut changed = original.clone();
    change_certificate(&mut changed, |document| {
        alter_keys(document, |key| {
            key[3] = json!("foreign-monitor");
        })
    });
    cases.push(changed);
    let mut changed = original.clone();
    change_certificate(&mut changed, |document| {
        document["ranking"].as_array_mut().unwrap().remove(0);
    });
    cases.push(changed);
    for request in cases {
        let (code, result) = run(&request, &[]);
        assert_ne!(code, 0, "corrupted evidence must not pass: {result}");
        assert_ne!(result["status"], "accepted");
        assert!(result.get("value").is_none());
    }
}

#[trace("TC-764", "FR-201-AC-11", "FR-282-AC-1", "FR-282-AC-12")]
#[test]
fn child_limits_and_cancellation_produce_no_truth_value() {
    let original = proof_request(CLAIM);
    for name in [
        "bytes",
        "storage_bytes",
        "states",
        "edges",
        "scalar_bits",
        "expansion",
    ] {
        let mut request = original.clone();
        request["limits"][name] = json!(0);
        let (code, result) = run(&request, &[]);
        assert_ne!(code, 0, "{name}: {result}");
        assert_ne!(result["status"], "accepted");
        assert!(result.get("value").is_none());
    }
    for setting in [
        "s1.work_units",
        "s1.input_bytes",
        "s1.tokens",
        "s1.nodes",
        "accounting.result_units",
        "accounting.work_units",
    ] {
        let mut request = original.clone();
        request["limits"]["stages"][setting] = json!(0);
        let (code, result) = run(&request, &[]);
        assert_ne!(code, 0, "{setting}: {result}");
        assert!(result.get("value").is_none());
    }
    let (code, result) = run(&original, &["--cancel"]);
    assert_ne!(code, 0, "{result}");
    assert!(result.get("value").is_none());
    let (code, result) = run(&original, &["--max-input-bytes", "1"]);
    assert_ne!(code, 0, "{result}");
    assert!(result.get("value").is_none());
}

#[trace("TC-764", "FR-282-AC-12")]
#[test]
fn exact_byte_ceilings_retain_the_actual_offered_lengths() {
    let original = proof_request(CLAIM);
    let mut request = original.clone();
    let certificate_bytes = request["certificate"].as_str().unwrap().len();
    request["limits"]["bytes"] = json!(certificate_bytes - 1);
    let (code, result) = run(&request, &[]);
    assert_eq!(code, 2);
    assert_eq!(result["limit"], json!({"setting":"certificate.bytes", "bound":certificate_bytes - 1, "actual":certificate_bytes}));
    let mut request = original.clone();
    let source_bytes = request["source"]["content"].as_str().unwrap().len();
    request["limits"]["stages"]["s1.input_bytes"] = json!(source_bytes - 1);
    let (code, result) = run(&request, &[]);
    assert_eq!(code, 2);
    assert_eq!(result["limit"], json!({"setting":"s1.input_bytes", "bound":source_bytes - 1, "actual":source_bytes}));
    let mut request = original;
    request["limits"]["storage_bytes"] = json!(0);
    let (code, result) = run(&request, &[]);
    assert_eq!(code, 2);
    assert_eq!(result["limit"]["setting"], "certificate.storage_bytes");
    assert_eq!(result["limit"]["bound"], 0);
    assert!(result["limit"]["actual"].as_u64().unwrap() > 0);
}
