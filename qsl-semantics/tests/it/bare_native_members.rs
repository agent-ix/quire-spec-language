// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-056-AC-43 / FR-505-AC-6 (TC-897 step 8, TC-1300): bare native Text,
//! Rational and Decimal members are admitted with no checked type and refused
//! where used.

use ix_trace_rs::trace;
use qsl_semantics::check::CheckCause;
use quire_semantic_value::checking::CheckingLimits;
use serde_json::{json, Value};

use crate::model_operations::{
    config_version_document, empty_frame, operation, operation_parameter, wire_field,
    PACKAGE_IDENTITY,
};

fn identity(path: &str) -> String {
    format!("ix://{PACKAGE_IDENTITY}/{path}")
}

/// `ConfigVersion` as FR-056-AC-43's `Note`: `memo`, `ratio`, `amount`,
/// `amounts` (`[0, 3]`) and `count`, the operations `scale(by): Integer`,
/// `peek(): Text` and `touch()` modifying `memo`.
fn note_document(rational: bool) -> Vec<u8> {
    note_document_with(rational, Vec::new(), &[])
}

/// [`note_document`] also declaring `types` and the required fields `fields`
/// (`(name, typeRef)`).
fn note_document_with(rational: bool, types: Vec<Value>, fields: &[(&str, &str)]) -> Vec<u8> {
    // The validator pinned at I1 (`agent-ix-semantic-ir`) names no native
    // `Rational`: the fixture then stands a `Decimal` in its place.
    let ratio = if rational { "Rational" } else { "Decimal" };
    let natives = [
        ("memo", "Text", 1, 1),
        ("ratio", ratio, 1, 1),
        ("amount", "Decimal", 1, 1),
        ("amounts", "Decimal", 0, 3),
        ("count", "Integer", 1, 1),
    ];
    let base = config_version_document(
        operation("unused", json!([]), None, empty_frame()),
        Vec::new(),
        types,
        json!([]),
    );
    let mut value: Value = serde_json::from_slice(&base).expect("fixture JSON");
    let owner = value["types"]
        .as_array_mut()
        .expect("types")
        .iter_mut()
        .find(|ty| ty["identity"] == identity("ConfigVersion"))
        .expect("ConfigVersion");
    for (name, native, lower, upper) in natives {
        let mut field = wire_field(
            &identity(&format!("ConfigVersion/{name}")),
            name,
            &format!("ix://quire/native/{native}"),
            "required",
            lower,
        );
        field["multiplicity"]["upper"] = json!(upper);
        if native == "Decimal" {
            field["decimal"] = json!({"precision": 5, "scale": 2});
        }
        owner["fields"].as_array_mut().expect("fields").push(field);
    }
    for (name, type_ref) in fields {
        let field = wire_field(
            &identity(&format!("ConfigVersion/{name}")),
            name,
            type_ref,
            "required",
            1,
        );
        owner["fields"].as_array_mut().expect("fields").push(field);
    }
    let mut by = operation_parameter("scale", "by", &format!("ix://quire/native/{ratio}"));
    if !rational {
        by["decimal"] = json!({"precision": 5, "scale": 2});
    }
    let mut scale = operation(
        "scale",
        json!([by]),
        Some("ix://quire/native/Integer"),
        empty_frame(),
    );
    scale["pre"] = json!([]);
    let peek = operation(
        "peek",
        json!([]),
        Some("ix://quire/native/Text"),
        empty_frame(),
    );
    let touch = operation(
        "touch",
        json!([]),
        None,
        json!({"modifies": [identity("ConfigVersion/memo")], "creates": [], "deletes": []}),
    );
    let operations = owner["operations"].as_array_mut().expect("operations");
    // `adjust(qty)`: a clause can name `qty`, which `by` (a keyword) cannot.
    let mut qty = operation_parameter("adjust", "qty", &format!("ix://quire/native/{ratio}"));
    if !rational {
        qty["decimal"] = json!({"precision": 5, "scale": 2});
    }
    let adjust = operation("adjust", json!([qty]), None, empty_frame());
    operations.extend([scale, peek, touch, adjust]);
    serde_json::to_vec(&value).expect("fixture encoding")
}

const HEADER: &str = "profile d = \"quire.model.dispatch.single/v1\";\n";

fn assembled(
    document: &[u8],
    clauses: &str,
) -> Result<qsl_semantics::check::CheckedGraph, Vec<qsl_semantics::check::CheckRefusal>> {
    let declarations = crate::model_operations::admit_and_assemble_with_profiles(
        document,
        HEADER,
        &format!("function keep using v(): Boolean pure {{ true }}\n{clauses}"),
    )
    .unwrap_or_else(|refusal| panic!("assembly admits: {refusal:?}"));
    declarations.check(
        CheckingLimits::default(),
        quire_semantic_value::checking::CheckMode::Linked,
    )
}

/// The refusals of `clauses` as (code, cause, owner, member, locus).
fn untyped(
    document: &[u8],
    clauses: &str,
) -> Vec<(String, Option<String>, String, String, String)> {
    let Err(refusals) = assembled(document, clauses) else {
        return Vec::new();
    };
    refusals
        .iter()
        .map(|refusal| {
            let inner = match &refusal.cause {
                CheckCause::StateModel(state) => &state.cause,
                other => other,
            };
            let CheckCause::UntypedMember(member) = inner else {
                panic!("an untyped-member refusal, not {refusal:?}");
            };
            (
                refusal.cause.code().as_str().to_owned(),
                refusal.cause.cause().map(str::to_owned),
                member.owner.clone(),
                member.member.clone(),
                member.locus.clone(),
            )
        })
        .collect()
}

fn expect(
    owner: &str,
    member: &str,
    locus: &str,
) -> Vec<(String, Option<String>, String, String, String)> {
    vec![(
        "ill_typed".to_owned(),
        Some("operator-ineligible".to_owned()),
        owner.to_owned(),
        member.to_owned(),
        locus.to_owned(),
    )]
}

fn invariant(clause: &str) -> String {
    format!("invariant I using v on Config::ConfigVersion at current {{ {clause} }}\n")
}

/// FR-056-AC-43 (TC-897 step 8): the package with the bare native members is
/// admitted, assembled and checked with no clause naming them.
#[trace("TC-897")]
#[test]
fn a_package_of_bare_native_members_is_admitted_by_a_unit_naming_none() {
    assert!(assembled(&note_document(false), "").is_ok());
}

/// FR-056-AC-43 (TC-897 step 8): each use of a bare native field refuses
/// `ill_typed`/`operator-ineligible` at the projection, naming the owner and
/// `Field.typeRef`; `count` is still checked.
#[trace("TC-897")]
#[test]
fn a_clause_using_a_bare_native_field_refuses_at_the_projection() {
    let document = note_document(false);
    for member in ["memo", "ratio", "amount", "amounts"] {
        assert_eq!(
            untyped(
                &document,
                &invariant(&format!("self.{member} = self.{member}"))
            ),
            expect("ConfigVersion", member, "Field.typeRef"),
            "{member}"
        );
    }
    assert!(assembled(&document, &invariant("self.count = self.count")).is_ok());
}

/// FR-056-AC-43 (TC-897 step 8): a call of `scale` refuses at the call
/// naming `scale` and `Operation.params[0].typeRef` (its argument, `count`, is
/// not refused), a call of `peek` at the call naming `Operation.returns.typeRef`.
#[trace("TC-897")]
#[test]
fn a_call_of_an_operation_with_a_bare_native_refuses_at_the_call() {
    let document = note_document(false);
    assert_eq!(
        untyped(&document, &invariant("self.scale(self.count) = 1")),
        expect("ConfigVersion", "scale", "Operation.params[0].typeRef")
    );
    assert_eq!(
        untyped(&document, &invariant("self.peek() = self.peek()")),
        expect("ConfigVersion", "peek", "Operation.returns.typeRef")
    );
}

/// FR-056-AC-43 (TC-897 step 8): `touch()` modifying `memo` with the
/// postcondition `self.memo = pre(self.memo)` refuses at the post-state
/// projection; with no postcondition it is admitted.
#[trace("TC-897")]
#[test]
fn a_postcondition_over_a_bare_native_field_refuses_and_none_is_admitted() {
    let document = note_document(false);
    assert_eq!(
        untyped(
            &document,
            "post TouchPost using v on Config::ConfigVersion::touch { self.memo = pre(self.memo) }\n"
        ),
        expect("ConfigVersion", "memo", "Field.typeRef")
    );
    assert!(assembled(&document, "").is_ok());
}

fn rational_value_type(name: &str, constraints: Vec<Value>) -> Value {
    crate::model_operations::wire_type(
        &identity(name),
        "value_type",
        json!({"scalar": "rational", "constraints": constraints}),
    )
}

/// FR-505-AC-6 (TC-1300): a field typed a Rational value type with no
/// constraints is admitted and its use in a clause refuses
/// `ill_typed`/`operator-ineligible` at the projection, as FR-056-AC-43 states.
#[trace("TC-1300")]
#[test]
fn an_unconstrained_rational_value_type_field_admits_and_refuses_where_used() {
    let raw = identity("Raw");
    let document = note_document_with(
        false,
        vec![rational_value_type("Raw", Vec::new())],
        &[("raw", &raw)],
    );
    assert!(assembled(&document, "").is_ok());
    assert_eq!(
        untyped(&document, &invariant("self.raw = self.raw")),
        expect("ConfigVersion", "raw", "Field.typeRef")
    );
}

/// FR-505-AC-6 (TC-1300): adding a bounded `Ratio` to the package, typing a
/// field by it, leaves every other declaration's node identity unchanged:
/// the object types' keys and every other scalar's resolved type.
#[trace("TC-1300")]
#[test]
fn adding_a_ratio_leaves_every_other_declaration_identity_unchanged() {
    let raw = identity("Raw");
    let ratio = identity("Ratio");
    let keys = |document: &[u8]| {
        let declarations = crate::model_operations::admit_and_assemble_with_profiles(
            document,
            HEADER,
            "function keep using v(): Boolean pure { true }",
        )
        .unwrap_or_else(|refusal| panic!("assembly admits: {refusal:?}"));
        let objects = declarations
            .types
            .object_types()
            .map(|declaration| (declaration.name().to_owned(), declaration.key()))
            .collect::<Vec<_>>();
        let scalars = declarations
            .aliases
            .iter()
            .filter(|(name, _)| !name.ends_with("::Ratio"))
            .cloned()
            .collect::<Vec<_>>();
        let has_ratio = declarations
            .aliases
            .iter()
            .any(|(name, _)| name.ends_with("::Ratio"));
        (objects, scalars, has_ratio)
    };
    let without = note_document_with(
        false,
        vec![rational_value_type("Raw", Vec::new())],
        &[("raw", &raw)],
    );
    let bounded = |keyword: &str, value: i64| {
        json!({
            "identity": identity(&format!("Ratio/{keyword}")), "keyword": keyword,
            "operands": {"value": value}, "appliesTo": ratio,
            "diagnosticCode": format!("rational.{keyword}"),
            "origin": {"generated": {
                "generatorIdentity": identity("Ratio"), "generatorVersion": "1.0.0",
                "inputIdentities": [identity("Ratio")]}},
        })
    };
    let with = note_document_with(
        false,
        vec![
            rational_value_type("Raw", Vec::new()),
            rational_value_type(
                "Ratio",
                vec![
                    bounded("minNumerator", -3),
                    bounded("maxNumerator", 7),
                    bounded("minDenominator", 1),
                    bounded("maxDenominator", 12),
                ],
            ),
        ],
        &[("raw", &raw), ("ratioed", &ratio)],
    );
    let (objects_without, scalars_without, ratio_without) = keys(&without);
    let (objects_with, scalars_with, ratio_with) = keys(&with);
    assert!(!ratio_without && ratio_with, "only `with` declares Ratio");
    assert!(!objects_without.is_empty() && !scalars_without.is_empty());
    assert_eq!(objects_without, objects_with);
    assert_eq!(scalars_without, scalars_with);
}

/// FR-056-AC-43 (TC-897 step 8): a clause naming a parameter of its operation
/// that has no checked type reaches an untyped member: it refuses
/// `ill_typed`/`operator-ineligible` naming the operation and
/// `Operation.params[0].typeRef`, not a missing name; a clause over an
/// operation whose untyped slot is only its result is admitted.
#[trace("TC-897")]
#[test]
fn a_clause_naming_an_untyped_parameter_refuses_operator_ineligible() {
    let document = note_document(false);
    assert_eq!(
        untyped(
            &document,
            "pre P using v on Config::ConfigVersion::adjust { qty = qty }\n"
        ),
        expect("ConfigVersion", "adjust", "Operation.params[0].typeRef")
    );
    // A postcondition and a conjunction naming it refuse the same way.
    for clause in [
        "post P using v on Config::ConfigVersion::adjust { qty = qty }\n",
        "pre P using v on Config::ConfigVersion::adjust { true and qty = qty }\n",
    ] {
        assert_eq!(
            untyped(&document, clause),
            expect("ConfigVersion", "adjust", "Operation.params[0].typeRef"),
            "{clause}"
        );
    }
    assert!(assembled(
        &document,
        "post P using v on Config::ConfigVersion::peek { true }\n"
    )
    .is_ok());
}
