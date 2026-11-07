// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-050-AC-4: the compiled-protocol `/1` and `/2` readers reject each other's
//! bytes and neither upgrades, strips or invents a temporal selection.
//!
//! Both byte streams are encoded here from the authored reader fixture by the
//! public `/1` and `/2` candidate encoders; no byte sequence is frozen.

use crate::support::protocol_artifact::Fixture;

use ix_trace_rs::trace;
use quire_spec_language::linking::definition_source::RegisteredDefinition as R;
use quire_spec_language::number::{NumberComponent, NumberError, NumberWire};
use quire_spec_language::protocol_artifact::{
    self as artifact, v2, v3, wire as w, Dimension, Error, Invalid, Limits, Unsupported,
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

/// One independently authored expected temporal row, owning its values.
#[derive(Clone)]
struct Row {
    source: w::ArtifactRef,
    name: String,
    span: w::Span,
    requirement: w::Requirement,
    clause: String,
    execution: w::Execution,
    identity: String,
    revision: w::Revision,
    artifact: w::ArtifactRef,
    clock: v2::wire::ClockConfiguration,
}

/// The expected rows an exact reader selects for `base`, derived from its
/// declarations, sources and definition dependencies.
fn rows(base: &v2::wire::Package) -> Vec<Row> {
    let inherited = &base.inherited;
    base.temporal_bindings
        .iter()
        .map(|binding| {
            let declaration = &inherited.declarations[binding.declaration as usize];
            let definition = &inherited.definitions[binding.definition as usize];
            Row {
                source: inherited.sources[declaration.locus.source as usize]
                    .artifact
                    .clone(),
                name: declaration.name.clone(),
                span: declaration.locus.span.clone(),
                requirement: declaration.requirement.clone(),
                clause: declaration.clause.clone(),
                execution: declaration.execution.clone(),
                identity: definition.identity.clone(),
                revision: definition.revision.clone(),
                artifact: inherited.dependencies[definition.artifact as usize]
                    .artifact
                    .clone(),
                clock: binding.clock.clone(),
            }
        })
        .collect()
}

/// Run `then` against the complete `/2` expectation for `rows` under the
/// independently selected artifact `selected`.
fn with_rows<T>(
    fixture: &Fixture,
    selected: &w::ArtifactRef,
    rows: &[Row],
    then: impl FnOnce(&v2::Expected<'_>) -> T,
) -> T {
    let declarations: Vec<_> = rows
        .iter()
        .map(|row| artifact::ExpectedDeclaration {
            name: &row.name,
            span: &row.span,
            requirement: &row.requirement,
            clause: &row.clause,
            execution: &row.execution,
        })
        .collect();
    let temporal: Vec<_> = rows
        .iter()
        .zip(&declarations)
        .map(|(row, declaration)| v2::ExpectedTemporal {
            source: &row.source,
            declaration,
            definition: v2::ExpectedDefinition {
                identity: &row.identity,
                revision: &row.revision,
                artifact: &row.artifact,
            },
            clock: &row.clock,
        })
        .collect();
    fixture.with_expected(selected, |inherited| {
        then(&v2::Expected {
            inherited: *inherited,
            temporal: &temporal,
        })
    })
}

fn read_rows(
    fixture: &Fixture,
    bytes: &[u8],
    rows: &[Row],
) -> artifact::Report<v2::AdmittedPackage> {
    read_rows_within(fixture, bytes, rows, Limits::default())
}

fn read_rows_within(
    fixture: &Fixture,
    bytes: &[u8],
    rows: &[Row],
    limits: Limits,
) -> artifact::Report<v2::AdmittedPackage> {
    with_rows(fixture, &v2_selection(), rows, |expected| {
        v2::read(bytes, expected, limits)
    })
}

/// Read the serialized `offered` package against the selections authored for
/// `base`, after `edit` has changed the expected rows.
fn read_edited(
    fixture: &Fixture,
    base: &v2::wire::Package,
    offered: &v2::wire::Package,
    edit: impl FnOnce(&mut Vec<Row>),
) -> artifact::Report<v2::AdmittedPackage> {
    let bytes = serde_json::to_vec(offered).expect("canonical adverse wire shape");
    let mut selected = rows(base);
    edit(&mut selected);
    read_rows(fixture, &bytes, &selected)
}

/// Read `offered` against `base`'s selections, choosing which expected rows
/// (by index into `base`'s table) are supplied.
fn read_table(
    fixture: &Fixture,
    base: &v2::wire::Package,
    offered: &v2::wire::Package,
    select: impl FnOnce(&mut Vec<usize>),
) -> artifact::Report<v2::AdmittedPackage> {
    let mut chosen: Vec<usize> = (0..base.temporal_bindings.len()).collect();
    select(&mut chosen);
    read_edited(fixture, base, offered, |all| {
        *all = chosen.iter().map(|index| all[*index].clone()).collect();
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
    let admitted = read.result().expect("exact table");
    assert_eq!(admitted.package(), &base);
    // The reader retains the independently selected identity and model schema.
    assert!(admitted.artifact().is_some());
    assert_eq!(
        admitted.schema_model(0).map(|model| model.artifact_bytes()),
        Some(fixture.model.artifact_bytes())
    );
    for binding in &admitted.package().temporal_bindings {
        assert_eq!(
            binding.definition,
            admitted.package().inherited.declarations[binding.declaration as usize].profile
        );
    }

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

/// `base` with its temporal declaration selecting `profile` and `clock`. The
/// offer-side table checks run before definitions are matched against the
/// registered catalog, so only the identity and clock need to agree.
fn profiled(
    fixture: &Fixture,
    profile: R,
    clock: v2::wire::ClockConfiguration,
) -> v2::wire::Package {
    let mut package = temporal_package(fixture, 0);
    let definition = package.temporal_bindings[0].definition as usize;
    package.inherited.definitions[definition].identity = profile.identity().into();
    package.temporal_bindings[0].clock = clock;
    package
}

fn integer(decimal: &str) -> w::Number {
    w::Number(NumberWire::Integer {
        decimal: decimal.into(),
    })
}

fn half() -> w::Number {
    w::Number(NumberWire::Rational {
        numerator: "1".into(),
        denominator: "2".into(),
    })
}

fn sample(epoch: &str, period: w::Number, unit: &str) -> v2::wire::ClockConfiguration {
    v2::wire::ClockConfiguration::FixedSample {
        epoch: integer(epoch),
        period,
        unit: unit.into(),
    }
}

fn clock_refusal(field: v2::ClockField) -> Error {
    Error::V2(v2::Refusal::Clock(field))
}

#[test]
#[trace("TC-138", "TC-150", "FR-050-AC-2", "FR-050-AC-3")]
fn every_header_and_both_binding_indices_have_stable_v2_refusals() {
    let fixture = Fixture::families();
    let base = temporal_package(&fixture, 0);
    type Edit = fn(&mut v2::wire::Package);
    let headers: [(v2::HeaderField, Edit); 6] = [
        (v2::HeaderField::Wire, |p| {
            p.inherited.wire = artifact::WIRE.into()
        }),
        (v2::HeaderField::Media, |p| {
            p.inherited.media = artifact::MEDIA.into()
        }),
        (v2::HeaderField::Schema, |p| {
            p.inherited.schema = artifact::SCHEMA.into()
        }),
        (v2::HeaderField::PackageType, |p| {
            p.inherited.package_type.push_str("Other")
        }),
        (v2::HeaderField::Encoding, |p| {
            p.inherited.encoding.push_str("-other")
        }),
        (v2::HeaderField::Numeric, |p| {
            p.inherited.numeric.push_str("-other")
        }),
    ];
    for (field, edit) in headers {
        let mut offered = base.clone();
        edit(&mut offered);
        assert_eq!(
            refusal(&read_table(&fixture, &base, &offered, |_| {})),
            &Error::V2(v2::Refusal::Header(field)),
            "{field:?}"
        );
    }
    let indices: [(v2::BindingIndex, Edit); 2] = [
        (v2::BindingIndex::Declaration, |p| {
            p.temporal_bindings[0].declaration = 9_999
        }),
        (v2::BindingIndex::Definition, |p| {
            p.temporal_bindings[0].definition = 9_999
        }),
    ];
    for (index, edit) in indices {
        let mut offered = base.clone();
        edit(&mut offered);
        assert_eq!(
            refusal(&read_table(&fixture, &base, &offered, |_| {})),
            &Error::V2(v2::Refusal::OfferIndex(index)),
            "{index:?}"
        );
    }
}

#[test]
#[trace("TC-138", "TC-150", "FR-050-AC-2", "FR-050-AC-3")]
fn each_clock_member_of_each_profile_is_compared_and_named() {
    let fixture = Fixture::families();
    let event = |authority: &str| v2::wire::ClockConfiguration::EventPosition {
        sequence_authority: authority.into(),
    };
    let timestamp = |unit: &str| v2::wire::ClockConfiguration::TimestampedEvent {
        timestamp_unit: unit.into(),
    };
    let cases = [
        (
            R::EventPosition,
            event("orders"),
            event("other-orders"),
            v2::ClockField::SequenceAuthority,
        ),
        (
            R::FixedSample,
            sample("0", half(), "second"),
            sample("1", half(), "second"),
            v2::ClockField::Epoch,
        ),
        (
            R::FixedSample,
            sample("0", half(), "second"),
            sample("0", integer("1"), "second"),
            v2::ClockField::Period,
        ),
        (
            R::FixedSample,
            sample("0", half(), "second"),
            sample("0", half(), "minute"),
            v2::ClockField::Unit,
        ),
        (
            R::TimestampedWindow,
            timestamp("millisecond"),
            timestamp("nanosecond"),
            v2::ClockField::TimestampUnit,
        ),
    ];
    for (profile, offered, selected, field) in cases {
        let base = profiled(&fixture, profile, offered);
        let report = read_edited(&fixture, &base, &base, |rows| {
            rows[0].clock = selected.clone();
        });
        assert_eq!(refusal(&report), &clock_refusal(field), "{field:?}");
    }
    // An offered clock of another profile's alternative, or an unnamed
    // authority, is refused on the offer itself.
    let base = profiled(&fixture, R::EventPosition, event("orders"));
    let mut wrong = base.clone();
    wrong.temporal_bindings[0].clock = timestamp("millisecond");
    assert_eq!(
        refusal(&read_table(&fixture, &base, &wrong, |_| {})),
        &clock_refusal(v2::ClockField::Alternative)
    );
    let mut unnamed = base.clone();
    unnamed.temporal_bindings[0].clock = event("");
    assert_eq!(
        refusal(&read_table(&fixture, &base, &unnamed, |_| {})),
        &clock_refusal(v2::ClockField::SequenceAuthority)
    );
}

#[test]
#[trace("TC-138", "TC-150", "FR-050-AC-2", "FR-050-AC-3")]
fn a_substituted_declaration_definition_or_owner_is_refused_on_its_own_field() {
    let fixture = Fixture::families();
    let base = temporal_package(&fixture, 0);
    let declaration = base.temporal_bindings[0].declaration as usize;
    let definition = base.temporal_bindings[0].definition as usize;
    type Edit = Box<dyn Fn(&mut v2::wire::Package)>;
    let offered: Vec<(Edit, Error)> = vec![
        (
            Box::new(move |p| {
                let source = p.inherited.declarations[declaration].locus.source as usize;
                p.inherited.sources[source]
                    .artifact
                    .identity
                    .push_str("-foreign")
            }),
            Error::V2(v2::Refusal::ForeignOwner(v2::SelectionSide::Expected)),
        ),
        (
            Box::new(move |p| {
                p.inherited.definitions[definition]
                    .identity
                    .push_str("-other")
            }),
            Error::V2(v2::Refusal::Definition(v2::DefinitionField::Identity)),
        ),
        (
            Box::new(move |p| {
                p.inherited.definitions[definition]
                    .revision
                    .value
                    .push_str("-other")
            }),
            Error::V2(v2::Refusal::Definition(v2::DefinitionField::Revision)),
        ),
        (
            Box::new(move |p| {
                let at = p.inherited.definitions[definition].artifact as usize;
                p.inherited.dependencies[at]
                    .artifact
                    .identity
                    .push_str("-other")
            }),
            Error::V2(v2::Refusal::Definition(v2::DefinitionField::Artifact(
                v2::ArtifactField::Identity,
            ))),
        ),
        (
            Box::new(move |p| p.inherited.declarations[declaration].name.push_str("Other")),
            Error::V2(v2::Refusal::Declaration(v2::DeclarationField::Name)),
        ),
        (
            Box::new(move |p| {
                p.inherited.declarations[declaration]
                    .requirement
                    .identity
                    .push_str("Other")
            }),
            Error::V2(v2::Refusal::Declaration(v2::DeclarationField::Requirement)),
        ),
        (
            Box::new(move |p| {
                p.inherited.declarations[declaration]
                    .clause
                    .push_str("-other")
            }),
            Error::V2(v2::Refusal::Declaration(v2::DeclarationField::Clause)),
        ),
        (
            Box::new(move |p| {
                p.inherited.declarations[declaration].execution = w::Execution::Handler {
                    name: "other".into(),
                }
            }),
            Error::V2(v2::Refusal::Declaration(v2::DeclarationField::Execution)),
        ),
    ];
    for (edit, expected) in offered {
        let mut changed = base.clone();
        edit(&mut changed);
        assert_eq!(
            refusal(&read_table(&fixture, &base, &changed, |_| {})),
            &expected
        );
    }
}

#[test]
#[trace("TC-138", "TC-150", "FR-050-AC-2", "FR-050-AC-3")]
fn a_substituted_expected_selection_is_refused_on_its_own_field() {
    let fixture = Fixture::families();
    let base = temporal_package(&fixture, 0);
    let report = read_edited(&fixture, &base, &base, |rows| rows[0].span.start += 1);
    assert_eq!(
        refusal(&report),
        &binding(v2::InventorySide::Expected, v2::BindingCause::Missing)
    );
    let report = read_edited(&fixture, &base, &base, |rows| {
        rows[0].revision.value.push_str("-other")
    });
    assert_eq!(
        refusal(&report),
        &Error::V2(v2::Refusal::Definition(v2::DefinitionField::Revision))
    );
    let report = read_edited(&fixture, &base, &base, |rows| {
        rows[0].source.identity.push_str("-foreign")
    });
    assert_eq!(
        refusal(&report),
        &Error::V2(v2::Refusal::ForeignOwner(v2::SelectionSide::Expected))
    );
    type Edit = fn(&mut w::ArtifactRef);
    let artifact_fields: [(v2::ArtifactField, Edit); 6] = [
        (v2::ArtifactField::RefVersion, |a| {
            a.ref_version.push_str("-other")
        }),
        (v2::ArtifactField::Kind, |a| {
            a.kind = w::ArtifactKind::GeneratedArtifact
        }),
        (v2::ArtifactField::Authority, |a| {
            a.authority.push_str("/other")
        }),
        (v2::ArtifactField::Identity, |a| {
            a.identity.push_str("/other")
        }),
        (v2::ArtifactField::WireIdentity, |a| {
            a.wire.identity.push_str("/canonical-json")
        }),
        (v2::ArtifactField::WireVersion, |a| {
            a.wire.version.push_str("-other")
        }),
    ];
    for (field, edit) in artifact_fields {
        let report = read_edited(&fixture, &base, &base, |rows| edit(&mut rows[0].artifact));
        assert_eq!(
            refusal(&report),
            &Error::V2(v2::Refusal::Definition(v2::DefinitionField::Artifact(
                field
            ))),
            "{field:?}"
        );
    }
}

#[test]
#[trace("TC-138", "FR-050-AC-2", "FR-050-AC-3")]
fn a_clock_object_missing_or_renaming_a_member_is_a_json_refusal() {
    let fixture = Fixture::families();
    let cases = [
        (
            R::EventPosition,
            sample_event(),
            "sequence_authority",
            "sequenceAuthority",
        ),
        (
            R::FixedSample,
            sample("0", half(), "second"),
            "period",
            "sample_period",
        ),
        (
            R::TimestampedWindow,
            timestamped(),
            "timestamp_unit",
            "timestampUnit",
        ),
    ];
    for (profile, clock, member, renamed) in cases {
        let base = profiled(&fixture, profile, clock);
        let original: serde_json::Value = serde_json::to_value(&base).unwrap();
        for replacement in [None, Some(renamed)] {
            let mut changed = original.clone();
            let object = changed["temporal_bindings"][0]["clock"]
                .as_object_mut()
                .unwrap();
            let value = object.remove(member).unwrap();
            if let Some(replacement) = replacement {
                object.insert(replacement.into(), value);
            }
            let bytes = serde_json::to_vec(&changed).unwrap();
            let report = read_rows(&fixture, &bytes, &rows(&base));
            // serde reports the absent member at the object's closing brace,
            // whether it was dropped or renamed.
            let column = clock_object_end(std::str::from_utf8(&bytes).unwrap());
            assert_eq!(
                refusal(&report),
                &Error::Json { line: 1, column },
                "{member} {replacement:?}"
            );
        }
    }
}

fn sample_event() -> v2::wire::ClockConfiguration {
    v2::wire::ClockConfiguration::EventPosition {
        sequence_authority: "orders".into(),
    }
}

fn timestamped() -> v2::wire::ClockConfiguration {
    v2::wire::ClockConfiguration::TimestampedEvent {
        timestamp_unit: "millisecond".into(),
    }
}

/// Byte offset just past the closing brace of the first binding's `clock` object.
fn clock_object_end(text: &str) -> usize {
    let key = "\"clock\":";
    let start = text.find("\"temporal_bindings\"").unwrap();
    let open = start + text[start..].find(key).unwrap() + key.len();
    let mut depth = 0usize;
    for (offset, byte) in text[open..].bytes().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return open + offset + 1;
                }
            }
            _ => {}
        }
    }
    unreachable!("a closed clock object")
}

fn v3_selection() -> w::ArtifactRef {
    let mut selected = Fixture::selected_artifact();
    selected.wire.version = "3".into();
    selected
}

/// `base` as a `/3` package carrying `mappings`.
fn v3_package(base: &v2::wire::Package, mappings: &[(w::Handle, u32)]) -> v3::wire::Package {
    let mut inherited = base.inherited.clone();
    inherited.wire = v3::WIRE.into();
    inherited.media = v3::MEDIA.into();
    inherited.schema = v3::SCHEMA.into();
    v3::wire::Package {
        inherited,
        temporal_bindings: base.temporal_bindings.clone(),
        activation_mappings: mappings
            .iter()
            .map(
                |(control, temporal_declaration)| v3::wire::ActivationMapping {
                    control: control.clone(),
                    temporal_declaration: *temporal_declaration,
                },
            )
            .collect(),
    }
}

fn read_v3(
    fixture: &Fixture,
    base: &v2::wire::Package,
    bytes: &[u8],
    activations: &[(w::Handle, u32)],
) -> artifact::Report<v3::AdmittedPackage> {
    let activations: Vec<_> = activations
        .iter()
        .map(|(control, temporal_declaration)| v3::ExpectedActivation {
            control: control.clone(),
            temporal_declaration: *temporal_declaration,
        })
        .collect();
    with_rows(fixture, &v3_selection(), &rows(base), |inherited| {
        v3::read(
            bytes,
            &v3::Expected {
                inherited: *inherited,
                activations: &activations,
            },
            Limits::default(),
        )
    })
}

struct Mapped {
    base: v2::wire::Package,
    event: w::Handle,
    other_event: w::Handle,
    non_event: w::Handle,
    first: u32,
    second: u32,
}

fn control(index: u32) -> w::Handle {
    w::Handle {
        declaration: 0,
        index,
    }
}

/// A `/2` base with two temporal declarations whose protocol declaration 0 has
/// two event controls and a non-event control. The mapping checks run before the
/// graph is validated, so only the control kinds matter.
fn mapped_base(fixture: &Fixture) -> Mapped {
    let mut base = temporal_package(fixture, 1);
    // The clone is a second declaration at its own span, so the two rows select
    // two distinct expected declarations.
    base.inherited
        .declarations
        .last_mut()
        .expect("the appended temporal declaration")
        .locus
        .span
        .start += 1;
    let operation = fixture.export(w::ExportKind::Operation, &["Node", "step"]);
    let w::Body::Protocol { controls, .. } = &mut base.inherited.declarations[0].body else {
        panic!("declaration 0 of the families fixture is the protocol");
    };
    let non_event = controls[0].clone();
    let mut event = non_event.clone();
    event.name = "Pulse".into();
    event.operation = w::ControlOperation::Event {
        event: w::Event::Attempt {
            owner: control(0),
            operation,
            contracts: vec![],
            instance: 0,
        },
        binder: control(0),
        related: vec![],
        constraint: control(0),
    };
    let mut other = event.clone();
    other.name = "Again".into();
    *controls = vec![event, other, non_event];
    let (first, second) = (
        base.temporal_bindings[0].declaration,
        base.temporal_bindings[1].declaration,
    );
    Mapped {
        base,
        event: control(0),
        other_event: control(1),
        non_event: control(2),
        first,
        second,
    }
}

fn v3_error(
    fixture: &Fixture,
    mapped: &Mapped,
    offered: &v3::wire::Package,
    activations: &[(w::Handle, u32)],
) -> Error {
    let bytes = serde_json::to_vec(offered).expect("canonical adverse wire shape");
    refusal(&read_v3(fixture, &mapped.base, &bytes, activations)).clone()
}

#[test]
#[trace("TC-143", "FR-054-AC-1", "FR-054-AC-2", "FR-054-AC-3", "FR-054-AC-4")]
fn a_v3_package_with_no_event_control_admits_an_empty_mapping_and_no_other_reader_admits_it() {
    let fixture = Fixture::families();
    let base = temporal_package(&fixture, 0);
    let package = v3_package(&base, &[]);
    let bytes = v3::encode_candidate(&package, Limits::default())
        .into_result()
        .expect("canonical /3 candidate")
        .bytes()
        .to_vec();
    let read = read_v3(&fixture, &base, &bytes, &[]);
    let admitted = read.result().expect("strict /3 reader");
    assert_eq!(admitted.package(), &package);
    assert!(admitted.activation_mappings().is_empty());
    assert!(admitted.artifact().is_some());
    // The /2 reader, the /1 reader and the /3 reader on /2 bytes each refuse
    // the other version at its own header or shape.
    assert_eq!(
        refusal(&read_rows(&fixture, &bytes, &rows(&base))),
        &Error::Json {
            line: 1,
            column: json_error_column(&bytes, "activation_mappings")
        }
    );
    assert_eq!(
        refusal(&read_v1(&fixture, &bytes)),
        &Error::Json {
            line: 1,
            column: json_error_column(&bytes, "temporal_bindings")
        }
    );
    let v2_bytes = v2_bytes(&fixture);
    assert_eq!(
        refusal(&read_v3(&fixture, &base, &v2_bytes, &[])),
        &Error::Json {
            line: 1,
            column: v2_bytes.len()
        }
    );
}

/// Byte offset just past the first `"<key>"` in `bytes`.
fn json_error_column(bytes: &[u8], key: &str) -> usize {
    let key = format!("\"{key}\"");
    std::str::from_utf8(bytes).unwrap().find(&key).unwrap() + key.len()
}

#[test]
#[trace("TC-143", "FR-054-AC-2", "FR-054-AC-3", "FR-054-AC-4")]
fn each_v3_mapping_refusal_names_the_control_declaration_order_or_inventory() {
    let fixture = Fixture::families();
    let mapped = mapped_base(&fixture);
    let exact = [(mapped.event.clone(), mapped.first)];
    let package = v3_package(&mapped.base, &exact);

    for control in [control(99), mapped.non_event.clone()] {
        let mut invalid = package.clone();
        invalid.activation_mappings[0].control = control;
        assert_eq!(
            v3_error(&fixture, &mapped, &invalid, &exact),
            Error::V3(v3::Refusal::Control)
        );
    }
    let mut non_temporal = package.clone();
    non_temporal.activation_mappings[0].temporal_declaration = 0;
    assert_eq!(
        v3_error(&fixture, &mapped, &non_temporal, &exact),
        Error::V3(v3::Refusal::TemporalDeclaration)
    );
    let mut substituted = package.clone();
    substituted.activation_mappings[0].temporal_declaration = mapped.second;
    assert_eq!(
        v3_error(&fixture, &mapped, &substituted, &exact),
        Error::V3(v3::Refusal::Expected(v3::MappingField::TemporalDeclaration))
    );

    let both = [
        (mapped.event.clone(), mapped.first),
        (mapped.other_event.clone(), mapped.first),
    ];
    let mut reordered = v3_package(&mapped.base, &both);
    reordered.activation_mappings.swap(0, 1);
    assert_eq!(
        v3_error(&fixture, &mapped, &reordered, &both),
        Error::V3(v3::Refusal::OfferOrder)
    );
    let mut missing = package.clone();
    missing.activation_mappings.clear();
    assert_eq!(
        v3_error(&fixture, &mapped, &missing, &exact),
        Error::V3(v3::Refusal::Mapping {
            side: v3::InventorySide::Offer,
            cause: v3::MappingCause::Missing,
        })
    );
    let mut duplicate = package.clone();
    duplicate
        .activation_mappings
        .push(duplicate.activation_mappings[0].clone());
    assert_eq!(
        v3_error(&fixture, &mapped, &duplicate, &exact),
        Error::V3(v3::Refusal::Mapping {
            side: v3::InventorySide::Offer,
            cause: v3::MappingCause::Duplicate,
        })
    );
    let mut surplus = package.clone();
    surplus
        .activation_mappings
        .push(v3::wire::ActivationMapping {
            control: mapped.other_event.clone(),
            temporal_declaration: mapped.first,
        });
    assert_eq!(
        v3_error(&fixture, &mapped, &surplus, &exact),
        Error::V3(v3::Refusal::Mapping {
            side: v3::InventorySide::Offer,
            cause: v3::MappingCause::Surplus,
        })
    );
    // The expected relation is independent of the offer: an exact offer against
    // an expected relation naming a different control is refused on that side.
    let elsewhere = [(mapped.other_event.clone(), mapped.first)];
    assert_eq!(
        v3_error(&fixture, &mapped, &package, &elsewhere),
        Error::V3(v3::Refusal::Mapping {
            side: v3::InventorySide::Expected,
            cause: v3::MappingCause::Missing,
        })
    );
}

#[test]
#[trace("TC-138", "TC-150", "FR-050-AC-5")]
fn v2_read_work_is_exactly_bounded_and_a_fresh_retry_is_reproducible() {
    let fixture = Fixture::families();
    let base = temporal_package(&fixture, 0);
    let bytes = serde_json::to_vec(&base).unwrap();
    let rows = rows(&base);
    let complete = read_rows(&fixture, &bytes, &rows);
    complete.result().expect("the exact table is admitted");
    let usage = complete.usage();
    assert!(
        usage.byte_work >= bytes.len(),
        "byte work charges at least the offered bytes"
    );
    for (dimension, amount) in [
        (Dimension::Entries, usage.entries),
        (Dimension::References, usage.references),
        (Dimension::ByteWork, usage.byte_work),
        (Dimension::OutputBytes, usage.output_bytes),
    ] {
        assert!(amount > 0, "{dimension:?}");
        let set = |limits: &mut Limits, value| match dimension {
            Dimension::Entries => limits.entries = value,
            Dimension::References => limits.references = value,
            Dimension::ByteWork => limits.byte_work = value,
            Dimension::OutputBytes => limits.output_bytes = value,
            _ => unreachable!(),
        };
        let mut short = Limits::default();
        set(&mut short, amount - 1);
        let report = read_rows_within(&fixture, &bytes, &rows, short);
        let Error::Incomplete(exhaustion) = report.result().unwrap_err() else {
            panic!("{dimension:?}: expected typed exhaustion")
        };
        assert_eq!(exhaustion.dimension, dimension);
        let mut exact = Limits::default();
        set(&mut exact, amount);
        assert!(read_rows_within(&fixture, &bytes, &rows, exact)
            .result()
            .is_ok());
    }
    let above_hard = Limits {
        payload_bytes: usize::MAX,
        output_bytes: usize::MAX,
        source_bytes: usize::MAX,
        content_bytes: usize::MAX,
        sources: usize::MAX,
        dependencies: usize::MAX,
        definitions: usize::MAX,
        models: usize::MAX,
        declarations: usize::MAX,
        entries: usize::MAX,
        references: usize::MAX,
        byte_work: usize::MAX,
        depth: usize::MAX,
        ..Limits::default()
    };
    let clamped = read_rows_within(&fixture, &bytes, &rows, above_hard);
    let hard_limits = Limits {
        payload_bytes: 64 * 1_048_576,
        output_bytes: 64 * 1_048_576,
        content_bytes: 64 * 1_048_576,
        entries: 10_000_000,
        byte_work: 512 * 1_048_576,
        depth: 1_000_000,
        ..Limits::default()
    };
    assert_eq!(clamped.limits(), hard_limits);
    assert!(clamped.result().is_ok());
    let retry = read_rows(&fixture, &bytes, &rows);
    assert_eq!(retry.usage(), usage);
    assert_eq!(
        retry.result().unwrap().package(),
        complete.result().unwrap().package()
    );
}

#[test]
#[trace("TC-121")]
fn a_binder_initializer_from_another_declaration_or_grouping_itself_is_refused() {
    let fixture = Fixture::families();
    // Declaration 1 is a predicate whose `prior` binder is initialized by value 1.
    let mut foreign = fixture.package.clone();
    foreign.declarations[1].binders[1]
        .initializer
        .0
        .as_mut()
        .expect("the let binder has an initializer")
        .declaration = 0;
    assert_eq!(
        refusal(&fixture.offered(&foreign)),
        &Error::Invalid(Invalid::Owner)
    );
    let mut recursive = fixture.package.clone();
    recursive.declarations[1].values[1].operation = w::ValueOperation::Group {
        value: w::Handle {
            declaration: 1,
            index: 1,
        },
    };
    assert_eq!(
        refusal(&fixture.offered(&recursive)),
        &Error::Invalid(Invalid::Cycle)
    );
}

/// The `/3` read of the authored finite-control protocol (declaration 0) and its
/// temporal declaration `Due` (declaration 1): the mapping is authored here, as
/// the independent expectation, and the bytes are encoded at test time.
#[test]
#[trace("TC-143", "FR-054-AC-1", "FR-054-AC-2", "FR-054-AC-3", "FR-054-AC-4")]
fn a_v3_event_control_mapped_to_a_temporal_declaration_is_admitted_exactly() {
    let fixture = Fixture::mapped();
    fixture
        .offered(&fixture.package)
        .result()
        .expect("the authored /1 package is admitted");
    let base = temporal_package(&fixture, 0);
    let due = base.temporal_bindings[0].declaration;
    assert_eq!(due, 1);
    // `Pulse` and `Matched` are the protocol's two plain event controls.
    let pulse = control(7);
    let matched = control(10);
    let mappings = [(pulse.clone(), due), (matched.clone(), due)];
    let package = v3_package(&base, &mappings);
    let bytes = v3::encode_candidate(&package, Limits::default())
        .into_result()
        .expect("canonical /3 candidate")
        .bytes()
        .to_vec();
    let read = read_v3(&fixture, &base, &bytes, &mappings);
    let admitted = read.result().expect("strict /3 reader");
    assert_eq!(
        admitted.activation_mappings(),
        [
            v3::wire::ActivationMapping {
                control: pulse.clone(),
                temporal_declaration: due
            },
            v3::wire::ActivationMapping {
                control: matched.clone(),
                temporal_declaration: due
            },
        ]
    );
    assert_eq!(admitted.package(), &package);

    let refuse = |offered: &v3::wire::Package, expected: &[(w::Handle, u32)]| {
        let bytes = serde_json::to_vec(offered).unwrap();
        refusal(&read_v3(&fixture, &base, &bytes, expected)).clone()
    };
    // A control that is not an event, and a target that is not temporal.
    let mut choice = package.clone();
    choice.activation_mappings[0].control = control(3);
    assert_eq!(refuse(&choice, &mappings), Error::V3(v3::Refusal::Control));
    let mut protocol_target = package.clone();
    protocol_target.activation_mappings[0].temporal_declaration = 0;
    assert_eq!(
        refuse(&protocol_target, &mappings),
        Error::V3(v3::Refusal::TemporalDeclaration)
    );
    // The expected relation is authored independently of the offer.
    let substituted = [(pulse.clone(), 0), (matched.clone(), due)];
    assert_eq!(
        refuse(&package, &substituted),
        Error::V3(v3::Refusal::Expected(v3::MappingField::TemporalDeclaration))
    );
    let mut reordered = package.clone();
    reordered.activation_mappings.swap(0, 1);
    assert_eq!(
        refuse(&reordered, &mappings),
        Error::V3(v3::Refusal::OfferOrder)
    );
    let mut missing = package.clone();
    missing.activation_mappings.pop();
    assert_eq!(
        refuse(&missing, &mappings),
        Error::V3(v3::Refusal::Mapping {
            side: v3::InventorySide::Offer,
            cause: v3::MappingCause::Missing,
        })
    );
    let mut duplicate = package.clone();
    duplicate.activation_mappings[1] = duplicate.activation_mappings[0].clone();
    assert_eq!(
        refuse(&duplicate, &mappings),
        Error::V3(v3::Refusal::Mapping {
            side: v3::InventorySide::Offer,
            cause: v3::MappingCause::Duplicate,
        })
    );
    let mut surplus = package.clone();
    surplus
        .activation_mappings
        .push(v3::wire::ActivationMapping {
            control: control(13),
            temporal_declaration: due,
        });
    assert_eq!(
        refuse(&surplus, &mappings),
        Error::V3(v3::Refusal::Mapping {
            side: v3::InventorySide::Offer,
            cause: v3::MappingCause::Surplus,
        })
    );
    // An expected relation naming a control the offer does not map.
    let elsewhere = [(pulse, due), (control(1), due)];
    assert_eq!(
        refuse(&package, &elsewhere),
        Error::V3(v3::Refusal::Mapping {
            side: v3::InventorySide::Expected,
            cause: v3::MappingCause::Missing,
        })
    );
}

/// Every refusal below is raised while the value graph of the protocol is built,
/// before the compensation's own members are validated, so the authored
/// compensation only has to name the capture and retry values.
#[test]
#[trace("TC-121")]
fn a_compensation_capture_or_retry_with_a_foreign_owner_or_a_cycle_is_refused_by_the_value_graph() {
    let (fixture, at) = Fixture::compensated();
    let base = fixture.offered(&fixture.package);
    // The authored compensation names only capture and retry values; its other
    // members are not a complete recovery contract, so the unmutated package is
    // refused at the compensation's bindings, after the value graph is built.
    assert_eq!(refusal(&base), &Error::Invalid(Invalid::Binding));
    let handle = |index: u32| w::Handle {
        declaration: 0,
        index,
    };
    let owner = Error::Invalid(Invalid::Owner);
    let cycle = Error::Invalid(Invalid::Cycle);
    type Mutation = Box<dyn Fn(&mut w::Package)>;
    let cases: Vec<(&str, Mutation, Error)> = vec![
        (
            "a capture initializer owned by another declaration",
            Box::new(move |p| {
                p.declarations[0].binders[at.registered as usize]
                    .initializer
                    .0
                    .as_mut()
                    .unwrap()
                    .declaration = 1
            }),
            owner.clone(),
        ),
        (
            "a retry value selecting a foreign value",
            Box::new(move |p| {
                p.declarations[0].values[at.retry as usize].operation = w::ValueOperation::Group {
                    value: w::Handle {
                        declaration: 1,
                        index: 0,
                    },
                }
            }),
            owner,
        ),
        (
            "a capture initializer that groups the retry that reads the capture",
            Box::new(move |p| {
                p.declarations[0].values[at.registered_initializer as usize].operation =
                    w::ValueOperation::Group {
                        value: handle(at.retry),
                    }
            }),
            cycle.clone(),
        ),
        (
            "two capture initializers that group each other",
            Box::new(move |p| {
                let d = &mut p.declarations[0];
                d.values[at.registered_initializer as usize].operation = w::ValueOperation::Group {
                    value: handle(at.activated_initializer),
                };
                d.values[at.activated_initializer as usize].operation = w::ValueOperation::Group {
                    value: handle(at.registered_initializer),
                };
            }),
            cycle,
        ),
    ];
    for (name, mutate, expected) in cases {
        let mut offered = fixture.package.clone();
        mutate(&mut offered);
        assert_eq!(refusal(&fixture.offered(&offered)), &expected, "{name}");
    }
}

const HALF: &str = r#""period":{"kind":"rational","numerator":"1","denominator":"2"}"#;

/// The `/2` and `/3` readers' result for `clock` offered as the (identical)
/// expected selection, with `edit` applied to the serialized text first.
fn clock_refusals(
    fixture: &Fixture,
    profile: R,
    clock: v2::wire::ClockConfiguration,
    edit: impl Fn(&str) -> String,
) -> (Error, Error) {
    let base = profiled(fixture, profile, clock);
    let text = |bytes: Vec<u8>| edit(std::str::from_utf8(&bytes).unwrap());
    let v2_text = text(serde_json::to_vec(&base).unwrap());
    let v2_error = refusal(&read_rows(fixture, v2_text.as_bytes(), &rows(&base))).clone();
    let v3_text = text(serde_json::to_vec(&v3_package(&base, &[])).unwrap());
    let v3_error = refusal(&read_v3(fixture, &base, v3_text.as_bytes(), &[])).clone();
    (v2_error, v3_error)
}

#[test]
#[trace("TC-138", "TC-150", "FR-050-AC-3")]
fn the_readers_refuse_a_period_that_is_not_a_positive_reduced_in_range_rational() {
    let fixture = Fixture::families();
    let numeric = |error| Error::Numeric(error);
    let cases = [
        (
            r#""period":{"kind":"integer","decimal":"0"}"#,
            Error::Invalid(Invalid::NumericDomain),
        ),
        (
            r#""period":{"kind":"integer","decimal":"-1"}"#,
            Error::Invalid(Invalid::NumericDomain),
        ),
        (
            r#""period":{"kind":"rational","numerator":"-1","denominator":"2"}"#,
            Error::Invalid(Invalid::NumericDomain),
        ),
        (
            r#""period":{"kind":"rational","numerator":"2","denominator":"4"}"#,
            numeric(NumberError::UnreducedRational),
        ),
        (
            r#""period":{"kind":"rational","numerator":"9223372036854775808","denominator":"1"}"#,
            numeric(NumberError::ComponentOutOfRange {
                component: NumberComponent::Numerator,
            }),
        ),
    ];
    for (period, expected) in cases {
        let (v2_error, v3_error) = clock_refusals(
            &fixture,
            R::FixedSample,
            sample("0", half(), "second"),
            |text| {
                assert!(
                    text.contains(HALF),
                    "the serialized period is spelled as expected"
                );
                text.replacen(HALF, period, 1)
            },
        );
        assert_eq!(v2_error, expected, "/2 {period}");
        assert_eq!(v3_error, expected, "/3 {period}");
    }
}

#[test]
#[trace("TC-138", "TC-150", "FR-050-AC-3")]
fn the_readers_refuse_an_overlong_clock_name_on_its_own_field() {
    let fixture = Fixture::families();
    let long = "x".repeat(4097);
    let cases = [
        (
            R::EventPosition,
            v2::wire::ClockConfiguration::EventPosition {
                sequence_authority: long.clone(),
            },
            v2::ClockField::SequenceAuthority,
        ),
        (
            R::FixedSample,
            sample("0", half(), &long),
            v2::ClockField::Unit,
        ),
        (
            R::TimestampedWindow,
            v2::wire::ClockConfiguration::TimestampedEvent {
                timestamp_unit: long.clone(),
            },
            v2::ClockField::TimestampUnit,
        ),
    ];
    for (profile, clock, field) in cases {
        let (v2_error, v3_error) = clock_refusals(&fixture, profile, clock, str::to_owned);
        assert_eq!(v2_error, clock_refusal(field), "/2 {field:?}");
        assert_eq!(v3_error, clock_refusal(field), "/3 {field:?}");
    }
}
