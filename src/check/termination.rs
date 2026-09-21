// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-146 termination: recursive components of the reachable call graph and
//! their lexicographic `decreases` obligations.

use std::collections::{BTreeSet, VecDeque};

use super::facts::{ArgumentShape, CallSite, EdgeKind};
use super::ir::{Node, NodeKind, Slot};
use super::refusal::{CheckCause, CheckRefusal, Location, MeasureObligation};
use crate::value::composite::ValueType;

/// One element of a measure, naming the parameter it measures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Element {
    Integer(Slot),
    Cardinality(Slot),
    Structural(Slot),
}

impl Element {
    fn same_kind(self, other: Self) -> bool {
        matches!(
            (self, other),
            (Self::Integer(_), Self::Integer(_))
                | (Self::Cardinality(_), Self::Cardinality(_))
                | (Self::Structural(_), Self::Structural(_))
        )
    }
}

/// What termination needs of one checked function.
pub(crate) struct Member<'a> {
    pub(crate) name: &'a str,
    pub(crate) parameters: &'a [(String, ValueType)],
    pub(crate) measure: Option<&'a Node>,
    pub(crate) calls: &'a [CallSite],
}

fn unwrap_coerce(node: &Node) -> &Node {
    match &node.kind {
        NodeKind::Coerce(operand, _) => operand,
        _ => node,
    }
}

/// The measure elements, `None` at a position that is not an element.
fn elements(member: &Member<'_>, measure: &Node) -> Vec<Option<Element>> {
    let parts: Vec<&Node> = match &measure.kind {
        NodeKind::Tuple { arguments, .. } => arguments.iter().collect(),
        _ => vec![measure],
    };
    parts
        .into_iter()
        .map(|part| {
            let part = unwrap_coerce(part);
            let parameter = |slot: Slot| {
                member
                    .parameters
                    .get(slot)
                    .map(|(_, value_type)| value_type)
            };
            match &part.kind {
                NodeKind::Local(slot) => match parameter(*slot)? {
                    ValueType::Integer | ValueType::Int(_) => Some(Element::Integer(*slot)),
                    ValueType::Composite(_) => Some(Element::Structural(*slot)),
                    _ => None,
                },
                NodeKind::Size(operand) => match unwrap_coerce(operand).kind {
                    NodeKind::Local(slot) => match parameter(slot)? {
                        ValueType::Collection(_) => Some(Element::Cardinality(slot)),
                        _ => None,
                    },
                    _ => None,
                },
                _ => None,
            }
        })
        .collect()
}

/// Check every recursive component, in first-declaration order.
pub(crate) fn check(members: &[Member<'_>]) -> Vec<CheckRefusal> {
    let count = members.len();
    let callees = |caller: usize| -> BTreeSet<usize> {
        members
            .get(caller)
            .map(|member| member.calls.iter().map(|call| call.callee).collect())
            .unwrap_or_default()
    };
    let reachable: Vec<BTreeSet<usize>> = (0..count)
        .map(|start| {
            let mut seen = BTreeSet::new();
            let mut pending: VecDeque<usize> = callees(start).into_iter().collect();
            while let Some(next) = pending.pop_front() {
                if seen.insert(next) {
                    pending.extend(callees(next));
                }
            }
            seen
        })
        .collect();
    let mut assigned = vec![false; count];
    let mut refusals = Vec::new();
    for first in 0..count {
        if assigned.get(first).copied().unwrap_or(true) {
            continue;
        }
        let component: Vec<usize> = (0..count)
            .filter(|&other| {
                other == first
                    || (reachable.get(first).is_some_and(|set| set.contains(&other))
                        && reachable.get(other).is_some_and(|set| set.contains(&first)))
            })
            .collect();
        for &index in &component {
            if let Some(flag) = assigned.get_mut(index) {
                *flag = true;
            }
        }
        let recursive =
            component.len() > 1 || reachable.get(first).is_some_and(|set| set.contains(&first));
        if recursive {
            if let Some(refusal) = dispatch_cycle_refusal(members, &component) {
                refusals.push(refusal);
                continue;
            }
            if let Err(refusal) = check_component(members, &component) {
                refusals.push(refusal);
            }
        }
    }
    refusals
}

/// FR-151 (TC-196 D08): `refused { code: invalid_package, cause:
/// definition-cycle }` for a strongly connected component containing a
/// dispatch edge, before any measure-decrease obligation is even attempted.
/// Operation bodies and contract clauses have no `decreases` form, so this
/// is checked ahead of, and instead of, [`check_component`]; an ordinary
/// (all-[`EdgeKind::Ordinary`]) component is entirely unaffected.
fn dispatch_cycle_refusal(members: &[Member<'_>], component: &[usize]) -> Option<CheckRefusal> {
    let mut located: Vec<((usize, usize), Location)> = component
        .iter()
        .filter_map(|&caller| members.get(caller).map(|member| (caller, member)))
        .flat_map(|(caller, member)| {
            member
                .calls
                .iter()
                .filter(move |call| {
                    call.kind == EdgeKind::Dispatch && component.contains(&call.callee)
                })
                .map(move |call| ((caller, call.callee), call.location.clone()))
        })
        .collect();
    if located.is_empty() {
        return None;
    }
    located.sort_by_key(|(edge, _)| *edge);
    let location = located.first().map(|(_, location)| location.clone())?;
    let mut edges: Vec<(usize, usize)> = located.into_iter().map(|(edge, _)| edge).collect();
    edges.dedup();
    let named_edges: Vec<(String, String)> = edges
        .into_iter()
        .filter_map(|(caller, callee)| {
            Some((
                members.get(caller)?.name.to_owned(),
                members.get(callee)?.name.to_owned(),
            ))
        })
        .collect();
    Some(CheckRefusal {
        location,
        cause: CheckCause::DefinitionCycle { edges: named_edges },
    })
}

/// The component path from `from` back to `to`, by breadth-first search.
fn path(members: &[Member<'_>], component: &[usize], from: usize, to: usize) -> Vec<usize> {
    let mut parent: Vec<Option<usize>> = vec![None; members.len()];
    let mut seen = BTreeSet::from([from]);
    let mut pending = VecDeque::from([from]);
    while let Some(current) = pending.pop_front() {
        if current == to {
            break;
        }
        let Some(member) = members.get(current) else {
            continue;
        };
        for call in member.calls {
            if component.contains(&call.callee) && seen.insert(call.callee) {
                if let Some(slot) = parent.get_mut(call.callee) {
                    *slot = Some(current);
                }
                pending.push_back(call.callee);
            }
        }
    }
    let mut reversed = vec![to];
    let mut current = to;
    while current != from {
        match parent.get(current).copied().flatten() {
            Some(previous) => {
                reversed.push(previous);
                current = previous;
            }
            None => break,
        }
    }
    reversed.reverse();
    reversed
}

fn check_component(members: &[Member<'_>], component: &[usize]) -> Result<(), CheckRefusal> {
    let edges: Vec<(usize, &CallSite)> = component
        .iter()
        .filter_map(|&caller| members.get(caller).map(|member| (caller, member)))
        .flat_map(|(caller, member)| {
            member
                .calls
                .iter()
                .filter(|call| component.contains(&call.callee))
                .map(move |call| (caller, call))
        })
        .collect();
    let Some(&(first_caller, first_edge)) = edges.first() else {
        return Ok(());
    };
    let refuse = |caller: usize, edge: &CallSite, obligation| {
        let mut cycle: Vec<usize> = vec![caller];
        cycle.extend(path(members, component, edge.callee, caller));
        CheckRefusal {
            location: edge.location.clone(),
            cause: CheckCause::UnprovedDecrease {
                cycle: cycle
                    .into_iter()
                    .filter_map(|index| members.get(index).map(|member| member.name.to_owned()))
                    .collect(),
                obligation,
            },
        }
    };
    let component_refusal = |obligation| refuse(first_caller, first_edge, obligation);
    let mut measures = Vec::with_capacity(component.len());
    for &index in component {
        let member = members
            .get(index)
            .ok_or_else(|| component_refusal(MeasureObligation::MissingMeasure))?;
        let measure = member
            .measure
            .ok_or_else(|| component_refusal(MeasureObligation::MissingMeasure))?;
        measures.push((index, elements(member, measure)));
    }
    let arity = measures.first().map_or(0, |(_, elements)| elements.len());
    if measures.iter().any(|(_, elements)| elements.len() != arity) {
        return Err(component_refusal(MeasureObligation::MeasureArity));
    }
    let mut typed: Vec<(usize, Vec<Element>)> = Vec::with_capacity(measures.len());
    for (index, elements) in measures {
        let elements: Option<Vec<Element>> = elements.into_iter().collect();
        let elements = elements.ok_or_else(|| component_refusal(MeasureObligation::MeasureKind))?;
        typed.push((index, elements));
    }
    if let Some((_, reference)) = typed.first() {
        let mismatched = typed.iter().any(|(_, elements)| {
            elements
                .iter()
                .zip(reference)
                .any(|(element, expected)| !element.same_kind(*expected))
        });
        if mismatched {
            return Err(component_refusal(MeasureObligation::MeasureKind));
        }
    }
    for (index, elements) in &typed {
        let nonnegative = elements.iter().all(|element| match element {
            Element::Integer(slot) => members
                .get(*index)
                .and_then(|member| member.parameters.get(*slot))
                .is_some_and(|(_, value_type)| match value_type {
                    ValueType::Int(domain) => !domain.lower().is_negative(),
                    _ => false,
                }),
            Element::Cardinality(_) | Element::Structural(_) => true,
        });
        if !nonnegative {
            return Err(component_refusal(MeasureObligation::Nonnegative));
        }
    }
    let measure_of = |index: usize| {
        typed
            .iter()
            .find(|(member, _)| *member == index)
            .map(|(_, elements)| elements.as_slice())
    };
    for (caller, edge) in edges {
        let (Some(caller_measure), Some(callee_measure)) =
            (measure_of(caller), measure_of(edge.callee))
        else {
            return Err(refuse(caller, edge, MeasureObligation::Decrease));
        };
        let mut decreased = false;
        for (at_caller, at_callee) in caller_measure.iter().zip(callee_measure) {
            let (caller_slot, callee_slot) = match (at_caller, at_callee) {
                (Element::Integer(c), Element::Integer(d))
                | (Element::Cardinality(c), Element::Cardinality(d))
                | (Element::Structural(c), Element::Structural(d)) => (*c, *d),
                _ => break,
            };
            let argument = edge
                .arguments
                .get(callee_slot)
                .copied()
                .unwrap_or(ArgumentShape::Other);
            let (equal, smaller) = match at_caller {
                Element::Integer(_) => (
                    argument == ArgumentShape::Parameter(caller_slot),
                    argument == ArgumentShape::Decremented(caller_slot),
                ),
                Element::Cardinality(_) => {
                    (argument == ArgumentShape::Parameter(caller_slot), false)
                }
                Element::Structural(_) => (
                    argument == ArgumentShape::Parameter(caller_slot),
                    argument == ArgumentShape::Projection(caller_slot),
                ),
            };
            if smaller {
                decreased = true;
                break;
            }
            if !equal {
                break;
            }
        }
        if !decreased {
            return Err(refuse(caller, edge, MeasureObligation::Decrease));
        }
    }
    Ok(())
}
