// SPDX-License-Identifier: AGPL-3.0-or-later
//! The `Value` family's local definition admission and selection-vector
//! behavior. The tests that need QSpec vectors read those vectors from the
//! checkout named by `QSPEC_DIR`; role identities are resolved by closed
//! typed enums, without a copied catalog.

use std::collections::BTreeSet;
use std::path::Path;

use ix_trace_rs::trace;
use qsl_semantics::value::{
    admit_selection, always_roles, qualification_roles, DefinitionRole, HeaderLayer, LayerForm,
    Trigger,
};
use quire_semantic_value::definition::SelectionRefusalCode;
use serde_json::Value;

/// The QSpec definitions directory inside a quire-specification checkout.
const DEFINITIONS: &str = "proposals/quire-v1/definitions";

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

/// The role enum is the single closed mapping from role names to identities.
#[test]
fn the_closed_roles_resolve_to_distinct_identity_references() {
    let roles = DefinitionRole::ALL;
    let identities: BTreeSet<_> = roles.iter().map(|role| role.identity()).collect();
    assert_eq!(identities.len(), DefinitionRole::ALL.len());
    for role in roles {
        assert_eq!(DefinitionRole::from_code(role.as_str()), Some(role));
        assert!(!role.identity().is_empty(), "{role:?}");
        assert_eq!(
            serde_json::to_value(role.reference()).unwrap(),
            serde_json::json!({"authority": "agent-ix", "identity": role.identity()})
        );
    }
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
#[trace("QSpec-TC-192")]
#[test]
fn conformance_admit_selection_matches_qspec_vectors() {
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
        let admitted =
            admit_selection(&triggers, &roles).unwrap_or_else(|code| panic!("{name}: {code:?}"));
        let expected: BTreeSet<Trigger> = triggers
            .iter()
            .map(|code| Trigger::from_code(code).unwrap())
            .collect();
        assert_eq!(admitted.triggers(), &expected, "{name}");
        let roles: BTreeSet<DefinitionRole> = roles
            .iter()
            .map(|code| DefinitionRole::from_code(code).unwrap())
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
        let code = admit_selection(
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

/// FR-110-AC-7: a selected layer's closure holds the layer and every layer
/// its `requires` edges name, transitively, in the lock's row order, and no
/// layer that requires it.
#[trace("TC-490", "FR-110-AC-7")]
#[test]
fn the_requires_closure_of_each_layer_is_computed() {
    use HeaderLayer::{
        ModelComplete, StateCore, StateGraph, StateQueries, TemporalComplete, ValueComplete,
    };
    let expected: [(HeaderLayer, &[HeaderLayer]); 6] = [
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
        (
            TemporalComplete,
            &[
                StateCore,
                StateQueries,
                StateGraph,
                ValueComplete,
                ModelComplete,
                TemporalComplete,
            ],
        ),
    ];
    for (selected, closure) in expected {
        assert_eq!(HeaderLayer::closure(&[selected]), closure, "{selected:?}");
    }
    // A union of selections is the union of their closures.
    assert_eq!(
        HeaderLayer::closure(&[StateQueries, ValueComplete]),
        [StateCore, StateQueries, ValueComplete]
    );
    assert_eq!(HeaderLayer::closure(&[]), Vec::<HeaderLayer>::new());
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
    assert!(StateCore.admits(StateClause));
    for form in [
        PredicateDeclaration,
        NamedPredicateCall,
        SequenceQuery,
        ObjectNavigation,
        FunctionDeclaration,
        ValueConstruct,
    ] {
        assert!(!StateCore.admits(form), "state core admits {form:?}");
    }
    for form in [
        StateClause,
        PredicateDeclaration,
        NamedPredicateCall,
        SequenceQuery,
    ] {
        assert!(StateQueries.admits(form), "queries refuses {form:?}");
    }
    for form in [ObjectNavigation, FunctionDeclaration, ValueConstruct] {
        assert!(!StateQueries.admits(form), "queries admits {form:?}");
    }
    assert!(StateGraph.admits(ObjectNavigation));
    assert!(!StateGraph.admits(FunctionDeclaration));
    assert!(ValueComplete.admits(FunctionDeclaration));
    assert!(ValueComplete.admits(ValueConstruct));
}

/// FR-110-AC-1: the qualification rows a package selects follow the lock's
/// `package_selection.layer`: the `always` rules when the layers hold
/// `quire.value.complete/v1`, and the `without_layer` rows otherwise.
#[trace("TC-490", "FR-110-AC-1")]
#[test]
fn the_qualification_rows_follow_the_value_complete_layer() {
    assert_eq!(
        qualification_roles(&HeaderLayer::closure(&[HeaderLayer::ValueComplete])),
        always_roles()
    );
    assert_eq!(
        qualification_roles(&HeaderLayer::closure(&[HeaderLayer::StateGraph])),
        [DefinitionRole::Edition]
    );
}
