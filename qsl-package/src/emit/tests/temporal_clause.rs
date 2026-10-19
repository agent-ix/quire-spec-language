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
fn clause(name: &str, body: &str) -> String {
    format!(
        "temporal {name} using t over (c: Config::ConfigVersion) clock \"model-steps\" \
         on origin {{ {body} }}"
    )
}

/// ADR-018 §6's `ReachesTwo` with fairness `fair` (empty for none).
fn reaches_two(fair: &str) -> String {
    clause("ReachesTwo", &format!("{fair}{EVENTUALLY_TWO}"))
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
fn admitted(compiled: &Compiled) -> std::sync::Arc<quire_contract_model::CheckedPackageV2> {
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
    assert_eq!(fair["body"]["operation"]["identity"], "quire.op.temporal.fair");
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
    assert_eq!(holds["body"]["operation"]["identity"], "quire.op.temporal.holds");
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
    assert_eq!(clause["occurrences"], json!([{"role": "claim", "ordinal": 0}]));
    // QSL's I2 read admits the package at its own `package_id`.
    let Read::Verified { package, .. } =
        read_bytes(compiled.emission.package.bytes(), wire, package_id(&compiled))
    else {
        panic!("the I2 read admits the clause");
    };
    assert_eq!(package.package_id(), package_id(&compiled));
}
