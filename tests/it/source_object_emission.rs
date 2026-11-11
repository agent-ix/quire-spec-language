// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-569 "Emission": the checked-package/v2 graph has no node for a source
//! object type, so a unit that declares one is refused at emission and writes
//! nothing; a unit that declares none emits as before.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use quire_spec_language::checked_handoff::{write, Corpus, ErrorCode};
use quire_spec_language::emit_checked_v2::{self, Error, Request};

use super::checked_handoff::request;

const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile P = \"quire.model.complete/v1\";\n\
    object Order { total: Int[0, 9]; population orders; operation size(): Integer; }\n\
    object Sub extends Order { }\n";

fn source_request(directory: &std::path::Path) -> Request {
    let unit = directory.join("source.native");
    std::fs::write(&unit, UNIT).unwrap();
    Request {
        source: unit,
        package_name: "r".into(),
        models: Vec::new(),
        libraries: Vec::new(),
        execution_points: None,
        temporal_selections: Vec::new(),
        out: None,
    }
}

/// FR-569-AC-11: a unit that declares a source object type is refused with
/// one `unsupported_construct`/`not-yet-implemented` naming
/// `SourceObjectEmission`, and the handoff writer creates no directory.
#[trace("FR-569-AC-11", "TC-1408")]
#[test]
fn a_unit_with_a_source_object_is_refused_at_emission_and_writes_nothing() {
    let holder = tempfile::tempdir().unwrap();
    let source = source_request(holder.path());
    let Err(Error::Producer(refusal)) = emit_checked_v2::emit(&source) else {
        panic!("the source object is refused");
    };
    assert_eq!(refusal.code(), Code::UnsupportedConstruct);
    assert_eq!(refusal.cause(), Some("not-yet-implemented"), "{refusal}");
    assert!(
        refusal.to_string().contains("SourceObjectEmission"),
        "{refusal}"
    );

    let out = holder.path().join("handoff");
    let mut handoff = request("event_position", Corpus::None);
    handoff.emit = source;
    let error = write(&handoff, &out).unwrap_err();
    assert_eq!(error.code(), ErrorCode::Compile);
    assert!(!out.exists());
    let mut entries: Vec<_> = std::fs::read_dir(holder.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    entries.sort();
    assert_eq!(entries, ["source.native"]);
}

/// A unit with no `object` declaration emits as before.
#[trace("FR-569-AC-11", "TC-1408")]
#[test]
fn a_unit_without_a_source_object_still_emits() {
    let holder = tempfile::tempdir().unwrap();
    let mut source = source_request(holder.path());
    std::fs::write(
        &source.source,
        "language \"ix:native\" edition \"1-draft\";\n\
         profile P = \"quire.model.complete/v1\";\n\
         function f using P (x: Boolean): Boolean pure { x }\n",
    )
    .unwrap();
    source.temporal_selections.clear();
    let bytes = emit_checked_v2::emit(&source).unwrap_or_else(|error| panic!("{error}"));
    assert!(!bytes.is_empty());
}
