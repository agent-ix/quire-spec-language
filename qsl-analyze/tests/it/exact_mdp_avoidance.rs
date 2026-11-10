// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact qualitative avoidance on unrestricted finite MDP products.

use ix_trace_rs::trace;
use qsl_analyze::exact_probabilistic::{
    build_product, decide_expected_reward, decide_reachability, decide_reachability_threshold,
    ExactProbLimits, ExtRational, MonitorVerdict, ProbBuild, ProbEdge, ProbKey, ProbProduct,
    ProbStateId, ProductAction, ProductKind, UnboundedError, UnboundedObjective, UnboundedOutcome,
};
use quire_exact::{Cancel, Identifier, Integer, Rational, Value};

fn ratio(n: i64, d: i64) -> Rational {
    Rational::new(Integer::from(n), Integer::from(d)).expect("nonzero denominator")
}

fn key(id: u8) -> ProbKey {
    ProbKey {
        binding: None,
        model: vec![id],
        monitor: None,
        accumulator: None,
        pending: None,
    }
}

fn edge(from: usize, to: usize, n: i64, d: i64, action: u8) -> ProbEdge {
    ProbEdge {
        from: ProbStateId(from),
        choice: ProductAction::PostState(Box::new(key(action))),
        action: None,
        draw: None,
        to: ProbStateId(to),
        probability: ratio(n, d),
        rewards: vec![(
            Identifier::new("cost").expect("identifier"),
            Value::Integer(Integer::zero()),
        )],
    }
}

fn product(mut edges: Vec<ProbEdge>) -> ProbProduct {
    edges.sort_by_key(|edge| edge.from);
    ProbProduct {
        states: (0..3).map(key).collect(),
        kind: ProductKind::Mdp,
        edges,
        initial: vec![(0, None, ProbStateId(0))],
        decided: vec![(ProbStateId(2), MonitorVerdict::Reject)],
        labels: Vec::new(),
        fairness_operations: Vec::new(),
        fairness: Vec::new(),
    }
}

fn two_actions() -> ProbProduct {
    product(vec![
        edge(0, 0, 1, 3, 10),
        edge(0, 1, 1, 3, 10),
        edge(0, 2, 1, 3, 10),
        edge(0, 0, 1, 4, 11),
        edge(0, 1, 1, 2, 11),
        edge(0, 2, 1, 4, 11),
        edge(1, 1, 1, 1, 12),
    ])
}

fn value(product: &ProbProduct, objective: UnboundedObjective) -> ExtRational {
    decide_reachability(
        product,
        &[false, true, false],
        objective,
        &ExactProbLimits::default(),
    )
    .expect("normalized finite MDP")
    .values[0]
        .clone()
}

#[trace("TC-633", "FR-198-AC-3")]
#[test]
fn two_actions_with_cyclic_positive_failure_keep_fractional_extrema() {
    let product = two_actions();
    assert_ne!(product.edges[0].choice, product.edges[3].choice);
    assert_eq!(
        value(&product, UnboundedObjective::Min),
        ExtRational::Finite(ratio(1, 2))
    );
    assert_eq!(
        value(&product, UnboundedObjective::Max),
        ExtRational::Finite(ratio(2, 3))
    );
    let limits = ExactProbLimits {
        max_iterations: 3,
        ..ExactProbLimits::default()
    };
    let outcome = decide_reachability_threshold(
        &product,
        &[false, true, false],
        UnboundedObjective::Min,
        &ratio(1, 2),
        &limits,
    )
    .expect("equality falls back to exact policy iteration");
    let UnboundedOutcome::Exact(result) = outcome else {
        panic!("expected exact equality: {outcome:?}")
    };
    assert_eq!(result.values[0], ExtRational::Finite(ratio(1, 2)));
}

#[trace("TC-633", "FR-198-AC-4")]
#[test]
fn fractional_minimum_observes_policy_and_arithmetic_budgets() {
    let product = two_actions();
    let limits = ExactProbLimits {
        max_iterations: 3,
        max_policy_iterations: 0,
        ..ExactProbLimits::default()
    };
    assert_eq!(
        decide_reachability_threshold(
            &product,
            &[false, true, false],
            UnboundedObjective::Min,
            &ratio(1, 2),
            &limits
        ),
        Ok(UnboundedOutcome::Stopped(
            qsl_replay::ExactProbLimit::MaxPolicyIterations
        ))
    );
    let limits = ExactProbLimits {
        max_rational_bits: 0,
        ..ExactProbLimits::default()
    };
    assert_eq!(
        decide_reachability(
            &product,
            &[false, true, false],
            UnboundedObjective::Min,
            &limits
        ),
        Err(UnboundedError::MaxRationalBits)
    );
}

#[trace("TC-633", "FR-198-AC-1")]
#[trace("TC-633", "FR-198-AC-3")]
#[test]
fn avoidance_controls_preserve_target_absorption_and_scheduler_choices() {
    let cases = [
        // Failure is reachable only after visiting the target.
        (
            vec![edge(0, 1, 1, 1, 10), edge(1, 2, 1, 1, 11)],
            ratio(1, 1),
            ratio(1, 1),
        ),
        // A target leak without a positive failure edge reaches almost surely.
        (
            vec![
                edge(0, 0, 1, 2, 10),
                edge(0, 1, 1, 2, 10),
                edge(0, 2, 0, 1, 10),
                edge(1, 1, 1, 1, 11),
            ],
            ratio(1, 1),
            ratio(1, 1),
        ),
        // Safe looping and reaching the target are distinct scheduler actions.
        (
            vec![
                edge(0, 0, 1, 1, 10),
                edge(0, 1, 1, 1, 11),
                edge(1, 1, 1, 1, 12),
            ],
            ratio(0, 1),
            ratio(1, 1),
        ),
    ];
    for (edges, minimum, maximum) in cases {
        let product = product(edges);
        assert_eq!(
            value(&product, UnboundedObjective::Min),
            ExtRational::Finite(minimum)
        );
        assert_eq!(
            value(&product, UnboundedObjective::Max),
            ExtRational::Finite(maximum)
        );
    }
}

#[trace("TC-633", "FR-198-AC-3")]
#[test]
fn acyclic_failure_sibling_does_not_require_other_successors_to_be_targets() {
    let mut product = product(vec![
        edge(0, 2, 1, 3, 10),
        edge(0, 3, 2, 3, 10),
        edge(3, 1, 1, 1, 11),
        edge(1, 1, 1, 1, 12),
    ]);
    product.states.push(key(3));
    let result = decide_reachability(
        &product,
        &[false, true, false, false],
        UnboundedObjective::Min,
        &ExactProbLimits::default(),
    )
    .expect("finite acyclic prefix");
    assert_eq!(result.values[0], ExtRational::Finite(ratio(2, 3)));
}

#[trace("TC-633", "FR-198-AC-2")]
#[test]
fn positive_nonreach_has_infinite_expected_until_even_with_zero_costs() {
    let mut product = two_actions();
    for objective in [UnboundedObjective::Min, UnboundedObjective::Max] {
        let result = decide_expected_reward(
            &product,
            &[false, true, false],
            "cost",
            objective,
            &ExactProbLimits::default(),
        )
        .expect("nonreach is decided qualitatively");
        assert_eq!(result.values[0], ExtRational::PlusInfinity);
        assert_eq!(result.values[1], ExtRational::Finite(ratio(0, 1)));
    }
    product.edges.push(edge(0, 1, 1, 1, 13));
    product.edges.sort_by_key(|edge| edge.from);
    let result = decide_expected_reward(
        &product,
        &[false, true, false],
        "cost",
        UnboundedObjective::Min,
        &ExactProbLimits::default(),
    )
    .expect("a proper zero-cost policy exists");
    assert_eq!(result.values[0], ExtRational::Finite(ratio(0, 1)));
    let result = decide_expected_reward(
        &product,
        &[false, true, false],
        "cost",
        UnboundedObjective::Max,
        &ExactProbLimits::default(),
    )
    .expect("the avoiding policy remains available");
    assert_eq!(result.values[0], ExtRational::PlusInfinity);
}

#[trace("TC-631", "FR-196-AC-4")]
#[trace("TC-633", "FR-198-AC-1")]
#[test]
fn pending_post_state_resolutions_remain_scheduler_choices_after_the_draw() {
    let world = crate::prob_world::World::new(&crate::prob_world::health(
        "self.healthy = (pre(self.healthy) and not fault)",
        "true: 1, false: 1",
        true,
        "probabilistic Anyhow using ev on M over (n: M::Node) under every scheduler \
         on origin { probability >= 0.99 [ always[0,2] holds(n.healthy) ] }",
    ));
    let ProbBuild::Built(product) = build_product(
        world.request("Anyhow", qsl_replay::ModelCheckLimits::default()),
        &Cancel::new(),
    )
    .expect("checked finite product") else {
        panic!("expected complete product")
    };
    let pending = product
        .ids()
        .find(|state| product.states[state.index()].pending.is_some())
        .expect("draw with unresolved post-state alternatives");
    let [first, second] = product.outgoing(pending) else {
        panic!("two post-state choices")
    };
    assert!(matches!(first.choice, ProductAction::PostState(_)));
    assert!(matches!(second.choice, ProductAction::PostState(_)));
    assert_ne!(first.choice, second.choice);
    assert_eq!(first.probability, ratio(1, 1));
    assert_eq!(second.probability, ratio(1, 1));
    let mut target = vec![false; product.states.len()];
    target[first.to.index()] = true;
    for (objective, expected) in [
        (UnboundedObjective::Min, ratio(0, 1)),
        (UnboundedObjective::Max, ratio(1, 1)),
    ] {
        let result = decide_reachability(&product, &target, objective, &ExactProbLimits::default())
            .expect("post-draw alternatives are independent scheduler actions");
        assert_eq!(
            result.values[pending.index()],
            ExtRational::Finite(expected)
        );
    }
}
