// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-515 (TC-1310 rows 1 and 14): the temporal subject read from the three
//! packages of the ported IT-001 recipe R that the checked-package handoff
//! writer emits (FR-517).

use ix_trace_rs::trace;
use qsl_package::checked_views::{
    checked_predicate, temporal_subject, Activation, PredicateKind, PredicateSelection,
    TemporalSubjectView,
};
use quire_contract_model::{
    CheckedBindingNameKind, CheckedLiteralValue, CheckedNodeId, CheckedNodeKind,
    CheckedOccurrenceRole, CheckedOperationInterval, CheckedPackageV2, CheckedTerm, ProtocolForm,
    ValueForm,
};
use quire_spec_language::checked_handoff::{evidence, write, Corpus};
use serde_json::Value;

use super::checked_handoff::{admitted, request, tree, KINDS};

/// The package of `kind` and its selection sidecar, as the handoff writer
/// wrote them and the reader admits them.
pub(super) fn recipe_package(kind: &str) -> (Box<CheckedPackageV2>, Value) {
    let holder = tempfile::tempdir().expect("temporary directory");
    let out = holder.path().join("handoff");
    write(&request(kind, Corpus::None), &out).expect("the handoff is written");
    let files = tree(&out);
    let mut held = evidence(&request(kind, Corpus::None)).expect("evidence");
    held.insert_domain_package_document(
        "agent-ix/checked-handoff",
        files["models/0.json"].as_slice(),
    )
    .expect("the evidence holds the same document");
    let package = admitted(&files["checked-package.json"], &held);
    let sidecar = serde_json::from_slice(&files["selection.json"]).expect("a sidecar");
    (package, sidecar)
}

fn id_of(value: &Value) -> CheckedNodeId {
    serde_json::from_value(value.clone()).expect("a node id")
}

/// The protocol clauses whose `requirements` aggregate references `clause`,
/// read from the protocol clause terms.
fn requiring(package: &CheckedPackageV2, clause: &CheckedNodeId) -> Vec<CheckedNodeId> {
    package
        .resolved_graph()
        .nodes
        .iter()
        .zip(package.node_kinds())
        .filter(|(_, kind)| **kind == CheckedNodeKind::Protocol(ProtocolForm::ProtocolClause))
        .filter(|(node, _)| {
            let Some(CheckedTerm::Application { arguments, .. }) = package.term(&node.node_id)
            else {
                return false;
            };
            arguments.iter().any(|argument| match argument {
                CheckedTerm::Binding { name, value }
                    if quire_contract_model::CheckedBindingNameKind::Other("requirements")
                        == name.kind() =>
                {
                    matches!(&**value, CheckedTerm::Aggregate { members }
                        if members.iter().any(|member| matches!(member,
                            CheckedTerm::Reference { target } if target == clause)))
                }
                _ => false,
            })
        })
        .map(|(node, _)| node.node_id.clone())
        .collect()
}

/// An integer interval's rendered lower and optional upper bound.
type Bounds = Option<(String, Option<String>)>;

fn bounds(view: &TemporalSubjectView<'_>) -> Vec<(String, Bounds)> {
    view.formula()
        .iter()
        .map(|used| {
            let interval = used.interval.as_ref().map(|interval| match interval {
                CheckedOperationInterval::Integer { lower, upper } => {
                    (lower.to_string(), upper.as_ref().map(ToString::to_string))
                }
                timed => panic!("an integer interval: {timed:?}"),
            });
            (used.operation.to_owned(), interval)
        })
        .collect()
}

/// FR-515-AC-1 (TC-1310 row 1): over each package of R and each of its
/// temporal clauses, every accessor equals the value the test reads from the
/// node, lock, source map, protocol clause or `temporal_bindings()` row.
#[trace("FR-515-AC-1", "TC-1310")]
#[test]
fn every_clause_of_each_recipe_package_reads_its_node_facts() {
    let mut seen = Vec::new();
    for kind in KINDS {
        let (package, sidecar) = recipe_package(kind);
        for row in sidecar["clauses"].as_array().unwrap() {
            let name = row["name"].as_str().unwrap();
            let id = id_of(&row["node"]);
            let view = temporal_subject(&package, &id).expect("the clause reads");
            seen.push(name.to_owned());

            assert_eq!(*view.declaration(), id, "{name}");
            let node = package
                .resolved_graph()
                .nodes
                .iter()
                .find(|node| node.node_id == id)
                .unwrap();
            let claims: Vec<_> = node
                .occurrences
                .iter()
                .filter(|occurrence| occurrence.role == CheckedOccurrenceRole::Claim)
                .map(|occurrence| occurrence.ordinal)
                .collect();
            let ordinals: Vec<_> = view.occurrences().iter().map(|o| o.ordinal).collect();
            assert_eq!(ordinals, claims, "{name}");
            assert!(view.occurrences().iter().all(|o| o.node == id));

            let Some(CheckedTerm::Application { arguments, .. }) = package.term(&id) else {
                panic!("a clause application");
            };
            let CheckedTerm::Reference { target } = &arguments[0] else {
                panic!("an over reference");
            };
            assert_eq!(view.over(), target, "{name}");

            let expected_requirements = requiring(&package, &id);
            let found: Vec<CheckedNodeId> =
                view.requirements().iter().map(|&r| r.clone()).collect();
            assert_eq!(found, expected_requirements, "{name}");
            // Only `Due` of the event-position package is required.
            assert_eq!(found.len(), usize::from(name == "Due"), "{name} in {kind}");

            let claim_entry = package
                .source_map()
                .iter()
                .find(|entry| entry.node_id == id && entry.role == CheckedOccurrenceRole::Claim)
                .unwrap();
            assert_eq!(view.source_span(), claim_entry, "{name}");

            let lock_profile = package
                .lock()
                .profile_selections
                .iter()
                .find(|selection| {
                    selection.definition.identity.as_ref()
                        == format!(
                            "{}/{}",
                            row["profile"]["identity"].as_str().unwrap(),
                            row["profile"]["revision"].as_str().unwrap()
                        )
                })
                .expect("the lock row of the clause's profile");
            assert_eq!(view.profile(), lock_profile, "{name}");

            let binding = package
                .temporal_bindings()
                .unwrap()
                .iter()
                .find(|binding| binding.declaration == id)
                .expect("the declaration's row");
            assert_eq!(view.clock(), Some(binding), "{name}");
            assert_eq!(
                serde_json::to_value(&binding.clock).unwrap(),
                row["binding"]["clock"],
                "{name}"
            );

            assert_eq!(*view.activation(), Activation::Origin, "{name}");
            assert_eq!(view.captures(), [], "{name}");

            let upper = if name == "DueLate" { "60" } else { "30" };
            assert_eq!(
                bounds(&view),
                [
                    (
                        "quire.op.temporal.always".to_owned(),
                        Some(("0".to_owned(), Some(upper.to_owned())))
                    ),
                    ("quire.op.temporal.holds".to_owned(), None)
                ],
                "{name}"
            );
            assert_eq!(view.predicate_leaves().len(), 1, "{name}");
        }
    }
    seen.sort();
    assert_eq!(seen, ["Due", "DueLate", "DueSample", "DueTimestamp"]);
}

/// The `value`/`parameter` node named `name` in `package`, read from its term.
fn parameter_named(package: &CheckedPackageV2, name: &str) -> CheckedNodeId {
    let found: Vec<_> = package
        .resolved_graph()
        .nodes
        .iter()
        .zip(package.node_kinds())
        .filter(|(_, kind)| **kind == CheckedNodeKind::Value(ValueForm::Parameter))
        .filter(|(node, _)| match package.term(&node.node_id) {
            Some(CheckedTerm::Aggregate { members }) => members.iter().any(|member| {
                matches!(member, CheckedTerm::Binding { name: bound, value }
                    if CheckedBindingNameKind::Other("name") == bound.kind()
                        && value.literal_read().is_some_and(|literal|
                            literal.value == CheckedLiteralValue::Text(name)))
            }),
            _ => false,
        })
        .map(|(node, _)| node.node_id.clone())
        .collect();
    let [id] = found.try_into().expect("one parameter of that name");
    id
}

/// FR-515-AC-4 (TC-1310 row 4): the `HoldsLeaf` of the recipe's `Due` reads as
/// a `HoldsLeaf` of that clause whose one binder is `view`, with a source span.
#[trace("FR-515-AC-4", "TC-1310")]
#[test]
fn the_recipe_due_leaf_reads_the_view_binder() {
    let (package, sidecar) = recipe_package("event_position");
    let row = sidecar["clauses"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == "Due")
        .expect("the clause Due");
    let id = id_of(&row["node"]);
    let subject = temporal_subject(&package, &id).unwrap();
    let [leaf] = subject.predicate_leaves() else {
        panic!("one holds leaf");
    };
    let view = checked_predicate(
        &package,
        &PredicateSelection {
            clause: id.clone(),
            leaf: leaf.clone(),
        },
    )
    .expect("the leaf reads");
    assert_eq!(view.kind(), PredicateKind::HoldsLeaf);
    assert_eq!(view.leaf(), leaf);
    assert_eq!(*view.declaration(), id);
    assert_eq!(view.binders(), [parameter_named(&package, "view")]);
    assert!(!view.source_span().regions.is_empty());
}

/// FR-515-AC-14 (TC-1310 row 14): `name()` reads the declared name from the
/// admitted package carrier for both clause and predicate views.
#[trace("FR-515-AC-14", "TC-1310")]
#[test]
fn a_view_names_its_clause_as_the_sidecar_does() {
    for kind in KINDS {
        let (package, sidecar) = recipe_package(kind);
        for row in sidecar["clauses"].as_array().unwrap() {
            let id = id_of(&row["node"]);
            let view = temporal_subject(&package, &id).unwrap();
            assert_eq!(view.name(), row["name"].as_str());
            let leaf = view.predicate_leaves()[0].clone();
            let predicate =
                checked_predicate(&package, &PredicateSelection { clause: id, leaf }).unwrap();
            assert_eq!(predicate.name(), row["name"].as_str());
        }
    }
}
