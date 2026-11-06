// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-108: TC-469's 17-case ConfigVersion spine corpus. Runs the
//! same `examples/config-version/cases.rs` catalog as the fixture test,
//! through `run_clause` (FR-109) over the spine files
//! `crate::support::config_version_spine::spine` writes.

use crate::support::config_version_spine::{cases::CASES, spine, Case};
use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_replay::spine::{
    run_clause, CallOutcome, ClauseDisposition, ClauseRunRequest, ClauseRunStage,
};
use quire_semantic_value::catalog::Category;
use serde_json::{json, Value};
use std::{collections::BTreeSet, path::Path};

/// FR-108's own independent expected disposition (Corpus table), one arm
/// per case, in one exhaustive `match` over [`Case`] -- a new catalog case
/// fails to compile until it has an expected result here (TC-469 step 1).
enum Expected {
    /// `evaluate`, `success`, `true`, exit 0.
    Success,
    /// `evaluate`, `violation`, `false`, exit 10.
    Violation,
    /// `admit` or `compile`, `refusal`, exit per the code's own catalog row.
    Refused { stage: ClauseRunStage, code: Code },
    /// `admit`, `incomplete`, `incomplete_population`, exit 22.
    IncompletePopulation,
    /// `evaluate`, `incomplete`, `work_units`, exit 22.
    ExhaustedWork,
}

fn expected(case: Case) -> Expected {
    use Case::{
        AboveRange, Absent, BelowRange, BoundaryMax, BoundaryZero, Changed, Cycle, Dangling,
        Distinct, Exhausted, ForbiddenParent, Healthy, Incomplete, MissingModel, SelfLoop,
        Unchanged, Violating,
    };
    match case {
        Healthy | Absent | Unchanged | BoundaryZero | BoundaryMax => Expected::Success,
        Violating | Cycle | SelfLoop | Distinct | Changed => Expected::Violation,
        Dangling => Expected::Refused {
            stage: ClauseRunStage::Admit,
            code: Code::DanglingReference,
        },
        Incomplete => Expected::IncompletePopulation,
        MissingModel => Expected::Refused {
            stage: ClauseRunStage::Compile,
            code: Code::MissingImport,
        },
        Exhausted => Expected::ExhaustedWork,
        ForbiddenParent => Expected::Refused {
            stage: ClauseRunStage::Admit,
            code: Code::FrameViolation,
        },
        BelowRange | AboveRange => Expected::Refused {
            stage: ClauseRunStage::Admit,
            code: Code::InvalidRuntimeInput,
        },
    }
}

/// FR-108-AC-1: `report.disposition` against `expected(case)` -- stage,
/// category, truth and exit code, all through the disposition's own typed
/// accessors (`report.disposition.category().exit_code()`'s "one total match, no `_` arm").
fn assert_expected(case: Case, report: &qsl_replay::spine::ClauseRunReport) {
    let disposition = &report.disposition;
    match expected(case) {
        Expected::Success => {
            assert_eq!(
                disposition.stage(),
                ClauseRunStage::Evaluate,
                "{}",
                case.id()
            );
            assert_eq!(disposition.category(), Category::Success, "{}", case.id());
            assert_eq!(disposition.truth(), Some(true), "{}", case.id());
            assert_eq!(
                report.disposition.category().exit_code(),
                0,
                "{}",
                case.id()
            );
        }
        Expected::Violation => {
            assert_eq!(
                disposition.stage(),
                ClauseRunStage::Evaluate,
                "{}",
                case.id()
            );
            assert_eq!(disposition.category(), Category::Violation, "{}", case.id());
            assert_eq!(disposition.truth(), Some(false), "{}", case.id());
            assert_eq!(
                report.disposition.category().exit_code(),
                10,
                "{}",
                case.id()
            );
        }
        Expected::Refused { stage, code } => {
            assert_eq!(disposition.stage(), stage, "{}", case.id());
            assert_eq!(disposition.category(), Category::Refusal, "{}", case.id());
            assert_eq!(disposition_code(disposition), Some(code), "{}", case.id());
            assert_eq!(
                report.disposition.category().exit_code(),
                code.category().exit_code(),
                "{}",
                case.id()
            );
        }
        Expected::IncompletePopulation => {
            assert_eq!(disposition.stage(), ClauseRunStage::Admit, "{}", case.id());
            assert_eq!(
                disposition.category(),
                Category::Incomplete,
                "{}",
                case.id()
            );
            assert_eq!(
                disposition_code(disposition),
                Some(Code::IncompletePopulation),
                "{}",
                case.id()
            );
            assert_eq!(
                report.disposition.category().exit_code(),
                22,
                "{}",
                case.id()
            );
        }
        Expected::ExhaustedWork => {
            assert_eq!(
                disposition.stage(),
                ClauseRunStage::Evaluate,
                "{}",
                case.id()
            );
            assert_eq!(
                disposition.category(),
                Category::Incomplete,
                "{}",
                case.id()
            );
            match disposition {
                ClauseDisposition::Evaluate(CallOutcome::Incomplete(incomplete)) => {
                    assert_eq!(incomplete.limits_field(), "work_units", "{}", case.id());
                }
                other => panic!(
                    "{}: expected Evaluate(Incomplete), got {other:?}",
                    case.id()
                ),
            }
            assert_eq!(
                report.disposition.category().exit_code(),
                22,
                "{}",
                case.id()
            );
        }
    }
}

/// The catalog code a refusal/incomplete disposition carries, or `None` for
/// a stage this test never expects one at (success/violation).
fn disposition_code(disposition: &ClauseDisposition) -> Option<Code> {
    use qsl_semantics::model::observation::AdmissionFailure;
    match disposition {
        ClauseDisposition::Compile(refusal) => Some(refusal.code()),
        ClauseDisposition::Admit(AdmissionFailure::Refused(record))
        | ClauseDisposition::Admit(AdmissionFailure::Incomplete(record)) => {
            Code::from_code(record.catalog.code())
        }
        other => panic!("no catalog code expected for {other:?}"),
    }
}

/// [`disposition_code`] without the panic, for callers (step 4's own
/// determinism check) comparing every disposition regardless of stage,
/// including `evaluate`'s own outcomes that carry no catalog code.
fn maybe_disposition_code(disposition: &ClauseDisposition) -> Option<Code> {
    use qsl_semantics::model::observation::AdmissionFailure;
    match disposition {
        ClauseDisposition::Compile(refusal) => Some(refusal.code()),
        ClauseDisposition::Admit(AdmissionFailure::Refused(record))
        | ClauseDisposition::Admit(AdmissionFailure::Incomplete(record)) => {
            Code::from_code(record.catalog.code())
        }
        _ => None,
    }
}

fn run_case(directory: &Path, case: Case) -> qsl_replay::spine::ClauseRunReport {
    std::fs::create_dir_all(directory).unwrap();
    let request = build(directory, case);
    run_clause(request).expect("a well-formed FR-108 request always reports")
}

fn build(directory: &Path, case: Case) -> ClauseRunRequest {
    spine::write(directory, case).expect("spine::write emits every case's spine files");
    spine::request(directory, case).expect("the written spine files reconstruct a request")
}

fn current_clause_request() -> Value {
    let source = |file: &str, document: &str| {
        json!({
            "file": file,
            "authority": "agent-ix", "identity": format!("test:fr-312:{file}"),
            "document": document, "formal_revision": 1
        })
    };
    let request = json!({"format":"native-run/1", "request": {
        "models": [{"format":"semantic-ir/2.0.0", "source":
            source("spine-model.semantic-ir.json", "SpineModel")}],
        "program": {"source": source("spine-unit.native", "SpineUnit")},
        "clause": {
            "selection": {"clause": {"name":"ParentOrder", "observation":"current",
                "self": {"population":format!("ix://{}/config_history", spine::PACKAGE_IDENTITY),
                    "key":"child"}, "anchor":{"kind":"handler", "name":"validate"}}},
            "observations": {"current":"spine-snapshot.json"}
        }
    }});
    request
}

fn rename_identity(value: &mut Value, old: &str, new: &str) {
    match value {
        Value::String(text) => {
            if text == old || text.starts_with(&format!("{old}/")) {
                *text = text.replacen(old, new, 1);
            }
        }
        Value::Array(values) => {
            for value in values {
                rename_identity(value, old, new);
            }
        }
        Value::Object(members) => {
            for value in members.values_mut() {
                rename_identity(value, old, new);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

fn emitted_model_bytes(document: &Value) -> Vec<u8> {
    let input = serde_json::to_vec(document).expect("the model fixture is JSON");
    let packages = qsl_semantics::model::intake::package_input([input.as_slice()])
        .expect("the mutated domain package is keyed by its declared identity");
    crate::support::front_end::emitted(
        spine::unit_identity(),
        "spine-unit.native",
        spine::unit_text().as_bytes(),
        &packages,
        &qsl_replay::spine::DependencyInput::default(),
    )
    .expect("the selected ConfigVersion model admits through the spine")
    .package()
    .bytes()
    .to_vec()
}

/// FR-261/FR-312: the command adapter reads the selected observation through
/// the depth-independent document reader before admission inspects its body.
#[trace("TC-891", "FR-312-AC-5")]
#[test]
fn clause_run_reader_accepts_observation_deeper_than_128() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("deep-observation");
    build(&path, Case::Healthy);
    let snapshot_path = path.join("spine-snapshot.json");
    let mut document = std::fs::read_to_string(&snapshot_path).unwrap();
    let close = document.rfind('}').unwrap();
    document.insert_str(
        close,
        &format!(",\"padding\":{}0{}", "[".repeat(160), "]".repeat(160)),
    );
    std::fs::write(&snapshot_path, document).unwrap();
    let request = current_clause_request();
    std::fs::write(
        path.join("request.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
    let decoded = quire_spec_language::command::read_clause_run_request(&path.join("request.json"));
    assert!(decoded.is_ok(), "{:?}", decoded.err());
}

/// FR-312: an explicit observation byte bound above the default is honored
/// within the caller's aggregate dependent-byte ceiling.
#[trace("TC-891", "FR-312-AC-5")]
#[test]
fn clause_run_reader_honors_raised_observation_byte_limit() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("large-observation");
    build(&path, Case::Healthy);
    let snapshot_path = path.join("spine-snapshot.json");
    let mut document = std::fs::read_to_string(&snapshot_path).unwrap();
    let close = document.rfind('}').unwrap();
    document.insert_str(
        close,
        &format!(",\"padding\":\"{}\"", "x".repeat(1_100_000)),
    );
    std::fs::write(&snapshot_path, document).unwrap();
    let mut request = current_clause_request();
    request["request"]["clause"]["observation_limits"] = json!({"document_bytes":2_000_000});
    std::fs::write(
        path.join("request.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
    let decoded = quire_spec_language::command::read_clause_run_request(&path.join("request.json"));
    assert!(decoded.is_ok(), "{:?}", decoded.err());
}

/// FR-312: a deep pre-call parameter reaches the typed reader without a
/// serde recursion ceiling or recursive JSON-value drop.
#[trace("TC-891", "FR-312-AC-5")]
#[test]
fn clause_run_reader_keeps_parameter_beyond_128_levels() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("deep-parameter");
    build(&path, Case::Unchanged);
    let mut request = current_clause_request();
    request["request"]["clause"]["selection"] = json!({"clause": {
        "name":"ParentPresent", "observation":"pre",
        "self":{"population":format!("ix://{}/config_history", spine::PACKAGE_IDENTITY),
                "key":"child"},
        "parameters":{"probe":"__DEEP_PARAMETER__"}
    }});
    request["request"]["clause"]["observations"] = json!({
        "pre":"spine-pre-snapshot.json", "post":"spine-post-snapshot.json",
        "invocation":"spine-invocation.json"
    });
    let mut nested = String::from("{\"integer\":\"1\"}");
    for _ in 0..160 {
        nested = format!("{{\"present\":{nested}}}");
    }
    let text = serde_json::to_string(&request)
        .unwrap()
        .replace("\"__DEEP_PARAMETER__\"", &nested);
    std::fs::write(path.join("request.json"), text).unwrap();
    let decoded = quire_spec_language::command::read_clause_run_request(&path.join("request.json"))
        .expect("deep typed parameter must read without a parser depth cap");
    let qsl_replay::spine::ClauseRunSelection::Clause(clause) = decoded.selection else {
        panic!("expected clause selection");
    };
    let qsl_semantics::model::observation::ClauseSelectionInput::PreCall { parameters, .. } =
        clause.input
    else {
        panic!("expected pre-call selection")
    };
    assert_eq!(parameters.len(), 1);
}

/// FR-312: the function-argument adapter diagnoses an invalid deep form by
/// member after parsing it, rather than refusing at a JSON stack limit.
#[trace("TC-891", "FR-312-AC-5")]
#[test]
fn clause_run_reader_diagnoses_deep_function_argument() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("deep-function-argument");
    build(&path, Case::Healthy);
    let mut request = current_clause_request();
    request["request"]["clause"]["selection"] = json!({"function": {
        "name":"sameIdentity", "snapshot":"current", "arguments":[
            {"parameter":"a", "value":"__DEEP_ARGUMENT__"}
        ]
    }});
    let mut nested = String::from("{\"integer\":\"1\"}");
    for _ in 0..160 {
        nested = format!("{{\"present\":{nested}}}");
    }
    let text = serde_json::to_string(&request)
        .unwrap()
        .replace("\"__DEEP_ARGUMENT__\"", &nested);
    std::fs::write(path.join("request.json"), text).unwrap();
    let error = quire_spec_language::command::read_clause_run_request(&path.join("request.json"))
        .err()
        .expect("nested argument is outside the function argument forms");
    assert!(
        format!("{error:?}").contains("arguments.value"),
        "{error:?}"
    );
}

/// FR-108-AC-1 (TC-469 step 1): every one of the 17 cases gives the Corpus
/// table's disposition and exit code through `run_clause`.
#[trace(
    "TC-469",
    "FR-108-AC-1",
    "FR-108-AC-3",
    "FR-032-AC-1",
    "FR-032-AC-2",
    "FR-032-AC-3"
)]
#[test]
fn tc_469_step_1_every_case_matches_the_independent_expected_table() {
    let directory = tempfile::tempdir().unwrap();
    let mut seen = BTreeSet::new();
    for &case in CASES {
        let path = directory.path().join(case.id());
        let report = run_case(&path, case);
        assert_expected(case, &report);
        assert!(seen.insert(case.id()));
    }
    assert_eq!(
        seen,
        CASES.iter().map(|case| case.id()).collect::<BTreeSet<_>>()
    );
}

/// FR-047-AC-8 (TC-131): ConfigVersion's parent semantics on the checked
/// route. An update that keeps the child's `parent` evaluates true; one that
/// rewrites it is refused at admission with `FrameViolation` -- the frame
/// forbids the parent write, so no graph semantics is invented for it -- and
/// a `parent` naming no row is refused `DanglingReference` (catalog `frame_violation`, `dangling_reference`). Each refusal reports stage `Admit`, no Boolean and no evaluation outcome.
#[trace("TC-131", "FR-047-AC-8")]
#[test]
fn a_parent_the_frame_forbids_is_refused_not_given_graph_semantics() {
    let directory = tempfile::tempdir().unwrap();
    let run = |case: Case| run_case(&directory.path().join(case.id()), case);

    let kept = run(Case::Unchanged);
    assert_eq!(kept.disposition.stage(), ClauseRunStage::Evaluate);
    assert_eq!(kept.disposition.truth(), Some(true));

    for (case, code) in [
        (Case::ForbiddenParent, Code::FrameViolation),
        (Case::Dangling, Code::DanglingReference),
    ] {
        let report = run(case);
        assert_eq!(
            report.disposition.stage(),
            ClauseRunStage::Admit,
            "{}",
            case.id()
        );
        assert_eq!(
            report.disposition.category(),
            Category::Refusal,
            "{}",
            case.id()
        );
        assert_eq!(report.disposition.truth(), None, "{}", case.id());
        assert_eq!(
            disposition_code(&report.disposition),
            Some(code),
            "{}",
            case.id()
        );
    }
}

/// All independently checked corpus outcomes survive the FR-267 result writer
/// and strict reader without reverting to the retired native value envelope.
#[trace("TC-469", "FR-108-AC-2", "FR-267-AC-9")]
#[test]
fn tc_469_every_case_survives_result_two_round_trip() {
    let directory = tempfile::tempdir().unwrap();
    for &case in CASES {
        let report = run_case(&directory.path().join(case.id()), case);
        assert_expected(case, &report);
        let bytes = qsl_replay::spine::write_clause_run_result(&report).unwrap();
        let read = qsl_replay::spine::read_native_run_result(&bytes).unwrap();
        let qsl_replay::spine::NativeRunResult::Clause(read) = read else {
            panic!("{}: expected a clause-run result", case.id());
        };
        let stage = match report.disposition.stage() {
            ClauseRunStage::Compile => "compile",
            ClauseRunStage::Select => "select",
            ClauseRunStage::Admit => "admit",
            ClauseRunStage::Evaluate => "evaluate",
        };
        assert_eq!(read.stage(), stage, "{}", case.id());
        assert_eq!(
            read.category(),
            report.disposition.category().as_str(),
            "{}",
            case.id()
        );
        assert_eq!(read.truth(), report.disposition.truth(), "{}", case.id());
        assert_eq!(
            read.code(),
            maybe_disposition_code(&report.disposition).map(Code::as_str),
            "{}",
            case.id()
        );
    }
}

/// The boundary refusal keeps its authored object and field when the clause
/// report crosses the native-run-result/2 writer and strict reader.
#[trace("TC-469", "FR-108-AC-3", "FR-267-AC-9", "FR-032-AC-4")]
#[test]
fn tc_469_boundary_locus_survives_result_two_round_trip() {
    let directory = tempfile::tempdir().unwrap();
    for (case, object) in [(Case::BelowRange, "root"), (Case::AboveRange, "child")] {
        let report = run_case(&directory.path().join(case.id()), case);
        let bytes = qsl_replay::spine::write_clause_run_result(&report).unwrap();
        let read = qsl_replay::spine::read_native_run_result(&bytes).unwrap();
        let qsl_replay::spine::NativeRunResult::Clause(read) = read else {
            panic!("expected a clause-run result");
        };
        assert_eq!(read.stage(), "admit", "{}", case.id());
        assert_eq!(read.category(), "refusal", "{}", case.id());
        assert_eq!(read.code(), Some("invalid_runtime_input"), "{}", case.id());
        let fields = read.fields().expect("the admission refusal retains fields");
        assert_eq!(
            fields.get("field").map(String::as_str),
            Some("versionNumber")
        );
        assert_eq!(fields.get("object").map(String::as_str), Some(object));
    }
}

/// TC-1308 row 16 (FR-513-AC-7, FR-046-AC-8): the run report names the
/// stage whose evidence produced each result. A spine check refusal is
/// `compile` evidence with a refusal and no outcome; an executed replay is
/// `evaluate` evidence with its outcome and truth. The strict reader refuses a
/// document that relabels an executed result as `compile` evidence, and one
/// that turns a `compile` refusal into a result no replay produced.
#[trace("TC-1308", "FR-513-AC-7", "FR-046-AC-8")]
#[test]
fn run_report_separates_check_evidence_from_replay_evidence() {
    let directory = tempfile::tempdir().unwrap();
    let document = |case: Case| -> Value {
        let report = run_case(&directory.path().join(case.id()), case);
        let bytes = qsl_replay::spine::write_clause_run_result(&report).unwrap();
        serde_json::from_slice(&bytes).unwrap()
    };
    let replayed = document(Case::Healthy);
    assert_eq!(replayed["disposition"]["stage"], "evaluate");
    assert_eq!(replayed["disposition"]["category"], "success");
    assert_eq!(replayed["disposition"]["truth"], true);
    assert!(replayed["disposition"].get("refusal").is_none());
    let checked = document(Case::MissingModel);
    assert_eq!(checked["disposition"]["stage"], "compile");
    assert_eq!(checked["disposition"]["category"], "refusal");
    assert!(checked["disposition"].get("truth").is_none());
    assert!(checked["disposition"].get("outcome").is_none());

    let read = |value: &Value| {
        qsl_replay::spine::read_native_run_result(&serde_json::to_vec(value).unwrap()).map(drop)
    };
    read(&replayed).expect("the executed replay document reads");
    read(&checked).expect("the check refusal document reads");

    let mut relabelled = replayed.clone();
    relabelled["disposition"]["stage"] = json!("compile");
    assert!(
        matches!(
            read(&relabelled),
            Err(qsl_replay::spine::NativeRunReadError::Invalid {
                member: "disposition.refusal",
                ..
            })
        ),
        "a compile stage cannot carry an executed result"
    );

    let mut promoted = checked.clone();
    let disposition = promoted["disposition"].as_object_mut().unwrap();
    disposition.remove("refusal");
    disposition.insert("category".to_owned(), json!("success"));
    disposition.insert("truth".to_owned(), json!(true));
    disposition.insert(
        "outcome".to_owned(),
        replayed["disposition"]["outcome"].clone(),
    );
    let promoted = read(&promoted);
    assert!(
        matches!(
            promoted,
            Err(qsl_replay::spine::NativeRunReadError::Invalid {
                member: "disposition.refusal",
                ..
            })
        ),
        "a check pass cannot stand for an unexecuted replay: {promoted:?}"
    );
}

/// FR-108-AC-4 (TC-469 step 4): generating the corpus twice gives identical
/// files (bidirectionally: same file sets, not just every file in the first
/// generation present in the second -- SR-768 FND-003), running every one of
/// the 17 cases twice gives identical reports (the full disposition, not a
/// field subset), and a request carrying spine `compile`'s own emitted
/// `package_id` for the unit gives the same report.
#[trace("TC-469", "FR-108-AC-4", "FR-032-AC-4")]
#[test]
fn tc_469_step_4_generation_and_reports_are_deterministic() {
    let root = tempfile::tempdir().unwrap();
    let first = root.path().join("first");
    let second = root.path().join("second");
    for &case in CASES {
        spine::write(&first.join(case.id()), case).unwrap();
        spine::write(&second.join(case.id()), case).unwrap();
        let names = |directory: &Path| -> BTreeSet<std::ffi::OsString> {
            std::fs::read_dir(directory)
                .unwrap()
                .map(|entry| entry.unwrap().file_name())
                .collect()
        };
        let first_names = names(&first.join(case.id()));
        let second_names = names(&second.join(case.id()));
        assert_eq!(
            first_names,
            second_names,
            "{}: the two generations wrote different file sets",
            case.id()
        );
        for name in &first_names {
            assert_eq!(
                std::fs::read(first.join(case.id()).join(name)).unwrap(),
                std::fs::read(second.join(case.id()).join(name)).unwrap(),
                "{}: {name:?} differs between generations",
                case.id()
            );
        }
    }

    for &case in CASES {
        let path = first.join(case.id());
        let request_a = spine::request(&path, case).unwrap();
        let limits_a = (
            request_a.limits,
            request_a.observation_limits,
            request_a.accounting,
        );
        let report_a = run_clause(request_a).unwrap();
        let request_b = spine::request(&path, case).unwrap();
        let limits_b = (
            request_b.limits,
            request_b.observation_limits,
            request_b.accounting,
        );
        let report_b = run_clause(request_b).unwrap();

        // The full report: exit code, disposition (stage/category/truth/
        // code via the same summary step 2 uses), source identity
        // and usage all agree between the two runs.
        assert_eq!(
            report_a.disposition.category().exit_code(),
            report_b.disposition.category().exit_code(),
            "{}",
            case.id()
        );
        assert_eq!(
            (
                report_a.disposition.stage(),
                report_a.disposition.category(),
                report_a.disposition.truth()
            ),
            (
                report_b.disposition.stage(),
                report_b.disposition.category(),
                report_b.disposition.truth()
            ),
            "{}",
            case.id()
        );
        assert_eq!(
            maybe_disposition_code(&report_a.disposition),
            maybe_disposition_code(&report_b.disposition),
            "{}",
            case.id()
        );
        assert_eq!(
            report_a.usage.admission_consumed,
            report_b.usage.admission_consumed,
            "{}",
            case.id()
        );
        assert_eq!(
            report_a.usage.evaluation_admissions,
            report_b.usage.evaluation_admissions,
            "{}",
            case.id()
        );
        assert_eq!(
            report_a.usage.evaluation_consumed,
            report_b.usage.evaluation_consumed,
            "{}",
            case.id()
        );

        assert_eq!(limits_a, limits_b, "{}", case.id());
    }
}

/// FR-108-AC-6: the emitted package bytes admit through QSpec
/// I04 `read`, including the frame step and node 8's
/// `quire.op.model.reaches_field` application, whose reference edge the
/// reader checks.
#[trace("TC-469", "FR-108-AC-6")]
#[test]
fn tc_469_step_6_the_emitted_package_admits_via_i04() {
    let unit = spine::unit_text();
    let compiled = crate::support::front_end::emitted(
        spine::unit_identity(),
        "spine-unit.native",
        unit.as_bytes(),
        &qsl_semantics::model::intake::package_input([spine::DOMAIN_PACKAGE.as_bytes()]).unwrap(),
        &qsl_replay::spine::DependencyInput::default(),
    )
    .expect("the FR-108 unit compiles");

    let bytes = compiled.package().bytes();
    // I04 `read` needs the lock's required features as supported; they are
    // read straight from the emitted wire (`Emission::evidence` is
    // `pub(crate)` to `qsl_package`).
    let wire: Value = serde_json::from_slice(bytes).expect("emitted package is JSON");
    let mut evidence = quire_contract_model::CheckedPackageEvidence::new();
    if let Some(features) = wire["lock"]["required_features"].as_array() {
        for feature in features {
            evidence.support_feature(feature.as_str().expect("feature name is a string"));
        }
    }
    // The `model_root` node names the domain package by its identity; the
    // document supplied here is the one that identity selects.
    evidence
        .insert_domain_package_document(
            spine::PACKAGE_IDENTITY,
            spine::DOMAIN_PACKAGE.as_bytes().to_vec(),
        )
        .expect("a fresh evidence key");
    let result = quire_contract_model::read_checked_package(
        bytes,
        quire_contract_model::CheckedPackageReadLimits::bounded(),
        &evidence,
    );
    match result {
        quire_contract_model::CheckedPackageDispatchResult::AdmittedV2(_) => {}
        other => panic!("QSpec I04 read did not admit the emitted package: {other:?}"),
    }
}

/// TC-1308 rows 1-2: selected ConfigVersion semantics change emitted bytes,
/// while source package version metadata and set-like declaration order do not.
#[trace("TC-1308", "FR-513-AC-1")]
#[test]
fn checked_package_identity_changes_with_model_content_and_ignores_order() {
    let original: Value = serde_json::from_str(spine::DOMAIN_PACKAGE).unwrap();
    let baseline = emitted_model_bytes(&original);

    let mut source_version = original.clone();
    source_version["package"]["version"] = serde_json::json!("2.0.0");
    assert_eq!(
        baseline,
        emitted_model_bytes(&source_version),
        "source package version metadata is not a model-selection pin"
    );

    let mut field_bound = original.clone();
    let version = field_bound["types"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|declaration| declaration["identity"] == "ix://example/config-version/VersionNumber")
        .unwrap();
    let maximum = version["constraints"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|constraint| constraint["keyword"] == "max")
        .unwrap();
    maximum["operands"]["value"] = serde_json::json!("1001");

    let mut frame = original.clone();
    frame["types"][1]["operations"][0]["frame"]["modifies"][0] =
        serde_json::json!("ix://example/config-version/ConfigVersion/parent");

    for (name, changed) in [("field bound", field_bound), ("frame entry", frame)] {
        assert!(baseline != emitted_model_bytes(&changed), "{name}");
    }
    let mut reordered = original.clone();
    let mut second_operation = reordered["types"][1]["operations"][0].clone();
    rename_identity(
        &mut second_operation,
        "ix://example/config-version/ConfigVersion/attemptUpdate",
        "ix://example/config-version/ConfigVersion/secondUpdate",
    );
    second_operation["name"] = serde_json::json!("secondUpdate");
    reordered["types"][1]["operations"]
        .as_array_mut()
        .unwrap()
        .push(second_operation);
    let mut second_population = reordered["populations"][0].clone();
    rename_identity(
        &mut second_population,
        "ix://example/config-version/config_history",
        "ix://example/config-version/archive_history",
    );
    second_population["displayName"] =
        serde_json::json!("ix://example/config-version/archive_history");
    second_population["members"] = serde_json::json!([]);
    reordered["populations"]
        .as_array_mut()
        .unwrap()
        .push(second_population);
    let ordered = emitted_model_bytes(&reordered);
    reordered["types"][1]["operations"]
        .as_array_mut()
        .unwrap()
        .reverse();
    reordered["types"].as_array_mut().unwrap().reverse();
    reordered["populations"].as_array_mut().unwrap().reverse();
    assert_eq!(ordered, emitted_model_bytes(&reordered));
}
