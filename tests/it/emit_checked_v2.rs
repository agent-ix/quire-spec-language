// SPDX-License-Identifier: AGPL-3.0-or-later
//! The `emit_checked_v2` runnable producer: it emits the bytes the spine's
//! producer emits for a source, those bytes read back through Contract IR's
//! strict checked-package v2 reader (the reader QSL's I2 read delegates to),
//! and a source the producer refuses exits nonzero with a diagnostic. Every
//! expectation is derived from the producer or the reader; no digest or byte
//! is frozen.

use std::ffi::OsString;
use std::path::Path;

use qsl_foundation::SourceIdentity;
use qsl_replay::spine::DependencyInput;
use quire_contract_model::{
    CheckedPackageEvidence, CheckedPackageReadLimits, CheckedPackageV2, CheckedPackageV2ReadResult,
};
use quire_spec_language::emit_checked_v2::{emit, run, Error, Request};
use serde_json::{json, Value};

#[path = "../../qsl-semantics/tests/support/tc235_k2.rs"]
#[allow(dead_code, reason = "this test uses part of the shared K2 builder")]
#[allow(
    clippy::duplicate_mod,
    reason = "a test binary also loads the K2 builder directly"
)]
mod base;

const NATIVE_BOOLEAN: &str = "ix://quire/native/Boolean";
const NATIVE_INTEGER: &str = "ix://quire/native/Integer";

fn args(items: &[&Path]) -> Vec<OsString> {
    items
        .iter()
        .map(|item| item.as_os_str().to_owned())
        .collect()
}

fn strings(items: &[&str]) -> Vec<OsString> {
    items.iter().map(OsString::from).collect()
}

/// IR's strict read of `bytes`, supplied `documents` as domain evidence.
fn read_back(bytes: &[u8], documents: &[&[u8]]) -> CheckedPackageV2 {
    let wire: Value = serde_json::from_slice(bytes).expect("the output is JSON");
    assert_eq!(wire["contract_version"], "quire.checked-package/v2");
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    for (digest, document) in qsl_semantics::model::intake::package_input(documents.iter().copied())
    {
        evidence
            .insert_domain_package_document(qsl_semantics::model::key::hex(&digest), document)
            .expect("a fresh evidence key");
    }
    match CheckedPackageV2::read(bytes, CheckedPackageReadLimits::bounded(), &evidence) {
        CheckedPackageV2ReadResult::Admitted(package) => *package,
        other => panic!("strict IR admission failed: {other:?}"),
    }
}

/// The producer's own bytes for `source` over `documents`, by the spine's
/// four operations rather than by the runnable output.
///
/// The bytes carry the source's labels and display path, so the oracle is
/// given the labels the runnable output derives from `path`.
fn producer_bytes(path: &Path, source: &str, documents: &[&[u8]]) -> Vec<u8> {
    crate::support::front_end::emitted(
        SourceIdentity::new("local", format!("source:{}", path.display()), "file", "1"),
        &path.display().to_string(),
        source.as_bytes(),
        &qsl_semantics::model::intake::package_input(documents.iter().copied()),
        &DependencyInput::default(),
    )
    .expect("the producer emits")
    .package()
    .bytes()
    .to_vec()
}

/// A model with a Boolean field `on` and an operation `set(flag)` that
/// modifies it (the shape of the derived Boolean model-field transition).
fn switch_document() -> Vec<u8> {
    let mut operation = base::operation("Switch", "set");
    operation["params"] = json!([{
        "identity": base::id("Switch/set/flag"), "name": "flag", "typeRef": NATIVE_BOOLEAN,
        "presence": "required", "nullable": false, "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": base::origin("Switch/set/flag"),
    }]);
    operation["frame"]["modifies"] = json!([base::id("Switch/on")]);
    let switch = base::declaration(
        "Switch",
        "thing",
        json!({
            "identityFields": [base::id("Switch/sid")],
            "fields": [
                base::field("Switch", "sid", NATIVE_INTEGER),
                base::field("Switch", "on", NATIVE_BOOLEAN),
            ],
            "operations": [operation],
        }),
    );
    let document = base::domain_document(
        vec![switch],
        vec![base::construct(
            "thing",
            qsl_semantics::model::intake::meaning::OBJECT_TYPE,
        )],
    );
    serde_json::to_vec(&document).unwrap()
}

fn switch_source(document: &[u8], clauses: &str) -> String {
    let packages = qsl_semantics::model::intake::package_input([document]);
    let digest = qsl_semantics::model::key::hex(packages.keys().next().unwrap());
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         profile p = \"quire.protocol.finite-global/v1\";\n\
         model M = {package:?} version \"1\" digest \"sha256-jcs:{digest}\";\n\
         {clauses}\n",
        package = base::PACKAGE,
    )
}

/// The checked-package v2 bytes of the spine fixtures, from the runnable
/// output, are the producer's bytes and read back admitted; `--out` writes the
/// same bytes the default stdout carries.
#[test]
fn the_output_is_the_producers_bytes_and_reads_back() {
    let source = std::fs::read_to_string("tests/fixtures/spine-model.native").unwrap();
    let document = std::fs::read("tests/fixtures/spine-model.semantic-ir.json").unwrap();
    let directory = tempfile::tempdir().unwrap();
    let out = directory.path().join("package.json");
    let model = Path::new("tests/fixtures/spine-model.semantic-ir.json");
    let program = Path::new("tests/fixtures/spine-model.native");

    let mut stdout = Vec::new();
    let mut command = strings(&["--model"]);
    command.extend(args(&[model, program]));
    run(&command, &mut stdout).expect("the fixture emits");
    assert!(!stdout.is_empty());
    assert_eq!(stdout, producer_bytes(program, &source, &[&document]));
    read_back(&stdout, &[&document]);

    let mut elsewhere = Vec::new();
    let mut command = strings(&["--out"]);
    command.extend(args(&[&out]));
    command.extend(strings(&["--model"]));
    command.extend(args(&[model, program]));
    run(&command, &mut elsewhere).expect("--out emits");
    assert!(elsewhere.is_empty(), "--out leaves stdout empty");
    assert_eq!(std::fs::read(&out).unwrap(), stdout);
}

/// The derived Boolean model-field transition (FR-105-AC-13) is emitted and
/// the package reads back admitted: the producer's `state_transition` node is
/// in the output, found by shape and not by a frozen id.
#[test]
fn a_boolean_model_field_transition_is_emitted_and_admitted() {
    let document = switch_document();
    let source = switch_source(&document, "post Q1 using v on M::Switch::set { self.on }\n");
    let directory = tempfile::tempdir().unwrap();
    let model = directory.path().join("switch.semantic-ir.json");
    let program = directory.path().join("switch.native");
    std::fs::write(&model, &document).unwrap();
    std::fs::write(&program, &source).unwrap();

    let bytes = emit(
        &Request::parse(&{
            let mut command = strings(&["--model"]);
            command.extend(args(&[&model, &program]));
            command
        })
        .unwrap(),
    )
    .expect("the transition form emits");
    assert_eq!(bytes, producer_bytes(&program, &source, &[&document]));
    read_back(&bytes, &[&document]);
    let wire: Value = serde_json::from_slice(&bytes).unwrap();
    let transitions = wire["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|node| node["node_tag"] == "state" && node["semantic_form"] == "transition")
        .count();
    assert_eq!(transitions, 1, "one derived transition for (set, on)");
}

/// A source the producer refuses (its `model` is not supplied) writes no bytes
/// and fails with the refusing stage's nonzero exit code; a malformed command
/// line and an unreadable file fail the same way.
#[test]
fn a_refused_source_writes_nothing_and_exits_nonzero() {
    let program = Path::new("tests/fixtures/spine-model.native");
    let mut stdout = Vec::new();
    let refusal = run(&args(&[program]), &mut stdout).expect_err("the model is not supplied");
    assert!(matches!(refusal, Error::Producer(_)), "{refusal:?}");
    assert_ne!(refusal.exit_code(), 0);
    assert!(!refusal.to_string().is_empty());
    assert!(stdout.is_empty());

    for bad in [
        strings(&[]),
        strings(&["--model"]),
        strings(&["--bogus", "x"]),
        strings(&["--library", "no-equals", "x"]),
        strings(&["a.native", "b.native"]),
    ] {
        let refusal = run(&bad, &mut stdout).expect_err("a malformed command line");
        assert!(matches!(refusal, Error::Usage), "{bad:?}: {refusal:?}");
        assert_ne!(refusal.exit_code(), 0);
    }
    let missing = run(
        &strings(&["/nonexistent/emit-checked-v2.native"]),
        &mut stdout,
    )
    .expect_err("an unreadable file");
    assert!(matches!(missing, Error::Read { .. }), "{missing:?}");
    assert_ne!(missing.exit_code(), 0);
    assert!(stdout.is_empty());
}
