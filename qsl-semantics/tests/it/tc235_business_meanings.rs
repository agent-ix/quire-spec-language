// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-235 business-meaning vectors at the real QSL intake/normalization boundary.

use ix_trace_rs::trace;
use qsl_forms::build_unit;
use qsl_foundation::diagnostic::Code;
use qsl_foundation::{IntakeLimits, SourceIdentity};
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::model::accounting::{ChargePoint, Meter, ModelNormalizationLimits};
use qsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, ProcessStepKind,
};
use qsl_semantics::model::intake::{
    admit, admit_unit, meaning, package_input, read_domain_package, read_records,
};
use qsl_semantics::model::key::{DeclarationKey, SHA256_JCS_DIGEST_DOMAIN};
use qsl_semantics::model::normalize::{
    normalize_with_meter, ModelRefusal, ModelRefusalCause, NormalizeOutcome,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

#[path = "../support/tc235_k2.rs"]
mod tc235_k2;
use tc235_k2::*;

fn read_k2(document: &Value) -> Result<DomainPackage, Vec<ModelRefusal>> {
    let bytes = serde_json::to_vec(document).expect("K2 serializes");
    let offered = package_input([bytes.as_slice()]);
    let digest = *offered.keys().next().expect("one offered package");
    let selection = DomainPackageRef {
        identity: PACKAGE.to_owned(),
        version: "1".to_owned(),
        digest,
    };
    let (_selection, admitted) = admit(
        &selection,
        SHA256_JCS_DIGEST_DOMAIN,
        &offered,
        IntakeLimits::default(),
    )
    .expect("S1 admits the authored package bytes");
    read_domain_package(selection, &admitted)
}

fn node_named_mut<'a>(value: &'a mut Value, identity: &str) -> Option<&'a mut Value> {
    if value.get("identity").and_then(Value::as_str) == Some(identity) {
        return Some(value);
    }
    match value {
        Value::Array(values) => values
            .iter_mut()
            .find_map(|value| node_named_mut(value, identity)),
        Value::Object(values) => values
            .values_mut()
            .find_map(|value| node_named_mut(value, identity)),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => None,
    }
}

fn refresh_manifest_digest(document: &mut Value) {
    let digest = jcs_digest(&document["types"]);
    document["source"]["digest"] = json!(digest);
    document["package"]["manifestDigest"] = json!(digest);
    document["package"]["lockDigest"] = json!(digest);
    for construct in document["constructs"].as_array_mut().unwrap() {
        construct["manifestDigest"] = json!(digest);
    }
}

fn tc213_fixture() -> Value {
    let mut count = declaration("Count", "value", json!({}));
    count["target"] = json!("ix://quire/native/Integer");
    let mut document = domain_document(
        vec![
            declaration(
                "A",
                "thing",
                json!({
                    "identityFields": [id("A/x")],
                    "fields": [field("A", "x", "ix://quire/native/UUID")],
                    "operations": [operation("A", "set")],
                }),
            ),
            declaration(
                "B",
                "thing",
                json!({
                    "identityFields": [id("A/x")],
                    "supertypes": [id("A")],
                }),
            ),
            count,
        ],
        vec![
            construct("thing", meaning::OBJECT_TYPE),
            construct("value", meaning::VALUE_TYPE),
        ],
    );
    let source = |node: &str, end: Option<(u64, u64)>| {
        let mut origin = json!({
            "source": {
                "sourceIdentity": "ix://test/origin/spec",
                "path": format!("spec/{node}.md"),
                "startLine": if node == "A" { 2 } else { 3 },
                "startColumn": if node == "A" { 1 } else { 2 },
            }
        });
        if let Some((line, column)) = end {
            origin["source"]["endLine"] = json!(line);
            origin["source"]["endColumn"] = json!(column);
        }
        origin
    };
    for (node, origin) in [
        ("A", source("A", Some((8, 9)))),
        ("B", source("B", None)),
        ("A/x", source("A-x", Some((4, 12)))),
        ("A/set", source("A-set", Some((5, 14)))),
        (
            "Count",
            json!({"generated": {
                "generatorIdentity": "ix://test/origin/generator",
                "generatorVersion": "1.2.3",
                "inputIdentities": [id("A"), id("B")],
            }}),
        ),
    ] {
        node_named_mut(&mut document, &id(node))
            .unwrap_or_else(|| panic!("TC-213 fixture has {node}"))["origin"] = origin;
    }
    refresh_manifest_digest(&mut document);
    document
}

fn tc213_retained_map(
    document: &Value,
) -> (BTreeSet<DeclarationKey>, BTreeMap<DeclarationKey, Value>) {
    let mut document = document.clone();
    refresh_manifest_digest(&mut document);
    let package = read_k2(&document).expect("TC-213 fixture admits");
    let admitted_keys = package
        .records
        .iter()
        .map(|record| record.key().clone())
        .collect::<BTreeSet<_>>();
    let admitted_origins = package
        .records
        .iter()
        .map(|record| {
            (
                record.key().clone(),
                package
                    .original(record.key())
                    .expect("every TC-213 record has an origin")
                    .origin
                    .clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let (outcome, _) = normalize_with_meter(&package, ModelNormalizationLimits::UNLIMITED);
    let NormalizeOutcome::Completed(view) = outcome else {
        panic!("TC-213 fixture normalizes: {outcome:?}");
    };
    let retained = view.domain_package();
    let retained_origins = retained
        .records
        .iter()
        .map(|record| {
            (
                record.key().clone(),
                retained
                    .original(record.key())
                    .expect("normalized inventory retains every origin")
                    .origin
                    .clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(retained_origins, admitted_origins);
    assert_eq!(
        retained_origins.keys().cloned().collect::<BTreeSet<_>>(),
        admitted_keys
    );
    for entry in view.declarations() {
        assert!(admitted_keys.contains(&entry.preimage.original));
        for fact in &entry.preimage.derivation {
            for key in fact.inputs.iter() {
                assert!(
                    admitted_keys.contains(key),
                    "fact {:?} cites a key outside the retained inventory",
                    fact
                );
            }
        }
    }
    (admitted_keys, retained_origins)
}

fn tc213_type_identities(
    document: &Value,
) -> BTreeMap<DeclarationKey, qsl_semantics::model::key::EffectiveId> {
    let mut document = document.clone();
    refresh_manifest_digest(&mut document);
    let package = read_k2(&document).expect("TC-213 fixture admits");
    let (outcome, _) = normalize_with_meter(&package, ModelNormalizationLimits::UNLIMITED);
    let NormalizeOutcome::Completed(view) = outcome else {
        panic!("TC-213 fixture normalizes: {outcome:?}");
    };
    view.type_identities().clone()
}

fn tc213_view_identity(document: &Value) -> qsl_semantics::model::key::EffectiveId {
    let mut document = document.clone();
    refresh_manifest_digest(&mut document);
    let package = read_k2(&document).expect("TC-213 fixture admits");
    let (outcome, _) = normalize_with_meter(&package, ModelNormalizationLimits::UNLIMITED);
    let NormalizeOutcome::Completed(view) = outcome else {
        panic!("TC-213 fixture normalizes: {outcome:?}");
    };
    view.identity()
}

#[trace("TC-213", "FR-081-AC-1", "FR-081-AC-7")]
#[test]
fn tc213_retains_exact_key_origin_map_across_order_selection_and_origin_controls() {
    let baseline = tc213_retained_map(&tc213_fixture());
    let baseline_ids = tc213_type_identities(&tc213_fixture());
    let baseline_view_id = tc213_view_identity(&tc213_fixture());

    let mut reversed = tc213_fixture();
    reversed["types"].as_array_mut().unwrap().reverse();
    let reordered = tc213_retained_map(&reversed);
    assert_eq!(
        reordered, baseline,
        "record order cannot change correspondence"
    );
    assert_eq!(
        tc213_type_identities(&reversed),
        baseline_ids,
        "equal declaration keys and facts retain their type identities"
    );
    assert_ne!(
        tc213_view_identity(&reversed),
        baseline_view_id,
        "a different package selection changes the effective-view identity"
    );

    let mut selected = tc213_fixture();
    selected["types"][0]["origin"]["source"]["endColumn"] = json!(17);
    selected["types"].as_array_mut().unwrap().reverse();
    let changed_selection = tc213_retained_map(&selected);
    assert_eq!(changed_selection.0, baseline.0);
    assert_ne!(
        changed_selection.1, baseline.1,
        "the control changed one origin only"
    );
    assert_eq!(
        changed_selection.1[&DeclarationKey::fixture("ix://test/orders/A")]["source"]["endColumn"],
        17
    );
    assert_eq!(
        tc213_type_identities(&selected),
        baseline_ids,
        "equal declaration keys and facts retain their type identities"
    );
    assert_ne!(
        tc213_view_identity(&selected),
        baseline_view_id,
        "a digest-changing selection changes the effective-view identity"
    );

    for (label, mutate) in [
        (
            "swap source origins",
            Box::new(|document: &mut Value| {
                let a = node_named_mut(document, &id("A")).unwrap()["origin"].clone();
                let b = node_named_mut(document, &id("B")).unwrap()["origin"].clone();
                node_named_mut(document, &id("A")).unwrap()["origin"] = b;
                node_named_mut(document, &id("B")).unwrap()["origin"] = a;
            }) as Box<dyn Fn(&mut Value)>,
        ),
        (
            "change generated version",
            Box::new(|document: &mut Value| {
                node_named_mut(document, &id("Count")).unwrap()["origin"]["generated"]
                    ["generatorVersion"] = json!("9.9.9");
            }),
        ),
        (
            "reverse generated inputs",
            Box::new(|document: &mut Value| {
                node_named_mut(document, &id("Count")).unwrap()["origin"]["generated"]
                    ["inputIdentities"] = json!([id("B"), id("A")]);
            }),
        ),
        (
            "remove optional source endLine",
            Box::new(|document: &mut Value| {
                node_named_mut(document, &id("A")).unwrap()["origin"]["source"]
                    .as_object_mut()
                    .unwrap()
                    .remove("endLine");
            }),
        ),
        (
            "replace generated origin with source origin",
            Box::new(|document: &mut Value| {
                node_named_mut(document, &id("Count")).unwrap()["origin"] = json!({
                    "source": {
                        "sourceIdentity": "ix://test/origin/spec",
                        "path": "spec/Count.md",
                        "startLine": 9,
                        "startColumn": 1,
                    }
                });
            }),
        ),
    ] {
        let mut mutant = tc213_fixture();
        mutate(&mut mutant);
        let actual = tc213_retained_map(&mutant);
        assert_eq!(actual.0, baseline.0, "{label} must not re-key declarations");
        assert_ne!(
            actual.1, baseline.1,
            "{label} must remain observable by key"
        );
    }
}

/// Admit a K2-derived package as model `M` and assemble a source unit which
/// uses its record and event value types. This deliberately runs the public
/// intake -> normalization -> PackageDeclarations::assemble path.
fn assemble_k2_unit(document: &Value, declarations: &str) -> PackageDeclarations {
    let bytes = serde_json::to_vec(document).expect("K2-derived fixture serializes");
    let packages = package_input([bytes.as_slice()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one K2-derived model package");
    };
    let digest = qsl_semantics::model::key::hex(digest);
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model M = {PACKAGE:?} version \"1\" digest \"sha256-jcs:{digest}\";\n\
         {declarations}\n"
    );
    let parsed = qsl_cst::parse(
        SourceIdentity::new("test", "tc235-assembly", "fixture", "fixture:1"),
        "tc235-assembly.native",
        source.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the source unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the source unit");
    let selected = admit_unit(
        &unit.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("I1 admits and normalizes the K2-derived model");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        selected,
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("model value assembly succeeds: {refusal:?}"))
}

/// Intake's own admitted record keys remain the exact original inventory of
/// the completed view, including declarations without effective entries.
#[trace("TC-213", "FR-081-AC-1")]
#[test]
fn admitted_original_keys_survive_normalization() {
    let document = k2();
    let bytes = serde_json::to_vec(&document).expect("K2 serializes");
    let offered = package_input([bytes.as_slice()]);
    let digest = *offered.keys().next().expect("one offered package");
    let selection = DomainPackageRef {
        identity: PACKAGE.to_owned(),
        version: "1".to_owned(),
        digest,
    };
    let (_, admitted) = admit(
        &selection,
        SHA256_JCS_DIGEST_DOMAIN,
        &offered,
        IntakeLimits::default(),
    )
    .expect("S1 admits K2");
    let admitted_records = read_records(PACKAGE, &admitted).expect("reader admits K2 records");
    let admitted_keys: BTreeSet<_> = admitted_records
        .iter()
        .map(|record| record.key().clone())
        .collect();
    let package = read_domain_package(selection, &admitted).expect("reader retains K2 origins");
    let view = match normalize_with_meter(&package, ModelNormalizationLimits::UNLIMITED).0 {
        NormalizeOutcome::Completed(view) => view,
        other => panic!("K2 normalization must complete: {other:?}"),
    };
    let retained_keys: BTreeSet<_> = view
        .domain_package()
        .records
        .iter()
        .map(|record| record.key().clone())
        .collect();
    assert_eq!(retained_keys, admitted_keys);
    for key in &admitted_keys {
        assert!(
            view.domain_package().original(key).is_some(),
            "missing origin for {key:?}"
        );
    }
    let effective_origins: BTreeSet<_> = view
        .declarations()
        .iter()
        .map(|entry| entry.preimage.original.clone())
        .collect();
    assert!(effective_origins.is_subset(&admitted_keys));
    assert!(effective_origins.contains(&DeclarationKey {
        package: PACKAGE.to_owned(),
        node: id("OrderLifecycle/place")
    }));
    assert!(!effective_origins.contains(&DeclarationKey {
        package: PACKAGE.to_owned(),
        node: id("OrderManagement")
    }));
    assert!(!effective_origins.contains(&DeclarationKey {
        package: PACKAGE.to_owned(),
        node: id("Instant")
    }));
}

/// The original inventory includes a scalar even though phase 2 gives it no
/// effective entry or derivation charge.
#[trace("FR-081-AC-8")]
#[test]
fn scalar_original_has_no_effective_entry_or_extra_charge() {
    let mut scalar = declaration("Instant", "value", json!({}));
    scalar["target"] = json!("ix://quire/native/Timestamp");
    let document = domain_document(
        vec![
            declaration(
                "Order",
                "thing",
                json!({
                    "identityFields": [id("Order/order_id")],
                    "fields": [field("Order", "order_id", "ix://quire/native/UUID")],
                    "operations": [operation("Order", "submit")],
                }),
            ),
            declaration(
                "Draft",
                "thing",
                json!({
                    "identityFields": [id("Order/order_id")],
                    "supertypes": [id("Order")],
                }),
            ),
            scalar,
        ],
        vec![
            construct("thing", meaning::OBJECT_TYPE),
            construct("value", meaning::VALUE_TYPE),
        ],
    );
    let package = read_k2(&document).expect("two objects, a field, operation, and scalar admit");
    let admitted_keys: BTreeSet<_> = package
        .records
        .iter()
        .map(|record| record.key().clone())
        .collect();
    let expected: BTreeSet<_> = [
        "Order",
        "Draft",
        "Order/order_id",
        "Order/submit",
        "Instant",
    ]
    .into_iter()
    .map(|node| DeclarationKey {
        package: PACKAGE.to_owned(),
        node: id(node),
    })
    .collect();
    assert_eq!(admitted_keys, expected);
    for key in &expected {
        assert_eq!(
            package.original(key).map(|original| &original.origin),
            Some(&origin(
                key.node
                    .strip_prefix(&format!("ix://{PACKAGE}/"))
                    .expect("fixture key prefix")
            ))
        );
    }
    let (outcome, meter) = normalize_with_meter(&package, ModelNormalizationLimits::UNLIMITED);
    let NormalizeOutcome::Completed(view) = outcome else {
        panic!("phase-2 fixture must normalize: {outcome:?}");
    };
    let effective: BTreeSet<_> = view
        .declarations()
        .iter()
        .map(|entry| entry.preimage.original.clone())
        .collect();
    assert_eq!(
        effective,
        expected
            .into_iter()
            .filter(|key| key.node != id("Instant"))
            .collect()
    );
    let without_scalar = domain_document(
        document["types"]
            .as_array()
            .expect("types array")
            .iter()
            .filter(|value| value["identity"] != id("Instant"))
            .cloned()
            .collect(),
        vec![construct("thing", meaning::OBJECT_TYPE)],
    );
    let control = read_k2(&without_scalar).expect("control admits");
    let (control_outcome, control_meter) =
        normalize_with_meter(&control, ModelNormalizationLimits::UNLIMITED);
    assert!(matches!(control_outcome, NormalizeOutcome::Completed(_)));
    for charge in [
        ChargePoint::NormalizeFact,
        ChargePoint::NormalizeDeclaration,
        ChargePoint::NormalizeHash,
    ] {
        let count = |meter: &qsl_semantics::model::accounting::Meter| {
            meter
                .admitted_charges()
                .iter()
                .filter(|point| **point == charge)
                .count()
        };
        assert_eq!(
            count(&meter),
            count(&control_meter),
            "scalar must not add {charge:?}"
        );
    }
}

fn read_refusals(document: &Value) -> Vec<ModelRefusal> {
    let bytes = serde_json::to_vec(document).expect("fixture serializes");
    let offered = package_input([bytes.as_slice()]);
    let digest = *offered.keys().next().expect("one offered package");
    let selection = DomainPackageRef {
        identity: PACKAGE.to_owned(),
        version: "1".to_owned(),
        digest,
    };
    let (_selection, admitted) = admit(
        &selection,
        SHA256_JCS_DIGEST_DOMAIN,
        &offered,
        IntakeLimits::default(),
    )
    .expect("S1 admits the authored package bytes");
    read_records(PACKAGE, &admitted).expect_err("mutation must refuse")
}

fn assert_normalized_malformed_refusal(document: &Value, declaration: &str) {
    let refusals = normalized_refusals(document, 21);
    assert_eq!(
        refusals.len(),
        1,
        "one normalized M07 refusal: {refusals:?}"
    );
    assert_eq!(refusals[0].code, Code::InvalidModelBinding);
    match &refusals[0].cause {
        ModelRefusalCause::IntakeMalformedDeclaration { node, .. } => {
            assert!(node.contains(declaration), "{refusals:?}");
        }
        ModelRefusalCause::MalformedDeclaration => {
            assert!(refusals[0].detail.contains(declaration), "{refusals:?}");
        }
        other => panic!("expected a malformed declaration, got {other:?}"),
    }
}

/// One typed `invalid_model_binding`/`malformed-declaration` naming `node`.
/// The member path has no typed field on this cause (queued: "Typed member-path
/// field on intake refusal causes"), so only node, code and cause are asserted.
fn assert_malformed_refusal(document: &Value, node: &str, charges: usize) {
    let refusals = normalized_refusals(document, charges);
    assert_eq!(refusals.len(), 1, "one M08/M09 refusal: {refusals:?}");
    assert_eq!(refusals[0].code, Code::InvalidModelBinding);
    assert_eq!(refusals[0].cause.as_str(), "malformed-declaration");
    let ModelRefusalCause::IntakeMalformedDeclaration { node: actual, .. } = &refusals[0].cause
    else {
        panic!("expected intake malformed-declaration, got {refusals:?}");
    };
    assert_eq!(actual, &id(node));
}

/// One typed `unsupported_construct`/`declaration-form` naming `node`.
fn assert_unsupported_refusal(document: &Value, node: &str, charges: usize) {
    let refusals = normalized_refusals(document, charges);
    assert_eq!(refusals.len(), 1, "one M08/M09 refusal: {refusals:?}");
    assert_eq!(refusals[0].code, Code::UnsupportedConstruct);
    assert_eq!(refusals[0].cause.as_str(), "declaration-form");
    let ModelRefusalCause::UnsupportedDeclarationForm { node: actual, .. } = &refusals[0].cause
    else {
        panic!("expected declaration-form, got {refusals:?}");
    };
    assert_eq!(actual, &id(node));
}

/// One typed `missing_declaration`/`missing-name` on `node` whose reader
/// pointer is `path` and whose absent declaration is `reference`.
fn assert_missing_refusal(
    document: &Value,
    node: &str,
    path: &str,
    reference: &str,
    charges: usize,
) {
    let refusals = normalized_refusals(document, charges);
    assert_eq!(refusals.len(), 1, "one M08/M09 refusal: {refusals:?}");
    assert_eq!(refusals[0].code, Code::MissingDeclaration);
    assert_eq!(refusals[0].cause.as_str(), "missing-name");
    let ModelRefusalCause::IntakeMissingDeclaration {
        node: actual,
        reference: absent,
        path: actual_path,
        ..
    } = &refusals[0].cause
    else {
        panic!("expected intake missing-name, got {refusals:?}");
    };
    assert_eq!(actual, &id(node));
    assert_eq!(absent, &id(reference));
    assert_eq!(actual_path.as_str(), path);
}

fn assert_process_duplicate_step_refusal(document: &Value) {
    let refusals = normalized_refusals(document, 21);
    assert_eq!(
        refusals.len(),
        1,
        "one process duplicate refusal: {refusals:?}"
    );
    assert_eq!(refusals[0].code, Code::InvalidModelBinding);
    let ModelRefusalCause::ConflictingBinding { key } = &refusals[0].cause else {
        panic!(
            "expected conflicting process step, got {:?}",
            refusals[0].cause
        );
    };
    assert_eq!(key.node, id("Fulfilment/reserve_stock"));
    assert!(refusals[0].detail.contains("steps[3]"), "{refusals:?}");
    assert!(
        refusals[0].detail.contains("/types/5/steps/3/identity"),
        "duplicate step pointer missing: {refusals:?}"
    );
}

fn assert_process_non_event_refusal(document: &Value) {
    let refusals = normalized_refusals(document, 21);
    assert_eq!(
        refusals.len(),
        1,
        "one process non-event refusal: {refusals:?}"
    );
    assert_eq!(refusals[0].code, Code::InvalidModelBinding);
    let ModelRefusalCause::IntakeMalformedDeclaration { node, .. } = &refusals[0].cause else {
        panic!(
            "expected malformed process event binding, got {:?}",
            refusals[0].cause
        );
    };
    assert_eq!(node, &id("Fulfilment/reserve_stock"));
    assert!(
        refusals[0].detail.contains("steps[1].emits[0]"),
        "reader locus missing from refusal: {refusals:?}"
    );
}

fn assert_process_missing_event_refusal(document: &Value, reference: &str) {
    let refusals = normalized_refusals(document, 21);
    assert_eq!(refusals.len(), 1, "one process event refusal: {refusals:?}");
    assert_eq!(refusals[0].code, Code::MissingDeclaration);
    let ModelRefusalCause::IntakeMissingDeclaration {
        node,
        reference: actual,
        path,
        ..
    } = &refusals[0].cause
    else {
        panic!(
            "expected intake missing declaration, got {:?}",
            refusals[0].cause
        );
    };
    assert_eq!(node, &id("Fulfilment"));
    assert_eq!(actual, &id(reference));
    assert_eq!(path.as_str(), "/types/5/steps/1/emits/0");
    assert!(
        refusals[0].detail.contains("steps[1].emits[0]"),
        "{refusals:?}"
    );
}

fn assert_record_charge_count(meter: &Meter, expected: usize) {
    assert_eq!(
        meter
            .admitted_charges()
            .iter()
            .filter(|charge| **charge == ChargePoint::NormalizeRecord)
            .count(),
        expected,
        "TC-235 normalize.record charge count",
    );
}

fn record_charges(
    document: &DomainPackage,
    expected: usize,
) -> qsl_semantics::model::normalize::EffectiveView {
    let (outcome, meter) = normalize_with_meter(document, ModelNormalizationLimits::default());
    let NormalizeOutcome::Completed(view) = outcome else {
        panic!("normalization must complete, got {outcome:?}");
    };
    assert_record_charge_count(&meter, expected);
    view
}

/// M01: literal TC-235 K2 must admit as 21 declarations and normalize with
/// exactly 21 `normalize.record` charges. Instant uses FCD's native Timestamp
/// alias form, Money uses the canonical native Integer name, and the state
/// transition guard names the declared clauseId.
#[test]
fn tc235_m01_literal_k2_is_admitted_as_declared() {
    let document = k2();
    let bytes = serde_json::to_vec(&document).expect("K2 serializes");
    let offered = package_input([bytes.as_slice()]);
    let digest = *offered.keys().next().expect("one offered package");
    let selection = DomainPackageRef {
        identity: PACKAGE.to_owned(),
        version: "1".to_owned(),
        digest,
    };
    let (selection, admitted) = admit(
        &selection,
        SHA256_JCS_DIGEST_DOMAIN,
        &offered,
        IntakeLimits::default(),
    )
    .expect("S1 admits the authored package bytes");
    let package = read_domain_package(selection, &admitted)
        .expect("TC-235 K2 must reach the semantic model reader");
    assert_eq!(package.records.len(), 21, "K2's declared source count");
    let mut keys: Vec<DeclarationKey> = package
        .records
        .iter()
        .map(|record| record.key().clone())
        .collect();
    keys.sort();
    let mut expected: Vec<DeclarationKey> = [
        "Order",
        "Order/order_id",
        "Order/total",
        "Instant",
        "Money",
        "Money/amount_minor",
        "Money/currency",
        "OrderPlaced",
        "OrderPlaced/occurred_at",
        "OrderPlaced/order_id",
        "OrderLifecycle",
        "OrderLifecycle/placed",
        "OrderLifecycle/place",
        "OrderLifecycle/cancel",
        "OrderLifecycle/NotYetPlaced",
        "Fulfilment",
        "Fulfilment/correlation_id",
        "OrderStore",
        "OrderStore/get",
        "OrderStore/save",
        "OrderManagement",
    ]
    .iter()
    .map(|node| DeclarationKey {
        package: PACKAGE.to_owned(),
        node: id(node),
    })
    .collect();
    expected.sort();
    assert_eq!(keys, expected, "K2's 21 declared keys");
    assert!(
        package.records.iter().any(|record| matches!(record,
            DomainPackageRecord::RecordValueType(record) if record.key.node == id("Money"))),
        "Money is a record value type"
    );
    let (outcome, meter) = normalize_with_meter(&package, ModelNormalizationLimits::default());
    let NormalizeOutcome::Completed(view) = outcome else {
        panic!("M01 K2 normalization must admit, got {outcome:?}");
    };
    assert_eq!(
        meter
            .admitted_charges()
            .iter()
            .filter(|charge| **charge == ChargePoint::NormalizeRecord)
            .count(),
        21,
        "M01 charges normalize.record once per original declaration",
    );
    assert!(view
        .type_identities()
        .contains_key(&qsl_semantics::model::key::DeclarationKey {
            package: PACKAGE.to_owned(),
            node: id("Money"),
        }));
    let process = qsl_semantics::model::key::DeclarationKey {
        package: PACKAGE.to_owned(),
        node: id("Fulfilment"),
    };
    assert!(
        view.type_identities().contains_key(&process),
        "a process has an effective receiver type identity"
    );
    assert!(
        view.object_universe_of(&process).is_some(),
        "process runs retain their object universe"
    );
    let persistence = qsl_semantics::model::key::DeclarationKey {
        package: PACKAGE.to_owned(),
        node: id("OrderStore"),
    };
    assert!(
        view.type_identities().contains_key(&persistence),
        "a persistence interface has an effective operation-owner identity"
    );
    assert!(
        view.object_universe_of(&persistence).is_none(),
        "a persistence interface does not gain an object universe"
    );
    assert!(!view
        .type_identities()
        .contains_key(&qsl_semantics::model::key::DeclarationKey {
            package: PACKAGE.to_owned(),
            node: id("OrderManagement"),
        }));
}

/// M03: record-value meanings reject identity and operations, while retaining
/// both native `quire` invariants and foreign-language clauses as originals.
#[test]
fn tc235_m03_record_shape_and_clause_meanings() {
    let identity_document = mutate_k2(|document| {
        type_named_mut(document, "Money")["identityFields"] = json!([id("Money/currency")]);
    });
    let package = read_k2(&identity_document).expect("reader retains the identity field");
    let (outcome, meter) = normalize_with_meter(&package, ModelNormalizationLimits::default());
    assert_record_charge_count(&meter, 21);
    let NormalizeOutcome::Refused(refusals) = outcome else {
        panic!("M03 run 1 must refuse during normalization, got {outcome:?}");
    };
    assert_eq!(refusals.len(), 1);
    assert_eq!(refusals[0].code, Code::InvalidModelBinding);
    let ModelRefusalCause::IntakeMalformedDeclaration { node, .. } = &refusals[0].cause else {
        panic!("M03 run 1 cause differs: {:?}", refusals[0].cause);
    };
    assert_eq!(node, &id("Money"));
    assert!(refusals[0].detail.contains("identityFields"));

    let operation_document = mutate_k2(|document| {
        type_named_mut(document, "Money")["operations"] = json!([operation("Money", "add")]);
    });
    let package = read_k2(&operation_document).expect("reader retains the operation");
    let (outcome, meter) = normalize_with_meter(&package, ModelNormalizationLimits::default());
    assert_record_charge_count(&meter, 22);
    let NormalizeOutcome::Refused(refusals) = outcome else {
        panic!("M03 run 2 must refuse during normalization, got {outcome:?}");
    };
    assert_eq!(refusals.len(), 1);
    assert_eq!(refusals[0].code, Code::UnsupportedConstruct);
    let ModelRefusalCause::UnsupportedDeclarationForm { node, what } = &refusals[0].cause else {
        panic!(
            "M03 run 2 must be declaration-form, got {:?}",
            refusals[0].cause
        );
    };
    assert_eq!(node, &id("Money/add"));
    assert_eq!(what, &format!("{}:operations", meaning::RECORD_VALUE_TYPE));
    assert!(refusals[0].detail.contains("operations[0]"));

    let invariant_document = mutate_k2(|document| {
        type_named_mut(document, "Money")["clauses"] =
            json!([clause("Money", "NonNegative", "self.amount_minor >= 0")]);
    });
    let package = read_k2(&invariant_document).expect("quire clause is admitted");
    let invariant_key = qsl_semantics::model::key::DeclarationKey {
        package: PACKAGE.to_owned(),
        node: id("Money/NonNegative"),
    };
    assert!(package.original(&invariant_key).is_some());
    let view = record_charges(&package, 22);
    let owner = view.type_identities()[&qsl_semantics::model::key::DeclarationKey {
        package: PACKAGE.to_owned(),
        node: id("Money"),
    }];
    assert!(view.declarations().iter().any(|declaration| {
        declaration.visible
            && declaration.preimage.original == invariant_key
            && declaration.preimage.owner_effective_type == Some(owner)
    }));

    let foreign_document = mutate_k2(|document| {
        let mut foreign = clause("Money", "NonNegative", "self.amount_minor >= 0");
        foreign["language"] = json!("ocl");
        type_named_mut(document, "Money")["clauses"] = json!([foreign]);
    });
    let package = read_k2(&foreign_document).expect("foreign-language clause is retained");
    let foreign_key = qsl_semantics::model::key::DeclarationKey {
        package: PACKAGE.to_owned(),
        node: id("Money/NonNegative"),
    };
    let DomainPackageRecord::Clause(foreign) = package
        .records
        .iter()
        .find(|record| record.key() == &foreign_key)
        .expect("foreign clause remains a typed original")
    else {
        panic!("M03 run 4 must retain a clause record");
    };
    assert_eq!(foreign.language, "ocl");
    assert_eq!(foreign.text, "self.amount_minor >= 0");
    let view = record_charges(&package, 22);
    assert!(!view
        .declarations()
        .iter()
        .any(|declaration| declaration.preimage.original == foreign_key));
}

/// Supporting control for the later full M17 vector: a record subtype keeps
/// its inherited field and invariant with its local field under one effective
/// type, and its parent is not silently substituted for the subtype identity.
#[test]
fn record_value_subtype_preserves_effective_fields_support() {
    let types = vec![
        declaration(
            "Order",
            "thing",
            json!({
                "identityFields": [id("Order/order_id")],
                "fields": [field("Order", "order_id", "ix://quire/native/UUID"),
                           field("Order", "total", &id("Money"))],
            }),
        ),
        declaration(
            "Money",
            "money",
            json!({
                "fields": [field("Money", "amount_minor", "ix://quire/native/Integer"),
                           field("Money", "currency", "ix://quire/native/String")],
                "clauses": [clause("Money", "NonNegative", "self.amount_minor >= 0")],
            }),
        ),
        declaration(
            "TaxedMoney",
            "money",
            json!({
                "supertypes": [id("Money")],
                "fields": [field("TaxedMoney", "tax_minor", "ix://quire/native/Integer")],
            }),
        ),
    ];
    let document = domain_document(
        types,
        vec![
            construct("thing", meaning::OBJECT_TYPE),
            construct("money", meaning::RECORD_VALUE_TYPE),
        ],
    );
    let package = read_k2(&document)
        .expect("reader admits the record-value slice without event declarations");
    let view = record_charges(&package, 9);
    let taxed_key = qsl_semantics::model::key::DeclarationKey {
        package: PACKAGE.to_owned(),
        node: id("TaxedMoney"),
    };
    let taxed_type = view.type_identities()[&taxed_key];
    let mut effective_fields: Vec<_> = view
        .declarations()
        .iter()
        .filter(|entry| entry.visible && entry.preimage.owner_effective_type == Some(taxed_type))
        .map(|entry| entry.preimage.original.node.clone())
        .collect();
    // FR-150 sorts effective declarations by identity; these same-package
    // node identities therefore use lexical order here.
    effective_fields.sort();
    assert_eq!(
        effective_fields,
        [
            id("Money/NonNegative"),
            id("Money/amount_minor"),
            id("Money/currency"),
            id("TaxedMoney/tax_minor")
        ]
    );
    assert!(view.object_universe_of(&taxed_key).is_none());
}

fn k2_record_event_subtype_slice(strip_money_string: bool) -> Value {
    mutate_k2(|document| {
        if strip_money_string {
            // QSL-781 owns native String admission. Keep this dependency out
            // of the record/event assembly control while preserving the
            // authored K2 subtype and inherited-field behavior.
            type_named_mut(document, "Money")["fields"] =
                json!([field("Money", "amount_minor", "ix://quire/native/Integer")]);
        }
        document["types"].as_array_mut().unwrap().retain(|ty| {
            ty["identity"] != id("OrderManagement") && ty["identity"] != id("OrderLifecycle")
        });
        document["constructs"]
            .as_array_mut()
            .unwrap()
            .retain(|construct| {
                construct["construct"]["meaning"] != meaning::NAMESPACE
                    && construct["construct"]["meaning"] != meaning::STATE_MACHINE
            });
        document["types"].as_array_mut().unwrap().extend([
            declaration(
                "TaxedMoney",
                "money",
                json!({
                    "supertypes": [id("Money")],
                    "fields": [field("TaxedMoney", "tax_minor", "ix://quire/native/Integer")],
                }),
            ),
            declaration(
                "ExpressOrderPlaced",
                "happened",
                json!({
                    "supertypes": [id("OrderPlaced")],
                    "occurrenceField": id("OrderPlaced/occurred_at"),
                    "fields": [field(
                        "ExpressOrderPlaced",
                        "promised_by",
                        &id("Instant"),
                    )],
                }),
            ),
        ]);
    })
}

#[trace("FR-056-AC-18", "FR-056-AC-21", "TC-235")]
#[test]
fn tc235_taxed_money_assembly_resolves_composite_name_and_inherited_fields() {
    let document = k2_record_event_subtype_slice(true);

    let package = read_k2(&document).expect("K2 record/event subtype slice reads");
    let view = record_charges(&package, package.records.len());
    for (child, parent) in [
        ("TaxedMoney", "Money"),
        ("ExpressOrderPlaced", "OrderPlaced"),
    ] {
        let key = qsl_semantics::model::key::DeclarationKey {
            package: PACKAGE.to_owned(),
            node: id(child),
        };
        let parent_key = qsl_semantics::model::key::DeclarationKey {
            package: PACKAGE.to_owned(),
            node: id(parent),
        };
        let record = package
            .records
            .iter()
            .find(|record| record.key() == &key)
            .expect("the subtype is retained as an original declaration");
        let supertypes = match record {
            DomainPackageRecord::RecordValueType(record) => &record.supertypes,
            DomainPackageRecord::EventType(record) => &record.supertypes,
            other => panic!("expected a record/event subtype, got {other:?}"),
        };
        assert_eq!(supertypes, &[parent_key]);
        let child_id = view.type_identities()[&key];
        let effective_fields: Vec<_> = view
            .declarations()
            .iter()
            .filter(|entry| entry.visible && entry.preimage.owner_effective_type == Some(child_id))
            .map(|entry| entry.preimage.original.node.as_str())
            .collect();
        assert!(
            effective_fields
                .iter()
                .any(|field| field.starts_with(&id(parent))),
            "{child} retains the parent's effective members: {effective_fields:?}"
        );
    }

    let assembled = assemble_k2_unit(
        &document,
        "function taxedTotal using v(money: M::TaxedMoney): Integer pure { 0 }",
    );
    let composite_fields = |name: &str| {
        let declaration = assembled
            .types
            .composites()
            .find(|declaration| declaration.name() == name)
            .unwrap_or_else(|| panic!("{name} is assembled"));
        let quire_semantic_value::declaration::CompositeShape::Record(fields) = declaration.shape()
        else {
            panic!("{name} is a record composite");
        };
        let mut names = fields
            .iter()
            .map(|field| field.name().to_owned())
            .collect::<Vec<_>>();
        names.sort();
        names
    };
    assert_eq!(
        composite_fields("M::TaxedMoney"),
        ["amount_minor", "tax_minor"]
    );
    assert_eq!(
        assembled
            .functions
            .iter()
            .map(|function| function.name.as_str())
            .collect::<Vec<_>>(),
        ["taxedTotal"]
    );
}

#[trace("FR-056-AC-18", "TC-235")]
#[test]
#[ignore = "requires PR #781 to map K2 Money.currency's native String field"]
fn tc235_k2_money_and_order_placed_composites_require_qsl_781() {
    let document = k2();
    let assembled = assemble_k2_unit(
        &document,
        "function moneyValue using v(money: M::Money): Integer pure { 0 }\n\
         function eventValue using v(placed: M::OrderPlaced): Boolean pure { true }",
    );
    let money = assembled
        .types
        .composites()
        .find(|declaration| declaration.name() == "M::Money")
        .expect("K2 Money is assembled with its source name");
    let quire_semantic_value::declaration::CompositeShape::Record(fields) = money.shape() else {
        panic!("K2 Money is a record composite");
    };
    assert!(fields.iter().any(|field| field.name() == "currency"));
    assert!(assembled
        .types
        .composites()
        .any(|declaration| declaration.name() == "M::OrderPlaced"));
}

#[trace("FR-056-AC-18", "TC-235")]
#[test]
#[ignore = "requires QSL-732 composite ancestor conformance in the pinned kernel"]
fn tc235_record_subtype_value_conforms_to_parent_typed_parameter_requires_qsl_732() {
    let document = k2_record_event_subtype_slice(true);
    let declarations = assemble_k2_unit(
        &document,
        "function acceptsMoney using v(money: M::Money): Integer pure { 0 }\n\
         function taxedTotal using v(money: M::TaxedMoney): Integer pure { acceptsMoney(money) }",
    );
    assert!(declarations
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .is_ok());
}

/// M07: processes retain their run identity and ordered event steps, while
/// ordinary operations and invariants use the object-type member readers.
#[trace("QSpec-TC-235")]
#[test]
fn tc235_m07_process_binder() {
    let package = read_k2(&k2()).expect("process admission");
    let _view = record_charges(&package, 21);
    let process = package
        .records
        .iter()
        .find_map(|record| match record {
            DomainPackageRecord::Process(process) if process.key.node == id("Fulfilment") => {
                Some(process)
            }
            _ => None,
        })
        .expect("Fulfilment process record");
    assert_eq!(
        process.identity_fields,
        vec![DeclarationKey {
            package: PACKAGE.to_owned(),
            node: id("Fulfilment/correlation_id"),
        }]
    );
    assert_eq!(
        process
            .steps
            .iter()
            .map(|step| step.name.as_str())
            .collect::<Vec<_>>(),
        vec!["receive_order", "reserve_stock", "await_shipment"]
    );
    assert_eq!(
        process
            .steps
            .iter()
            .map(|step| step.step_kind)
            .collect::<Vec<_>>(),
        vec![
            ProcessStepKind::Event,
            ProcessStepKind::Command,
            ProcessStepKind::Wait
        ]
    );
    assert_eq!(
        process.steps[0].consumes,
        vec![DeclarationKey {
            package: PACKAGE.to_owned(),
            node: id("OrderPlaced"),
        }]
    );
    assert!(process.steps[0].emits.is_empty());
    assert!(process.steps[1].consumes.is_empty());
    assert!(process.steps[1].emits.is_empty());
    assert!(process.steps[2].consumes.is_empty());
    assert!(process.steps[2].emits.is_empty());

    let mut document = k2();
    type_named_mut(&mut document, "Fulfilment")["identityFields"] = json!([]);
    assert_normalized_malformed_refusal(&document, "Fulfilment");

    let mut document = k2();
    type_named_mut(&mut document, "Fulfilment")["steps"] = json!([]);
    assert_normalized_malformed_refusal(&document, "Fulfilment");

    let mut document = k2();
    type_named_mut(&mut document, "Fulfilment")["steps"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "identity": id("Fulfilment/reserve_stock"),
            "name": "reserve_stock",
            "stepKind": "command",
            "consumes": [],
            "emits": [],
            "origin": origin("Fulfilment/reserve_stock"),
        }));
    assert_process_duplicate_step_refusal(&document);

    let mut document = k2();
    type_named_mut(&mut document, "Fulfilment")["steps"][0]["consumes"] = json!([id("Money")]);
    assert_normalized_malformed_refusal(&document, "Fulfilment");

    let mut document = k2();
    type_named_mut(&mut document, "Fulfilment")["steps"][1]["emits"] = json!([id("Order")]);
    assert_process_non_event_refusal(&document);

    let mut document = k2();
    type_named_mut(&mut document, "Fulfilment")["steps"][1]["emits"] = json!([id("StockReserved")]);
    assert_process_missing_event_refusal(&document, "StockReserved");

    let mut document = k2();
    type_named_mut(&mut document, "Fulfilment")["operations"]
        .as_array_mut()
        .unwrap()
        .push(operation("Fulfilment", "restart"));
    let package = read_k2(&document).expect("process operation admission");
    let _view = record_charges(&package, 22);
    assert!(package.records.iter().any(|record| {
        matches!(record, DomainPackageRecord::OperationMember(operation)
            if operation.key.node == id("Fulfilment/restart"))
    }));

    let mut document = k2();
    type_named_mut(&mut document, "Fulfilment")["clauses"] = json!([clause(
        "Fulfilment",
        "Bounded",
        "self.correlation_id = self.correlation_id"
    )]);
    let package = read_k2(&document).expect("process invariant admission");
    let _view = record_charges(&package, 22);
    assert!(package.records.iter().any(|record| {
        matches!(record, DomainPackageRecord::Clause(clause)
            if clause.key.node == id("Fulfilment/Bounded"))
    }));
}

/// M08: persistence interfaces bind their object targets and operation
/// signatures, while storage contracts and clauses remain outside this
/// structural binder.
#[test]
fn tc235_m08_persistence_interface_binder() {
    let package = read_k2(&k2()).expect("M08 positive K2 admission");
    let view = record_charges(&package, 21);
    assert!(view
        .type_identities()
        .contains_key(&qsl_semantics::model::key::DeclarationKey {
            package: PACKAGE.to_owned(),
            node: id("Order")
        }));
    // M01: `persists: [Order]`.
    assert!(package.records.iter().any(|record| matches!(
        record,
        DomainPackageRecord::PersistenceInterface(interface)
            if interface.key.node == id("OrderStore")
                && interface.persists.iter().map(|key| key.node.clone()).eq([id("Order")])
    )));
    // FR-322-AC-25: a persistence interface is a declaring `model` node for
    // its operations, so it has an effective identity; it has no object
    // universe (FR-208: only a process does).
    let store = qsl_semantics::model::key::DeclarationKey {
        package: PACKAGE.to_owned(),
        node: id("OrderStore"),
    };
    assert!(view.type_identities().contains_key(&store));
    assert!(view.object_universe_of(&store).is_none());

    let mut document = k2();
    type_named_mut(&mut document, "OrderStore")["fields"] =
        json!([field("OrderStore", "cache", &id("Order"))]);
    assert_malformed_refusal(&document, "OrderStore/cache", 22);

    let mut document = k2();
    type_named_mut(&mut document, "OrderStore")["operations"] = json!([]);
    assert_malformed_refusal(&document, "OrderStore", 19);

    let mut document = k2();
    type_named_mut(&mut document, "OrderStore")["persists"] = json!([]);
    assert_malformed_refusal(&document, "OrderStore", 21);

    let mut document = k2();
    type_named_mut(&mut document, "OrderStore")["persists"] = json!([id("Money")]);
    assert_malformed_refusal(&document, "OrderStore", 21);

    let mut document = k2();
    type_named_mut(&mut document, "OrderStore")["persists"] = json!([id("Invoice")]);
    assert_missing_refusal(
        &document,
        "OrderStore",
        "/types/6/persists/0",
        "Invoice",
        21,
    );

    let mut document = k2();
    type_named_mut(&mut document, "OrderStore")["operations"][1]["pre"] = json!([{
        "language": "quire",
        "text": "true",
        "origin": origin("OrderStore/save"),
    }]);
    assert_unsupported_refusal(&document, "OrderStore/save", 21);

    let mut document = k2();
    type_named_mut(&mut document, "OrderStore")["clauses"] =
        json!([clause("OrderStore", "Healthy", "true")]);
    assert_unsupported_refusal(&document, "OrderStore/Healthy", 22);
}

/// M09: namespaces preserve their ordered payload, never become effective
/// types, and reject field/operation members, namespace members, duplicate
/// membership and Quire clauses with the typed intake result.
#[test]
fn tc235_m09_namespace_binder() {
    let mut document = k2();
    let package = read_k2(&document).expect("M09 K2 baseline admission");
    let namespace = package
        .original(&qsl_semantics::model::key::DeclarationKey {
            package: PACKAGE.to_owned(),
            node: id("OrderManagement"),
        })
        .expect("OrderManagement original");
    assert_eq!(namespace.meaning.as_deref(), Some(meaning::NAMESPACE));
    assert_eq!(namespace.origin, origin("OrderManagement"));
    // M01: members `[Order, Money, OrderPlaced]`, vocabulary `[Place]`.
    let key = |name: &str| qsl_semantics::model::key::DeclarationKey {
        package: PACKAGE.to_owned(),
        node: id(name),
    };
    let record = package
        .records
        .iter()
        .find_map(|record| match record {
            DomainPackageRecord::Namespace(namespace)
                if namespace.key == key("OrderManagement") =>
            {
                Some(namespace)
            }
            _ => None,
        })
        .expect("OrderManagement retains its namespace record");
    assert_eq!(
        record.members,
        [key("Order"), key("Money"), key("OrderPlaced")]
    );
    assert_eq!(record.vocabulary.len(), 1);
    assert_eq!(record.vocabulary[0].term, "Place");
    let view = record_charges(&package, 21);
    assert!(!view
        .type_identities()
        .contains_key(&qsl_semantics::model::key::DeclarationKey {
            package: PACKAGE.to_owned(),
            node: id("OrderManagement")
        }));

    type_named_mut(&mut document, "OrderManagement")["fields"] =
        json!([field("OrderManagement", "name", "ix://quire/native/String")]);
    assert_malformed_refusal(&document, "OrderManagement/name", 22);

    let mut document = k2();
    type_named_mut(&mut document, "OrderManagement")["operations"] =
        json!([operation("OrderManagement", "close")]);
    assert_malformed_refusal(&document, "OrderManagement/close", 22);

    let mut document = k2();
    document["types"].as_array_mut().unwrap().push(declaration(
        "Billing",
        "context",
        json!({
            "members": [id("Order")],
            "vocabulary": [{"term": "Bill", "doc": "Bill an order", "origin": origin("Billing")}],
        }),
    ));
    assert_malformed_refusal(&document, "OrderManagement", 22);

    let mut document = k2();
    document["types"].as_array_mut().unwrap().push(declaration(
        "Outer",
        "context",
        json!({
            "members": [id("OrderManagement")],
            "vocabulary": [{"term": "Scope", "doc": "An outer scope", "origin": origin("Outer")}],
        }),
    ));
    assert_malformed_refusal(&document, "Outer", 22);

    let mut document = k2();
    type_named_mut(&mut document, "OrderManagement")["members"] =
        json!([id("Order"), id("Money"), id("OrderPlaced"), id("Ghost")]);
    assert_missing_refusal(
        &document,
        "OrderManagement",
        "/types/7/members/3",
        "Ghost",
        21,
    );

    let mut document = k2();
    type_named_mut(&mut document, "OrderManagement")["clauses"] =
        json!([clause("OrderManagement", "Scoped", "true")]);
    assert_unsupported_refusal(&document, "OrderManagement/Scoped", 22);
}

/// A foreign-package or native identity in `persists` or `members` is not a
/// missing name of this package: the foreign/native guard still applies.
#[test]
fn foreign_identity_in_persists_and_members_is_not_a_missing_name() {
    for foreign in ["ix://other/pkg/Thing", "ix://quire/native/Integer"] {
        let mut persists = k2();
        type_named_mut(&mut persists, "OrderStore")["persists"] = json!([foreign]);
        let mut members = k2();
        type_named_mut(&mut members, "OrderManagement")["members"] = json!([foreign]);
        for document in [persists, members] {
            let refusals = read_refusals(&document);
            assert!(!refusals.is_empty());
            assert!(
                refusals
                    .iter()
                    .all(|refusal| refusal.code != Code::MissingDeclaration),
                "{foreign}: {refusals:?}"
            );
        }
    }
}

fn normalized_refusals(document: &Value, expected_charges: usize) -> Vec<ModelRefusal> {
    let package = read_k2(document).expect("intake retains the typed refusal for normalization");
    let (outcome, meter) = normalize_with_meter(&package, ModelNormalizationLimits::default());
    assert_record_charge_count(&meter, expected_charges);
    let NormalizeOutcome::Refused(refusals) = outcome else {
        panic!("the TC-235 mutation must be refused, got {outcome:?}");
    };
    refusals.iter().cloned().collect()
}

fn assert_malformed_at(refusal: &ModelRefusal, node: &str, path: &str) {
    assert_eq!(refusal.code, Code::InvalidModelBinding);
    assert_eq!(refusal.cause.as_str(), "malformed-declaration");
    let ModelRefusalCause::IntakeMalformedDeclaration { node: actual, .. } = &refusal.cause else {
        panic!(
            "expected the typed intake malformed-declaration, got {:?}",
            refusal.cause
        );
    };
    assert_eq!(actual, node);
    assert!(
        refusal.detail.contains(path),
        "{}: {}",
        refusal.detail,
        path
    );
}

/// M04: event-type identity, occurrence field, operation, and Timestamp rules.
#[test]
fn tc235_m04_event_type_refusal_vectors() {
    let identity_document = mutate_k2(|document| {
        type_named_mut(document, "OrderPlaced")["identityFields"] =
            json!([id("OrderPlaced/order_id")]);
    });
    let refusals = normalized_refusals(&identity_document, 21);
    assert_eq!(refusals.len(), 1);
    assert_malformed_at(
        &refusals[0],
        &id("OrderPlaced"),
        "TypeDefinition.identityFields",
    );

    let wrong_timestamp_document = mutate_k2(|document| {
        type_named_mut(document, "OrderPlaced")["occurrenceField"] =
            json!(id("OrderPlaced/order_id"));
    });
    let refusals = normalized_refusals(&wrong_timestamp_document, 21);
    assert_eq!(refusals.len(), 1);
    assert_malformed_at(
        &refusals[0],
        &id("OrderPlaced"),
        "TypeDefinition.occurrenceField",
    );

    let absent_occurrence_document = mutate_k2(|document| {
        type_named_mut(document, "OrderPlaced")["occurrenceField"] = json!(id("OrderPlaced/at"));
    });
    let refusals = normalized_refusals(&absent_occurrence_document, 21);
    assert_eq!(refusals.len(), 1);
    assert_eq!(refusals[0].code, Code::MissingDeclaration);
    assert_eq!(refusals[0].cause.as_str(), "missing-name");
    assert_eq!(
        refusals[0].cause,
        ModelRefusalCause::IntakeMissingDeclaration {
            node: id("OrderPlaced"),
            reference: id("OrderPlaced/at"),
            origin: origin("OrderPlaced"),
            path: "/types/3/occurrenceField".parse().unwrap(),
        }
    );

    let operation_document = mutate_k2(|document| {
        type_named_mut(document, "OrderPlaced")["operations"] =
            json!([operation("OrderPlaced", "replay")]);
    });
    let refusals = normalized_refusals(&operation_document, 22);
    assert_eq!(refusals.len(), 1);
    assert_malformed_at(
        &refusals[0],
        &id("OrderPlaced/replay"),
        "TypeDefinition.operations[0]",
    );

    // The negative binding changes the alias target from native Timestamp
    // to native Integer, retaining the value construct and occurrence.
    let non_timestamp_document = mutate_k2(|document| {
        type_named_mut(document, "Instant")["target"] = json!("ix://quire/native/Integer");
    });
    let refusals = normalized_refusals(&non_timestamp_document, 21);
    assert_eq!(refusals.len(), 1);
    assert_malformed_at(
        &refusals[0],
        &id("OrderPlaced"),
        "TypeDefinition.occurrenceField",
    );
}

/// M18: event subtypes preserve the inherited occurrence field and reject a
/// conflicting occurrence field or an ancestor of another business meaning.
#[test]
fn tc235_m18_event_type_specialization_vectors() {
    let positive_document = mutate_k2(|document| {
        document["types"].as_array_mut().unwrap().push(declaration(
            "ExpressOrderPlaced",
            "happened",
            json!({
                "supertypes": [id("OrderPlaced")],
                "occurrenceField": id("OrderPlaced/occurred_at"),
                "fields": [field(
                    "ExpressOrderPlaced",
                    "promised_by",
                    &id("Instant"),
                )],
            }),
        ));
    });
    let package = read_k2(&positive_document).expect("M18 event subtype reads through K2");
    let event_key = qsl_semantics::model::key::DeclarationKey {
        package: PACKAGE.to_owned(),
        node: id("ExpressOrderPlaced"),
    };
    let DomainPackageRecord::EventType(event) = package
        .records
        .iter()
        .find(|record| record.key() == &event_key)
        .expect("the event subtype has a typed source record")
    else {
        panic!("ExpressOrderPlaced must retain its event-type meaning");
    };
    assert_eq!(
        event.occurrence_field,
        Some(qsl_semantics::model::key::DeclarationKey {
            package: PACKAGE.to_owned(),
            node: id("OrderPlaced/occurred_at"),
        })
    );
    assert_eq!(
        event.supertypes,
        [qsl_semantics::model::key::DeclarationKey {
            package: PACKAGE.to_owned(),
            node: id("OrderPlaced"),
        }]
    );
    let view = record_charges(&package, 23);
    let parent_key = qsl_semantics::model::key::DeclarationKey {
        package: PACKAGE.to_owned(),
        node: id("OrderPlaced"),
    };
    let event_type = view.type_identities()[&event_key];
    let mut effective_members: Vec<_> = view
        .declarations()
        .iter()
        .filter(|entry| entry.visible && entry.preimage.owner_effective_type == Some(event_type))
        .map(|entry| entry.preimage.original.node.clone())
        .collect();
    effective_members.sort();
    assert_eq!(
        effective_members,
        [
            id("ExpressOrderPlaced/promised_by"),
            id("OrderPlaced/occurred_at"),
            id("OrderPlaced/order_id"),
        ]
    );
    assert!(view.type_identities().contains_key(&parent_key));
    assert!(view.object_universe_of(&event_key).is_none());

    let conflict_document = mutate_k2(|document| {
        document["types"].as_array_mut().unwrap().push(declaration(
            "ExpressOrderPlaced",
            "happened",
            json!({
                "supertypes": [id("OrderPlaced")],
                "occurrenceField": id("ExpressOrderPlaced/promised_by"),
                "fields": [field(
                    "ExpressOrderPlaced",
                    "promised_by",
                    &id("Instant"),
                )],
            }),
        ));
    });
    let refusals = normalized_refusals(&conflict_document, 23);
    assert_eq!(refusals.len(), 1);
    assert_eq!(refusals[0].code, Code::InvalidModelBinding);
    assert_eq!(refusals[0].cause.as_str(), "derivation-conflict");
    assert!(refusals[0]
        .detail
        .contains("ExpressOrderPlaced.occurrenceField"));
    assert!(refusals[0].detail.contains("OrderPlaced.occurrenceField"));

    let wrong_general_document = mutate_k2(|document| {
        document["types"].as_array_mut().unwrap().push(declaration(
            "ExpressOrderPlaced",
            "happened",
            json!({
                "supertypes": [id("Money")],
                "occurrenceField": id("OrderPlaced/occurred_at"),
                "fields": [field(
                    "ExpressOrderPlaced",
                    "promised_by",
                    &id("Instant"),
                )],
            }),
        ));
    });
    let refusals = normalized_refusals(&wrong_general_document, 23);
    assert_eq!(refusals.len(), 2);
    assert_malformed_at(
        &refusals[0],
        &id("ExpressOrderPlaced"),
        "TypeDefinition.occurrenceField",
    );
    assert_malformed_at(
        &refusals[1],
        &id("ExpressOrderPlaced"),
        "TypeDefinition.supertypes[0]",
    );
}

#[path = "tc235_state_machine.rs"]
mod tc235_state_machine;

#[path = "tc235_record_event.rs"]
mod tc235_record_event;
