// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-050-AC-4: the compiled-protocol `/1` and `/2` readers reject each other's
//! bytes and neither upgrades, strips or invents a temporal selection.
//!
//! Both byte streams are encoded here from the authored reader fixture by the
//! public `/1` and `/2` candidate encoders; no byte sequence is frozen.

use crate::support::protocol_artifact::Fixture;

use ix_trace_rs::trace;
use quire_spec_language::protocol_artifact::{
    self as artifact, v2, wire as w, Error, Invalid, Limits, Unsupported,
};
use serde_json::Value;

fn v2_package(fixture: &Fixture) -> v2::wire::Package {
    let mut inherited = fixture.package.clone();
    inherited.wire = v2::WIRE.into();
    inherited.media = v2::MEDIA.into();
    inherited.schema = v2::SCHEMA.into();
    v2::wire::Package {
        inherited,
        temporal_bindings: Vec::new(),
    }
}

fn v2_bytes(fixture: &Fixture) -> Vec<u8> {
    v2::encode_candidate(&v2_package(fixture), Limits::default())
        .into_result()
        .expect("authored /2 package encodes")
        .bytes()
        .to_vec()
}

fn v2_selection() -> w::ArtifactRef {
    let mut selected = Fixture::selected_artifact();
    selected.wire.version = "2".into();
    selected
}

fn read_v1(fixture: &Fixture, bytes: &[u8]) -> artifact::Report<artifact::AdmittedPackage> {
    fixture.read(bytes, &Fixture::selected_artifact(), Limits::default())
}

fn read_v2(
    fixture: &Fixture,
    bytes: &[u8],
    selected: &w::ArtifactRef,
) -> artifact::Report<v2::AdmittedPackage> {
    fixture.with_expected(selected, |inherited| {
        let expected = v2::Expected {
            inherited: *inherited,
            temporal: &[],
        };
        v2::read(bytes, &expected, Limits::default())
    })
}

fn refusal<T>(report: &artifact::Report<T>) -> &Error {
    match report.result() {
        Ok(_) => panic!("the offered bytes were admitted"),
        Err(error) => error,
    }
}

#[test]
#[trace("TC-138", "TC-150", "FR-050-AC-4")]
fn each_reader_admits_its_own_version_and_keeps_the_package_it_read() {
    let fixture = Fixture::new();
    let v1 = read_v1(&fixture, fixture.candidate().bytes());
    assert_eq!(
        v1.result().expect("the /1 reader admits /1").package(),
        &fixture.package
    );
    let v2 = read_v2(&fixture, &v2_bytes(&fixture), &v2_selection());
    let admitted = v2.result().expect("the /2 reader admits /2");
    assert_eq!(admitted.package(), &v2_package(&fixture));
    assert!(admitted.package().temporal_bindings.is_empty());
}

#[test]
#[trace("TC-138", "TC-150", "FR-050-AC-4")]
fn the_v2_reader_refuses_v1_bytes_at_the_wire_header() {
    let fixture = Fixture::new();
    // The /1 bytes carry no temporal table, so the /2 reader cannot decode them.
    assert_eq!(
        refusal(&read_v2(
            &fixture,
            fixture.candidate().bytes(),
            &v2_selection()
        )),
        &Error::Json {
            line: 1,
            column: fixture.candidate().bytes().len()
        }
    );
    // With an empty table added, they decode and are refused on the header the
    // /1 producer wrote: the reader does not accept `/1` as `/2`.
    let mut value: Value = serde_json::from_slice(fixture.candidate().bytes()).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("temporal_bindings".into(), Value::Array(Vec::new()));
    let upgraded = serde_json::to_vec(&value).unwrap();
    assert_eq!(
        refusal(&read_v2(&fixture, &upgraded, &v2_selection())),
        &Error::V2(v2::Refusal::Header(v2::HeaderField::Wire))
    );
}

#[test]
#[trace("TC-138", "TC-150", "FR-050-AC-4")]
fn the_v1_reader_refuses_v2_bytes_and_a_stripped_table_does_not_downgrade_them() {
    let fixture = Fixture::new();
    // The /1 shape is closed: the reader stops at the /2 table's member name.
    let offered = v2_bytes(&fixture);
    let key = "\"temporal_bindings\"";
    let key_end = std::str::from_utf8(&offered).unwrap().find(key).unwrap() + key.len();
    assert_eq!(
        refusal(&read_v1(&fixture, &offered)),
        &Error::Json {
            line: 1,
            column: key_end
        }
    );
    // Removing the /2 table leaves the /2 headers: the /1 reader refuses them
    // rather than reading the remainder as /1.
    let mut value: Value = serde_json::from_slice(&v2_bytes(&fixture)).unwrap();
    value.as_object_mut().unwrap().remove("temporal_bindings");
    let stripped = serde_json::to_vec(&value).unwrap();
    assert_ne!(stripped, fixture.candidate().bytes());
    assert_eq!(
        refusal(&read_v1(&fixture, &stripped)),
        &Error::Unsupported(Unsupported::Wire)
    );
}

#[test]
#[trace("TC-138", "TC-150", "FR-050-AC-4")]
fn each_reader_refuses_the_other_versions_artifact_selection() {
    let fixture = Fixture::new();
    // The independently selected artifact wire version is part of each reader's
    // contract: bytes of the right version under the other version's selection
    // are refused on that selection, not admitted.
    let v1_under_v2 = fixture.read(
        fixture.candidate().bytes(),
        &v2_selection(),
        Limits::default(),
    );
    assert_eq!(refusal(&v1_under_v2), &Error::Invalid(Invalid::Selection));
    let v2_under_v1 = read_v2(&fixture, &v2_bytes(&fixture), &Fixture::selected_artifact());
    assert_eq!(
        refusal(&v2_under_v1),
        &Error::V2(v2::Refusal::Header(v2::HeaderField::ArtifactVersion))
    );
}

#[test]
#[trace("TC-138", "TC-150", "FR-050-AC-4")]
fn the_v2_reader_names_each_v1_header_it_is_offered() {
    let fixture = Fixture::new();
    for (field, revert) in [
        (
            v2::HeaderField::Wire,
            (|p: &mut w::Package| p.wire = artifact::WIRE.into()) as fn(&mut w::Package),
        ),
        (v2::HeaderField::Media, |p| p.media = artifact::MEDIA.into()),
        (v2::HeaderField::Schema, |p| {
            p.schema = artifact::SCHEMA.into()
        }),
    ] {
        let mut package = v2_package(&fixture);
        revert(&mut package.inherited);
        let bytes = v2::encode_candidate(&package, Limits::default())
            .into_result()
            .expect("a candidate with one reverted header still encodes")
            .bytes()
            .to_vec();
        assert_eq!(
            refusal(&read_v2(&fixture, &bytes, &v2_selection())),
            &Error::V2(v2::Refusal::Header(field)),
            "{field:?}"
        );
    }
}

/// The authored families fixture as a `/2` package whose table binds each of its
/// temporal declarations, optionally followed by `extra` clones of the first one.
fn temporal_package(fixture: &Fixture, extra: usize) -> v2::wire::Package {
    let mut package = v2_package(fixture);
    let first = package
        .inherited
        .declarations
        .iter()
        .position(|declaration| matches!(declaration.body, w::Body::Temporal { .. }))
        .expect("the families fixture authors a temporal declaration");
    for _ in 0..extra {
        let clone = package.inherited.declarations[first].clone();
        package.inherited.declarations.push(clone);
    }
    // The `/2` declaration spells its clock binding `clock:<name>`.
    for declaration in &mut package.inherited.declarations {
        if matches!(declaration.body, w::Body::Temporal { .. }) {
            for binding in &mut declaration.bindings {
                if binding.kind == w::BindingKind::Clock {
                    binding.name = format!("clock:{}", binding.name);
                }
            }
        }
    }
    package.temporal_bindings = package
        .inherited
        .declarations
        .iter()
        .enumerate()
        .filter(|(_, declaration)| matches!(declaration.body, w::Body::Temporal { .. }))
        .map(|(index, declaration)| v2::wire::TemporalBinding {
            declaration: u32::try_from(index).unwrap(),
            definition: declaration.profile,
            clock: v2::wire::ClockConfiguration::EventPosition {
                sequence_authority: "orders".into(),
            },
            clock_name: "ticks".into(),
        })
        .collect();
    package
}

/// Read `offered` against the independent selections authored for `base`, with
/// the expected temporal rows adjusted by `rows`.
fn read_table(
    fixture: &Fixture,
    base: &v2::wire::Package,
    offered: &v2::wire::Package,
    rows: impl FnOnce(&mut Vec<usize>),
) -> artifact::Report<v2::AdmittedPackage> {
    let bytes = v2::encode_candidate(offered, Limits::default())
        .into_result()
        .expect("the adverse table is a typed candidate")
        .bytes()
        .to_vec();
    let inherited = &base.inherited;
    let mut selected: Vec<usize> = (0..base.temporal_bindings.len()).collect();
    rows(&mut selected);
    let declarations: Vec<_> = base
        .temporal_bindings
        .iter()
        .map(|binding| {
            let declaration = &inherited.declarations[binding.declaration as usize];
            artifact::ExpectedDeclaration {
                name: &declaration.name,
                span: &declaration.locus.span,
                requirement: &declaration.requirement,
                clause: &declaration.clause,
                execution: &declaration.execution,
            }
        })
        .collect();
    let temporal: Vec<_> = selected
        .iter()
        .map(|row| {
            let binding = &base.temporal_bindings[*row];
            let declaration = &inherited.declarations[binding.declaration as usize];
            let definition = &inherited.definitions[binding.definition as usize];
            v2::ExpectedTemporal {
                source: &inherited.sources[declaration.locus.source as usize].artifact,
                declaration: &declarations[*row],
                definition: v2::ExpectedDefinition {
                    identity: &definition.identity,
                    revision: &definition.revision,
                    artifact: &inherited.dependencies[definition.artifact as usize].artifact,
                },
                clock: &binding.clock,
            }
        })
        .collect();
    fixture.with_expected(&v2_selection(), |inherited| {
        let expected = v2::Expected {
            inherited: *inherited,
            temporal: &temporal,
        };
        v2::read(&bytes, &expected, Limits::default())
    })
}

fn binding(side: v2::InventorySide, cause: v2::BindingCause) -> Error {
    Error::V2(v2::Refusal::Binding { side, cause })
}

#[test]
#[trace("TC-138", "TC-150", "FR-050-AC-2", "FR-050-AC-3", "FR-050-AC-4")]
fn a_temporal_table_is_admitted_only_when_it_matches_the_independent_selection() {
    let fixture = Fixture::families();
    let base = temporal_package(&fixture, 0);
    assert_eq!(base.temporal_bindings.len(), 1);
    let read = read_table(&fixture, &base, &base, |_| {});
    assert_eq!(read.result().expect("exact table").package(), &base);

    let mut missing = base.clone();
    missing.temporal_bindings.pop();
    assert_eq!(
        refusal(&read_table(&fixture, &base, &missing, |_| {})),
        &binding(v2::InventorySide::Offer, v2::BindingCause::Missing)
    );
    let mut surplus = base.clone();
    surplus
        .temporal_bindings
        .push(base.temporal_bindings[0].clone());
    assert_eq!(
        refusal(&read_table(&fixture, &base, &surplus, |_| {})),
        &binding(v2::InventorySide::Offer, v2::BindingCause::Surplus)
    );
    assert_eq!(
        refusal(&read_table(&fixture, &base, &base, |rows| rows.clear())),
        &binding(v2::InventorySide::Expected, v2::BindingCause::Missing)
    );
    assert_eq!(
        refusal(&read_table(&fixture, &base, &base, |rows| rows.push(0))),
        &binding(v2::InventorySide::Expected, v2::BindingCause::Surplus)
    );
}

#[test]
#[trace("TC-138", "TC-150", "FR-050-AC-2", "FR-050-AC-3", "FR-050-AC-4")]
fn a_repeated_or_misordered_temporal_row_is_refused() {
    let fixture = Fixture::families();
    let base = temporal_package(&fixture, 1);
    assert_eq!(base.temporal_bindings.len(), 2);
    let mut duplicate = base.clone();
    duplicate.temporal_bindings[1] = duplicate.temporal_bindings[0].clone();
    assert_eq!(
        refusal(&read_table(&fixture, &base, &duplicate, |_| {})),
        &binding(v2::InventorySide::Offer, v2::BindingCause::Duplicate)
    );
    let mut reordered = base.clone();
    reordered.temporal_bindings.swap(0, 1);
    assert_eq!(
        refusal(&read_table(&fixture, &base, &reordered, |_| {})),
        &Error::V2(v2::Refusal::OfferOrder)
    );
    // Two temporal declarations at one source span select one expected row twice.
    assert_eq!(
        refusal(&read_table(&fixture, &base, &base, |_| {})),
        &binding(v2::InventorySide::Expected, v2::BindingCause::Duplicate)
    );
}
