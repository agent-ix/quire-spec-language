// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-196 (TC-631, TC-641): EN-5's probabilistic product, over ADR-024 §7's
//! and ADR-028 §15's models (see [`crate::prob_world`]).

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use ix_trace_rs::trace;
use qsl_analyze::exact_probabilistic::{
    backward_induction, build_product, decide_bounded_event, decide_fair_long_run_mdp,
    decide_fair_reachability, maximal_fair_end_components, Accumulator, Amount, ExactProbLimits,
    FhObjective, FhResult, FormulaState, MeasurePhase, MonitorState, MonitorVerdict, PredicateRole,
    ProbBuild, ProbEdge, ProbKey, ProbProduct, ProbStateId, ProductAction, ProductKind,
    ProductRefusal, ProductRequest, ProductStop, UnboundedObjective, UndefinedRead,
};
use qsl_eval::model_subject::ModelSubject;
use qsl_eval::simulation::Limits;
use qsl_eval::temporal::UndefinedCause;
use qsl_replay::{ModelCheckLimit, ModelCheckLimits};
use quire_exact::{Cancel, CancelCause, Integer, Rational, Value};
use quire_semantic_value::catalog::UndefinedReason;

use crate::prob_world::{self, World};

fn ratio(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("nonzero")
}

fn dyadic(value: &qsl_analyze::exact_probabilistic::Dyadic) -> Rational {
    Rational::from_integer(Integer::from_big(value.mantissa.clone()))
        .divided_by_power_of_two(u64::from(value.exponent))
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

/// FR-254-AC-3 (TC-709): an exact request for the strict-guard timed RPC is
/// refused by the exact timed-profile route rather than admitted as untimed.
#[trace("TC-709", "FR-254-AC-3")]
#[test]
fn exact_request_over_strict_guard_timed_rpc_is_refused() {
    let document =
        crate::timed_world::document(&["x"], &[("send", &["phase"]), ("reply", &["phase"])]);
    let body = format!(
        "{}\
         pre Waiting using v on M::Call::send {{ self.phase = 0 }}\n\
         post Sent using v on M::Call::send {{ self.phase = 1 }}\n\
         pre StrictReply using v on M::Call::reply {{ self.phase = 1 and self.x > 3 ms }}\n\
         post Replied using v on M::Call::reply {{ self.phase = 2 }}\n\
         delay M::Call::send ~ discrete {{ 0 ms: 1 }};\n\
         delay M::Call::reply ~ discrete {{ 1 ms: 1 }};\n\
         workload Even on M {{ weight Call::send = 1; weight Call::reply = 1; }}\n\
         probabilistic StrictExact using ev on M over (c: M::Call) under Even on origin\n\
             with alpha 0.4, beta 0.4, indifference 0.005\n\
             {{ probability <= 0.01 [ eventually[0 ms, 4 ms] holds(c.phase = 2) ] }}\n",
        crate::timed_world::DENSE,
    );
    let world = crate::timed_world::world(
        &document,
        "profile ev = \"quire.temporal.timed/v1\";\n",
        &body,
    );
    let claim = world
        .package
        .graph()
        .probabilistic_model()
        .claim("StrictExact")
        .expect("the strict timed claim checks");
    let refusal = build_product(
        ProductRequest {
            subject: ModelSubject {
                simulation: world.request(),
                proof_bounds: Vec::new(),
            },
            claim,
            limits: ExactProbLimits::default(),
        },
        &Cancel::new(),
    )
    .expect_err("EN-5 refuses a strict-guard timed exact request");
    assert!(matches!(
        refusal,
        ProductRefusal::Translation(
            qsl_eval::automaton::AutomatonError::ProfileNotTranslated { .. }
        )
    ));
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

fn measure_event_terminals(product: &ProbProduct, threshold: &Rational) -> Vec<Option<Rational>> {
    product
        .states
        .iter()
        .map(|key| match (key.monitor, key.accumulator.as_ref()) {
            (
                Some(MonitorState::Measure {
                    phase:
                        MeasurePhase::Complete | MeasurePhase::Censored | MeasurePhase::Unactivated,
                    ..
                }),
                Some(Accumulator::Reward { activated, amount })
                | Some(Accumulator::Steps { activated, amount }),
            ) => {
                let holds =
                    *activated && matches!(amount, Amount::Exact(value) if value <= threshold);
                Some(if holds { ratio(1, 1) } else { ratio(0, 1) })
            }
            _ => None,
        })
        .collect()
}

fn quantile_margin_terminals(
    product: &ProbProduct,
    threshold: &Rational,
    level: &Rational,
) -> Vec<Option<Rational>> {
    product
        .states
        .iter()
        .map(|key| match (key.monitor, key.accumulator.as_ref()) {
            (
                Some(MonitorState::Measure {
                    phase:
                        MeasurePhase::Complete | MeasurePhase::Censored | MeasurePhase::Unactivated,
                    ..
                }),
                Some(Accumulator::Reward { activated, amount })
                | Some(Accumulator::Steps { activated, amount }),
            ) => {
                let contribution = if *activated {
                    let event = matches!(amount, Amount::Exact(value) if value <= threshold);
                    if event { ratio(1, 1) } else { ratio(0, 1) }.sub(level)
                } else {
                    ratio(0, 1)
                };
                Some(contribution)
            }
            _ => None,
        })
        .collect()
}

fn selected_operation(product: &ProbProduct, state: ProbStateId, choice: usize) -> String {
    let mut identities = Vec::new();
    for edge in product.outgoing(state) {
        let identity = edge.action.as_ref().expect("a model action");
        if !identities.iter().any(|seen| seen == identity) {
            identities.push(identity.clone());
        }
    }
    identities
        .get(choice)
        .expect("the policy selects an enabled action")
        .operation
        .clone()
}

const NO_FAULT: &str = "probabilistic NoFault using ev on M over (n: M::Node) under Steady \
     on origin { probability >= 0.999 [ always[0,1000] holds(n.healthy) ] }";

/// Checks FR-197-AC-3 (TC-632 step 3): exact backward induction keeps the
/// §15.1 rational while it fits, then outward dyadic rounding encloses it.
#[trace("TC-632", "FR-197-AC-3")]
#[test]
fn no_fault_uses_exact_then_dyadic_backward_induction() {
    let world = World::new(&prob_world::no_fault(NO_FAULT));
    let product = built(&world, "NoFault");
    let start = start(&product);
    let exact = decide_bounded_event(
        &product,
        &qsl_semantics::check::Bound::AtLeast(ratio(999, 1000)),
        &ExactProbLimits {
            max_rational_bits: 23_254,
            ..ExactProbLimits::default()
        },
    )
    .expect("acyclic bounded product");
    let FhResult::Exact { values, .. } = exact else {
        panic!("the published rational budget keeps NoFault exact");
    };
    let expected = ratio(9_999_999, 10_000_000)
        .pow(&Integer::from(1000_i64))
        .expect("positive integer exponent");
    assert_eq!(values[start.index()], expected);

    let bounded = decide_bounded_event(
        &product,
        &qsl_semantics::check::Bound::AtLeast(ratio(999, 1000)),
        &ExactProbLimits {
            max_rational_bits: 1024,
            ..ExactProbLimits::default()
        },
    )
    .expect("dyadic fallback");
    let FhResult::Bounds {
        lower,
        upper,
        precision_bits,
        ..
    } = bounded
    else {
        panic!("the interval clears the threshold at 64 bits");
    };
    let start_lower = dyadic(&lower[start.index()]);
    let start_upper = dyadic(&upper[start.index()]);
    assert!(start_lower <= expected && expected <= start_upper);
    assert!(start_lower > ratio(999, 1000));
    assert_eq!(precision_bits, 64);
    let maximum_width = ratio(2 * 1001, 1).divided_by_power_of_two(64);
    assert!(start_upper.sub(&start_lower) <= maximum_width);
}

/// Checks FR-196-AC-1 (TC-631 step 1): §15.1's `NoFault` product is a DTMC whose
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
     within 10) <= 5 }\n\
     probabilistic P95Two using ev on M over (s: M::Server) under Steady on origin \
     { quantile 0.95 of (accumulate duration from holds(s.phase = 1) until holds(s.phase = 2) \
     within 10) <= 2 }\n\
     probabilistic P95Every using ev on M over (s: M::Server) under every scheduler on origin \
     { quantile 0.95 of (accumulate duration from holds(s.phase = 1) until holds(s.phase = 2) \
     within 10) <= 5 }\n\
     probabilistic P95EveryTwo using ev on M over (s: M::Server) under every scheduler on origin \
     { quantile 0.95 of (accumulate duration from holds(s.phase = 1) until holds(s.phase = 2) \
     within 10) <= 2 }";

/// Checks FR-196-AC-2 (TC-631 step 2): §15.2's product for `P95` at 5 carries the
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

/// Checks FR-197-AC-2 (TC-632 step 2): exact §15.2 probabilities and the
/// XF-2 transformation agree under `Steady` and every scheduler.
#[trace("TC-632", "FR-197-AC-2")]
#[test]
fn p95_probabilities_match_the_exact_scheduler_transform() {
    let world = World::new(&prob_world::service(P95));
    let limits = ExactProbLimits::default();
    let quantile = ratio(19, 20);
    for (workload_name, every_name, threshold, expected) in [
        ("P95", "P95Every", ratio(5, 1), ratio(24_233, 25_000)),
        ("P95Two", "P95EveryTwo", ratio(2, 1), ratio(4_491, 5_000)),
    ] {
        let workload_product = built(&world, workload_name);
        let every_product = built(&world, every_name);
        assert_eq!(workload_product.kind, ProductKind::Dtmc);
        assert_eq!(every_product.kind, ProductKind::Mdp);

        let workload_result = backward_induction(
            &workload_product,
            &measure_event_terminals(&workload_product, &threshold),
            FhObjective::Min,
            &quantile,
            limits.max_rational_bits,
            limits.precision_bits,
            limits.max_precision_bits,
        )
        .expect("the bounded workload measure is acyclic");
        let FhResult::Exact {
            values: workload_values,
            ..
        } = workload_result
        else {
            panic!("the §15.2 workload probability remains exact");
        };
        let workload_initial = start(&workload_product);
        let probability = &workload_values[workload_initial.index()];
        assert_eq!(probability, &expected, "{workload_name}");

        let every_result = backward_induction(
            &every_product,
            &quantile_margin_terminals(&every_product, &threshold, &quantile),
            FhObjective::Min,
            &ratio(0, 1),
            limits.max_rational_bits,
            limits.precision_bits,
            limits.max_precision_bits,
        )
        .expect("the XF-2 bounded scheduler product is acyclic");
        let FhResult::Exact {
            values: every_values,
            ..
        } = every_result
        else {
            panic!("the §15.2 scheduler transform remains exact");
        };
        let every_initial = start(&every_product);
        let transformed = &every_values[every_initial.index()];
        assert_eq!(transformed, &expected.sub(&quantile), "{every_name}");

        let workload_holds = probability >= &quantile;
        let every_holds = transformed >= &ratio(0, 1);
        assert_eq!(every_holds, workload_holds, "{every_name}");
    }
}

const THRESHOLDS: &str = "probabilistic Four using ev on M over (s: M::Server) under Steady \
     on origin { quantile 0.95 of (accumulate duration from holds(s.phase = 1) \
     until holds(s.phase = 2) within 10) <= 4 }\n\
     probabilistic Three using ev on M over (s: M::Server) under Steady \
     on origin { quantile 0.95 of (accumulate duration from holds(s.phase = 1) \
     until holds(s.phase = 2) within 10) >= 3 }";

/// Checks FR-196 "Behavior": an amount equal to the claim's threshold stays exact,
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
     probabilistic DeliverMax using ev on M over (m: M::Msg) under every scheduler \
     on origin { probability <= 0.97 [ eventually[0,2] holds(m.delivered) ] }\n\
     probabilistic DeliverFair using ev on M over (m: M::Msg) \
     under every scheduler fair weak Msg::send_a \
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

/// Checks FR-197-AC-1 (TC-632 step 1): §15.4's `Deliver` scheduler extrema,
/// selected actions and workload values match the published exact vectors.
#[trace("TC-632", "FR-197-AC-1")]
#[test]
fn deliver_backward_induction_matches_the_published_scheduler_vectors() {
    let world = World::new(&prob_world::link(DELIVER));
    let limits = ExactProbLimits::default();
    let minimum_product = built(&world, "Deliver");
    let maximum_product = built(&world, "DeliverMax");
    let workload_product = built(&world, "DeliverEven");
    let initial = start(&minimum_product);
    let lost = minimum_product
        .outgoing(initial)
        .iter()
        .find(|edge| edge.draw.as_ref().is_some_and(|draw| boolean_of(&draw[0])))
        .expect("the first send can be lost")
        .to;

    let FhResult::Exact {
        values: minimum,
        policy: minimum_policy,
    } = decide_bounded_event(
        &minimum_product,
        &qsl_semantics::check::Bound::AtLeast(ratio(97, 100)),
        &limits,
    )
    .expect("the bounded scheduler product is acyclic")
    else {
        panic!("the §15.4 minimum remains exact");
    };
    assert_eq!(minimum[initial.index()], ratio(24, 25));
    assert_eq!(minimum[lost.index()], ratio(4, 5));
    assert!(selected_operation(
        &minimum_product,
        initial,
        minimum_policy[initial.index()].expect("the initial state has actions"),
    )
    .contains("send_b"));
    assert!(selected_operation(
        &minimum_product,
        lost,
        minimum_policy[lost.index()].expect("the post-loss state has actions"),
    )
    .contains("send_b"));

    let FhResult::Exact {
        values: maximum, ..
    } = decide_bounded_event(
        &maximum_product,
        &qsl_semantics::check::Bound::AtMost(ratio(97, 100)),
        &limits,
    )
    .expect("the bounded scheduler product is acyclic")
    else {
        panic!("the §15.4 maximum remains exact");
    };
    assert_eq!(maximum[initial.index()], ratio(99, 100));

    let FhResult::Exact {
        values: workload, ..
    } = decide_bounded_event(
        &workload_product,
        &qsl_semantics::check::Bound::AtLeast(ratio(97, 100)),
        &limits,
    )
    .expect("the workload product is acyclic")
    else {
        panic!("the §15.4 workload values remain exact");
    };
    let workload_initial = start(&workload_product);
    let workload_lost = workload_product
        .outgoing(workload_initial)
        .iter()
        .find(|edge| edge.draw.as_ref().is_some_and(|draw| boolean_of(&draw[0])))
        .expect("the first workload send can be lost")
        .to;
    assert_eq!(workload[workload_initial.index()], ratio(391, 400));
    assert_eq!(workload[workload_lost.index()], ratio(17, 20));
}

/// FR-200-AC-4 (TC-635): bounded decisions are unchanged by fairness, while
/// the checked fairness set accompanies the output.
#[trace("TC-635", "FR-200-AC-4")]
#[test]
fn fair_deliver_keeps_the_24_over_25_minimum_and_records_weak_fairness() {
    let world = World::new(&prob_world::link(DELIVER));
    let ordinary = built(&world, "Deliver");
    let fair = built(&world, "DeliverFair");
    let fair_claim = world
        .package
        .graph()
        .probabilistic_model()
        .claim("DeliverFair")
        .expect("fair claim checks");
    let checked_fairness = fair_claim.fairness.clone();
    let result = qsl_analyze::exact_probabilistic::decide_fair_bounded_event(
        &fair,
        &qsl_semantics::check::Bound::AtLeast(ratio(97, 100)),
        &ExactProbLimits::default(),
    )
    .expect("bounded event is unchanged under fairness");
    let qsl_analyze::exact_probabilistic::FhResult::Exact { values, .. } = result.outcome else {
        panic!("bounded result remains exact");
    };
    assert_eq!(values[start(&ordinary).index()], ratio(24, 25));
    assert_eq!(values[start(&fair).index()], ratio(24, 25));
    assert_eq!(fair.fairness, checked_fairness);
    assert_eq!(result.fairness, fair.fairness);
}

/// FR-200-AC-3 (TC-635): the fair infimum is the slow-repair limit, while a
/// k=1 exact randomized witness mixes both repairs equally and refutes .9995.
#[trace("TC-635", "FR-200-AC-3")]
#[test]
fn fair_availability_uses_long_run_limit_and_randomized_refutation() {
    const CLAIM: &str = "probabilistic AvailFair using inf on M over (s: M::Server) \
         under every scheduler fair strong Server::repair_fast on origin \
         { long_run_fraction holds(s.up) >= rational(9995, 10000) }";
    let world = World::new(&prob_world::avail2(CLAIM));
    let product = built(&world, "AvailFair");
    let components = maximal_fair_end_components(&product, &product.fairness)
        .expect("well-formed checked fairness");
    assert_eq!(components.len(), 1, "{components:?}");
    assert_eq!(components[0].states.len(), 2);
    assert_eq!(components[0].actions.len(), 3);

    let predicate = product
        .labels
        .iter()
        .map(|labels| labels[0])
        .collect::<Vec<_>>();
    let result = decide_fair_long_run_mdp(
        &product,
        &predicate,
        None,
        &qsl_semantics::check::Bound::AtLeast(ratio(9995, 10000)),
        &ExactProbLimits::default(),
    )
    .expect("exact fair long-run decision");
    assert_eq!(result.components[0].rho, ratio(1000, 1001));
    assert_eq!(
        result.values[start(&product).index()],
        qsl_analyze::exact_probabilistic::ExtRational::Finite(ratio(1000, 1001))
    );
    let witness = result.witness.as_ref().expect("the threshold is refuted");
    assert_eq!(witness.k, 1);
    assert_eq!(
        witness.values[start(&product).index()],
        qsl_analyze::exact_probabilistic::ExtRational::Finite(ratio(1400, 1401))
    );
    let down = product
        .ids()
        .find(|state| !predicate[state.index()])
        .expect("down state");
    let randomized = match witness.policy[down.index()]
        .as_ref()
        .expect("down scheduler choice")
    {
        qsl_analyze::exact_probabilistic::SchedulerChoice::Randomized(choices) => choices,
        other => panic!("expected a randomized fair witness, got {other:?}"),
    };
    assert_eq!(randomized.len(), 2);
    assert!(randomized
        .iter()
        .all(|(_, probability)| probability == &ratio(1, 2)));
    let mut names = randomized
        .iter()
        .map(|(choice, _)| {
            product
                .outgoing(down)
                .iter()
                .filter_map(|edge| edge.action.as_ref())
                .fold(Vec::<String>::new(), |mut names, action| {
                    if !names.contains(&action.operation) {
                        names.push(action.operation.clone());
                    }
                    names
                })
                .get(choice.0)
                .expect("selected action index")
                .clone()
        })
        .collect::<Vec<_>>();
    names.sort();
    assert!(names[0].contains("repair_fast"));
    assert!(names[1].contains("repair_slow"));
    assert_eq!(result.fairness, product.fairness);
}

/// FR-198-AC-2 under FR-200: a zero-reward wait MEC collapses while retaining
/// its positive-reward exit, yielding a finite maximum of five.
#[trace("TC-633", "FR-198-AC-2")]
#[test]
fn fair_max_expected_reward_preserves_zero_reward_mec_exits() {
    const CLAIM: &str = "probabilistic CostFair using inf on M over (p: M::Proc) \
         under every scheduler fair strong Proc::exit on origin \
         { expected accumulate cost until holds(p.done) <= 5 }";
    let world = World::new(&prob_world::zero_reward_exit(CLAIM));
    let product = built(&world, "CostFair");
    let target = product
        .labels
        .iter()
        .map(|labels| labels[0])
        .collect::<Vec<_>>();
    let result = qsl_analyze::exact_probabilistic::decide_fair_expected_reward(
        &product,
        &target,
        "cost",
        UnboundedObjective::Max,
        &ExactProbLimits::default(),
    )
    .expect("the strong-fair expected maximum is finite");
    assert_eq!(
        result.values[start(&product).index()],
        qsl_analyze::exact_probabilistic::ExtRational::Finite(ratio(5, 1))
    );
    let action = match result.policy.as_ref().expect("a fair scheduler")[start(&product).index()]
        .as_ref()
        .expect("a live choice")
    {
        qsl_analyze::exact_probabilistic::SchedulerChoice::Deterministic(
            qsl_analyze::exact_probabilistic::ActionChoice(index),
        ) => *index,
        other => panic!("expected the exit action, got {other:?}"),
    };
    let mut operations = Vec::new();
    for edge in product.outgoing(start(&product)) {
        if let Some(identity) = edge.action.as_ref() {
            if !operations.iter().any(|seen| seen == identity) {
                operations.push(identity.clone());
            }
        }
    }
    assert!(operations[action].operation.contains("exit"));
}

/// FR-200 FS-8: an unfair wait MEC collapses to a proper route into the fair
/// recurrent component instead of becoming a closed policy class.
#[trace("TC-635")]
#[test]
fn fair_long_run_routes_away_from_a_nonfair_mec() {
    const CLAIM: &str = "probabilistic FairEscape using inf on M over (p: M::Proc) \
         under every scheduler fair strong Proc::go_fair on origin \
         { long_run_fraction holds(p.fair) >= rational(9, 10) }";
    let world = World::new(&prob_world::fair_longrun_escape(CLAIM));
    let product = built(&world, "FairEscape");
    let fair_components = maximal_fair_end_components(&product, &product.fairness)
        .expect("well-formed checked fairness");
    assert_eq!(fair_components.len(), 1);
    assert_eq!(fair_components[0].states.len(), 1);
    assert!(fair_components[0].actions[0]
        .1
        .operation
        .contains("continue_fair"));
    let predicate = product
        .labels
        .iter()
        .map(|labels| labels[0])
        .collect::<Vec<_>>();
    let result = decide_fair_long_run_mdp(
        &product,
        &predicate,
        None,
        &qsl_semantics::check::Bound::AtLeast(ratio(9, 10)),
        &ExactProbLimits::default(),
    )
    .expect("fair long-run routes through the non-fair MEC");
    assert_eq!(
        result.values[start(&product).index()],
        qsl_analyze::exact_probabilistic::ExtRational::Finite(ratio(1, 1))
    );
    assert_eq!(
        result.gain_bias[0].bias,
        vec![(fair_components[0].states[0], ratio(0, 1))]
    );
}

/// Checks FR-196-AC-3 (TC-631 step 3): §15.4's `Deliver` over every scheduler is an
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

/// Checks FR-196 "Behavior": a terminal model state closes a bounded event under
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

/// Checks FR-196-AC-4 (TC-631 step 4): FR-187-AC-3's `Health` variant with two
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

/// Checks FR-196-AC-5 (TC-641 step 1): when the claim reads a value that is
/// undefined at a product state of positive probability, the build stops at
/// the first such state in breadth-first order and returns the path to it.
/// The spec's `Coin` divides by `1 - c.v`; S3 types neither `/` in a model
/// unit nor a population name in a claim's predicate, so this `Coin` is
/// dropped when `b = 0` and the claim's `c.v` then reads `absent-key`
/// undefined. The path is `drop` with `b = 0`, probability 1/2, at
/// position 1.
#[trace("TC-641", "FR-196-AC-5", "FR-196-AC-12")]
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
    assert_eq!(step.target, path.key);
    assert!(step.resolved.is_none());
    assert_eq!(path.binding, path.key.binding);
    assert!(path.key.pending.is_none());
    assert!(matches!(path.read, UndefinedRead::Atom(_)));
    assert!(matches!(
        path.cause,
        UndefinedCause::Family(ref record) if record.reason == UndefinedReason::AbsentKey
    ));
}

const ARMED: &str = "probabilistic Armed using ev on M over (c: M::Coin) under Even on origin \
     { probability >= 0.5 [ always[0,3] holds(c.armed = c.armed) ] }";

/// Checks FR-196-AC-5 (TC-641 step 1): the path to an undefined state lists every
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
    assert!(path.steps.iter().all(|step| step.resolved.is_none()));
}

const GHOST: &str = "probabilistic Ghost using ev on M over (c: M::Coin) under Even on origin \
     { probability >= 0.5 [ always[0,3] holds(c.v = 0) ] }\n\
     probabilistic GhostSteps using ev on M over (c: M::Coin) under Even on origin \
     { probability >= 0.5 [ steps from holds(c.v = 0) until holds(c.v = 1) within 3 <= 3 ] }\n\
     probabilistic GhostFrom using inf on M over (c: M::Coin) under Even on origin \
     { probability >= 0.5 [ holds(c.v = 0) until holds(c.v = 1) ] }\n\
     probabilistic GhostTarget using inf on M over (c: M::Coin) under Even on origin \
     { probability >= 0.5 [ eventually holds(c.v = 0) ] }";

/// Checks FR-196 "Behavior": the claim's `over` instances are the subject's
/// universe, in order, and a state at which a predicate is undefined for an
/// instance ends the build. The second instance has no object in the initial
/// snapshot, so its predicate is undefined at position 0, with no step to
/// it.
#[trace("TC-641", "FR-196-AC-13")]
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
    assert_eq!(path.initial, 0);
    assert_eq!(path.probability, ratio(1, 1));
    assert_eq!(path.binding, path.key.binding);
    assert!(path
        .key
        .monitor
        .is_some_and(|monitor| matches!(monitor, MonitorState::Formula(FormulaState::Running(_)))));
    assert!(path.key.accumulator.is_none());
    assert!(path.key.pending.is_none());
    assert!(!path.key.model.is_empty());
    assert!(matches!(path.read, UndefinedRead::Atom(_)));
    assert!(matches!(
        &path.binding,
        Some(binding) if binding.key == "ghost"
    ));
    let ProbBuild::Undefined(path) = build_product(
        world.request("GhostSteps", ModelCheckLimits::default()),
        &Cancel::new(),
    )
    .expect("the initial measure read is undefined") else {
        panic!("an undefined measure read");
    };
    assert_eq!(path.position, 0);
    assert_eq!(path.initial, 0);
    assert!(matches!(
        path.key.monitor,
        Some(MonitorState::Measure {
            remaining: 3,
            phase: MeasurePhase::Inactive
        })
    ));
    assert_eq!(
        path.key.accumulator,
        Some(Accumulator::Steps {
            activated: false,
            amount: Amount::Exact(ratio(0, 1))
        })
    );
    assert!(path.key.pending.is_none());
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

/// Checks FR-196 "Behavior": a bounded measure continues on the terminal stutter
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

/// Checks FR-196 "Behavior": over every scheduler, draws of one action that reach
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
     probabilistic WeakFairTerminates using inf on M over (p: M::Proc) \
     under every scheduler fair weak Proc::flip \
     on origin { probability >= 0.99 [ eventually holds(p.done) ] }\n\
     probabilistic FairMax using inf on M over (p: M::Proc) \
     under every scheduler fair strong Proc::wait \
     on origin { probability <= 0.99 [ eventually holds(p.done) ] }\n\
     probabilistic Unless using inf on M over (p: M::Proc) under Even on origin \
     { probability >= 0.9 [ holds(not p.done) until holds(p.done) ] }";

/// Checks FR-196 "Behavior", exact-only forms: an unbounded reachability claim uses
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

/// FR-200-AC-1 (TC-635): fairness removes the live wait component for both
/// weak and strong fairness to `flip`, making termination's infimum one.
#[trace("TC-635", "FR-200-AC-1")]
#[test]
fn fair_coin_has_no_nonterminal_fair_end_component() {
    let world = World::new(&prob_world::process(REACH));
    for name in ["FairTerminates", "WeakFairTerminates"] {
        let product = built(&world, name);
        let target = product
            .labels
            .iter()
            .map(|labels| labels[0])
            .collect::<Vec<_>>();
        let components = maximal_fair_end_components(&product, &product.fairness)
            .expect("well-formed checked fairness");
        assert!(
            components
                .iter()
                .all(|component| component.states.iter().all(|state| target[state.index()])),
            "only target terminal stutter components may remain globally: {name}: {components:?}"
        );
        let result = decide_fair_reachability(
            &product,
            None,
            &target,
            UnboundedObjective::Min,
            &ExactProbLimits::default(),
        )
        .expect("exact fair reachability");
        assert!(
            result.components.is_empty(),
            "the non-target restriction has no fair end component"
        );
        assert_eq!(
            result.values[start(&product).index()],
            qsl_analyze::exact_probabilistic::ExtRational::Finite(ratio(1, 1)),
            "target={target:?}, product labels={:?}, fair states={:?}",
            product.labels,
            result.components
        );
        assert_eq!(result.fairness, product.fairness);
    }
    let plain = built(&world, "Terminates");
    let target = plain
        .labels
        .iter()
        .map(|labels| labels[0])
        .collect::<Vec<_>>();
    let plain_min = qsl_analyze::exact_probabilistic::decide_reachability(
        &plain,
        &target,
        UnboundedObjective::Min,
        &ExactProbLimits::default(),
    )
    .expect("exact unrestricted reachability");
    assert_eq!(
        plain_min.values[start(&plain).index()],
        qsl_analyze::exact_probabilistic::ExtRational::Finite(ratio(0, 1))
    );

    let maximizing = built(&world, "FairMax");
    let components = maximal_fair_end_components(&maximizing, &maximizing.fairness)
        .expect("well-formed fairness");
    assert!(components.iter().any(|component| {
        component.states.contains(&start(&maximizing))
            && component
                .actions
                .iter()
                .any(|(_, action)| action.operation.contains("wait"))
    }));
    let target = maximizing
        .labels
        .iter()
        .map(|labels| labels[0])
        .collect::<Vec<_>>();
    let maximum = decide_fair_reachability(
        &maximizing,
        None,
        &target,
        UnboundedObjective::Max,
        &ExactProbLimits::default(),
    )
    .expect("fair maximum reachability");
    assert_eq!(
        maximum.values[start(&maximizing).index()],
        qsl_analyze::exact_probabilistic::ExtRational::Finite(ratio(1, 1))
    );
    let flip_index = match maximum.policy[start(&maximizing).index()]
        .as_ref()
        .expect("a maximizing action")
    {
        qsl_analyze::exact_probabilistic::SchedulerChoice::Deterministic(
            qsl_analyze::exact_probabilistic::ActionChoice(index),
        ) => *index,
        other => panic!("the positive-value state takes FR-198's policy: {other:?}"),
    };
    let mut operations = Vec::new();
    for edge in maximizing.outgoing(start(&maximizing)) {
        if let Some(action) = edge.action.as_ref() {
            if !operations.iter().any(|seen| seen == action) {
                operations.push(action.clone());
            }
        }
    }
    assert!(operations[flip_index].operation.contains("flip"));
}

/// A terminal stutter-only component is fair when it remains in the
/// non-target graph, despite having no scheduled identities.
#[trace("TC-635", "FR-200-AC-5")]
#[test]
fn non_target_terminal_stutter_belongs_to_fair_reachability_u() {
    const CLAIM: &str = "probabilistic TerminalFair using inf on M over (t: M::TerminalFair) \
         under every scheduler fair strong TerminalFair::proceed on origin \
         { probability >= 0.5 [ eventually holds(t.done) ] }";
    let world = World::new(&prob_world::fair_terminal_stutter(CLAIM));
    let product = built(&world, "TerminalFair");
    let target = product
        .labels
        .iter()
        .map(|labels| labels[0])
        .collect::<Vec<_>>();
    let result = decide_fair_reachability(
        &product,
        None,
        &target,
        UnboundedObjective::Min,
        &ExactProbLimits::default(),
    )
    .expect("non-target terminal stutter is a fair end component");
    let terminal_stutter = result
        .components
        .iter()
        .find(|component| component.actions.is_empty())
        .expect("terminal stutter is included in U");
    assert!(terminal_stutter
        .states
        .iter()
        .any(|state| !target[state.index()]));
    assert_eq!(
        result.values[start(&product).index()],
        qsl_analyze::exact_probabilistic::ExtRational::Finite(ratio(0, 1))
    );
    let initial = start(&product);
    let selected = match result.policy[initial.index()]
        .as_ref()
        .expect("the initial state has a fair scheduler choice")
    {
        qsl_analyze::exact_probabilistic::SchedulerChoice::Deterministic(
            qsl_analyze::exact_probabilistic::ActionChoice(index),
        ) => vec![*index],
        qsl_analyze::exact_probabilistic::SchedulerChoice::Randomized(choices) => choices
            .iter()
            .filter(|(_, probability)| probability > &ratio(0, 1))
            .map(|(choice, _)| choice.0)
            .collect(),
    };
    let mut actions = Vec::new();
    for edge in product.outgoing(initial) {
        if let Some(identity) = edge.action.as_ref() {
            if !actions.iter().any(|seen| seen == identity) {
                actions.push(identity.clone());
            }
        }
    }
    assert!(selected
        .iter()
        .any(|index| actions[*index].operation.contains("abandon")));
}

/// Expected-reward minimization ranges over fair schedulers: `free` alone is
/// an unfair recurrent choice, so the unit-cost fair exit is required.
#[test]
fn fair_expected_reward_minimum_excludes_an_unfair_free_bottom_class() {
    const CLAIM: &str = "probabilistic FairMinimum using inf on M over (p: M::Proc) \
         under every scheduler fair strong Proc::costly on origin \
         { expected accumulate cost until holds(p.done) <= 1 }";
    let world = World::new(&prob_world::fair_min_reward(CLAIM));
    let product = built(&world, "FairMinimum");
    let target = product
        .labels
        .iter()
        .map(|labels| labels[0])
        .collect::<Vec<_>>();
    let unrestricted = qsl_analyze::exact_probabilistic::decide_expected_reward(
        &product,
        &target,
        "cost",
        UnboundedObjective::Min,
        &ExactProbLimits::default(),
    )
    .expect("FR-198 minimum over the same fixture");
    assert_eq!(
        unrestricted.values[start(&product).index()],
        qsl_analyze::exact_probabilistic::ExtRational::Finite(ratio(1, 1))
    );
    let unrestricted_choice =
        unrestricted.policy[start(&product).index()].expect("the live FR-198 state has a policy");
    let mut unrestricted_actions = Vec::new();
    for edge in product.outgoing(start(&product)) {
        if let Some(identity) = edge.action.as_ref() {
            if !unrestricted_actions.iter().any(|seen| seen == identity) {
                unrestricted_actions.push(identity.clone());
            }
        }
    }
    assert!(unrestricted_actions[unrestricted_choice]
        .operation
        .contains("costly"));
    let result = qsl_analyze::exact_probabilistic::decide_fair_expected_reward(
        &product,
        &target,
        "cost",
        UnboundedObjective::Min,
        &ExactProbLimits::default(),
    )
    .expect("fair minimum uses the unit-cost exit");
    assert_eq!(
        result.values[start(&product).index()],
        qsl_analyze::exact_probabilistic::ExtRational::Finite(ratio(1, 1))
    );
    let policy = result
        .policy
        .as_ref()
        .expect("the result carries a fair witness");
    let selected = match policy[start(&product).index()]
        .as_ref()
        .expect("the initial state has a scheduler choice")
    {
        qsl_analyze::exact_probabilistic::SchedulerChoice::Deterministic(
            qsl_analyze::exact_probabilistic::ActionChoice(index),
        ) => *index,
        other => panic!("the fair expected-reward minimum is deterministic: {other:?}"),
    };
    let mut actions = Vec::new();
    for edge in product.outgoing(start(&product)) {
        if let Some(identity) = edge.action.as_ref() {
            if !actions.iter().any(|seen| seen == identity) {
                actions.push(identity.clone());
            }
        }
    }
    assert!(
        actions[selected].operation.contains("costly"),
        "{actions:?}"
    );
}

/// FR-200-AC-2 (TC-635): a strong-fair scheduler can reach the stuck state,
/// where `stall` is the only action and the fairness constraint is disabled.
#[trace("TC-635", "FR-200-AC-2")]
#[test]
fn coin2_stall_is_a_fair_end_component_and_witness() {
    const CLAIM: &str = "probabilistic StuckFair using inf on M over (p: M::Proc) \
         under every scheduler fair strong Proc::flip on origin \
         { probability >= 0.99 [ eventually holds(p.done) ] }\n\
         probabilistic RewardFair using inf on M over (p: M::Proc) \
         under every scheduler fair strong Proc::flip on origin \
         { expected accumulate cost until holds(p.done) <= 3 }";
    let world = World::new(&prob_world::process_coin2(CLAIM));
    let product = built(&world, "StuckFair");
    let components = maximal_fair_end_components(&product, &product.fairness)
        .expect("well-formed checked fairness");
    let component = components
        .iter()
        .find(|component| {
            component
                .actions
                .iter()
                .any(|(_, identity)| identity.operation.contains("stall"))
        })
        .expect("the stuck fair component is present");
    assert_eq!(component.states.len(), 1);
    assert_eq!(component.actions.len(), 1);
    assert!(component.actions[0].1.operation.contains("stall"));

    let target = product
        .labels
        .iter()
        .map(|labels| labels[0])
        .collect::<Vec<_>>();
    let result = decide_fair_reachability(
        &product,
        None,
        &target,
        UnboundedObjective::Min,
        &ExactProbLimits::default(),
    )
    .expect("exact fair reachability");
    assert_eq!(
        result.values[start(&product).index()],
        qsl_analyze::exact_probabilistic::ExtRational::Finite(ratio(0, 1))
    );
    let live = start(&product);
    let selected = match result.policy[live.index()].as_ref().expect("live choice") {
        qsl_analyze::exact_probabilistic::SchedulerChoice::Deterministic(
            qsl_analyze::exact_probabilistic::ActionChoice(index),
        ) => *index,
        other => panic!("expected the exact reachability action, got {other:?}"),
    };
    let mut identities = Vec::new();
    for edge in product.outgoing(live) {
        if let Some(identity) = edge.action.as_ref() {
            if !identities.iter().any(|seen| seen == identity) {
                identities.push(identity.clone());
            }
        }
    }
    assert!(identities[selected].operation.contains("stall"));
    assert!(component.states.contains(&component.actions[0].0));

    let reward_product = built(&world, "RewardFair");
    let target = reward_product
        .labels
        .iter()
        .map(|labels| labels[0])
        .collect::<Vec<_>>();
    let maximum = qsl_analyze::exact_probabilistic::decide_fair_expected_reward(
        &reward_product,
        &target,
        "cost",
        UnboundedObjective::Max,
        &ExactProbLimits::default(),
    )
    .expect("the fair reward maximum is unbounded");
    assert_eq!(
        maximum.values[start(&reward_product).index()],
        qsl_analyze::exact_probabilistic::ExtRational::PlusInfinity
    );
    assert!(
        maximum.policy.is_some(),
        "the result carries a fair scheduler"
    );
    let stuck = component.actions[0].0;
    let operation_choices = |state| {
        let mut choices = Vec::new();
        for edge in reward_product.outgoing(state) {
            if let Some(identity) = edge.action.as_ref() {
                if !choices.iter().any(|seen| seen == identity) {
                    choices.push(identity.clone());
                }
            }
        }
        choices
    };
    let reward_policy = maximum.policy.as_ref().expect("a fair scheduler");
    let selected = match reward_policy[stuck.index()]
        .as_ref()
        .expect("stuck scheduler choice")
    {
        qsl_analyze::exact_probabilistic::SchedulerChoice::Deterministic(
            qsl_analyze::exact_probabilistic::ActionChoice(index),
        ) => vec![*index],
        qsl_analyze::exact_probabilistic::SchedulerChoice::Randomized(choices) => choices
            .iter()
            .filter(|(_, probability)| probability > &ratio(0, 1))
            .map(|(choice, _)| choice.0)
            .collect(),
    };
    assert!(selected
        .iter()
        .any(|index| operation_choices(stuck)[*index].operation.contains("stall")));
    let minimum = qsl_analyze::exact_probabilistic::decide_fair_expected_reward(
        &reward_product,
        &target,
        "cost",
        UnboundedObjective::Min,
        &ExactProbLimits::default(),
    )
    .expect("the fair reward minimum uses FR-198");
    let fr198_minimum = qsl_analyze::exact_probabilistic::decide_expected_reward(
        &reward_product,
        &target,
        "cost",
        UnboundedObjective::Min,
        &ExactProbLimits::default(),
    )
    .expect("the FR-198 minimum is exact");
    assert_eq!(minimum.values, fr198_minimum.values);
    assert!(
        minimum.policy.is_some(),
        "the result carries a fair scheduler"
    );
}

/// `each` fairness creates an obligation for every scheduled identity,
/// including the receiver component of an otherwise shared operation.
#[test]
fn fair_each_refines_one_operation_into_per_receiver_identities() {
    const CLAIM: &str = "probabilistic EachIdentity using inf on M over (p: M::Proc) \
         under every scheduler fair strong each Proc::wait on origin \
         { probability >= 0.5 [ eventually holds(p.done) ] }";
    let fixture = prob_world::process(CLAIM);
    let world = World::symmetric_objects(&fixture, &["a", "b"]);
    let product = built(&world, "EachIdentity");
    assert_eq!(product.fairness.len(), 1);
    assert_eq!(
        product.fairness[0].granularity(),
        quire_semantic_value::fairness::FairnessGranularity::Each
    );

    let components = maximal_fair_end_components(&product, &product.fairness)
        .expect("checked per-identity fairness");
    let wait_identities = components
        .iter()
        .flat_map(|component| component.actions.iter())
        .filter(|(_, identity)| identity.operation.contains("wait"))
        .map(|(_, identity)| identity.receiver.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(wait_identities.len(), 2, "{components:?}");
}

#[trace("TC-635", "FR-200-AC-6")]
#[test]
fn fair_weak_each_refines_one_operation_into_per_receiver_identities() {
    const CLAIM: &str = "probabilistic WeakEachIdentity using inf on M over (p: M::Pair) \
         under every scheduler fair weak each Pair::tick on origin \
         { probability >= 0.5 [ eventually holds(p.live) ] }";
    let fixture = prob_world::pair_tick(CLAIM);
    let world = World::symmetric_objects(&fixture, &["a", "b"]);
    let product = built(&world, "WeakEachIdentity");
    let components = maximal_fair_end_components(&product, &product.fairness)
        .expect("checked per-identity weak fairness");
    let wait_identities = components
        .iter()
        .flat_map(|component| component.actions.iter())
        .filter(|(_, identity)| identity.operation.contains("tick"))
        .map(|(_, identity)| identity.receiver.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(wait_identities.len(), 2, "{components:?}");
}

/// A randomized split reaches either of two distinct fair bottom SCCs; the
/// witness is checked against both components.
#[trace("TC-635", "FR-200-AC-7")]
#[test]
fn fair_witness_validates_multiple_bottom_sccs() {
    const CLAIM: &str = "probabilistic SplitFair using inf on M over (s: M::Split) \
         under every scheduler fair strong Split::step on origin \
         { probability >= 0.5 [ eventually holds(s.right) ] }";
    let world = World::new(&prob_world::fair_two_bottoms(CLAIM));
    let product = built(&world, "SplitFair");
    let components = maximal_fair_end_components(&product, &product.fairness)
        .expect("two disjoint fair end components");
    assert_eq!(components.len(), 2, "{components:?}");
    assert!(
        components.iter().all(|component| component
            .actions
            .iter()
            .any(|(_, identity)| identity.operation.contains("step"))),
        "components={components:?} states={:?} edges={:?}",
        product.states,
        product.edges
    );

    let target = product
        .labels
        .iter()
        .map(|labels| labels[0])
        .collect::<Vec<_>>();
    let result = decide_fair_reachability(
        &product,
        None,
        &target,
        UnboundedObjective::Min,
        &ExactProbLimits::default(),
    )
    .expect("the returned scheduler is checked over every bottom SCC");
    assert_eq!(
        result.values[start(&product).index()],
        qsl_analyze::exact_probabilistic::ExtRational::Finite(ratio(1, 2))
    );
    let bottom_states = components
        .iter()
        .map(|component| *component.states.first().expect("a bottom state"))
        .collect::<BTreeSet<_>>();
    assert_eq!(bottom_states.len(), 2);
    let mut reachable = BTreeSet::from([start(&product)]);
    let mut pending = VecDeque::from([start(&product)]);
    while let Some(state) = pending.pop_front() {
        for edge in product.outgoing(state) {
            if reachable.insert(edge.to) {
                pending.push_back(edge.to);
            }
        }
    }
    for bottom in bottom_states {
        assert!(reachable.contains(&bottom));
        assert!(result.policy[bottom.index()].is_some());
    }
}

const COST: &str = "probabilistic Cost using ev on M over (m: M::Msg) under Even on origin \
     { expected accumulate cost until holds(m.delivered or m.attempts = 2) <= rational(6, 5) }";

/// Checks FR-196 "Behavior", expected reward: the DTMC itself is the product, the
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

/// Checks FR-196 "Behavior", long-run fraction: the DTMC itself is the product with
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

/// Checks FR-196 "Behavior", bounded measures: a `steps` measure counts one per
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

/// Checks FR-196 "Behavior": reaching `max_states`, `max_transitions` or
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
            limit: ModelCheckLimit::ExactProbMaxStates,
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
            limit: ModelCheckLimit::ExactProbMaxTransitions,
            value: 3
        }
    );
    assert_eq!(
        stopped(ModelCheckLimits {
            max_automaton_states: 2,
            ..defaults
        }),
        ProductStop::Limit {
            limit: ModelCheckLimit::ExactProbMaxAutomatonStates,
            value: 2
        }
    );
    let setting_of = |limits: ModelCheckLimits| match stopped(limits) {
        ProductStop::Limit { limit, value } => (limit.setting(), value),
        other => panic!("expected a limit stop: {other:?}"),
    };
    assert_eq!(
        setting_of(ModelCheckLimits {
            limits: Limits {
                max_states: 5,
                ..defaults.limits
            },
            ..defaults
        }),
        ("exact_prob.max_states", 5)
    );
    assert_eq!(
        setting_of(ModelCheckLimits {
            limits: Limits {
                max_transitions: 3,
                ..defaults.limits
            },
            ..defaults
        }),
        ("exact_prob.max_transitions", 3)
    );
    assert_eq!(
        setting_of(ModelCheckLimits {
            max_automaton_states: 2,
            ..defaults
        }),
        ("exact_prob.max_automaton_states", 2)
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

/// Checks FR-195-AC-4, refusal half only (TC-630 step 4; the manifest and the
/// FR-192 settlement are not built): a claim only exact evidence decides is
/// refused at negotiation, before any sample, with the cause that names why:
/// `EveryScheduler` for `FairTerminates`, `ExactOnlyForm` for an
/// expected-reward claim under a workload, `MissingConfidence` for a bounded
/// event with no confidence parameters; a statistical claim passes.
#[trace("TC-630")]
#[test]
fn statistical_evidence_is_refused_for_a_claim_only_exact_evidence_decides() {
    use qsl_analyze::statistical::require_statistical;
    use qsl_replay::statistical::StatisticalRefusal;

    let process = World::new(&prob_world::process(&format!("{REACH}\n{NEGOTIATED}")));
    let verdict = |claim: &str| require_statistical(process.claim(claim));
    // The gate refuses exactly the claims `exact_only` marks.
    for name in [
        "FairTerminates",
        "Terminates",
        "Anywhere",
        "Reached",
        "Unless",
        "Unseeded",
        "Seeded",
    ] {
        assert_eq!(
            verdict(name).is_err(),
            process.claim(name).exact_only(),
            "{name}"
        );
    }
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

const ACTIVATED_AT_TERMINAL: &str = "probabilistic Late using ev on M over (m: M::Msg) under Even \
     on origin { probability >= 0.5 [ steps from holds(m.delivered) \
     until holds(not m.delivered) within 4 <= 10 ] }";

const HORIZON: &str =
    "probabilistic Idle using ev on M over (s: M::Server) under Steady on origin \
     { probability >= 0.5 [ steps from holds(s.phase = 1 and s.attempts = 2) \
     until holds(s.phase = 2) within 3 <= 5 ] }\n\
     probabilistic Source using ev on M over (s: M::Server) under Steady on origin \
     { probability >= 0.5 [ fraction holds(s.phase = 0) over [0, 2] >= rational(1, 2) ] }";

/// Checks FR-196 "Behavior" (TC-631): a `steps` measure whose `A` never holds ends
/// every path at `Measure{remaining: 0, phase: Unactivated}` with accumulator
/// `Steps{activated: false, amount: 0}`, decided and not expanded; a `steps`
/// measure activated at a terminal model state counts each stutter step up to
/// its horizon; a fraction reads its predicate at the position the step
/// leaves, not at the position it reaches.
#[trace("TC-631")]
#[test]
fn an_unactivated_measure_ends_decided_and_a_fraction_reads_the_source() {
    let world = World::new(&prob_world::service(HORIZON));
    let idle = built(&world, "Idle");
    let ended: Vec<_> = idle
        .states
        .iter()
        .filter(|key| key.monitor.is_some_and(MonitorState::is_decided))
        .collect();
    assert!(!ended.is_empty());
    for key in ended {
        assert_eq!(
            key.monitor,
            Some(MonitorState::Measure {
                remaining: 0,
                phase: MeasurePhase::Unactivated
            })
        );
        assert_eq!(
            key.accumulator,
            Some(Accumulator::Steps {
                activated: false,
                amount: Amount::Exact(ratio(0, 1))
            })
        );
    }
    // Every path ends there, and a decided state is not expanded.
    for state in idle.ids() {
        assert_eq!(
            idle.outgoing(state).is_empty(),
            is_decided(&idle, state),
            "{:?}",
            idle.states[state.index()]
        );
    }

    // Idle holds only at the first position; the positions reached never do.
    let source = built(&world, "Source");
    let complete = distinct(
        source
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
            .collect(),
    );
    assert_eq!(complete, [(1, 3)]);

    // `Late` activates where the message is delivered, a terminal model
    // state: each stutter step from there adds one step until the window
    // ends censored.
    let late = World::new(&prob_world::link(ACTIVATED_AT_TERMINAL));
    let product = built(&late, "Late");
    let steps = |state: ProbStateId| match &product.states[state.index()] {
        ProbKey {
            monitor: Some(MonitorState::Measure { remaining, .. }),
            accumulator: Some(Accumulator::Steps { activated, amount }),
            ..
        } => (*remaining, *activated, amount.clone()),
        other => panic!("a steps measure: {other:?}"),
    };
    let activated_terminal: Vec<ProbStateId> = product
        .edges
        .iter()
        .filter(|edge| matches!(edge.choice, ProductAction::TerminalStutter))
        .map(|edge| edge.from)
        .filter(|state| {
            let (_, activated, amount) = steps(*state);
            activated && amount == Amount::Exact(ratio(0, 1))
        })
        .collect();
    assert!(!activated_terminal.is_empty());
    for start in activated_terminal {
        let mut chain = vec![steps(start)];
        let mut at = start;
        while let [edge] = product.outgoing(at) {
            assert!(matches!(edge.choice, ProductAction::TerminalStutter));
            at = edge.to;
            chain.push(steps(at));
        }
        let remaining = chain[0].0;
        let expected: Vec<_> = (0..=remaining)
            .map(|taken| {
                let amount = if taken == remaining {
                    Amount::Censored
                } else {
                    Amount::Exact(ratio(i64::try_from(taken).expect("small"), 1))
                };
                (remaining - taken, true, amount)
            })
            .collect();
        assert_eq!(chain, expected);
    }
}

const LOOSE_COST: &str = "reward cost on Node::tick = 2;\n\
     probabilistic Cost using ev on M over (n: M::Node) under every scheduler on origin \
     { probability >= 0.5 [ accumulate cost from holds(n.healthy) \
     until holds(not n.healthy) within 3 >= 5 ] }";

/// Checks FR-196 "Behavior", SCH-2: a draw with several post-states advances the
/// accumulator once, at the resolution, with the draw's rewards; the edge
/// that took the draw carries them, and the resolution edges carry every
/// reward at 0.
#[trace("TC-631")]
#[test]
fn a_resolution_advances_the_accumulator_with_the_draws_rewards() {
    let world = World::new(&prob_world::health(
        "self.healthy = (pre(self.healthy) and not fault)",
        "true: 1, false: 9999999",
        true,
        LOOSE_COST,
    ));
    let product = built(&world, "Cost");
    let held = distinct(
        product
            .states
            .iter()
            .filter(|key| key.monitor.is_some_and(MonitorState::is_decided))
            .filter_map(|key| match &key.accumulator {
                Some(Accumulator::Reward { amount, .. }) => Some(amount.clone()),
                _ => None,
            })
            .collect(),
    );
    // A fault at the first tick ends at 2, at the second at 4; a third tick
    // without a fault is above 5.
    for value in [2, 4] {
        assert!(
            held.contains(&Amount::Exact(ratio(value, 1))),
            "{value}: {held:?}"
        );
    }
    let cost = |edge: &ProbEdge| {
        let [(name, value)] = &edge.rewards[..] else {
            panic!("one reward: {:?}", edge.rewards);
        };
        assert_eq!(name.as_str(), "cost");
        integer_of(value)
    };
    let mut draws = 0;
    for edge in &product.edges {
        match edge.choice {
            ProductAction::Identity(_) => {
                draws += 1;
                assert_eq!(cost(edge), 2);
            }
            ProductAction::PostState(_) => assert_eq!(cost(edge), 0),
            ProductAction::TerminalStutter => {}
        }
    }
    assert!(draws > 0);
}

const BINDINGS: &str = "probabilistic Terminates using inf on M over (p: M::Proc) \
     under every scheduler on origin \
     { probability >= 0.99 [ eventually holds(p.done) ] }";

/// Checks FR-196-AC-10 (TC-631): over a population of two live objects, a
/// claim `over` the object type builds two initial product states for the
/// subject's one initial state, one per binding; the same model state under
/// the same monitor state and the two bindings is two product states whose
/// keys differ in `binding`, and the two have different labels.
#[trace("TC-631", "FR-196-AC-10")]
#[test]
fn each_over_binding_has_its_own_states() {
    let world = World::with_objects(&prob_world::process(BINDINGS), &["p", "q"]);
    let product = built(&world, "Terminates");
    let [(initial_a, first, a), (initial_b, second, b)] = &product.initial[..] else {
        panic!("one initial state per binding: {:?}", product.initial);
    };
    assert_eq!(initial_a, initial_b, "both start at the one initial state");
    assert_ne!(a, b);
    assert!(first.is_some() && second.is_some() && first != second);
    assert_eq!(product.states[a.index()].binding, *first);
    assert_eq!(product.states[b.index()].binding, *second);
    let (x, y) = (&product.states[a.index()], &product.states[b.index()]);
    assert_eq!((&x.model, &x.monitor), (&y.model, &y.monitor));
    assert_ne!(x, y);
    // One model state, two readings: where `p` is done and `q` is not.
    let differing = product.ids().any(|left| {
        product.ids().any(|right| {
            let (x, y) = (
                &product.states[left.index()],
                &product.states[right.index()],
            );
            x.model == y.model
                && x.monitor == y.monitor
                && x.binding != y.binding
                && x != y
                && product.labels[left.index()] != product.labels[right.index()]
        })
    });
    assert!(differing);
}

/// Checks FR-196 "Behavior": reaching `max_states` or `max_transitions` stops the
/// build, and a build of exactly that size completes.
#[trace("TC-631")]
#[test]
fn a_budget_equal_to_the_product_size_completes() {
    let world = World::new(&prob_world::no_fault(NO_FAULT));
    let product = built(&world, "NoFault");
    let (states, edges) = (product.states.len(), product.edges.len());
    assert_eq!((states, edges), (1002, 2000));
    let defaults = ModelCheckLimits::default();
    let with = |max_states, max_transitions| ModelCheckLimits {
        limits: Limits {
            max_states,
            max_transitions,
        },
        ..defaults
    };
    let ran = |limits| build_product(world.request("NoFault", limits), &Cancel::new());
    assert!(matches!(ran(with(states, edges)), Ok(ProbBuild::Built(_))));
    assert!(matches!(
        ran(with(states - 1, edges)),
        Err(ProductRefusal::Stopped(ProductStop::Limit {
            limit: ModelCheckLimit::ExactProbMaxStates,
            ..
        }))
    ));
    assert!(matches!(
        ran(with(states, edges - 1)),
        Err(ProductRefusal::Stopped(ProductStop::Limit {
            limit: ModelCheckLimit::ExactProbMaxTransitions,
            ..
        }))
    ));
}

const TOSS: &str = "probabilistic Tossed using ev on M over (c: M::Coin) under every scheduler \
     on origin { probability >= 0.5 [ always[0,3] holds(c.v = 0) ] }\n\
     probabilistic Plain using ev on M under every scheduler on origin \
     { probability >= 0.5 [ always[0,2] holds(true) ] }";

/// Checks FR-196 "Behavior", SCH-2: an undefined path over every scheduler names
/// the post-state the scheduler took at a draw with several post-states, so
/// it replays to one state.
#[trace("TC-641", "FR-196-AC-12")]
#[test]
fn an_mdp_undefined_path_names_the_post_state_taken_at_each_draw() {
    let world = World::new(&prob_world::tossed_coin(TOSS));
    let ProbBuild::Undefined(path) = build_product(
        world.request("Tossed", ModelCheckLimits::default()),
        &Cancel::new(),
    )
    .expect("the build ends with a refutation") else {
        panic!("an undefined path");
    };
    let [step] = &path.steps[..] else {
        panic!("one step: {:?}", path.steps);
    };
    assert_eq!(path.position, 1);
    assert_eq!(step.probability, ratio(1, 2));
    let resolved = step.resolved.as_ref().expect("the post-state taken");
    // The coin is gone: a complete product key for the selected post-state,
    // and not the initial one.
    let plain = built(&world, "Plain");
    let initial = &plain.states[start(&plain).index()].model;
    assert_ne!(&resolved.model, initial);
    assert!(plain.states.iter().any(|key| key.model == resolved.model));
    assert_eq!(step.target, *resolved);
}
