//! The C1b3 protocol members over fixture R of the protocol obligation
//! tests (FR-211, FR-218): the `terminal when` predicate, replicated roles
//! and the binder reader sets with the `scope` region.

use ix_trace_rs::trace;
use qsl_forms::build_unit;
use qsl_foundation::SourceIdentity;
use quire_exact::ValueType;
use quire_semantic_value::checking::CheckingLimits;

use super::{CheckedLifetime, CheckedTerminalMember};
use crate::check::protocol_clause::ProtocolNodeId;
use crate::check::refusal::{ProtocolAnchorCause, SlotKind};
use crate::check::{CheckCause, CheckRefusal, CheckedGraph, PackageDeclarations};

mod fixture {
    use crate as semantics;
    include!("../../../tests/fixtures/protocol_obligations.rs");
}
use fixture::{obligation_model_with, HEADER};

/// ADR-027 §7's `Fill` over `k: M::Node`, activated `activation`, with
/// `members` written before `run`.
fn fill(activation: &str, members: &str) -> String {
    format!(
        "{HEADER}protocol Fill using v over (k: M::Node) {activation} {{\n\
         capture start: M::Total = k.total;\n\
         role w on M::Node;\n\
         {members}\n\
         run sequence Main {{\n\
         parallel Both {{\n\
         branch left attempt A by w on M::Node::step contracts [] as (a: M::Flag) {{ true }};\n\
         branch right attempt B by w on M::Node::step contracts [] as (b: M::Flag) {{ true }};\n\
         }} join all [left, right];\n\
         }}\n\
         finish Done as (d: M::Receipt) {{ true }};\n\
         }}\n"
    )
}

const ORIGIN: &str = "on origin";
const EACH: &str = "on each (opened: M::Tick) when (opened.ready)";

/// Fixture R with the populations `nodes` of its `Node`s and `jobs` of a
/// second object type `Job`.
fn obligation_model() -> crate::model::intake::SelectedModel {
    use crate::model::domain_package::{
        DomainPackageRecord, Extent, ObjectTypeRecord, PopulationRecord,
    };
    use crate::model::key::DeclarationKey;
    let key = |name: &str| DeclarationKey {
        package: "example/protocol-obligations".to_owned(),
        node: format!("ix://example/protocol-obligations/{name}"),
    };
    let population = |name: &str, member: &str| {
        DomainPackageRecord::Population(PopulationRecord {
            key: key(name),
            member_types: vec![key(member)],
            extent: Extent::Closed,
            symmetric: false,
        })
    };
    obligation_model_with(vec![
        DomainPackageRecord::ObjectType(ObjectTypeRecord {
            key: key("Job"),
            interface_features: None,
            abstract_type: false,
            supertypes: Vec::new(),
        }),
        population("nodes", "Node"),
        population("jobs", "Job"),
    ])
}

fn check(text: &str) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        vec![obligation_model()],
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("{text}: the assembler refuses: {refusal:?}"))
    .check(CheckingLimits::default())
}

fn checks(text: &str) -> CheckedGraph {
    check(text).unwrap_or_else(|refusals| panic!("{text}: {refusals:?}"))
}

/// The one refusal's cause, its catalog pair and the text at its span.
fn only(
    text: &str,
) -> (
    ProtocolAnchorCause,
    (&'static str, Option<&'static str>),
    &str,
) {
    let refusals = check(text).map(|_| ()).expect_err("the protocol refuses");
    let [refusal] = refusals.as_slice() else {
        panic!("{text}: one refusal, got {refusals:?}");
    };
    let CheckCause::ProtocolAnchor(cause) = &refusal.cause else {
        panic!("a protocol cause, got {refusal:?}");
    };
    let span = cause.span();
    (
        (**cause).clone(),
        (refusal.cause.code().as_str(), refusal.cause.cause()),
        &text[span.start..span.end],
    )
}

fn terminal(graph: &CheckedGraph) -> &CheckedTerminalMember {
    &graph
        .protocol("Fill")
        .expect("the checked protocol")
        .slots
        .terminal
}

/// FR-211 "The `terminal` member" (FR-211-AC-1's checking half; its
/// package-identity half waits on FR-383's emission of the member): `Fill`
/// checks with no `terminal` member, with `terminal when` a Boolean
/// predicate over `k`'s field and with `terminal any`, and records each.
/// A predicate that reads binder `a` refuses `missing_declaration`/
/// `missing-name` at the read, as FR-104 refuses a non-state read in an
/// invariant, and a non-Boolean predicate refuses `non-boolean-root`.
#[trace("TC-656")]
#[test]
fn a_terminal_predicate_reads_the_model_observation_and_the_captures() {
    assert_eq!(
        terminal(&checks(&fill(ORIGIN, ""))),
        &CheckedTerminalMember::None
    );
    assert_eq!(
        terminal(&checks(&fill(ORIGIN, "terminal any;"))),
        &CheckedTerminalMember::Any
    );
    let graph = checks(&fill(
        ORIGIN,
        "terminal when { k.total = 1 and start >= 0 };",
    ));
    let CheckedTerminalMember::When(predicate) = terminal(&graph) else {
        panic!("a `when` member");
    };
    assert_eq!(predicate.root().value_type(), &ValueType::Boolean);

    let text = fill(ORIGIN, "terminal when { a.ready };");
    let (cause, pair, at) = only(&text);
    assert_eq!(pair, ("missing_declaration", Some("missing-name")));
    let ProtocolAnchorCause::SlotClause {
        node, slot, cause, ..
    } = &cause
    else {
        panic!("a slot refusal, got {cause:?}");
    };
    assert_eq!((node.as_str(), *slot), ("Fill", SlotKind::Terminal));
    assert_eq!(**cause, CheckCause::MissingName("a".to_owned()));
    assert_eq!(at, "a");

    let (cause, pair, _) = only(&fill(ORIGIN, "terminal when { k.total };"));
    assert_eq!(pair, ("ill_typed", Some("non-boolean-root")));
    assert!(matches!(
        cause,
        ProtocolAnchorCause::SlotClause {
            slot: SlotKind::Terminal,
            ..
        }
    ));
}

/// QSpec FR-429 "The `terminal` member": under `activation on each` the
/// predicate has no instance binder, so a read of a protocol capture or of
/// the activation binder refuses `missing_declaration`/`missing-name` at the
/// read; the `over` input is still read.
#[trace("TC-656")]
#[test]
fn an_on_each_terminal_predicate_reads_no_capture_and_no_activation_binder() {
    checks(&fill(EACH, "terminal when { k.total = 1 };"));
    for (read, name) in [("start >= 0", "start"), ("opened.ready", "opened")] {
        let text = fill(EACH, &format!("terminal when {{ {read} }};"));
        let (cause, pair, at) = only(&text);
        assert_eq!(
            pair,
            ("missing_declaration", Some("missing-name")),
            "{read}"
        );
        let ProtocolAnchorCause::SlotClause { cause, .. } = &cause else {
            panic!("a slot refusal, got {cause:?}");
        };
        assert_eq!(**cause, CheckCause::MissingName(name.to_owned()));
        assert_eq!(at, name);
    }
}

/// FR-208-AC-1's protocol over fixture R: a replicated role `r` drawn from
/// `nodes`, with `lifetime`, and an `event Work by r` inside `sequence S`,
/// followed by a `check` outside it.
fn workers(lifetime: &str) -> String {
    format!(
        "{HEADER}protocol Workers using v over (k: M::Node) on origin {{\n\
         capture start: M::Total = k.total;\n\
         role w on M::Node;\n\
         role r each M::Node from nodes max 1 lifetime {lifetime};\n\
         run sequence Main {{\n\
         sequence S {{ event Work by r as (worked: M::Flag) {{ true }}; }}\n\
         check After using v {{ true }};\n\
         }}\n\
         finish Done as (d: M::Receipt) {{ true }};\n\
         }}\n"
    )
}

/// The id of the node `name` of the one protocol `text` declares.
fn node_id(text: &str, name: &str) -> ProtocolNodeId {
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    let assembled = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        vec![obligation_model()],
        Vec::new(),
    )
    .expect("the assembler admits the unit");
    let index = assembled.protocols[0]
        .declarations
        .iter()
        .position(|declaration| declaration.name.name == name)
        .unwrap_or_else(|| panic!("a node {name}"));
    ProtocolNodeId(index)
}

/// FR-218 "Scopes" and QSpec FR-465 "Role": FR-208-AC-1's replicated role
/// checks with each lifetime and records its population, maximum and
/// lifetime; its `until(p)` is a Boolean over `C0` and the role's own
/// binder, typed `Reference` to its object type, so a read of the instance
/// and of the `over` input checks, a read of a `run`-tree binder refuses
/// `missing_declaration`/`missing-name` at the read and a non-Boolean
/// predicate `non-boolean-root`; `max 0` refuses `ill_typed`/`type-mismatch`
/// at the maximum.
#[trace("TC-653")]
#[test]
fn a_replicated_role_records_its_population_maximum_and_lifetime() {
    let graph = checks(&workers("until(r.total > 0 and k.total >= start)"));
    let protocol = graph.protocol("Workers").expect("checked");
    let [role] = protocol.slots.roles.as_slice() else {
        panic!("one replicated role");
    };
    assert_eq!(
        (role.role, role.population.node.as_str(), role.max),
        (1, "ix://example/protocol-obligations/nodes", 1)
    );
    let CheckedLifetime::Until(until) = &role.lifetime else {
        panic!("an until lifetime, got {:?}", role.lifetime);
    };
    assert_eq!(until.root().value_type(), &ValueType::Boolean);
    let graph = checks(&workers("workflow"));
    assert_eq!(
        graph.protocol("Workers").expect("checked").slots.roles[0].lifetime,
        CheckedLifetime::Workflow
    );

    let text = workers("until(worked.ready)");
    let (cause, pair, at) = only(&text);
    assert_eq!(pair, ("missing_declaration", Some("missing-name")));
    let ProtocolAnchorCause::SlotClause {
        node, slot, cause, ..
    } = &cause
    else {
        panic!("a slot refusal, got {cause:?}");
    };
    assert_eq!((node.as_str(), *slot), ("r", SlotKind::Lifetime));
    assert_eq!(**cause, CheckCause::MissingName("worked".to_owned()));
    assert_eq!(at, "worked");

    let (_, pair, _) = only(&workers("until(r.total)"));
    assert_eq!(pair, ("ill_typed", Some("non-boolean-root")));

    let text = workers("workflow").replacen("max 1", "max 0", 1);
    let (cause, pair, at) = only(&text);
    assert_eq!(pair, ("ill_typed", Some("type-mismatch")));
    assert_eq!(
        cause,
        ProtocolAnchorCause::RoleMax {
            role: "r".to_owned(),
            minimum: 1,
            actual: 0,
            span: cause.span(),
        }
    );
    assert_eq!(at, "0");
}

/// FR-218-AC-3: in `Fill` with a `check` after the join that reads `a`, the
/// checked clause records that `check` as `a`'s only reader, and a node in
/// branch `right` that reads `a` refuses `missing_declaration`/
/// `missing-name` as FR-113 refuses an unresolved name; in FR-208-AC-1's
/// protocol with lifetime `scope`, the recorded region is `S`, the
/// smallest control holding `Work`, and with a second `by r` node after `S`
/// it is `Main`.
#[trace("TC-663", "FR-218-AC-3")]
#[test]
fn readers_are_recorded_and_a_scope_role_records_its_region() {
    let read_after = fill(ORIGIN, "").replacen(
        "} join all [left, right];\n",
        "} join all [left, right];\ncheck Read using v { a.ready };\n",
        1,
    );
    let graph = checks(&read_after);
    let readers = &graph.protocol("Fill").expect("checked").slots.readers;
    let a = readers
        .iter()
        .find(|readers| readers.binder == "a")
        .expect("a's readers");
    assert_eq!(a.readers, [node_id(&read_after, "Read")]);

    let text =
        fill(ORIGIN, "").replacen("as (b: M::Flag) { true }", "as (b: M::Flag) { a.ready }", 1);
    let (cause, pair, at) = only(&text);
    assert_eq!(pair, ("missing_declaration", Some("missing-name")));
    let ProtocolAnchorCause::SlotClause { node, cause, .. } = &cause else {
        panic!("a slot refusal, got {cause:?}");
    };
    assert_eq!(node, "B");
    assert_eq!(**cause, CheckCause::MissingName("a".to_owned()));
    assert_eq!(at, "a");

    let text = workers("scope");
    let graph = checks(&text);
    assert_eq!(
        graph.protocol("Workers").expect("checked").slots.roles[0].lifetime,
        CheckedLifetime::Scope(Some(node_id(&text, "S")))
    );
    let text = workers("scope").replacen(
        "check After using v { true };",
        "event Again by r as (again: M::Flag) { true };",
        1,
    );
    let graph = checks(&text);
    assert_eq!(
        graph.protocol("Workers").expect("checked").slots.roles[0].lifetime,
        CheckedLifetime::Scope(Some(node_id(&text, "Main")))
    );
}

/// FR-218-AC-28 (TC-663): `role w each M::Worker from workers` checks when
/// `workers` is a population of the role's type and the checked role holds
/// that population; an undeclared name refuses `missing_declaration`/
/// `missing-name` and a population of another type `ill_typed`/
/// `type-mismatch`, each at the name and retaining the role and population.
#[trace("TC-663", "FR-218-AC-28")]
#[test]
fn a_replicated_roles_population_resolves_in_the_package_of_its_type() {
    let graph = checks(&workers("workflow"));
    let [role] = graph
        .protocol("Workers")
        .expect("checked")
        .slots
        .roles
        .as_slice()
    else {
        panic!("one replicated role");
    };
    assert_eq!(
        role.population.node,
        "ix://example/protocol-obligations/nodes"
    );

    for (name, pair, missing) in [
        (
            "ghosts",
            ("missing_declaration", Some("missing-name")),
            true,
        ),
        ("jobs", ("ill_typed", Some("type-mismatch")), false),
    ] {
        let text = workers("workflow").replacen("from nodes", &format!("from {name}"), 1);
        let (cause, found, at) = only(&text);
        assert_eq!(found, pair, "{name}");
        assert_eq!(at, name);
        let expected = if missing {
            ProtocolAnchorCause::MissingRolePopulation {
                role: "r".to_owned(),
                population: name.to_owned(),
                span: cause.span(),
            }
        } else {
            ProtocolAnchorCause::RolePopulationType {
                role: "r".to_owned(),
                population: name.to_owned(),
                span: cause.span(),
            }
        };
        assert_eq!(cause, expected);
    }
}

/// FR-218-AC-29 (TC-663): with channels `from w to c` and `from c to w` the
/// checked clause holds the owner `w` for the `send` and `c` for the
/// `receive` on the first, and `c` for the `send` and `w` for the `receive`
/// on the second; a `scope` role `w` with its `Work` inside `sequence S` and
/// a `send` from `w` after `S` has the region `Main`, `S`'s parent control,
/// not `S`.
#[trace("TC-663", "FR-218-AC-29")]
#[test]
fn send_and_receive_are_owned_by_their_channel_roles_and_count_in_the_region() {
    let text = format!(
        "{HEADER}protocol Duplex using v over (k: M::Node) on origin {{\n\
         role c on M::Node;\n\
         role w each M::Node from nodes max 1 lifetime workflow;\n\
         channel out from w to c carries M::Flag ordering unordered delivery unknown \
         capacity 2 overflow block;\n\
         channel back from c to w carries M::Flag ordering unordered delivery unknown \
         capacity 2 overflow block;\n\
         terminal any;\n\
         run sequence Main {{\n\
         parallel Go {{\n\
           branch a send Tx via out as (tx: M::Flag) {{ true }};\n\
           branch b receive Rx via out of a::Tx as (rx: M::Flag) {{ true }};\n\
         }} join all [a, b];\n\
         parallel Return {{\n\
           branch d send Ty via back as (ty: M::Flag) {{ true }};\n\
           branch e receive Ry via back of d::Ty as (ry: M::Flag) {{ true }};\n\
         }} join all [d, e];\n\
         }}\n\
         finish Done as (f: M::Receipt) {{ true }};\n\
         }}\n"
    );
    let graph = checks(&text);
    let protocol = graph.protocol("Duplex").expect("checked");
    let owner = |node: &str| {
        let node = node_id(&text, node);
        protocol
            .owners
            .iter()
            .find(|owner| owner.node == node)
            .map(|owner| protocol.roles[owner.role].name.as_str())
    };
    assert_eq!(
        [owner("Tx"), owner("Rx"), owner("Ty"), owner("Ry")],
        [Some("w"), Some("c"), Some("c"), Some("w")]
    );

    let scoped = workers("scope")
        .replacen(
            "run sequence Main",
            "channel ch from r to w carries M::Flag ordering unordered delivery unknown \
             capacity 2 overflow block;\nrun sequence Main",
            1,
        )
        .replacen(
            "check After using v { true };",
            "send Out via ch as (o: M::Flag) { true };",
            1,
        );
    let graph = checks(&scoped);
    assert_eq!(
        graph.protocol("Workers").expect("checked").slots.roles[0].lifetime,
        CheckedLifetime::Scope(Some(node_id(&scoped, "Main")))
    );
}

/// ADR-027 RR-2, RR-4: the checked protocol records the role that owns each
/// node a role acts at: in FR-208-AC-1's protocol, `Work` is owned by the
/// replicated role `r`, and the `check` and the `finish` node by none.
#[trace("TC-653")]
#[test]
fn a_node_records_the_role_that_owns_it() {
    let text = workers("workflow");
    let graph = checks(&text);
    let protocol = graph.protocol("Workers").expect("checked");
    assert_eq!(
        protocol.owners,
        [crate::check::CheckedOwner {
            node: node_id(&text, "Work"),
            role: 1,
        }]
    );
    assert_eq!(protocol.roles[1].name, "r");
    assert!(protocol.roles[1].replicated);
}

/// ADR-027 §7.1's `Pay` over fixture R: an attempt, its effect with a
/// refund template, and a failure event.
const PAY: &str = "protocol Pay using v over (k: M::Node) on origin {\n\
    role w on M::Node;\n\
    compensate Refund for Main::Paid as (fwd: M::Receipt)\n\
        by w on M::Node::undo using v clock \"c\" {\n\
      activate first (f: M::Flag) when { f.ready } { }\n\
      within [0, 10];\n\
      attempts 1 of M::Attempt;\n\
      retry (earlier: M::Attempt, later: M::Attempt) { false };\n\
      commit never;\n\
      recover (recovered: M::Receipt) { recovered.n = 0 };\n\
    }\n\
    run sequence Main {\n\
      attempt C by w on M::Node::step contracts [] as (c: M::Flag) { true };\n\
      effect Paid of Main::C as (pd: M::Receipt) { true };\n\
      event Fail by w as (failure: M::Flag) { true };\n\
    }\n\
    finish Done as (d: M::Receipt) { true };\n\
    }\n";

/// FR-212's `Chatter` over fixture R: a send and an attempt in one branch,
/// the receive in the other, over a channel of capacity 2.
const CHATTER: &str = "protocol Chatter using v over (k: M::Node) on origin {\n\
    role w on M::Node;\n\
    channel ch from w to w carries M::Flag ordering unordered delivery unknown \
    capacity 2 overflow block;\n\
    terminal any;\n\
    run parallel Both {\n\
      branch s sequence Talk { send Tx via ch as (tx: M::Flag) { true };\n\
        attempt Flip by w on M::Node::step contracts [] as (flipped: M::Flag) { true }; }\n\
      branch r receive Rx via ch of s::Talk::Tx as (rx: M::Flag) { true };\n\
    } join all [s, r];\n\
    finish Done as (d: M::Receipt) { true };\n\
    }\n";

/// FR-218-AC-1: ADR-027 §7's `Fill`, §7.1's `Pay`, FR-212's `Chatter` and
/// FR-208-AC-1's replicated-role protocol each check to a checked clause,
/// none refusing `unsupported_construct`/`not-yet-implemented`; `Fill`'s
/// relation holds in -> `A`, in -> `B`, `A` -> join and `B` -> join, and
/// `Chatter`'s `Tx`.out -> `Rx`.in.
#[trace("TC-663", "FR-218-AC-1")]
#[test]
fn the_reference_protocols_check_with_their_edges() {
    use crate::check::protocol_clause::{CausalEdge, EdgeKind, Port, PortRef};
    let port = |node, port| PortRef { node, port };

    let fill_text = fill(ORIGIN, "");
    let graph = checks(&fill_text);
    let both = node_id(&fill_text, "Both");
    let edges = &graph.protocol("Fill").expect("checked").edges;
    for leaf in ["A", "B"] {
        let leaf = node_id(&fill_text, leaf);
        assert!(edges.contains(&CausalEdge {
            kind: EdgeKind::Branch,
            from: port(both, Port::Enter),
            to: port(leaf, Port::Enter),
        }));
        assert!(edges.contains(&CausalEdge {
            kind: EdgeKind::Join,
            from: port(leaf, Port::Exit),
            to: port(both, Port::Exit),
        }));
    }

    let pay = format!("{HEADER}{PAY}");
    let graph = checks(&pay);
    assert_eq!(
        graph
            .protocol("Pay")
            .expect("checked")
            .obligations
            .compensations
            .len(),
        1
    );

    let chatter = format!("{HEADER}{CHATTER}");
    let graph = checks(&chatter);
    assert!(graph
        .protocol("Chatter")
        .expect("checked")
        .edges
        .contains(&CausalEdge {
            kind: EdgeKind::Message,
            from: port(node_id(&chatter, "Tx"), Port::Exit),
            to: port(node_id(&chatter, "Rx"), Port::Enter),
        }));

    let graph = checks(&workers("until(r.total > 0)"));
    assert_eq!(
        graph
            .protocol("Workers")
            .expect("checked")
            .slots
            .roles
            .len(),
        1
    );
}
