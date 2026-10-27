// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-231 (TC-686): the timed subject of ADR-026 §11's `Rpc` model, its
//! delays and discrete steps over exact clock valuations; FR-232 (TC-687):
//! terminal states, time-locks and weak fairness read over time.
//!
//! `Call` holds `phase` (`0` Idle, `1` Waiting, `2` Replied, `3` TimedOut),
//! `inflight`, `late` and the clock `x`, a package field typed
//! `ix://quire/native/Clock`. The guards, resets and time invariants are
//! the unit's own clauses, checked at S3 (FR-230).

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_eval::simulation::model::{
    check_model_bounds, BehaviourReading, ClockValuation, ExpansionLimits, ModelSimulationRequest,
    ModelSystem, MoveRefusal, PopulationUniverse, TimedBehaviour, TimedState, TimedStop,
    TimedSystem,
};
use qsl_eval::simulation::{NotSimulated, BOUND_POSITIONS};
use qsl_package::CheckedPackage;
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, meaning, package_input};
use qsl_semantics::model::key::hex;
use qsl_semantics::model::observation::{DocumentRef, ObservationLimits};
use quire_checked::key::DeclarationKey;
use quire_exact::{Integer, Rational, ScalarLimits};
use quire_semantic_value::catalog::CatalogCode;
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::model_transition::ModelTransition;
use serde_json::{json, Value as Json};

const PACKAGE: &str = "test/rpc";
const PLACEHOLDER: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

fn ident(suffix: &str) -> String {
    format!("ix://{PACKAGE}/{suffix}")
}

fn origin(identity: &str) -> Json {
    json!({"generated": {"generatorIdentity": identity, "generatorVersion": "1.0.0",
        "inputIdentities": [identity]}})
}

fn wire_type(identity: &str, kind: &str, extra: Json) -> Json {
    let mut node = json!({
        "identity": identity, "displayName": identity,
        "kind": {"module": PACKAGE, "name": kind}, "roles": [],
        "origin": origin(identity), "constraints": [], "extensions": [],
        "unknownPolicy": "reject",
    });
    for (name, value) in extra.as_object().expect("extra members") {
        node[name] = value.clone();
    }
    node
}

fn wire_field(name: &str, type_ref: &str) -> Json {
    let identity = ident(&format!("Call/{name}"));
    let unique = type_ref != "ix://quire/native/Clock";
    json!({
        "identity": identity, "name": name, "typeRef": type_ref, "presence": "required",
        "nullable": false, "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": unique},
        "origin": origin(&identity),
    })
}

fn phase_type() -> Json {
    let identity = ident("Phase");
    let constraint = |keyword: &str, value: &str| {
        json!({
            "identity": format!("{identity}/constraints/{keyword}"), "keyword": keyword,
            "operands": {"value": value}, "appliesTo": "ix://quire/native/Integer",
            "diagnosticCode": format!("bound.{keyword}"), "origin": origin(&identity),
        })
    };
    wire_type(
        &identity,
        "value_type",
        json!({"scalar": "integer",
            "constraints": [constraint("min", "0"), constraint("max", "3")]}),
    )
}

fn operation(name: &str, modifies: &[&str]) -> Json {
    let identity = ident(&format!("Call/{name}"));
    json!({
        "identity": identity, "name": name, "params": [], "pre": [], "post": [],
        "origin": {"source": {"sourceIdentity": ident("spec"), "path": "spec.qspec",
            "startLine": 1, "startColumn": 1}},
        "frame": {
            "modifies": modifies.iter().map(|field| ident(&format!("Call/{field}"))).collect::<Vec<_>>(),
            "creates": if name == "spawn" { vec![ident("Call")] } else { vec![] },
            "deletes": [],
        },
    })
}

/// `Call`, with the clock `x` when `clocked`.
fn rpc_document(clocked: bool) -> Vec<u8> {
    let extra: &[&str] = &[];
    clocked_document(clocked, extra)
}

/// [`rpc_document`] with the further clock fields `extra`.
fn clocked_document(clocked: bool, extra: &[&str]) -> Vec<u8> {
    let call = ident("Call");
    let constructs: Vec<Json> = [
        ("object_type", meaning::OBJECT_TYPE),
        ("population", meaning::POPULATION),
        ("value_type", meaning::VALUE_TYPE),
    ]
    .into_iter()
    .map(|(name, meaning)| {
        json!({"kind": {"module": PACKAGE, "name": name}, "moduleVersion": "1.0.0",
            "manifestDigest": PLACEHOLDER,
            "construct": {"identity": "none", "shape": "record", "members": {},
                "meaning": meaning}})
    })
    .collect();
    let population = ident("calls");
    let mut fields = vec![
        wire_field("phase", &ident("Phase")),
        wire_field("inflight", "ix://quire/native/Boolean"),
        wire_field("late", "ix://quire/native/Boolean"),
    ];
    if clocked {
        fields.push(wire_field("x", "ix://quire/native/Clock"));
    }
    fields.extend(
        extra
            .iter()
            .map(|name| wire_field(name, "ix://quire/native/Clock")),
    );
    json!({
        "contractVersion": "2.0.0",
        "canonicalization": {"algorithm": "rfc8785-v1", "digest": "sha256-jcs"},
        "source": {"identity": ident("spec"), "version": "1.0.0", "dialect": "spec-bundle",
            "digest": PLACEHOLDER},
        "package": {"identity": PACKAGE, "version": "1.0.0", "manifestDigest": PLACEHOLDER,
            "mappingVersions": [], "profileVersions": [], "lockDigest": PLACEHOLDER},
        "occurrences": [], "extensions": [],
        "constructs": constructs,
        "types": [
            phase_type(),
            wire_type(&call, "object_type", json!({
                "supertypes": [],
                "fields": fields,
                "operations": [
                    operation("send", if clocked { &["phase", "inflight", "late", "x"] } else { &["phase", "inflight", "late"] }),
                    operation("reply", &["phase", "inflight", "late"]),
                    operation("timeout", &["phase"]),
                    operation("reset", &["phase"]),
                    operation("tick", &[]),
                    operation("work", &[]),
                    operation("spawn", &[]),
                ],
                "relationships": [],
            })),
        ],
        "populations": [{
            "identity": population, "displayName": population,
            "kind": {"module": PACKAGE, "name": "population"},
            "members": [call], "extent": "closed", "origin": origin(&population),
        }],
    })
    .to_string()
    .into_bytes()
}

/// ADR-026 §11's members with the client deadline `t` (in ms) and `extra`
/// declarations.
fn rpc_unit(t: u32, extra: &str) -> String {
    rpc_unit_guarded(t, ">=", extra)
}

/// [`rpc_unit`] with the guards' comparison `guard`: with `>=`, `reply`
/// needs `x >= 1 ms` and `timeout` `x >= T`; with `>`, ADR-026 §11's
/// strict-guard variant, `reply` needs `x > 3 ms` and `timeout` `x > T`.
fn rpc_unit_guarded(t: u32, guard: &str, extra: &str) -> String {
    let reply = if guard == ">" { 3 } else { 1 };
    format!(
        "dimension Time;\nunit s : Time = rational(1, 1);\n\
         unit ms : Time = rational(1, 1000) * s;\n\
         time M dense unit ms;\n\
         pre SendIdle using v on M::Call::send {{ self.phase = 0 }}\n\
         post Send using v on M::Call::send {{ \
             self.phase = 1 and self.inflight and not self.late and self.x = 0 ms }}\n\
         pre Reply using v on M::Call::reply {{ self.inflight and self.x {guard} {reply} ms }}\n\
         post Replied using v on M::Call::reply {{ not self.inflight and \
             self.late = (pre(self.phase) = 3) and \
             (pre(self.phase) = 1 implies self.phase = 2) and \
             (pre(self.phase) != 1 implies self.phase = pre(self.phase)) }}\n\
         pre Timeout using v on M::Call::timeout {{ self.phase = 1 and self.x {guard} {t} ms }}\n\
         post TimedOut using v on M::Call::timeout {{ self.phase = 3 }}\n\
         pre Reset using v on M::Call::reset {{ \
             not self.inflight and (self.phase = 2 or self.phase = 3) }}\n\
         post ResetIdle using v on M::Call::reset {{ self.phase = 0 }}\n\
         pre NoTick using v on M::Call::tick {{ false }}\n\
         pre NoWork using v on M::Call::work {{ false }}\n\
         pre NoSpawn using v on M::Call::spawn {{ false }}\n\
         time invariant ServerDeadline when {{ self.inflight }} {{ self.x <= 3 ms }}\n\
         time invariant ClientDeadline when {{ self.phase = 1 }} {{ self.x <= {t} ms }}\n\
         {extra}\n"
    )
}

struct World {
    package: CheckedPackage,
    packages: BTreeMap<[u8; 32], Vec<u8>>,
    snapshots: BTreeMap<[u8; 32], Vec<u8>>,
    initial: DocumentRef,
}

fn world(unit_body: &str) -> World {
    world_with("", unit_body)
}

/// [`world`] with the header `profiles` after the model and temporal ones.
fn world_with(profiles: &str, unit_body: &str) -> World {
    world_over(rpc_document(true), profiles, unit_body)
}

/// [`world_with`] over `document`.
fn world_over(document: Vec<u8>, profiles: &str, unit_body: &str) -> World {
    world_holding(document, profiles, unit_body, json!({}))
}

/// [`world_over`] whose initial `Call` also holds the field values
/// `fields`.
fn world_holding(document: Vec<u8>, profiles: &str, unit_body: &str, fields: Json) -> World {
    world_of(
        document,
        profiles,
        unit_body,
        json!([{"key": "c", "type": ident("Call"), "fields": initial_fields(fields)}]),
    )
}

/// [`world_over`] whose initial state holds the `Call` objects `objects`.
fn world_of(document: Vec<u8>, profiles: &str, unit_body: &str, objects: Json) -> World {
    let packages = package_input([document.as_slice()]);
    let digest = *packages.keys().next().expect("one model");
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         profile t = \"quire.temporal.complete/v1\";\n{profiles}\
         model M = \"{PACKAGE}\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n{unit_body}",
        hex(&digest)
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "fr-231", "fixture", "1"),
        "fr-231.native",
        unit.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 parses the unit");
    assert!(parsed.is_admissible(), "S1: {:?}", parsed.diagnostics());
    let built = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let models = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("the model selection admits")
    .with_parsed_operation_clauses(|text| {
        qsl_forms::parse_clause_text(text, qsl_cst::Limits::default())
    })
    .expect("the kept operation clauses parse");
    let graph = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        built,
        models,
        Vec::new(),
    )
    .expect("the unit assembles")
    .with_unwritten_timed_members()
    .check(CheckingLimits::default())
    .unwrap_or_else(|refusals| panic!("the unit checks: {refusals:?}"));
    let header = json!({"identity": PACKAGE, "version": "1.0.0",
        "digest": format!("sha256-jcs:{}", hex(&digest))});
    let document = json!({
        "format": "quire.state.snapshot/v1",
        "identity": {"authority": "test", "identity": "s0", "revision_namespace": "fixture",
            "revision": "fixture:1"},
        "observation": "current", "model": header,
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [{"population": ident("calls"), "complete": true, "objects": objects}],
    });
    let bytes = document.to_string().into_bytes();
    let read = quire_canonical::read(&bytes, u64::MAX).expect("JSON fixture");
    let snapshot_digest = *quire_canonical::sha256(&read, quire_canonical::Limits::new(u64::MAX))
        .expect("canonical fixture")
        .as_bytes();
    World {
        package: CheckedPackage::link(graph),
        packages,
        snapshots: BTreeMap::from([(snapshot_digest, bytes)]),
        initial: DocumentRef {
            authority: "test".to_owned(),
            identity: "s0".to_owned(),
            revision_namespace: "fixture".to_owned(),
            revision: "fixture:1".to_owned(),
            digest: snapshot_digest,
        },
    }
}

/// The initial `Call`'s field values: `phase` 0, `inflight` and `late`
/// false, and `extra`.
fn initial_fields(extra: Json) -> Json {
    let mut fields = json!({"phase": {"integer": "0"}, "inflight": {"boolean": false},
        "late": {"boolean": false}});
    for (name, value) in extra.as_object().expect("field values") {
        fields[name] = value.clone();
    }
    fields
}

const METER: ScalarLimits = ScalarLimits {
    integer_bits: 1 << 20,
    decimal_digits: 1 << 20,
    scale_expansion: 1 << 20,
    text_input_bytes: 1 << 20,
    text_scalars: 1 << 20,
    normalized_scalars: 1 << 20,
    unit_edges: 1 << 20,
    value_occurrences: 1 << 20,
    work_units: 1 << 30,
    result_units: 1 << 30,
};

fn system(world: &World) -> ModelSystem<'_> {
    system_over(world, &["c"])
}

/// [`system`] over the universe `keys`.
fn system_over<'w>(world: &'w World, keys: &[&str]) -> ModelSystem<'w> {
    system_limited(world, keys, 1_000_000)
}

/// [`system_over`] under the expansion bound `max_candidates`.
fn system_limited<'w>(world: &'w World, keys: &[&str], max_candidates: u64) -> ModelSystem<'w> {
    ModelSystem::new(ModelSimulationRequest {
        package: &world.package,
        packages: &world.packages,
        normalization: ModelNormalizationLimits::default(),
        snapshots: &world.snapshots,
        initial: vec![world.initial.clone()],
        universes: vec![PopulationUniverse {
            population: DeclarationKey {
                package: PACKAGE.to_owned(),
                node: ident("calls"),
            },
            keys: keys.iter().map(|key| (*key).to_owned()).collect(),
        }],
        observation: ObservationLimits::default(),
        expansion: ExpansionLimits { max_candidates },
        meter: METER,
    })
    .expect("the system admits")
}

fn exact(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("a rational")
}

/// The transition identity of `operation` on `c`, as the timed subject
/// offers it from the behaviour's last state.
fn transition(
    timed: &TimedSystem<'_, '_>,
    behaviour: &TimedBehaviour,
    name: &str,
) -> ModelTransition {
    let suffix = format!("Call/{name}");
    timed
        .guarded_steps(behaviour.last().state().discrete())
        .expect("the state expands")
        .into_iter()
        .find(|step| step.operation().contains(&suffix))
        .unwrap_or_else(|| panic!("`{name}` is enabled by its data precondition"))
        .transition()
        .clone()
}

/// FR-231-AC-1: from `(Idle, x = 0)` a delay of 7 is admissible; after
/// `send`, a delay of 3 is and `3 + 1/1000` is not; `reply` is enabled after
/// a delay of 1 and not after `999/1000`. `send` at 0 then `reply` after
/// `5/2` has time stamps `0, 0, 5/2` and `x = 5/2` at position 2.
#[trace("TC-686", "FR-231-AC-1")]
#[test]
fn rpc_delays_and_steps_read_exact_clock_values() {
    let world = world(&rpc_unit(4, ""));
    let system = system(&world);
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    assert_eq!(timed.places().len(), 1);
    let [initial] = &timed.initial()[..] else {
        panic!("one initial state");
    };
    assert!(timed
        .delay(initial, &exact(7, 1))
        .expect("decided")
        .is_some());

    let mut behaviour = TimedBehaviour::new(initial.clone());
    let send = transition(&timed, &behaviour, "send");
    behaviour
        .advance(&timed, &exact(0, 1), &send)
        .expect("send at 0");
    let waiting = behaviour.last().state().clone();
    assert!(timed
        .delay(&waiting, &exact(3, 1))
        .expect("decided")
        .is_some());
    assert!(timed
        .delay(&waiting, &exact(3001, 1000))
        .expect("decided")
        .is_none());
    let reply = transition(&timed, &behaviour, "reply");
    let enabled_after = |delay: Rational| {
        let delayed = timed
            .delay(&waiting, &delay)
            .expect("decided")
            .expect("admissible");
        timed
            .steps(&delayed)
            .expect("decided")
            .iter()
            .any(|(step, _)| step.transition() == &reply)
    };
    assert!(enabled_after(exact(1, 1)));
    assert!(!enabled_after(exact(999, 1000)));

    behaviour
        .advance(&timed, &exact(5, 2), &reply)
        .expect("reply after 5/2");
    let stamps: Vec<Rational> = behaviour
        .positions()
        .iter()
        .map(|position| position.stamp().clone())
        .collect();
    assert_eq!(stamps, [exact(0, 1), exact(0, 1), exact(5, 2)]);
    assert_eq!(behaviour.last().state().clocks().values(), [exact(5, 2)]);
    // A delay past the server deadline before `reply` is refused.
    let mut late = TimedBehaviour::new(initial.clone());
    late.advance(&timed, &exact(0, 1), &send).expect("send");
    assert_eq!(
        late.advance(&timed, &exact(7, 2), &reply),
        Err(MoveRefusal::Delay)
    );
}

/// FR-236-AC-1's digest half (TC-691): over `Rpc` with `T = 3 ms`, the steps of
/// `NoLateReply`'s counterexample, `send` after 0, `timeout` after 3 and
/// `reply` after 0, reach post-states whose FR-236 digest changes when only
/// the resulting clock value changes, and is equal for an equal timed state.
/// The counterexample's steps and delays are the zone search's
/// (`qsl-analyze` `zone_search`), which writes this digest as each step's
/// `post_state`; its steps, delays and `trace_position` are asserted there.
#[trace("TC-691", "FR-236-AC-1")]
#[test]
fn a_timed_post_state_digest_changes_with_its_clock_value_alone() {
    let world = world(&rpc_unit(3, ""));
    let system = system(&world);
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    let [initial] = &timed.initial()[..] else {
        panic!("one initial state");
    };
    let mut behaviour = TimedBehaviour::new(initial.clone());
    for (operation, delay) in [("send", 0), ("timeout", 3), ("reply", 0)] {
        let step = transition(&timed, &behaviour, operation);
        behaviour
            .advance(&timed, &exact(delay, 1), &step)
            .unwrap_or_else(|refusal| panic!("{operation} after {delay}: {refusal:?}"));
    }
    let posts: Vec<_> = behaviour.positions()[1..]
        .iter()
        .map(|position| position.state().clone())
        .collect();
    assert_eq!(posts.len(), 3);
    for (index, post) in posts.iter().enumerate() {
        let digest = timed.state_digest(post).expect("a digest");
        assert_eq!(
            timed.state_digest(&post.clone()).expect("a digest"),
            digest,
            "step {index}: an equal timed state has an equal digest"
        );
        let shifted = TimedState::new(post.discrete().clone(), post.clocks().delayed(&exact(1, 2)));
        assert_ne!(
            timed.state_digest(&shifted).expect("a digest"),
            digest,
            "step {index}: only the clock value changed"
        );
    }
    assert_eq!(posts[0].clocks().values(), [exact(0, 1)]);
    assert_eq!(posts[1].clocks().values(), [exact(3, 1)]);
}

/// Part of FR-231-AC-2 (TC-686): with `urgent when { self.phase = 1 }`, no
/// positive delay is admissible after `send`; the `Replied` state is
/// quiescent and the waiting one is not. The idle-tail reading is the next
/// test's.
#[trace("TC-686", "FR-231-AC-2")]
#[test]
fn an_urgent_state_admits_no_positive_delay() {
    let urgent = world(&rpc_unit(4, "urgent Eager when { self.phase = 1 };"));
    let urgent_system = system(&urgent);
    let timed = TimedSystem::new(&urgent_system, "M").expect("a timed subject");
    let initial = timed.initial().remove(0);
    let mut behaviour = TimedBehaviour::new(initial);
    let send = transition(&timed, &behaviour, "send");
    behaviour
        .advance(&timed, &exact(0, 1), &send)
        .expect("send at 0");
    let waiting = behaviour.last().state().clone();
    assert!(timed.is_urgent(waiting.discrete()).expect("decided"));
    assert!(timed
        .delay(&waiting, &exact(1, 1000))
        .expect("decided")
        .is_none());
    assert!(timed
        .delay(&waiting, &exact(0, 1))
        .expect("decided")
        .is_some());

    // Without urgency `reply` comes after a delay of 1 and reaches the
    // quiescent `Replied` state.
    let plain = world(&rpc_unit(4, ""));
    let plain_system = system(&plain);
    let timed = TimedSystem::new(&plain_system, "M").expect("a timed subject");
    let mut behaviour = TimedBehaviour::new(timed.initial().remove(0));
    let send = transition(&timed, &behaviour, "send");
    behaviour
        .advance(&timed, &exact(0, 1), &send)
        .expect("send");
    assert!(!timed
        .is_quiescent(behaviour.last().state().discrete())
        .expect("decided"));
    let reply = transition(&timed, &behaviour, "reply");
    behaviour
        .advance(&timed, &exact(1, 1), &reply)
        .expect("reply");
    let replied = behaviour.last().state();
    assert!(timed.is_quiescent(replied.discrete()).expect("decided"));
    assert!(timed
        .delay(replied, &exact(100, 1))
        .expect("decided")
        .is_some());
}

/// Part of FR-231-AC-2 (TC-686): a behaviour of `Rpc` that ends at the
/// quiescent `Replied` state reads under the timed profile as its stutter
/// step repeated after positive delays, so `always holds(c.phase != 1)` is
/// true from position 2 and false from position 1; the closed reading ends
/// at position 2. The event-position reading is
/// `under_event_position_the_execution_closes_at_its_last_position`.
#[trace("TC-686", "FR-231-AC-2")]
#[test]
fn a_quiescent_end_reads_as_an_idle_tail() {
    use qsl_eval::model_subject::ModelInstance;
    use qsl_eval::simulation::model::BehaviourLetters;
    use qsl_eval::temporal::{evaluate_timed_at, TemporalOutcome, TemporalPosition};

    let world = world_with(
        "profile tt = \"quire.temporal.timed/v1\";\n",
        &rpc_unit(
            4,
            "temporal NoWait using tt over (c: M::Call) clock \"model-time\" on origin \
             { always holds(c.phase != 1) }",
        ),
    );
    let system = system(&world);
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    let mut behaviour = TimedBehaviour::new(timed.initial().remove(0));
    let send = transition(&timed, &behaviour, "send");
    behaviour
        .advance(&timed, &exact(0, 1), &send)
        .expect("send");
    let reply = transition(&timed, &behaviour, "reply");
    behaviour
        .advance(&timed, &exact(1, 1), &reply)
        .expect("reply");

    // The clause's own profile chooses the idle tail.
    let node = &world.package.graph().temporal_clauses()[0];
    let BehaviourReading::Timed(trace) = behaviour
        .clause_reading(&timed, node.clause().profile())
        .expect("a timed lasso")
    else {
        panic!("the timed profile reads time stamps");
    };
    assert!(trace.is_lasso());
    let closed = behaviour
        .timed_trace(&timed, false)
        .expect("a timed prefix");
    assert!(!closed.is_lasso());
    assert_eq!(closed.stamps().len(), 3);

    let letters = system.clause_letters(node).expect("the clause has letters");
    let instance = ModelInstance {
        population: DeclarationKey {
            package: PACKAGE.to_owned(),
            node: ident("calls"),
        },
        key: "c".to_owned(),
    };
    let at = |position: u64| {
        let mut letters = BehaviourLetters::new(&letters, &instance, &behaviour);
        evaluate_timed_at(
            node.clause(),
            &trace,
            TemporalPosition::new(position),
            &mut letters,
            &mut quire_exact::Meter::new(METER),
        )
        .expect("evaluated")
    };
    assert_eq!(at(2), TemporalOutcome::Holds);
    assert!(matches!(at(1), TemporalOutcome::Violated { .. }));
}

/// Part of FR-231-AC-2 (TC-686): in a unit selecting
/// `quire.temporal.event-position.false-extension/v1`, the behaviour `send`
/// then `reply` of `Rpc` closes at position 2: the clause's profile reads it
/// as the closed trace of its three positions, with no loop (under
/// infinite-trace the same behaviour is the lasso looping on position 2),
/// and over it `always[0,1] holds(c.phase = 2)` is violated at position 2,
/// not pending: false extension reads no position 3, where the idle tail
/// would repeat `Replied`, while `always[0,0] holds(c.phase = 2)` holds
/// there.
#[trace("TC-686", "FR-231-AC-2")]
#[test]
fn under_event_position_the_execution_closes_at_its_last_position() {
    use qsl_eval::model_subject::ModelInstance;
    use qsl_eval::simulation::model::BehaviourLetters;
    use qsl_eval::temporal::{evaluate_at, TemporalOutcome, TemporalPosition, TemporalTrace};

    let world = world_with(
        "profile ep = \"quire.temporal.event-position.false-extension/v1\";\n",
        &rpc_unit(
            4,
            "temporal Stays using ep over (c: M::Call) clock \"model-steps\" on origin \
             { always[0,1] holds(c.phase = 2) }\n\
             temporal Here using ep over (c: M::Call) clock \"model-steps\" on origin \
             { always[0,0] holds(c.phase = 2) }",
        ),
    );
    let system = system(&world);
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    let mut behaviour = TimedBehaviour::new(timed.initial().remove(0));
    for (operation, delay) in [("send", 0), ("reply", 1)] {
        let step = transition(&timed, &behaviour, operation);
        behaviour
            .advance(&timed, &exact(delay, 1), &step)
            .expect("the step is admitted");
    }
    let closed = behaviour
        .timed_trace(&timed, false)
        .expect("a closed timed trace");
    assert!(!closed.is_lasso());
    assert_eq!(closed.stamps(), [exact(0, 1), exact(0, 1), exact(1, 1)]);

    let instance = ModelInstance {
        population: DeclarationKey {
            package: PACKAGE.to_owned(),
            node: ident("calls"),
        },
        key: "c".to_owned(),
    };
    let reading = |clause: usize| {
        let node = &world.package.graph().temporal_clauses()[clause];
        behaviour
            .clause_reading(&timed, node.clause().profile())
            .expect("a closed reading")
    };
    assert_eq!(
        reading(0),
        BehaviourReading::Untimed(TemporalTrace::Finite { len: 3 })
    );
    assert_eq!(
        behaviour
            .clause_reading(&timed, qsl_semantics::check::TemporalProfile::InfiniteTrace)
            .expect("an idle tail"),
        BehaviourReading::Untimed(TemporalTrace::Lasso {
            prefix: 2,
            loop_len: 1
        })
    );
    let at = |clause: usize, position: u64| {
        let node = &world.package.graph().temporal_clauses()[clause];
        let letters = system.clause_letters(node).expect("the clause has letters");
        let mut letters = BehaviourLetters::new(&letters, &instance, &behaviour);
        let BehaviourReading::Untimed(trace) = reading(clause) else {
            panic!("an untimed reading");
        };
        evaluate_at(
            node.clause(),
            trace,
            TemporalPosition::new(position),
            &mut letters,
            &mut quire_exact::Meter::new(METER),
        )
        .expect("evaluated")
    };
    assert!(
        matches!(at(0, 2), TemporalOutcome::Violated { .. }),
        "{:?}",
        at(0, 2)
    );
    // Position 2 itself reads `Replied`.
    assert_eq!(at(1, 2), TemporalOutcome::Holds);
}

/// FR-231-AC-3: under `time M tick M::Call::tick period 5 ms unit ms`,
/// `work, tick, work, tick` has the time stamps `0, 0, 5, 5, 10`, every
/// positive delay is refused, and a behaviour that ends idle diverges
/// through its stutter step counted as a tick.
#[trace("TC-686", "FR-231-AC-3")]
#[test]
fn a_tick_source_stamps_positions_by_tick_count() {
    let world = world_over(
        rpc_document(false),
        "",
        "dimension Time;\nunit s : Time = rational(1, 1);\n\
         unit ms : Time = rational(1, 1000) * s;\n\
         time M tick M::Call::tick period 5 ms unit ms;\n",
    );
    let system = system(&world);
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    let mut behaviour = TimedBehaviour::new(timed.initial().remove(0));
    assert!(timed
        .delay(behaviour.last().state(), &exact(1, 1))
        .expect("decided")
        .is_none());
    for name in ["work", "tick", "work", "tick"] {
        let step = transition(&timed, &behaviour, name);
        behaviour
            .advance(&timed, &exact(0, 1), &step)
            .unwrap_or_else(|refusal| panic!("{name}: {refusal:?}"));
    }
    let stamps: Vec<Rational> = behaviour
        .positions()
        .iter()
        .map(|position| position.stamp().clone())
        .collect();
    assert_eq!(
        stamps,
        [
            exact(0, 1),
            exact(0, 1),
            exact(5, 1),
            exact(5, 1),
            exact(10, 1)
        ]
    );
    let trace = behaviour.timed_trace(&timed, true).expect("a timed lasso");
    assert!(trace.is_lasso());
    assert_eq!(timed.tick_period(), Some(&exact(5, 1)));
}

/// FR-230-AC-4's subject half (TC-685 step 4): a model with 64 package
/// clock fields over a universe of 100 objects has 6,400 clock places, each
/// 0 initially, and its guard `x <= 10^40 ms` checks with no refusal and
/// reads the exact constant. The S2 forms are `qsl-forms`' and the convex
/// parts `qsl-semantics`' `timed_model` tests.
#[trace("TC-685", "FR-230-AC-4")]
#[test]
fn every_object_holds_its_own_clock_of_every_clock_field() {
    let names: Vec<String> = (0..63).map(|index| format!("c{index:02}")).collect();
    let extra: Vec<&str> = names.iter().map(String::as_str).collect();
    let huge = format!("1{}", "0".repeat(40));
    let world = world_over(
        clocked_document(true, &extra),
        "",
        &format!(
            "dimension Time;\nunit s : Time = rational(1, 1);\n\
             unit ms : Time = rational(1, 1000) * s;\ntime M dense unit ms;\n\
             pre Big using v on M::Call::work {{ self.x <= {huge} ms }}\n"
        ),
    );
    let keys: Vec<String> = std::iter::once("c".to_owned())
        .chain((1..100).map(|index| format!("k{index:03}")))
        .collect();
    let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
    let system = system_over(&world, &keys);
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    assert_eq!(timed.model().clocks().len(), 64);
    assert_eq!(timed.places().len(), 6_400);
    let [guard] = timed.model().guards() else {
        panic!("one guard: {:?}", timed.model().guards());
    };
    assert_eq!(
        guard.parts()[0].clocks()[0].bound().numerator().to_string(),
        huge
    );
    let initial = timed.initial().remove(0);
    assert!(initial
        .clocks()
        .values()
        .iter()
        .all(|value| value == &exact(0, 1)));
}

/// Part of FR-231-AC-4 (TC-686): `always holds(not c.late)` reads the same
/// at every position of a behaviour of `Rpc` on its timed lasso under the
/// timed profile and, in a unit selecting infinite-trace instead, on the
/// untimed lasso of the same positions. Not covered: a claim atom
/// `c.x <= 3 ms` reading the clock valuation, which S3 does not admit in a
/// temporal `holds` yet.
#[trace("TC-686")]
#[test]
fn a_claim_reads_alike_with_and_without_time_stamps() {
    use qsl_eval::model_subject::ModelInstance;
    use qsl_eval::simulation::model::BehaviourLetters;
    use qsl_eval::temporal::{
        evaluate_at, evaluate_timed_at, TemporalOutcome, TemporalPosition, TemporalTrace,
    };

    let instance = ModelInstance {
        population: DeclarationKey {
            package: PACKAGE.to_owned(),
            node: ident("calls"),
        },
        key: "c".to_owned(),
    };
    // Each position's reading of the claim, in a unit selecting `profile`.
    let readings = |profile: &str, clock: &str| -> Vec<TemporalOutcome> {
        let world = world_with(
            &format!("profile tt = \"{profile}\";\n"),
            &rpc_unit(
                4,
                &format!(
                    "temporal NotLate using tt over (c: M::Call){clock} on origin \
                     {{ always holds(not c.late) }}"
                ),
            ),
        );
        let system = system(&world);
        let timed = TimedSystem::new(&system, "M").expect("a timed subject");
        let mut behaviour = TimedBehaviour::new(timed.initial().remove(0));
        let send = transition(&timed, &behaviour, "send");
        behaviour
            .advance(&timed, &exact(0, 1), &send)
            .expect("send");
        let reply = transition(&timed, &behaviour, "reply");
        behaviour
            .advance(&timed, &exact(5, 2), &reply)
            .expect("reply");
        let node = &world.package.graph().temporal_clauses()[0];
        let letters = system.clause_letters(node).expect("letters");
        let stamped = behaviour.timed_trace(&timed, true).expect("a timed lasso");
        (0..3)
            .map(|position| {
                let position = TemporalPosition::new(position);
                let mut letters = BehaviourLetters::new(&letters, &instance, &behaviour);
                let mut meter = quire_exact::Meter::new(METER);
                if clock.contains("model-steps") {
                    let untimed = TemporalTrace::Lasso {
                        prefix: 2,
                        loop_len: 1,
                    };
                    evaluate_at(node.clause(), untimed, position, &mut letters, &mut meter)
                } else {
                    evaluate_timed_at(node.clause(), &stamped, position, &mut letters, &mut meter)
                }
                .expect("evaluated")
            })
            .collect()
    };
    let with_time = readings("quire.temporal.timed/v1", " clock \"model-time\"");
    let without_time = readings("quire.temporal.infinite-trace/v1", " clock \"model-steps\"");
    assert_eq!(with_time, without_time);
    assert!(with_time
        .iter()
        .all(|reading| reading == &TemporalOutcome::Holds));
}

/// QSpec FR-181 "Timed subjects" (TC-686): a clock of an object a step makes
/// present takes the constant of its write, 0 when the step writes none,
/// not the value its place held while the object was absent. Over the
/// universe `{c, d}` with `d` absent, `spawn` after a delay of 2 creates `d`
/// with `x = 0` while `c`'s clock reads 2.
#[trace("TC-686")]
#[test]
fn a_created_objects_clock_starts_at_its_write() {
    let world = world(&only(&["spawn"], ""));
    let system = system_over(&world, &["c", "d"]);
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    assert_eq!(timed.places().len(), 2);
    let initial = timed.initial().remove(0);
    let delayed = timed
        .delay(&initial, &exact(2, 1))
        .expect("decided")
        .expect("Idle is quiescent");
    let fresh = |state: &qsl_eval::simulation::model::TimedState| {
        timed
            .places()
            .iter()
            .position(|place| place.present(state.discrete()) && !place.present(delayed.discrete()))
    };
    let (step, created) = timed
        .steps(&delayed)
        .expect("decided")
        .into_iter()
        .find(|(step, reached)| step.operation().contains("Call/spawn") && fresh(reached).is_some())
        .expect("spawn creates d");
    let d = fresh(&created).expect("d is new");
    assert!(step.resets().contains(&(d, exact(0, 1))));
    let c = 1 - d;
    assert_eq!(created.clocks().get(d), Some(&exact(0, 1)));
    assert_eq!(created.clocks().get(c), Some(&exact(2, 1)));
}

/// The unit over [`rpc_document`] with a dense source and `body`, every
/// operation `body` leaves unguarded except `work` disabled.
fn only(enabled: &[&str], body: &str) -> String {
    let disabled: String = ["send", "reply", "timeout", "reset", "tick", "work", "spawn"]
        .iter()
        .filter(|name| !enabled.contains(name))
        .map(|name| format!("pre No{name} using v on M::Call::{name} {{ false }}\n"))
        .collect();
    format!(
        "dimension Time;\nunit s : Time = rational(1, 1);\n\
         unit ms : Time = rational(1, 1000) * s;\ntime M dense unit ms;\n{disabled}{body}"
    )
}

/// FR-232-AC-2: in the strict-guard variant `(Waiting, inflight, x = 3)` is
/// a local time-lock and not deadlocked; in the unmodified model
/// `(Replied, x = 2)` is quiescent and not terminal.
#[trace("TC-687", "FR-232-AC-2")]
#[test]
fn a_strict_guard_time_locks_where_a_closed_one_does_not() {
    use qsl_eval::simulation::model::{TimeLock, TimedTerminalClass};

    let strict = world(&rpc_unit_guarded(3, ">", ""));
    let strict_system = system(&strict);
    let timed = TimedSystem::new(&strict_system, "M").expect("a timed subject");
    let mut behaviour = TimedBehaviour::new(timed.initial().remove(0));
    let send = transition(&timed, &behaviour, "send");
    behaviour
        .advance(&timed, &exact(0, 1), &send)
        .expect("send");
    let locked = timed
        .delay(behaviour.last().state(), &exact(3, 1))
        .expect("decided")
        .expect("admissible");
    assert_eq!(
        timed.time_lock(&locked, 64).expect("decided"),
        Some(TimeLock::Local)
    );
    assert_eq!(
        timed.terminal_class(&locked).expect("decided"),
        TimedTerminalClass::TimeLock
    );
    assert!(!timed.deadlocked(&locked).expect("decided"));

    let plain = world(&rpc_unit(3, ""));
    let plain_system = system(&plain);
    let timed = TimedSystem::new(&plain_system, "M").expect("a timed subject");
    let mut behaviour = TimedBehaviour::new(timed.initial().remove(0));
    let send = transition(&timed, &behaviour, "send");
    behaviour
        .advance(&timed, &exact(0, 1), &send)
        .expect("send");
    let waiting = timed
        .delay(behaviour.last().state(), &exact(3, 1))
        .expect("decided")
        .expect("admissible");
    assert_eq!(timed.time_lock(&waiting, 64).expect("decided"), None);
    let reply = transition(&timed, &behaviour, "reply");
    behaviour
        .advance(&timed, &exact(2, 1), &reply)
        .expect("reply");
    let replied = behaviour.last().state();
    assert_eq!(replied.clocks().values(), [exact(2, 1)]);
    assert!(timed.is_quiescent(replied.discrete()).expect("decided"));
    assert_eq!(
        timed.terminal_class(replied).expect("decided"),
        TimedTerminalClass::NotTerminal
    );
    assert!(!timed.deadlocked(replied).expect("decided"));
}

/// Part of FR-232-AC-3 (TC-687): in `Stall` the timed state
/// `(s0, x = 1)` is a non-local time-lock, since `ping` is enabled there and
/// no state it reaches admits a positive delay. Not covered: `always
/// holds(true)` settling `inconclusive`, `NoAdmittedBehaviour`, which needs
/// the zone search and FR-235's settlement (QSL-581).
#[trace("TC-687")]
#[test]
fn a_stall_that_only_pings_is_a_non_local_time_lock() {
    use qsl_eval::simulation::model::TimeLock;

    let stall = world(&only(
        &["work"],
        "time invariant Stall when { true } { self.x <= 1 ms }\n",
    ));
    let stall_system = system(&stall);
    let timed = TimedSystem::new(&stall_system, "M").expect("a timed subject");
    let initial = timed.initial().remove(0);
    assert_eq!(timed.time_lock(&initial, 64).expect("decided"), None);
    let stuck = timed
        .delay(&initial, &exact(1, 1))
        .expect("decided")
        .expect("admissible");
    assert!(timed
        .delay(&stuck, &exact(1, 1000))
        .expect("decided")
        .is_none());
    assert_eq!(timed.steps(&stuck).expect("decided").len(), 1);
    assert_eq!(
        timed.time_lock(&stuck, 64).expect("decided"),
        Some(TimeLock::NonLocal)
    );
}

/// FR-232-AC-4: in `Serve` the lasso of `idle` steps with `x > 1` and loop
/// delay 1 is unfair under `fair weak serve` when `serve` needs
/// `x >= 1 ms`, enabled at every instant of the loop, and fair when it needs
/// `x <= 1 ms`, disabled at every instant.
#[trace("TC-687", "FR-232-AC-4")]
#[test]
fn weak_fairness_reads_enabledness_at_every_instant_of_the_loop() {
    let fair = |guard: &str| {
        let serve = world(&only(
            &["work", "timeout"],
            &format!(
                "pre Serve using v on M::Call::timeout {{ self.x {guard} 1 ms }}\n\
                 post Served using v on M::Call::timeout {{ self.phase = 3 }}\n"
            ),
        ));
        let serve_system = system(&serve);
        let timed = TimedSystem::new(&serve_system, "M").expect("a timed subject");
        let mut behaviour = TimedBehaviour::new(timed.initial().remove(0));
        let idle = transition(&timed, &behaviour, "work");
        behaviour
            .advance(&timed, &exact(2, 1), &idle)
            .expect("idle");
        behaviour
            .advance(&timed, &exact(1, 1), &idle)
            .expect("idle");
        timed
            .weakly_fair(&behaviour, 1, &exact(0, 1), |step| {
                step.operation().contains("Call/timeout")
            })
            .expect("decided")
    };
    assert!(!fair(">="));
    assert!(fair("<="));
}

/// FR-232 "Fairness over time" over the idle tail (SR-1598001 FND-001): a
/// behaviour of `Serve` that stays at its initial state, `x = 0`, and
/// repeats its stutter step after a delay of 2 holds `serve`'s guard
/// `x >= 1 ms` from time 1 on at every instant, so it is unfair under
/// `fair weak serve`; with the guard `x <= 1 ms`, disabled from time 1 on,
/// it is fair.
#[trace("TC-687")]
#[test]
fn an_idle_tail_is_unfair_to_a_constraint_its_delay_enables() {
    let fair = |guard: &str| {
        let serve = world(&only(
            &["timeout"],
            &format!(
                "pre Serve using v on M::Call::timeout {{ self.x {guard} 1 ms }}\n\
                 post Served using v on M::Call::timeout {{ self.phase = 3 }}\n"
            ),
        ));
        let serve_system = system(&serve);
        let timed = TimedSystem::new(&serve_system, "M").expect("a timed subject");
        let behaviour = TimedBehaviour::new(timed.initial().remove(0));
        timed
            .weakly_fair(&behaviour, 0, &exact(2, 1), |step| {
                step.operation().contains("Call/timeout")
            })
            .expect("decided")
    };
    assert!(!fair(">="));
    assert!(fair("<="));
}

/// FR-232 "Fairness over time" with a guard constant of `10^40 ms`, which
/// FR-230-AC-4 admits (SR-1598001 FND-005): the idle tail's clock is moved
/// past the constant in one step, so the verdict is the one a small
/// constant gives, unfair under `x >= c` and fair under `x <= c`, and it
/// returns at once.
#[trace("TC-687")]
#[test]
fn a_huge_guard_constant_decides_fairness_as_a_small_one_does() {
    let fair = |guard: &str, constant: &str| {
        let serve = world(&only(
            &["timeout"],
            &format!(
                "pre Serve using v on M::Call::timeout {{ self.x {guard} {constant} ms }}\n\
                 post Served using v on M::Call::timeout {{ self.phase = 3 }}\n"
            ),
        ));
        let serve_system = system(&serve);
        let timed = TimedSystem::new(&serve_system, "M").expect("a timed subject");
        let behaviour = TimedBehaviour::new(timed.initial().remove(0));
        timed
            .weakly_fair(&behaviour, 0, &exact(2, 1), |step| {
                step.operation().contains("Call/timeout")
            })
            .expect("decided")
    };
    let huge = format!("1{}", "0".repeat(40));
    let started = std::time::Instant::now();
    for guard in [">=", "<="] {
        assert_eq!(fair(guard, &huge), fair(guard, "1"), "{guard}");
    }
    assert!(!fair(">=", &huge));
    assert!(fair("<=", &huge));
    assert!(started.elapsed() < std::time::Duration::from_secs(30));
}

/// [`rpc_document`] with the package record `Timer { x: Clock; y: Clock;
/// n: Phase }` held by the `Call` field `t`.
fn timer_document() -> Vec<u8> {
    let mut document: Json = serde_json::from_slice(&rpc_document(true)).expect("JSON");
    let timer = ident("Timer");
    let member = |name: &str, type_ref: &str| {
        let mut member = wire_field(name, type_ref);
        let identity = format!("{timer}/{name}");
        member["identity"] = json!(identity);
        member["origin"] = origin(&identity);
        member
    };
    let types = document["types"].as_array_mut().expect("types");
    types.push(wire_type(
        &timer,
        "value_record",
        json!({"fields": [
            member("x", "ix://quire/native/Clock"),
            member("y", "ix://quire/native/Clock"),
            member("n", &ident("Phase")),
        ]}),
    ));
    types[1]["fields"]
        .as_array_mut()
        .expect("Call fields")
        .push(wire_field("t", &timer));
    // `send` also writes `t`.
    for operation in types[1]["operations"].as_array_mut().expect("operations") {
        if operation["name"] == "send" {
            operation["frame"]["modifies"]
                .as_array_mut()
                .expect("a frame")
                .push(json!(ident("Call/t")));
        }
    }
    document["constructs"]
        .as_array_mut()
        .expect("constructs")
        .push(json!({
        "kind": {"module": PACKAGE, "name": "value_record"}, "moduleVersion": "1.0.0",
        "manifestDigest": PLACEHOLDER,
        "construct": {"identity": "none", "shape": "record", "members": {},
            "meaning": meaning::RECORD_VALUE_TYPE}}));
    serde_json::to_vec(&document).expect("JSON")
}

/// Part of FR-230-AC-11 (TC-685): with the record `Timer { x: Clock; y:
/// Clock; n }` held by `Call.t` over a universe of three objects, every
/// object holds its own clock of each place, `t.x`, `t.y` and `x`: nine
/// clocks. The digest of a state holding the record value is defined: an
/// equal state has an equal digest, and it changes with a clock value and
/// with the record's data field (FR-181's `record` form).
#[trace("TC-685")]
#[test]
fn every_object_holds_its_own_clock_of_every_record_place() {
    let holding = |n: &str| {
        world_holding(
            timer_document(),
            "",
            "dimension Time;\nunit s : Time = rational(1, 1);\n\
             unit ms : Time = rational(1, 1000) * s;\ntime M dense unit ms;\n\
             pre Near using v on M::Call::work { self.t.x <= 3 ms }\n",
            json!({"t": {"record": {"type": ident("Timer"), "fields": {"n": {"integer": n}}}}}),
        )
    };
    let world = holding("0");
    let system = system_over(&world, &["c", "d", "e"]);
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    let names: Vec<&str> = timed
        .model()
        .clocks()
        .iter()
        .map(|clock| clock.name())
        .collect();
    assert_eq!(names, ["t.x", "t.y", "x"]);
    assert_eq!(timed.places().len(), 9);
    let [initial] = &timed.initial()[..] else {
        panic!("one initial state");
    };
    // The initial state holds `c` alone, with its three clocks at 0.
    let held = timed
        .places()
        .iter()
        .filter(|place| place.present(initial.discrete()))
        .count();
    assert_eq!(held, 3);
    let digest = timed
        .state_digest(initial)
        .expect("a record state has a digest");
    assert_eq!(
        timed.state_digest(&initial.clone()).expect("a digest"),
        digest
    );
    let shifted = TimedState::new(
        initial.discrete().clone(),
        initial.clocks().delayed(&exact(1, 2)),
    );
    assert_ne!(timed.state_digest(&shifted).expect("a digest"), digest);
    // One value at `t.x` and the other at `t.y` are two states.
    let at = |name: &str| {
        timed
            .places()
            .iter()
            .position(|place| {
                place.object().object().as_str() == "c"
                    && timed.model().clock(place.clock()).map(|clock| clock.name()) == Some(name)
            })
            .expect("a place of c")
    };
    let one_at = |place: usize| {
        let mut values = vec![exact(0, 1); timed.places().len()];
        values[place] = exact(1, 1);
        timed
            .state_digest(&TimedState::new(
                initial.discrete().clone(),
                ClockValuation::new(values),
            ))
            .expect("a digest")
    };
    assert_ne!(one_at(at("t.x")), one_at(at("t.y")));
    let other = holding("1");
    let other_system = system_over(&other, &["c", "d", "e"]);
    let other_timed = TimedSystem::new(&other_system, "M").expect("a timed subject");
    let [other_initial] = &other_timed.initial()[..] else {
        panic!("one initial state");
    };
    assert_ne!(
        other_timed.state_digest(other_initial).expect("a digest"),
        digest,
        "the record's data field is in the state key"
    );
}

/// Part of FR-230-AC-11 (TC-685): a state holding a record value steps.
/// From `Call.t = Timer { n: 0 }`, after a delay of 2, `send` (whose
/// postcondition writes `phase` and keeps `t.n`) has its four successors,
/// each keeping `t` and every clock at 2, and each post state has a digest.
/// `t`'s frame grants it, so its data part ranges over its finite domain
/// where the postcondition leaves it free
/// ([`a_granted_field_ranges_over_its_finite_domain_or_stops`]).
#[trace("TC-685")]
#[test]
fn a_record_holding_state_steps() {
    let world = world_holding(
        timer_document(),
        "",
        &only(
            &["send"],
            "post Send using v on M::Call::send { self.phase = 1 and self.t.n = pre(self.t.n) }\n",
        ),
        json!({"t": {"record": {"type": ident("Timer"), "fields": {"n": {"integer": "0"}}}}}),
    );
    let system = system(&world);
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    let [initial] = &timed.initial()[..] else {
        panic!("one initial state");
    };
    let later = timed
        .delay(initial, &exact(2, 1))
        .expect("decided")
        .expect("no invariant bounds the delay");
    let steps = timed.steps(&later).expect("a record-holding state steps");
    assert_eq!(
        steps.len(),
        4,
        "one successor per value of `inflight` and `late`"
    );
    let t_key = DeclarationKey {
        package: PACKAGE.to_owned(),
        node: ident("Call/t"),
    };
    let t_of = |state: &TimedState| {
        state
            .discrete()
            .state()
            .populations
            .values()
            .find_map(|objects| objects.get("c"))
            .and_then(|object| object.fields.get(&t_key))
            .cloned()
            .expect("c holds t")
    };
    for (step, post) in &steps {
        assert!(step.operation().contains("Call/send\""));
        assert!(post
            .clocks()
            .values()
            .iter()
            .all(|value| value == &exact(2, 1)));
        let (quire_exact::Value::Composite(before), quire_exact::Value::Composite(after)) =
            (t_of(&later), t_of(post))
        else {
            panic!("t is a record");
        };
        assert_eq!(
            format!("{:?}", before.slots()),
            format!("{:?}", after.slots()),
            "send keeps t"
        );
        timed
            .state_digest(post)
            .expect("the post state has a digest");
    }
}

/// Part of FR-230-AC-11 (TC-685), SR-1646005: a record holding an enum
/// data field compares by case, so an operation whose frame does not grant
/// the record keeps it unchanged and steps. With `Timer { x; y: Clock; n:
/// Phase; m: Mode }` and `Call.t = Timer { n: 0, m: A }`, `reply`, whose
/// frame does not name `t`, is offered alongside `send`, whose frame does.
#[trace("TC-685")]
#[test]
fn a_record_holding_an_enum_steps_where_its_frame_does_not_grant_it() {
    let mut document: Json = serde_json::from_slice(&timer_document()).expect("JSON");
    let mode = ident("Mode");
    let case = |name: &str, line: u64| {
        json!({"identity": ident(&format!("variant/Mode-{name}")), "name": name,
            "origin": {"source": {"sourceIdentity": ident("spec"), "path": "spec.qspec",
                "startLine": line, "startColumn": 3}}})
    };
    let types = document["types"].as_array_mut().expect("types");
    types.push(wire_type(
        &mode,
        "mode",
        json!({"variants": [case("A", 10), case("B", 11)]}),
    ));
    let timer = types
        .iter_mut()
        .find(|ty| ty["identity"] == ident("Timer"))
        .expect("Timer");
    let identity = format!("{}/m", ident("Timer"));
    let mut member = wire_field("m", &mode);
    member["identity"] = json!(identity);
    member["origin"] = origin(&identity);
    timer["fields"].as_array_mut().expect("fields").push(member);
    document["constructs"]
        .as_array_mut()
        .expect("constructs")
        .push(json!({
            "kind": {"module": PACKAGE, "name": "mode"}, "moduleVersion": "1.0.0",
            "manifestDigest": PLACEHOLDER,
            "construct": {"identity": "none", "shape": "enumeration",
                "members": {"variants": "required"}, "meaning": meaning::VARIANT_TYPE}}));
    let world = world_holding(
        serde_json::to_vec(&document).expect("JSON"),
        "",
        "dimension Time;\nunit s : Time = rational(1, 1);\n\
         unit ms : Time = rational(1, 1000) * s;\ntime M dense unit ms;\n",
        json!({"t": {"record": {"type": ident("Timer"), "fields": {
            "n": {"integer": "0"},
            "m": {"variant": {"type": mode, "case": "A"}},
        }}}}),
    );
    let held = system(&world);
    let timed = TimedSystem::new(&held, "M").expect("a timed subject");
    let [initial] = &timed.initial()[..] else {
        panic!("one initial state");
    };
    let offered: Vec<String> = timed
        .guarded_steps(initial.discrete())
        .expect("the state expands")
        .iter()
        .map(|step| step.operation().to_owned())
        .collect();
    for name in ["reply", "send"] {
        let suffix = format!("Call/{name}\"");
        assert!(
            offered.iter().any(|operation| operation.contains(&suffix)),
            "`{name}` is offered: {offered:?}"
        );
    }
}

/// FR-230-AC-10 (TC-685), over a universe: `Call` with `peer:
/// Reference<Call>` and `function peer_clock(o: Call): Clock { o.peer.x }`.
/// `peer_clock(self) <= 2 ms` constrains the clock of the object each
/// state's `self.peer` refers to: with `c1.peer = c2` and `c2.peer = c1`,
/// `c1`'s `work` is guarded by `c2.x <= 2` and `c2`'s by `c1.x <= 2`, and
/// after a delay of 3 and `c2`'s `send` resets `c2.x`, `c1`'s guard holds
/// and `c2`'s does not. `own_clock(self.peer) <= 2 ms` of `function
/// own_clock(o: Call): Clock { o.x }` reads the same places, and `not
/// (peer_clock(self) <= 2 ms)` their negation (SR-1651 GA FND-002).
#[trace("TC-685")]
#[test]
fn a_peer_clock_is_the_clock_of_the_referenced_object() {
    // `peer_clock(self)`, `own_clock(self.peer)` and the negation of the
    // first: each receiver's `work` is guarded by its peer's clock.
    let peer_clock = "function peer_clock using v(o: M::Call): Clock pure { o.peer.x }\n";
    let own_clock = "function own_clock using v(o: M::Call): Clock pure { o.x }\n";
    for (declaration, guard, holds) in [
        (peer_clock, "peer_clock(self) <= 2 ms", true),
        (own_clock, "own_clock(self.peer) <= 2 ms", true),
        (peer_clock, "not (peer_clock(self) <= 2 ms)", false),
    ] {
        peer_guarded_work(declaration, guard, holds);
    }
}

/// [`a_peer_clock_is_the_clock_of_the_referenced_object`] over the guard
/// `guard` of `work`, which reads the peer's clock `x <= 2` when `holds`
/// and its negation otherwise.
fn peer_guarded_work(declaration: &str, guard: &str, holds: bool) {
    let mut document: Json = serde_json::from_slice(&rpc_document(true)).expect("JSON");
    document["types"][1]["fields"]
        .as_array_mut()
        .expect("Call fields")
        .push(wire_field("peer", &ident("Call")));
    let document = serde_json::to_vec(&document).expect("JSON");
    let peer = |key: &str| json!({"reference": {"population": ident("calls"), "key": key}});
    let object = |key: &str, other: &str| {
        json!({"key": key, "type": ident("Call"),
            "fields": initial_fields(json!({"peer": peer(other)}))})
    };
    let world = world_of(
        document,
        "",
        &only(
            &["send", "work"],
            &format!(
                "{declaration}pre Near using v on M::Call::work {{ {guard} }}\n\
                 post Send using v on M::Call::send {{ self.x = 0 ms }}\n"
            ),
        ),
        json!([object("c1", "c2"), object("c2", "c1")]),
    );
    let system = system_over(&world, &["c1", "c2"]);
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    let key =
        |place: usize| -> String { timed.places()[place].object().object().as_str().to_owned() };
    let [initial] = &timed.initial()[..] else {
        panic!("one initial state");
    };
    // Each receiver's `work` names its peer's clock.
    let guarded = |state: &TimedState| -> Vec<(String, String, bool)> {
        timed
            .guarded_steps(state.discrete())
            .expect("the state expands")
            .iter()
            .filter(|step| step.operation().contains("Call/work\""))
            .flat_map(|step| {
                let receiver = step.receiver().object().as_str().to_owned();
                step.parts()
                    .iter()
                    .flatten()
                    .map(|constraint| {
                        (
                            receiver.clone(),
                            key(constraint.place()),
                            constraint.holds(state.clocks()),
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    };
    let mut named = guarded(initial);
    named.sort();
    assert_eq!(
        named,
        [
            ("c1".to_owned(), "c2".to_owned(), holds),
            ("c2".to_owned(), "c1".to_owned(), holds)
        ],
        "{guard}"
    );
    let late = timed
        .delay(initial, &exact(3, 1))
        .expect("decided")
        .expect("no invariant bounds the delay");
    let send = timed
        .steps(&late)
        .expect("the state expands")
        .into_iter()
        .find(|(step, _)| {
            step.operation().contains("Call/send\"") && step.receiver().object().as_str() == "c2"
        })
        .expect("c2 sends");
    let mut named = guarded(&send.1);
    named.sort();
    assert_eq!(
        named,
        [
            ("c1".to_owned(), "c2".to_owned(), holds),
            ("c2".to_owned(), "c1".to_owned(), !holds)
        ],
        "{guard}"
    );
}

/// [`timer_document`] whose `Call` also holds `ts: Timer` of multiplicity
/// `[0, upper]` (`None`: unbounded), ordered and not unique: the package
/// form of `Sequence<Timer>`.
fn timer_sequence_document(upper: Option<u64>) -> Vec<u8> {
    timer_collection_document(upper, true, false)
}

/// [`timer_sequence_document`] whose `ts` is `ordered` and `unique` as
/// given: the package form of `Set<Timer>` or `Bag<Timer>` as well.
fn timer_collection_document(upper: Option<u64>, ordered: bool, unique: bool) -> Vec<u8> {
    let mut document: Json = serde_json::from_slice(&timer_document()).expect("JSON");
    let mut held = wire_field("ts", &ident("Timer"));
    held["multiplicity"] = json!({"lower": 0, "ordered": ordered, "unique": unique});
    if let Some(upper) = upper {
        held["multiplicity"]["upper"] = json!(upper);
    }
    document["types"][1]["fields"]
        .as_array_mut()
        .expect("Call fields")
        .push(held);
    // `send` also writes `ts`.
    for operation in document["types"][1]["operations"]
        .as_array_mut()
        .expect("operations")
    {
        if operation["name"] == "send" {
            operation["frame"]["modifies"]
                .as_array_mut()
                .expect("a frame")
                .push(json!(ident("Call/ts")));
        }
    }
    serde_json::to_vec(&document).expect("JSON")
}

/// Part of FR-230-AC-19 and FR-230-AC-23 (TC-685), the record parts: with
/// `ts: Sequence<Timer>` of at most 2 elements, an object holds one clock of
/// `ts.[slot].x` and of `ts.[slot].y` for each slot up to the maximum, 7
/// places in all. A slot is live exactly while the sequence holds it: from
/// `ts = [Timer { n: 0 }]` slot 0 is live and slot 1 is not, so a value of
/// slot 1 is no part of the state digest while those of slot 0 are, each
/// by its slot. The state steps, every live clock advancing with the delay.
/// A sequence with no declared maximum is bounded by `max_clocks`
/// ([`an_unbounded_collection_is_bounded_by_max_clocks`]).
#[trace("TC-685")]
#[test]
fn every_slot_of_a_record_sequence_holds_its_own_clocks() {
    let timer =
        |n: &str| json!({"record": {"type": ident("Timer"), "fields": {"n": {"integer": n}}}});
    let world = world_holding(
        timer_sequence_document(Some(2)),
        "",
        &only(
            &["send"],
            "post Send using v on M::Call::send { self.phase = 1 and self.ts = pre(self.ts) }\n",
        ),
        json!({"t": timer("0"), "ts": {"sequence": [timer("0")]}}),
    );
    let held = system(&world);
    let timed = TimedSystem::new(&held, "M").expect("a timed subject");
    let names: Vec<&str> = timed
        .model()
        .clocks()
        .iter()
        .map(|clock| clock.name())
        .collect();
    assert_eq!(names, ["t.x", "t.y", "ts.[slot].x", "ts.[slot].y", "x"]);
    let slots: Vec<(&str, &[usize])> = timed
        .places()
        .iter()
        .map(|place| {
            (
                timed.model().clock(place.clock()).expect("a clock").name(),
                place.slots(),
            )
        })
        .collect();
    assert_eq!(
        slots,
        [
            ("t.x", &[][..]),
            ("t.y", &[][..]),
            ("ts.[slot].x", &[0][..]),
            ("ts.[slot].x", &[1][..]),
            ("ts.[slot].y", &[0][..]),
            ("ts.[slot].y", &[1][..]),
            ("x", &[][..]),
        ]
    );
    let [initial] = &timed.initial()[..] else {
        panic!("one initial state");
    };
    let live: Vec<bool> = timed
        .places()
        .iter()
        .map(|place| timed.live(place, initial.discrete()))
        .collect();
    assert_eq!(live, [true, true, true, false, true, false, true]);
    let one_at = |place: usize| {
        let mut values = vec![exact(0, 1); timed.places().len()];
        values[place] = exact(1, 1);
        timed
            .state_digest(&TimedState::new(
                initial.discrete().clone(),
                ClockValuation::new(values),
            ))
            .expect("a digest")
    };
    let base = timed.state_digest(initial).expect("a digest");
    assert_eq!(one_at(3), base, "slot 1 is not live");
    assert_ne!(one_at(2), base);
    assert_ne!(one_at(2), one_at(4), "ts[0].x and ts[0].y are two places");
    assert_ne!(one_at(2), one_at(0), "ts[0].x and t.x are two places");

    let later = timed
        .delay(initial, &exact(2, 1))
        .expect("decided")
        .expect("no invariant bounds the delay");
    let steps = timed
        .steps(&later)
        .expect("a collection-holding state steps");
    assert!(!steps.is_empty());
    for (_, post) in &steps {
        assert!(post
            .clocks()
            .values()
            .iter()
            .all(|value| value == &exact(2, 1)));
        timed.state_digest(post).expect("a digest");
    }
}

/// Part of FR-230-AC-19 (TC-685), SR-1655002 FND-002 and SR-1676001
/// FND-001: a sequence with no declared maximum records `max: None` and is
/// bounded in analysis by `max_clocks` on the live clocks of a state (QSpec
/// FR-415 "Cardinality bound"). The live clocks of a state are `t.x`, `t.y`
/// and `x` and two for each element of `ts`. With `max_clocks` 7, a state
/// holding two elements (7 live clocks) steps, and one holding three (9)
/// stops `MaxClocks { max_clocks: 7 }` when it is expanded, as it does not
/// under the default bound. With `max_clocks` 3, 4 and 5 a state holding two
/// elements stops, whatever room the bound leaves for listing a slot, and
/// does not step with its element clocks missing.
#[trace("TC-685")]
#[test]
fn an_unbounded_collection_is_bounded_by_max_clocks() {
    let timer =
        |n: &str| json!({"record": {"type": ident("Timer"), "fields": {"n": {"integer": n}}}});
    let holding = |held: usize| {
        world_holding(
            timer_sequence_document(None),
            "",
            &only(
                &["send"],
                "post Send using v on M::Call::send { self.phase = 1 and self.ts = pre(self.ts) }\n",
            ),
            json!({"t": timer("0"), "ts": {"sequence": vec![timer("1"); held]}}),
        )
    };
    for (held, max_clocks, stops) in [
        (2, 7, false),
        (3, 7, true),
        (2, 3, true),
        (2, 4, true),
        (2, 5, true),
    ] {
        let world = holding(held);
        let held_system = system(&world);
        let bound = std::num::NonZeroU64::new(max_clocks).expect("a positive bound");
        let timed =
            TimedSystem::with_max_clocks(&held_system, "M", bound).expect("a timed subject");
        let [initial] = &timed.initial()[..] else {
            panic!("one initial state");
        };
        assert_eq!(timed.live_clocks(initial.discrete()), 3 + 2 * held as u64);
        let steps = timed.steps(initial);
        if stops {
            assert_eq!(steps.err(), Some(TimedStop::MaxClocks { max_clocks }));
            let roomy = TimedSystem::new(&held_system, "M").expect("a timed subject");
            let [initial] = &roomy.initial()[..] else {
                panic!("one initial state");
            };
            assert!(!roomy.steps(initial).expect("the state steps").is_empty());
        } else {
            assert!(!steps.expect("the state steps").is_empty());
        }
    }
}

/// One guard part's constraints: each constrained place's slots and
/// whether the constraint holds.
type GuardParts = Vec<(Vec<usize>, bool)>;

/// Part of FR-230-AC-21 (TC-685), "Quantified reads" over a universe: with
/// `ts: Sequence<Timer>[0, 2]` holding one `Timer { n: 2 }`, `work`'s guard
/// `exists(e in self.ts: e.n = 2 and e.x >= 1 ms)` is the one part of slot
/// 0 (slot 1 is not held), `ts[0].x >= 1`, false at 0 and true after a
/// delay of 2; `forall(e in self.ts: e.x <= 1 ms)` is the entry of size 1,
/// `ts[0].x <= 1`, true at 0 and false after the delay. With `n: 3` the
/// `exists` part's data predicate is false and `work` has no part.
#[trace("TC-685")]
#[test]
fn a_quantified_guard_reads_the_clocks_of_the_held_slots() {
    let timer =
        |n: &str| json!({"record": {"type": ident("Timer"), "fields": {"n": {"integer": n}}}});
    let guarded = |guard: &str, n: &str| -> (Vec<GuardParts>, Vec<GuardParts>) {
        let world = world_holding(
            timer_sequence_document(Some(2)),
            "",
            &only(
                &["work"],
                &format!("pre Near using v on M::Call::work {{ {guard} }}\n"),
            ),
            json!({"t": timer("0"), "ts": {"sequence": [timer(n)]}}),
        );
        let held = system(&world);
        let timed = TimedSystem::new(&held, "M").expect("a timed subject");
        let [initial] = &timed.initial()[..] else {
            panic!("one initial state");
        };
        let read = |state: &TimedState| {
            timed
                .guarded_steps(state.discrete())
                .expect("the state expands")
                .iter()
                .filter(|step| step.operation().contains("Call/work\""))
                .flat_map(|step| step.parts().to_vec())
                .map(|part| {
                    part.iter()
                        .map(|constraint| {
                            (
                                timed.places()[constraint.place()].slots().to_vec(),
                                constraint.holds(state.clocks()),
                            )
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        };
        let later = timed
            .delay(initial, &exact(2, 1))
            .expect("decided")
            .expect("no invariant bounds the delay");
        (read(initial), read(&later))
    };
    assert_eq!(
        guarded("exists(e in self.ts: e.n = 2 and e.x >= 1 ms)", "2"),
        (vec![vec![(vec![0], false)]], vec![vec![(vec![0], true)]])
    );
    assert_eq!(
        guarded("forall(e in self.ts: e.x <= 1 ms)", "2"),
        (vec![vec![(vec![0], true)]], vec![vec![(vec![0], false)]])
    );
    assert_eq!(
        guarded("exists(e in self.ts: e.n = 2 and e.x >= 1 ms)", "3"),
        (vec![], vec![])
    );
}

/// Part of FR-230-AC-21 (TC-685), SR-1659002 FND-001: over `ts:
/// Sequence<Timer>` with no declared maximum holding `[Timer { n: 2 },
/// Timer { n: 1 }]`, the family of a quantified guard is read at each held
/// slot. `forall(e in self.ts: e.x <= 1 ms)` is one part constraining slots
/// 0 and 1, true at 0 and false after a delay of 2; `exists(e in self.ts:
/// e.n = 2 and e.x >= 1 ms)` is the part of slot 0 alone, whose element has
/// `n = 2`; `forall(e in self.ts: e.n = 2 implies e.x >= 1 ms)` constrains
/// slot 0, and slot 1 either not at all (its `n` is not 2) or by `x >= 1`.
#[trace("TC-685")]
#[test]
fn an_unbounded_quantified_guard_reads_each_held_slot() {
    let timer =
        |n: &str| json!({"record": {"type": ident("Timer"), "fields": {"n": {"integer": n}}}});
    let guarded = |guard: &str| -> (Vec<GuardParts>, Vec<GuardParts>) {
        let world = world_holding(
            timer_sequence_document(None),
            "",
            &only(
                &["work"],
                &format!("pre Near using v on M::Call::work {{ {guard} }}\n"),
            ),
            json!({"t": timer("0"), "ts": {"sequence": [timer("2"), timer("1")]}}),
        );
        let held = system(&world);
        let timed = TimedSystem::with_max_clocks(
            &held,
            "M",
            std::num::NonZeroU64::new(16).expect("positive"),
        )
        .expect("a timed subject");
        let [initial] = &timed.initial()[..] else {
            panic!("one initial state");
        };
        let read = |state: &TimedState| {
            timed
                .guarded_steps(state.discrete())
                .expect("the state expands")
                .iter()
                .filter(|step| step.operation().contains("Call/work\""))
                .flat_map(|step| step.parts().to_vec())
                .map(|part| {
                    part.iter()
                        .map(|constraint| {
                            (
                                timed.places()[constraint.place()].slots().to_vec(),
                                constraint.holds(state.clocks()),
                            )
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        };
        let later = timed
            .delay(initial, &exact(2, 1))
            .expect("decided")
            .expect("no invariant bounds the delay");
        (read(initial), read(&later))
    };
    assert_eq!(
        guarded("forall(e in self.ts: e.x <= 1 ms)"),
        (
            vec![vec![(vec![0], true), (vec![1], true)]],
            vec![vec![(vec![0], false), (vec![1], false)]]
        )
    );
    assert_eq!(
        guarded("exists(e in self.ts: e.n = 2 and e.x >= 1 ms)"),
        (vec![vec![(vec![0], false)]], vec![vec![(vec![0], true)]])
    );
    assert_eq!(
        guarded("forall(e in self.ts: e.n = 2 implies e.x >= 1 ms)"),
        (
            vec![
                vec![(vec![0], false)],
                vec![(vec![0], false), (vec![1], false)]
            ],
            vec![
                vec![(vec![0], true)],
                vec![(vec![0], true), (vec![1], true)]
            ]
        )
    );
}

/// Part of FR-230-AC-21 (TC-685), "Resets by selection" over a universe:
/// from `ts = [Timer { n: 2 }, Timer { n: 1 }]` after a delay of 3, `send`
/// with `forall(e in self.ts: e.n > 1 implies e.x = 0 ms)` resets the clock
/// `x` of slot 0, whose element has `n = 2`, and keeps slot 1's and every
/// other clock at 3.
#[trace("TC-685")]
#[test]
fn a_forall_reset_writes_the_selected_slots() {
    let timer =
        |n: &str| json!({"record": {"type": ident("Timer"), "fields": {"n": {"integer": n}}}});
    let world = world_holding(
        timer_sequence_document(Some(2)),
        "",
        &only(
            &["send"],
            "post Send using v on M::Call::send { forall(e in self.ts: e.n > 1 implies e.x = 0 ms) }\n",
        ),
        json!({"t": timer("0"), "ts": {"sequence": [timer("2"), timer("1")]}}),
    );
    let held = system(&world);
    let timed = TimedSystem::new(&held, "M").expect("a timed subject");
    assert!(!timed.writes_collections());
    let [initial] = &timed.initial()[..] else {
        panic!("one initial state");
    };
    let later = timed
        .delay(initial, &exact(3, 1))
        .expect("decided")
        .expect("no invariant bounds the delay");
    let steps = timed.steps(&later).expect("the state steps");
    assert!(!steps.is_empty());
    for (step, post) in &steps {
        assert!(step.operation().contains("Call/send\""));
        let values: Vec<(String, Vec<usize>, Rational)> = timed
            .places()
            .iter()
            .zip(post.clocks().values())
            .map(|(place, value)| {
                (
                    timed
                        .model()
                        .clock(place.clock())
                        .expect("a clock")
                        .name()
                        .to_owned(),
                    place.slots().to_vec(),
                    value.clone(),
                )
            })
            .filter(|(name, _, _)| name == "ts.[slot].x")
            .collect();
        assert_eq!(
            values,
            [
                ("ts.[slot].x".to_owned(), vec![0], exact(0, 1)),
                ("ts.[slot].x".to_owned(), vec![1], exact(3, 1)),
            ]
        );
        assert_eq!(
            post.clocks()
                .values()
                .iter()
                .filter(|value| **value == exact(3, 1))
                .count(),
            timed.places().len() - 1
        );
    }
}

/// Part of FR-230-AC-19 (TC-685), SR-1655002 FND-004: a `Set<Timer>[0, 3]`
/// and a `Bag<Timer>[0, 3]` holding two elements list three slots of
/// `ts.[slot].x` per object, of which slots 0 and 1 are live.
#[trace("TC-685")]
#[test]
fn a_set_or_bag_of_records_holds_a_clock_per_held_slot() {
    let timer =
        |n: &str| json!({"record": {"type": ident("Timer"), "fields": {"n": {"integer": n}}}});
    for (unique, kind) in [(true, "set"), (false, "bag")] {
        let world = world_holding(
            timer_collection_document(Some(3), false, unique),
            "",
            "dimension Time;\nunit s : Time = rational(1, 1);\n\
             unit ms : Time = rational(1, 1000) * s;\ntime M dense unit ms;\n",
            json!({"t": timer("0"), "ts": {kind: [timer("1"), timer("2")]}}),
        );
        let held = system(&world);
        let timed = TimedSystem::new(&held, "M").expect("a timed subject");
        let [initial] = &timed.initial()[..] else {
            panic!("one initial state");
        };
        let live: Vec<(Vec<usize>, bool)> = timed
            .places()
            .iter()
            .filter(|place| {
                timed.model().clock(place.clock()).map(|clock| clock.name()) == Some("ts.[slot].x")
            })
            .map(|place| {
                (
                    place.slots().to_vec(),
                    timed.live(place, initial.discrete()),
                )
            })
            .collect();
        assert_eq!(
            live,
            [(vec![0], true), (vec![1], true), (vec![2], false)],
            "{kind}"
        );
    }
}

/// [`timer_sequence_document`] of at most 3 elements whose `reply` writes
/// `ts` as well as `send`.
fn timer_sequence_written_by_reply() -> Vec<u8> {
    let mut document: Json =
        serde_json::from_slice(&timer_sequence_document(Some(3))).expect("JSON");
    for operation in document["types"][1]["operations"]
        .as_array_mut()
        .expect("operations")
    {
        if operation["name"] == "reply" {
            operation["frame"]["modifies"]
                .as_array_mut()
                .expect("a frame")
                .push(json!(ident("Call/ts")));
        }
    }
    serde_json::to_vec(&document).expect("JSON")
}

/// The value of each live place of `ts.[slot].x` and `ts.[slot].y` at
/// `state`, by clock name and slot.
fn slot_clocks(timed: &TimedSystem<'_, '_>, state: &TimedState) -> Vec<(String, usize, Rational)> {
    timed
        .places()
        .iter()
        .zip(state.clocks().values())
        .filter(|(place, _)| timed.live(place, state.discrete()))
        .filter_map(|(place, value)| {
            let name = timed.model().clock(place.clock())?.name();
            name.starts_with("ts.")
                .then(|| (name.to_owned(), place.slots()[0], value.clone()))
        })
        .collect()
}

/// A state `operation` steps to from `state`, after checking that every
/// step of `operation` gives `ts` the same clocks: the frame leaves the
/// operation's other fields free, so it has a step for each of their values.
/// Where the operation's frame grants `ts` and its postcondition leaves
/// `ts`'s data free, the steps that keep `ts`'s data.
fn only_step(timed: &TimedSystem<'_, '_>, state: &TimedState, operation: &str) -> TimedState {
    let steps = timed.steps(state).expect("the state steps");
    let ts_key = DeclarationKey {
        package: PACKAGE.to_owned(),
        node: ident("Call/ts"),
    };
    let ts_of = |state: &TimedState| {
        state
            .discrete()
            .state()
            .populations
            .values()
            .find_map(|objects| objects.get("c"))
            .and_then(|object| object.fields.get(&ts_key))
            .map(|value| format!("{value:?}"))
    };
    let steps: Vec<_> = steps
        .into_iter()
        .filter(|(step, _)| step.operation().contains(&format!("Call/{operation}\"")))
        .collect();
    let kept: Vec<_> = steps
        .iter()
        .filter(|(_, post)| ts_of(post) == ts_of(state))
        .cloned()
        .collect();
    let mut posts = if kept.is_empty() { steps } else { kept }
        .into_iter()
        .map(|(_, post)| post);
    let first = posts.next().expect("the operation steps");
    for post in posts {
        assert_eq!(slot_clocks(timed, &post), slot_clocks(timed, &first));
    }
    first
}

/// Part of QSpec FR-181-AC-25 and FR-230-AC-20 (TC-685), the record part
/// of collection-valued writes, executed: from `ts = [Timer { n: 1 },
/// Timer { n: 2 }]`, a delay of 3 and a `send` resetting slot 1's `x` give
/// slot 0 `x = 3` and slot 1 `x = 0`; after a delay of 1 `reply` writes
/// `self.ts = filter(e in pre(self.ts): e.n > 1)`, which filters out slot 0
/// and moves slot 1's clocks to slot 0 with their values (`x = 1`, `y =
/// 4`), slot 1 no longer live. A `send` writing the literal
/// `sequence[Timer { x: 2 ms, y: 5 ms, n: 3 }]` gives the fresh slot 0 its
/// literal's constants. The model writes a collection, which
/// `writes_collections` reports so that a zone analysis refuses it by name;
/// a model with only resets by selection does not. Not covered: a fresh
/// insert at the front, which
/// AC-20 spells with `flatten` over a nested literal that does not type
/// (STD-493 item 29).
#[trace("TC-685")]
#[test]
fn a_collection_write_moves_each_clock_with_its_element() {
    let timer =
        |n: &str| json!({"record": {"type": ident("Timer"), "fields": {"n": {"integer": n}}}});
    let world = world_holding(
        timer_sequence_written_by_reply(),
        "",
        &only(
            &["send", "reply"],
            "post Send using v on M::Call::send { forall(e in self.ts: e.n = 2 implies e.x = 0 ms) }\n\
             post Reply using v on M::Call::reply { self.ts = filter(e in pre(self.ts): e.n > 1) }\n",
        ),
        json!({"t": timer("0"), "ts": {"sequence": [timer("1"), timer("2")]}}),
    );
    let held = system(&world);
    let timed = TimedSystem::new(&held, "M").expect("a timed subject");
    let [initial] = &timed.initial()[..] else {
        panic!("one initial state");
    };
    let delayed = |state: &TimedState, by: i64| {
        timed
            .delay(state, &exact(by, 1))
            .expect("decided")
            .expect("no invariant bounds the delay")
    };
    // A zone over places maps no carried clock, so zone analyses refuse
    // the model by name.
    assert!(timed.writes_collections());
    let sent = only_step(&timed, &delayed(initial, 3), "send");
    let at = |name: &str, slot: usize, value: i64| (name.to_owned(), slot, exact(value, 1));
    assert_eq!(
        slot_clocks(&timed, &sent),
        [
            at("ts.[slot].x", 0, 3),
            at("ts.[slot].x", 1, 0),
            at("ts.[slot].y", 0, 3),
            at("ts.[slot].y", 1, 3),
        ]
    );
    let replied = only_step(&timed, &delayed(&sent, 1), "reply");
    assert_eq!(
        slot_clocks(&timed, &replied),
        [at("ts.[slot].x", 0, 1), at("ts.[slot].y", 0, 4)]
    );

    let world = world_holding(
        timer_sequence_document(Some(3)),
        "",
        &only(
            &["send"],
            "post Send using v on M::Call::send { self.ts = sequence[M::Timer { x: 2 ms, y: 5 ms, n: 3 }] }\n",
        ),
        json!({"t": timer("0"), "ts": {"sequence": [timer("1"), timer("2")]}}),
    );
    let held = system(&world);
    let timed = TimedSystem::new(&held, "M").expect("a timed subject");
    let [initial] = &timed.initial()[..] else {
        panic!("one initial state");
    };
    let sent = only_step(&timed, &delayed(initial, 3), "send");
    assert_eq!(
        slot_clocks(&timed, &sent),
        [at("ts.[slot].x", 0, 2), at("ts.[slot].y", 0, 5)]
    );
}

/// Part of FR-230-AC-20 (TC-685), SR-1662001 FND-003: occurrences of a set
/// literal equal as values are one element, however they are spelled.
/// `set[Timer { x: 0 ms, y: 0 ms, n: 0 }, Timer { x: 5 ms, y: 5 ms, n:
/// pre(self.t.n) }]`, with `t.n = 0`, writes one element whose clocks take
/// the first occurrence's constants.
#[trace("TC-685")]
#[test]
fn equal_occurrences_of_a_set_literal_are_one_element() {
    let timer =
        |n: &str| json!({"record": {"type": ident("Timer"), "fields": {"n": {"integer": n}}}});
    let world = world_holding(
        timer_collection_document(Some(3), false, true),
        "",
        &only(
            &["send"],
            "post Send using v on M::Call::send { self.ts = set[M::Timer { x: 0 ms, y: 0 ms, n: 0 }, \
             M::Timer { x: 5 ms, y: 5 ms, n: pre(self.t.n) }] }\n",
        ),
        json!({"t": timer("0"), "ts": {"set": [timer("1"), timer("2")]}}),
    );
    let held = system(&world);
    let timed = TimedSystem::new(&held, "M").expect("a timed subject");
    let [initial] = &timed.initial()[..] else {
        panic!("one initial state");
    };
    let later = timed
        .delay(initial, &exact(3, 1))
        .expect("decided")
        .expect("no invariant bounds the delay");
    let sent = only_step(&timed, &later, "send");
    let at = |name: &str, slot: usize, value: i64| (name.to_owned(), slot, exact(value, 1));
    assert_eq!(
        slot_clocks(&timed, &sent),
        [at("ts.[slot].x", 0, 0), at("ts.[slot].y", 0, 0)]
    );
}

/// SR-1662001 FND-001: a collection write the engine cannot compute from
/// the pre-state refuses the step by name and never drops it.
/// `self.ts = filter(e in self.ts: e.n > 1)` reads `ts` in the post-state
/// (FR-104, STD-493 item 28), so expanding the state stops
/// `unsupported_construct`/`collection-write-not-computed`.
#[trace("TC-685")]
#[test]
fn a_collection_write_read_in_the_post_state_refuses_its_step() {
    let timer =
        |n: &str| json!({"record": {"type": ident("Timer"), "fields": {"n": {"integer": n}}}});
    let world = world_holding(
        timer_sequence_document(Some(3)),
        "",
        &only(
            &["send"],
            "post Send using v on M::Call::send { self.ts = filter(e in self.ts: e.n > 1) }\n",
        ),
        json!({"t": timer("0"), "ts": {"sequence": [timer("1"), timer("2")]}}),
    );
    let held = system(&world);
    let timed = TimedSystem::new(&held, "M").expect("a timed subject");
    let [initial] = &timed.initial()[..] else {
        panic!("one initial state");
    };
    let Err(TimedStop::Expansion(stop)) = timed.steps(initial) else {
        panic!("the step is refused");
    };
    assert_eq!(
        stop.cause,
        CatalogCode::new("unsupported_construct", "collection-write-not-computed")
    );
}

/// [`timer_document`] with a record `Holder { ts: Timer [0, 2] }` held by
/// `Call.h`.
fn timer_holder_document() -> Vec<u8> {
    let mut document: Json = serde_json::from_slice(&timer_document()).expect("JSON");
    let holder = ident("Holder");
    let mut ts = wire_field("ts", &ident("Timer"));
    let identity = format!("{holder}/ts");
    ts["identity"] = json!(identity);
    ts["origin"] = origin(&identity);
    ts["multiplicity"] = json!({"lower": 0, "upper": 2, "ordered": true, "unique": false});
    let types = document["types"].as_array_mut().expect("types");
    types.push(wire_type(&holder, "value_record", json!({"fields": [ts]})));
    types[1]["fields"]
        .as_array_mut()
        .expect("Call fields")
        .push(wire_field("h", &holder));
    serde_json::to_vec(&document).expect("JSON")
}

/// Part of FR-230-AC-21 (TC-685), "Quantified reads" over a collection
/// below a record field (SR-1659002 FND-003): with `h = Holder { ts:
/// [Timer { n: 1 }, Timer { n: 2 }] }`, `work`'s guard `exists(e in
/// self.h.ts: e.n = 2 and e.x >= 1 ms)` is the one part of slot 1, whose
/// element has `n = 2`: false at 0 and true after a delay of 2.
#[trace("TC-685")]
#[test]
fn a_quantified_guard_reads_a_collection_below_a_record_field() {
    let timer =
        |n: &str| json!({"record": {"type": ident("Timer"), "fields": {"n": {"integer": n}}}});
    let world = world_holding(
        timer_holder_document(),
        "",
        &only(
            &["work"],
            "pre Near using v on M::Call::work { exists(e in self.h.ts: e.n = 2 and e.x >= 1 ms) }\n",
        ),
        json!({
            "t": timer("0"),
            "h": {"record": {"type": ident("Holder"), "fields": {
                "ts": {"sequence": [timer("1"), timer("2")]}
            }}}
        }),
    );
    let held = system(&world);
    let timed = TimedSystem::new(&held, "M").expect("a timed subject");
    let [initial] = &timed.initial()[..] else {
        panic!("one initial state");
    };
    let read = |state: &TimedState| {
        timed
            .guarded_steps(state.discrete())
            .expect("the state expands")
            .iter()
            .filter(|step| step.operation().contains("Call/work\""))
            .flat_map(|step| step.parts().to_vec())
            .map(|part| {
                part.iter()
                    .map(|constraint| {
                        (
                            timed.places()[constraint.place()].slots().to_vec(),
                            constraint.holds(state.clocks()),
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    };
    let later = timed
        .delay(initial, &exact(2, 1))
        .expect("decided")
        .expect("no invariant bounds the delay");
    assert_eq!(read(initial), vec![vec![(vec![1], false)]]);
    assert_eq!(read(&later), vec![vec![(vec![1], true)]]);
}

/// Part of FR-230-AC-25 (TC-685): with `Timer.x` typed by the alias
/// `Timeout` of `Clock`, whose constraint `max` `"3"` bounds the clocks it
/// types, and `ts = [Timer { n: 1 }]` of at most 2 elements, the time
/// invariant bounds `t.x` and the live slot 0 of `ts.[slot].x` by 3, and not
/// the slot 1 the sequence does not hold: a delay of 3 is admissible and one
/// of 4 is not, with the constraint's identity named as the broken bound.
#[trace("TC-685")]
#[test]
fn a_max_constraint_on_a_clock_alias_bounds_the_live_places() {
    let mut document: Json =
        serde_json::from_slice(&timer_sequence_document(Some(2))).expect("JSON");
    let timeout = ident("Timeout");
    let types = document["types"].as_array_mut().expect("types");
    for definition in types.iter_mut() {
        if definition["identity"] == ident("Timer") {
            for member in definition["fields"].as_array_mut().expect("Timer fields") {
                if member["name"] == "x" {
                    member["typeRef"] = json!(timeout);
                }
            }
        }
    }
    let mut alias = wire_type(
        &timeout,
        "unused",
        json!({"target": "ix://quire/native/Clock", "constraints": [{
            "identity": format!("{timeout}/max"), "keyword": "max",
            "operands": {"value": "3"}, "appliesTo": timeout,
            "diagnosticCode": "bound.max", "origin": origin(&timeout),
        }]}),
    );
    alias["kind"] = json!("alias");
    types.push(alias);
    let timer =
        |n: &str| json!({"record": {"type": ident("Timer"), "fields": {"n": {"integer": n}}}});
    let world = world_holding(
        serde_json::to_vec(&document).expect("JSON"),
        "",
        &only(
            &["send"],
            "post Send using v on M::Call::send { self.phase = 1 }\n",
        ),
        json!({"t": timer("0"), "ts": {"sequence": [timer("1")]}}),
    );
    let held = system(&world);
    let timed = TimedSystem::new(&held, "M").expect("a timed subject");
    let [initial] = &timed.initial()[..] else {
        panic!("one initial state");
    };
    let bounded: Vec<(String, Vec<usize>)> = timed
        .invariants(initial.discrete())
        .expect("the invariants read")
        .iter()
        .map(|bound| {
            let place = &timed.places()[bound.place()];
            (
                timed
                    .model()
                    .clock(place.clock())
                    .expect("a clock")
                    .name()
                    .to_owned(),
                place.slots().to_vec(),
            )
        })
        .collect();
    assert_eq!(
        bounded,
        [
            ("t.x".to_owned(), vec![]),
            ("ts.[slot].x".to_owned(), vec![0]),
        ]
    );
    assert!(timed
        .delay(initial, &exact(3, 1))
        .expect("decided")
        .is_some());
    assert!(timed
        .delay(initial, &exact(4, 1))
        .expect("decided")
        .is_none());
    let past = TimedState::new(
        initial.discrete().clone(),
        initial.clocks().delayed(&exact(4, 1)),
    );
    assert_eq!(
        timed.broken_invariant(&past).expect("decided"),
        Some(format!("{timeout}/max"))
    );
}

/// Part of FR-230-AC-21 (TC-685), SR-1676001 FND-002: the alternatives of
/// a `forall` family whose body has two parts at each held slot are a
/// product over the held slots, charged against the expansion's
/// `max_candidates`. Under `max_candidates` 12, `forall(e in self.ts: e.n
/// = 2 implies e.x >= 1 ms)` over two `Timer { n: 1 }` elements (2 + 4
/// alternatives charged) expands, and over four (2 + 4 + 8 + 16) stops
/// `resource_exhausted` naming `max_candidates` instead of listing them.
#[trace("TC-685")]
#[test]
fn a_family_product_is_charged_against_max_candidates() {
    let timer =
        |n: &str| json!({"record": {"type": ident("Timer"), "fields": {"n": {"integer": n}}}});
    let expanded = |held: usize| {
        let world = world_holding(
            timer_sequence_document(None),
            "",
            &only(
                &["work"],
                "pre Near using v on M::Call::work { forall(e in self.ts: e.n = 2 implies e.x >= 1 ms) }\n",
            ),
            json!({"t": timer("0"), "ts": {"sequence": vec![timer("1"); held]}}),
        );
        let limited = system_limited(&world, &["c"], 12);
        let timed = TimedSystem::new(&limited, "M").expect("a timed subject");
        let [initial] = &timed.initial()[..] else {
            panic!("one initial state");
        };
        timed
            .guarded_steps(initial.discrete())
            .map(|steps| steps.len())
    };
    assert!(expanded(2).is_ok(), "{:?}", expanded(2));
    let Err(TimedStop::Expansion(stop)) = expanded(4) else {
        panic!("the product stops: {:?}", expanded(4));
    };
    assert_eq!(
        stop.limit,
        Some(qsl_eval::simulation::ExpansionLimit::MaxCandidates)
    );
}

/// `timer_document` with a record `Holder { ts: Timer [0, 2] }` and a field
/// `hs: Sequence<Holder>` with no declared maximum.
fn holders_document() -> Vec<u8> {
    let mut document: Json = serde_json::from_slice(&timer_document()).expect("JSON");
    let holder = ident("Holder");
    let mut ts = wire_field("ts", &ident("Timer"));
    let identity = format!("{holder}/ts");
    ts["identity"] = json!(identity);
    ts["origin"] = origin(&identity);
    ts["multiplicity"] = json!({"lower": 0, "upper": 2, "ordered": true, "unique": false});
    let types = document["types"].as_array_mut().expect("types");
    types.push(wire_type(&holder, "value_record", json!({"fields": [ts]})));
    let mut hs = wire_field("hs", &holder);
    hs["multiplicity"] = json!({"lower": 0, "ordered": true, "unique": false});
    types[1]["fields"]
        .as_array_mut()
        .expect("Call fields")
        .push(hs);
    serde_json::to_vec(&document).expect("JSON")
}

/// Part of FR-230-AC-19 (TC-685), SR-1676001 round 2 CR FND-001: over `hs:
/// Sequence<Holder>` with no declared maximum, `Holder { ts: Timer [0, 2]
/// }`, an element holds as few as no clocks, so `hs` lists `max_clocks`
/// slots and every state within the bound has a place for each live clock.
/// With `max_clocks` 11, three holders of one `Timer` each (9 live clocks:
/// `t.x`, `t.y`, `x` and two for each `Timer`) and four (11) step with every
/// live clock placed, and five (13) stop `MaxClocks`; with `max_clocks` 15
/// six holders (15) step.
#[trace("TC-685")]
#[test]
fn an_unbounded_collection_of_records_places_every_live_clock() {
    let timer =
        |n: &str| json!({"record": {"type": ident("Timer"), "fields": {"n": {"integer": n}}}});
    let holder =
        json!({"record": {"type": ident("Holder"), "fields": {"ts": {"sequence": [timer("1")]}}}});
    for (holders, max_clocks, steps) in
        [(3, 11, true), (4, 11, true), (5, 11, false), (6, 15, true)]
    {
        let world = world_holding(
            holders_document(),
            "",
            &only(
                &["send"],
                "post Send using v on M::Call::send { self.phase = 1 }\n",
            ),
            json!({"t": timer("0"), "hs": {"sequence": vec![holder.clone(); holders]}}),
        );
        let held = system(&world);
        let timed = TimedSystem::with_max_clocks(
            &held,
            "M",
            std::num::NonZeroU64::new(max_clocks).expect("positive"),
        )
        .expect("a timed subject");
        let [initial] = &timed.initial()[..] else {
            panic!("one initial state");
        };
        let live = timed.live_clocks(initial.discrete());
        assert_eq!(live, 3 + 2 * holders as u64);
        let placed = timed
            .places()
            .iter()
            .filter(|place| timed.live(place, initial.discrete()))
            .count() as u64;
        let stepped = timed.steps(initial);
        if steps {
            assert_eq!(placed, live, "{holders} holders at {max_clocks}");
            assert!(
                !stepped.expect("the state steps").is_empty(),
                "{holders} holders at {max_clocks}"
            );
        } else {
            assert_eq!(stepped.err(), Some(TimedStop::MaxClocks { max_clocks }));
        }
    }
}

/// FR-120 "Successors" (TC-472), STD-493 item 21's ruling: a field a frame
/// grants takes every value of its domain, a record or tuple of finite data
/// components included, and a field with no finite domain takes the value a
/// postcondition conjunct `self.f = E` computes, never silently its
/// pre-state value. From `t = Timer { n: 0 }`, `send` (frame `phase`,
/// `inflight`, `late`, `x`, `t`) with the postcondition `self.phase = 1`
/// has 16 successors, one for each `inflight`, `late` and `t.n` (`Phase`
/// holds 4 values), and with `self.t.n = pre(self.t.n)` too has 4. A field
/// `h: Holder { k: Integer }`, whose `k` has no finite domain, granted by
/// `send` and left free stops `unsupported_construct`/`write-not-computed`,
/// and `self.h = M::Holder { k: pre(self.h).k + 1 }` writes `k = 1`; so does
/// a free `count: Integer`, and `self.count = pre(self.count) + 2` writes
/// 5 from 3. A field `ds:
/// Sequence<Quantity<ms>>` with no declared maximum: `self.ds =
/// sequence[5 ms, 11 ms]` writes it (it gave no successor before), `self.ds
/// = self.ds` (no function of the pre-state) stops `write-not-computed`.
#[trace("TC-472")]
#[test]
fn a_granted_field_ranges_over_its_finite_domain_or_stops() {
    use qsl_eval::simulation::TransitionSystem;

    let timer = json!({"record": {"type": ident("Timer"), "fields": {"n": {"integer": "0"}}}});
    let steps = |post: &str| {
        let world = world_holding(
            timer_document(),
            "",
            &only(
                &["send"],
                &format!("post Send using v on M::Call::send {{ {post} }}\n"),
            ),
            json!({"t": timer.clone()}),
        );
        let held = system(&world);
        let initial = held.initial().remove(0);
        held.successors(&initial)
            .map(|expansion| expansion.successors.len())
    };
    assert_eq!(steps("self.phase = 1"), Ok(16));
    assert_eq!(steps("self.phase = 1 and self.t.n = pre(self.t.n)"), Ok(4));

    // A record with an unbounded component, and a quantity sequence.
    let document = {
        let mut document: Json = serde_json::from_slice(&timer_document()).expect("JSON");
        let holder = ident("Holder");
        let mut k = wire_field("k", "ix://quire/native/Integer");
        let identity = format!("{holder}/k");
        k["identity"] = json!(identity);
        k["origin"] = origin(&identity);
        let types = document["types"].as_array_mut().expect("types");
        types.push(wire_type(&holder, "value_record", json!({"fields": [k]})));
        let mut ds = wire_field("ds", "ix://quire/unit/ms");
        ds["multiplicity"] = json!({"lower": 0, "ordered": true, "unique": false});
        let fields = types[1]["fields"].as_array_mut().expect("Call fields");
        fields.push(wire_field("h", &holder));
        fields.push(ds);
        fields.push(wire_field("count", "ix://quire/native/Integer"));
        for operation in types[1]["operations"].as_array_mut().expect("operations") {
            if operation["name"] == "send" {
                let modifies = operation["frame"]["modifies"]
                    .as_array_mut()
                    .expect("a frame");
                modifies.push(json!(ident("Call/h")));
                modifies.push(json!(ident("Call/ds")));
                modifies.push(json!(ident("Call/count")));
            }
        }
        serde_json::to_vec(&document).expect("JSON")
    };
    let holder =
        |k: &str| json!({"record": {"type": ident("Holder"), "fields": {"k": {"integer": k}}}});
    let written = |post: &str| {
        let world = world_holding(
            document.clone(),
            "",
            &only(&["send"], &format!("post Send using v on M::Call::send {{ self.phase = 1 and self.inflight and self.late and self.t.n = pre(self.t.n) and {post} }}\n")),
            json!({"t": timer.clone(), "h": holder("0"), "ds": {"sequence": []},
                "count": {"integer": "3"}}),
        );
        let held = system(&world);
        let initial = held.initial().remove(0);
        held.successors(&initial).map(|expansion| {
            expansion
                .successors
                .into_iter()
                .map(|(_, post)| {
                    let object = post
                        .state()
                        .populations
                        .values()
                        .find_map(|objects| objects.get("c"))
                        .expect("c")
                        .clone();
                    let field = |name: &str| {
                        object
                            .fields
                            .get(&DeclarationKey {
                                package: PACKAGE.to_owned(),
                                node: ident(&format!("Call/{name}")),
                            })
                            .map(|value| format!("{value:?}"))
                            .unwrap_or_default()
                    };
                    (field("h"), field("ds"), field("count"))
                })
                .collect::<Vec<_>>()
        })
    };
    // FR-120-AC-25: each of these fields is an unbounded root, so an
    // analysis settles `NotSimulated::RequiresBound` before expanding.
    {
        let world = world_holding(
            document.clone(),
            "",
            &only(&["send"], ""),
            json!({"t": timer.clone(), "h": holder("0"), "ds": {"sequence": []},
                "count": {"integer": "3"}}),
        );
        let held = system(&world);
        let Err(NotSimulated::RequiresBound(bound)) = check_model_bounds(&held, BOUND_POSITIONS)
        else {
            panic!("h, ds and count require a bound");
        };
        assert_eq!(format!("{:?}", bound.domains).matches("Node {").count(), 3);
    }
    let not_computed = Err(qsl_eval::simulation::ExpansionStop {
        cause: CatalogCode::new("unsupported_construct", "write-not-computed"),
        limit: None,
    });
    assert_eq!(
        written("self.ds = sequence[5 ms] and self.count = 0").map(|posts| posts.len()),
        not_computed.map(|_: ()| 0)
    );
    assert_eq!(
        written("self.h = M::Holder { k: 0 } and self.ds = sequence[5 ms]")
            .map(|posts| posts.len()),
        not_computed.map(|_: ()| 0)
    );
    let posts = written(
        "self.h = M::Holder { k: pre(self.h).k + 1 } and self.ds = sequence[5 ms, 11 ms] \
         and self.count = pre(self.count) + 2",
    )
    .expect("the state expands");
    let [(h, ds, count)] = &posts[..] else {
        panic!("one successor: {posts:?}");
    };
    assert!(h.contains("Integer(1)"), "{h}");
    assert!(count.contains("Integer(5)"), "{count}");
    assert!(
        ds.contains("Integer(5)") && ds.contains("Integer(11)"),
        "{ds}"
    );
    assert_eq!(
        written("self.h = M::Holder { k: 0 } and self.ds = self.ds and self.count = 0")
            .map(|posts| posts.len()),
        not_computed.map(|_: ()| 0)
    );
}

/// TC-1270: a scalar type `Int[0, max]` named `name`.
fn int_type(name: &str, max: u64) -> Json {
    let identity = ident(name);
    let constraint = |keyword: &str, value: String| {
        json!({
            "identity": format!("{identity}/constraints/{keyword}"), "keyword": keyword,
            "operands": {"value": value}, "appliesTo": "ix://quire/native/Integer",
            "diagnosticCode": format!("bound.{keyword}"), "origin": origin(&identity),
        })
    };
    wire_type(
        &identity,
        "value_type",
        json!({"scalar": "integer",
            "constraints": [constraint("min", "0".to_owned()), constraint("max", max.to_string())]}),
    )
}

/// TC-1270: a record type `name` with `fields` (name, type, optional).
fn record_type(name: &str, fields: &[(&str, &str, bool)]) -> Json {
    let identity = ident(name);
    let fields: Vec<Json> = fields
        .iter()
        .map(|(field, type_ref, optional)| {
            let mut member = wire_field(field, type_ref);
            let member_identity = format!("{identity}/{field}");
            member["identity"] = json!(member_identity);
            member["origin"] = origin(&member_identity);
            if *optional {
                member["presence"] = json!("optional");
            }
            member
        })
        .collect();
    wire_type(&identity, "value_record", json!({"fields": fields}))
}

/// TC-1270's fixture over `timer_document`: the types `types`, the fields
/// `fields` on `Call` (name, type, optional, multiplicity), `work` granting
/// `work` and `tick` granting `tick`, an enum `Color` and an `Int[0, 2]`
/// and `Int[0, 1]` when `colors`.
fn cells_document(
    types: Vec<Json>,
    fields: Vec<(&str, String, Option<Json>)>,
    work: &[&str],
    tick: &[&str],
) -> Vec<u8> {
    let mut document: Json = serde_json::from_slice(&timer_document()).expect("JSON");
    let color = ident("Color");
    let case = |name: &str, line: u64| {
        json!({"identity": ident(&format!("variant/Color-{name}")), "name": name,
            "origin": {"source": {"sourceIdentity": ident("spec"), "path": "spec.qspec",
                "startLine": line, "startColumn": 3}}})
    };
    let all = document["types"].as_array_mut().expect("types");
    all.push(int_type("Int2", 2));
    all.push(int_type("Int1", 1));
    all.push(wire_type(
        &color,
        "mode",
        json!({"variants": [case("red", 10), case("blue", 11), case("green", 12)]}),
    ));
    all.extend(types);
    for (name, type_ref, multiplicity) in fields {
        let mut member = wire_field(name, &type_ref);
        match multiplicity {
            Some(Json::String(presence)) => member["presence"] = json!(presence),
            Some(multiplicity) => member["multiplicity"] = multiplicity,
            None => {}
        }
        all[1]["fields"]
            .as_array_mut()
            .expect("Call fields")
            .push(member);
    }
    for operation in all[1]["operations"].as_array_mut().expect("operations") {
        // No operation creates a `Call`, so only the fields a frame names
        // are granted.
        operation["frame"]["creates"] = json!([]);
        let granted: &[&str] = match operation["name"].as_str() {
            Some("work") => work,
            Some("tick") => tick,
            _ => continue,
        };
        operation["frame"]["modifies"] = json!(granted
            .iter()
            .map(|field| ident(&format!("Call/{field}")))
            .collect::<Vec<_>>());
    }
    let constructs = document["constructs"].as_array_mut().expect("constructs");
    constructs.push(json!({
        "kind": {"module": PACKAGE, "name": "mode"}, "moduleVersion": "1.0.0",
        "manifestDigest": PLACEHOLDER,
        "construct": {"identity": "none", "shape": "enumeration",
            "members": {"variants": "required"}, "meaning": meaning::VARIANT_TYPE}}));
    serde_json::to_vec(&document).expect("JSON")
}

/// The successors `operation` has from the initial state of `world`, each
/// as the debug text of `field`'s value.
fn granted_values(world: &World, operation: &str, field: &str) -> Vec<String> {
    use qsl_eval::simulation::TransitionSystem;
    let held = system(world);
    let initial = held.initial().remove(0);
    let key = DeclarationKey {
        package: PACKAGE.to_owned(),
        node: ident(&format!("Call/{field}")),
    };
    held.successors(&initial)
        .expect("the state expands")
        .successors
        .into_iter()
        .filter(|(transition, _)| format!("{transition:?}").contains(&format!("Call/{operation}")))
        .map(|(_, post)| {
            post.state()
                .populations
                .values()
                .find_map(|objects| objects.get("c"))
                .and_then(|object| object.fields.get(&key))
                .map(|value| format!("{value:?}"))
                .unwrap_or_else(|| "absent".to_owned())
        })
        .collect()
}

/// FR-120-AC-22, FR-120-AC-23 and FR-120-AC-24 (TC-1270): over `Call`, a
/// granted `cfg: Cfg { on: Boolean, n: Int[0, 2] }` has 6 values, `on`
/// most significant; `w: Wrap { c: Cfg, o?: Boolean }` 18; `r1: R1 { c:
/// Cfg, a: Boolean }` 12; a granted `col: Color` 3 in declaration order;
/// `pick: Pick { col: Color, on: Boolean }` 6 and `note: Note { note?:
/// Boolean, n: Int[0, 1] }` 6, `note` absent first; `t: Timer { x, y:
/// Clock; n: Phase }` 4, its clocks no factor; `sw: Stopwatch { x, y:
/// Clock }` one value; `mx: Mix { t: Timer, b: Boolean }` 8; `oc?: Cfg`
/// and `sq: Cfg [0, 1]` 7 each. No such field
/// is an unbounded root, so the model passes FR-120's requires-bound check.
/// Not covered: the
/// tuple `Pr`, which a domain package cannot declare.
#[trace("TC-1270")]
#[test]
fn a_record_of_finite_components_is_a_finite_domain() {
    let int2 = ident("Int2");
    let int1 = ident("Int1");
    let color = ident("Color");
    let types = vec![
        record_type(
            "Cfg",
            &[
                ("on", "ix://quire/native/Boolean", false),
                ("n", &int2, false),
            ],
        ),
        record_type(
            "Wrap",
            &[
                ("c", &ident("Cfg"), false),
                ("o", "ix://quire/native/Boolean", true),
            ],
        ),
        record_type(
            "R1",
            &[
                ("c", &ident("Cfg"), false),
                ("a", "ix://quire/native/Boolean", false),
            ],
        ),
        record_type(
            "Pick",
            &[
                ("col", &color, false),
                ("on", "ix://quire/native/Boolean", false),
            ],
        ),
        record_type(
            "Note",
            &[
                ("note", "ix://quire/native/Boolean", true),
                ("n", &int1, false),
            ],
        ),
        record_type(
            "Stopwatch",
            &[
                ("x", "ix://quire/native/Clock", false),
                ("y", "ix://quire/native/Clock", false),
            ],
        ),
        record_type(
            "Mix",
            &[
                ("t", &ident("Timer"), false),
                ("b", "ix://quire/native/Boolean", false),
            ],
        ),
    ];
    let cfg = |on: bool, n: u8| {
        json!({"record": {"type": ident("Cfg"), "fields": {
        "on": {"boolean": on}, "n": {"integer": n.to_string()}}}})
    };
    let timer = json!({"record": {"type": ident("Timer"), "fields": {"n": {"integer": "0"}}}});
    let initial = json!({
        "t": timer.clone(),
        "cfg": cfg(false, 0),
        "w": {"record": {"type": ident("Wrap"), "fields": {"c": cfg(false, 0), "o": {"absent": {}}}}},
        "r1": {"record": {"type": ident("R1"), "fields": {"c": cfg(false, 0), "a": {"boolean": false}}}},
        "col": {"variant": {"type": color, "case": "red"}},
        "pick": {"record": {"type": ident("Pick"), "fields": {
            "col": {"variant": {"type": color, "case": "red"}}, "on": {"boolean": false}}}},
        "note": {"record": {"type": ident("Note"), "fields": {"note": {"absent": {}}, "n": {"integer": "0"}}}},
        "sw": {"record": {"type": ident("Stopwatch"), "fields": {}}},
        "mx": {"record": {"type": ident("Mix"), "fields": {"t": timer, "b": {"boolean": false}}}},
        "sq": {"sequence": []},
        "oc": {"absent": {}},
    });
    let fields = vec![
        ("cfg", ident("Cfg"), None),
        ("w", ident("Wrap"), None),
        ("r1", ident("R1"), None),
        ("col", color.clone(), None),
        ("pick", ident("Pick"), None),
        ("note", ident("Note"), None),
        ("sw", ident("Stopwatch"), None),
        ("mx", ident("Mix"), None),
        ("oc", ident("Cfg"), Some(json!("optional"))),
        (
            "sq",
            ident("Cfg"),
            Some(json!({"lower": 0, "upper": 1, "ordered": true, "unique": false})),
        ),
    ];
    let counts = |granted: &str| {
        let world = world_holding(
            cells_document(types.clone(), fields.clone(), &[granted], &[]),
            "",
            &only(&["work"], ""),
            initial.clone(),
        );
        let held = system(&world);
        check_model_bounds(&held, BOUND_POSITIONS)
            .unwrap_or_else(|refusal| panic!("{granted} is bounded: {refusal:?}"));
        granted_values(&world, "work", granted)
    };
    let values = counts("cfg");
    assert_eq!(values.len(), 6, "{values:#?}");
    // `on` most significant: false for the first three, `n` ascending.
    assert!(
        values[..3]
            .iter()
            .all(|value| value.contains("Boolean(false)")),
        "{values:#?}"
    );
    assert!(values[3..]
        .iter()
        .all(|value| value.contains("Boolean(true)")));
    assert_eq!(counts("w").len(), 18);
    assert_eq!(counts("r1").len(), 12);
    let colors = counts("col");
    assert_eq!(colors.len(), 3);
    assert!(
        colors[0].contains("rank: 0") && colors[2].contains("rank: 2"),
        "{colors:?}"
    );
    assert_eq!(counts("pick").len(), 6);
    let notes = counts("note");
    assert_eq!(notes.len(), 6);
    assert!(notes[0].contains("Absent"), "{notes:?}");
    assert_eq!(counts("t").len(), 4);
    assert_eq!(counts("sw").len(), 1);
    assert_eq!(counts("mx").len(), 8);
    assert_eq!(counts("oc").len(), 7);
    assert_eq!(counts("sq").len(), 7);
}

/// FR-120-AC-25 (TC-1270): a granted record with an unbounded non-clock
/// component, `op: Open { n: Integer, b: Boolean }`, `tm: Tm { x: Clock, k:
/// Integer }` (its clock does not bound it) and `wr: Wrap2 { o: Open }`, is
/// an unbounded root: `explore_model` returns `NotSimulated::RequiresBound`
/// naming one domain, before any state is expanded. Not covered: the union
/// `U { a, b(Boolean) }`, whose payload case QSL's package intake refuses
/// (`Variant.payloadType`).
#[trace("TC-1270", "FR-120-AC-25")]
#[test]
fn a_record_with_an_unbounded_component_requires_a_bound() {
    let types = vec![
        record_type(
            "Open",
            &[
                ("n", "ix://quire/native/Integer", false),
                ("b", "ix://quire/native/Boolean", false),
            ],
        ),
        record_type(
            "Tm",
            &[
                ("x", "ix://quire/native/Clock", false),
                ("k", "ix://quire/native/Integer", false),
            ],
        ),
        record_type("Wrap2", &[("o", &ident("Open"), false)]),
    ];
    let open = json!({"record": {"type": ident("Open"), "fields": {
        "n": {"integer": "0"}, "b": {"boolean": false}}}});
    let initial = json!({
        "t": {"record": {"type": ident("Timer"), "fields": {"n": {"integer": "0"}}}},
        "op": open.clone(),
        "tm": {"record": {"type": ident("Tm"), "fields": {"k": {"integer": "0"}}}},
        "wr": {"record": {"type": ident("Wrap2"), "fields": {"o": open}}},
    });
    for granted in ["op", "tm", "wr"] {
        let world = world_holding(
            cells_document(
                types.clone(),
                vec![
                    ("op", ident("Open"), None),
                    ("tm", ident("Tm"), None),
                    ("wr", ident("Wrap2"), None),
                ],
                &[granted],
                &[],
            ),
            "",
            &only(&["work"], ""),
            initial.clone(),
        );
        let held = system(&world);
        let explored = qsl_eval::simulation::model::explore_model(
            &held,
            BOUND_POSITIONS,
            qsl_eval::simulation::Limits::default(),
            || false,
        );
        let Err(NotSimulated::RequiresBound(bound)) = explored else {
            panic!("{granted} requires a bound: {explored:?}");
        };
        assert_eq!(
            format!("{:?}", bound.domains).matches("Node {").count(),
            1,
            "{granted}: {:?}",
            bound.domains
        );
    }
}

/// Part of FR-120-AC-23 (TC-1270): an operation parameter `c: Color` and a
/// result of type `Color` (`red`, `blue`, `green`) are finite domains, so
/// the model passes FR-120's requires-bound check, and `work` (no clause,
/// granting nothing) has one successor for each parameter value, its result
/// the first of its domain in canonical order.
#[trace("TC-1270")]
#[test]
fn an_enum_parameter_and_result_are_finite() {
    use qsl_eval::simulation::TransitionSystem;

    let mut document: Json =
        serde_json::from_slice(&cells_document(vec![], vec![], &[], &[])).expect("JSON");
    let color = ident("Color");
    for operation in document["types"][1]["operations"]
        .as_array_mut()
        .expect("operations")
    {
        if operation["name"] == "work" {
            let identity = ident("Call/work/c");
            operation["params"] = json!([{
                "identity": identity, "origin": origin(&identity), "name": "c",
                "typeRef": color, "presence": "required", "nullable": false,
                "defaultKind": "none",
                "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            }]);
            operation["returns"] = json!({"typeRef": color, "nullable": false,
                "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true}});
        }
    }
    let world = world_holding(
        serde_json::to_vec(&document).expect("JSON"),
        "",
        &only(&["work"], ""),
        json!({"t": {"record": {"type": ident("Timer"), "fields": {"n": {"integer": "0"}}}}}),
    );
    let held = system(&world);
    check_model_bounds(&held, BOUND_POSITIONS)
        .unwrap_or_else(|refusal| panic!("an enum is bounded: {refusal:?}"));
    let initial = held.initial().remove(0);
    let work = held
        .successors(&initial)
        .expect("the state expands")
        .successors
        .iter()
        .filter(|(transition, _)| format!("{transition:?}").contains("Call/work"))
        .count();
    assert_eq!(work, 3);
}

/// [`timer_document`] with `ds: Sequence<Quantity<ms>>` on `Call`, granted
/// by `send`, and a `max` `"3"` on `ms` on `Call` (QSpec FR-154 "Unit
/// bounds").
fn bounded_quantity_document() -> Vec<u8> {
    let mut document: Json = serde_json::from_slice(&timer_document()).expect("JSON");
    let call = ident("Call");
    let mut ds = wire_field("ds", "ix://quire/unit/ms");
    ds["multiplicity"] = json!({"lower": 0, "ordered": true, "unique": false});
    let types = document["types"].as_array_mut().expect("types");
    types[1]["fields"]
        .as_array_mut()
        .expect("Call fields")
        .push(ds);
    let identity = format!("{call}/constraints/max");
    types[1]["constraints"] = json!([{
        "identity": identity, "keyword": "max", "operands": {"value": "3"},
        "appliesTo": "ix://quire/unit/ms", "diagnosticCode": "bound.max",
        "origin": origin(&identity),
    }]);
    for operation in types[1]["operations"].as_array_mut().expect("operations") {
        if operation["name"] == "send" {
            operation["frame"]["modifies"]
                .as_array_mut()
                .expect("a frame")
                .push(json!(ident("Call/ds")));
        }
    }
    serde_json::to_vec(&document).expect("JSON")
}

/// A Quantity object of `ms` with `magnitude`.
fn ms(magnitude: Json) -> Json {
    json!({"kind": "quantity", "unit": "ms", "magnitude": magnitude})
}

/// A Rational magnitude `numerator`/`denominator`.
fn ratio(numerator: &str, denominator: &str) -> Json {
    json!({"kind": "rational", "numerator": numerator, "denominator": denominator})
}

/// The refusal admitting `world`'s initial snapshot gives, as its debug text.
fn initial_refusal(world: &World) -> String {
    let refused = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = system(world);
    }));
    let Err(panicked) = refused else {
        panic!("the initial snapshot refuses");
    };
    panicked
        .downcast_ref::<String>()
        .cloned()
        .unwrap_or_default()
}

/// FR-106 "A Quantity value" (part of FR-106-AC-59, TC-1171) and part of
/// FR-230-AC-26 (TC-685) with QSpec FR-154-AC-29: over `ds:
/// Sequence<Quantity<ms>>` and a `max` `"3"` on `ms`, an initial `ds` of
/// `[3/6 ms, 1.5 ms (Decimal)]` admits as `1/2` and `3/2` ms; `unit` `"s"`
/// refuses `wrong-value-kind` naming `unit`, a `denominator` `"0"`
/// `invalid-value` naming `denominator` at the path `0`, `magnitude`, and a
/// Decimal `scale` `"-1"` `invalid-value` naming `scale`; an initial `4 ms`
/// refuses `invalid-value` naming the constraint, `max` and `3` at element
/// 0. From `[1/2 ms]`, `send` writing `self.ds = sequence[5 ms]` steps to a
/// state whose expansion finds the bound's `InvariantViolated`, and one
/// writing `sequence[2 ms]` finds nothing.
#[trace("TC-685")]
#[test]
fn a_unit_bound_is_checked_on_supplied_and_stepped_quantities() {
    use qsl_eval::simulation::model::ModelFinding;
    use qsl_eval::simulation::TransitionSystem;

    let timer = json!({"record": {"type": ident("Timer"), "fields": {"n": {"integer": "0"}}}});
    let world_with = |ds: Json, post: &str| {
        world_holding(
            bounded_quantity_document(),
            "",
            &only(
                &["send"],
                &format!(
                    "post Send using v on M::Call::send {{ self.phase = 1 and self.inflight \
                     and self.late and self.t.n = pre(self.t.n) and {post} }}\n"
                ),
            ),
            json!({"t": timer.clone(), "ds": {"sequence": ds}}),
        )
    };
    let decimal = json!({"kind": "decimal", "coefficient": "15", "scale": "1"});
    let world = world_with(
        json!([ms(ratio("3", "6")), ms(decimal)]),
        "self.ds = pre(self.ds)",
    );
    let held = system(&world);
    let initial = held.initial().remove(0);
    let ds = format!("{:?}", initial.state().populations);
    assert!(
        ds.contains("numerator: Integer(1), denominator: Integer(2)")
            && ds.contains("numerator: Integer(3), denominator: Integer(2)"),
        "{ds}"
    );

    for (ds, expected) in [
        (
            json!([{"kind": "quantity", "unit": "s", "magnitude": ratio("1", "2")}]),
            ["wrong-value-kind", "\"member\": \"unit\""],
        ),
        (
            json!([ms(ratio("1", "0"))]),
            ["invalid-value", "\"member\": \"denominator\""],
        ),
        (
            json!([ms(
                json!({"kind": "decimal", "coefficient": "15", "scale": "-1"})
            )]),
            ["invalid-value", "\"member\": \"scale\""],
        ),
        (
            json!([ms(ratio("4", "1"))]),
            ["invalid-value", "\"keyword\": \"max\""],
        ),
    ] {
        let refusal = initial_refusal(&world_with(ds.clone(), "self.ds = pre(self.ds)"));
        for part in expected {
            assert!(refusal.contains(part), "{ds}: {refusal}");
        }
    }
    let refusal = initial_refusal(&world_with(
        json!([ms(ratio("1", "0"))]),
        "self.ds = pre(self.ds)",
    ));
    assert!(
        refusal.contains("Index(0), Field(\"magnitude\")"),
        "{refusal}"
    );

    let findings = |post: &str| {
        let world = world_with(json!([ms(ratio("1", "2"))]), post);
        let held = system(&world);
        let initial = held.initial().remove(0);
        let expansion = held.successors(&initial).expect("the state expands");
        let [(_, next)] = &expansion.successors[..] else {
            panic!("one successor: {}", expansion.successors.len());
        };
        held.successors(next).expect("the step expands").findings
    };
    let violated = findings("self.ds = sequence[5 ms]");
    assert!(
        matches!(
            &violated[..],
            [ModelFinding::InvariantViolated { clause, .. }] if clause.ends_with("Call/constraints/max")
        ),
        "{violated:?}"
    );
    assert_eq!(findings("self.ds = sequence[2 ms]"), Vec::new());
}

/// A multiplicity `{lower, upper, ordered, unique}`; no `upper` when `max`
/// is `None`.
fn shape(max: Option<u64>, ordered: bool, unique: bool) -> Json {
    match max {
        Some(max) => json!({"lower": 0, "upper": max, "ordered": ordered, "unique": unique}),
        None => json!({"lower": 0, "ordered": ordered, "unique": unique}),
    }
}

/// The empty value of a collection of `shape`.
fn empty(ordered: bool, unique: bool) -> Json {
    match (ordered, unique) {
        (true, false) => json!({"sequence": []}),
        (false, true) => json!({"set": []}),
        _ => json!({"bag": []}),
    }
}

/// FR-120-AC-26 (TC-1270): a granted sequence, set or bag with a declared
/// maximum over a finite element is a finite domain, clock components
/// adding no factor: `Sequence<Cfg>[0, 2]` 43 (`[]`, then `[c0]` to `[c5]`,
/// then `[c0, c0]`, `[c0, c1]`, ...), `Sequence<Timer>[0, 2]` 21 and `[0,
/// 3]` 85 (`Timer`'s `n` is `Int[0, 3]`), `Sequence<Tick>[0, 3]` 4,
/// `Set<Cfg>[0, 1]` 7 and `[0, 2]` 22, ordered by count and then ascending
/// `Cfg`, `Bag<Cfg>[0, 2]` 28, `Set<Timer>[0, 2]` 11 and `Bag<Timer>[0, 2]`
/// 15. `Sequence<Open>[0, 1]` and `Sequence<Cfg>`, `Set<Cfg>` with no
/// declared maximum are unbounded roots. Not covered: `Sequence<Clock>`,
/// `Sequence<Option<Clock>>` and `Set<Clock>`, whose fields hold no data in
/// the checked package, a `Reference<Clock>` field and a `Map`, which a
/// domain package here cannot declare.
#[trace("TC-1270")]
#[test]
fn a_bounded_collection_of_finite_elements_is_a_finite_domain() {
    let int2 = ident("Int2");
    let types = vec![
        record_type(
            "Cfg",
            &[
                ("on", "ix://quire/native/Boolean", false),
                ("n", &int2, false),
            ],
        ),
        record_type("Tick", &[("x", "ix://quire/native/Clock", false)]),
        record_type(
            "Open",
            &[
                ("n", "ix://quire/native/Integer", false),
                ("b", "ix://quire/native/Boolean", false),
            ],
        ),
    ];
    let timer = json!({"record": {"type": ident("Timer"), "fields": {"n": {"integer": "0"}}}});
    let granted = |element: &str, max: Option<u64>, ordered: bool, unique: bool| {
        let world = world_holding(
            cells_document(
                types.clone(),
                vec![("g", ident(element), Some(shape(max, ordered, unique)))],
                &["g"],
                &[],
            ),
            "",
            &only(&["work"], ""),
            json!({"t": timer.clone(), "g": empty(ordered, unique)}),
        );
        let held = system(&world);
        let bounded = check_model_bounds(&held, BOUND_POSITIONS).is_ok();
        let values = bounded.then(|| granted_values(&world, "work", "g"));
        values
    };
    let sequence = |element, max| granted(element, Some(max), true, false);
    let set = |element, max| granted(element, Some(max), false, true);
    let bag = |element, max| granted(element, Some(max), false, false);
    let count = |values: Option<Vec<String>>| values.map(|values| values.len());

    let cfg = sequence("Cfg", 2).expect("bounded");
    assert_eq!(cfg.len(), 43);
    let elements = |value: &str| value.matches("CompositeValue").count();
    let n = |value: &str, k: u8| value.matches(&format!("Integer(Integer({k}))")).count();
    let falses = |value: &str| value.matches("Boolean(false)").count();
    // `[]`, then `[c0]` = (false, 0) and `[c1]` = (false, 1), then `[c0,
    // c0]` and `[c0, c1]`.
    assert_eq!(elements(&cfg[0]), 0);
    assert_eq!(
        (elements(&cfg[1]), n(&cfg[1], 0), falses(&cfg[1])),
        (1, 1, 1)
    );
    assert_eq!(
        (elements(&cfg[2]), n(&cfg[2], 1), falses(&cfg[2])),
        (1, 1, 1)
    );
    assert_eq!(
        (elements(&cfg[7]), n(&cfg[7], 0), falses(&cfg[7])),
        (2, 2, 2)
    );
    assert_eq!((elements(&cfg[8]), n(&cfg[8], 0), n(&cfg[8], 1)), (2, 1, 1));
    assert_eq!(count(sequence("Timer", 2)), Some(21));
    assert_eq!(count(sequence("Timer", 3)), Some(85));
    assert_eq!(count(sequence("Tick", 3)), Some(4));
    assert_eq!(count(set("Cfg", 1)), Some(7));
    let sets = set("Cfg", 2).expect("bounded");
    assert_eq!(sets.len(), 22);
    assert_eq!(elements(&sets[0]), 0);
    assert_eq!(elements(&sets[6]), 1);
    assert_eq!(elements(&sets[7]), 2);
    assert_eq!(count(bag("Cfg", 2)), Some(28));
    assert_eq!(count(set("Timer", 2)), Some(11));
    assert_eq!(count(bag("Timer", 2)), Some(15));
    assert_eq!(sequence("Open", 1), None);
    assert_eq!(granted("Cfg", None, true, false), None);
    assert_eq!(granted("Cfg", None, false, true), None);
}

/// FR-120-AC-26 (TC-1270): no candidate ranges over a clock slot. From a
/// pre-state `ts = [Timer { n: 1 }]` whose `x` is 5 ms, a granted
/// `Sequence<Timer>[0, 2]`'s candidate `[n 1, n 2]` holds `x` 5 ms in slot
/// 0 and a fresh 0 ms in slot 1; a granted `Set<Timer>[0, 2]` from `{ n 1 }`
/// at 5 ms gives the candidate `{n 1, n 2}` `x` 5 ms for `n 1` (matched by
/// equal `n`) and 0 ms for `n 2`, and `{n 2}` 0 ms, and `{n 0, n 1}` 0 ms
/// for `n 0` and 5 ms for `n 1`, carried from slot 0 to slot 1.
#[trace("TC-1270")]
#[test]
fn a_candidate_slot_keeps_its_matched_clock_or_is_fresh() {
    let timer =
        |n: &str| json!({"record": {"type": ident("Timer"), "fields": {"n": {"integer": n}}}});
    let rational =
        |value: i64| Rational::new(Integer::from(value), Integer::from(1_i64)).expect("a rational");
    // Each `work` successor from `g` holding `Timer { n: 1 }` with `x` at 5
    // ms: the element `n` and the `x` of each slot of `g`, in slot order.
    let successors = |ordered: bool| {
        let held = if ordered {
            json!({"sequence": [timer("1")]})
        } else {
            json!({"set": [timer("1")]})
        };
        let world = world_holding(
            cells_document(
                vec![],
                vec![("g", ident("Timer"), Some(shape(Some(2), ordered, !ordered)))],
                &["g"],
                &[],
            ),
            "",
            &only(&["work"], ""),
            json!({"t": timer("0"), "g": held}),
        );
        let held = system(&world);
        let timed = TimedSystem::new(&held, "M").expect("a timed subject");
        let [initial] = &timed.initial()[..] else {
            panic!("one initial state");
        };
        let later = TimedState::new(
            initial.discrete().clone(),
            initial.clocks().delayed(&rational(5)),
        );
        let key = DeclarationKey {
            package: PACKAGE.to_owned(),
            node: ident("Call/g"),
        };
        let mut found: Vec<Vec<(String, Rational)>> = Vec::new();
        for (step, next) in timed.steps(&later).expect("the state expands") {
            if !step.operation().contains("Call/work\"") {
                continue;
            }
            let Some(quire_exact::Value::Collection(collection)) = next
                .discrete()
                .state()
                .populations
                .values()
                .find_map(|objects| objects.get("c"))
                .and_then(|object| object.fields.get(&key))
            else {
                panic!("g holds a collection");
            };
            let mut slots: Vec<(usize, Rational)> = timed
                .places()
                .iter()
                .zip(next.clocks().values())
                .filter(|(place, _)| timed.live(place, next.discrete()))
                .filter(|(place, _)| {
                    timed.model().clock(place.clock()).is_some_and(|clock| {
                        clock.name().starts_with("g.") && clock.name().ends_with(".x")
                    })
                })
                .map(|(place, value)| (place.slots()[0], value.clone()))
                .collect();
            slots.sort();
            found.push(
                slots
                    .into_iter()
                    .map(|(slot, value)| {
                        let element = format!("{:?}", collection.elements()[slot]);
                        let n = element
                            .split("Integer(Integer(")
                            .nth(1)
                            .and_then(|rest| rest.split(')').next())
                            .unwrap_or_default()
                            .to_owned();
                        (n, value)
                    })
                    .collect(),
            );
        }
        found
    };
    let pair = |n: &str, x: i64| (n.to_owned(), rational(x));
    let sequences = successors(true);
    assert!(
        sequences.contains(&vec![pair("1", 5), pair("2", 0)]),
        "{sequences:?}"
    );
    assert!(
        sequences.contains(&vec![pair("2", 5), pair("1", 0)]),
        "{sequences:?}"
    );
    assert!(sequences.contains(&vec![]), "{sequences:?}");
    let sets = successors(false);
    assert!(sets.contains(&vec![pair("1", 5), pair("2", 0)]), "{sets:?}");
    assert!(sets.contains(&vec![pair("2", 0)]), "{sets:?}");
    assert!(sets.contains(&vec![pair("0", 0), pair("1", 5)]), "{sets:?}");
    assert!(sets.contains(&vec![pair("1", 5)]), "{sets:?}");
}

/// FR-120-AC-26 (TC-1270) end to end through EN-1: a model whose `work`
/// grants `ts: Sequence<Timer>[0, 3]` and constrains nothing explores
/// exhaustively, with no requires-bound refusal, to the 85 values of `ts`.
#[trace("TC-1270")]
#[test]
fn exploration_reaches_every_value_of_a_bounded_clock_collection() {
    use qsl_eval::simulation::model::explore_model;
    use qsl_eval::simulation::Limits;
    use qsl_eval::simulation::Outcome;

    let world = world_holding(
        cells_document(
            vec![],
            vec![("ts", ident("Timer"), Some(shape(Some(3), true, false)))],
            &["ts"],
            &[],
        ),
        "",
        &only(&["work"], ""),
        json!({"t": {"record": {"type": ident("Timer"), "fields": {"n": {"integer": "0"}}}},
            "ts": {"sequence": []}}),
    );
    let held = system(&world);
    let exploration =
        explore_model(&held, BOUND_POSITIONS, Limits::default(), || false).expect("explores");
    let Outcome::Exhaustive(stats) = &exploration.outcome else {
        panic!("an exhaustive run: {:?}", exploration.outcome);
    };
    assert_eq!(stats.states, 85);
}
