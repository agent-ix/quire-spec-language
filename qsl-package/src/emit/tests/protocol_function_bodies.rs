// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1021, TC-1062: a protocol body that calls a function or refers to a
//! protocol (QSL-1121g; QSpec FR-464 "Reader order", FR-383 "Cyclic
//! reference components"). A node's key hashes the keys of the nodes it
//! names, so the functions and protocols are keyed in dependency order.
//! Every emitted package is read back through IR's v2 reader.

use super::protocol_graph::{checked, clause, model, target, PROFILES};
use super::reachability::{lower_with, Lowered};
use super::*;
use qsl_semantics::check::{CheckCause, ProtocolConstruct, ProtocolReferenceCause};

/// A protocol `Flow` over `M::Order` whose `finish` constraint is `finish`,
/// with `activation`, `captures` and a `terminal` member.
fn flow(activation: &str, captures: &str, terminal: &str, finish: &str) -> String {
    format!(
        "record Tick {{ ready: Boolean; n: Integer; }}\n\
         protocol Flow using v over (input: M::Order) {activation} {{\n\
         {captures}\n\
         role Buyer on M::Order;\n\
         {terminal}\n\
         run sequence Main {{ }}\n\
         finish End as (outcome: Boolean) {{ {finish} }};\n\
         }}\n"
    )
}

/// `text` emitted over [`model`] and read back by IR; the reader must admit
/// it.
fn admitted(text: &str) -> Lowered {
    let lowered = lower_with(model(), PROFILES, text);
    assert!(lowered.ir.is_ok(), "{:?}\n{text}", lowered.ir.err());
    assert!(lowered.verified, "{text}");
    lowered
}

/// The digest of the node `node`.
fn digest(node: &Value) -> String {
    node["node_id"].to_string()
}

/// The `function` node declared `name`.
fn function<'w>(wire: &'w Value, name: &str) -> &'w Value {
    nodes(wire)
        .iter()
        .find(|node| {
            node["node_tag"] == "function" && node["declaration"]["qualified_name"][0] == name
        })
        .unwrap_or_else(|| panic!("no function {name}"))
}

/// The nodes `node` reaches through its dependencies, transitively.
fn reaches(wire: &Value, node: &Value, wanted: &Value) -> bool {
    super::protocol_graph::reads(wire, node, &wanted["node_id"])
}

/// The clause key.
fn identity(lowered: &Lowered) -> String {
    digest(clause(&lowered.wire))
}

/// FR-383 "Outputs", QSL-1121g: a `finish` constraint, a capture
/// initializer, an `each` guard and a `terminal when` condition that call a
/// function are emitted after the function's node, the clause reaches that
/// node, and the reader admits the package.
#[test]
fn a_protocol_body_that_calls_a_function_is_keyed_after_the_function() {
    let ok = "function ok using v(): Boolean pure { true }\n";
    let sites = [
        flow("on origin", "", "", "ok()"),
        flow("on origin", "capture c: Boolean = ok();", "", "true"),
        flow("on each (t: Tick) when (ok())", "", "", "true"),
        flow("on origin", "", "terminal when { ok() };", "true"),
    ];
    for site in sites {
        let text = format!("{ok}{site}");
        let lowered = admitted(&text);
        let callee = function(&lowered.wire, "ok");
        assert!(
            reaches(&lowered.wire, clause(&lowered.wire), callee),
            "the clause reaches `ok`: {text}"
        );
        // The same unit with the declarations in the other order is the same
        // package.
        let swapped = format!("{site}{ok}");
        assert_eq!(identity(&admitted(&swapped)), identity(&lowered), "{text}");
    }
}

/// A control body (here a `check` condition) that calls a function is keyed
/// after it, like every other body. (IR's reader refuses a `check` condition
/// that refers to a protocol as a model-state read, `ModelStateRead`
/// `check.condition`: a reader rule, not an ordering one.)
#[test]
fn a_control_body_that_calls_a_function_is_admitted() {
    let text = format!(
        "function ok using v(): Boolean pure {{ true }}\n{}",
        flow("on origin", "", "", "true").replace(
            "run sequence Main { }",
            "run sequence Main { check Gate using v { ok() }; }"
        )
    );
    let lowered = admitted(&text);
    let callee = function(&lowered.wire, "ok");
    let checks: Vec<&Value> = nodes(&lowered.wire)
        .iter()
        .filter(|node| node["semantic_form"] == "control")
        .collect();
    assert!(
        checks
            .iter()
            .any(|node| reaches(&lowered.wire, node, callee)),
        "a control node reaches `ok`"
    );
}

/// FR-464 "Reader order": a function that refers to a protocol whose body
/// calls a function is keyed after that protocol, and a protocol that calls
/// the function after both. The two `finish` binders are named differently,
/// so the reader proves no binder is shared.
#[test]
fn a_chain_of_functions_and_protocols_is_keyed_in_dependency_order() {
    let parts = [
        "function leaf using v(): Boolean pure { true }\n",
        "protocol Inner using v over (input: M::Order) on origin {\n\
         role Buyer on M::Order;\nrun sequence Main { }\n\
         finish End as (inner: Boolean) { leaf() };\n}\n",
        "function mid using v(): Boolean pure { protocol Inner }\n",
        "protocol Outer using v over (input: M::Order) on origin {\n\
         role Buyer on M::Order;\nrun sequence Main { }\n\
         finish End as (outer: Boolean) { mid() };\n}\n",
    ];
    let text = parts.concat();
    let text = text.as_str();
    let lowered = admitted(text);
    let clauses: Vec<&Value> = nodes(&lowered.wire)
        .iter()
        .filter(|node| node["semantic_form"] == "protocol_clause")
        .collect();
    assert_eq!(clauses.len(), 2);
    let (leaf, mid) = (
        function(&lowered.wire, "leaf"),
        function(&lowered.wire, "mid"),
    );
    let inner = clauses
        .iter()
        .find(|clause| reaches(&lowered.wire, clause, leaf) && !reaches(&lowered.wire, clause, mid))
        .expect("Inner reaches leaf alone");
    let outer = clauses
        .iter()
        .find(|clause| reaches(&lowered.wire, clause, mid))
        .expect("Outer reaches mid");
    assert!(reaches(&lowered.wire, mid, inner), "mid reads Inner");
    assert!(reaches(&lowered.wire, outer, inner), "Outer reads Inner");
    assert_ne!(digest(inner), digest(outer));
    // Declaring them in the reverse order changes nothing.
    let reversed: String = parts.iter().rev().copied().collect();
    let clause_ids = |lowered: &Lowered| -> std::collections::BTreeSet<String> {
        nodes(&lowered.wire)
            .iter()
            .filter(|node| node["semantic_form"] == "protocol_clause")
            .map(digest)
            .collect()
    };
    assert_eq!(clause_ids(&admitted(&reversed)), clause_ids(&lowered));
}

/// QSL-844: a component of function calls and protocol references that
/// closes a cycle is a recursion group (FR-383 "Cyclic reference
/// components"), which is not written yet: the protocol refuses naming
/// `ReferenceCycle` through a function that reads it back, whichever body
/// holds the call (a capture, a guard, a `terminal when`, a control or the
/// `finish`), and no clause node is written. The declaration and the
/// function's reference to it are refused, once each.
#[test]
fn a_protocol_in_a_cycle_refuses_naming_the_recursion_group_ticket() {
    let reach = ProtocolConstruct::ReferenceCycle;
    assert_eq!(reach.tracking(), "QSL-844");
    let cycle = CheckCause::ProtocolReference(Box::new(ProtocolReferenceCause::NotEmitted {
        construct: reach,
    }));
    let back = "function back using v(): Boolean pure { protocol Flow }\n";
    let sites = [
        flow("on origin", "capture c: Boolean = back();", "", "true"),
        flow("on each (t: Tick) when (back())", "", "", "true"),
        flow("on origin", "", "terminal when { back() };", "true"),
        flow("on origin", "", "", "back()"),
        flow("on origin", "", "", "true").replace(
            "run sequence Main { }",
            "run sequence Main { check Gate using v { back() }; }",
        ),
    ];
    for site in sites {
        let text = format!("{back}{site}");
        let refusals = checked(&text, false).expect_err(&text);
        assert_eq!(refusals.len(), 2, "{text}: {refusals:?}");
        assert!(refusals.iter().all(|r| r.cause == cycle), "{refusals:?}");
        // The declaration order does not matter.
        let swapped = format!("{site}{back}");
        let again = checked(&swapped, false).expect_err(&swapped);
        assert_eq!(again.len(), 2, "{swapped}: {again:?}");
        assert!(again.iter().all(|r| r.cause == cycle), "{again:?}");
    }
    // A function that only reads a protocol, and a protocol that only calls
    // a function, are not a cycle.
    let acyclic = format!("{back}{}", flow("on origin", "", "", "true"));
    let lowered = admitted(&acyclic);
    assert!(target(
        &lowered.wire,
        &clause(&lowered.wire)["body"]["arguments"][0]["value"]
    )
    .is_object());
}

/// A second protocol, `Other`, whose `finish` binder is named `done` (not
/// `outcome`, the name of `Flow`'s), so that no binder is shared.
const OTHER: &str = "protocol Other using v over (input: M::Order) on origin {\n\
     role Buyer on M::Order;\nrun sequence Main { }\n\
     finish End as (done: Boolean) { true };\n}\n";

/// The five protocol slots that hold an expression, each filled with `body`.
fn slots(body: &str) -> [String; 5] {
    [
        flow(
            "on origin",
            &format!("capture c: Boolean = {body};"),
            "",
            "true",
        ),
        flow(&format!("on each (t: Tick) when ({body})"), "", "", "true"),
        flow(
            "on origin",
            "",
            &format!("terminal when {{ {body} }};"),
            "true",
        ),
        flow("on origin", "", "", body),
        flow("on origin", "", "", "true").replace(
            "run sequence Main { }",
            &format!("run sequence Main {{ check Gate using v {{ {body} }}; }}"),
        ),
    ]
}

/// QSL-1121g, QSpec FR-464 "Binder scope": a protocol reference written
/// directly in a capture, a guard, a `terminal when`, a `finish` or a control
/// body is refused typed, whatever the other protocol's `finish` binder is
/// called: IR's reader follows the dependency closure through the referenced
/// clause to its binder and refuses the body (`BinderScope`, `ModelStateRead`),
/// so QSL writes no such package. Opening the gate writes only `Other`.
#[test]
fn a_protocol_reference_written_in_a_protocol_body_refuses_typed() {
    let construct = ProtocolConstruct::ProtocolReferenceInBody;
    assert_eq!(construct.tracking(), "QSL-1121g");
    let refused =
        CheckCause::ProtocolReference(Box::new(ProtocolReferenceCause::NotEmitted { construct }));
    for site in slots("protocol Other") {
        let text = format!("{OTHER}{site}");
        let refusals = checked(&text, false).expect_err(&text);
        assert_eq!(refusals.len(), 1, "{text}: {refusals:?}");
        assert_eq!(refusals[0].cause, refused, "{text}");
        let open = checked(&text, true).unwrap_or_else(|refusals| panic!("{text}: {refusals:?}"));
        let written = open
            .semantic_graph()
            .nodes()
            .filter(|node| node.semantic_form() == "protocol_clause")
            .count();
        assert_eq!(written, 1, "only Other is written: {text}");
    }
}

/// The same five slots, reading `Other` through a function, are admitted by
/// IR's reader (its closure stops at a `function` node) with `Other`'s
/// `finish` binder named differently from `Flow`'s.
#[test]
fn a_protocol_read_through_a_function_in_a_protocol_body_is_admitted() {
    let rd = "function rd using v(): Boolean pure { protocol Other }\n";
    for site in slots("rd()") {
        let text = format!("{OTHER}{rd}{site}");
        let lowered = admitted(&text);
        let clauses = nodes(&lowered.wire)
            .iter()
            .filter(|node| node["semantic_form"] == "protocol_clause")
            .count();
        assert_eq!(clauses, 2, "{text}");
    }
}
