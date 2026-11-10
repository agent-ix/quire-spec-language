// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-201 closed cyclic DTMC admission and checker-owned support analysis.

use super::*;

/// Check original subject acyclicity before constructing native unbounded
/// refutation paths. This grants admission only and emits no document or
/// product keys, so certificate transport-byte limits do not apply.
pub fn admit_acyclic_probabilistic_refutation_subject(
    system: &ModelSystem<'_>,
    limits: &crate::ExactProbLimits,
    cancel: &Cancel,
) -> Result<bool, ProbabilityDocumentError> {
    native_with_identity(
        system,
        limits,
        cancel,
        qsl_foundation::IdentityLimits::default(),
    )
}

/// Qualify the real native preflight under an explicit owner identity ceiling.
#[cfg(feature = "test-support")]
pub fn admit_acyclic_probabilistic_refutation_subject_with_identity(
    system: &ModelSystem<'_>,
    limits: &crate::ExactProbLimits,
    cancel: &Cancel,
    identity: qsl_foundation::IdentityLimits,
) -> Result<bool, ProbabilityDocumentError> {
    native_with_identity(system, limits, cancel, identity)
}

fn native_with_identity(
    system: &ModelSystem<'_>,
    limits: &crate::ExactProbLimits,
    cancel: &Cancel,
    identity: qsl_foundation::IdentityLimits,
) -> Result<bool, ProbabilityDocumentError> {
    let mut meter = quire_exact::Meter::new(system.meter_limits()).with_cancel(cancel.clone());
    let mut storage = 0;
    let graph_limits = ProbabilityDocumentLimits {
        bytes: 0,
        storage_bytes: limits.document.storage_bytes,
        states: limits.model.limits.max_states,
        edges: limits.model.limits.max_transitions,
        scalar_bits: limits.max_rational_bits,
        accounting: system.meter_limits(),
    };
    let graph = match enumerate_graph(
        system,
        None::<&qsl_semantics::check::CheckedPredicate>,
        None,
        None,
        graph_limits,
        cancel,
        identity,
        &mut meter,
        &mut storage,
        false,
        false,
    ) {
        Ok(graph) => graph,
        Err(ProbabilityDocumentError::Unsupported) => return Ok(false),
        Err(error) => return Err(error),
    };
    for (key, state) in &graph.states {
        visit(cancel, &mut meter)?;
        charge(
            &mut storage,
            graph_limits.storage_bytes,
            key.as_bytes()
                .len()
                .saturating_mul(3)
                .saturating_add(std::mem::size_of::<ProbabilityProductKey>().saturating_mul(3)),
        )?;
        for edge in state.choices.iter().flatten() {
            visit(cancel, &mut meter)?;
            document::check_scalar(&edge.probability, graph_limits)?;
        }
    }
    match topological_order(&graph, cancel) {
        Ok(_) => Ok(true),
        Err(ExactProbabilityGraphError::Cycle) => Ok(false),
        Err(error) => Err(error.into()),
    }
}

/// Rebuild a graph using one caller scope for raw identities, complete keys,
/// semantic candidate/post keys and retained graph storage.
#[allow(clippy::too_many_arguments)]
pub(super) fn enumerate_graph<P: StatePredicate>(
    system: &ModelSystem<'_>,
    target: Option<&P>,
    binding: Option<&ObjectReference>,
    workload: Option<&Workload>,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    identity: qsl_foundation::IdentityLimits,
    meter: &mut quire_exact::Meter,
    storage: &mut u64,
    stop_at_target: bool,
    full_key: bool,
) -> Result<CheckerGraph, ProbabilityDocumentError> {
    let scope = std::cell::RefCell::new((meter, storage));
    let stopped = std::cell::RefCell::new(None);
    let mut issue = None;
    let key_of = |state: &ModelSimState| {
        let mut borrowed = scope.borrow_mut();
        let (meter, storage) = &mut *borrowed;
        let mut admission =
            ProbabilityKeyAdmission::new(meter, storage, limits.storage_bytes, cancel);
        let encoded = (|| {
            let bytes =
                system.state_key_bytes_with_budget(state, identity, &mut admission, cancel)?;
            if full_key {
                ProbabilityProductKey::for_model_state_with_budget(
                    binding,
                    &bytes,
                    identity,
                    &mut admission,
                    cancel,
                )
            } else {
                // Private scratch equality labels never escape as FR-196 evidence.
                Ok(ProbabilityProductKey::from_canonical_bytes(bytes))
            }
        })();
        encoded.map_err(|error| {
            *stopped.borrow_mut() = Some(admission.refusal(error));
            ExactProbabilityGraphError::ExpansionRefused
        })
    };
    let result = enumerate_model_graph_visiting(
        system,
        target,
        binding,
        limits.states,
        limits.edges,
        cancel,
        stop_at_target,
        &key_of,
        &mut |bytes| {
            let mut borrowed = scope.borrow_mut();
            let (meter, storage) = &mut *borrowed;
            let mut admission =
                ProbabilityKeyAdmission::new(meter, storage, limits.storage_bytes, cancel);
            quire_exact::Reservation::reserve(&mut admission, bytes).map_err(|_| {
                *stopped.borrow_mut() =
                    Some(admission.refusal(qsl_eval::simulation::model::ModelKeyError::Denied));
                ExactProbabilityGraphError::ExpansionRefused
            })
        },
        &mut |state, _| {
            let mut borrowed = scope.borrow_mut();
            let (meter, storage) = &mut *borrowed;
            let mut admission =
                ProbabilityKeyAdmission::new(meter, storage, limits.storage_bytes, cancel);
            model_distributions_with_budget(
                system,
                state,
                workload,
                identity,
                &mut admission,
                cancel,
            )
            .map_err(|error| {
                *stopped.borrow_mut() = Some(error);
                ExactProbabilityGraphError::ExpansionRefused
            })
        },
        ModelVisitPhase::BeforeTarget,
        &mut |_, _, _| {
            visit(cancel, scope.borrow_mut().0).map_err(|error| {
                *stopped.borrow_mut() = Some(error);
                ExactProbabilityGraphError::ExpansionRefused
            })
        },
        &mut |cause| {
            *stopped.borrow_mut() = Some(ProbabilityDocumentError::UndefinedPredicate(cause));
        },
        &mut |failure| issue = Some(failure),
    );
    if let Some(error) = stopped.into_inner() {
        return Err(error);
    }
    if let Some(issue) = issue {
        return Err(ProbabilityDocumentError::Build(Box::new(issue)));
    }
    result.map_err(|error| match error {
        ExactProbabilityGraphError::LimitReached => ProbabilityDocumentError::Limit {
            limit: ProbabilityDocumentLimit::States,
            bound: limits.states as u64,
            actual: (limits.states as u64).saturating_add(1),
        },
        ExactProbabilityGraphError::EdgeLimitReached | ExactProbabilityGraphError::TooManyEdges => {
            ProbabilityDocumentError::Limit {
                limit: ProbabilityDocumentLimit::Edges,
                bound: limits.edges as u64,
                actual: (limits.edges as u64).saturating_add(1),
            }
        }
        other => other.into(),
    })
}

fn charge(used: &mut u64, bound: u64, bytes: usize) -> Result<(), ProbabilityDocumentError> {
    let actual = used.saturating_add(u64::try_from(bytes).unwrap_or(u64::MAX));
    if actual > bound {
        return Err(ProbabilityDocumentError::Limit {
            limit: ProbabilityDocumentLimit::StorageBytes,
            bound,
            actual,
        });
    }
    *used = actual;
    Ok(())
}

/// Rebuild the original graph before absorbing targets; a choice after an
/// early target still prevents DTMC admission.
#[allow(clippy::too_many_arguments)]
pub(super) fn enumerate<P: StatePredicate>(
    system: &ModelSystem<'_>,
    target: &P,
    binding: Option<&ObjectReference>,
    workload: Option<&Workload>,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    identity: qsl_foundation::IdentityLimits,
    meter: &mut quire_exact::Meter,
    storage: &mut u64,
) -> Result<(CheckerGraph, BTreeMap<ProbabilityProductKey, u64>), ProbabilityDocumentError> {
    let meter = std::cell::RefCell::new(meter);
    let used = std::cell::RefCell::new(storage);
    let mut graph = enumerate_graph(
        system,
        Some(target),
        binding,
        workload,
        limits,
        cancel,
        identity,
        &mut meter.borrow_mut(),
        &mut used.borrow_mut(),
        false,
        true,
    )?;
    for state in graph.states.values() {
        visit(cancel, &mut meter.borrow_mut())?;
        if !state.choices.is_empty() && state.choices.len() != 1 {
            return Err(ProbabilityDocumentError::Unsupported);
        }
    }
    // Reserve stopped traversal, complete-key copies and analysis storage
    // before retaining them. The shared raw model key owner is separate.
    for (key, state) in &graph.states {
        visit(cancel, &mut meter.borrow_mut())?;
        charge(
            &mut used.borrow_mut(),
            limits.storage_bytes,
            key.as_bytes()
                .len()
                .saturating_mul(5)
                .saturating_add(std::mem::size_of::<CheckerState>())
                .saturating_add(32 * std::mem::size_of::<usize>())
                .saturating_add(
                    state
                        .choices
                        .iter()
                        .flatten()
                        .count()
                        .saturating_mul(8 * std::mem::size_of::<usize>()),
                ),
        )?;
    }
    let mut pending = VecDeque::from([graph.initial.clone()]);
    let mut seen = BTreeSet::from([graph.initial.clone()]);
    let mut stopped = BTreeMap::new();
    while let Some(key) = pending.pop_front() {
        visit(cancel, &mut meter.borrow_mut())?;
        let mut state = graph
            .states
            .remove(&key)
            .ok_or(ProbabilityDocumentError::Malformed)?;
        if state.terminal == Some(true) {
            state.choices.clear();
        }
        for edge in state.choices.iter().flatten() {
            visit(cancel, &mut meter.borrow_mut())?;
            if seen.insert(edge.target.clone()) {
                pending.push_back(edge.target.clone());
            }
        }
        stopped.insert(key, state);
    }
    graph.states = stopped;
    let ranks = analyze(&mut graph, limits, cancel, &mut meter.borrow_mut())?;
    Ok((graph, ranks))
}

/// Iterative Kosaraju plus a reverse breadth-first search. Graph boundaries
/// are checker-derived; no SCC or qualitative producer labels enter here.
fn analyze(
    graph: &mut CheckerGraph,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut quire_exact::Meter,
) -> Result<BTreeMap<ProbabilityProductKey, u64>, ProbabilityDocumentError> {
    let indexes: std::collections::HashMap<_, _> = graph
        .states
        .keys()
        .enumerate()
        .map(|(i, key)| (key.as_bytes(), i))
        .collect();
    let n = graph.states.len();
    let mut outgoing = vec![Vec::new(); n];
    let mut reverse = vec![Vec::new(); n];
    let mut targets = vec![false; n];
    for (i, state) in graph.states.values().enumerate() {
        visit(cancel, meter)?;
        targets[i] = state.terminal == Some(true);
        if state.choices.is_empty() {
            continue;
        }
        if state.choices.len() != 1 {
            return Err(ProbabilityDocumentError::Unsupported);
        }
        let mut mass = rational(0);
        for edge in state.choices.iter().flatten() {
            visit(cancel, meter)?;
            if edge.probability <= rational(0) {
                return Err(ProbabilityDocumentError::Malformed);
            }
            document::check_scalar(&edge.probability, limits)?;
            mass = document::arithmetic(
                quire_exact::RationalArithmetic::Add(&mass, &edge.probability),
                meter,
                cancel,
            )?;
            document::check_scalar(&mass, limits)?;
            let next = *indexes
                .get(edge.target.as_bytes())
                .ok_or(ProbabilityDocumentError::Malformed)?;
            outgoing[i].push(next);
            reverse[next].push(i);
        }
        if mass != rational(1) {
            return Err(ProbabilityDocumentError::Malformed);
        }
    }
    let mut visited = vec![false; n];
    let mut order = Vec::with_capacity(n);
    for start in 0..n {
        visit(cancel, meter)?;
        if visited[start] {
            continue;
        }
        visited[start] = true;
        let mut stack = vec![(start, 0_usize)];
        while let Some((node, edge)) = stack.last_mut() {
            visit(cancel, meter)?;
            if let Some(&next) = outgoing[*node].get(*edge) {
                *edge += 1;
                if !visited[next] {
                    visited[next] = true;
                    stack.push((next, 0));
                }
            } else {
                order.push(*node);
                stack.pop();
            }
        }
    }
    let mut component = vec![usize::MAX; n];
    let mut count = 0;
    for start in order.into_iter().rev() {
        visit(cancel, meter)?;
        if component[start] != usize::MAX {
            continue;
        }
        component[start] = count;
        let mut stack = vec![start];
        while let Some(node) = stack.pop() {
            visit(cancel, meter)?;
            for &next in &reverse[node] {
                visit(cancel, meter)?;
                if component[next] == usize::MAX {
                    component[next] = count;
                    stack.push(next);
                }
            }
        }
        count += 1;
    }
    let mut closed = vec![true; count];
    let mut has_target = vec![false; count];
    for node in 0..n {
        visit(cancel, meter)?;
        has_target[component[node]] |= targets[node];
        for &next in &outgoing[node] {
            visit(cancel, meter)?;
            if component[node] != component[next] {
                closed[component[node]] = false;
            }
        }
    }
    let mut distance: Vec<Option<u64>> = vec![None; n];
    let mut pending = VecDeque::new();
    for (node, target) in targets.iter().enumerate() {
        visit(cancel, meter)?;
        if *target {
            distance[node] = Some(0);
            pending.push_back(node);
        }
    }
    while let Some(node) = pending.pop_front() {
        visit(cancel, meter)?;
        let rank = distance[node].ok_or(ProbabilityDocumentError::Malformed)?;
        for &previous in &reverse[node] {
            visit(cancel, meter)?;
            if distance[previous].is_none() {
                let next = rank
                    .checked_add(1)
                    .filter(|rank| *rank <= 9_007_199_254_740_991)
                    .ok_or(ProbabilityDocumentError::Limit {
                        limit: ProbabilityDocumentLimit::Rank,
                        bound: 9_007_199_254_740_991,
                        actual: u64::MAX,
                    })?;
                distance[previous] = Some(next);
                pending.push_back(previous);
            }
        }
    }
    let mut ranks = BTreeMap::new();
    for (node, (key, state)) in graph.states.iter_mut().enumerate() {
        visit(cancel, meter)?;
        if !targets[node]
            && (distance[node].is_none()
                || (closed[component[node]] && !has_target[component[node]]))
        {
            state.terminal = Some(false);
        }
        ranks.insert(key.clone(), distance[node].unwrap_or(0));
    }
    Ok(ranks)
}

fn visit(cancel: &Cancel, meter: &mut quire_exact::Meter) -> Result<(), ProbabilityDocumentError> {
    poll_cancel(cancel)?;
    meter
        .charge(quire_exact::Charge::new(
            quire_exact::ChargePoint::CollectionMemberTest,
        ))
        .map_err(|incomplete| match cancel.tripped() {
            Some(cause) => ProbabilityDocumentError::Cancelled(cause),
            None => ProbabilityDocumentError::Accounting(Box::new(incomplete)),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn graph() -> CheckerGraph {
        let key = |i| ProbabilityProductKey::from_canonical_bytes(vec![i]);
        let edge = |i, n, d| ProbabilityEdge {
            target: key(i),
            probability: Rational::new(Integer::from(n), Integer::from(d)).unwrap(),
        };
        CheckerGraph {
            initial: key(0),
            states: BTreeMap::from([
                (
                    key(0),
                    CheckerState {
                        terminal: None,
                        choices: vec![vec![edge(1, 1_i64, 1_i64)]],
                    },
                ),
                (
                    key(1),
                    CheckerState {
                        terminal: None,
                        choices: vec![vec![edge(0, 1, 2), edge(2, 1, 4), edge(3, 1, 4)]],
                    },
                ),
                (
                    key(2),
                    CheckerState {
                        terminal: Some(true),
                        choices: vec![],
                    },
                ),
                (
                    key(3),
                    CheckerState {
                        terminal: None,
                        choices: vec![vec![edge(3, 1, 1)]],
                    },
                ),
            ]),
        }
    }

    fn limits() -> ProbabilityDocumentLimits {
        ProbabilityDocumentLimits {
            bytes: 1_000_000,
            storage_bytes: 8_000_000,
            states: 100,
            edges: 100,
            scalar_bits: 128,
            accounting: crate::AccountingLimits::default().0,
        }
    }

    #[trace("TC-636", "FR-201-AC-26", "FR-282-AC-19")]
    #[test]
    fn every_support_scc_and_distance_poll_preserves_typed_cancellation() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::{Arc, OnceLock};
        let live = Cancel::new();
        let _count = live.count_charges();
        let ranks = analyze(
            &mut graph(),
            limits(),
            &live,
            &mut quire_exact::Meter::new(limits().accounting).with_cancel(live.clone()),
        )
        .unwrap();
        assert_eq!(
            ranks.values().copied().collect::<Vec<_>>(),
            vec![2, 1, 0, 0]
        );
        let total = live.charges();
        assert!(total > 0);
        for cause in [CancelCause::Requested, CancelCause::Deadline] {
            for at in 1..=total {
                let slot = Arc::new(OnceLock::<Cancel>::new());
                let weak = Arc::downgrade(&slot);
                let seen = Arc::new(AtomicU32::new(0));
                let count = Arc::clone(&seen);
                let cancel = Cancel::observing(move || {
                    if count.fetch_add(1, Ordering::Relaxed) + 1 == at {
                        weak.upgrade().unwrap().get().unwrap().cancel(cause);
                    }
                });
                slot.set(cancel.clone()).unwrap();
                assert_eq!(
                    analyze(
                        &mut graph(),
                        limits(),
                        &cancel,
                        &mut quire_exact::Meter::new(limits().accounting)
                            .with_cancel(cancel.clone())
                    ),
                    Err(ProbabilityDocumentError::Cancelled(cause)),
                    "support pass poll {at}/{total}"
                );
                assert_eq!(seen.load(Ordering::Relaxed), at);
            }
        }
    }

    #[trace("TC-636", "FR-201-AC-26", "FR-282-AC-19")]
    #[test]
    fn support_passes_preserve_each_exhausted_work_budget() {
        let cancel = Cancel::new();
        let mut meter = quire_exact::Meter::new(limits().accounting).with_cancel(cancel.clone());
        analyze(&mut graph(), limits(), &cancel, &mut meter).unwrap();
        let total = meter.consumed(quire_exact::LimitKind::WorkUnits);
        assert!(total > 0);
        for bound in 0..total {
            let mut accounting = limits().accounting;
            accounting.work_units = bound;
            let mut meter = quire_exact::Meter::new(accounting).with_cancel(cancel.clone());
            let Err(ProbabilityDocumentError::Accounting(incomplete)) =
                analyze(&mut graph(), limits(), &cancel, &mut meter)
            else {
                panic!("support analysis must stop under work budget {bound}/{total}");
            };
            assert_eq!(incomplete.limit_kind, quire_exact::LimitKind::WorkUnits);
            assert_eq!(incomplete.limit, bound);
            assert!(incomplete.consumed <= bound);
        }
    }
}
