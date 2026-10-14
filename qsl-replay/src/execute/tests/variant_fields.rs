// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-360-AC-11 (TC-1010): a replay argument holding a variant value is
//! decoded by the checked node key of its variant type and the case name,
//! admitted at a slot of exactly that type, and encoded again as the enum
//! shape of FR-070 with the same type and case. Two units that select the
//! package under different aliases assign equal keys of both kinds.

use ix_trace_rs::trace;
use qsl_foundation::digest::WireNodeId;
use qsl_package::CheckedPackage;
use quire_exact::{EnumMember, Value, ValueType};
use quire_semantic_value::call::InputRefusal;
use quire_semantic_value::checking::CheckingLimits;

use super::{WitnessValue, UNLIMITED};
use crate::execute::argument::{convert_arguments, Stopped};
use crate::execute::ReplayRefusal;

#[path = "../../../../qsl-semantics/tests/support/variant_fields.rs"]
mod fixture;

/// The FR-360 package linked as model `alias`.
fn package_as(alias: &str) -> CheckedPackage {
    package_over(&fixture::document(), alias)
}

/// `document` linked as model `alias`.
fn package_over(document: &serde_json::Value, alias: &str) -> CheckedPackage {
    let bytes = serde_json::to_vec(document).expect("the fixture serializes");
    let packages = qsl_semantics::model::intake::package_input([bytes.as_slice()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one model package");
    };
    let digest = qsl_semantics::model::key::hex(digest);
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model {alias} = {package:?} version \"1\" digest \"sha256-jcs:{digest}\";\n\
         function keep using v(): Boolean pure {{ true }}\n",
        package = fixture::PACKAGE,
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "variant-fields", "fixture", "fixture:1"),
        "variant-fields.native",
        source.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the source unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the source unit");
    let selected = qsl_semantics::model::intake::admit_unit(
        &unit.selections().models,
        &packages,
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("I1 admits the fixture package");
    let declarations = qsl_semantics::check::PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        selected,
        Vec::new(),
    )
    .expect("assembly succeeds");
    CheckedPackage::link(
        declarations
            .check(CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("the unit checks: {refusals:?}")),
    )
}

fn declared<'a>(
    package: &'a CheckedPackage,
    alias: &str,
    name: &str,
) -> &'a qsl_semantics::check::VariantTypeDeclaration {
    let wanted = format!("{alias}::{name}");
    package
        .graph()
        .scope()
        .variant_types()
        .iter()
        .find(|declared| declared.name() == wanted)
        .unwrap_or_else(|| panic!("{wanted} is declared"))
}

fn witness(declaration: &qsl_semantics::check::VariantTypeDeclaration, case: &str) -> WitnessValue {
    WitnessValue::Enum {
        declaration: WireNodeId::from_digest(*declaration.node().as_bytes()),
        member: case.to_owned(),
    }
}

/// The one argument `value` converted at the parameter type `declared`.
fn converted(
    package: &CheckedPackage,
    value: WitnessValue,
    slot: ValueType,
) -> Result<Value, usize> {
    match convert_arguments(package, &[slot], &[value], &UNLIMITED) {
        Ok(mut converted) => Ok(converted.values.remove(0)),
        Err(Stopped::Refusal(refusal)) => match *refusal {
            ReplayRefusal::Input(InputRefusal::WrongValueKind { parameter }) => Err(parameter),
            other => panic!("a wrong-value-kind refusal, not {other:?}"),
        },
        Err(Stopped::Limit(limit)) => panic!("no limit applies: {limit:?}"),
    }
}

/// Whether two values are one value by the kernel's canonical key order.
fn same_value(left: &Value, right: &Value) -> bool {
    quire_exact::compare_keys(left, right) == Some(std::cmp::Ordering::Equal)
}

fn slot_of(declaration: &qsl_semantics::check::VariantTypeDeclaration) -> ValueType {
    ValueType::Enum(declaration.shape())
}

fn member_of(declaration: &qsl_semantics::check::VariantTypeDeclaration, case: &str) -> Value {
    let rank = declaration
        .cases()
        .iter()
        .position(|candidate| candidate.name() == case)
        .expect("the case is declared");
    Value::Enum(EnumMember::new(
        declaration.cases()[rank].variant(),
        u32::try_from(rank).unwrap(),
    ))
}

/// A witness enum value naming the variant type's checked node and a declared
/// case is admitted at a slot of that type and equals the case the unit's
/// literal has; another variant type's node, an undeclared case and a
/// letter-case variant of a case refuse `WrongValueKind` naming the
/// parameter.
#[trace("FR-360-AC-11", "TC-1010")]
#[test]
fn a_witness_enum_is_admitted_by_its_type_node_and_case_name() {
    let package = package_as("Config");
    let tier = declared(&package, "Config", "Tier");
    let level = declared(&package, "Config", "Level");
    for case in ["Gold", "Standard"] {
        let admitted = converted(&package, witness(tier, case), slot_of(tier))
            .unwrap_or_else(|parameter| panic!("{case} refused at parameter {parameter}"));
        assert!(same_value(&admitted, &member_of(tier, case)), "{case}");
    }
    assert_eq!(
        converted(&package, witness(level, "Gold"), slot_of(tier)).err(),
        Some(0),
        "another variant type's node"
    );
    assert_eq!(
        converted(&package, witness(tier, "Silver"), slot_of(tier)).err(),
        Some(0),
        "an undeclared case"
    );
    assert_eq!(
        converted(&package, witness(tier, "gold"), slot_of(tier)).err(),
        Some(0),
        "a letter-case variant of a case"
    );
}

/// Two units that select the package as `Config` and as `Cfg` assign equal
/// keys of both kinds, and a case admitted in one equals the same case
/// admitted in the other.
#[trace("FR-360-AC-11", "TC-1010")]
#[test]
fn a_case_admitted_under_two_aliases_is_one_value() {
    let config = package_as("Config");
    let cfg = package_as("Cfg");
    let in_config = declared(&config, "Config", "Tier");
    let in_cfg = declared(&cfg, "Cfg", "Tier");
    assert_eq!(in_config.key(), in_cfg.key());
    assert_eq!(in_config.node(), in_cfg.node());
    let left = converted(&config, witness(in_config, "Gold"), slot_of(in_config)).unwrap();
    let right = converted(&cfg, witness(in_cfg, "Gold"), slot_of(in_cfg)).unwrap();
    assert!(same_value(&left, &right));
}

/// A witness enum value naming a case that is not a source identifier is
/// admitted by the exact case name at a slot of its variant type, and equals
/// the case the type declares; a letter-case variant of the name refuses.
#[trace("FR-360-AC-11", "TC-1010")]
#[test]
fn a_witness_enum_names_a_non_identifier_case_by_its_exact_name() {
    const CASES: [&str; 3] = ["2x", "Café", "Gold Plus"];
    let package = package_over(
        &fixture::with_types(vec![fixture::variant_type("Odd", &CASES)]),
        "Config",
    );
    let odd = declared(&package, "Config", "Odd");
    for case in CASES {
        let admitted = converted(&package, witness(odd, case), slot_of(odd))
            .unwrap_or_else(|parameter| panic!("{case} refused at parameter {parameter}"));
        assert!(same_value(&admitted, &member_of(odd, case)), "{case}");
    }
    assert_eq!(
        converted(&package, witness(odd, "gold plus"), slot_of(odd)).err(),
        Some(0),
        "a letter-case variant of a case"
    );
}
