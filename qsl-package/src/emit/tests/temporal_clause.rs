// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-337 (TC-524, TC-1079): a checked temporal clause emits as QSpec
//! FR-370's `temporal`/`temporal_clause` node over ADR-018 §6's
//! ConfigVersion subject, QSL's I2 read admits it and refuses each FR-370
//! defect at FR-370's locus, and the I2 lookup returns each declaration's
//! identity triple and `over` node.

use quire_contract_model::CheckedPackageRefusalCode;
use quire_exact::Origin;

use super::admission_corpus::config_version_unit;
use super::*;
use crate::checked_v2::tests::rekey_stale;
use crate::checked_v2::{clause_roots, resolve_clause_root, ClauseRoot, V2ReadRefusal};

const INFINITE: &str = "quire.temporal.infinite-trace/v1";
const EVENT: &str = "quire.temporal.event-position.false-extension/v1";

/// ADR-018 §6's clause formula.
const EVENTUALLY_TWO: &str = "always eventually holds(c.versionNumber = 2)";

/// The ConfigVersion domain package document.
fn document() -> &'static [u8] {
    config_version_unit::DOMAIN_PACKAGE.as_bytes()
}

/// `document`'s `sha256-jcs` digest, as `intake::package_input` keys it.
fn document_digest() -> String {
    let packages = qsl_semantics::model::intake::package_input([document()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    qsl_semantics::model::key::hex(digest)
}

/// A unit selecting the ConfigVersion model and the temporal profile
/// `profile` as `t`, then `declarations`.
fn unit(profile: &str, declarations: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         profile t = \"{profile}\";\n\
         model Config = \"example/config-version\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
         {declarations}\n",
        document_digest()
    )
}

/// The clause `name` over `c: Config::ConfigVersion` whose body is `body`.
fn declared(name: &str, body: &str) -> String {
    format!(
        "temporal {name} using t over (c: Config::ConfigVersion) clock \"model-steps\" \
         on origin {{ {body} }}"
    )
}

/// ADR-018 §6's `ReachesTwo` with fairness `fair` (empty for none).
fn reaches_two(fair: &str) -> String {
    declared("ReachesTwo", &format!("{fair}{EVENTUALLY_TWO}"))
}

/// One compiled temporal unit: the checked graph's clauses, the emission
/// and its wire.
struct Compiled {
    clauses: Vec<qsl_semantics::check::TemporalClauseNode>,
    emission: Emission,
    wire: Value,
}

/// `unit` through S1, S2, I1, the assembler, check, link and the v2
/// emitter, with nothing omitted.
fn compile(unit: &str) -> Compiled {
    let packages = qsl_semantics::model::intake::package_input([document()]);
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    let clauses = graph.temporal_clauses().to_vec();
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    Compiled {
        clauses,
        emission,
        wire,
    }
}

/// The I2 evidence for a ConfigVersion package: its required features and
/// the domain package document.
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

/// QSL's I2 read of the wire `bytes`, pinned at `package_id`.
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
    let read = crate::checked_v2::read_checked_package_v2(
        compiled.emission.package.bytes(),
        library(),
        V2ReadLimits::default(),
        &evidence(&compiled.wire),
        &qsl_semantics::library::fixtures::single_pin(
            library(),
            compiled.emission.package.package_id(),
        ),
    )
    .unwrap_or_else(|failure| panic!("the I2 read admits the package: {failure:?}"));
    read.into_value().admitted
}

/// The wire node whose id digest is `digest`.
fn node_with<'w>(wire: &'w Value, digest: &str) -> &'w Value {
    nodes(wire)
        .iter()
        .find(|node| node["node_id"]["digest"] == digest)
        .unwrap_or_else(|| panic!("a node {digest}"))
}

/// The target digest of the `reference` term `term`.
fn target(term: &Value) -> &str {
    assert_eq!(term["term"], "reference", "{term}");
    term["target"]["digest"].as_str().expect("a target digest")
}

/// The one `temporal`/`temporal_clause` node of `wire`.
fn clause_node(wire: &Value) -> &Value {
    only_node(wire, "temporal_clause")
}

/// The formula node the clause node's formula argument references.
fn formula_root<'w>(wire: &'w Value, clause: &Value) -> &'w Value {
    node_with(wire, target(&clause["body"]["arguments"][5]))
}

/// The formula node the `index`-th argument of `formula` references.
fn operand<'w>(wire: &'w Value, formula: &Value, index: usize) -> &'w Value {
    node_with(wire, target(&formula["body"]["arguments"][index]))
}

/// The fairness nodes the clause node's fairness argument references.
fn fairness<'w>(wire: &'w Value, clause: &Value) -> Vec<&'w Value> {
    clause["body"]["arguments"][4]["members"]
        .as_array()
        .expect("a fairness aggregate")
        .iter()
        .map(|member| node_with(wire, target(member)))
        .collect()
}

fn package_id(compiled: &Compiled) -> qsl_semantics::library::PackageId {
    compiled.emission.package.package_id()
}

/// FR-337-AC-1: ADR-018 §6's `ReachesTwo` emits as a
/// `temporal`/`temporal_clause` node whose body is the
/// `quire.op.temporal.clause` application FR-370 states, and QSL's I2 read
/// admits it.
#[trace("TC-524", "FR-337-AC-1")]
#[test]
fn reaches_two_emits_as_a_v2_temporal_clause_node_the_i2_read_admits() {
    let compiled = compile(&unit(
        INFINITE,
        &reaches_two("fair weak each Config::ConfigVersion::attemptUpdate;\n"),
    ));
    let wire = &compiled.wire;
    let clause = clause_node(wire);
    assert_eq!(clause["node_tag"], "temporal");
    let body = &clause["body"];
    assert_eq!(body["term"], "application");
    assert_eq!(body["operator"], "temporal");
    assert_eq!(body["operation"]["identity"], "quire.op.temporal.clause");
    assert_eq!(
        body["operation"]["laws"],
        json!([{"role": "temporal_profile", "definition": {
            "authority": "agent-ix", "identity": INFINITE}}])
    );
    assert_eq!(body["operation"]["member"], Value::Null);
    // The lock selects the same profile under the same role.
    assert!(wire["lock"]["profile_selections"]
        .as_array()
        .unwrap()
        .contains(&json!({"role": "temporal_profile", "definition": {
            "authority": "agent-ix", "identity": INFINITE}})));
    let arguments = body["arguments"].as_array().expect("arguments");
    assert_eq!(arguments.len(), 6);
    // over: a reference to `c`'s parameter node.
    let over = node_with(wire, target(&arguments[0]));
    assert_eq!(over["node_tag"], "value");
    assert_eq!(over["semantic_form"], "parameter");
    assert_eq!(over["body"]["members"][0]["value"]["value"], "c");
    // clock, activation, captures.
    assert_eq!(arguments[1]["value"], "model-steps");
    assert_eq!(arguments[1]["value_kind"], "text");
    assert_eq!(arguments[2]["term"], "aggregate");
    assert_eq!(arguments[2]["members"][0]["name"], "kind");
    assert_eq!(arguments[2]["members"][0]["value"]["value"], "origin");
    assert_eq!(arguments[2]["members"].as_array().unwrap().len(), 1);
    assert_eq!(arguments[3], json!({"term": "aggregate", "members": []}));
    // fairness: one `fair` application, `weak`, `each`, `attemptUpdate`.
    let [fair] = fairness(wire, clause)[..] else {
        panic!("one fairness node");
    };
    assert_eq!(fair["semantic_form"], "fairness");
    assert_eq!(fair["body"]["operator"], "temporal_fairness");
    assert_eq!(
        fair["body"]["operation"]["identity"],
        "quire.op.temporal.fair"
    );
    let member = &fair["body"]["operation"]["member"];
    assert_eq!(member["kind"], "fairness");
    assert_eq!(member["fairness_kind"], "weak");
    assert_eq!(member["granularity"], "each");
    assert_eq!(member["name"], "attemptUpdate");
    let declaring = node_with(wire, member["declaration"]["digest"].as_str().unwrap());
    assert_eq!(declaring["node_tag"], "model");
    assert_eq!(declaring["semantic_form"], "object_type");
    // formula: always(eventually(holds(...))), each interval `null`.
    let always = formula_root(wire, clause);
    let eventually = operand(wire, always, 0);
    let holds = operand(wire, eventually, 0);
    for (node, identity) in [
        (always, "quire.op.temporal.always"),
        (eventually, "quire.op.temporal.eventually"),
    ] {
        assert_eq!(node["semantic_form"], "formula");
        assert_eq!(node["body"]["operator"], "temporal_formula");
        assert_eq!(node["body"]["operation"]["identity"], identity);
        assert_eq!(
            node["body"]["operation"]["member"],
            json!({"kind": "temporal_interval", "interval": null})
        );
    }
    assert_eq!(
        holds["body"]["operation"]["identity"],
        "quire.op.temporal.holds"
    );
    assert_eq!(holds["body"]["operation"]["member"], Value::Null);
    let condition = node_with(wire, target(&holds["body"]["arguments"][0]));
    assert_eq!(condition["node_tag"], "expression");
    // Every node of the clause is typed Boolean and every clause occurrence
    // is a `claim`.
    let boolean = &clause["semantic_type"];
    for node in [always, eventually, holds, fair] {
        assert_eq!(&node["semantic_type"], boolean);
        assert_eq!(node["occurrences"][0]["role"], "expression");
    }
    assert_eq!(body["result_type"], *boolean);
    assert_eq!(
        clause["occurrences"],
        json!([{"role": "claim", "ordinal": 0}])
    );
    // QSL's I2 read admits the package at its own `package_id`.
    let Read::Verified { package, .. } = read_bytes(
        compiled.emission.package.bytes(),
        wire,
        package_id(&compiled),
    ) else {
        panic!("the I2 read admits the clause");
    };
    assert_eq!(package.package_id(), package_id(&compiled));
}

/// The clause node digest and `package_id` of `ReachesTwo` with `fair`.
fn identity_with(profile: &str, body: &str) -> (Compiled, String) {
    let compiled = compile(&unit(profile, &declared("ReachesTwo", body)));
    let digest = clause_node(&compiled.wire)["node_id"]["digest"]
        .as_str()
        .expect("a node id")
        .to_owned();
    (compiled, digest)
}

/// FR-337-AC-2: `fair weak attemptUpdate` and `fair weak whole
/// attemptUpdate` emit byte-identical fairness applications and equal node
/// keys; `each` in place of `whole`, and `eventually[0,5]` in place of
/// `eventually`, each change the node key and the `package_id`.
#[trace("TC-524", "FR-337-AC-2")]
#[test]
fn the_resolved_fairness_set_and_the_intervals_decide_the_node_key() {
    let fair = |granularity: &str| {
        format!("fair weak {granularity}Config::ConfigVersion::attemptUpdate;\n{EVENTUALLY_TWO}")
    };
    let (unmarked, unmarked_key) = identity_with(INFINITE, &fair(""));
    let (whole, whole_key) = identity_with(INFINITE, &fair("whole "));
    let (each, each_key) = identity_with(INFINITE, &fair("each "));
    let [unmarked_fair] = fairness(&unmarked.wire, clause_node(&unmarked.wire))[..] else {
        panic!("one fairness node");
    };
    let [whole_fair] = fairness(&whole.wire, clause_node(&whole.wire))[..] else {
        panic!("one fairness node");
    };
    assert_eq!(jcs(&unmarked_fair["body"]), jcs(&whole_fair["body"]));
    assert_eq!(unmarked_fair["node_id"], whole_fair["node_id"]);
    assert_eq!(unmarked_key, whole_key);
    assert_ne!(each_key, whole_key);
    assert_ne!(package_id(&each), package_id(&whole));

    let bounded = "fair weak Config::ConfigVersion::attemptUpdate;\n\
         always eventually[0,5] holds(c.versionNumber = 2)";
    let (interval, interval_key) = identity_with(INFINITE, bounded);
    assert_ne!(interval_key, unmarked_key);
    assert_ne!(package_id(&interval), package_id(&unmarked));
}

/// The `temporal_interval` member of the formula root of `wire`'s clause.
fn root_interval(wire: &Value) -> Value {
    formula_root(wire, clause_node(wire))["body"]["operation"]["member"].clone()
}

/// TC-524 step 3: under event-position false-extension `eventually[0,5]`
/// emits `quire.op.temporal.eventually` with `{lower: "0", upper: "5"}` and
/// an empty fairness argument; `once[1,1]`, the encoding FR-337 gives
/// strong previous, emits `quire.op.temporal.once` with `{lower: "1",
/// upper: "1"}`; under infinite-trace `eventually[3,*]` emits `{lower: "3",
/// upper: null}`. Each package is admitted by QSL's I2 read.
#[trace("TC-524")]
#[test]
fn each_interval_form_emits_its_temporal_interval_member() {
    let cases = [
        (
            EVENT,
            "eventually[0,5] holds(c.versionNumber = 2)",
            "quire.op.temporal.eventually",
            json!({"lower": "0", "upper": "5"}),
        ),
        (
            EVENT,
            "once[1,1] holds(c.versionNumber = 2)",
            "quire.op.temporal.once",
            json!({"lower": "1", "upper": "1"}),
        ),
        (
            INFINITE,
            "eventually[3,*] holds(c.versionNumber = 2)",
            "quire.op.temporal.eventually",
            json!({"lower": "3", "upper": null}),
        ),
    ];
    for (profile, body, identity, interval) in cases {
        let (compiled, _) = identity_with(profile, body);
        let wire = &compiled.wire;
        let clause = clause_node(wire);
        assert_eq!(
            formula_root(wire, clause)["body"]["operation"]["identity"],
            identity,
            "{body}"
        );
        assert_eq!(
            root_interval(wire),
            json!({"kind": "temporal_interval", "interval": interval}),
            "{body}"
        );
        assert_eq!(
            clause["body"]["arguments"][4],
            json!({"term": "aggregate", "members": []}),
            "{body}"
        );
        assert!(
            matches!(
                read_bytes(
                    compiled.emission.package.bytes(),
                    wire,
                    package_id(&compiled)
                ),
                Read::Verified { .. }
            ),
            "{body}: the I2 read admits it"
        );
    }
}

/// `compiled`'s wire with the node `digest` edited by `edit`, every stale
/// key renamed and the `package_id` recomputed; returns the envelope and
/// the renames.
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

/// QSL's I2 read of `envelope` at its own `package_id`, expected to refuse
/// in IR's reader: the IR refusal.
fn ir_refusal(
    compiled: &Compiled,
    envelope: &Value,
) -> quire_contract_model::CheckedPackageRefusal {
    let package_id =
        qsl_semantics::library::PackageId::of_preimage(&jcs(&envelope["identity_preimage"]));
    match read_bytes(&jcs(envelope), &compiled.wire, package_id) {
        Read::Refused(V2ReadRefusal::Envelope { refusal, .. }) => *refusal,
        other => panic!("an IR refusal, not {other:?}"),
    }
}

/// The JSON pointer IR reported, as text.
fn path(refusal: &quire_contract_model::CheckedPackageRefusal) -> String {
    refusal
        .path
        .as_ref()
        .map(|pointer| pointer.as_str().to_owned())
        .unwrap_or_default()
}

/// The position in `envelope`'s graph of the node now keyed `digest`.
fn position(envelope: &Value, digest: &str) -> usize {
    nodes(envelope)
        .iter()
        .position(|node| node["node_id"]["digest"] == digest)
        .expect("the node is in the graph")
}

/// `digest` after `renames`.
fn renamed(renames: &BTreeMap<String, String>, digest: &str) -> String {
    renames
        .get(digest)
        .cloned()
        .unwrap_or_else(|| digest.to_owned())
}

/// FR-337-AC-4: fed to QSL's I2 read, AC-1's node with its arguments
/// reordered fails the schema; a `null` interval under a bounded profile
/// refuses `invalid_package`/`operation-member-mismatch` at the formula
/// node; a fairness member naming an absent operation refuses
/// `missing_declaration`/`missing-name` at the member's name; and a
/// reference to a `temporal`/`fairness` node inside the formula refuses
/// `ill_typed`/`operator-ineligible` at the referencing node.
#[trace("TC-524", "FR-337-AC-4")]
#[test]
fn the_i2_read_refuses_each_fr370_defect_at_its_locus() {
    use quire_contract_model::CheckedPackageRefusalCause as Cause;
    let compiled = compile(&unit(
        INFINITE,
        &reaches_two("fair weak each Config::ConfigVersion::attemptUpdate;\n"),
    ));
    let wire = &compiled.wire;
    let clause = clause_node(wire);
    let clause_digest = clause["node_id"]["digest"].as_str().unwrap().to_owned();

    // Arguments reordered: over and clock swapped. The read refuses at the
    // first misplaced argument. FR-370 calls this a schema failure; IR's
    // reader, at the revision this workspace locks, reports it from the
    // operation catalog's operand check as `ill_typed`/`operator-ineligible`
    // at the same argument, so only the refusal and its locus are pinned.
    let (envelope, renames) = edited(&compiled, &clause_digest, |node| {
        node["body"]["arguments"]
            .as_array_mut()
            .expect("arguments")
            .swap(0, 1);
    });
    let refusal = ir_refusal(&compiled, &envelope);
    let clause_at = position(&envelope, &renamed(&renames, &clause_digest));
    assert_eq!(
        path(&refusal),
        format!("/semantic_graph/nodes/{clause_at}/body/arguments/0"),
        "{refusal:?}"
    );

    // A fairness member naming an operation `ConfigVersion` does not have.
    let [fair] = fairness(wire, clause)[..] else {
        panic!("one fairness node");
    };
    let fair_digest = fair["node_id"]["digest"].as_str().unwrap().to_owned();
    let (envelope, renames) = edited(&compiled, &fair_digest, |node| {
        node["body"]["operation"]["member"]["name"] = json!("Absent");
    });
    let refusal = ir_refusal(&compiled, &envelope);
    assert_eq!(
        (refusal.code, refusal.cause),
        (
            CheckedPackageRefusalCode::MissingDeclaration,
            Some(Cause::MissingName)
        ),
        "{refusal:?}"
    );
    let fair_at = position(&envelope, &renamed(&renames, &fair_digest));
    assert_eq!(
        path(&refusal),
        format!("/semantic_graph/nodes/{fair_at}/body/operation/member/name")
    );

    // A reference to the fairness node as `eventually`'s operand.
    let always = formula_root(wire, clause);
    let eventually = operand(wire, always, 0);
    let eventually_digest = eventually["node_id"]["digest"].as_str().unwrap().to_owned();
    let (envelope, renames) = edited(&compiled, &eventually_digest, |node| {
        node["body"]["arguments"][0]["target"]["digest"] = json!(fair_digest);
        node["dependencies"] = json!([fair["node_id"]]);
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
    let eventually_at = position(&envelope, &renamed(&renames, &eventually_digest));
    assert_eq!(
        path(&refusal),
        format!("/semantic_graph/nodes/{eventually_at}")
    );

    // A `null` interval under a bounded profile.
    let bounded = compile(&unit(
        EVENT,
        &declared("Bounded", "eventually[0,5] holds(c.versionNumber = 2)"),
    ));
    let root = formula_root(&bounded.wire, clause_node(&bounded.wire));
    let root_digest = root["node_id"]["digest"].as_str().unwrap().to_owned();
    let (envelope, renames) = edited(&bounded, &root_digest, |node| {
        node["body"]["operation"]["member"]["interval"] = Value::Null;
    });
    let refusal = ir_refusal(&bounded, &envelope);
    assert_eq!(
        (refusal.code, refusal.cause),
        (
            CheckedPackageRefusalCode::InvalidPackage,
            Some(Cause::OperationMemberMismatch)
        ),
        "{refusal:?}"
    );
    let root_at = position(&envelope, &renamed(&renames, &root_digest));
    assert_eq!(
        path(&refusal),
        format!("/semantic_graph/nodes/{root_at}/body")
    );
}

/// `key` as a wire node id.
fn wire_id(key: NodeKey) -> WireNodeId {
    WireNodeId::from_digest(*key.as_bytes())
}

/// FR-337-AC-5 (TC-1079 steps 1 and 2): for ADR-018 §6's `ReachesTwo` the
/// I2 lookup returns the triple `[node, "claim", ordinal]`, `node` the
/// emitted clause node, and the `over` node `c`'s parameter node; the
/// triple resolves to the same clause in the recompiled package.
#[trace("TC-1079", "FR-337-AC-5")]
#[test]
fn the_i2_lookup_returns_the_clause_triple_and_its_over_node() {
    let text = unit(
        INFINITE,
        &reaches_two("fair weak each Config::ConfigVersion::attemptUpdate;\n"),
    );
    let compiled = compile(&text);
    let [checked] = compiled.clauses.as_slice() else {
        panic!("one emitted clause");
    };
    let clause = clause_node(&compiled.wire);
    assert_eq!(
        clause["node_id"]["digest"],
        wire_id(checked.identity()).to_string()
    );
    let over = node_with(&compiled.wire, target(&clause["body"]["arguments"][0]));
    assert_eq!(
        over["node_id"]["digest"],
        wire_id(checked.over()).to_string()
    );

    let admitted = admit(&compiled);
    let [root] = clause_roots(&admitted)
        .expect("the roots read")
        .try_into()
        .unwrap_or_else(|roots: Vec<ClauseRoot>| panic!("one root: {roots:?}"));
    let triple = root.occurrence().clone();
    assert_eq!(triple.node(), wire_id(checked.identity()));
    assert_eq!(triple.origin().role().as_str(), "claim");
    assert_eq!(triple.origin(), checked.claim());
    assert_eq!(root.over(), Some(wire_id(checked.over())));

    let recompiled = admit(&compile(&text));
    assert_eq!(resolve_clause_root(&recompiled, &triple), Ok(root));
}

/// FR-337-AC-5 (TC-1079 step 3): two structurally identical clause
/// declarations share one node; the I2 lookup returns two triples with
/// distinct ordinals, each the `claim` occurrence of its own declaration
/// and each resolving.
#[trace("TC-1079", "FR-337-AC-5")]
#[test]
fn identical_declarations_share_one_node_and_keep_their_own_triples() {
    let body = format!("fair weak each Config::ConfigVersion::attemptUpdate;\n{EVENTUALLY_TWO}");
    let compiled = compile(&unit(
        INFINITE,
        &format!(
            "{}\n{}",
            declared("ReachesTwo", &body),
            declared("ReachesTwoAgain", &body)
        ),
    ));
    let [first, second] = compiled.clauses.as_slice() else {
        panic!("two emitted clauses");
    };
    assert_eq!(first.clause().name(), "ReachesTwo");
    assert_eq!(second.clause().name(), "ReachesTwoAgain");
    assert_eq!(first.identity(), second.identity());
    assert_ne!(first.claim(), second.claim());
    let clause = clause_node(&compiled.wire);
    assert_eq!(clause["occurrences"].as_array().map(Vec::len), Some(2));

    let admitted = admit(&compiled);
    let roots = clause_roots(&admitted).expect("the roots read");
    let triples: Vec<&OccurrenceKey> = roots.iter().map(ClauseRoot::occurrence).collect();
    let declared: Vec<OccurrenceKey> = [first, second]
        .iter()
        .map(|declaration| {
            OccurrenceKey::new(wire_id(declaration.identity()), declaration.claim().clone())
        })
        .collect();
    assert_eq!(triples, declared.iter().collect::<Vec<_>>());
    for (root, declaration) in roots.iter().zip([first, second]) {
        assert_eq!(
            resolve_clause_root(&admitted, root.occurrence()),
            Ok(root.clone())
        );
        assert_eq!(root.over(), Some(wire_id(declaration.over())));
    }
}

/// FR-337-AC-5 (TC-1079 step 4): a triple with an unknown node, with a
/// non-temporal node, with role `invariant` and with another ordinal each
/// return `clause-root-unresolved` at that triple.
#[trace("TC-1079", "FR-337-AC-5")]
#[test]
fn a_foreign_clause_root_is_unresolved_at_its_triple() {
    let compiled = compile(&unit(INFINITE, &reaches_two("")));
    let [checked] = compiled.clauses.as_slice() else {
        panic!("one emitted clause");
    };
    let admitted = admit(&compiled);
    let node = wire_id(checked.identity());
    let ordinal = checked.claim().ordinal();
    let claim = |ordinal| Origin::new(Role::new("claim"), ordinal);
    let foreign = [
        OccurrenceKey::new(WireNodeId::from_digest([7; 32]), claim(ordinal)),
        OccurrenceKey::new(wire_id(checked.over()), claim(0)),
        OccurrenceKey::new(node, Origin::new(Role::new("invariant"), ordinal)),
        OccurrenceKey::new(node, claim(ordinal + 1)),
    ];
    for triple in foreign {
        assert_eq!(
            resolve_clause_root(&admitted, &triple),
            Err(crate::checked_v2::ClauseRootUnresolved {
                root: triple.clone()
            }),
            "{triple}"
        );
    }
    assert_eq!(
        crate::checked_v2::ClauseRootUnresolved::TAG,
        "clause-root-unresolved"
    );
}

/// The checked graph of `unit`, with its one temporal clause and that
/// clause's `temporal-satisfaction` record.
fn record_of(
    unit: &str,
) -> (
    qsl_semantics::check::CheckedGraph,
    qsl_foundation::bound::DomainKey,
    qsl_semantics::family::Requirements,
) {
    let packages = qsl_semantics::model::intake::package_input([document()]);
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    let [checked] = graph.temporal_clauses() else {
        panic!("one emitted clause");
    };
    let clause = qsl_foundation::bound::DomainKey::Node {
        node: wire_id(checked.identity()),
        path: Vec::new(),
    };
    let key = OccurrenceKey::new(wire_id(checked.identity()), checked.claim().clone());
    let record = graph.requirements().get(&key).expect("the clause's record");
    let requirements = record.requirements().clone();
    (graph, clause, requirements)
}

/// FR-336-AC-1 through the real emission: the compiled `ReachesTwo`'s
/// `temporal-satisfaction` record is `Unbounded` with exactly the
/// population's domain and the formula's infinite-trace domain, no domain
/// of any universe; the unit's `bound` on `config_history` takes the
/// population out.
#[trace("TC-846", "FR-336-AC-1")]
#[test]
fn the_emitted_record_carries_the_population_domain_and_no_universe() {
    use qsl_foundation::bound::DomainKind;
    use qsl_semantics::family::ClaimExtent;

    let text = unit(INFINITE, &reaches_two(""));
    let (graph, clause, requirements) = record_of(&text);
    let population = graph
        .population_domain(
            &Identifier::new("Config").expect("an identifier"),
            &Identifier::new("config_history").expect("an identifier"),
        )
        .expect("keyed")
        .expect("config_history resolves");
    assert_eq!(requirements.kind(), qsl_semantics::check::Capability::TemporalSatisfaction);
    let ClaimExtent::Unbounded(domains) = requirements.extent() else {
        panic!("unbounded: {:?}", requirements.extent());
    };
    assert_eq!(domains.len(), 2);
    assert_eq!(domains.kind(&population), Some(DomainKind::Population));
    assert_eq!(domains.kind(&clause), Some(DomainKind::InfiniteTrace));

    let bounded = unit(
        INFINITE,
        &format!(
            "bound Cap on Config::config_history [3]\n{}",
            reaches_two("")
        ),
    );
    let (_, _, requirements) = record_of(&bounded);
    let ClaimExtent::Unbounded(domains) = requirements.extent() else {
        panic!("the formula is infinite-trace");
    };
    assert_eq!(domains.len(), 1);
}
