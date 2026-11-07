// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-050-AC-4: the compiled-protocol `/1` and `/2` readers reject each other's
//! bytes and neither upgrades, strips or invents a temporal selection.
//!
//! Both byte streams are encoded here from the authored reader fixture by the
//! public `/1` and `/2` candidate encoders; no byte sequence is frozen.

use crate::support::protocol_artifact::Fixture;

use ix_trace_rs::trace;
use quire_spec_language::protocol_artifact::{
    self as artifact, v2, wire as w, Error, Limits, Unsupported,
};
use serde_json::Value;

fn v1_bytes(fixture: &Fixture) -> Vec<u8> {
    fixture.candidate().bytes().to_vec()
}

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

fn v1_selection() -> w::ArtifactRef {
    Fixture::selected_artifact()
}

fn v2_selection() -> w::ArtifactRef {
    let mut selected = Fixture::selected_artifact();
    selected.wire.version = "2".into();
    selected
}

fn read_v1(fixture: &Fixture, bytes: &[u8]) -> artifact::Report<artifact::AdmittedPackage> {
    fixture.read(bytes, &v1_selection(), Limits::default())
}

fn read_v2(fixture: &Fixture, bytes: &[u8]) -> artifact::Report<v2::AdmittedPackage> {
    fixture.with_expected(&v2_selection(), |inherited| {
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
    let v1 = read_v1(&fixture, &v1_bytes(&fixture));
    assert_eq!(
        v1.result().expect("the /1 reader admits /1").package(),
        &fixture.package
    );
    let v2 = read_v2(&fixture, &v2_bytes(&fixture));
    let admitted = v2.result().expect("the /2 reader admits /2");
    assert_eq!(admitted.package(), &v2_package(&fixture));
    assert!(admitted.package().temporal_bindings.is_empty());
}

#[test]
#[trace("TC-138", "TC-150", "FR-050-AC-4")]
fn the_v2_reader_refuses_v1_bytes_at_the_wire_header() {
    let fixture = Fixture::new();
    // The /1 bytes carry no temporal table, so the /2 reader cannot decode them.
    assert!(matches!(
        refusal(&read_v2(&fixture, &v1_bytes(&fixture))),
        Error::Json { .. }
    ));
    // With an empty table added, they decode and are refused on the header the
    // /1 producer wrote: the reader does not accept `/1` as `/2`.
    let mut value: Value = serde_json::from_slice(&v1_bytes(&fixture)).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("temporal_bindings".into(), Value::Array(Vec::new()));
    let upgraded = serde_json::to_vec(&value).unwrap();
    assert_eq!(
        refusal(&read_v2(&fixture, &upgraded)),
        &Error::V2(v2::Refusal::Header(v2::HeaderField::Wire))
    );
}

#[test]
#[trace("TC-138", "TC-150", "FR-050-AC-4")]
fn the_v1_reader_refuses_v2_bytes_and_a_stripped_table_does_not_downgrade_them() {
    let fixture = Fixture::new();
    assert!(matches!(
        refusal(&read_v1(&fixture, &v2_bytes(&fixture))),
        Error::Json { .. }
    ));
    // Removing the /2 table leaves the /2 headers: the /1 reader refuses them
    // rather than reading the remainder as /1.
    let mut value: Value = serde_json::from_slice(&v2_bytes(&fixture)).unwrap();
    value.as_object_mut().unwrap().remove("temporal_bindings");
    let stripped = serde_json::to_vec(&value).unwrap();
    assert_ne!(stripped, v1_bytes(&fixture));
    assert_eq!(
        refusal(&read_v1(&fixture, &stripped)),
        &Error::Unsupported(Unsupported::Wire)
    );
}

#[test]
#[trace("TC-138", "TC-150", "FR-050-AC-4")]
fn a_v2_selection_does_not_admit_v1_bytes_and_a_v1_selection_does_not_admit_v2_bytes() {
    let fixture = Fixture::new();
    // The independently selected artifact wire version is part of each reader's
    // contract: the same bytes under the other version's selection are refused.
    let wrong_v1 = fixture.read(&v2_bytes(&fixture), &v1_selection(), Limits::default());
    assert!(wrong_v1.result().is_err());
    let wrong_v2 = fixture.with_expected(&v1_selection(), |inherited| {
        let expected = v2::Expected {
            inherited: *inherited,
            temporal: &[],
        };
        v2::read(&v2_bytes(&fixture), &expected, Limits::default())
    });
    assert_eq!(
        refusal(&wrong_v2),
        &Error::V2(v2::Refusal::Header(v2::HeaderField::ArtifactVersion))
    );
}
