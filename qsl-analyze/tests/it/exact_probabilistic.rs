// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-196 (TC-631, TC-641): EN-5's probabilistic product, over ADR-024 §7's
//! and ADR-028 §15's models (see [`crate::prob_world`]).

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use ix_trace_rs::trace;
use qsl_analyze::exact_probabilistic::{
    build_product, Accumulator, Amount, FormulaState, MeasurePhase, MonitorState, MonitorVerdict,
    PredicateRole, ProbBuild, ProbEdge, ProbProduct, ProbStateId, ProductAction, ProductKind,
    ProductRefusal, ProductStop, UndefinedRead,
};
use qsl_eval::simulation::Limits;
use qsl_eval::temporal::UndefinedCause;
use qsl_foundation::diagnostic::UndefinedReason;
use qsl_replay::{ModelCheckLimit, ModelCheckLimits};
use quire_exact::{Cancel, CancelCause, Integer, Rational, Value};

use crate::prob_world::{self, World};

fn ratio(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("nonzero")
}

fn integer_of(value: &Value) -> i64 {
    match value {
        Value::Integer(integer) => integer.to_string().parse().expect("small"),
        other => panic!("not an integer: {other:?}"),
    }
}

fn boolean_of(value: &Value) -> bool {
    match value {
        Value::Boolean(flag) => *flag,
        other => panic!("not a Boolean: {other:?}"),
    }
}

fn built(world: &World, claim: &str) -> ProbProduct {
    match build_product(
        world.request(claim, ModelCheckLimits::default()),
        &Cancel::new(),
    ) {
        Ok(ProbBuild::Built(product)) => product,
        other => panic!("{claim}: expected a product, got {other:?}"),
    }
}

fn refused(world: &World, claim: &str) -> ProductRefusal {
    match build_product(
        world.request(claim, ModelCheckLimits::default()),
        &Cancel::new(),
    ) {
        Err(refusal) => refusal,
        other => panic!("{claim}: expected a refusal, got {other:?}"),
    }
}

/// The probabilities of `edges`, summed.
fn total(edges: &[ProbEdge]) -> Rational {
    edges
        .iter()
        .fold(ratio(0, 1), |sum, edge| sum.add(&edge.probability))
}

/// The one state of `product` with no incoming edge and every initial state
/// of the subject: the product has one.
fn start(product: &ProbProduct) -> ProbStateId {
    let [(_, _, state)] = &product.initial[..] else {
        panic!("one initial state: {:?}", product.initial);
    };
    *state
}

/// Each state's breadth-first depth from `from`, by its index.
fn depths(product: &ProbProduct, from: ProbStateId) -> BTreeMap<usize, usize> {
    let mut depth = BTreeMap::from([(from.index(), 0)]);
    let mut queue = VecDeque::from([from]);
    while let Some(at) = queue.pop_front() {
        let here = depth[&at.index()];
        for edge in product.outgoing(at) {
            if let Entry::Vacant(slot) = depth.entry(edge.to.index()) {
                slot.insert(here + 1);
                queue.push_back(edge.to);
            }
        }
    }
    depth
}

/// `items` in ascending order, each once: a set whose members hold exact
/// rationals, which a `BTreeSet` key may not.
fn distinct<T: Ord>(mut items: Vec<T>) -> Vec<T> {
    items.sort();
    items.dedup();
    items
}

fn is_decided(product: &ProbProduct, state: ProbStateId) -> bool {
    product.states[state.index()]
        .monitor
        .is_some_and(MonitorState::is_decided)
}

const NO_FAULT: &str = "probabilistic NoFault using ev on M over (n: M::Node) under Steady \
     on origin { probability >= 0.999 [ always[0,1000] holds(n.healthy) ] }";

/// FR-196-AC-1 (TC-631 step 1): §15.1's `NoFault` product is a DTMC whose
/// undecided states hold the healthy model state at one position each, at
/// most 1,001 of them, plus one accepting and one rejecting decided state.
/// Every undecided state's outgoing probabilities sum to 1, and every path
/// reaches a decided state within 1,001 positions.
#[trace("TC-631", "FR-196-AC-1")]
#[test]
fn the_no_fault_product_is_a_layered_dtmc_with_two_decided_states() {
    let world = World::new(&prob_world::no_fault(NO_FAULT));
    let product = built(&world, "NoFault");
    assert_eq!(product.kind, ProductKind::Dtmc);
    let decided: BTreeSet<MonitorVerdict> = product
        .decided
        .iter()
        .map(|(_, verdict)| *verdict)
        .collect();
    assert_eq!(product.decided.len(), 2);
    assert_eq!(
        decided,
        BTreeSet::from([MonitorVerdict::Accept, MonitorVerdict::Reject])
    );
    let undecided: Vec<ProbStateId> = product
        .ids()
        .filter(|state| !product.outgoing(*state).is_empty())
        .collect();
    assert!(
        undecided.len() <= 1001,
        "{} undecided states",
        undecided.len()
    );
    assert_eq!(
        undecided.len() + product.decided.len(),
        product.states.len()
    );
    // One position each: the monitor state differs at every undecided state.
    let monitors: BTreeSet<_> = undecided
        .iter()
        .map(|state| product.states[state.index()].monitor)
        .collect();
    assert_eq!(monitors.len(), undecided.len());
    for state in &undecided {
        let edges = product.outgoing(*state);
        assert_eq!(edges.len(), 2, "tick draws fault or not");
        assert_eq!(total(edges), ratio(1, 1));
        let faulty = edges
            .iter()
            .find(|edge| boolean_of(&edge.draw.as_ref().expect("a draw")[0]));
        assert_eq!(
            faulty.map(|edge| edge.probability.clone()),
            Some(ratio(1, 10_000_000))
        );
    }
    for (state, _) in &product.decided {
        assert!(
            product.outgoing(*state).is_empty(),
            "a decided state is absorbing"
        );
    }
    let depth = depths(&product, start(&product));
    assert_eq!(
        depth.len(),
        product.states.len(),
        "every state is reachable"
    );
    let deepest = depth.values().copied().max().expect("states");
    assert!(
        deepest <= 1001,
        "every path is decided within {deepest} positions"
    );
    let accept = product
        .decided
        .iter()
        .find(|(_, verdict)| *verdict == MonitorVerdict::Accept)
        .expect("an accepting state");
    assert_eq!(depth[&accept.0.index()], 1000);
}

const P95: &str = "probabilistic P95 using ev on M over (s: M::Server) under Steady on origin \
     { quantile 0.95 of (accumulate duration from holds(s.phase = 1) until holds(s.phase = 2) \
     within 10) <= 5 }";

/// FR-196-AC-2 (TC-631 step 2): §15.2's product for `P95` at 5 carries the
/// `duration` accumulator, which takes the values 0, 1, 2, 3, 4 and the one
/// value above the threshold; the activation flag is set from position 1.
/// The edge for `attempt` with `d = 3`, `outcome = Ok` carries `343/5000`.
#[trace("TC-631", "FR-196-AC-2")]
#[test]
fn the_p95_product_accumulates_duration_and_saturates_above_the_threshold() {
    let world = World::new(&prob_world::service(P95));
    let product = built(&world, "P95");
    assert_eq!(product.kind, ProductKind::Dtmc);
    let held = distinct(
        product
            .states
            .iter()
            .map(|key| match &key.accumulator {
                Some(Accumulator::Reward { activated, amount }) => (*activated, amount.clone()),
                other => panic!("a reward accumulator: {other:?}"),
            })
            .collect(),
    );
    let exact = |value| (true, Amount::Exact(ratio(value, 1)));
    assert_eq!(
        held,
        distinct(vec![
            (false, Amount::Exact(ratio(0, 1))),
            exact(0),
            exact(1),
            exact(2),
            exact(3),
            exact(4),
            (true, Amount::AboveBound),
        ])
    );
    // Position 0 is not activated; the step into position 1 activates.
    let first = start(&product);
    assert!(matches!(
        product.states[first.index()].accumulator,
        Some(Accumulator::Reward {
            activated: false,
            ..
        })
    ));
    let [request] = product.outgoing(first) else {
        panic!("only `request` is enabled at Idle");
    };
    assert_eq!(request.probability, ratio(1, 1));
    let busy = request.to;
    assert_eq!(
        product.states[busy.index()].monitor,
        Some(MonitorState::Measure {
            remaining: 9,
            phase: MeasurePhase::Active
        })
    );
    assert_eq!(
        product.states[busy.index()].accumulator,
        Some(Accumulator::Reward {
            activated: true,
            amount: Amount::Exact(ratio(0, 1))
        })
    );
    assert_eq!(total(product.outgoing(busy)), ratio(1, 1));
    let three_ok = product
        .outgoing(busy)
        .iter()
        .find(|edge| {
            let draw = edge.draw.as_ref().expect("a draw");
            integer_of(&draw[0]) == 3 && boolean_of(&draw[1])
        })
        .expect("attempt with d = 3 and ok");
    assert_eq!(three_ok.probability, ratio(343, 5000));
    let (name, reward) = &three_ok.rewards[0];
    assert_eq!((name.as_str(), integer_of(reward)), ("duration", 3));
    let done = &product.states[three_ok.to.index()];
    assert_eq!(
        done.monitor,
        Some(MonitorState::Measure {
            remaining: 8,
            phase: MeasurePhase::Complete
        })
    );
    assert_eq!(
        done.accumulator,
        Some(Accumulator::Reward {
            activated: true,
            amount: Amount::Exact(ratio(3, 1))
        })
    );
    assert!(product.outgoing(three_ok.to).is_empty());
    // 12 is above the threshold 5; equal to it would stay exact.
    let twelve_ok = product
        .outgoing(busy)
        .iter()
        .find(|edge| integer_of(&edge.draw.as_ref().expect("a draw")[0]) == 12)
        .expect("attempt with d = 12");
    assert_eq!(
        product.states[twelve_ok.to.index()].accumulator,
        Some(Accumulator::Reward {
            activated: true,
            amount: Amount::AboveBound
        })
    );
}

const THRESHOLDS: &str = "probabilistic Four using ev on M over (s: M::Server) under Steady \
     on origin { quantile 0.95 of (accumulate duration from holds(s.phase = 1) \
     until holds(s.phase = 2) within 10) <= 4 }\n\
     probabilistic Three using ev on M over (s: M::Server) under Steady \
     on origin { quantile 0.95 of (accumulate duration from holds(s.phase = 1) \
     until holds(s.phase = 2) within 10) >= 3 }";

/// FR-196 "Behavior": an amount equal to the claim's threshold stays exact,
/// and one above it is the one value above it, whichever way the comparison
/// goes.
#[trace("TC-631", "FR-196-AC-2")]
#[test]
fn an_amount_equal_to_the_threshold_stays_exact() {
    let world = World::new(&prob_world::service(THRESHOLDS));
    let amounts = |claim: &str| {
        distinct(
            built(&world, claim)
                .states
                .iter()
                .filter_map(|key| match &key.accumulator {
                    Some(Accumulator::Reward {
                        activated: true,
                        amount,
                    }) => Some(amount.clone()),
                    _ => None,
                })
                .collect(),
        )
    };
    let exact = |value| Amount::Exact(ratio(value, 1));
    // 1 + 3 and 3 + 1 reach 4 exactly; 3 + 3 and any 12 are above it.
    assert_eq!(
        amounts("Four"),
        distinct(vec![
            exact(0),
            exact(1),
            exact(2),
            exact(3),
            exact(4),
            Amount::AboveBound
        ])
    );
    // At threshold 3, 4 is above and 3 is not.
    assert_eq!(
        amounts("Three"),
        distinct(vec![
            exact(0),
            exact(1),
            exact(2),
            exact(3),
            Amount::AboveBound
        ])
    );
}

const DELIVER: &str = "probabilistic Deliver using ev on M over (m: M::Msg) under every scheduler \
     on origin { probability >= 0.97 [ eventually[0,2] holds(m.delivered) ] }\n\
     probabilistic DeliverEven using ev on M over (m: M::Msg) under Even \
     on origin { probability >= 0.97 [ eventually[0,2] holds(m.delivered) ] }";

/// The edges of `edges` taking the operation named `operation`, with the
/// probability of each draw by its `lost` value.
fn sends(edges: &[ProbEdge], operation: &str) -> BTreeMap<bool, Rational> {
    edges
        .iter()
        .filter(|edge| {
            edge.action
                .as_ref()
                .is_some_and(|action| action.operation.contains(operation))
        })
        .map(|edge| {
            let draw = edge.draw.as_ref().expect("a draw");
            (boolean_of(&draw[0]), edge.probability.clone())
        })
        .collect()
}

/// FR-196-AC-3 (TC-631 step 3): §15.4's `Deliver` over every scheduler is an
/// MDP with two actions at each live state, whose draws carry 9/10 and 1/10
/// (`send_a`) and 4/5 and 1/5 (`send_b`); under `Even` it is a DTMC with
/// FR-187-AC-2's probabilities.
#[trace("TC-631", "FR-196-AC-3")]
#[test]
fn deliver_is_an_mdp_over_every_scheduler_and_a_dtmc_under_a_workload() {
    let world = World::new(&prob_world::link(DELIVER));
    let mdp = built(&world, "Deliver");
    assert_eq!(mdp.kind, ProductKind::Mdp);
    let first = start(&mdp);
    let edges = mdp.outgoing(first);
    assert_eq!(edges.len(), 4);
    assert_eq!(
        sends(edges, "send_a"),
        BTreeMap::from([(false, ratio(9, 10)), (true, ratio(1, 10))])
    );
    assert_eq!(
        sends(edges, "send_b"),
        BTreeMap::from([(false, ratio(4, 5)), (true, ratio(1, 5))])
    );
    // Each action's draws sum to 1: the scheduler chooses among actions.
    for operation in ["send_a", "send_b"] {
        let draws: Vec<ProbEdge> = edges
            .iter()
            .filter(|edge| {
                edge.action
                    .as_ref()
                    .is_some_and(|action| action.operation.contains(operation))
            })
            .cloned()
            .collect();
        assert_eq!(total(&draws), ratio(1, 1), "{operation}");
    }
    // Delivering decides the event; losing leaves a live state with the same
    // two actions.
    let lost = edges
        .iter()
        .find(|edge| boolean_of(&edge.draw.as_ref().expect("a draw")[0]))
        .expect("a lost draw");
    assert_eq!(mdp.outgoing(lost.to).len(), 4);
    let delivered = edges
        .iter()
        .find(|edge| !boolean_of(&edge.draw.as_ref().expect("a draw")[0]))
        .expect("a delivered draw");
    assert!(is_decided(&mdp, delivered.to));
    assert!(mdp
        .decided
        .contains(&(delivered.to, MonitorVerdict::Accept)));

    let dtmc = built(&world, "DeliverEven");
    assert_eq!(dtmc.kind, ProductKind::Dtmc);
    let edges = dtmc.outgoing(start(&dtmc));
    assert_eq!(edges.len(), 4);
    assert_eq!(total(edges), ratio(1, 1));
    assert_eq!(
        sends(edges, "send_a"),
        BTreeMap::from([(false, ratio(9, 20)), (true, ratio(1, 20))])
    );
    assert_eq!(
        sends(edges, "send_b"),
        BTreeMap::from([(false, ratio(2, 5)), (true, ratio(1, 10))])
    );
}

const CLOSING: &str = "probabilistic Late using ev on M over (m: M::Msg) under every scheduler \
     on origin { probability >= 0.5 [ eventually[0,5] holds(m.delivered) ] }\n\
     probabilistic Safe using ev on M over (m: M::Msg) under every scheduler \
     on origin { probability >= 0.5 \
     [ always[0,5] (holds(m.delivered) implies holds(m.attempts < 3)) ] }";

/// FR-196 "Behavior": a terminal model state closes a bounded event under
/// the profile's false extension (FR-126), into the decided state, with no
/// model step; an event the false extension violates is rejected there and
/// one it keeps is accepted.
#[trace("TC-631")]
#[test]
fn a_terminal_state_closes_a_bounded_event_by_false_extension() {
    let world = World::new(&prob_world::link(CLOSING));
    for (claim, verdict) in [
        ("Late", MonitorVerdict::Reject),
        ("Safe", MonitorVerdict::Accept),
    ] {
        let product = built(&world, claim);
        let closing: Vec<&ProbEdge> = product
            .edges
            .iter()
            .filter(|edge| matches!(edge.choice, ProductAction::TerminalStutter))
            .collect();
        assert!(!closing.is_empty(), "{claim}: a terminal state closes");
        for edge in &closing {
            assert_eq!(edge.probability, ratio(1, 1), "{claim}");
            assert!(edge.action.is_none() && edge.draw.is_none(), "{claim}");
            let [(name, cost)] = &edge.rewards[..] else {
                panic!("{claim}: the stutter step carries every reward");
            };
            assert_eq!((name.as_str(), integer_of(cost)), ("cost", 0));
            assert_eq!(
                product.states[edge.to.index()].monitor,
                Some(MonitorState::Formula(FormulaState::Decided(verdict))),
                "{claim}"
            );
            assert!(product.outgoing(edge.to).is_empty(), "{claim}");
        }
    }
}

const HEALTH: &str = "probabilistic Steadily using ev on M over (n: M::Node) under Steady \
     on origin { probability >= 0.99 [ always[0,2] holds(n.healthy) ] }\n\
     probabilistic Anyhow using ev on M over (n: M::Node) under every scheduler \
     on origin { probability >= 0.99 [ always[0,2] holds(n.healthy) ] }";

fn loose_health() -> World {
    World::new(&prob_world::health(
        "self.healthy = (pre(self.healthy) and not fault)",
        "true: 1, false: 9999999",
        true,
        HEALTH,
    ))
}

fn blocked_health() -> World {
    World::new(&prob_world::health(
        "not fault and self.healthy = pre(self.healthy) and self.spare = pre(self.spare)",
        "true: 1, false: 1",
        true,
        HEALTH,
    ))
}

/// FR-196-AC-4 (TC-631 step 4): FR-187-AC-3's `Health` variant with two
/// post-states per draw builds, over every scheduler, an intermediate state
/// per draw with two probability-1 actions, and the monitor does not advance
/// on it; under a workload the build stops with `NotMarkov`; the variant with
/// no post-state stops with `NotMarkov` under both. Two builds of one request
/// are equal.
#[trace("TC-631", "FR-196-AC-4")]
#[test]
fn a_draw_with_several_post_states_leads_to_an_intermediate_state() {
    let world = loose_health();
    let product = built(&world, "Anyhow");
    assert_eq!(product.kind, ProductKind::Mdp);
    let first = start(&product);
    let draws = product.outgoing(first);
    let [healthy, faulty] = draws else {
        panic!("two draws of `tick`: {draws:?}");
    };
    assert_eq!(healthy.probability, ratio(9_999_999, 10_000_000));
    assert_eq!(faulty.probability, ratio(1, 10_000_000));
    let source = &product.states[first.index()];
    for (draw, post_healthy) in [(healthy, true), (faulty, false)] {
        let intermediate = &product.states[draw.to.index()];
        let pending = intermediate.pending.as_ref().expect("a pending draw");
        assert!(pending.identity.operation.contains("tick"));
        assert_eq!(boolean_of(&pending.random[0]), !post_healthy);
        // The monitor and the accumulator are the source's: nothing advanced.
        assert_eq!(intermediate.monitor, source.monitor);
        assert_eq!(intermediate.accumulator, source.accumulator);
        assert_eq!(intermediate.model, source.model);
        let resolutions = product.outgoing(draw.to);
        assert_eq!(resolutions.len(), 2, "one per `spare`");
        assert_eq!(total(resolutions), ratio(2, 1));
        for edge in resolutions {
            assert_eq!(edge.probability, ratio(1, 1));
            assert!(edge.action.is_none() && edge.draw.is_none());
            let ProductAction::PostState(target) = &edge.choice else {
                panic!("a resolution names its post-state: {:?}", edge.choice);
            };
            assert_eq!(**target, product.states[edge.to.index()]);
            let reached = &product.states[edge.to.index()];
            assert!(reached.pending.is_none());
            // It advanced once: a fault is a bad prefix, health is position 1.
            let verdict = if post_healthy {
                assert_ne!(reached.monitor, source.monitor);
                None
            } else {
                Some(MonitorVerdict::Reject)
            };
            assert_eq!(
                reached.monitor.is_some_and(MonitorState::is_decided),
                verdict.is_some()
            );
            if let Some(verdict) = verdict {
                assert!(product.decided.contains(&(edge.to, verdict)));
            }
        }
    }
    let again = built(&world, "Anyhow");
    assert_eq!(product.states, again.states);
    assert_eq!(format!("{product:?}"), format!("{again:?}"));

    let ProductRefusal::NotMarkov(loose) = refused(&world, "Steadily") else {
        panic!("a workload needs one post-state");
    };
    assert_eq!(loose.post_states, 2);
    assert!(loose.transition.operation.contains("tick"));
    assert_eq!(
        loose.random.iter().map(boolean_of).collect::<Vec<_>>(),
        [false]
    );

    let blocked = blocked_health();
    for claim in ["Steadily", "Anyhow"] {
        let ProductRefusal::NotMarkov(none) = refused(&blocked, claim) else {
            panic!("{claim}: a draw with no post-state is not Markov");
        };
        assert_eq!(none.post_states, 0, "{claim}");
        assert_eq!(
            none.random.iter().map(boolean_of).collect::<Vec<_>>(),
            [true],
            "{claim}"
        );
    }
}

const COIN: &str = "probabilistic Dropped using ev on M over (c: M::Coin) under Even on origin \
     { probability >= 0.5 [ always[0,3] holds(c.v = 0) ] }";

/// FR-196-AC-5 (TC-641 step 1): when the claim reads a value that is
/// undefined at a product state of positive probability, the build stops at
/// the first such state in breadth-first order and returns the path to it.
/// The spec's `Coin` divides by `1 - c.v`; S3 types neither `/` in a model
/// unit nor a population name in a claim's predicate, so this `Coin` is
/// dropped when `b = 0` and the claim's `c.v` then reads `absent-key`
/// undefined. The path is `drop` with `b = 0`, probability 1/2, at
/// position 1.
#[trace("TC-641", "FR-196-AC-5")]
#[test]
fn an_undefined_state_stops_the_build_with_its_path() {
    let world = World::new(&prob_world::coin(COIN));
    let ProbBuild::Undefined(path) = build_product(
        world.request("Dropped", ModelCheckLimits::default()),
        &Cancel::new(),
    )
    .expect("the build ends with a refutation") else {
        panic!("an undefined path");
    };
    let [step] = &path.steps[..] else {
        panic!("one step: {:?}", path.steps);
    };
    assert!(step
        .action
        .as_ref()
        .is_some_and(|action| action.operation.contains("drop")));
    assert_eq!(
        step.draw
            .as_ref()
            .map(|draw| draw.iter().map(integer_of).collect::<Vec<_>>()),
        Some(vec![0])
    );
    assert_eq!(step.probability, ratio(1, 2));
    assert_eq!(path.probability, ratio(1, 2));
    assert_eq!(path.position, 1);
    assert!(matches!(path.read, UndefinedRead::Atom(_)));
    assert!(matches!(
        path.cause,
        UndefinedCause::Family(ref record) if record.reason == UndefinedReason::AbsentKey
    ));
}

const ARMED: &str = "probabilistic Armed using ev on M over (c: M::Coin) under Even on origin \
     { probability >= 0.5 [ always[0,3] holds(c.armed = c.armed) ] }";

/// FR-196-AC-5 (TC-641 step 1): the path to an undefined state lists every
/// model step from the initial state, in order, with the product of their
/// probabilities and the position, the step count.
#[trace("TC-641", "FR-196-AC-5")]
#[test]
fn the_undefined_path_lists_every_step_in_order() {
    let world = World::new(&prob_world::armed_coin(ARMED));
    let ProbBuild::Undefined(path) = build_product(
        world.request("Armed", ModelCheckLimits::default()),
        &Cancel::new(),
    )
    .expect("the build ends with a refutation") else {
        panic!("an undefined path");
    };
    let operations: Vec<bool> = path
        .steps
        .iter()
        .map(|step| {
            step.action
                .as_ref()
                .is_some_and(|action| action.operation.contains("Coin/arm"))
        })
        .collect();
    assert_eq!(
        operations,
        [true, false],
        "`arm`, then `drop`: {:?}",
        path.steps
    );
    assert_eq!(path.steps[0].probability, ratio(1, 1));
    assert_eq!(path.steps[1].probability, ratio(1, 2));
    assert_eq!(
        path.steps[1]
            .draw
            .as_ref()
            .map(|draw| draw.iter().map(integer_of).collect::<Vec<_>>()),
        Some(vec![0])
    );
    assert_eq!(path.probability, ratio(1, 2));
    assert_eq!(path.position, 2);
}

const GHOST: &str = "probabilistic Ghost using ev on M over (c: M::Coin) under Even on origin \
     { probability >= 0.5 [ always[0,3] holds(c.v = 0) ] }\n\
     probabilistic GhostFrom using inf on M over (c: M::Coin) under Even on origin \
     { probability >= 0.5 [ holds(c.v = 0) until holds(c.v = 1) ] }\n\
     probabilistic GhostTarget using inf on M over (c: M::Coin) under Even on origin \
     { probability >= 0.5 [ eventually holds(c.v = 0) ] }";

/// FR-196 "Behavior": the claim's `over` instances are the subject's
/// universe, in order, and a state at which a predicate is undefined for an
/// instance ends the build. The second instance has no object in the initial
/// snapshot, so its predicate is undefined at position 0, with no step to
/// it.
#[trace("TC-641")]
#[test]
fn an_instance_undefined_at_an_initial_state_ends_at_position_zero() {
    let world = World::with_universe(&prob_world::coin(GHOST), &["c", "ghost"]);
    let ProbBuild::Undefined(path) = build_product(
        world.request("Ghost", ModelCheckLimits::default()),
        &Cancel::new(),
    )
    .expect("the build ends with a refutation") else {
        panic!("an undefined path");
    };
    assert!(path.steps.is_empty());
    assert_eq!(path.position, 0);
    assert_eq!(path.probability, ratio(1, 1));
    assert!(matches!(path.read, UndefinedRead::Atom(_)));
    // An unbounded form names the role of the predicate that has no value.
    for (claim, role) in [
        ("GhostFrom", PredicateRole::From),
        ("GhostTarget", PredicateRole::Target),
    ] {
        let ProbBuild::Undefined(path) = build_product(
            world.request(claim, ModelCheckLimits::default()),
            &Cancel::new(),
        )
        .expect("the build ends with a refutation") else {
            panic!("{claim}: an undefined path");
        };
        assert_eq!(path.read, UndefinedRead::Predicate(role), "{claim}");
    }
}

const STUTTER: &str = "probabilistic Steps using ev on M over (m: M::Msg) under Even on origin \
     { probability >= 0.5 [ steps from holds(m.attempts = 0) until holds(m.delivered and m.attempts = 0) \
     within 4 <= 10 ] }\n\
     probabilistic Cost using ev on M over (m: M::Msg) under every scheduler on origin \
     { probability >= 0.5 [ accumulate cost from holds(m.attempts = 0) \
     until holds(m.delivered and m.attempts = 0) within 4 >= 1 ] }";

/// FR-196 "Behavior": a bounded measure continues on the terminal stutter
/// steps through its window, each a step of zero reward, and the window's end
/// decides it: here `B` never holds, so the measure ends censored.
#[trace("TC-631")]
#[test]
fn a_measure_continues_on_the_stutter_steps_to_the_end_of_its_window() {
    let world = World::new(&prob_world::link(STUTTER));
    for (claim, kind) in [("Steps", ProductKind::Dtmc), ("Cost", ProductKind::Mdp)] {
        let product = built(&world, claim);
        assert_eq!(product.kind, kind, "{claim}");
        let stutters: Vec<&ProbEdge> = product
            .edges
            .iter()
            .filter(|edge| matches!(edge.choice, ProductAction::TerminalStutter))
            .collect();
        assert!(!stutters.is_empty(), "{claim}");
        for edge in &stutters {
            let remaining = |state: ProbStateId| match product.states[state.index()].monitor {
                Some(MonitorState::Measure { remaining, .. }) => remaining,
                other => panic!("{claim}: a measure: {other:?}"),
            };
            assert_eq!(remaining(edge.to) + 1, remaining(edge.from), "{claim}");
            assert_eq!(edge.probability, ratio(1, 1), "{claim}");
            // A stutter step adds a step to `steps` and nothing to `cost`.
            let (before, after) = (
                &product.states[edge.from.index()].accumulator,
                &product.states[edge.to.index()].accumulator,
            );
            match (before, after, claim) {
                (
                    Some(Accumulator::Steps {
                        amount: Amount::Exact(before),
                        ..
                    }),
                    Some(Accumulator::Steps { amount, .. }),
                    "Steps",
                ) => {
                    if remaining(edge.to) == 0 {
                        assert_eq!(*amount, Amount::Censored);
                    } else {
                        assert_eq!(*amount, Amount::Exact(before.add(&ratio(1, 1))));
                    }
                }
                (
                    Some(Accumulator::Reward { amount: before, .. }),
                    Some(Accumulator::Reward { amount, .. }),
                    "Cost",
                ) => {
                    if remaining(edge.to) == 0 {
                        assert_eq!(*amount, Amount::Censored);
                    } else {
                        assert_eq!(amount, before);
                    }
                }
                other => panic!("{claim}: {other:?}"),
            }
        }
        let ended: Vec<_> = product
            .states
            .iter()
            .filter(|key| key.monitor.is_some_and(MonitorState::is_decided))
            .collect();
        assert!(!ended.is_empty(), "{claim}");
        for key in ended {
            assert!(
                matches!(
                    key.monitor,
                    Some(MonitorState::Measure {
                        remaining: 0,
                        phase: MeasurePhase::Censored
                    })
                ),
                "{claim}: {:?}",
                key.monitor
            );
        }
    }
}

const SPREAD: &str = "probabilistic Merge using inf on M over (n: M::Node) under every scheduler \
     on origin { probability >= 0.5 [ eventually holds(n.healthy) ] }\n\
     probabilistic Plain using inf on M over (n: M::Node) under Steady \
     on origin { probability >= 0.5 [ eventually holds(n.healthy) ] }";

const REWARDS: &str = "probabilistic Spread using ev on M over (s: M::Server) \
     under every scheduler on origin { long_run_fraction holds(s.phase = 0) >= 0.5 }";

/// FR-196 "Behavior": over every scheduler, draws of one action that reach
/// the same post-state with the same rewards are one edge with their summed
/// probability and no draw; draws whose rewards differ stay apart, and under
/// a workload every draw is its own edge.
#[trace("TC-631")]
#[test]
fn draws_to_one_post_state_are_one_edge_over_every_scheduler() {
    let model = prob_world::health(
        "self.healthy = pre(self.healthy)",
        "true: 1, false: 3",
        false,
        SPREAD,
    );
    let world = World::new(&model);
    let merged = built(&world, "Merge");
    let first = start(&merged);
    let [edge] = merged.outgoing(first) else {
        panic!("one edge: {:?}", merged.outgoing(first));
    };
    assert_eq!(
        (edge.to, edge.probability.clone(), edge.draw.is_none()),
        (first, ratio(1, 1), true)
    );
    assert!(edge.action.is_some());
    let plain = built(&world, "Plain");
    let edges = plain.outgoing(start(&plain));
    assert_eq!(edges.len(), 2);
    assert_eq!(
        distinct(edges.iter().map(|edge| edge.probability.clone()).collect()),
        [ratio(1, 4), ratio(3, 4)]
    );
    assert!(edges.iter().all(|edge| edge.draw.is_some()));

    let service = World::new(&prob_world::service(REWARDS));
    let product = built(&service, "Spread");
    let by_edges: Vec<(usize, bool)> = product
        .ids()
        .map(|state| product.outgoing(state))
        .filter(|edges| edges.len() > 1)
        .map(|edges| (edges.len(), edges.iter().all(|edge| edge.draw.is_none())))
        .collect();
    // From attempt 0 the outcome decides the next state: six draws, six
    // edges. From attempt 1 both outcomes reach the same state, so the two
    // draws of one `d` are one edge.
    assert!(by_edges.contains(&(6, false)), "{by_edges:?}");
    assert!(by_edges.contains(&(3, true)), "{by_edges:?}");
    let merged_edge = product
        .edges
        .iter()
        .filter(|edge| edge.draw.is_none())
        .find(|edge| integer_of(&edge.rewards[0].1) == 3)
        .expect("a merged edge for d = 3");
    assert_eq!(merged_edge.probability, ratio(7, 100));
}

const REACH: &str =
    "probabilistic Terminates using inf on M over (p: M::Proc) under every scheduler \
     on origin { probability >= 0.99 [ eventually holds(p.done) ] }\n\
     probabilistic FairTerminates using inf on M over (p: M::Proc) \
     under every scheduler fair strong Proc::flip \
     on origin { probability >= 0.99 [ eventually holds(p.done) ] }\n\
     probabilistic Unless using inf on M over (p: M::Proc) under Even on origin \
     { probability >= 0.9 [ holds(not p.done) until holds(p.done) ] }";

/// FR-196 "Behavior", exact-only forms: an unbounded reachability claim uses
/// the MDP or DTMC itself as the product, with the claim's predicates as
/// state labels and no monitor, accumulator or pending draw in any key. The
/// fairness set belongs to the claim, not the product.
#[trace("TC-631")]
#[test]
fn an_unbounded_form_labels_the_model_states() {
    let world = World::new(&prob_world::process(REACH));
    let product = built(&world, "Terminates");
    assert_eq!(product.kind, ProductKind::Mdp);
    for key in &product.states {
        assert!(key.monitor.is_none() && key.accumulator.is_none() && key.pending.is_none());
    }
    assert_eq!(product.states.len(), 2);
    assert!(product.decided.is_empty());
    assert_eq!(product.labels, [[false], [true]]);
    let live = start(&product);
    let edges = product.outgoing(live);
    let flip: Vec<&ProbEdge> = edges
        .iter()
        .filter(|edge| {
            edge.action
                .as_ref()
                .is_some_and(|a| a.operation.contains("flip"))
        })
        .collect();
    let wait: Vec<&ProbEdge> = edges
        .iter()
        .filter(|edge| {
            edge.action
                .as_ref()
                .is_some_and(|a| a.operation.contains("wait"))
        })
        .collect();
    assert_eq!(flip.len(), 2);
    assert_eq!(wait.len(), 1);
    // `wait` repeats the state; `flip` ends it with probability 1/2.
    assert_eq!(
        (wait[0].to, wait[0].probability.clone()),
        (live, ratio(1, 1))
    );
    for edge in &flip {
        assert_eq!(edge.probability, ratio(1, 2));
        let heads = boolean_of(&edge.draw.as_ref().expect("a draw")[0]);
        assert_eq!(edge.to == live, !heads);
    }
    let done = flip.iter().find(|edge| edge.to != live).expect("done").to;
    let [stutter] = product.outgoing(done) else {
        panic!("the terminal state only stutters");
    };
    assert!(matches!(stutter.choice, ProductAction::TerminalStutter));
    assert_eq!(
        (stutter.to, stutter.probability.clone()),
        (done, ratio(1, 1))
    );

    let fair = built(&world, "FairTerminates");
    assert_eq!(fair.states, product.states);
    assert_eq!(fair.labels, product.labels);
    assert_eq!(fair.edges.len(), product.edges.len());

    let until = built(&world, "Unless");
    assert_eq!(until.kind, ProductKind::Dtmc);
    // `A` then `B`: not yet done, then done.
    assert_eq!(until.labels, [[true, false], [false, true]]);
}

const COST: &str = "probabilistic Cost using ev on M over (m: M::Msg) under Even on origin \
     { expected accumulate cost until holds(m.delivered or m.attempts = 2) <= rational(6, 5) }";

/// FR-196 "Behavior", expected reward: the DTMC itself is the product, the
/// target is the one state label and each send carries its `cost` on its
/// edge; the stutter step of a terminal state carries cost 0.
#[trace("TC-631")]
#[test]
fn an_expected_reward_product_keeps_rewards_on_edges() {
    let world = World::new(&prob_world::link(COST));
    let product = built(&world, "Cost");
    assert_eq!(product.kind, ProductKind::Dtmc);
    let cost = |edge: &ProbEdge| {
        let [(name, value)] = &edge.rewards[..] else {
            panic!("one reward: {:?}", edge.rewards);
        };
        assert_eq!(name.as_str(), "cost");
        integer_of(value)
    };
    let first = start(&product);
    assert_eq!(product.labels[first.index()], [false]);
    let sends = product.outgoing(first);
    assert_eq!(sends.len(), 4);
    assert!(sends.iter().all(|edge| cost(edge) == 1));
    let stutters: Vec<&ProbEdge> = product
        .edges
        .iter()
        .filter(|edge| matches!(edge.choice, ProductAction::TerminalStutter))
        .collect();
    assert!(!stutters.is_empty());
    assert!(stutters.iter().all(|edge| cost(edge) == 0));
    for edge in stutters {
        assert_eq!(
            product.labels[edge.from.index()],
            [true],
            "terminal states hold B"
        );
        assert_eq!(edge.to, edge.from);
    }
}

const LONG_RUN: &str =
    "probabilistic LongRun using ev on M over (s: M::Server) under Steady on origin \
     { long_run_fraction holds(s.phase = 0) >= 0.5 }";

/// FR-196 "Behavior", long-run fraction: the DTMC itself is the product with
/// the predicate as the state label and no control in any key.
#[trace("TC-631")]
#[test]
fn a_long_run_fraction_product_is_the_chain_with_labels() {
    let world = World::new(&prob_world::service(LONG_RUN));
    let product = built(&world, "LongRun");
    assert_eq!(product.kind, ProductKind::Dtmc);
    assert_eq!(product.labels.len(), product.states.len());
    for state in product.ids() {
        let edges = product.outgoing(state);
        assert_eq!(total(edges), ratio(1, 1), "state {state:?}");
        let key = &product.states[state.index()];
        assert!(key.monitor.is_none() && key.accumulator.is_none());
    }
    // The chain cycles through Idle: some state is labelled and some is not.
    let idle = product.labels.iter().filter(|label| label[0]).count();
    assert!(idle >= 1 && idle < product.labels.len());
}

const MEASURES: &str = "probabilistic Steps using ev on M over (s: M::Server) under Steady on origin \
     { probability >= 0.5 [ steps from holds(s.phase = 1) until holds(s.phase = 2) within 10 <= 1 ] }\n\
     probabilistic Count using ev on M over (s: M::Server) under Steady on origin \
     { probability >= 0.5 [ fraction holds(s.phase = 1) over [0, 2] >= rational(1, 2) ] }\n\
     probabilistic Weighted using ev on M over (s: M::Server) under Steady on origin \
     { probability >= 0.5 [ fraction holds(s.phase = 1 and s.attempts = 1) over [0, 2] \
     weighted by duration >= rational(1, 2) ] }\n\
     probabilistic Short using ev on M over (s: M::Server) under Steady on origin \
     { probability >= 0.5 [ steps from holds(s.phase = 1) until holds(s.phase = 2) within 1 <= 5 ] }";

/// FR-196 "Behavior", bounded measures: a `steps` measure counts one per
/// step and saturates above its threshold; a fraction counts the included
/// positions, each decided at the step that leaves it, and a weighted
/// fraction keeps both sums exactly; a measure whose window ends first is
/// censored.
#[trace("TC-631")]
#[test]
fn steps_and_fractions_accumulate_exactly_through_their_windows() {
    let world = World::new(&prob_world::service(MEASURES));

    let steps = built(&world, "Steps");
    let held = distinct(
        steps
            .states
            .iter()
            .map(|key| match &key.accumulator {
                Some(Accumulator::Steps { activated, amount }) => (*activated, amount.clone()),
                other => panic!("a steps accumulator: {other:?}"),
            })
            .collect(),
    );
    let exact = |value| (true, Amount::Exact(ratio(value, 1)));
    assert_eq!(
        held,
        distinct(vec![
            (false, Amount::Exact(ratio(0, 1))),
            exact(0),
            exact(1),
            (true, Amount::AboveBound),
        ])
    );

    let count = built(&world, "Count");
    let complete: BTreeSet<(u64, u64)> = count
        .states
        .iter()
        .filter(|key| key.monitor == Some(MonitorState::Fraction { remaining: 0 }))
        .map(|key| match key.accumulator {
            Some(Accumulator::CountFraction {
                true_count,
                position_count,
            }) => (true_count, position_count),
            ref other => panic!("a count accumulator: {other:?}"),
        })
        .collect();
    assert_eq!(complete, BTreeSet::from([(1, 3), (2, 3)]));
    assert!(count
        .ids()
        .filter(|state| is_decided(&count, *state))
        .all(|state| count.outgoing(state).is_empty()));

    let weighted = built(&world, "Weighted");
    let sums = distinct(
        weighted
            .states
            .iter()
            .filter(|key| key.monitor == Some(MonitorState::Fraction { remaining: 0 }))
            .map(|key| match &key.accumulator {
                Some(Accumulator::WeightedFraction {
                    true_sum,
                    total_sum,
                }) => (true_sum.clone(), total_sum.clone()),
                other => panic!("a weighted accumulator: {other:?}"),
            })
            .collect(),
    );
    assert!(sums.contains(&(ratio(0, 1), ratio(1, 1))), "{sums:?}");
    assert!(sums.contains(&(ratio(3, 1), ratio(4, 1))), "{sums:?}");

    // `within 1` ends the window after one step: a measure activated at
    // position 1 and not ended is censored.
    let short = built(&world, "Short");
    let censored: Vec<_> = short
        .states
        .iter()
        .filter(|key| {
            matches!(
                key.monitor,
                Some(MonitorState::Measure {
                    phase: MeasurePhase::Censored,
                    ..
                })
            )
        })
        .collect();
    assert!(!censored.is_empty());
    for key in censored {
        assert_eq!(
            key.accumulator,
            Some(Accumulator::Steps {
                activated: true,
                amount: Amount::Censored
            })
        );
    }
}

/// FR-196 "Behavior": reaching `max_states`, `max_transitions` or
/// `max_automaton_states`, or a cancellation, stops the build and names it.
#[trace("TC-631")]
#[test]
fn a_budget_or_a_cancellation_stops_the_build() {
    let world = World::new(&prob_world::no_fault(NO_FAULT));
    let stopped = |limits: ModelCheckLimits| match build_product(
        world.request("NoFault", limits),
        &Cancel::new(),
    ) {
        Err(ProductRefusal::Stopped(stop)) => stop,
        other => panic!("expected a stop: {other:?}"),
    };
    let defaults = ModelCheckLimits::default();
    assert_eq!(
        stopped(ModelCheckLimits {
            limits: Limits {
                max_states: 5,
                ..defaults.limits
            },
            ..defaults
        }),
        ProductStop::Limit {
            limit: ModelCheckLimit::MaxStates,
            value: 5
        }
    );
    assert_eq!(
        stopped(ModelCheckLimits {
            limits: Limits {
                max_transitions: 3,
                ..defaults.limits
            },
            ..defaults
        }),
        ProductStop::Limit {
            limit: ModelCheckLimit::MaxTransitions,
            value: 3
        }
    );
    assert_eq!(
        stopped(ModelCheckLimits {
            max_automaton_states: 2,
            ..defaults
        }),
        ProductStop::Limit {
            limit: ModelCheckLimit::MaxAutomatonStates,
            value: 2
        }
    );
    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    assert!(matches!(
        build_product(world.request("NoFault", defaults), &cancel),
        Err(ProductRefusal::Stopped(ProductStop::Cancelled(
            CancelCause::Requested
        )))
    ));
}

const NEGOTIATED: &str = "probabilistic Seeded using ev on M over (p: M::Proc) under Even \
     on origin with alpha 0.01, beta 0.01, indifference 0.002 \
     { probability >= 0.9 [ always[0,3] holds(not p.done) ] }\n\
     probabilistic Reached using inf on M over (p: M::Proc) under Even \
     on origin with alpha 0.01, beta 0.01, indifference 0.002 \
     { probability >= 0.9 [ eventually holds(p.done) ] }\n\
     probabilistic Anywhere using ev on M over (p: M::Proc) under every scheduler \
     on origin with alpha 0.01, beta 0.01, indifference 0.002 \
     { probability >= 0.9 [ always[0,3] holds(not p.done) ] }\n\
     probabilistic Unseeded using ev on M over (p: M::Proc) under Even on origin \
     { probability >= 0.9 [ always[0,3] holds(not p.done) ] }";

/// FR-195-AC-4 (TC-630 step 4): a claim only exact evidence decides is
/// refused at negotiation, before any sample, with the cause that names why:
/// `EveryScheduler` for `FairTerminates`, `ExactOnlyForm` for an
/// expected-reward claim under a workload, `MissingConfidence` for a bounded
/// event with no confidence parameters; a statistical claim passes.
#[trace("TC-630", "FR-195-AC-4")]
#[test]
fn statistical_evidence_is_refused_for_a_claim_only_exact_evidence_decides() {
    use qsl_analyze::statistical::require_statistical;
    use qsl_replay::statistical::StatisticalRefusal;

    let process = World::new(&prob_world::process(&format!("{REACH}\n{NEGOTIATED}")));
    let verdict = |claim: &str| require_statistical(process.claim(claim));
    assert_eq!(
        verdict("FairTerminates"),
        Err(StatisticalRefusal::EveryScheduler)
    );
    assert_eq!(
        verdict("Terminates"),
        Err(StatisticalRefusal::EveryScheduler)
    );
    assert_eq!(verdict("Anywhere"), Err(StatisticalRefusal::EveryScheduler));
    // A form only EN-5 decides is refused whatever its scheduler and
    // confidence say.
    assert_eq!(verdict("Reached"), Err(StatisticalRefusal::ExactOnlyForm));
    assert_eq!(verdict("Unless"), Err(StatisticalRefusal::ExactOnlyForm));
    assert_eq!(
        verdict("Unseeded"),
        Err(StatisticalRefusal::MissingConfidence)
    );
    assert_eq!(verdict("Seeded"), Ok(()));

    let link = World::new(&prob_world::link(COST));
    assert_eq!(
        require_statistical(link.claim("Cost")),
        Err(StatisticalRefusal::ExactOnlyForm)
    );
}
