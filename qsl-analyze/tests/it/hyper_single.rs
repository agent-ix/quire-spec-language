// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-181 (TC-606, TC-615, TC-616): EN-1 checks a single-existential
//! hyperproperty (HP-5) through the possible family, on ADR-023 section 8's
//! vault (`crate::hyper_world`).
//!
//! Every expected outcome below is derived by hand from the vault's
//! transition relation (secure `l = i`, leaky `l = i xor h`; `h` never
//! changes) and from FR-181's rules. Each witness lasso is also read back by
//! an oracle written here that re-executes it through the vault's
//! `ModelSystem`, checks that its loop closes, that its loop is fair and that
//! the clause evaluator reads the body true on it; a brute-force oracle
//! enumerates every lasso of at most three steps and decides the claim without
//! the product.
//!
//! The replay of a witness lasso and of a trap, the product-closure
//! certificate and the settlement of an HP-5 outcome (the `proved` and
//! `refuted` records of FR-181-AC-1 to AC-6) are not built, so no test below
//! claims them. Remaining work: QSL-964.

use ix_trace_rs::trace;
use qsl_analyze::model_check::{HyperRefusal, ModelCheckRefusal};
use qsl_eval::hyper::HyperLetters;
use qsl_eval::simulation::model::{ModelSimState, ModelSystem};
use qsl_eval::simulation::{StopReason, TransitionSystem};
use qsl_eval::temporal::{
    evaluate_tuple, AtomReading, ComponentPosition, ComponentTrace, LetterStop, TemporalOutcome,
    TemporalPosition, TupleLetters, UndefinedCause,
};
use qsl_foundation::digest::DigestRecord;
use qsl_replay::{
    ExistentialInstanceOutcome, ExistentialOutcome, IncompleteCause, LassoWitness, ModelCheckLimit,
    ModelCheckLimits, ModelLasso, ModelPath, OpenCause, ReachedLimit, RunEnd, WitnessSource,
};
use qsl_semantics::check::{FormulaNodeId, HyperForm};
use quire_exact::{Cancel, CancelCause, Meter, Undefined};
use quire_semantic_value::catalog::InternalFault;
use std::cell::RefCell;
use std::collections::HashMap;

use crate::counter::UNLIMITED;
use crate::hyper_world::{high, input_of, instance, low, non_interference, Vault, World, STARTS};

/// A start whose first vault has output 1 already.
const MIXED: [(i64, i64); 2] = [(0, 1), (1, 0)];

/// The clause of one `exists` with the fairness text `fair` over `body`.
fn clause(fair: &str, body: &str) -> String {
    format!(
        "hyper Possible using inf over behaviours (v: V::Vault) \
         exists trace b of V {fair} {{ {body} }}"
    )
}

const SETTLES: &str = "eventually always holds(v.l @ b = 1)";

fn weak() -> &'static str {
    "fair weak V::Vault::reset;"
}

/// The limits with phase 0 off.
fn unsampled() -> ModelCheckLimits {
    ModelCheckLimits {
        witness_samples: 0,
        ..ModelCheckLimits::default()
    }
}

/// The one instance of the run of `world` under `limits`.
fn single(world: &World, limits: ModelCheckLimits) -> ExistentialInstanceOutcome {
    let mut run = world
        .run_single(limits, None, &Cancel::new())
        .expect("the run is not refused");
    assert_eq!(run.instances.len(), 1, "one vault, one instance");
    run.instances.remove(0)
}

fn witnessed(outcome: &ExistentialOutcome) -> &[LassoWitness] {
    match outcome {
        ExistentialOutcome::Witnessed { lassos } => lassos,
        other => panic!("expected Witnessed, got {other:?}"),
    }
}

/// The body's atoms on the states of a lasso, through the world's seam.
struct Letters<'w> {
    world: &'w World,
    states: Vec<ModelSimState>,
}

impl TupleLetters for Letters<'_> {
    fn atom(
        &mut self,
        atom: FormulaNodeId,
        _joint: TemporalPosition,
        components: &[ComponentPosition],
        meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        let index = usize::try_from(components[0].position.index()).expect("a small position");
        let state = self
            .states
            .get(index)
            .ok_or(LetterStop::Fault(InternalFault::new("oracle", "position")))?;
        self.world
            .seams()
            .atom(atom, &instance(), &[state], meter)
            .map_err(LetterStop::Incomplete)
    }
}

/// How the oracle reads the vault's fairness set on the one operation
/// `reset`.
#[derive(Clone, Copy)]
enum Fairness {
    None,
    Weak,
    Strong,
}

impl Fairness {
    fn text(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Weak => "fair weak V::Vault::reset;",
            Self::Strong => "fair strong V::Vault::reset;",
        }
    }
}

fn is_reset(operation: &str) -> bool {
    operation.contains("/Vault/reset\"")
}

/// A state with its state-key digest, which is slow to compute.
struct Node {
    state: ModelSimState,
    key: DigestRecord,
}

/// The vault's own system, which the oracle re-executes lassos through, and
/// the clause evaluator it reads them with.
struct Oracle<'w> {
    world: &'w World,
    system: ModelSystem<'w>,
    /// Whether `reset` is enabled at the state with the digest.
    enables_reset: RefCell<HashMap<DigestRecord, bool>>,
}

impl<'w> Oracle<'w> {
    fn new(world: &'w World) -> Self {
        Self {
            world,
            system: ModelSystem::new(world.subject().simulation).expect("the subject admits"),
            enables_reset: RefCell::new(HashMap::new()),
        }
    }

    fn node(&self, state: ModelSimState) -> Node {
        let key = self.system.state_key(&state).expect("a key").1;
        Node { state, key }
    }

    /// The states of `lasso` by re-executing its steps: one for the initial
    /// state and one for each step.
    fn replayed(&self, lasso: &ModelLasso) -> Vec<Node> {
        let mut nodes = vec![self.node(self.system.initial()[lasso.path.initial as usize].clone())];
        for step in &lasso.path.steps {
            let last = &nodes.last().expect("a state").state;
            let next = self
                .system
                .successors(last)
                .expect("the vault expands")
                .successors
                .into_iter()
                .map(|(transition, state)| (transition, self.node(state)))
                .find(|(transition, next)| {
                    *transition == step.transition && next.key == step.post_state
                })
                .unwrap_or_else(|| panic!("{step:?} is not a step of the vault"))
                .1;
            nodes.push(next);
        }
        nodes
    }

    fn enables_reset(&self, node: &Node) -> bool {
        *self
            .enables_reset
            .borrow_mut()
            .entry(node.key)
            .or_insert_with(|| {
                self.system
                    .successors(&node.state)
                    .expect("the vault expands")
                    .successors
                    .iter()
                    .any(|(transition, _)| is_reset(transition.operation()))
            })
    }

    /// Whether `lasso` is a lasso of the vault whose loop closes, is fair under
    /// `fairness` and satisfies the body: the oracle's reading of a witness.
    fn is_witness(&self, lasso: &ModelLasso, fairness: Fairness) -> bool {
        let nodes = self.replayed(lasso);
        let steps = lasso.path.steps.len();
        let entry = lasso.loop_entry as usize;
        let terminal = entry == steps;
        if terminal {
            let last = nodes.last().expect("a state");
            assert!(
                self.system
                    .successors(&last.state)
                    .expect("expands")
                    .successors
                    .is_empty(),
                "a stutter loop is at a terminal state"
            );
        } else {
            assert!(entry < steps, "a loop has a step");
            assert_eq!(
                nodes[steps].key, nodes[entry].key,
                "the loop closes at its entry"
            );
        }
        self.satisfies(&nodes, &lasso.path.steps, entry, terminal, fairness)
    }

    /// Whether the lasso through `nodes` (one more than `steps`) that
    /// re-enters at `entry` is fair under `fairness` and satisfies the body.
    fn satisfies(
        &self,
        nodes: &[Node],
        steps: &[qsl_replay::ModelStep],
        entry: usize,
        terminal: bool,
        fairness: Fairness,
    ) -> bool {
        let (component, positions, loop_nodes, loop_steps) = if terminal {
            (
                ComponentTrace::Terminal {
                    len: (steps.len() + 1) as u64,
                },
                nodes,
                &nodes[nodes.len() - 1..],
                &steps[steps.len()..],
            )
        } else {
            (
                ComponentTrace::Lasso {
                    prefix: entry as u64,
                    loop_len: (steps.len() - entry) as u64,
                },
                &nodes[..steps.len()],
                &nodes[entry..steps.len()],
                &steps[entry..],
            )
        };
        let taken = loop_steps
            .iter()
            .any(|step| is_reset(step.transition.operation()));
        let enabled = |node: &Node| self.enables_reset(node);
        let fair = match fairness {
            Fairness::None => true,
            Fairness::Weak => taken || !loop_nodes.iter().all(enabled),
            Fairness::Strong => taken || !loop_nodes.iter().any(enabled),
        };
        if !fair {
            return false;
        }
        let mut letters = Letters {
            world: self.world,
            states: positions.iter().map(|node| node.state.clone()).collect(),
        };
        let outcome = evaluate_tuple(
            &self.world.clause,
            &[component],
            &mut letters,
            &mut Meter::new(UNLIMITED),
            None,
        )
        .expect("the evaluator reads the lasso");
        outcome == TemporalOutcome::Holds
    }

    /// Whether some lasso of at most `depth` steps from initial state
    /// `initial` is a witness: the brute-force search.
    fn exists(&self, initial: usize, fairness: Fairness, depth: usize) -> bool {
        let start = self.node(self.system.initial()[initial].clone());
        // Each prefix: the steps taken and the nodes reached.
        let mut stack = vec![(Vec::new(), vec![start])];
        while let Some((steps, nodes)) = stack.pop() {
            let last = nodes.last().expect("a state");
            let successors = self
                .system
                .successors(&last.state)
                .expect("expands")
                .successors;
            let mut lassos = Vec::new();
            if successors.is_empty() {
                lassos.push((steps.len(), true));
            }
            for (entry, node) in nodes.iter().enumerate().take(nodes.len() - 1) {
                if node.key == last.key {
                    lassos.push((entry, false));
                }
            }
            for (entry, terminal) in lassos {
                if self.satisfies(&nodes, &steps, entry, terminal, fairness) {
                    return true;
                }
            }
            if steps.len() < depth {
                for (transition, post) in successors {
                    let post = self.node(post);
                    let mut next_steps = steps.clone();
                    next_steps.push(qsl_replay::ModelStep {
                        transition,
                        post_state: post.key,
                    });
                    let mut next_nodes: Vec<Node> = nodes
                        .iter()
                        .map(|node| Node {
                            state: node.state.clone(),
                            key: node.key,
                        })
                        .collect();
                    next_nodes.push(post);
                    stack.push((next_steps, next_nodes));
                }
            }
        }
        false
    }
}

/// Whether `lasso` is a witness for the world's clause under `fairness`.
fn is_witness(world: &World, lasso: &ModelLasso, fairness: Fairness) -> bool {
    Oracle::new(world).is_witness(lasso, fairness)
}

/// The input of the model step `step`, which must be a `step(i)`.
fn input(step: &qsl_replay::ModelStep) -> i64 {
    input_of(&qsl_eval::hyper::StepLabel::Operation(&step.transition))
        .expect("the step is a `step`")
}

/// The `(h, l)` of each position of the lasso's path.
fn positions(world: &World, lasso: &ModelLasso) -> Vec<(i64, i64)> {
    Oracle::new(world)
        .replayed(lasso)
        .iter()
        .map(|node| (high(&node.state), low(&node.state)))
        .collect()
}

/// The secure vault's lasso for `eventually always holds(v.l @ b = 1)` from
/// initial state `initial` with secret `secret`: the stem `step(1)` and the
/// loop `step(1)`.
fn settles(world: &World, witness: &LassoWitness, initial: usize, secret: i64) {
    let lasso = &witness.lasso;
    assert_eq!(lasso.path.initial as usize, initial);
    assert_eq!(lasso.loop_entry, 1);
    let inputs: Vec<i64> = lasso.path.steps.iter().map(input).collect();
    assert_eq!(inputs, [1, 1]);
    assert_eq!(
        positions(world, lasso),
        [(secret, 0), (secret, 1), (secret, 1)]
    );
    assert!(is_witness(world, lasso, Fairness::None));
}

/// Over the secure vault, `exists trace b of V { eventually
/// always holds(v.l @ b = 1) }` with `witness_samples` 0 is `Witnessed` with
/// one lasso per initial state, in initial-state order, each the stem
/// `step(1)` and the loop `step(1)` that the oracle reads back, source
/// `Explored`; the product exploration completed with no open node.
///
/// Engine half of the criterion: the outcome shape. Remaining work: QSL-964 (replay, certificate, settlement).
#[trace("TC-606", "FR-181-AC-1")]
#[test]
fn the_secure_vault_is_witnessed_by_a_step_1_loop_per_initial_state() {
    let world = World::new(Vault::Secure, false, &STARTS, &clause("", SETTLES));
    let result = single(&world, unsampled());
    let lassos = witnessed(&result.outcome);
    assert_eq!(lassos.len(), 2);
    for (initial, witness) in lassos.iter().enumerate() {
        settles(&world, witness, initial, initial as i64);
        assert_eq!(witness.source, WitnessSource::Explored);
    }
    assert_eq!(result.instance, [("v".to_owned(), "v".to_owned())]);
    assert!(result.sampled.is_empty());
    assert_eq!(result.walks_stopped, 0);
    // Per initial state, the automaton of `eventually always p` waits (`q0`)
    // or has committed to `always p` (`q1`), or is dead: `(h, 0)` with `q0`,
    // `(h, 1)` with `q1` and with `q0`, and the dead pair at `(h, 0)` and at
    // `(h, 1)`, which keep reading letters; five, so ten in all.
    assert_eq!(result.stats.product_states, 10);
}

/// With the default `witness_samples`, the same claim is `Witnessed` too, from
/// sampled lassos that name their walk: the seed 0 and a trace index in the
/// initial state's block of 64, and the record's sampled lassos are the
/// outcome's.
///
/// Engine half of the criterion: the outcome shape. Remaining work: QSL-964 (replay, certificate, settlement).
#[trace("TC-606", "FR-181-AC-1")]
#[test]
fn sampled_witnesses_name_their_walk_by_seed_and_trace_index() {
    let world = World::new(Vault::Secure, false, &STARTS, &clause("", SETTLES));
    let result = single(&world, ModelCheckLimits::default());
    let lassos = witnessed(&result.outcome);
    assert_eq!(lassos.len(), 2);
    for (initial, witness) in lassos.iter().enumerate() {
        let WitnessSource::Sampled(provenance) = &witness.source else {
            panic!("expected a sampled witness, got {:?}", witness.source);
        };
        assert_eq!(provenance.seed, 0);
        assert!(
            (initial as u64 * 64..initial as u64 * 64 + 64).contains(&provenance.trace),
            "trace {} is not of initial state {initial}",
            provenance.trace
        );
        assert_eq!(provenance.stopped, StopReason::Reached);
        // A sampled walk ends at its first revisit; the oracle reads it back.
        assert!(is_witness(&world, &witness.lasso, Fairness::None));
        assert_eq!(result.sampled[initial].as_ref(), Some(witness));
    }
}

/// The request's seed names phase 0's walks: another seed draws other walks,
/// and the run is a function of the seed.
///
/// Engine half of the criterion: the outcome shape. Remaining work: QSL-964 (replay, certificate, settlement).
#[trace("TC-606", "FR-181-AC-1")]
#[test]
fn the_seed_names_the_walks_and_the_run_is_reproducible() {
    let world = World::new(Vault::Secure, false, &STARTS, &clause("", SETTLES));
    let run = |seed| {
        world
            .run_single(ModelCheckLimits::default(), seed, &Cancel::new())
            .expect("the run is not refused")
    };
    assert_eq!(run(None).seed, 0);
    assert_eq!(run(Some(7)).seed, 7);
    assert_eq!(run(Some(7)), run(Some(7)));
    let trace_of = |run: &qsl_analyze::model_check::SingleExistentialRun| {
        witnessed(&run.instances[0].outcome)
            .iter()
            .map(|witness| match &witness.source {
                WitnessSource::Sampled(provenance) => (provenance.seed, provenance.trace),
                WitnessSource::Explored => panic!("expected a sampled witness"),
            })
            .collect::<Vec<_>>()
    };
    assert!(trace_of(&run(Some(7))).iter().all(|(seed, _)| *seed == 7));
    assert_ne!(
        trace_of(&run(Some(7))),
        trace_of(&run(Some(8))),
        "the seeds draw different walks"
    );
}

/// The claim `exists trace b of V { eventually holds(v.l @ b = 2) }`, which
/// the output's type never allows, is `Trapped` at initial state 0 with an
/// empty stem: each initial state's part of the product is closed and holds no
/// fair accepting cycle, and the first in subject order is named.
///
/// Engine half of the criterion: the outcome shape. Remaining work: QSL-964 (replay, certificate, settlement).
#[trace("TC-606", "FR-181-AC-2")]
#[test]
fn an_unreachable_output_traps_the_first_initial_state() {
    let world = World::new(
        Vault::Secure,
        false,
        &STARTS,
        &clause("", "eventually holds(v.l @ b = 2)"),
    );
    for limits in [unsampled(), ModelCheckLimits::default()] {
        let result = single(&world, limits);
        assert_eq!(
            result.outcome,
            ExistentialOutcome::Trapped {
                stem: ModelPath {
                    initial: 0,
                    steps: Vec::new(),
                }
            }
        );
    }
}

/// The trap names the first initial state in subject order whose part is
/// closed with no fair accepting cycle, not the first initial state: a start
/// at `l = 1` has `always holds(v.l @ b = 0)` false at its position 0, and
/// the start before it settles.
///
/// Engine half of the criterion: the outcome shape. Remaining work: QSL-964 (replay, certificate, settlement).
#[trace("TC-606", "FR-181-AC-2")]
#[test]
fn the_trap_is_the_first_initial_state_with_no_witness() {
    let world = World::new(
        Vault::Secure,
        false,
        &[(0, 0), (0, 1)],
        &clause("", "always holds(v.l @ b = 0)"),
    );
    let result = single(&world, unsampled());
    assert_eq!(
        result.outcome,
        ExistentialOutcome::Trapped {
            stem: ModelPath {
                initial: 1,
                steps: Vec::new(),
            }
        }
    );
    // The same start order with the claim true of both: both settle.
    let both = World::new(
        Vault::Secure,
        false,
        &[(0, 0), (1, 0)],
        &clause("", "always holds(v.l @ b = 0)"),
    );
    let lassos = witnessed(&single(&both, unsampled()).outcome).to_vec();
    assert_eq!(lassos.len(), 2);
    for witness in &lassos {
        assert_eq!(witness.lasso.loop_entry, 0);
        assert_eq!(witness.lasso.path.steps.len(), 1);
        assert_eq!(input(&witness.lasso.path.steps[0]), 0);
    }
}

/// Over the secure vault with `reset`, the claim under
/// `fair weak V::Vault::reset;` is `Trapped` at initial state 0, since a loop
/// of `step(1)` leaves `reset` enabled and untaken, and a loop that takes
/// `reset` has `l = 0`; the same clause with no fairness set is `Witnessed`.
///
/// Engine half of the criterion: the outcome shape. Remaining work: QSL-964 (replay, certificate, settlement).
#[trace("TC-606", "FR-181-AC-3")]
#[test]
fn a_fairness_set_on_the_existential_variable_changes_the_verdict() {
    let fair = World::new(Vault::Secure, true, &STARTS, &clause(weak(), SETTLES));
    assert_eq!(
        single(&fair, unsampled()).outcome,
        ExistentialOutcome::Trapped {
            stem: ModelPath {
                initial: 0,
                steps: Vec::new(),
            }
        }
    );
    // Sampling finds the unfair loop `step(1)` and rejects it.
    let sampled = single(&fair, ModelCheckLimits::default());
    assert_eq!(sampled.sampled, vec![None, None]);
    assert!(matches!(
        sampled.outcome,
        ExistentialOutcome::Trapped { .. }
    ));
    let free = World::new(Vault::Secure, true, &STARTS, &clause("", SETTLES));
    let result = single(&free, unsampled());
    for (initial, witness) in witnessed(&result.outcome).iter().enumerate() {
        assert_eq!(witness.lasso.path.initial as usize, initial);
        assert!(is_witness(&free, &witness.lasso, Fairness::None));
    }
}

/// A loop that takes the fair operation is fair: with `reset` the claim
/// `eventually always holds(v.l @ b = 0)` holds under `fair weak reset` by the
/// loop `reset`, and a fairness set on an operation the loop does not need
/// does not hide a witness.
///
/// Engine half of the criterion: the outcome shape. Remaining work: QSL-964 (replay, certificate, settlement).
#[trace("TC-606", "FR-181-AC-3")]
#[test]
fn a_loop_that_takes_the_fair_operation_is_a_witness() {
    let world = World::new(
        Vault::Secure,
        true,
        &STARTS,
        &clause(weak(), "eventually always holds(v.l @ b = 0)"),
    );
    let result = single(&world, unsampled());
    let lassos = witnessed(&result.outcome);
    assert_eq!(lassos.len(), 2);
    for witness in lassos {
        assert!(is_witness(&world, &witness.lasso, Fairness::Weak));
    }
}

/// The claim of the first test with `max_depth` 1 and `witness_samples` 0
/// is `NoDecision`: the run completed through depth 0 and the states at the
/// horizon are open with cause `MaxDepth`, which the possible family's map
/// settles `BoundReached{depth: 1}`.
///
/// Engine half of the criterion: the outcome shape. Remaining work: QSL-964 (replay, certificate, settlement).
#[trace("TC-606", "FR-181-AC-4")]
#[test]
fn a_horizon_of_one_leaves_the_states_at_depth_one_open() {
    let mut world = World::new(Vault::Secure, false, &STARTS, &clause("", SETTLES));
    world.max_depth = Some(1);
    let result = single(&world, unsampled());
    assert_eq!(
        result.outcome,
        ExistentialOutcome::NoDecision {
            end: RunEnd::Completed,
            open: [OpenCause::MaxDepth].into(),
        }
    );
    // The horizon is a method parameter: it counts no limit.
    assert_eq!(result.stats.depth, 1);
    // Depth 2 reaches the loop `step(1)` of the stem `step(1)` at depth 1.
    world.max_depth = Some(2);
    let result = single(&world, unsampled());
    assert_eq!(
        witnessed_or_unchecked(&result.outcome).len(),
        2,
        "{:?}",
        result.outcome
    );
}

fn witnessed_or_unchecked(outcome: &ExistentialOutcome) -> &[LassoWitness] {
    match outcome {
        ExistentialOutcome::WitnessedUnchecked { lassos, .. } => lassos,
        other => panic!("expected WitnessedUnchecked, got {other:?}"),
    }
}

/// Over the secure vault, `eventually always holds(1 / (1 - v.l @
/// b) = 1 and v.l @ b = 0)`, whose division has no value at `l = 1`, is
/// `Undefined` at the first product state with `l = 1`, in both runs: the
/// path is the one step `step(1)` from initial state 0, `where` names position
/// 1, the atom and the division's cause. With sampling on, phase 0 found a
/// witness lasso per initial state looping `step(0)` at `(h, 0)`, which does
/// not settle the item.
///
/// Engine half of the criterion: the outcome shape. Remaining work: QSL-964 (replay, certificate, settlement).
#[trace("TC-615", "FR-181-AC-5")]
#[test]
fn an_undefined_letter_refutes_although_every_initial_state_has_a_witness() {
    let body = "eventually always holds(1 / (1 - v.l @ b) = 1 and v.l @ b = 0)";
    let world = World::new(Vault::Secure, false, &STARTS, &clause("", body));
    for limits in [unsampled(), ModelCheckLimits::default()] {
        let result = single(&world, limits);
        let ExistentialOutcome::Undefined { stem, undefined } = &result.outcome else {
            panic!("expected Undefined, got {:?}", result.outcome);
        };
        assert_eq!(stem.initial, 0);
        assert_eq!(stem.steps.len(), 1);
        assert_eq!(input(&stem.steps[0]), 1);
        assert_eq!(undefined.site.position, 1);
        assert_eq!(undefined.site.state_key, stem.steps[0].post_state);
        assert_eq!(undefined.site.transition, None);
        let qsl_replay::UndefinedLocus::Atom(atom) = undefined.site.locus else {
            panic!("expected an atom, got {:?}", undefined.site.locus);
        };
        assert_eq!(
            world.atom_text(atom),
            "holds(1 / (1 - v.l @ b) = 1 and v.l @ b = 0)"
        );
        assert_eq!(
            undefined.cause,
            UndefinedCause::Kernel(Undefined::DivisionByZero)
        );
    }
    let sampled = single(&world, ModelCheckLimits::default());
    assert_eq!(sampled.sampled.len(), 2);
    for (initial, witness) in sampled.sampled.iter().enumerate() {
        let witness = witness.as_ref().expect("phase 0 found a witness");
        let lasso = &witness.lasso;
        assert_eq!(lasso.path.initial as usize, initial);
        assert_eq!(lasso.loop_entry, 0);
        assert_eq!(lasso.path.steps.len(), 1);
        assert_eq!(input(&lasso.path.steps[0]), 0);
        assert_eq!(positions(&world, lasso), [(initial as i64, 0); 2]);
        assert!(matches!(witness.source, WitnessSource::Sampled(_)));
    }
}

/// An undefined letter at an initial position ends the instance at path length
/// 0 although the start's part is also closed with no fair accepting cycle:
/// the letter is read before the automaton steps, so it comes first.
///
/// Engine half of the criterion: the outcome shape. Remaining work: QSL-964 (replay, certificate, settlement).
#[trace("TC-615", "FR-181-AC-5")]
#[test]
fn an_undefined_letter_at_a_trapped_start_is_undefined_not_trapped() {
    let body = "always holds(1 / (1 - v.l @ b) = 1 and v.l @ b = 0)";
    let world = World::new(Vault::Secure, false, &[(0, 0), (0, 1)], &clause("", body));
    let result = single(&world, unsampled());
    let ExistentialOutcome::Undefined { stem, undefined } = &result.outcome else {
        panic!("expected Undefined, got {:?}", result.outcome);
    };
    assert_eq!(stem.initial, 1);
    assert!(stem.steps.is_empty());
    assert_eq!(undefined.site.position, 0);
}

/// A trap comes before an undefined letter later in canonical breadth-first
/// order: `holds(l = 1)` is false at the starts, so no fair accepting cycle
/// is left, and the division is undefined only after `step(1)`.
///
/// Engine half of the criterion: the outcome shape. Remaining work: QSL-964 (replay, certificate, settlement).
#[trace("TC-615", "FR-181-AC-5")]
#[test]
fn a_trap_comes_before_a_later_undefined_letter() {
    let body = "holds(v.l @ b = 1) and always holds(1 / (1 - v.l @ b) = 1 and v.l @ b = 0)";
    let world = World::new(Vault::Secure, false, &STARTS, &clause("", body));
    let result = single(&world, unsampled());
    assert_eq!(
        result.outcome,
        ExistentialOutcome::Trapped {
            stem: ModelPath {
                initial: 0,
                steps: Vec::new(),
            }
        }
    );
}

/// The claim of the first test with the default
/// `witness_samples` is `Witnessed` only from a product exploration that
/// completed with no open node; the same run under `max_depth` 2 has open
/// states at the horizon and is not.
///
/// Engine half of the criterion: the outcome shape. Remaining work: QSL-964 (replay, certificate, settlement).
#[trace("TC-615", "FR-181-AC-5")]
#[test]
fn a_witnessed_outcome_needs_a_completed_run_with_no_open_node() {
    let mut world = World::new(Vault::Secure, false, &STARTS, &clause("", SETTLES));
    let result = single(&world, ModelCheckLimits::default());
    assert!(matches!(
        result.outcome,
        ExistentialOutcome::Witnessed { .. }
    ));
    world.max_depth = Some(2);
    let result = single(&world, ModelCheckLimits::default());
    let ExistentialOutcome::WitnessedUnchecked { lassos, end, open } = &result.outcome else {
        panic!("expected WitnessedUnchecked, got {:?}", result.outcome);
    };
    assert_eq!(lassos.len(), 2);
    assert_eq!(*end, RunEnd::Completed);
    assert_eq!(*open, [OpenCause::MaxDepth].into());
}

/// The claim of the first test with the default `witness_samples` and
/// `max_states` 2 gets a sampled witness lasso per initial state, the
/// exploration stops at `max_states`, and the instance is `WitnessedUnchecked`
/// with the run's end and open causes; with `witness_samples` 0 the same run
/// is `NoDecision` with the same end.
///
/// Engine half of the criterion: the outcome shape. Remaining work: QSL-964 (replay, certificate, settlement).
#[trace("TC-616", "FR-181-AC-6")]
#[test]
fn a_stopped_exploration_with_sampled_witnesses_is_witnessed_unchecked() {
    let world = World::new(Vault::Secure, false, &STARTS, &clause("", SETTLES));
    let stopped = RunEnd::Stopped {
        cause: IncompleteCause::ResourceExhausted,
        limit: Some(ReachedLimit {
            limit: ModelCheckLimit::MaxStates,
            value: 2,
        }),
    };
    let mut limits = ModelCheckLimits::default();
    limits.limits.max_states = 2;
    let result = single(&world, limits);
    let ExistentialOutcome::WitnessedUnchecked { lassos, end, open } = &result.outcome else {
        panic!("expected WitnessedUnchecked, got {:?}", result.outcome);
    };
    assert_eq!(*end, stopped);
    assert_eq!(*open, [OpenCause::NotExpanded].into());
    assert_eq!(lassos.len(), 2);
    for (initial, witness) in lassos.iter().enumerate() {
        assert!(matches!(witness.source, WitnessSource::Sampled(_)));
        assert!(is_witness(&world, &witness.lasso, Fairness::None));
        assert_eq!(witness.lasso.path.initial as usize, initial);
    }
    assert_eq!(result.stats.product_states, 2);

    let mut limits = unsampled();
    limits.limits.max_states = 2;
    let result = single(&world, limits);
    assert_eq!(
        result.outcome,
        ExistentialOutcome::NoDecision {
            end: stopped,
            open: [OpenCause::NotExpanded].into(),
        }
    );
}

/// A part the run did not finish is not a trap: `max_transitions` 1 stops the
/// exploration inside initial state 0's part, and the second initial state's
/// part is never discovered, so neither is closed.
#[trace("TC-606")]
#[test]
fn an_unfinished_part_is_not_a_trap() {
    let world = World::new(
        Vault::Secure,
        false,
        &STARTS,
        &clause("", "eventually holds(v.l @ b = 2)"),
    );
    for (states, transitions) in [(2, usize::MAX), (usize::MAX, 1), (1, usize::MAX)] {
        let mut limits = unsampled();
        limits.limits.max_states = states;
        limits.limits.max_transitions = transitions;
        let result = single(&world, limits);
        assert!(
            matches!(
                result.outcome,
                ExistentialOutcome::NoDecision {
                    end: RunEnd::Stopped { .. },
                    ..
                }
            ),
            "{states} states, {transitions} transitions: {:?}",
            result.outcome
        );
    }
}

/// A cycle through a state the run only partly expanded is not counted: what
/// the state enables is not known, so a weak constraint cannot be shown
/// disabled there. With `fair weak V::Vault::step;` the loop `reset` at `(0, 0)`
/// is unfair, since `step` is enabled and untaken; `max_transitions` 1 stops the
/// expansion of `(0, 0)` after its `reset` edge, which a cycle that ignored the
/// missing enabledness would take for fair. The run with no limit finds the
/// fair loop `step(0)`.
#[trace("TC-606")]
#[test]
fn a_cycle_through_a_partly_expanded_state_is_not_a_witness() {
    let world = World::new(
        Vault::Secure,
        true,
        &[(0, 0)],
        &clause("fair weak V::Vault::step;", "always holds(v.l @ b = 0)"),
    );
    let mut limits = unsampled();
    limits.limits.max_transitions = 1;
    let result = single(&world, limits);
    assert_eq!(
        result.outcome,
        ExistentialOutcome::NoDecision {
            end: RunEnd::Stopped {
                cause: IncompleteCause::ResourceExhausted,
                limit: Some(ReachedLimit {
                    limit: ModelCheckLimit::MaxTransitions,
                    value: 1,
                }),
            },
            open: [OpenCause::NotExpanded].into(),
        }
    );
    let result = single(&world, unsampled());
    let lassos = witnessed(&result.outcome);
    assert_eq!(lassos.len(), 1);
    assert_eq!(lassos[0].lasso.path.steps.len(), 1);
    assert_eq!(input(&lassos[0].lasso.path.steps[0]), 0);
}

/// The run's `max_states` is shared by its instances: two vaults in one start
/// are two instances, each with a product of its own, and the budget the first
/// uses is not the second's.
#[trace("TC-606", "FR-181-AC-7")]
#[test]
fn instances_share_the_runs_limits() {
    let objects = [("v", (0, 0)), ("w", (0, 1))];
    let world = World::with_vaults(
        Vault::Secure,
        &objects,
        &clause("", "always holds(v.l @ b = 0)"),
    );
    let run = world
        .run_single(unsampled(), None, &Cancel::new())
        .expect("not refused");
    // `v` starts at `l = 0` and stays there; `w` starts at `l = 1`.
    let keys: Vec<_> = run
        .instances
        .iter()
        .map(|instance| instance.instance.clone())
        .collect();
    assert_eq!(
        keys,
        [
            vec![("v".to_owned(), "v".to_owned())],
            vec![("v".to_owned(), "w".to_owned())]
        ]
    );
    assert!(matches!(
        run.instances[0].outcome,
        ExistentialOutcome::Witnessed { .. }
    ));
    assert_eq!(
        run.instances[1].outcome,
        ExistentialOutcome::Trapped {
            stem: ModelPath {
                initial: 0,
                steps: Vec::new(),
            }
        }
    );
    // The budget the first instance's product spends is not the second's.
    let used = run.instances[0].stats.product_states;
    assert!(used >= 1);
    let mut limits = unsampled();
    limits.limits.max_states = usize::try_from(used).expect("small");
    let tight = world
        .run_single(limits, None, &Cancel::new())
        .expect("not refused");
    assert!(matches!(
        tight.instances[0].outcome,
        ExistentialOutcome::Witnessed { .. }
    ));
    assert!(
        matches!(
            tight.instances[1].outcome,
            ExistentialOutcome::NoDecision {
                end: RunEnd::Stopped { .. },
                ..
            }
        ),
        "{:?}",
        tight.instances[1].outcome
    );
}

/// A start with no vault gives the clause no instance, and a subject with no
/// initial state gives each instance `NoInitialState`.
#[trace("TC-606", "FR-181-AC-7")]
#[test]
fn no_instance_and_no_initial_state_decide_nothing() {
    let none = World::with_vaults(Vault::Secure, &[], &clause("", SETTLES));
    let run = none
        .run_single(unsampled(), None, &Cancel::new())
        .expect("not refused");
    assert!(run.instances.is_empty());
    let empty = World::new(Vault::Secure, false, &[], &clause("", SETTLES));
    let result = single(&empty, ModelCheckLimits::default());
    assert_eq!(result.outcome, ExistentialOutcome::NoInitialState);
}

/// The clause must be HP-5: a universal clause is refused, naming its form.
#[trace("TC-606")]
#[test]
fn a_clause_that_is_not_single_existential_is_refused() {
    let world = World::new(Vault::Secure, false, &STARTS, &non_interference(""));
    let refusal = world
        .run_single(unsampled(), None, &Cancel::new())
        .expect_err("a universal clause is refused");
    assert!(
        matches!(
            refusal,
            ModelCheckRefusal::Hyper(HyperRefusal::NotSingleExistential {
                form: Some(HyperForm::Universal)
            })
        ),
        "{refusal:?}"
    );
}

/// The leaky `step`, with a precondition that has no value in the state
/// `(0, 1)`.
const GUARDED: Vault = Vault::Custom(
    "pre Guard using v on V::Vault::step { 1 div (1 - self.l + self.h) >= 0 }\n\
     post Step using v on V::Vault::step { (i = self.h and self.l = 0) or (i != self.h and self.l = 1) }\n",
);

/// A state whose expansion evaluated a contract conjunction with no value is
/// open with cause `UndecidedSuccessor` (ADR-022 GM-6): the lasso that reaches
/// the claim there is not a witness, since what the state enables is unknown,
/// and the instance is `NoDecision`, never `Witnessed`.
#[trace("TC-616")]
#[test]
fn a_contract_with_no_value_leaves_its_state_open() {
    let world = World::new(
        GUARDED,
        false,
        &[(0, 0)],
        &clause("", "eventually holds(v.l @ b = 1)"),
    );
    let result = single(&world, unsampled());
    // `(0, 1)` looks terminal, but whether its `step` is enabled is unknown,
    // so the stutter loop there is no witness and nothing is decided.
    assert_eq!(
        result.outcome,
        ExistentialOutcome::NoDecision {
            end: RunEnd::Completed,
            open: [OpenCause::UndecidedSuccessor].into(),
        }
    );
}

/// The run's limits and cancellation stop it: a witness-sample count whose
/// trace indices overflow stops before the first walk naming
/// `witness_samples`, a cancelled run is stopped as `Cancelled`, and
/// `max_automaton_states` 1 stops the exploration naming that limit.
#[trace("TC-616")]
#[test]
fn the_limits_and_cancellation_stop_the_run() {
    let world = World::new(Vault::Secure, false, &STARTS, &clause("", SETTLES));
    let limits = ModelCheckLimits {
        witness_samples: u64::MAX,
        ..ModelCheckLimits::default()
    };
    let result = single(&world, limits);
    assert_eq!(
        result.outcome,
        ExistentialOutcome::NoDecision {
            end: RunEnd::Stopped {
                cause: IncompleteCause::ResourceExhausted,
                limit: Some(ReachedLimit {
                    limit: ModelCheckLimit::WitnessSamples,
                    value: u64::MAX,
                }),
            },
            open: Default::default(),
        }
    );

    let mut limits = unsampled();
    limits.max_automaton_states = 1;
    let result = single(&world, limits);
    let ExistentialOutcome::NoDecision { end, .. } = &result.outcome else {
        panic!("expected NoDecision, got {:?}", result.outcome);
    };
    assert_eq!(
        *end,
        RunEnd::Stopped {
            cause: IncompleteCause::ResourceExhausted,
            limit: Some(ReachedLimit {
                limit: ModelCheckLimit::MaxAutomatonStates,
                value: 1,
            }),
        }
    );

    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    let run = world
        .run_single(unsampled(), None, &cancel)
        .expect("not refused");
    assert_eq!(
        run.instances[0].outcome,
        ExistentialOutcome::NoDecision {
            end: RunEnd::Stopped {
                cause: IncompleteCause::Cancelled {
                    source: CancelCause::Requested
                },
                limit: None,
            },
            open: Default::default(),
        }
    );
}

/// The walks `max_walk_steps` stopped are counted (FR-166): with one step per
/// walk, a walk that does not step back onto its state ends at the ceiling,
/// and each initial state draws walks until one revisits, so the count is the
/// walks before each success: the trace index less the initial state's base.
#[trace("TC-616")]
#[test]
fn walks_the_step_ceiling_stopped_are_counted() {
    let world = World::new(
        Vault::Secure,
        false,
        &STARTS,
        &clause("", "always holds(v.l @ b = 0)"),
    );
    let limits = ModelCheckLimits {
        max_walk_steps: 1,
        ..ModelCheckLimits::default()
    };
    let result = single(&world, limits);
    let mut expected = 0;
    for (initial, witness) in result.sampled.iter().enumerate() {
        let witness = witness.as_ref().expect("a walk steps back onto its state");
        let WitnessSource::Sampled(provenance) = &witness.source else {
            panic!("expected a sampled witness");
        };
        assert_eq!(witness.lasso.loop_entry, 0);
        assert_eq!(witness.lasso.path.steps.len(), 1);
        expected += provenance.trace - initial as u64 * 64;
    }
    assert_eq!(result.walks_stopped, expected);
}

/// A terminal state is a lasso whose loop is the stutter step (ADR-018 SM-4):
/// `step` is enabled only while `l = 0`, so `step(1)` ends in a terminal state
/// and the witness is the path to it with the loop
/// entry at its end. A walk reaching the terminal state revisits nothing, so
/// it is explored.
///
/// Engine half of the criterion: the outcome shape. Remaining work: QSL-964 (replay, certificate, settlement).
#[trace("TC-606", "FR-181-AC-1")]
#[test]
fn a_terminal_state_is_a_stutter_lasso() {
    let world = World::new(Vault::Terminal, false, &STARTS, &clause("", SETTLES));
    let result = single(&world, ModelCheckLimits::default());
    let lassos = witnessed(&result.outcome);
    assert_eq!(lassos.len(), 2);
    for (initial, witness) in lassos.iter().enumerate() {
        assert_eq!(witness.source, WitnessSource::Explored);
        let lasso = &witness.lasso;
        assert_eq!(lasso.path.initial as usize, initial);
        assert_eq!(lasso.loop_entry, 1);
        assert_eq!(lasso.path.steps.len(), 1);
        assert_eq!(input(&lasso.path.steps[0]), 1);
        assert_eq!(
            positions(&world, lasso),
            [(initial as i64, 0), (initial as i64, 1)]
        );
        assert!(is_witness(&world, lasso, Fairness::None));
    }
    assert_eq!(result.sampled, vec![None, None]);
    // A walk that ends at the terminal state is not stopped by the ceiling.
    assert_eq!(result.walks_stopped, 0);
}

/// The brute-force oracle: over the secure, leaky and terminal vaults, with and
/// without `reset`, under no, weak and strong fairness on `reset`, for bodies
/// of each temporal shape, the engine finds a witness for an initial state
/// exactly when some lasso of at most three steps is a witness, and every
/// lasso it returns is one the oracle reads back. Where an initial state has
/// none, the trap names the first such, in subject order.
///
/// Engine half of the criterion: the outcome shape. Remaining work: QSL-964 (replay, certificate, settlement).
#[trace("TC-606", "FR-181-AC-1", "FR-181-AC-2", "FR-181-AC-3")]
#[test]
fn the_engine_agrees_with_a_brute_force_search_of_lassos() {
    let bodies = [
        "eventually always holds(v.l @ b = 1)",
        "eventually always holds(v.l @ b = 0)",
        "always holds(v.l @ b = 0)",
        "eventually holds(v.l @ b = 1)",
        "always eventually holds(v.l @ b = 1)",
        "holds(v.l @ b = 0) and eventually always holds(v.l @ b = 1)",
        "holds(v.l @ b = 1) or always holds(v.l @ b = 0)",
        "holds(v.l @ b = 1) or eventually holds(v.l @ b = 0)",
        "(not holds(v.l @ b = 1)) or ((holds(v.l @ b = 1) until holds(v.l @ b = 1)) \
         or (holds(v.l @ b = 1) or holds(v.l @ b = 1)))",
    ];
    let mut witnesses = 0;
    let mut traps = 0;
    for vault in [Vault::Secure, Vault::Leaky, Vault::Terminal] {
        for reset in [false, true] {
            let fairness: &[Fairness] = if reset {
                &[Fairness::None, Fairness::Weak, Fairness::Strong]
            } else {
                &[Fairness::None]
            };
            for fairness in fairness.iter().copied() {
                for starts in [&STARTS, &MIXED] {
                    for body in bodies {
                        let world =
                            World::new(vault, reset, starts, &clause(fairness.text(), body));
                        let oracle = Oracle::new(&world);
                        let expected: Vec<bool> = (0..starts.len())
                            .map(|initial| oracle.exists(initial, fairness, 3))
                            .collect();
                        for limits in [unsampled(), ModelCheckLimits::default()] {
                            let result = single(&world, limits);
                            let context = format!(
                                "{} reset {reset} {starts:?} {body}: {expected:?} {:?}",
                                fairness.text(),
                                result.outcome
                            );
                            match &result.outcome {
                                ExistentialOutcome::Witnessed { lassos } => {
                                    assert!(expected.iter().all(|found| *found), "{context}");
                                    assert_eq!(lassos.len(), starts.len(), "{context}");
                                    for (initial, witness) in lassos.iter().enumerate() {
                                        assert_eq!(
                                            witness.lasso.path.initial as usize, initial,
                                            "{context}"
                                        );
                                        assert!(
                                            oracle.is_witness(&witness.lasso, fairness),
                                            "{context}"
                                        );
                                    }
                                    witnesses += 1;
                                }
                                ExistentialOutcome::Trapped { stem } => {
                                    let first = expected.iter().position(|found| !found);
                                    assert_eq!(first, Some(stem.initial as usize), "{context}");
                                    assert!(stem.steps.is_empty(), "{context}");
                                    traps += 1;
                                }
                                other => panic!("{context}: undecided {other:?}"),
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(
        witnesses > 40 && traps > 40,
        "{witnesses} witnessed, {traps} trapped"
    );
}

/// The leaky-free `step` whose precondition has no value at `(h, l) = (0, 1)`,
/// with a `reset` that keeps `l` at 1, so `(0, 1)` has a model loop whose
/// state also enables a `step` the contracts cannot decide.
const GUARDED_RESET: Vault = Vault::Custom(
    "pre Guard using v on V::Vault::step { 1 div (1 - self.l + self.h) >= 0 }\n\
     post Step using v on V::Vault::step { self.l = i }\n\
     post Reset using v on V::Vault::reset { self.l = 1 }\n",
);

/// A loop through a state whose expansion evaluated a contract with no value
/// is not shown fair: whether `step` is enabled at `(0, 1)` is unknown, so the
/// `reset` loop there is no witness under `fair weak step`, in the explored
/// product and in phase 0 alike. The state is open, the run completed, and
/// nothing is decided.
#[trace("TC-606")]
#[test]
fn a_loop_through_an_undecided_state_is_not_a_witness() {
    let world = World::new(
        GUARDED_RESET,
        true,
        &[(0, 1)],
        &clause("fair weak V::Vault::step;", "always holds(v.l @ b = 1)"),
    );
    let undecided = ExistentialOutcome::NoDecision {
        end: RunEnd::Completed,
        open: [OpenCause::UndecidedSuccessor].into(),
    };
    let result = single(&world, unsampled());
    assert_eq!(result.outcome, undecided);
    let result = single(&world, ModelCheckLimits::default());
    assert_eq!(result.outcome, undecided);
    assert_eq!(
        result.sampled,
        vec![None],
        "phase 0 drops the candidate through the undecided state"
    );
}

/// A run that stops before it has a product graph, here `max_automaton_states`
/// 0, still returns the sampled witnesses it has: `WitnessedUnchecked`, with
/// the stop and no open node.
#[trace("TC-616")]
#[test]
fn a_run_with_no_graph_keeps_its_sampled_witnesses() {
    let world = World::new(Vault::Secure, false, &STARTS, &clause("", SETTLES));
    let limits = ModelCheckLimits {
        max_automaton_states: 0,
        ..ModelCheckLimits::default()
    };
    let result = single(&world, limits);
    let ExistentialOutcome::WitnessedUnchecked { lassos, end, open } = &result.outcome else {
        panic!("expected WitnessedUnchecked, got {:?}", result.outcome);
    };
    assert_eq!(
        *end,
        RunEnd::Stopped {
            cause: IncompleteCause::ResourceExhausted,
            limit: Some(ReachedLimit {
                limit: ModelCheckLimit::MaxAutomatonStates,
                value: 0,
            }),
        }
    );
    assert!(open.is_empty(), "{:?}", result.outcome);
    assert_eq!(lassos.len(), 2);
    for witness in lassos {
        assert!(matches!(witness.source, WitnessSource::Sampled(_)));
    }
}

/// The canonical lasso of FR-126 (CX-5), asserted where it matters. A part
/// with two fair accepting components, `eventually always l = 1` and
/// `eventually always l = 0`, gets the lasso of the one discovered first; and
/// a loop of two vault states, which must visit both `l = 0` and `l = 1`,
/// re-enters at the first state of its component.
#[trace("TC-606")]
#[test]
fn the_explored_lasso_is_the_canonical_one() {
    let two = World::new(
        Vault::Secure,
        false,
        &[(0, 0)],
        &clause(
            "",
            "(eventually always holds(v.l @ b = 1)) or (eventually always holds(v.l @ b = 0))",
        ),
    );
    let result = single(&two, unsampled());
    let lassos = witnessed(&result.outcome);
    let lasso = &lassos[0].lasso;
    assert_eq!(
        (
            lasso.loop_entry,
            lasso.path.steps.iter().map(input).collect::<Vec<_>>()
        ),
        (0, vec![0]),
        "{lasso:?}"
    );

    let both = World::new(
        Vault::Secure,
        false,
        &[(0, 0)],
        &clause(
            "",
            "(always eventually holds(v.l @ b = 1)) and (always eventually holds(v.l @ b = 0))",
        ),
    );
    let result = single(&both, unsampled());
    let lassos = witnessed(&result.outcome);
    let lasso = &lassos[0].lasso;
    assert_eq!(
        (
            lasso.loop_entry,
            lasso.path.steps.iter().map(input).collect::<Vec<_>>()
        ),
        (0, vec![1, 0]),
        "{lasso:?}"
    );
    assert!(is_witness(&both, lasso, Fairness::None));
}

/// Phase 0 with several instances: each walk is drawn once and tested against
/// every instance that has no witness yet, so each instance keeps its own first
/// sampled witness (or none) and its own stopped count. The first instance
/// (`w`, starting at `l = 1`) never has a witness, so every walk is tested
/// against the later ones.
#[trace("TC-606", "FR-181-AC-7")]
#[test]
fn sampling_gives_each_instance_its_own_first_witness() {
    let objects = [("v", (0, 0)), ("w", (0, 1)), ("x", (1, 0))];
    let world = World::with_vaults(Vault::Secure, &objects, &clause("", SETTLES));
    let run = world
        .run_single(ModelCheckLimits::default(), None, &Cancel::new())
        .expect("not refused");
    assert_eq!(run.instances.len(), 3);
    // The first walk (trace 0) is a witness for `w`, which starts at `l = 1`;
    // `x` and `v` need a walk that steps to `l = 1`, the third and the fourth.
    let traces: Vec<_> = run
        .instances
        .iter()
        .map(|instance| {
            let witness = instance.sampled[0].as_ref().expect("a sampled witness");
            let WitnessSource::Sampled(provenance) = &witness.source else {
                panic!("expected a sampled witness");
            };
            (instance.instance[0].1.clone(), provenance.trace)
        })
        .collect();
    assert_eq!(
        traces,
        [
            ("v".to_owned(), 3),
            ("w".to_owned(), 0),
            ("x".to_owned(), 2)
        ]
    );
    for instance in &run.instances {
        assert_eq!(instance.walks_stopped, 0);
    }
}
