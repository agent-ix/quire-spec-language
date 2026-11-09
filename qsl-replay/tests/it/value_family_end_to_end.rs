// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-784 (FR-299): one `1-draft` source with `Value`-family function claims
//! goes through `check`, `package` and `execute` by the library operations
//! the CLI calls, with and without a supplied library. As an integration
//! test it reaches only the crate's public API.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::{StageFailure, Staged};
use qsl_foundation::SourceIdentity;
use qsl_replay::spine::{
    self, default_accounting, Call, CallArgument, CallOutcome, CallValue, CheckedUnit,
    DependencyInput, EmittedUnit, FrontEndFailure, PackageLimits, ParseRequest, SpineLimits,
    SuppliedLibrary,
};
use qsl_replay::{compile_package, CompileUnit, OutcomeDocument, ReplayLimits, StageLimits};
use quire_exact::{Cancel, CancelCause, Integer, Value};
use serde_json::Value as Json;

const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\n";
const CLAIMS: &str = "function p using v(x: Int[0, 9]): Boolean pure { x < 10 }\n\
                      function q using v(x: Int[0, 9]): Boolean pure { x < 5 }\n";
const GEOMETRY: &str = "function f using v(x: Int[0, 9]): Boolean pure { x < 5 }\n";
const IMPORTING: &str = "import \"test/geometry\" as g;\n\
                         function p using v(x: Int[0, 9]): Boolean pure { g::f(x) }\n\
                         function q using v(x: Int[0, 9]): Boolean pure { x < 5 }\n";

type Checked = Result<Staged<CheckedUnit>, FrontEndFailure>;
type Packaged = Result<Staged<EmittedUnit>, FrontEndFailure>;

fn program_identity() -> SourceIdentity {
    SourceIdentity::new("agent-ix", "test:program")
}

fn geometry_identity() -> SourceIdentity {
    SourceIdentity::new("agent-ix", "test:geometry")
}

fn program(body: &str) -> String {
    format!("{HEADER}{body}")
}

fn dependencies(with_library: bool) -> DependencyInput {
    if !with_library {
        return DependencyInput::default();
    }
    DependencyInput::new([SuppliedLibrary {
        identity: "test/geometry".to_owned(),
        source: geometry_identity(),
        path: "geometry.native".to_owned(),
        bytes: program(GEOMETRY).into_bytes(),
    }])
    .expect("one library under its own owner")
}

/// The library `check` operation over `text`, under `cancel`.
fn check(text: &str, dependencies: &DependencyInput, cancel: &Cancel) -> Checked {
    let source = program_identity();
    let limits = SpineLimits::default();
    let parsed = spine::parse(
        &ParseRequest {
            source: &source,
            path: "program.native",
            bytes: text.as_bytes(),
        },
        limits.source,
        &Cancel::new(),
    )
    .expect("the source parses")
    .into_value();
    let models = spine::select(&parsed, &BTreeMap::new(), limits.model, &Cancel::new())
        .expect("no model is selected")
        .into_value();
    spine::check(
        &parsed,
        "fixture".to_owned(),
        &models,
        dependencies,
        limits,
        cancel,
    )
}

fn package(checked: &Checked) -> Packaged {
    let staged = checked.as_ref().expect("the source checks");
    spine::package(staged.value(), PackageLimits::default(), &Cancel::new())
}

/// `execute` of `p` with `x = 3` on the interpreter, the default backend.
fn execute_p(text: &str, dependencies: &DependencyInput) -> Result<CallOutcome, String> {
    let call = Call::new(
        "p".to_owned(),
        vec![CallArgument {
            parameter: "x".to_owned(),
            value: Value::Integer(Integer::from(3_i64)),
        }],
        default_accounting(spine::DEFAULT_WORK_UNITS),
        Default::default(),
    );
    spine::run(
        program_identity(),
        "fixture".to_owned(),
        "program.native",
        text.as_bytes(),
        &BTreeMap::new(),
        dependencies,
        SpineLimits::default(),
        &call,
        &Default::default(),
        &Cancel::new(),
    )
    .map_err(|refusal| refusal.to_string())
}

/// The FR-278 composition's bytes over the same source and inputs.
fn composed_bytes(text: &str, dependencies: &DependencyInput) -> Vec<u8> {
    compile_package(
        CompileUnit {
            source: program_identity(),
            package_name: "fixture".to_owned(),
            path: "program.native",
            bytes: text.as_bytes(),
        },
        [],
        dependencies,
        StageLimits::default(),
        ReplayLimits::default(),
    )
    .expect("the composition compiles the source")
    .bytes()
    .to_vec()
}

fn is_true(outcome: &CallOutcome) -> bool {
    matches!(outcome, CallOutcome::Completed(CallValue::Boolean(true)))
}

fn json_of(document: &OutcomeDocument) -> Json {
    serde_json::from_slice(&document.to_bytes().expect("encodes")).expect("is JSON")
}

/// The three outcomes over `text` and their documents, shared by AC-1 and AC-3.
fn all_three(text: &str, dependencies: &DependencyInput) -> [(Json, u8); 3] {
    let checked = check(text, dependencies, &Cancel::new());
    let emitted = package(&checked);
    let executed = execute_p(text, dependencies).expect("p runs");
    assert!(is_true(&executed), "p completes true");
    let bytes = emitted
        .as_ref()
        .expect("packages")
        .value()
        .package()
        .bytes();
    assert_eq!(bytes, composed_bytes(text, dependencies).as_slice());
    let documents = [
        OutcomeDocument::from_check(&checked),
        OutcomeDocument::from_package(&emitted),
        OutcomeDocument::from_call(&executed),
    ];
    documents.map(|document| (json_of(&document), document.category().exit_code()))
}

/// FR-299-AC-1: `check` succeeds with exit 0, `package` returns the bytes
/// the FR-278 composition writes, and `execute` of `p` with `x = 3`
/// completes `true` with exit 0.
#[trace("TC-784", "FR-299-AC-1")]
#[test]
fn one_source_checks_packages_and_executes() {
    let text = program(CLAIMS);
    let outcomes = all_three(&text, &dependencies(false));
    for (document, exit) in &outcomes {
        assert_eq!(document["category"], "success");
        assert_eq!(*exit, 0);
    }
    assert_eq!(
        outcomes[2].0["result"],
        serde_json::json!({"kind": "completed", "value": {"kind": "boolean", "value": true}})
    );
}

/// FR-299-AC-2: each outcome serializes to a document whose `operation`,
/// `category` and `artifacts` match the outcome, byte-equal when serialized
/// twice.
#[trace("TC-784", "FR-299-AC-2")]
#[test]
fn each_outcome_serializes_to_one_stable_document() {
    let text = program(CLAIMS);
    let no_libraries = dependencies(false);
    let checked = check(&text, &no_libraries, &Cancel::new());
    let emitted = package(&checked);
    let executed = execute_p(&text, &no_libraries).expect("p runs");
    let documents = [
        (OutcomeDocument::from_check(&checked), "check"),
        (OutcomeDocument::from_package(&emitted), "package"),
        (OutcomeDocument::from_call(&executed), "execute"),
    ];
    for (document, operation) in documents {
        let first = document.to_bytes().expect("encodes");
        assert_eq!(first, document.to_bytes().expect("encodes twice"));
        let written = json_of(&document);
        assert_eq!(written["operation"], operation);
        assert_eq!(written["category"], "success");
        assert_eq!(written["artifacts"], serde_json::json!([]));
    }
}

/// FR-299-AC-3: over a source importing `test/geometry`, with that library
/// supplied, the dispositions and exit codes equal AC-1's.
#[trace("TC-784", "FR-299-AC-3")]
#[test]
fn a_supplied_library_gives_the_same_dispositions() {
    let plain = all_three(&program(CLAIMS), &dependencies(false));
    let imported = all_three(&program(IMPORTING), &dependencies(true));
    for ((plain, plain_exit), (imported, imported_exit)) in plain.iter().zip(&imported) {
        assert_eq!(plain["category"], imported["category"]);
        assert_eq!(plain["operation"], imported["operation"]);
        assert_eq!(plain_exit, imported_exit);
    }
}

/// FR-299-AC-4: `check` with its `Cancel` handle cancelled before the call
/// returns `StageFailure::Cancelled`, exit 22.
#[trace("TC-784", "FR-299-AC-4")]
#[test]
fn a_cancelled_check_returns_cancelled_with_exit_22() {
    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    let checked = check(&program(CLAIMS), &dependencies(false), &cancel);
    assert!(matches!(checked, Err(StageFailure::Cancelled(_))));
    assert_eq!(
        OutcomeDocument::from_check(&checked).category().exit_code(),
        22
    );
}
