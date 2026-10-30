// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1061: the `terminal` obligation of a protocol clause (QSpec FR-465
//! "Terminal", FR-464 "Clause body"; FR-383 "Outputs"). Every emitted
//! package is read back through IR's v2 reader, which checks the layout, the
//! condition's type and the binders the `terminal` stage may read.

use super::protocol_graph::{checked, clause, member, model, occurrences, target, PROFILES};
use super::protocol_reference::clause_argument;
use super::reachability::{lower_with, Lowered};
use super::*;
use qsl_semantics::check::{CheckCause, ProtocolConstruct, ProtocolReferenceCause};

/// A record the trigger reads.
const TICK: &str = "record Tick { ready: Boolean; n: Integer; }\n";

/// A protocol `Flow` over `M::Order` with `activation`, `members` before an
/// empty `run`, and a `finish`.
fn flow(activation: &str, members: &str) -> String {
    format!(
        "{TICK}protocol Flow using v over (input: M::Order) {activation} {{\n\
         role Buyer on M::Order;\n\
         {members}\n\
         run sequence Main {{ }}\n\
         finish End as (outcome: Boolean) {{ true }};\n\
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

/// The `terminal` node the clause names, if it names one.
fn terminal(wire: &Value) -> Option<&Value> {
    let named = clause_argument(clause(wire), "terminal");
    let members = named["members"].as_array().unwrap();
    match members.as_slice() {
        [] => None,
        [one] => Some(target(wire, one)),
        _ => panic!("at most one terminal node: {named}"),
    }
}

/// The value of the argument `name` of the application node `node`.
fn argument<'n>(node: &'n Value, name: &str) -> &'n Value {
    node["body"]["arguments"]
        .as_array()
        .unwrap()
        .iter()
        .find(|argument| argument["name"] == name)
        .map(|argument| &argument["value"])
        .unwrap_or_else(|| panic!("an argument {name}: {node}"))
}

/// FR-465 "Terminal": `terminal any` is a node declared at `P`, `terminal`
/// with `mode` the text `any` and `condition` an empty optional, named by the
/// clause's `terminal` argument; the reader admits it.
#[trace("FR-465-AC-7", "FR-383-AC-5", "TC-947")]
#[test]
fn terminal_any_emits_a_node_with_no_condition() {
    let lowered = admitted(&flow("on origin", "terminal any;"));
    let wire = &lowered.wire;
    let node = terminal(wire).expect("the clause names its terminal node");
    assert_eq!(
        (node["node_tag"].as_str(), node["semantic_form"].as_str()),
        (Some("protocol"), Some("obligation"))
    );
    assert_eq!(
        node["declaration"]["qualified_name"],
        json!(["Flow", "terminal"])
    );
    assert_eq!(argument(node, "mode")["value"], "any");
    assert_eq!(
        argument(node, "condition"),
        &json!({"term": "aggregate", "members": []})
    );
    assert_eq!(
        occurrences(&lowered, node)
            .iter()
            .map(|(role, _)| *role)
            .collect::<Vec<_>>(),
        ["declaration"]
    );
}

/// FR-465 "Terminal": `terminal when` is mode `when` with the checked
/// Boolean condition, read over `over`.
#[trace("FR-465-AC-7", "FR-383-AC-5", "TC-947")]
#[test]
fn terminal_when_emits_the_condition_over_the_model_observation() {
    let lowered = admitted(&flow(
        "on origin",
        "terminal when { present(deref(input).trail) };",
    ));
    let node = terminal(&lowered.wire).expect("a terminal node");
    assert_eq!(argument(node, "mode")["value"], "when");
    let condition = argument(node, "condition");
    assert_eq!(condition["term"], "aggregate");
    assert_eq!(condition["members"].as_array().unwrap().len(), 1);
    let over = clause_argument(clause(&lowered.wire), "over");
    assert!(
        super::protocol_graph::reads(&lowered.wire, node, &over["target"]),
        "the condition reads the `over` binder"
    );
}

/// FR-464 stage `terminal`: under `on origin` the condition may read the
/// protocol captures, which are the very binder nodes the clause's `captures`
/// records name.
#[trace("FR-465-AC-7", "TC-947")]
#[test]
fn an_origin_terminal_condition_may_read_a_capture() {
    let lowered = admitted(&format!(
        "{TICK}protocol Flow using v over (input: M::Order) on origin {{\n\
         capture seen: Boolean = true;\n\
         role Buyer on M::Order;\n\
         terminal when {{ seen }};\n\
         run sequence Main {{ }}\n\
         finish End as (outcome: Boolean) {{ true }};\n\
         }}\n"
    ));
    let wire = &lowered.wire;
    let captures = clause_argument(clause(wire), "captures");
    let binder = member(&captures["members"][0], "binder");
    let node = terminal(wire).expect("a terminal node");
    assert!(super::protocol_graph::reads(wire, node, &binder["target"]));
}

/// FR-464 stage `terminal`: under `on each` a condition that reads nothing
/// is admitted. One that reads `over` (TC-947 TM11) is admitted by QSpec but
/// refused by IR's reader (`terminal` stage allows `over` only under
/// `origin`), so QSL refuses it naming the construct, never writing a
/// package the reader rejects.
#[trace("FR-465-AC-7", "TC-947")]
#[test]
fn an_each_terminal_condition_reads_nothing_and_over_is_an_ir_gap() {
    let lowered = admitted(&flow("on each (t: Tick)", "terminal when { true };"));
    let node = terminal(&lowered.wire).expect("a terminal node");
    assert_eq!(argument(node, "mode")["value"], "when");

    let text = flow(
        "on each (t: Tick)",
        "terminal when { present(deref(input).trail) };",
    );
    let refused = checked(&text, false).expect_err("over under each is an IR gap");
    assert!(
        refused.iter().all(|refusal| matches!(
            &refusal.cause,
            CheckCause::ProtocolReference(cause)
                if matches!(**cause, ProtocolReferenceCause::NotEmitted {
                    construct: ProtocolConstruct::TerminalReadsOverUnderEach
                })
        )),
        "{refused:?}"
    );
    assert_eq!(
        ProtocolConstruct::TerminalReadsOverUnderEach.tracking(),
        "IR-1069"
    );
}

/// FR-465 "Terminal", TC-947 TM9: a protocol without a `terminal` member
/// names none, while `any` and `when` each name their own terminal node.
#[trace("FR-465-AC-7", "FR-383-AC-5", "TC-947")]
#[test]
fn the_absent_any_and_when_terminals_are_three_clauses() {
    let none = admitted(&flow("on origin", ""));
    let any = admitted(&flow("on origin", "terminal any;"));
    let when = admitted(&flow("on origin", "terminal when { true };"));
    assert!(terminal(&none.wire).is_none());
    assert!(terminal(&any.wire).is_some() && terminal(&when.wire).is_some());
    assert_ne!(
        terminal(&any.wire).unwrap()["body"],
        terminal(&when.wire).unwrap()["body"]
    );
}
