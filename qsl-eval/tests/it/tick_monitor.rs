// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-251 (TC-706, TC-1081 step 4) and FR-160 (TC-199): the tick-trace
//! reference evaluator, over real checked clauses on a 1 MHz counter
//! (one tick is one microsecond, so one millisecond is 1000 ticks).

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_eval::temporal::{
    definitely_before, within_bound, TickBound, TickMonitorReference, TickMonitorSpec, TickOutcome,
    TickStep,
};
use qsl_semantics::check::{CheckedInterval, FormulaNode};
use quire_exact::{Integer, Rational};

use crate::timed_trace::timed;

fn ratio(n: i64, d: i64) -> Rational {
    Rational::new(Integer::from(n), Integer::from(d)).expect("nonzero")
}

struct Plan {
    fixture: crate::timed_trace::Fixture,
    uncertainty: Rational,
    width_bits: u32,
    max_gap: u64,
    capacity_rate: Option<Rational>,
}

impl Plan {
    fn new(body: &str, uncertainty: Rational) -> Self {
        Self {
            fixture: timed(body),
            uncertainty,
            width_bits: 20,
            max_gap: 100_000,
            capacity_rate: None,
        }
    }

    fn holds(&self) -> Vec<usize> {
        self.fixture
            .clause
            .formula()
            .nodes()
            .iter()
            .enumerate()
            .filter(|(_, node)| matches!(node, FormulaNode::Holds { .. }))
            .map(|(index, _)| index)
            .collect()
    }

    fn monitor(&self) -> TickMonitorReference<'_> {
        let mut bounds = BTreeMap::new();
        let mut capacities = BTreeMap::new();
        for operator in self.fixture.clause.operators() {
            if let Some(CheckedInterval::Timed(interval)) = &operator.interval {
                let scale = ratio(1000, 1);
                let lower = interval.lower().mul(&scale);
                let upper = interval.upper().mul(&scale);
                if let Some(rate) = &self.capacity_rate {
                    // FR-251 "Buffers": the upper bound plus the tick cover.
                    let window = upper
                        .add(&ratio(1, 1))
                        .add(&ratio(2, 1).mul(&self.uncertainty));
                    let _ = &lower;
                    let product = rate.mul(&window);
                    let cap = product.numerator().as_big().clone();
                    let den = product.denominator().as_big().clone();
                    let ceil = (cap + &den - 1) / den;
                    capacities.insert(operator.node.index(), u64::try_from(ceil).unwrap() + 1);
                }
                bounds.insert(
                    operator.node.index(),
                    TickBound {
                        lower,
                        lower_closed: interval.lower_closed(),
                        upper,
                        upper_closed: interval.upper_closed(),
                    },
                );
            }
        }
        TickMonitorReference::new(TickMonitorSpec {
            formula: self.fixture.clause.formula(),
            activation: self.fixture.clause.activation(),
            bounds,
            uncertainty: self.uncertainty.clone(),
            width_bits: self.width_bits,
            max_reading_gap: self.max_gap,
            capacities,
            activation_capacity: 8,
            unprojectable: Default::default(),
        })
    }
}

fn reading(reading: u64, truths: &[(usize, bool)]) -> TickStep {
    TickStep::Reading {
        reading,
        atoms: truths.iter().copied().collect(),
        guard: true,
    }
}

const DEADLINE: &str =
    "always (holds(c.level = 1) implies eventually[0 ms, 3 ms] holds(c.level = 2))";

fn deadline_outcome(uncertainty: Rational, ack_tick: u64, watermark: Option<u64>) -> TickOutcome {
    let plan = Plan::new(DEADLINE, uncertainty);
    let atoms = plan.holds();
    let (req, ack) = (atoms[0], atoms[1]);
    let mut monitor = plan.monitor();
    monitor
        .step(&reading(0, &[(req, true), (ack, false)]))
        .unwrap();
    monitor
        .step(&reading(ack_tick, &[(req, false), (ack, true)]))
        .unwrap();
    if let Some(boundary) = watermark {
        monitor.step(&TickStep::Watermark { boundary }).unwrap();
    }
    let eventually = plan
        .fixture
        .clause
        .operators()
        .iter()
        .find(|operator| operator.interval.is_some())
        .unwrap()
        .node
        .index();
    monitor.outcome(eventually, 0).unwrap()
}

#[trace("TC-706", "FR-251-AC-1")]
#[test]
fn deadline_decides_true_false_and_indeterminate() {
    assert_eq!(deadline_outcome(ratio(0, 1), 2998, None), TickOutcome::True);
    assert_eq!(
        deadline_outcome(ratio(0, 1), 3001, None),
        TickOutcome::False
    );
    assert_eq!(
        deadline_outcome(ratio(0, 1), 3000, None),
        TickOutcome::Indeterminate
    );
    assert_eq!(
        deadline_outcome(ratio(2, 1), 2998, None),
        TickOutcome::Indeterminate
    );
}

#[trace("TC-1081", "FR-251-AC-10")]
#[test]
fn watermark_settles_the_open_deadline_and_a_poll_does_not() {
    let plan = Plan::new(DEADLINE, ratio(0, 1));
    let req = plan.holds()[0];
    let ack = plan.holds()[1];
    let node = plan
        .fixture
        .clause
        .operators()
        .iter()
        .find(|operator| operator.interval.is_some())
        .unwrap()
        .node
        .index();
    let mut monitor = plan.monitor();
    monitor
        .step(&reading(0, &[(req, true), (ack, false)]))
        .unwrap();
    assert_eq!(monitor.outcome(node, 0), Some(TickOutcome::Pending));
    assert!(monitor
        .step(&TickStep::Poll { reading: 5000 })
        .unwrap()
        .is_empty());
    assert_eq!(monitor.outcome(node, 0), Some(TickOutcome::Pending));
    let settled = monitor
        .step(&TickStep::Watermark { boundary: 3001 })
        .unwrap();
    assert!(settled
        .iter()
        .any(|result| result.subformula == node && result.outcome == TickOutcome::False));
}

fn overflow_outcome(body: &str, events: u64, probe: Option<u64>) -> Option<TickOutcome> {
    let mut plan = Plan::new(body, ratio(0, 1));
    plan.capacity_rate = Some(ratio(2, 1000));
    let p = plan.holds()[0];
    let node = plan
        .fixture
        .clause
        .operators()
        .iter()
        .find(|operator| operator.interval.is_some())
        .unwrap()
        .node
        .index();
    let mut monitor = plan.monitor();
    let mut last = None;
    let mut ticks: Vec<u64> = (0..events).map(|event| event * 100).collect();
    ticks.extend(probe);
    for (event, tick) in ticks.into_iter().enumerate() {
        let results = monitor.step(&reading(tick, &[(p, false)])).unwrap();
        last = results
            .into_iter()
            .find(|result| result.subformula == node && result.position == event)
            .map(|result| result.outcome);
    }
    last
}

#[trace("TC-706", "FR-251-AC-2")]
#[test]
fn a_full_buffer_never_drops_an_event_and_settles_incomplete() {
    let body = "once[0 ms, 10 ms] holds(c.level = 2)";
    // Capacity 22: the 22nd event still fits, the 23rd overflows.
    assert_ne!(
        overflow_outcome(body, 22, None),
        Some(TickOutcome::Incomplete { capacity: 22 })
    );
    assert_eq!(
        overflow_outcome(body, 23, None),
        Some(TickOutcome::Incomplete { capacity: 22 })
    );
}

#[trace("TC-706", "FR-251-AC-13")]
#[test]
fn a_positive_lower_bound_keeps_the_upper_bound_capacity() {
    assert_eq!(
        overflow_outcome("once[5 ms, 10 ms] holds(c.level = 2)", 23, Some(7500)),
        Some(TickOutcome::Incomplete { capacity: 22 })
    );
}

#[trace("TC-1081", "FR-251-AC-15")]
#[test]
fn after_a_gap_history_is_gone_and_past_windows_are_indeterminate() {
    let mut plan = Plan::new("once[0 ms, 10 ms] holds(c.level = 2)", ratio(0, 1));
    plan.width_bits = 16;
    plan.max_gap = 2000;
    let p = plan.holds()[0];
    let node = plan
        .fixture
        .clause
        .operators()
        .iter()
        .find(|operator| operator.interval.is_some())
        .unwrap()
        .node
        .index();
    let mut monitor = plan.monitor();
    monitor.step(&reading(65530, &[(p, true)])).unwrap();
    // 65500 is a backwards reading: a gap fault, no event.
    monitor.step(&reading(65500, &[(p, false)])).unwrap();
    // Poll between readings leaves the baseline at 65500.
    monitor.step(&TickStep::Poll { reading: 1 }).unwrap();
    let results = monitor.step(&reading(65510, &[(p, false)])).unwrap();
    assert!(results
        .iter()
        .any(|result| result.subformula == node && result.outcome == TickOutcome::Indeterminate));
    // Once the window has refilled past the marker the result decides.
    let mut tick = 65510_u64;
    let mut last = TickOutcome::Indeterminate;
    // The window is ten milliseconds, ten thousand ticks of this fixture, so
    // a dozen readings a thousand ticks apart refill it past the marker.
    for _ in 0..12 {
        tick += 1000;
        // keep within 16 bits: a delta of 1000 wraps and stays in range.
        let results = monitor.step(&reading(tick % 65536, &[(p, false)])).unwrap();
        if let Some(result) = results.iter().find(|result| result.subformula == node) {
            last = result.outcome;
        }
    }
    // Every reading after the gap is false, so once the window has refilled
    // the operator decides false: the marker no longer covers it.
    assert_eq!(last, TickOutcome::False);
}

#[trace("TC-706", "FR-251-AC-16")]
#[test]
fn two_events_in_one_tick_are_neither_ordered_nor_clamped() {
    let plan = Plan::new("eventually[0 ms, 3 ms] holds(c.level = 2)", ratio(0, 1));
    let p = plan.holds()[0];
    let node = plan.fixture.clause.formula().root().index();
    let mut monitor = plan.monitor();
    monitor.step(&reading(7, &[(p, false)])).unwrap();
    monitor.step(&reading(7, &[(p, true)])).unwrap();
    // The offset interval is (-1, 1): not inside [0, 3000], not outside it.
    assert_eq!(monitor.outcome(node, 0), Some(TickOutcome::Indeterminate));
}

#[trace("TC-706", "FR-251-AC-3")]
#[test]
fn counter_wraps_and_a_gap_fails_the_open_instance() {
    let mut plan = Plan::new(DEADLINE, ratio(0, 1));
    plan.width_bits = 16;
    plan.max_gap = 100;
    let atoms = plan.holds();
    let (req, ack) = (atoms[0], atoms[1]);
    let mut monitor = plan.monitor();
    monitor
        .step(&reading(65530, &[(req, true), (ack, false)]))
        .unwrap();
    let node = plan
        .fixture
        .clause
        .operators()
        .iter()
        .find(|operator| operator.interval.is_some())
        .unwrap()
        .node
        .index();
    let results = monitor
        .step(&reading(65500, &[(req, false), (ack, false)]))
        .unwrap();
    assert!(results.iter().any(|result| result.subformula == node
        && result.outcome
            == TickOutcome::Failed {
                gap: 65506,
                max_reading_gap: 100
            }));
    // A wrapped difference of 10 is an ordinary event.
    let mut monitor = plan.monitor();
    monitor
        .step(&reading(65530, &[(req, false), (ack, false)]))
        .unwrap();
    monitor
        .step(&reading(4, &[(req, false), (ack, false)]))
        .unwrap();
    assert_eq!(monitor.now(), 10);
}

#[trace("TC-199", "FR-160-AC-7")]
#[test]
fn tick_cover_vectors_decide_by_the_endpoint_rule() {
    let path = std::path::Path::new(&std::env::var("QSPEC_DIR").expect("QSPEC_DIR"))
        .join("proposals/checked-package-v2/tick-cover-vectors.json");
    let vectors: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    for case in vectors["cases"].as_array().unwrap() {
        let u = ratio(case["uncertainty"].as_i64().unwrap(), 1);
        match case["kind"].as_str().unwrap() {
            "within_bound" => {
                let got = within_bound(
                    i128::from(case["bound"].as_i64().unwrap()),
                    &u,
                    i128::from(case["request_tick"].as_i64().unwrap()),
                    i128::from(case["ack_tick"].as_i64().unwrap()),
                );
                let want = match case["expected"].as_str().unwrap() {
                    "true" => Some(true),
                    "false" => Some(false),
                    _ => None,
                };
                assert_eq!(got, want, "{}", case["id"]);
            }
            _ => assert_eq!(
                definitely_before(
                    &u,
                    i128::from(case["a_tick"].as_i64().unwrap()),
                    i128::from(case["b_tick"].as_i64().unwrap())
                ),
                case["expected"].as_bool().unwrap(),
                "{}",
                case["id"]
            ),
        }
    }
}

fn root_outcome(body: &str, steps: &[Vec<(usize, bool)>], ticks: &[u64]) -> TickOutcome {
    let plan = Plan::new(body, ratio(0, 1));
    let atoms = plan.holds();
    let mut monitor = plan.monitor();
    for (truths, tick) in steps.iter().zip(ticks) {
        let named: Vec<(usize, bool)> = truths
            .iter()
            .map(|(slot, value)| (atoms[*slot], *value))
            .collect();
        monitor.step(&reading(*tick, &named)).unwrap();
    }
    monitor
        .outcome(plan.fixture.clause.formula().root().index(), 0)
        .unwrap()
}

#[trace("TC-706", "FR-251")]
#[test]
fn open_and_closed_lower_endpoints_differ_at_the_event_itself() {
    // The reading is its own witness at offset exactly 0: a closed lower
    // end contains it, an open one does not (pending for a later witness).
    let step = vec![vec![(0, true)]];
    assert_eq!(
        root_outcome("eventually[0 ms, 3 ms] holds(c.level = 2)", &step, &[0]),
        TickOutcome::True
    );
    assert_eq!(
        root_outcome("eventually(0 ms, 3 ms] holds(c.level = 2)", &step, &[0]),
        TickOutcome::Pending
    );
}

#[trace("TC-706", "FR-251")]
#[test]
fn the_reversed_implication_settles_by_its_own_operands() {
    // `eventually ack implies req`: ack comes true at tick 2998 inside the
    // window of position 0, req is false, so position 0's implication is
    // false and the always-claim is false.
    let outcome = root_outcome(
        "always (eventually[0 ms, 3 ms] holds(c.level = 2) implies holds(c.level = 1))",
        &[vec![(0, false), (1, false)], vec![(0, true), (1, false)]],
        &[0, 2998],
    );
    assert_eq!(outcome, TickOutcome::False);
    // With req true at position 0 and nothing violated the claim stays
    // pending: an always never decides true on a prefix.
    let outcome = root_outcome(
        "always (eventually[0 ms, 3 ms] holds(c.level = 2) implies holds(c.level = 1))",
        &[vec![(0, false), (1, true)]],
        &[0],
    );
    assert_eq!(outcome, TickOutcome::Pending);
}
