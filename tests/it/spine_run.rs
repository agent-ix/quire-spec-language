// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-100: CLI `run` routes a `1-draft` program through the spine.

use ix_trace_rs::trace;
use qsl_foundation::ByteDigest;
use serde_json::{json, Value};
use std::{
    path::Path,
    process::{Command, Output},
};

fn run(directory: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quire-spec"))
        .arg("run")
        .arg(directory.join("request.json"))
        .current_dir(directory.parent().unwrap())
        .output()
        .unwrap()
}

/// The `seven`/`px` fixture (FR-027-AC-5), reused here since `seven` is
/// exactly TC-450 step 1's fixture.
const SEVEN_FIXTURE: &str = "tests/fixtures/spine-compile.native";

/// The TC-451 fixture: `lt`, `flag`, `id`, `px`, `corner` and `seven`.
const RUN_FIXTURE: &str = "tests/fixtures/spine-run.native";

fn program_source(program: &[u8]) -> Value {
    json!({"file":"program.native","authority":"agent-ix","identity":"test:spine-run",
        "revision_namespace":"fixture","revision":"fixture:1",
        "digest":ByteDigest::of(program).to_string(),"document":"SpineRun","formal_revision":1})
}

/// Write `program` as `program.native` and a native-run/1 request selecting
/// it with `call`, no models and no libraries.
fn spine_run_request(directory: &Path, program: &[u8], call: Value) {
    std::fs::write(directory.join("program.native"), program).unwrap();
    let request = json!({"format":"native-run/1","request":{
        "models":[],
        "program":{"source":program_source(program)},
        "call":call,
    }});
    std::fs::write(
        directory.join("request.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
}

fn call(function: &str, arguments: Value) -> Value {
    json!({"function": function, "arguments": arguments})
}

/// FR-100-AC-1 (TC-450 step 1): `seven()` completes integer `7`, and the
/// document's `package_id`, `source` and `request_digest` agree with a
/// direct `qsl_replay::spine::compile`.
#[test]
#[trace("TC-450", "FR-100-AC-1")]
fn tc_450_step_1_seven_completes_through_the_spine() {
    let program = std::fs::read(SEVEN_FIXTURE).unwrap();
    let directory = tempfile::tempdir().unwrap();
    spine_run_request(directory.path(), &program, call("seven", json!([])));
    let output = run(directory.path());
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    assert!(output.stdout.ends_with(b"\n"), "{output:?}");
    let request_bytes = std::fs::read(directory.path().join("request.json")).unwrap();
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["format"], "spine-run-result/1");
    assert_eq!(
        document["request_digest"],
        ByteDigest::of(&request_bytes).to_string()
    );
    let compiled = qsl_replay::spine::compile(
        qsl_foundation::SourceIdentity::new("agent-ix", "test:spine-run", "fixture", "fixture:1"),
        "program.native",
        &program,
        &std::collections::BTreeMap::new(),
        &qsl_replay::spine::DependencyInput::default(),
        qsl_replay::spine::SpineLimits::default(),
    )
    .unwrap();
    assert_eq!(document["package_id"], compiled.emitted.package_id().hex());
    assert_eq!(document["source"]["authority"], "agent-ix");
    assert_eq!(document["source"]["identity"], "test:spine-run");
    assert_eq!(document["source"]["revision_namespace"], "fixture");
    assert_eq!(document["source"]["revision"], "fixture:1");
    assert_eq!(
        document["source"]["digest"],
        ByteDigest::of(&program).to_string()
    );
    assert_eq!(document["source"]["path"], "program.native");
    assert_eq!(document["function"], "seven");
    assert_eq!(
        document["outcome"],
        json!({"kind": "completed", "value": {"kind": "integer", "decimal": "7"}})
    );
    // FND-008: the whole document, not only its members individually --
    // catches a stray extra member the per-field assertions above would
    // miss.
    assert_eq!(
        document,
        json!({
            "format": "spine-run-result/1",
            "request_digest": ByteDigest::of(&request_bytes).to_string(),
            "package_id": compiled.emitted.package_id().hex(),
            "source": {
                "authority": "agent-ix",
                "identity": "test:spine-run",
                "revision_namespace": "fixture",
                "revision": "fixture:1",
                "digest": ByteDigest::of(&program).to_string(),
                "path": "program.native",
            },
            "function": "seven",
            "outcome": {"kind": "completed", "value": {"kind": "integer", "decimal": "7"}},
        })
    );
}

/// FR-100-AC-2 (TC-450 step 3): a program declaring `edition "7-draft"`
/// refuses `unknown_edition` at stage `profile`, exit 20, empty stdout,
/// naming the file and the edition.
#[test]
#[trace("TC-450", "FR-100-AC-2")]
fn tc_450_step_3_unknown_edition_refuses_at_profile() {
    let program = std::fs::read_to_string(SEVEN_FIXTURE)
        .unwrap()
        .replacen("1-draft", "7-draft", 1)
        .into_bytes();
    let directory = tempfile::tempdir().unwrap();
    spine_run_request(directory.path(), &program, call("seven", json!([])));
    let output = run(directory.path());
    assert_eq!(output.status.code(), Some(20));
    assert!(output.stdout.is_empty());
    let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(failure["code"], "unknown_edition", "{failure}");
    assert_eq!(failure["stage"], "profile");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("program.native")
            && failure["message"].as_str().unwrap().contains("7-draft"),
        "{failure}"
    );
}

/// FR-100-AC-3 (TC-450 step 4): a `1-draft` request carrying each member the
/// refusal table names, or carrying no `call`, refuses `invalid-request` at
/// stage `request`, exit 20, empty stdout, whatever state its model files
/// are in -- no model file is read.
#[test]
#[trace("TC-450", "FR-100-AC-3")]
fn tc_450_step_4_native_only_members_refuse_before_any_model_is_read() {
    let program = std::fs::read(SEVEN_FIXTURE).unwrap();
    let base = |directory: &Path| {
        std::fs::write(directory.join("program.native"), &program).unwrap();
        json!({"format":"native-run/1","request":{
            "models":[{"format":"native-rule-model/1","source":{
                "file":"missing-model.json","authority":"agent-ix","identity":"acme/model",
                "revision_namespace":"fixture","revision":"fixture:1",
                "digest":format!("sha256:{}", "0".repeat(64)),"document":"M","formal_revision":1}}],
            "program":{"source":program_source(&program)},
            "call":call("seven", json!([])),
        }})
    };
    type Mutation = fn(&mut Value);
    let mutations: Vec<(&str, Mutation)> = vec![
        (
            "selection",
            (|job: &mut Value| {
                job["request"]["selection"] = json!({"owner":{"package":"p","requirement":"r","revision":1},
                    "clause":"c","observation":{"kind":"invocation","invocation":{"identity":"x"}}});
            }) as Mutation,
        ),
        (
            "snapshots",
            (|job: &mut Value| job["request"]["snapshots"] = json!([])) as Mutation,
        ),
        (
            "invocations",
            (|job: &mut Value| job["request"]["invocations"] = json!([])) as Mutation,
        ),
        (
            "package",
            (|job: &mut Value| {
                job["request"]["package"] =
                    json!({"file":"p.bin","digest":format!("sha256:{}", "0".repeat(64))})
            }) as Mutation,
        ),
        (
            "limits.validation_work",
            (|job: &mut Value| job["request"]["limits"] = json!({"validation_work": 1}))
                as Mutation,
        ),
        (
            "limits.expression_steps",
            (|job: &mut Value| job["request"]["limits"] = json!({"expression_steps": 1}))
                as Mutation,
        ),
        (
            "clauses",
            (|job: &mut Value| {
                job["request"]["program"]["clauses"] = json!([{
                    "name":"c","owner":{"package":"p","requirement":"r","revision":1},
                    "clause":"c","point":{"kind":"initialization","name":"n"}}]);
            }) as Mutation,
        ),
        (
            // FND-002: any `clauses` key at all -- including an empty array
            // -- is "carrying" `clauses` for a `1-draft` program.
            "clauses (empty array)",
            (|job: &mut Value| {
                job["request"]["program"]["clauses"] = json!([]);
            }) as Mutation,
        ),
        (
            "no call",
            (|job: &mut Value| {
                job["request"].as_object_mut().unwrap().remove("call");
            }) as Mutation,
        ),
    ];
    for (label, mutate) in mutations {
        let directory = tempfile::tempdir().unwrap();
        let mut job = base(directory.path());
        mutate(&mut job);
        std::fs::write(
            directory.path().join("request.json"),
            serde_json::to_vec(&job).unwrap(),
        )
        .unwrap();
        let output = run(directory.path());
        assert_eq!(output.status.code(), Some(20), "{label}");
        assert!(output.stdout.is_empty(), "{label}");
        let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(failure["code"], "invalid-request", "{label}: {failure}");
        assert_eq!(failure["stage"], "request", "{label}: {failure}");
    }
}

/// FR-100-AC-3 (TC-450 step 5): a `0-draft` standalone request carrying
/// `call` or `libraries` refuses `invalid-request` at stage `request`, exit
/// 20, empty stdout.
#[test]
#[trace("TC-450", "FR-100-AC-3")]
fn tc_450_step_5_zero_draft_carrying_call_or_libraries_refuses() {
    use crate::support::standalone_setup as fixtures;
    for mutate in [
        (|job: &mut Value| {
            job["request"]["call"] = json!({"function": "f", "arguments": []});
        }) as fn(&mut Value),
        (|job: &mut Value| {
            job["request"]["libraries"] = json!([{"identity":"x","version":"1","source":{
                "file":"lib.native","authority":"agent-ix","identity":"x","revision_namespace":"fixture",
                "revision":"fixture:1","digest":format!("sha256:{}", "0".repeat(64)),"document":"L","formal_revision":1}}]);
        }) as fn(&mut Value),
    ] {
        let directory = tempfile::tempdir().unwrap();
        fixtures::write(directory.path(), fixtures::Case::Boolean { flag: true }).unwrap();
        let path = directory.path().join("request.json");
        let mut job: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        mutate(&mut job);
        std::fs::write(&path, serde_json::to_vec(&job).unwrap()).unwrap();
        let output = run(directory.path());
        assert_eq!(output.status.code(), Some(20));
        assert!(output.stdout.is_empty());
        let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(failure["code"], "invalid-request", "{failure}");
        assert_eq!(failure["stage"], "request", "{failure}");
    }
}

/// FR-100-AC-4 (TC-451 step 1): `lt` binds `b` before `a` by declared name;
/// `flag`/`id` complete their declared kind.
#[test]
#[trace("TC-451", "FR-100-AC-4")]
fn tc_451_step_1_arguments_bind_by_declared_name() {
    let program = std::fs::read(RUN_FIXTURE).unwrap();
    let directory = tempfile::tempdir().unwrap();
    spine_run_request(
        directory.path(),
        &program,
        call(
            "lt",
            json!([{"parameter":"b","value":3}, {"parameter":"a","value":5}]),
        ),
    );
    let output = run(directory.path());
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        document["outcome"],
        json!({"kind": "completed", "value": {"kind": "boolean", "value": false}})
    );

    // FND-008: CLI-level coverage for `flag` and `id`, not only `lt`.
    let directory = tempfile::tempdir().unwrap();
    spine_run_request(
        directory.path(),
        &program,
        call("flag", json!([{"parameter": "b", "value": 1}])),
    );
    let output = run(directory.path());
    assert_eq!(output.status.code(), Some(0));
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        document["outcome"],
        json!({"kind": "completed", "value": {"kind": "boolean", "value": true}})
    );

    let directory = tempfile::tempdir().unwrap();
    spine_run_request(
        directory.path(),
        &program,
        call("id", json!([{"parameter": "x", "value": 4}])),
    );
    let output = run(directory.path());
    assert_eq!(output.status.code(), Some(0));
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        document["outcome"],
        json!({"kind": "completed", "value": {"kind": "integer", "decimal": "4"}})
    );
}

/// FR-100-AC-4 (TC-451 step 2): a value outside the parameter's kind or
/// declared domain refuses `invalid_runtime_input` at stage `call`, exit
/// 20, empty stdout, `details` `{"position": 0}`.
#[test]
#[trace("TC-451", "FR-100-AC-4")]
fn tc_451_step_2_wrong_value_kind_refuses_at_call() {
    let program = std::fs::read(RUN_FIXTURE).unwrap();
    for (function, arguments) in [
        ("flag", json!([{"parameter": "b", "value": 2}])),
        ("id", json!([{"parameter": "x", "value": 12}])),
        ("px", json!([{"parameter": "p", "value": 1}])),
    ] {
        let directory = tempfile::tempdir().unwrap();
        spine_run_request(directory.path(), &program, call(function, arguments));
        let output = run(directory.path());
        assert_eq!(output.status.code(), Some(20), "{function}");
        assert!(output.stdout.is_empty(), "{function}");
        let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(
            failure["code"], "invalid_runtime_input",
            "{function}: {failure}"
        );
        assert_eq!(failure["stage"], "call", "{function}: {failure}");
        assert_eq!(failure["details"]["position"], 0, "{function}: {failure}");
    }
}

/// FR-100-AC-4 (TC-451 step 3): an argument naming no parameter, a
/// parameter bound twice, and a parameter left unbound each refuse
/// `invalid_runtime_input` at stage `call`, naming the parameter.
#[test]
#[trace("TC-451", "FR-100-AC-4")]
fn tc_451_step_3_argument_binding_names_the_parameter() {
    let program = std::fs::read(RUN_FIXTURE).unwrap();
    for (arguments, parameter) in [
        (json!([{"parameter": "y", "value": 4}]), "y"),
        (
            json!([{"parameter": "x", "value": 1}, {"parameter": "x", "value": 2}]),
            "x",
        ),
        (json!([]), "x"),
    ] {
        let directory = tempfile::tempdir().unwrap();
        spine_run_request(directory.path(), &program, call("id", arguments));
        let output = run(directory.path());
        assert_eq!(output.status.code(), Some(20), "{parameter}");
        assert!(output.stdout.is_empty(), "{parameter}");
        let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(
            failure["code"], "invalid_runtime_input",
            "{parameter}: {failure}"
        );
        assert_eq!(failure["stage"], "call", "{parameter}: {failure}");
        assert_eq!(
            failure["details"]["parameter"], parameter,
            "{parameter}: {failure}"
        );
    }
}

/// FR-100-AC-5 (TC-451 step 6): a `1-draft` source with a syntax error
/// refuses at stage `source` (`invalid_syntax`), and one declaring
/// `inv(x: Digit): Boolean { 1 / x > 0 }` refuses at stage `check` with
/// `ill_typed`; each exits 20 with empty stdout.
#[test]
#[trace("TC-451", "FR-100-AC-5")]
fn tc_451_step_6_compile_refusals_carry_their_own_stage() {
    let syntax_error = b"language \"ix:native\" edition \"1-draft\";\nfunction (".to_vec();
    let ill_typed = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"quire.value.complete/v1\" version \"1\" digest \
        \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n\
        type Digit = Int[0, 9];\n\
        function inv using v(x: Digit): Boolean pure { 1 / x > 0 }\n"
        .to_owned()
        .into_bytes();
    for (program, function, stage, code) in [
        (syntax_error, "f", "source", "invalid_syntax"),
        (ill_typed, "inv", "check", "ill_typed"),
    ] {
        let directory = tempfile::tempdir().unwrap();
        spine_run_request(
            directory.path(),
            &program,
            call(function, json!([{"parameter": "x", "value": 1}])),
        );
        let output = run(directory.path());
        assert_eq!(
            output.status.code(),
            Some(20),
            "{function}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty(), "{function}");
        let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(failure["code"], code, "{function}: {failure}");
        assert_eq!(failure["stage"], stage, "{function}: {failure}");
    }
}

/// FR-100-AC-5 (TC-451 step 5): `function` shapes that malform or fail
/// lookup refuse `missing_declaration` at stage `call`, and `corner`'s
/// record result refuses `unsupported_construct`, both exit codes as
/// tabled, empty stdout.
#[test]
#[trace("TC-451", "FR-100-AC-5")]
fn tc_451_step_5_function_shape_and_lookup_refusals() {
    let program = std::fs::read(RUN_FIXTURE).unwrap();
    for name in ["nope", "module.seven", "", "seven.", "7x"] {
        let directory = tempfile::tempdir().unwrap();
        spine_run_request(directory.path(), &program, call(name, json!([])));
        let output = run(directory.path());
        assert_eq!(output.status.code(), Some(20), "{name}");
        assert!(output.stdout.is_empty(), "{name}");
        let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(failure["code"], "missing_declaration", "{name}: {failure}");
        assert_eq!(failure["stage"], "call", "{name}: {failure}");
        assert_eq!(failure["details"]["function"], name, "{name}: {failure}");
    }
    let directory = tempfile::tempdir().unwrap();
    spine_run_request(
        directory.path(),
        &program,
        call("corner", json!([{"parameter":"p","value":0}])),
    );
    let output = run(directory.path());
    assert_eq!(output.status.code(), Some(21));
    assert!(output.stdout.is_empty());
    let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(failure["code"], "unsupported_construct", "{failure}");
    assert_eq!(failure["stage"], "call");
    assert_eq!(failure["details"]["function"], "corner");
}

/// FR-100-AC-6 (TC-451 step 7): `seven` with `work_units` 0 writes outcome
/// `{"kind": "incomplete", "limit": "work_units"}`, exit 22.
#[test]
#[trace("TC-451", "FR-100-AC-6")]
fn tc_451_step_7_zero_work_units_is_incomplete() {
    let program = std::fs::read(RUN_FIXTURE).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let mut request = call("seven", json!([]));
    request["work_units"] = json!(0);
    spine_run_request(directory.path(), &program, request);
    let output = run(directory.path());
    assert_eq!(
        output.status.code(),
        Some(22),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        document["outcome"],
        json!({"kind": "incomplete", "limit": "work_units"})
    );
}

/// FR-100-AC-3 (TC-450 step 4): a `work_units` of `18446744073709551616`
/// (above `u64::MAX`), `-1` or `1.5` refuses `invalid-request` at stage
/// `request`, exit 20.
#[test]
#[trace("TC-450", "FR-100-AC-3")]
fn tc_450_step_4_malformed_work_units_refuses() {
    let program = std::fs::read(SEVEN_FIXTURE).unwrap();
    // `18446744073709551616` (2^64) does not fit `serde_json::Value`'s own
    // `u64`/`i64`/`f64` number representation, so it is spliced into the
    // request text directly rather than built through `json!`.
    for work_units in ["18446744073709551616", "-1", "1.5"] {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("program.native"), &program).unwrap();
        let mut placeholder_call = call("seven", json!([]));
        placeholder_call["work_units"] = json!(0);
        let request = json!({"format":"native-run/1","request":{
            "models":[],
            "program":{"source":program_source(&program)},
            "call": placeholder_call,
        }});
        let text = serde_json::to_string(&request).unwrap().replacen(
            "\"work_units\":0",
            &format!("\"work_units\":{work_units}"),
            1,
        );
        std::fs::write(directory.path().join("request.json"), text).unwrap();
        let output = run(directory.path());
        assert_eq!(output.status.code(), Some(20), "{work_units}");
        assert!(output.stdout.is_empty(), "{work_units}");
        let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(
            failure["code"], "invalid-request",
            "{work_units}: {failure}"
        );
        assert_eq!(failure["stage"], "request", "{work_units}: {failure}");
    }
}

/// FR-100-AC-4 (TC-451 step 4): an argument `value` of `true`, `"7"`, `1.5`
/// or `9223372036854775808` (above `i64::MAX`) refuses `invalid-request` at
/// stage `request`, exit 20.
#[test]
#[trace("TC-451", "FR-100-AC-4")]
fn tc_451_step_4_malformed_argument_value_refuses() {
    let program = std::fs::read(RUN_FIXTURE).unwrap();
    for value in [
        json!(true),
        json!("7"),
        json!(1.5),
        json!(9223372036854775808u64),
    ] {
        let directory = tempfile::tempdir().unwrap();
        spine_run_request(
            directory.path(),
            &program,
            call("id", json!([{"parameter": "x", "value": value}])),
        );
        let output = run(directory.path());
        assert_eq!(output.status.code(), Some(20), "{value}");
        assert!(output.stdout.is_empty(), "{value}");
        let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(failure["code"], "invalid-request", "{value}: {failure}");
        assert_eq!(failure["stage"], "request", "{value}: {failure}");
    }
}

/// FR-100-AC-2 (TC-450 step 2): a `0-draft` standalone request's stdout and
/// exit status are unaffected by the spine-run routing change.
#[test]
#[trace("TC-450", "FR-100-AC-2")]
fn tc_450_step_2_zero_draft_requests_are_unaffected() {
    use crate::support::standalone_setup as fixtures;
    let directory = tempfile::tempdir().unwrap();
    fixtures::write(directory.path(), fixtures::Case::Boolean { flag: true }).unwrap();
    let output = run(directory.path());
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["format"], "native-run-result/1");
    // FND-008: a stronger oracle than the format alone -- the fixture's
    // clause (`true implies flag`, `flag` bound `true`) actually evaluates
    // and completes `true`, not merely reaching some other stage or a
    // refusal that happens to still exit 0.
    assert_eq!(document["stage"], "evaluate");
    assert_eq!(document["truth"], true);
}

/// FR-100-AC-3 (TC-450 step 6): a `1-draft` request whose `libraries`
/// supplies an imported library and whose `models` supplies a domain
/// package the program selects runs, exit 0. Neither the imported function
/// nor the model type is called: this is FR-100's own text ("supplies ...
/// and ... selects"), not a claim that the call itself touches them.
#[test]
#[trace("TC-450", "FR-100-AC-3")]
fn tc_450_step_6_libraries_and_models_both_present_runs() {
    const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\" version \"1\" digest \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n";
    const LIBRARY: &str = "function f using v(x: Int[0, 9]): Boolean pure { x < 5 }\n";
    let library = format!("{HEADER}{LIBRARY}").into_bytes();
    let library_digest = qsl_replay::spine::compile(
        qsl_foundation::SourceIdentity::new("agent-ix", "test:geometry", "fixture", "fixture:1"),
        "geometry.native",
        &library,
        &std::collections::BTreeMap::new(),
        &qsl_replay::spine::DependencyInput::default(),
        qsl_replay::spine::SpineLimits::default(),
    )
    .unwrap()
    .emitted
    .package_id()
    .hex();
    let document = std::fs::read("tests/fixtures/spine-model.semantic-ir.json").unwrap();
    let program = format!(
        "{HEADER}\
         import \"test/geometry\" version \"1\" digest \"{library_digest}\" as g;\n\
         model M = \"acme/orders\" version \"1.0.0\" digest \"sha256-jcs:5fc327ab7b2b90151ae6713296e15512c38930d9ba61cf5f586eaa2a70145bfd\";\n\
         function seven using v(): Integer pure {{ 7 }}\n"
    )
    .into_bytes();

    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("program.native"), &program).unwrap();
    std::fs::write(directory.path().join("geometry.native"), &library).unwrap();
    std::fs::write(directory.path().join("orders.json"), &document).unwrap();
    let request = json!({"format":"native-run/1","request":{
        "models":[{"format":"semantic-ir/2.0.0","source":{
            "file":"orders.json","authority":"agent-ix","identity":"acme/orders",
            "revision_namespace":"fixture","revision":"fixture:1",
            "digest":ByteDigest::of(&document).to_string(),"document":"Orders","formal_revision":1}}],
        "libraries":[{"identity":"test/geometry","version":"1","source":{
            "file":"geometry.native","authority":"agent-ix","identity":"test:geometry",
            "revision_namespace":"fixture","revision":"fixture:1",
            "digest":ByteDigest::of(&library).to_string(),"document":"Geometry","formal_revision":1}}],
        "program":{"source":program_source(&program)},
        "call":call("seven", json!([])),
    }});
    std::fs::write(
        directory.path().join("request.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
    let output = run(directory.path());
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        document["outcome"],
        json!({"kind": "completed", "value": {"kind": "integer", "decimal": "7"}})
    );
}
