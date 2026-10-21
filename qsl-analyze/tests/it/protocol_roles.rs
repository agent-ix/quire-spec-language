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
use qsl_eval::model_subject::{ProtocolSubject, RoleBinding};
use qsl_eval::simulation::model::TerminalClass;
use qsl_eval::simulation::protocol::{
    visible, AnchorName, BlockedThread, ClaimReads, ProtocolLocation, ProtocolSystem,
    ProtocolTransition, RestKind, RestPoint, Sc, StepKind, ThreadPath, WaitCause,
};
use qsl_eval::simulation::{Location, SubjectSystem, TransitionSystem};
use qsl_package::CheckedPackage;
use qsl_semantics::model::observation::DocumentRef;
use serde_json::Value as Json;

use crate::fill::{
    contracts, json_of, model_subject, package, reachable, sc_system, source_memory, v, Documents,
    Reached, PACKAGE,
};

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
    for (at, state) in reached.iter().enumerate() {
        // `retire` is enabled exactly where `p` holds.
        for object in objects(&system, &reached, at, StepKind::Retire) {
            assert!(v(state.state.model(), &object) >= 1, "state {at}");
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
        respawned,
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

/// FR-211-AC-5 (TC-656), the role part: a node owned by a replicated role
/// whose every object is retired, so that it has no live instance and no
/// `spawn` is enabled, deadlocks with the thread `NoRoleInstance{r}`; where a
/// `spawn` is enabled the thread has no such cause and the state is not
/// terminal.
#[trace("TC-656", "FR-211-AC-5")]
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
        SubjectSystem::terminal_class(&system, &reached[terminal[0]].state, false)
            .expect("a class"),
        TerminalClass::Deadlocked
    );

    // At the initial state `spawn` is enabled: no `NoRoleInstance`.
    let initial = SubjectSystem::blocked(&system, &reached[0].state, &[]).expect("blocked");
    assert!(initial
        .iter()
        .all(|thread| !matches!(thread.cause, WaitCause::NoRoleInstance { .. })));
    assert_eq!(
        SubjectSystem::terminal_class(&system, &reached[0].state, true).expect("a class"),
        TerminalClass::NotTerminal
    );
}

/// FR-211-AC-5 (TC-656), the role part for a `receive`: a `receive` owned by
/// a replicated role (its channel's `to` role) whose every object is retired
/// gives `NoRoleInstance{r}` and not `Constraint`, `HeadBlocked` or
/// `EmptyQueue`, although its send's entry is queued.
#[trace("TC-656", "FR-211-AC-5")]
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
    let causes: Vec<WaitCause> = reached
        .iter()
        .filter(|state| state.steps.is_empty() && state.state.has_live_instance())
        .flat_map(|state| {
            SubjectSystem::blocked(&system, &state.state, &[]).expect("the blocked threads")
        })
        .filter(|thread| thread.rest_point.node.as_deref() == Some("Both/q/Rx"))
        .map(|thread| thread.cause)
        .collect();
    assert_eq!(
        causes,
        [WaitCause::NoRoleInstance {
            role: "r".to_owned()
        }]
    );
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
    let class = |at: usize| {
        SubjectSystem::terminal_class(&system, &reached[at].state, !reached[at].steps.is_empty())
            .expect("a class")
    };

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
            || SubjectSystem::terminal_class(&system, &state.state, !state.steps.is_empty())
                .expect("a class")
                == TerminalClass::NotTerminal
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
