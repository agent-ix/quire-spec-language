// SPDX-License-Identifier: AGPL-3.0-or-later
//! The checker owns admission of authored enum type names against its package scope.

use qsl_forms::{BuiltinType, Expression, FunctionDeclaration, TypeForm};
use qsl_foundation::{Code, IdentityLimits, Span};
use qsl_semantics::check::{fixture_source, CheckCause, EnumBinding, PackageDeclarations};
use qsl_semantics::value::enumeration::{
    AdmittedEnumDeclaration, EnumDeclarationPreimage, EnumMemberPreimage,
};
use qsl_semantics::value::{NodeIdentityPreimage, NodeOwner, OwnerSelection, OwnerSubject};
use quire_exact::{NodeKey, NODE_KEY_DOMAIN};
use quire_semantic_value::checking::CheckingLimits;

fn admitted_status() -> EnumBinding {
    let identity = IdentityLimits::default();
    let owners = OwnerSelection::new([NodeOwner::Definition(OwnerSubject {
        authority: "agent-ix".to_owned(),
        identity: "enum-check".to_owned(),
    })]);
    let preimage = EnumDeclarationPreimage::from_json(serde_json::json!({
        "version": "quire.enum-declaration-node/v1",
        "owner": {"kind": "definition", "authority": "agent-ix", "identity": "enum-check"},
        "qualified_declaration": ["Status"],
        "ordered": false,
        "members": ["READY"],
    }))
    .expect("valid enum declaration preimage");
    let key = NodeKey::from_digest(preimage.digest(identity).expect("identity encodes"));
    let declaration =
        AdmittedEnumDeclaration::admit(preimage, key, &owners, identity).expect("Status admits");
    let member = EnumMemberPreimage::from_json(serde_json::json!({
        "version": "quire.enum-member-node/v1",
        "declaration_node_id": {
            "domain": NODE_KEY_DOMAIN,
            "digest": declaration.key().to_string(),
        },
        "case": "READY",
    }))
    .expect("valid enum member preimage");
    let key = NodeKey::from_digest(member.digest(identity).expect("member identity encodes"));
    let ready = declaration
        .admit_member(&member, key, identity)
        .expect("READY admits");
    EnumBinding {
        name: "Status".to_owned(),
        declaration,
        members: vec![ready],
    }
}

fn package_with_status(parameter_type: &str, status: EnumBinding) -> PackageDeclarations {
    let span = Span { start: 0, end: 0 };
    let function = FunctionDeclaration::new(
        "reads_status",
        vec![("status".to_owned(), TypeForm::name(parameter_type, span))],
        TypeForm::builtin(BuiltinType::Boolean, span),
        None,
        Expression::boolean(true),
    );
    PackageDeclarations {
        enums: vec![status],
        functions: vec![function],
        ..PackageDeclarations::new(fixture_source(), IdentityLimits::default())
    }
}

/// RT TC-024 procedure 3: the QSL checker owns enum type-name admission.
#[test]
fn an_absent_enum_type_name_is_a_typed_missing_name() {
    let status = admitted_status();
    package_with_status("Status", status.clone())
        .check(CheckingLimits::default())
        .expect("an admitted enum type name resolves");

    let refusals = package_with_status("AbsentStatus", status)
        .check(CheckingLimits::default())
        .expect_err("an undeclared enum type name must be refused");
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert!(matches!(
        &refusals[0].cause,
        CheckCause::MissingName(name) if name == "AbsentStatus"
    ));
    assert_eq!(refusals[0].cause.code(), Code::MissingDeclaration);
}
