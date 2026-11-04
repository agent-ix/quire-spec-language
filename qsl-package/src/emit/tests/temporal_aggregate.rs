// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-337-AC-15 (TC-1267 step 8): a clause with a window aggregate atom
//! emits the atom as a `temporal`/`formula` node applying
//! `quire.op.temporal.aggregate`, whose arguments are the bindings FR-337
//! "Aggregate formula emission" orders. Contract IR's reader admits no such
//! operation yet (QSL-1644), so these tests read the emitted wire.

use super::temporal_clause::{
    clause_node, compile, document, formula_root, node_key, node_with, parameter_named, reaches,
    target, unit, EVENT,
};
use super::*;

const TIMESTAMPED: &str = "quire.temporal.timestamped-event.finite-window/v1";

/// A unit whose clause `Window` over `e: Sample` under the event-position
/// profile has the formula `formula`.
fn sampled(formula: &str) -> String {
    unit(
        EVENT,
        &format!(
            "record Sample {{ v: Int[0, 9]; w: Int[0, 9]; }}\n\
             temporal Window using t over (e: Sample) clock \"model-steps\" on origin \
             {{ {formula} }}"
        ),
    )
}

/// A unit whose clause `Window` over `e: Quantity<ms>` under the
/// timestamped-event profile has the formula `formula`.
fn timestamped(formula: &str) -> String {
    unit(
        TIMESTAMPED,
        &format!(
            "dimension Time;\n\
             unit s : Time = rational(1, 1);\n\
             unit ms : Time = rational(1, 1000) * s;\n\
             temporal Window using t over (e: Quantity<ms>) clock \"model-steps\" on origin \
             {{ {formula} }}"
        ),
    )
}

/// The one aggregate formula node of `wire`.
fn aggregate(wire: &Value) -> &Value {
    let found: Vec<&Value> = nodes(wire)
        .iter()
        .filter(|node| node["body"]["operation"]["identity"] == "quire.op.temporal.aggregate")
        .collect();
    let [node] = found[..] else {
        panic!("one aggregate node: {found:#?}");
    };
    assert_eq!(node["node_tag"], "temporal");
    assert_eq!(node["semantic_form"], "formula");
    assert_eq!(node["body"]["operator"], "temporal_formula");
    assert_eq!(node["body"]["operation"]["member"], Value::Null);
    node
}

/// The argument names of the aggregate node `node`, in order.
fn names(node: &Value) -> Vec<&str> {
    node["body"]["arguments"]
        .as_array()
        .expect("arguments")
        .iter()
        .map(|argument| {
            assert_eq!(argument["term"], "binding", "{argument}");
            argument["name"].as_str().expect("a name")
        })
        .collect()
}

/// The value bound to `name` among the aggregate node's arguments.
fn bound<'n>(node: &'n Value, name: &str) -> &'n Value {
    node["body"]["arguments"]
        .as_array()
        .expect("arguments")
        .iter()
        .find(|argument| argument["name"] == name)
        .map(|argument| &argument["value"])
        .unwrap_or_else(|| panic!("an argument {name}: {node:#}"))
}

/// The literal text of the literal term `term`.
fn literal(term: &Value) -> &str {
    assert_eq!(term["term"], "literal", "{term}");
    term["value"].as_str().expect("a literal value")
}

/// The window bindings of the aggregate node `node`, as `(name, value)`.
fn window(node: &Value) -> Vec<(String, String)> {
    bound(node, "window")["members"]
        .as_array()
        .expect("a window aggregate")
        .iter()
        .map(|member| {
            (
                member["name"].as_str().expect("a name").to_owned(),
                literal(&member["value"]).to_owned(),
            )
        })
        .collect()
}

/// FR-337-AC-15 (TC-1267 step 8): each aggregate kind emits its own
/// `quire.op.temporal.aggregate` node whose bindings follow FR-337's order,
/// with the kind, the normalized `min_count`, the window offsets, the
/// comparison, and value, predicate and filter terms that read the clause's
/// `over` parameter; the clause's `always` reads the node as its operand.
#[trace("TC-1267", "FR-337-AC-15")]
#[test]
fn each_aggregate_kind_emits_its_ordered_bindings() {
    let cases = [
        (
            "count where holds(e.v > 1) over past[0,2] >= 1",
            "count",
            vec![
                "kind",
                "filter",
                "window",
                "min_count",
                "comparison",
                "threshold",
            ],
            "1",
            ">=",
        ),
        (
            "sum of e.v over past[0,2] min_count 0 <= 15",
            "sum",
            vec![
                "kind",
                "window",
                "min_count",
                "expression",
                "comparison",
                "threshold",
            ],
            "0",
            "<=",
        ),
        (
            "min of e.v over past[0,2] min_count 3 < 3",
            "min",
            vec![
                "kind",
                "window",
                "min_count",
                "expression",
                "comparison",
                "threshold",
            ],
            "3",
            "<",
        ),
        (
            "max of e.v over past[0,2] > 3",
            "max",
            vec![
                "kind",
                "window",
                "min_count",
                "expression",
                "comparison",
                "threshold",
            ],
            "1",
            ">",
        ),
        (
            "fraction holds(e.v > 1) over past[0,2] >= 1",
            "fraction",
            vec![
                "kind",
                "window",
                "min_count",
                "expression",
                "comparison",
                "threshold",
            ],
            "1",
            ">=",
        ),
        (
            "quantile 0.5 of e.v where holds(e.w = 0) over past[0,2] = 4",
            "quantile",
            vec![
                "kind",
                "level",
                "filter",
                "window",
                "min_count",
                "expression",
                "comparison",
                "threshold",
            ],
            "1",
            "=",
        ),
    ];
    for (atom, kind, order, min_count, comparison) in cases {
        let compiled = compile(&sampled(&format!("always[0,3] ({atom})")));
        let wire = &compiled.wire;
        let node = aggregate(wire);
        assert_eq!(names(node), order, "{atom}");
        assert_eq!(literal(bound(node, "kind")), kind, "{atom}");
        assert_eq!(literal(bound(node, "min_count")), min_count, "{atom}");
        assert_eq!(literal(bound(node, "comparison")), comparison, "{atom}");
        assert_eq!(
            window(node),
            [
                ("lower".to_owned(), "0".to_owned()),
                ("upper".to_owned(), "2".to_owned())
            ],
            "{atom}"
        );
        let over = &parameter_named(wire, "e")["node_id"];
        for read in ["expression", "filter"] {
            if order.contains(&read) {
                assert!(reaches(wire, bound(node, read), over), "{atom}: {read}");
            }
        }
        // The clause's `always` reads the aggregate node as its operand.
        let always = formula_root(wire, clause_node(wire));
        assert_eq!(
            always["body"]["operation"]["identity"],
            "quire.op.temporal.always"
        );
        assert_eq!(target(&always["body"]["arguments"][0]), &node["node_id"]);
    }

    // A `quantile`'s level is its exact rational.
    let quantile = compile(&sampled(
        "always[0,3] (quantile 0.5 of e.v over past[0,2] <= 4)",
    ));
    assert_eq!(literal(bound(aggregate(&quantile.wire), "level")), "1/2");
}

/// FR-337-AC-15 (TC-1267 step 8): an omitted `min_count` and an explicit
/// `min_count 1` emit one node and one clause, reformatting the source
/// changes no node, and changing the window, `min_count`, expression,
/// comparison or threshold changes the aggregate node, the clause node and
/// the package's node set.
#[trace("TC-1267", "FR-337-AC-15")]
#[test]
fn the_aggregate_bindings_decide_the_node_identity() {
    let identity = |formula: &str| {
        let compiled = compile(&sampled(formula));
        let wire = &compiled.wire;
        let [clause] = compiled.clauses.as_slice() else {
            panic!("one clause");
        };
        (
            aggregate(wire)["body"].clone(),
            clause.identity(),
            wire["semantic_graph"].clone(),
        )
    };
    let base = identity("always[0,3] (sum of e.v over past[0,2] <= 15)");
    let explicit = identity("always[0,3] (sum of e.v over past[0,2] min_count 1 <= 15)");
    assert_eq!(base, explicit, "an omitted min_count is min_count 1");
    let reformatted = identity("always[0,3](  sum   of e.v\n over past[ 0 , 2 ]<=15 )");
    assert_eq!(base, reformatted, "spans enter no identity");

    for mutated in [
        "always[0,3] (sum of e.v over past[0,3] <= 15)",
        "always[0,3] (sum of e.v over past[1,2] <= 15)",
        "always[0,3] (sum of e.v over past[0,2] min_count 2 <= 15)",
        "always[0,3] (sum of e.v + 1 over past[0,2] <= 15)",
        "always[0,3] (sum of e.v over past[0,2] < 15)",
        "always[0,3] (sum of e.v over past[0,2] <= 14)",
        "always[0,3] (sum of e.v where holds(e.w = 0) over past[0,2] <= 15)",
        "always[0,3] (max of e.v over past[0,2] <= 15)",
    ] {
        let (body, clause, graph) = identity(mutated);
        assert_ne!(body, base.0, "{mutated}");
        assert_ne!(clause, base.1, "{mutated}");
        assert_ne!(graph, base.2, "{mutated}");
    }
}

/// FR-337-AC-15 (TC-1267 step 8): under the timestamped-event profile a
/// duration window is written in the lower bound's unit with both bounds
/// converted exactly, and a threshold in another unit of the value's
/// dimension is converted to the value's unit; one in the value's unit is
/// written as it is.
#[trace("TC-1267", "FR-337-AC-15")]
#[test]
fn a_duration_window_and_threshold_are_converted() {
    let converted = compile(&timestamped(
        "always[0,3] (quantile 0.5 of e over past[0 s, 1500 ms] <= 1 s)",
    ));
    let wire = &converted.wire;
    let node = aggregate(wire);
    assert_eq!(
        window(node),
        [
            ("lower".to_owned(), "0".to_owned()),
            ("upper".to_owned(), "3/2".to_owned()),
            ("unit".to_owned(), "s".to_owned())
        ]
    );
    let expression = bound(node, "expression");
    let value_type = &node_with(wire, target(expression))["semantic_type"];
    let threshold = node_with(wire, target(bound(node, "threshold")));
    assert_eq!(
        &threshold["semantic_type"], value_type,
        "the threshold is in the value's unit"
    );
    assert!(
        threshold["body"]["operation"]["identity"]
            .as_str()
            .is_some_and(|identity| identity.contains("convert")),
        "a converted threshold: {threshold:#}"
    );

    let same_unit = compile(&timestamped(
        "always[0,3] (quantile 0.5 of e over past[0 s, 1500 ms] <= 1000 ms)",
    ));
    let node = aggregate(&same_unit.wire);
    let threshold = node_with(&same_unit.wire, target(bound(node, "threshold")));
    assert_eq!(threshold["semantic_form"], "literal");
}

/// FR-337-AC-15 (TC-1267 step 8): the aggregate node is located at the
/// aggregate atom and each of its expressions at its own expression tree.
#[trace("TC-1267", "FR-337-AC-15")]
#[test]
fn the_aggregate_spans_resolve_through_the_source_map() {
    let atom = "sum of e.v where holds(e.w = 0) over past[0,2] <= 15";
    let text = sampled(&format!("always[0,3] ({atom})"));
    let compiled = compile(&text);
    let wire = &compiled.wire;
    let node = aggregate(wire);

    let packages = qsl_semantics::model::intake::package_input([document()]).unwrap();
    let declarations = assemble_with_models(text.as_bytes(), &packages).expect("assembles");
    let regions = declarations.regions();
    let graph = declarations
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    let located = |id: &Value| -> Vec<(u64, u64)> {
        let sought = node_key(wire, id);
        graph
            .occurrences()
            .filter(|(key, origin, _)| key == &sought && origin.role().as_str() == "expression")
            .map(|(_, _, location)| {
                let region = regions.region(location).expect("a region");
                (region.start(), region.end())
            })
            .collect()
    };
    let span = |needle: &str| {
        let start = u64::try_from(text.find(needle).expect("in the text")).unwrap();
        (start, start + u64::try_from(needle.len()).unwrap())
    };
    assert_eq!(located(&node["node_id"]), [span(atom)]);
    assert!(located(target(bound(node, "filter"))).contains(&span("e.w = 0")));
    assert!(located(target(bound(node, "threshold"))).contains(&span("15")));
}
