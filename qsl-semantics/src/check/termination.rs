// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-146 termination: recursive components of the reachable call graph and
//! their lexicographic `decreases` obligations.

use std::collections::VecDeque;

use super::facts::{ArgumentShape, CallSite, EdgeKind};
use super::ir::{Node, NodeKind, Slot};
use super::refusal::{CheckCause, CheckRefusal, Location, MeasureObligation};
use quire_exact::ValueType;

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

/// A fixed-size set of member indices, one bit each.
struct Bits(Vec<u64>);

impl Bits {
    fn new(len: usize) -> Self {
        Self(vec![0; len.div_ceil(64)])
    }

    fn insert(&mut self, index: usize) {
        if let Some(word) = self.0.get_mut(index / 64) {
            *word |= 1 << (index % 64);
        }
    }

    fn remove(&mut self, index: usize) {
        if let Some(word) = self.0.get_mut(index / 64) {
            *word &= !(1 << (index % 64));
        }
    }

    /// Whether `index` is in the set; an index past the end is not.
    fn contains(&self, index: usize) -> bool {
        self.0
            .get(index / 64)
            .is_some_and(|word| word & (1 << (index % 64)) != 0)
    }
}

/// The recursive components of the call graph, and the component each
/// member is in.
struct Components {
    /// Each component's members, ascending; components ascending by first
    /// member.
    list: Vec<Vec<usize>>,
    /// The position in `list` of each member's component, `None` for a
    /// member in no recursive component.
    of: Vec<Option<usize>>,
}

impl Components {
    /// Index `list` over `count` members. Each component is sorted here,
    /// then the list by first member, so the order holds whatever produced
    /// `list`.
    fn new(mut list: Vec<Vec<usize>>, count: usize) -> Self {
        for component in &mut list {
            component.sort_unstable();
        }
        list.sort_unstable_by_key(|component| component.first().copied());
        let mut of = vec![None; count];
        for (id, component) in list.iter().enumerate() {
            for &member in component {
                if let Some(slot) = of.get_mut(member) {
                    *slot = Some(id);
                }
            }
        }
        Self { list, of }
    }

    fn iter(&self) -> impl Iterator<Item = Component<'_>> {
        self.list.iter().enumerate().map(|(id, members)| Component {
            id,
            members,
            of: &self.of,
        })
    }
}

/// One recursive component: its members, ascending, and an O(1)
/// membership test through the shared member-to-component index.
struct Component<'c> {
    id: usize,
    members: &'c [usize],
    of: &'c [Option<usize>],
}

impl Component<'_> {
    fn contains(&self, index: usize) -> bool {
        self.of.get(index).copied().flatten() == Some(self.id)
    }
}

/// One member on Tarjan's explicit call stack, with the position of the
/// next of its calls to follow.
struct Frame {
    member: usize,
    next_call: usize,
}

/// Tarjan's strongly-connected-component traversal over the call graph.
struct Tarjan<'m, 'a> {
    members: &'m [Member<'a>],
    /// Visit order of each member, `None` until visited.
    order: Vec<Option<usize>>,
    /// Lowest visit order reachable from each member through the members
    /// still on `stack`.
    low: Vec<usize>,
    on_stack: Bits,
    stack: Vec<usize>,
    frames: Vec<Frame>,
    visited: usize,
    recursive: Vec<Vec<usize>>,
}

impl<'m, 'a> Tarjan<'m, 'a> {
    fn new(members: &'m [Member<'a>]) -> Self {
        let count = members.len();
        Self {
            members,
            order: vec![None; count],
            low: vec![0; count],
            on_stack: Bits::new(count),
            stack: Vec::new(),
            frames: Vec::new(),
            visited: 0,
            recursive: Vec::new(),
        }
    }

    fn order_of(&self, member: usize) -> Option<usize> {
        self.order.get(member).copied().flatten()
    }

    fn lower(&mut self, member: usize, to: usize) {
        if let Some(slot) = self.low.get_mut(member) {
            *slot = (*slot).min(to);
        }
    }

    fn visit(&mut self, member: usize) {
        if let Some(slot) = self.order.get_mut(member) {
            *slot = Some(self.visited);
        }
        if let Some(slot) = self.low.get_mut(member) {
            *slot = self.visited;
        }
        self.visited += 1;
        self.on_stack.insert(member);
        self.stack.push(member);
        self.frames.push(Frame {
            member,
            next_call: 0,
        });
    }

    /// Visit everything reachable from `root`, emitting each component as
    /// it closes.
    fn run(&mut self, root: usize) {
        let members = self.members;
        self.visit(root);
        while let Some(frame) = self.frames.last_mut() {
            let current = frame.member;
            let call = members
                .get(current)
                .and_then(|member| member.calls.get(frame.next_call));
            if let Some(call) = call {
                frame.next_call += 1;
                let callee = call.callee;
                if callee >= members.len() {
                    continue;
                }
                match self.order_of(callee) {
                    None => self.visit(callee),
                    Some(callee_order) if self.on_stack.contains(callee) => {
                        self.lower(current, callee_order);
                    }
                    Some(_) => {}
                }
                continue;
            }
            self.frames.pop();
            let current_low = self.low.get(current).copied().unwrap_or(0);
            if let Some(parent) = self.frames.last().map(|frame| frame.member) {
                self.lower(parent, current_low);
            }
            if self.order_of(current) == Some(current_low) {
                self.close(current);
            }
        }
    }

    /// Pop the component rooted at `root` off the stack, keeping it when it
    /// is recursive.
    fn close(&mut self, root: usize) {
        let mut component = Vec::new();
        while let Some(member) = self.stack.pop() {
            self.on_stack.remove(member);
            component.push(member);
            if member == root {
                break;
            }
        }
        let calls_itself = || {
            self.members
                .get(root)
                .is_some_and(|member| member.calls.iter().any(|call| call.callee == root))
        };
        if component.len() > 1 || calls_itself() {
            self.recursive.push(component);
        }
    }
}

/// The recursive components of the call graph -- every strongly connected
/// component with more than one member, or whose single member calls
/// itself -- each with its members ascending, ordered ascending by first
/// member.
///
/// Tarjan's algorithm, O(V + E), run on an explicit stack so that stack
/// depth does not grow with the length of a call chain: FR-146, "a host
/// stack overflow is not a Complete-V1 outcome". A callee index with no
/// member has no calls, so it lies on no cycle and is skipped.
fn recursive_components(members: &[Member<'_>]) -> Components {
    let mut tarjan = Tarjan::new(members);
    for root in 0..members.len() {
        if tarjan.order_of(root).is_none() {
            tarjan.run(root);
        }
    }
    // Tarjan emits components in reverse topological order;
    // `Components::new` puts them in first-declaration order, the order
    // refusals are reported in.
    Components::new(tarjan.recursive, members.len())
}

/// Check every recursive component, in first-declaration order.
pub(crate) fn check(members: &[Member<'_>]) -> Vec<CheckRefusal> {
    check_components(members, &recursive_components(members))
}

/// The refusals of `components`, one at most per component, in order. The
/// refusal-cycle search shares one parent buffer across components, so the
/// whole pass is O(V + E).
fn check_components(members: &[Member<'_>], components: &Components) -> Vec<CheckRefusal> {
    let mut parent: Vec<Option<usize>> = vec![None; members.len()];
    let mut refusals = Vec::new();
    for component in components.iter() {
        if let Some(refusal) = dispatch_cycle_refusal(members, &component) {
            refusals.push(refusal);
            continue;
        }
        if let Err(refusal) = check_component(members, &component, &mut parent) {
            refusals.push(refusal);
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
fn dispatch_cycle_refusal(members: &[Member<'_>], component: &Component) -> Option<CheckRefusal> {
    let mut located: Vec<((usize, usize), Location)> = component
        .members
        .iter()
        .filter_map(|&caller| members.get(caller).map(|member| (caller, member)))
        .flat_map(|(caller, member)| {
            member
                .calls
                .iter()
                .filter(move |call| {
                    call.kind == EdgeKind::Dispatch && component.contains(call.callee)
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
///
/// `parent` is a buffer over every member index, shared across components;
/// only this component's entries are cleared and written, so one search
/// costs O(component), not O(V).
fn path(
    members: &[Member<'_>],
    component: &Component<'_>,
    parent: &mut [Option<usize>],
    from: usize,
    to: usize,
) -> Vec<usize> {
    for &member in component.members {
        if let Some(slot) = parent.get_mut(member) {
            *slot = None;
        }
    }
    let mut pending = VecDeque::from([from]);
    while let Some(current) = pending.pop_front() {
        if current == to {
            break;
        }
        let Some(member) = members.get(current) else {
            continue;
        };
        for call in member.calls {
            let callee = call.callee;
            let unseen = callee != from && parent.get(callee).copied().flatten().is_none();
            if component.contains(callee) && unseen {
                if let Some(slot) = parent.get_mut(callee) {
                    *slot = Some(current);
                }
                pending.push_back(callee);
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

/// A failed component obligation: the caller, the call edge it is about,
/// and the obligation.
type Failure<'m> = (usize, &'m CallSite, MeasureObligation);

fn check_component(
    members: &[Member<'_>],
    component: &Component<'_>,
    parent: &mut [Option<usize>],
) -> Result<(), CheckRefusal> {
    let Err((caller, edge, obligation)) = component_obligations(members, component) else {
        return Ok(());
    };
    let mut cycle: Vec<usize> = vec![caller];
    cycle.extend(path(members, component, parent, edge.callee, caller));
    Err(CheckRefusal {
        location: edge.location.clone(),
        cause: CheckCause::UnprovedDecrease {
            cycle: cycle
                .into_iter()
                .filter_map(|index| members.get(index).map(|member| member.name.to_owned()))
                .collect(),
            obligation,
        },
    })
}

/// The first failed obligation of `component`, in FR-146 check order.
fn component_obligations<'m>(
    members: &'m [Member<'_>],
    component: &Component<'_>,
) -> Result<(), Failure<'m>> {
    let edges: Vec<(usize, &CallSite)> = component
        .members
        .iter()
        .filter_map(|&caller| members.get(caller).map(|member| (caller, member)))
        .flat_map(|(caller, member)| {
            member
                .calls
                .iter()
                .filter(|call| component.contains(call.callee))
                .map(move |call| (caller, call))
        })
        .collect();
    let Some(&(first_caller, first_edge)) = edges.first() else {
        return Ok(());
    };
    let component_refusal = |obligation| (first_caller, first_edge, obligation);
    let mut measures = Vec::with_capacity(component.members.len());
    for &index in component.members {
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
    // `typed` follows `component.members`, which `Components::new` sorted.
    debug_assert!(typed.windows(2).all(|pair| pair[0].0 < pair[1].0));
    let measure_of = |index: usize| {
        typed
            .binary_search_by_key(&index, |(member, _)| *member)
            .ok()
            .and_then(|position| typed.get(position))
            .map(|(_, elements)| elements.as_slice())
    };
    for (caller, edge) in edges {
        let (Some(caller_measure), Some(callee_measure)) =
            (measure_of(caller), measure_of(edge.callee))
        else {
            return Err((caller, edge, MeasureObligation::Decrease));
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
            return Err((caller, edge, MeasureObligation::Decrease));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use ix_trace_rs::trace;
    use quire_exact::{Integer, IntegerInterval, NodeKey};

    use super::super::refusal::Origin;
    use super::*;

    /// The pre-QSL-203 component partition, kept only as this module's
    /// differential oracle: a reachability set per member, then an
    /// all-pairs mutual-reachability filter, O(V^2) time and memory.
    fn closure_components(members: &[Member<'_>]) -> Vec<Vec<usize>> {
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
        let mut components = Vec::new();
        for first in 0..count {
            if assigned[first] {
                continue;
            }
            let component: Vec<usize> = (0..count)
                .filter(|&other| {
                    other == first
                        || (reachable[first].contains(&other) && reachable[other].contains(&first))
                })
                .collect();
            for &index in &component {
                assigned[index] = true;
            }
            if component.len() > 1 || reachable[first].contains(&first) {
                components.push(component);
            }
        }
        components
    }

    /// `check` as it was before QSL-203: the oracle partition, then the
    /// same per-component obligations.
    fn closure_check(members: &[Member<'_>]) -> Vec<CheckRefusal> {
        check_components(
            members,
            &Components::new(closure_components(members), members.len()),
        )
    }

    /// SplitMix64: a fixed-seed generator, so every generated graph is
    /// reproducible from its seed.
    struct SplitMix(u64);

    impl SplitMix {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        }

        fn below(&mut self, bound: usize) -> usize {
            usize::try_from(self.next() % u64::try_from(bound).unwrap()).unwrap()
        }

        fn chance(&mut self, percent: u64) -> bool {
            self.next() % 100 < percent
        }
    }

    fn call(callee: usize, kind: EdgeKind, position: usize) -> CallSite {
        CallSite {
            callee,
            location: Location {
                origin: Origin::Expression,
                path: vec![position],
            },
            arguments: Vec::new(),
            kind,
        }
    }

    /// The call lists of one generated graph shape.
    fn graph(seed: u64) -> Vec<Vec<CallSite>> {
        let mut rng = SplitMix(seed);
        let count = 1 + rng.below(40);
        let mut calls: Vec<Vec<CallSite>> = vec![Vec::new(); count];
        let mut add = |rng: &mut SplitMix, from: usize, to: usize| {
            let kind = if rng.chance(10) {
                EdgeKind::Dispatch
            } else {
                EdgeKind::Ordinary
            };
            let position = calls[from].len();
            let mut site = call(to, kind, position);
            site.arguments = (0..PARAMETERS)
                .map(|_| match rng.below(5) {
                    0 => ArgumentShape::Parameter(0),
                    1 => ArgumentShape::Decremented(0),
                    2 => ArgumentShape::Parameter(1),
                    3 => ArgumentShape::Decremented(1),
                    _ => ArgumentShape::Other,
                })
                .collect();
            calls[from].push(site);
        };
        match seed % 4 {
            // Sparse random edges, with self-loops and repeated edges.
            0 => {
                for _ in 0..rng.below(2 * count + 1) {
                    let (from, to) = (rng.below(count), rng.below(count));
                    add(&mut rng, from, to);
                }
            }
            // Several disjoint rings, linked forward, so there are multiple
            // components with edges between them.
            1 => {
                let mut start = 0;
                while start < count {
                    let end = (start + 1 + rng.below(6)).min(count);
                    for member in start..end {
                        let next = if member + 1 < end { member + 1 } else { start };
                        if end - start > 1 || rng.chance(50) {
                            add(&mut rng, member, next);
                        }
                    }
                    if end < count && rng.chance(70) {
                        let (from, to) =
                            (start + rng.below(end - start), end + rng.below(count - end));
                        add(&mut rng, from, to);
                    }
                    start = end;
                }
            }
            // A long chain, declared forwards or backwards, optionally closed
            // into a cycle somewhere along it.
            2 => {
                let backwards = rng.chance(50);
                for member in 1..count {
                    let (from, to) = if backwards {
                        (member, member - 1)
                    } else {
                        (member - 1, member)
                    };
                    add(&mut rng, from, to);
                }
                if rng.chance(60) {
                    let (from, to) = (rng.below(count), rng.below(count));
                    add(&mut rng, from, to);
                }
            }
            // Dense random edges: mostly one large component.
            _ => {
                for from in 0..count {
                    for to in 0..count {
                        if rng.chance(15) {
                            add(&mut rng, from, to);
                        }
                    }
                }
            }
        }
        calls
    }

    fn members<'a>(names: &'a [String], calls: &'a [Vec<CallSite>]) -> Vec<Member<'a>> {
        names
            .iter()
            .zip(calls)
            .map(|(name, calls)| Member {
                name,
                parameters: &[],
                measure: None,
                calls,
            })
            .collect()
    }

    /// Every generated member's parameters: `n: Int[0, 9]` (a nonnegative
    /// integer element), `m: Integer` (an integer element that is not
    /// nonnegative) and `b: Boolean` (not a measure element).
    const PARAMETERS: usize = 3;

    fn parameters() -> Vec<(String, ValueType)> {
        let bounded = IntegerInterval::new(Integer::from(0_i64), Integer::from(9_i64)).unwrap();
        vec![
            ("n".to_owned(), ValueType::Int(bounded)),
            ("m".to_owned(), ValueType::Integer),
            ("b".to_owned(), ValueType::Boolean),
        ]
    }

    fn local(slot: Slot) -> Node {
        Node {
            kind: NodeKind::Local(slot),
            value_type: ValueType::Integer,
            location: Location {
                origin: Origin::Expression,
                path: Vec::new(),
            },
        }
    }

    /// Per member, a measure drawn so that every obligation can fail: a
    /// missing measure, `n` or `m` alone, the pair `(n, m)` (another
    /// arity), or `b` (not an element). Odd seeds have no measures at all.
    fn measures(seed: u64, count: usize) -> Vec<Option<Node>> {
        let mut rng = SplitMix(seed ^ 0x5EED);
        (0..count)
            .map(|_| {
                if seed % 2 == 1 {
                    return None;
                }
                match rng.below(20) {
                    0 => None,
                    1..=10 => Some(local(0)),
                    11..=13 => Some(local(1)),
                    14..=17 => Some(Node {
                        kind: NodeKind::Tuple {
                            declaration: NodeKey::from_digest([3; 32]),
                            arguments: vec![local(0), local(1)],
                        },
                        value_type: ValueType::Integer,
                        location: local(0).location,
                    }),
                    _ => Some(local(2)),
                }
            })
            .collect()
    }

    fn measured_members<'a>(
        names: &'a [String],
        parameters: &'a [(String, ValueType)],
        measures: &'a [Option<Node>],
        calls: &'a [Vec<CallSite>],
    ) -> Vec<Member<'a>> {
        names
            .iter()
            .zip(measures)
            .zip(calls)
            .map(|((name, measure), calls)| Member {
                name,
                parameters,
                measure: measure.as_ref(),
                calls,
            })
            .collect()
    }

    fn names(count: usize) -> Vec<String> {
        (0..count).map(|index| format!("f{index}")).collect()
    }

    /// QSL-203 AC 2: on generated call graphs -- random, ringed, chained and
    /// dense, with self-loops, repeated edges, multiple components and
    /// dispatch edges, and with measures that fail each obligation or none --
    /// Tarjan's components and every termination refusal, in order, equal
    /// the pre-QSL-203 closure-and-filter algorithm's.
    #[trace("TC-191", "FR-146-AC-4")]
    #[trace("TC-191", "FR-146-AC-7")]
    #[test]
    fn tarjan_components_and_refusals_match_the_closure_oracle() {
        let mut recursive_components_seen = 0;
        let mut multi_member_components = 0;
        let mut refusals_seen = 0;
        let mut obligations_seen: BTreeSet<&'static str> = BTreeSet::new();
        let parameters = parameters();
        for seed in 0..4_000 {
            let calls = graph(seed);
            let names = names(calls.len());
            let measures = measures(seed, calls.len());
            let members = measured_members(&names, &parameters, &measures, &calls);
            let expected = closure_components(&members);
            assert_eq!(recursive_components(&members).list, expected, "seed {seed}");
            let refusals = check(&members);
            assert_eq!(refusals, closure_check(&members), "seed {seed}");
            recursive_components_seen += expected.len();
            multi_member_components += expected.iter().filter(|c| c.len() > 1).count();
            refusals_seen += refusals.len();
            for refusal in &refusals {
                obligations_seen.insert(match &refusal.cause {
                    CheckCause::UnprovedDecrease { obligation, .. } => obligation.as_str(),
                    CheckCause::DefinitionCycle { .. } => "definition-cycle",
                    other => panic!("unexpected refusal {other:?}"),
                });
            }
        }
        assert_eq!(
            obligations_seen.into_iter().collect::<Vec<_>>(),
            [
                "decrease",
                "definition-cycle",
                "measure-arity",
                "measure-kind",
                "missing-measure",
                "nonnegative"
            ]
        );
        // The generator reaches the shapes the comparison is about.
        assert!(
            recursive_components_seen > 4_000,
            "{recursive_components_seen}"
        );
        assert!(multi_member_components > 2_000, "{multi_member_components}");
        assert!(refusals_seen > 4_000, "{refusals_seen}");
    }

    /// A self-call is a recursive singleton; a member on no cycle is not a
    /// component; components come back ascending by first member although
    /// Tarjan closes the later one first.
    #[trace("TC-191", "FR-146-AC-7")]
    #[test]
    fn components_are_recursive_only_and_in_first_member_order() {
        // f0 -> f3, f1 -> f1, f2 -> f4 -> f2, f3 -> f2
        let calls = vec![
            vec![call(3, EdgeKind::Ordinary, 0)],
            vec![call(1, EdgeKind::Ordinary, 0)],
            vec![call(4, EdgeKind::Ordinary, 0)],
            vec![call(2, EdgeKind::Ordinary, 0)],
            vec![call(2, EdgeKind::Ordinary, 0)],
        ];
        let names = names(calls.len());
        assert_eq!(
            recursive_components(&members(&names, &calls)).list,
            vec![vec![1], vec![2, 4]]
        );
    }

    /// FR-146: "a host stack overflow is not a Complete-V1 outcome". A
    /// 200,000-member chain and a 200,000-member ring are traversed on a
    /// 512 KiB thread stack, which a recursive Tarjan would overflow.
    #[trace("TC-191", "FR-146-AC-7")]
    #[test]
    fn a_long_chain_and_ring_do_not_grow_the_stack() {
        const LENGTH: usize = 200_000;
        std::thread::Builder::new()
            .stack_size(512 * 1024)
            .spawn(|| {
                let chain: Vec<Vec<CallSite>> = (0..LENGTH)
                    .map(|member| {
                        if member + 1 < LENGTH {
                            vec![call(member + 1, EdgeKind::Ordinary, 0)]
                        } else {
                            Vec::new()
                        }
                    })
                    .collect();
                let names = names(LENGTH);
                assert!(recursive_components(&members(&names, &chain))
                    .list
                    .is_empty());
                let ring: Vec<Vec<CallSite>> = (0..LENGTH)
                    .map(|member| vec![call((member + 1) % LENGTH, EdgeKind::Ordinary, 0)])
                    .collect();
                let components = recursive_components(&members(&names, &ring)).list;
                assert_eq!(components.len(), 1);
                assert_eq!(components[0], (0..LENGTH).collect::<Vec<_>>());
            })
            .unwrap()
            .join()
            .unwrap();
    }
}

/// QSL-203 review finding 2: informal timing of `check` alone on N
/// self-recursive members, each its own refused component. Wall-clock, so
/// `#[ignore]`d out of the default run; run with
/// `cargo test --release -p qsl-semantics termination_scaling -- --ignored --nocapture`.
#[cfg(test)]
mod scaling {
    use super::super::refusal::Origin;
    use super::*;

    #[test]
    #[ignore = "timing lane: prints wall time, asserts only the refusal count"]
    fn termination_scaling_over_self_recursive_members() {
        for count in [5_000, 10_000, 20_000, 40_000] {
            let names: Vec<String> = (0..count).map(|index| format!("f{index}")).collect();
            let calls: Vec<Vec<CallSite>> = (0..count)
                .map(|index| {
                    vec![CallSite {
                        callee: index,
                        location: Location {
                            origin: Origin::Expression,
                            path: Vec::new(),
                        },
                        arguments: Vec::new(),
                        kind: EdgeKind::Ordinary,
                    }]
                })
                .collect();
            let members: Vec<Member<'_>> = names
                .iter()
                .zip(&calls)
                .map(|(name, calls)| Member {
                    name,
                    parameters: &[],
                    measure: None,
                    calls,
                })
                .collect();
            let start = std::time::Instant::now();
            let refusals = check(&members);
            let elapsed = start.elapsed();
            assert_eq!(refusals.len(), count);
            println!(
                "termination self-recursive members={count} wall_ms={:.1}",
                elapsed.as_secs_f64() * 1e3
            );
        }
    }
}
