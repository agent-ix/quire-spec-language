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
use crate::checked_v2::{clause_roots, resolve_clause_root, ClauseRoot, V2ReadRefusal};

const INFINITE: &str = "quire.temporal.infinite-trace/v1";
const EVENT: &str = "quire.temporal.event-position.false-extension/v1";

/// ADR-018 §6's clause formula.
const EVENTUALLY_TWO: &str = "always eventually holds(c.versionNumber = 2)";

/// The ConfigVersion domain package document.
fn document() -> &'static [u8] {
    config_version_unit::DOMAIN_PACKAGE.as_bytes()
}

/// A unit selecting the ConfigVersion model and the temporal profile
/// `profile` as `t`, then `declarations`.
fn unit(profile: &str, declarations: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         profile t = \"{profile}\";\n\
         model Config = \"example/config-version\";\n\
         {declarations}\n"
    )
}

/// The clause `name` over `c: Config::ConfigVersion` whose body is `body`.
fn declared(name: &str, body: &str) -> String {
    declared_on(name, "on origin", body)
}

/// The clause `name` over `c: Config::ConfigVersion`, activated as
/// `activation`, whose body is `body`.
fn declared_on(name: &str, activation: &str, body: &str) -> String {
    format!(
        "temporal {name} using t over (c: Config::ConfigVersion) clock \"model-steps\" \
         {activation} {{ {body} }}"
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
    let graph = check_graph(unit);
    let selections = graph
        .temporal_clauses()
        .iter()
        .filter_map(|temporal| {
            Some(crate::emit::TemporalClockSelection {
                source: graph.source().clone(),
                declaration_span: temporal.clause().span(),
                clock: fixture_clock(temporal.clause().profile())?,
            })
        })
        .collect::<Vec<_>>();
    let clauses = graph.temporal_clauses().to_vec();
    let emission = crate::emit::emit_checked_with_temporal_selections(
        &CheckedPackage::link(graph),
        &selections,
    )
    .expect("the package emits");
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    Compiled {
        clauses,
        emission,
        wire,
    }
}

fn fixture_clock(
    profile: qsl_semantics::check::TemporalProfile,
) -> Option<quire_contract_model::ClockConfiguration> {
    use qsl_semantics::check::TemporalProfile as P;
    use quire_contract_model::{ClockConfiguration as C, ClockExactNumber as N};

    match profile {
        P::EventPositionFalseExtension => Some(C::EventPosition {
            sequence_authority: "test-order-events".into(),
        }),
        P::FixedSampleFalseExtension => Some(C::FixedSample {
            epoch: N::Integer {
                decimal: "0".into(),
            },
            period: N::Rational {
                numerator: "1".into(),
                denominator: "10".into(),
            },
            unit: "test-milliseconds".into(),
        }),
        P::TimestampedEventFiniteWindow => Some(C::TimestampedEvent {
            timestamp_unit: "test-rfc3339".into(),
        }),
        P::InfiniteTrace | P::Timed => None,
    }
}

fn check_graph(unit: &str) -> qsl_semantics::check::CheckedGraph {
    let packages = qsl_semantics::model::intake::package_input([document()]).unwrap();
    assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"))
}

/// FR-322-AC-294: the emitter joins each explicit clock selection to one
/// checked source locus and writes the selected typed clock beside the exact
/// emitted declaration id. The clock name comes from the checked clause.
#[trace("TC-134", "FR-322-AC-294")]
#[test]
fn event_position_binding_uses_the_exact_checked_selection() {
    let compiled = compile(&unit(
        EVENT,
        &declared("Bound", "eventually[0,1] holds(true)"),
    ));
    let bindings = compiled.wire["temporal_bindings"]
        .as_array()
        .expect("clock-configured clauses emit bindings");
    let [binding] = bindings.as_slice() else {
        panic!("one checked clause produces one selected clock binding: {bindings:?}");
    };
    assert_eq!(binding["clock_name"], "model-steps");
    assert_eq!(
        binding["clock"],
        json!({"kind": "event_position", "sequence_authority": "test-order-events"})
    );
    assert_eq!(
        binding["declaration"],
        clause_node(&compiled.wire)["node_id"]
    );
}

/// FR-322-AC-294: the compatibility entry point cannot invent a selection
/// for a profile whose checked declarations require clock configuration.
#[trace("TC-134", "FR-322-AC-294")]
#[test]
fn configured_temporal_clause_without_selection_refuses_emission() {
    let source_text = unit(
        EVENT,
        &declared("Bound", "eventually[0,1] holds(c.versionNumber = 2)"),
    );
    let source_len = source_text.len() as u64;
    let package = CheckedPackage::link(check_graph(&source_text));
    let refusal = emit_checked(&package)
        .expect_err("configured temporal clauses require explicit clock selection");
    assert!(matches!(
        refusal,
        crate::emit::EmitRefusal::MissingTemporalClockSelection { .. }
    ));
    let refusal = crate::emit::emit_checked_with_cancel(&package, &quire_exact::Cancel::new())
        .expect_err("the cancelable default also refuses to invent a clock configuration");
    assert!(matches!(
        refusal,
        crate::emit::EmitRefusal::MissingTemporalClockSelection { .. }
    ));
    let source = package.graph().source().clone();
    let refusal = crate::emit::emit_package(&package, move |_| {
        Some(
            qsl_foundation::source::provenance::SourceRegion::new(source.clone(), 0, source_len)
                .unwrap(),
        )
    })
    .expect_err("caller-placed emission also requires an explicit clock configuration");
    assert!(matches!(
        refusal,
        crate::emit::EmitRefusal::MissingTemporalClockSelection { .. }
    ));
}

/// FR-322-AC-294: configuration shape is checked against the selected law,
/// and selections cannot be duplicated or moved to a different source locus.
#[trace("TC-134", "FR-322-AC-294")]
#[test]
fn temporal_clock_selections_are_unique_locus_bound_and_profile_typed() {
    let graph = check_graph(&unit(
        EVENT,
        &declared("Bound", "eventually[0,1] holds(c.versionNumber = 2)"),
    ));
    let temporal = &graph.temporal_clauses()[0];
    let event_selection = crate::emit::TemporalClockSelection {
        source: graph.source().clone(),
        declaration_span: temporal.clause().span(),
        clock: fixture_clock(temporal.clause().profile()).expect("event profile selection"),
    };
    let package = CheckedPackage::link(graph);

    let duplicate = [event_selection.clone(), event_selection.clone()];
    assert!(matches!(
        crate::emit::emit_checked_with_temporal_selections(&package, &duplicate),
        Err(crate::emit::EmitRefusal::DuplicateTemporalClockSelection { .. })
    ));

    let wrong_profile = crate::emit::TemporalClockSelection {
        clock: quire_contract_model::ClockConfiguration::TimestampedEvent {
            timestamp_unit: "test-rfc3339".into(),
        },
        ..event_selection.clone()
    };
    assert!(matches!(
        crate::emit::emit_checked_with_temporal_selections(&package, &[wrong_profile]),
        Err(crate::emit::EmitRefusal::TemporalClockProfileMismatch { .. })
    ));

    let wrong_source = crate::emit::TemporalClockSelection {
        source: qsl_foundation::source::provenance::RawSourceRef::new(
            "test-authority",
            "different-source",
        )
        .expect("valid fixture source"),
        ..event_selection
    };
    assert!(matches!(
        crate::emit::emit_checked_with_temporal_selections(&package, &[wrong_source]),
        Err(crate::emit::EmitRefusal::MissingTemporalClockSelection { .. })
    ));
}

/// The I2 evidence for a ConfigVersion package: its required features and
/// the domain package document.
fn evidence(wire: &Value) -> CheckedPackageEvidence {
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    evidence
        .insert_domain_package_document(package_identity(document()), document().to_vec())
        .expect("a fresh evidence key");
    evidence
}

/// QSL's I2 read of the wire `bytes`.
fn read_bytes(bytes: &[u8], wire: &Value) -> Read {
    read_v2(bytes, library(), V2ReadLimits::default(), &evidence(wire))
}

/// QSL's full I2 read of `compiled`, and IR's admitted package.
fn admit(compiled: &Compiled) -> std::sync::Arc<quire_contract_model::CheckedPackageV2> {
    let read = crate::checked_v2::read_checked_package_v2(
        compiled.emission.package.bytes(),
        library(),
        V2ReadLimits::default(),
        &evidence(&compiled.wire),
    )
    .unwrap_or_else(|failure| panic!("the I2 read admits the package: {failure:?}"));
    read.into_value().admitted
}

/// The wire node with structural id `key`.
fn node_with<'w>(wire: &'w Value, key: &Value) -> &'w Value {
    nodes(wire)
        .iter()
        .find(|node| &node["node_id"] == key)
        .unwrap_or_else(|| panic!("a node {key}"))
}

/// The target of the `reference` term `term`.
fn target(term: &Value) -> &Value {
    assert_eq!(term["term"], "reference", "{term}");
    &term["target"]
}

fn node_key(wire: &Value, value: &Value) -> NodeKey {
    qsl_semantics::check::wire_id::WireGraph::of_package_value(wire)
        .unwrap()
        .key_of_value(value)
        .expect("a structural node key")
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
    assert_eq!(member["target"]["kind"], "operation");
    assert_eq!(member["target"]["name"], "attemptUpdate");
    let declaring = node_with(wire, &member["target"]["declaration"]);
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
    // QSL's I2 read admits the package by its selected library name.
    let Read::Verified { .. } = read_bytes(compiled.emission.package.bytes(), wire) else {
        panic!("the I2 read admits the clause");
    };
}

/// The emitted clause node of `ReachesTwo` with `body`.
fn identity_with(profile: &str, body: &str) -> (Compiled, Value) {
    let compiled = compile(&unit(profile, &declared("ReachesTwo", body)));
    let key = clause_node(&compiled.wire)["node_id"].clone();
    (compiled, key)
}

/// FR-337-AC-2: implicit `whole` and explicit `whole` emit equal fairness
/// applications; `each` and bounded `eventually` emit their distinct members.
#[trace("TC-524", "FR-337-AC-2")]
#[test]
fn the_resolved_fairness_set_and_the_intervals_decide_the_node_key() {
    let fair = |granularity: &str| {
        format!("fair weak {granularity}Config::ConfigVersion::attemptUpdate;\n{EVENTUALLY_TWO}")
    };
    let (unmarked, unmarked_key) = identity_with(INFINITE, &fair(""));
    let (whole, whole_key) = identity_with(INFINITE, &fair("whole "));
    let (each, _) = identity_with(INFINITE, &fair("each "));
    let [unmarked_fair] = fairness(&unmarked.wire, clause_node(&unmarked.wire))[..] else {
        panic!("one fairness node");
    };
    let [whole_fair] = fairness(&whole.wire, clause_node(&whole.wire))[..] else {
        panic!("one fairness node");
    };
    assert_eq!(jcs(&unmarked_fair["body"]), jcs(&whole_fair["body"]));
    assert_eq!(unmarked_fair["node_id"], whole_fair["node_id"]);
    assert_eq!(unmarked_key, whole_key);
    let [each_fair] = fairness(&each.wire, clause_node(&each.wire))[..] else {
        panic!("one fairness node");
    };
    assert_eq!(
        each_fair["body"]["operation"]["member"]["granularity"],
        "each"
    );

    let bounded = "fair weak Config::ConfigVersion::attemptUpdate;\n\
         always eventually[0,5] holds(c.versionNumber = 2)";
    let (interval, _) = identity_with(INFINITE, bounded);
    // The root is `always`; the interval belongs to the `eventually` it holds.
    let always = formula_root(&interval.wire, clause_node(&interval.wire));
    let eventually = operand(&interval.wire, always, 0);
    assert_eq!(
        eventually["body"]["operation"]["member"]["interval"],
        json!({"lower": "0", "upper": "5"})
    );
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
                read_bytes(compiled.emission.package.bytes(), wire),
                Read::Verified { .. }
            ),
            "{body}: the I2 read admits it"
        );
    }
}

/// `compiled`'s wire with one structurally keyed node edited by `edit`.
fn edited(compiled: &Compiled, key: &Value, edit: impl FnOnce(&mut Value)) -> Value {
    let mut envelope = compiled.wire.clone();
    let node = envelope["semantic_graph"]["nodes"]
        .as_array_mut()
        .expect("nodes")
        .iter_mut()
        .find(|node| &node["node_id"] == key)
        .expect("the edited node");
    edit(node);
    envelope
}

/// QSL's I2 read of `envelope`, expected to refuse in IR's reader.
fn ir_refusal(
    compiled: &Compiled,
    envelope: &Value,
) -> quire_contract_model::CheckedPackageRefusal {
    match read_bytes(&jcs(envelope), &compiled.wire) {
        Read::Refused(V2ReadRefusal::Envelope { refusal, .. }) => *refusal,
        other => panic!("an IR refusal, not {other:?}"),
    }
}

/// The JSON pointer IR reported, as text.
fn path(refusal: &quire_contract_model::CheckedPackageRefusal) -> String {
    refusal.loci[0]
        .path
        .as_ref()
        .map(|pointer| pointer.as_str().to_owned())
        .unwrap_or_default()
}

/// The position in `envelope`'s graph of the node keyed `key`.
fn position(envelope: &Value, key: &Value) -> usize {
    nodes(envelope)
        .iter()
        .position(|node| &node["node_id"] == key)
        .expect("the node is in the graph")
}

/// FR-337-AC-4: fed to QSL's I2 read, AC-1's node with its arguments
/// reordered fails the schema; a `null` interval under a bounded profile
/// refuses `invalid_package`/`operation-member-mismatch` at the formula
/// node; a fairness member naming an absent operation refuses
/// `missing_declaration`/`missing-name` at the target's name; and a
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
    let clause_key = clause["node_id"].clone();

    // Arguments reordered: over and clock swapped. The read refuses at the
    // first misplaced argument. FR-370 calls this a schema failure; IR's
    // reader, at the revision this workspace locks, reports it from the
    // operation catalog's operand check as `ill_typed`/`operator-ineligible`
    // at the same argument, so only the refusal and its locus are pinned.
    let envelope = edited(&compiled, &clause_key, |node| {
        node["body"]["arguments"]
            .as_array_mut()
            .expect("arguments")
            .swap(0, 1);
    });
    let refusal = ir_refusal(&compiled, &envelope);
    let clause_at = position(&envelope, &clause_key);
    assert_eq!(
        path(&refusal),
        format!("/semantic_graph/nodes/{clause_at}/body/arguments/0"),
        "{refusal:?}"
    );

    // A fairness member naming an operation `ConfigVersion` does not have.
    let [fair] = fairness(wire, clause)[..] else {
        panic!("one fairness node");
    };
    let fair_key = fair["node_id"].clone();
    let envelope = edited(&compiled, &fair_key, |node| {
        node["body"]["operation"]["member"]["target"]["name"] = json!("Absent");
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
    let fair_at = position(&envelope, &fair_key);
    assert_eq!(
        path(&refusal),
        format!("/semantic_graph/nodes/{fair_at}/body/operation/member/target/name")
    );

    // A reference to the fairness node as `eventually`'s operand.
    let always = formula_root(wire, clause);
    let eventually = operand(wire, always, 0);
    let eventually_key = eventually["node_id"].clone();
    let envelope = edited(&compiled, &eventually_key, |node| {
        node["body"]["arguments"][0]["target"] = fair_key.clone();
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
    let eventually_at = position(&envelope, &eventually_key);
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
    let root_key = root["node_id"].clone();
    let envelope = edited(&bounded, &root_key, |node| {
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
    let root_at = position(&envelope, &root_key);
    assert_eq!(
        path(&refusal),
        format!("/semantic_graph/nodes/{root_at}/body")
    );
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
        node_key(&compiled.wire, &clause["node_id"]),
        checked.identity()
    );
    let over = node_with(&compiled.wire, target(&clause["body"]["arguments"][0]));
    assert_eq!(node_key(&compiled.wire, &over["node_id"]), checked.over());

    let admitted = admit(&compiled);
    let [root] = clause_roots(&admitted)
        .expect("the roots read")
        .try_into()
        .unwrap_or_else(|roots: Vec<ClauseRoot>| panic!("one root: {roots:?}"));
    let triple = root.occurrence().clone();
    assert_eq!(triple.node(), &checked.identity());
    assert_eq!(triple.origin().role().as_str(), "claim");
    assert_eq!(triple.origin(), checked.claim());
    assert_eq!(root.over(), Some(&checked.over()));

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
        .map(|declaration| OccurrenceKey::new(declaration.identity(), declaration.claim().clone()))
        .collect();
    assert_eq!(triples, declared.iter().collect::<Vec<_>>());
    for (root, declaration) in roots.iter().zip([first, second]) {
        assert_eq!(
            resolve_clause_root(&admitted, root.occurrence()),
            Ok(root.clone())
        );
        assert_eq!(root.over(), Some(&declaration.over()));
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
    let node = checked.identity();
    let ordinal = checked.claim().ordinal();
    let claim = |ordinal| Origin::new(Role::new("claim"), ordinal);
    let foreign = [
        OccurrenceKey::new(
            NodeKey::new("pkg", vec![PathSegment::Text("missing".into())], 0).unwrap(),
            claim(ordinal),
        ),
        OccurrenceKey::new(checked.over(), claim(0)),
        OccurrenceKey::new(node.clone(), Origin::new(Role::new("invariant"), ordinal)),
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
    let packages = qsl_semantics::model::intake::package_input([document()]).unwrap();
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    let [checked] = graph.temporal_clauses() else {
        panic!("one emitted clause");
    };
    let clause = qsl_foundation::bound::DomainKey::Node {
        node: checked.identity(),
        path: Vec::new(),
    };
    let key = OccurrenceKey::new(checked.identity(), checked.claim().clone());
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
            &quire_exact::Identifier::new("Config").expect("an identifier"),
            &quire_exact::Identifier::new("config_history").expect("an identifier"),
        )
        .expect("keyed")
        .expect("config_history resolves");
    assert_eq!(
        requirements.kind(),
        qsl_semantics::check::Capability::TemporalSatisfaction
    );
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

/// The body of the `Over` clause every activation test shares.
const BODY: &str = "always holds(c.versionNumber <= 1)";

/// `ONE` compiled `on each (q: Config::ConfigVersion)` under `suffix`.
fn each(binder: &str, suffix: &str) -> String {
    format!("on each ({binder}: Config::ConfigVersion){suffix}")
}

/// Whether `term` reaches `key` through graph references.
fn reaches(wire: &Value, term: &Value, key: &Value) -> bool {
    match term {
        Value::Object(fields) => {
            if fields.get("term").and_then(Value::as_str) == Some("reference") {
                let target = target(term);
                return target == key || reaches(wire, &node_with(wire, target)["body"], key);
            }
            fields.values().any(|value| reaches(wire, value, key))
        }
        Value::Array(items) => items.iter().any(|item| reaches(wire, item, key)),
        _ => false,
    }
}

/// The member of `activation` named `name`.
fn member<'a>(activation: &'a Value, name: &str) -> Option<&'a Value> {
    activation["members"]
        .as_array()
        .expect("an activation aggregate")
        .iter()
        .find(|member| member["name"] == name)
}

/// The structural clause node key of `activation`.
fn key_of(activation: &str, body: &str) -> (Compiled, Value) {
    let compiled = compile(&unit(INFINITE, &declared_on("Each", activation, body)));
    let key = clause_node(&compiled.wire)["node_id"].clone();
    (compiled, key)
}

/// FR-337-AC-6 (TC-1265 steps 1 and 2): an `on each` clause emits with no
/// `NotEmitted` refusal; its activation binds `kind` `each`, `trigger` the
/// level-1 parameter node `q` typed as `over` is, and `guard` only when
/// written, the guard's read of `c` referencing the level-0 `over` node. The
/// guard-less, `when (true)`, guarded and `on origin` clauses have four
/// keys; the guard and the binder name each change the key, formatting does
/// not; the I2 read admits each package and returns its triple and `over`.
#[trace("TC-1265", "FR-337-AC-6")]
#[test]
fn an_on_each_clause_emits_its_trigger_and_guard() {
    let guarded = each("q", " when (c.versionNumber >= 3)");
    let (compiled, _) = key_of(&guarded, BODY);
    let wire = &compiled.wire;
    let clause = clause_node(wire);
    let arguments = clause["body"]["arguments"].as_array().expect("arguments");
    let over = node_with(wire, target(&arguments[0]));
    let activation = &arguments[2];
    assert_eq!(
        member(activation, "kind").unwrap()["value"]["value"],
        "each"
    );
    let trigger = node_with(
        wire,
        target(&member(activation, "trigger").unwrap()["value"]),
    );
    assert_eq!(trigger["node_tag"], "value");
    assert_eq!(trigger["semantic_form"], "parameter");
    assert_eq!(trigger["body"]["members"][0]["value"]["value"], "q");
    assert_eq!(trigger["body"]["members"][1]["value"]["value"], "1");
    assert_eq!(over["body"]["members"][1]["value"]["value"], "0");
    assert_eq!(trigger["semantic_type"], over["semantic_type"]);
    assert_ne!(trigger["node_id"], over["node_id"]);
    let over_key = &over["node_id"];
    let guard = &member(activation, "guard").expect("a guard")["value"];
    assert!(reaches(wire, guard, over_key), "the guard reads `c`");
    assert!(
        !reaches(wire, guard, &trigger["node_id"]),
        "the guard does not read `q`"
    );

    // The trigger node's `expression` occurrence and its type node's `type`
    // occurrence are located at the binder's name.
    let text = unit(INFINITE, &declared_on("Each", &guarded, BODY));
    let packages = qsl_semantics::model::intake::package_input([document()]).unwrap();
    let declarations = assemble_with_models(text.as_bytes(), &packages).expect("assembles");
    let regions = declarations.regions();
    let graph = declarations
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    let at = u64::try_from(text.find("(q:").unwrap() + 1).unwrap();
    let located = |wire_key: &Value, role: &str| -> Vec<u64> {
        let sought = node_key(wire, wire_key);
        graph
            .occurrences()
            .filter(|(key, origin, _)| key == &sought && origin.role().as_str() == role)
            .map(|(_, _, location)| regions.region(location).expect("a region").start())
            .collect()
    };
    assert_eq!(located(&trigger["node_id"], "expression"), [at]);
    assert!(located(&over["semantic_type"], "type").contains(&at));

    let (none, _) = key_of(&each("q", ""), BODY);
    let (truth, _) = key_of(&each("q", " when (true)"), BODY);
    let (origin, _) = key_of("on origin", BODY);
    let [none_activation, truth_activation, origin_activation] = [&none, &truth, &origin]
        .map(|compiled| clause_node(&compiled.wire)["body"]["arguments"][2].clone());
    assert!(member(&none_activation, "trigger").is_some());
    assert!(member(&none_activation, "guard").is_none());
    assert!(member(&truth_activation, "guard").is_some());
    // `when (true)` lowers to the Boolean literal `true`, located at `true`.
    let truth_wire = &truth.wire;
    let truth_guard = node_with(
        truth_wire,
        target(&member(&truth_activation, "guard").unwrap()["value"]),
    );
    assert_eq!(truth_guard["body"]["term"], "literal");
    assert_eq!(truth_guard["body"]["value_kind"], "boolean");
    assert_eq!(truth_guard["body"]["value"], true);
    let truth_text = unit(
        INFINITE,
        &declared_on("Each", &each("q", " when (true)"), BODY),
    );
    let truth_packages = qsl_semantics::model::intake::package_input([document()]).unwrap();
    let truth_declarations =
        assemble_with_models(truth_text.as_bytes(), &truth_packages).expect("assembles");
    let truth_regions = truth_declarations.regions();
    let truth_graph = truth_declarations
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    let guard_key = node_key(truth_wire, &truth_guard["node_id"]);
    let guard_at: Vec<_> = truth_graph
        .occurrences()
        .filter(|(key, origin, _)| *key == guard_key && origin.role().as_str() == "expression")
        .map(|(_, _, location)| truth_regions.region(location).expect("a region").start())
        .collect();
    let literal_at = truth_text.find("when (true)").unwrap() + "when (".len();
    assert_eq!(guard_at, [u64::try_from(literal_at).unwrap()]);
    assert_eq!(
        origin_activation["members"].as_array().map(Vec::len),
        Some(1)
    );
    for compiled in [&compiled, &none, &truth, &origin] {
        let [checked] = compiled.clauses.as_slice() else {
            panic!("one emitted clause");
        };
        let admitted = admit(compiled);
        let [root] = clause_roots(&admitted)
            .expect("the roots read")
            .try_into()
            .unwrap_or_else(|roots: Vec<ClauseRoot>| panic!("one root: {roots:?}"));
        assert_eq!(root.occurrence().node(), &checked.identity());
        assert_eq!(root.over(), Some(&checked.over()));
    }
}

/// The code and tag of the one refusal of `activation` and `body`, and the
/// start offset of its region in the unit text.
fn refused_on(activation: &str, body: &str) -> (Code, Option<&'static str>, u64, String) {
    let text = unit(INFINITE, &declared_on("Each", activation, body));
    let packages = qsl_semantics::model::intake::package_input([document()]).unwrap();
    let declarations = assemble_with_models(text.as_bytes(), &packages).expect("assembles");
    let regions = declarations.regions();
    let refusals = declarations
        .check(CheckingLimits::default())
        .expect_err("the clause is refused");
    let [refusal] = refusals.as_slice() else {
        panic!("one refusal: {refusals:?}");
    };
    let start = regions.refusal_region(refusal).expect("a region").start();
    (refusal.cause.code(), refusal.cause.cause(), start, text)
}

/// FR-337-AC-7 (TC-1265 steps 3 and 4): S3 checks the guard as a `holds`
/// condition, reads of `q` resolve in the guard alone, the binder is typed as
/// `over` is and named otherwise.
#[trace("TC-1265", "FR-337-AC-7")]
#[test]
fn s3_checks_the_on_each_guard_and_binder() {
    // A guard reading `q` checks, and its read references the trigger node.
    let (compiled, _) = key_of(&each("q", " when (q.versionNumber = 1)"), BODY);
    let wire = &compiled.wire;
    let activation = &clause_node(wire)["body"]["arguments"][2];
    let trigger = target(&member(activation, "trigger").unwrap()["value"]);
    assert!(reaches(
        wire,
        &member(activation, "guard").unwrap()["value"],
        trigger
    ));

    // Each case: the activation, the body, the refusal's code and tag, and
    // the text it is located at with the index of that text's occurrence.
    let cases: [(String, &str, Code, &str, &str, usize); 6] = [
        (
            each("q", " when (c.versionNumber)"),
            BODY,
            Code::IllTyped,
            "non-boolean-root",
            "c.versionNumber)",
            0,
        ),
        (
            each("q", ""),
            "always holds(q.versionNumber <= 1)",
            Code::MissingDeclaration,
            "missing-name",
            "q.versionNumber",
            0,
        ),
        (
            each("q", " when (c.versionNumber = c.versionNumber + 1)"),
            BODY,
            Code::UndefinedExpression,
            "unproved-range",
            "c.versionNumber + 1",
            0,
        ),
        (
            "on each (q: Config::Absent)".to_owned(),
            BODY,
            Code::MissingDeclaration,
            "missing-name",
            "Config::Absent",
            0,
        ),
        (
            "on each (q: Integer)".to_owned(),
            BODY,
            Code::IllTyped,
            "type-mismatch",
            "Integer",
            0,
        ),
        (
            each("c", ""),
            BODY,
            Code::ConflictingDeclaration,
            "duplicate-declaration",
            "c:",
            1,
        ),
    ];
    for (activation, body, code, tag, at, nth) in cases {
        let (found, found_tag, start, text) = refused_on(&activation, body);
        assert_eq!((found, found_tag), (code, Some(tag)), "{activation}");
        let (offset, _) = text.match_indices(at).nth(nth).expect("the refused text");
        assert_eq!(start, u64::try_from(offset).unwrap(), "{activation}");
    }

    // A divisor with no S3 law checks, in a unit that selects the complete
    // value layer through a function.
    let text = unit(
        INFINITE,
        &format!(
            "function Noop using v(): Boolean pure {{ true }}\n{}",
            declared_on(
                "Each",
                &each("q", " when (6 mod (2 - c.versionNumber) = 0)"),
                BODY
            )
        ),
    );
    compile(&text);

    // A guard that is not Boolean and a `holds` condition that is not refuse
    // once, at the guard.
    let (code, tag, start, text) = refused_on(
        &each("q", " when (c.versionNumber)"),
        "always holds(c.versionNumber)",
    );
    assert_eq!((code, tag), (Code::IllTyped, Some("non-boolean-root")));
    assert_eq!(
        start,
        u64::try_from(text.find("c.versionNumber)").unwrap()).unwrap()
    );
}

/// The one refusal of `activation` and `body` and the unit text it refuses.
fn refusal_of(activation: &str, body: &str) -> (qsl_semantics::check::CheckRefusal, String) {
    let text = unit(INFINITE, &declared_on("Each", activation, body));
    let packages = qsl_semantics::model::intake::package_input([document()]).unwrap();
    let refusals = assemble_with_models(text.as_bytes(), &packages)
        .expect("assembles")
        .check(CheckingLimits::default())
        .expect_err("the clause is refused");
    let [refusal] = <[_; 1]>::try_from(refusals).unwrap_or_else(|refusals| {
        panic!("one refusal: {refusals:?}");
    });
    (refusal, text)
}

/// FR-337-AC-7: a binder that repeats the `over` parameter's name refuses
/// `duplicate-declaration` at the binder, and the cause's `first` is the
/// `over` parameter's name, so that both declarations are named.
#[trace("TC-1265", "FR-337-AC-7")]
#[test]
fn a_repeated_binder_name_refuses_naming_both_declarations() {
    use qsl_semantics::check::{CheckCause, TemporalCause};

    let (refusal, text) = refusal_of(&each("c", ""), BODY);
    let CheckCause::Temporal(cause) = &refusal.cause else {
        panic!("a temporal refusal: {refusal:?}");
    };
    let TemporalCause::DuplicateTrigger { name, first, span } = cause.as_ref() else {
        panic!("a duplicate trigger: {cause:?}");
    };
    assert_eq!(name, "c");
    let over = text.find("(c:").unwrap() + 1;
    let binder = text.match_indices("c:").nth(1).unwrap().0;
    assert_eq!((first.start, first.end), (over, over + 1));
    assert_eq!((span.start, span.end), (binder, binder + 1));
}

/// FR-337-AC-7 "checked in source order": the activation binder is checked
/// before the captures, and the guard before the interval and the fairness
/// constraints, so the first defect in source order is the one refusal.
#[trace("TC-1265", "FR-337-AC-7")]
#[test]
fn the_activation_is_checked_before_the_captures_and_the_body() {
    let guard = " when (c.versionNumber)";
    let cases: [(String, &str, Code, &str, &str); 5] = [
        // The binder's type before an ill-typed capture.
        (
            "on each (q: Integer)".to_owned(),
            "capture n: Config::Absent = c.versionNumber; always holds(c.versionNumber <= 1)",
            Code::IllTyped,
            "type-mismatch",
            "Integer",
        ),
        // The binder's name before an ill-typed capture.
        (
            each("c", ""),
            "capture n: Config::Absent = c.versionNumber; always holds(c.versionNumber <= 1)",
            Code::ConflictingDeclaration,
            "duplicate-declaration",
            "c:",
        ),
        // The guard before an unresolved fairness operation.
        (
            each("q", guard),
            "fair weak each Config::ConfigVersion::nothing; always holds(c.versionNumber <= 1)",
            Code::IllTyped,
            "non-boolean-root",
            "c.versionNumber)",
        ),
        // The guard before an inverted interval.
        (
            each("q", guard),
            "always[3, 1] holds(c.versionNumber <= 1)",
            Code::IllTyped,
            "non-boolean-root",
            "c.versionNumber)",
        ),
        // The guard before an ill-typed `holds` condition.
        (
            each("q", guard),
            "always holds(c.versionNumber)",
            Code::IllTyped,
            "non-boolean-root",
            "c.versionNumber)",
        ),
    ];
    for (activation, body, code, tag, at) in cases {
        let (found, found_tag, start, text) = refused_on(&activation, body);
        assert_eq!((found, found_tag), (code, Some(tag)), "{activation} {body}");
        let nth = usize::from(at == "c:");
        let (offset, _) = text.match_indices(at).nth(nth).expect("the refused text");
        assert_eq!(start, u64::try_from(offset).unwrap(), "{activation} {body}");
    }
}

/// The captures of AC-8, as a clause body written with `holds` conditions
/// that read them.
const CAPTURES: &str = "capture n: Int[0, 1000] = c.versionNumber; \
    capture m: Int[0, 1000] = q.versionNumber; \
    always (holds(c.versionNumber >= n) and holds(m = n))";

/// The `Int[0, 1000]` captures `n` and `m` under `activation`, written
/// before `rest`.
fn captured(rest: &str) -> String {
    format!(
        "capture n: Int[0, 1000] = c.versionNumber; \
         capture m: Int[0, 1000] = q.versionNumber; {rest}"
    )
}

/// FR-337-AC-8 (TC-1267 steps 1 and 2): S3 types each capture like a `holds`
/// condition, keeps `n` then `m` with their slot counts, and a `holds`
/// condition counts the captures after `c` and `q`.
#[trace("TC-1267", "FR-337-AC-8")]
#[test]
fn s3_checks_and_keeps_each_capture() {
    for (activation, slots, holds) in [
        (each("q", " when (c.versionNumber >= 1)"), [2, 3], 4),
        (each("q", ""), [2, 3], 4),
        ("on origin".to_owned(), [1, 2], 3),
    ] {
        let text = unit(
            INFINITE,
            &declared_on(
                "Each",
                &activation,
                &captured("always (holds(c.versionNumber >= n) and holds(m = n))").replace(
                    "q.versionNumber",
                    if activation == "on origin" {
                        "c.versionNumber"
                    } else {
                        "q.versionNumber"
                    },
                ),
            ),
        );
        let compiled = compile(&text);
        let [node] = compiled.clauses.as_slice() else {
            panic!("one clause");
        };
        let names: Vec<_> = node.clause().captures().map(|(name, _)| name).collect();
        assert_eq!(names, ["n", "m"], "{activation}");
        for (index, expected) in slots.into_iter().enumerate() {
            let (_, found) = node.capture(index).expect("a typed initializer");
            assert_eq!(found, expected, "{activation} capture {index}");
        }
        assert!(node.capture(2).is_none());
        let counts: Vec<usize> = (0..node.clause().formula().nodes().len())
            .filter_map(|index| node.clause().formula().node_id(index))
            .filter_map(|atom| node.condition(atom))
            .map(|(_, count)| count)
            .collect();
        assert_eq!(counts, [holds, holds], "{activation}");
    }
}

/// FR-337-AC-8 (TC-1267 step 2): each defect refuses with its existing code
/// and tag at the stated text, and the first in the order activation,
/// captures, `holds` conditions is the one refusal.
#[trace("TC-1267", "FR-337-AC-8")]
#[test]
fn s3_refuses_each_capture_defect_at_its_locus() {
    let tail = "always holds(c.versionNumber <= 1)";
    let guard = " when (c.versionNumber)";
    // The activation, the body, code and tag, the text the refusal is
    // located at and the index of that text's occurrence.
    let cases: [(String, String, Code, &str, &str, usize); 11] = [
        (
            each("q", ""),
            format!("capture b: Boolean = c.versionNumber; {tail}"),
            Code::IllTyped,
            "type-mismatch",
            "c.versionNumber;",
            0,
        ),
        (
            each("q", ""),
            format!("capture x: Config::Absent = c.versionNumber; {tail}"),
            Code::MissingDeclaration,
            "missing-name",
            "Config::Absent",
            0,
        ),
        (
            each("q", ""),
            format!("capture n: Int[0, 1000] = c.versionNumber + 1; {tail}"),
            Code::UndefinedExpression,
            "unproved-range",
            "c.versionNumber + 1",
            0,
        ),
        (
            each("q", ""),
            format!(
                "capture a: Int[0, 1000] = b; capture b: Int[0, 1000] = c.versionNumber; {tail}"
            ),
            Code::MissingDeclaration,
            "missing-name",
            "b;",
            0,
        ),
        (
            each("q", ""),
            format!("capture a: Int[0, 1000] = a; {tail}"),
            Code::MissingDeclaration,
            "missing-name",
            "a;",
            0,
        ),
        (
            each("q", " when (n = 1)"),
            format!("capture n: Int[0, 1000] = c.versionNumber; {tail}"),
            Code::MissingDeclaration,
            "missing-name",
            "n = 1",
            0,
        ),
        (
            each("q", ""),
            format!("capture c: Int[0, 1000] = 1; {tail}"),
            Code::ConflictingDeclaration,
            "duplicate-declaration",
            "c: Int",
            0,
        ),
        (
            each("q", ""),
            format!("capture q: Int[0, 1000] = 1; {tail}"),
            Code::ConflictingDeclaration,
            "duplicate-declaration",
            "q: Int",
            0,
        ),
        (
            each("q", ""),
            format!("capture n: Int[0, 1000] = 1; capture n: Int[0, 1000] = 2; {tail}"),
            Code::ConflictingDeclaration,
            "duplicate-declaration",
            "n: Int",
            1,
        ),
        (
            each("q", guard),
            format!("capture b: Boolean = c.versionNumber; {tail}"),
            Code::IllTyped,
            "non-boolean-root",
            "c.versionNumber)",
            0,
        ),
        (
            each("q", ""),
            "capture b: Boolean = c.versionNumber; always holds(c.versionNumber)".to_owned(),
            Code::IllTyped,
            "type-mismatch",
            "c.versionNumber;",
            0,
        ),
    ];
    for (activation, body, code, tag, at, nth) in cases {
        let (found, found_tag, start, text) = refused_on(&activation, &body);
        assert_eq!((found, found_tag), (code, Some(tag)), "{activation} {body}");
        let (offset, _) = text.match_indices(at).nth(nth).expect("the refused text");
        assert_eq!(start, u64::try_from(offset).unwrap(), "{activation} {body}");
    }
}

/// FR-337-AC-8: a capture that repeats an earlier declaration names both.
#[trace("TC-1267", "FR-337-AC-8")]
#[test]
fn a_repeated_capture_name_refuses_naming_both_declarations() {
    use qsl_semantics::check::{CheckCause, TemporalCause};

    let (refusal, text) = refusal_of(
        &each("q", ""),
        "capture n: Int[0, 1000] = 1; capture n: Int[0, 1000] = 2; always holds(true)",
    );
    let CheckCause::Temporal(cause) = &refusal.cause else {
        panic!("a temporal refusal: {refusal:?}");
    };
    let TemporalCause::DuplicateCapture { name, first, span } = cause.as_ref() else {
        panic!("a duplicate capture: {cause:?}");
    };
    assert_eq!(name, "n");
    let mut names = text.match_indices("n: Int").map(|(at, _)| at);
    let (one, two) = (names.next().unwrap(), names.next().unwrap());
    assert_eq!((first.start, span.start), (one, two));
}

/// The node of `wire` that is a `value`/`parameter` named `name`.
fn parameter_named<'w>(wire: &'w Value, name: &str) -> &'w Value {
    nodes(wire)
        .iter()
        .find(|node| {
            node["node_tag"] == "value"
                && node["semantic_form"] == "parameter"
                && node["body"]["members"][0]["value"]["value"] == name
        })
        .unwrap_or_else(|| panic!("a parameter named {name}"))
}

/// The captures aggregate of the clause node of `wire`.
fn captures_of(wire: &Value) -> &Value {
    &clause_node(wire)["body"]["arguments"][3]
}

/// FR-337-AC-9 (TC-1267 step 3): the captures aggregate binds each capture to
/// its initializer's term, each capture has its own parameter node at level
/// `1 + e + j` typed by its declared type, a `holds` read references the
/// node, the nodes are located at the capture names, and the I2 reader
/// returns the clause's `over` node.
#[trace("TC-1267", "FR-337-AC-9")]
#[test]
fn the_captures_aggregate_and_nodes_are_emitted() {
    let activation = each("q", " when (c.versionNumber >= 1)");
    let (compiled, _) = key_of(&activation, CAPTURES);
    let wire = &compiled.wire;
    let aggregate = captures_of(wire);
    let members = aggregate["members"].as_array().expect("an aggregate");
    let names: Vec<_> = members
        .iter()
        .map(|member| member["name"].clone())
        .collect();
    assert_eq!(names, ["n", "m"]);
    let over = parameter_named(wire, "c");
    let trigger = parameter_named(wire, "q");
    let n = parameter_named(wire, "n");
    let m = parameter_named(wire, "m");
    assert_eq!(n["body"]["members"][1]["value"]["value"], "2");
    assert_eq!(m["body"]["members"][1]["value"]["value"], "3");
    assert_eq!(n["semantic_type"], m["semantic_type"]);
    assert_ne!(n["semantic_type"], over["semantic_type"]);
    fn key(node: &Value) -> &Value {
        &node["node_id"]
    }
    assert!(reaches(wire, &members[0]["value"], key(over)));
    assert!(!reaches(wire, &members[0]["value"], key(trigger)));
    assert!(reaches(wire, &members[1]["value"], key(trigger)));
    let formula = formula_root(wire, clause_node(wire));
    assert!(reaches(wire, &formula["body"], key(n)), "a holds reads n");
    assert!(reaches(wire, &formula["body"], key(m)));

    // The capture node and its type node are located at the capture's name.
    let text = unit(INFINITE, &declared_on("Each", &activation, CAPTURES));
    let packages = qsl_semantics::model::intake::package_input([document()]).unwrap();
    let declarations = assemble_with_models(text.as_bytes(), &packages).expect("assembles");
    let regions = declarations.regions();
    let graph = declarations
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    let located = |wire_key: &Value, role: &str| -> Vec<u64> {
        let sought = node_key(wire, wire_key);
        graph
            .occurrences()
            .filter(|(key, origin, _)| key == &sought && origin.role().as_str() == role)
            .map(|(_, _, location)| regions.region(location).expect("a region").start())
            .collect()
    };
    let start = |needle: &str| u64::try_from(text.find(needle).unwrap()).unwrap();
    assert!(located(key(n), "expression").contains(&start("n: Int")));
    assert!(located(key(m), "expression").contains(&start("m: Int")));
    let at_types = located(&n["semantic_type"], "type");
    assert!(at_types.contains(&start("n: Int")) && at_types.contains(&start("m: Int")));

    // Under `on origin` the nodes are at levels 1 and 2, and a clause with no
    // capture binds the empty aggregate.
    let origin_body = CAPTURES.replace("q.versionNumber", "c.versionNumber");
    let (origin, _) = key_of("on origin", &origin_body);
    assert_eq!(
        parameter_named(&origin.wire, "n")["body"]["members"][1]["value"]["value"],
        "1"
    );
    assert_eq!(
        parameter_named(&origin.wire, "m")["body"]["members"][1]["value"]["value"],
        "2"
    );
    let (none, _) = key_of(&activation, BODY);
    assert_eq!(
        captures_of(&none.wire)["members"].as_array().map(Vec::len),
        Some(0)
    );

    // The I2 reader admits each package and returns the clause's triple and
    // `over` node, never a capture node.
    for compiled in [&compiled, &origin, &none] {
        let [checked] = compiled.clauses.as_slice() else {
            panic!("one emitted clause");
        };
        let admitted = admit(compiled);
        let [root] = clause_roots(&admitted)
            .expect("the roots read")
            .try_into()
            .unwrap_or_else(|roots: Vec<ClauseRoot>| panic!("one root: {roots:?}"));
        assert_eq!(root.occurrence().node(), &checked.identity());
        assert_eq!(root.over(), Some(&checked.over()));
    }
}

/// FR-337-AC-10 (TC-1267 step 4): the node a clause's `temporal-satisfaction`
/// record (the requirements set `requires temporal Due;` reads) holds is the
/// emitted clause node: its key is the clause key, it is the same each time
/// the clause is emitted, and it differs with and without the captures.
#[trace("TC-1267", "FR-337-AC-10")]
#[test]
fn the_requirement_node_is_the_clause_node_and_changes_with_the_captures() {
    use qsl_foundation::bound::DomainKind;
    use qsl_semantics::family::ClaimExtent;

    let activation = each("q", " when (c.versionNumber >= 1)");
    let requirement = |activation: &str, body: &str| {
        let text = unit(INFINITE, &declared_on("Each", activation, body));
        let (graph, clause, requirements) = record_of(&text);
        let ClaimExtent::Unbounded(domains) = requirements.extent() else {
            panic!("infinite-trace: {:?}", requirements.extent());
        };
        assert_eq!(domains.kind(&clause), Some(DomainKind::InfiniteTrace));
        // The record is keyed by the emitted clause's identity.
        let (compiled, key) = key_of(activation, body);
        let [checked] = graph.temporal_clauses() else {
            panic!("one emitted clause");
        };
        assert_eq!(checked.identity(), compiled.clauses[0].identity());
        assert_eq!(checked.identity(), node_key(&compiled.wire, &key));
        clause
    };
    let captured = requirement(&activation, CAPTURES);
    assert_eq!(captured, requirement(&activation, CAPTURES));
    requirement(&activation, BODY);
    for body in [CAPTURES, BODY] {
        requirement(
            "on origin",
            &body.replace("q.versionNumber", "c.versionNumber"),
        );
    }
}
