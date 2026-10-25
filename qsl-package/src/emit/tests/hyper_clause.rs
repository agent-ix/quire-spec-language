// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-488 (TC-1266): a checked hyper clause over `behaviours` emits as QSpec
//! FR-402's `claim`/`hyperproperty` node with its body wrapper and slot
//! nodes, QSL's I2 read admits it through strict wire validation and the
//! structural stage, and the I2 lookup returns each declaration's identity
//! triple. The subject is the ConfigVersion domain package, whose one
//! operation is `attemptUpdate`.

use qsl_semantics::check::{CheckCause, TemporalCause};

use super::admission_corpus::config_version_unit;
use super::*;
use crate::checked_v2::{
    clause_roots, hyper_roots, resolve_clause_root, resolve_hyper_root, HyperRootUnresolved,
};

const INFINITE: &str = "quire.temporal.infinite-trace/v1";
const EVENT: &str = "quire.temporal.event-position.false-extension/v1";

/// `a` and `b` read the same version number.
const READ_A_B: &str = "always holds(c.versionNumber @ a = c.versionNumber @ b)";

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

/// A hyper clause `name` over two traces; `quantifiers` follows `forall trace
/// a of Config` and `match_block` is the whole `match { .. }` text or empty.
fn hyper(name: &str, fair_a: &str, fair_b: &str, match_block: &str, body: &str) -> String {
    format!(
        "hyper {name} using t over behaviours (c: Config::ConfigVersion) \
         forall trace a of Config {fair_a} forall trace b of Config {fair_b} \
         {match_block} {{ {body} }}"
    )
}

fn non_interference() -> String {
    hyper(
        "NonInterference",
        "",
        "",
        "match { a.step.op = b.step.op }",
        READ_A_B,
    )
}

struct Compiled {
    graph_clauses: Vec<qsl_semantics::check::CheckedHyperClause>,
    emission: Emission,
    wire: Value,
}

fn compile(unit: &str) -> Compiled {
    let packages = qsl_semantics::model::intake::package_input([document()]);
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    let graph_clauses = graph.hyper_clauses().to_vec();
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    Compiled {
        graph_clauses,
        emission,
        wire,
    }
}

/// The refusals of `unit`'s check.
fn refusals(unit: &str) -> Vec<qsl_semantics::check::CheckRefusal> {
    let packages = qsl_semantics::model::intake::package_input([document()]);
    assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect_err("the unit is refused")
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

/// QSL's full I2 read of `compiled`: strict wire validation, the structural
/// stage and every later stage of IR's reader.
fn admit(compiled: &Compiled) -> std::sync::Arc<quire_contract_model::CheckedPackageV2> {
    crate::checked_v2::read_checked_package_v2(
        compiled.emission.package.bytes(),
        library(),
        V2ReadLimits::default(),
        &evidence(&compiled.wire),
        &qsl_semantics::library::fixtures::single_pin(
            library(),
            compiled.emission.package.package_id(),
        ),
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

/// The member of the aggregate `body` bound to `name`.
fn binding<'v>(body: &'v Value, name: &str) -> &'v Value {
    body["members"]
        .as_array()
        .expect("an aggregate")
        .iter()
        .find(|member| member["name"] == name)
        .map(|member| &member["value"])
        .unwrap_or_else(|| panic!("a binding {name} in {body}"))
}

fn hyper_node(wire: &Value) -> &Value {
    only_node(wire, "hyperproperty")
}

fn text(term: &Value) -> &str {
    assert_eq!(term["value_kind"], "text", "{term}");
    term["value"].as_str().expect("text")
}

/// The slot parameter nodes of `wire`, by name.
fn slots(wire: &Value) -> BTreeMap<String, &Value> {
    nodes(wire)
        .iter()
        .filter(|node| node["semantic_form"] == "parameter")
        .map(|node| (text(binding(&node["body"], "name")).to_owned(), node))
        .collect()
}

/// FR-488-AC-1, AC-2, AC-3, AC-6: `NonInterference` emits one
/// `claim`/`hyperproperty` node whose body binds the seven FR-402 members in
/// order, with the `quantifiers` Tuple of Groups.
#[trace("TC-1266", "FR-488-AC-1", "FR-488-AC-2", "FR-488-AC-3", "FR-488-AC-6")]
#[test]
fn a_hyper_clause_emits_the_hyperproperty_node_with_its_seven_bindings() {
    let compiled = compile(&unit(INFINITE, &non_interference()));
    let node = hyper_node(&compiled.wire);
    assert_eq!(node["node_tag"], "claim");
    let names: Vec<&str> = node["body"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|member| member["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "domain",
            "subjects",
            "parameters",
            "quantifiers",
            "match",
            "align_skip",
            "body"
        ]
    );
    let body = &node["body"];
    assert_eq!(text(binding(body, "domain")), "behaviours");
    let subjects = binding(body, "subjects")["members"].as_array().unwrap();
    assert_eq!(subjects.len(), 1);
    assert_eq!(subjects[0]["name"], "Config");
    let parameters = binding(body, "parameters")["members"].as_array().unwrap();
    assert_eq!(parameters.len(), 1);
    assert_eq!(parameters[0]["name"], "c");
    assert_eq!(
        target(&subjects[0]["value"]),
        target(&parameters[0]["value"])
    );
    let quantifiers = binding(body, "quantifiers")["members"].as_array().unwrap();
    assert_eq!(quantifiers.len(), 2);
    for (group, variable) in quantifiers.iter().zip(["a", "b"]) {
        let members = &group["members"];
        let field = |name: &str| {
            text(
                members
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|member| member["name"] == name)
                    .map(|member| &member["value"])
                    .unwrap(),
            )
        };
        assert_eq!(field("kind"), "quantifier");
        assert_eq!(field("variable"), variable);
        assert_eq!(field("quantifier"), "forall");
        assert_eq!(field("alias"), "Config");
    }
    assert_eq!(
        binding(body, "match")["members"].as_array().unwrap().len(),
        1
    );
    assert_eq!(binding(body, "align_skip")["members"], json!([]));
}

/// FR-488-AC-4, AC-5: the `body` binding references a
/// `temporal`/`temporal_clause` wrapper over the slot-0 node, and the slot
/// nodes are named by FR-488 "Slot node names" at their index as level.
#[trace("TC-1266", "FR-488-AC-4", "FR-488-AC-5", "FR-488-AC-23")]
#[test]
fn the_body_wrapper_reads_slot_zero_and_slots_are_named_identifier_safe() {
    let compiled = compile(&unit(INFINITE, &non_interference()));
    let wire = &compiled.wire;
    let node = hyper_node(wire);
    let wrapper = node_with(wire, target(binding(&node["body"], "body")));
    assert_eq!(wrapper["semantic_form"], "temporal_clause");
    let arguments = &wrapper["body"]["arguments"];
    assert_eq!(
        wrapper["body"]["operation"]["identity"],
        "quire.op.temporal.clause"
    );
    assert_eq!(text(&arguments[1]), "model-steps");
    assert_eq!(arguments[3]["members"], json!([]));
    assert_eq!(arguments[4]["members"], json!([]));
    let slots = slots(wire);
    let over = node_with(wire, target(&arguments[0]));
    assert_eq!(text(binding(&over["body"], "name")), "slot_sc_sa");
    assert_eq!(binding(&over["body"], "level")["value"], "0");
    let other = slots.get("slot_sc_sb").expect("the slot c@b");
    assert_eq!(binding(&other["body"], "level")["value"], "1");
    for name in slots.keys() {
        assert!(
            name.bytes()
                .next()
                .is_some_and(|first| first.is_ascii_alphabetic())
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'),
            "{name}"
        );
    }
    // The conjunct's `op` reads of `a` and `b`.
    assert!(slots.contains_key("op_sa") && slots.contains_key("op_sb"));
}

/// FR-488-AC-22: every emitted shape is admitted by the I2 read, which runs
/// strict wire validation first: no `malformed_wire`, no `InvalidSemanticGraph`.
#[trace("TC-1266", "FR-488-AC-22")]
#[test]
fn every_emitted_hyper_shape_is_admitted_by_the_i2_read() {
    let cases = [
        unit(INFINITE, &non_interference()),
        unit(
            INFINITE,
            &hyper(
                "Fair",
                "fair weak each Config::ConfigVersion::attemptUpdate;",
                "fair Config::ConfigVersion::attemptUpdate;",
                "",
                READ_A_B,
            ),
        ),
        unit(
            INFINITE,
            &hyper(
                "Skip",
                "",
                "",
                "align skip { Config::ConfigVersion::attemptUpdate } match { a.step.op = b.step.op }",
                READ_A_B,
            ),
        ),
        unit(
            INFINITE,
            &hyper("Receivers", "", "", "match { a.step.receiver = b.step.receiver }", READ_A_B),
        ),
    ];
    for case in cases {
        let compiled = compile(&case);
        admit(&compiled);
    }
}

/// FR-488-AC-13: the fairness Groups follow their quantifier, with the
/// resolved kind and granularity (unmarked is `weak` and `whole`).
#[trace("TC-1266", "FR-488-AC-13")]
#[test]
fn fairness_constraints_follow_their_quantifier_as_resolved() {
    let compiled = compile(&unit(
        INFINITE,
        &hyper(
            "Fair",
            "fair weak Config::ConfigVersion::attemptUpdate;",
            "fair strong each Config::ConfigVersion::attemptUpdate;",
            "",
            READ_A_B,
        ),
    ));
    let node = hyper_node(&compiled.wire);
    let groups = binding(&node["body"], "quantifiers")["members"]
        .as_array()
        .unwrap();
    let field = |group: &Value, name: &str| {
        group["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|member| member["name"] == name)
            .map(|member| member["value"].clone())
            .unwrap_or_else(|| panic!("{name} in {group}"))
    };
    assert_eq!(groups.len(), 4);
    assert_eq!(text(&field(&groups[1], "kind")), "fairness");
    assert_eq!(text(&field(&groups[1], "variable")), "a");
    assert_eq!(text(&field(&groups[1], "fairness_kind")), "weak");
    assert_eq!(text(&field(&groups[1], "granularity")), "whole");
    assert_eq!(text(&field(&groups[1], "name")), "attemptUpdate");
    assert_eq!(text(&field(&groups[3], "fairness_kind")), "strong");
    assert_eq!(text(&field(&groups[3], "granularity")), "each");
    assert_eq!(text(&field(&groups[3], "variable")), "b");
}

/// FR-488-AC-10: an `align skip` operation is the `operation_anchor` node a
/// state clause naming it shares.
#[trace("TC-1266", "FR-488-AC-10")]
#[test]
fn an_align_skip_operation_shares_the_state_clause_anchor() {
    let declarations = format!(
        "post VersionUnchanged using v on Config::ConfigVersion::attemptUpdate {{ \
         self.versionNumber = pre(self.versionNumber) }}\n{}",
        hyper(
            "Skip",
            "",
            "",
            "align skip { Config::ConfigVersion::attemptUpdate } match { a.step.op = b.step.op }",
            READ_A_B,
        )
    );
    let compiled = compile(&unit(INFINITE, &declarations));
    let node = hyper_node(&compiled.wire);
    let skipped = binding(&node["body"], "align_skip")["members"]
        .as_array()
        .unwrap();
    assert_eq!(skipped.len(), 1);
    let anchor = node_with(&compiled.wire, target(&skipped[0]));
    assert_eq!(anchor["semantic_form"], "operation_anchor");
    assert_eq!(
        nodes(&compiled.wire)
            .iter()
            .filter(|node| node["semantic_form"] == "operation_anchor")
            .count(),
        1
    );
    admit(&compiled);
}

/// FR-488-AC-11, AC-12: the `match` block changes the NodeId; a renamed
/// clause shares one node with distinct `claim` ordinals; a renamed trace
/// variable changes the NodeId.
#[trace("TC-1266", "FR-488-AC-11", "FR-488-AC-12")]
#[test]
fn equal_clauses_share_a_node_and_differing_ones_do_not() {
    let id = |declarations: &str| {
        let compiled = compile(&unit(INFINITE, declarations));
        hyper_node(&compiled.wire)["node_id"]["digest"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    let base = id(&non_interference());
    let no_match = id(&hyper("NonInterference", "", "", "", READ_A_B));
    assert_ne!(base, no_match);
    let renamed_trace = id(
        "hyper NonInterference using t over behaviours (c: Config::ConfigVersion) \
         forall trace x of Config forall trace b of Config \
         match { x.step.op = b.step.op } { always holds(c.versionNumber @ x = c.versionNumber @ b) }",
    );
    assert_ne!(base, renamed_trace);

    let twice = compile(&unit(
        INFINITE,
        &format!(
            "{}\n{}",
            non_interference(),
            non_interference().replace("NonInterference", "Other")
        ),
    ));
    let hyper_nodes: Vec<_> = nodes(&twice.wire)
        .iter()
        .filter(|node| node["semantic_form"] == "hyperproperty")
        .collect();
    assert_eq!(hyper_nodes.len(), 1);
    let ordinals: Vec<u64> = hyper_nodes[0]["occurrences"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|occurrence| occurrence["role"] == "claim")
        .map(|occurrence| occurrence["ordinal"].as_u64().unwrap())
        .collect();
    assert_eq!(ordinals, [0, 1]);
}

/// FR-488-AC-14, AC-15: the checked clause's `occurrence()` equals the I2
/// reader's triple; an unknown or non-hyper node is `hyper-root-unresolved`;
/// the body wrapper is no temporal clause root.
#[trace("TC-1266", "FR-488-AC-14", "FR-488-AC-15")]
#[test]
fn the_occurrence_round_trips_and_the_wrapper_is_no_clause_root() {
    let compiled = compile(&unit(INFINITE, &non_interference()));
    let admitted = admit(&compiled);
    let roots = hyper_roots(&admitted).expect("the roots read");
    assert_eq!(roots.len(), 1);
    let [clause] = &compiled.graph_clauses[..] else {
        panic!("one clause");
    };
    assert_eq!(clause.occurrence(), roots[0].occurrence());
    assert_eq!(clause.assembled_occurrence(), Some(roots[0].occurrence()));
    let resolved = resolve_hyper_root(&admitted, clause.occurrence()).expect("resolves");
    assert_eq!(&resolved, &roots[0]);

    let wrapper = roots[0].body().expect("a body wrapper");
    let wrapper_triple = quire_exact::Origin::new(quire_exact::Role::new("claim"), 0);
    let wrapper_key = OccurrenceKey::new(wrapper, wrapper_triple.clone());
    assert_eq!(
        resolve_hyper_root(&admitted, &wrapper_key),
        Err(HyperRootUnresolved {
            root: wrapper_key.clone()
        })
    );
    assert_eq!(HyperRootUnresolved::TAG, "hyper-root-unresolved");
    let wrong_ordinal = OccurrenceKey::new(
        roots[0].occurrence().node(),
        quire_exact::Origin::new(quire_exact::Role::new("claim"), 7),
    );
    assert!(resolve_hyper_root(&admitted, &wrong_ordinal).is_err());

    assert_eq!(clause_roots(&admitted).expect("reads"), []);
    assert!(resolve_clause_root(&admitted, &wrapper_key).is_err());
}

/// FR-488-AC-18, AC-19: a fairness set under a bounded profile is refused
/// `unsupported_construct`/`not-yet-implemented` as not emitted.
#[trace("TC-1266", "FR-488-AC-18", "FR-488-AC-19")]
#[test]
fn a_fairness_set_under_a_bounded_profile_is_not_emitted() {
    let declarations = hyper(
        "Bounded",
        "fair weak Config::ConfigVersion::attemptUpdate;",
        "",
        "",
        "eventually [0, 2] holds(c.versionNumber @ a = c.versionNumber @ b)",
    );
    let refusals = refusals(&unit(EVENT, &declarations));
    assert!(
        refusals.iter().any(|refusal| matches!(
            &refusal.cause,
            CheckCause::Temporal(cause) if matches!(**cause, TemporalCause::NotEmitted { .. })
        )),
        "{refusals:?}"
    );
}

/// FR-488-AC-21: the nodes of the clause count against the node ceiling.
#[trace("TC-1266", "FR-488-AC-21")]
#[test]
fn the_emitted_nodes_count_against_the_node_ceiling() {
    let text = unit(INFINITE, &non_interference());
    let packages = qsl_semantics::model::intake::package_input([document()]);
    let tight = CheckingLimits::default().with_nodes(8);
    let refused = assemble_with_models(text.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(tight);
    assert!(refused.is_err(), "eight nodes cannot hold the clause");
}
