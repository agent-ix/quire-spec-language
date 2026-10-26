// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1021, TC-1061: the roles, channels and queues of a
//! protocol clause (QSpec FR-464 "Clause body", FR-465 "Role", "Channel",
//! "Queue"; FR-383 "Outputs"). Every emitted package is read back through
//! IR's v2 reader, which checks the layouts, declaration paths, set order,
//! binder types and the one-queue-per-channel membership rule.

use super::population_bound::document;
use super::protocol_reference::{clause_argument, node_by_id, node_of, not_emitted};
use super::reachability::{lower_with, Lowered};
use super::*;
use qsl_semantics::check::{CheckRefusal, ProtocolConstruct};

const PROFILES: &str = "profile p = \"quire.protocol.finite-global/v1\";\n";

/// The `acme/orders` document of TC-1054 (`Order`, `Rush`, `Person` and the
/// populations `orders` and `people`).
fn model() -> Vec<u8> {
    document()
}

/// A protocol `Flow` with `activation` and `members` before an empty `run`
/// and a `finish`.
fn flow(activation: &str, members: &str) -> String {
    format!(
        "protocol Flow using v over (input: Boolean) {activation} {{\n\
         {members}\n\
         run sequence Main {{ }}\n\
         finish End as (outcome: Boolean) {{ true }};\n\
         }}\n"
    )
}

const BUYER: &str = "role Buyer on M::Order;";
const SELLER: &str = "role Seller on M::Person;";

/// `flow` over [`model`], emitted and read back by IR; the reader must
/// admit it.
fn emitted(activation: &str, members: &str) -> Lowered {
    let lowered = lower_with(model(), PROFILES, &flow(activation, members));
    assert!(
        lowered.ir.is_ok(),
        "{:?}\n{}",
        lowered.ir.err(),
        lowered.text
    );
    assert!(lowered.verified, "{}", lowered.text);
    lowered
}

/// One `channel` member.
fn channel(name: &str, ends: (&str, &str), ordering: &str, delivery: &str, bound: &str) -> String {
    format!(
        "channel {name} from {} to {} carries Boolean ordering {ordering} \
         delivery {delivery} {bound};",
        ends.0, ends.1
    )
}

fn clause(wire: &Value) -> &Value {
    node_of(wire, "protocol", "protocol_clause")
}

/// The node the reference `term` names.
fn target<'w>(wire: &'w Value, term: &Value) -> &'w Value {
    assert_eq!(term["term"], "reference", "{term}");
    node_by_id(wire, &term["target"])
}

/// The nodes the set argument `name` of the clause lists, in order.
fn set<'w>(wire: &'w Value, name: &str) -> Vec<&'w Value> {
    clause_argument(clause(wire), name)["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|term| target(wire, term))
        .collect()
}

/// The digests of `nodes`, which must be strictly ascending.
fn ascending(nodes: &[&Value]) -> Vec<String> {
    let digests: Vec<String> = nodes
        .iter()
        .map(|node| node["node_id"]["digest"].as_str().unwrap().to_owned())
        .collect();
    assert!(
        digests.windows(2).all(|pair| pair[0] < pair[1]),
        "strictly ascending: {digests:?}"
    );
    digests
}

/// The value of the argument `name` of `node`, after checking the
/// operator's argument names and order.
fn argument<'n>(node: &'n Value, names: &[&str], name: &str) -> &'n Value {
    let arguments = node["body"]["arguments"].as_array().unwrap();
    let actual: Vec<&str> = arguments
        .iter()
        .map(|argument| argument["name"].as_str().unwrap())
        .collect();
    assert_eq!(actual, names);
    arguments
        .iter()
        .find(|argument| argument["name"] == name)
        .map(|argument| &argument["value"])
        .unwrap()
}

const CHANNEL: [&str; 5] = ["from", "to", "carries", "ordering", "delivery"];
const QUEUE: [&str; 4] = ["channel", "capacity", "overflow", "loss"];
const ROLE: [&str; 2] = ["model", "replication"];

/// The text of a `literal` term.
fn lit(term: &Value) -> &str {
    assert_eq!(term["term"], "literal", "{term}");
    term["value"].as_str().unwrap()
}

/// The `name` member of the Group `group`.
fn member<'g>(group: &'g Value, name: &str) -> &'g Value {
    assert_eq!(group["term"], "aggregate", "{group}");
    group["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|member| member["name"] == name)
        .unwrap_or_else(|| panic!("a member {name}: {group}"))
        .get("value")
        .unwrap()
}

/// The names of the members of the Group `group`, in order.
fn member_names(group: &Value) -> Vec<&str> {
    group["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|member| member["name"].as_str().unwrap_or(""))
        .collect()
}

/// The `(role, region text)` of each occurrence of `node`.
fn occurrences<'l>(lowered: &'l Lowered, node: &Value) -> Vec<(&'l str, &'l str)> {
    lowered.wire["source_map"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["node_id"] == node["node_id"])
        .map(|entry| {
            let region = &entry["regions"][0];
            let start = usize::try_from(region["start"].as_u64().unwrap()).unwrap();
            let end = usize::try_from(region["end"].as_u64().unwrap()).unwrap();
            (entry["role"].as_str().unwrap(), &lowered.text[start..end])
        })
        .collect()
}

/// The one `queue` node of `wire` that bounds `channel`.
fn queue_of<'w>(wire: &'w Value, channel: &Value) -> &'w Value {
    let queues: Vec<&Value> = nodes(wire)
        .iter()
        .filter(|node| node["semantic_form"] == "queue")
        .filter(|node| argument(node, &QUEUE, "channel")["target"] == channel["node_id"])
        .collect();
    let [queue] = queues[..] else {
        panic!("one queue of the channel: {queues:?}");
    };
    queue
}

/// FR-465 "Channel", "Queue", FR-383-AC-4: a channel emits its two role
/// nodes, its message type, its ordering Group and its delivery text, each in
/// FR-465's order; each of the four delivery spellings and each overflow
/// policy emits; a `loss` selector exists exactly for overflow `loss` and
/// the three selectors give three queue keys; a queue has no declaration and
/// one `generated` occurrence at its channel's declaration; and IR's reader
/// admits every package.
#[trace("FR-383-AC-4", "TC-1061")]
#[test]
fn a_channel_and_its_queue_emit_every_premise() {
    let ends = ("Buyer", "Seller");
    let source = |ordering: &str, delivery: &str, bound: &str| {
        channel("Pay", ends, ordering, delivery, bound)
    };
    let members = |channel: &str| format!("{BUYER}\n{SELLER}\n{channel}");

    for delivery in [
        "unknown",
        "at-most-once",
        "at-least-once",
        "exactly-once-premise",
    ] {
        let text = source("unordered", delivery, "capacity 4 overflow block");
        let lowered = emitted("on origin", &members(&text));
        let channels = set(&lowered.wire, "channels");
        let [pay] = channels[..] else {
            panic!("one channel: {channels:?}");
        };
        assert_eq!(pay["semantic_form"], "channel");
        assert_eq!(
            pay["declaration"]["qualified_name"],
            json!(["Flow", "channel", "Pay"])
        );
        assert_eq!(pay["body"]["operation"]["member"]["operator"], "channel");
        let from = target(&lowered.wire, argument(pay, &CHANNEL, "from"));
        let to = target(&lowered.wire, argument(pay, &CHANNEL, "to"));
        assert_eq!(
            from["declaration"]["qualified_name"],
            json!(["Flow", "role", "Buyer"])
        );
        assert_eq!(
            to["declaration"]["qualified_name"],
            json!(["Flow", "role", "Seller"])
        );
        let carries = target(&lowered.wire, argument(pay, &CHANNEL, "carries"));
        assert_eq!(
            (
                carries["node_tag"].as_str(),
                carries["semantic_form"].as_str()
            ),
            (Some("scalar_type"), Some("boolean"))
        );
        let ordering = argument(pay, &CHANNEL, "ordering");
        assert_eq!(member_names(ordering), ["kind"]);
        assert_eq!(lit(member(ordering, "kind")), "unordered");
        assert_eq!(lit(argument(pay, &CHANNEL, "delivery")), delivery);
        assert_eq!(occurrences(&lowered, pay), [("declaration", text.as_str())]);
    }

    // A FIFO channel carries its message binder and key.
    let fifo = source(
        "fifo by (m: Boolean) { not m }",
        "unknown",
        "capacity 1 overflow reject",
    );
    let lowered = emitted("on origin", &members(&fifo));
    let [pay] = set(&lowered.wire, "channels")[..] else {
        panic!("one channel");
    };
    let ordering = argument(pay, &CHANNEL, "ordering");
    assert_eq!(member_names(ordering), ["kind", "binder", "key"]);
    assert_eq!(lit(member(ordering, "kind")), "fifo");
    let binder = target(&lowered.wire, member(ordering, "binder"));
    assert_eq!(binder["semantic_form"], "parameter");
    assert_eq!(
        binder["semantic_type"],
        argument(pay, &CHANNEL, "carries")["target"]
    );
    let key = target(&lowered.wire, member(ordering, "key"));
    assert!(
        reads(&lowered.wire, key, &binder["node_id"]),
        "the key reads the message binder"
    );

    // One queue of each overflow policy and capacity.
    let mut keys = BTreeMap::new();
    for (bound, capacity, overflow, loss) in [
        (
            "capacity 4 overflow reject",
            ("literal", "value", "4"),
            "reject",
            None,
        ),
        (
            "capacity 0 overflow block",
            ("literal", "value", "0"),
            "block",
            None,
        ),
        (
            "capacity symbolic(Bound) overflow loss head",
            ("symbolic", "name", "Bound"),
            "loss",
            Some("head"),
        ),
        (
            "capacity 4 overflow loss tail",
            ("literal", "value", "4"),
            "loss",
            Some("tail"),
        ),
        (
            "capacity 4 overflow loss new",
            ("literal", "value", "4"),
            "loss",
            Some("new"),
        ),
    ] {
        let text = source("unordered", "at-most-once", bound);
        let lowered = emitted("on origin", &members(&text));
        let queues = set(&lowered.wire, "queues");
        let [queue] = queues[..] else {
            panic!("one queue: {queues:?}");
        };
        let [pay] = set(&lowered.wire, "channels")[..] else {
            panic!("one channel");
        };
        assert_eq!(queue, queue_of(&lowered.wire, pay));
        assert_eq!(queue["body"]["operation"]["member"]["operator"], "queue");
        assert!(queue["declaration"].is_null(), "a queue has no declaration");
        assert_eq!(queue["dependencies"], json!([pay["node_id"]]));
        let group = argument(queue, &QUEUE, "capacity");
        assert_eq!(member_names(group), ["kind", capacity.1]);
        assert_eq!(lit(member(group, "kind")), capacity.0);
        assert_eq!(lit(member(group, capacity.1)), capacity.2);
        assert_eq!(lit(argument(queue, &QUEUE, "overflow")), overflow);
        let selector = argument(queue, &QUEUE, "loss")["members"]
            .as_array()
            .unwrap();
        assert_eq!(
            selector.iter().map(lit).collect::<Vec<_>>(),
            loss.into_iter().collect::<Vec<_>>()
        );
        assert_eq!(
            occurrences(&lowered, queue),
            [("generated", text.as_str())],
            "one generated occurrence at the channel's declaration"
        );
        keys.insert(
            bound,
            queue["node_id"]["digest"].as_str().unwrap().to_owned(),
        );
    }
    let selectors: Vec<&String> = [
        "capacity 4 overflow reject",
        "capacity 4 overflow loss tail",
        "capacity 4 overflow loss new",
    ]
    .iter()
    .map(|bound| &keys[bound])
    .collect();
    assert_ne!(selectors[0], selectors[1]);
    assert_ne!(selectors[1], selectors[2]);
    assert_ne!(selectors[0], selectors[2]);
}

/// FR-465 "Role", FR-383-AC-3: a static role emits an empty `replication`
/// Group; replicated roles of each lifetime emit the population text, `max`,
/// the binder node, the lifetime text and, for `until` only, the `until`
/// term; the four roles have four different keys; the binder is named by the
/// role, at the level that follows `over`, typed `Reference<T>`.
#[trace("FR-383-AC-3", "TC-1061")]
#[test]
fn replicated_roles_emit_their_population_maximum_binder_and_lifetime() {
    let lowered = emitted(
        "on origin",
        "role Static on M::Person;\n\
         role Agent each M::Person from people max 3 lifetime workflow;\n\
         role Clerk each M::Person from people max 2 lifetime scope;\n\
         role Guest each M::Order from orders max 1 lifetime until(input and Guest = Guest);",
    );
    let roles = set(&lowered.wire, "roles");
    assert_eq!(roles.len(), 4);
    ascending(&roles);
    let role = |name: &str| -> &Value {
        roles
            .iter()
            .find(|role| role["declaration"]["qualified_name"][2] == name)
            .copied()
            .unwrap()
    };
    let replication = |name: &str| argument(role(name), &ROLE, "replication");

    assert_eq!(
        replication("Static"),
        &json!({"term": "aggregate", "members": []})
    );
    for (name, population, max, lifetime, model) in [
        ("Agent", "people", "3", "workflow", "Person"),
        ("Clerk", "people", "2", "scope", "Person"),
        ("Guest", "orders", "1", "until", "Order"),
    ] {
        let group = replication(name);
        let mut expected = vec!["population", "max", "binder", "lifetime"];
        if lifetime == "until" {
            expected.push("until");
        }
        assert_eq!(member_names(group), expected, "{name}");
        assert_eq!(lit(member(group, "population")), population);
        assert_eq!(lit(member(group, "max")), max);
        assert_eq!(lit(member(group, "lifetime")), lifetime);
        let binder = target(&lowered.wire, member(group, "binder"));
        let level = &binder["body"]["members"];
        assert_eq!(level[0]["value"]["value"], name, "named by the role");
        assert_eq!(level[1]["value"]["value"], "1", "after `over`");
        let semantic_type = node_by_id(&lowered.wire, &binder["semantic_type"]);
        assert_eq!(semantic_type["semantic_form"], "reference");
        let model_node = target(&lowered.wire, argument(role(name), &ROLE, "model"));
        assert_eq!(model_node["semantic_form"], "object_type");
        assert_eq!(
            semantic_type["body"]["members"][0]["target"],
            model_node["node_id"]
        );
        assert!(
            model_node["owner"]["node"]
                .as_str()
                .unwrap()
                .ends_with(&format!("/{model}")),
            "{name}: the role's model is {model}"
        );
    }
    let until = target(&lowered.wire, member(replication("Guest"), "until"));
    let binder = target(&lowered.wire, member(replication("Guest"), "binder"));
    assert!(
        reads(&lowered.wire, until, &binder["node_id"]),
        "the `until` term reads the role's binder"
    );
    let keys: BTreeSet<&str> = roles
        .iter()
        .map(|role| role["node_id"]["digest"].as_str().unwrap())
        .collect();
    assert_eq!(keys.len(), 4, "four roles, four keys");
}

/// Whether `node` reaches `wanted` through its dependencies.
fn reads(wire: &Value, node: &Value, wanted: &Value) -> bool {
    let mut pending = vec![node];
    let mut seen = BTreeSet::new();
    while let Some(node) = pending.pop() {
        if !seen.insert(node["node_id"]["digest"].as_str().unwrap().to_owned()) {
            continue;
        }
        for dependency in node["dependencies"].as_array().unwrap() {
            if dependency == wanted {
                return true;
            }
            pending.push(node_by_id(wire, dependency));
        }
    }
    false
}

/// FR-464 "Sets", FR-383-AC-12: `roles`, `channels` and
/// `queues` follow the node digests, not the source order. Two channels,
/// three roles, declared in either order,
/// give one clause key and the same four sets; the queues are ordered by
/// their own digests, which is not their channels' order for the channel
/// names chosen here.
#[trace("FR-383-AC-12", "TC-1062")]
#[test]
fn the_sets_of_a_clause_do_not_follow_source_order() {
    let source = |forward: bool| -> String {
        let mut members = vec![
            BUYER.to_owned(),
            SELLER.to_owned(),
            "role Agent each M::Person from people max 3 lifetime workflow;".to_owned(),
        ];
        members.extend(
            CHANNELS.map(|(name, ends, bound)| channel(name, ends, "unordered", "unknown", bound)),
        );
        if !forward {
            members.reverse();
        }
        // A role before the channels that name it.
        members.sort_by_key(|member| !(member.starts_with("role ")));
        members.join("\n")
    };
    const CHANNELS: [(&str, (&str, &str), &str); 3] = [
        ("Pay", ("Buyer", "Seller"), "capacity 1 overflow block"),
        ("Ack", ("Seller", "Buyer"), "capacity 2 overflow reject"),
        (
            "Note",
            ("Agent", "Seller"),
            "capacity symbolic(N) overflow loss new",
        ),
    ];
    let forward = emitted("on origin", &source(true));
    let backward = emitted("on origin", &source(false));
    assert_eq!(
        clause(&forward.wire)["node_id"],
        clause(&backward.wire)["node_id"],
        "the clause key follows the content, not the order"
    );
    for name in ["roles", "channels", "queues"] {
        let first = set(&forward.wire, name);
        let expected = 3;
        assert_eq!(first.len(), expected, "{name}");
        assert_eq!(ascending(&first), ascending(&set(&backward.wire, name)));
    }
    // The queue digests order the queues; the channel digests order the
    // channels; the two differ for this protocol.
    let channels = set(&forward.wire, "channels");
    let by_channel: Vec<&Value> = channels
        .iter()
        .map(|channel| queue_of(&forward.wire, channel))
        .collect();
    let by_queue = set(&forward.wire, "queues");
    assert_ne!(
        by_channel
            .iter()
            .map(|queue| &queue["node_id"])
            .collect::<Vec<_>>(),
        by_queue
            .iter()
            .map(|queue| &queue["node_id"])
            .collect::<Vec<_>>(),
        "the queues follow their own digests, not their channels'"
    );
}

/// FR-464 "Binders", "Level": under `activation on each` the trigger is
/// level 1, so a replicated role's binder and a FIFO channel's message
/// binder are level 2, the count of binders readable at the clause; the role
/// and channel nodes depend on their own binder.
#[trace("TC-1021")]
#[test]
fn role_and_channel_binders_follow_the_clause_binders() {
    let lowered = emitted(
        "on each (trigger: M::Person)",
        &format!(
            "{BUYER}\n{SELLER}\n\
             role Agent each M::Person from people max 3 lifetime until(trigger = Agent);\n{}",
            channel(
                "Pay",
                ("Buyer", "Agent"),
                "fifo by (msg: Boolean) { msg }",
                "unknown",
                "capacity 2 overflow block",
            )
        ),
    );
    let level = |node: &Value| -> (String, String) {
        let members = &node["body"]["members"];
        (
            members[0]["value"]["value"].as_str().unwrap().to_owned(),
            members[1]["value"]["value"].as_str().unwrap().to_owned(),
        )
    };
    let agent = set(&lowered.wire, "roles")
        .into_iter()
        .find(|role| role["declaration"]["qualified_name"][2] == "Agent")
        .unwrap();
    let binder = target(
        &lowered.wire,
        member(argument(agent, &ROLE, "replication"), "binder"),
    );
    assert_eq!(level(binder), ("Agent".to_owned(), "2".to_owned()));
    assert!(agent["dependencies"]
        .as_array()
        .unwrap()
        .contains(&binder["node_id"]));
    let [pay] = set(&lowered.wire, "channels")[..] else {
        panic!("one channel");
    };
    let message = target(
        &lowered.wire,
        member(argument(pay, &CHANNEL, "ordering"), "binder"),
    );
    assert_eq!(level(message), ("msg".to_owned(), "2".to_owned()));
    // The trigger keeps level 1 and `over` level 0.
    let clause = clause(&lowered.wire);
    assert_eq!(
        level(target(&lowered.wire, clause_argument(clause, "over"))).1,
        "0"
    );
    let activation = clause_argument(clause, "activation");
    assert_eq!(
        level(target(&lowered.wire, member(activation, "trigger"))).1,
        "1"
    );
}

/// FR-464-AC-3: a channel's premises change its key, and the clause's: the
/// channel node and its queue follow every premise, so two protocols that
/// differ in one premise have different clause keys.
#[trace("FR-464-AC-3", "TC-1021")]
#[test]
fn every_channel_premise_changes_the_clause_key() {
    let key = |ordering: &str, delivery: &str, bound: &str, ends: (&str, &str)| -> String {
        let lowered = emitted(
            "on origin",
            &format!(
                "{BUYER}\n{SELLER}\n{}",
                channel("Pay", ends, ordering, delivery, bound)
            ),
        );
        clause(&lowered.wire)["node_id"]["digest"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    let base = (
        "unordered",
        "unknown",
        "capacity 1 overflow block",
        ("Buyer", "Seller"),
    );
    let reference = key(base.0, base.1, base.2, base.3);
    assert_eq!(reference, key(base.0, base.1, base.2, base.3), "stable");
    let changed = [
        key("fifo by (m: Boolean) { m }", base.1, base.2, base.3),
        key(base.0, "at-most-once", base.2, base.3),
        key(base.0, base.1, "capacity 2 overflow block", base.3),
        key(base.0, base.1, "capacity 1 overflow reject", base.3),
        key(base.0, base.1, "capacity 1 overflow loss head", base.3),
        key(base.0, base.1, base.2, ("Seller", "Buyer")),
    ];
    let mut all: BTreeSet<&String> = changed.iter().collect();
    assert_eq!(all.len(), changed.len(), "each premise gives its own key");
    all.insert(&reference);
    assert_eq!(all.len(), changed.len() + 1);
}

/// The check of `text` over [`model`], with the unwritten-construct emission
/// gate open or closed.
fn checked(
    text: &str,
    open: bool,
) -> Result<qsl_semantics::check::CheckedGraph, Vec<CheckRefusal>> {
    let document = model();
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n{PROFILES}\
         model M = \"acme/orders\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n{text}",
        qsl_semantics::model::key::hex(digest)
    );
    let assembled = assemble_with_models(unit.as_bytes(), &packages).expect("the unit assembles");
    let assembled = if open {
        assembled.with_unwritten_protocols()
    } else {
        assembled
    };
    assembled.check(CheckingLimits::default())
}

/// FR-092-AC-18, FR-383: every construct whose node is still not written
/// refuses the protocol typed (`unsupported_construct`/`not-yet-implemented`,
/// once, naming the construct and its ticket), and with the gate open writes
/// no `protocol_clause` node, so a protocol is never written without part of
/// its graph. The same protocol with only the written constructs checks and
/// writes the clause.
#[trace("FR-092-AC-18", "TC-1021")]
#[test]
fn every_remaining_unwritten_construct_refuses_typed_and_writes_no_clause() {
    use ProtocolConstruct as C;
    // A capture is written before the roles, every other member after.
    let plain = |activation: &str, members: &str, run: &str, finish: &str| {
        let (captures, members) = if members.starts_with("capture ") {
            (members, "")
        } else {
            ("", members)
        };
        format!(
            "function ok using v(): Boolean pure {{ true }}\n\
             protocol Flow using v over (input: Boolean) {activation} {{\n\
             {captures}\n{BUYER}\n{members}\nrun {run}\nfinish End as (outcome: Boolean) {{ {finish} }};\n}}\n"
        )
    };
    let main = "sequence Main { }";
    let carrying = |carries: &str| {
        format!(
            "{SELLER}\n{}",
            channel(
                "C",
                ("Buyer", "Seller"),
                "unordered",
                "unknown",
                "capacity 4 overflow block"
            )
            .replace("Boolean", carries)
        )
    };
    let cases = [
        (
            C::WholeModelRole,
            "IR-874",
            plain("on origin", "role W on M;", main, "true"),
        ),
        (
            C::Captures,
            "QSL-1121e",
            plain("on origin", "capture c: Boolean = input;", main, "true"),
        ),
        (
            C::ActivationGuard,
            "QSL-1121e",
            plain("on each (g: M::Person) when (true)", "", main, "true"),
        ),
        (
            C::Terminal,
            "QSL-1121f",
            plain("on origin", "terminal any;", main, "true"),
        ),
        (
            C::FinishReadsDeclaration,
            "QSL-1121g",
            plain("on origin", "", main, "ok()"),
        ),
        (
            C::ChannelCarries,
            "QSL-1137",
            plain("on origin", &carrying("Int[0, 5]"), main, "true"),
        ),
        (
            C::ChannelCarries,
            "QSL-1137",
            plain(
                "on origin",
                &carrying("Sequence<Boolean>[0, 3]"),
                main,
                "true",
            ),
        ),
        (
            C::Control("check"),
            "QSL-1126",
            plain(
                "on origin",
                "",
                "sequence Main { check Gate using v { true }; }",
                "true",
            ),
        ),
    ];
    let clauses = |graph: &qsl_semantics::check::CheckedGraph| {
        graph
            .semantic_graph()
            .nodes()
            .filter(|node| node.semantic_form() == "protocol_clause")
            .count()
    };
    let written = checked(&plain("on origin", "", main, "true"), false)
        .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    assert_eq!(clauses(&written), 1);
    for (construct, ticket, text) in cases {
        assert_eq!(construct.tracking(), ticket, "{construct:?}");
        let refusals = checked(&text, false).expect_err(&text);
        assert_eq!(
            refusals
                .iter()
                .map(|refusal| &refusal.cause)
                .collect::<Vec<_>>(),
            [&not_emitted(construct)],
            "{text}"
        );
        let open = checked(&text, true).unwrap_or_else(|refusals| panic!("{text}: {refusals:?}"));
        assert_eq!(clauses(&open), 0, "{construct:?} writes no clause node");
    }
}
