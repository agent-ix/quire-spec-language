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
    ExpansionLimits, ModelSimulationRequest, ModelSystem, MoveRefusal, PopulationUniverse,
    TimedBehaviour, TimedSystem,
};
use qsl_package::CheckedPackage;
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, meaning, package_input};
use qsl_semantics::model::key::hex;
use qsl_semantics::model::observation::{DocumentRef, ObservationLimits};
use quire_checked::key::DeclarationKey;
use quire_exact::{Integer, Rational, ScalarLimits};
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
        "populations": [{"population": ident("calls"), "complete": true, "objects": [{
            "key": "c", "type": ident("Call"),
            "fields": {"phase": {"integer": "0"}, "inflight": {"boolean": false},
                "late": {"boolean": false}},
        }]}],
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
        expansion: ExpansionLimits {
            max_candidates: 1_000_000,
        },
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

/// Part of FR-231-AC-2 (TC-686): with `urgent when { self.phase = 1 }`, no
/// positive delay is admissible after `send`; the `Replied` state is
/// quiescent and the waiting one is not. The idle-tail reading is the next
/// test's.
#[trace("TC-686")]
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
/// at position 2, and one ending at the waiting state has no idle tail. Not
/// covered: the closed reading under a unit selecting the event-position
/// profile.
#[trace("TC-686")]
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

    let trace = behaviour.timed_trace(&timed, true).expect("a timed lasso");
    assert!(trace.is_lasso());
    let closed = behaviour
        .timed_trace(&timed, false)
        .expect("a timed prefix");
    assert!(!closed.is_lasso());
    assert_eq!(closed.stamps().len(), 3);

    let node = &world.package.graph().temporal_clauses()[0];
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

/// Part of FR-230-AC-4 (TC-685): 64 package clock fields over a universe of
/// 100 objects are 6,400 clock places, each 0 initially. The guard's convex
/// parts and the `10^40 ms` constant are S3's (qsl-semantics `timed_model`).
#[trace("TC-685")]
#[test]
fn every_object_holds_its_own_clock_of_every_clock_field() {
    let names: Vec<String> = (0..63).map(|index| format!("c{index:02}")).collect();
    let extra: Vec<&str> = names.iter().map(String::as_str).collect();
    let world = world_over(
        clocked_document(true, &extra),
        "",
        "dimension Time;\nunit s : Time = rational(1, 1);\n\
         unit ms : Time = rational(1, 1000) * s;\ntime M dense unit ms;\n",
    );
    let keys: Vec<String> = std::iter::once("c".to_owned())
        .chain((1..100).map(|index| format!("k{index:03}")))
        .collect();
    let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
    let system = system_over(&world, &keys);
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    assert_eq!(timed.model().clocks().len(), 64);
    assert_eq!(timed.places().len(), 6_400);
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
