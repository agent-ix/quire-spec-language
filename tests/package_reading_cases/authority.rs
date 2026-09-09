// SPDX-License-Identifier: AGPL-3.0-only
//! FR-020: external authority and actual native compiler failures.
use super::*;
use quire_spec_language::{Phase, Source};

#[test]
#[trace("TC-082", "TC-086", "FR-020-AC-7", "FR-020-AC-10")]
fn ordered_import_occurrence_and_observation_claims_cannot_be_permuted() {
    let models = [native_rule_model::parts().model()];
    let base = rule(&models, "post", "pre(self.n) = self.n and result");
    let mut bindings = base.checked().bindings().clone();
    let text = bindings.source.source().text().replacen(
        "post Rule",
        &format!(
            "model Extra = \"example/rule-tests\" version \"1\" digest \"{}\";\npost Rule",
            models[0].digest()
        ),
        1,
    );
    let unit = parse(
        bindings.source.source().identity().clone(),
        "ordered.native",
        text.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    bindings.source = FormalSource::new(unit.source().clone(), bindings.source.identity().clone());
    let linked = link_native(unit, &models, LinkLimits::default()).unwrap();
    let checked = check(linked, bindings.clone(), CheckLimits::default()).unwrap();
    let package = NativePackage::new(checked, PackageLimits::default()).unwrap();
    let original: Value = serde_json::from_slice(package.bytes()).unwrap();
    read(&original, bindings.clone(), &models).unwrap();
    for pointer in [
        "/models",
        "/clauses/0/occurrences",
        "/clauses/0/runtime/context_observations",
        "/clauses/0/projections",
    ] {
        let mut changed = original.clone();
        let array = changed
            .pointer_mut(pointer)
            .unwrap()
            .as_array_mut()
            .unwrap();
        assert!(array.len() >= 2);
        array.reverse();
        assert_ne!(changed.pointer(pointer), original.pointer(pointer));
        let error = read(&changed, bindings.clone(), &models).unwrap_err();
        // Projection slots have different closed shapes; swapping them is
        // already invalid during typed intake. Other ordered claims reach comparison.
        let stage = if pointer.ends_with("/projections") {
            PackageStage::Decode
        } else {
            PackageStage::Compare
        };
        assert_eq!(
            (error.code, error.stage),
            (Code::InvalidPackage, stage),
            "{pointer}"
        );
    }
}

#[test]
#[trace("TC-085", "TC-082", "FR-020-AC-6", "FR-020-AC-10")]
fn equal_length_authored_conflicts_and_reordered_wire_clauses_refuse() {
    let models = [model()];
    let [_, _, vector] = package_vector_setup::cases();
    let bindings = package_vector_setup::checked(&vector, &models)
        .bindings()
        .clone();
    let original: Value = serde_json::from_slice(vector.artifact).unwrap();
    let mut duplicate_name = bindings.clone();
    duplicate_name.clauses[1].name = duplicate_name.clauses[0].name.clone();
    let mut duplicate_identity = bindings.clone();
    duplicate_identity.clauses[1].clause = duplicate_identity.clauses[0].clause.clone();
    for external in [duplicate_name, duplicate_identity] {
        let error = read(&original, external, &models).unwrap_err();
        assert_eq!(
            (error.code, error.stage),
            (Code::InvalidModelBinding, PackageStage::Rebind)
        );
    }
    let mut changed = original.clone();
    changed["clauses"].as_array_mut().unwrap().reverse();
    let error = read(&changed, bindings, &models).unwrap_err();
    assert_eq!(
        (error.code, error.stage),
        (Code::InvalidPackage, PackageStage::Compare)
    );
}

#[test]
#[trace("TC-085", "FR-020-AC-5", "FR-020-AC-6")]
fn source_and_authored_axes_are_checked_against_external_bindings() {
    let models = [model()];
    let [vector, _, _] = package_vector_setup::cases();
    let bindings = package_vector_setup::checked(&vector, &models)
        .bindings()
        .clone();
    let original: Value = serde_json::from_slice(vector.artifact).unwrap();
    for (pointer, value, code) in [
        (
            "/source/identity",
            json!("foreign"),
            Code::InvalidModelBinding,
        ),
        (
            "/source/revision",
            json!("foreign"),
            Code::InvalidModelBinding,
        ),
        (
            "/source/digest",
            json!(ByteDigest::of(b"foreign").to_string()),
            Code::StaleDependency,
        ),
        (
            "/source/formal/document",
            json!("Foreign"),
            Code::InvalidModelBinding,
        ),
        (
            "/source/formal/revision",
            json!(2),
            Code::InvalidModelBinding,
        ),
        (
            "/clauses/0/name",
            json!("Foreign"),
            Code::InvalidModelBinding,
        ),
        (
            "/clauses/0/owner/package",
            json!("example/foreign"),
            Code::InvalidModelBinding,
        ),
        (
            "/clauses/0/owner/requirement",
            json!("Foreign"),
            Code::InvalidModelBinding,
        ),
        (
            "/clauses/0/owner/revision",
            json!(3),
            Code::InvalidModelBinding,
        ),
        (
            "/clauses/0/clause",
            json!("foreign"),
            Code::InvalidModelBinding,
        ),
        (
            "/clauses/0/execution_point/name",
            json!("foreign"),
            Code::InvalidModelBinding,
        ),
    ] {
        let mut value_changed = original.clone();
        *value_changed.pointer_mut(pointer).unwrap() = value;
        let error = read(&value_changed, bindings.clone(), &models).unwrap_err();
        assert_eq!(
            (error.code, error.stage),
            (code, PackageStage::Rebind),
            "{pointer}"
        );
        assert!(error.usage.checking.is_none());
    }
    let mut missing = bindings.clone();
    missing.clauses.clear();
    let mut duplicate = bindings.clone();
    duplicate.clauses.push(duplicate.clauses[0].clone());
    let mut foreign = bindings.clone();
    foreign.clauses[0].name = "Foreign".into();
    for external in [missing, duplicate, foreign] {
        assert_eq!(
            read(&original, external, &models).unwrap_err().code,
            Code::InvalidModelBinding
        );
    }
    assert_eq!(
        read(&original, bindings.clone(), &[]).unwrap_err().code,
        Code::MissingImport
    );
}

#[test]
#[trace("TC-085", "FR-020-AC-5")]
fn conflicting_unselected_inventory_is_not_hidden_by_wire_imports() {
    let original_model = native_rule_model::parts().model();
    let changed = native_rule_model::from_text(
        &(native_rule_model::FIXTURE.to_owned() + " "),
        "changed.native-model",
        "draft:1",
    )
    .unwrap()
    .model();
    assert_eq!(
        original_model.environment().owner(),
        changed.environment().owner()
    );
    assert_ne!(original_model.digest(), changed.digest());
    let models = [original_model, changed];
    let package = rule(&models[..1], "invariant", "true");
    for (offered, code) in [
        (&models[..], Code::InvalidModelBinding),
        (&models[1..], Code::StaleDependency),
    ] {
        let error = NativePackage::read_verified(
            package.bytes(),
            package.reference(),
            package.checked().bindings().clone(),
            offered,
            &PackageSupport::default(),
            PackageReadLimits::default(),
        )
        .unwrap_err();
        assert_eq!((error.code, error.stage), (code, PackageStage::Rebind));
        assert!(matches!(error.cause, Some(PackageCause::Native(_))));
    }
}

#[test]
#[trace("TC-087", "FR-020-AC-8")]
fn forged_checked_claims_cannot_bypass_real_parse_link_type_or_definedness_failures() {
    let models = [native_rule_model::parts().model()];
    let package = rule(&models, "invariant", "true");
    let bindings = package.checked().bindings().clone();
    let original: Value = serde_json::from_slice(package.bytes()).unwrap();
    let text = bindings.source.source().text();
    let header = include_str!("../fixtures/native-package/header.native");
    let imported = text.split_once("invariant Rule").unwrap().0;
    for (changed, code, phase) in [
        (header.to_string(), Code::InvalidSyntax, Phase::Parse),
        (imported.to_string(), Code::InvalidSyntax, Phase::Parse),
        (
            text.replace("M::Node", "M::Absent"),
            Code::MissingDeclaration,
            Phase::Link,
        ),
        (
            text.replace("{ true }", "{ 1 }"),
            Code::IllTyped,
            Phase::Check,
        ),
        (
            text.replace("{ true }", "{ self.signed div self.den = self.signed }"),
            Code::UndefinedExpression,
            Phase::Check,
        ),
    ] {
        assert_ne!(changed, text, "fixture changes actual source");
        let native = Source::read(
            bindings.source.source().identity().clone(),
            "reader-rule.native",
            changed.as_bytes(),
            1_048_576,
        )
        .unwrap();
        let mut external = bindings.clone();
        external.source = FormalSource::new(native.clone(), bindings.source.identity().clone());
        let mut wire = original.clone();
        wire["source"]["digest"] = json!(native.digest().to_string());
        let direct = parse(
            native.identity().clone(),
            native.path(),
            native.text().as_bytes(),
            Limits::default(),
        )
        .and_then(|unit| link_native(unit, &models, LinkLimits::default()))
        .and_then(|linked| check(linked, external.clone(), CheckLimits::default()))
        .unwrap_err();
        assert_eq!(
            (direct.code, direct.phase),
            (code, phase),
            "independent expected native refusal"
        );
        let error = read(&wire, external, &models).unwrap_err();
        assert_eq!((error.code, error.stage), (code, PackageStage::Rebind));
        let Some(PackageCause::Native(cause)) = error.cause else {
            panic!("missing original native cause");
        };
        assert_eq!(cause.phase, phase);
        assert_eq!(cause.span, direct.span);
        assert!(error.usage.derive.is_none());
    }
}
