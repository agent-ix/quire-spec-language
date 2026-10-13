// SPDX-License-Identifier: AGPL-3.0-or-later
//! The FR-360 domain package: variant types and the members typed by them,
//! shared by the QSL semantics, evaluator and replay tests (TC-1006 to
//! TC-1010).
//!
//! - Variant types `Tier` (`Gold`, `Standard`), `Level` (`Gold`) and `Grade`
//!   (`Standard`, `Gold`, `Bronze`, declared in that order).
//! - The record value type `Settings { tier: Tier; size: Integer }`.
//! - The object type `Account` with a required `tier`, an optional `backup`
//!   and an ordered non-unique `[0, 3]` `history` of `Tier`, and the
//!   operations `promote(t: Tier): Tier` and `rank(l: Level): Level`.
//! - The state machine `Lifecycle` and the process `Fulfilment`, each with a
//!   `tier: Tier` field.

#![allow(dead_code, reason = "each test binary uses part of the fixture")]

use qsl_semantics::model::intake::meaning;
use serde_json::{json, Value};

#[path = "tc235_k2.rs"]
#[allow(dead_code, reason = "this fixture uses part of the shared K2 builder")]
#[allow(
    clippy::duplicate_mod,
    reason = "a test binary also loads the K2 builder directly"
)]
mod base;
pub(crate) use base::{id, PACKAGE};

/// A variant type's source position, which the reader requires of every
/// case.
fn sourced(line: u64) -> Value {
    json!({"source": {
        "sourceIdentity": id("spec"),
        "path": "spec.qspec",
        "startLine": line,
        "startColumn": 3,
    }})
}

/// The slug the lift mints a case identity from: every run of characters
/// that are not ASCII alphanumeric becomes one `-`, with none leading or
/// trailing.
fn slug(name: &str) -> String {
    let mut slug = String::new();
    let mut separated = false;
    for character in name.chars() {
        if character.is_ascii_alphanumeric() {
            if separated && !slug.is_empty() {
                slug.push('-');
            }
            separated = false;
            slug.push(character);
        } else {
            separated = true;
        }
    }
    slug
}

/// A case with the identity the lift mints for it.
fn case(owner: &str, name: &str, line: u64) -> Value {
    let label = slug(name);
    json!({
        "identity": id(&format!("variant/{owner}-{label}")),
        "name": name,
        "origin": sourced(line),
    })
}

/// The variant type `name` with `cases`, in the order given.
pub(crate) fn variant_type(name: &str, cases: &[&str]) -> Value {
    let cases: Vec<Value> = cases
        .iter()
        .enumerate()
        .map(|(position, case_name)| case(name, case_name, 10 + position as u64))
        .collect();
    base::declaration(name, "tier", json!({"variants": cases}))
}

fn optional(owner: &str, name: &str, type_ref: &str) -> Value {
    let mut field = base::field(owner, name, type_ref);
    field["presence"] = json!("optional");
    field
}

fn many(owner: &str, name: &str, type_ref: &str, ordered: bool, unique: bool) -> Value {
    let mut field = base::field(owner, name, type_ref);
    field["multiplicity"] = json!({"lower": 0, "upper": 3, "ordered": ordered, "unique": unique});
    field
}

/// An operation `owner/name` taking `parameters` (name, type) and returning
/// `result`.
fn operation_with(owner: &str, name: &str, parameters: &[(&str, String)], result: &str) -> Value {
    let node = format!("{owner}/{name}");
    let params: Vec<Value> = parameters
        .iter()
        .map(|(parameter, type_ref)| {
            let identity = format!("{node}/{parameter}");
            json!({
                "identity": id(&identity), "name": parameter, "typeRef": type_ref,
                "presence": "required", "nullable": false, "defaultKind": "none",
                "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
                "origin": base::origin(&identity),
            })
        })
        .collect();
    json!({
        "identity": id(&node), "name": name, "params": params,
        "returns": {
            "typeRef": result,
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "nullable": false,
        },
        "pre": [], "post": [],
        "frame": {"modifies": [], "creates": [], "deletes": []},
        "origin": base::origin(&node),
    })
}

fn variant_construct() -> Value {
    json!({
        "kind": {"module": base::BUSINESS, "name": "tier"},
        "moduleVersion": "1.0.0",
        "construct": {
            "identity": "none", "shape": "enumeration",
            "members": {"variants": "required"}, "meaning": meaning::VARIANT_TYPE,
        },
    })
}

/// `types` and the constructs the FR-360 fixture needs, as a package
/// document.
fn package(types: Vec<Value>, populated: bool) -> Value {
    // The schema refuses a construct no type is of.
    let used = |kind: &str| types.iter().any(|ty| ty["kind"]["name"] == kind);
    let constructs = [
        ("tier", variant_construct()),
        ("thing", base::construct("thing", meaning::OBJECT_TYPE)),
        (
            "money",
            base::construct("money", meaning::RECORD_VALUE_TYPE),
        ),
        (
            "lifecycle",
            base::construct("lifecycle", meaning::STATE_MACHINE),
        ),
        ("saga", base::construct("saga", meaning::PROCESS)),
        ("value", base::construct("value", meaning::VALUE_TYPE)),
        ("happened", base::construct("happened", meaning::EVENT_TYPE)),
    ]
    .into_iter()
    .filter(|(kind, _)| used(kind))
    .map(|(_, construct)| construct)
    .chain(populated.then(|| base::construct("population", meaning::POPULATION)))
    .collect();
    base::domain_document(types, constructs)
}

fn state(name: &str) -> Value {
    let node = format!("Lifecycle/{name}");
    json!({"identity": id(&node), "name": name, "origin": base::origin(&node)})
}

/// The types of the FR-360 package.
pub(crate) fn types() -> Vec<Value> {
    let tier = id("Tier");
    let level = id("Level");
    vec![
        variant_type("Tier", &["Gold", "Standard"]),
        variant_type("Level", &["Gold"]),
        variant_type("Grade", &["Standard", "Gold", "Bronze"]),
        base::declaration(
            "Settings",
            "money",
            json!({
                "fields": [
                    base::field("Settings", "tier", &tier),
                    base::field("Settings", "size", "ix://quire/native/Integer"),
                ],
            }),
        ),
        {
            let mut account = base::declaration(
                "Account",
                "thing",
                json!({
                    "identityFields": [id("Account/account_id")],
                    "fields": [
                        base::field("Account", "account_id", "ix://quire/native/Integer"),
                        base::field("Account", "tier", &tier),
                        optional("Account", "backup", &tier),
                        many("Account", "history", &tier, true, false),
                    ],
                }),
            );
            account["operations"] = json!([
                operation_with("Account", "promote", &[("t", tier.clone())], &tier),
                operation_with("Account", "rank", &[("l", level.clone())], &level),
            ]);
            account
        },
        base::declaration(
            "Lifecycle",
            "lifecycle",
            json!({
                "fields": [base::field("Lifecycle", "tier", &tier)],
                "operations": [base::operation("Lifecycle", "advance")],
                "clauses": [],
                "states": [state("Open"), state("Closed")],
                "transitions": [{
                    "identity": id("Lifecycle/step"),
                    "from": id("Lifecycle/Open"), "to": id("Lifecycle/Closed"),
                    "trigger": id("Lifecycle/advance"), "emits": [],
                    "origin": base::origin("Lifecycle/step"),
                }],
            }),
        ),
        base::declaration(
            "Fulfilment",
            "saga",
            json!({
                "identityFields": [id("Fulfilment/correlation_id")],
                "fields": [
                    base::field("Fulfilment", "correlation_id", "ix://quire/native/Integer"),
                    base::field("Fulfilment", "tier", &tier),
                ],
                "operations": [],
                "steps": [{
                    "identity": id("Fulfilment/receive"), "name": "receive",
                    "stepKind": "wait", "consumes": [], "emits": [],
                    "origin": base::origin("Fulfilment/receive"),
                }],
            }),
        ),
    ]
}

/// The FR-360 package document.
pub(crate) fn document() -> Value {
    let mut document = package(types(), true);
    document["populations"] = document_populations();
    document
}

/// The package of [`document`] with only its variant types, so no member
/// names them.
pub(crate) fn variants_only() -> Value {
    package(
        types()
            .into_iter()
            .filter(|ty| ty["kind"]["name"] == "tier")
            .collect(),
        false,
    )
}

/// [`document`] with the variant type `Odd`, holding `cases`, and an optional
/// `grade: Odd` field on `Account`.
pub(crate) fn with_odd_grade(cases: &[&str]) -> Value {
    let mut all = types();
    all.push(variant_type("Odd", cases));
    let account = all
        .iter_mut()
        .find(|declaration| declaration["displayName"] == "Account")
        .expect("the fixture declares Account");
    account["fields"]
        .as_array_mut()
        .expect("Account has fields")
        .push(optional("Account", "grade", &id("Odd")));
    let mut document = package(all, true);
    document["populations"] = document_populations();
    document
}

/// [`document`] with `extra` types appended.
pub(crate) fn with_types(extra: Vec<Value>) -> Value {
    let mut all = types();
    all.extend(extra);
    let mut document = package(all, true);
    document["populations"] = document_populations();
    document
}

fn document_populations() -> Value {
    json!([{
        "identity": id("accounts"), "displayName": id("accounts"),
        "kind": {"module": base::BUSINESS, "name": "population"},
        "members": [id("Account")], "extent": "closed",
        "origin": base::origin("accounts"),
    }])
}

/// The value type `Instant`, a native Timestamp.
pub(crate) fn instant() -> Value {
    let mut instant = base::declaration("Instant", "value", json!({}));
    instant["target"] = json!("ix://quire/native/Timestamp");
    instant
}

/// An event type `name` whose occurrence field is `occurrence`, typed
/// `occurrence_type`, with the further fields `fields`.
pub(crate) fn event(
    name: &str,
    occurrence: &str,
    occurrence_type: &str,
    mut fields: Vec<Value>,
) -> Value {
    fields.insert(0, base::field(name, occurrence, occurrence_type));
    base::declaration(
        name,
        "happened",
        json!({
            "supertypes": [],
            "occurrenceField": id(&format!("{name}/{occurrence}")),
            "fields": fields,
        }),
    )
}

/// A field `owner/name` of `type_ref`.
pub(crate) fn field(owner: &str, name: &str, type_ref: &str) -> Value {
    base::field(owner, name, type_ref)
}
