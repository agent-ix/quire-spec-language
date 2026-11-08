// SPDX-License-Identifier: AGPL-3.0-or-later
//! Public compile/run/replay qualification adapters, independent of catalog counts.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use ix_trace_rs::trace;
use serde_json::{json, Value};
use tempfile::TempDir;

const COUNTER: &str = "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\nfunction digits using v(x: Int[0,9], y: Int[0,9]): Integer pure { x * 10 + y }\n";
const FALSE: &str = "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\nfunction negative using v(x: Int[0,9]): Boolean pure { x < 0 }\n";

struct Fixture {
    _temp: TempDir,
    corpus: PathBuf,
    qspec: PathBuf,
    value: Value,
    expected_output: Option<Vec<u8>>,
    native_replay: Option<qsl_replay::ReplayRequestWire>,
}

fn qspec_repository() -> PathBuf {
    std::env::var_os("QSPEC_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .ancestors()
                .find_map(|parent| {
                    let candidate = parent.join("quire-specification");
                    candidate.is_dir().then_some(candidate)
                })
                .expect("a QSpec checkout or QSPEC_DIR")
        })
}

fn published(path: &str) -> Vec<u8> {
    fs::read(qspec_repository().join(path)).expect("read the selected QSpec contract")
}

impl Fixture {
    fn new(subject: &str, operation: &str, source: &str, expected: Value) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let corpus = temp.path().join("corpus");
        let qspec = temp.path().join("qspec");
        fs::create_dir_all(corpus.join("inputs")).unwrap();
        for path in [
            "proposals/conformance-corpus-v1/schema.json",
            "proposals/checked-package-v2/schema.json",
        ] {
            let destination = qspec.join(path);
            fs::create_dir_all(destination.parent().unwrap()).unwrap();
            fs::write(destination, published(path)).unwrap();
        }
        fs::write(corpus.join("inputs/unit.native"), source).unwrap();
        let value = json!({
            "contract_version":"quire.conformance-corpus/v1",
            "vectors":[{"id":"V1-TOOL-004-operation", "capability_id":"V1-TOOL-004", "class":"positive", "subject":{"consumer_type":subject,"operation":operation}, "applicability":{"consumer_types":[subject]}, "input":{"files":[{"kind":"source","path":"inputs/unit.native"}]}, "expected":expected}],
            "results":[], "coverage":{"primary":[{"capability_id":"V1-TOOL-004","applicability":{"consumer_types":[subject],"semantic_profiles":["quire.value.complete/v1"]}}],"criteria_rollups":[],"consumer_rollups":[]}
        });
        Self {
            _temp: temp,
            corpus,
            qspec,
            value,
            expected_output: None,
            native_replay: None,
        }
    }

    fn write(&self) {
        fs::write(
            self.corpus.join("complete-v1.json"),
            serde_json_canonicalizer::to_vec(&self.value).unwrap(),
        )
        .unwrap();
    }

    fn file(&mut self, kind: &str, path: &str, bytes: &[u8]) {
        fs::write(self.corpus.join(path), bytes).unwrap();
        self.value["vectors"][0]["input"]["files"]
            .as_array_mut()
            .unwrap()
            .push(json!({"kind":kind,"path":path}));
    }

    fn execute(&self, extra: &[&str]) -> xtask::qualification::Execution {
        self.write();
        let mut args = vec![
            OsString::from("--corpus"),
            self.corpus.clone().into_os_string(),
        ];
        args.extend(extra.iter().map(OsString::from));
        xtask::qualification::execute_with_qspec(&self.qspec, &args)
            .expect("the fixture corpus is valid")
    }

    fn run_call(&mut self, arguments: Value, accounting: Value) {
        self.file(
            "call",
            "inputs/call.json",
            &serde_json::to_vec(
                &json!({"function":"digits", "arguments":arguments,"accounting":accounting}),
            )
            .unwrap(),
        );
    }
}

fn report(execution: &xtask::qualification::Execution) -> Value {
    serde_json::from_str(&execution.outcome.stdout).unwrap()
}

#[test]
#[trace("TC-875", "FR-350-AC-1", "FR-350-AC-8", "FR-350-AC-5")]
fn runtime_vectors_bind_positional_values_and_retain_real_outcome_bytes() {
    let expected = json!({"disposition":"value","value":{"kind":"completed","value":{"kind":"integer","decimal":"12"}}});
    let mut fixture = Fixture::new("runtime", "run", COUNTER, expected);
    fixture.run_call(
        json!([{"kind":"integer","decimal":"1"},{"kind":"integer","decimal":"2"}]),
        json!({}),
    );
    let first = fixture.execute(&[]);
    assert_eq!(first.outcome.code, 0, "{}", first.outcome.stdout);
    let document: Value = serde_json::from_slice(&first.outputs["V1-TOOL-004-operation"]).unwrap();
    assert_eq!(document["result"]["value"]["decimal"], "12");
    assert_eq!(document["category"], "success");
    let second = fixture.execute(&[]);
    assert_eq!(first.outputs, second.outputs);
    assert_eq!(first.outcome.stdout, second.outcome.stdout);
    assert!(
        report(&first).get("outputs").is_none(),
        "operation bytes are not an invented FR-352 member"
    );

    fixture.value["vectors"][0]["expected"]["value"]["value"]["decimal"] = json!("21");
    let wrong = fixture.execute(&[]);
    assert_eq!(wrong.outcome.code, 1);
    assert_eq!(
        report(&wrong)["capabilities"][0]["vectors"][0]["actual"]["value"]["decimal"],
        "12"
    );
    assert!(
        wrong.outputs.is_empty(),
        "failed vectors have no passed output"
    );
}

#[test]
#[trace("TC-875", "FR-350-AC-3", "FR-350-AC-5")]
fn runtime_limits_remain_incomplete_and_are_fresh_for_siblings() {
    let expected = json!({"disposition":"value","value":{"kind":"completed","value":{"kind":"integer","decimal":"12"}}});
    let mut fixture = Fixture::new("runtime", "run", COUNTER, expected);
    fixture.run_call(
        json!([{"kind":"integer","decimal":"1"},{"kind":"integer","decimal":"2"}]),
        json!({"work_units":"0"}),
    );
    let incomplete = fixture.execute(&[]);
    let actual = report(&incomplete);
    assert_eq!(
        actual["capabilities"][0]["outcome"], "incomplete",
        "{actual}"
    );
    let limit = &actual["capabilities"][0]["vectors"][0]["actual"]["limit"];
    assert!(
        limit.is_object(),
        "the public incomplete payload survives: {actual}"
    );
    let raised = fixture.execute(&["--limit", "accounting.work_units=100"]);
    assert_eq!(raised.outcome.code, 0, "{}", raised.outcome.stdout);
}

#[test]
#[trace("TC-875", "FR-350-AC-19")]
fn compiler_vectors_admit_the_actual_model_documents() {
    let source = include_str!("../../tests/fixtures/spine-model.native");
    let model = include_bytes!("../../tests/fixtures/spine-model.semantic-ir.json");
    let expected = qsl_replay::compile_package(
        qsl_replay::CompileUnit {
            source: qsl_foundation::SourceIdentity::new("agent-ix", "inputs/unit.native"),
            package_name: "qualification".into(),
            path: "inputs/unit.native",
            bytes: source.as_bytes(),
        },
        [model.as_slice()],
        &qsl_replay::DependencyInput::default(),
        qsl_replay::StageLimits::default(),
        qsl_replay::ReplayLimits::default(),
    )
    .expect("real model-backed compiler fixture");
    let mut fixture = Fixture::new(
        "language",
        "compile",
        source,
        json!({"disposition":"value","value":serde_json::from_slice::<Value>(expected.bytes()).unwrap()}),
    );
    fixture.file("model", "inputs/model.json", model);
    let admitted = fixture.execute(&[]);
    assert_eq!(admitted.outcome.code, 0, "{}", admitted.outcome.stdout);
    assert_eq!(admitted.outputs["V1-TOOL-004-operation"], expected.bytes());
    fixture.value["vectors"][0]["input"]["files"]
        .as_array_mut()
        .unwrap()
        .retain(|file| file["kind"] != "model");
    let absent = fixture.execute(&[]);
    let result = report(&absent);
    assert_eq!(absent.outcome.code, 1);
    assert_eq!(
        result["capabilities"][0]["vectors"][0]["actual"]["code"], "missing_import",
        "{result}"
    );
}

#[test]
#[trace("TC-875", "FR-350-AC-8")]
fn malformed_calls_and_excess_arguments_are_corpus_failures() {
    let mut fixture = Fixture::new(
        "runtime",
        "run",
        COUNTER,
        json!({"disposition":"value","value":null}),
    );
    fixture.run_call(json!([{"kind":"integer","decimal":"1"},{"kind":"integer","decimal":"2"},{"kind":"integer","decimal":"3"}]), json!({}));
    let excess = fixture.execute(&[]);
    let actual = report(&excess);
    assert_eq!(
        actual["capabilities"][0]["outcome"], "tool-failure",
        "{actual}"
    );
    assert_eq!(
        actual["capabilities"][0]["vectors"][0]["actual"]["pointer"],
        "/arguments/2"
    );
    fs::write(
        fixture.corpus.join("inputs/call.json"),
        br#"{"function":"digits","arguments":[],"accounting":{},"unknown":0}"#,
    )
    .unwrap();
    let malformed = fixture.execute(&[]);
    assert_eq!(
        report(&malformed)["capabilities"][0]["vectors"][0]["actual"]["cause"],
        "corpus-decode"
    );
}

fn replay_fixture() -> Fixture {
    replay_fixture_source(FALSE)
}

fn replay_fixture_source(source: &str) -> Fixture {
    let mut fixture = Fixture::new(
        "runtime",
        "replay",
        source,
        json!({"disposition":"value","value":null}),
    );
    let selected = qsl_replay::call_site(
        qsl_foundation::SourceIdentity::new("agent-ix", "inputs/unit.native"),
        "qualification".into(),
        "inputs/unit.native",
        source.as_bytes(),
        [],
        &qsl_replay::DependencyInput::default(),
        &qsl_replay::QualifiedName::new(vec![qsl_replay::Identifier::new("negative").unwrap()])
            .unwrap(),
    )
    .unwrap();
    let graph = qsl_replay::WireGraph::of_package_bytes(&selected.package).unwrap();
    let node = |key| graph.wire_value(key).unwrap();
    let limits = qsl_replay::AccountingLimits::default()
        .bounds()
        .into_iter()
        .map(|(setting, bound)| {
            (
                setting
                    .name()
                    .strip_prefix("accounting.")
                    .unwrap()
                    .to_owned(),
                json!(bound.to_string()),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    let request = json!({
        "package":serde_json::from_slice::<Value>(&selected.package).unwrap(),"profile_selections":[],"dependencies":[],
        "replay_target":{"members":{"function":"negative"},"arguments":false,"occurrence_key":{"node_id":node(&selected.site.function),"role":selected.site.declaration.origin().role().as_str(),"ordinal":selected.site.declaration.origin().ordinal().to_string()},"over":null},
        "source":{"kind":"input","assignments":[{"parameter":node(&selected.site.parameters[0].1),"value":{"kind":"integer","decimal":"1"}}]},"obligation_identity":{"node_id":node(selected.site.obligation_identity().node()),"model":selected.site.obligation_identity().model().map(|model|model.as_str()),"ordinal":selected.site.obligation_identity().ordinal()},"backend":"fixture", "state_environment":[],"accounting_limits":limits,"stage_limits":{},"supplied_limits":null,"object_environment":null,"declared_domains":[],"byte_provision":[{"authority":"agent-ix","identity":"inputs/unit.native","bytes_path":"inputs/bytes.native"}]
    });
    let native_request = qsl_replay::ReplayRequestWire {
        package: qsl_replay::ReplayPackage::descriptor_only(
            &"qualification".to_owned(),
            &vec![("agent-ix".to_owned(), "inputs/unit.native".to_owned())],
            None,
        ),
        profile_selections: Vec::new(),
        dependencies: Vec::new(),
        replay_target: qsl_replay::RequestTargetWire::function(
            qsl_replay::QualifiedName::new(vec![qsl_replay::Identifier::new("negative").unwrap()])
                .unwrap(),
            selected.site.declaration.clone(),
        ),
        source: qsl_replay::ReplaySource::Input(vec![qsl_replay::CanonicalAssignment {
            parameter: selected.site.parameters[0].1.clone(),
            value: qsl_replay::WitnessValue::Integer(1),
        }]),
        obligation_identity: selected.site.obligation_identity().clone(),
        backend: "fixture".to_owned(),
        state_environment: qsl_replay::StateEnvironment::new(Vec::new()),
        accounting_limits: qsl_replay::AccountingLimits::default().0,
        stage_limits: Default::default(),
        supplied_limits: None,
        object_environment: None,
        declared_domains: Vec::new(),
        byte_provision: vec![(
            "agent-ix".to_owned(),
            "inputs/unit.native".to_owned(),
            source.as_bytes().to_vec(),
        )],
        documents: Vec::new(),
    };
    let native = qsl_replay::replay(native_request.clone(), qsl_replay::ReplayLimits::default())
        .expect("the actual public function replay completes");
    let qsl_replay::ReplayResult::Input(input) = &native else {
        panic!("an input replay");
    };
    if source == FALSE {
        assert_eq!(
            input.value(),
            Some(qsl_replay::EvaluatedValue::Boolean(false))
        );
    }
    let row: Value = serde_json::from_str(&native.to_function_json().unwrap()).unwrap();
    fixture.value["vectors"][0]["expected"] =
        json!({"disposition":"value", "value":{"results":[row]}});
    fixture.expected_output = Some(
        qsl_replay::OutcomeDocument::from_replay(&Ok(native))
            .to_bytes()
            .unwrap(),
    );
    fixture.native_replay = Some(native_request);
    fixture.value["vectors"][0]["input"]["files"] = json!([]);
    fixture.file(
        "replay-request",
        "inputs/request.json",
        &serde_json::to_vec(&request).unwrap(),
    );
    fixture.file("bytes", "inputs/bytes.native", source.as_bytes());
    fixture
}

#[test]
#[trace("TC-1325", "FR-350-AC-11", "FR-350-AC-12", "FR-350-AC-16")]
fn replay_vectors_decode_and_execute_without_a_source_file_kind() {
    let fixture = replay_fixture();
    let result = fixture.execute(&[]);
    let actual = report(&result);
    assert_eq!(result.outcome.code, 0, "{actual}");
    assert_eq!(
        result.outputs["V1-TOOL-004-operation"],
        *fixture.expected_output.as_ref().unwrap()
    );
    assert_eq!(result.outputs, fixture.execute(&[]).outputs);
}

#[test]
#[trace(
    "TC-1325",
    "FR-350-AC-14",
    "FR-350-AC-16",
    "FR-350-AC-17",
    "FR-350-AC-18"
)]
fn replay_corruption_is_an_attributed_tool_failure() {
    let fixture = replay_fixture();
    let path = fixture.corpus.join("inputs/request.json");
    let original: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    for (pointer, value, expected) in [
        ("/package", Value::Null, "/package"),
        (
            "/package/contract_version",
            json!("quire.checked-package/v1"),
            "/package/contract_version",
        ),
        (
            "/byte_provision/0/bytes_path",
            json!("../outside"),
            "/byte_provision/0/bytes_path",
        ),
        (
            "/stage_limits",
            json!({"depth":"1024"}),
            "/stage_limits/depth",
        ),
    ] {
        let mut mutated = original.clone();
        *mutated.pointer_mut(pointer).unwrap() = value;
        fs::write(&path, serde_json::to_vec(&mutated).unwrap()).unwrap();
        let result = report(&fixture.execute(&[]));
        let actual = &result["capabilities"][0]["vectors"][0]["actual"];
        assert_eq!(actual["cause"], "corpus-decode", "{pointer}: {result}");
        assert_eq!(actual["pointer"], expected, "{pointer}: {result}");
    }
}

#[test]
#[trace("TC-875", "FR-350-AC-3", "FR-350-AC-19")]
fn compiler_limits_preserve_the_stage_setting_and_refusal_partition() {
    let compiled = qsl_replay::compile_package(
        qsl_replay::CompileUnit {
            source: qsl_foundation::SourceIdentity::new("agent-ix", "inputs/unit.native"),
            package_name: "qualification".to_owned(),
            path: "inputs/unit.native",
            bytes: COUNTER.as_bytes(),
        },
        [],
        &qsl_replay::DependencyInput::default(),
        qsl_replay::StageLimits::default(),
        qsl_replay::ReplayLimits::default(),
    )
    .unwrap();
    let fixture = Fixture::new(
        "language",
        "compile",
        COUNTER,
        json!({"disposition":"value","value":serde_json::from_slice::<Value>(compiled.bytes()).unwrap()}),
    );
    let limited = report(&fixture.execute(&["--limit", "s3.nodes=0"]));
    assert_eq!(
        limited["capabilities"][0]["outcome"], "incomplete",
        "{limited}"
    );
    let actual = &limited["capabilities"][0]["vectors"][0]["actual"];
    assert_eq!(actual["code"], "stage_limit_exceeded");
    assert!(
        serde_json::to_string(actual).unwrap().contains("s3.nodes"),
        "the typed stage setting survives: {actual}"
    );
    assert_eq!(fixture.execute(&[]).outcome.code, 0);
    let exact = compiled.bytes().len();
    assert_eq!(
        fixture
            .execute(&["--limit", &format!("emit.output_bytes={exact}")])
            .outcome
            .code,
        0
    );
    let one_short_execution =
        fixture.execute(&["--limit", &format!("emit.output_bytes={}", exact - 1)]);
    assert!(one_short_execution.outputs.is_empty());
    let one_short = report(&one_short_execution);
    assert_eq!(
        one_short["capabilities"][0]["outcome"], "incomplete",
        "{one_short}"
    );
    assert_eq!(
        one_short["capabilities"][0]["vectors"][0]["actual"]["fields"]["setting"],
        "emit.output_bytes"
    );
    let mut runtime = Fixture::new(
        "runtime",
        "run",
        COUNTER,
        json!({"disposition":"value", "value":{"kind":"completed", "value":{"kind":"integer", "decimal":"12"}}}),
    );
    runtime.run_call(
        json!([{"kind":"integer", "decimal":"1"},{"kind":"integer", "decimal":"2"}]),
        json!({}),
    );
    let runtime_limited =
        runtime.execute(&["--limit", &format!("emit.output_bytes={}", exact - 1)]);
    assert!(runtime_limited.outputs.is_empty());
    let runtime_limit = report(&runtime_limited);
    assert_eq!(
        runtime_limit["capabilities"][0]["outcome"], "incomplete",
        "{runtime_limit}"
    );
    assert_eq!(
        runtime_limit["capabilities"][0]["vectors"][0]["actual"]["fields"]["setting"],
        "emit.output_bytes"
    );
    assert_eq!(
        runtime
            .execute(&["--limit", &format!("emit.output_bytes={exact}")])
            .outcome
            .code,
        0
    );
    assert_eq!(runtime.execute(&[]).outcome.code, 0);
}

#[test]
#[trace("TC-875", "FR-350-AC-4", "FR-350-AC-5", "FR-350-AC-8")]
fn runtime_vector_failures_do_not_change_sibling_budgets_or_source_bytes() {
    let expected = json!({"disposition":"value","value":{"kind":"completed","value":{"kind":"integer","decimal":"12"}}});
    let mut fixture = Fixture::new("runtime", "run", COUNTER, expected);
    fixture.run_call(
        json!([{"kind":"integer","decimal":"1"},{"kind":"integer","decimal":"2"}]),
        json!({}),
    );
    let required = (1..=100)
        .find(|bound| {
            fixture
                .execute(&["--limit", &format!("accounting.work_units={bound}")])
                .outcome
                .code
                == 0
        })
        .expect("the real call has a finite bound");
    let first = fixture.value["vectors"][0].clone();
    let mut sibling = first.clone();
    sibling["id"] = json!("V1-TOOL-004-sibling");
    let mut absent = first.clone();
    absent["id"] = json!("V1-TOOL-004-missing");
    absent["input"]["files"][0]["path"] = json!("inputs/missing.native");
    fixture.value["vectors"] = json!([first, absent, sibling]);
    let source = fs::read(fixture.corpus.join("inputs/unit.native")).unwrap();
    let limits = format!("accounting.work_units={required}");
    let result = fixture.execute(&["--limit", &limits]);
    let report_value = report(&result);
    assert_eq!(report_value["counts"]["tool-failure"], 1, "{report_value}");
    assert_eq!(
        result.outputs.len(),
        2,
        "both siblings receive the complete budget: {report_value}"
    );
    assert_eq!(
        result.outputs["V1-TOOL-004-operation"],
        result.outputs["V1-TOOL-004-sibling"]
    );
    fixture.value["vectors"].as_array_mut().unwrap().reverse();
    let reversed = fixture.execute(&["--limit", &limits]);
    assert_eq!(reversed.outputs, result.outputs);
    assert_eq!(reversed.outcome.stdout, result.outcome.stdout);
    assert_eq!(
        fs::read(fixture.corpus.join("inputs/unit.native")).unwrap(),
        source
    );
}

#[test]
#[trace("TC-875", "FR-350-AC-1", "FR-350-AC-5")]
fn another_consumers_open_operation_is_filtered_without_opening_its_input() {
    let expected = json!({"disposition":"value","value":{"kind":"completed","value":{"kind":"integer","decimal":"12"}}});
    let mut fixture = Fixture::new("runtime", "run", COUNTER, expected);
    fixture.run_call(
        json!([{"kind":"integer","decimal":"1"},{"kind":"integer","decimal":"2"}]),
        json!({}),
    );
    let mut other = fixture.value["vectors"][0].clone();
    other["id"] = json!("V1-TOOL-004-monitor");
    other["subject"] = json!({"consumer_type":"monitor", "operation":"observe"});
    other["applicability"]["consumer_types"] = json!(["monitor"]);
    other["input"]["files"] = json!([{"kind":"input", "path":"inputs/not-opened.json"}]);
    fixture.value["vectors"].as_array_mut().unwrap().push(other);
    fixture.value["coverage"]["primary"][0]["applicability"]["consumer_types"] =
        json!(["runtime", "monitor"]);
    let executed = fixture.execute(&[]);
    assert_eq!(executed.outcome.code, 0, "{}", executed.outcome.stdout);
    assert_eq!(executed.outputs.len(), 1);
}

fn model_population_fixture() -> Fixture {
    let source = "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.model.complete/v1\";\nmodel M = \"acme/orders\";\nfunction seen using v(p: Population<M::Gadget>[1], ok: Boolean): Boolean pure { ok }\n";
    let mut model: Value = serde_json::from_slice(include_bytes!(
        "../../tests/fixtures/spine-model.semantic-ir.json"
    ))
    .unwrap();
    model["constructs"].as_array_mut().unwrap().push(json!({
        "kind":{"module":"acme/orders", "name":"population"}, "moduleVersion":"1.0.0",
        "construct":{"identity":"none", "shape":"record", "members":{}, "meaning":"quire.meaning.model.population/v1"}
    }));
    model["populations"] = json!([{
        "identity":"ix://acme/orders/Fleet", "displayName":"Fleet", "kind":{"module":"acme/orders", "name":"population"},
        "members":["ix://acme/orders/Gadget"], "extent":"closed",
        "origin":{"generated":{"generatorIdentity":"ix://acme/orders/Fleet", "generatorVersion":"1.0.0", "inputIdentities":["ix://acme/orders/Fleet"]}}
    }]);
    let mut fixture = Fixture::new(
        "runtime",
        "run",
        source,
        json!({"disposition":"value", "value":{"kind":"completed", "value":{"kind":"boolean", "value":true}}}),
    );
    fixture.file(
        "model",
        "inputs/model.json",
        &serde_json::to_vec(&model).unwrap(),
    );
    fixture.file("population", "inputs/population.json", &serde_json::to_vec(&json!({
        "format":"quire.state.snapshot/v1", "identity":{"authority":"fixture", "identity":"current"}, "observation":"current",
        "model":{"identity":"acme/orders", "version":"1.0.0"},
        "populations":[{"population":"ix://acme/orders/Fleet", "complete":true,
            "objects":[{"key":"g1", "type":"ix://acme/orders/Gadget", "fields":{"code":{"integer":"7"}}}]}]
    })).unwrap());
    fixture.file("call", "inputs/call.json", &serde_json::to_vec(&json!({
        "function":"seen", "arguments":[{"kind":"population", "name":"ix://acme/orders/Fleet"},{"kind":"boolean", "value":true}], "accounting":{}
    })).unwrap());
    fixture
}

#[test]
#[trace("TC-875", "FR-350-AC-8", "FR-350-AC-9")]
fn population_arguments_use_the_actual_admitted_model_binding() {
    let fixture = model_population_fixture();
    let result = fixture.execute(&[]);
    assert_eq!(result.outcome.code, 0, "{}", result.outcome.stdout);
    let call_path = fixture.corpus.join("inputs/call.json");
    let mut call: Value = serde_json::from_slice(&fs::read(&call_path).unwrap()).unwrap();
    call["arguments"][0]["name"] = json!("ix://foreign/Fleet");
    fs::write(&call_path, serde_json::to_vec(&call).unwrap()).unwrap();
    let refused = report(&fixture.execute(&[]));
    assert_eq!(
        refused["capabilities"][0]["outcome"], "incomplete",
        "{refused}"
    );
    assert_eq!(
        refused["capabilities"][0]["vectors"][0]["actual"]["cause"],
        "missing-required-artifact"
    );
}

#[test]
#[trace("TC-1325", "FR-350-AC-3", "FR-350-AC-10", "FR-350-AC-11")]
fn replay_pre_call_limits_keep_the_full_owner_context_and_zero_evaluation_charges() {
    let mut fixture = replay_fixture();
    let path = fixture.corpus.join("inputs/request.json");
    let mut request: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let supplied =
        quire_semantic_value::supplied_stop::SuppliedLimits::default().with_admission_work_units(0);
    request["supplied_limits"] = json!({
        "admission_work_units":"0", "conversion_work_units":supplied.conversion_work_units().to_string(),
        "numeric_magnitude_bits":supplied.numeric_magnitude_bits().to_string()
    });
    fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    let mut direct = fixture.native_replay.as_ref().unwrap().clone();
    direct.supplied_limits = Some(supplied);
    let native = qsl_replay::replay(direct, qsl_replay::ReplayLimits::default()).unwrap();
    let stop = native
        .pre_call_limit()
        .expect("the native caller budget stops before evaluation");
    assert_eq!(
        stop.context.phase,
        quire_semantic_value::supplied_stop::SuppliedPhase::Admission
    );
    let expected: Value = serde_json::from_str(&native.to_function_json().unwrap()).unwrap();
    let mismatch = report(&fixture.execute(&[]));
    assert_eq!(
        mismatch["capabilities"][0]["outcome"], "incomplete",
        "{mismatch}"
    );
    let row = &mismatch["capabilities"][0]["vectors"][0]["actual"]["results"][0];
    assert_eq!(row["supplied"]["phase"], "admission");
    assert_eq!(row["supplied"]["consumed"], "0");
    assert_eq!(row["supplied"]["cause"]["kind"], "limit");
    assert_eq!(
        row["supplied"]["cause"]["field"],
        "supplied.admission_work_units"
    );
    assert_eq!(row["supplied"]["cause"]["limit"], "0");
    assert_eq!(row["charges"]["work_units"], "0");
    assert!(row["value"].is_null());
    assert!(
        row.get("limit").is_none(),
        "pre-call stops have no evaluated charge point"
    );
    fixture.value["vectors"][0]["expected"] =
        json!({"disposition":"value", "value":{"results":[expected]}});
    assert_eq!(
        fixture.execute(&[]).outcome.code,
        0,
        "an independently expected native limit matches"
    );
}

#[test]
#[trace("TC-1325", "FR-350-AC-3", "FR-350-AC-10")]
fn replay_evaluated_limits_retain_the_actual_named_charge_and_bound() {
    let fixture = replay_fixture();
    let limited = report(&fixture.execute(&["--limit", "accounting.work_units=1"]));
    assert_eq!(
        limited["capabilities"][0]["outcome"], "incomplete",
        "{limited}"
    );
    let row = &limited["capabilities"][0]["vectors"][0]["actual"]["results"][0];
    assert_eq!(row["limit"]["kind"], "work_units", "{row}");
    assert_eq!(row["limit"]["bound"], "1");
    assert_eq!(row["limit"]["setting"], "accounting.work_units");
    assert!(row["limit"]["charge_point"]
        .as_str()
        .is_some_and(|point| !point.is_empty()));
    assert!(row.get("supplied").is_none());
    assert!(row["value"].is_null());
}

#[test]
#[trace("TC-1325", "FR-350-AC-10", "FR-350-AC-11")]
fn refinement_internals_members_reach_the_replay_executor() {
    // Replaces the retired "unavailable" test: the members now execute, so a
    // defective list is a real refusal that names its request locus. Like
    // the other `replay_fixture` tests it cannot run until the fixture
    // matches the published corpus schema (`/package/semantic_graph/operand_rows`).
    let fixture = replay_fixture();
    let path = fixture.corpus.join("inputs/request.json");
    let mut request: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    request["package"]["lock"]["refinement_internals"] =
        json!([{"refinement":"R", "internal":["unit/hidden"]}]);
    fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    let run = fixture.execute(&[]);
    let first = report(&run);
    let actual = &first["capabilities"][0]["vectors"][0]["actual"];
    assert_eq!(actual["code"], "missing_declaration", "{first}");
    assert_eq!(actual["cause"], "missing-name", "{first}");
    assert!(
        actual["message"].as_str().is_some_and(
            |message| message.contains("/package/lock/refinement_internals/0/refinement")
        ),
        "{first}"
    );
    assert_ne!(
        first["capabilities"][0]["outcome"], "unsupported",
        "{first}"
    );
}

fn subset_population_fixture(redefined: bool) -> Fixture {
    let fixture = model_population_fixture();
    let source = fs::read_to_string(fixture.corpus.join("inputs/unit.native"))
        .unwrap()
        .replace("[1]", "[2]");
    fs::write(fixture.corpus.join("inputs/unit.native"), source).unwrap();
    let path = fixture.corpus.join("inputs/model.json");
    let mut model: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let template = model["types"][0]["fields"][0].clone();
    let field = |owner: &str, name: &str, many: bool, optional: bool| {
        let mut field = template.clone();
        field["identity"] = json!(format!("ix://acme/orders/{owner}/{name}"));
        field["name"] = json!(name);
        field["typeRef"] = json!("ix://acme/orders/Gadget");
        field["presence"] = json!(if optional { "optional" } else { "required" });
        field["multiplicity"] = json!({"lower":if many {0} else {1}, "upper":if many {2} else {1}, "ordered":many, "unique":false});
        field
    };
    model["types"][0]["fields"]
        .as_array_mut()
        .unwrap()
        .push(field("Widget", "allowed", true, false));
    let mut chosen = field("Gadget", "chosen", true, false);
    chosen["subsets"] = json!(["ix://acme/orders/Widget/allowed"]);
    let mut maybe = field("Gadget", "maybe", false, true);
    maybe["multiplicity"]["ordered"] = json!(true);
    maybe["subsets"] = json!(["ix://acme/orders/Widget/allowed"]);
    model["types"][1]["fields"] = json!([chosen, maybe]);
    if redefined {
        let mut allowed = field("Gadget", "allowed", true, false);
        allowed["redefines"] = json!("ix://acme/orders/Widget/allowed");
        model["types"][1]["fields"]
            .as_array_mut()
            .unwrap()
            .push(allowed);
    }
    fs::write(path, serde_json::to_vec(&model).unwrap()).unwrap();
    let reference = |key| json!({"reference":{"population":"ix://acme/orders/Fleet", "key":key}});
    let path = fixture.corpus.join("inputs/population.json");
    let mut snapshot: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    snapshot["populations"][0]["objects"] = json!([
        {"key":"g2", "type":"ix://acme/orders/Gadget", "fields":{"code":{"integer":"8"}, "allowed":{"sequence":[]}, "chosen":{"sequence":[]}, "maybe":{"absent":{}}}},
        {"key":"g1", "type":"ix://acme/orders/Gadget", "fields":{"code":{"integer":"7"}, "allowed":{"sequence":[reference("g1")]}, "chosen":{"sequence":[reference("g1")]}, "maybe":{"present":reference("g1")}}}
    ]);
    fs::write(path, serde_json::to_vec(&snapshot).unwrap()).unwrap();
    fixture
}

#[test]
#[trace("TC-875", "FR-350-AC-8", "FR-350-AC-9")]
fn population_subsetting_retains_inherited_redefined_optional_and_collection_values() {
    for redefined in [false, true] {
        let fixture = subset_population_fixture(redefined);
        let passed = fixture.execute(&[]);
        assert_eq!(passed.outcome.code, 0, "{}", passed.outcome.stdout);
        let path = fixture.corpus.join("inputs/population.json");
        let original = fs::read(&path).unwrap();
        for field in ["chosen", "maybe"] {
            let mut snapshot: Value = serde_json::from_slice(&original).unwrap();
            let reference =
                json!({"reference":{"population":"ix://acme/orders/Fleet", "key":"g2"}});
            snapshot["populations"][0]["objects"][1]["fields"][field] = if field == "chosen" {
                json!({"sequence":[reference]})
            } else {
                json!({"present":reference})
            };
            fs::write(&path, serde_json::to_vec(&snapshot).unwrap()).unwrap();
            let failed = report(&fixture.execute(&[]));
            assert_eq!(failed["capabilities"][0]["outcome"], "failed", "{failed}");
            assert_eq!(
                failed["capabilities"][0]["vectors"][0]["actual"]["cause"], "subsetting-violation",
                "{failed}"
            );
        }
        fs::write(&path, original).unwrap();
        assert_eq!(fixture.execute(&[]).outcome.code, 0);
        let limited = report(&fixture.execute(&["--limit", "admission.work_units=3"]));
        assert_eq!(
            limited["capabilities"][0]["outcome"], "incomplete",
            "{limited}"
        );
        assert_eq!(
            limited["capabilities"][0]["vectors"][0]["actual"]["fields"]["charge_point"],
            "binding.subset-value"
        );
        assert_eq!(
            fixture
                .execute(&["--limit", "admission.work_units=4"])
                .outcome
                .code,
            0
        );
        // Reusing a population for a second parameter keeps one cumulative
        // admission meter, even when its identical binding already exists.
        let source_path = fixture.corpus.join("inputs/unit.native");
        let source = fs::read_to_string(&source_path).unwrap().replace(
            "p: Population<M::Gadget>[2], ok",
            "p: Population<M::Gadget>[2], q: Population<M::Gadget>[2], ok",
        );
        fs::write(source_path, source).unwrap();
        let call_path = fixture.corpus.join("inputs/call.json");
        let mut call: Value = serde_json::from_slice(&fs::read(&call_path).unwrap()).unwrap();
        let population = call["arguments"][0].clone();
        call["arguments"]
            .as_array_mut()
            .unwrap()
            .insert(1, population);
        fs::write(call_path, serde_json::to_vec(&call).unwrap()).unwrap();
        assert_eq!(
            report(&fixture.execute(&["--limit", "admission.work_units=7"]))["capabilities"][0]
                ["outcome"],
            "incomplete"
        );
        assert_eq!(
            fixture
                .execute(&["--limit", "admission.work_units=8"])
                .outcome
                .code,
            0
        );
    }
}

fn use_witness(fixture: &mut Fixture) -> qsl_replay::ReplayResult {
    let request = fixture.native_replay.as_mut().unwrap();
    let qsl_replay::ReplaySource::Input(assignments) = &request.source else {
        panic!("input fixture")
    };
    let transcript = format!(
        "<<<assertion|qualification|negative|{}=1>>>",
        qsl_replay::witness_entry_name(&assignments[0].parameter).unwrap()
    );
    request.source =
        qsl_replay::ReplaySource::Witness(qsl_replay::Witness::parse(&transcript).unwrap());
    let result = qsl_replay::replay(request.clone(), qsl_replay::ReplayLimits::default()).unwrap();
    let path = fixture.corpus.join("inputs/request.json");
    let mut wire: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    wire["source"] = json!({"kind":"witness", "transcript":transcript});
    fs::write(path, serde_json::to_vec(&wire).unwrap()).unwrap();
    let row: Value = serde_json::from_str(&result.to_function_json().unwrap()).unwrap();
    fixture.value["vectors"][0]["expected"] =
        json!({"disposition":"value", "value":{"results":[row]}});
    result
}

#[test]
#[trace("TC-1325", "FR-350-AC-5", "FR-350-AC-10", "FR-350-AC-11")]
fn genuine_witness_replay_retains_record_absence_regions_and_disagreement() {
    let mut fixture = replay_fixture();
    let native = use_witness(&mut fixture);
    let qsl_replay::ReplayResult::Witness(arm) = &native else {
        panic!("witness arm")
    };
    assert_eq!(
        arm.settlement(),
        qsl_replay::WitnessSettlement::ReproducedWithEvaluatedWitness
    );
    assert_eq!(
        arm.value(),
        Some(qsl_replay::EvaluatedValue::Boolean(false))
    );
    assert!(
        arm.record().is_none(),
        "ADR-031 SW-7 function calls have no decisive occurrence"
    );
    let row = fixture.value["vectors"][0]["expected"]["value"]["results"][0].clone();
    assert_eq!(row["arm"], "witness");
    assert!(row.as_object().unwrap().contains_key("record"));
    assert_eq!(row["record"], Value::Null);
    assert_eq!(row["resolved_regions"], json!([]));
    assert!(row.get("disagreement").is_none());
    let passed = fixture.execute(&[]);
    assert_eq!(passed.outcome.code, 0, "{}", passed.outcome.stdout);
    assert_eq!(
        passed.outputs["V1-TOOL-004-operation"],
        qsl_replay::OutcomeDocument::from_replay(&Ok(native))
            .to_bytes()
            .unwrap()
    );
    for (member, incorrect) in [
        ("record", json!({"invented":true})),
        ("resolved_regions", json!([{"invented":true}])),
        ("disagreement", json!({"kind":"no-value"})),
    ] {
        fixture.value["vectors"][0]["expected"]["value"]["results"][0] = row.clone();
        fixture.value["vectors"][0]["expected"]["value"]["results"][0][member] = incorrect;
        assert_eq!(
            report(&fixture.execute(&[]))["capabilities"][0]["outcome"],
            "failed"
        );
    }
    let true_source = FALSE.replace("x < 0", "x > 0");
    let mut true_fixture = replay_fixture_source(&true_source);
    let true_native = use_witness(&mut true_fixture);
    let qsl_replay::ReplayResult::Witness(true_arm) = true_native else {
        panic!("witness arm")
    };
    assert_eq!(
        true_arm.value(),
        Some(qsl_replay::EvaluatedValue::Boolean(true))
    );
    assert!(matches!(
        true_arm.disagreement(),
        Some(qsl_replay::DisagreementCause::Verdicts { .. })
    ));
    let true_row = &true_fixture.value["vectors"][0]["expected"]["value"]["results"][0];
    assert_eq!(
        true_row["disagreement"],
        json!({"cause":"verdicts", "proved":"violation", "replayed":"success"})
    );
    assert!(true_row.as_object().unwrap().contains_key("record"));
    assert_eq!(true_fixture.execute(&[]).outcome.code, 0);
    let undefined_source = FALSE.replace("x < 0", "10 mod (x - 1) > 0");
    let mut undefined_fixture = replay_fixture_source(&undefined_source);
    let undefined_native = use_witness(&mut undefined_fixture);
    let qsl_replay::ReplayResult::Witness(undefined_arm) = undefined_native else {
        panic!("witness arm")
    };
    assert!(undefined_arm.value().is_none());
    assert!(
        !undefined_arm.resolved_regions().is_empty(),
        "the actual undefined operation retains its source"
    );
    let undefined_row = &undefined_fixture.value["vectors"][0]["expected"]["value"]["results"][0];
    assert!(!undefined_row["resolved_regions"]
        .as_array()
        .unwrap()
        .is_empty());
    let region = &undefined_arm.resolved_regions()[0];
    assert!(undefined_source
        [usize::try_from(region.start()).unwrap()..usize::try_from(region.end()).unwrap()]
        .contains("mod"));
    assert_eq!(undefined_row["resolved_regions"][0]["kind"], "region");
    assert_eq!(
        undefined_row["resolved_regions"][0]["start"],
        region.start()
    );
    assert_eq!(undefined_row["resolved_regions"][0]["end"], region.end());
    assert_eq!(undefined_row["disagreement"]["cause"], "no-value");
    assert_eq!(undefined_fixture.execute(&[]).outcome.code, 0);
}
