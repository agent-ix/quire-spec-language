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
    node["node_id"]["digest"].as_str().unwrap().to_owned()
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

/// The clause key and the package identity.
fn identity(lowered: &Lowered) -> (String, String) {
    (
        digest(clause(&lowered.wire)),
        lowered.wire["package_id"]["digest"]
            .as_str()
            .unwrap()
            .to_owned(),
    )
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

/// FR-383-AC-14: a call to a function whose key changes changes the
/// protocol's key and the package's; a reformat and a comment change
/// neither.
#[test]
fn the_clause_key_follows_the_called_function_and_ignores_layout() {
    let unit = |body: &str, pad: &str| {
        format!(
            "function ok using v(): Boolean pure {{ {body} }}\n{}",
            flow("on origin", "", "", &format!("{pad}ok(){pad}"))
        )
    };
    let base = identity(&admitted(&unit("true", "")));
    assert_eq!(base, identity(&admitted(&unit("true", "   "))));
    let commented = format!("// a comment\n{}", unit("true  ", ""));
    assert_eq!(base, identity(&admitted(&commented)));
    let changed = identity(&admitted(&unit("true and true", "")));
    assert_ne!(base.0, changed.0, "the clause key follows the callee");
    assert_ne!(base.1, changed.1, "the package identity follows it");
    // Calling a different function is a different clause.
    let other = format!(
        "function ok using v(): Boolean pure {{ true }}\n\
         function no using v(): Boolean pure {{ false }}\n{}",
        flow("on origin", "", "", "no()")
    );
    assert_ne!(base.0, identity(&admitted(&other)).0);
}

/// FR-464 "Reader order": a protocol that refers to another is keyed after
/// it, and a function that refers to a protocol whose body calls a function
/// is keyed between the two.
#[test]
fn a_chain_of_functions_and_protocols_is_keyed_in_dependency_order() {
    let parts = [
        "function leaf using v(): Boolean pure { true }\n",
        "protocol Inner using v over (input: M::Order) on origin {\n\
         role Buyer on M::Order;\nrun sequence Main { }\n\
         finish End as (outcome: Boolean) { leaf() };\n}\n",
        "function mid using v(): Boolean pure { protocol Inner }\n",
        "protocol Outer using v over (input: M::Order) on origin {\n\
         role Buyer on M::Order;\nrun sequence Main { }\n\
         finish End as (outcome: Boolean) { mid() and protocol Inner };\n}\n",
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
    assert_eq!(
        admitted(&reversed).wire["package_id"],
        lowered.wire["package_id"]
    );
}

/// QSL-844: a component of function calls and protocol references that
/// closes a cycle is a recursion group (FR-383 "Cyclic reference
/// components"), which is not written yet: the protocol refuses naming
/// `ReferenceCycle`, whether it reaches itself or through a function, and
/// no clause node is written.
#[test]
fn a_protocol_in_a_cycle_refuses_naming_the_recursion_group_ticket() {
    let reach = ProtocolConstruct::ReferenceCycle;
    assert_eq!(reach.tracking(), "QSL-844");
    let cycle = CheckCause::ProtocolReference(Box::new(ProtocolReferenceCause::NotEmitted {
        construct: reach,
    }));
    let own = flow("on origin", "", "", "protocol Flow");
    let through = format!(
        "function back using v(): Boolean pure {{ protocol Flow }}\n{}",
        flow("on origin", "", "", "back()")
    );
    let terminal = format!(
        "function back using v(): Boolean pure {{ protocol Flow }}\n{}",
        flow("on origin", "", "terminal when { back() };", "true")
    );
    for text in [own, through, terminal] {
        let refusals = checked(&text, false).expect_err(&text);
        // The declaration, and each reference to it, refuse alike.
        assert!(!refusals.is_empty(), "{text}");
        assert!(refusals.iter().all(|r| r.cause == cycle), "{refusals:?}");
    }
    // A function that only reads a protocol, and a protocol that only calls
    // a function, are not a cycle.
    let acyclic = format!(
        "function back using v(): Boolean pure {{ protocol Flow }}\n{}",
        flow("on origin", "", "", "true")
    );
    let lowered = admitted(&acyclic);
    assert!(target(
        &lowered.wire,
        &clause(&lowered.wire)["body"]["arguments"][0]["value"]
    )
    .is_object());
}
