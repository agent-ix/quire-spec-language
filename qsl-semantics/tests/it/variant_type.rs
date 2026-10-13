// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-056 "Variant types" (QSL-734): a variant type is admitted as one `enum`
//! export and one `variant` export per case (TC-994), refuses a payload
//! (TC-995), refuses malformed and duplicate cases with their causes
//! (TC-996), and refuses a clause or field that names it while its own
//! declaration stays admitted (TC-997).
//!
//! Every assertion compares typed code, cause variant and node identity.
//! The typed member path of these causes (`Variant.payloadType`,
//! `TypeDefinition.variants[1]`, `Field.typeRef`) arrives with QSL-736's
//! `MemberPath`; the reader names it in each refusal's detail until then.

use ix_trace_rs::trace;
use qsl_foundation::{ByteDigest, Code, IntakeLimits};
use qsl_semantics::check::AssemblyCause;
use qsl_semantics::model::accounting::{Meter, ModelNormalizationLimits};
use qsl_semantics::model::admitted::{AdmittedPackage, DeclarationKind};
use qsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, EnumRecord, VariantRecord,
};
use qsl_semantics::model::intake::{admit, meaning, package_input, read_domain_package};
use qsl_semantics::model::key::{hex, DeclarationKey, SHA256_JCS_DIGEST_DOMAIN};
use qsl_semantics::model::normalize::{
    normalize_with_meter, ModelRefusal, ModelRefusalCause, NormalizeOutcome,
};
use qsl_semantics::model::refusal::MemberPath;
use serde_json::{json, Value};

use qsl_semantics::model::intake::admit_unit;

use crate::model_operations::{
    admit_and_assemble_with_body, config_unit_with_body, config_version_document, empty_frame,
    operation, operation_parameter, parse_and_build, PACKAGE_IDENTITY,
};

const P: &str = PACKAGE_IDENTITY;
const MODULE: &str = "test/variant";

fn id(node: &str) -> String {
    format!("ix://{P}/{node}")
}

fn key(node: &str) -> DeclarationKey {
    DeclarationKey {
        package: P.to_owned(),
        node: id(node),
    }
}

fn generated(node: &str) -> Value {
    json!({"generated": {
        "generatorIdentity": id(node),
        "generatorVersion": "1.0.0",
        "inputIdentities": [id(node)],
    }})
}

fn sourced(line: u64) -> Value {
    json!({"source": {
        "sourceIdentity": id("spec"),
        "path": "spec.qspec",
        "startLine": line,
        "startColumn": 3,
    }})
}

fn construct(name: &str, meaning: &str, shape: &str, members: Value) -> Value {
    json!({
        "kind": {"module": MODULE, "name": name},
        "moduleVersion": "1.0.0",
        "manifestDigest": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        "construct": {
            "identity": "none", "shape": shape, "members": members, "meaning": meaning,
        },
    })
}

fn variant_construct() -> Value {
    construct(
        "variant_type",
        meaning::VARIANT_TYPE,
        "enumeration",
        json!({"variants": "required"}),
    )
}

fn definition(name: &str, kind: &str, members: Value) -> Value {
    let mut definition = json!({
        "identity": id(name), "displayName": name,
        "kind": {"module": MODULE, "name": kind},
        "roles": [], "origin": generated(name),
        "constraints": [], "extensions": [], "unknownPolicy": "reject",
    });
    definition
        .as_object_mut()
        .expect("definition is an object")
        .extend(members.as_object().expect("members are an object").clone());
    definition
}

/// A case with the identity the lift mints for it.
fn case(owner: &str, name: &str, line: u64) -> Value {
    json!({
        "identity": id(&format!("variant/{owner}-{name}")),
        "name": name,
        "origin": sourced(line),
    })
}

fn variant_type(name: &str, cases: Vec<Value>) -> Value {
    definition(name, "variant_type", json!({"variants": cases}))
}

fn document(mut constructs: Vec<Value>, types: Vec<Value>) -> Value {
    // The wire requires content digests; bind them to the authored types.
    let digest = format!(
        "sha256:{}",
        hex(&ByteDigest::of(&serde_json::to_vec(&types).unwrap()).as_bytes())
    );
    for construct in &mut constructs {
        construct["manifestDigest"] = json!(digest);
    }
    json!({
        "contractVersion": "2.0.0",
        "canonicalization": {"algorithm": "rfc8785-v1", "digest": "sha256-jcs"},
        "source": {
            "identity": id("spec"), "version": "1.0.0",
            "dialect": "spec-bundle", "digest": digest,
        },
        "package": {
            "identity": P, "version": "1.0.0", "manifestDigest": digest,
            "mappingVersions": [], "profileVersions": [], "lockDigest": digest,
        },
        "constructs": constructs, "types": types, "occurrences": [], "extensions": [],
    })
}

/// TC-994 step 1: `Tier` with `Gold` then `Standard`, and `Level` with `Gold`.
fn tiers(tier_cases: &[&str]) -> Value {
    let tier = tier_cases
        .iter()
        .enumerate()
        .map(|(position, name)| case("Tier", name, 10 + position as u64))
        .collect();
    document(
        vec![variant_construct()],
        vec![
            variant_type("Tier", tier),
            variant_type("Level", vec![case("Level", "Gold", 20)]),
        ],
    )
}

fn read(document: &Value) -> Result<DomainPackage, Vec<ModelRefusal>> {
    let bytes = serde_json::to_vec(document).expect("authored document serializes");
    let packages = package_input([bytes.as_slice()]);
    let digest = *packages.keys().next().expect("one authored package");
    let selection = DomainPackageRef {
        identity: document["package"]["identity"].as_str().unwrap().to_owned(),
        version: document["package"]["version"].as_str().unwrap().to_owned(),
        digest,
    };
    let (selection, admitted) = admit(
        &selection,
        SHA256_JCS_DIGEST_DOMAIN,
        &packages,
        IntakeLimits::default(),
    )
    .expect("the selected authored package passes byte admission");
    read_domain_package(selection, &admitted)
}

fn admitted(document: &Value) -> DomainPackage {
    let package = read(document).expect("the variant types are read");
    let (outcome, _) = normalize_with_meter(&package, ModelNormalizationLimits::UNLIMITED);
    assert!(
        matches!(outcome, NormalizeOutcome::Completed(_)),
        "{outcome:?}"
    );
    package
}

/// Every refusal the reader and normalization report for `document`, in
/// report order. Nothing is admitted when this is non-empty.
fn refusals(document: &Value) -> Vec<ModelRefusal> {
    let package = match read(document) {
        Ok(package) => package,
        Err(refusals) => return refusals,
    };
    let (outcome, _) = normalize_with_meter(&package, ModelNormalizationLimits::UNLIMITED);
    match outcome {
        NormalizeOutcome::Refused(refusals) => refusals.iter().cloned().collect(),
        other => panic!("the document must refuse, got {other:?}"),
    }
}

fn enum_of<'a>(package: &'a DomainPackage, node: &str) -> &'a EnumRecord {
    package
        .records
        .iter()
        .find_map(|record| match record {
            DomainPackageRecord::Enum(record) if record.key == key(node) => Some(record),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no enum export for {node}"))
}

fn variants_of<'a>(package: &'a DomainPackage, owner: &str) -> Vec<&'a VariantRecord> {
    package
        .records
        .iter()
        .filter_map(|record| match record {
            DomainPackageRecord::Variant(record) if record.owner == key(owner) => Some(record),
            _ => None,
        })
        .collect()
}

fn node_of(refusal: &ModelRefusal) -> &str {
    match &refusal.cause {
        ModelRefusalCause::IntakeMalformedDeclaration { node, .. }
        | ModelRefusalCause::UnsupportedDeclarationForm { node, .. } => node,
        ModelRefusalCause::ConflictingBinding { key, .. } => &key.node,
        other => panic!("no node on {other:?}"),
    }
}

fn member_of(refusal: &ModelRefusal) -> MemberPath {
    match &refusal.cause {
        ModelRefusalCause::IntakeMalformedDeclaration { member, .. }
        | ModelRefusalCause::UnsupportedDeclarationForm { member, .. } => member.clone(),
        ModelRefusalCause::ConflictingBinding {
            member: Some(member),
            ..
        } => member.clone(),
        other => panic!("no member on {other:?}"),
    }
}

fn path(text: &str) -> MemberPath {
    text.parse().expect("a member path")
}

fn assert_malformed(refusal: &ModelRefusal, node: &str, member: &str) {
    assert_eq!(refusal.code, Code::InvalidModelBinding);
    assert!(
        matches!(
            refusal.cause,
            ModelRefusalCause::IntakeMalformedDeclaration { .. }
        ),
        "{refusal:?}"
    );
    assert_eq!(node_of(refusal), id(node));
    assert_eq!(member_of(refusal), path(member));
}

/// TC-994 step 1: one declaration for `Tier` and one per case node; one
/// `enum` keyed by `Tier`'s identity with its cases as a sorted set; one
/// `variant` per case keyed by owner and case name; `Level`'s `Gold` is a
/// distinct `variant`.
#[trace("TC-994", "FR-056-AC-24")]
#[test]
fn a_variant_type_is_admitted_as_one_enum_and_one_variant_per_case() {
    let package = admitted(&tiers(&["Gold", "Standard"]));

    let mut declared: Vec<_> = package.records.iter().map(|r| r.key().clone()).collect();
    declared.sort();
    let mut expected = vec![
        key("Tier"),
        key("variant/Tier-Gold"),
        key("variant/Tier-Standard"),
        key("Level"),
        key("variant/Level-Gold"),
    ];
    expected.sort();
    assert_eq!(declared, expected);

    let tier = enum_of(&package, "Tier");
    assert_eq!(tier.name, "Tier");
    assert_eq!(tier.members, ["Gold", "Standard"]);

    let cases = variants_of(&package, "Tier");
    assert_eq!(cases.len(), 2);
    for (variant, name) in cases.iter().zip(["Gold", "Standard"]) {
        assert_eq!(variant.case, name);
        assert_eq!(variant.owner, tier.key);
        assert_eq!(variant.key, key(&format!("variant/Tier-{name}")));
        assert_eq!(variant.export_key(), (&key("Tier"), name));
        let original = package
            .original(&variant.key)
            .expect("a case retains its own origin as the export locus");
        assert_eq!(original.member_name.as_deref(), Some(name));
        assert!(original.origin.get("source").is_some());
    }

    // The admitted package finds each case under its owner and name.
    let admitted_package = AdmittedPackage::admit(
        package.clone(),
        &mut Meter::new(ModelNormalizationLimits::UNLIMITED),
    )
    .expect("the variant types admit for linking");
    for name in ["Gold", "Standard"] {
        let found = admitted_package
            .member(&key("Tier"), name)
            .expect("a case is a member of its variant type");
        assert_eq!(found.kind, DeclarationKind::VariantCase);
    }
    assert_eq!(
        admitted_package
            .declaration("Tier")
            .expect("the variant type is a declaration")
            .kind,
        DeclarationKind::VariantType
    );

    let level = variants_of(&package, "Level");
    assert_eq!(level.len(), 1);
    assert_eq!(level[0].case, "Gold");
    assert_ne!(level[0].export_key(), cases[0].export_key());
}

/// TC-994 step 2: listing the cases in the other order changes no key,
/// export or ordering.
#[trace("TC-994", "FR-056-AC-24")]
#[test]
fn reordering_the_cases_changes_no_key_export_or_ordering() {
    let gold_first = admitted(&tiers(&["Gold", "Standard"]));
    let standard_first = admitted(&tiers(&["Standard", "Gold"]));
    assert_eq!(gold_first.records, standard_first.records);
    assert_eq!(
        enum_of(&standard_first, "Tier").members,
        ["Gold", "Standard"]
    );
}

/// TC-994 step 3: `Gold` and `gold` are distinct cases, compared as bytes.
#[trace("TC-994", "FR-056-AC-24")]
#[test]
fn cases_that_differ_only_in_letter_case_are_distinct() {
    let package = admitted(&tiers(&["Gold", "gold"]));
    assert_eq!(enum_of(&package, "Tier").members, ["Gold", "gold"]);
    let cases = variants_of(&package, "Tier");
    assert_eq!(cases.len(), 2);
    assert_ne!(cases[0].export_key(), cases[1].export_key());
}

fn payload_tiers() -> Value {
    let mut gold = case("Tier", "Gold", 10);
    gold["payloadType"] = json!("ix://quire/native/Integer");
    document(
        vec![variant_construct()],
        vec![variant_type(
            "Tier",
            vec![case("Tier", "Standard", 11), gold],
        )],
    )
}

/// TC-995 step 1: a case with a payload refuses as an unsupported
/// declaration form at the case node, never as a malformed binding, and
/// `Standard` raises nothing of its own.
#[trace("TC-995", "FR-056-AC-25")]
#[test]
fn a_payload_on_a_case_refuses_as_an_unsupported_declaration_form() {
    let found = refusals(&payload_tiers());
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].code, Code::UnsupportedConstruct);
    assert!(
        matches!(
            found[0].cause,
            ModelRefusalCause::UnsupportedDeclarationForm { .. }
        ),
        "{:?}",
        found[0]
    );
    assert_eq!(node_of(&found[0]), id("variant/Tier-Gold"));
    assert_eq!(member_of(&found[0]), path("Variant.payloadType"));
}

/// TC-995 step 2: without the payload the type admits as in TC-994 step 1.
#[trace("TC-995", "FR-056-AC-25")]
#[test]
fn removing_the_payload_admits_the_type() {
    let mut without = payload_tiers();
    without["types"][0]["variants"][1]
        .as_object_mut()
        .unwrap()
        .remove("payloadType");
    let package = admitted(&without);
    assert_eq!(enum_of(&package, "Tier").members, ["Gold", "Standard"]);
}

fn single_type(mutate: impl FnOnce(&mut Value)) -> Value {
    let mut document = tiers(&["Gold", "Standard"]);
    document["types"].as_array_mut().unwrap().truncate(1);
    mutate(&mut document);
    document
}

fn assert_one_malformed(document: &Value, node: &str, member: &str) {
    let found = refusals(document);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_malformed(&found[0], node, member);
}

/// TC-996 step 1: a variant type with no case.
#[trace("TC-996", "FR-056-AC-26")]
#[test]
fn a_variant_type_with_no_case_refuses_as_malformed() {
    assert_one_malformed(
        &single_type(|d| d["types"][0]["variants"] = json!([])),
        "Tier",
        "TypeDefinition.variants",
    );
}

/// TC-996 step 2: a case whose origin is generated has no source span.
#[trace("TC-996", "FR-056-AC-26")]
#[test]
fn a_case_with_no_source_span_refuses_as_malformed() {
    assert_one_malformed(
        &single_type(|d| d["types"][0]["variants"][0]["origin"] = generated("variant/Tier-Gold")),
        "Tier",
        "TypeDefinition.variants[0].origin",
    );
}

/// TC-996 step 3: the variant-type meaning on a construct of shape `record`.
#[trace("TC-996", "FR-056-AC-26")]
#[test]
fn a_variant_meaning_on_a_record_shaped_construct_refuses_as_malformed() {
    let document = single_type(|d| {
        d["constructs"][0]["construct"]["shape"] = json!("record");
    });
    assert_one_malformed(&document, "Tier", "TypeDefinition.kind");
}

/// TC-996 step 4: two cases of one name and one identity are refused by the
/// reader before intake; nothing is declared and the diagnostic is not the
/// intake `conflicting-binding`.
#[trace("TC-996", "FR-056-AC-26")]
#[test]
fn two_cases_sharing_one_identity_are_refused_by_the_reader() {
    let document = single_type(|d| {
        d["types"][0]["variants"] = json!([case("Tier", "Gold", 10), case("Tier", "Gold", 11)]);
    });
    let found = refusals(&document);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(node_of(&found[0]), id("variant/Tier-Gold"));
    assert_eq!(found[0].code, Code::InvalidModelBinding);
    assert!(
        !found
            .iter()
            .any(|refusal| matches!(refusal.cause, ModelRefusalCause::ConflictingBinding { .. })),
        "the reader's diagnostic is kept, not intake's conflicting-binding"
    );
}

/// TC-996 step 5: two cases of one name with distinct identities, which the
/// reader admits, refuse `conflicting-binding` at the type node; a case whose
/// identity is not the minted one refuses `malformed-declaration`.
#[trace("TC-996", "FR-056-AC-26")]
#[test]
fn equal_case_names_with_distinct_identities_conflict_at_the_type() {
    let mut second = case("Tier", "Gold", 11);
    second["identity"] = json!(id("variant/Tier-Gold2"));
    let found = refusals(&single_type(|d| {
        d["types"][0]["variants"] = json!([case("Tier", "Gold", 10), second]);
    }));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].code, Code::InvalidModelBinding);
    assert_eq!(
        found[0].cause,
        ModelRefusalCause::ConflictingBinding {
            key: key("Tier"),
            member: Some(path("TypeDefinition.variants[1]")),
        }
    );
}

#[trace("TC-996", "FR-056-AC-26")]
#[test]
fn a_case_identity_that_is_not_minted_refuses_as_malformed() {
    assert_one_malformed(
        &single_type(|d| {
            d["types"][0]["variants"][0]["identity"] = json!(id("variant/Tier-Platinum"));
        }),
        "variant/Tier-Platinum",
        "Variant.identity",
    );
}

/// TC-996 step 6: a case with no name is refused by the reader's schema.
#[trace("TC-996", "FR-056-AC-26")]
#[test]
fn a_case_with_no_name_is_refused_by_the_reader_schema() {
    let found = refusals(&single_type(|d| {
        d["types"][0]["variants"][0]
            .as_object_mut()
            .unwrap()
            .remove("name");
    }));
    assert!(!found.is_empty());
    for refusal in &found {
        assert_eq!(refusal.code, Code::InvalidModelBinding);
        assert!(matches!(
            refusal.cause,
            ModelRefusalCause::IntakeMalformedDeclaration { .. }
        ));
    }
}

/// TC-997 step 1: no clause names `Tier`, so no refusal is raised.
#[trace("TC-997", "FR-056-AC-27")]
#[test]
fn a_unit_with_no_clause_naming_the_variant_type_assembles() {
    let document = serde_json::to_vec(&tiers(&["Gold", "Standard"])).unwrap();
    let assembled =
        admit_and_assemble_with_body(&document, "function noop using v(): Boolean pure { true }");
    assert!(assembled.is_ok(), "{:?}", assembled.err());
}

/// TC-997 step 2: a clause naming `Tier` refuses `unsupported_construct`/
/// `declaration-form` at the clause, while the declaration and its exports
/// stay admitted and equal to step 1's.
#[trace("TC-997", "FR-056-AC-27")]
#[test]
fn a_clause_naming_a_variant_type_refuses_while_the_declaration_is_admitted() {
    let document = tiers(&["Gold", "Standard"]);
    let bytes = serde_json::to_vec(&document).unwrap();
    let refusal = admit_and_assemble_with_body(
        &bytes,
        "invariant OnTier using v on Config::Tier at current { true }",
    )
    .expect_err("a clause naming the variant type refuses");
    assert_eq!(refusal.errors.len(), 1, "{refusal:?}");
    let cause = &refusal.errors[0].cause;
    assert_eq!(
        cause,
        &AssemblyCause::UnsupportedVariantType {
            name: "Config::Tier".to_owned()
        }
    );
    assert_eq!(
        cause.catalog_code().to_string(),
        "unsupported_construct/declaration-form"
    );

    // The package the refused unit selected carries the declaration and
    // exports of step 1's package, which no clause names.
    let step_one = admitted(&document);
    let (unit, packages) = config_unit_with_body(
        &bytes,
        "invariant OnTier using v on Config::Tier at current { true }",
    );
    let built = parse_and_build(&unit);
    let models = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("the package is admitted although the clause is refused");
    assert_eq!(models[0].view.domain_package().records, step_one.records);
    assert_eq!(enum_of(&step_one, "Tier").members, ["Gold", "Standard"]);
    assert_eq!(variants_of(&step_one, "Tier").len(), 2);
}

/// FR-056 "Variant types": a `Reference<Config::Tier>` in source names the
/// variant type too, and refuses the same way.
#[trace("TC-997", "FR-056-AC-27")]
#[test]
fn a_reference_to_a_variant_type_in_source_refuses_as_a_declaration_form() {
    let bytes = serde_json::to_vec(&tiers(&["Gold", "Standard"])).unwrap();
    let refusal = admit_and_assemble_with_body(
        &bytes,
        "function pick using v(r: Reference<Config::Tier>): Boolean pure { true }",
    )
    .expect_err("a reference to the variant type refuses");
    assert_eq!(refusal.errors.len(), 1, "{refusal:?}");
    assert_eq!(
        refusal.errors[0].cause,
        AssemblyCause::UnsupportedVariantType {
            name: "Config::Tier".to_owned()
        }
    );
}

/// TC-997 step 3: a field whose `typeRef` names `Tier` refuses
/// `invalid_model_binding`/`malformed-declaration` at the field node and
/// admits nothing. QSL-738 replaces this placeholder.
#[trace("TC-997", "FR-056-AC-28")]
#[test]
fn a_field_typed_by_a_variant_type_refuses_at_the_field() {
    let field = json!({
        "identity": id("Account/tier"), "name": "tier",
        "typeRef": id("Tier"), "presence": "required", "nullable": false,
        "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": generated("Account/tier"),
    });
    let account = definition(
        "Account",
        "object_type",
        json!({"supertypes": [], "fields": [field], "operations": [], "relationships": []}),
    );
    let document = document(
        vec![
            variant_construct(),
            construct("object_type", meaning::OBJECT_TYPE, "record", json!({})),
        ],
        vec![
            variant_type(
                "Tier",
                vec![case("Tier", "Gold", 10), case("Tier", "Standard", 11)],
            ),
            account,
        ],
    );
    let found = refusals(&document);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_malformed(&found[0], "Account/tier", "Field.typeRef");
}

/// FR-056 "Variant types": a signature typeRef naming a variant type is a
/// reference to a node of a meaning it may not name, `malformed-declaration`
/// (parameter and result alike).
#[trace("TC-997", "FR-056-AC-28")]
#[test]
fn an_operation_signature_naming_a_variant_type_refuses_as_malformed() {
    let tier = variant_type(
        "Tier",
        vec![case("Tier", "Gold", 10), case("Tier", "Standard", 11)],
    );
    let setter = operation(
        "setTier",
        json!([operation_parameter("setTier", "t", &id("Tier"))]),
        Some(&id("Tier")),
        empty_frame(),
    );
    let bytes = config_version_document(setter, vec![variant_construct()], vec![tier], json!([]));
    let document: Value = serde_json::from_slice(&bytes).unwrap();
    let found = refusals(&document);
    assert_eq!(found.len(), 2, "parameter and result: {found:?}");
    for refusal in &found {
        assert_eq!(refusal.code, Code::InvalidModelBinding);
        assert_eq!(refusal.cause, ModelRefusalCause::MalformedDeclaration);
    }
}

/// FR-154: every failing check of one node is reported. A type with two
/// payload cases reports both, each at its own case node.
#[trace("TC-995", "FR-056-AC-25")]
#[test]
fn every_payload_case_of_one_type_is_reported() {
    let mut document = payload_tiers();
    let standard = &mut document["types"][0]["variants"][0];
    standard["payloadType"] = json!("ix://quire/native/Integer");
    let found = refusals(&document);
    assert_eq!(found.len(), 2, "{found:?}");
    let mut nodes: Vec<_> = found.iter().map(node_of).collect();
    nodes.sort_unstable();
    assert_eq!(
        nodes,
        [id("variant/Tier-Gold"), id("variant/Tier-Standard")]
    );
    for refusal in &found {
        assert_eq!(refusal.code, Code::UnsupportedConstruct);
        assert_eq!(member_of(refusal), path("Variant.payloadType"));
    }
}

/// FR-154: an empty `variants` list and a wrong construct shape can both
/// hold, and both are reported for the one type node.
#[trace("TC-996", "FR-056-AC-26")]
#[test]
fn an_empty_and_mis_shaped_variant_type_reports_both() {
    let found = refusals(&single_type(|d| {
        d["constructs"][0]["construct"]["shape"] = json!("record");
        d["constructs"][0]["construct"]["members"] = json!({});
        d["types"][0]["variants"] = json!([]);
    }));
    assert_eq!(found.len(), 2, "{found:?}");
    // Member-path order: the construct shape, then the empty list.
    assert_malformed(&found[0], "Tier", "TypeDefinition.kind");
    assert_malformed(&found[1], "Tier", "TypeDefinition.variants");
}

fn lifted(values: &[&str]) -> Result<Vec<u8>, qsl_semantics::model::intake::LiftFailure> {
    let fixtures = crate::model_intake::fcd_fixtures_dir();
    let bundle = tempfile::tempdir().expect("tempdir");
    let functional = bundle.path().join("spec/functional");
    std::fs::create_dir_all(&functional).expect("bundle dirs");
    std::fs::copy(
        fixtures.join("resolve/enumeration/spec/spec.md"),
        bundle.path().join("spec/spec.md"),
    )
    .expect("copy master spec");
    let mut rows = String::new();
    for value in values {
        rows.push_str(&format!("| {value} | A tier |\n"));
    }
    std::fs::write(
        functional.join("EN_001-tier.md"),
        format!(
            "---\nid: EN_001\ntitle: Tier\nobject: enumeration\ntype: FR\n---\n\n\
             # EN_001: Tier\n\n## Values\n\n| Value | Description |\n|-------|-------------|\n{rows}"
        ),
    )
    .expect("write enumeration");
    qsl_semantics::model::intake::lift_document(
        bundle.path(),
        &[fixtures.join("modules/fixture-domain")],
    )
}

/// TC-994 step 1 through the real lift: the lifted enumeration is a variant
/// type whose minted case identities the reader accepts.
#[trace("TC-994", "FR-056-AC-24")]
#[test]
fn a_lifted_enumeration_is_admitted_as_a_variant_type() {
    let bytes = lifted(&["Gold", "Standard"]).expect("the bundle lifts");
    let document: Value = serde_json::from_slice(&bytes).unwrap();
    let package = admitted(&document);
    let tier = package
        .records
        .iter()
        .find_map(|record| match record {
            DomainPackageRecord::Enum(record) => Some(record),
            _ => None,
        })
        .expect("one enum export");
    assert_eq!(tier.name, "EN_001");
    assert_eq!(tier.members, ["Gold", "Standard"]);
    let cases: Vec<_> = package
        .records
        .iter()
        .filter_map(|record| match record {
            DomainPackageRecord::Variant(case) => Some((case.case.as_str(), &case.owner)),
            _ => None,
        })
        .collect();
    assert_eq!(cases, [("Gold", &tier.key), ("Standard", &tier.key)]);
}

/// TC-996 step 4, lift half: enumeration rows that slug alike are refused by
/// the lift with `UNSLUGGABLE_NAME`, before any document exists.
#[trace("TC-996", "FR-056-AC-26")]
#[test]
fn enumeration_rows_that_slug_alike_are_refused_by_the_lift() {
    let refused = lifted(&["Snap_Shot", "Snap__Shot"]).expect_err("slug-alike rows are refused");
    let qsl_semantics::model::intake::LiftFailure::Blocked(diagnostics) = refused else {
        panic!("a blocking diagnostic, got {refused:?}");
    };
    assert!(
        diagnostics
            .iter()
            .any(|d| d.code.to_string() == "agent-ix.extraction-frontend.UNSLUGGABLE_NAME"),
        "{diagnostics:?}"
    );
}
