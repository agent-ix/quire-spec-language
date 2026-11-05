// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1335 step 1 (FR-093-AC-59): the packages `dropFirst` and `removeOne`
//! lower to are admitted by IR's v2 reader, each carrying its
//! one-occurrence identity.

use super::*;

fn element(builtin: BuiltinType, bounds: &[&str]) -> TypeForm {
    TypeForm::builtin(builtin, SPAN).with_bounds(bounds.iter().map(|b| (*b).to_owned()).collect())
}

fn of(kind: CollectionKind, element: TypeForm, maximum: &str) -> TypeForm {
    TypeForm::collection(kind, SPAN)
        .with_arguments(vec![element])
        .with_bounds(vec!["0".into(), maximum.into()])
}

fn declared(
    name: &str,
    parameters: &[(&str, TypeForm)],
    result: TypeForm,
    body: Expression,
) -> FunctionDeclaration {
    FunctionDeclaration::new(
        name,
        parameters
            .iter()
            .map(|(name, form)| ((*name).to_owned(), form.clone()))
            .collect(),
        result,
        None,
        body,
    )
}

/// The read of `function`'s package, and the identities of its collection
/// application nodes.
fn read_identities(function: FunctionDeclaration) -> Vec<String> {
    let emission = emit(&package(vec![function]));
    assert_eq!(emission.omitted, [], "nothing is omitted");
    let read = read_back(&emission);
    assert!(
        matches!(read, Read::Verified { .. }),
        "IR's v2 reader must admit the package, got {read:?}"
    );
    let wire = wire(&emission);
    nodes(&wire)
        .iter()
        .filter(|node| node["body"]["operator"] == "collection")
        .map(|node| {
            node["body"]["operation"]["identity"]
                .as_str()
                .expect("an identity")
                .to_owned()
        })
        .collect()
}

#[trace("FR-093-AC-59", "TC-1335")]
#[test]
fn one_occurrence_packages_are_admitted_by_the_v2_reader() {
    let digit = || element(BuiltinType::Int, &["0", "9"]);
    let sequence = |maximum: &str| of(CollectionKind::Sequence, digit(), maximum);
    let ordered = |maximum: &str| of(CollectionKind::OrderedSet, digit(), maximum);
    let cases = [
        (
            declared(
                "f1",
                &[("s", sequence("9"))],
                sequence("8"),
                Expression::drop_first(name("s")),
            ),
            "quire.op.collection.sequence.drop_first",
        ),
        (
            declared(
                "f2",
                &[("o", ordered("5"))],
                ordered("4"),
                Expression::drop_first(name("o")),
            ),
            "quire.op.collection.ordered_set.drop_first",
        ),
        (
            declared(
                "g1",
                &[("s", sequence("9"))],
                sequence("9"),
                Expression::remove_one(
                    name("s"),
                    Expression::integer(quire_exact::Integer::from(3_i64)),
                ),
            ),
            "quire.op.collection.remove_one",
        ),
        (
            declared(
                "g2",
                &[
                    (
                        "t",
                        of(
                            CollectionKind::Bag,
                            element(BuiltinType::Text, &["0", "8", "nfc"]),
                            "9",
                        ),
                    ),
                    ("x", element(BuiltinType::Text, &["0", "8", "nfc"])),
                ],
                of(
                    CollectionKind::Bag,
                    element(BuiltinType::Text, &["0", "8", "nfc"]),
                    "9",
                ),
                Expression::remove_one(name("t"), name("x")),
            ),
            "quire.op.collection.remove_one",
        ),
    ];
    for (function, identity) in cases {
        let name = function.name.clone();
        assert_eq!(read_identities(function), [identity], "{name}");
    }
}

/// QSL FR-507-AC-11, QSpec FR-290/FR-484: the Boolean clause occurrence pair
/// is carried by its Boolean expression node and accepted by Contract IR.
#[trace("FR-507-AC-11", "QSpec-FR-290", "QSpec-FR-484")]
#[test]
fn a_boolean_value_clause_expression_is_admitted_by_the_v2_reader() {
    let emission = emit(&package(vec![both()]));
    let wire = wire(&emission);
    let expression = nodes(&wire)
        .iter()
        .find(|node| {
            node["node_tag"] == "expression"
                && node["occurrences"].as_array().is_some_and(|occurrences| {
                    occurrences
                        .iter()
                        .filter(|occurrence| occurrence["role"] == "claim")
                        .count()
                        == 1
                        && occurrences
                            .iter()
                            .filter(|occurrence| occurrence["role"] == "declaration")
                            .count()
                            == 1
                })
        })
        .expect("a Boolean expression with claim and declaration occurrences");
    let semantic_type = nodes(&wire)
        .iter()
        .find(|node| node["node_id"] == expression["semantic_type"])
        .expect("the expression's type node");
    assert_eq!(semantic_type["semantic_form"], "boolean");
    let admitted = match read_back(&emission) {
        Read::Verified { admitted, .. } => admitted,
        other => panic!("Contract IR admits the Boolean clause: {other:?}"),
    };
    let admitted_expression = admitted
        .graph()
        .nodes
        .iter()
        .find(|node| {
            node.node_tag.as_ref() == "expression"
                && node
                    .occurrences
                    .iter()
                    .filter(|occurrence| {
                        occurrence.role == quire_contract_model::CheckedOccurrenceRole::Claim
                    })
                    .count()
                    == 1
                && node
                    .occurrences
                    .iter()
                    .filter(|occurrence| {
                        occurrence.role == quire_contract_model::CheckedOccurrenceRole::Declaration
                    })
                    .count()
                    == 1
        })
        .expect("the reader retains the clause expression and both occurrences");
    assert!(admitted_expression.occurrences.iter().any(|occurrence| {
        occurrence.role == quire_contract_model::CheckedOccurrenceRole::Claim
            && occurrence.ordinal == 0
    }));
    assert!(admitted_expression.occurrences.iter().any(|occurrence| {
        occurrence.role == quire_contract_model::CheckedOccurrenceRole::Declaration
            && occurrence.ordinal == 0
    }));
}

/// QSL FR-507-AC-11, QSpec FR-290/FR-484: a Boolean function whose body is an
/// equality application keeps its equality producer row on the application's
/// `expression` occurrence, although the same node also holds the clause's
/// `claim` and `declaration` occurrences at the same body location.
#[trace("FR-507-AC-11", "QSpec-FR-290", "QSpec-FR-484")]
#[test]
fn an_equality_or_inequality_bodied_boolean_clause_emits_and_keeps_all_three_occurrences() {
    for operator in [BinaryOperator::Equal, BinaryOperator::NotEqual] {
        equality_bodied_clause_keeps_all_three_occurrences(operator);
    }
}

fn equality_bodied_clause_keeps_all_three_occurrences(operator: BinaryOperator) {
    let same = function(
        "same",
        &["a", "b"],
        Expression::binary(operator, name("a"), name("b")),
    );
    let emission = emit(&package(vec![same]));
    assert_eq!(emission.omitted, [], "nothing is omitted");
    let wire = wire(&emission);
    let equality = nodes(&wire)
        .iter()
        .find(|node| node["node_tag"] == "expression" && node["body"]["operator"] == "equal")
        .or_else(|| {
            nodes(&wire).iter().find(|node| {
                node["occurrences"].as_array().is_some_and(|occurrences| {
                    occurrences
                        .iter()
                        .any(|occurrence| occurrence["role"] == "claim")
                })
            })
        })
        .expect("the equality application node");
    let roles: Vec<_> = equality["occurrences"]
        .as_array()
        .expect("occurrences")
        .iter()
        .map(|occurrence| occurrence["role"].as_str().expect("role"))
        .collect();
    assert_eq!(roles, ["claim", "declaration", "expression"]);
    assert!(
        matches!(read_back(&emission), Read::Verified { .. }),
        "IR's v2 reader admits the package"
    );
    // The clause is the body expression: its claim and declaration occurrences
    // sit at the body's location, shared with the `expression` occurrence.
    let checked = graph_of(vec![function(
        "same",
        &["a", "b"],
        Expression::binary(operator, name("a"), name("b")),
    )]);
    let mut sites: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    for (node, origin, location) in checked.occurrences() {
        if matches!(
            origin.role().as_str(),
            "claim" | "declaration" | "expression"
        ) {
            sites
                .entry(format!("{node:?}"))
                .or_default()
                .push(format!("{}@{location:?}", origin.role().as_str()));
        }
    }
    let shared = sites
        .values()
        .find(|entries| entries.iter().any(|entry| entry.starts_with("claim@")))
        .expect("the clause node holds a claim occurrence");
    let places: std::collections::BTreeSet<_> = shared
        .iter()
        .map(|entry| entry.split_once('@').expect("role@location").1)
        .collect();
    assert_eq!(places.len(), 1, "one body location: {shared:?}");
}
