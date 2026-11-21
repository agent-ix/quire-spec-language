// SPDX-License-Identifier: AGPL-3.0-or-later
//! The FR-418 task-product certificate of EN-6's symbolic pass.
//!
//! [`task_product_certificate`] turns the engine's closed quotient
//! ([`SymbolicGraph`]) into the `ZoneCertificate` the checker accepts: one
//! node per task product key holding the convex hull of that key's regions,
//! one edge per distinct (node, identity, node) triple, `initials`, the
//! `task_product` member and the TD-6 `admitted` witness, which is built
//! here from the engine's own graph.

use std::collections::{BTreeMap, VecDeque};

use filament_exact::Integer;
use fsl_eval::simulation::model::{ClockRole, TaskAutomaton};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_replay::{
    CertEdge, CertNode, DivergenceWitness, DivergentTarget, TaskClockCeiling,
    TaskDiagonalConstraint, TaskDiagonalOperator, TaskProductCertificate, TaskQueueBound,
    TaskSubtraction, Zone, ZoneCertificate,
};
use num_bigint::BigInt;

use super::dbm::Bound;
use super::task_symbolic::{SymbolicGraph, SymbolicIdentity};

/// Why no certificate was built from a closed quotient.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskCertificateRefusal {
    /// A deadline miss is reachable: the set is not proved schedulable.
    Missed,
    /// The product admits no time-divergent behaviour the checker can read.
    NoDivergence,
    /// The checked task set has no finite product member.
    NoMember,
    /// A zone or a constant does not fit the certificate's shape.
    Shape,
}

type Cell = Option<(BigInt, bool)>;

fn cells(zone: &super::dbm::Dbm) -> Option<Vec<Cell>> {
    let dim = zone.dim();
    let mut out = Vec::with_capacity(dim * dim);
    for i in 0..dim {
        for j in 0..dim {
            out.push(match zone.bound(i, j).ok()? {
                Bound::Finite { value, strict } => Some((value.clone(), *strict)),
                Bound::Infinite => None,
            });
        }
    }
    Some(out)
}

/// The looser of two entries: the larger value, the non-strict one on a tie.
fn looser(left: &Cell, right: &Cell) -> Cell {
    match (left, right) {
        (Some(a), Some(b)) => Some(if (&a.0, !a.1) >= (&b.0, !b.1) {
            a.clone()
        } else {
            b.clone()
        }),
        _ => None,
    }
}

fn core_zone(dim: usize, cells: Vec<Cell>) -> Option<Zone> {
    let cells: Vec<_> = cells
        .into_iter()
        .map(|cell| cell.map(|(value, strict)| (Integer::from_big(value), strict)))
        .collect();
    Zone::from_matrix(dim, cells).ok()
}

/// Whether a cycle's effects advance time by a fixed positive amount: some
/// guard `x >= b` (or `x > b`) with `b` above the value of the last reset of
/// `x` before it, going back around the cycle; an assignment from another
/// clock, or no reset at all, proves nothing.
fn advances(effects: &[&super::task_symbolic::SymbolicEffect]) -> bool {
    let length = effects.len();
    for (position, effect) in effects.iter().enumerate() {
        for (clock, bound, _) in &effect.lowers {
            for back in 1..=length {
                let earlier = effects[(position + length - back) % length];
                if earlier.assigned.contains(clock) {
                    break;
                }
                if let Some((_, value)) = earlier.resets.iter().rev().find(|(r, _)| r == clock) {
                    if bound > value {
                        return true;
                    }
                    break;
                }
            }
        }
    }
    false
}

/// The `task_product` member the engine states for the closed quotient.
fn member(
    automaton: &TaskAutomaton<'_, '_, '_>,
    graph: &SymbolicGraph,
) -> Option<TaskProductCertificate> {
    let rows = TaskQueueBound::from_checked_task_set(automaton.product().plan().task_set)?;
    let clocks = automaton.clocks();
    let mut ceilings = Vec::new();
    for (index, clock) in clocks.iter().enumerate() {
        if matches!(clock.role, ClockRole::Task(_)) {
            ceilings.push(TaskClockCeiling {
                clock: clock.name.clone(),
                maximum: Integer::from_big(graph.ceilings[index].clone()),
            });
        }
    }
    let name = |index: usize| clocks[index - 1].name.clone();
    let mut diagonals = Vec::new();
    for diagonal in automaton.diagonals() {
        let value = diagonal
            .constant
            .mul(&filament_exact::Rational::from_integer(Integer::from_big(
                graph.scale.clone(),
            )));
        diagonals.push(TaskDiagonalConstraint {
            left: name(diagonal.left),
            operator: if diagonal.strict {
                TaskDiagonalOperator::Greater
            } else {
                TaskDiagonalOperator::GreaterOrEqual
            },
            right: name(diagonal.right),
            constant: Integer::from_big(value.numerator().as_big().clone()),
        });
    }
    let subtractions = graph
        .subtractions
        .iter()
        .map(|row| TaskSubtraction {
            transition: row.template.clone(),
            clock: row.clock.clone(),
            constant: Integer::from_big(row.constant.clone()),
        })
        .collect();
    TaskProductCertificate::closed_finite_product(rows, ceilings, diagonals, subtractions)
}

/// Builds the certificate of the closed quotient `graph` of `automaton` for
/// the claim occurrence `obligation`.
///
/// # Errors
///
/// [`TaskCertificateRefusal`] when a miss is reachable, no time-divergent
/// witness can be built, or the member or a zone cannot be written.
pub fn task_product_certificate(
    automaton: &TaskAutomaton<'_, '_, '_>,
    graph: &SymbolicGraph,
    obligation: OccurrenceKey,
) -> Result<ZoneCertificate<OccurrenceKey>, TaskCertificateRefusal> {
    use TaskCertificateRefusal as Refusal;
    if graph.vertices.iter().any(|vertex| vertex.missed) {
        return Err(Refusal::Missed);
    }
    let dim = automaton.clocks().len() + 1;
    let mut node_of_key = BTreeMap::new();
    let mut hulls: Vec<Vec<Cell>> = Vec::new();
    let mut nodes: Vec<CertNode> = Vec::new();
    let mut node_of_vertex = Vec::with_capacity(graph.vertices.len());
    for vertex in &graph.vertices {
        let own = cells(&vertex.zone).ok_or(Refusal::Shape)?;
        let node = *node_of_key.entry(vertex.key.clone()).or_insert_with(|| {
            hulls.push(own.clone());
            nodes.push(CertNode {
                state_key: vertex.key.clone(),
                automaton_state: 0,
                zone: Zone::zero(dim),
                locked: false,
                product: None,
            });
            nodes.len() - 1
        });
        let merged = hulls[node]
            .iter()
            .zip(&own)
            .map(|(a, b)| looser(a, b))
            .collect();
        hulls[node] = merged;
        node_of_vertex.push(node);
    }
    for (node, hull) in nodes.iter_mut().zip(hulls) {
        node.zone = core_zone(dim, hull).ok_or(Refusal::Shape)?;
    }
    // One certificate edge per distinct (node, identity, node).
    let mut edge_of = BTreeMap::new();
    let mut list: Vec<CertEdge> = Vec::new();
    for step in &graph.discrete {
        let triple = (
            node_of_vertex[step.from],
            step.identity,
            node_of_vertex[step.to],
        );
        edge_of.entry(triple).or_insert_with(|| {
            let mut edge = CertEdge {
                from: triple.0,
                transition: None,
                task_transition: None,
                part: None,
                horizon: None,
                to: triple.2,
                silent: None,
            };
            match &graph.identities[step.identity] {
                SymbolicIdentity::Source(transition) => edge.transition = Some(transition.clone()),
                SymbolicIdentity::Template(template) => {
                    edge.task_transition = Some(template.clone());
                }
            }
            list.push(edge);
            list.len() - 1
        });
    }
    let mut order: Vec<usize> = (0..list.len()).collect();
    order.sort_by_key(|&index| (list[index].from, index));
    let mut renumber = vec![0; list.len()];
    for (new, &old) in order.iter().enumerate() {
        renumber[old] = new;
    }
    let edges: Vec<CertEdge> = order.iter().map(|&old| list[old].clone()).collect();
    for index in edge_of.values_mut() {
        *index = renumber[*index];
    }
    let initials = graph
        .initials
        .iter()
        .map(|initial| node_of_key[&initial.key])
        .collect();
    let admitted = witness(graph, &node_of_vertex, &edge_of).ok_or(Refusal::NoDivergence)?;
    Ok(ZoneCertificate {
        obligation,
        scale: Integer::from_big(graph.scale.clone()),
        lu_bounds: fsl_replay::LuBounds {
            lower: Vec::new(),
            upper: Vec::new(),
        },
        nodes,
        edges,
        initials,
        components: None,
        non_zeno: None,
        divergence: None,
        admitted: Some(admitted),
        state_graph: None,
        refinement: None,
        task_product: Some(member(automaton, graph).ok_or(Refusal::NoMember)?),
    })
}

/// One move of the quotient seen as a graph: a vertex, the certificate edge
/// a discrete move takes (`None` for a delay) and its clock effect.
type Move = (usize, Option<usize>, Option<usize>);

/// The TD-6 witness over the quotient: a path from an initial vertex to a
/// vertex where time idles forever, else to a closed walk that advances time
/// by a fixed amount each round.
fn witness(
    graph: &SymbolicGraph,
    node_of_vertex: &[usize],
    edge_of: &BTreeMap<(usize, usize, usize), usize>,
) -> Option<DivergenceWitness> {
    let count = graph.vertices.len();
    let mut out: Vec<Vec<Move>> = vec![Vec::new(); count];
    for step in &graph.discrete {
        let triple = (
            node_of_vertex[step.from],
            step.identity,
            node_of_vertex[step.to],
        );
        out[step.from].push((step.to, Some(edge_of[&triple]), Some(step.effect)));
    }
    for &(from, to) in &graph.time {
        out[from].push((to, None, None));
    }
    let reach = |from: &[usize]| {
        let mut parent: Vec<Option<(usize, Option<usize>)>> = vec![None; count];
        let mut seen = vec![false; count];
        let mut queue = VecDeque::new();
        for &vertex in from {
            seen[vertex] = true;
            queue.push_back(vertex);
        }
        while let Some(vertex) = queue.pop_front() {
            for &(to, edge, _) in &out[vertex] {
                if !seen[to] {
                    seen[to] = true;
                    parent[to] = Some((vertex, edge));
                    queue.push_back(to);
                }
            }
        }
        (seen, parent)
    };
    let path_to = |parent: &[Option<(usize, Option<usize>)>], mut vertex: usize| {
        let mut path = Vec::new();
        while let Some((before, edge)) = parent[vertex] {
            path.extend(edge);
            vertex = before;
        }
        path.reverse();
        (vertex, path)
    };
    let initials: Vec<usize> = graph
        .initials
        .iter()
        .flat_map(|initial| initial.vertices.iter().copied())
        .collect();
    let (seen, parent) = reach(&initials);
    let start_node = |vertex: usize| {
        let (root, _) = path_to(&parent, vertex);
        node_of_vertex[root]
    };
    // The wire carries a witness as one edge list, read back canonically
    // (`canonical`): only a witness that reads back unchanged is written.
    let mut ends = vec![(0usize, 0usize); edge_of.len()];
    for (&(from, _, to), &index) in edge_of {
        ends[index] = (from, to);
    }
    for idle in (0..count).filter(|&v| seen[v] && graph.time.contains(&(v, v))) {
        let (_, path) = path_to(&parent, idle);
        let candidate = DivergenceWitness {
            start: start_node(idle),
            path,
            target: DivergentTarget::Quiescent(node_of_vertex[idle]),
        };
        if canonical(&candidate.path, candidate.start, &ends).as_ref() == Some(&candidate) {
            return Some(candidate);
        }
    }
    // A discrete edge with a lower-bound guard, closed by a walk back to its
    // source over any moves.
    for step in &graph.discrete {
        if !seen[step.from] || graph.effects[step.effect].lowers.is_empty() {
            continue;
        }
        let (back, back_parent) = reach(&[step.to]);
        if !back[step.from] {
            continue;
        }
        // The walk back, with the effects of its discrete moves.
        let mut labels = Vec::new();
        let mut effects = vec![step.effect];
        let mut vertex = step.from;
        while let Some((before, edge)) = back_parent[vertex] {
            if let Some(edge) = edge {
                labels.push(edge);
                let effect = out[before]
                    .iter()
                    .find(|(to, own, _)| *to == vertex && *own == Some(edge))
                    .and_then(|(_, _, effect)| *effect)?;
                effects.push(effect);
            }
            vertex = before;
        }
        // The walk back was collected from its end; the cycle runs the step
        // first, then the walk from `step.to`.
        labels.reverse();
        effects[1..].reverse();
        let own = edge_of[&(
            node_of_vertex[step.from],
            step.identity,
            node_of_vertex[step.to],
        )];
        let mut cycle = vec![own];
        cycle.extend(labels);
        let (cycle, effects) = erase_loops(&cycle, &effects, &ends);
        let refs: Vec<_> = effects.iter().map(|&e| &graph.effects[e]).collect();
        if !advances(&refs) {
            continue;
        }
        let (_, path) = path_to(&parent, step.from);
        let candidate = DivergenceWitness {
            start: start_node(step.from),
            target: DivergentTarget::Cycle(cycle.clone()),
            path,
        };
        let mut flat = candidate.path.clone();
        flat.extend(&cycle);
        if canonical(&flat, candidate.start, &ends).as_ref() == Some(&candidate) {
            return Some(candidate);
        }
    }
    None
}

/// The reading of a wire edge list `flat` from `start`, as
/// `ZoneCertificate::from_wire` decodes it: the smallest closed suffix is the
/// cycle and the rest the path; with none, the path ends at a quiescent node.
/// `ends` is each certificate edge's `(from, to)`.
fn canonical(flat: &[usize], start: usize, ends: &[(usize, usize)]) -> Option<DivergenceWitness> {
    let Some(&last) = flat.last() else {
        return Some(DivergenceWitness {
            start,
            path: Vec::new(),
            target: DivergentTarget::Quiescent(start),
        });
    };
    let end = ends.get(last)?.1;
    Some(
        match (0..flat.len()).rev().find(|&k| ends[flat[k]].0 == end) {
            Some(k) => DivergenceWitness {
                start,
                path: flat[..k].to_vec(),
                target: DivergentTarget::Cycle(flat[k..].to_vec()),
            },
            None => DivergenceWitness {
                start,
                path: flat.to_vec(),
                target: DivergentTarget::Quiescent(end),
            },
        },
    )
}

/// The closed walk `cycle` (with the effect of each edge) from its first
/// edge's source, with every loop that does not contain its first edge
/// removed, so no node but the start is visited twice and the wire's canonical
/// reading returns the cycle itself.
fn erase_loops(
    cycle: &[usize],
    effects: &[usize],
    ends: &[(usize, usize)],
) -> (Vec<usize>, Vec<usize>) {
    let start = ends[cycle[0]].0;
    let mut kept: Vec<(usize, usize)> = Vec::new();
    for (&edge, &effect) in cycle.iter().zip(effects) {
        let to = ends[edge].1;
        kept.push((edge, effect));
        if to == start {
            break;
        }
        // A node left before: drop the loop that returned to it.
        if let Some(at) = kept.iter().position(|&(earlier, _)| ends[earlier].0 == to) {
            kept.truncate(at);
        }
    }
    kept.into_iter().unzip()
}

#[cfg(test)]
mod tests {
    use super::{canonical, erase_loops};
    use fsl_replay::{DivergenceWitness, DivergentTarget};

    /// Edges 0: A->B, 1: B->A, 2: A->C, 3: C->A, 4: B->B (nodes A=0, B=1, C=2).
    const ENDS: [(usize, usize); 5] = [(0, 1), (1, 0), (0, 2), (2, 0), (1, 1)];

    /// A closed walk that returns to its start node mid-way would be read
    /// back from the wire as a shorter cycle with a longer path; the producer
    /// writes the first return instead, which reads back unchanged.
    #[test]
    fn a_cycle_that_revisits_its_start_is_cut_at_the_first_return() {
        let cycle = [0, 1, 2, 3];
        let effects = [10, 11, 12, 13];
        // Read back as written, the long walk is a different witness.
        let long = canonical(&cycle, 0, &ENDS).expect("a reading");
        assert_eq!(
            long,
            DivergenceWitness {
                start: 0,
                path: vec![0, 1],
                target: DivergentTarget::Cycle(vec![2, 3]),
            }
        );
        let (cut, kept) = erase_loops(&cycle, &effects, &ENDS);
        assert_eq!(
            (cut.as_slice(), kept.as_slice()),
            (&[0, 1][..], &[10, 11][..])
        );
        let again = canonical(&cut, 0, &ENDS).expect("a reading");
        assert_eq!(again.target, DivergentTarget::Cycle(cut));
        assert!(again.path.is_empty());
    }

    /// A loop that does not contain the first edge is removed.
    #[test]
    fn an_inner_loop_is_erased() {
        let (cut, kept) = erase_loops(&[0, 4, 1], &[5, 6, 7], &ENDS);
        assert_eq!(
            (cut.as_slice(), kept.as_slice()),
            (&[0, 1][..], &[5, 7][..])
        );
    }

    /// A path whose end node an earlier edge leaves reads back as a cycle, so
    /// a quiescent witness there is not written.
    #[test]
    fn a_quiescent_path_through_its_end_node_reads_back_as_a_cycle() {
        let read = canonical(&[0, 1], 0, &ENDS).expect("a reading");
        assert_eq!(read.target, DivergentTarget::Cycle(vec![0, 1]));
        let read = canonical(&[0], 0, &ENDS).expect("a reading");
        assert_eq!(read.target, DivergentTarget::Quiescent(1));
    }
}
