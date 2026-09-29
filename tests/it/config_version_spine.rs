// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-108 (QSL-314): TC-469's 17-case ConfigVersion spine corpus. Runs the
//! same `examples/config-version/cases.rs` catalog `config_version.rs`'s
//! native test uses, through `run_clause` (FR-109) over the spine files
//! `crate::support::config_version::spine` writes beside the native ones.

use crate::support::config_version::{spine, Case};
use ix_trace_rs::trace;
use qsl_foundation::diagnostic::{Category, Code};
use qsl_replay::spine::{
    run_clause, CallOutcome, ClauseDisposition, ClauseRunRequest, ClauseRunStage,
};
use serde_json::Value;
use std::{collections::BTreeSet, path::Path, process::Command};

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
/// accessors (`report.exit_code()`'s "one total match, no `_` arm").
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
            assert_eq!(report.exit_code(), 0, "{}", case.id());
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
            assert_eq!(report.exit_code(), 10, "{}", case.id());
        }
        Expected::Refused { stage, code } => {
            assert_eq!(disposition.stage(), stage, "{}", case.id());
            assert_eq!(disposition.category(), Category::Refusal, "{}", case.id());
            assert_eq!(disposition_code(disposition), Some(code), "{}", case.id());
            assert_eq!(report.exit_code(), code.exit_code(), "{}", case.id());
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
            assert_eq!(report.exit_code(), 22, "{}", case.id());
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
                ClauseDisposition::Evaluate(CallOutcome::Incomplete { limit }) => {
                    assert_eq!(*limit, "work_units", "{}", case.id());
                }
                other => panic!(
                    "{}: expected Evaluate(Incomplete), got {other:?}",
                    case.id()
                ),
            }
            assert_eq!(report.exit_code(), 22, "{}", case.id());
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
        | ClauseDisposition::Admit(AdmissionFailure::Incomplete(record)) => Some(record.code),
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
        | ClauseDisposition::Admit(AdmissionFailure::Incomplete(record)) => Some(record.code),
        _ => None,
    }
}

fn run_case(directory: &Path, case: Case) -> qsl_replay::spine::ClauseRunReport {
    std::fs::create_dir_all(directory).unwrap();
    let request = build(directory, case);
    run_clause(request).expect("a well-formed FR-108 request always reports")
}

fn build(directory: &Path, case: Case) -> ClauseRunRequest {
    let model = crate::support::config_version::model().expect("native model admits");
    crate::support::config_version::write(directory, &model, case)
        .expect("fixtures::write emits every case's native and spine files");
    spine::request(directory, case).expect("the written spine files reconstruct a request")
}

/// FR-108-AC-1 (TC-469 step 1): every one of the 17 cases gives the Corpus
/// table's disposition and exit code through `run_clause`.
#[trace("TC-469", "FR-108-AC-1")]
#[test]
fn tc_469_step_1_every_case_matches_the_independent_expected_table() {
    let directory = tempfile::tempdir().unwrap();
    let mut seen = BTreeSet::new();
    for &case in crate::support::config_version::CASES {
        let path = directory.path().join(case.id());
        let report = run_case(&path, case);
        assert_expected(case, &report);
        assert!(seen.insert(case.id()));
    }
    assert_eq!(
        seen,
        crate::support::config_version::CASES
            .iter()
            .map(|case| case.id())
            .collect::<BTreeSet<_>>()
    );
}

/// One native `quire-spec run` outcome, read from whichever stream holds it.
struct NativeRun {
    exit: i32,
    value: Value,
}

fn run_native(directory: &Path) -> NativeRun {
    let output = Command::new(env!("CARGO_BIN_EXE_quire-spec"))
        .arg("run")
        .arg(directory.join("request.json"))
        .current_dir(directory.parent().unwrap())
        .output()
        .unwrap();
    let stdout = !output.stdout.is_empty();
    let bytes = if stdout {
        &output.stdout
    } else {
        &output.stderr
    };
    NativeRun {
        exit: output.status.code().unwrap(),
        value: serde_json::from_slice(bytes).unwrap(),
    }
}

/// Native's own `stage`/`status`/`truth`/code JSON fields, mapped onto
/// FR-109's spine vocabulary so [`tc_469_step_2_and_3_native_and_spine_agree_and_name_the_boundary_locus`]
/// can compare the two directly rather than by exit code alone (SR-768
/// FND-001): native `validate`→spine `admit`, native `link`/`quire`→spine
/// `compile`, native `evaluate`→spine `evaluate`.
struct NativeSummary {
    stage: ClauseRunStage,
    category: Category,
    truth: Option<bool>,
    /// Every catalog code native's own top-level `code` or `diagnostics[]`
    /// carries. Empty for `completed` (no code at all).
    codes: BTreeSet<String>,
}

fn native_summary(value: &Value) -> NativeSummary {
    let stage = match value["stage"].as_str().unwrap() {
        "validate" => ClauseRunStage::Admit,
        "link" | "quire" => ClauseRunStage::Compile,
        "evaluate" => ClauseRunStage::Evaluate,
        other => panic!("unknown native stage {other:?}: {value}"),
    };
    let truth = value.get("truth").and_then(Value::as_bool);
    let category = match value["status"].as_str().unwrap() {
        "completed" => {
            if truth.expect("a completed status always carries truth") {
                Category::Success
            } else {
                Category::Violation
            }
        }
        "refused" => Category::Refusal,
        "incomplete" => Category::Incomplete,
        other => panic!("unknown native status {other:?}: {value}"),
    };
    let mut codes = BTreeSet::new();
    if let Some(code) = value.get("code").and_then(Value::as_str) {
        codes.insert(code.to_owned());
    }
    if let Some(diagnostics) = value.get("diagnostics").and_then(Value::as_array) {
        codes.extend(
            diagnostics
                .iter()
                .map(|entry| entry["code"].as_str().unwrap().to_owned()),
        );
    }
    NativeSummary {
        stage,
        category,
        truth,
        codes,
    }
}

/// The same shape as [`native_summary`], read from spine's own report.
/// `codes` is `None` where spine's disposition carries no catalog code
/// comparable to native's own vocabulary (the `evaluate`-stage `work_units`
/// resource limit, which step 1 already pins precisely) -- comparing `None`
/// against native's codes would be meaningless, so that case skips the code
/// comparison rather than asserting an empty set.
fn spine_summary(
    report: &qsl_replay::spine::ClauseRunReport,
) -> (
    ClauseRunStage,
    Category,
    Option<bool>,
    Option<BTreeSet<String>>,
) {
    use qsl_semantics::model::observation::AdmissionFailure;
    let disposition = &report.disposition;
    let codes = match disposition {
        ClauseDisposition::Compile(refusal) => {
            Some(BTreeSet::from([refusal.code().as_str().to_owned()]))
        }
        ClauseDisposition::Admit(AdmissionFailure::Refused(record))
        | ClauseDisposition::Admit(AdmissionFailure::Incomplete(record)) => {
            Some(BTreeSet::from([record.code.as_str().to_owned()]))
        }
        _ => None,
    };
    (
        disposition.stage(),
        disposition.category(),
        disposition.truth(),
        codes,
    )
}

/// FR-108-AC-2 (TC-469 step 2): native `quire-spec run` and `run_clause`
/// agree on stage, category, truth, code and exit code for all 17 cases,
/// under FR-108's own stage/category map (native `validate`→spine `admit`,
/// native `link`→spine `compile`). Comparing more than the exit code alone
/// matters because exit 20 is shared by `dangling_reference`,
/// `frame_violation`, `invalid_runtime_input` and `missing_import` (SR-768
/// FND-001): a spine refusal with the wrong code at the wrong stage would
/// still pass an exit-code-only check.
/// FR-108-AC-3 (TC-469 step 3): `below-range`/`above-range` name exactly
/// `root`/`child` (not either, for either case -- SR-768 FND-002) and
/// `versionNumber` in both outputs' locus.
#[trace("TC-469", "FR-108-AC-2", "FR-108-AC-3", "FR-032-AC-1")]
#[test]
fn tc_469_step_2_and_3_native_and_spine_agree_and_name_the_boundary_locus() {
    let directory = tempfile::tempdir().unwrap();
    for &case in crate::support::config_version::CASES {
        let path = directory.path().join(case.id());
        let spine_request = build(&path, case);
        let spine_report = run_clause(spine_request).unwrap();
        let native = run_native(&path);
        assert_eq!(
            native.exit,
            spine_report.exit_code() as i32,
            "{}: native {} spine {:?}",
            case.id(),
            native.value,
            spine_report.disposition,
        );
        let native_summary = native_summary(&native.value);
        let (spine_stage, spine_category, spine_truth, spine_codes) = spine_summary(&spine_report);
        assert_eq!(
            native_summary.stage,
            spine_stage,
            "{}: stage; native {}",
            case.id(),
            native.value
        );
        assert_eq!(
            native_summary.category,
            spine_category,
            "{}: category; native {}",
            case.id(),
            native.value
        );
        assert_eq!(
            native_summary.truth,
            spine_truth,
            "{}: truth; native {}",
            case.id(),
            native.value
        );
        if let Some(spine_codes) = spine_codes {
            assert_eq!(
                native_summary.codes,
                spine_codes,
                "{}: code; native {}",
                case.id(),
                native.value
            );
        }
    }

    // The boundary object below-range/above-range's own object, taken from
    // native's `diagnostics[0].runtime.path` array entry that carries an
    // `object` key (verified live: `{"object":"root"}` for below-range,
    // `{"object":"child"}` for above-range), and from spine's own
    // `record.fields["object"]`. Neither side accepts the other case's
    // object.
    for case in [Case::BelowRange, Case::AboveRange] {
        let expected_object = match case {
            Case::BelowRange => "root",
            Case::AboveRange => "child",
            _ => unreachable!("only the two boundary cases are iterated"),
        };
        let path = directory.path().join(format!("{}-locus", case.id()));
        let spine_request = build(&path, case);
        let spine_report = run_clause(spine_request).unwrap();
        let native = run_native(&path);
        let runtime_path = native.value["diagnostics"][0]["runtime"]["path"]
            .as_array()
            .unwrap_or_else(|| panic!("{}: native locus {}", case.id(), native.value));
        let native_field = runtime_path
            .iter()
            .find_map(|entry| entry.get("field").and_then(Value::as_str));
        assert_eq!(
            native_field,
            Some("versionNumber"),
            "{}: native locus {}",
            case.id(),
            native.value
        );
        let native_object = runtime_path
            .iter()
            .find_map(|entry| entry.get("object").and_then(Value::as_str));
        assert_eq!(
            native_object,
            Some(expected_object),
            "{}: native locus {}",
            case.id(),
            native.value
        );

        let ClauseDisposition::Admit(qsl_semantics::model::observation::AdmissionFailure::Refused(
            record,
        )) = &spine_report.disposition
        else {
            panic!(
                "{}: expected Admit(Refused), got {:?}",
                case.id(),
                spine_report.disposition
            );
        };
        assert_eq!(record.code, Code::InvalidRuntimeInput, "{}", case.id());
        assert_eq!(
            record.fields.get("field").map(String::as_str),
            Some("versionNumber"),
            "{}: {:?}",
            case.id(),
            record.fields
        );
        assert_eq!(
            record.fields.get("object").map(String::as_str),
            Some(expected_object),
            "{}: {:?}",
            case.id(),
            record.fields
        );
    }
}

/// The two variants of a [`ClauseRunSelection`] compared for equality.
/// `ClauseRunSelection` itself carries no `PartialEq` (its `Function`
/// variant's `ClauseArgument`/`ClauseArgumentValue` don't either), so this
/// compares the `Clause` variant's own `PartialEq` and the `Function`
/// variant's `name`/`snapshot` (both `PartialEq`) plus its `arguments` by
/// `Debug` text -- a test-only comparator, not a reason to add derives to
/// production types nothing else needs them for.
fn selections_match(
    a: &qsl_replay::spine::ClauseRunSelection,
    b: &qsl_replay::spine::ClauseRunSelection,
) -> bool {
    use qsl_replay::spine::ClauseRunSelection::{Clause, Function};
    match (a, b) {
        (Clause(a), Clause(b)) => a == b,
        (
            Function {
                name: name_a,
                arguments: arguments_a,
                snapshot: snapshot_a,
            },
            Function {
                name: name_b,
                arguments: arguments_b,
                snapshot: snapshot_b,
            },
        ) => {
            name_a == name_b
                && snapshot_a == snapshot_b
                && format!("{arguments_a:?}") == format!("{arguments_b:?}")
        }
        _ => false,
    }
}

/// FR-108-AC-4 (TC-469 step 4): generating the corpus twice gives identical
/// files (bidirectionally: same file sets, not just every file in the first
/// generation present in the second -- SR-768 FND-003), running every one of
/// the 17 cases twice gives identical reports (the full disposition, not a
/// field subset), and a request carrying spine `compile`'s own emitted
/// `package_id` for the unit gives the same report.
#[trace("TC-469", "FR-108-AC-4")]
#[test]
fn tc_469_step_4_generation_and_reports_are_deterministic() {
    let root = tempfile::tempdir().unwrap();
    let first = root.path().join("first");
    let second = root.path().join("second");
    let model = crate::support::config_version::model().unwrap();
    for &case in crate::support::config_version::CASES {
        crate::support::config_version::write(&first.join(case.id()), &model, case).unwrap();
        crate::support::config_version::write(&second.join(case.id()), &model, case).unwrap();
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

    for &case in crate::support::config_version::CASES {
        let path = first.join(case.id());
        let request_a = spine::request(&path, case).unwrap();
        let limits_a = (
            request_a.limits,
            request_a.observation_limits,
            request_a.model_limits,
            request_a.accounting,
        );
        let report_a = run_clause(request_a).unwrap();
        let request_b = spine::request(&path, case).unwrap();
        let limits_b = (
            request_b.limits,
            request_b.observation_limits,
            request_b.model_limits,
            request_b.accounting,
        );
        let report_b = run_clause(request_b).unwrap();

        // The full report: exit code, disposition (stage/category/truth/
        // code via the same summary step 2 uses), source digest, package_id
        // and usage all agree between the two runs.
        assert_eq!(report_a.exit_code(), report_b.exit_code(), "{}", case.id());
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
            report_a.source_digest,
            report_b.source_digest,
            "{}",
            case.id()
        );
        assert_eq!(report_a.package_id, report_b.package_id, "{}", case.id());
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

        // Provenance names the source digest, package_id, the domain
        // package's own sha256-jcs digest, every observation's identity
        // and digest, the selection and the limits -- verified as content
        // (not vacuously present) and as agreeing between the two runs, so
        // the case is fixed by its inputs (FR-108-AC-4).
        assert!(!report_a.source_digest.is_empty(), "{}", case.id());
        assert_eq!(
            report_a.provenance.documents,
            report_b.provenance.documents,
            "{}",
            case.id()
        );
        assert_eq!(
            report_a.provenance.extraction,
            report_b.provenance.extraction,
            "{}",
            case.id()
        );
        assert!(
            selections_match(
                &report_a.provenance.selection,
                &report_b.provenance.selection
            ),
            "{}: {:?} vs {:?}",
            case.id(),
            report_a.provenance.selection,
            report_b.provenance.selection
        );
        assert_eq!(limits_a, limits_b, "{}", case.id());
        assert_eq!(
            report_a.provenance.model_selections,
            report_b.provenance.model_selections,
            "{}",
            case.id()
        );
        if matches!(case, Case::MissingModel) {
            assert!(
                report_a.provenance.model_selections.is_empty(),
                "{}: no package to name",
                case.id()
            );
        } else {
            assert!(
                report_a
                    .provenance
                    .model_selections
                    .iter()
                    .any(|selection| selection.identity == spine::PACKAGE_IDENTITY
                        && qsl_semantics::model::key::hex(&selection.digest)
                            == spine::model_digest_hex()),
                "{}: {:?}",
                case.id(),
                report_a.provenance.model_selections
            );
        }
        if matches!(
            report_a.disposition.stage(),
            ClauseRunStage::Evaluate | ClauseRunStage::Admit
        ) && !matches!(case, Case::Dangling)
        {
            assert!(
                !report_a.provenance.documents.is_empty(),
                "{}: an admitted case names every observation it read",
                case.id()
            );
        }

        // A request carrying spine `compile`'s own emitted `package_id`
        // gives the same report (FR-032-AC-4's package half). Only cases
        // that reach compile have one to pin.
        if let Some(package_id) = report_a.package_id {
            let mut pinned_request = spine::request(&path, case).unwrap();
            pinned_request.expected_package_id = Some(package_id);
            let pinned_report = run_clause(pinned_request).unwrap();
            assert_eq!(
                pinned_report.exit_code(),
                report_a.exit_code(),
                "{}",
                case.id()
            );
            assert_eq!(pinned_report.package_id, Some(package_id), "{}", case.id());
        }
    }
}

/// FR-108-AC-5 (TC-469 step 5): with `quire-extraction`, each case's unit
/// embedded in a Markdown fence and run through the I3 adapter gives the
/// direct run's disposition, a different source identity and digest, and
/// the extraction's original identity and digest in its provenance.
#[cfg(feature = "quire-extraction")]
mod extraction {
    use super::{build, run_clause, Case, ClauseRunRequest};
    use ix_trace_rs::trace;
    use qsl_foundation::{ByteDigest, Source, SourceIdentity};

    const QUIRE_PACKAGE: &str = "example/config-version-spine";

    fn original_identity(case: Case) -> SourceIdentity {
        SourceIdentity::new(
            "agent-ix",
            format!("ix://example/config-version/spine/markdown/{}", case.id()),
            "example",
            "1",
        )
    }

    fn body_identity(case: Case) -> SourceIdentity {
        SourceIdentity::new(
            "agent-ix",
            format!("ix://example/config-version/spine/extracted/{}", case.id()),
            "example",
            "1",
        )
    }

    /// The extracted body's own text and digest for `case`'s already-built
    /// `request`'s unit source.
    fn extracted(case: Case, request: &ClauseRunRequest) -> (qsl_source::ExtractedSource, String) {
        let qsl_replay::spine::ClauseRunSource::Program { bytes, .. } = &request.source else {
            unreachable!("fixtures always build a Program source")
        };
        let unit = String::from_utf8(bytes.clone()).unwrap();
        // Quire's `extract_clauses` (FR-071) only reads fences under a
        // `## Invariants` section, each owned by its own `### <clauseId>`
        // heading -- a bare `## <case>` heading with no `### ` clause id
        // leaves `sections.first()` empty and the whole document
        // `NotApplicable` (confirmed against the pinned quire-rs revision's
        // `src/semantic/clauses.rs::extract_clauses`/`level2_sections`).
        let clause_id = case.id().replace('-', "_");
        let markdown = format!(
            "# ConfigVersion spine corpus\n\n## Invariants\n\n### {clause_id}\n```ix:native\n{unit}```\n"
        );
        let original = Source::read(
            original_identity(case),
            "spine-rules.md",
            markdown.as_bytes(),
            markdown.len(),
        )
        .expect("the fixture document reads");
        let context = qsl_source::clause_context(QUIRE_PACKAGE, &original)
            .expect("the clause-only Quire context validates");
        let extracted = qsl_source::extract(
            original,
            &context,
            qsl_source::Selection {
                clause_id: case.id().replace('-', "_"),
                package: QUIRE_PACKAGE.to_owned(),
                body: body_identity(case),
            },
            qsl_source::Limits::default(),
        )
        .unwrap_or_else(|err| panic!("{}: the unit fence extracts: {err:?}", case.id()));
        (extracted, markdown)
    }

    /// FR-108-AC-5 (TC-469 step 5).
    #[trace("TC-469", "FR-108-AC-5")]
    #[test]
    fn tc_469_step_5_markdown_run_matches_the_direct_run_via_i3() {
        let directory = tempfile::tempdir().unwrap();
        for &case in crate::support::config_version::CASES {
            let path = directory.path().join(case.id());
            let direct_request = build(&path, case);
            let (extracted, markdown) = extracted(case, &direct_request);
            let direct_report = run_clause(build(&path, case)).unwrap();

            let mut extracted_request = build(&path, case);
            extracted_request.source = qsl_replay::spine::ClauseRunSource::Extracted(extracted);
            let extracted_report = run_clause(extracted_request).unwrap();

            assert_eq!(
                extracted_report.exit_code(),
                direct_report.exit_code(),
                "{}",
                case.id()
            );
            assert_eq!(
                extracted_report.disposition.stage(),
                direct_report.disposition.stage(),
                "{}",
                case.id()
            );
            assert_eq!(
                extracted_report.disposition.truth(),
                direct_report.disposition.truth(),
                "{}",
                case.id()
            );
            assert_ne!(
                extracted_report.provenance.source,
                direct_report.provenance.source,
                "{}",
                case.id()
            );
            assert_ne!(
                extracted_report.source_digest,
                direct_report.source_digest,
                "{}",
                case.id()
            );
            let origin = extracted_report
                .provenance
                .extraction
                .as_ref()
                .unwrap_or_else(|| panic!("{}: extraction origin present", case.id()));
            assert_eq!(origin.identity, original_identity(case), "{}", case.id());
            assert_eq!(
                origin.digest,
                ByteDigest::of(markdown.as_bytes()),
                "{}",
                case.id()
            );
        }
    }
}

/// Records every artifact reference found anywhere in `value` (an object
/// carrying `authority`/`identity`/`revision`/`digest_domain`/`digest`) into
/// `evidence` under its own claimed digest, recursing through arrays and
/// nested objects. Mirrors `qsl_package::emit::tests::locked_artifacts`'s
/// walk (private to that crate) so this I04 round-trip test can supply
/// evidence from the emitted wire alone, without reaching into
/// `qsl_package`'s internals.
fn record_locked_artifacts(
    value: &Value,
    evidence: &mut quire_contract_ir::CheckedPackageEvidence,
) {
    match value {
        Value::Object(members) => {
            if let (Some(authority), Some(identity), Some(revision), Some(domain), Some(digest)) = (
                members.get("authority").and_then(Value::as_str),
                members.get("identity").and_then(Value::as_str),
                members.get("revision"),
                members.get("digest_domain").and_then(Value::as_str),
                members.get("digest").and_then(Value::as_str),
            ) {
                evidence.insert_artifact_digest(
                    quire_contract_ir::CheckedArtifactLocator {
                        authority: authority.into(),
                        identity: identity.into(),
                        revision_namespace: revision["namespace"].as_str().unwrap().into(),
                        revision_value: revision["value"].as_str().unwrap().into(),
                        domain: domain.into(),
                    },
                    digest,
                );
            }
            for member in members.values() {
                record_locked_artifacts(member, evidence);
            }
        }
        Value::Array(items) => {
            for item in items {
                record_locked_artifacts(item, evidence);
            }
        }
        _ => {}
    }
}

/// FR-108-AC-6 (TC-469 step 6), its package_id half: the expected table
/// pins the FR-108 unit's `package_id` (the one spine `compile` emits for
/// it), and every case's report carries exactly that value. This half needs
/// no STD-111 wire spellings and runs today; the I04 `read` half is a
/// separate, currently-`#[ignore]`d test below, blocked on IR-370 (the
/// pinned `quire-contract-model` reader's `reaches_field`/`ReferenceEdge`
/// gap), not on STD-111. QSL-315's earlier `modifies` frame-entry rejection
/// is fixed by the `quire-contract-ir` pin bump to `2a28643`.
#[trace("TC-469", "FR-108-AC-6")]
#[test]
fn tc_469_step_6_package_id_is_pinned_across_every_case() {
    let directory = tempfile::tempdir().unwrap();
    let mut pinned = None;
    for &case in crate::support::config_version::CASES {
        // `missing-model` never reaches compile with a package to pin.
        if matches!(case, Case::MissingModel) {
            continue;
        }
        let path = directory.path().join(case.id());
        let request = build(&path, case);
        let report = run_clause(request).unwrap();
        let package_id = report
            .package_id
            .unwrap_or_else(|| panic!("{}: compile reached E4", case.id()));
        match pinned {
            None => pinned = Some(package_id),
            Some(pinned) => assert_eq!(
                package_id,
                pinned,
                "{}: every case shares the same unit and package",
                case.id()
            ),
        }
    }
    let pinned = pinned.expect("at least one case compiles");

    // The bytes spine `compile` emits for the FR-108 unit, independent of
    // any one case's request, share the same pinned `package_id`.
    let unit = spine::unit_text();
    let compiled = qsl_replay::spine::compile(
        spine::unit_identity(),
        "spine-unit.native",
        unit.as_bytes(),
        &spine::domain_packages(),
        &qsl_replay::spine::DependencyInput::default(),
        qsl_replay::spine::SpineLimits::default(),
    )
    .expect("the FR-108 unit compiles");
    assert_eq!(compiled.emitted.package_id(), pinned);
}

/// FR-108-AC-6's other half: the emitted package bytes admit through QSpec
/// I04 `read`. QSL-315 (the `frame_eligibility` gate rejecting `FrameField`'s
/// wire shape) is fixed by the `quire-contract-ir` pin bump this PR carries:
/// run with `--ignored` at this bump, the read now gets past the frame step.
/// It still refuses, but at a different, later node: node 8's body is a
/// `quire.op.model.reaches_field` application, and IR's own
/// `checked_package/v2/operations.rs::check_operands` refuses every
/// `reaches` application outright (`ill_typed`/`operator-ineligible`) because
/// its `OperationConstraintKind::ReferenceEdge` arm is unconditional. This is
/// the same IR-370 gap as TC-463 (`qsl-replay/src/spine/clause/tests.rs`), in
/// a separate pinned dependency this ticket does not own. No QSL-side
/// workaround exists. Un-ignore once IR implements the `reference_edge`
/// check.
#[trace("TC-469", "FR-108-AC-6")]
#[test]
#[ignore = "blocked on IR-370: quire-contract-model implementing the reaches_field reference_edge check (see doc comment); QSL-315's frame_eligibility gap is fixed by this PR's pin bump"]
fn tc_469_step_6_the_emitted_package_admits_via_i04() {
    let unit = spine::unit_text();
    let compiled = qsl_replay::spine::compile(
        spine::unit_identity(),
        "spine-unit.native",
        unit.as_bytes(),
        &spine::domain_packages(),
        &qsl_replay::spine::DependencyInput::default(),
        qsl_replay::spine::SpineLimits::default(),
    )
    .expect("the FR-108 unit compiles");
    let pinned = compiled.emitted.package_id();

    let bytes = compiled.emitted.bytes();
    // I04 `read` checks every locked artifact (`lock.sources`, `lock.edition.
    // definition`, `lock.definition_selections`, `diagnostics.catalog`)
    // against caller-supplied evidence that its digest is still current
    // (`quire-contract-model`'s `validate_locked_artifact`); an empty
    // evidence store refuses every one of them `StaleDependency`. This
    // mirrors `qsl_package::emit::tests::locked_artifacts`/`read_evidence`
    // (not reusable here: `Emission::evidence` is `pub(crate)` to that
    // crate), recording each artifact reference's own claimed digest as
    // current evidence directly from the emitted wire bytes.
    let wire: Value = serde_json::from_slice(bytes).expect("emitted package is JSON");
    let mut evidence = quire_contract_ir::CheckedPackageEvidence::new();
    record_locked_artifacts(&wire["lock"], &mut evidence);
    record_locked_artifacts(&wire["diagnostics"], &mut evidence);
    if let Some(features) = wire["lock"]["required_features"].as_array() {
        for feature in features {
            evidence.support_feature(feature.as_str().expect("feature name is a string"));
        }
    }
    // The lock's `model_selections` names the domain package by its own
    // `sha256-jcs` digest (FR-154/FR-322); I04 recomputes that digest from
    // the document bytes supplied here rather than trusting the lock.
    evidence.insert_domain_package_document(
        spine::model_digest_hex(),
        spine::DOMAIN_PACKAGE.as_bytes().to_vec(),
    );
    let result = quire_contract_ir::read_checked_package(
        bytes,
        quire_contract_ir::CheckedPackageReadLimits::bounded(),
        &evidence,
    );
    match result {
        quire_contract_ir::CheckedPackageDispatchResult::AdmittedV2(package) => {
            assert_eq!(
                package.package_id().digest.as_ref(),
                pinned.hex(),
                "I04 read recomputes the same package_id spine compile emitted"
            );
        }
        other => panic!("QSpec I04 read did not admit the emitted package: {other:?}"),
    }
}
