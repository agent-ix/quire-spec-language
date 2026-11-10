// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-201 closed cyclic DTMC admission and checker-owned support analysis.

use super::*;

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
) -> Result<(CheckerGraph, BTreeMap<ProbabilityProductKey, u64>), ProbabilityDocumentError> {
    let used = std::cell::RefCell::new(0_u64);
    let failure = std::cell::RefCell::new(None);
    let mut issue = None;
    let key_of = |state: &ModelSimState| {
        let (bytes, _) = system
            .state_key(state)
            .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?;
        document::model_product_key_with_storage(
            binding,
            &bytes,
            None,
            ProbabilityDocumentIoLimits {
                bytes: limits.bytes,
                storage_bytes: limits.storage_bytes,
            },
            cancel,
            &mut used.borrow_mut(),
        )
        .map_err(|error| {
            *failure.borrow_mut() = Some(error);
            ExactProbabilityGraphError::ExpansionRefused
        })
    };
    let result = enumerate_model_graph_with_keys_and_reservation(
        system,
        Some(target),
        binding,
        workload,
        limits.states,
        limits.edges,
        cancel,
        false,
        &key_of,
        &mut |bytes| {
            charge(&mut used.borrow_mut(), limits.storage_bytes, bytes).map_err(|error| {
                *failure.borrow_mut() = Some(error);
                ExactProbabilityGraphError::ExpansionRefused
            })
        },
        #[cfg(feature = "test-support")]
        &mut |_| {},
        &mut |_, _| Ok(()),
        &mut |failure| issue = Some(failure),
    );
    if let Some(error) = failure.into_inner() {
        return Err(error);
    }
    if let Some(issue) = issue {
        return Err(ProbabilityDocumentError::Build(Box::new(issue)));
    }
    let mut graph = result?;
    for state in graph.states.values() {
        poll_cancel(cancel)?;
        if !state.choices.is_empty() && state.choices.len() != 1 {
            return Err(ProbabilityDocumentError::Unsupported);
        }
    }
    // Reserve stopped traversal, complete-key copies and analysis storage
    // before retaining them. The shared raw model key owner is separate.
    for (key, state) in &graph.states {
        poll_cancel(cancel)?;
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
        poll_cancel(cancel)?;
        let mut state = graph
            .states
            .remove(&key)
            .ok_or(ProbabilityDocumentError::Malformed)?;
        if state.terminal == Some(true) {
            state.choices.clear();
        }
        for edge in state.choices.iter().flatten() {
            poll_cancel(cancel)?;
            if seen.insert(edge.target.clone()) {
                pending.push_back(edge.target.clone());
            }
        }
        stopped.insert(key, state);
    }
    graph.states = stopped;
    let ranks = analyze(&mut graph, limits, cancel)?;
    Ok((graph, ranks))
}

/// Iterative Kosaraju plus a reverse breadth-first search. Graph boundaries
/// are checker-derived; no SCC or qualitative producer labels enter here.
fn analyze(
    graph: &mut CheckerGraph,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<BTreeMap<ProbabilityProductKey, u64>, ProbabilityDocumentError> {
    let mut meter = quire_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
    let keys: Vec<_> = graph.states.keys().cloned().collect();
    let indexes: BTreeMap<_, _> = keys
        .iter()
        .enumerate()
        .map(|(i, key)| (key.clone(), i))
        .collect();
    let n = keys.len();
    let mut outgoing = vec![Vec::new(); n];
    let mut reverse = vec![Vec::new(); n];
    let mut targets = vec![false; n];
    for (i, key) in keys.iter().enumerate() {
        visit(cancel, &mut meter)?;
        let state = graph
            .states
            .get(key)
            .ok_or(ProbabilityDocumentError::Malformed)?;
        targets[i] = state.terminal == Some(true);
        if state.choices.is_empty() {
            continue;
        }
        if state.choices.len() != 1 {
            return Err(ProbabilityDocumentError::Unsupported);
        }
        let mut mass = rational(0);
        for edge in state.choices.iter().flatten() {
            visit(cancel, &mut meter)?;
            if edge.probability <= rational(0) {
                return Err(ProbabilityDocumentError::Malformed);
            }
            document::check_scalar(&edge.probability, limits)?;
            mass = document::arithmetic(
                quire_exact::RationalArithmetic::Add(&mass, &edge.probability),
                &mut meter,
                cancel,
            )?;
            document::check_scalar(&mass, limits)?;
            let next = *indexes
                .get(&edge.target)
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
        visit(cancel, &mut meter)?;
        if visited[start] {
            continue;
        }
        visited[start] = true;
        let mut stack = vec![(start, 0_usize)];
        while let Some((node, edge)) = stack.last_mut() {
            visit(cancel, &mut meter)?;
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
        visit(cancel, &mut meter)?;
        if component[start] != usize::MAX {
            continue;
        }
        component[start] = count;
        let mut stack = vec![start];
        while let Some(node) = stack.pop() {
            visit(cancel, &mut meter)?;
            for &next in &reverse[node] {
                visit(cancel, &mut meter)?;
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
        visit(cancel, &mut meter)?;
        has_target[component[node]] |= targets[node];
        for &next in &outgoing[node] {
            visit(cancel, &mut meter)?;
            if component[node] != component[next] {
                closed[component[node]] = false;
            }
        }
    }
    let mut distance: Vec<Option<u64>> = vec![None; n];
    let mut pending = VecDeque::new();
    for (node, target) in targets.iter().enumerate() {
        visit(cancel, &mut meter)?;
        if *target {
            distance[node] = Some(0);
            pending.push_back(node);
        }
    }
    while let Some(node) = pending.pop_front() {
        visit(cancel, &mut meter)?;
        let rank = distance[node].ok_or(ProbabilityDocumentError::Malformed)?;
        for &previous in &reverse[node] {
            visit(cancel, &mut meter)?;
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
    for (node, key) in keys.into_iter().enumerate() {
        visit(cancel, &mut meter)?;
        if !targets[node]
            && (distance[node].is_none()
                || (closed[component[node]] && !has_target[component[node]]))
        {
            graph
                .states
                .get_mut(&key)
                .ok_or(ProbabilityDocumentError::Malformed)?
                .terminal = Some(false);
        }
        ranks.insert(key, distance[node].unwrap_or(0));
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
