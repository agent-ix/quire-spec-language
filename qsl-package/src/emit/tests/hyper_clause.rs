// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-488 (TC-1266): a checked hyper clause over `behaviours` emits as QSpec
//! FR-402's `claim`/`hyperproperty` node with its body wrapper and slot
//! nodes, QSL's I2 read admits it through strict wire validation and the
//! structural stage, and the I2 lookup returns each declaration's identity
//! triple. The subject is the ConfigVersion domain package, whose one
//! operation is `attemptUpdate`.

use qsl_semantics::check::{CheckCause, CheckRefusal, CheckingLimitKind, MatchPart, TemporalCause};

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


/// A unit selecting the ConfigVersion model, then `declarations`.
fn unit(profile: &str, declarations: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         profile t = \"{profile}\";\n\
         model Config = \"example/config-version\";\n\
         {declarations}\n"
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
    let packages = qsl_semantics::model::intake::package_input([document()]).unwrap();
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
    let packages = qsl_semantics::model::intake::package_input([document()]).unwrap();
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
        .insert_domain_package_document(package_identity(&document()), document().to_vec())
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
#[trace("TC-1266", "FR-488-AC-1", "FR-488-AC-2", "FR-488-AC-3")]
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
#[trace("TC-1266", "FR-488-AC-20")]
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

// ---------------------------------------------------------------------
// Models with operations that take parameters, and several aliases.
// ---------------------------------------------------------------------

/// The ConfigVersion domain package under `identity`, with `step(i)` and
/// `mix()` added to `ConfigVersion` (ADR-023 §8's vault shape).
fn vault(identity: &str) -> Vec<u8> {
    let mut document: Value = serde_json::from_str(config_version_unit::DOMAIN_PACKAGE).unwrap();
    let attempt = document["types"][1]["operations"][0].clone();
    let operation = |name: &str, params: Value| {
        let mut operation = attempt.clone();
        operation["name"] = json!(name);
        operation["identity"] = json!(format!("ix://example/config-version/ConfigVersion/{name}"));
        operation["params"] = params;
        operation["frame"] = json!({"modifies": [], "creates": [], "deletes": []});
        operation
    };
    let parameter = json!({
        "identity": "ix://example/config-version/ConfigVersion/step/i",
        "name": "i",
        "typeRef": "ix://example/config-version/VersionNumber",
        "presence": "required",
        "nullable": false,
        "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": {"generated": {
            "generatorIdentity": "ix://example/config-version/ConfigVersion/step/i",
            "generatorVersion": "1.0.0",
            "inputIdentities": ["ix://example/config-version/ConfigVersion/step/i"],
        }},
    });
    let step = operation("step", json!([parameter]));
    let mix = operation("mix", json!([]));
    document["types"][1]["operations"] = json!([attempt, step, mix]);
    serde_json::to_string(&document)
        .unwrap()
        .replace("example/config-version", identity)
        .into_bytes()
}

fn alias_identity(alias: &str) -> &'static str {
    match alias {
        "V" => "example/config-version",
        "S" => "example/device-saving",
        "N" => "example/device-normal",
        other => panic!("no model {other}"),
    }
}


/// A unit over the models `aliases` under `profile`, then `declarations`.
fn world_unit(profile: &str, aliases: &[&str], declarations: &str) -> (String, Vec<Vec<u8>>) {
    let mut text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         profile t = \"{profile}\";\n"
    );
    let mut documents = Vec::new();
    for alias in aliases {
        let identity = alias_identity(alias);
        let document = vault(identity);
        text.push_str(&format!(
            "model {alias} = \"{identity}\";\n"
        ));
        documents.push(document);
    }
    text.push_str(declarations);
    text.push('\n');
    (text, documents)
}

fn packages_of(documents: &[Vec<u8>]) -> qsl_semantics::model::intake::PackageInput {
    qsl_semantics::model::intake::package_input(documents.iter().map(Vec::as_slice)).unwrap()
}

/// A built world: the checked clauses, the emission, the wire and the
/// documents it was admitted against.
struct Built {
    clauses: Vec<qsl_semantics::check::CheckedHyperClause>,
    emission: Emission,
    wire: Value,
    documents: Vec<Vec<u8>>,
}

fn build(profile: &str, aliases: &[&str], declarations: &str) -> Built {
    let (text, documents) = world_unit(profile, aliases, declarations);
    let graph = assemble_with_models(text.as_bytes(), &packages_of(&documents))
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}\n{text}"));
    let clauses = graph.hyper_clauses().to_vec();
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    Built {
        clauses,
        emission,
        wire,
        documents,
    }
}

/// The refusals of the world, with the unit text.
fn refused(profile: &str, aliases: &[&str], declarations: &str) -> (String, Vec<CheckRefusal>) {
    let (text, documents) = world_unit(profile, aliases, declarations);
    let refusals = assemble_with_models(text.as_bytes(), &packages_of(&documents))
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect_err("the unit is refused");
    (text, refusals)
}

/// QSL's full I2 read of `built`.
fn admitted(built: &Built) -> std::sync::Arc<quire_contract_model::CheckedPackageV2> {
    let mut evidence = CheckedPackageEvidence::new();
    for feature in built.wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    for document in &built.documents {
        evidence
            .insert_domain_package_document(package_identity(&document.clone()), document.clone())
            .expect("a fresh evidence key");
    }
    crate::checked_v2::read_checked_package_v2(
        built.emission.package.bytes(),
        library(),
        V2ReadLimits::default(),
        &evidence,
        &qsl_semantics::library::fixtures::single_pin(
            library(),
            built.emission.package.package_id(),
        ),
    )
    .unwrap_or_else(|failure| panic!("the I2 read admits the package: {failure:?}"))
    .into_value()
    .admitted
}

/// The text a not-emitted refusal's span covers in `text`.
fn not_emitted_text<'t>(text: &'t str, refusals: &[CheckRefusal]) -> &'t str {
    let [CheckRefusal {
        cause: CheckCause::Temporal(cause),
        ..
    }] = refusals
    else {
        panic!("one temporal refusal: {refusals:?}");
    };
    let TemporalCause::NotEmitted { span, .. } = &**cause else {
        panic!("not emitted: {cause:?}");
    };
    &text[span.start..span.end]
}

/// FR-488-AC-6: `SavesPower` over two aliases lists its subjects in order of
/// first naming, its parameters, and its slots by object parameter and trace.
#[trace("TC-1266", "FR-488-AC-6")]
#[test]
fn two_aliases_list_their_subjects_parameters_and_slots() {
    let declarations = "hyper SavesPower using t over behaviours \
         (d: S::ConfigVersion, e: N::ConfigVersion) forall trace s of S forall trace n of N \
         match { s.step.ConfigVersion::step.i = n.step.ConfigVersion::step.i } \
         { always holds(d.versionNumber @ s <= e.versionNumber @ n) }";
    let built = build(INFINITE, &["S", "N"], declarations);
    let body = &hyper_node(&built.wire)["body"];
    let subjects = binding(body, "subjects")["members"].as_array().unwrap();
    let names: Vec<_> = subjects
        .iter()
        .map(|m| m["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["S", "N"]);
    assert_ne!(target(&subjects[0]["value"]), target(&subjects[1]["value"]));
    let parameters = binding(body, "parameters")["members"].as_array().unwrap();
    let names: Vec<_> = parameters
        .iter()
        .map(|m| m["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["d", "e"]);
    let slots = slots(&built.wire);
    assert!(slots.contains_key("slot_sd_ss") && slots.contains_key("slot_se_sn"));
    admitted(&built);
}

/// The `(name, level)` of every slot parameter node of `wire`.
fn slot_levels(wire: &Value) -> Vec<(String, String)> {
    let mut levels: Vec<(String, String)> = nodes(wire)
        .iter()
        .filter(|node| node["semantic_form"] == "parameter")
        .map(|node| {
            (
                text(binding(&node["body"], "name")).to_owned(),
                binding(&node["body"], "level")["value"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
            )
        })
        .collect();
    levels.sort();
    levels
}

fn clause_over(name: &str, quantifiers: &str, match_block: &str) -> String {
    format!(
        "hyper {name} using t over behaviours (c: V::ConfigVersion) {quantifiers} \
         {match_block} {{ always holds(c.versionNumber @ a = c.versionNumber @ b) }}"
    )
}

const TWO_FORALL: &str = "forall trace a of V forall trace b of V";

/// FR-488-AC-7, AC-8: argument, receiver and operation reads mint slot nodes
/// named by their step label with the types FR-487 gives; a constant
/// conjunct is a reference to a boolean literal node; `op_sb` is a different
/// node at level 0 and at level 1.
#[trace("TC-1266", "FR-488-AC-7", "FR-488-AC-8")]
#[test]
fn match_conjuncts_mint_typed_step_label_slots_at_their_own_levels() {
    let declarations = clause_over(
        "Reads",
        TWO_FORALL,
        "match { a.step.op = V::ConfigVersion::step and b.step.op = stutter and true and \
         a.step.ConfigVersion::step.i = b.step.ConfigVersion::step.i and \
         a.step.receiver = b.step.receiver }",
    );
    let built = build(INFINITE, &["V"], &declarations);
    let slots = slots(&built.wire);
    let type_form = |name: &str| {
        let slot = slots.get(name).unwrap_or_else(|| panic!("a slot {name}"));
        node_with(
            &built.wire,
            slot["semantic_type"]["digest"].as_str().unwrap(),
        )["semantic_form"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    let op = type_form("op_sa");
    assert_eq!(op, type_form("op_sb"));
    let argument = type_form("arg_sa_sConfigVersion_sstep_si");
    assert_eq!(argument, type_form("arg_sb_sConfigVersion_sstep_si"));
    assert_ne!(
        op, argument,
        "an argument read is an Option, an op a String"
    );
    assert_eq!(argument, type_form("rcv_sa"));
    // Five conjuncts in written order: the constant one is a reference.
    let node = hyper_node(&built.wire);
    let terms = binding(&node["body"], "match")["members"]
        .as_array()
        .unwrap();
    assert_eq!(terms.len(), 5);
    let constant = node_with(&built.wire, target(&terms[2]));
    assert_eq!(constant["node_tag"], "value");
    assert_eq!(constant["body"]["term"], "literal");
    assert_eq!(constant["body"]["value_kind"], "boolean");
    admitted(&built);

    // `op_sb` alone is level 0; after `a.step.op` it is level 1.
    let both = build(
        INFINITE,
        &["V"],
        &clause_over(
            "Levels",
            TWO_FORALL,
            "match { b.step.op = stutter and a.step.op = b.step.op }",
        ),
    );
    let levels = slot_levels(&both.wire);
    assert!(levels.contains(&("op_sa".into(), "0".into())), "{levels:?}");
    assert!(levels.contains(&("op_sb".into(), "0".into())), "{levels:?}");
    assert!(levels.contains(&("op_sb".into(), "1".into())), "{levels:?}");
    admitted(&both);
}

/// FR-488-AC-9: interleaved universal and existential conjuncts keep written
/// order in `match` and `conjuncts()`.
#[trace("TC-1266", "FR-488-AC-9")]
#[test]
fn interleaved_conjuncts_keep_their_written_order() {
    let declarations = clause_over(
        "Interleaved",
        "forall trace a of V exists trace b of V",
        "match { a.step.op = a.step.op and a.step.op = b.step.op and \
         a.step.receiver = a.step.receiver and b.step.op = a.step.op }",
    );
    let built = build(INFINITE, &["V"], &declarations);
    let [clause] = &built.clauses[..] else {
        panic!("one clause");
    };
    assert_eq!(
        clause.conjuncts(),
        [
            (MatchPart::Universal, 0),
            (MatchPart::Existential, 0),
            (MatchPart::Universal, 1),
            (MatchPart::Existential, 1),
        ]
    );
    let node = hyper_node(&built.wire);
    assert_eq!(
        binding(&node["body"], "match")["members"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    admitted(&built);
}

/// FR-488-AC-16, AC-17: a `let` rooted in `a`'s slot read inside `@ a`
/// emits; the same read inside `@ b` is refused at the read it dereferences;
/// a read with no slot root is refused.
#[trace("TC-1266", "FR-488-AC-16", "FR-488-AC-17")]
#[test]
fn a_read_must_stay_in_the_component_its_slot_root_names() {
    let linked = |atom: &str| {
        format!(
            "hyper Linked using t over behaviours (c: V::ConfigVersion) {TWO_FORALL} \
             {{ always holds({atom}) }}"
        )
    };
    let emits =
        linked("(let r = c.parent in present(r) and deref(value(r)).versionNumber = 1) @ a");
    admitted(&build(INFINITE, &["V"], &emits));

    let crossing =
        linked("(let r = (c.parent @ a) in present(r) and deref(value(r)).versionNumber = 1) @ b");
    let (text, refusals) = refused(INFINITE, &["V"], &crossing);
    assert_eq!(
        not_emitted_text(&text, &refusals),
        "deref(value(r)).versionNumber"
    );
}

/// FR-488-AC-18: a clause with two refusals reports the first in the order
/// of "Clauses S4 does not emit": an alias that binds no object parameter
/// (at the clause) before a fairness set under a bounded profile (at the
/// constraint).
#[trace("TC-1266", "FR-488-AC-18")]
#[test]
fn the_first_refusal_in_precedence_order_is_reported() {
    let body = "eventually [0, 2] holds(d.versionNumber @ s = d.versionNumber @ s)";
    let alias_only = format!(
        "hyper Unbound using t over behaviours (d: S::ConfigVersion) \
         forall trace s of S forall trace n of N {{ {body} }}"
    );
    let (text, refusals) = refused(EVENT, &["S", "N"], &alias_only);
    assert!(
        not_emitted_text(&text, &refusals).starts_with("hyper Unbound"),
        "an alias with no parameter is refused at the clause"
    );
    let both = format!(
        "hyper Unbound using t over behaviours (d: S::ConfigVersion) \
         forall trace s of S fair weak S::ConfigVersion::mix; forall trace n of N {{ {body} }}"
    );
    let (text, refusals) = refused(EVENT, &["S", "N"], &both);
    assert!(
        not_emitted_text(&text, &refusals).starts_with("hyper Unbound"),
        "the clause-level refusal precedes the constraint's"
    );
}

/// FR-488-AC-19: bounded and open intervals emit with their values.
#[trace("TC-1266", "FR-488-AC-19")]
#[test]
fn bounded_and_open_intervals_emit_and_are_admitted() {
    for (profile, formula, interval) in [
        (
            EVENT,
            "always [0, 3] holds(c.versionNumber @ a = c.versionNumber @ b)",
            ("0", "3"),
        ),
        (
            INFINITE,
            "eventually [2, *] holds(c.versionNumber @ a = c.versionNumber @ b)",
            ("2", ""),
        ),
    ] {
        let declarations = format!(
            "hyper Window using t over behaviours (c: V::ConfigVersion) {TWO_FORALL} {{ {formula} }}"
        );
        let built = build(profile, &["V"], &declarations);
        let wire = serde_json::to_string(&built.wire).unwrap();
        assert!(wire.contains(&format!("\"{}\"", interval.0)), "{wire}");
        admitted(&built);
    }
}

/// FR-488-AC-20: a fairness set under a bounded profile is refused at the
/// constraint.
#[trace("TC-1266", "FR-488-AC-20")]
#[test]
fn a_bounded_fairness_refusal_is_located_at_the_constraint() {
    let declarations = clause_over(
        "Bounded",
        "forall trace a of V fair weak V::ConfigVersion::mix; forall trace b of V",
        "",
    )
    .replace("always holds", "always [0, 3] holds");
    let (text, refusals) = refused(EVENT, &["V"], &declarations);
    assert!(
        not_emitted_text(&text, &refusals).starts_with("fair weak V::ConfigVersion::mix"),
        "{}",
        not_emitted_text(&text, &refusals)
    );
}

/// FR-488-AC-21: the nodes of the clause count against the node ceiling:
/// the exact count compiles and one fewer is refused with the node limit.
#[trace("TC-1266", "FR-488-AC-21")]
#[test]
fn the_emitted_nodes_count_against_the_node_ceiling_exactly() {
    let (text, documents) = world_unit(INFINITE, &["V"], &non_interference_v());
    let packages = packages_of(&documents);
    let check = |nodes: u64| {
        assemble_with_models(text.as_bytes(), &packages)
            .expect("the unit assembles")
            .check(CheckingLimits::default().with_nodes(nodes))
    };
    let needed = (1..200)
        .find(|nodes| check(*nodes).is_ok())
        .expect("a ceiling admits the clause");
    assert!(needed > 1);
    let refusals = check(needed - 1).expect_err("one node short");
    let [refusal] = &refusals[..] else {
        panic!("one refusal: {refusals:?}");
    };
    let CheckCause::ResourceExhausted(limit) = &refusal.cause else {
        panic!("a node limit: {refusal:?}");
    };
    assert_eq!(limit.kind, CheckingLimitKind::Nodes);
    assert_eq!(limit.limit, needed - 1);
    assert!(check(needed).is_ok());
}

fn non_interference_v() -> String {
    clause_over(
        "NonInterference",
        TWO_FORALL,
        "match { a.step.op = b.step.op }",
    )
}

/// FR-488-AC-11, AC-12, AC-13: each variant has a different NodeId;
/// reformatting and a changed clause name keep it; `weak whole` equals the
/// unmarked constraint and `each` differs.
#[trace("TC-1266", "FR-488-AC-11", "FR-488-AC-12", "FR-488-AC-13")]
#[test]
fn node_ids_follow_the_checked_clause() {
    let id = |declarations: String| {
        let built = build(INFINITE, &["V"], &declarations);
        hyper_node(&built.wire)["node_id"]["digest"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    let with =
        |quantifiers: &str, matched: &str| clause_over("NonInterference", quantifiers, matched);
    let base = id(non_interference_v());
    let m = "match { a.step.op = b.step.op }";
    let fair = "fair weak V::ConfigVersion::mix;";
    let variants = [
        id(with("forall trace a of V exists trace b of V", m)),
        id(with("forall trace b of V forall trace a of V", m)),
        id(with(
            &format!("forall trace a of V {fair} forall trace b of V"),
            m,
        )),
        id(with(
            &format!("forall trace a of V forall trace b of V {fair}"),
            m,
        )),
        id(with(
            TWO_FORALL,
            "match { a.step.op = b.step.op and a.step.receiver = b.step.receiver }",
        )),
        id(with(
            "forall trace a of V forall trace b of V align skip { V::ConfigVersion::mix }",
            m,
        )),
        id(non_interference_v()
            .replace("<=", "=")
            .replace("= c.version", "!= c.version")),
    ];
    for (i, variant) in variants.iter().enumerate() {
        assert_ne!(&base, variant, "variant {i}");
        for other in &variants[..i] {
            assert_ne!(other, variant, "variant {i}");
        }
    }
    // Reformatting and a renamed clause keep the NodeId.
    let reformatted = non_interference_v()
        .replace("NonInterference", "Renamed")
        .replace(" forall", "\n   forall")
        .replace("{ always", "{\n always");
    assert_eq!(base, id(reformatted));
    // `weak whole` is the unmarked constraint; `each` is not.
    let marked = |written: &str| {
        id(with(
            &format!("forall trace a of V {written} V::ConfigVersion::mix; forall trace b of V"),
            m,
        ))
    };
    assert_eq!(marked("fair"), marked("fair weak whole"));
    assert_ne!(marked("fair weak whole"), marked("fair weak each"));
}

/// FR-488-AC-4, AC-5, AC-13: the wrapper's law, activation and formula, the
/// fairness `declaration` reference, and the slot nodes the formula reads.
#[trace("TC-1266", "FR-488-AC-4", "FR-488-AC-5", "FR-488-AC-13")]
#[test]
fn the_wrapper_carries_the_profile_law_activation_and_formula() {
    let declarations = clause_over(
        "Fair",
        "forall trace a of V fair weak V::ConfigVersion::mix; forall trace b of V",
        "",
    );
    let built = build(INFINITE, &["V"], &declarations);
    let wire = &built.wire;
    let node = hyper_node(wire);
    let wrapper = node_with(wire, target(binding(&node["body"], "body")));
    assert_eq!(
        wrapper["body"]["operation"]["laws"],
        json!([{"role": "temporal_profile", "definition": {
            "authority": "agent-ix", "identity": INFINITE}}])
    );
    let arguments = &wrapper["body"]["arguments"];
    assert_eq!(arguments[2]["members"].as_array().unwrap().len(), 1);
    assert_eq!(arguments[2]["members"][0]["name"], "kind");
    assert_eq!(text(&arguments[2]["members"][0]["value"]), "origin");
    let formula = node_with(wire, target(&arguments[5]));
    assert_eq!(formula["semantic_form"], "formula");
    assert_eq!(
        formula["body"]["operation"]["identity"],
        "quire.op.temporal.always"
    );
    let operand = formula["body"]["arguments"]
        .as_array()
        .unwrap()
        .iter()
        .find(|argument| argument["term"] == "reference")
        .expect("the formula's operand");
    let holds = node_with(wire, target(operand));
    assert_eq!(holds["semantic_form"], "formula");
    // The fairness Group's `declaration` is the model node `subjects` names.
    let groups = binding(&node["body"], "quantifiers")["members"]
        .as_array()
        .unwrap();
    let declaration = groups[1]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|member| member["name"] == "declaration")
        .unwrap();
    let subjects = binding(&node["body"], "subjects")["members"]
        .as_array()
        .unwrap();
    assert_eq!(target(&declaration["value"]), target(&subjects[0]["value"]));
    admitted(&built);
}

/// FR-488-AC-14, AC-15: lookups of an unknown NodeId, a non-hyper NodeId,
/// another role and another ordinal return `hyper-root-unresolved`.
#[trace("TC-1266", "FR-488-AC-14", "FR-488-AC-15")]
#[test]
fn lookups_of_what_is_no_hyper_clause_are_unresolved() {
    let built = build(INFINITE, &["V"], &non_interference_v());
    let package = admitted(&built);
    let root = hyper_roots(&package).unwrap().remove(0);
    let claim = |node, role: &str, ordinal| {
        OccurrenceKey::new(
            node,
            quire_exact::Origin::new(quire_exact::Role::new(role), ordinal),
        )
    };
    let slot = qsl_foundation::digest::WireNodeId::from_hex(
        slots(&built.wire)["slot_sc_sa"]["node_id"]["digest"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    for key in [
        claim(
            qsl_foundation::digest::WireNodeId::from_digest([9; 32]),
            "claim",
            0,
        ),
        claim(slot, "claim", 0),
        claim(root.occurrence().node(), "invariant", 0),
        claim(root.occurrence().node(), "claim", 9),
    ] {
        assert_eq!(
            resolve_hyper_root(&package, &key),
            Err(HyperRootUnresolved { root: key })
        );
    }
}
