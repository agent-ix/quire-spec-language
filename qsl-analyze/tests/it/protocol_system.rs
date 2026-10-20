// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-205, FR-206, FR-210, FR-211, FR-215 and FR-217 (TC-650, TC-651,
//! TC-655, TC-656, TC-660, TC-662): `ProtocolSystem<Sc>` over ADR-027 §7's
//! `Fill` and its variants, EN-1 over a protocol subject, and the replay and
//! settlement of its outcomes.

use std::borrow::Cow;
use std::collections::BTreeSet;

use ix_trace_rs::trace;
use qsl_analyze::model_check::{check_protocol, ModelCheckRefusal, ProtocolCheckRequest};
use qsl_eval::model_subject::ProtocolSubject;
use qsl_eval::simulation::model::{
    ExpansionLimits, ModelFinding, ModelSimState, ModelSystem, TerminalClass,
};
use qsl_eval::simulation::protocol::{
    binder_root, explore_protocol, BlockedThread, Disposition, GatedStep, MemoryModel, MemoryStep,
    ProtocolRefusal, ProtocolState, ProtocolSystem, ProtocolTransition, RestKind, RestPoint, Sc,
    ScComponent, SendOutcome, StepKind, ThreadPath, WaitCause,
};
use qsl_eval::simulation::{Limits, NotSimulated, Outcome, SubjectSystem, TransitionSystem};
use qsl_foundation::bound::DomainKey;
use qsl_foundation::diagnostic::Code;
use qsl_package::CheckedPackage;
use qsl_replay::{
    deadlock_freedom_clause, replay_protocol_trace, settle_protocol_check, CounterexampleKind,
    InconclusiveCause, ModelCheckItem, ModelCheckLimits, ModelCheckOutcome, ModelCheckRecord,
    ModelStep, ModelTraceRefusal, ModelTraceReplayResult, ModelTraceSettlement, ModelTraceSubject,
    ProofBasis, ProtocolCertificateRequest, ProtocolTraceSubject, ReplayLimits, ReplayRefusal,
    ResultLabel, TemporalCounterexample, TerminalValue, UndefinedLocus,
};
use qsl_route::deadlock::{protocol_deadlock_freedom_items, ProtocolTemporalItem};
use qsl_semantics::model::observation::DocumentRef;
use quire_canonical::{Encode, Error, Sink, Writer};
use quire_exact::Cancel;
use quire_semantic_value::fairness::FairnessConstraint;
use serde_json::{json, Value as Json};

use crate::counter::Letters;
use crate::fill::{
    attempt, compiled, contracts, document, fill_unit, instance, json_of, package, package_id,
    projections, protocol, reachable, sc_system, subject, unit_text, universe, v, Documents,
    Reached, PACKAGE,
};
use crate::model_settle::{envelope_at, request};

/// A replay that reads no atom: the deadlock-freedom item has none.
const NO_ATOMS: Letters = Letters { atom: |_| None };

/// The root thread of instance 0.
fn root() -> ThreadPath {
    ThreadPath::root(0)
}

/// The branch thread `label` of `Main/Both`'s first instance.
fn branch(label: &str) -> ThreadPath {
    root().child("Main/Both", label, 0)
}

/// One case: the unit `body`, its documents, its initial snapshots, and its
/// checked package.
pub(crate) struct Case {
    body: String,
    documents: Documents,
    initial: Vec<DocumentRef>,
    checked: CheckedPackage,
}

impl Case {
    /// `body` from one initial snapshot per value of `c.v` in `values`.
    pub(crate) fn new(body: &str, values: &[i64]) -> Self {
        let mut documents = Documents::new();
        let initial = values
            .iter()
            .enumerate()
            .map(|(index, value)| documents.start(&format!("s{index}"), &[("c", *value)]))
            .collect();
        Self {
            body: body.to_owned(),
            documents,
            initial,
            checked: package(body),
        }
    }

    /// The protocol subject of `protocol` with `k` bound to `c`.
    pub(crate) fn subject(&self, protocol: &str) -> ProtocolSubject<'_> {
        subject(
            &self.checked,
            &self.documents,
            self.initial.clone(),
            protocol,
            "c",
        )
    }

    /// EN-1's deadlock-freedom outcome over `protocol`.
    pub(crate) fn deadlock(&self, protocol: &str) -> ModelCheckOutcome<ProtocolTransition> {
        check_protocol(
            ProtocolCheckRequest {
                subject: self.subject(protocol),
                item: ModelCheckItem::DeadlockFreedom,
                max_depth: None,
                limits: ModelCheckLimits::default(),
            },
            &Cancel::new(),
        )
        .expect("the protocol subject checks")
    }

    /// FR-217's replay of `payload` against this case.
    fn replay(
        &self,
        protocol: &str,
        payload: TemporalCounterexample<ProtocolTransition>,
    ) -> Result<ModelTraceReplayResult, ReplayRefusal> {
        self.replay_against(self, protocol, payload)
    }

    /// FR-217's replay of `payload`, recorded over `recorded`'s package,
    /// against this case's unit and documents.
    fn replay_against(
        &self,
        recorded: &Case,
        protocol: &str,
        payload: TemporalCounterexample<ProtocolTransition>,
    ) -> Result<ModelTraceReplayResult, ReplayRefusal> {
        let unit = unit_text(&self.body).into_bytes();
        let recorded_id = package_id(&recorded.body);
        let (node, occurrence) = deadlock_freedom_clause(&recorded_id);
        replay_protocol_trace(
            request(
                &document(),
                self.documents.snapshots.iter(),
                &unit,
                package_id(&self.body),
            ),
            &ProtocolTraceSubject {
                model: ModelTraceSubject {
                    initial: self.initial.clone(),
                    universes: universe(&["c"]),
                    expansion: ExpansionLimits {
                        max_candidates: 1_000_000,
                    },
                },
                protocol: protocol.to_owned(),
                over: instance("c"),
                roles: self.subject(protocol).roles,
            },
            &envelope_at(&unit, recorded_id, payload, node, occurrence),
            &NO_ATOMS,
            ReplayLimits::default(),
        )
    }

    /// FR-127's record of `outcome` over `protocol`, its counterexample
    /// replayed by FR-217.
    fn settle(
        &self,
        protocol: &str,
        outcome: &ModelCheckOutcome<ProtocolTransition>,
    ) -> ModelCheckRecord<ProtocolTransition> {
        settle_protocol_check(
            outcome,
            &ModelCheckLimits::default(),
            &package_id(&self.body),
            &ProtocolCertificateRequest {
                subject: self.subject(protocol),
                item: ModelCheckItem::DeadlockFreedom,
            },
            &Cancel::new(),
            |payload| self.replay(protocol, payload.clone()),
        )
    }
}

/// The counterexample of a `Violated` outcome.
fn violated(
    outcome: &ModelCheckOutcome<ProtocolTransition>,
) -> TemporalCounterexample<ProtocolTransition> {
    match outcome {
        ModelCheckOutcome::Violated { counterexample, .. } => counterexample.as_ref().clone(),
        other => panic!("expected a violation, got {other:?}"),
    }
}

/// Each step's kind and node, in order.
fn steps(prefix: &[ModelStep<ProtocolTransition>]) -> Vec<(StepKind, String)> {
    prefix
        .iter()
        .map(|step| {
            (
                step.transition.kind(),
                step.transition.node().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

/// The kind, node and target of each of `reached[at]`'s steps.
fn shape(reached: &[Reached], at: usize) -> Vec<(StepKind, String, usize)> {
    reached[at]
        .steps
        .iter()
        .map(|(transition, next)| {
            (
                transition.kind(),
                transition.node().unwrap_or_default().to_owned(),
                *next,
            )
        })
        .collect()
}

/// ADR-027 §7's deadlock's blocked threads (FR-211-AC-2), in ascending JCS
/// bytes of their paths: the root at `join Both` waiting for `right`, and
/// `right` at `B` with `TwoPre` false.
fn fill_blocked() -> Vec<BlockedThread> {
    vec![
        BlockedThread {
            path: root(),
            rest_point: RestPoint {
                kind: RestKind::Join,
                node: Some("Main/Both".to_owned()),
            },
            cause: WaitCause::JoinWaiting {
                threads: vec![branch("right")],
            },
        },
        BlockedThread {
            path: branch("right"),
            rest_point: RestPoint {
                kind: RestKind::Node,
                node: Some("Main/Both/right/B".to_owned()),
            },
            cause: WaitCause::Precondition {
                clause: "TwoPre".to_owned(),
            },
        },
    ]
}

/// FR-101's exploration of `system`: its state and transition counts.
fn explored(system: &ProtocolSystem<'_, Sc>) -> (usize, usize) {
    let exploration =
        explore_protocol(system, 1 << 20, Limits::default(), || false).expect("explores");
    let Outcome::Exhaustive(stats) = exploration.outcome else {
        panic!("{:?}", exploration.outcome);
    };
    (stats.states, stats.transitions)
}

/// FR-206-AC-1 (TC-651): over `Fill` with `TwoPre: self.v = 0` the reachable
/// states are exactly ADR-027 §7's s0 to s6 with its table's steps: seven
/// states and six transitions, and no thread rests inside a `sequence`.
#[trace("TC-651", "FR-206-AC-1")]
#[test]
fn fill_reaches_exactly_the_seven_states_of_its_table() {
    let case = Case::new(&fill_unit("self.v = 0", ""), &[0]);
    let subject = case.subject("Fill");
    let system = sc_system(&subject);
    let reached = reachable(&system);
    assert_eq!(reached.len(), 7);
    let step = |kind, node: &str, to| (kind, node.to_owned(), to);
    let (a, b) = ("Main/Both/left/A", "Main/Both/right/B");
    assert_eq!(
        shape(&reached, 0),
        vec![step(StepKind::Fork, "Main/Both", 1)]
    );
    assert_eq!(
        shape(&reached, 1),
        vec![step(StepKind::Attempt, a, 2), step(StepKind::Attempt, b, 3)]
    );
    assert_eq!(shape(&reached, 2), Vec::new());
    assert_eq!(shape(&reached, 3), vec![step(StepKind::Attempt, a, 4)]);
    assert_eq!(
        shape(&reached, 4),
        vec![step(StepKind::Join, "Main/Both", 5)]
    );
    assert_eq!(shape(&reached, 5), vec![step(StepKind::Finish, "Done", 6)]);
    assert_eq!(shape(&reached, 6), Vec::new());
    let values: Vec<i64> = reached
        .iter()
        .map(|state| v(state.state.model(), "c"))
        .collect();
    assert_eq!(values, [0, 0, 1, 2, 3, 3, 3]);
    let rests: BTreeSet<Option<String>> = reached
        .iter()
        .flat_map(|state| system.threads(&state.state).expect("threads"))
        .map(|thread| thread.rest_point.node)
        .collect();
    let allowed: BTreeSet<Option<String>> =
        [None, Some("Main/Both"), Some(a), Some(b), Some("Done")]
            .into_iter()
            .map(|node| node.map(str::to_owned))
            .collect();
    assert_eq!(rests, allowed);
    assert_eq!(explored(&system), (7, 6));
}

/// FR-215-AC-1 (TC-660): FR-101's engine reaches seven states and six
/// transitions; at s1 `successors` gives `attempt(A)` then `attempt(B)`, in
/// ascending JCS bytes of their identities, each of which serializes with
/// `"type":"protocol-transition"`, its step kind, thread path, node,
/// operation and argument vector.
#[trace("TC-660", "FR-215-AC-1")]
#[test]
fn an_attempt_identity_serializes_its_step_thread_node_operation_and_arguments() {
    let case = Case::new(&fill_unit("self.v = 0", ""), &[0]);
    let subject = case.subject("Fill");
    let system = sc_system(&subject);
    let reached = reachable(&system);
    let expansion = system.successors(&reached[1].state).expect("s1 expands");
    let identities: Vec<Vec<u8>> = expansion
        .successors
        .iter()
        .map(|(transition, _)| system.transition_key(transition).expect("an identity"))
        .collect();
    assert_eq!(identities.len(), 2);
    assert!(identities[0] < identities[1]);
    let first = json_of(&identities[0]);
    assert_eq!(first["type"], "protocol-transition");
    assert_eq!(first["step"], "attempt");
    assert_eq!(first["node"], "Main/Both/left/A");
    assert_eq!(
        first["thread"],
        json!({"instance": "0", "type": "thread-path",
            "steps": [{"node": "Main/Both", "branch": "left", "ordinal": "0"}]})
    );
    assert!(first["operation"]
        .as_str()
        .is_some_and(|operation| operation.contains("ix://test/fill/Cell/inc")));
    assert_eq!(first["arguments"], json!([]));
    assert_eq!(json_of(&identities[1])["node"], "Main/Both/right/B");
    assert_eq!(explored(&system), (7, 6));
}

/// The `control` entries of `state`'s key, asserting they are sorted by
/// their keys' JCS bytes.
fn control(system: &ProtocolSystem<'_, Sc>, state: &ProtocolState<Sc>) -> Vec<Json> {
    let (bytes, _) = system.state_key(state).expect("a key");
    let entries = json_of(&bytes)["control"]["entries"]
        .as_array()
        .expect("entries")
        .clone();
    let keys: Vec<String> = entries
        .iter()
        .map(|entry| serde_json::to_string(&entry["key"]).expect("a key"))
        .collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted, "control is sorted by JCS bytes");
    entries
}

/// The `thread-state` of `path` among `entries`.
fn thread_state(entries: &[Json], path: &ThreadPath) -> Json {
    let steps: Vec<Json> = path
        .steps
        .iter()
        .map(|step| {
            json!({"node": step.node, "branch": step.branch,
                "ordinal": step.ordinal.to_string()})
        })
        .collect();
    let wanted = json!({"instance": "0", "steps": steps, "type": "thread-path"});
    entries
        .iter()
        .find(|entry| entry["key"] == wanted)
        .map(|entry| entry["value"].clone())
        .unwrap_or_else(|| panic!("{path:?} is in control"))
}

/// The steps a behaviour has taken, each as its kind and node.
type Taken = Vec<(StepKind, String)>;

/// Walk every behaviour from `at`, asserting that each state's binder
/// environments hold exactly what `expected` gives for the steps taken to
/// reach it: each entry as its binder node's last segment and the thread
/// whose environment holds it.
fn environments_follow(
    system: &ProtocolSystem<'_, Sc>,
    reached: &[Reached],
    at: usize,
    taken: &mut Taken,
    expected: &dyn Fn(&Taken) -> BTreeSet<(String, ThreadPath)>,
) {
    let held: BTreeSet<(String, ThreadPath)> = system
        .binders(&reached[at].state)
        .expect("binders")
        .into_iter()
        .map(|(node, thread)| {
            let last = node.rsplit('/').next().unwrap_or_default().to_owned();
            (last, thread)
        })
        .collect();
    assert_eq!(held, expected(taken), "after {taken:?}");
    for (transition, next) in &reached[at].steps {
        taken.push((
            transition.kind(),
            transition.node().unwrap_or_default().to_owned(),
        ));
        environments_follow(system, reached, *next, taken, expected);
        taken.pop();
    }
}

/// Whether `taken` holds a step of `kind` at a node whose last segment is
/// `node`.
fn took(taken: &Taken, kind: StepKind, node: &str) -> bool {
    taken
        .iter()
        .any(|(step, at)| *step == kind && at.rsplit('/').next() == Some(node))
}

/// FR-205-AC-2 (TC-650): `Fill`'s initial key holds the root at `fork
/// Both`, empty `observations`, `queues` and `bounds`, and `roles` binding
/// `w`; after `fork(Both)` its `control` holds the root at `join Both` and
/// both branches at their attempts, running; its maps are sorted and it
/// serializes twice equal. No state of `Fill` holds a binder. In a variant
/// whose node after the join reads `a`, `a` is held from `attempt(A)` to
/// that node's step, under `left` until the join and under the root after
/// it, and in no state after the node's step; so it is when `left` takes a
/// second attempt after `A`; and a binder of a branch of an all-joined
/// `parallel` inside `left` is held by that branch through its join, by
/// `left` through `Both`'s join, and by the root until its reader steps.
#[trace("TC-650", "FR-205-AC-2")]
#[test]
fn a_binder_is_held_by_each_environment_that_reaches_its_reader() {
    let case = Case::new(&fill_unit("self.v = 0", ""), &[0]);
    let subject = case.subject("Fill");
    let system = sc_system(&subject);
    let initial = system.initial();
    assert_eq!(initial.len(), 1);
    let (bytes, _) = system.state_key(&initial[0]).expect("a key");
    let (again, _) = system.state_key(&initial[0]).expect("a key");
    assert_eq!(bytes, again);
    let key = json_of(&bytes);
    let empty = json!({"entries": [], "type": "map"});
    for member in ["observations", "queues", "bounds", "memory"] {
        assert_eq!(key[member], empty, "{member}");
    }
    assert_eq!(
        key["roles"]["entries"],
        json!([{"key": {"instance": "0", "role": "w", "type": "role"},
            "value": {"type": "text", "value": PACKAGE}}])
    );
    assert_eq!(
        thread_state(&control(&system, &initial[0]), &root())["rest"],
        json!({"kind": "fork", "node": "Main/Both", "type": "rest"})
    );
    let forked = &system.successors(&initial[0]).expect("expands").successors[0].1;
    let after = control(&system, forked);
    assert_eq!(thread_state(&after, &root())["rest"]["kind"], "join");
    for (label, node) in [("left", "Main/Both/left/A"), ("right", "Main/Both/right/B")] {
        let thread = thread_state(&after, &branch(label));
        assert_eq!(thread["rest"]["node"], node);
        assert_eq!(thread["disposition"], "running");
    }
    for state in reachable(&system) {
        assert!(system.binders(&state.state).expect("binders").is_empty());
    }

    // Variants: `A` is `flag`, whose Boolean result binds `a`, and `C` after
    // the join reads it. `left` holds `a` until the join, then the root
    // holds it until `C`'s step.
    let reader = attempt("C", "touch", "TouchPre", "cc", "a or not a");
    let lefts = [
        attempt("A", "flag", "FlagPre, FlagPost", "a", "true"),
        format!(
            "sequence L {{ {} {} }}",
            attempt("A", "flag", "FlagPre, FlagPost", "a", "true"),
            attempt("A2", "touch", "TouchPre", "a2", "true"),
        ),
    ];
    for left in &lefts {
        let run = format!(
            "sequence Main {{ parallel Both {{ branch left {left} branch right {} }} \
             join all [left, right]; {reader} }}",
            attempt("B", "touch", "TouchPre", "b", "true"),
        );
        let variant = Case::new(
            &(contracts("true") + &protocol("Read", "", &run, "d")),
            &[0],
        );
        let subject = variant.subject("Read");
        let system = sc_system(&subject);
        let reached = reachable(&system);
        let expected = |taken: &Taken| {
            let holder =
                if !took(taken, StepKind::Attempt, "A") || took(taken, StepKind::Attempt, "C") {
                    None
                } else if took(taken, StepKind::Join, "Both") {
                    Some(root())
                } else {
                    Some(branch("left"))
                };
            holder
                .map(|thread| ("A".to_owned(), thread))
                .into_iter()
                .collect()
        };
        environments_follow(&system, &reached, 0, &mut Vec::new(), &expected);
        assert!(reached.iter().any(|state| state
            .steps
            .iter()
            .any(|(transition, _)| transition.node() == Some("Main/C"))));
    }

    // A variant: `left` forks `R`, whose branch `x` binds `b`, read after
    // `Both`'s join. `x` keeps `b` through `join(R)`, `left` through
    // `join(Both)`, and the root until the reading node's step.
    let run = format!(
        "sequence Main {{ parallel Both {{ branch left sequence L {{ parallel R {{ branch x {} branch y {} }} \
         join all [x, y]; {} }} branch right {} }} join all [left, right]; {} }}",
        attempt("X", "flag", "FlagPre, FlagPost", "b", "true"),
        attempt("Y", "touch", "TouchPre", "y", "true"),
        attempt("L2", "touch", "TouchPre", "l2", "true"),
        attempt("B", "touch", "TouchPre", "bb", "true"),
        attempt("C", "touch", "TouchPre", "cc", "b or not b"),
    );
    let variant = Case::new(
        &(contracts("true") + &protocol("Read", "", &run, "d")),
        &[0],
    );
    let subject = variant.subject("Read");
    let system = sc_system(&subject);
    let reached = reachable(&system);
    let x = branch("left").child("Main/Both/left/L/R", "x", 0);
    let expected = |taken: &Taken| {
        let holder = if !took(taken, StepKind::Attempt, "X") || took(taken, StepKind::Attempt, "C")
        {
            None
        } else if took(taken, StepKind::Join, "Both") {
            Some(root())
        } else if took(taken, StepKind::Join, "R") {
            Some(branch("left"))
        } else {
            Some(x.clone())
        };
        holder
            .map(|thread| ("X".to_owned(), thread))
            .into_iter()
            .collect()
    };
    environments_follow(&system, &reached, 0, &mut Vec::new(), &expected);
    assert!(reached
        .iter()
        .any(|state| !system.binders(&state.state).expect("binders").is_empty()));
}

/// Part of FR-205-AC-1 (TC-650): the `Fill` subject builds; naming an
/// absent protocol refuses `invalid_runtime_input`/`invalid-value`, leaving
/// `w` unbound refuses `invalid_runtime_input`/`wrong-role-mapping`, and
/// binding `k` outside the universe refuses `invalid-value`. Not covered:
/// the obligation identities of two subjects that differ only in the `over`
/// binding or the `terminal` member.
#[test]
fn a_protocol_subject_refuses_its_bindings() {
    let case = Case::new(&fill_unit("self.v = 0", ""), &[0]);
    let build = |subject: &ProtocolSubject<'_>| {
        let model = ModelSystem::new(subject.model.simulation.clone()).expect("admits");
        ProtocolSystem::new(model, subject, Sc).map(|_| ())
    };
    assert_eq!(build(&case.subject("Fill")), Ok(()));
    let refusal = build(&case.subject("Absent")).expect_err("no such protocol");
    assert_eq!(
        (refusal.code(), refusal.cause()),
        (Code::InvalidRuntimeInput, "invalid-value")
    );
    let mut unbound = case.subject("Fill");
    unbound.roles.clear();
    let refusal = build(&unbound).expect_err("w is unbound");
    assert_eq!(
        refusal,
        ProtocolRefusal::RoleMapping {
            role: "w".to_owned()
        }
    );
    assert_eq!(
        (refusal.code(), refusal.cause()),
        (Code::InvalidRuntimeInput, "wrong-role-mapping")
    );
    let mut outside = case.subject("Fill");
    outside.over = instance("d");
    let refusal = build(&outside).expect_err("d is outside the universe");
    assert!(matches!(refusal, ProtocolRefusal::OverBinding { .. }));
    assert_eq!(refusal.cause(), "invalid-value");
}

/// Part of FR-205-AC-4 (TC-650): two initial snapshots give two initial
/// protocol states, each with the root at `fork Both`. Not covered: an `on
/// each` protocol.
#[test]
fn each_initial_snapshot_gives_one_initial_protocol_state() {
    let case = Case::new(&fill_unit("self.v = 0", ""), &[0, 1]);
    let subject = case.subject("Fill");
    let system = sc_system(&subject);
    let initial = system.initial();
    assert_eq!(initial.len(), 2);
    for state in &initial {
        let threads = system.threads(state).expect("threads");
        assert_eq!(threads.len(), 1);
        assert_eq!(
            threads[0].rest_point,
            RestPoint {
                kind: RestKind::Fork,
                node: Some("Main/Both".to_owned())
            }
        );
    }
}

/// A unit of the contracts and the protocol `P` with `run`.
fn one_protocol(two_pre: &str, run: &str) -> String {
    contracts(two_pre) + &protocol("P", "", run, "d")
}

/// The system of the protocol `P` with `run`, from `c.v = value`: its case.
fn p_case(run: &str, value: i64) -> Case {
    Case::new(&one_protocol("self.v = 0", run), &[value])
}

/// FR-206-AC-2 (TC-651): an attempt of `bump()` from `c.v = 0` gives three
/// steps, to 1, 2 and 3, each equal to the successor `ModelSystem` gives
/// the same application; an attempt whose `contracts` omit `TwoPre` is
/// enabled where only `TwoPre` is false; an attempt whose binder constraint
/// no record satisfies has no step, and its thread waits with cause
/// `constraint`.
#[trace("TC-651", "FR-206-AC-2")]
#[test]
fn an_attempt_is_the_model_application_over_its_selected_clauses() {
    let run = format!(
        "sequence Main {{ {} }}",
        attempt("X", "bump", "BumpPost", "x", "true")
    );
    let case = p_case(&run, 0);
    let subject = case.subject("P");
    let system = sc_system(&subject);
    let start = &system.initial()[0];
    let stepped: BTreeSet<(String, i64)> = system
        .successors(start)
        .expect("expands")
        .successors
        .iter()
        .map(|(transition, state)| {
            assert_eq!(transition.kind(), StepKind::Attempt);
            let application = transition.application().expect("an application");
            (format!("{application:?}"), v(state.model(), "c"))
        })
        .collect();
    let model = ModelSystem::new(subject.model.simulation.clone()).expect("admits");
    let direct: BTreeSet<(String, i64)> = model
        .successors(&model.initial()[0])
        .expect("expands")
        .successors
        .iter()
        .filter(|(transition, _)| transition.operation().contains("Cell/bump"))
        .map(|(transition, state)| (format!("{transition:?}"), v(state, "c")))
        .collect();
    assert_eq!(stepped, direct);
    let values: BTreeSet<i64> = stepped.iter().map(|(_, value)| *value).collect();
    assert_eq!(values, BTreeSet::from([1, 2, 3]));

    // `TwoPre` is false at `c.v = 1`: an attempt that omits it is enabled,
    // one that lists it is not.
    for (listed, enabled) in [("TwoPost", true), ("TwoPre, TwoPost", false)] {
        let run = format!(
            "sequence Main {{ {} }}",
            attempt("Y", "setTwo", listed, "y", "true")
        );
        let case = p_case(&run, 1);
        let subject = case.subject("P");
        let system = sc_system(&subject);
        let successors = system
            .successors(&system.initial()[0])
            .expect("expands")
            .successors;
        assert_eq!(!successors.is_empty(), enabled, "{listed}");
        if enabled {
            assert_eq!(v(successors[0].1.model(), "c"), 2);
        }
    }

    let run = format!(
        "sequence Main {{ {} }}",
        attempt("Z", "flag", "FlagPre, FlagPost", "z", "false")
    );
    let case = p_case(&run, 0);
    let subject = case.subject("P");
    let system = sc_system(&subject);
    let start = &system.initial()[0];
    assert!(system
        .successors(start)
        .expect("expands")
        .successors
        .is_empty());
    assert_eq!(
        system.blocked(start, &[]).expect("blocked threads"),
        vec![BlockedThread {
            path: root(),
            rest_point: RestPoint {
                kind: RestKind::Node,
                node: Some("Main/Z".to_owned())
            },
            cause: WaitCause::Constraint,
        }]
    );
}

/// The `parallel` `Both` of `branches` (label, body) with its `join`.
fn parallel(branches: &[(&str, String)], join: &str) -> String {
    let branches: String = branches
        .iter()
        .map(|(label, body)| format!("branch {label} {body} "))
        .collect();
    format!("sequence Main {{ parallel Both {{ {branches}}} {join}; }}")
}

/// An always-enabled attempt `node`.
pub(crate) fn touch(node: &str) -> String {
    attempt(node, "touch", "TouchPre", &node.to_lowercase(), "true")
}

/// The first state of `reached` with the root at `join Both` and the
/// branch threads' dispositions `wanted`.
fn at_join(
    system: &ProtocolSystem<'_, Sc>,
    reached: &[Reached],
    wanted: &[(&str, Disposition)],
) -> usize {
    reached
        .iter()
        .position(|state| {
            let threads = system.threads(&state.state).expect("threads");
            threads
                .iter()
                .any(|thread| thread.path == root() && thread.rest_point.kind == RestKind::Join)
                && wanted.iter().all(|(label, disposition)| {
                    threads.iter().any(|thread| {
                        thread.path == branch(label) && thread.disposition == *disposition
                    })
                })
        })
        .unwrap_or_else(|| panic!("a state at the join with {wanted:?}"))
}

/// The target of `reached[at]`'s `join(Both)` step, if it has one.
fn join_step(reached: &[Reached], at: usize) -> Option<usize> {
    reached[at]
        .steps
        .iter()
        .find(|(transition, _)| transition.kind() == StepKind::Join)
        .map(|(_, next)| *next)
}

/// Part of FR-206-AC-3 (TC-651): three branches of one always-enabled
/// attempt. `join any … outstanding cancel` is enabled after the first
/// completion and its step leaves no branch thread to step; `outstanding
/// continue` leaves the others running, each still taking its attempt;
/// `join quorum(2)` is enabled after two completions and not after one.
/// Not covered: `join predicate(f)`, which this system refuses as
/// unimplemented.
#[test]
fn a_join_policy_decides_when_its_join_is_enabled() {
    use Disposition::{Completed, Running};
    let branches = [("a", touch("TA")), ("b", touch("TB")), ("c", touch("TC"))];
    let first = [("a", Completed), ("b", Running), ("c", Running)];
    for (outstanding, survive) in [("cancel", false), ("continue", true)] {
        let run = parallel(
            &branches,
            &format!("join any [a, b, c] outstanding {outstanding}"),
        );
        let case = p_case(&run, 0);
        let subject = case.subject("P");
        let system = sc_system(&subject);
        let reached = reachable(&system);
        let joined = join_step(&reached, at_join(&system, &reached, &first))
            .expect("the join is enabled after one completion");
        let running = system
            .threads(&reached[joined].state)
            .expect("threads")
            .into_iter()
            .filter(|thread| thread.disposition == Running && thread.path != root())
            .count();
        let attempts = reached[joined]
            .steps
            .iter()
            .filter(|(transition, _)| transition.kind() == StepKind::Attempt)
            .count();
        let expected = if survive { 2 } else { 0 };
        assert_eq!((running, attempts), (expected, expected), "{outstanding}");
    }
    let run = parallel(&branches, "join quorum(2) [a, b, c] outstanding continue");
    let case = p_case(&run, 0);
    let subject = case.subject("P");
    let system = sc_system(&subject);
    let reached = reachable(&system);
    assert_eq!(
        join_step(&reached, at_join(&system, &reached, &first)),
        None
    );
    let second = [("a", Completed), ("b", Completed), ("c", Running)];
    assert!(join_step(&reached, at_join(&system, &reached, &second)).is_some());
}

/// FR-206-AC-6 (TC-651): `a` one attempt and `b` a sequence of two, joined
/// `any … outstanding continue`, then `finish`: while `b` runs, `finish` is
/// not enabled at the root; once `b` completes it is, and its step removes
/// the instance.
#[trace("TC-651", "FR-206-AC-6")]
#[test]
fn finish_waits_for_every_other_thread_of_its_instance() {
    let run = parallel(
        &[
            ("a", touch("TA")),
            (
                "b",
                format!("sequence Seq {{ {} {} }}", touch("TB"), touch("TC")),
            ),
        ],
        "join any [a, b] outstanding continue",
    );
    let case = p_case(&run, 0);
    let subject = case.subject("P");
    let system = sc_system(&subject);
    let reached = reachable(&system);
    let at_finish = |state: &Reached| {
        system
            .threads(&state.state)
            .expect("threads")
            .iter()
            .any(|thread| {
                thread.path == root() && thread.rest_point.node.as_deref() == Some("Done")
            })
    };
    let b_running = |state: &Reached| {
        system
            .threads(&state.state)
            .expect("threads")
            .iter()
            .any(|thread| thread.path == branch("b") && thread.disposition == Disposition::Running)
    };
    let others_running = |state: &Reached| {
        system
            .threads(&state.state)
            .expect("threads")
            .iter()
            .any(|thread| thread.path != root() && thread.disposition == Disposition::Running)
    };
    let finish = |state: &Reached| {
        state
            .steps
            .iter()
            .find(|(transition, _)| transition.kind() == StepKind::Finish)
            .map(|(_, next)| *next)
    };
    let mut waited = false;
    let mut finished = false;
    for state in reached.iter().filter(|state| at_finish(state)) {
        if b_running(state) {
            waited = true;
            assert_eq!(finish(state), None);
        } else if !others_running(state) {
            finished = true;
            let next = finish(state).expect("finish is enabled");
            assert!(!reached[next].state.has_live_instance());
        }
    }
    assert!(waited && finished);
}

/// `orders` as a set of owned projections.
fn orders(names: &[&[&str]]) -> BTreeSet<Vec<String>> {
    names
        .iter()
        .map(|order| order.iter().map(|name| (*name).to_owned()).collect())
        .collect()
}

/// The attempt-and-finish projections of the protocol `P` with `run`.
fn projected(run: &str) -> BTreeSet<Vec<String>> {
    let case = p_case(run, 0);
    let subject = case.subject("P");
    let system = sc_system(&subject);
    projections(&reachable(&system))
}

/// FR-210-AC-1 (TC-655): three branches of one attempt each give exactly
/// the six orders of the three attempts, whatever the branches' source
/// order.
#[trace("TC-655", "FR-210-AC-1")]
#[test]
fn three_independent_branches_give_every_order() {
    let forward = projected(&parallel(
        &[("a", touch("TA")), ("b", touch("TB")), ("c", touch("TC"))],
        "join all [a, b, c]",
    ));
    let mut every = Vec::new();
    for first in ["TA", "TB", "TC"] {
        for second in ["TA", "TB", "TC"] {
            for third in ["TA", "TB", "TC"] {
                if first != second && second != third && first != third {
                    every.push(vec![first, second, third, "Done"]);
                }
            }
        }
    }
    let every: Vec<&[&str]> = every.iter().map(Vec::as_slice).collect();
    assert_eq!(forward, orders(&every));
    let backward = projected(&parallel(
        &[("c", touch("TC")), ("b", touch("TB")), ("a", touch("TA"))],
        "join all [c, b, a]",
    ));
    assert_eq!(backward, forward);
}

/// FR-210-AC-2 (TC-655): `a` = `X` then `Y`, `b` = `Z`: the projections are
/// exactly `X Y Z`, `X Z Y` and `Z X Y`.
#[trace("TC-655", "FR-210-AC-2")]
#[test]
fn a_sequence_in_a_branch_keeps_its_order() {
    let run = parallel(
        &[
            (
                "a",
                format!("sequence Seq {{ {} {} }}", touch("X"), touch("Y")),
            ),
            ("b", touch("Z")),
        ],
        "join all [a, b]",
    );
    assert_eq!(
        projected(&run),
        orders(&[
            &["X", "Y", "Z", "Done"],
            &["X", "Z", "Y", "Done"],
            &["Z", "X", "Y", "Done"],
        ])
    );
}

/// FR-210-AC-4 (TC-655): QSpec FR-052's split shipments, `S1` = `pack1
/// ship1` and `S2` = `pack2 ship2` joined `all`, then `finish`: the six
/// interleavings that keep each branch's order, each followed by `finish`.
#[trace("TC-655", "FR-210-AC-4")]
#[test]
fn split_shipments_interleave_and_finish_after_both() {
    let run = parallel(
        &[
            (
                "s1",
                format!("sequence One {{ {} {} }}", touch("Pack1"), touch("Ship1")),
            ),
            (
                "s2",
                format!("sequence Two {{ {} {} }}", touch("Pack2"), touch("Ship2")),
            ),
        ],
        "join all [s1, s2]",
    );
    let (p1, s1, p2, s2, done) = ("Pack1", "Ship1", "Pack2", "Ship2", "Done");
    assert_eq!(
        projected(&run),
        orders(&[
            &[p1, s1, p2, s2, done],
            &[p1, p2, s1, s2, done],
            &[p1, p2, s2, s1, done],
            &[p2, p1, s1, s2, done],
            &[p2, p1, s2, s1, done],
            &[p2, s2, p1, s1, done],
        ])
    );
}

/// FR-211-AC-2 (TC-656) with FR-217's replay: over `Fill` with no
/// `terminal` member s6 is intended and s2 deadlocked; the deadlock-freedom
/// item settles `refuted` with the prefix `fork(Both)`, `attempt(A)` and
/// ADR-027 §7's blocked threads, once FR-217's replay reproduces it. With
/// `terminal when k.v = 1` s2 is intended and the item settles `proved`;
/// with `terminal any` the request carries no deadlock-freedom item.
#[trace("TC-656", "FR-211-AC-2")]
#[test]
fn a_protocol_deadlock_names_its_blocked_threads_and_settles_refuted() {
    let case = Case::new(&fill_unit("self.v = 0", ""), &[0]);
    let subject = case.subject("Fill");
    let system = sc_system(&subject);
    let reached = reachable(&system);
    let class = |at: usize| {
        system
            .terminal_class(&reached[at].state, !reached[at].steps.is_empty())
            .expect("classifies")
    };
    assert_eq!(class(6), TerminalClass::Intended);
    assert_eq!(class(2), TerminalClass::Deadlocked);
    let outcome = case.deadlock("Fill");
    let counterexample = violated(&outcome);
    assert_eq!(counterexample.kind, CounterexampleKind::Deadlock);
    assert_eq!(
        steps(&counterexample.prefix),
        vec![
            (StepKind::Fork, "Main/Both".to_owned()),
            (StepKind::Attempt, "Main/Both/left/A".to_owned()),
        ]
    );
    assert_eq!(counterexample.blocked, fill_blocked());
    let record = case.settle("Fill", &outcome);
    assert_eq!(record.label, ResultLabel::Refuted, "{record:?}");
    assert_eq!(record.counterexample, Some(counterexample));

    let intended = Case::new(&fill_unit("self.v = 0", "terminal when { k.v = 1 };"), &[0]);
    let outcome = intended.deadlock("Fill");
    assert!(
        matches!(outcome, ModelCheckOutcome::Holds { .. }),
        "{outcome:?}"
    );
    let record = intended.settle("Fill", &outcome);
    assert_eq!(record.label, ResultLabel::Proved, "{record:?}");

    let any = Case::new(&fill_unit("self.v = 0", "terminal any;"), &[0]);
    let protocol = any.checked.graph().protocol("Fill").expect("Fill");
    let occurrence = deadlock_freedom_clause(&package_id(&any.body)).1;
    let items = [ProtocolTemporalItem {
        occurrence: &occurrence,
        subject: &"Fill",
        protocol,
    }];
    assert!(protocol_deadlock_freedom_items(&items).is_empty());
}

/// Part of FR-211-AC-5 (TC-656, ADR-027 PD-4): each channel wait cause is reported
/// for a thread blocked by exactly that condition, with the channel in its
/// qualified name, in the model checker's deadlock counterexample, and
/// replay recomputes it from the prefix. Every channel has `delivery
/// exactly-once-premise` unless stated. At capacity 1 under `block` a
/// second send waits with `FullQueue`, or `Constraint` when no payload
/// satisfies it although the queue is full. At capacity 2 a `fifo` receive
/// of `Q` behind `P` waits with `HeadBlocked`, and a receive of `P` whose
/// constraint fails with `Constraint`, `unordered` too. At capacity 1,
/// `loss head` displaces `P` and `loss new` loses `Q`, each reported
/// `Displaced`, and so is a `lose(ch, P)` under `at-most-once`; when the
/// last removal of `P`'s entries was a `receive`, the cause is
/// `EmptyQueue`. The root at `finish` while a branch still runs is
/// `FinishWaiting`, naming it. Not covered: the `Pay` compensation fixture
/// (`CommitClosed`), replicated roles (`NoRoleInstance`) and the `tso` fence,
/// which this system refuses.
#[test]
fn each_channel_wait_cause_names_its_condition() {
    let fifo = "ordering fifo by (m: Boolean) { true }";
    let channel = |ordering: &str, delivery: &str, capacity: u64, overflow: &str| {
        format!(
            "channel ch from w to w carries Boolean {ordering} delivery {delivery} \
             capacity {capacity} overflow {overflow};"
        )
    };
    let once = "exactly-once-premise";
    let sends = |q: &str, receive: &str| {
        format!(
            "sequence Main {{ send P via ch as (p: Boolean) {{ p }}; \
             send Q via ch as (q: Boolean) {{ {q} }}; {receive} }}"
        )
    };
    let receive = |name: &str, of: &str, constraint: &str| {
        let binder = name.to_lowercase();
        format!("receive {name} via ch of {of} as ({binder}: Boolean) {{ {constraint} }};")
    };
    let channel_name = || "P::ch".to_owned();
    let cases = [
        (
            channel(fifo, once, 1, "block"),
            sends("q", ""),
            "Q",
            WaitCause::FullQueue {
                channel: channel_name(),
            },
        ),
        (
            channel(fifo, once, 1, "block"),
            sends("false", ""),
            "Q",
            WaitCause::Constraint,
        ),
        (
            channel(fifo, once, 2, "block"),
            sends("q", &receive("RQ", "Q", "true")),
            "RQ",
            WaitCause::HeadBlocked {
                channel: channel_name(),
            },
        ),
        (
            channel(fifo, once, 2, "block"),
            sends("q", &receive("RP", "P", "false")),
            "RP",
            WaitCause::Constraint,
        ),
        (
            channel("ordering unordered", once, 2, "block"),
            sends("q", &receive("RP", "P", "false")),
            "RP",
            WaitCause::Constraint,
        ),
        (
            channel(fifo, once, 1, "loss head"),
            sends("q", &receive("RP", "P", "true")),
            "RP",
            WaitCause::Displaced {
                channel: channel_name(),
            },
        ),
        (
            channel(fifo, once, 1, "loss new"),
            sends("q", &receive("RQ", "Q", "true")),
            "RQ",
            WaitCause::Displaced {
                channel: channel_name(),
            },
        ),
        (
            channel(fifo, "at-most-once", 1, "block"),
            format!(
                "sequence Main {{ send P via ch as (p: Boolean) {{ p }}; {} }}",
                receive("RP", "P", "true")
            ),
            "RP",
            WaitCause::Displaced {
                channel: channel_name(),
            },
        ),
        (
            channel(fifo, once, 1, "loss head"),
            format!(
                "sequence Main {{ repeat Loop by w visible () max 2 while {{ true }} \
                 send P via ch as (p: Boolean) {{ p }}; exhausted sequence Out {{ }} {} {} }}",
                receive("R1", "Loop::P", "true"),
                receive("R2", "Loop::P", "true"),
            ),
            "R2",
            WaitCause::EmptyQueue {
                channel: channel_name(),
            },
        ),
    ];
    for (members, run, node, cause) in cases {
        let case = Case::new(
            &(contracts("self.v = 0") + &protocol("P", &members, &run, "d")),
            &[0],
        );
        let outcome = case.deadlock("P");
        let counterexample = violated(&outcome);
        let [blocked] = &counterexample.blocked[..] else {
            panic!("one waiting thread: {:?}", counterexample.blocked);
        };
        assert_eq!(blocked.cause, cause, "{members} {run}");
        assert!(
            blocked
                .rest_point
                .node
                .as_deref()
                .is_some_and(|at| at.rsplit('/').next() == Some(node)),
            "{blocked:?}"
        );
        let record = case.settle("P", &outcome);
        assert_eq!(record.label, ResultLabel::Refuted, "{run}: {record:?}");
    }

    // The root at `finish` while branch `b` waits at a false `check`.
    let run = format!(
        "parallel Both {{ branch a {} branch b check Never using v {{ false }}; }} \
         join any [a, b] outstanding continue;",
        touch("TA"),
    );
    let case = p_case(&run, 0);
    let outcome = case.deadlock("P");
    let counterexample = violated(&outcome);
    let finishing = counterexample
        .blocked
        .iter()
        .find(|thread| thread.path == root())
        .expect("the root waits");
    assert_eq!(
        finishing.cause,
        WaitCause::FinishWaiting {
            threads: vec![root().child("Both", "b", 0)]
        }
    );
    let record = case.settle("P", &outcome);
    assert_eq!(record.label, ResultLabel::Refuted, "{record:?}");
}

/// FR-211-AC-3 (TC-656): over `Fill` with `TwoPre: self.v <= 1` the
/// deadlock-freedom item holds over ten states and settles `proved`, basis
/// `Exhaustive`; two temporal items over one protocol subject give one
/// deadlock-freedom item.
#[trace("TC-656", "FR-211-AC-3")]
#[test]
fn the_repaired_fill_is_deadlock_free_over_ten_states() {
    let case = Case::new(&fill_unit("self.v <= 1", ""), &[0]);
    let outcome = case.deadlock("Fill");
    let ModelCheckOutcome::Holds { stats, .. } = &outcome else {
        panic!("{outcome:?}");
    };
    assert_eq!(stats.product_states, 10);
    let record = case.settle("Fill", &outcome);
    assert!(
        matches!(
            record.value,
            TerminalValue::Proved {
                basis: ProofBasis::Exhaustive,
                ..
            }
        ),
        "{record:?}"
    );
    let protocol = case.checked.graph().protocol("Fill").expect("Fill");
    let occurrence = deadlock_freedom_clause(&package_id(&case.body)).1;
    let item = ProtocolTemporalItem {
        occurrence: &occurrence,
        subject: &"Fill",
        protocol,
    };
    let derived = protocol_deadlock_freedom_items(&[item, item]);
    assert_eq!(derived.len(), 1);
    assert_eq!(*derived[0].subject(), "Fill");
}

/// FR-215-AC-4 (TC-660): a protocol with an `event` node whose record type
/// has an unbounded `Integer` field returns `RequiresBound` naming that
/// binder and the field before any state is explored. EN-1 over the `Fill` deadlock-freedom item
/// returns `Violated` with `kind: Deadlock`, and over the repaired `Fill`
/// `Holds` over ten states; running each twice gives equal outcomes and
/// byte-equal counterexamples.
#[trace("TC-660", "FR-215-AC-4")]
#[test]
fn a_protocol_model_check_is_deterministic() {
    let run = format!(
        "sequence Main {{ event Tally by w as (tally: Count) {{ true }}; {} }}",
        touch("T")
    );
    let counted = Case::new(
        &format!(
            "record Count {{ n: Integer; }}\n{}",
            one_protocol("self.v = 0", &run)
        ),
        &[0],
    );
    let refusal = check_protocol(
        ProtocolCheckRequest {
            subject: counted.subject("P"),
            item: ModelCheckItem::DeadlockFreedom,
            max_depth: None,
            limits: ModelCheckLimits::default(),
        },
        &Cancel::new(),
    )
    .expect_err("the event binder is unbounded");
    let ModelCheckRefusal::RequiresBound(bound) = refusal else {
        panic!("{refusal:?}");
    };
    let named: Vec<_> = bound.domains.iter().map(|(key, _)| key.clone()).collect();
    assert_eq!(
        named,
        [DomainKey::Node {
            node: binder_root("P", "tally"),
            path: vec![0],
        }]
    );

    let case = Case::new(&fill_unit("self.v = 0", ""), &[0]);
    let first = case.deadlock("Fill");
    assert_eq!(violated(&first).kind, CounterexampleKind::Deadlock);
    assert_eq!(first, case.deadlock("Fill"));
    let repaired = Case::new(&fill_unit("self.v <= 1", ""), &[0]);
    let holds = repaired.deadlock("Fill");
    assert!(matches!(
        &holds,
        ModelCheckOutcome::Holds { stats, .. } if stats.product_states == 10
    ));
    assert_eq!(holds, repaired.deadlock("Fill"));
}

/// A memory model that implements `Sc`'s eight members on its own
/// (FR-215-AC-3).
#[derive(Clone, Copy, Debug)]
struct Interleaving;

/// Its component, which encodes as the empty `map`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Nothing;

impl Encode for Nothing {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        writer.begin_object()?;
        writer.name("entries")?;
        writer.begin_array()?;
        writer.end_array()?;
        writer.name("type")?;
        writer.string("map")?;
        writer.end_object()
    }
}

impl MemoryModel for Interleaving {
    type Component = Nothing;

    fn initial(&self) -> Nothing {
        Nothing
    }

    fn observe<'s>(
        &self,
        _: &Nothing,
        _: &ThreadPath,
        model: &'s ModelSimState,
    ) -> Cow<'s, ModelSimState> {
        Cow::Owned(model.clone())
    }

    fn write(
        &self,
        _: &Nothing,
        _: &ThreadPath,
        _: &ModelSimState,
        post: ModelSimState,
    ) -> (ModelSimState, Nothing) {
        (post, Nothing)
    }

    fn internal_steps(&self, _: &Nothing, _: &ModelSimState) -> Vec<MemoryStep<Nothing>> {
        Vec::new()
    }

    fn gate(&self, _: &Nothing, _: &ThreadPath, _: GatedStep) -> bool {
        true
    }

    fn split(&self, _: &Nothing, _: &ThreadPath, _: &[ThreadPath]) -> Nothing {
        Nothing
    }

    fn merge(&self, _: &Nothing, _: &ThreadPath, _: &[ThreadPath]) -> Nothing {
        Nothing
    }

    fn atom_values<'s>(&self, _: &Nothing, model: &'s ModelSimState) -> Cow<'s, ModelSimState> {
        Cow::Owned(model.clone())
    }

    fn fairness(&self) -> Vec<FairnessConstraint> {
        Vec::new()
    }
}

/// The state keys of every state reachable in `system`; no step is a
/// memory model's internal step.
fn keys<M: MemoryModel>(system: &ProtocolSystem<'_, M>) -> BTreeSet<Vec<u8>> {
    let mut seen = BTreeSet::new();
    let mut queue: Vec<ProtocolState<M>> = system.initial();
    while let Some(state) = queue.pop() {
        let (key, _) = system.state_key(&state).expect("a key");
        if !seen.insert(key) {
            continue;
        }
        for (transition, next) in system.successors(&state).expect("expands").successors {
            assert_ne!(transition.kind(), StepKind::Memory);
            queue.push(next);
        }
    }
    seen
}

/// FR-215-AC-3 (TC-660): `Sc`'s component is the empty member of every key,
/// it takes no internal step and opens every gate: the states of `Fill`
/// under `Sc` equal those under a model that implements `Sc`'s eight members
/// independently.
#[trace("TC-660", "FR-215-AC-3")]
#[test]
fn sc_is_the_identity_of_the_memory_seam() {
    let case = Case::new(&fill_unit("self.v = 0", ""), &[0]);
    let subject = case.subject("Fill");
    let under_sc = sc_system(&subject);
    let model = ModelSystem::new(subject.model.simulation.clone()).expect("admits");
    let independent =
        ProtocolSystem::new(model, &subject, Interleaving).expect("the system builds");
    let sc_keys = keys(&under_sc);
    assert_eq!(sc_keys.len(), 7);
    assert_eq!(sc_keys, keys(&independent));
    for key in &sc_keys {
        assert_eq!(
            json_of(key)["memory"],
            json!({"entries": [], "type": "map"})
        );
    }
    for thread in [root(), branch("left")] {
        for step in [
            GatedStep::Fork,
            GatedStep::Join,
            GatedStep::Send,
            GatedStep::Receive,
            GatedStep::Attempt,
        ] {
            assert!(Sc.gate(&ScComponent, &thread, step));
        }
    }
}

/// FR-217-AC-2 (TC-662): the deadlock counterexample reproduces; changing
/// its second step's post-state digest refuses `stale_dependency`/
/// `content-mismatch` naming the step, the recorded digest and the digest
/// the step reaches; replacing the second
/// step with `join(Both)` refuses `invalid_runtime_input`/`invalid-value`;
/// replaying the steps against the repaired `Fill` refuses on the package
/// identity.
#[trace("TC-662", "FR-217-AC-2")]
#[test]
fn a_protocol_replay_refuses_a_stale_or_disabled_step() {
    let case = Case::new(&fill_unit("self.v = 0", ""), &[0]);
    let counterexample = violated(&case.deadlock("Fill"));
    let replayed = case
        .replay("Fill", counterexample.clone())
        .expect("the counterexample replays");
    assert!(
        matches!(replayed.settlement, ModelTraceSettlement::Reproduced { .. }),
        "{replayed:?}"
    );
    let mut stale = counterexample.clone();
    let recorded = stale.prefix[0].post_state;
    stale.prefix[1].post_state = recorded;
    let Err(ReplayRefusal::ModelTrace(refusal)) = case.replay("Fill", stale) else {
        panic!("a stale digest refuses");
    };
    assert_eq!(
        *refusal,
        ModelTraceRefusal::PostState {
            step: 1,
            recorded,
            reached: vec![counterexample.prefix[1].post_state]
        }
    );
    assert_eq!(
        (refusal.code(), refusal.cause()),
        (Code::StaleDependency, "content-mismatch")
    );
    let mut disabled = counterexample.clone();
    disabled.prefix[1].transition = ProtocolTransition::join(root(), "Main/Both");
    let Err(ReplayRefusal::ModelTrace(refusal)) = case.replay("Fill", disabled) else {
        panic!("a disabled step refuses");
    };
    assert_eq!(
        (refusal.code(), refusal.cause()),
        (Code::InvalidRuntimeInput, "invalid-value")
    );
    let repaired = Case::new(&fill_unit("self.v <= 1", ""), &[0]);
    assert!(matches!(
        repaired.replay_against(&case, "Fill", counterexample),
        Err(ReplayRefusal::PackageIdMismatch { .. })
    ));
}

/// FR-217-AC-5 and FR-217-AC-6 (TC-662, ADR-027 PX-1, PX-2): a deadlock
/// counterexample whose prefix holds sends over FR-206-AC-7's `overflow loss
/// head` channel carries each send's `send_record` on that step alone, and
/// neither its transition identities nor its state keys hold one; it
/// replays. Changing a recorded `outcome` refuses `stale_dependency`/
/// `content-mismatch`, naming the step and both records. Removing a send's
/// record, or adding one to the attempt, refuses `invalid_runtime_input`/
/// `invalid-value` before any step is replayed: so it does when an earlier
/// step's digest is also stale, which replaying that step would refuse.
#[trace("TC-662", "FR-217-AC-5", "FR-217-AC-6")]
#[test]
fn a_protocol_replay_checks_each_send_record() {
    let members = "channel ch from w to w carries Boolean ordering fifo by (m: Boolean) { true } \
                   delivery exactly-once-premise capacity 2 overflow loss head;";
    let run = format!(
        "sequence Main {{ {} send P1 via ch as (p1: Boolean) {{ p1 }}; \
         send P2 via ch as (p2: Boolean) {{ not p2 }}; send N via ch as (n: Boolean) {{ n }}; \
         receive R1 via ch of P1 as (r1: Boolean) {{ true }}; }}",
        touch("T")
    );
    let case = Case::new(
        &(contracts("self.v = 0") + &protocol("P", members, &run, "d")),
        &[0],
    );
    let counterexample = violated(&case.deadlock("P"));
    let kinds: Vec<StepKind> = counterexample
        .prefix
        .iter()
        .map(|step| step.transition.kind())
        .collect();
    assert_eq!(
        kinds,
        [
            StepKind::Attempt,
            StepKind::Send,
            StepKind::Send,
            StepKind::Send
        ]
    );
    for step in &counterexample.prefix {
        assert_eq!(
            step.transition.send_record().is_some(),
            step.transition.kind() == StepKind::Send
        );
    }
    let displacing = counterexample.prefix[3]
        .transition
        .send_record()
        .expect("the third send's record")
        .clone();
    assert_eq!(
        (displacing.channel(), displacing.outcome()),
        ("P::ch", SendOutcome::Accepted)
    );
    let subject = case.subject("P");
    let system = sc_system(&subject);
    for state in reachable(&system) {
        let (key, _) = system.state_key(&state.state).expect("a key");
        assert!(!String::from_utf8_lossy(&key).contains("send-record"));
        for (transition, _) in &state.steps {
            let identity = system.transition_key(transition).expect("an identity");
            assert!(!String::from_utf8_lossy(&identity).contains("send-record"));
        }
    }
    let replayed = case
        .replay("P", counterexample.clone())
        .expect("the counterexample replays");
    assert!(
        matches!(replayed.settlement, ModelTraceSettlement::Reproduced { .. }),
        "{replayed:?}"
    );

    let mut changed = counterexample.clone();
    let altered = displacing.clone().with_outcome(SendOutcome::Rejected);
    changed.prefix[3].transition = changed.prefix[3]
        .transition
        .clone()
        .carrying(Some(altered.clone()));
    let Err(ReplayRefusal::ModelTrace(refusal)) = case.replay("P", changed) else {
        panic!("a changed send record refuses");
    };
    assert_eq!(
        *refusal,
        ModelTraceRefusal::SendRecord {
            step: 3,
            recorded: Box::new(altered),
            replayed: Box::new(displacing.clone()),
        }
    );
    assert_eq!(
        (refusal.code(), refusal.cause()),
        (Code::StaleDependency, "content-mismatch")
    );

    let mut missing = counterexample.clone();
    missing.prefix[1].transition = missing.prefix[1].transition.clone().carrying(None);
    // The shape check runs before any step: with step 0's post-state digest
    // also stale, a misplaced record on step 3 still refuses as a shape
    // defect, not as step 0's stale digest.
    let mut first = counterexample.clone();
    first.prefix[0].post_state = first.prefix[1].post_state;
    first.prefix[3].transition = first.prefix[3].transition.clone().carrying(None);
    let mut extra = counterexample;
    extra.prefix[0].transition = extra.prefix[0]
        .transition
        .clone()
        .carrying(Some(displacing.clone()));
    for (step, shaped) in [(1, missing), (3, first), (0, extra)] {
        let Err(ReplayRefusal::ModelTrace(refusal)) = case.replay("P", shaped) else {
            panic!("a misplaced send record refuses");
        };
        assert_eq!(*refusal, ModelTraceRefusal::SendRecordShape { step });
        assert_eq!(
            (refusal.code(), refusal.cause()),
            (Code::InvalidRuntimeInput, "invalid-value")
        );
    }
}

/// FR-217-AC-3 (TC-662): a deadlock envelope whose blocked list drops
/// `right` settles `inconclusive`, `ReplayParity`, as does one whose prefix
/// runs on to s6, which holds no live instance.
#[trace("TC-662", "FR-217-AC-3")]
#[test]
fn a_protocol_replay_recomputes_the_blocked_threads() {
    let case = Case::new(&fill_unit("self.v = 0", ""), &[0]);
    let outcome = case.deadlock("Fill");
    let ModelCheckOutcome::Violated {
        counterexample,
        stats,
    } = &outcome
    else {
        panic!("a violation");
    };
    let parity = |payload: TemporalCounterexample<ProtocolTransition>| {
        let record = case.settle(
            "Fill",
            &ModelCheckOutcome::Violated {
                counterexample: Box::new(payload),
                stats: *stats,
            },
        );
        assert!(
            matches!(
                record.value,
                TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(_))
            ),
            "{record:?}"
        );
    };
    let mut dropped = counterexample.as_ref().clone();
    dropped
        .blocked
        .retain(|thread| thread.path != branch("right"));
    parity(dropped);
    // s0 to s6 through `B` first, recorded with its digests.
    let subject = case.subject("Fill");
    let system = sc_system(&subject);
    let reached = reachable(&system);
    let mut prefix = Vec::new();
    let mut at = 0;
    for wanted in [1, 3, 4, 5, 6] {
        let (transition, _) = reached[at]
            .steps
            .iter()
            .find(|(_, next)| *next == wanted)
            .expect("an edge");
        let (_, digest) = system.state_key(&reached[wanted].state).expect("a key");
        prefix.push(ModelStep {
            transition: transition.clone(),
            post_state: digest,
        });
        at = wanted;
    }
    let mut finished = counterexample.as_ref().clone();
    finished.prefix = prefix;
    finished.blocked = Vec::new();
    parity(finished);
}

/// A protocol with a construct this system does not step yet is refused
/// with `unsupported_construct`/`not-yet-implemented` before any state is
/// explored.
#[test]
fn check_protocol_refuses_an_unstepped_construct() {
    let run = format!(
        "parallel Both memory tso {{ branch left {} branch right {} }} join all [left, right];",
        touch("T"),
        touch("U")
    );
    let case = p_case(&run, 0);
    let refusal = check_protocol(
        ProtocolCheckRequest {
            subject: case.subject("P"),
            item: ModelCheckItem::DeadlockFreedom,
            max_depth: None,
            limits: ModelCheckLimits::default(),
        },
        &Cancel::new(),
    )
    .expect_err("`memory tso` is not stepped yet");
    let ModelCheckRefusal::Protocol(refusal) = refusal else {
        panic!("{refusal:?}");
    };
    assert_eq!(
        (refusal.code(), refusal.cause()),
        (Code::UnsupportedConstruct, "not-yet-implemented")
    );
}

/// Part of FR-211-AC-1 (TC-656): `Fill` checks and compiles with no
/// `terminal` member, with `terminal when` a predicate over `k.v` and with
/// `terminal any`. A second `terminal` member refuses
/// `ambiguous_declaration`/`ambiguous-name`, and a `terminal when` predicate
/// that reads binder `a` refuses `missing_declaration`/`missing-name`, as
/// FR-104 refuses a non-state read in an invariant. Not covered: the three
/// packages' pairwise different identities. The compiled `package_id` does
/// not bind the `terminal` member until FR-383 emits it, so today the three
/// identities are equal.
#[test]
fn fill_checks_under_each_terminal_form() {
    for form in ["", "terminal when { k.v = 1 };", "terminal any;"] {
        let body = fill_unit("self.v = 0", form);
        package(&body);
        package_id(&body);
    }
    let refusal = |members: &str| {
        let refusal = compiled(&fill_unit("self.v = 0", members))
            .err()
            .unwrap_or_else(|| panic!("{members} refuses"));
        (refusal.code(), refusal.cause(), refusal.to_string())
    };
    let (code, cause, message) = refusal("terminal any; terminal when { k.v = 1 };");
    assert_eq!(
        (code, cause),
        (Code::AmbiguousDeclaration, Some("ambiguous-name")),
        "{message}"
    );
    let (code, cause, message) = refusal("terminal when { a = a };");
    assert_eq!(
        (code, cause),
        (Code::MissingDeclaration, Some("missing-name")),
        "{message}"
    );
}

/// The protocol `P` with `run` over `contracts` whose `BumpPost` is
/// `post_body`, from `c.v = value`.
fn bump_case(post_body: &str, run: &str, value: i64) -> Case {
    let contracts = contracts("self.v = 0").replace(
        "post BumpPost using v on M::Cell::bump { self.v - pre(self.v) >= 1 }",
        &format!("post BumpPost using v on M::Cell::bump {{ {post_body} }}"),
    );
    Case::new(&(contracts + &protocol("P", "", run, "d")), &[value])
}

/// Part of FR-215-AC-2 (TC-660): for the attempt `bump()` of FR-206-AC-2
/// the successor model states and FR-120 transition identities
/// `ProtocolSystem` gives equal those `ModelSystem` gives. A postcondition
/// the evaluator finds undefined records `ContractUndetermined` and the
/// model check ends UE-6's undefined-evaluation counterexample at the
/// contract; an undefined binder constraint records
/// `ProtocolExpressionUndetermined` and ends one at the attempt's node (the
/// protocol locus of ADR-027 TS-4). Not covered: a postcondition the
/// evaluator refuses, neither undefined nor incomplete, which settles V-6
/// `UndecidedSuccessor`; no fixture of this suite yields such an
/// evaluation.
#[test]
fn an_undefined_contract_or_constraint_ends_an_undefined_evaluation() {
    let run = |constraint: &str| {
        format!(
            "sequence Main {{ {} }}",
            attempt("X", "bump", "BumpPost", "x", constraint)
        )
    };
    let decided = bump_case("self.v - pre(self.v) >= 1", &run("true"), 0);
    let subject = decided.subject("P");
    let system = sc_system(&subject);
    let stepped: BTreeSet<(String, i64)> = system
        .successors(&system.initial()[0])
        .expect("expands")
        .successors
        .iter()
        .map(|(transition, state)| {
            let application = transition.application().expect("an application");
            (format!("{application:?}"), v(state.model(), "c"))
        })
        .collect();
    let model = ModelSystem::new(subject.model.simulation.clone()).expect("admits");
    let direct: BTreeSet<(String, i64)> = model
        .successors(&model.initial()[0])
        .expect("expands")
        .successors
        .iter()
        .filter(|(transition, _)| transition.operation().contains("Cell/bump"))
        .map(|(transition, state)| (format!("{transition:?}"), v(state, "c")))
        .collect();
    assert_eq!(stepped, direct);
    assert!(!stepped.is_empty());

    let undefined_at = |case: &Case| {
        let counterexample = violated(&case.deadlock("P"));
        let CounterexampleKind::UndefinedEvaluation(undefined) = counterexample.kind else {
            panic!("{counterexample:?}");
        };
        undefined.site.locus
    };
    // The postcondition divides by zero at the post-state `c.v = 2`.
    let post = bump_case("6 mod (2 - self.v) >= 0", &run("true"), 0);
    let subject = post.subject("P");
    let system = sc_system(&subject);
    let findings = system
        .successors(&system.initial()[0])
        .expect("expands")
        .findings;
    assert!(
        findings
            .iter()
            .any(|finding| matches!(finding, ModelFinding::ContractUndetermined { .. })),
        "{findings:?}"
    );
    assert_eq!(
        undefined_at(&post),
        UndefinedLocus::Contract("BumpPost".to_owned())
    );

    // The constraint divides by zero at the pre-state `c.v = 2`.
    let constraint = bump_case("self.v - pre(self.v) >= 1", &run("6 mod (2 - k.v) >= 0"), 2);
    let subject = constraint.subject("P");
    let system = sc_system(&subject);
    let findings = system
        .successors(&system.initial()[0])
        .expect("expands")
        .findings;
    assert!(
        findings.iter().any(|finding| matches!(
            finding,
            ModelFinding::ProtocolExpressionUndetermined { node, .. } if node == "Main/X"
        )),
        "{findings:?}"
    );
    assert_eq!(
        undefined_at(&constraint),
        UndefinedLocus::Protocol("Main/X".to_owned())
    );
}

/// Part of FR-206-AC-3 (TC-651): `outstanding cancel` cancels a running
/// branch together with every thread it forked. `b` forks the inner
/// `parallel Q`; once `a` completes, `join any [a, b]` cancels `b`, after
/// which neither of `Q`'s attempts is enabled and only the root is left.
#[test]
fn a_cancelled_branch_takes_the_threads_it_forked_with_it() {
    let inner = format!(
        "sequence Inner {{ parallel Q {{ branch c {} branch d {} }} join all [c, d]; }}",
        touch("TY"),
        touch("TZ")
    );
    let run = parallel(
        &[("a", touch("TX")), ("b", inner)],
        "join any [a, b] outstanding cancel",
    );
    let case = p_case(&run, 0);
    let subject = case.subject("P");
    let system = sc_system(&subject);
    let reached = reachable(&system);
    let mut joined = 0;
    for state in &reached {
        for (transition, next) in &state.steps {
            if transition.kind() != StepKind::Join || transition.node() != Some("Main/Both") {
                continue;
            }
            joined += 1;
            let after = &reached[*next];
            let threads = system.threads(&after.state).expect("threads");
            assert_eq!(
                threads
                    .iter()
                    .map(|thread| &thread.path)
                    .collect::<Vec<_>>(),
                vec![&root()],
                "{threads:?}"
            );
            assert!(after.steps.iter().all(|(step, _)| {
                step.kind() != StepKind::Attempt
                    || !step
                        .node()
                        .is_some_and(|node| node.ends_with("/TY") || node.ends_with("/TZ"))
            }));
        }
    }
    // `a` completes before, between or after any of `b`'s inner steps.
    assert!(joined > 1, "{joined}");
}

/// FR-215's requires-bound pre-check over a replayed protocol subject: a
/// `finish` binder of the unbounded type `Integer` refuses the check
/// `RequiresBound`, and the replay of a counterexample over the same subject
/// refuses `invalid_runtime_input`/`invalid-value` with the typed
/// `RequiresBound`, not an internal fault.
#[test]
fn a_replay_over_an_unbounded_protocol_subject_refuses_requires_bound() {
    let unbounded = Case::new(
        &(contracts("self.v = 0")
            + &protocol("Fill", "", &crate::fill::fill_run(), "d = 0")
                .replace("(d: Boolean)", "(d: Integer)")),
        &[0],
    );
    let refusal = check_protocol(
        ProtocolCheckRequest {
            subject: unbounded.subject("Fill"),
            item: ModelCheckItem::DeadlockFreedom,
            max_depth: None,
            limits: ModelCheckLimits::default(),
        },
        &Cancel::new(),
    )
    .expect_err("the finish binder is unbounded");
    assert!(
        matches!(refusal, ModelCheckRefusal::RequiresBound(_)),
        "{refusal:?}"
    );
    let bounded = Case::new(&fill_unit("self.v = 0", ""), &[0]);
    let payload = violated(&bounded.deadlock("Fill"));
    let Err(ReplayRefusal::ModelTrace(refusal)) = unbounded.replay("Fill", payload) else {
        panic!("the replay refuses");
    };
    assert!(
        matches!(&*refusal, ModelTraceRefusal::NotSimulated(cause)
            if matches!(**cause, NotSimulated::RequiresBound(_))),
        "{refusal:?}"
    );
    assert_eq!(
        (refusal.code(), refusal.cause()),
        (Code::InvalidRuntimeInput, "invalid-value")
    );
}
