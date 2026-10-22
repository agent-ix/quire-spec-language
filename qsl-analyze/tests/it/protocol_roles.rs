// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-208 (TC-653), FR-209-AC-3 (TC-654), FR-205-AC-4, FR-211-AC-4,
//! FR-211-AC-5 and FR-216-AC-5: replicated roles and `activation on each`
//! on `ProtocolSystem<Sc>` (ADR-027 ST-13 to ST-15, AE-1 to AE-4, RR-1 to
//! RR-4).
//!
//! Over the `test/fill` fixture: a replicated role `r each M::Cell from
//! cells` has the cells of the universe as its population, and `activation
//! on each (x: M::Level)` triggers on the four values of `Level`.

use std::collections::{BTreeMap, BTreeSet};

use ix_trace_rs::trace;
use qsl_analyze::model_check::{
    check_protocol, ModelCheckRefusal, ProtocolCheckRequest, SubjectContext, SubjectIdentity,
};
use qsl_eval::model_subject::{ModelInstance, ModelLetters, ProtocolSubject, RoleBinding};
use qsl_eval::simulation::model::{ExpansionLimits, ModelSimState, TerminalClass};
use qsl_eval::simulation::protocol::{
    visible, AnchorName, BlockedThread, ClaimReads, ProtocolLocation, ProtocolSystem,
    ProtocolTransition, RestKind, RestPoint, Sc, StepKind, ThreadPath, WaitCause,
};
use qsl_eval::simulation::{Location, SubjectSystem, TransitionSystem};
use qsl_eval::temporal::AtomReading;
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_package::CheckedPackage;
use qsl_replay::{
    node_occurrence, replay_protocol_trace, settle_protocol_check, InconclusiveCause,
    InstanceBound, ModelCheckItem, ModelCheckLimits, ModelCheckOutcome, ModelCheckRecord,
    ModelTraceReplayResult, ModelTraceSubject, ProtocolCertificateRequest, ProtocolTraceSubject,
    ReplayLimits, ReplayRefusal, ResultLabel, StatedInstanceBound, TemporalCounterexample,
    TerminalValue, UndecidedCause,
};
use qsl_route::deadlock::{ItemObligation, ObligationKind};
use qsl_route::model_request::ModelRequest;
use qsl_semantics::check::FormulaNodeId;
use qsl_semantics::model::observation::DocumentRef;
use quire_exact::{Cancel, Incomplete, Meter};
use serde_json::Value as Json;

use crate::fill::{
    class_of, contracts, document, instance, json_of, model_subject, package, package_id,
    reachable, sc_system, source_memory, unit_text, universe, v, Documents, Reached, PACKAGE,
};
use crate::model_settle::{envelope_at, request};
use crate::protocol_compensation::{pay_run, refund, REFUND};
use crate::protocol_system::touch;

/// `activation on origin`.
pub(crate) const ORIGIN: &str = "on origin";

/// `activation on each` over the four trigger values of `M::Level`.
pub(crate) const EACH: &str = "on each (x: M::Level) when (true)";

/// A unit of protocol `R` over `k: M::Cell`, activated `activation`, with
/// the role declarations `roles`, the members `members` and the control
/// `run`. `bump` has a precondition that is false in every state.
pub(crate) fn unit(activation: &str, roles: &str, members: &str, run: &str) -> String {
    contracts("self.v = 0")
        + "pre StuckPre using v on M::Cell::bump { false }\n"
        + &format!(
            "protocol R using v over (k: M::Cell) {activation} {{\n\
             role w on M::Cell;\n\
             {roles}\n\
             {members}\n\
             run {run}\n\
             finish Done as (d: Boolean) {{ true }};\n\
             }}\n"
        )
}

/// A role `r` with `max` live instances and `lifetime`.
pub(crate) fn role(max: u64, lifetime: &str) -> String {
    format!("role r each M::Cell from cells max {max} lifetime {lifetime};")
}

/// An `event Work` taken `by r`.
const WORK: &str = "event Work by r as (worked: Boolean) { worked };";

/// The unit's world: cells `keys`, each starting at `v = start` for every
/// `start`, one initial snapshot per start.
pub(crate) struct World {
    body: String,
    documents: Documents,
    initial: Vec<DocumentRef>,
    checked: CheckedPackage,
    keys: Vec<&'static str>,
}

impl World {
    pub(crate) fn new(body: &str, keys: &[&'static str], starts: &[i64]) -> Self {
        let mut documents = Documents::new();
        let initial = starts
            .iter()
            .enumerate()
            .map(|(index, start)| {
                let objects: Vec<(&str, i64)> = keys.iter().map(|key| (*key, *start)).collect();
                documents.start(&format!("s{index}"), &objects)
            })
            .collect();
        Self {
            body: body.to_owned(),
            documents,
            initial,
            checked: package(body),
            keys: keys.to_vec(),
        }
    }

    /// The subject of protocol `R` with `k` bound to `c`.
    pub(crate) fn subject(&self) -> ProtocolSubject<'_> {
        ProtocolSubject {
            model: model_subject(
                &self.checked,
                &self.documents,
                self.initial.clone(),
                &self.keys,
            ),
            protocol: "R".to_owned(),
            over: crate::fill::instance("c"),
            roles: vec![RoleBinding {
                role: "w".to_owned(),
                model: PACKAGE.to_owned(),
            }],
            memory: source_memory(&self.checked, "R"),
        }
    }
}

impl World {
    /// The unit's one claim as an item, its atom read by `letters`.
    fn claim<'a>(&'a self, letters: &'a dyn ModelLetters) -> ModelCheckItem<'a> {
        ModelCheckItem::Clause {
            clause: self.checked.graph().temporal_clauses()[0].clause(),
            letters,
        }
    }

    /// EN-1 over `item` with the search horizon `max_depth` and the
    /// instance bound `max_live_instances`.
    fn check(
        &self,
        item: ModelCheckItem<'_>,
        max_depth: Option<usize>,
        max_live_instances: Option<u64>,
    ) -> ModelCheckOutcome<ProtocolTransition> {
        check_protocol(
            ProtocolCheckRequest {
                subject: self.subject(),
                item,
                max_depth,
                max_store_buffer: None,
                max_live_instances,
                limits: ModelCheckLimits::default(),
            },
            &Cancel::new(),
        )
        .expect("the protocol subject checks")
    }

    /// FR-127's record of `outcome` over `item`, its counterexample
    /// replayed by FR-217 under the same instance bound.
    fn settle(
        &self,
        item: ModelCheckItem<'_>,
        letters: &dyn ModelLetters,
        outcome: &ModelCheckOutcome<ProtocolTransition>,
        max_live_instances: Option<u64>,
    ) -> ModelCheckRecord<ProtocolTransition> {
        settle_protocol_check(
            outcome,
            &ModelCheckLimits::default(),
            &package_id(&self.body),
            &ProtocolCertificateRequest {
                subject: self.subject(),
                item,
                max_store_buffer: None,
                max_live_instances,
            },
            &Cancel::new(),
            |payload| self.replay(payload.clone(), letters, max_live_instances),
        )
    }

    /// FR-217's replay of `payload`, a counterexample to the claim.
    fn replay(
        &self,
        payload: TemporalCounterexample<ProtocolTransition>,
        letters: &dyn ModelLetters,
        max_live_instances: Option<u64>,
    ) -> Result<ModelTraceReplayResult, ReplayRefusal> {
        let unit = unit_text(&self.body).into_bytes();
        let id = package_id(&self.body);
        let node = &self.checked.graph().temporal_clauses()[0];
        let clause_node = WireNodeId::from_digest(*node.identity().as_bytes());
        let occurrence = OccurrenceKey::new(clause_node, node.claim().clone());
        let subject = self.subject();
        replay_protocol_trace(
            request(&document(), self.documents.snapshots.iter(), &unit, id),
            &ProtocolTraceSubject {
                model: ModelTraceSubject {
                    initial: self.initial.clone(),
                    universes: universe(&self.keys),
                    expansion: ExpansionLimits {
                        max_candidates: 1_000_000,
                    },
                },
                protocol: "R".to_owned(),
                over: instance("c"),
                roles: subject.roles,
                memory: subject.memory,
                max_store_buffer: None,
                max_live_instances,
            },
            &envelope_at(&unit, id, payload, clause_node, occurrence),
            letters,
            ReplayLimits::default(),
        )
    }
}

type System<'a> = ProtocolSystem<'a, Sc>;

fn kinds(reached: &[Reached], at: usize) -> Vec<StepKind> {
    reached[at]
        .steps
        .iter()
        .map(|(step, _)| step.kind())
        .collect()
}

fn identity(system: &System<'_>, step: &ProtocolTransition) -> Json {
    json_of(&system.transition_key(step).expect("an identity"))
}

/// The cell a reference names, by its key.
fn named(reference: &Json) -> String {
    let text = reference["identity"].as_str().expect("an identity");
    let bytes: Vec<u8> = (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&text[at..at + 2], 16).expect("hex"))
        .collect();
    String::from_utf8(bytes).expect("a key")
}

/// The cells the `kind` steps of `reached[at]` name as their `object`.
fn objects(
    system: &System<'_>,
    reached: &[Reached],
    at: usize,
    kind: StepKind,
) -> BTreeSet<String> {
    reached[at]
        .steps
        .iter()
        .filter(|(step, _)| step.kind() == kind)
        .map(|(step, _)| named(&identity(system, step)["object"]))
        .collect()
}

/// The instance (an object reference) each step of `reached[at]` is taken
/// by, with repeats.
fn actors(system: &System<'_>, reached: &[Reached], at: usize) -> Vec<String> {
    reached[at]
        .steps
        .iter()
        .filter_map(|(step, _)| {
            let identity = identity(system, step);
            identity.get("actor").map(named)
        })
        .collect()
}

/// The state the `kind` step naming `object` reaches from `reached[at]`.
fn after(
    system: &System<'_>,
    reached: &[Reached],
    at: usize,
    kind: StepKind,
    object: &str,
) -> usize {
    let found: Vec<usize> = reached[at]
        .steps
        .iter()
        .filter(|(step, _)| {
            step.kind() == kind && named(&identity(system, step)["object"]) == object
        })
        .map(|(_, next)| *next)
        .collect();
    assert_eq!(found.len(), 1, "one {kind:?} of {object} from {at}");
    found[0]
}

/// The status of each instance of role `r` in the key of `state`, by the
/// cell's key.
fn statuses(system: &System<'_>, state: &Reached) -> BTreeMap<String, String> {
    let (bytes, _) = SubjectSystem::state_key(system, &state.state).expect("a key");
    let key = json_of(&bytes);
    key["roles"]["entries"]
        .as_array()
        .expect("a map")
        .iter()
        .filter(|entry| entry["key"]["role"] == "r")
        .flat_map(|entry| entry["value"]["entries"].as_array().expect("a map").clone())
        .map(|entry| {
            (
                named(&entry["key"]),
                entry["value"]["value"]
                    .as_str()
                    .expect("a status")
                    .to_owned(),
            )
        })
        .collect()
}

/// FR-208-AC-1 (TC-653): a role `r each M::Cell max 1 lifetime until(p)`
/// over the cells `{c, d}`, with `p` true once the instance's cell has
/// worked (`inc`): both spawns are enabled initially; after one, no spawn is
/// enabled until `retire`; `retire` is enabled exactly where `p` holds; after
/// it the other cell's spawn is enabled and the retired cell's never is
/// again, and the retired cell stays in `roles`.
#[trace("TC-653", "FR-208-AC-1")]
#[test]
fn a_retired_role_instance_is_never_spawned_again() {
    let run = "sequence Main { attempt Work by r on M::Cell::inc contracts [IncPre, IncPost] \
               as (worked: Boolean) { true }; }";
    let world = World::new(
        &unit(ORIGIN, &role(1, "until(r.v >= 1)"), "", run),
        &["c", "d"],
        &[0],
    );
    let subject = world.subject();
    let system = sc_system(&subject);
    let reached = reachable(&system);

    assert_eq!(kinds(&reached, 0), [StepKind::Spawn, StepKind::Spawn]);
    assert_eq!(
        objects(&system, &reached, 0, StepKind::Spawn),
        BTreeSet::from(["c".to_owned(), "d".to_owned()])
    );
    let spawned = after(&system, &reached, 0, StepKind::Spawn, "c");
    assert!(!kinds(&reached, spawned).contains(&StepKind::Spawn));
    assert!(!kinds(&reached, spawned).contains(&StepKind::Retire));
    assert_eq!(
        statuses(&system, &reached[spawned]),
        BTreeMap::from([("c".to_owned(), "live".to_owned())])
    );

    let mut respawned = false;
    let mut followed = false;
    for (at, state) in reached.iter().enumerate() {
        // `retire` is enabled exactly where `p` holds, and the state it
        // reaches holds that instance retired with a `spawn` enabled for
        // exactly the cells not spawned yet.
        for object in objects(&system, &reached, at, StepKind::Retire) {
            assert!(v(state.state.model(), &object) >= 1, "state {at}");
            let next = after(&system, &reached, at, StepKind::Retire, &object);
            let known = statuses(&system, &reached[next]);
            assert_eq!(known.get(&object).map(String::as_str), Some("retired"));
            let unspawned: BTreeSet<String> = ["c", "d"]
                .into_iter()
                .filter(|cell| !known.contains_key(*cell))
                .map(str::to_owned)
                .collect();
            assert_eq!(
                objects(&system, &reached, next, StepKind::Spawn),
                unspawned,
                "state {next} after retiring {object}"
            );
            followed |= !unspawned.is_empty();
        }
        for (object, status) in statuses(&system, state) {
            let live = status == "live";
            let held = v(state.state.model(), &object) >= 1;
            if live && held {
                assert!(
                    objects(&system, &reached, at, StepKind::Retire).contains(&object),
                    "state {at}: {object} has worked"
                );
            }
            if status == "retired" {
                assert!(
                    !objects(&system, &reached, at, StepKind::Spawn).contains(&object),
                    "state {at}: {object} is spawned again"
                );
                respawned |= !objects(&system, &reached, at, StepKind::Spawn).is_empty();
            }
        }
    }
    assert!(
        respawned && followed,
        "the other cell is spawned after the first retires"
    );
}

/// FR-208-AC-2 (TC-653): with `max 2`, spawning `c` then `d` and `d` then
/// `c` reach one state, and with both live the `Work` event gives two steps
/// whose identities name `c` and `d`.
#[trace("TC-653", "FR-208-AC-2")]
#[test]
fn spawn_order_is_no_part_of_the_key_and_each_live_instance_acts() {
    let world = World::new(
        &unit(
            ORIGIN,
            &role(2, "workflow"),
            "",
            &format!("sequence Main {{ {WORK} }}"),
        ),
        &["c", "d"],
        &[0],
    );
    let subject = world.subject();
    let system = sc_system(&subject);
    let reached = reachable(&system);

    let c_then_d = {
        let c = after(&system, &reached, 0, StepKind::Spawn, "c");
        after(&system, &reached, c, StepKind::Spawn, "d")
    };
    let d_then_c = {
        let d = after(&system, &reached, 0, StepKind::Spawn, "d");
        after(&system, &reached, d, StepKind::Spawn, "c")
    };
    assert_eq!(c_then_d, d_then_c);
    assert_eq!(
        kinds(&reached, c_then_d),
        [StepKind::Event, StepKind::Event]
    );
    let mut acting = actors(&system, &reached, c_then_d);
    acting.sort();
    assert_eq!(acting, ["c", "d"]);
}

/// FR-208-AC-3 (TC-653): with lifetime `workflow`, every live instance
/// retires in the `finish` step, which removes the protocol instance with its
/// role instances; with lifetime `scope` an instance retires in the step
/// after which no running thread can reach a node `by r`, and no `retire`
/// step exists.
#[trace("TC-653", "FR-208-AC-3")]
#[test]
fn workflow_and_scope_lifetimes_retire_in_the_step_that_ends_them() {
    let run = format!("sequence Main {{ {WORK} }}");
    let workflow = World::new(&unit(ORIGIN, &role(1, "workflow"), "", &run), &["c"], &[0]);
    let subject = workflow.subject();
    let system = sc_system(&subject);
    let reached = reachable(&system);
    let worked = (0..reached.len())
        .find(|at| kinds(&reached, *at).contains(&StepKind::Finish))
        .expect("a state that can finish");
    assert_eq!(
        statuses(&system, &reached[worked]),
        BTreeMap::from([("c".to_owned(), "live".to_owned())])
    );
    let finished = reached[worked]
        .steps
        .iter()
        .find(|(step, _)| step.kind() == StepKind::Finish)
        .map(|(_, next)| *next)
        .expect("a finish step");
    assert!(!reached[finished].state.has_live_instance());
    assert!(statuses(&system, &reached[finished]).is_empty());
    assert!(reached.iter().all(|state| state
        .steps
        .iter()
        .all(|(step, _)| step.kind() != StepKind::Retire)));

    let scoped = World::new(&unit(ORIGIN, &role(1, "scope"), "", &run), &["c"], &[0]);
    let subject = scoped.subject();
    let system = sc_system(&subject);
    let reached = reachable(&system);
    assert!(reached.iter().all(|state| state
        .steps
        .iter()
        .all(|(step, _)| step.kind() != StepKind::Retire)));
    let spawned = after(&system, &reached, 0, StepKind::Spawn, "c");
    assert_eq!(
        statuses(&system, &reached[spawned]),
        BTreeMap::from([("c".to_owned(), "live".to_owned())])
    );
    let event = reached[spawned]
        .steps
        .iter()
        .find(|(step, _)| step.kind() == StepKind::Event)
        .map(|(_, next)| *next)
        .expect("the Work event");
    assert_eq!(
        statuses(&system, &reached[event]),
        BTreeMap::from([("c".to_owned(), "retired".to_owned())])
    );
}

/// The root thread of instance 0.
fn root() -> ThreadPath {
    ThreadPath::root(0)
}

/// Part of FR-211-AC-5 (TC-656), the role part at an `event`: a node owned
/// by a replicated role whose every object is retired, so that it has no
/// live instance and no `spawn` is enabled, deadlocks with the thread
/// `NoRoleInstance{r}`; where a `spawn` is enabled the thread has no such
/// cause and the state is not terminal.
#[test]
fn a_node_owned_by_a_role_with_no_instance_waits_with_no_role_instance() {
    let world = World::new(
        &unit(
            ORIGIN,
            &role(1, "until(r.v >= 0)"),
            "",
            &format!("sequence Main {{ {WORK} }}"),
        ),
        &["c"],
        &[0],
    );
    let subject = world.subject();
    let system = sc_system(&subject);
    let reached = reachable(&system);

    let terminal: Vec<usize> = (0..reached.len())
        .filter(|at| reached[*at].steps.is_empty() && reached[*at].state.has_live_instance())
        .collect();
    assert_eq!(terminal.len(), 1);
    let blocked = SubjectSystem::blocked(&system, &reached[terminal[0]].state, &[])
        .expect("the blocked threads");
    assert_eq!(
        blocked,
        [BlockedThread {
            path: root(),
            rest_point: RestPoint {
                kind: RestKind::Node,
                node: Some("Main/Work".to_owned()),
            },
            cause: WaitCause::NoRoleInstance {
                role: "r".to_owned()
            },
        }]
    );
    assert_eq!(
        class_of(&system, &reached[terminal[0]].state),
        TerminalClass::Deadlocked
    );

    // At the initial state `spawn` is enabled: no `NoRoleInstance`.
    let initial = SubjectSystem::blocked(&system, &reached[0].state, &[]).expect("blocked");
    assert!(initial
        .iter()
        .all(|thread| !matches!(thread.cause, WaitCause::NoRoleInstance { .. })));
    assert_eq!(
        class_of(&system, &reached[0].state),
        TerminalClass::NotTerminal
    );
}

/// Part of FR-211-AC-5 (TC-656), the role part for a `receive`: a `receive`
/// owned by a replicated role (its channel's `to` role) whose every object
/// is retired gives `NoRoleInstance{r}` and not `Constraint`, `HeadBlocked`
/// or `EmptyQueue`, although its send's entry is queued.
#[test]
fn a_receive_owned_by_a_role_with_no_instance_gives_no_role_instance() {
    let channel = "channel ch from w to r carries Boolean ordering unordered \
                   delivery exactly-once-premise capacity 1 overflow block;";
    let run = "parallel Both { branch s send Tx via ch as (tx: Boolean) { tx }; \
               branch q receive Rx via ch of s::Tx as (rx: Boolean) { true }; } join all [s, q];";
    let world = World::new(
        &unit(ORIGIN, &role(1, "until(r.v >= 0)"), channel, run),
        &["c"],
        &[0],
    );
    let subject = world.subject();
    let system = sc_system(&subject);
    let reached = reachable(&system);
    assert_eq!(
        deadlocked_at(&system, &reached, "Both/q/Rx"),
        [no_role_instance()]
    );
}

/// `NoRoleInstance{r}`.
fn no_role_instance() -> WaitCause {
    WaitCause::NoRoleInstance {
        role: "r".to_owned(),
    }
}

/// The causes of the threads resting at the node `node` in the deadlocked
/// states of `reached`, those with a live instance and no step.
fn deadlocked_at(system: &System<'_>, reached: &[Reached], node: &str) -> Vec<WaitCause> {
    reached
        .iter()
        .filter(|state| state.steps.is_empty() && state.state.has_live_instance())
        .flat_map(|state| {
            SubjectSystem::blocked(system, &state.state, &[]).expect("the blocked threads")
        })
        .filter(|thread| thread.rest_point.node.as_deref() == Some(node))
        .map(|thread| thread.cause)
        .collect()
}

/// A world whose role `r` retires its only instance at once, so that `run`
/// finds `r` with no live instance and no `spawn` enabled.
fn retired_role_world(members: &str, run: &str) -> World {
    World::new(
        &unit(ORIGIN, &role(1, "until(r.v >= 0)"), members, run),
        &["c"],
        &[0],
    )
}

/// Part of FR-211-AC-5 (TC-656), the role part at an `attempt` and at a
/// `send`: a node owned by a replicated role with every object retired
/// deadlocks with `NoRoleInstance{r}`, a `send` owned by its channel's
/// `from` role too. This is not FR-208-AC-5's empty-universe case: the
/// world has one object, spawned and retired at once, because `over` needs a
/// cell and the role's universe is the cells.
#[trace("TC-653")]
#[test]
fn an_attempt_and_a_send_owned_by_a_role_with_no_instance_give_no_role_instance() {
    let attempt = retired_role_world(
        "",
        "sequence Main { attempt Work by r on M::Cell::inc contracts [IncPre, IncPost] \
         as (worked: Boolean) { true }; }",
    );
    let subject = attempt.subject();
    let system = sc_system(&subject);
    let reached = reachable(&system);
    assert_eq!(
        deadlocked_at(&system, &reached, "Main/Work"),
        [no_role_instance()]
    );

    let channel = "channel ch from r to w carries Boolean ordering unordered \
                   delivery exactly-once-premise capacity 1 overflow block;";
    let send = retired_role_world(
        channel,
        "sequence Main { send Tx via ch as (tx: Boolean) { tx }; }",
    );
    let subject = send.subject();
    let system = sc_system(&subject);
    let reached = reachable(&system);
    assert_eq!(
        deadlocked_at(&system, &reached, "Main/Tx"),
        [no_role_instance()]
    );
}

/// Part of FR-211-AC-5 (TC-656), the role part for a `receive` of a send
/// node with nothing queued: the send never happens (its branch is stuck at
/// a `bump` whose precondition is false), and the `receive` owned by a role
/// with every object retired gives `NoRoleInstance{r}` and not
/// `EmptyQueue`.
#[test]
fn a_receive_with_nothing_queued_owned_by_a_role_with_no_instance_gives_no_role_instance() {
    let channel = "channel ch from w to r carries Boolean ordering unordered \
                   delivery exactly-once-premise capacity 1 overflow block;";
    let run = "parallel Both { branch s sequence S { attempt Stuck by w on M::Cell::bump \
               contracts [StuckPre] as (st: Boolean) { true }; \
               send Tx via ch as (tx: Boolean) { tx }; } \
               branch q receive Rx via ch of s::S::Tx as (rx: Boolean) { true }; } \
               join all [s, q];";
    let world = retired_role_world(channel, run);
    let subject = world.subject();
    let system = sc_system(&subject);
    let reached = reachable(&system);
    assert!(reached.iter().all(|state| state
        .steps
        .iter()
        .all(|(step, _)| step.kind() != StepKind::Send)));
    assert_eq!(
        deadlocked_at(&system, &reached, "Both/q/Rx"),
        [no_role_instance()]
    );
}

/// ADR-027 §7.1's `Pay` with its template `Refund` taken `by r` and the
/// replicated role `r` of `lifetime`.
fn pay_by_role(lifetime: &str) -> String {
    contracts("self.v = 0")
        + REFUND
        + &format!(
            "protocol R using v over (k: M::Cell) on origin {{\n\
             role w on M::Cell;\n\
             {}\n\
             {}\n\
             run {}\n\
             finish Done as (d: Boolean) {{ d }};\n\
             }}\n",
            role(1, lifetime),
            refund("Main::Paid", "dec", 1, "false", "never", "true").replace("by w on", "by r on"),
            pay_run("")
        )
}

/// The compensation thread of `R::Refund` with ordinal 0.
fn refund_thread() -> ThreadPath {
    ThreadPath::compensation(0, "R::Refund", 0)
}

/// Part of FR-211-AC-5 (TC-656), the role part at a compensation attempt: a
/// template taken `by r`, whose compensation thread is created when `r` has
/// no live instance and no `spawn` is enabled, deadlocks with the thread
/// `NoRoleInstance{r}`.
#[test]
fn a_compensation_attempt_owned_by_a_role_with_no_instance_gives_no_role_instance() {
    let world = World::new(&pay_by_role("until(r.v >= 0)"), &["c"], &[0]);
    let subject = world.subject();
    let system = sc_system(&subject);
    let reached = reachable(&system);
    let waits: Vec<WaitCause> = reached
        .iter()
        .filter(|state| state.steps.is_empty() && state.state.has_live_instance())
        .flat_map(|state| {
            SubjectSystem::blocked(&system, &state.state, &[]).expect("the blocked threads")
        })
        .filter(|thread| thread.path == refund_thread())
        .map(|thread| thread.cause)
        .collect();
    assert_eq!(waits, [no_role_instance()]);
}

/// ADR-027 RR-4: a `scope` role owning only the template's compensation
/// attempt stays live while the template can still compensate, so the
/// compensation thread finds an instance and attempts, and no state waits
/// with `NoRoleInstance`.
#[test]
fn a_scope_role_stays_live_for_the_compensation_attempt_it_owns() {
    let world = World::new(&pay_by_role("scope"), &["c"], &[0]);
    let subject = world.subject();
    let system = sc_system(&subject);
    let reached = reachable(&system);
    assert!(reached.iter().any(|state| state
        .steps
        .iter()
        .any(|(step, _)| step.kind() == StepKind::CAttempt)));
    for state in reached
        .iter()
        .filter(|state| state.steps.is_empty() && state.state.has_live_instance())
    {
        let blocked = SubjectSystem::blocked(&system, &state.state, &[]).expect("blocked");
        assert!(blocked
            .iter()
            .all(|thread| thread.cause != no_role_instance()));
    }
}

/// An `on each` unit whose body is one attempt of `bump`, never enabled.
fn stuck() -> String {
    unit(
        EACH,
        "",
        "",
        "sequence Main { attempt Stuck by w on M::Cell::bump contracts [StuckPre] \
         as (stuck: Boolean) { true }; }",
    )
}

/// FR-205-AC-4 (TC-650), the `on each` half: an `on each` protocol's initial
/// protocol state, one per initial snapshot, holds no instance, and
/// `activate` is its only step.
#[trace("TC-650", "FR-205-AC-4")]
#[test]
fn an_on_each_protocol_starts_with_no_instance() {
    let world = World::new(&stuck(), &["c"], &[0, 1]);
    let subject = world.subject();
    let system = sc_system(&subject);
    let initial = system.initial();
    assert_eq!(initial.len(), 2);
    for state in &initial {
        assert!(!state.has_live_instance());
        let (bytes, _) = SubjectSystem::state_key(&system, state).expect("a key");
        let key = json_of(&bytes);
        assert!(key["control"]["entries"]
            .as_array()
            .expect("a map")
            .is_empty());
    }
    let reached = reachable(&system);
    assert!(kinds(&reached, 0)
        .iter()
        .all(|kind| *kind == StepKind::Activate));
    let started = reached[0].steps[0].1;
    assert!(reached[started].state.has_live_instance());
    assert_eq!(
        system
            .threads(&reached[started].state)
            .expect("threads")
            .iter()
            .map(|view| view.path.clone())
            .collect::<Vec<_>>(),
        [root()]
    );
}

/// FR-211-AC-4 (TC-656): an `on each` protocol whose one attempt has a
/// precondition false in every state, with no `terminal` member and
/// `max_live_instances` 2: the state with one live instance, where only
/// `activate` is enabled, is a deadlock with blocked thread `(0)` at the
/// attempt, cause `Precondition`; the state with two live instances is
/// instance-limited and no deadlock; the initial state is no deadlock.
#[trace("TC-656", "FR-211-AC-4")]
#[test]
fn an_on_each_state_that_can_only_activate_is_a_deadlock() {
    let world = World::new(&stuck(), &["c"], &[0]);
    let subject = world.subject();
    let system = sc_system(&subject).with_max_live_instances(2);
    let reached = reachable(&system);
    assert_eq!(reached.len(), 3);
    let class = |at: usize| class_of(&system, &reached[at].state);

    assert_eq!(class(0), TerminalClass::NotTerminal);
    let one = reached[0].steps[0].1;
    assert!(kinds(&reached, one)
        .iter()
        .all(|kind| *kind == StepKind::Activate));
    assert_eq!(class(one), TerminalClass::Deadlocked);
    assert_eq!(
        SubjectSystem::blocked(&system, &reached[one].state, &[]).expect("blocked"),
        [BlockedThread {
            path: root(),
            rest_point: RestPoint {
                kind: RestKind::Node,
                node: Some("Main/Stuck".to_owned()),
            },
            cause: WaitCause::Precondition {
                clause: "StuckPre".to_owned()
            },
        }]
    );

    let two = reached[one].steps[0].1;
    assert!(reached[two].steps.is_empty());
    assert!(system
        .instance_limited(&reached[two].state)
        .expect("limited"));
    assert!(!system
        .instance_limited(&reached[one].state)
        .expect("limited"));
    assert_eq!(class(two), TerminalClass::NotTerminal);
}

/// FR-209-AC-3 (TC-654): with `max_live_instances` 2, after two `activate`
/// steps the instances hold ordinals 0 and 1; once instance 0 finishes and
/// is removed, the next `activate` creates ordinal 0. An instance-limited
/// state is not reported as a deadlock. The activation guard and the
/// activation binder are read: only the trigger `0` activates, and
/// the attempt's constraint reads the binder.
#[trace("TC-654", "FR-209-AC-3")]
#[test]
fn an_instance_takes_the_smallest_free_ordinal_and_the_bound_is_no_deadlock() {
    let run = "sequence Main { attempt T by w on M::Cell::touch contracts [TouchPre] \
               as (t: Boolean) { x = 0 }; }";
    let world = World::new(
        &unit("on each (x: M::Level) when (x = 0)", "", "", run),
        &["c"],
        &[0],
    );
    let subject = world.subject();
    let system = sc_system(&subject).with_max_live_instances(2);
    let reached = reachable(&system);
    let live = |at: usize| -> Vec<u64> {
        system
            .threads(&reached[at].state)
            .expect("threads")
            .iter()
            .map(|view| view.path.instance)
            .collect()
    };
    let activate = |at: usize| -> usize {
        let found: Vec<usize> = reached[at]
            .steps
            .iter()
            .filter(|(step, _)| step.kind() == StepKind::Activate)
            .map(|(_, next)| *next)
            .collect();
        assert_eq!(found.len(), 1, "one activate from {at}");
        found[0]
    };

    assert_eq!(kinds(&reached, 0), [StepKind::Activate]);
    let first = activate(0);
    assert_eq!(live(first), [0]);
    let second = activate(first);
    assert_eq!(live(second), [0, 1]);
    assert!(!kinds(&reached, second).contains(&StepKind::Activate));
    assert!(system
        .instance_limited(&reached[second].state)
        .expect("limited"));

    // Instance 0 touches and finishes; the next activation takes ordinal 0.
    let mut at = second;
    for kind in [StepKind::Attempt, StepKind::Finish] {
        at = reached[at]
            .steps
            .iter()
            .find(|(step, _)| {
                step.kind() == kind && step.thread().is_some_and(|thread| thread.instance == 0)
            })
            .map(|(_, next)| *next)
            .unwrap_or_else(|| panic!("{kind:?} of instance 0 from {at}"));
    }
    assert_eq!(live(at), [1]);
    let refilled = activate(at);
    assert_eq!(live(refilled), [0, 1]);
    assert!(reached.iter().all(|state| {
        !system.instance_limited(&state.state).expect("limited")
            || class_of(&system, &state.state) == TerminalClass::NotTerminal
    }));
}

/// FR-216-AC-5 (TC-661), the role and `pinst` parts: a step `by` a
/// replicated role reads `role(r)`, `spawn` reads and writes it, `activate`
/// reads and writes `pinst`; a step `by` the role is visible to an atom that
/// reads its position's anchor and names the role, and invisible to one
/// naming another role.
#[trace("TC-661", "FR-216-AC-5")]
#[test]
fn role_and_activation_steps_have_role_and_instance_footprints() {
    let world = World::new(
        &unit(
            ORIGIN,
            &role(2, "workflow"),
            "",
            &format!("sequence Main {{ {WORK} }}"),
        ),
        &["c", "d"],
        &[0],
    );
    let subject = world.subject();
    let system = sc_system(&subject);
    let reached = reachable(&system);
    let role_location = Location::Protocol(ProtocolLocation::Role {
        instance: 0,
        role: "r".to_owned(),
    });

    let (spawn, _) = &reached[0].steps[0];
    let spawning = system
        .footprint(&reached[0].state, spawn)
        .expect("a footprint");
    assert!(spawning.reads.contains(&role_location));
    assert!(spawning.writes.contains(&role_location));

    let both = {
        let c = after(&system, &reached, 0, StepKind::Spawn, "c");
        after(&system, &reached, c, StepKind::Spawn, "d")
    };
    let (event, _) = reached[both]
        .steps
        .iter()
        .find(|(step, _)| step.kind() == StepKind::Event)
        .expect("an event by r");
    let acting = system
        .footprint(&reached[both].state, event)
        .expect("a footprint");
    assert!(acting.enabling.contains(&role_location));
    assert!(!acting.writes.contains(&role_location));
    let naming = |role: &str| ClaimReads {
        locations: BTreeSet::new(),
        anchors: BTreeSet::from([AnchorName::Role(role.to_owned())]),
    };
    assert!(visible(event, &acting, &naming("r")));
    assert!(!visible(event, &acting, &naming("q")));
    assert!(visible(spawn, &spawning, &naming("r")));

    let each = World::new(&stuck(), &["c"], &[0]);
    let subject = each.subject();
    let system = sc_system(&subject);
    let reached = reachable(&system);
    let (activate, _) = &reached[0].steps[0];
    let activating = system
        .footprint(&reached[0].state, activate)
        .expect("a footprint");
    let live = Location::Protocol(ProtocolLocation::LiveInstances);
    assert!(activating.reads.contains(&live) && activating.writes.contains(&live));
    assert!(activating
        .writes
        .contains(&Location::Protocol(ProtocolLocation::Control(root()))));
}

/// The claim `always holds(condition)` over `k: M::Cell`.
fn claim(condition: &str) -> String {
    format!(
        "temporal Bound using t0 over (k: M::Cell) clock \"model-steps\" on origin \
         {{ always holds({condition}) }}\n"
    )
}

/// The one atom of a claim, as the predicate `holds` of the bound cell's `v`.
struct CellV {
    holds: fn(i64) -> bool,
}

impl ModelLetters for CellV {
    crate::units::unguarded!();

    fn reads(&self, _atom: FormulaNodeId) -> crate::units::Reads {
        crate::units::unread()
    }

    fn atom(
        &self,
        _atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, Incomplete> {
        Ok(if (self.holds)(v(state, &instance.key)) {
            AtomReading::True
        } else {
            AtomReading::False
        })
    }
}

/// `activation on each` over the four trigger values of `M::Level`, with the
/// guard `when`.
fn each_when(when: &str) -> String {
    format!("on each (x: M::Level) when ({when})")
}

/// FR-209-AC-1 (TC-654): an `on each` protocol whose body is one
/// always-enabled attempt, with the claim `always holds(k.v <= 3)`: with
/// `max_live_instances` 1 the run settles V-6 `InstanceBoundReached` with
/// the bound 1 and at least one instance-limited state, with 2 it settles
/// V-6 with the bound 2, and with the field unset the record states the
/// default 3, reached. Every record states the bound used and that it was
/// reached. The default is 3: no state of an unset run holds a fourth
/// instance, and a state of three is instance-limited.
#[trace("TC-654", "FR-209-AC-1")]
#[test]
fn the_instance_bound_settles_v6_naming_max_live_instances_and_defaults_to_three() {
    let run = format!("sequence Main {{ {} }}", touch("T"));
    let world = World::new(
        &(unit(EACH, "", "", &run) + &claim("k.v <= 3")),
        &["c"],
        &[0],
    );
    let letters = CellV { holds: |v| v <= 3 };
    let item = world.claim(&letters);
    assert_eq!(InstanceBound::MEMBER, "max_live_instances");
    for (requested, bound) in [(Some(1), 1), (Some(2), 2), (None, 3)] {
        let outcome = world.check(item, None, requested);
        let ModelCheckOutcome::Undecided {
            cause:
                UndecidedCause::InstanceBoundReached {
                    bound: used,
                    states,
                },
            stats,
        } = &outcome
        else {
            panic!("{requested:?}: {outcome:?}");
        };
        assert_eq!(used.value, bound);
        assert!(*states >= 1);
        assert_eq!(stats.instance_limited, *states);
        let record = world.settle(item, &letters, &outcome, requested);
        assert_eq!(
            record.value,
            TerminalValue::Inconclusive(InconclusiveCause::InstanceBoundReached {
                bound: *used,
                states: *states,
            })
        );
        assert_eq!(record.label, ResultLabel::Inconclusive);
        assert_eq!(
            record.instance_bound,
            Some(StatedInstanceBound {
                bound: *used,
                reached: true,
            })
        );
        assert_eq!(record.memory_bound, None);
    }

    let subject = world.subject();
    let system = sc_system(&subject);
    let reached = reachable(&system);
    let live = |at: usize| -> usize {
        system
            .threads(&reached[at].state)
            .expect("threads")
            .iter()
            .map(|view| view.path.instance)
            .collect::<BTreeSet<_>>()
            .len()
    };
    assert_eq!((0..reached.len()).map(live).max(), Some(3));
    assert!((0..reached.len()).any(|at| live(at) == 3
        && system
            .instance_limited(&reached[at].state)
            .expect("limited")));
}

/// FR-209-AC-2 (TC-654): the same protocol with the guard `false` has no
/// `activate` step, settles V-1 `proved`, and its record states
/// `max_live_instances` 3, not reached. With the guard back at `true`, the
/// claim `always holds(k.v = 0)` and `max_live_instances` 1 the run settles
/// V-4 `refuted`, with the counterexample `activate`, `attempt`.
#[trace("TC-654", "FR-209-AC-2")]
#[test]
fn a_run_with_no_activation_proves_and_a_counterexample_is_real_under_any_bound() {
    let run = format!("sequence Main {{ {} }}", touch("T"));
    let never = World::new(
        &(unit(&each_when("false"), "", "", &run) + &claim("k.v <= 3")),
        &["c"],
        &[0],
    );
    let letters = CellV { holds: |v| v <= 3 };
    let item = never.claim(&letters);
    let subject = never.subject();
    let system = sc_system(&subject);
    assert!(reachable(&system).iter().all(|state| state
        .steps
        .iter()
        .all(|(step, _)| step.kind() != StepKind::Activate)));
    let outcome = never.check(item, None, None);
    assert!(
        matches!(outcome, ModelCheckOutcome::Holds { .. }),
        "{outcome:?}"
    );
    let record = never.settle(item, &letters, &outcome, None);
    assert_eq!(record.label, ResultLabel::Proved);
    assert_eq!(
        record.instance_bound,
        Some(StatedInstanceBound {
            bound: InstanceBound { value: 3 },
            reached: false,
        })
    );

    let bump = "sequence Main { attempt A by w on M::Cell::inc contracts [IncPre, IncPost] \
                as (a: Boolean) { true }; }";
    let always = World::new(
        &(unit(&each_when("true"), "", "", bump) + &claim("k.v = 0")),
        &["c"],
        &[0],
    );
    let letters = CellV { holds: |v| v == 0 };
    let item = always.claim(&letters);
    let outcome = always.check(item, None, Some(1));
    let ModelCheckOutcome::Violated { counterexample, .. } = &outcome else {
        panic!("{outcome:?}");
    };
    let kinds: Vec<StepKind> = counterexample
        .prefix
        .iter()
        .map(|step| step.transition.kind())
        .collect();
    assert_eq!(kinds, [StepKind::Activate, StepKind::Attempt]);
    let record = always.settle(item, &letters, &outcome, Some(1));
    assert_eq!(record.label, ResultLabel::Refuted);
    assert_eq!(
        record.instance_bound,
        Some(StatedInstanceBound {
            bound: InstanceBound { value: 1 },
            reached: true,
        })
    );
}

/// Part of FR-209-AC-4 (TC-654): a run where the instance bound limited a
/// state and the `max_depth` horizon also applies settles
/// `InstanceBoundReached` and not `BoundReached`, and a horizon that cuts
/// before any state is instance-limited still settles `BoundReached`. The
/// memory bound settles before the instance bound in `weak_memory.rs`.
#[test]
fn the_instance_bound_settles_before_the_horizon() {
    let run = format!("sequence Main {{ {} }}", touch("T"));
    let world = World::new(
        &(unit(EACH, "", "", &run) + &claim("k.v <= 3")),
        &["c"],
        &[0],
    );
    let letters = CellV { holds: |v| v <= 3 };
    let item = world.claim(&letters);
    let horizon = world.check(item, Some(2), Some(1));
    assert!(
        matches!(
            horizon,
            ModelCheckOutcome::Undecided {
                cause: UndecidedCause::InstanceBoundReached { .. },
                ..
            }
        ),
        "{horizon:?}"
    );
    let record = world.settle(item, &letters, &horizon, Some(1));
    assert!(matches!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::InstanceBoundReached { .. })
    ));
    let shallow = world.check(item, Some(1), None);
    assert!(
        matches!(shallow, ModelCheckOutcome::BoundReached { .. }),
        "{shallow:?}"
    );
}

/// ADR-027 AE-4 (TC-654): an instance-limited state with no expanded
/// successor is no terminal state. FR-211-AC-4's stuck unit keeps `v` at 0,
/// so the bounded claim `always[0, 5] holds(k.v = 0)` holds on every real
/// behaviour; the default bound 3 leaves the state of three instances
/// unexpanded, and the run settles V-6 `InstanceBoundReached` rather than
/// closing that state with every atom false and refuting the claim.
#[test]
fn an_instance_limited_state_with_no_successor_is_not_closed_as_terminal() {
    let bounded = "temporal Bound using tb over (k: M::Cell) clock \"model-steps\" on origin \
                   { always[0, 5] holds(k.v = 0) }\n";
    let world = World::new(&(stuck() + bounded), &["c"], &[0]);
    let letters = CellV { holds: |v| v == 0 };
    let item = world.claim(&letters);
    let outcome = world.check(item, None, None);
    let ModelCheckOutcome::Undecided {
        cause: UndecidedCause::InstanceBoundReached { bound, states },
        ..
    } = &outcome
    else {
        panic!("{outcome:?}");
    };
    assert_eq!(bound.value, 3);
    assert!(*states >= 1);
}

/// FR-212 (TC-657): a liveness clause over an `activation on each` protocol
/// refuses `unsupported_construct`/`not-yet-implemented`, since the liveness
/// engine does not read `max_live_instances` (FR-209) yet.
#[test]
fn a_liveness_clause_over_an_on_each_protocol_refuses() {
    let eventually = "temporal Live using t0 over (k: M::Cell) clock \"model-steps\" \
                      on origin { eventually holds(k.v = 1) }\n";
    let run = format!("sequence Main {{ {} }}", touch("T"));
    let world = World::new(&(unit(EACH, "", "", &run) + eventually), &["c"], &[0]);
    let letters = CellV { holds: |v| v == 1 };
    let refusal = check_protocol(
        ProtocolCheckRequest {
            subject: world.subject(),
            item: world.claim(&letters),
            max_depth: None,
            max_store_buffer: None,
            max_live_instances: None,
            limits: ModelCheckLimits::default(),
        },
        &Cancel::new(),
    )
    .expect_err("the liveness engine does not read the instance bound");
    assert!(
        matches!(refusal, ModelCheckRefusal::InstanceLiveness),
        "{refusal:?}"
    );
    assert_eq!(
        refusal.to_string(),
        "unsupported_construct/not-yet-implemented: EN-1 does not decide a liveness clause \
         over an `activation on each` protocol yet"
    );
}

/// FR-212 (TC-657): an authored `each` fairness member over a node a
/// replicated role owns resolves to its attempt node and refuses
/// `ProtocolFairnessNotEmitted` at S4, as over a node of a static role; it
/// neither panics nor drops the member.
#[test]
fn an_each_fairness_member_over_a_role_owned_node_refuses_as_over_any_node() {
    let run = "sequence Main { attempt Work by r on M::Cell::inc contracts [IncPre, IncPost] \
               as (worked: Boolean) { true }; }";
    let claim = "temporal Live using t0 over (k: M::Cell) clock \"model-steps\" \
                 on origin { fair weak each R::Work; eventually holds(k.v = 1) }\n";
    let body = unit(ORIGIN, &role(1, "workflow"), "", run) + claim;
    let refusal = crate::scheduler_fairness::check_refusal(&body);
    assert!(
        matches!(
            crate::scheduler_fairness::temporal(&refusal),
            qsl_semantics::check::TemporalCause::ProtocolFairnessNotEmitted { .. }
        ),
        "{refusal:?}"
    );
}

/// The state the `kind` step of protocol instance `instance` naming `object`
/// reaches from `reached[at]`.
fn after_in(
    system: &System<'_>,
    reached: &[Reached],
    at: usize,
    kind: StepKind,
    object: &str,
    instance: &str,
) -> usize {
    let found: Vec<usize> = reached[at]
        .steps
        .iter()
        .filter(|(step, _)| {
            let identity = identity(system, step);
            step.kind() == kind
                && named(&identity["object"]) == object
                && identity["instance"] == instance
        })
        .map(|(_, next)| *next)
        .collect();
    assert_eq!(found.len(), 1, "one {kind:?} of {object} in {instance}");
    found[0]
}

/// FR-208-AC-4 (TC-653): under `activation on each` with protocol instances
/// 0 and 1 live and `spawn` enabled for `c` in each, the two `spawn(r, c)`
/// steps have different identities that differ in the instance ordinal and
/// hold no `thread` member; so do the two `retire(r, c)` steps.
#[trace("TC-653", "FR-208-AC-4")]
#[test]
fn spawn_and_retire_identities_carry_the_protocol_instance() {
    let run = "sequence Main { attempt T by w on M::Cell::touch contracts [TouchPre] \
               as (t: Boolean) { x = 0 }; }";
    let world = World::new(
        &unit(
            "on each (x: M::Level) when (x = 0)",
            &role(1, "until(r.v >= 0)"),
            "",
            run,
        ),
        &["c"],
        &[0],
    );
    let subject = world.subject();
    let system = sc_system(&subject).with_max_live_instances(2);
    let reached = reachable(&system);
    let activate = |at: usize| -> usize {
        reached[at]
            .steps
            .iter()
            .find(|(step, _)| step.kind() == StepKind::Activate)
            .map(|(_, next)| *next)
            .expect("an activate")
    };
    let both = activate(activate(0));

    let identities = |at: usize, kind: StepKind| -> Vec<Json> {
        reached[at]
            .steps
            .iter()
            .filter(|(step, _)| step.kind() == kind)
            .map(|(step, _)| identity(&system, step))
            .collect()
    };
    let spawns = identities(both, StepKind::Spawn);
    assert_eq!(spawns.len(), 2);
    assert_ne!(spawns[0], spawns[1]);
    let mut ordinals: Vec<&Json> = spawns.iter().map(|spawn| &spawn["instance"]).collect();
    ordinals.sort_by_key(|ordinal| ordinal.as_str().map(str::to_owned));
    assert_eq!(ordinals, ["0", "1"]);
    assert!(spawns.iter().all(|spawn| spawn.get("thread").is_none()));
    assert!(spawns.iter().all(|spawn| spawn.get("node").is_none()));

    let spawned = after_in(&system, &reached, both, StepKind::Spawn, "c", "0");
    let spawned = after_in(&system, &reached, spawned, StepKind::Spawn, "c", "1");
    let retires = identities(spawned, StepKind::Retire);
    assert_eq!(retires.len(), 2);
    assert_ne!(retires[0], retires[1]);
    let mut ordinals: Vec<&Json> = retires.iter().map(|retire| &retire["instance"]).collect();
    ordinals.sort_by_key(|ordinal| ordinal.as_str().map(str::to_owned));
    assert_eq!(ordinals, ["0", "1"]);
    assert!(retires.iter().all(|retire| retire.get("thread").is_none()));
}

/// FR-208-AC-5 (TC-653): a channel `from r to w` with `r` replicated, `max 2`
/// and two live instances gives two `send` steps naming `c` and `d`; a
/// channel `from w to r` gives two `receive` steps likewise.
#[trace("TC-653", "FR-208-AC-5")]
#[test]
fn a_send_and_a_receive_owned_by_a_replicated_role_are_taken_by_each_instance() {
    let world_of = |channel: &str| {
        let run = "parallel Both { branch s send Tx via ch as (tx: Boolean) { tx }; \
                   branch q receive Rx via ch of s::Tx as (rx: Boolean) { true }; } \
                   join all [s, q];";
        World::new(
            &unit(ORIGIN, &role(2, "workflow"), channel, run),
            &["c", "d"],
            &[0],
        )
    };
    let first = |reached: &[Reached], at: usize, kind: StepKind| {
        reached[at]
            .steps
            .iter()
            .find(|(step, _)| step.kind() == kind)
            .map(|(_, next)| *next)
            .unwrap_or_else(|| panic!("a {kind:?} from {at}"))
    };
    // Both cells spawned, then the `parallel` forked.
    let forked = |system: &System<'_>, reached: &[Reached]| {
        let c = after(system, reached, 0, StepKind::Spawn, "c");
        let both = after(system, reached, c, StepKind::Spawn, "d");
        first(reached, both, StepKind::Fork)
    };
    let actors_of = |system: &System<'_>, reached: &[Reached], at: usize, kind: StepKind| {
        let mut named: Vec<String> = reached[at]
            .steps
            .iter()
            .filter(|(step, _)| step.kind() == kind)
            .filter_map(|(step, _)| identity(system, step).get("actor").map(named))
            .collect();
        named.sort();
        named
    };

    let sends = world_of(
        "channel ch from r to w carries Boolean ordering unordered \
         delivery exactly-once-premise capacity 1 overflow block;",
    );
    let subject = sends.subject();
    let system = sc_system(&subject);
    let reached = reachable(&system);
    let at = forked(&system, &reached);
    assert_eq!(actors_of(&system, &reached, at, StepKind::Send), ["c", "d"]);

    let receives = world_of(
        "channel ch from w to r carries Boolean ordering unordered \
         delivery exactly-once-premise capacity 1 overflow block;",
    );
    let subject = receives.subject();
    let system = sc_system(&subject);
    let reached = reachable(&system);
    let at = forked(&system, &reached);
    let sent = first(&reached, at, StepKind::Send);
    assert_eq!(
        actors_of(&system, &reached, sent, StepKind::Receive),
        ["c", "d"]
    );
}

/// FR-209 "The bound" (TC-654, ADR-013 O-09, QSpec FR-428-AC-3): this test
/// cannot fail today. The request writer and `ItemObligation` take no
/// `max_live_instances`, so equality holds by their signatures; it pins the
/// spec for the day a protocol bound is passed to the writer. Two
/// protocol check requests that differ only in `max_live_instances` write
/// one item with one obligation identity through the request writer, because
/// the bound is a method bound outside the subject.
#[trace("TC-654")]
#[test]
fn requests_differing_only_in_max_live_instances_share_one_obligation_identity() {
    let run = format!("sequence Main {{ {} }}", touch("T"));
    let world = World::new(
        &(unit(EACH, "", "", &run) + &claim("k.v <= 3")),
        &["c"],
        &[0],
    );
    let letters = CellV { holds: |v| v <= 3 };
    let request = |max_live_instances| ProtocolCheckRequest {
        subject: world.subject(),
        item: world.claim(&letters),
        max_depth: None,
        max_store_buffer: None,
        max_live_instances,
        limits: ModelCheckLimits::default(),
    };
    let [clause] = world.checked.graph().temporal_clauses() else {
        panic!("one declared clause");
    };
    let obligation = |request: &ProtocolCheckRequest<'_>| ItemObligation {
        subject: SubjectIdentity::of(&request.subject.model).expect("the package emits"),
        kind: ObligationKind::Authored(node_occurrence(clause)),
    };
    let mut written = ModelRequest::new();
    let mut indices = Vec::new();
    let requests = [request(Some(1)), request(Some(5)), request(None)];
    for each in &requests {
        let context = SubjectContext::new(&each.subject.model).expect("the package emits");
        indices.push(
            context
                .author_clause(&mut written, clause)
                .expect("the clause has a record"),
        );
    }
    assert_eq!(indices[0], indices[1]);
    assert_eq!(indices[1], indices[2]);
    assert_eq!(written.items().len(), 1);
    assert_eq!(obligation(&requests[0]), obligation(&requests[1]));
    assert_eq!(obligation(&requests[1]), obligation(&requests[2]));
}
