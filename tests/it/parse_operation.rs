// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-002, FR-010, FR-256: the `parse` operation called in process, and the
//! outcome document and FR-285 exit code it serializes to. These are the
//! library halves of the criteria the retired `quire-spec parse` command was
//! tested through; the frontend's arguments and streams are the driver's
//! (FR-287-AC-3 to AC-9).
use ix_trace_rs::trace;
use qsl_cst::Limits;
use qsl_foundation::diagnostic::Staged;
use qsl_foundation::source::SourceReadCause;
use qsl_foundation::{Source, SourceIdentity};
use qsl_replay::spine::{parse, FrontEndFailure, ParseRequest, ParsedUnitSource, SpineLimits};
use qsl_replay::OutcomeDocument;
use quire_exact::Cancel;
use quire_semantic_value::catalog::Category;
use serde_json::{json, Value};

const FIXTURE: &[u8] = include_bytes!("../fixtures/spine-compile.native");
const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\n";

fn parse_with(
    identity: &SourceIdentity,
    path: &str,
    bytes: &[u8],
    limits: Limits,
) -> (
    Result<Staged<ParsedUnitSource>, FrontEndFailure>,
    OutcomeDocument,
) {
    let result = parse(
        &ParseRequest {
            source: identity,
            path,
            bytes,
        },
        limits,
        &Cancel::new(),
    );
    let document = OutcomeDocument::from_parse(&result);
    (result, document)
}

fn document_value(document: &OutcomeDocument) -> Value {
    serde_json::from_slice(&document.to_bytes().expect("the document encodes"))
        .expect("the document is JSON")
}

/// FR-002-AC-1, FR-010-AC-1 (TC-012, TC-015): the fixture parses to an
/// admissible `ParsedSource` holding its exact bytes, and its outcome document
/// is `parse`, last stage S2, success, exit code 0.
#[test]
#[trace("TC-012", "FR-002-AC-1", "FR-002-AC-7")]
#[trace("TC-015", "FR-010-AC-1")]
fn the_fixture_parses_to_an_admissible_source_and_a_success_document() {
    let identity = SourceIdentity::new("agent-ix", "test:parent");
    let (result, document) = parse_with(
        &identity,
        "tests/fixtures/spine-compile.native",
        FIXTURE,
        Limits::default(),
    );
    let parsed = result.as_ref().expect("the fixture parses");
    let syntax = parsed.value().syntax();
    assert!(syntax.is_admissible(), "{:?}", syntax.diagnostics());
    assert!(syntax.diagnostics().is_empty());
    assert_eq!(parsed.value().source().text().as_bytes(), FIXTURE);
    assert_eq!(document.category(), Category::Success);
    assert_eq!(document.category().exit_code(), 0);
    assert_eq!(
        document_value(&document),
        json!({
            "format": "quire-outcome/1", "operation": "parse", "last_stage": "S2",
            "category": "success", "items": [], "diagnostics": [], "artifacts": [],
            "result": null
        })
    );
}

/// FR-010-AC-2, AC-9 (TC-015): a source S1 refuses as malformed exits 20 at
/// S1 with `invalid_syntax`; a construct S1 recognizes and the profile does
/// not support exits 21 at S2 with `unsupported_construct`.
#[test]
#[trace("TC-015", "FR-010-AC-2", "FR-010-AC-9")]
fn malformed_syntax_exits_20_and_an_unsupported_construct_exits_21() {
    let identity = SourceIdentity::new("agent-ix", "test:refused");
    for (declaration, exit, category, stage, code) in [
        (
            "function f using v(: Boolean pure { true }",
            20,
            "refusal",
            "S1",
            "invalid_syntax",
        ),
        (
            "function f using v(): Boolean pure { null }",
            21,
            "unsupported",
            "S2",
            "unsupported_construct",
        ),
    ] {
        let text = format!("{HEADER}{declaration}\n");
        let (result, document) =
            parse_with(&identity, "unit.native", text.as_bytes(), Limits::default());
        assert!(result.is_err(), "{code}");
        assert_eq!(document.category().exit_code(), exit, "{code}");
        let value = document_value(&document);
        assert_eq!(value["category"], category, "{code}");
        assert_eq!(value["last_stage"], stage, "{code}");
        assert_eq!(value["diagnostics"][0]["code"], code, "{code}");
    }
}

/// FR-010-AC-4 (TC-015): an exhausted parser request exits 22, incomplete.
#[test]
#[trace("TC-015", "FR-010-AC-4")]
fn an_exhausted_parse_exits_22() {
    let identity = SourceIdentity::new("agent-ix", "test:exhausted");
    let limits = Limits {
        source_bytes: FIXTURE.len() - 1,
        ..Limits::default()
    };
    let (_, document) = parse_with(&identity, "unit.native", FIXTURE, limits);
    assert_eq!(document.category(), Category::Incomplete);
    assert_eq!(document.category().exit_code(), 22);
    let value = document_value(&document);
    assert_eq!(value["last_stage"], "S1");
    assert_eq!(value["diagnostics"][0]["code"], "stage_limit_exceeded");
}

/// FR-010-AC-6 (TC-017): valid source at a non-UTF-8 Unix file path, read
/// through source intake, parses and carries the exact labels.
#[cfg(all(unix, not(target_os = "macos")))]
#[test]
#[trace("TC-017", "FR-010-AC-6")]
fn source_at_a_non_utf8_path_parses_with_the_exact_labels() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let temp = tempfile::tempdir().unwrap();
    let path = temp
        .path()
        .join(OsString::from_vec(b"source-\xff.native".to_vec()));
    std::fs::write(&path, FIXTURE).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let identity = SourceIdentity::new("agent-ix", "exact:é");
    let (result, document) = parse_with(
        &identity,
        &path.to_string_lossy(),
        &bytes,
        Limits::default(),
    );
    assert_eq!(document.category().exit_code(), 0);
    let parsed = result.unwrap();
    let reference = parsed.value().source().reference();
    assert_eq!(reference.authority(), "agent-ix");
    assert_eq!(reference.identity(), "exact:é");
}

/// FR-010-AC-7 (TC-015, TC-017): a missing or unreadable source path refuses
/// at source intake, exit code 20, before any parse. Through `command`, a
/// request naming a missing file and one naming a directory each refuse with
/// exit 20 and no package bytes.
#[test]
#[trace("TC-015", "FR-010-AC-7")]
#[trace("TC-017", "FR-010-AC-7")]
fn a_missing_or_unreadable_source_path_refuses_at_intake_with_exit_20() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::create_dir(directory.path().join("a-directory.native")).unwrap();
    for file in ["missing.native", "a-directory.native"] {
        let request = json!({"format":"native-compile/1","request":{"models":[],"program":{
            "source":{"file":file,"authority":"agent-ix","identity":"test:intake",
            "document":"Intake","formal_revision":1}}}});
        std::fs::write(
            directory.path().join("compile.json"),
            serde_json::to_vec(&request).unwrap(),
        )
        .unwrap();
        let output = crate::command_harness::compile(directory.path());
        assert_eq!(output.status.code(), Some(20), "{file}");
        assert!(output.stdout.is_empty(), "{file}");
        assert!(!output.stderr.is_empty(), "{file}");
    }
}

/// FR-010-AC-11 (TC-425): `parse` called in process names its source by
/// authority and identity only; a blank label refuses at source intake as
/// `invalid_source_identity`, cause `blank-label`, naming `identity`, before
/// any file is involved; `format` takes the same two labels.
#[test]
#[trace("TC-425", "FR-010-AC-11")]
fn parse_and_format_take_the_two_source_labels() {
    let identity = SourceIdentity::new("agent-ix", "specs/a.quire");
    let (result, _) = parse_with(
        &identity,
        "tests/fixtures/spine-compile.native",
        FIXTURE,
        Limits::default(),
    );
    assert_eq!(
        serde_json::to_value(result.unwrap().value().source().reference()).unwrap(),
        json!({"authority": "agent-ix", "identity": "specs/a.quire"})
    );

    // A refusal names the labels it was given, so a swap or a drop fails.
    let limits = Limits {
        tokens: 0,
        ..Limits::default()
    };
    let (_, limited) = parse_with(&identity, "unit.native", FIXTURE, limits);
    assert_eq!(limited.category().exit_code(), 22);
    assert_eq!(
        document_value(&limited)["diagnostics"][0]["locus"]["source"],
        json!({"authority": "agent-ix", "identity": "specs/a.quire"})
    );

    let blank = SourceIdentity::new("agent-ix", "");
    let refusal = Source::read_typed(blank, "missing.native", b"", usize::MAX)
        .expect_err("a blank label refuses");
    let fields = refusal.cause.identity_fields().expect("an identity cause");
    assert_eq!(fields.cause, "blank-label");
    assert_eq!(fields.label, Some("identity"));
    assert!(matches!(refusal.cause, SourceReadCause::BlankLabel { .. }));

    let value_format = std::fs::read_to_string("tests/fixtures/value-format.native").unwrap();
    let parsed = qsl_cst::parse(
        identity,
        "tests/fixtures/value-format.native",
        value_format.as_bytes(),
        Limits::default(),
    )
    .expect("the format fixture parses");
    let formatted = qsl_cst::format::format(&parsed).expect("admissible source formats");
    assert!(formatted.contains("// the larger coordinate"));
}

/// FR-256-AC-5 (TC-750): `parse` takes its S1 limits as given. The defaults
/// refuse a 100,000-deep source `stage_limit_exceeded` (exit 22); the limits
/// raised to fit parse it (exit 0). `s1.work_units` lowered below the
/// source's need refuses and raised admits it.
#[test]
#[trace("TC-750", "FR-256-AC-5")]
fn parse_takes_caller_limits_as_given() {
    let identity = SourceIdentity::new("test", "test:deep");
    let depth = 100_000;
    let text = format!(
        "{HEADER}function f using v(): Boolean pure {{ {}1{} }}\n",
        "(".repeat(depth),
        ")".repeat(depth)
    );
    let defaults = SpineLimits::default().source;
    let (_, refused) = parse_with(&identity, "deep.native", text.as_bytes(), defaults);
    assert_eq!(refused.category().exit_code(), 22);
    assert_eq!(
        document_value(&refused)["diagnostics"][0]["code"],
        "stage_limit_exceeded"
    );
    let raised = Limits {
        source_bytes: 10_000_000,
        tokens: 1_000_000,
        nodes: 10_000_000,
        work_units: 256,
    };
    let (_, parsed) = parse_with(&identity, "deep.native", text.as_bytes(), raised);
    assert_eq!(parsed.category().exit_code(), 0);
    assert_eq!(document_value(&parsed)["category"], "success");

    for (units, exit) in [(16, 22), (24, 0), (256, 0)] {
        let limits = Limits {
            work_units: units,
            ..defaults
        };
        let (_, document) = parse_with(&identity, "unit.native", FIXTURE, limits);
        assert_eq!(document.category().exit_code(), exit, "{units}");
        if exit == 22 {
            let value = document_value(&document);
            assert_eq!(value["diagnostics"][0]["cause"], "work-budget-exceeded");
            assert!(value["diagnostics"][0]["message"]
                .as_str()
                .unwrap()
                .contains("S1 work limit"));
        }
    }
}
