// SPDX-License-Identifier: AGPL-3.0-or-later
//! Real bounded EN-5 documents checked by a separate qualified-core process.

use std::io::Write;
use std::process::{Command, Stdio};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ix_trace_rs::trace;
use qsl_analyze::analyze::{
    analyze_items, exact_probabilistic_item, DispositionKind, Engines, Settlement,
};
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

fn model_source() -> String {
    SOURCE.replace(
        "quire.temporal.infinite-trace/v1",
        "quire.temporal.event-position.false-extension/v1",
    )
}

fn claim(scheduler: &str, operator: &str, threshold: &str, horizon: u64) -> String {
    format!("probabilistic Likely using inf on M over (m: M::Msg) under {scheduler} on origin {{ probability {operator} {threshold} [ eventually[0,{horizon}] holds(m.delivered) ] }}\n")
}

pub(super) fn compile_source(
    source: &str,
) -> (
    spine::CheckedUnit,
    qsl_semantics::model::intake::PackageInput,
) {
    compile_source_with(source, DOMAIN)
}

pub(super) fn compile_source_with(
    source: &str,
    domain: &[u8],
) -> (
    spine::CheckedUnit,
    qsl_semantics::model::intake::PackageInput,
) {
    let identity = SourceIdentity::new("test", "bounded-certificate-source");
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let packages = package_input([domain]).unwrap();
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
    (checked, packages)
}

pub(super) fn request(
    source: &str,
    snapshot: &[u8],
    exact: ExactProbLimits,
) -> (Option<Value>, Settlement) {
    request_with(source, snapshot, exact, DOMAIN)
}

pub(super) fn request_with(
    source: &str,
    snapshot: &[u8],
    exact: ExactProbLimits,
    domain: &[u8],
) -> (Option<Value>, Settlement) {
    let identity = SourceIdentity::new("test", "bounded-certificate-source");
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let (checked, packages) = compile_source_with(source, domain);
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
    let system = qsl_eval::simulation::model::ModelSystem::new(simulation.clone()).unwrap();
    let binding = qsl_replay::claim_binding(&system, claim).unwrap().unwrap();
    if matches!(
        claim.form,
        qsl_semantics::check::ProbForm::Probability { .. }
    ) {
        let context =
            qsl_replay::probabilistic_formula_monitor_context(claim, Some(&binding.reference))
                .unwrap();
        assert_eq!(
            context.binding[0].parameter,
            *claim.over.as_ref().unwrap().node().unwrap()
        );
        assert!(
            matches!(&context.binding[0].value, qsl_eval::automaton::ContextBindingValue::Instance(key) if key == &binding.instance.key)
        );
    }
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
    let Some(terminal) = record.terminal() else {
        return (None, record.settlement);
    };
    if terminal.value() == &qsl_replay::TerminalValue::Refuted {
        let token = record
            .probabilistic_counterexample
            .as_ref()
            .expect("a native bounded refutation retains its reproduced path evidence");
        let evidence = token.evidence();
        assert_eq!(
            evidence.item,
            qsl_replay::probabilistic_claim_identity(claim).unwrap()
        );
        let qsl_replay::ProbabilityWitnessReplay::Reproduced(replayed) =
            qsl_replay::replay_probabilistic_witness(
                package,
                &system,
                &evidence.item,
                evidence,
                &exact,
                &cancel,
            )
            .expect("the retained native evidence independently replays")
        else {
            panic!("native refutation evidence must reproduce");
        };
        assert_eq!(replayed.mass(), token.mass());
    }
    let Some(bytes) = terminal.exact_certificate_document() else {
        return (None, record.settlement);
    };
    let item = qsl_replay::probabilistic_claim_identity(claim).unwrap();
    let request = json!({
        "source": {"authority":identity.authority, "identity":identity.identity, "content":source},
        "package_name":"bounded-fixture",
        "item":{"node_id":qsl_foundation::node_wire::node_key_to_json(item.node()).unwrap(), "model":item.model().map(|model|model.as_str()), "ordinal":item.ordinal()},
        "byte_provision":[["test","bounded-certificate-source",source.as_bytes()]],
        "documents":[domain,snapshot],
        "initial":[{"authority":"test","identity":"s0"}],
        "universes":[{"population":{"package":"test/link","node":"ix://test/link/msgs"},"keys":["m"]}],
        "certificate":String::from_utf8(bytes.to_vec()).unwrap(),
        "limits":{"bytes":1_000_000,"storage_bytes":8_000_000,"states":100,"edges":1000,"scalar_bits":128,"expansion":100_000,"stages":{}},
    });
    (Some(request), record.settlement)
}

pub(super) fn proof(source: &str, snapshot: &[u8]) -> Value {
    let (request, value) = request(source, snapshot, ExactProbLimits::default());
    request.unwrap_or_else(|| {
        panic!(
            "{}: bounded exact proof carried no document: {value:?}",
            source.lines().last().unwrap_or(source)
        )
    })
}

pub(super) fn run(request: &Value) -> (i32, Value) {
    run_with(request, &[])
}

pub(super) fn run_with(request: &Value, arguments: &[&str]) -> (i32, Value) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_qsl-probability-check"))
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Err(error) = child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(request).unwrap())
    {
        assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
    }
    let output = child.wait_with_output().unwrap();
    let value = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|_| panic!("child output: {output:?}"));
    (output.status.code().unwrap(), value)
}

pub(super) fn document(request: &Value) -> Value {
    serde_json::from_str(request["certificate"].as_str().unwrap()).unwrap()
}

pub(super) fn key(text: &Value) -> Value {
    serde_json::from_slice(&URL_SAFE_NO_PAD.decode(text.as_str().unwrap()).unwrap()).unwrap()
}

#[trace("TC-636", "FR-201-AC-14")]
#[test]
fn bounded_claim_over_is_a_real_s3_parameter_with_its_actual_model_type() {
    use qsl_semantics::check::{BodyTerm, LeafTerm, MemberTerm, NodeTag, Owner};
    let (checked, _) = compile_source(&cyclic_source(2, "0.75"));
    let package = checked.package();
    let claim = package
        .graph()
        .probabilistic_model()
        .claim("Likely")
        .unwrap();
    let parameter = claim.over.as_ref().unwrap().node().unwrap();
    let graph = package.graph().semantic_graph();
    let node = graph
        .node(parameter.clone())
        .expect("S3 retains the real parameter node");
    assert_eq!(node.node_tag(), NodeTag::Value);
    assert_eq!(node.semantic_form(), "parameter");
    assert_ne!(parameter, claim.occurrence().unwrap().node());
    let reference_type = graph.node(node.semantic_type().unwrap()).unwrap();
    assert_eq!(reference_type.semantic_form(), "reference");
    let BodyTerm::Aggregate(body) = reference_type.body() else {
        panic!("reference type has an aggregate body");
    };
    let [MemberTerm::Leaf(LeafTerm::Reference { target })] = body.members.as_slice() else {
        panic!("reference type names its model object");
    };
    let object = graph.node(target.admitted().unwrap().clone()).unwrap();
    let Some(Owner::Model(owner)) = object.owner() else {
        panic!("object type keeps its actual model owner");
    };
    assert_eq!(owner.identity(), "test/link");
    assert_eq!(owner.node(), "ix://test/link/Msg");
    let root = graph
        .node(claim.occurrence().unwrap().node().clone())
        .unwrap();
    assert!(matches!(root.body(), BodyTerm::Aggregate(body) if body.members.is_empty()));
}

pub(super) fn mutate(request: &Value, change: impl FnOnce(&mut Value)) -> Value {
    let mut changed = request.clone();
    let mut doc = document(request);
    change(&mut doc);
    changed["certificate"] = json!(serde_json::to_string(&doc).unwrap());
    changed
}

pub(super) fn alter_keys(document: &mut Value, change: impl Fn(&mut Value)) {
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
    for member in ["targets", "failures"] {
        if let Some(targets) = document["objective"][member].as_array_mut() {
            for target in targets.iter_mut() {
                alter(target);
            }
            targets.sort_by_key(|target| URL_SAFE_NO_PAD.decode(target.as_str().unwrap()).unwrap());
        }
    }
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
        let source = format!(
            "{}{}",
            model_source(),
            claim(scheduler, operator, threshold, 2)
        );
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
    let model = model_source()
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
        let source = if delivered {
            cyclic_source(0, "0.5")
        } else {
            cyclic_source(0, "0.5").replace("probability >= 0.5", "probability <= 0.5")
        };
        let mut request = proof(&source, &serde_json::to_vec(&snapshot).unwrap());
        request["limits"]["states"] = json!(1);
        let (code, result) = run(&request);
        assert_eq!(code, 0, "{result}");
        assert_eq!(
            result["value"],
            json!({"numerator":if delivered {"1"} else {"0"},"denominator":"1"})
        );
    }
    let source = cyclic_source(2, "0.5")
        .replace("when { self.delivered }", "when { true }")
        .replace("{ not self.delivered }", "{ false }")
        .replace("probability >= 0.5", "probability <= 0.5");
    let request = proof(&source, SNAPSHOT);
    let doc = document(&request);
    let mut ranks: Vec<_> = doc["ranking"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["rank"].as_u64().unwrap())
        .collect();
    ranks.sort();
    assert_eq!(
        ranks,
        vec![0, 2],
        "terminal closure keeps real debt rather than graph height"
    );
    let (code, result) = run(&request);
    assert_eq!(code, 0, "{result}");
    assert_eq!(result["value"], json!({"numerator":"0","denominator":"1"}));
}

#[trace(
    "TC-636",
    "FR-201-AC-13",
    "FR-201-AC-14",
    "FR-282-AC-12",
    "FR-282-AC-13"
)]
#[test]
fn bounded_graph_and_document_budget_stops_retain_limits_without_truth_or_evidence() {
    let original = proof(&cyclic_source(2, "0.75"), SNAPSHOT);
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
        let (code, result) = run(&request);
        assert_ne!(code, 0, "{name}: {result}");
        assert!(result.get("value").is_none(), "{name}: {result}");
    }
    let count = document(&original)["values"].as_array().unwrap().len();
    let mut exact = original.clone();
    exact["limits"]["states"] = json!(count);
    let (code, result) = run(&exact);
    assert_eq!(
        code, 0,
        "the exact product state ceiling suffices: {result}"
    );
    let mut limited = mutate(&original, |document| {
        document["values"].as_array_mut().unwrap().pop();
        document["ranking"].as_array_mut().unwrap().pop();
    });
    limited["limits"]["states"] = json!(count - 1);
    let (code, result) = run(&limited);
    assert_ne!(code, 0, "{result}");
    assert_eq!(result["limit"]["setting"], "exact_prob.max_states");
    assert_eq!(result["limit"]["bound"], json!(count - 1));
    assert_eq!(result["limit"]["actual"], json!(count));
    for arguments in [vec!["--cancel"], vec!["--max-input-bytes", "1"]] {
        let (code, result) = run_with(&original, &arguments);
        assert_ne!(code, 0, "{result}");
        assert!(result.get("value").is_none(), "{result}");
    }
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
    let (evidence, missed) = request(
        &cyclic_source(2, "0.9"),
        SNAPSHOT,
        ExactProbLimits::default(),
    );
    assert!(
        evidence.is_none(),
        "a replayed bounded refutation is not a proof certificate document"
    );
    assert!(
        matches!(&missed, Settlement::Terminal(terminal) if terminal.value() == &qsl_replay::TerminalValue::Refuted),
        "{missed:?}"
    );
    let mut original = proof(&cyclic_source(2, "0.75"), SNAPSHOT);
    original["package_name"] = json!("another-source-package");
    let (code, result) = run(&original);
    assert_ne!(code, 0, "{result}");
    for (threshold, precision, maximum, incomplete) in
        [("0.5", 64, 64, false), ("0.75", 1, 1, true)]
    {
        let limits = ExactProbLimits {
            max_rational_bits: 1,
            precision_bits: precision,
            max_precision_bits: maximum,
            ..ExactProbLimits::default()
        };
        let (document, terminal) = request(&cyclic_source(2, threshold), SNAPSHOT, limits);
        assert!(
            document.is_none(),
            "interval rows must never become exact evidence"
        );
        if incomplete {
            assert!(
                matches!(&terminal, Settlement::Terminal(record) if matches!(record.value(), qsl_replay::TerminalValue::Incomplete(qsl_replay::IncompleteCause::PrecisionBudget { .. }))),
                "{terminal:?}"
            );
        } else {
            assert!(
                matches!(&terminal, Settlement::Disposed(disposed) if disposed.kind == DispositionKind::Unsupported),
                "{terminal:?}"
            );
        }
    }
}
