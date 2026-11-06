// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-337-AC-15 (TC-1267 step 8): a clause with a window aggregate atom
//! emits the atom as a `temporal`/`formula` node applying
//! `quire.op.temporal.aggregate`, whose one argument is QSpec FR-370's
//! "Window aggregate atom" aggregate of bindings. QSL's locked Contract IR
//! reader admits no such operation yet, so these tests read the emitted wire
//! and compare it with the accepted arguments of QSpec's
//! `temporal-aggregate-vectors.json`, read in place from `$QSPEC_DIR`.

use super::temporal_clause::{
    check_graph, clause_node, compile, document, formula_root, node_key, parameter_named, reaches,
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

/// A timestamped-event clock whose timestamps are in the unit `unit`.
fn clock_in(unit: &str) -> quire_contract_model::ClockConfiguration {
    quire_contract_model::ClockConfiguration::TimestampedEvent {
        timestamp_unit: unit.into(),
    }
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

/// The one `aggregate` argument of the aggregate node `node`.
fn argument(node: &Value) -> &Value {
    let arguments = node["body"]["arguments"].as_array().expect("arguments");
    let [argument] = &arguments[..] else {
        panic!("one argument: {node:#}");
    };
    assert_eq!(argument["term"], "aggregate", "{argument}");
    argument
}

/// The value bound to `name` in the aggregate node's argument.
fn bound<'n>(node: &'n Value, name: &str) -> &'n Value {
    argument(node)["members"]
        .as_array()
        .expect("members")
        .iter()
        .find(|member| member["name"] == name)
        .map(|member| &member["value"])
        .unwrap_or_else(|| panic!("a binding {name}: {node:#}"))
}

/// The package-free shape of an aggregate argument: each binding's name
/// and its literal (`value_kind`, `value` and the type's path) or
/// `reference`.
fn shape(argument: &Value) -> Vec<(String, String)> {
    argument["members"]
        .as_array()
        .expect("members")
        .iter()
        .map(|member| {
            assert_eq!(member["term"], "binding", "{member}");
            let value = &member["value"];
            let spelled = match value["term"].as_str() {
                Some("literal") => format!(
                    "{} {} {}",
                    value["value_kind"].as_str().expect("a value kind"),
                    value["value"].as_str().expect("a value"),
                    value["type"]["path"]
                ),
                Some("reference") => "reference".to_owned(),
                _ => panic!("a literal or a reference: {value}"),
            };
            (member["name"].as_str().expect("a name").to_owned(), spelled)
        })
        .collect()
}

/// The accepted argument of QSpec's window aggregate vector `id`.
fn vector(id: &str) -> Value {
    let path = std::path::Path::new(
        &std::env::var_os("QSPEC_DIR").expect("QSPEC_DIR names a quire-specification checkout"),
    )
    .join("proposals/checked-package-v2/temporal-aggregate-vectors.json");
    let vectors: Value =
        serde_json::from_slice(&std::fs::read(&path).expect("the vectors read")).unwrap();
    vectors["positive"]
        .as_array()
        .expect("positive vectors")
        .iter()
        .find(|case| case["id"] == id)
        .map(|case| case["argument"].clone())
        .unwrap_or_else(|| panic!("a positive vector {id}"))
}

/// FR-337-AC-15 (TC-1267 step 8): each aggregate kind emits its own
/// `quire.op.temporal.aggregate` node whose one argument binds, in QSpec
/// FR-370's order, exactly the members of QSpec's accepted vector over the
/// same inputs; its value, predicate and filter references read the
/// clause's `over` parameter, and the clause's `always` reads the node as
/// its operand.
#[trace("TC-1267", "FR-337-AC-15")]
#[test]
fn each_aggregate_kind_emits_the_qspec_vector_argument() {
    let cases = [
        (
            "count where holds(e.v > 1) over past[0,9] <= 2",
            "AG-01-count",
        ),
        ("sum of e.v over past[0,2] <= 15", "AG-02-sum"),
        ("min of e.v over past[0,9] >= 1", "AG-03-min"),
        ("max of e.v over past[0,9] < 10", "AG-04-max"),
        (
            "fraction holds(e.w = 0) over past[0,3] >= 1/2",
            "AG-05-fraction",
        ),
        ("quantile 0.5 of e.v over past[0,9] = 4", "AG-06-quantile"),
        (
            "count where holds(e.v > 1) over past[0,9] min_count 0 <= 2",
            "AG-08-count-min-count-0",
        ),
        (
            "sum of e.v where holds(e.v > 1) over past[0,9] min_count 0 = 0",
            "AG-09-sum-min-count-0",
        ),
    ];
    for (atom, id) in cases {
        let compiled = compile(&sampled(&format!("always[0,3] ({atom})")));
        let wire = &compiled.wire;
        let node = aggregate(wire);
        let expected = shape(&vector(id));
        assert_eq!(shape(argument(node)), expected, "{atom}");
        let over = &parameter_named(wire, "e")["node_id"];
        for read in ["predicate", "value", "filter"] {
            if expected.iter().any(|(name, _)| name == read) {
                assert!(reaches(wire, bound(node, read), over), "{atom}: {read}");
            }
        }
        let always = formula_root(wire, clause_node(wire));
        assert_eq!(
            always["body"]["operation"]["identity"],
            "quire.op.temporal.always"
        );
        assert_eq!(target(&always["body"]["arguments"][0]), &node["node_id"]);
    }
}

/// FR-337-AC-15 (TC-1267 step 8): an omitted `min_count` and an explicit
/// `min_count 1` emit one node and one clause, reformatting the source
/// changes no node, and changing the window, `min_count`, value, filter,
/// comparison, threshold or kind changes the aggregate node, the clause
/// node and the package's node set.
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

/// FR-337 "Aggregate formula emission": QSpec FR-370 writes a `duration`
/// window's bounds as quantities of the `scalar_type`/`unit` node whose path
/// is `["scalar_type", "unit", T]`, `T` the selected `timestamp_unit`. A
/// source-declared unit's node path is its declared name, so a window in it
/// has no encoding whatever the clock names, its own unit included, and the
/// emission refuses with no package.
#[trace("TC-1267", "FR-337-AC-15")]
#[test]
fn a_duration_window_outside_the_clock_unit_node_refuses() {
    let graph = check_graph(&timestamped(
        "always[0,3] (quantile 0.5 of e over past[0 ms, 300 s] <= 5 ms)",
    ));
    let selections: Vec<_> = graph
        .temporal_clauses()
        .iter()
        .map(|temporal| {
            crate::emit::TemporalClockSelection::new(
                graph.source().clone(),
                temporal.clause().span(),
                clock_in("ms"),
            )
        })
        .collect();
    let refusal = crate::emit::emit_checked_with_temporal_selections(
        &CheckedPackage::link(graph),
        &selections,
    )
    .expect_err("a source-declared unit is no clock unit node");
    assert!(
        matches!(
            refusal,
            EmitRefusal::InvalidTemporalClockValue {
                field: "timestamp_unit",
                ..
            }
        ),
        "{refusal:?}"
    );
}

/// FR-337-AC-15 (TC-1267 step 8): the aggregate node is located at the
/// aggregate atom and each of its expressions at its own expression tree;
/// a literal threshold is bound inline.
#[trace("TC-1267", "FR-337-AC-15")]
#[test]
fn the_aggregate_spans_resolve_through_the_source_map() {
    let atom = "sum of e.v where holds(e.w = 0) over past[0,2] <= 15";
    let text = sampled(&format!("always[0,3] ({atom})"));
    let compiled = compile(&text);
    let wire = &compiled.wire;
    let node = aggregate(wire);
    assert_eq!(bound(node, "threshold")["term"], "literal");
    assert_eq!(bound(node, "threshold")["value"], "15");

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
    assert!(located(target(bound(node, "value"))).contains(&span("e.v")));
}

/// The `example/config-version` package's time unit `unit` as the clock's
/// `timestamp_unit`: its FCD identity, the last segment of its node path.
fn package_unit(unit: &str) -> String {
    format!("ix://example/config-version/unit/{unit}")
}

/// A unit selecting `example/config-version` (base dimension `Time`, units
/// `s` and `ms`) as `Config`, whose timestamped-event clause `Window` over
/// `e: Sample` has `count over {window} >= 1` under `always[0,3]`, after
/// `declarations`.
fn package_window(declarations: &str, window: &str) -> String {
    unit(
        TIMESTAMPED,
        &format!(
            "{declarations}record Sample {{ v: Int[0, 9]; }}\n\
             temporal Window using t over (e: Sample) clock \"model-steps\" on origin \
             {{ always[0,3] (count over {window} >= 1) }}"
        ),
    )
}

/// `text` checked with its one clause's clock in `timestamp_unit`, given to
/// S3 as the spine gives it, then emitted with that clock selected.
fn emit_with_clock(
    text: &str,
    timestamp_unit: &str,
) -> Result<Result<Value, EmitRefusal>, Vec<qsl_semantics::check::CheckRefusal>> {
    let document = super::timed_source::package_units_document();
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let assembled =
        || assemble_with_models(text.as_bytes(), &packages).expect("the unit assembles");
    let probe = assembled()
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the clause checks without a clock: {refusals:?}"));
    let [temporal] = probe.temporal_clauses() else {
        panic!("one clause");
    };
    let span = temporal.clause().span();
    let graph = assembled()
        .with_clock_units(vec![qsl_semantics::check::ClockUnitSelection {
            declaration_span: span,
            timestamp_unit: timestamp_unit.to_owned(),
        }])
        .check(CheckingLimits::default())?;
    let selections = [crate::emit::TemporalClockSelection::new(
        graph.source().clone(),
        span,
        clock_in(timestamp_unit),
    )];
    Ok(crate::emit::emit_checked_with_temporal_selections(
        &CheckedPackage::link(graph),
        &selections,
    )
    .map(|emission| wire(&emission)))
}

/// FR-337 "Aggregate formula emission" item 7, QSpec FR-370: a `duration`
/// window's `lower` and `upper` are quantity literals of the clause's clock
/// unit node, `["scalar_type", "unit", T]` with `T` the selected
/// `timestamp_unit`, both bounds converted exactly to it whatever units they
/// are written in; the model-owned clock unit node is written with an empty
/// body, its model owner and a `generated` occurrence.
#[trace("TC-1267", "FR-337-AC-15")]
#[test]
fn a_duration_window_converts_both_bounds_to_the_selected_clock_unit() {
    let cases = [
        ("past[0 Config::s, 300000 Config::ms]", "ms", "0", "300000"),
        ("past[0 Config::s, 300000 Config::ms]", "s", "0", "300"),
        ("past[0 Config::ms, 300 Config::s]", "ms", "0", "300000"),
        ("past[1 Config::ms, 1 Config::s]", "s", "1/1000", "1"),
    ];
    for (window, clock, lower, upper) in cases {
        let clock_unit = package_unit(clock);
        let wire = emit_with_clock(&package_window("", window), &clock_unit)
            .unwrap_or_else(|refusals| panic!("{window}: {refusals:?}"))
            .unwrap_or_else(|refusal| panic!("{window} under {clock}: {refusal:?}"));
        let node = aggregate(&wire);
        let path = json!(["scalar_type", "unit", clock_unit]);
        for (name, expected) in [("lower", lower), ("upper", upper)] {
            let bound = bound(node, name);
            assert_eq!(bound["value_kind"], "quantity", "{window} {name}");
            assert_eq!(bound["value"], expected, "{window} under {clock}: {name}");
            assert_eq!(
                bound["type"]["path"], path,
                "{window} under {clock}: {name}"
            );
        }
        // The clock unit's model-owned closure: the unit itself, its `Time`
        // dimension and, for `ms`, the `s` it is defined over.
        let time = "ix://example/config-version/dimension/Time";
        let mut closure = vec![("unit", clock_unit.clone()), ("dimension", time.to_owned())];
        if clock == "ms" {
            closure.push(("unit", package_unit("s")));
        }
        for (form, identity) in closure {
            let path = json!(["scalar_type", form, identity]);
            let [written] = nodes(&wire)
                .iter()
                .filter(|candidate| candidate["node_id"]["path"] == path)
                .collect::<Vec<_>>()[..]
            else {
                panic!("{window} under {clock}: one {form} node {identity}");
            };
            assert_eq!(written["semantic_form"], form, "{identity}");
            assert_eq!(written["owner"]["kind"], "model", "{identity}");
            assert_eq!(written["owner"]["node"], identity.as_str());
            assert_eq!(
                written["body"],
                json!({"term": "aggregate", "members": []}),
                "{identity}"
            );
            assert_eq!(written["occurrences"][0]["role"], "generated", "{identity}");
        }
    }
}

/// FR-194, QSpec FR-370: a `duration` bound that does not convert to the
/// selected clock unit is no duration of the clause's clock: a source
/// `Time` unit under a clock in the model package's `Time` (another
/// dimension, no equivalence) refuses `ill_typed`/`unit-mismatch` at S3.
#[trace("TC-629", "FR-194-AC-7")]
#[test]
fn a_duration_bound_of_another_time_dimension_than_the_clock_refuses() {
    let text = package_window(
        "dimension Time;\nunit s : Time = rational(1, 1);\n",
        "past[0 s, 1 s]",
    );
    let refusals = emit_with_clock(&text, &package_unit("ms"))
        .expect_err("a source second is no duration of the package clock");
    let [refusal] = &refusals[..] else {
        panic!("one refusal: {refusals:?}");
    };
    assert_eq!(refusal.cause.code(), qsl_foundation::Code::IllTyped);
    assert_eq!(refusal.cause.cause(), Some("unit-mismatch"));
    // At the lower bound's unit `s`.
    let start = text.find("past[0 s").expect("the window") + "past[0 ".len();
    let qsl_semantics::check::CheckCause::Temporal(cause) = &refusal.cause else {
        panic!("a temporal refusal: {refusal:?}");
    };
    assert_eq!(
        **cause,
        qsl_semantics::check::TemporalCause::AggregateWindowKindMismatch {
            span: qsl_foundation::Span {
                start,
                end: start + 1
            }
        }
    );
}
