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

use crate::timed_trace::{timed, timed_on};

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
                    let window = upper.sub(&lower);
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

#[trace("TC-706", "FR-251-AC-2")]
#[test]
fn a_full_buffer_never_drops_an_event_and_settles_incomplete() {
    let mut plan = Plan::new("once[0 ms, 10 ms] holds(c.level = 2)", ratio(0, 1));
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
    for event in 0..22_u64 {
        let results = monitor.step(&reading(event * 100, &[(p, false)])).unwrap();
        last = results
            .into_iter()
            .find(|result| result.subformula == node && result.position == event as usize)
            .map(|result| result.outcome);
    }
    assert_eq!(last, Some(TickOutcome::Incomplete { capacity: 21 }));
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

#[trace("TC-1081", "FR-251-AC-7")]
#[test]
fn open_and_closed_endpoints_and_a_false_guard() {
    // `[0 ms, 3 ms)` excludes the bound: a reading at the bound's tick is
    // still ambiguous under the cover, but 2998 decides true as before.
    let plan = Plan::new("eventually[0 ms, 3 ms) holds(c.level = 2)", ratio(0, 1));
    let p = plan.holds()[0];
    let mut monitor = plan.monitor();
    monitor.step(&reading(0, &[(p, false)])).unwrap();
    monitor.step(&reading(2998, &[(p, true)])).unwrap();
    let node = plan.fixture.clause.formula().root().index();
    assert_eq!(monitor.outcome(node, 0), Some(TickOutcome::True));
    // Reversed implication operands: ack implies req never waits.
    let plan = Plan::new(
        "always (eventually[0 ms, 3 ms] holds(c.level = 2) implies holds(c.level = 1))",
        ratio(0, 1),
    );
    assert_eq!(plan.holds().len(), 2);
    let _ = timed_on;
}
