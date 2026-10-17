// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-056 "Variant types" (QSL-734): a variant type is admitted as one `enum`
//! export and one `variant` export per case (TC-994), refuses a payload
//! (TC-995), refuses malformed and duplicate cases with their causes
//! (TC-996), and admits a clause, field, parameter or result that names it
//! while its own declaration stays unchanged (TC-997; FR-360 states how a
//! member typed by it is assembled and checked).
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
    DomainPackage, DomainPackageRecord, DomainPackageRef, EnumRecord, ValueTypeRef, VariantRecord,
};
use qsl_semantics::model::intake::{admit, meaning, package_input, read_domain_package};
use qsl_semantics::model::key::{
    hex, DeclarationKey, EffectiveDeclarationPreimage, RULE_QUALIFY, SHA256_JCS_DIGEST_DOMAIN,
};
use qsl_semantics::model::normalize::{
    normalize_with_meter, EffectiveView, ModelRefusal, ModelRefusalCause, NormalizeOutcome,
};
use qsl_semantics::model::refusal::MemberPath;
use quire_exact::EffectiveId;
use quire_semantic_value::checking::CheckingLimits;
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
    let (outcome, _) = normalize_with_meter(&package, ModelNormalizationLimits::default());
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
    let (outcome, _) = normalize_with_meter(&package, ModelNormalizationLimits::default());
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
        &mut Meter::new(ModelNormalizationLimits::default()),
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

/// TC-996 step 4: two cases of one name and one identity are refused at the
/// reader's duplicate position as `conflicting-binding` at the shared case identity, nothing
/// is declared (FR-154: two nodes sharing one identity).
#[trace("TC-996", "FR-056-AC-26")]
#[test]
fn two_cases_sharing_one_identity_are_refused_by_the_reader() {
    let document = single_type(|d| {
        d["types"][0]["variants"] = json!([case("Tier", "Gold", 10), case("Tier", "Gold", 11)]);
    });
    let found = refusals(&document);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].code, Code::InvalidModelBinding);
    assert_eq!(
        found[0].cause,
        ModelRefusalCause::ConflictingBinding {
            key: key("variant/Tier-Gold"),
            member: Some(path("TypeDefinition.variants[1]")),
        }
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

/// TC-997 step 2: a signature naming `Tier` raises no refusal, and the
/// package the unit selected carries the declaration and exports of step 1's
/// package, which no member names.
#[trace("TC-997", "FR-056-AC-27", "FR-056-AC-31")]
#[test]
fn a_unit_naming_a_variant_type_leaves_the_declaration_and_exports_unchanged() {
    let document = tiers(&["Gold", "Standard"]);
    let bytes = serde_json::to_vec(&document).unwrap();
    let body = "function pick using v(t: Config::Tier): Config::Tier pure { t }";
    admit_and_assemble_with_body(&bytes, body)
        .expect("a function typed by the variant type assembles");

    let step_one = admitted(&document);
    let (unit, packages) = config_unit_with_body(&bytes, body);
    let built = parse_and_build(&unit);
    let models = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("the package is admitted");
    assert_eq!(models[0].view.domain_package().records, step_one.records);
    assert_eq!(enum_of(&step_one, "Tier").members, ["Gold", "Standard"]);
    assert_eq!(variants_of(&step_one, "Tier").len(), 2);
}

/// FR-360 item 4: a `Reference<Config::Tier>` is no object reference, so the
/// target names no object type and refuses as an unresolved type name.
#[trace("TC-997", "FR-056-AC-27")]
#[test]
fn a_reference_to_a_variant_type_names_no_object_type() {
    let bytes = serde_json::to_vec(&tiers(&["Gold", "Standard"])).unwrap();
    let refusal = admit_and_assemble_with_body(
        &bytes,
        "function pick using v(r: Reference<Config::Tier>): Boolean pure { true }",
    )
    .expect_err("a reference to the variant type refuses");
    assert_eq!(refusal.errors.len(), 1, "{refusal:?}");
    assert_eq!(
        refusal.errors[0].cause,
        AssemblyCause::UnresolvedTypeName {
            name: "Config::Tier".to_owned()
        }
    );
}

fn tier_field_package() -> Value {
    let fields = vec![json!({
        "identity": id("Account/tier"), "name": "tier",
        "typeRef": id("Tier"), "presence": "required", "nullable": false,
        "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": generated("Account/tier"),
    })];
    let account = definition(
        "Account",
        "object_type",
        json!({"supertypes": [], "fields": fields, "operations": [], "relationships": []}),
    );
    document(
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
    )
}

/// TC-997 step 3: a field whose `typeRef` names `Tier` is admitted with one
/// field export and no reference export, and the variant type keeps its
/// `enum` and `variant` exports.
#[trace("TC-997", "FR-056-AC-28")]
#[test]
fn a_field_typed_by_a_variant_type_is_admitted() {
    let package = admitted(&tier_field_package());
    let fields: Vec<_> = package
        .records
        .iter()
        .filter_map(|record| match record {
            DomainPackageRecord::FieldMember(field) => Some(field),
            _ => None,
        })
        .collect();
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].key, key("Account/tier"));
    assert_eq!(fields[0].value_type, ValueTypeRef::Package(key("Tier")));
    assert_eq!(enum_of(&package, "Tier").members, ["Gold", "Standard"]);
    assert!(!package
        .records
        .iter()
        .any(|record| matches!(record, DomainPackageRecord::Relationship(_))));
}

/// The origin the signature refusals retain: the operation's own FCD
/// `origin.source` (the `operation` helper writes `spec`, line 1, column 1).
fn assert_operation_origin(refusal: &ModelRefusal) {
    let ModelRefusalCause::IntakeMalformedDeclaration { artifact, span, .. } = &refusal.cause
    else {
        panic!("expected a malformed declaration: {refusal:?}");
    };
    assert_eq!(artifact.as_deref(), Some(id("spec").as_str()));
    let span = span.expect("the operation's source position is retained");
    assert_eq!((span.start.line, span.start.column), (1, 1));
}

/// FR-360 item 8: a signature `typeRef` naming a variant type is admitted,
/// parameter and result alike, at any parameter position.
#[trace("TC-997", "FR-056-AC-32")]
#[test]
fn an_operation_signature_naming_a_variant_type_is_admitted() {
    let tier = variant_type(
        "Tier",
        vec![case("Tier", "Gold", 10), case("Tier", "Standard", 11)],
    );
    let setter = operation(
        "setTier",
        json!([
            operation_parameter("setTier", "n", "ix://quire/native/Integer"),
            operation_parameter("setTier", "t", &id("Tier")),
        ]),
        Some(&id("Tier")),
        empty_frame(),
    );
    let bytes = config_version_document(setter, vec![variant_construct()], vec![tier], json!([]));
    let document: Value = serde_json::from_slice(&bytes).unwrap();
    let package = admitted(&document);
    let operation = package
        .records
        .iter()
        .find_map(|record| match record {
            DomainPackageRecord::OperationMember(operation) => Some(operation),
            _ => None,
        })
        .expect("the operation is admitted");
    assert_eq!(
        operation.parameters[1].value_type,
        ValueTypeRef::Package(key("Tier"))
    );
    assert_eq!(
        operation.result.as_ref().map(|result| &result.value_type),
        Some(&ValueTypeRef::Package(key("Tier")))
    );
}

/// FR-360 item 8: the signature refusal stays for the meanings FR-056 lists:
/// a state machine named by a parameter or the result still refuses at the
/// operation node with the parameter's own position.
#[trace("TC-997", "FR-056-AC-32")]
#[test]
fn a_later_parameter_naming_a_forbidden_meaning_keeps_its_own_position() {
    let tier = variant_type(
        "Tier",
        vec![case("Tier", "Gold", 10), case("Tier", "Standard", 11)],
    );
    let machine = definition(
        "Machine",
        "state_machine",
        json!({
            "fields": [],
            "operations": [{
                "identity": id("Machine/go"), "name": "go", "params": [], "pre": [], "post": [],
                "frame": {"modifies": [], "creates": [], "deletes": []},
                "origin": generated("Machine/go"),
            }],
            "clauses": [],
            "states": [
                {"identity": id("Machine/Open"), "name": "Open", "origin": generated("Machine/Open")},
                {"identity": id("Machine/Closed"), "name": "Closed", "origin": generated("Machine/Closed")},
            ],
            "transitions": [{
                "identity": id("Machine/step"), "from": id("Machine/Open"),
                "to": id("Machine/Closed"), "trigger": id("Machine/go"),
                "emits": [], "origin": generated("Machine/step"),
            }],
        }),
    );
    let setter = operation(
        "setTier",
        json!([
            operation_parameter("setTier", "n", "ix://quire/native/Integer"),
            operation_parameter("setTier", "m", &id("Machine")),
            operation_parameter("setTier", "t", &id("Tier")),
        ]),
        Some(&id("Machine")),
        empty_frame(),
    );
    let bytes = config_version_document(
        setter,
        vec![
            variant_construct(),
            construct(
                "state_machine",
                meaning::STATE_MACHINE,
                "record",
                json!({"states": "required", "transitions": "required"}),
            ),
        ],
        vec![tier, machine],
        json!([]),
    );
    let document: Value = serde_json::from_slice(&bytes).unwrap();
    let found = refusals(&document);
    assert_eq!(found.len(), 2, "parameter and result: {found:?}");
    let operation = "ConfigVersion/setTier";
    assert_malformed(&found[0], operation, "Operation.params[1].typeRef");
    assert_malformed(&found[1], operation, "Operation.returns.typeRef");
    found.iter().for_each(assert_operation_origin);
}

/// FR-056 "Variant types": an undeclared signature type keeps the operation
/// node, the typed path and the operation's reader origin. The reader refuses
/// an absent type itself, so the reference is broken after the read.
#[trace("TC-997", "FR-056-AC-32")]
#[test]
fn an_undeclared_signature_type_keeps_the_operations_origin() {
    let setter = operation(
        "setCount",
        json!([
            operation_parameter("setCount", "n", "ix://quire/native/Integer"),
            operation_parameter("setCount", "m", "ix://quire/native/Integer"),
        ]),
        Some("ix://quire/native/Integer"),
        empty_frame(),
    );
    let bytes = config_version_document(setter, vec![], vec![], json!([]));
    let document: Value = serde_json::from_slice(&bytes).unwrap();
    let mut package = read(&document).expect("the document reads");
    let missing = ValueTypeRef::Package(key("Missing"));
    for record in &mut package.records {
        if let DomainPackageRecord::OperationMember(op) = record {
            if op.key == key("ConfigVersion/setCount") {
                op.parameters[1].value_type = missing.clone();
                op.result.as_mut().expect("a result").value_type = missing.clone();
            }
        }
    }
    let (outcome, _) = normalize_with_meter(&package, ModelNormalizationLimits::default());
    let NormalizeOutcome::Refused(found) = outcome else {
        panic!("the undeclared type must refuse: {outcome:?}");
    };
    assert_eq!(found.len(), 2, "{found:?}");
    let mut members = Vec::new();
    for refusal in found.iter() {
        assert_eq!(refusal.code, Code::DanglingReference);
        let ModelRefusalCause::UnknownValueType {
            operation, site, ..
        } = &refusal.cause
        else {
            panic!("expected unknown-value-type: {refusal:?}");
        };
        assert_eq!(operation, &key("ConfigVersion/setCount"));
        assert_eq!(site.artifact.as_deref(), Some(id("spec").as_str()));
        let span = site.span.expect("the operation's source position");
        assert_eq!((span.start.line, span.start.column), (1, 1));
        members.push(site.member.to_string());
    }
    members.sort();
    assert_eq!(
        members,
        ["Operation.params[1].typeRef", "Operation.returns.typeRef"]
    );
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

#[cfg(feature = "test-support")]
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
#[cfg(feature = "test-support")]
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
#[cfg(feature = "test-support")]
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

/// The effective view normalization completes for `document`.
fn view_of(document: &Value) -> EffectiveView {
    let package = read(document).expect("the variant types are read");
    match normalize_with_meter(&package, ModelNormalizationLimits::default()).0 {
        NormalizeOutcome::Completed(view) => view,
        other => panic!("normalization must complete, got {other:?}"),
    }
}

/// The effective declaration whose original is `original`.
fn entry_of<'a>(
    view: &'a EffectiveView,
    original: &DeclarationKey,
) -> &'a qsl_semantics::model::normalize::ViewEntry {
    let mut found = view
        .declarations()
        .iter()
        .filter(|entry| &entry.preimage.original == original);
    let entry = found
        .next()
        .unwrap_or_else(|| panic!("no effective declaration for {original:?}"));
    assert!(found.next().is_none(), "one effective entry per original");
    entry
}

/// The single phase-2 qualify fact `[original]` and nothing else.
fn assert_only_qualify(preimage: &EffectiveDeclarationPreimage) {
    assert_eq!(preimage.derivation.len(), 1, "{preimage:?}");
    let fact = &preimage.derivation[0];
    assert_eq!(fact.ordinal, 0);
    assert_eq!(fact.rule, RULE_QUALIFY);
    assert_eq!(
        fact.inputs.iter().collect::<Vec<_>>(),
        [&preimage.original],
        "the qualify fact names the original alone"
    );
}

/// TC-1003 step 1: a variant type is one effective type and each case one
/// effective member of it, derived from the case node's own key.
#[trace("TC-1003", "FR-056-AC-29", "FR-081-AC-16")]
#[test]
fn a_variant_type_and_its_cases_have_effective_identities() {
    let view = view_of(&tiers(&["Gold", "Standard"]));
    assert_eq!(view.declarations().len(), 5, "two types and three cases");

    let identities = view.type_identities();
    for node in ["Tier", "Level"] {
        let entry = entry_of(&view, &key(node));
        assert_eq!(entry.preimage.owner_effective_type, None);
        assert_only_qualify(&entry.preimage);
        assert_eq!(identities.get(&key(node)), Some(&entry.effective_id));
    }
    assert_eq!(identities.len(), 2, "only the variant types are types");

    let tier = identities[&key("Tier")];
    let level = identities[&key("Level")];
    for (owner, node) in [
        (tier, "variant/Tier-Gold"),
        (tier, "variant/Tier-Standard"),
        (level, "variant/Level-Gold"),
    ] {
        let entry = entry_of(&view, &key(node));
        assert_eq!(entry.preimage.owner_effective_type, Some(owner));
        assert!(entry.visible);
        assert_only_qualify(&entry.preimage);
    }

    // `Tier`'s `Gold` and `Level`'s `Gold` are different effective members.
    assert_ne!(
        entry_of(&view, &key("variant/Tier-Gold")).effective_id,
        entry_of(&view, &key("variant/Level-Gold")).effective_id
    );
}

/// TC-1003 step 2: the order the document lists the cases in changes no
/// effective identity, entry or position.
#[trace("TC-1003", "FR-056-AC-29", "FR-081-AC-16")]
#[test]
fn reordering_the_cases_changes_no_effective_identity() {
    let gold_first = view_of(&tiers(&["Gold", "Standard"]));
    let standard_first = view_of(&tiers(&["Standard", "Gold"]));
    // The packages differ in their bytes (case origins), so the views'
    // package digests differ; every effective entry and its position match.
    assert_eq!(gold_first.declarations(), standard_first.declarations());
    assert_eq!(
        gold_first.type_identities(),
        standard_first.type_identities()
    );
}

/// TC-1003 step 3: cases that differ only in letter case are distinct
/// effective members, and no case shares an identity with its owner.
#[trace("TC-1003", "FR-056-AC-29")]
#[test]
fn cases_differing_in_letter_case_have_distinct_effective_identities() {
    let view = view_of(&tiers(&["Gold", "gold"]));
    let upper = entry_of(&view, &key("variant/Tier-Gold")).effective_id;
    let lower = entry_of(&view, &key("variant/Tier-gold")).effective_id;
    assert_ne!(upper, lower);
    let ids: std::collections::BTreeSet<EffectiveId> = view
        .declarations()
        .iter()
        .map(|entry| entry.effective_id)
        .collect();
    assert_eq!(ids.len(), view.declarations().len());
}

/// TC-1003 step 4: two same-name cases with distinct identities refuse and
/// leave no effective view, even beside a valid variant type.
#[trace("TC-1003", "FR-056-AC-30")]
#[test]
fn a_same_name_case_conflict_yields_no_effective_view() {
    let mut second = case("Tier", "Gold", 11);
    second["identity"] = json!(id("variant/Tier-Gold2"));
    let found = refusals(&single_type(|d| {
        d["types"][0]["variants"] = json!([case("Tier", "Gold", 10), second]);
        d["types"]
            .as_array_mut()
            .unwrap()
            .push(variant_type("Level", vec![case("Level", "Gold", 20)]));
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

/// TC-1003 step 5: two cases that share one identity are refused once, at the
/// second case of the `variants` list, whichever of them is minted correctly:
/// `invalid_model_binding`/`conflicting-binding` at the shared case identity,
/// path `TypeDefinition.variants[1]` (FR-154: two nodes sharing one identity).
#[trace("TC-1003", "FR-056-AC-30")]
#[test]
fn two_cases_sharing_an_identity_are_refused_at_the_second_listed() {
    for (gold_line, silver_line) in [(10_u64, 11_u64), (11, 10)] {
        let gold = case("Tier", "Gold", gold_line);
        let mut silver = case("Tier", "Silver", silver_line);
        silver["identity"] = json!(id("variant/Tier-Gold"));
        let list = if gold_line < silver_line {
            json!([gold, silver])
        } else {
            json!([silver, gold])
        };
        let found = refusals(&single_type(|d| d["types"][0]["variants"] = list));
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].code, Code::InvalidModelBinding);
        assert_eq!(
            found[0].cause,
            ModelRefusalCause::ConflictingBinding {
                key: key("variant/Tier-Gold"),
                member: Some(path("TypeDefinition.variants[1]")),
            }
        );
    }
}

/// TC-1004: the assembler declares each variant type in the type environment
/// under its effective identity, with every case keyed by its own effective
/// identity in ascending name order, while no source name resolves to it.
#[trace("TC-1004", "FR-056-AC-31")]
#[test]
fn the_assembler_declares_variant_types_in_the_type_environment() {
    let document = tiers(&["Standard", "Gold"]);
    let bytes = serde_json::to_vec(&document).unwrap();
    let declarations =
        admit_and_assemble_with_body(&bytes, "function noop using v(): Boolean pure { true }")
            .expect("a unit naming no variant type assembles");
    let view = view_of(&document);

    assert_eq!(declarations.variant_types.len(), 2);
    for (node, name, cases) in [
        ("Tier", "Config::Tier", vec!["Gold", "Standard"]),
        ("Level", "Config::Level", vec!["Gold"]),
    ] {
        let declared = declarations
            .variant_types
            .iter()
            .find(|declared| declared.name() == name)
            .unwrap_or_else(|| panic!("{name} is declared"));
        assert_eq!(declared.key(), view.type_identities()[&key(node)]);
        let declared_cases: Vec<_> = declared.cases().iter().map(|c| c.name()).collect();
        assert_eq!(declared_cases, cases);
        for case in declared.cases() {
            let entry = entry_of(&view, &key(&format!("variant/{node}-{}", case.name())));
            assert_eq!(case.key(), entry.effective_id);
            assert_eq!(entry.preimage.owner_effective_type, Some(declared.key()));
        }
    }
    let keys: Vec<_> = declarations.variant_types.iter().map(|v| v.key()).collect();
    assert!(keys.windows(2).all(|pair| pair[0] < pair[1]));

    // The checked scope holds them too, found by effective identity.
    let expected = declarations.variant_types.clone();
    let graph = declarations
        .check(CheckingLimits::default())
        .expect("the unit checks");
    let scope = graph.scope();
    assert_eq!(scope.variant_types(), expected.as_slice());
    for declared in &expected {
        assert_eq!(scope.variant_type(declared.key()), Some(declared));
    }
}

/// FR-302, FR-360 item 1: an `operation-body` whose target names a variant
/// type no longer refuses as a declaration form; a variant type is no object
/// type, so the target resolves to none, as a clause context naming it does.
#[trace("TC-997", "FR-056-AC-27")]
#[test]
fn an_operation_body_targeting_a_variant_type_names_no_object_type() {
    let bytes = serde_json::to_vec(&tiers(&["Gold", "Standard"])).unwrap();
    let refusal =
        admit_and_assemble_with_body(&bytes, "body b using v on Config::Tier::size() { 1 }")
            .expect_err("a body targeting the variant type refuses");
    assert_eq!(refusal.errors.len(), 1, "{refusal:?}");
    assert_eq!(
        refusal.errors[0].cause,
        AssemblyCause::UnresolvedTypeName {
            name: "Config::Tier".to_owned()
        }
    );
}
