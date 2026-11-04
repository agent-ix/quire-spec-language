// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-027: CLI compilation of `1-draft` sources through the spine.

use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::{
    path::Path,
    process::{Command, Output},
};

fn compile(directory: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quire-spec"))
        .arg("compile")
        .arg(directory.join("compile.json"))
        .current_dir(directory.parent().unwrap())
        .output()
        .unwrap()
}

fn compile_with_args(directory: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quire-spec"))
        .args(args)
        .current_dir(directory)
        .output()
        .unwrap()
}

fn compile_in_process(
    directory: &Path,
) -> Result<Vec<u8>, Box<quire_spec_language::command::RunError>> {
    quire_spec_language::command::compile(&directory.join("compile.json"))
}

/// The complete-V1 compile fixture (FR-027-AC-5).
const SPINE_FIXTURE: &str = "tests/fixtures/spine-compile.native";

/// The fixture's header and profile selection, which every stage-refusal
/// body below follows.
const SPINE_HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.model.complete/v1\";\n";

/// Write `program` as `program.native` under `directory` with a
/// native-compile/1 request selecting it, with no model imports.
fn spine_request(directory: &Path, program: &[u8]) {
    std::fs::write(directory.join("program.native"), program).unwrap();
    let request = json!({"format":"native-compile/1","request":{"models":[],"program":{
        "source":{"file":"program.native","authority":"agent-ix","identity":"test:spine",
        "document":"Spine","formal_revision":1}}}});
    std::fs::write(
        directory.join("compile.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
}

/// FR-027-AC-2 / AC-5 (TC-105 step 1, TC-435 step 2): a `1-draft` program compiles through the
/// spine; stdout is exactly the bytes `qsl_replay::spine`'s operations emit, which
/// qsl-package's TC-435 step 1 reads back Verified with nothing omitted.
#[test]
#[trace(
    "TC-105",
    "FR-027-AC-2",
    "TC-435",
    "FR-027-AC-5",
    "TC-785",
    "FR-027-AC-11"
)]
fn a_complete_v1_program_compiles_through_the_spine() {
    let program = std::fs::read(SPINE_FIXTURE).unwrap();
    let directory = tempfile::tempdir().unwrap();
    spine_request(directory.path(), &program);
    let mut selected_files = std::fs::read_dir(directory.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<Vec<_>>();
    selected_files.sort();
    assert_eq!(selected_files, ["compile.json", "program.native"]);
    let output = compile(directory.path());
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let library = crate::support::front_end::emitted(
        qsl_foundation::SourceIdentity::new("agent-ix", "test:spine"),
        "program.native",
        &program,
        &std::collections::BTreeMap::new(),
        &qsl_replay::spine::DependencyInput::default(),
    )
    .unwrap();
    assert_eq!(output.stdout, library.package().bytes());
    let wire: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(wire["contract_version"], "quire.checked-package/v2");
}

/// FR-027-AC-2 (TC-105 step 2): compile rejects run-format requests and
/// runtime-only members before trying to read their referenced files.
#[test]
#[trace("TC-105", "FR-027-AC-2")]
fn compile_refuses_runtime_request_shapes_without_reading_runtime_files() {
    let program = std::fs::read(SPINE_FIXTURE).unwrap();
    let directory = tempfile::tempdir().unwrap();
    spine_request(directory.path(), &program);
    let path = directory.path().join("compile.json");
    let mut request: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let base_request = request.clone();

    request["format"] = json!("native-run/1");
    request["request"]["program"]["source"]["file"] = json!("missing-program.native");
    request["request"]["models"] = json!([{"format":"missing-model-format","source":{
        "file":"missing-model.json"}}]);
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    let error = compile_in_process(directory.path()).unwrap_err();
    let value = error.value().unwrap();
    assert_eq!(value["code"], "unknown_wire", "{value}");
    assert_eq!(value["stage"], "envelope", "{value}");
    assert!(value.get("artifact").is_none(), "{value}");

    for member in ["snapshots", "invocations", "selection", "package"] {
        let mut request = base_request.clone();
        request["format"] = json!("native-compile/1");
        request["request"]["program"]["source"]["file"] = json!("missing-program.native");
        request["request"]
            .as_object_mut()
            .unwrap()
            .insert(member.to_owned(), json!({"file":"missing-runtime.json"}));
        std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
        let error = compile_in_process(directory.path()).unwrap_err();
        let value = error.value().unwrap();
        assert_eq!(value["code"], "invalid-request", "{member}: {value}");
        assert_eq!(value["stage"], "request", "{member}: {value}");
        assert!(value.get("artifact").is_none(), "{member}: {value}");
    }
}

/// FR-031-AC-6 (TC-109): `native-compile/1` recognizes the run-only
/// `extraction` member and refuses before opening the selected source.
#[test]
#[trace("TC-109", "FR-031-AC-6")]
fn compile_refuses_extraction_before_reading_selected_files() {
    for extraction in [json!(null), json!({"source":"missing.md"})] {
        let directory = tempfile::tempdir().unwrap();
        let request = json!({"format":"native-compile/1","request":{
            "models":[{"format":"semantic-ir/2.0.0","source":{
                "file":"missing-model.json","authority":"agent-ix",
                "identity":"test:model","document":"Model","formal_revision":1}}],
            "program":{"source":{"file":"missing-program.native","authority":"agent-ix",
                "identity":"test:compile","document":"Compile","formal_revision":1},
                "extraction":extraction}}});
        let request_path = directory.path().join("compile.json");
        std::fs::write(&request_path, serde_json::to_vec(&request).unwrap()).unwrap();
        let error = quire_spec_language::command::compile(&request_path).unwrap_err();
        let value = error.value().unwrap();
        assert_eq!(error.category().exit_code(), 20);
        assert_eq!(value["code"], "extraction-requires-run", "{value}");
        assert_eq!(value["stage"], "request", "{value}");
        assert!(value.get("artifact").is_none(), "{value}");
    }
}

/// FR-027-AC-3 (TC-105 steps 3–5): source selection and dependent-byte
/// refusals retain typed causes and produce no partial package.
#[test]
#[trace("TC-105", "FR-027-AC-3")]
fn source_selection_and_dependent_byte_refusals_are_typed() {
    let program = std::fs::read(SPINE_FIXTURE).unwrap();
    for member in ["identity", "authority"] {
        let directory = tempfile::tempdir().unwrap();
        spine_request(directory.path(), &program);
        let path = directory.path().join("compile.json");
        let mut request: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        request["request"]["program"]["source"]
            .as_object_mut()
            .unwrap()
            .remove(member);
        std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
        let error = compile_in_process(directory.path()).unwrap_err();
        let value = error.value().unwrap();
        assert_eq!(value["code"], "invalid-request", "{member}: {value}");
        assert_eq!(value["stage"], "request", "{member}: {value}");
        assert!(value.get("artifact").is_none(), "{member}: {value}");
    }

    let directory = tempfile::tempdir().unwrap();
    spine_request(directory.path(), &program);
    let path = directory.path().join("compile.json");
    let mut request = std::fs::read(&path).unwrap();
    request.resize(1_048_577, b' ');
    std::fs::write(&path, request).unwrap();
    let error = compile_in_process(directory.path()).unwrap_err();
    let value = error.value().unwrap();
    assert_eq!(value["code"], "resource_exhausted", "{value}");
    assert_eq!(value["details"]["limit"], "request_bytes", "{value}");
    assert_eq!(value["details"]["maximum"], 1_048_576, "{value}");
    assert!(value["details"].get("option").is_none(), "{value}");
    assert!(value.get("artifact").is_none(), "{value}");

    let directory = tempfile::tempdir().unwrap();
    spine_request(directory.path(), &program);
    let path = directory.path().join("compile.json");
    let mut request: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    request["request"]["program"]["source"]["file"] = json!("missing-program.native");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    let error = compile_in_process(directory.path()).unwrap_err();
    let value = error.value().unwrap();
    assert_eq!(value["code"], "io-error", "{value}");
    assert_eq!(value["stage"], "file", "{value}");
    assert!(value.get("artifact").is_none(), "{value}");

    let directory = tempfile::tempdir().unwrap();
    let oversized = format!(
        "{SPINE_HEADER}function f using v(): Int[0, 9] pure {{ 7 }}\n//{}",
        "x".repeat(8 * 1024 * 1024)
    );
    spine_request(directory.path(), oversized.as_bytes());
    let error = compile_in_process(directory.path()).unwrap_err();
    let value = error.value().unwrap();
    assert_eq!(value["code"], "resource_exhausted", "{value}");
    assert_eq!(value["details"]["limit"], "dependent_bytes", "{value}");
    assert_eq!(value["details"]["maximum"], 8_388_608, "{value}");
    assert!(value["details"].get("option").is_none(), "{value}");
    assert!(value.get("artifact").is_none(), "{value}");
}

/// FR-027-AC-14 (TC-1210): native-compile/1 accepts a source-unit array,
/// emits the FR-278 composition, and applies edition and byte limits to all
/// units.
#[test]
#[trace("TC-1210", "FR-027-AC-14", "TC-446", "FR-027-AC-12")]
fn compile_accepts_a_multi_unit_program_array() {
    let first = format!("{SPINE_HEADER}function alphaUnit using v(): Boolean pure {{ true }}\n");
    let second = format!("{SPINE_HEADER}function betaUnit using v(): Boolean pure {{ false }}\n");
    let units = [
        ("unit-a.native", "test:unit-a", first.as_bytes()),
        ("unit-b.native", "test:unit-b", second.as_bytes()),
    ];
    let directory = tempfile::tempdir().unwrap();
    for (file, _, bytes) in units {
        std::fs::write(directory.path().join(file), bytes).unwrap();
    }
    let write_job = |indices: &[usize]| {
        let program = indices
            .iter()
            .map(|index| {
                let (file, identity, _) = units[*index];
                json!({"file":file,"authority":"agent-ix","identity":identity,
                    "document":"Unit","formal_revision":1})
            })
            .collect::<Vec<_>>();
        let job = json!({"format":"native-compile/1","request":{
            "models":[],"program":program,"refinement_internals":[]}});
        std::fs::write(
            directory.path().join("compile.json"),
            serde_json::to_vec(&job).unwrap(),
        )
        .unwrap();
    };
    write_job(&[0, 1]);
    let forward = compile(directory.path());
    assert_eq!(
        forward.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&forward.stderr)
    );

    let source_identities: Vec<_> = units
        .iter()
        .map(|(_, identity, _)| qsl_foundation::SourceIdentity::new("agent-ix", *identity))
        .collect();
    let parse_requests: Vec<_> = units
        .iter()
        .zip(&source_identities)
        .map(
            |((path, _, bytes), source)| qsl_replay::spine::ParseRequest {
                source,
                path,
                bytes,
            },
        )
        .collect();
    let dependencies = qsl_replay::spine::DependencyInput::default();
    qsl_replay::spine::preflight_unit_list(&parse_requests, &dependencies).unwrap();
    let cancel = quire_exact::Cancel::new();
    let limits = qsl_replay::spine::SpineLimits::default();
    let parsed = parse_requests
        .iter()
        .map(|request| {
            qsl_replay::spine::parse(request, limits.source, &cancel)
                .unwrap()
                .into_value()
        })
        .collect();
    let admitted = qsl_replay::spine::admit_parsed_unit_list(parsed, &dependencies).unwrap();
    let packages =
        qsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>()).unwrap();
    let models =
        qsl_replay::spine::select_units(&admitted, &packages, limits.model, limits.source, &cancel)
            .unwrap()
            .into_value();
    let checked = qsl_replay::spine::check_units_with_refinement_internals(
        &admitted,
        "test:unit-a".to_owned(),
        Some(json!([])),
        &models,
        &dependencies,
        limits,
        &cancel,
    )
    .unwrap()
    .into_value();
    let composed = qsl_replay::spine::package(
        &checked[0],
        qsl_replay::spine::PackageLimits::default(),
        &cancel,
    )
    .unwrap()
    .into_value();
    assert_eq!(forward.stdout, composed.package().bytes());

    write_job(&[1, 0]);
    let reversed = compile(directory.path());
    assert_eq!(
        reversed.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&reversed.stderr)
    );
    assert_eq!(reversed.stdout, forward.stdout);

    let invalid_second = SPINE_HEADER.replacen("1-draft", "0-draft", 1)
        + "function gammaUnit using v(): Boolean pure { true }\n";
    std::fs::write(directory.path().join("unit-b.native"), &invalid_second).unwrap();
    write_job(&[0, 1]);
    let invalid = compile(directory.path());
    let failure = refusal(&invalid, 20, "unknown_edition", "later unit edition");
    assert_eq!(failure["details"]["path"], "unit-b.native", "{failure}");
    let edition_literal = invalid_second.find("\"0-draft\"").unwrap();
    assert_eq!(
        failure["details"]["span"]["start"]["byte"],
        edition_literal,
        "{failure}"
    );
    assert_eq!(
        failure["details"]["span"]["end"]["byte"],
        edition_literal + "\"0-draft\"".len(),
        "{failure}"
    );

    let empty = json!({"format":"native-compile/1","request":{"models":[],"program":[]}});
    std::fs::write(
        directory.path().join("compile.json"),
        serde_json::to_vec(&empty).unwrap(),
    )
    .unwrap();
    let failure = refusal(
        &compile(directory.path()),
        20,
        "invalid-request",
        "empty list",
    );
    assert_eq!(failure["stage"], "request", "{failure}");

    let oversized_first = format!("{first}//{}", "x".repeat(4_200_000));
    let oversized_second = format!("{second}//{}", "x".repeat(4_200_000));
    std::fs::write(directory.path().join("unit-a.native"), oversized_first).unwrap();
    std::fs::write(directory.path().join("unit-b.native"), oversized_second).unwrap();
    write_job(&[0, 1]);
    let failure = refusal(
        &compile(directory.path()),
        22,
        "resource_exhausted",
        "aggregate dependent bytes",
    );
    assert_eq!(failure["details"]["limit"], "dependent_bytes", "{failure}");
    assert_eq!(failure["details"]["maximum"], 8_388_608, "{failure}");
}

/// FR-287-AC-4 (TC-1262 step 2): the compile CLI enforces fixed request and
/// dependent-byte ceilings and rejects run-only limit options.
#[test]
#[trace("TC-1262", "FR-287-AC-4", "FR-287-AC-5")]
fn compile_cli_uses_fixed_byte_ceilings_and_rejects_limit_options() {
    let program = std::fs::read(SPINE_FIXTURE).unwrap();

    let request_directory = tempfile::tempdir().unwrap();
    spine_request(request_directory.path(), &program);
    let request_path = request_directory.path().join("compile.json");
    let mut request = std::fs::read(&request_path).unwrap();
    request.resize(1_048_577, b' ');
    std::fs::write(&request_path, request).unwrap();
    let output = compile(request_directory.path());
    assert_eq!(output.status.code(), Some(22));
    assert!(output.stdout.is_empty());
    let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(failure["details"]["limit"], "request_bytes", "{failure}");
    assert_eq!(failure["details"]["maximum"], 1_048_576, "{failure}");
    assert!(failure["details"].get("option").is_none(), "{failure}");

    let dependent_directory = tempfile::tempdir().unwrap();
    let oversized = format!(
        "{SPINE_HEADER}function f using v(): Int[0, 9] pure {{ 7 }}\n//{}",
        "x".repeat(8 * 1024 * 1024)
    );
    spine_request(dependent_directory.path(), oversized.as_bytes());
    let output = compile(dependent_directory.path());
    assert_eq!(output.status.code(), Some(22));
    assert!(output.stdout.is_empty());
    let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(failure["details"]["limit"], "dependent_bytes", "{failure}");
    assert_eq!(failure["details"]["maximum"], 8_388_608, "{failure}");
    assert!(failure["details"].get("option").is_none(), "{failure}");

    let no_files = tempfile::tempdir().unwrap();
    let output = compile_with_args(
        no_files.path(),
        &["compile", "missing.json", "--dependent-bytes", "10000000"],
    );
    assert_eq!(output.status.code(), Some(20));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("usage: quire-spec"));
}

/// Rewrite `compile.json` under `directory` through `change` and run `compile`.
fn compile_changed(directory: &Path, change: impl FnOnce(&mut Value)) -> Output {
    let path = directory.join("compile.json");
    let mut job: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    change(&mut job);
    std::fs::write(&path, serde_json::to_vec(&job).unwrap()).unwrap();
    compile(directory)
}

/// Assert `output` is a refusal with `code` on stderr, `exit` as its status and
/// no package bytes; return the stderr document.
fn refusal(output: &Output, exit: i32, code: &str, label: &str) -> Value {
    assert_eq!(output.status.code(), Some(exit), "{label}");
    assert!(output.stdout.is_empty(), "{label}");
    let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(failure["code"], code, "{label}: {failure}");
    failure
}

/// FR-027-AC-3 (TC-105): `compile` checks its arity before opening a request.
#[test]
#[trace("TC-105", "FR-027-AC-3")]
fn compile_arity_is_checked_before_opening_any_request() {
    for arguments in [vec!["compile"], vec!["compile", "missing.json", "extra"]] {
        let output = Command::new(env!("CARGO_BIN_EXE_quire-spec"))
            .args(arguments)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(20));
        assert!(output.stdout.is_empty());
        assert_eq!(
            std::str::from_utf8(&output.stderr).unwrap(),
            "usage: quire-spec compile <request-file>\n"
        );
    }
}

/// FR-027-AC-3 (TC-105): a package write that fails is a tool failure, exit 30.
#[cfg(target_os = "linux")]
#[test]
#[trace("TC-105", "FR-027-AC-3")]
fn failed_artifact_output_is_a_tool_failure_exit() {
    let directory = tempfile::tempdir().unwrap();
    spine_request(directory.path(), &std::fs::read(SPINE_FIXTURE).unwrap());
    let full = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_quire-spec"))
        .arg("compile")
        .arg(directory.path().join("compile.json"))
        .stdout(std::process::Stdio::from(full))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(30));
    assert!(std::str::from_utf8(&output.stderr)
        .unwrap()
        .starts_with("output failed:"));
}

/// FR-027-AC-3 (TC-105, TC-104, FR-026-AC-4): a compile request selecting more
/// models than the shared file limit allows refuses `resource_exhausted`
/// naming the `models` group before any file is read.
#[test]
#[trace("TC-105", "FR-027-AC-3", "TC-104", "FR-026-AC-4")]
fn compile_file_count_refusal_names_the_models_group_before_file_io() {
    let directory = tempfile::tempdir().unwrap();
    spine_request(directory.path(), b"never read");
    let output = compile_changed(directory.path(), |job| {
        let model = json!({"format":"semantic-ir/2.0.0","source":{
            "file":"missing-model.json","authority":"agent-ix","identity":"acme/model",
            "document":"M","formal_revision":1}});
        job["request"]["models"] = json!(vec![model; 64]);
        job["request"]["program"]["source"]["file"] = json!("missing.native");
    });
    let failure = refusal(&output, 22, "resource_exhausted", "models");
    assert_eq!(
        failure["details"],
        json!({"limit":"selected files","category":"models",
            "requested":64,"remaining":63,"maximum":64})
    );
}

/// FR-027-AC-14 (TC-1210): each source selection in a program array consumes
/// one slot in the shared 64-file limit and reports the `programs` group.
#[test]
#[trace("TC-1210", "FR-027-AC-14", "FR-027-AC-10")]
fn compile_program_array_file_count_refusal_names_the_programs_group() {
    let directory = tempfile::tempdir().unwrap();
    let sources = (0..65)
        .map(|index| {
            json!({"file":format!("missing-{index}.native"),"authority":"agent-ix",
                "identity":format!("test:unit-{index}"),"document":"Unit","formal_revision":1})
        })
        .collect::<Vec<_>>();
    let request = json!({"format":"native-compile/1","request":{
        "models":[],"program":sources}});
    let path = directory.path().join("compile.json");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();

    let error = quire_spec_language::command::compile(&path).unwrap_err();
    let value = error.value().unwrap();
    assert_eq!(error.category().exit_code(), 22);
    assert_eq!(value["code"], "resource_exhausted", "{value}");
    assert_eq!(value["details"], json!({
        "limit":"selected files","category":"programs",
        "requested":65,"remaining":64,"maximum":64
    }));
}

/// FR-026-AC-4 (TC-104): a clause run request selecting more observation files
/// than the shared file limit allows refuses `resource_exhausted` with the
/// category spelling `observations`, before any file is read.
#[test]
#[trace("TC-104", "FR-026-AC-4")]
fn clause_run_file_count_refusal_names_the_observations_group() {
    let directory = tempfile::tempdir().unwrap();
    let observations: serde_json::Map<String, Value> = (0..64)
        .map(|index| (format!("o{index}"), json!(format!("missing-{index}.json"))))
        .collect();
    let request = json!({"format":"native-run/1","request":{
        "models":[],
        "program":{"source":{"file":"missing.native","authority":"agent-ix",
            "identity":"test:count","document":"Count","formal_revision":1}},
        "clause":{"selection":{"clause":{"name":"C","observation":"o0",
            "self":{"population":"ix://p/q","key":"k"},
            "anchor":{"kind":"handler","name":"h"}}},
            "observations":observations}}});
    std::fs::write(
        directory.path().join("request.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_quire-spec"))
        .arg("run")
        .arg(directory.path().join("request.json"))
        .output()
        .unwrap();
    let failure = refusal(&output, 22, "resource_exhausted", "observations");
    assert_eq!(
        failure["details"],
        json!({"limit":"selected files","category":"observations",
            "requested":64,"remaining":63,"maximum":64})
    );
}

/// FR-010-AC-3 (TC-015): the native `lower` command is retired; it is an
/// unknown command that exits 20 without reading its operand.
#[test]
#[trace("TC-015", "FR-010-AC-3")]
fn lower_is_an_unknown_command() {
    let directory = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_quire-spec"))
        .args(["lower", "missing.json"])
        .current_dir(directory.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(20));
    assert!(output.stdout.is_empty());
    assert_eq!(
        std::str::from_utf8(&output.stderr).unwrap(),
        "unknown command: lower\n"
    );
}

/// FR-027-AC-5 (TC-435 step 2): a `1-draft` compile validates the program's
/// `document` and `formal_revision` but the v2 wire does not record them,
/// so two requests differing only there write identical bytes; an invalid
/// `document` still refuses.
#[test]
#[trace("TC-435", "FR-027-AC-5")]
fn formal_labels_are_validated_but_not_recorded_by_a_spine_compile() {
    let program = std::fs::read(SPINE_FIXTURE).unwrap();
    let directory = tempfile::tempdir().unwrap();
    spine_request(directory.path(), &program);
    let first = compile(directory.path());
    assert_eq!(first.status.code(), Some(0));
    let path = directory.path().join("compile.json");
    let mut job: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    job["request"]["program"]["source"]["document"] = json!("Elsewhere");
    job["request"]["program"]["source"]["formal_revision"] = json!(9);
    std::fs::write(&path, serde_json::to_vec(&job).unwrap()).unwrap();
    let second = compile(directory.path());
    assert_eq!(
        second.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );
    assert!(!first.stdout.is_empty());
    assert_eq!(first.stdout, second.stdout);
    job["request"]["program"]["source"]["document"] = json!("");
    std::fs::write(&path, serde_json::to_vec(&job).unwrap()).unwrap();
    let invalid = compile(directory.path());
    assert_eq!(invalid.status.code(), Some(20));
    assert!(invalid.stdout.is_empty());
    let failure: Value = serde_json::from_slice(&invalid.stderr).unwrap();
    assert_eq!(failure["stage"], "request", "{failure}");
}

/// FR-493-AC-2 / FR-027-AC-7 (TC-1276, TC-435): the CLI refuses both
/// the retired `0-draft` edition and an unrecognized edition at S1.
#[test]
#[trace("TC-1276", "FR-493-AC-2", "TC-435", "FR-027-AC-7")]
fn retired_and_unknown_editions_refuse_before_emitting_package_bytes() {
    for edition in ["0-draft", "7-draft"] {
        let program = SPINE_HEADER.replacen("1-draft", edition, 1)
            + "function f using v(): Int[0, 9] pure { 7 }\n";
        let directory = tempfile::tempdir().unwrap();
        spine_request(directory.path(), program.as_bytes());
        let output = compile(directory.path());
        assert_eq!(output.status.code(), Some(20), "{edition}");
        assert!(output.stdout.is_empty(), "{edition}");
        let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(failure["code"], "unknown_edition", "{failure}");
        assert_eq!(failure["stage"], "profile", "{failure}");
        assert!(failure["message"].as_str().unwrap().contains(edition));
        let literal = program.find(&format!("\"{edition}\"")).unwrap();
        assert_eq!(failure["details"]["span"]["start"]["byte"], literal);
        assert_eq!(
            failure["details"]["span"]["end"]["byte"],
            literal + edition.len() + 2
        );
    }
}

/// FR-027-AC-8 (TC-435 step 6): each spine stage's refusal reports that
/// stage and its cause's code, exits on that code, and writes no bytes.
#[test]
#[trace(
    "TC-105",
    "FR-027-AC-3",
    "TC-435",
    "FR-027-AC-8",
    "TC-785",
    "FR-027-AC-11"
)]
fn each_spine_stage_refusal_reports_its_stage_and_code() {
    for (body, stage, code, exit, located) in [
        (
            "function f using v(): Int[0, 9] pure { 7\n",
            "source",
            "invalid_syntax",
            20,
            None,
        ),
        (
            "synthesis Syn using v grammar M::G domain M::D satisfies { true };\n",
            "forms",
            "unsupported_construct",
            21,
            Some("synthesis Syn using v grammar M::G domain M::D satisfies { true };"),
        ),
        (
            "function f using v(p: Nope): Int[0, 9] pure { 1 }\n",
            "assembly",
            "missing_declaration",
            20,
            Some("Nope"),
        ),
        (
            "function f using v(): Int[0, 9] pure { true }\n",
            "check",
            "ill_typed",
            20,
            Some("true"),
        ),
        (
            "type Digit = Int[0, 9];\n",
            "emit",
            "unsupported_projection",
            21,
            None,
        ),
    ] {
        let text = format!("{SPINE_HEADER}{body}");
        let directory = tempfile::tempdir().unwrap();
        spine_request(directory.path(), text.as_bytes());
        let output = compile(directory.path());
        assert_eq!(output.status.code(), Some(exit), "{stage}");
        assert!(output.stdout.is_empty(), "{stage}");
        let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(failure["stage"], stage, "{failure}");
        assert_eq!(failure["code"], code, "{failure}");
        assert_eq!(failure["status"], "refused", "{failure}");
        assert_eq!(failure["details"]["path"], "program.native", "{failure}");
        let message = failure["message"].as_str().unwrap();
        assert!(!message.contains('{'), "a Debug dump: {message}");
        let span = &failure["details"]["span"];
        match located {
            Some(located) => {
                let start = SPINE_HEADER.len() + body.find(located).unwrap();
                assert_eq!(span["start"]["byte"], start, "{failure}");
                assert_eq!(span["end"]["byte"], start + located.len(), "{failure}");
            }
            None if stage == "emit" => assert!(span.is_null(), "{failure}"),
            None => assert!(span.is_object(), "{failure}"),
        }
    }
}

/// [`spine_request`], with `document` written as `orders.json` and selected
/// as a `semantic-ir/2.0.0` model.
fn spine_model_request(directory: &Path, program: &[u8], document: &[u8]) {
    spine_request(directory, program);
    std::fs::write(directory.join("orders.json"), document).unwrap();
    let path = directory.join("compile.json");
    let mut job: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    job["request"]["models"] = json!([{"format":"semantic-ir/2.0.0","source":{
        "file":"orders.json","authority":"agent-ix","identity":"acme/orders",
        "document":"Orders","formal_revision":1}}]);
    std::fs::write(&path, serde_json::to_vec(&job).unwrap()).unwrap();
}

/// FR-027-AC-9 (TC-442 step 2): a `1-draft` request selecting a domain
/// package document compiles its program through the spine. Stdout is
/// exactly the bytes `qsl_replay::spine`'s operations emit over the same source and
/// package input, and the lock and identity preimage select the domain
/// package by the identity the program's `model` declaration
/// names.
#[test]
#[trace("TC-442", "FR-027-AC-9", "TC-785", "FR-027-AC-11")]
fn a_complete_v1_request_with_a_domain_package_locks_its_model_selection() {
    // The complete-V1 compile fixture with a domain package (FR-027-AC-9).
    let program = crate::support::spine_model::unit().into_bytes();
    let document = crate::support::spine_model::DOCUMENT.to_vec();
    let directory = tempfile::tempdir().unwrap();
    spine_model_request(directory.path(), &program, &document);
    let output = compile(directory.path());
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let library = crate::support::front_end::emitted(
        qsl_foundation::SourceIdentity::new("agent-ix", "test:spine"),
        "program.native",
        &program,
        &qsl_semantics::model::intake::package_input([document.as_slice()]).unwrap(),
        &qsl_replay::spine::DependencyInput::default(),
    )
    .unwrap();
    assert_eq!(output.stdout, library.package().bytes());
    let wire: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(wire["lock"].get("model_selections").is_none());
}

/// FR-027-AC-9, FR-056-AC-9 (TC-442 step 4): the model fixture refuses at
/// the stage that owns each defect, at its region: `M::Nope`, which the
/// admitted package does not declare, at `assembly`; `deref(g).nope`, a
/// field neither `Gadget` nor its supertype declares, and `g = w` over a
/// `Gadget` and an unrelated `Rock` (FR-082 compares references of one
/// type only), at `check`; and the `model` declaration, when the request
/// supplies no domain package, at `intake`.
#[test]
#[trace("TC-442", "TC-136", "FR-027-AC-9", "FR-056-AC-9", "FR-049-AC-11")]
fn a_model_bearing_request_refuses_at_the_owning_stage() {
    let program = crate::support::spine_model::unit();
    let document = crate::support::spine_model::DOCUMENT.to_vec();
    let declaration = program
        .lines()
        .find(|line| line.starts_with("model M = "))
        .unwrap();
    for (text, supplied, stage, code, located) in [
        (
            program.replacen("deref(g).code", "deref(g).nope", 1),
            Some(document.clone()),
            "check",
            "ill_typed",
            "deref(g).nope".to_owned(),
        ),
        (
            program.replacen("w: M::Widget): Boolean", "w: M::Rock): Boolean", 1),
            Some(document.clone()),
            "check",
            "ill_typed",
            "g = w".to_owned(),
        ),
        (
            program.replacen("Reference<M::Widget>", "Reference<M::Nope>", 1),
            Some(document.clone()),
            "assembly",
            "missing_declaration",
            "M::Nope".to_owned(),
        ),
        (
            program.clone(),
            None,
            "intake",
            "missing_import",
            declaration.to_owned(),
        ),
    ] {
        let directory = tempfile::tempdir().unwrap();
        match &supplied {
            Some(supplied) => spine_model_request(directory.path(), text.as_bytes(), supplied),
            None => spine_request(directory.path(), text.as_bytes()),
        }
        let output = compile(directory.path());
        assert_eq!(output.status.code(), Some(20), "{stage} {code}");
        assert!(output.stdout.is_empty());
        let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(failure["stage"], stage, "{failure}");
        assert_eq!(failure["code"], code, "{failure}");
        let message = failure["message"].as_str().unwrap();
        assert!(!message.contains('{'), "a Debug dump: {message}");
        let start = text.find(&located).unwrap();
        let span = &failure["details"]["span"];
        assert_eq!(span["start"]["byte"], start, "{failure}");
        assert_eq!(span["end"]["byte"], start + located.len(), "{failure}");
    }
}

/// `test/geometry`'s source, a library `f` the importing program calls.
const GEOMETRY: &str = "function f using v(x: Int[0, 9]): Boolean pure { x < 5 }\n";

/// The source identity `geometry.native` is selected under.
fn geometry_identity() -> qsl_foundation::SourceIdentity {
    qsl_foundation::SourceIdentity::new("agent-ix", "test:geometry")
}

/// [`spine_request`] for a program importing `test/geometry`, with
/// `library` written as `geometry.native` and selected as that library
/// under `identity`.
fn spine_library_request(directory: &Path, library: &[u8], identity: &str) -> Vec<u8> {
    let program = format!(
        "{SPINE_HEADER}import \"test/geometry\" as g;\n\
         function u using v(x: Int[0, 9]): Boolean pure {{ g::f(x) }}\n"
    )
    .into_bytes();
    spine_request(directory, &program);
    std::fs::write(directory.join("geometry.native"), library).unwrap();
    let path = directory.join("compile.json");
    let mut job: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    job["request"]["libraries"] = json!([{"identity":identity,"source":{
        "file":"geometry.native","authority":"agent-ix","identity":"test:geometry",
        "document":"Geometry","formal_revision":1}}]);
    std::fs::write(&path, serde_json::to_vec(&job).unwrap()).unwrap();
    program
}

/// A proven model protocol refinement with an optional importing function,
/// used to test command forwarding of non-empty internal-list values.
fn refinement_unit(protocol: &str, refinement: &str, imports_library: bool) -> String {
    let function = if imports_library {
        "function entry using v(): Boolean pure { g::geometryFn(0) }\n"
    } else {
        "function geometryFn using v(x: Int[0, 9]): Boolean pure { true }\n"
    };
    let declaration = format!(
        "profile p = \"quire.protocol.finite-global/v1\";\n\
         protocol {protocol} using p over (c: Spec::Counter) on origin {{\n\
         role w on Spec::Counter; scheduling adversarial;\n\
         run sequence Main {{\n\
         attempt a1 by w on Spec::Counter::inc contracts [] as (r1: Spec::Counter) {{ true }};\n\
         attempt a2 by w on Spec::Counter::inc contracts [] as (r2: Spec::Counter) {{ true }};\n\
         }}\n\
         finish Done as (d: Boolean) {{ true }};\n\
         }}\n\
         refinement {refinement} using inf abstract Spec::{protocol} concrete Impl {{\n\
         population Spec::counters from Impl::counters;\n\
         object Spec::Counter from Impl::Counter {{ value = self.value; }}\n\
         step Impl::Counter::commitA -> Spec::Counter::inc(self);\n\
         step Impl::Counter::commitB -> Spec::Counter::inc(self);\n\
         step Impl::Counter::beginA -> stutter;\n\
         step Impl::Counter::beginB -> stutter;\n\
         step Impl::Counter::retryA -> stutter;\n\
         step Impl::Counter::retryB -> stutter;\n\
         step Impl::Counter::peek -> stutter;\n\
         }}\n"
    );
    let packages =
        crate::support::refinement_cas::packages(crate::support::refinement_cas::Variant::CAS);
    let mut unit = crate::support::refinement_cas::unit_text(
        &packages,
        &declaration,
        crate::support::refinement_cas::Variant::CAS,
    );
    if imports_library {
        unit = unit.replacen(
            "model Spec =",
            "import \"test/geometry\" as g;\nmodel Spec =",
            1,
        );
    }
    unit.push_str(function);
    unit
}

/// FR-027-AC-10 (TC-446 step 7): a `1-draft` request's `libraries` supply
/// the program's import. Stdout is exactly the bytes `qsl_replay::spine`'s
/// operations emit over the same program and dependency input.
#[test]
#[trace(
    "TC-446",
    "FR-027-AC-10",
    "TC-885",
    "FR-003-AC-9",
    "TC-785",
    "FR-027-AC-11"
)]
fn a_complete_v1_request_supplies_its_libraries_to_the_spine() {
    let library = format!("{SPINE_HEADER}{GEOMETRY}").into_bytes();
    crate::support::front_end::emitted(
        geometry_identity(),
        "geometry.native",
        &library,
        &std::collections::BTreeMap::new(),
        &qsl_replay::spine::DependencyInput::default(),
    )
    .expect("the supplied library retains its formatted checked identity");
    let directory = tempfile::tempdir().unwrap();
    let program = spine_library_request(directory.path(), &library, "test/geometry");
    let output = compile(directory.path());
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let dependencies =
        qsl_replay::spine::DependencyInput::new([qsl_replay::spine::SuppliedLibrary {
            identity: "test/geometry".to_owned(),
            source: geometry_identity(),
            path: "geometry.native".to_owned(),
            bytes: library,
        }])
        .unwrap();
    let compiled = crate::support::front_end::emitted(
        qsl_foundation::SourceIdentity::new("agent-ix", "test:spine"),
        "program.native",
        &program,
        &std::collections::BTreeMap::new(),
        &dependencies,
    )
    .unwrap();
    assert_eq!(output.stdout, compiled.package().bytes());
}

/// FR-027-AC-12 (TC-446): the command carries root and library internal-list
/// values into the combined spine inputs before package emission.
#[test]
#[trace("TC-446", "FR-027-AC-12")]
fn refinement_internals_compile_matches_the_spine_composition() {
    use crate::support::refinement_cas::{self, Variant};

    let documents = refinement_cas::documents(Variant::CAS);
    let program = refinement_unit("Twice", "CasTwice", true).into_bytes();
    let library = refinement_unit("Twice", "CasTwice", false).into_bytes();
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("program.native"), &program).unwrap();
    std::fs::write(directory.path().join("geometry.native"), &library).unwrap();
    for (index, document) in documents.iter().enumerate() {
        std::fs::write(
            directory.path().join(format!("model-{index}.json")),
            document,
        )
        .unwrap();
    }
    let job = json!({"format":"native-compile/1","request":{
        "models":[
            {"format":"semantic-ir/2.0.0","source":{"file":"model-0.json",
                "authority":"agent-ix","identity":refinement_cas::SPEC,"document":"Abstract","formal_revision":1}},
            {"format":"semantic-ir/2.0.0","source":{"file":"model-1.json",
                "authority":"agent-ix","identity":refinement_cas::CAS,"document":"Concrete","formal_revision":1}},
            {"format":"semantic-ir/2.0.0","source":{"file":"model-2.json",
                "authority":"agent-ix","identity":refinement_cas::THIRD,"document":"Third","formal_revision":1}}
        ],
        "program":{"source":{"file":"program.native","authority":"agent-ix",
            "identity":"test:spine","document":"Spine","formal_revision":1}},
        "refinement_internals":[{"refinement":"CasTwice","internal":["Spec::Twice::Done"]}],
        "libraries":[{"identity":"test/geometry","source":{"file":"geometry.native",
            "authority":"agent-ix","identity":"test:geometry","document":"Geometry","formal_revision":1},
            "refinement_internals":[{"refinement":"CasTwice","internal":["Spec::Twice::Done"]}]}]
    }});
    let path = directory.path().join("compile.json");
    std::fs::write(&path, serde_json::to_vec(&job).unwrap()).unwrap();

    let checked_internals = |root_internals: Option<Value>, library_internals: Option<Value>| {
        let dependencies =
            qsl_replay::spine::DependencyInput::new([qsl_replay::spine::SuppliedLibrary {
                identity: "test/geometry".to_owned(),
                source: geometry_identity(),
                path: "geometry.native".to_owned(),
                bytes: library.clone(),
            }])
            .unwrap();
        let dependencies = match library_internals {
            Some(value) => dependencies
                .with_refinement_internals([("test/geometry".to_owned(), value)])
                .unwrap(),
            None => dependencies,
        };
        let source = qsl_foundation::SourceIdentity::new("agent-ix", "test:spine");
        let cancel = quire_exact::Cancel::new();
        let limits = qsl_replay::spine::SpineLimits::default();
        let parse_requests = [qsl_replay::spine::ParseRequest {
            source: &source,
            path: "program.native",
            bytes: &program,
        }];
        qsl_replay::spine::preflight_unit_list(&parse_requests, &dependencies).unwrap();
        let parsed = parse_requests
            .iter()
            .map(|request| {
                qsl_replay::spine::parse(request, limits.source, &cancel)
                    .unwrap()
                    .into_value()
            })
            .collect();
        let admitted = qsl_replay::spine::admit_parsed_unit_list(parsed, &dependencies).unwrap();
        let packages =
            qsl_semantics::model::intake::package_input(documents.iter().map(Vec::as_slice))
                .unwrap();
        let models = qsl_replay::spine::select_units(
            &admitted,
            &packages,
            limits.model,
            limits.source,
            &cancel,
        )
        .unwrap()
        .into_value();
        let checked = qsl_replay::spine::check_units_with_refinement_internals(
            &admitted,
            "test:spine".to_owned(),
            root_internals,
            &models,
            &dependencies,
            limits,
            &cancel,
        )
        .unwrap()
        .into_value();
        let mut internal_lists = checked[0]
            .package()
            .graph()
            .refinements()
            .iter()
            .map(|refinement| {
                refinement
                    .abstract_internal()
                    .iter()
                    .map(|class| class.spelling().to_owned())
                    .collect::<Vec<_>>()
            })
            .filter(|list| !list.is_empty())
            .collect::<Vec<_>>();
        internal_lists.sort();
        internal_lists
    };
    let root_internals = json!([{"refinement":"CasTwice","internal":["Spec::Twice::Done"]}]);
    let library_internals = json!([{"refinement":"CasTwice","internal":["Spec::Twice::a1"]}]);
    assert_eq!(
        checked_internals(
        Some(root_internals.clone()),
        Some(library_internals.clone()),
        ),
        [
            vec!["Twice::Done".to_owned()],
            vec!["Twice::a1".to_owned()]
        ]
    );
    assert_eq!(
        checked_internals(None, Some(library_internals.clone())),
        [vec!["Twice::a1".to_owned()]]
    );
    assert_eq!(
        checked_internals(Some(root_internals.clone()), None),
        [vec!["Twice::Done".to_owned()]]
    );
    assert!(checked_internals(None, None).is_empty());

    let error = compile_in_process(directory.path()).unwrap_err();
    let value = error.value().unwrap();
    assert_eq!(value["stage"], "emit", "{value}");
    assert!(value.get("artifact").is_none(), "{value}");
}

/// FR-147-AC-10 (TC-552): an internal-list defect is attributed to the
/// unit's value or to the supplied library's own value, including when the
/// library is not imported by the unit.
#[test]
#[trace("TC-552", "FR-147-AC-10", "TC-446", "FR-027-AC-12")]
fn refinement_internal_list_defects_keep_their_input_owner() {
    let unit = tempfile::tempdir().unwrap();
    spine_request(
        unit.path(),
        format!("{SPINE_HEADER}function u using v(): Boolean pure {{ true }}\n").as_bytes(),
    );
    let path = unit.path().join("compile.json");
    let mut job: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    job["request"]["refinement_internals"] =
        json!([{"refinement":"NoSuchRefinement","internal":[]}]);
    std::fs::write(&path, serde_json::to_vec(&job).unwrap()).unwrap();
    let output = compile(unit.path());
    assert_eq!(output.status.code(), Some(20));
    assert!(output.stdout.is_empty());
    let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(failure["code"], "missing_declaration", "{failure}");
    assert!(!failure.to_string().contains("compile request"));

    let library = tempfile::tempdir().unwrap();
    spine_request(
        library.path(),
        format!("{SPINE_HEADER}function u using v(): Boolean pure {{ true }}\n").as_bytes(),
    );
    std::fs::write(
        library.path().join("geometry.native"),
        format!("{SPINE_HEADER}{GEOMETRY}"),
    )
    .unwrap();
    let path = library.path().join("compile.json");
    let mut job: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    job["request"]["libraries"] = json!([{"identity":"test/geometry","source":{
        "file":"geometry.native","authority":"agent-ix","identity":"test:geometry",
        "document":"Geometry","formal_revision":1},
        "refinement_internals":[{"refinement":"NoSuchRefinement","internal":[]}] }]);
    std::fs::write(&path, serde_json::to_vec(&job).unwrap()).unwrap();
    let output = compile(library.path());
    assert_eq!(output.status.code(), Some(20));
    assert!(output.stdout.is_empty());
    let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(failure["code"], "missing_declaration", "{failure}");
    assert!(failure.to_string().contains("test/geometry"), "{failure}");
    assert!(
        !failure.to_string().contains("compile request"),
        "{failure}"
    );
}

/// FR-027-AC-10 (TC-446 step 7): a library with an empty identity or a
/// `version` member refuses `invalid-request` with nothing written.
#[test]
#[trace("TC-446", "FR-027-AC-10")]
fn malformed_libraries_refuse_as_invalid_request() {
    let library = format!("{SPINE_HEADER}{GEOMETRY}").into_bytes();
    let empty_identity = tempfile::tempdir().unwrap();
    spine_library_request(empty_identity.path(), &library, "");
    let output = compile(empty_identity.path());
    assert_eq!(output.status.code(), Some(20));
    assert!(output.stdout.is_empty());
    let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(failure["code"], "invalid-request", "{failure}");

    // A library carries no version: the member is not part of the request.
    let versioned = tempfile::tempdir().unwrap();
    spine_library_request(versioned.path(), &library, "test/geometry");
    let path = versioned.path().join("compile.json");
    let mut job: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    job["request"]["libraries"][0]["version"] = json!("1");
    std::fs::write(&path, serde_json::to_vec(&job).unwrap()).unwrap();
    let output = compile(versioned.path());
    assert_eq!(output.status.code(), Some(20));
    assert!(output.stdout.is_empty());
    let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(failure["code"], "invalid-request", "{failure}");
}

/// FR-027-AC-10: a library's own refusal is reported through the program's
/// import and rendered over the library's source, not the program's.
#[test]
#[trace("TC-446", "FR-027-AC-10")]
fn a_library_refusal_renders_over_the_library_source() {
    let body = "function f using v(x: Int[0, 9]): Boolean pure { x < true }\n";
    let library = format!("{SPINE_HEADER}{body}").into_bytes();
    let directory = tempfile::tempdir().unwrap();
    spine_library_request(directory.path(), &library, "test/geometry");
    let output = compile(directory.path());
    assert_eq!(output.status.code(), Some(20));
    assert!(output.stdout.is_empty());
    let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(failure["details"]["path"], "geometry.native", "{failure}");
    let start = SPINE_HEADER.len() + body.find("x < true").unwrap();
    let end = (start + "x < true".len()) as u64;
    let span = &failure["details"]["span"];
    let (from, to) = (
        span["start"]["byte"].as_u64().unwrap(),
        span["end"]["byte"].as_u64().unwrap(),
    );
    assert!(from >= start as u64 && to <= end, "{failure}");
}

/// A `libraries` row selecting `file` as `identity`, under the
/// source identity `test:<file>`.
fn library_row(file: &str, identity: &str) -> Value {
    json!({"identity":identity,"source":{
        "file":file,"authority":"agent-ix","identity":format!("test:{file}"),
        "document":"Library","formal_revision":1}})
}

/// FR-027-AC-10: an import cycle between two supplied libraries is a
/// closure-level refusal located inside a library, so it renders against
/// that library's file and keeps its span, not against the program.
#[test]
#[trace("TC-446", "FR-027-AC-10")]
fn a_cycle_inside_the_libraries_renders_over_the_library_source() {
    let import = |identity: &str| {
        format!(
            "import \"{identity}\" as x;\n\
                 function h using v(): Boolean pure {{ true }}\n"
        )
    };
    let a = format!("{SPINE_HEADER}{}", import("test/b")).into_bytes();
    let b = format!("{SPINE_HEADER}{}", import("test/a")).into_bytes();
    // The program and library source text are distinct.
    let program = format!(
        "{SPINE_HEADER}{}function u using v(): Boolean pure {{ false }}\n",
        import("test/a")
    )
    .into_bytes();
    let directory = tempfile::tempdir().unwrap();
    spine_request(directory.path(), &program);
    std::fs::write(directory.path().join("a.native"), &a).unwrap();
    std::fs::write(directory.path().join("b.native"), &b).unwrap();
    let path = directory.path().join("compile.json");
    let mut job: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    job["request"]["libraries"] = json!([
        library_row("a.native", "test/a"),
        library_row("b.native", "test/b")
    ]);
    std::fs::write(&path, serde_json::to_vec(&job).unwrap()).unwrap();
    let output = compile(directory.path());
    assert_eq!(output.status.code(), Some(20));
    assert!(output.stdout.is_empty());
    let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(failure["code"], "invalid_package", "{failure}");
    let file = failure["details"]["path"].as_str().unwrap();
    let text = match file {
        "a.native" => &a,
        "b.native" => &b,
        _ => panic!("the cycle renders against a library file: {failure}"),
    };
    let start = usize::try_from(
        failure["details"]["span"]["start"]["byte"]
            .as_u64()
            .unwrap(),
    )
    .unwrap();
    assert!(
        start >= SPINE_HEADER.len() && start < text.len(),
        "{failure}"
    );
}

/// FR-027-AC-10: library sources count toward the request's `programs`
/// file limit (QSL-265), so 64 libraries beside the program refuse.
#[test]
#[trace("TC-105", "FR-027-AC-13", "TC-446", "FR-027-AC-10")]
fn sixty_four_libraries_exceed_the_file_limit() {
    let library = format!("{SPINE_HEADER}{GEOMETRY}").into_bytes();
    let directory = tempfile::tempdir().unwrap();
    spine_library_request(directory.path(), &library, "test/geometry");
    let path = directory.path().join("compile.json");
    let mut job: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let row = job["request"]["libraries"][0].clone();
    job["request"]["libraries"] = json!(vec![row; 64]);
    std::fs::write(&path, serde_json::to_vec(&job).unwrap()).unwrap();
    let output = compile(directory.path());
    assert_eq!(output.status.code(), Some(22));
    assert!(output.stdout.is_empty());
    let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(failure["code"], "resource_exhausted", "{failure}");
    assert_eq!(failure["details"]["limit"], "selected files", "{failure}");
    assert_eq!(failure["details"]["category"], "programs", "{failure}");
    assert_eq!(failure["details"]["requested"], 65, "{failure}");
}

/// FR-027-AC-13 (TC-105 step 6): a supplied library source is charged to the
/// compile request's fixed dependent-byte ceiling, with no limit override.
#[test]
#[trace("TC-105", "FR-027-AC-13")]
fn a_library_source_over_the_fixed_dependent_byte_ceiling_is_refused() {
    let program = format!(
        "{SPINE_HEADER}import \"test/geometry\" as g;\n\
         function u using v(x: Int[0, 9]): Boolean pure {{ g::f(x) }}\n"
    )
    .into_bytes();
    let base_library = format!("{SPINE_HEADER}{GEOMETRY}");
    let directory = tempfile::tempdir().unwrap();
    let base = spine_library_request(directory.path(), base_library.as_bytes(), "test/geometry");
    assert_eq!(base, program);

    let library_size = 8_388_609 - program.len() - base_library.len();
    let padded_library = format!("{base_library}//{}", "x".repeat(library_size - 2));
    std::fs::write(directory.path().join("geometry.native"), padded_library).unwrap();

    let error = compile_in_process(directory.path()).unwrap_err();
    let value = error.value().unwrap();
    assert_eq!(value["code"], "resource_exhausted", "{value}");
    assert_eq!(value["stage"], "intake", "{value}");
    assert_eq!(value["details"]["limit"], "dependent_bytes", "{value}");
    assert_eq!(value["details"]["maximum"], 8_388_608, "{value}");
    assert!(value["details"].get("option").is_none(), "{value}");
    assert!(value.get("artifact").is_none(), "{value}");
}
