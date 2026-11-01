// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-360-AC-8, FR-360-AC-4 (TC-1009, TC-1007 step 3): a unit whose
//! function, clause or field read names a model variant type emits it as one
//! model-owned `variant_type` node, each case as a `value`/`enum_value` node
//! of it (any non-empty case name, kept exactly), passes a variant value by
//! `ref` with no conversion node, and reads back `Verified` through IR's
//! checked-package/v2 reader. A case that is not the owner's, or an owner that
//! is not the declaring variant type, is refused by the same reader.

use super::owners::node_where;
use super::*;

#[path = "../../../../qsl-semantics/tests/support/variant_fields.rs"]
pub(super) mod fixture;

/// The cases of the variant type `Odd`, in ascending UTF-8 byte order: none
/// is a source identifier.
const ODD_CASES: [&str; 3] = ["2x", "Café", "Gold Plus"];

/// The units that name only `Tier`, whose cases are identifiers.
const TIER_UNIT: &str = "\
function pick using v(t: M::Tier): M::Tier pure { t }\n\
function isGold using v(t: M::Tier): Boolean pure { t = M::Tier::Gold }\n\
function notGold using v(t: M::Tier): Boolean pure { t != M::Tier::Gold }\n\
function tierOf using v(s: M::Settings): M::Tier pure { s.tier }\n\
post PromoteKeeps using v on M::Account::promote { result = t }\n";

/// [`TIER_UNIT`] and a function over `Odd`.
const UNIT: &str = "\
function pick using v(t: M::Tier): M::Tier pure { t }\n\
function isGold using v(t: M::Tier): Boolean pure { t = M::Tier::Gold }\n\
function notGold using v(t: M::Tier): Boolean pure { t != M::Tier::Gold }\n\
function tierOf using v(s: M::Settings): M::Tier pure { s.tier }\n\
function oddSame using v(a: M::Odd, b: M::Odd): Boolean pure { a = b }\n\
post PromoteKeeps using v on M::Account::promote { result = t }\n";

struct Emitted {
    emission: Emission,
    wire: Value,
    evidence: CheckedPackageEvidence,
}

fn emit_unit(unit_body: &str) -> Emitted {
    let document = serde_json::to_vec(&fixture::with_types(vec![fixture::variant_type(
        "Odd", &ODD_CASES,
    )]))
    .expect("the fixture serializes");
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model M = {package:?};\n\
         {unit_body}\n",
        package = fixture::PACKAGE,
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    evidence
        .insert_domain_package_document(package_identity(&document), document.as_slice())
        .expect("a fresh evidence key");
    Emitted {
        emission,
        wire,
        evidence,
    }
}

impl Emitted {
    fn read(&self, bytes: &[u8]) -> Read {
        read_v2(bytes, library(), V2ReadLimits::default(), &self.evidence)
    }

    /// The node of the variant type `name`.
    fn variant_node(&self, name: &str) -> &Value {
        let found: Vec<&Value> = node_where(&self.wire, "model", "variant_type")
            .into_iter()
            .filter(|node| node["owner"]["node"] == fixture::id(name))
            .collect();
        assert_eq!(found.len(), 1, "{name} is one variant_type node");
        found[0]
    }

    /// The case names of the `enum_value` nodes typed by `variant`, in the
    /// order the wire holds them.
    fn cases_of(&self, variant: &Value) -> Vec<String> {
        node_where(&self.wire, "value", "enum_value")
            .into_iter()
            .filter(|node| node["semantic_type"] == variant["node_id"])
            .map(|node| node["body"]["value"].as_str().unwrap().to_owned())
            .collect()
    }
}

/// FR-360-AC-8 item 12: each variant type the unit names is one
/// `model`/`variant_type` node with a `ModelOwner` of the declaring package
/// and the type's node, none is a unit-owned `scalar_type`/`enum`, and the
/// cases of the type are its `enum_value` nodes, any non-empty name kept
/// exactly.
#[trace("FR-360-AC-8", "TC-1009")]
#[test]
fn a_variant_type_is_one_model_owned_node_with_one_enum_value_per_case() {
    let emitted = emit_unit(UNIT);
    for name in ["Tier", "Odd"] {
        assert_eq!(
            emitted.variant_node(name)["owner"],
            json!({"kind": "model", "identity": fixture::PACKAGE, "node": fixture::id(name)})
        );
    }
    assert_eq!(
        node_where(&emitted.wire, "scalar_type", "enum").len(),
        0,
        "no variant type is a unit-owned enum"
    );
    let mut cases = emitted.cases_of(emitted.variant_node("Tier"));
    cases.sort();
    assert_eq!(cases, ["Gold", "Standard"]);
    let mut odd = emitted.cases_of(emitted.variant_node("Odd"));
    odd.sort();
    assert_eq!(odd, ODD_CASES);
}

/// FR-354: selected model enum rows retain the selected document and the
/// original declaration/member identities, then join to the admitted nodes.
#[trace("FR-354", "TC-418")]
#[test]
fn selected_model_enum_rows_join_and_refuse_missing_mismatched_duplicate_and_foreign_rows() {
    use quire_contract_model::{
        SelectedModelEnumDeclarationRow, SelectedModelEnumErrorCode, SelectedModelEnumMemberRow,
        SelectedModelEnums,
    };

    let emitted = emit_unit(UNIT);
    let Read::Verified { admitted, .. } = emitted.read(emitted.emission.package.bytes()) else {
        panic!("the emitted model enums are admitted")
    };
    let declarations = emitted.emission.selected_model_enum_declaration_rows();
    let members = emitted.emission.selected_model_enum_member_rows();
    assert_eq!(declarations.len(), 2);
    assert_eq!(members.len(), 5);
    for row in declarations {
        assert_eq!(row.selected_document.as_ref(), fixture::PACKAGE);
        assert!([fixture::id("Tier"), fixture::id("Odd")]
            .iter()
            .any(|identity| identity.as_str() == row.original_identity.as_ref()));
        let selected = emitted
            .emission
            .selected_model_enums(&admitted, 128)
            .expect("the selected enum declaration rows join");
        assert_eq!(
            selected.read_declaration(&admitted, &row.structural_node),
            Ok(row)
        );
    }
    for row in members {
        assert_eq!(row.selected_document.as_ref(), fixture::PACKAGE);
        let (owner, member) = match row.case.as_ref() {
            "Gold" if row.enum_original_identity == fixture::id("Tier") => {
                ("Tier", "variant/Tier-Gold")
            }
            "Standard" if row.enum_original_identity == fixture::id("Tier") => {
                ("Tier", "variant/Tier-Standard")
            }
            "2x" => ("Odd", "variant/Odd-2x"),
            "Café" => ("Odd", "variant/Odd-Caf"),
            "Gold Plus" => ("Odd", "variant/Odd-Gold-Plus"),
            other => panic!("unexpected selected model case {other:?}"),
        };
        assert_eq!(
            row.enum_original_identity.as_ref(),
            fixture::id(owner).as_str()
        );
        assert_eq!(
            row.member_original_identity.as_ref(),
            fixture::id(member).as_str()
        );
        let selected = emitted
            .emission
            .selected_model_enums(&admitted, 128)
            .expect("the selected enum member rows join");
        assert_eq!(
            selected.read_member(&admitted, &row.structural_node),
            Ok(row)
        );
    }

    let declaration = declarations[0].clone();
    let missing = SelectedModelEnums::from_stage_rows(
        &admitted,
        Vec::<SelectedModelEnumDeclarationRow>::new(),
        Vec::<SelectedModelEnumMemberRow>::new(),
        128,
    )
    .expect("an empty row index is valid until read");
    assert_eq!(
        missing
            .read_declaration(&admitted, &declaration.structural_node)
            .unwrap_err()
            .code,
        SelectedModelEnumErrorCode::UnavailableWitness
    );

    let mut mismatched = declaration.clone();
    mismatched.selected_document = "foreign/document".into();
    assert_eq!(
        SelectedModelEnums::from_stage_rows(
            &admitted,
            [mismatched],
            Vec::<SelectedModelEnumMemberRow>::new(),
            128,
        )
        .unwrap_err()
        .code,
        SelectedModelEnumErrorCode::MismatchedWitness
    );

    assert_eq!(
        SelectedModelEnums::from_stage_rows(
            &admitted,
            [declaration.clone(), declaration.clone()],
            Vec::<SelectedModelEnumMemberRow>::new(),
            128,
        )
        .unwrap_err()
        .code,
        SelectedModelEnumErrorCode::ConflictingWitness
    );

    let member = members[0].clone();
    let declaration = declarations
        .iter()
        .find(|row| row.structural_node == member.declaration_node)
        .expect("the selected member has its declaration row")
        .clone();
    let mut mismatched_member = member.clone();
    mismatched_member.case = "unknown-case".into();
    assert_eq!(
        SelectedModelEnums::from_stage_rows(
            &admitted,
            [declaration.clone()],
            [mismatched_member],
            128,
        )
        .unwrap_err()
        .code,
        SelectedModelEnumErrorCode::MismatchedWitness
    );
    assert_eq!(
        SelectedModelEnums::from_stage_rows(
            &admitted,
            [declaration.clone()],
            [member.clone(), member],
            128,
        )
        .unwrap_err()
        .code,
        SelectedModelEnumErrorCode::ConflictingWitness
    );

    let selected = emitted
        .emission
        .selected_model_enums(&admitted, 128)
        .expect("all rows join");
    let Read::Verified {
        admitted: foreign, ..
    } = emitted.read(emitted.emission.package.bytes())
    else {
        panic!("a second reader admits an independent package")
    };
    assert_eq!(
        selected
            .read_declaration(&foreign, &declaration.structural_node)
            .unwrap_err()
            .code,
        SelectedModelEnumErrorCode::ForeignPackage
    );
}

/// FR-360-AC-8: a call passing a variant value takes it by `ref`, no node
/// converts it, an equality over variant values is the enumeration equality
/// application, and `!=` negates it.
#[trace("FR-360-AC-8", "TC-1009")]
#[test]
fn a_variant_value_is_passed_by_ref_and_compared_by_the_enumeration_equality() {
    let emitted = emit_unit(UNIT);
    assert!(
        !emitted.wire.to_string().contains("convert"),
        "no conversion node anywhere"
    );
    let applied = |identity: &str| {
        nodes(&emitted.wire)
            .iter()
            .filter(|node| node["body"]["operation"]["identity"] == identity)
            .count()
    };
    assert_eq!(
        applied("quire.op.enum.eq"),
        3,
        "isGold, oddSame and the clause"
    );
    assert_eq!(applied("quire.op.enum.ne"), 1, "notGold");
}

/// FR-360-AC-8: a unit naming only a variant type with identifier cases emits
/// and reads back `Verified` (the minimal case of the test below).
#[trace("FR-360-AC-8", "TC-1009")]
#[test]
fn an_identifier_cased_variant_type_emits_and_reads_back_verified() {
    let emitted = emit_unit(TIER_UNIT);
    assert_eq!(
        node_where(&emitted.wire, "model", "variant_type").len(),
        1,
        "Tier is the one variant type the unit names"
    );
    assert!(matches!(
        emitted.read(emitted.emission.package.bytes()),
        Read::Verified { .. }
    ));
}

/// FR-360-AC-8, FR-360-AC-4: the package of a unit with a variant-typed
/// parameter, result, field read, clause and a type whose cases are not
/// identifiers reads back `Verified`.
#[trace("FR-360-AC-8", "FR-360-AC-4", "TC-1009", "TC-1007")]
#[test]
fn a_package_naming_variant_types_reads_back_verified() {
    let emitted = emit_unit(UNIT);
    match emitted.read(emitted.emission.package.bytes()) {
        Read::Verified { .. } => {}
        other => panic!("expected Verified, got {other:?}"),
    }
}

/// The package with `edit` applied to the wire of the first `Odd` case node
/// named `case`.
fn tampered_read(case: &str, edit: impl FnOnce(&mut Value, &Value)) -> Read {
    let emitted = emit_unit(UNIT);
    let tier = emitted.variant_node("Tier")["node_id"].clone();
    let mut wire = emitted.wire.clone();
    let node = wire["semantic_graph"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["semantic_form"] == "enum_value" && node["body"]["value"] == case)
        .expect("the case node is emitted");
    edit(node, &tier);
    emitted.read(&jcs(&wire))
}

/// A case of the owner's own node that the declaring variant type does not
/// hold is refused by IR's reader, as is a case moved to
/// another variant type's node.
#[trace("FR-360-AC-8", "TC-1009")]
#[test]
fn a_case_outside_its_variant_type_is_refused_by_the_reader() {
    let renamed = tampered_read("Gold Plus", |node, _| {
        node["body"]["value"] = json!("Silver");
    });
    assert!(matches!(renamed, Read::Refused(_)), "{renamed:?}");
    let moved = tampered_read("Gold Plus", |node, tier| {
        node["semantic_type"] = tier.clone();
        node["dependencies"] = json!([tier]);
        node["body"]["type"] = tier.clone();
    });
    assert!(matches!(moved, Read::Refused(_)), "{moved:?}");
}

/// An empty case name is refused by IR's reader.
#[trace("FR-360-AC-8", "TC-1009")]
#[test]
fn an_empty_case_name_is_refused_by_the_reader() {
    let emptied = tampered_read("2x", |node, _| {
        node["body"]["value"] = json!("");
    });
    assert!(matches!(emptied, Read::Refused(_)), "{emptied:?}");
}
