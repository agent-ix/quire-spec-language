//! FR-218 (TC-663, TC-1056): the checked control tree, the compiled causal
//! edge relation and its two structural rules, the node kinds' own roles
//! and channels, and the `choice` and `repeat` decisions over constant
//! guards.

use ix_trace_rs::trace;
use qsl_forms::ProtocolNodeKind;

use super::{assemble, checks, protocol_causes, refusals};
use crate::check::protocol_clause::{CausalEdge, EdgeKind, Port, PortRef, ProtocolNodeId};
use crate::check::refusal::{ArmFault, ProtocolAnchorCause};
use crate::check::{CheckRefusal, CheckedGraph};

/// A protocol `Flow` with roles `R` and `S` on `M::Actor`, a record type
/// `Plain`, the `members` written before `run` and the `run` control.
fn flow(members: &str, run: &str) -> String {
    format!(
        "record Plain {{ ready: Boolean; }}\n\
         protocol Flow using v over (input: Boolean) on origin {{\n\
         role R on M::Actor;\n\
         role S on M::Actor;\n\
         {members}\n\
         run {run}\n\
         finish End as (outcome: Boolean) {{ true }};\n\
         }}"
    )
}

/// An `attempt` named `name` by `R`, its binder the name in lower case.
fn attempt(name: &str) -> String {
    format!(
        "attempt {name} by R on M::Actor::op contracts [] as ({}: Boolean) {{ true }};",
        name.to_lowercase()
    )
}

/// A channel `name` from `R` to `S` carrying `carries`.
fn channel(name: &str, carries: &str) -> String {
    format!(
        "channel {name} from R to S carries {carries} ordering unordered delivery unknown \
         capacity 2 overflow block;"
    )
}

/// The id of the node `name` of `kind` in the one protocol `source` holds.
fn node(source: &str, name: &str, kind: ProtocolNodeKind) -> ProtocolNodeId {
    let (_, assembled) = assemble(source);
    let index = assembled.protocols[0]
        .declarations
        .iter()
        .position(|declaration| declaration.name.name == name && declaration.kind == kind)
        .unwrap_or_else(|| panic!("a {kind:?} {name}"));
    ProtocolNodeId(index)
}

fn port(node: ProtocolNodeId, port: Port) -> PortRef {
    PortRef { node, port }
}

fn edges(graph: &CheckedGraph) -> &[CausalEdge] {
    &graph.protocol("Flow").expect("the checked protocol").edges
}

/// The protocol causes of `source`'s refusals, with the unit's text.
fn causes(source: &str) -> (String, Vec<ProtocolAnchorCause>) {
    let (text, found) = refusals(source);
    let causes = protocol_causes(&found).into_iter().cloned().collect();
    (text, causes)
}

fn located<'t>(text: &'t str, cause: &ProtocolAnchorCause) -> &'t str {
    let span = cause.span();
    &text[span.start..span.end]
}

fn catalog(refusal: &CheckRefusal) -> (&'static str, Option<&'static str>) {
    (refusal.cause.code().as_str(), refusal.cause.cause())
}

/// TC-663, the edges of ADR-027 §7's `Fill` and of FR-212's `Chatter`
/// (FR-218-AC-1 is not claimed: its replicated-role protocol and bodies
/// that read names are not checked yet). `Fill`'s relation holds in -> `A`,
/// in -> `B`, `A` -> join and `B` -> join; `Chatter`'s holds `Tx`.out ->
/// `Rx`.in.
#[trace("TC-663")]
#[test]
fn the_edge_relation_holds_the_branch_join_and_message_edges() {
    let fill = flow(
        "",
        &format!(
            "sequence Main {{ parallel Both {{ branch left {} branch right {} }} \
             join all [left, right]; }}",
            attempt("A"),
            attempt("B")
        ),
    );
    let graph = checks(&fill);
    let both = node(&fill, "Both", ProtocolNodeKind::Parallel);
    for name in ["A", "B"] {
        let leaf = node(&fill, name, ProtocolNodeKind::Attempt);
        let relation = edges(&graph);
        assert!(relation.contains(&CausalEdge {
            kind: EdgeKind::Branch,
            from: port(both, Port::Enter),
            to: port(leaf, Port::Enter),
        }));
        assert!(relation.contains(&CausalEdge {
            kind: EdgeKind::Join,
            from: port(leaf, Port::Exit),
            to: port(both, Port::Exit),
        }));
    }

    let chatter = flow(
        &channel("ch", "Boolean"),
        "parallel Both { \
         branch s sequence Talk { send Tx via ch as (tx: Boolean) { true }; } \
         branch r receive Rx via ch of s::Talk::Tx as (rx: Boolean) { true }; \
         } join all [s, r];",
    );
    let graph = checks(&chatter);
    assert!(edges(&graph).contains(&CausalEdge {
        kind: EdgeKind::Message,
        from: port(node(&chatter, "Tx", ProtocolNodeKind::Send), Port::Exit),
        to: port(node(&chatter, "Rx", ProtocolNodeKind::Receive), Port::Enter),
    }));
}

/// FR-218-AC-4: a `parallel` nested inside a `repeat` inside a branch of
/// another `parallel`, three levels deep, checks, and the checked tree holds
/// all three levels with their paths.
#[trace("TC-663", "FR-218-AC-4")]
#[test]
fn three_nested_levels_check_with_their_paths() {
    let source = flow(
        "",
        &format!(
            "parallel Outer {{ \
             branch east repeat Loop by R visible () while {{ true }} \
             parallel Inner {{ branch up {} branch down {} }} join all [up, down]; \
             branch west {} \
             }} join all [east, west];",
            attempt("Up"),
            attempt("Down"),
            attempt("West")
        ),
    );
    let graph = checks(&source);
    let tree = &graph.protocol("Flow").expect("the checked protocol").controls;
    for (path, kind) in [
        (&["Outer"][..], ProtocolNodeKind::Parallel),
        (&["Outer", "east", "Loop"][..], ProtocolNodeKind::Repeat),
        (
            &["Outer", "east", "Loop", "Inner"][..],
            ProtocolNodeKind::Parallel,
        ),
        (
            &["Outer", "east", "Loop", "Inner", "up", "Up"][..],
            ProtocolNodeKind::Attempt,
        ),
        (&["Outer", "west", "West"][..], ProtocolNodeKind::Attempt),
    ] {
        let control = tree
            .at_path(path)
            .unwrap_or_else(|| panic!("a node at {path:?}: {tree:?}"));
        assert_eq!(control.kind, kind, "{path:?}");
    }
    let root = tree.root().expect("the root");
    assert_eq!(root.name, "Outer");
    assert_eq!(root.children.len(), 2);
}

/// The one refusal of `source`, with its cause and the unit's text.
fn only(source: &str) -> (String, CheckRefusal, ProtocolAnchorCause) {
    let (text, found) = refusals(source);
    let [refusal] = found.as_slice() else {
        panic!("{source}: one refusal, got {found:?}");
    };
    let cause = protocol_causes(&found)[0].clone();
    (text, refusal.clone(), cause)
}

/// FR-218-AC-5: a `repeat` whose body is one attempt checks with a maximum,
/// with an invariant and variant and no maximum, with both, and with
/// neither, and the checked clause records the maximum, invariant and
/// variant each has; a `repeat` whose body is only a `check`, or a `choice`
/// with an attempt in one case and an empty `sequence` in the other, refuses
/// `unsupported_construct`/`declaration-form` at the `repeat`, naming it.
#[trace("TC-663", "FR-218-AC-5")]
#[test]
fn a_repeat_records_its_parts_and_needs_progress() {
    let body = attempt("Work");
    for (head, max, proof) in [
        ("max 3 while { true }", Some(3), false),
        ("invariant { true } variant { 4 } while { true }", None, true),
        (
            "max 2 invariant { true } variant { 4 } while { true }",
            Some(2),
            true,
        ),
        ("while { true }", None, false),
    ] {
        let exhausted = if max.is_some() {
            " exhausted sequence Done { }"
        } else {
            ""
        };
        let source = flow(
            "",
            &format!("repeat Loop by R visible () {head} {body}{exhausted}"),
        );
        let graph = checks(&source);
        let [repeat] = graph.protocol("Flow").expect("checked").decisions.repeats.as_slice()
        else {
            panic!("one checked repeat");
        };
        assert_eq!(repeat.max, max, "{head}");
        assert_eq!(repeat.loop_proof.is_some(), proof, "{head}");
        assert_eq!(repeat.owner, 0);
    }

    for body in [
        "check Only using v { true };".to_owned(),
        format!(
            "choice Pick by R visible () {{ case work when {{ true }} {} \
             case idle when {{ false }} sequence Idle {{ }} }}",
            attempt("Work")
        ),
    ] {
        let source = flow(
            "",
            &format!("repeat Loop by R visible () while {{ true }} {body}"),
        );
        let (text, refusal, cause) = only(&source);
        assert_eq!(catalog(&refusal), ("unsupported_construct", Some("declaration-form")));
        let ProtocolAnchorCause::NoProgress { repeat, profile, .. } = &cause else {
            panic!("a no-progress refusal, got {cause:?}");
        };
        assert_eq!(repeat, "Loop");
        assert_eq!(*profile, None);
        assert_eq!(located(&text, &cause), "Loop");
    }
}

/// FR-218-AC-6: a `send Tx` on channel `ch` that follows `receive Rx` on the
/// same thread, while `Rx` receives only from `Tx`, refuses
/// `invalid_package`/`definition-cycle` naming `Tx` and `Rx`; with `Tx` on
/// one branch and `Rx` on another the protocol checks.
#[trace("TC-663", "FR-218-AC-6")]
#[test]
fn a_receive_before_its_own_send_is_a_causal_cycle() {
    let source = flow(
        &channel("ch", "Boolean"),
        "sequence Main { receive Rx via ch of Tx as (rx: Boolean) { true }; \
         send Tx via ch as (tx: Boolean) { true }; }",
    );
    let (text, refusal, cause) = only(&source);
    assert_eq!(catalog(&refusal), ("invalid_package", Some("definition-cycle")));
    let ProtocolAnchorCause::CausalCycle { nodes, .. } = &cause else {
        panic!("a cycle, got {cause:?}");
    };
    assert_eq!(nodes, &["Rx".to_owned(), "Tx".to_owned()]);
    assert_eq!(located(&text, &cause), "Rx");

    checks(&flow(
        &channel("ch", "Boolean"),
        "parallel Both { branch s send Tx via ch as (tx: Boolean) { true }; \
         branch r receive Rx via ch of s::Tx as (rx: Boolean) { true }; } join all [s, r];",
    ));
}

/// The `await` named `name`, after `after`, with an empty `then` and
/// `timeout`.
fn await_control(name: &str, after: &str) -> String {
    format!(
        "await {name} after {after} using v clock \"c\" within [0,1] \
         match event Seen{name} by R as (seen{}: Boolean) {{ true }}; \
         then sequence Then{name} {{ }} timeout sequence Timeout{name} {{ }}",
        name.to_lowercase()
    )
}

/// FR-218-AC-7: a `repeat` whose body is a `choice` with an attempt in one
/// case and a nested `repeat` around an attempt in the other refuses
/// `unsupported_construct`/`declaration-form` at the outer `repeat`; a
/// `repeat` whose body is one `await` with a `timeout` control, or one
/// `parallel` with a `join`, checks.
#[trace("TC-663", "FR-218-AC-7")]
#[test]
fn a_skippable_nested_repeat_gives_no_progress() {
    let source = flow(
        "",
        &format!(
            "repeat Outer by R visible () while {{ true }} \
             choice Pick by R visible () {{ \
             case work when {{ true }} {} \
             case more when {{ false }} repeat Inner by R visible () while {{ true }} {} }}",
            attempt("Work"),
            attempt("More")
        ),
    );
    let (text, refusal, cause) = only(&source);
    assert_eq!(catalog(&refusal), ("unsupported_construct", Some("declaration-form")));
    assert!(matches!(&cause, ProtocolAnchorCause::NoProgress { repeat, .. } if repeat == "Outer"));
    assert_eq!(located(&text, &cause), "Outer");

    checks(&flow(
        "",
        &format!(
            "sequence Main {{ {} repeat Loop by R visible () while {{ true }} {} }}",
            attempt("Start"),
            await_control("Wait", "Start")
        ),
    ));
    checks(&flow(
        "",
        &format!(
            "repeat Loop by R visible () while {{ true }} parallel Both {{ \
             branch a {} branch b {} }} join all [a, b];",
            attempt("A"),
            attempt("B")
        ),
    ));
}

/// An `await` anchored at a later control closes a cycle (QSpec FR-464-AC-9
/// as FR-218 states it), and the `repeat` back edge is not one.
#[trace("TC-663")]
#[test]
fn an_await_after_a_later_control_is_a_cycle() {
    let source = flow(
        "",
        &format!(
            "sequence Main {{ {} {} }}",
            await_control("Wait", "Later"),
            attempt("Later")
        ),
    );
    let (_, refusal, cause) = only(&source);
    assert_eq!(catalog(&refusal), ("invalid_package", Some("definition-cycle")));
    let ProtocolAnchorCause::CausalCycle { nodes, .. } = &cause else {
        panic!("a cycle, got {cause:?}");
    };
    assert!(nodes.contains(&"Wait".to_owned()) && nodes.contains(&"Later".to_owned()));

    let graph = checks(&flow(
        "",
        &format!("repeat Loop by R visible () while {{ true }} {}", attempt("Work")),
    ));
    assert!(edges(&graph)
        .iter()
        .any(|edge| edge.kind == EdgeKind::RepeatBack));
}

/// FR-218-AC-11: a `send` via a name that is no channel refuses
/// `missing_declaration`/`missing-name` at the name; a `send` or `receive`
/// whose binder type differs from the channel's `carries` type refuses
/// `ill_typed`/`type-mismatch` at the binder type, retaining channel,
/// expected and actual; the same nodes with the channel's type check, and a
/// `receive` via another channel than its `send`'s still refuses as
/// FR-113-AC-7 states.
#[trace("TC-1056", "FR-218-AC-11")]
#[test]
fn a_message_names_a_channel_and_carries_its_type() {
    let members = format!("{}\n{}", channel("ch", "Plain"), channel("other", "Plain"));
    let pair = |send: &str, receive: &str| {
        flow(
            &members,
            &format!(
                "parallel Both {{ branch s {send} branch r {receive} }} join all [s, r];"
            ),
        )
    };
    let send = "send Tx via ch as (tx: Plain) { true };";
    let receive = "receive Rx via ch of s::Tx as (rx: Plain) { true };";
    checks(&pair(send, receive));

    let (text, refusal, cause) = only(&flow(
        &members,
        "sequence Main { send Tx via nowhere as (tx: Plain) { true }; }",
    ));
    assert_eq!(catalog(&refusal), ("missing_declaration", Some("missing-name")));
    let ProtocolAnchorCause::MissingChannel { node, name, .. } = &cause else {
        panic!("a missing channel, got {cause:?}");
    };
    assert_eq!((node.as_str(), name.as_str()), ("Tx", "nowhere"));
    assert_eq!(located(&text, &cause), "nowhere");

    for (send_type, receive_type, at) in [("Boolean", "Plain", "Boolean"), ("Plain", "Boolean", "Boolean")] {
        let (text, refusal, cause) = only(&pair(
            &send.replacen("(tx: Plain)", &format!("(tx: {send_type})"), 1),
            &receive.replacen("(rx: Plain)", &format!("(rx: {receive_type})"), 1),
        ));
        assert_eq!(catalog(&refusal), ("ill_typed", Some("type-mismatch")));
        let ProtocolAnchorCause::CarriesMismatch {
            channel,
            expected,
            actual,
            ..
        } = &cause
        else {
            panic!("a carries mismatch, got {cause:?}");
        };
        assert_eq!(channel, "ch");
        assert_ne!(expected, actual);
        assert_eq!(*actual, quire_exact::ValueType::Boolean);
        assert_eq!(located(&text, &cause), at);
    }

    let (_, causes) = causes(&pair(send, &receive.replacen("via ch", "via other", 1)));
    assert!(
        causes
            .iter()
            .any(|cause| matches!(cause, ProtocolAnchorCause::ChannelMismatch { .. })),
        "{causes:?}"
    );
}

/// FR-218 "Choice and repeat decisions", the owner and the constant
/// guards (FR-218-AC-17 and AC-19 are not claimed: guards that read visible
/// facts are not checked yet): a `choice` or `repeat` owned by an
/// undeclared role refuses `missing_declaration`/`missing-name` at the
/// owner; constant guards that both hold refuse
/// `undefined_expression`/`unproved-exhaustiveness` with `duplicate-arm`,
/// guards that both fail with `missing-arm`, each naming the cases; one true
/// and one false check and are recorded.
#[trace("TC-1056")]
#[test]
fn a_choice_owner_is_a_role_and_its_constant_guards_partition() {
    let choice = |owner: &str, first: &str, second: &str| {
        flow(
            "",
            &format!(
                "choice Pick by {owner} visible () {{ case yes when {{ {first} }} {} \
                 case no when {{ {second} }} {} }}",
                attempt("Yes"),
                attempt("No")
            ),
        )
    };
    let (text, refusal, cause) = only(&choice("Nobody", "true", "false"));
    assert_eq!(catalog(&refusal), ("missing_declaration", Some("missing-name")));
    let ProtocolAnchorCause::NodeRole { node, name, .. } = &cause else {
        panic!("a node role refusal, got {cause:?}");
    };
    assert_eq!((node.as_str(), name.as_str()), ("Pick", "Nobody"));
    assert_eq!(located(&text, &cause), "Nobody");

    let repeat = flow(
        "",
        &format!("repeat Loop by Nobody visible () while {{ true }} {}", attempt("Work")),
    );
    let (_, _, cause) = only(&repeat);
    assert!(matches!(&cause, ProtocolAnchorCause::NodeRole { node, .. } if node == "Loop"));

    for (first, second, failed, cases) in [
        ("true", "not false", ArmFault::DuplicateArm, vec!["yes", "no"]),
        ("false", "not true", ArmFault::MissingArm, vec!["yes", "no"]),
    ] {
        let (text, refusal, cause) = only(&choice("R", first, second));
        assert_eq!(
            catalog(&refusal),
            ("undefined_expression", Some("unproved-exhaustiveness"))
        );
        let ProtocolAnchorCause::Exhaustiveness {
            choice,
            cases: found,
            failed: faults,
            ..
        } = &cause
        else {
            panic!("an exhaustiveness refusal, got {cause:?}");
        };
        assert_eq!(choice, "Pick");
        assert_eq!(faults, &[failed]);
        assert_eq!(found, &cases);
        assert_eq!(located(&text, &cause), "Pick");
    }

    let graph = checks(&choice("S", "true and true", "false"));
    let [checked] = graph.protocol("Flow").expect("checked").decisions.choices.as_slice() else {
        panic!("one checked choice");
    };
    assert_eq!(checked.owner, 1);
    let names: Vec<&str> = checked.cases.iter().map(|case| case.name.as_str()).collect();
    assert_eq!(names, ["yes", "no"]);
}

/// FR-218 "Constructs": an `event` or `commit` written `by` an undeclared
/// role refuses `missing_declaration`/`missing-name` at the role, and a
/// `check` written `using` a name that is no profile selection refuses as a
/// protocol's `using` does.
#[trace("TC-663")]
#[test]
fn a_node_role_and_a_check_profile_resolve() {
    for (node_text, name) in [
        ("event Seen by Nobody as (seen: Boolean) { true };", "Seen"),
        ("commit Done by Nobody as (done: Boolean) { true };", "Done"),
    ] {
        let source = flow("", &format!("sequence Main {{ {node_text} }}"));
        let (text, refusal, cause) = only(&source);
        assert_eq!(catalog(&refusal), ("missing_declaration", Some("missing-name")));
        assert!(matches!(&cause, ProtocolAnchorCause::NodeRole { node, .. } if node == name));
        assert_eq!(located(&text, &cause), "Nobody");
    }
    let source = flow("", "sequence Main { check Fine using nothing { true }; }");
    let (text, refusal, cause) = only(&source);
    assert_eq!(catalog(&refusal), ("missing_declaration", Some("missing-name")));
    assert!(matches!(cause, ProtocolAnchorCause::MissingProfile { .. }));
    assert_eq!(located(&text, &cause), "nothing");
    checks(&flow("", "sequence Main { check Fine using v { true }; }"));
}

/// A guard, `visible` entry or loop proof that reads a name is not checked
/// yet, so it refuses `unsupported_construct`/`not-yet-implemented` rather
/// than compiling unchecked.
#[trace("TC-1056")]
#[test]
fn a_decision_expression_that_reads_a_name_is_not_compiled_unchecked() {
    for (run, construct) in [
        (
            format!(
                "repeat Loop by R visible () while {{ input }} {}",
                attempt("Work")
            ),
            "repeat guard",
        ),
        (
            format!(
                "repeat Loop by R visible (input) while {{ true }} {}",
                attempt("Work")
            ),
            "visible",
        ),
        (
            format!(
                "repeat Loop by R visible () invariant {{ true }} variant {{ input }} \
                 while {{ true }} {}",
                attempt("Work")
            ),
            "variant",
        ),
        (
            format!(
                "choice Pick by R visible () {{ case yes when {{ input }} {} \
                 case no when {{ not input }} {} }}",
                attempt("Yes"),
                attempt("No")
            ),
            "case guard",
        ),
    ] {
        let (_, refusal, cause) = only(&flow("", &run));
        assert_eq!(catalog(&refusal), ("unsupported_construct", Some("not-yet-implemented")));
        let ProtocolAnchorCause::Unimplemented { construct: found, .. } = &cause else {
            panic!("an unimplemented refusal, got {cause:?}");
        };
        assert_eq!(*found, construct, "{run}");
    }
}
