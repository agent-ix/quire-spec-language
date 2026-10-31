// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-081 identity boundaries (TC-216 and TC-217).

use ix_trace_rs::trace;
use qsl_foundation::IntakeLimits;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::domain_package::{DomainPackage, DomainPackageRef};
use qsl_semantics::model::intake::{admit, package_input, read_domain_package};
use qsl_semantics::model::normalize::{normalize, EffectiveView, NormalizeOutcome};
use serde_json::{json, Value};
use std::process::Command;

use crate::tc235_k2;

fn domain_package(document: &Value) -> DomainPackage {
    let bytes = serde_json::to_vec(document).expect("the K2 document serializes");
    let offered = package_input([bytes.as_slice()]).expect("the package input is well formed");
    let selection = DomainPackageRef {
        identity: tc235_k2::PACKAGE.to_owned(),
    };
    let (_, admitted) = admit(
        selection.identity.as_str(),
        &offered,
        IntakeLimits::default(),
    )
    .expect("the package bytes admit");
    read_domain_package(selection, &admitted).expect("the domain package reads")
}

fn normalized(document: &Value) -> EffectiveView {
    match normalize(
        &domain_package(document),
        ModelNormalizationLimits::UNLIMITED,
    ) {
        NormalizeOutcome::Completed(view) => view,
        other => panic!("the domain package normalizes, got {other:?}"),
    }
}

fn identity_output(document: &Value) -> String {
    format!("{:?}", normalized(document).declarations())
}

#[trace("TC-216", "FR-081-AC-4")]
#[test]
fn changing_only_a_display_name_preserves_keys_identities_and_order() {
    let original = tc235_k2::k2();
    let renamed = tc235_k2::mutate_k2(|document| {
        tc235_k2::type_named_mut(document, "Order")["displayName"] = json!("Sales Order (legacy)");
    });

    let original_view = normalized(&original);
    let renamed_view = normalized(&renamed);
    assert_eq!(
        original_view.declarations(),
        renamed_view.declarations(),
        "displayName does not contribute to original keys, effective identities or ordering"
    );
}

#[trace("TC-217", "FR-081-AC-5")]
#[test]
fn an_intervening_package_does_not_change_normalization_results() {
    let package = tc235_k2::k2();
    let before = identity_output(&package);

    let mut unrelated = package.clone();
    unrelated["types"]
        .as_array_mut()
        .expect("types are an array")
        .push(tc235_k2::declaration(
            "Other",
            "thing",
            json!({
                "identityFields": [tc235_k2::id("Other/id")],
                "fields": [tc235_k2::field("Other", "id", "ix://quire/native/UUID")],
            }),
        ));
    let _ = identity_output(&unrelated);

    assert_eq!(before, identity_output(&package));

    let output = Command::new(std::env::current_exe().expect("the test executable path"))
        .arg("--exact")
        .arg("model_identity_contracts::tc217_child_reports_normalization_output")
        .arg("--nocapture")
        .env("QSL_TC217_CHILD", "1")
        .output()
        .expect("the isolated test process starts");
    assert!(
        output.status.success(),
        "the isolated test process succeeds: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("test output is UTF-8");
    let child_output = stdout
        .lines()
        .find_map(|line| line.strip_prefix("QSL_TC217_OUTPUT="))
        .expect("the child emitted its normalized identities");
    assert_eq!(before, child_output);
}

#[test]
fn tc217_child_reports_normalization_output() {
    if std::env::var_os("QSL_TC217_CHILD").is_some() {
        println!("QSL_TC217_OUTPUT={}", identity_output(&tc235_k2::k2()));
    }
}
