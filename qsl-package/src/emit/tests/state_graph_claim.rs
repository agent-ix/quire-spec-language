// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-410 (TC-1140): a checked state-graph claim emits as QSpec FR-475's
//! `claim`/`state_graph` node over the ConfigVersion domain package, QSL's I2
//! read admits it and refuses each FR-475 defect at FR-475's locus, and the
//! I2 state-graph lookup returns each declaration's identity triple.

use quire_contract_model::CheckedPackageRefusalCode;
use quire_exact::Origin;

use super::admission_corpus::config_version_unit;
use super::*;
use crate::checked_v2::tests::rekey_stale;
use crate::checked_v2::{
    clause_roots, resolve_clause_root, resolve_state_graph_root, state_graph_roots,
    ClauseRootUnresolved, StateGraphKind, StateGraphRoot, V2ReadRefusal,
};

fn document() -> &'static [u8] {
    config_version_unit::DOMAIN_PACKAGE.as_bytes()
}

fn document_digest() -> String {
    let packages = qsl_semantics::model::intake::package_input([document()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    qsl_semantics::model::key::hex(digest)
}

/// A unit selecting the ConfigVersion model, then `declarations`.
fn unit(declarations: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         profile t = \"quire.temporal.infinite-trace/v1\";\n\
         model Config = \"example/config-version\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
         {declarations}\n",
        document_digest()
    )
}

const REACHES_TWO: &str =
    "possible ReachesTwo using v over (c: Config::ConfigVersion) { c.versionNumber = 2 }";
const CAN_STILL_WIN: &str = "always possible CanStillWin using v over (c: Config::ConfigVersion) \
     from (c.versionNumber != 0) { c.versionNumber = 2 }";
const IN_ONE_WAY: &str = "unique path InOneWay using v over (c: Config::ConfigVersion) \
     from (c.versionNumber = 0) to (c.versionNumber = 2);";
const OPEN_ALWAYS: &str =
    "always possible Open using v over (c: Config::ConfigVersion) { c.versionNumber = 2 }";
const TRUE_FROM: &str = "always possible Open using v over (c: Config::ConfigVersion) \
     from (true) { c.versionNumber = 2 }";
const ANYTHING: &str = "possible Anything using v { true }";

struct Compiled {
    emission: Emission,
    wire: Value,
}

fn compile(unit: &str) -> Compiled {
    let packages = qsl_semantics::model::intake::package_input([document()]);
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    Compiled { emission, wire }
}

fn evidence(wire: &Value) -> CheckedPackageEvidence {
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    evidence
        .insert_domain_package_document(document_digest(), document().to_vec())
        .expect("a fresh evidence key");
    evidence
}

fn package_id(compiled: &Compiled) -> qsl_semantics::library::PackageId {
    compiled.emission.package.package_id()
}

fn read_bytes(bytes: &[u8], wire: &Value, package_id: qsl_semantics::library::PackageId) -> Read {
    read_v2(
        bytes,
        library(),
        V2ReadLimits::default(),
        &evidence(wire),
        &qsl_semantics::library::fixtures::single_pin(library(), package_id),
    )
}

/// QSL's full I2 read of `compiled`, and IR's admitted package.
fn admit(compiled: &Compiled) -> std::sync::Arc<quire_contract_model::CheckedPackageV2> {
    crate::checked_v2::read_checked_package_v2(
        compiled.emission.package.bytes(),
        library(),
        V2ReadLimits::default(),
        &evidence(&compiled.wire),
        &qsl_semantics::library::fixtures::single_pin(library(), package_id(compiled)),
    )
    .unwrap_or_else(|failure| panic!("the I2 read admits the package: {failure:?}"))
    .into_value()
    .admitted
}

fn node_with<'w>(wire: &'w Value, digest: &str) -> &'w Value {
    nodes(wire)
        .iter()
        .find(|node| node["node_id"]["digest"] == digest)
        .unwrap_or_else(|| panic!("a node {digest}"))
}

fn target(term: &Value) -> &str {
    assert_eq!(term["term"], "reference", "{term}");
    term["target"]["digest"].as_str().expect("a target digest")
}

fn binding<'v>(body: &'v Value, name: &str) -> &'v Value {
    body["members"]
        .as_array()
        .expect("an aggregate")
        .iter()
        .find(|member| member["name"] == name)
        .map(|member| &member["value"])
        .unwrap_or_else(|| panic!("a binding {name} in {body}"))
}

fn text(term: &Value) -> &str {
    assert_eq!(term["value_kind"], "text", "{term}");
    term["value"].as_str().expect("text")
}

/// The `claim`/`state_graph` nodes of `wire`, in graph order.
fn claim_nodes(wire: &Value) -> Vec<&Value> {
    nodes(wire)
        .iter()
        .filter(|node| node["node_tag"] == "claim" && node["semantic_form"] == "state_graph")
        .collect()
}

/// The one claim node of `wire` whose `kind` is `kind`.
fn claim_of<'w>(wire: &'w Value, kind: &str) -> &'w Value {
    let found: Vec<&Value> = claim_nodes(wire)
        .into_iter()
        .filter(|node| text(binding(&node["body"], "kind")) == kind)
        .collect();
    let [node] = found[..] else {
        panic!("one {kind} claim node");
    };
    node
}

fn digest(node: &Value) -> String {
    node["node_id"]["digest"].as_str().unwrap().to_owned()
}

fn is_empty_aggregate(term: &Value) -> bool {
    term["term"] == "aggregate" && term["members"] == json!([])
}

/// FR-410-AC-1: the three forms emit as `claim`/`state_graph` nodes whose
/// body binds `kind`, `over`, `from` and `target` in that order, with no
/// `declaration` or `owner`; every occurrence has role `claim`; the I2 read
/// admits the package and the state-graph lookup returns each form, its
/// predicates and its `over` node.
#[trace("TC-1140", "FR-410-AC-1")]
#[test]
fn the_three_forms_emit_claim_nodes_the_i2_read_admits() {
    let compiled = compile(&unit(&format!(
        "{REACHES_TWO}\n{CAN_STILL_WIN}\n{IN_ONE_WAY}"
    )));
    let wire = &compiled.wire;
    assert_eq!(claim_nodes(wire).len(), 3);
    for node in claim_nodes(wire) {
        let names: Vec<&str> = node["body"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .map(|member| member["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["kind", "over", "from", "target"]);
        assert!(node.get("declaration").is_none(), "{node}");
        assert!(node.get("owner").is_none(), "{node}");
        let roles: Vec<&Value> = node["occurrences"]
            .as_array()
            .unwrap()
            .iter()
            .map(|occurrence| &occurrence["role"])
            .collect();
        assert_eq!(roles, [&json!("claim")]);
        let boolean = node_with(wire, node["semantic_type"]["digest"].as_str().unwrap());
        assert_eq!(boolean["semantic_form"], "boolean");
    }

    let possible = claim_of(wire, "possible");
    let body = &possible["body"];
    let over = node_with(wire, target(binding(body, "over")));
    assert_eq!(over["semantic_form"], "parameter");
    assert_eq!(text(binding(&over["body"], "name")), "c");
    assert!(is_empty_aggregate(binding(body, "from")));
    let goal = node_with(wire, target(binding(body, "target")));
    assert_eq!(goal["node_tag"], "expression");

    let always = claim_of(wire, "always-possible");
    let from = node_with(wire, target(binding(&always["body"], "from")));
    assert_eq!(from["node_tag"], "expression");
    assert_eq!(
        target(binding(&always["body"], "over")),
        target(binding(body, "over"))
    );
    let unique = claim_of(wire, "unique-path");
    assert!(!is_empty_aggregate(binding(&unique["body"], "from")));

    let admitted = admit(&compiled);
    let roots = state_graph_roots(&admitted).expect("the roots read");
    assert_eq!(roots.len(), 3);
    let root_of = |kind| {
        roots
            .iter()
            .find(|root| root.kind() == kind)
            .unwrap_or_else(|| panic!("a {kind:?} root"))
    };
    let possible_root = root_of(StateGraphKind::Possible);
    assert_eq!(possible_root.from(), None);
    assert_eq!(
        possible_root.over().map(|id| id.to_string()),
        Some(digest(over))
    );
    assert_eq!(possible_root.target().to_string(), digest(goal));
    let always_root = root_of(StateGraphKind::AlwaysPossible);
    assert_eq!(
        always_root.from().map(|id| id.to_string()),
        Some(digest(from))
    );
    let unique_root = root_of(StateGraphKind::UniquePath);
    assert_eq!(
        unique_root.from().map(|id| id.to_string()),
        Some(target(binding(&unique["body"], "from")).to_owned())
    );
    assert_eq!(
        unique_root.target().to_string(),
        target(binding(&unique["body"], "target"))
    );
}

/// FR-410-AC-2: `possible P` and `always possible P` emit different node ids
/// and `package_id`s; an `always possible` claim without `from` and with
/// `from (true)` emit one node id with `from` the `true` literal node; a
/// claim with no `over` emits the empty aggregate for `over` and the `true`
/// literal node as its `target`; one claim declared twice under two names
/// emits one node with two `claim` occurrences and no `declaration`.
#[trace("TC-1140", "FR-410-AC-2")]
#[test]
fn the_form_enters_the_identity_and_equal_claims_share_one_node() {
    let possible = compile(&unit(REACHES_TWO));
    let always = compile(&unit(
        "always possible ReachesTwo using v over (c: Config::ConfigVersion) \
         { c.versionNumber = 2 }",
    ));
    assert_ne!(
        digest(claim_of(&possible.wire, "possible")),
        digest(claim_of(&always.wire, "always-possible"))
    );
    assert_ne!(package_id(&possible), package_id(&always));

    let omitted = compile(&unit(OPEN_ALWAYS));
    let written = compile(&unit(TRUE_FROM));
    let (omitted_node, written_node) = (
        claim_of(&omitted.wire, "always-possible"),
        claim_of(&written.wire, "always-possible"),
    );
    assert_eq!(digest(omitted_node), digest(written_node));
    let truth = node_with(
        &omitted.wire,
        target(binding(&omitted_node["body"], "from")),
    );
    assert_eq!(
        (truth["node_tag"].as_str(), truth["semantic_form"].as_str()),
        (Some("value"), Some("literal"))
    );
    assert_eq!(truth["body"]["value"], json!(true));

    let anything = compile(&unit(ANYTHING));
    let node = claim_of(&anything.wire, "possible");
    assert!(is_empty_aggregate(binding(&node["body"], "over")));
    let goal = node_with(&anything.wire, target(binding(&node["body"], "target")));
    assert_eq!(
        (goal["node_tag"].as_str(), goal["semantic_form"].as_str()),
        (Some("value"), Some("literal"))
    );
    admit(&anything);

    let twice = compile(&unit(&format!(
        "{REACHES_TWO}\n{}",
        REACHES_TWO.replace("ReachesTwo", "ReachesTwoAgain")
    )));
    let [shared] = claim_nodes(&twice.wire)[..] else {
        panic!("one shared node");
    };
    assert!(shared.get("declaration").is_none());
    let ordinals: Vec<&Value> = shared["occurrences"]
        .as_array()
        .unwrap()
        .iter()
        .map(|occurrence| &occurrence["ordinal"])
        .collect();
    assert_eq!(ordinals, [&json!(0), &json!(1)]);
}

/// The node id of the one `kind` claim that compiling `declarations` emits.
fn claim_id(declarations: &str, kind: &str) -> String {
    digest(claim_of(&compile(&unit(declarations)).wire, kind))
}

const OVER_C: &str = "using v over (c: Config::ConfigVersion)";

/// QSL-1108 (SR-17237 FND-001): the three forms over equal predicates give
/// three distinct node ids, and swapping `unique path`'s `from` and `to`
/// gives a fourth, so the form and each predicate's role enter the identity.
#[trace("TC-1140")]
#[test]
fn the_form_and_the_unique_path_roles_each_change_the_node_id() {
    let two = "c.versionNumber = 2";
    let possible = claim_id(&format!("possible P {OVER_C} {{ {two} }}"), "possible");
    let always = claim_id(
        &format!("always possible P {OVER_C} from (true) {{ {two} }}"),
        "always-possible",
    );
    let unique = claim_id(
        &format!("unique path P {OVER_C} from (true) to ({two});"),
        "unique-path",
    );
    assert_ne!(possible, always);
    assert_ne!(always, unique);
    assert_ne!(possible, unique);

    let forward = claim_id(
        &format!("unique path P {OVER_C} from (c.versionNumber = 0) to ({two});"),
        "unique-path",
    );
    let swapped = claim_id(
        &format!("unique path P {OVER_C} from ({two}) to (c.versionNumber = 0);"),
        "unique-path",
    );
    assert_ne!(forward, swapped);
}

/// QSL-1108 (SR-17237 FND-001): whitespace, redundant parentheses, the claim's
/// name, the order of the unit's claims and other claims in the unit leave a
/// claim's node id unchanged.
#[trace("TC-1140")]
#[test]
fn spelling_order_and_other_claims_leave_the_node_id_unchanged() {
    let plain = claim_id(REACHES_TWO, "possible");
    let spaced = claim_id(
        "possible   ReachesTwo   using v\n  over (c: Config::ConfigVersion)\n \
         {   c.versionNumber   =   2   }",
        "possible",
    );
    let parenthesised = claim_id(
        "possible ReachesTwo using v over (c: Config::ConfigVersion) \
         { ((c.versionNumber = 2)) }",
        "possible",
    );
    let renamed = claim_id(&REACHES_TWO.replace("ReachesTwo", "Elsewhere"), "possible");
    for same in [&spaced, &parenthesised, &renamed] {
        assert_eq!(&plain, same);
    }

    let others = format!("{CAN_STILL_WIN}\n{IN_ONE_WAY}");
    let before = format!("{REACHES_TWO}\n{others}");
    let after = format!("{others}\n{REACHES_TWO}");
    for declarations in [before, after] {
        let compiled = compile(&unit(&declarations));
        assert_eq!(claim_nodes(&compiled.wire).len(), 3);
        assert_eq!(digest(claim_of(&compiled.wire, "possible")), plain);
    }
}

/// QSL-1108 (SR-17237 FND-001): a `{ true }` target is the `true` literal
/// node the omitted `from` already references, so one literal node serves
/// both; and renaming the `over` parameter changes the node id, because the
/// parameter node carries its name (SR-17236 FND-007, documented behavior).
#[trace("TC-1140")]
#[test]
fn a_true_target_reuses_the_true_literal_and_the_over_name_enters_the_id() {
    let compiled = compile(&unit(&format!(
        "always possible Always {OVER_C} {{ true }}"
    )));
    let node = claim_of(&compiled.wire, "always-possible");
    let from = target(binding(&node["body"], "from"));
    assert_eq!(from, target(binding(&node["body"], "target")));
    let truth = node_with(&compiled.wire, from);
    assert_eq!(truth["body"]["value"], json!(true));
    let literals = nodes(&compiled.wire)
        .iter()
        .filter(|node| node["semantic_form"] == "literal" && node["body"]["value"] == json!(true))
        .count();
    assert_eq!(literals, 1, "one true literal node in the package");

    let renamed = REACHES_TWO
        .replace("(c:", "(d:")
        .replace("c.version", "d.version");
    assert_ne!(
        claim_id(REACHES_TWO, "possible"),
        claim_id(&renamed, "possible")
    );
}

fn wire_id(digest: &str) -> WireNodeId {
    WireNodeId::from_hex(digest).expect("a node digest")
}

/// FR-410-AC-3: the lookup returns `[node, "claim", ordinal]` for
/// `ReachesTwo`, and it resolves in the recompiled package; a triple with an
/// unknown node, a `temporal`/`temporal_clause` node, or another ordinal is
/// `clause-root-unresolved` from the state-graph lookup, and `ReachesTwo`'s
/// triple given to the temporal lookup is `clause-root-unresolved`.
#[trace("TC-1140", "FR-410-AC-3")]
#[test]
fn the_i2_lookup_resolves_the_triple_and_each_lookup_takes_only_its_own_form() {
    let text = unit(&format!(
        "{REACHES_TWO}\n\
         temporal Liveness using t over (c: Config::ConfigVersion) clock \"model-steps\" \
         on origin {{ always eventually holds(c.versionNumber = 2) }}"
    ));
    let compiled = compile(&text);
    let admitted = admit(&compiled);
    let claim = claim_of(&compiled.wire, "possible");
    let [root] = state_graph_roots(&admitted)
        .expect("the roots read")
        .try_into()
        .unwrap_or_else(|roots: Vec<StateGraphRoot>| panic!("one root: {roots:?}"));
    let triple = root.occurrence().clone();
    assert_eq!(triple.node(), wire_id(&digest(claim)));
    assert_eq!(triple.origin().role().as_str(), "claim");

    let recompiled = admit(&compile(&text));
    assert_eq!(resolve_state_graph_root(&recompiled, &triple), Ok(Ok(root)));

    let temporal = nodes(&compiled.wire)
        .iter()
        .find(|node| node["semantic_form"] == "temporal_clause")
        .expect("the temporal clause node");
    let claim_at =
        |node, ordinal| OccurrenceKey::new(node, Origin::new(Role::new("claim"), ordinal));
    let ordinal = triple.origin().ordinal();
    for foreign in [
        claim_at(WireNodeId::from_digest([7; 32]), ordinal),
        claim_at(wire_id(&digest(temporal)), 0),
        claim_at(triple.node(), ordinal + 1),
    ] {
        assert_eq!(
            resolve_state_graph_root(&recompiled, &foreign),
            Ok(Err(ClauseRootUnresolved {
                root: foreign.clone()
            })),
            "{foreign}"
        );
    }
    assert_eq!(
        resolve_clause_root(&recompiled, &triple),
        Err(ClauseRootUnresolved {
            root: triple.clone()
        })
    );
    assert!(clause_roots(&recompiled)
        .expect("the roots read")
        .iter()
        .all(|clause| clause.occurrence() != &triple));
}

/// `compiled`'s wire with the node `digest` edited by `edit`, every stale
/// key renamed and the `package_id` recomputed.
fn edited(
    compiled: &Compiled,
    digest: &str,
    edit: impl FnOnce(&mut Value),
) -> (Value, BTreeMap<String, String>) {
    let mut envelope = compiled.wire.clone();
    let node = envelope["semantic_graph"]["nodes"]
        .as_array_mut()
        .expect("nodes")
        .iter_mut()
        .find(|node| node["node_id"]["digest"] == digest)
        .expect("the edited node");
    edit(node);
    let renames = rekey_stale(&mut envelope);
    (envelope, renames)
}

fn ir_refusal(
    compiled: &Compiled,
    envelope: &Value,
) -> quire_contract_model::CheckedPackageRefusal {
    let package_id = crate::checked_v2::tests::derived_package_id(envelope);
    match read_bytes(&jcs(envelope), &compiled.wire, package_id) {
        Read::Refused(V2ReadRefusal::Envelope { refusal, .. }) => *refusal,
        other => panic!("an IR refusal, not {other:?}"),
    }
}

fn path(refusal: &quire_contract_model::CheckedPackageRefusal) -> String {
    refusal
        .path
        .as_ref()
        .map(|pointer| pointer.as_str().to_owned())
        .unwrap_or_default()
}

fn position(envelope: &Value, digest: &str) -> usize {
    nodes(envelope)
        .iter()
        .position(|node| node["node_id"]["digest"] == digest)
        .expect("the node is in the graph")
}

fn renamed(renames: &BTreeMap<String, String>, digest: &str) -> String {
    renames
        .get(digest)
        .cloned()
        .unwrap_or_else(|| digest.to_owned())
}

/// FR-410-AC-4: AC-1's `ReachesTwo` node with `kind` `eventually-possible`
/// refuses `invalid_package`/`invalid-value` at the `kind` binding, and with
/// `target` a reference to `c`'s parameter node and `dependencies` rewritten
/// to the body's targets refuses `ill_typed`/`operator-ineligible` at the
/// target, the node key and `package_id` recomputed after each edit.
#[trace("TC-1140", "FR-410-AC-4")]
#[test]
fn the_i2_read_refuses_each_fr475_defect_at_its_locus() {
    use quire_contract_model::CheckedPackageRefusalCause as Cause;
    let compiled = compile(&unit(REACHES_TWO));
    let node = claim_of(&compiled.wire, "possible");
    let claim_digest = digest(node);

    let (envelope, renames) = edited(&compiled, &claim_digest, |node| {
        node["body"]["members"][0]["value"]["value"] = json!("eventually-possible");
    });
    let refusal = ir_refusal(&compiled, &envelope);
    assert_eq!(
        (refusal.code, refusal.cause),
        (
            CheckedPackageRefusalCode::InvalidPackage,
            Some(Cause::InvalidValue)
        ),
        "{refusal:?}"
    );
    let at = position(&envelope, &renamed(&renames, &claim_digest));
    assert_eq!(
        path(&refusal),
        format!("/semantic_graph/nodes/{at}/body/members/0")
    );

    let over = target(binding(&node["body"], "over")).to_owned();
    let node_ref = node_with(&compiled.wire, &over)["node_id"].clone();
    let (envelope, renames) = edited(&compiled, &claim_digest, |node| {
        node["body"]["members"][3]["value"]["target"]["digest"] = json!(over);
        node["dependencies"] = json!([node_ref]);
    });
    let refusal = ir_refusal(&compiled, &envelope);
    assert_eq!(
        (refusal.code, refusal.cause),
        (
            CheckedPackageRefusalCode::IllTyped,
            Some(Cause::OperatorIneligible)
        ),
        "{refusal:?}"
    );
    let at = position(&envelope, &renamed(&renames, &claim_digest));
    assert_eq!(
        path(&refusal),
        format!("/semantic_graph/nodes/{at}/body/members/3/value/target"),
        "{refusal:?}"
    );
}
