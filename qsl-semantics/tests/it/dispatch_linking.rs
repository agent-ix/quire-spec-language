// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-102, FR-103, FR-302: intake reads an operation's `redefines`, and the
//! unit compile links dispatch for each called operation
//! (`PackageDeclarations::link_dispatch`) from the retained bodies and unit
//! clauses and the admitted model view.
//!
//! The model is `ConfigVersion` (`A`) with a chain of subtypes `B`, `C`, `D`
//! each redefining `size`, over the `example/config-version` document.

use ix_trace_rs::trace;
use qsl_semantics::model::domain_package::{DomainPackageRecord, OperationMemberRecord};
use qsl_semantics::model::key::DeclarationKey;
use serde_json::{json, Value};

use super::model_operations::{
    config_version_document_with_operations, empty_frame, operation, operation_parameter,
    PACKAGE_IDENTITY,
};

pub(super) fn key(owner: &str, name: &str) -> DeclarationKey {
    DeclarationKey {
        package: PACKAGE_IDENTITY.to_owned(),
        node: format!("ix://{PACKAGE_IDENTITY}/{owner}/{name}"),
    }
}

/// A query `owner.name(params): Integer` with empty `requires`/`ensures`,
/// redefining `redefines.name` when given.
pub(super) fn query(
    owner: &str,
    name: &str,
    params: &[(&str, &str)],
    redefines: Option<&str>,
) -> Value {
    let parameters: Vec<Value> = params
        .iter()
        .map(|(parameter, type_ref)| {
            let mut value = operation_parameter(name, parameter, type_ref);
            let identity = format!("ix://{PACKAGE_IDENTITY}/{owner}/{name}/{parameter}");
            value["identity"] = json!(identity);
            value
        })
        .collect();
    let mut value = operation(
        name,
        json!(parameters),
        Some("ix://quire/native/Integer"),
        empty_frame(),
    );
    value["identity"] = json!(format!("ix://{PACKAGE_IDENTITY}/{owner}/{name}"));
    if let Some(parent) = redefines {
        value["redefines"] = json!(format!("ix://{PACKAGE_IDENTITY}/{parent}/{name}"));
    }
    value
}

/// The document: `ConfigVersion` declaring `root_operations`, and each of
/// `subtypes` as `(name, supertype, operations)`.
pub(super) fn document(
    root_operations: Vec<Value>,
    subtypes: &[(&str, &str, Vec<Value>)],
) -> Vec<u8> {
    let mut envelope: Value =
        serde_json::from_slice(&config_version_document_with_operations(root_operations))
            .expect("valid JSON");
    for (name, supertype, operations) in subtypes {
        let identity = format!("ix://{PACKAGE_IDENTITY}/{name}");
        envelope["types"]
            .as_array_mut()
            .expect("types")
            .push(json!({
                "identity": identity,
                "displayName": identity,
                "kind": {"module": PACKAGE_IDENTITY, "name": "object_type"},
                "roles": [],
                "origin": {
                    "generated": {
                        "generatorIdentity": identity,
                        "generatorVersion": "1.0.0",
                        "inputIdentities": [identity],
                    }
                },
                "constraints": [],
                "extensions": [],
                "unknownPolicy": "reject",
                "supertypes": [format!("ix://{PACKAGE_IDENTITY}/{supertype}")],
                "fields": [],
                "operations": operations,
            }));
    }
    serde_json::to_vec(&envelope).expect("valid JSON")
}

pub(super) fn operation_record(
    package: &qsl_semantics::check::PackageDeclarations,
    target: &DeclarationKey,
) -> OperationMemberRecord {
    package.selected_views["Config"]
        .domain_package()
        .records
        .iter()
        .find_map(|record| match record {
            DomainPackageRecord::OperationMember(operation) if &operation.key == target => {
                Some(operation.clone())
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("the view declares {target:?}"))
}

/// FR-103: intake reads an operation's inline `redefines` property, the edge
/// dispatch linking walks to build a family. An operation without the
/// property redefines nothing.
#[trace("FR-103-AC-10")]
#[test]
fn intake_reads_an_operations_redefines_property() {
    let document = document(
        vec![query("ConfigVersion", "size", &[], None)],
        &[(
            "Sub",
            "ConfigVersion",
            vec![query("Sub", "size", &[], Some("ConfigVersion"))],
        )],
    );
    let package = super::model_operations::admit_and_assemble_with_body(
        &document,
        "body forA using v on Config::ConfigVersion::size() { 1 }\n\
         body forB using v on Config::Sub::size() { 2 }\n",
    )
    .unwrap_or_else(|refusal| panic!("{refusal:?}"));
    assert_eq!(
        operation_record(&package, &key("Sub", "size")).redefines,
        Some(key("ConfigVersion", "size"))
    );
    assert_eq!(
        operation_record(&package, &key("ConfigVersion", "size")).redefines,
        None
    );
}
