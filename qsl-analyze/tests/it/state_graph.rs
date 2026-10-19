// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-166, FR-167 and FR-168 over ADR-022 §7's worked examples
//! (`crate::graph_world`): phase 0 walks, the one exploration and its open
//! nodes, and the decision of each state-graph item instance.
//!
//! Every expected state, path and count is computed here independently of
//! the engine: from a breadth-first enumeration of the subject's
//! `ModelSystem` written in this file, and from each world's arithmetic.
//! Each expected path is re-executed against `ModelSystem` step by step.
//!
//! Not covered here: the terminal record each outcome settles to (FR-169)
//! and the replay of its evidence (FR-170).

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use ix_trace_rs::trace;
use qsl_analyze::model_check::{
    check_state_graph, ModelCheckRefusal, NodeIndex, StateGraphRequest, StateGraphRun,
};
use qsl_eval::simulation::model::{ModelSimState, ModelSystem};
use qsl_eval::simulation::TransitionSystem;
use qsl_foundation::digest::DigestRecord;
use qsl_replay::{
    CounterexampleKind, GraphWitness, IncompleteCause, ModelCheckLimit, ModelCheckLimits,
    ModelCheckOutcome, ModelPath, ModelStep, OpenCause, ReachedLimit, RunEnd, StateGraphOutcome,
    WitnessSource, DEFAULT_MAX_WALK_STEPS, DEFAULT_WITNESS_SAMPLES, DEFAULT_WITNESS_SEED,
};
use qsl_semantics::check::PredicateRole;
use quire_exact::{Cancel, CancelCause};

use crate::counter::UNLIMITED;
use crate::graph_world::{
    config_version, config_world, config_world_over, fork_world, game_world, job_world, Field,
    Model, Operation, World, CAN_STILL_WIN, DIVIDES, REACHES_THREE, REACHES_TWO,
};

const AB: &[&str] = &["a", "b"];

fn run(request: StateGraphRequest<'_>) -> StateGraphRun {
    check_state_graph(request, &Cancel::new()).expect("the subject is checked")
}

fn system<'w>(world: &'w World, keys: &[&str]) -> ModelSystem<'w> {
    ModelSystem::new(world.subject(keys, UNLIMITED).simulation).expect("admits")
}

/// Every reachable state of `world` over universe `keys`, by breadth-first
/// enumeration of its `ModelSystem`, with its state-key digest, and the
/// number of edges.
fn reachable(world: &World, keys: &[&str]) -> (BTreeMap<DigestRecord, ModelSimState>, usize) {
    let system = system(world, keys);
    let mut seen = BTreeMap::new();
    let mut edges = 0;
    let mut queue: VecDeque<ModelSimState> = system.initial().into();
    while let Some(state) = queue.pop_front() {
        let (_, digest) = system.state_key(&state).expect("encodes");
        if seen.contains_key(&digest) {
            continue;
        }
        let expansion = system.successors(&state).expect("expands");
        edges += expansion.successors.len();
        queue.extend(expansion.successors.into_iter().map(|(_, next)| next));
        seen.insert(digest, state);
    }
    (seen, edges)
}

/// The state at each position of `path`, re-executed from its initial
/// state: each step's post-state is the successor of its transition whose
/// digest it records.
fn positions(world: &World, keys: &[&str], path: &ModelPath) -> Vec<ModelSimState> {
    let system = system(world, keys);
    let index = usize::try_from(path.initial).expect("fits");
    let mut current = system.initial()[index].clone();
    let mut states = vec![current.clone()];
    for step in &path.steps {
        current = system
            .successors(&current)
            .expect("expands")
            .successors
            .into_iter()
            .find(|(transition, state)| {
                *transition == step.transition
                    && system.state_key(state).expect("encodes").1 == step.post_state
            })
            .map(|(_, state)| state)
            .expect("the step is a successor");
        states.push(current.clone());
    }
    states
}

/// The operation name of a step's transition.
fn operation(step: &ModelStep) -> String {
    let text: serde_json::Value =
        serde_json::from_str(step.transition.operation()).expect("an identity document");
    let node = text["node"].as_str().expect("the operation's node");
    node.rsplit('/')
        .next()
        .expect("an identity path")
        .to_owned()
}

/// The receiver key a step's transition names: its reference's identity,
/// hex-encoded in the transition identity.
fn receiver(step: &ModelStep) -> String {
    fn identity(value: &serde_json::Value) -> Option<String> {
        match value {
            serde_json::Value::Object(members) => members
                .get("identity")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
                .or_else(|| members.values().find_map(identity)),
            serde_json::Value::Array(items) => items.iter().find_map(identity),
            _ => None,
        }
    }
    let text: serde_json::Value =
        serde_json::from_str(step.transition.receiver().expect("a receiver"))
            .expect("a reference document");
    let hex = identity(&text).expect("the reference's identity");
    let bytes: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&hex[at..at + 2], 16).expect("hex"))
        .collect();
    String::from_utf8(bytes).expect("a key")
}

fn names(steps: &[ModelStep]) -> Vec<String> {
    steps.iter().map(operation).collect()
}

fn witnesses(outcome: &StateGraphOutcome) -> &[GraphWitness] {
    match outcome {
        StateGraphOutcome::Witnessed { paths }
        | StateGraphOutcome::WitnessedUnchecked { paths, .. } => paths,
        other => panic!("expected witnesses, got {other:?}"),
    }
}

/// `(va, vb)` at each position of `path` over §7.1's world.
fn versions(world: &World, path: &ModelPath) -> Vec<(i64, i64)> {
    positions(world, AB, path)
        .iter()
        .map(|state| {
            (
                world.value(state, "a", "versionNumber"),
                world.value(state, "b", "versionNumber"),
            )
        })
        .collect()
}

/// The digest of the one reachable state where `select` holds.
fn digest_where(
    world: &World,
    keys: &[&str],
    select: impl Fn(&ModelSimState) -> bool,
) -> DigestRecord {
    let (states, _) = reachable(world, keys);
    let found: Vec<_> = states
        .iter()
        .filter(|(_, state)| select(state))
        .map(|(digest, _)| *digest)
        .collect();
    assert_eq!(found.len(), 1, "one state selected");
    found[0]
}

fn node_of(run: &StateGraphRun, digest: DigestRecord) -> NodeIndex {
    let index = run
        .graph
        .nodes
        .iter()
        .position(|node| node.key == digest)
        .expect("the state was explored");
    nodes(run)
        .into_iter()
        .find(|node| node.index() == index)
        .expect("an explored node has a handle")
}

/// Every node handle the graph names: its initial states, its edges'
/// endpoints and its open nodes.
fn nodes(run: &StateGraphRun) -> Vec<NodeIndex> {
    let mut all: BTreeSet<NodeIndex> = run.graph.initial().iter().copied().collect();
    for edge in &run.graph.edges {
        all.insert(edge.source);
        all.insert(edge.target);
    }
    all.extend(run.graph.open.keys().copied());
    all.into_iter().collect()
}

fn max_states(limits: ModelCheckLimits, value: usize) -> ModelCheckLimits {
    let mut limits = limits;
    limits.limits.max_states = value;
    limits
}

fn no_samples() -> ModelCheckLimits {
    ModelCheckLimits {
        witness_samples: 0,
        ..ModelCheckLimits::default()
    }
}

/// The walk budget of the tests whose walks never reach their target: an
/// unoptimized `ModelSystem` expansion of §7.1 takes milliseconds, so 64
/// walks of the published 4,096 steps per instance would take minutes. The
/// budget's reading and counting are the same at any value.
const SHORT_WALK: u64 = 32;

fn short_walks() -> ModelCheckLimits {
    ModelCheckLimits {
        max_walk_steps: SHORT_WALK,
        ..ModelCheckLimits::default()
    }
}

fn stopped_at(limit: ModelCheckLimit, value: u64) -> RunEnd {
    RunEnd::Stopped {
        cause: IncompleteCause::ResourceExhausted,
        limit: Some(ReachedLimit { limit, value }),
    }
}

/// The steps from `start` that apply `ops` in order, each `(operation,
/// receiver)`, found by expanding `ModelSystem` directly.
fn expected_steps(
    world: &World,
    keys: &[&str],
    start: usize,
    ops: &[(&str, &str)],
) -> Vec<ModelStep> {
    let system = system(world, keys);
    let mut current = system.initial()[start].clone();
    let mut steps = Vec::new();
    for (name, key) in ops {
        let (transition, next) = system
            .successors(&current)
            .expect("expands")
            .successors
            .into_iter()
            .find(|(transition, _)| {
                let step = ModelStep {
                    transition: transition.clone(),
                    post_state: system.state_key(&current).expect("encodes").1,
                };
                operation(&step) == *name && receiver(&step) == *key
            })
            .expect("the operation is enabled");
        steps.push(ModelStep {
            transition,
            post_state: system.state_key(&next).expect("encodes").1,
        });
        current = next;
    }
    steps
}

// ---------------------------------------------------------------- FR-167

/// FR-167-AC-1: §7.1 explores to its 9 states and 18 edges, every node
/// closed; §7.2's game to 4 nodes and 3 edges in discovery order `Start`,
/// `Mid`, `Lost`, `Won`; §7.3's job to 5 nodes and 5 edges. A request with
/// `ReachesTwo`, `ReachesThree` and the deadlock-freedom item over §7.1
/// explores once: its statistics count 9 nodes, not one exploration per item.
#[trace("TC-592", "FR-167-AC-1")]
#[test]
fn each_worked_example_explores_once_to_its_graph() {
    let world = config_world(&format!("{REACHES_TWO}{REACHES_THREE}"), &[(0, 0)]);
    let items = vec![world.item("ReachesTwo", AB), world.item("ReachesThree", AB)];
    let mut request = world.request(AB, items, no_samples());
    request.deadlock_freedom = true;
    let result = run(request);
    let (states, edges) = reachable(&world, AB);
    assert_eq!((states.len(), edges), (9, 18));
    assert_eq!(result.graph.nodes.len(), 9);
    assert_eq!(result.graph.edges.len(), 18);
    assert_eq!(result.graph.stats.nodes, 9);
    assert_eq!(result.graph.stats.edges, 18);
    assert!(result.graph.open.is_empty(), "{:?}", result.graph.open);
    assert_eq!(result.graph.end, RunEnd::Completed);
    let keys: BTreeSet<_> = result.graph.nodes.iter().map(|node| node.key).collect();
    assert_eq!(keys, states.keys().copied().collect());

    let game = game_world(false, false, CAN_STILL_WIN);
    let result = run(game.request(&["g"], vec![game.item("CanStillWin", &["g"])], no_samples()));
    assert_eq!((result.graph.nodes.len(), result.graph.edges.len()), (4, 3));
    let (states, _) = reachable(&game, &["g"]);
    let phases: Vec<i64> = result
        .graph
        .nodes
        .iter()
        .map(|node| game.value(&states[&node.key], "g", "phase"))
        .collect();
    // Start 0, Mid 1, Lost 2, Won 3.
    assert_eq!(phases, vec![0, 1, 2, 3]);
    assert!(result.graph.open.is_empty());

    let job = job_world(false, false);
    let result = run(job.request(&["j"], vec![job.item("InOneWay", &["j"])], no_samples()));
    assert_eq!((result.graph.nodes.len(), result.graph.edges.len()), (5, 5));
    assert_eq!(reachable(&job, &["j"]).1, 5);
}

/// FR-167-AC-2: §7.1 under horizon 1 leaves `(1, 0)` and `(0, 1)` open,
/// `MaxDepth`, and `(0, 0)` closed, with `end` `Completed`; `max_states` 3
/// stops the run at `max_states` with every discovered unexpanded node open,
/// `NotExpanded`.
#[trace("TC-592", "FR-167-AC-2")]
#[test]
fn a_horizon_and_a_state_limit_leave_their_nodes_open() {
    let world = config_world(REACHES_TWO, &[(0, 0)]);
    let at = |va, vb| {
        digest_where(&world, AB, |state| {
            world.value(state, "a", "versionNumber") == va
                && world.value(state, "b", "versionNumber") == vb
        })
    };
    let mut request = world.request(AB, vec![world.item("ReachesTwo", AB)], no_samples());
    request.max_depth = Some(1);
    let result = run(request);
    assert_eq!(result.graph.end, RunEnd::Completed);
    let open: BTreeMap<DigestRecord, OpenCause> = result
        .graph
        .open
        .iter()
        .map(|(node, cause)| (result.graph.nodes[node.index()].key, *cause))
        .collect();
    assert_eq!(
        open,
        BTreeMap::from([
            (at(1, 0), OpenCause::MaxDepth),
            (at(0, 1), OpenCause::MaxDepth)
        ])
    );
    assert_eq!(result.graph.nodes.len(), 3);

    let request = world.request(
        AB,
        vec![world.item("ReachesTwo", AB)],
        max_states(no_samples(), 3),
    );
    let result = run(request);
    assert_eq!(result.graph.end, stopped_at(ModelCheckLimit::MaxStates, 3));
    // (0, 0) expanded to (1, 0) and (0, 1); (1, 0)'s expansion found (2, 0)
    // with the state budget spent: it and (0, 1) were never fully expanded.
    assert_eq!(result.graph.nodes.len(), 3);
    let not_expanded: Vec<usize> = result
        .graph
        .open
        .iter()
        .filter(|(_, cause)| **cause == OpenCause::NotExpanded)
        .map(|(node, _)| node.index())
        .collect();
    assert_eq!(not_expanded, vec![1, 2]);
    assert_eq!(result.graph.open.len(), 2);
}

/// FR-167-AC-2: a guard with no value where `versionNumber` is 2 leaves that
/// node open, `UndecidedSuccessor`, and every other reachable node is still
/// expanded and closed.
#[trace("TC-592", "FR-167-AC-2")]
#[test]
fn an_undecided_contract_conjunction_leaves_only_its_node_open() {
    let guard = "pre Guard using v on Config::ConfigVersion::attemptUpdate { \
                 2 div (2 - self.versionNumber) >= 1 }\n";
    let world = config_world_over(guard, REACHES_TWO, &["a"], &[vec![0]]);
    let result = run(world.request(&["a"], vec![world.item("ReachesTwo", &["a"])], no_samples()));
    assert_eq!(result.graph.end, RunEnd::Completed);
    let (states, _) = reachable(&world, &["a"]);
    assert_eq!(result.graph.nodes.len(), states.len());
    let versions: Vec<i64> = result
        .graph
        .nodes
        .iter()
        .map(|node| world.value(&states[&node.key], "a", "versionNumber"))
        .collect();
    assert_eq!(versions, vec![0, 1, 2]);
    assert_eq!(
        result
            .graph
            .open
            .iter()
            .map(|(node, cause)| (node.index(), *cause))
            .collect::<Vec<_>>(),
        vec![(2, OpenCause::UndecidedSuccessor)]
    );
}

/// FR-167-AC-3: §7.2's deadlock-freedom item is violated at `Lost` with the
/// path `play, lose`; a subject with no initial state ends every item
/// `NoInitialState`; an unbounded root refuses `RequiresBound` before any
/// node is explored.
#[trace("TC-592", "FR-167-AC-3")]
#[test]
fn the_deadlock_item_and_the_subject_refusals() {
    let game = game_world(false, false, CAN_STILL_WIN);
    let mut request = game.request(&["g"], vec![game.item("CanStillWin", &["g"])], no_samples());
    request.deadlock_freedom = true;
    let result = run(request);
    let Some(ModelCheckOutcome::Violated { counterexample, .. }) = &result.deadlock else {
        panic!("a deadlock: {:?}", result.deadlock);
    };
    assert_eq!(counterexample.kind, CounterexampleKind::Deadlock);
    assert_eq!(names(&counterexample.prefix), ["play", "lose"]);
    let path = ModelPath {
        initial: 0,
        steps: counterexample.prefix.clone(),
    };
    let last = positions(&game, &["g"], &path).pop().expect("a state");
    assert_eq!(game.value(&last, "g", "phase"), 2);

    let empty = World::new(
        config_version(),
        &format!("{}{REACHES_TWO}", crate::graph_world::CONFIG_CONTRACT),
        &[],
    );
    let mut request = empty.request(AB, vec![empty.item("ReachesTwo", AB)], no_samples());
    request.deadlock_freedom = true;
    let result = run(request);
    for instance in &result.items[0] {
        assert_eq!(instance.outcome, StateGraphOutcome::NoInitialState);
    }
    assert!(matches!(
        result.deadlock,
        Some(ModelCheckOutcome::Undecided {
            cause: qsl_replay::UndecidedCause::NoInitialState,
            ..
        })
    ));

    let unbounded = Model {
        package: "test/open",
        alias: "O",
        object: "Counter",
        population: "counters",
        fields: vec![Field {
            name: "value",
            range: None,
        }],
        operations: vec![Operation {
            name: "set",
            modifies: &["value"],
        }],
    };
    let world = World::new(
        unbounded,
        "possible Big using v over (c: O::Counter) { c.value = 7 }\n",
        &[&[("c", &[("value", 0)])]],
    );
    let refusal = check_state_graph(
        world.request(&["c"], vec![world.item("Big", &["c"])], no_samples()),
        &Cancel::new(),
    )
    .expect_err("an unbounded root refuses");
    assert!(
        matches!(refusal, ModelCheckRefusal::RequiresBound(_)),
        "{refusal:?}"
    );
}

/// FR-167-AC-5: AC-1's §7.2 request, run twice, gives equal explored graphs,
/// labels and open sets.
#[trace("TC-592", "FR-167-AC-5")]
#[test]
fn the_explored_graph_is_a_function_of_the_request() {
    let game = game_world(false, false, CAN_STILL_WIN);
    let once = || run(game.request(&["g"], vec![game.item("CanStillWin", &["g"])], no_samples()));
    let (first, second) = (once(), once());
    assert_eq!(first.graph, second.graph);
    assert!(first.graph.nodes.iter().all(|node| !node.labels.is_empty()));
}

// ---------------------------------------------------------------- FR-166

/// FR-166-AC-1: with the default limits and no seed, each `ReachesTwo`
/// instance gets a sampled witness from `(0, 0)` whose last state is the first
/// visited state where its bound config's `versionNumber` is 2, with seed 0
/// and a trace index below 64; phase 1 adds no explored witness.
#[trace("TC-591", "FR-166-AC-1")]
#[test]
fn default_phase_zero_samples_a_witness_for_each_instance() {
    let world = config_world(REACHES_TWO, &[(0, 0)]);
    let result = run(world.request(
        AB,
        vec![world.item("ReachesTwo", AB)],
        ModelCheckLimits::default(),
    ));
    assert_eq!(result.seed, DEFAULT_WITNESS_SEED);
    for (instance, key) in result.items[0].iter().zip(["a", "b"]) {
        let [witness] = witnesses(&instance.outcome) else {
            panic!("one witness per initial state");
        };
        let WitnessSource::Sampled(provenance) = &witness.source else {
            panic!("sampled: {:?}", witness.source);
        };
        assert_eq!(provenance.seed, 0);
        assert!(provenance.trace < DEFAULT_WITNESS_SAMPLES);
        assert_eq!(instance.sampled, vec![Some(witness.clone())]);
        let bound: Vec<i64> = positions(&world, AB, &witness.path)
            .iter()
            .map(|state| world.value(state, key, "versionNumber"))
            .collect();
        assert_eq!(bound.last(), Some(&2), "{key}: {bound:?}");
        assert!(!bound[..bound.len() - 1].contains(&2), "{key}: {bound:?}");
    }
}

/// FR-166-AC-2: `witness_samples` 0 draws no walk, so every witness is
/// phase 1's; `ReachesThree` under the default `witness_samples` draws its 64
/// walks per instance, finds none, and phase 1 decides it. The walks take
/// [`SHORT_WALK`] steps, not the default 4,096 (see [`SHORT_WALK`]).
#[trace("TC-591", "FR-166-AC-2")]
#[test]
fn phase_zero_off_and_a_target_no_walk_reaches() {
    let world = config_world(&format!("{REACHES_TWO}{REACHES_THREE}"), &[(0, 0)]);
    let result = run(world.request(AB, vec![world.item("ReachesTwo", AB)], no_samples()));
    for instance in &result.items[0] {
        assert!(instance.sampled.is_empty());
        assert_eq!(instance.walks_stopped, 0);
        for witness in witnesses(&instance.outcome) {
            assert_eq!(witness.source, WitnessSource::Explored);
        }
    }
    let result = run(world.request(AB, vec![world.item("ReachesThree", AB)], short_walks()));
    for instance in &result.items[0] {
        assert_eq!(instance.sampled, vec![None]);
        assert_eq!(instance.walks_stopped, DEFAULT_WITNESS_SAMPLES);
        assert!(matches!(
            instance.outcome,
            StateGraphOutcome::Trapped { .. }
        ));
    }
}

/// FR-166-AC-3: over two snapshots `(0, 0)` and `(1, 1)` the witness for
/// initial state 1 has a trace index in `64..128`; seed 7 is recorded; two
/// requests with no seed give equal witnesses and record the default seed.
#[trace("TC-591", "FR-166-AC-3")]
#[test]
fn trace_indices_and_seeds_name_each_walk() {
    let world = config_world(REACHES_TWO, &[(0, 0), (1, 1)]);
    let request = |seed| {
        let mut request = world.request(
            AB,
            vec![world.item("ReachesTwo", AB)],
            ModelCheckLimits::default(),
        );
        request.seed = seed;
        run(request)
    };
    let unseeded = request(None);
    assert_eq!(unseeded.seed, DEFAULT_WITNESS_SEED);
    for instance in &unseeded.items[0] {
        let paths = witnesses(&instance.outcome);
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[1].path.initial, 1);
        let WitnessSource::Sampled(provenance) = &paths[1].source else {
            panic!("sampled: {:?}", paths[1].source);
        };
        assert!(
            (64..128).contains(&provenance.trace),
            "{}",
            provenance.trace
        );
    }
    assert_eq!(unseeded.items, request(None).items);
    let seeded = request(Some(7));
    assert_eq!(seeded.seed, 7);
    for instance in &seeded.items[0] {
        for witness in witnesses(&instance.outcome) {
            let WitnessSource::Sampled(provenance) = &witness.source else {
                panic!("sampled: {:?}", witness.source);
            };
            assert_eq!(provenance.seed, 7);
        }
    }
}

/// FR-166-AC-4: `witness_samples` `u64::MAX` over two snapshots overflows
/// the trace index and stops the run before any walk, naming the limit and
/// its value; a cancelled handle stops it before the first walk.
#[trace("TC-591", "FR-166-AC-4")]
#[test]
fn an_overflowing_trace_index_and_a_cancellation_stop_before_any_walk() {
    let world = config_world(REACHES_TWO, &[(0, 0), (1, 1)]);
    let limits = ModelCheckLimits {
        witness_samples: u64::MAX,
        ..ModelCheckLimits::default()
    };
    let result = run(world.request(AB, vec![world.item("ReachesTwo", AB)], limits));
    let end = stopped_at(ModelCheckLimit::WitnessSamples, u64::MAX);
    assert_eq!(result.graph.end, end);
    assert!(result.graph.nodes.is_empty());
    assert_eq!(ModelCheckLimit::WitnessSamples.setting(), "witness_samples");
    for instance in &result.items[0] {
        assert_eq!(
            instance.outcome,
            StateGraphOutcome::NoDecision {
                end: end.clone(),
                open: BTreeSet::new()
            }
        );
    }

    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    let result = check_state_graph(
        world.request(
            AB,
            vec![world.item("ReachesTwo", AB)],
            ModelCheckLimits::default(),
        ),
        &cancel,
    )
    .expect("checked");
    let cancelled = RunEnd::Stopped {
        cause: IncompleteCause::Cancelled {
            source: CancelCause::Requested,
        },
        limit: None,
    };
    assert_eq!(result.graph.end, cancelled);
    assert!(result.graph.nodes.is_empty());
    for instance in &result.items[0] {
        assert!(instance.sampled.is_empty());
    }
}

/// TC-613 (FR-166-AC-5's engine half; its settlement is FR-169's): the
/// `Divides` target holds at `(1, 0)` and has no value at `(2, 0)`. Phase 0
/// finds a witness ending at `va = 1`, and exploration still runs to
/// completion and returns `Undefined` at `(2, 0)`; `ReachesTwo` under the
/// same limits is `Witnessed` only after exploration completes with no open
/// node.
#[trace("TC-613", "FR-168-AC-7")]
#[test]
fn a_sampled_witness_never_hides_an_undefined_target() {
    let world = config_world(&format!("{REACHES_TWO}{DIVIDES}"), &[(0, 0)]);
    let result = run(world.request(
        AB,
        vec![world.item("Divides", &["a"])],
        ModelCheckLimits::default(),
    ));
    let instance = &result.items[0][0];
    let [Some(sampled)] = instance.sampled.as_slice() else {
        panic!("a sampled witness: {:?}", instance.sampled);
    };
    assert!(matches!(sampled.source, WitnessSource::Sampled(_)));
    assert_eq!(versions(&world, &sampled.path).last().map(|v| v.0), Some(1));
    assert_eq!(result.graph.end, RunEnd::Completed);
    assert_eq!(result.graph.nodes.len(), 9);
    let StateGraphOutcome::Undefined { stem, undefined } = &instance.outcome else {
        panic!("undefined: {:?}", instance.outcome);
    };
    assert_eq!(versions(&world, stem), vec![(0, 0), (1, 0), (2, 0)]);
    assert_eq!(undefined.predicate, PredicateRole::Target);
    assert_eq!(
        undefined.cause,
        qsl_eval::temporal::UndefinedCause::Kernel(quire_exact::Undefined::DivisionByZero)
    );

    let result = run(world.request(
        AB,
        vec![world.item("ReachesTwo", AB)],
        ModelCheckLimits::default(),
    ));
    assert!(result.graph.open.is_empty());
    assert_eq!(result.graph.end, RunEnd::Completed);
    for instance in &result.items[0] {
        let StateGraphOutcome::Witnessed { paths } = &instance.outcome else {
            panic!("witnessed: {:?}", instance.outcome);
        };
        assert!(matches!(paths[0].source, WitnessSource::Sampled(_)));
    }
}

/// TC-614 (FR-166-AC-6's engine half; its settlement is FR-169's):
/// `ReachesTwo` with the default `witness_samples` and `max_states` 2 gets a
/// sampled witness per instance, the exploration stops at `max_states`, and
/// the outcome is `WitnessedUnchecked` with `Sampled` sources; with
/// `witness_samples` 0 the same run decides nothing.
#[trace("TC-614", "FR-168-AC-8")]
#[test]
fn a_stopped_exploration_leaves_sampled_witnesses_unchecked() {
    let world = config_world(REACHES_TWO, &[(0, 0)]);
    let end = stopped_at(ModelCheckLimit::MaxStates, 2);
    let result = run(world.request(
        AB,
        vec![world.item("ReachesTwo", AB)],
        max_states(ModelCheckLimits::default(), 2),
    ));
    for instance in &result.items[0] {
        let StateGraphOutcome::WitnessedUnchecked {
            paths,
            end: ended,
            open,
        } = &instance.outcome
        else {
            panic!("unchecked: {:?}", instance.outcome);
        };
        assert_eq!(*ended, end);
        assert_eq!(*open, BTreeSet::from([OpenCause::NotExpanded]));
        assert!(matches!(paths[0].source, WitnessSource::Sampled(_)));
    }
    let result = run(world.request(
        AB,
        vec![world.item("ReachesTwo", AB)],
        max_states(no_samples(), 2),
    ));
    for instance in &result.items[0] {
        assert_eq!(
            instance.outcome,
            StateGraphOutcome::NoDecision {
                end: end.clone(),
                open: BTreeSet::from([OpenCause::NotExpanded])
            }
        );
    }
}

/// TC-618 (FR-166-AC-7's engine half; the terminal record is FR-169's):
/// `ReachesThree` under the default `witness_samples` draws 64 walks per
/// instance, each stopped by `max_walk_steps` ([`SHORT_WALK`] steps here, the
/// published default being 4,096); with `max_walk_steps` 1
/// `ReachesTwo` gets no sampled witness, its target two steps away; with
/// horizon 1 no walk is counted as stopped by the budget.
#[trace("TC-618")]
#[test]
fn the_walk_budget_bounds_and_counts_each_walk() {
    let world = config_world(&format!("{REACHES_TWO}{REACHES_THREE}"), &[(0, 0)]);
    let defaults = ModelCheckLimits::default();
    assert_eq!(defaults.max_walk_steps, DEFAULT_MAX_WALK_STEPS);
    assert_eq!(DEFAULT_MAX_WALK_STEPS, 4_096);
    assert_eq!(ModelCheckLimit::MaxWalkSteps.setting(), "max_walk_steps");
    let result = run(world.request(AB, vec![world.item("ReachesThree", AB)], short_walks()));
    for instance in &result.items[0] {
        assert_eq!(instance.walks_stopped, 64);
    }

    let short = ModelCheckLimits {
        max_walk_steps: 1,
        ..defaults
    };
    let result = run(world.request(AB, vec![world.item("ReachesTwo", AB)], short));
    for instance in &result.items[0] {
        assert_eq!(instance.sampled, vec![None]);
        assert_eq!(instance.walks_stopped, 64);
        for witness in witnesses(&instance.outcome) {
            assert_eq!(witness.source, WitnessSource::Explored);
        }
    }

    let mut request = world.request(AB, vec![world.item("ReachesThree", AB)], defaults);
    request.max_depth = Some(1);
    let result = run(request);
    for instance in &result.items[0] {
        assert_eq!(instance.walks_stopped, 0);
    }
}

// ---------------------------------------------------------------- FR-168

/// FR-168-AC-1: with phase 0 off, `ReachesTwo` for `c = a` returns
/// `Witnessed` with `(0, 0) -upd(a)-> (1, 0) -upd(a)-> (2, 0)` and for
/// `c = b` the same with `upd(b)`; `ReachesThree` returns `Trapped` at
/// `(0, 0)` with an empty stem for each instance.
#[trace("TC-593", "FR-168-AC-1")]
#[test]
fn explored_witnesses_and_an_initial_trap() {
    let world = config_world(&format!("{REACHES_TWO}{REACHES_THREE}"), &[(0, 0)]);
    let result = run(world.request(
        AB,
        vec![world.item("ReachesTwo", AB), world.item("ReachesThree", AB)],
        no_samples(),
    ));
    for (instance, key) in result.items[0].iter().zip(["a", "b"]) {
        let StateGraphOutcome::Witnessed { paths } = &instance.outcome else {
            panic!("witnessed: {:?}", instance.outcome);
        };
        let expected = expected_steps(
            &world,
            AB,
            0,
            &[("attemptUpdate", key), ("attemptUpdate", key)],
        );
        assert_eq!(
            paths,
            &[GraphWitness {
                path: ModelPath {
                    initial: 0,
                    steps: expected
                },
                source: WitnessSource::Explored,
            }]
        );
    }
    assert_eq!(
        versions(&world, &witnesses(&result.items[0][0].outcome)[0].path),
        vec![(0, 0), (1, 0), (2, 0)]
    );
    for instance in &result.items[1] {
        assert_eq!(
            instance.outcome,
            StateGraphOutcome::Trapped {
                stem: ModelPath {
                    initial: 0,
                    steps: Vec::new()
                }
            }
        );
    }
}

/// FR-168-AC-2: §7.2's `CanStillWin` is `Trapped` at `Lost` with stem
/// `play, lose`; with `from (x.phase != Lost)` it `Holds`; with `restart`
/// and no `from` it `Holds`. AC-6: each, run twice, gives equal outcomes.
#[trace("TC-593", "FR-168-AC-2", "FR-168-AC-6")]
#[test]
fn always_possible_traps_and_holds() {
    let decide = |restart: bool, body: &str, name: &str| {
        let world = game_world(false, restart, body);
        let once = || run(world.request(&["g"], vec![world.item(name, &["g"])], no_samples()));
        let (first, second) = (once(), once());
        assert_eq!(first.items, second.items);
        (world, first.items[0][0].outcome.clone())
    };
    let (world, outcome) = decide(false, CAN_STILL_WIN, "CanStillWin");
    let StateGraphOutcome::Trapped { stem } = &outcome else {
        panic!("trapped: {outcome:?}");
    };
    assert_eq!(names(&stem.steps), ["play", "lose"]);
    let last = positions(&world, &["g"], stem).pop().expect("a state");
    assert_eq!(world.value(&last, "g", "phase"), 2);

    let guarded = "always possible Guarded using v over (x: G::Game) from (x.phase != 2) \
                   { x.phase = 3 }\n";
    assert_eq!(
        decide(false, guarded, "Guarded").1,
        StateGraphOutcome::Holds
    );
    assert_eq!(
        decide(true, CAN_STILL_WIN, "CanStillWin").1,
        StateGraphOutcome::Holds
    );
}

/// FR-168-AC-3: §7.3's `InOneWay` returns `PathPair` with an empty stem,
/// first `stepA, stepB, finish` and second `stepB, stepA, finish`; the
/// sequenced job `Holds`; the sequenced job with `reset` returns `PathPair`
/// with second `stepA, reset, stepA, stepB, finish`.
#[trace("TC-593", "FR-168-AC-3")]
#[test]
fn unique_path_counts_paths() {
    let decide = |sequenced, reset| {
        let world = job_world(sequenced, reset);
        let result = run(world.request(&["j"], vec![world.item("InOneWay", &["j"])], no_samples()));
        result.items[0][0].outcome.clone()
    };
    let StateGraphOutcome::PathPair {
        stem,
        first,
        second,
    } = decide(false, false)
    else {
        panic!("a path pair");
    };
    assert!(stem.steps.is_empty());
    assert_eq!(names(&first), ["stepA", "stepB", "finish"]);
    assert_eq!(names(&second), ["stepB", "stepA", "finish"]);
    assert_eq!(decide(true, false), StateGraphOutcome::Holds);
    let StateGraphOutcome::PathPair {
        stem,
        first,
        second,
    } = decide(true, true)
    else {
        panic!("a path pair");
    };
    assert!(stem.steps.is_empty());
    assert_eq!(names(&first), ["stepA", "stepB", "finish"]);
    assert_eq!(
        names(&second),
        ["stepA", "reset", "stepA", "stepB", "finish"]
    );
}

/// FR-168-AC-4: the game with `n` and `celebrate` under horizon 3: the trap
/// at `Lost` counts although the node `Won` with `n = 1` is open, and
/// `possible x.phase = Won and x.n = 50` decides nothing: `end` `Completed`,
/// open cause `MaxDepth`.
#[trace("TC-593", "FR-168-AC-4")]
#[test]
fn a_trap_counts_past_a_horizon_and_an_unreached_target_does_not() {
    let body = format!(
        "{CAN_STILL_WIN}possible Fifty using v over (x: G::Game) {{ x.phase = 3 and x.n = 50 }}\n"
    );
    let world = game_world(true, false, &body);
    let mut request = world.request(
        &["g"],
        vec![
            world.item("CanStillWin", &["g"]),
            world.item("Fifty", &["g"]),
        ],
        no_samples(),
    );
    request.max_depth = Some(3);
    let result = run(request);
    let won_one = digest_where(&world, &["g"], |state| {
        world.value(state, "g", "phase") == 3 && world.value(state, "g", "n") == 1
    });
    let open = node_of(&result, won_one);
    assert_eq!(result.graph.open.get(&open), Some(&OpenCause::MaxDepth));
    let StateGraphOutcome::Trapped { stem } = &result.items[0][0].outcome else {
        panic!("trapped: {:?}", result.items[0][0].outcome);
    };
    assert_eq!(names(&stem.steps), ["play", "lose"]);
    assert_eq!(
        result.items[1][0].outcome,
        StateGraphOutcome::NoDecision {
            end: RunEnd::Completed,
            open: BTreeSet::from([OpenCause::MaxDepth])
        }
    );
}

/// FR-168-AC-5: `Divides` for `c = a` returns `Undefined` with the canonical
/// path `(0, 0) -upd(a)-> (1, 0) -upd(a)-> (2, 0)`, `where` `(2, 0)` and
/// cause division by zero, although `(1, 0)` satisfies the target.
#[trace("TC-612", "FR-168-AC-5")]
#[test]
fn an_undefined_target_refutes_in_place_of_a_witness() {
    let world = config_world(DIVIDES, &[(0, 0)]);
    let result = run(world.request(AB, vec![world.item("Divides", &["a"])], no_samples()));
    let StateGraphOutcome::Undefined { stem, undefined } = &result.items[0][0].outcome else {
        panic!("undefined: {:?}", result.items[0][0].outcome);
    };
    assert_eq!(
        stem.steps,
        expected_steps(
            &world,
            AB,
            0,
            &[("attemptUpdate", "a"), ("attemptUpdate", "a")]
        )
    );
    assert_eq!(versions(&world, stem), vec![(0, 0), (1, 0), (2, 0)]);
    assert_eq!(undefined.state_key, stem.steps[1].post_state);
    assert_eq!(undefined.predicate, PredicateRole::Target);
    assert!(undefined.locus.is_some());
    assert_eq!(
        undefined.cause,
        qsl_eval::temporal::UndefinedCause::Kernel(quire_exact::Undefined::DivisionByZero)
    );
}

/// FR-168-AC-6: `ReachesTwo` with `witness_samples` 64 and with 0 both
/// return `Witnessed`, with `Sampled` and `Explored` sources.
#[trace("TC-593", "FR-168-AC-6")]
#[test]
fn phase_zero_changes_the_source_never_the_outcome_kind() {
    let world = config_world(REACHES_TWO, &[(0, 0)]);
    for (limits, sampled) in [(ModelCheckLimits::default(), true), (no_samples(), false)] {
        let result = run(world.request(AB, vec![world.item("ReachesTwo", AB)], limits));
        for instance in &result.items[0] {
            let StateGraphOutcome::Witnessed { paths } = &instance.outcome else {
                panic!("witnessed: {:?}", instance.outcome);
            };
            assert_eq!(
                matches!(paths[0].source, WitnessSource::Sampled(_)),
                sampled
            );
        }
    }
}

/// FR-168-AC-9: `Fork`'s `possible x.pos = 4` with phase 0 off under horizon
/// 2: `pos = 3` is open, and the explored witness descends the distance to
/// the target, `toB, fromB`, not to the nearest open node through `toA`; the
/// outcome is `WitnessedUnchecked`.
#[trace("TC-619", "FR-168-AC-9")]
#[test]
fn an_explored_witness_descends_toward_the_target_not_an_open_node() {
    let world = fork_world("possible Four using v over (x: F::Fork) { x.pos = 4 }\n");
    let mut request = world.request(&["f"], vec![world.item("Four", &["f"])], no_samples());
    request.max_depth = Some(2);
    let result = run(request);
    let three = digest_where(&world, &["f"], |state| world.value(state, "f", "pos") == 3);
    assert_eq!(
        result.graph.open.get(&node_of(&result, three)),
        Some(&OpenCause::MaxDepth)
    );
    let StateGraphOutcome::WitnessedUnchecked { paths, end, open } = &result.items[0][0].outcome
    else {
        panic!("unchecked: {:?}", result.items[0][0].outcome);
    };
    assert_eq!(*end, RunEnd::Completed);
    assert_eq!(*open, BTreeSet::from([OpenCause::MaxDepth]));
    assert_eq!(names(&paths[0].path.steps), ["toB", "fromB"]);
    let last = positions(&world, &["f"], &paths[0].path)
        .pop()
        .expect("a state");
    assert_eq!(world.value(&last, "f", "pos"), 4);
}
