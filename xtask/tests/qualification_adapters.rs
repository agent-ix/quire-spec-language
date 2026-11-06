// SPDX-License-Identifier: AGPL-3.0-or-later
//! Public compile/run/replay qualification adapters, independent of catalog counts.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use ix_trace_rs::trace;
use serde_json::{json, Value};
use tempfile::TempDir;

const COUNTER: &str = "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\nfunction digits using v(x: Int[0,9], y: Int[0,9]): Integer pure { x * 10 + y }\n";
const FALSE: &str = "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\nfunction negative using v(): Boolean pure { false }\n";

struct Fixture {
    _temp: TempDir,
    corpus: PathBuf,
    qspec: PathBuf,
    value: Value,
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
    let output = Command::new("git")
        .arg("-C")
        .arg(qspec_repository())
        .args(["show", &format!("origin/main:{path}")])
        .output()
        .expect("read published QSpec input");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
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
        result["capabilities"][0]["vectors"][0]["actual"]["code"], "invalid_model_binding",
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
    let mut fixture = Fixture::new(
        "runtime",
        "replay",
        FALSE,
        json!({"disposition":"value","value":null}),
    );
    let selected = qsl_replay::call_site(
        qsl_foundation::SourceIdentity::new("agent-ix", "inputs/unit.native"),
        "qualification".into(),
        "inputs/unit.native",
        FALSE.as_bytes(),
        [],
        &qsl_replay::DependencyInput::default(),
        &qsl_replay::QualifiedName::new(vec![qsl_replay::Identifier::new("negative").unwrap()])
            .unwrap(),
    )
    .unwrap();
    let node = |key| qsl_foundation::node_wire::node_key_to_json(key).unwrap();
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
        "source":{"kind":"input","assignments":[]},"obligation_identity":{"node_id":node(selected.site.obligation_identity().node()),"model":selected.site.obligation_identity().model().map(|model|model.as_str()).unwrap_or("qualification"),"ordinal":selected.site.obligation_identity().ordinal()},"backend":"fixture", "state_environment":[],"accounting_limits":limits,"stage_limits":{},"supplied_limits":null,"object_environment":null,"declared_domains":[],"byte_provision":[{"authority":"agent-ix","identity":"inputs/unit.native","bytes_path":"inputs/bytes.native"}]
    });
    fixture.value["vectors"][0]["input"]["files"] = json!([]);
    fixture.file(
        "replay-request",
        "inputs/request.json",
        &serde_json::to_vec(&request).unwrap(),
    );
    fixture.file("bytes", "inputs/bytes.native", FALSE.as_bytes());
    fixture
}

#[test]
#[trace("TC-1325", "FR-350-AC-11", "FR-350-AC-12", "FR-350-AC-16")]
fn replay_vectors_decode_and_execute_without_a_source_file_kind() {
    let fixture = replay_fixture();
    let result = fixture.execute(&[]);
    let actual = report(&result);
    assert_ne!(
        actual["capabilities"][0]["outcome"], "tool-failure",
        "{actual}"
    );
    assert!(
        actual["capabilities"][0]["vectors"][0]["actual"]["results"].is_array(),
        "real typed replay result: {actual}"
    );
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
