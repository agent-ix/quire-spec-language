// SPDX-License-Identifier: AGPL-3.0-or-later
//! The `Value` family's definition catalog and its package-selection
//! admission. The conformance tests compare them with QSpec's
//! `complete-value-lock.json` and its package-selection vectors, read at run
//! time from the quire-specification checkout `QSPEC_DIR` names (`make
//! conformance`); nothing of QSpec is copied into this repository.

use std::collections::BTreeSet;
use std::path::Path;

use ix_trace_rs::trace;
use qsl_semantics::value::{CatalogRole, DefinitionLock, HeaderLayer, LayerForm, Trigger};
use quire_semantic_value::definition::SelectionRefusalCode;
use serde_json::Value;

/// The QSpec definitions directory inside a quire-specification checkout.
const DEFINITIONS: &str = "proposals/quire-v1/definitions";

fn lock() -> DefinitionLock {
    DefinitionLock::pinned()
}

/// QSpec's definition document `name`, read from `$QSPEC_DIR`; `None` when
/// `QSPEC_DIR` is unset.
fn qspec_document(name: &str) -> Option<Value> {
    let qspec = std::env::var_os("QSPEC_DIR")?;
    let path = Path::new(&qspec).join(DEFINITIONS).join(name);
    let bytes =
        std::fs::read(&path).unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
    Some(
        serde_json::from_slice(&bytes)
            .unwrap_or_else(|error| panic!("parsing {}: {error}", path.display())),
    )
}

fn strings(value: &Value) -> Vec<&str> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item.as_str().unwrap())
        .collect()
}

/// Every role has exactly one catalog entry, which names its definition by
/// a nonempty authority and identity, and no two roles name one identity.
#[test]
fn the_catalog_covers_every_role_exactly_once() {
    let lock = lock();
    let roles: Vec<_> = lock.catalog().iter().map(|entry| entry.role).collect();
    assert_eq!(roles, CatalogRole::ALL);
    let identities: BTreeSet<_> = lock.catalog().iter().map(|entry| entry.identity).collect();
    assert_eq!(identities.len(), CatalogRole::ALL.len());
    for entry in lock.catalog() {
        assert_eq!(
            CatalogRole::from_code(entry.role.as_str()),
            Some(entry.role)
        );
        assert!(!entry.identity.is_empty(), "{:?}", entry.role);
        assert_eq!(entry.authority, "agent-ix", "{:?}", entry.role);
        assert_eq!(lock.entry(entry.role), entry);
        assert_eq!(
            serde_json::to_value(entry.reference()).unwrap(),
            serde_json::json!({"authority": entry.authority, "identity": entry.identity})
        );
    }
}

/// The catalog is QSpec's lock: every `qualification_catalog` row's role,
/// authority and identity, in order, every package-selection rule, the
/// trigger vocabulary and the selection refusal codes. Skipped (and
/// passing) when `QSPEC_DIR` is unset; `make conformance` requires it.
#[trace("TC-490", "FR-110-AC-9")]
#[test]
fn conformance_catalog_matches_qspec_complete_value_lock() {
    let Some(document) = qspec_document("complete-value-lock.json") else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let lock = lock();
    let rows: Vec<(&str, &str, &str)> = document["qualification_catalog"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                row["role"].as_str().unwrap(),
                row["definition"]["authority"].as_str().unwrap(),
                row["definition"]["identity"].as_str().unwrap(),
            )
        })
        .collect();
    let catalog: Vec<(&str, &str, &str)> = lock
        .catalog()
        .iter()
        .map(|entry| (entry.role.as_str(), entry.authority, entry.identity))
        .collect();
    assert_eq!(catalog, rows);

    let selection = &document["package_selection"];
    let always: Vec<&str> = lock.always_roles().iter().map(|r| r.as_str()).collect();
    assert_eq!(always, strings(&selection["always"]));
    let conditional: Vec<(&str, &str)> = lock
        .conditional_roles()
        .iter()
        .map(|(trigger, role)| (trigger.as_str(), role.as_str()))
        .collect();
    let expected: Vec<(&str, &str)> = selection["conditional"]
        .as_array()
        .unwrap()
        .iter()
        .map(|rule| {
            (
                rule["trigger"].as_str().unwrap(),
                rule["role"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(conditional, expected);
    let exactly_one: Vec<(&str, Vec<&str>)> = lock
        .exactly_one_roles()
        .iter()
        .map(|(trigger, roles)| (trigger.as_str(), roles.iter().map(|r| r.as_str()).collect()))
        .collect();
    let expected: Vec<(&str, Vec<&str>)> = selection["exactly_one"]
        .as_array()
        .unwrap()
        .iter()
        .map(|rule| (rule["trigger"].as_str().unwrap(), strings(&rule["roles"])))
        .collect();
    assert_eq!(exactly_one, expected);

    let vocabulary: BTreeSet<&str> = strings(&document["trigger_vocabulary"])
        .into_iter()
        .collect();
    assert_eq!(
        vocabulary,
        Trigger::ALL.into_iter().map(Trigger::as_str).collect()
    );
    let refusals: BTreeSet<&str> = strings(&document["selection_refusal_codes"])
        .into_iter()
        .collect();
    assert_eq!(
        refusals,
        SelectionRefusalCode::ALL
            .into_iter()
            .map(SelectionRefusalCode::as_str)
            .collect()
    );
    println!(
        "conformance: {} catalog rows match QSpec's lock",
        catalog.len()
    );
}

/// The vector's selected roles: the always roles less any omitted, then its
/// additional roles.
fn vector_roles<'a>(vectors: &'a Value, vector: &'a Value) -> Vec<&'a str> {
    let omitted = vector
        .get("omitted_always_roles")
        .map(strings)
        .unwrap_or_default();
    let mut roles: Vec<&str> = strings(&vectors["always_roles"])
        .into_iter()
        .filter(|role| !omitted.contains(role))
        .collect();
    roles.extend(strings(&vector["additional_roles"]));
    roles
}

/// Every accepted QSpec vector admits, with exactly its triggers and roles,
/// and a division law exactly when `integer_div_rem` is present. Every
/// refused vector, including the two ordering vectors, refuses with exactly
/// its `expected_code`, and the refused vectors together reach every closed
/// refusal code. Skipped (and passing) when `QSPEC_DIR` is unset; `make
/// conformance` requires it.
#[trace("QSpec-TC-192", "TC-490", "FR-110-AC-9")]
#[test]
fn conformance_admit_selection_matches_qspec_selection_vectors() {
    let Some(vectors) = qspec_document("complete-value-selection-vectors.json") else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let accepted = vectors["accepted"].as_array().unwrap();
    assert!(!accepted.is_empty());
    for vector in accepted {
        let name = &vector["name"];
        let triggers = strings(&vector["triggers"]);
        let roles = vector_roles(&vectors, vector);
        let admitted = lock()
            .admit_selection(&triggers, &roles)
            .unwrap_or_else(|code| panic!("{name}: {code:?}"));
        let expected: BTreeSet<Trigger> = triggers
            .iter()
            .map(|code| Trigger::from_code(code).unwrap())
            .collect();
        assert_eq!(admitted.triggers(), &expected, "{name}");
        let roles: BTreeSet<CatalogRole> = roles
            .iter()
            .map(|code| CatalogRole::from_code(code).unwrap())
            .collect();
        assert_eq!(admitted.roles(), &roles, "{name}");
        assert_eq!(
            admitted.division_profile().is_some(),
            expected.contains(&Trigger::IntegerDivRem),
            "{name}"
        );
    }

    let refused = vectors["refused"].as_array().unwrap();
    let mut codes = BTreeSet::new();
    for vector in refused {
        let name = &vector["name"];
        let expected = SelectionRefusalCode::from_code(vector["expected_code"].as_str().unwrap())
            .unwrap_or_else(|| panic!("{name}: unknown expected_code"));
        let code = lock()
            .admit_selection(
                &strings(&vector["triggers"]),
                &vector_roles(&vectors, vector),
            )
            .expect_err(&format!("{name} admitted"));
        assert_eq!(code, expected, "{name}");
        codes.insert(code);
    }
    assert_eq!(codes, SelectionRefusalCode::ALL.into_iter().collect());
    println!(
        "conformance: {} accepted and {} refused selection vectors",
        accepted.len(),
        refused.len()
    );
}

fn layer(identity: &str) -> HeaderLayer {
    HeaderLayer::from_identity(identity).unwrap_or_else(|| panic!("no layer {identity}"))
}

/// FR-110-AC-7: a selected layer's closure holds the layer and every layer
/// its `requires` edges name, transitively, in the lock's row order, and no
/// layer that requires it.
#[trace("TC-490", "FR-110-AC-7")]
#[test]
fn the_requires_closure_of_each_layer_is_computed() {
    use HeaderLayer::{ModelComplete, StateCore, StateGraph, StateQueries, ValueComplete};
    let lock = lock();
    let expected: [(HeaderLayer, &[HeaderLayer]); 5] = [
        (StateCore, &[StateCore]),
        (StateQueries, &[StateCore, StateQueries]),
        (StateGraph, &[StateCore, StateQueries, StateGraph]),
        (ValueComplete, &[StateCore, ValueComplete]),
        (
            ModelComplete,
            &[
                StateCore,
                StateQueries,
                StateGraph,
                ValueComplete,
                ModelComplete,
            ],
        ),
    ];
    for (selected, closure) in expected {
        assert_eq!(lock.closure(&[selected]), closure, "{selected:?}");
    }
    // A union of selections is the union of their closures.
    assert_eq!(
        lock.closure(&[StateQueries, ValueComplete]),
        [StateCore, StateQueries, ValueComplete]
    );
    assert_eq!(lock.closure(&[]), Vec::<HeaderLayer>::new());
}

/// FR-110-AC-8: the admitted-form set of a layer is the forms of its closure.
/// State core holds no named predicate, state queries do, and no layer
/// admits a form that only a wider layer adds.
#[trace("TC-490", "FR-110-AC-8")]
#[test]
fn a_layer_admits_the_forms_of_its_closure_and_no_wider_layer() {
    use HeaderLayer::{StateCore, StateGraph, StateQueries, ValueComplete};
    use LayerForm::{
        FunctionDeclaration, NamedPredicateCall, ObjectNavigation, PredicateDeclaration,
        SequenceQuery, StateClause, ValueConstruct,
    };
    let lock = lock();
    assert!(lock.admits(StateCore, StateClause));
    for form in [
        PredicateDeclaration,
        NamedPredicateCall,
        SequenceQuery,
        ObjectNavigation,
        FunctionDeclaration,
        ValueConstruct,
    ] {
        assert!(!lock.admits(StateCore, form), "state core admits {form:?}");
    }
    for form in [
        StateClause,
        PredicateDeclaration,
        NamedPredicateCall,
        SequenceQuery,
    ] {
        assert!(lock.admits(StateQueries, form), "queries refuses {form:?}");
    }
    for form in [ObjectNavigation, FunctionDeclaration, ValueConstruct] {
        assert!(!lock.admits(StateQueries, form), "queries admits {form:?}");
    }
    assert!(lock.admits(StateGraph, ObjectNavigation));
    assert!(!lock.admits(StateGraph, FunctionDeclaration));
    assert!(lock.admits(ValueComplete, FunctionDeclaration));
    assert!(lock.admits(ValueComplete, ValueConstruct));
}

/// FR-110-AC-1: the qualification rows a package selects follow the lock's
/// `package_selection.layer`: the `always` rules when the layers hold
/// `quire.value.complete/v1`, and the `without_layer` rows otherwise.
#[trace("TC-490", "FR-110-AC-1")]
#[test]
fn the_qualification_rows_follow_the_value_complete_layer() {
    let lock = lock();
    assert_eq!(
        lock.qualification_roles(&lock.closure(&[HeaderLayer::ValueComplete])),
        lock.always_roles()
    );
    assert_eq!(
        lock.qualification_roles(&lock.closure(&[HeaderLayer::StateGraph])),
        [CatalogRole::Edition]
    );
}

/// The layer rows are QSpec's `header_selectable_layers`: each row's
/// identity, `requires`, admitted-form file and witness request, in order,
/// and the lock's `package_selection.layer` and `without_layer`. Skipped
/// (and passing) when `QSPEC_DIR` is unset; `make conformance` requires it.
#[trace("TC-490", "FR-110-AC-9")]
#[test]
fn conformance_layer_rows_match_qspec_complete_value_lock() {
    let Some(document) = qspec_document("complete-value-lock.json") else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let lock = lock();
    let rows = document["header_selectable_layers"].as_array().unwrap();
    assert_eq!(rows.len(), lock.layers().len());
    for (row, ours) in rows.iter().zip(lock.layers()) {
        assert_eq!(row["identity"], ours.layer.identity());
        let requires: Vec<&str> = ours.requires.iter().map(|l| l.identity()).collect();
        assert_eq!(strings(&row["requires"]), requires);
        assert_eq!(row["admitted_forms"], ours.admitted_forms);
        assert_eq!(
            row["witness_request"]["kind"],
            ours.witness_request.kind.as_str()
        );
        assert_eq!(
            row["witness_request"]["extent"],
            ours.witness_request.extent
        );
        assert_eq!(layer(ours.layer.identity()), ours.layer);
    }
    let selection = &document["package_selection"];
    assert_eq!(
        layer(selection["layer"].as_str().unwrap()),
        HeaderLayer::ValueComplete
    );
    let without: Vec<&str> = lock
        .qualification_roles(&[])
        .iter()
        .map(|role| role.as_str())
        .collect();
    assert_eq!(without, strings(&selection["without_layer"]));
}
