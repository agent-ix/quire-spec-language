// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-201 exact-value certificate types and the independent Bellman checker
//! core. Model re-enumeration and the public replay facade are separate work.
//!
//! The low-level graph evaluator is retained here until the replay facade
//! supplies its independently enumerated graph; it is exercised by this
//! module's tests and is not wired to the analyzer's product.

#![allow(dead_code)]

use qsl_semantics::check::Bound;
use quire_exact::{Integer, Rational};
use std::collections::{BTreeMap, BTreeSet};

/// A complete canonical product-state key, encoded by the checker.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ProbabilityProductKey(Vec<u8>);

impl ProbabilityProductKey {
    /// Construct a key from its canonical encoding.
    pub fn from_canonical_bytes(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// The canonical product-state encoding.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// The exact rational value certified for one probabilistic product state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactProbabilityValue {
    /// The complete canonical product-state key.
    pub state: ProbabilityProductKey,
    /// The exact value at that state.
    pub value: Rational,
}

/// An exact-value certificate's state values (FR-201).
///
/// This certificate form does not itself establish graph completeness. The
/// replay facade must re-enumerate the subject and pass that checker-owned
/// graph to the independent checker before accepting the certificate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactProbabilityCertificate {
    /// One value for every state in the checker-owned graph, in key order.
    pub values: Vec<ExactProbabilityValue>,
}

/// The scheduler objective for an exact reachability certificate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProbabilityObjective {
    /// The minimum over all enabled actions.
    Minimum,
    /// The maximum over all enabled actions.
    Maximum,
    /// A workload with exactly one enabled probability distribution.
    Workload,
}

/// Whether the certified initial value satisfies the claim's probability bound.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProbabilityBoundSide {
    /// The value satisfies an authored `>=` bound.
    AtLeast,
    /// The value satisfies an authored `<=` bound.
    AtMost,
}

/// An exact certificate rejection and the first canonical state at fault.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProbabilityCertificateRejection {
    /// The failed FR-201 condition.
    pub rule: ProbabilityCertificateRule,
    /// The state or member at which it failed.
    pub at: ProbabilityCertificateLocus,
}

/// The exact-certificate rejection rules implemented by this checker core.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProbabilityCertificateRule {
    /// A certificate value does not equal the recomputed Bellman value.
    InequalityFails,
    /// A structurally valid certificate's initial value misses the claim bound.
    ThresholdMissed,
    /// A certificate omits, duplicates, or adds a state value.
    Malformed,
}

/// The locus of an exact-certificate rejection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProbabilityCertificateLocus {
    /// A full probabilistic product state key.
    ProductState(ProbabilityProductKey),
    /// A malformed certificate member.
    Member(&'static str),
}

/// The result of checking an exact-value certificate against a graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExactProbabilityCertificateCheck {
    /// Every value matches the independently evaluated graph and the initial
    /// value satisfies the claim bound.
    Accepted {
        /// The checked initial value.
        value: Rational,
        /// The authored bound side that was checked.
        side: ProbabilityBoundSide,
    },
    /// The certificate fails a checked condition.
    Rejected(ProbabilityCertificateRejection),
}

/// Why the exact Bellman-check core could not check a graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExactProbabilityGraphError {
    /// A listed successor has no state in the checker graph.
    MissingSuccessor,
    /// The graph's edge count exceeds the representable in-memory state count.
    TooManyEdges,
    /// The graph is cyclic; cyclic certificates require the later
    /// unbounded-certificate checking path.
    Cycle,
}

/// One successor and its exact probability mass.
#[derive(Clone, Debug)]
struct ProbabilityEdge {
    target: ProbabilityProductKey,
    probability: Rational,
}

/// One action's probabilistic successors.
type ProbabilityChoice = Vec<ProbabilityEdge>;

/// One checker-enumerated state. This type is private so an eventual replay
/// facade owns graph construction instead of trusting solver data.
#[derive(Clone, Debug)]
struct CheckerState {
    terminal: Option<bool>,
    choices: Vec<ProbabilityChoice>,
}

/// The graph produced by a checker-side enumerator.
#[derive(Clone, Debug)]
struct CheckerGraph {
    initial: ProbabilityProductKey,
    states: BTreeMap<ProbabilityProductKey, CheckerState>,
}

/// Check exact reachability values on a checker-owned acyclic graph.
///
/// The graph representation and this evaluator are independent of
/// `qsl-analyze`; this core performs no policy iteration or linear algebra.
/// Cyclic graphs are rejected until FR-201's unbounded graph checks are
/// implemented.
fn check_exact_reachability(
    graph: &CheckerGraph,
    certificate: &ExactProbabilityCertificate,
    objective: ProbabilityObjective,
    bound: &Bound,
) -> Result<ExactProbabilityCertificateCheck, ExactProbabilityGraphError> {
    let values = match certificate_values(graph, certificate) {
        Ok(values) => values,
        Err(rejection) => return Ok(ExactProbabilityCertificateCheck::Rejected(rejection)),
    };
    let order = topological_order(graph)?;

    let zero = rational(0);
    let one = rational(1);
    let mut computed = BTreeMap::new();
    for key in order.into_iter().rev() {
        let Some(state) = graph.states.get(&key) else {
            return Ok(rejected(ProbabilityCertificateRule::InequalityFails, key));
        };
        let actual = if let Some(target) = state.terminal {
            if target {
                one.clone()
            } else {
                zero.clone()
            }
        } else {
            if state.choices.is_empty()
                || (objective == ProbabilityObjective::Workload && state.choices.len() != 1)
            {
                return Ok(rejected(ProbabilityCertificateRule::InequalityFails, key));
            }
            let mut choice_values = Vec::with_capacity(state.choices.len());
            for choice in &state.choices {
                let mut mass = zero.clone();
                let mut value = zero.clone();
                for edge in choice {
                    if edge.probability < zero {
                        return Ok(rejected(
                            ProbabilityCertificateRule::InequalityFails,
                            key.clone(),
                        ));
                    }
                    if edge.probability == zero {
                        continue;
                    }
                    let Some(successor) = computed.get(&edge.target) else {
                        return Ok(rejected(
                            ProbabilityCertificateRule::InequalityFails,
                            key.clone(),
                        ));
                    };
                    mass = mass.add(&edge.probability);
                    value = value.add(&edge.probability.mul(successor));
                }
                if mass != one {
                    return Ok(rejected(
                        ProbabilityCertificateRule::InequalityFails,
                        key.clone(),
                    ));
                }
                choice_values.push(value);
            }
            match objective {
                ProbabilityObjective::Minimum => choice_values.into_iter().min(),
                ProbabilityObjective::Maximum => choice_values.into_iter().max(),
                ProbabilityObjective::Workload => choice_values.into_iter().next(),
            }
            .unwrap_or_else(|| zero.clone())
        };
        let Some(certified) = values.get(&key) else {
            return Ok(rejected(ProbabilityCertificateRule::Malformed, key));
        };
        if actual != *certified {
            return Ok(rejected(ProbabilityCertificateRule::InequalityFails, key));
        }
        computed.insert(key, actual);
    }

    let Some(initial) = computed.get(&graph.initial) else {
        return Ok(rejected(
            ProbabilityCertificateRule::InequalityFails,
            graph.initial.clone(),
        ));
    };
    let meets_bound = match bound {
        Bound::AtLeast(threshold) => initial >= threshold,
        Bound::AtMost(threshold) => initial <= threshold,
    };
    if !meets_bound {
        return Ok(rejected(
            ProbabilityCertificateRule::ThresholdMissed,
            graph.initial.clone(),
        ));
    }
    Ok(ExactProbabilityCertificateCheck::Accepted {
        value: initial.clone(),
        side: match bound {
            Bound::AtLeast(_) => ProbabilityBoundSide::AtLeast,
            Bound::AtMost(_) => ProbabilityBoundSide::AtMost,
        },
    })
}

fn certificate_values(
    graph: &CheckerGraph,
    certificate: &ExactProbabilityCertificate,
) -> Result<BTreeMap<ProbabilityProductKey, Rational>, ProbabilityCertificateRejection> {
    let mut values = BTreeMap::new();
    let mut previous = None;
    for entry in &certificate.values {
        if previous.as_ref().is_some_and(|key| key >= &entry.state) {
            return Err(ProbabilityCertificateRejection {
                rule: ProbabilityCertificateRule::Malformed,
                at: ProbabilityCertificateLocus::ProductState(entry.state.clone()),
            });
        }
        previous = Some(entry.state.clone());
        if values
            .insert(entry.state.clone(), entry.value.clone())
            .is_some()
            || !graph.states.contains_key(&entry.state)
        {
            return Err(ProbabilityCertificateRejection {
                rule: ProbabilityCertificateRule::Malformed,
                at: ProbabilityCertificateLocus::ProductState(entry.state.clone()),
            });
        }
    }
    if values.len() != graph.states.len() {
        let missing = graph
            .states
            .keys()
            .find(|key| !values.contains_key(*key))
            .cloned()
            .unwrap_or_else(|| graph.initial.clone());
        return Err(ProbabilityCertificateRejection {
            rule: ProbabilityCertificateRule::Malformed,
            at: ProbabilityCertificateLocus::ProductState(missing),
        });
    }
    Ok(values)
}

fn topological_order(
    graph: &CheckerGraph,
) -> Result<Vec<ProbabilityProductKey>, ExactProbabilityGraphError> {
    let zero = rational(0);
    let mut indegree = graph
        .states
        .keys()
        .cloned()
        .map(|key| (key, 0_usize))
        .collect::<BTreeMap<_, _>>();
    for state in graph.states.values() {
        for edge in state.choices.iter().flatten() {
            if edge.probability == zero {
                continue;
            }
            let Some(count) = indegree.get_mut(&edge.target) else {
                return Err(ExactProbabilityGraphError::MissingSuccessor);
            };
            *count = count
                .checked_add(1)
                .ok_or(ExactProbabilityGraphError::TooManyEdges)?;
        }
    }
    let mut ready = indegree
        .iter()
        .filter_map(|(key, degree)| (*degree == 0).then_some(key.clone()))
        .collect::<BTreeSet<_>>();
    let mut order = Vec::with_capacity(graph.states.len());
    while let Some(key) = ready.pop_first() {
        order.push(key.clone());
        let Some(state) = graph.states.get(&key) else {
            return Err(ExactProbabilityGraphError::MissingSuccessor);
        };
        for edge in state.choices.iter().flatten() {
            if edge.probability == zero {
                continue;
            }
            let Some(count) = indegree.get_mut(&edge.target) else {
                return Err(ExactProbabilityGraphError::MissingSuccessor);
            };
            let Some(remaining) = count.checked_sub(1) else {
                return Err(ExactProbabilityGraphError::MissingSuccessor);
            };
            *count = remaining;
            if *count == 0 {
                ready.insert(edge.target.clone());
            }
        }
    }
    if order.len() == graph.states.len() {
        Ok(order)
    } else {
        Err(ExactProbabilityGraphError::Cycle)
    }
}

fn rejected(
    rule: ProbabilityCertificateRule,
    state: ProbabilityProductKey,
) -> ExactProbabilityCertificateCheck {
    ExactProbabilityCertificateCheck::Rejected(ProbabilityCertificateRejection {
        rule,
        at: ProbabilityCertificateLocus::ProductState(state),
    })
}

fn rational(value: i64) -> Rational {
    Rational::from_integer(Integer::from(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn key(value: u8) -> ProbabilityProductKey {
        ProbabilityProductKey(vec![value])
    }

    fn edge(target: u8, numerator: i64, denominator: i64) -> ProbabilityEdge {
        ProbabilityEdge {
            target: key(target),
            probability: Rational::new(Integer::from(numerator), Integer::from(denominator))
                .expect("nonzero denominator"),
        }
    }

    fn graph() -> CheckerGraph {
        let initial = key(0);
        let mut states = BTreeMap::new();
        states.insert(
            initial.clone(),
            CheckerState {
                terminal: None,
                choices: vec![
                    vec![edge(1, 4, 5), edge(2, 1, 5)],
                    vec![edge(1, 9, 10), edge(2, 1, 10)],
                ],
            },
        );
        states.insert(
            key(1),
            CheckerState {
                terminal: Some(true),
                choices: Vec::new(),
            },
        );
        states.insert(
            key(2),
            CheckerState {
                terminal: Some(false),
                choices: Vec::new(),
            },
        );
        CheckerGraph { initial, states }
    }

    fn certificate(initial_value: Rational) -> ExactProbabilityCertificate {
        ExactProbabilityCertificate {
            values: vec![
                ExactProbabilityValue {
                    state: key(0),
                    value: initial_value,
                },
                ExactProbabilityValue {
                    state: key(1),
                    value: rational(1),
                },
                ExactProbabilityValue {
                    state: key(2),
                    value: rational(0),
                },
            ],
        }
    }

    #[trace("FR-201-AC-6")]
    #[test]
    fn exact_certificate_checks_each_state_and_reports_first_bad_key() {
        let graph = graph();
        let result = check_exact_reachability(
            &graph,
            &certificate(Rational::new(Integer::from(4_i64), Integer::from(5_i64)).unwrap()),
            ProbabilityObjective::Minimum,
            &Bound::AtLeast(Rational::new(Integer::from(3_i64), Integer::from(4_i64)).unwrap()),
        )
        .expect("the checker graph is acyclic and closed");
        assert_eq!(
            result,
            ExactProbabilityCertificateCheck::Accepted {
                value: Rational::new(Integer::from(4_i64), Integer::from(5_i64)).unwrap(),
                side: ProbabilityBoundSide::AtLeast,
            }
        );

        let rejected_result = check_exact_reachability(
            &graph,
            &certificate(Rational::new(Integer::from(9_i64), Integer::from(10_i64)).unwrap()),
            ProbabilityObjective::Minimum,
            &Bound::AtLeast(Rational::new(Integer::from(3_i64), Integer::from(4_i64)).unwrap()),
        )
        .expect("the checker graph is acyclic and closed");
        assert_eq!(
            rejected_result,
            rejected(ProbabilityCertificateRule::InequalityFails, key(0))
        );
    }

    #[trace("FR-201-AC-6")]
    #[test]
    fn exact_certificate_rejects_threshold_miss_and_cycles() {
        let graph = graph();
        let missed = check_exact_reachability(
            &graph,
            &certificate(Rational::new(Integer::from(4_i64), Integer::from(5_i64)).unwrap()),
            ProbabilityObjective::Minimum,
            &Bound::AtLeast(Rational::new(Integer::from(9_i64), Integer::from(10_i64)).unwrap()),
        )
        .expect("the checker graph is acyclic and closed");
        assert_eq!(
            missed,
            rejected(ProbabilityCertificateRule::ThresholdMissed, key(0))
        );

        let mut cyclic = graph;
        let state = cyclic.states.get_mut(&key(2)).unwrap();
        state.terminal = None;
        state.choices = vec![vec![edge(2, 1, 1)]];
        assert_eq!(
            check_exact_reachability(
                &cyclic,
                &certificate(Rational::new(Integer::from(4_i64), Integer::from(5_i64)).unwrap()),
                ProbabilityObjective::Workload,
                &Bound::AtLeast(rational(0)),
            ),
            Err(ExactProbabilityGraphError::Cycle)
        );
    }
}
