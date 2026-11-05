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
