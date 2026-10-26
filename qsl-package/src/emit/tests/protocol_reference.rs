// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1021: a protocol reference expression resolves its target by typed
//! node key and emits exactly the carrier QSpec FR-322-AC-66 specifies, next
//! to the `protocol`/`protocol_clause` node it targets (FR-092-AC-16 to
//! AC-18, FR-218-AC-8 and AC-9). Every emitted package is read back through
//! IR's v2 reader.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_semantics::check::{
    CheckCause, CheckRefusal, LawRole, ProtocolConstruct, ProtocolReferenceCause,
};
use quire_semantic_value::checking::CheckingLimits;
use serde_json::{json, Value};

use super::*;

const PROTOCOL_PROFILE: &str = "quire.protocol.finite-global/v1";

/// One protocol named `name`: a static role over the spine model's `Gadget`, an
/// empty `run` sequence and a `finish`.
fn protocol(name: &str) -> String {
    format!(
        "protocol {name} using v over (input: Boolean) on origin {{\n\
         role R on M::Gadget;\n\
         run sequence Main {{ }}\n\
         finish End as (outcome: Boolean) {{ true }};\n\
         }}\n"
    )
}

/// The spine-model unit with `header` after its `v` profile, then
/// `declarations`.
fn unit(header: &str, declarations: &str) -> String {
    let fixture = spine_model_without(&[FIELD_ACCESS, CONFORMING_EQUALITY]);
    let fixture = fixture.replace(
        "profile v = \"quire.model.complete/v1\";\n",
        &format!("profile v = \"quire.model.complete/v1\";\n{header}"),
    );
    format!("{fixture}{declarations}")
}

/// `unit` with the protocol profile selected.
fn selecting(declarations: &str) -> String {
    unit(
        &format!("profile p = \"{PROTOCOL_PROFILE}\";\n"),
        declarations,
    )
}

/// The refusals the spine-model unit `text` checks to.
fn refusals(text: &str) -> Vec<CheckRefusal> {
    let packages = qsl_semantics::model::intake::package_input([SPINE_MODEL_DOCUMENT]);
    assemble_with_models(text.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect_err("the unit refuses")
}

/// A unit over the ConfigVersion model whose one protocol `Flow` has `root`
/// as its `run` root, with the protocol profile selected or not, and a
/// function that refers to `Flow` or not.
fn rooted(root: &str, profile: bool, reference: bool) -> String {
    let profile = if profile {
        format!("profile p = \"{PROTOCOL_PROFILE}\";\n")
    } else {
        String::new()
    };
    let function = if reference {
        "function refers using v(): Boolean pure { protocol Flow }\n"
    } else {
        ""
    };
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n{profile}\
         model Config = \"example/config-version\" version \"1\" digest \
         \"sha256-jcs:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\";\n\
         {function}\
         protocol Flow using v over (input: Boolean) on origin {{\n\
         role R on Config::ConfigVersion;\n\
         run {root}\n\
         finish End as (outcome: Boolean) {{ true }};\n\
         }}\n"
    )
}

const ATTEMPT_ROOT: &str = "attempt Update by R on Config::ConfigVersion::attemptUpdate \
     contracts [] as (tried: Boolean) { true };";

/// The refusal of a protocol that names `construct`, whose node is not
/// written yet.
pub(super) fn not_emitted(construct: ProtocolConstruct) -> CheckCause {
    CheckCause::ProtocolReference(Box::new(ProtocolReferenceCause::NotEmitted { construct }))
}

/// SR-9123 FND-001: a protocol whose `run` root is an event node (a
/// `commit` root does not check at S3 at all, on main or here) is a unit that checks, whether or not it is referred to, until a protocol
/// profile is selected. With no profile selected there is no node to build,
/// so the unit checks and emits no protocol node.
#[trace("FR-092-AC-18", "TC-1021")]
#[test]
fn a_non_control_root_checks_and_emits_no_protocol_node_without_a_profile() {
    let graph = check_attempt_unit(&rooted(ATTEMPT_ROOT, false, false), false)
        .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    assert!(graph
        .semantic_graph()
        .nodes()
        .all(|node| node.node_tag() != qsl_semantics::check::NodeTag::Protocol));
}

/// With a protocol profile selected, a protocol that names a construct
/// whose node is not written yet refuses `unsupported_construct`/
/// `not-yet-implemented` at its declaration, once for each such construct and
/// naming the ticket that writes it, never an internal fault; a reference to
/// it refuses with the first of them at the reference.
#[trace("FR-092-AC-18", "FR-218-AC-8", "TC-1021")]
#[test]
fn an_unwritten_construct_refuses_a_selected_profile_and_a_reference_to_it() {
    let attempt = not_emitted(ProtocolConstruct::Control("attempt"));
    assert_eq!(attempt.code(), qsl_foundation::Code::UnsupportedConstruct);
    assert_eq!(attempt.cause(), Some("not-yet-implemented"));
    for (construct, ticket) in [
        (ProtocolConstruct::WholeModelRole, "IR-874"),
        (ProtocolConstruct::Control("attempt"), "QSL-1126"),
        (ProtocolConstruct::Requirements, "QSL-1127"),
        (ProtocolConstruct::Compensations, "QSL-1127"),
        (ProtocolConstruct::Captures, "QSL-1121e"),
        (ProtocolConstruct::ActivationGuard, "QSL-1121e"),
        (ProtocolConstruct::Terminal, "QSL-1121f"),
        (ProtocolConstruct::FinishReadsDeclaration, "QSL-1121g"),
    ] {
        assert_eq!(construct.tracking(), ticket, "{construct:?}");
    }

    let declared = check_attempt_unit(&rooted(ATTEMPT_ROOT, true, false), false)
        .expect_err("the protocol names an unwritten construct");
    assert_eq!(
        declared.iter().map(|r| &r.cause).collect::<Vec<_>>(),
        [&attempt]
    );

    let referred = check_attempt_unit(&rooted(ATTEMPT_ROOT, true, true), false)
        .expect_err("the reference refuses too");
    assert_eq!(referred.len(), 2, "{referred:?}");
    assert!(referred.iter().all(|r| r.cause == attempt));
    assert_ne!(referred[0].location, referred[1].location);
}

pub(super) fn node_of<'w>(wire: &'w Value, tag: &str, form: &str) -> &'w Value {
    let matching: Vec<&Value> = nodes(wire)
        .iter()
        .filter(|node| node["node_tag"] == tag && node["semantic_form"] == form)
        .collect();
    let [node] = matching[..] else {
        panic!("one {tag}/{form} node: {matching:#?}");
    };
    node
}

/// The node `digest` names.
pub(super) fn node_by_id<'w>(wire: &'w Value, id: &Value) -> &'w Value {
    nodes(wire)
        .iter()
        .find(|node| &node["node_id"] == id)
        .unwrap_or_else(|| panic!("a node {id}"))
}

/// The value of the clause argument `name`, after checking that the clause
/// holds its thirteen arguments in FR-464's order.
pub(super) fn clause_argument<'w>(clause: &'w Value, name: &str) -> &'w Value {
    let arguments = clause["body"]["arguments"].as_array().unwrap();
    let names: Vec<&str> = arguments
        .iter()
        .map(|argument| argument["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "over",
            "activation",
            "captures",
            "roles",
            "relationships",
            "channels",
            "queues",
            "requirements",
            "compensations",
            "terminal",
            "scheduling",
            "run",
            "finish"
        ]
    );
    arguments
        .iter()
        .find(|argument| argument["name"] == name)
        .map(|argument| &argument["value"])
        .unwrap()
}

/// The graph the clause `Fill` is the root of: the `over` binder at level 0,
/// an `origin` activation, one static role, the `run` sequence and the
/// `finish` obligation, each its own node declared at its FR-464 path, and
/// every set but `roles` empty.
fn assert_clause_graph(wire: &Value, clause: &Value) {
    let reference = |value: &Value| -> &Value {
        assert_eq!(value["term"], "reference");
        node_by_id(wire, &value["target"])
    };
    let level = |parameter: &Value| -> (String, String) {
        let members = parameter["body"]["members"].as_array().unwrap();
        (
            members[0]["value"]["value"].as_str().unwrap().to_owned(),
            members[1]["value"]["value"].as_str().unwrap().to_owned(),
        )
    };
    let over = reference(clause_argument(clause, "over"));
    assert_eq!(
        (over["node_tag"].as_str(), over["semantic_form"].as_str()),
        (Some("value"), Some("parameter"))
    );
    assert_eq!(level(over), ("input".to_owned(), "0".to_owned()));
    assert_eq!(
        clause_argument(clause, "activation"),
        &json!({"term": "aggregate", "members": [{
            "term": "binding", "name": "kind",
            "value": {"term": "literal", "value": "origin", "value_kind": "text",
                      "type": clause_argument(clause, "activation")["members"][0]["value"]["type"]}
        }]})
    );
    for empty in [
        "captures",
        "relationships",
        "channels",
        "queues",
        "requirements",
        "compensations",
        "terminal",
        "scheduling",
    ] {
        assert_eq!(
            clause_argument(clause, empty),
            &json!({"term": "aggregate", "members": []}),
            "{empty}"
        );
    }

    let roles = clause_argument(clause, "roles")["members"]
        .as_array()
        .unwrap();
    let [role] = roles.as_slice() else {
        panic!("one role: {roles:?}");
    };
    let role = reference(role);
    assert_eq!(role["semantic_form"], "role");
    assert_eq!(
        role["declaration"]["qualified_name"],
        json!(["Fill", "role", "R"])
    );
    assert_eq!(role["body"]["operation"]["member"]["operator"], "role");
    assert_eq!(
        role["body"]["arguments"][1]["value"],
        json!({"term": "aggregate", "members": []}),
        "a static role replicates nothing"
    );
    let model = reference(&role["body"]["arguments"][0]["value"]);
    assert_eq!(model["semantic_form"], "object_type");

    let run = reference(clause_argument(clause, "run"));
    assert_eq!(run["semantic_form"], "control");
    assert_eq!(
        run["declaration"]["qualified_name"],
        json!(["Fill", "run", "Main"])
    );
    assert_eq!(run["body"]["operation"]["member"]["operator"], "sequence");
    assert_eq!(
        run["body"]["arguments"],
        json!([{"term": "binding", "name": "children", "value": {"term": "aggregate", "members": []}}])
    );

    let finish = reference(clause_argument(clause, "finish"));
    assert_eq!(finish["semantic_form"], "obligation");
    assert_eq!(
        finish["declaration"]["qualified_name"],
        json!(["Fill", "finish", "End"])
    );
    assert_eq!(finish["body"]["operation"]["member"]["operator"], "finish");
    let binder = reference(&finish["body"]["arguments"][0]["value"]);
    assert_eq!(
        level(binder),
        ("outcome".to_owned(), "1".to_owned()),
        "after `over`"
    );
    let constraint = reference(&finish["body"]["arguments"][1]["value"]);
    assert_eq!(constraint["body"]["value"], json!(true));
}

fn reference_key(text: &str) -> (String, Value) {
    let (_, wire, _, read) = emit_model_unit(text, None);
    assert!(matches!(read, Read::Verified { .. }), "{read:?}");
    let reference = node_of(&wire, "protocol", "protocol_reference").clone();
    (
        reference["node_id"]["digest"].as_str().unwrap().to_owned(),
        wire,
    )
}

const REFERS_TO_FILL: &str = "function refers using v(): Boolean pure { protocol Fill }\n";

/// The spine-model unit whose protocol `Flow` has two roles and a channel
/// between them, so a queue is generated (FR-465).
pub(super) fn channel_unit() -> String {
    selecting(
        "protocol Flow using v over (input: Boolean) on origin {\n\
         role R on M::Gadget;\nrole S on M::Gadget;\n\
         channel C from R to S carries Boolean ordering unordered \
         delivery at-least-once capacity 4 overflow block;\n\
         run sequence Main { }\n\
         finish End as (outcome: Boolean) { true };\n\
         }\n",
    )
}

/// The unit TC-1021 step 3 emits: `Fill`, and a function that refers to it.
pub(super) fn reference_unit() -> String {
    selecting(&format!("{REFERS_TO_FILL}{}", protocol("Fill")))
}

/// FR-092-AC-16, FR-092-AC-18 (TC-1021 step 3): the unit emits the target
/// `protocol_clause` node and the one `protocol_reference` carrier, and
/// IR's reader admits it.
#[trace("FR-092-AC-16", "FR-092-AC-18", "TC-1021")]
#[test]
fn a_reference_emits_the_target_node_and_one_carrier_the_reader_admits() {
    let text = reference_unit();
    let (emission, wire, _, read) = emit_model_unit(&text, None);
    assert_eq!(emission.omitted, []);
    assert!(matches!(read, Read::Verified { .. }), "{read:?}");

    let protocol_nodes: Vec<&Value> = nodes(&wire)
        .iter()
        .filter(|node| node["node_tag"] == "protocol")
        .collect();
    assert_eq!(protocol_nodes.len(), 5);
    let clause = node_of(&wire, "protocol", "protocol_clause");
    let reference = node_of(&wire, "protocol", "protocol_reference");

    // AC-18: the target node.
    let law = json!({
        "role": "protocol_profile",
        "definition": {"authority": "agent-ix", "identity": PROTOCOL_PROFILE},
    });
    assert_eq!(clause["body"]["term"], "application");
    assert_eq!(clause["body"]["operator"], "protocol_control");
    assert_eq!(
        clause["body"]["operation"]["identity"],
        "quire.op.protocol.control"
    );
    assert_eq!(clause["body"]["operation"]["laws"], json!([law]));
    assert_eq!(
        clause["body"]["operation"]["member"],
        json!({"kind": "profile_operator", "operator": "protocol"})
    );
    assert_clause_graph(&wire, clause);
    assert_eq!(clause["declaration"]["qualified_name"], json!(["Fill"]));
    assert_eq!(
        clause["occurrences"],
        json!([{"role": "declaration", "ordinal": 0}])
    );
    let boolean = nodes(&wire)
        .iter()
        .find(|node| node["node_tag"] == "scalar_type" && node["semantic_form"] == "boolean")
        .expect("the Boolean scalar type node is written");
    assert_eq!(clause["semantic_type"], boolean["node_id"]);
    assert_eq!(clause["body"]["result_type"], boolean["node_id"]);
    assert_eq!(wire["lock"]["profile_selections"], json!([law]));

    // AC-16: the carrier.
    assert_eq!(
        reference["body"],
        json!({"term": "reference", "target": clause["node_id"]})
    );
    assert_eq!(reference["semantic_type"], clause["semantic_type"]);
    assert_eq!(reference["dependencies"], json!([clause["node_id"]]));
    assert_eq!(
        reference["occurrences"],
        json!([{"role": "expression", "ordinal": 0}])
    );
    for absent in ["declaration", "owner", "nominal_identity_preimage"] {
        assert!(
            reference.get(absent).is_none_or(Value::is_null),
            "{absent}: {reference}"
        );
    }
    assert!(reference.get("recursion_group").is_none_or(Value::is_null));
    assert!(wire.get("protocol_references").is_none());
    assert!(wire["semantic_graph"].get("protocol_references").is_none());

    // The reference is placed at the two tokens of `protocol Fill`.
    let entries: Vec<&Value> = wire["source_map"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["node_id"] == reference["node_id"])
        .collect();
    let [entry] = entries[..] else {
        panic!("one source-map entry: {entries:?}");
    };
    assert_eq!(entry["role"], "expression");
    let region = &entry["regions"][0];
    let start = usize::try_from(region["start"].as_u64().unwrap()).unwrap();
    let end = usize::try_from(region["end"].as_u64().unwrap()).unwrap();
    assert_eq!(&text[start..end], "protocol Fill");
}

/// FR-092-AC-16, FR-092-AC-18: an `each` trigger, a nested `run` tree, two
/// static roles and `scheduling adversarial` emit the
/// graph IR admits: the trigger at level 1 and the finish binder after it,
/// the child control declared under its holder, the roles ascending by digest
/// with the whole-model role's `model` the selection's text identity, and the
/// adversarial text.
#[trace("FR-092-AC-16", "FR-092-AC-18", "TC-1021")]
#[test]
fn a_trigger_nested_runs_roles_and_scheduling_emit_what_the_reader_admits() {
    let text = selecting(
        "protocol Flow using v over (input: Boolean) on each (gadget: M::Gadget) {\n\
         role R on M::Gadget;\n\
         role W on M::Widget;\n\
         scheduling adversarial;\n\
         run sequence Outer { sequence Inner { } }\n\
         finish End as (outcome: Boolean) { true };\n\
         }\n",
    );
    let (emission, wire, _, read) = emit_model_unit(&text, None);
    assert_eq!(emission.omitted, []);
    assert!(matches!(read, Read::Verified { .. }), "{read:?}");
    let clause = node_of(&wire, "protocol", "protocol_clause");
    let reference = |value: &Value| node_by_id(&wire, &value["target"]);
    let level = |parameter: &Value| {
        parameter["body"]["members"][1]["value"]["value"]
            .as_str()
            .unwrap()
            .to_owned()
    };

    let activation = clause_argument(clause, "activation")["members"]
        .as_array()
        .unwrap();
    assert_eq!(activation[0]["value"]["value"], "each");
    assert_eq!(activation[1]["name"], "trigger");
    let trigger = reference(&activation[1]["value"]);
    assert_eq!(level(trigger), "1");
    assert_eq!(level(reference(clause_argument(clause, "over"))), "0");
    let finish = reference(clause_argument(clause, "finish"));
    assert_eq!(
        level(reference(&finish["body"]["arguments"][0]["value"])),
        "2",
        "the finish binder follows `over` and the trigger"
    );

    let scheduling = clause_argument(clause, "scheduling")["members"]
        .as_array()
        .unwrap();
    assert_eq!(scheduling.len(), 1);
    assert_eq!(scheduling[0]["value"], "adversarial");

    let outer = reference(clause_argument(clause, "run"));
    assert_eq!(
        outer["declaration"]["qualified_name"],
        json!(["Flow", "run", "Outer"])
    );
    let children = outer["body"]["arguments"][0]["value"]["members"]
        .as_array()
        .unwrap();
    let [inner] = children.as_slice() else {
        panic!("one child: {children:?}");
    };
    assert_eq!(
        reference(inner)["declaration"]["qualified_name"],
        json!(["Flow", "run", "Outer", "Inner"])
    );

    let roles = clause_argument(clause, "roles")["members"]
        .as_array()
        .unwrap();
    let digests: Vec<&str> = roles
        .iter()
        .map(|role| role["target"]["digest"].as_str().unwrap())
        .collect();
    assert_eq!(digests.len(), 2);
    assert!(
        digests.windows(2).all(|pair| pair[0] < pair[1]),
        "ascending: {digests:?}"
    );
    let models: Vec<&str> = roles
        .iter()
        .map(|role| {
            reference(&reference(role)["body"]["arguments"][0]["value"])["semantic_form"]
                .as_str()
                .unwrap()
        })
        .collect();
    assert_eq!(
        models,
        ["object_type", "object_type"],
        "each role refers to its type's node"
    );
}

/// FR-464 "Sets": the order of `roles` follows the node digests, not the
/// source order, so swapping two roles' order in the source leaves the
/// clause's `roles` argument unchanged.
#[trace("FR-464-AC-3", "TC-1021")]
#[test]
fn the_roles_set_does_not_follow_source_order() {
    let source = |first: &str, second: &str| {
        selecting(&format!(
            "protocol Flow using v over (input: Boolean) on origin {{\n\
             {first}\n{second}\n\
             run sequence Main {{ }}\n\
             finish End as (outcome: Boolean) {{ true }};\n\
             }}\n"
        ))
    };
    let roles_of = |text: &str| {
        let (_, wire, _, read) = emit_model_unit(text, None);
        assert!(matches!(read, Read::Verified { .. }), "{read:?}");
        let clause = node_of(&wire, "protocol", "protocol_clause");
        clause_argument(clause, "roles").clone()
    };
    let (gadget, widget) = ("role R on M::Gadget;", "role W on M::Widget;");
    let in_order = roles_of(&source(gadget, widget));
    assert_eq!(in_order["members"].as_array().unwrap().len(), 2);
    assert_eq!(in_order, roles_of(&source(widget, gadget)));
}

/// FR-092-AC-18: an activation guard and a terminal member
/// each refuse at the protocol, naming the construct, never writing the
/// protocol without it; a role on a whole model refuses as the IR gap it is.
#[trace("FR-092-AC-18", "TC-1021")]
#[test]
fn constructs_without_a_node_yet_refuse_naming_the_construct() {
    let flow = |extra: &str, activation: &str| {
        selecting(&format!(
            "protocol Flow using v over (input: Boolean) {activation} {{\n\
             role R on M::Gadget;\n\
             {extra}\n\
             run sequence Main {{ }}\n\
             finish End as (outcome: Boolean) {{ true }};\n\
             }}\n"
        ))
    };
    let causes = |text: &str| -> Vec<CheckCause> {
        refusals(text)
            .into_iter()
            .map(|refusal| refusal.cause)
            .collect()
    };
    assert_eq!(
        causes(&flow("", "on each (g: M::Gadget) when (true)")),
        [not_emitted(ProtocolConstruct::ActivationGuard)]
    );
    assert_eq!(
        causes(&selecting(
            "protocol Flow using v over (input: Boolean) on origin {\n\
             role W on M;\n\
             run sequence Main { }\n\
             finish End as (outcome: Boolean) { true };\n\
             }\n"
        )),
        [not_emitted(ProtocolConstruct::WholeModelRole)]
    );
    assert_eq!(ProtocolConstruct::WholeModelRole.tracking(), "IR-874");
    assert_eq!(
        causes(&flow("terminal any;", "on origin")),
        [not_emitted(ProtocolConstruct::Terminal)]
    );
}

/// FR-464 "Term shapes": a `sequence`'s `children` are an ordered list whose
/// order is the execution order, so it is the source order, not the name or
/// the digest order: three children in two source orders each come back in
/// the order written.
#[trace("TC-1021")]
#[test]
fn a_sequences_children_follow_the_source_order() {
    let children_of = |order: [&str; 3]| -> Vec<String> {
        let body: String = order
            .iter()
            .map(|name| format!("sequence {name} {{ }} "))
            .collect();
        let text = selecting(&format!(
            "protocol Flow using v over (input: Boolean) on origin {{\n\
             role R on M::Gadget;\n\
             run sequence Main {{ {body} }}\n\
             finish End as (outcome: Boolean) {{ true }};\n\
             }}\n"
        ));
        let (_, wire, _, read) = emit_model_unit(&text, None);
        assert!(matches!(read, Read::Verified { .. }), "{read:?}");
        let clause = node_of(&wire, "protocol", "protocol_clause");
        let run = node_by_id(
            &wire,
            clause_argument(clause, "run")["target"]
                .as_object()
                .map(|_| &clause_argument(clause, "run")["target"])
                .unwrap(),
        );
        run["body"]["arguments"][0]["value"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .map(|child| {
                let node = node_by_id(&wire, &child["target"]);
                node["declaration"]["qualified_name"]
                    .as_array()
                    .unwrap()
                    .last()
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect()
    };
    assert_eq!(
        children_of(["Zeta", "Alpha", "Mid"]),
        ["Zeta", "Alpha", "Mid"]
    );
    assert_eq!(
        children_of(["Mid", "Alpha", "Zeta"]),
        ["Mid", "Alpha", "Zeta"]
    );
}

/// FR-464-AC-3: a protocol's `protocol_clause` key follows its own content
/// and nothing else: two distinct protocols have distinct keys, and
/// declaring them in the other order leaves each key unchanged.
#[trace("FR-464-AC-3", "TC-1021")]
#[test]
fn a_clause_key_is_distinct_per_protocol_and_independent_of_declaration_order() {
    let keys_of = |declarations: &str| -> BTreeMap<String, String> {
        let (_, wire, _, read) = emit_model_unit(&selecting(declarations), None);
        assert!(matches!(read, Read::Verified { .. }), "{read:?}");
        nodes(&wire)
            .iter()
            .filter(|node| node["semantic_form"] == "protocol_clause")
            .map(|node| {
                (
                    node["declaration"]["qualified_name"][0]
                        .as_str()
                        .unwrap()
                        .to_owned(),
                    node["node_id"]["digest"].as_str().unwrap().to_owned(),
                )
            })
            .collect()
    };
    let (fill, drain) = (protocol("Fill"), protocol("Drain"));
    let forward = keys_of(&format!("{fill}{drain}"));
    assert_eq!(forward.len(), 2);
    assert_ne!(forward["Fill"], forward["Drain"]);
    assert_eq!(forward, keys_of(&format!("{drain}{fill}")));
}

/// FR-092-AC-17 (TC-1021 step 6): the reference's key is the digest of its
/// `quire.structural-node/v1` preimage, follows the bound target, repeats for
/// the same unit and changes with the target's key.
#[trace("FR-092-AC-17", "TC-1021")]
#[test]
fn a_references_key_follows_its_bound_target() {
    let both = format!("{}{}", protocol("Fill"), protocol("Drain"));
    let against_fill = selecting(&format!("{REFERS_TO_FILL}{both}"));
    let against_drain = selecting(&format!(
        "function refers using v(): Boolean pure {{ protocol Drain }}\n{both}"
    ));
    let (fill, fill_wire) = reference_key(&against_fill);
    let (drain, _) = reference_key(&against_drain);
    let (again, _) = reference_key(&against_fill);
    assert_ne!(fill, drain, "different targets, different keys");
    assert_eq!(fill, again, "the same unit, the same key");

    let carrier = node_of(&fill_wire, "protocol", "protocol_reference");
    let preimage = json!({
        "version": "quire.structural-node/v1",
        "node_tag": "protocol",
        "semantic_form": "protocol_reference",
        "semantic_type": carrier["semantic_type"],
        "declaration": null,
        "recursion": null,
        "body": {"term": "reference", "target": carrier["body"]["target"]},
    });
    assert_eq!(fill, sha256_hex(&jcs(&preimage)));

    // `Fill` renamed: its node key changes, so the reference's does too.
    let renamed =
        selecting(&format!("{REFERS_TO_FILL}{}", protocol("Fill")).replace("Fill", "Pour"));
    let (poured, _) = reference_key(&renamed);
    assert_ne!(fill, poured, "a changed target key changes the reference");
}

/// FR-218-AC-8 (TC-1021 step 4): an unknown name, a name only qualified, a
/// name that resolves to a function and a second reference to one target.
#[trace("FR-218-AC-8", "TC-1021")]
#[test]
fn unresolved_non_protocol_and_repeated_references_refuse_by_cause() {
    let flow = protocol("Fill");
    let missing = refusals(&selecting(&format!(
        "function refers using v(): Boolean pure {{ protocol Missing }}\n{flow}"
    )));
    let [missing] = missing.as_slice() else {
        panic!("one refusal: {missing:?}");
    };
    assert_eq!(missing.cause, CheckCause::MissingName("Missing".to_owned()));

    let qualified = refusals(&selecting(&format!(
        "function refers using v(): Boolean pure {{ protocol M::Fill }}\n{flow}"
    )));
    let [qualified] = qualified.as_slice() else {
        panic!("one refusal: {qualified:?}");
    };
    assert_eq!(
        qualified.cause,
        CheckCause::MissingName("M::Fill".to_owned())
    );

    let function = refusals(&selecting(&format!(
        "function other using v(): Boolean pure {{ true }}\n\
         function refers using v(): Boolean pure {{ protocol other }}\n{flow}"
    )));
    let [function] = function.as_slice() else {
        panic!("one refusal: {function:?}");
    };
    assert_eq!(function.cause.code(), qsl_foundation::Code::IllTyped);
    assert_eq!(function.cause.cause(), Some("operator-ineligible"));

    let twice = refusals(&selecting(&format!(
        "function refers using v(): Boolean pure {{ protocol Fill and protocol Fill }}\n{flow}"
    )));
    let [twice] = twice.as_slice() else {
        panic!("one refusal: {twice:?}");
    };
    assert_eq!(
        twice.cause.code(),
        qsl_foundation::Code::UnsupportedConstruct
    );
    assert_eq!(twice.cause.cause(), Some("expression-form"));
    let CheckCause::ProtocolReference(cause) = &twice.cause else {
        panic!("a protocol reference cause: {twice:?}");
    };
    assert_ne!(
        cause.first(),
        Some(&twice.location),
        "names the first reference"
    );
}

/// FR-092 "The protocol node a reference targets": with no protocol profile
/// selected there is no `protocol_profile` law to carry, so a reference
/// refuses `missing_declaration`/`missing-selection` and a protocol nobody
/// refers to emits as before.
#[trace("FR-092-AC-18", "TC-1021")]
#[test]
fn a_reference_without_a_selected_protocol_profile_refuses_missing_selection() {
    let refused = refusals(&unit("", &format!("{REFERS_TO_FILL}{}", protocol("Fill"))));
    let [refused] = refused.as_slice() else {
        panic!("one refusal: {refused:?}");
    };
    assert_eq!(
        refused.cause,
        CheckCause::MissingSelection {
            role: LawRole::ProtocolProfile
        }
    );
}

/// FR-218-AC-9 (TC-1021 step 5): a reference whose bound target is no node
/// of the graph refuses `missing_declaration`/`missing-name`, naming the
/// bound key, and writes no package.
#[trace("FR-218-AC-9", "TC-1021")]
#[test]
fn a_reference_bound_to_a_key_no_node_carries_refuses_missing_name() {
    let text = selecting(&format!("{REFERS_TO_FILL}{}", protocol("Fill")));
    let packages = qsl_semantics::model::intake::package_input([SPINE_MODEL_DOCUMENT]);
    let mut graph = assemble_with_models(text.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect("the package checks");
    let key_of = |form: &str| {
        graph
            .semantic_graph()
            .nodes()
            .find(|node| node.semantic_form() == form)
            .unwrap_or_else(|| panic!("a {form} node"))
            .key()
    };
    let (target, reference) = (key_of("protocol_clause"), key_of("protocol_reference"));
    assert!(graph.semantic_graph_mut().remove(target).is_some());
    let refusal = emit_checked(&CheckedPackage::link(graph)).expect_err("no package");
    assert_eq!(refusal.code(), qsl_foundation::Code::MissingDeclaration);
    assert_eq!(refusal.cause(), Some("missing-name"));
    assert_eq!(
        refusal,
        EmitRefusal::StaleProtocolReference {
            reference: node_id(reference),
            target: node_id(target),
        }
    );
}
