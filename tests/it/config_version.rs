// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-032: the concrete named model and repeatable fixture files.

use crate::support::config_version as fixtures;

use fixtures::Case;
use ix_trace_rs::trace;
use qsl_foundation::ByteDigest;
use quire_contract_model as ir;
use serde_json::Value;

#[test]
#[trace("TC-110", "FR-032-AC-1")]
fn concrete_model_retains_exact_bounds_roles_and_original_source() {
    let model = fixtures::model().unwrap();
    assert_eq!(
        model.environment().owner().package().as_str(),
        "example/config-version"
    );
    assert_eq!(
        model.source().source().digest(),
        ByteDigest::of(fixtures::MODEL.as_bytes())
    );
    let record = model
        .environment()
        .types()
        .iter()
        .find_map(|ty| match ty {
            ir::TypeDeclaration::Record { declaration }
                if declaration.name().as_str() == "ConfigVersion" =>
            {
                Some(declaration)
            }
            _ => None,
        })
        .unwrap();
    let version = record
        .fields()
        .iter()
        .find(|field| field.name().as_str() == "versionNumber")
        .unwrap();
    assert_eq!(
        version.value_type(),
        &ir::ValueType::integer(
            ir::IntegerType::new(
                ir::IntegerDomain::Signed,
                0,
                1000,
                ir::OverflowPolicy::Reject
            )
            .unwrap()
        )
    );
    let object = &model.roles().objects[0];
    assert_eq!(object.record.as_str(), "ConfigVersion");
    assert_eq!(object.reference.as_str(), "ConfigVersionRef");
    assert_eq!(object.universe.as_str(), "config_history");
    assert_eq!(object.identity_field.as_str(), "id");
    let parent = record
        .fields()
        .iter()
        .find(|field| field.name().as_str() == "parent")
        .unwrap();
    assert_eq!(
        parent.value_type(),
        &ir::ValueType::option(ir::ValueType::Record {
            name: ir::SymbolName::new("ConfigVersionRef").unwrap()
        })
    );
    let operation = &model.roles().operations[0];
    assert_eq!(operation.name.as_str(), "attemptUpdate");
    assert_eq!(
        operation.result.as_ref().unwrap().as_str(),
        "attempt_result"
    );
    assert_eq!(operation.frame.fields.len(), 1);
    assert_eq!(operation.frame.fields[0].0.as_str(), "ConfigVersion");
    assert_eq!(operation.frame.fields[0].1.as_str(), "versionNumber");
    assert!(operation.frame.created.is_empty() && operation.frame.deleted.is_empty());
    for (source, key, expected) in [
        (version.source(), "name", "versionNumber"),
        (parent.source(), "name", "parent"),
        (&object.source, "record", "ConfigVersion"),
        (&operation.source, "name", "attemptUpdate"),
    ] {
        let span = model.source().to_native(source).unwrap();
        let declaration: Value =
            serde_json::from_str(model.source().source().slice(span).unwrap()).unwrap();
        assert_eq!(declaration[key], expected);
    }
}

#[test]
#[trace("TC-110", "FR-032-AC-1", "FR-032-AC-3")]
fn fixture_output_is_repeatable_and_filesystem_failure_is_returned() {
    let directory = tempfile::tempdir().unwrap();
    let model = fixtures::model().unwrap();
    let first = directory.path().join("first");
    let second = directory.path().join("second");
    fixtures::write(&first, &model, Case::Healthy).unwrap();
    fixtures::write(&second, &model, Case::Healthy).unwrap();
    for entry in std::fs::read_dir(&first).unwrap() {
        let entry = entry.unwrap();
        assert_eq!(
            std::fs::read(entry.path()).unwrap(),
            std::fs::read(second.join(entry.file_name())).unwrap()
        );
    }
    let file = directory.path().join("occupied");
    std::fs::write(&file, b"existing file").unwrap();
    assert!(fixtures::write(&file, &model, Case::Healthy).is_err());
    assert_eq!(std::fs::read(file).unwrap(), b"existing file");
}
