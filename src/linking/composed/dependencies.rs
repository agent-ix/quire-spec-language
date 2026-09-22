// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-036: bounded native declaration references, cycles and refusal propagation.
use super::work::{Dimension, Exhaustion, Work};
use super::{DeclarationId, DeclarationRefusal, SyntaxNamespace, UnitId};
use crate::syntax::composed::{
    ComposedUnit, ControlId, ControlKind, DeclarationKind, EventKind, ProtocolRequirement,
    ValueKind,
};
use crate::syntax::{ClauseKind, ExprId};
use qsl_foundation::{Span, Spanned};
use std::collections::VecDeque;

/// The native declaration kind required by one authored reference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DependencyKind {
    /// A shared predicate invoked from any value-expression environment.
    PredicateCall,
    /// A protocol requirement selecting a temporal declaration.
    TemporalRequirement,
    /// A protocol attempt selecting a state precondition or postcondition.
    OperationContract,
}

/// Syntax owning an occurrence; handles remain local to the reference's unit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DependencySite {
    /// A predicate invocation in the unit's flat value arena.
    Expression(ExprId),
    /// An operation-contract name on an attempt control node.
    Control(ControlId),
    /// A temporal requirement in the owning protocol declaration.
    Declaration,
}

/// One authored reference, retaining its original spelling and source token span.
/// A selected target establishes its native name/kind, not model or value typing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyReference {
    /// The source unit qualifying both the span and any expression/control handle.
    pub unit: UnitId,
    /// Required native declaration kind.
    pub kind: DependencyKind,
    /// Typed syntax occurrence in the original unit.
    pub site: DependencySite,
    /// Authored name token; source spelling is never reconstructed from a target.
    pub name: Spanned<String>,
    /// Unique package target, including a wrong-kind target when one was found.
    pub target: Option<DeclarationId>,
}

/// A dependency-local refusal. `reference` indexes its owning entry's references.
/// Direct targets retain causal chains without copying a whole cycle into each entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DependencyRefusal {
    /// No package declaration supplies the authored name.
    MissingTarget {
        /// Index into the owning declaration's references.
        reference: usize,
    },
    /// Namespace lookup retains multiple conflicting declaration candidates.
    AmbiguousTarget {
        /// Index into the owning declaration's references.
        reference: usize,
    },
    /// The unique declaration has a different kind than this occurrence requires.
    WrongTargetKind {
        /// Index into the owning declaration's references.
        reference: usize,
        /// The unique declaration found, whose kind does not satisfy the reference.
        target: DeclarationId,
    },
    /// This reference is an edge within a semantic declaration cycle.
    Cycle {
        /// Index into the owning declaration's references.
        reference: usize,
        /// The declaration this edge leads to within the cycle.
        target: DeclarationId,
    },
    /// A directly required declaration refused, possibly through another dependency.
    RefusedTarget {
        /// Index into the owning declaration's references.
        reference: usize,
        /// The required declaration whose refusal propagated through this reference.
        target: DeclarationId,
    },
}

pub(super) fn resolve(namespace: &mut SyntaxNamespace, work: &mut Work) -> Result<(), Exhaustion> {
    // These slots belong to the already charged, closed declaration inventory.
    let mut owners: Vec<Vec<Option<DeclarationId>>> = namespace
        .units
        .iter()
        .map(|unit| vec![None; unit.declarations().len()])
        .collect();
    for (index, entry) in namespace.declarations.iter().enumerate() {
        owners[entry.unit.0][entry.declaration] = Some(DeclarationId(index));
    }
    let mut graph = Dependencies::new(namespace.declarations.len());
    for (unit_index, unit_owners) in owners.iter().enumerate() {
        let unit_id = UnitId(unit_index);
        let mut declaration = 0;
        for index in 0..namespace.units[unit_index].expressions().len() {
            work.charge(Dimension::References, 1)?;
            let reference = {
                let unit = &namespace.units[unit_index];
                let expression = &unit.expressions()[index];
                declaration_at(unit, expression.span, &mut declaration);
                if let ValueKind::Invoke { name, .. } = &expression.kind {
                    work.charge(Dimension::References, 1)?;
                    Some(DependencyReference {
                        unit: unit_id,
                        kind: DependencyKind::PredicateCall,
                        site: DependencySite::Expression(ExprId(index)),
                        name: name.clone(),
                        target: None,
                    })
                } else {
                    None
                }
            };
            if let Some(reference) = reference {
                record(
                    namespace,
                    unit_owners[declaration].expect("closed declaration inventory"),
                    reference,
                    &mut graph,
                    work,
                )?;
            }
        }

        declaration = 0;
        for index in 0..namespace.units[unit_index].controls().len() {
            work.charge(Dimension::References, 1)?;
            let count = {
                let unit = &namespace.units[unit_index];
                let control = &unit.controls()[index];
                declaration_at(unit, control.span, &mut declaration);
                contracts(&control.kind).len()
            };
            for contract in 0..count {
                work.charge(Dimension::References, 1)?;
                let reference = DependencyReference {
                    unit: unit_id,
                    kind: DependencyKind::OperationContract,
                    site: DependencySite::Control(ControlId(index)),
                    name: contracts(&namespace.units[unit_index].controls()[index].kind)[contract]
                        .clone(),
                    target: None,
                };
                record(
                    namespace,
                    unit_owners[declaration].expect("closed declaration inventory"),
                    reference,
                    &mut graph,
                    work,
                )?;
            }
        }

        for (index, owner) in unit_owners.iter().enumerate() {
            let count = requirements(&namespace.units[unit_index].declarations()[index].kind).len();
            for requirement in 0..count {
                work.charge(Dimension::References, 1)?;
                let reference = {
                    let requirements =
                        requirements(&namespace.units[unit_index].declarations()[index].kind);
                    match &requirements[requirement] {
                        ProtocolRequirement::Temporal { name, .. } => {
                            work.charge(Dimension::References, 1)?;
                            Some(DependencyReference {
                                unit: unit_id,
                                kind: DependencyKind::TemporalRequirement,
                                site: DependencySite::Declaration,
                                name: name.clone(),
                                target: None,
                            })
                        }
                        ProtocolRequirement::Compensation(_) => None,
                    }
                };
                if let Some(reference) = reference {
                    record(
                        namespace,
                        owner.expect("closed declaration inventory"),
                        reference,
                        &mut graph,
                        work,
                    )?;
                }
            }
        }
    }
    let components = graph.components(work)?;
    graph.refuse_cycles(namespace, &components, work)?;
    graph.propagate(namespace, work)
}

// Parser arenas are postorder within each source-ordered declaration. Advancing
// between declaration spans therefore visits each declaration at most once per
// arena, without interpreting expression text or treating an ExprId as global.
fn declaration_at(unit: &ComposedUnit, span: Span, cursor: &mut usize) {
    let declarations = unit.declarations();
    while span.start >= declarations[*cursor].span.end {
        *cursor += 1;
    }
    debug_assert!(span.start >= declarations[*cursor].span.start);
    debug_assert!(span.end <= declarations[*cursor].span.end);
}

fn contracts(kind: &ControlKind) -> &[Spanned<String>] {
    match kind {
        ControlKind::Event(event) => match &event.kind {
            EventKind::Attempt { contracts, .. } => contracts,
            EventKind::Send { .. }
            | EventKind::Receive { .. }
            | EventKind::Effect { .. }
            | EventKind::Event { .. } => &[],
        },
        ControlKind::Sequence(_)
        | ControlKind::Choice { .. }
        | ControlKind::Parallel { .. }
        | ControlKind::Repeat { .. }
        | ControlKind::Await { .. }
        | ControlKind::Check { .. }
        | ControlKind::Commit { .. } => &[],
    }
}

fn requirements(kind: &DeclarationKind) -> &[ProtocolRequirement] {
    match kind {
        DeclarationKind::Protocol(protocol) => &protocol.requirements,
        DeclarationKind::Predicate { .. }
        | DeclarationKind::State { .. }
        | DeclarationKind::Temporal { .. } => &[],
    }
}

impl DependencyKind {
    fn accepts(self, kind: &DeclarationKind) -> bool {
        match self {
            Self::PredicateCall => matches!(kind, DeclarationKind::Predicate { .. }),
            Self::TemporalRequirement => matches!(kind, DeclarationKind::Temporal { .. }),
            Self::OperationContract => matches!(
                kind,
                DeclarationKind::State {
                    kind: ClauseKind::Precondition | ClauseKind::Postcondition,
                    ..
                }
            ),
        }
    }
}

// The caller charges the resolution attempt before copying or looking up its
// name. Each authored occurrence is retained, including repeated references.
fn record(
    namespace: &mut SyntaxNamespace,
    owner: DeclarationId,
    mut occurrence: DependencyReference,
    graph: &mut Dependencies,
    work: &mut Work,
) -> Result<(), Exhaustion> {
    let reference = namespace.declarations[owner.0].references.len();
    let refusal = match namespace.lookup(&occurrence.name.value) {
        [] => Some(DependencyRefusal::MissingTarget { reference }),
        [target] => {
            let target = *target;
            occurrence.target = Some(target);
            if occurrence.kind.accepts(
                &namespace
                    .syntax(target)
                    .expect("closed declaration target")
                    .kind,
            ) {
                None
            } else {
                Some(DependencyRefusal::WrongTargetKind { reference, target })
            }
        }
        _ => Some(DependencyRefusal::AmbiguousTarget { reference }),
    };
    let target = occurrence.target;
    namespace.declarations[owner.0].references.push(occurrence);
    if let Some(refusal) = refusal {
        namespace.declarations[owner.0]
            .refusals
            .push(DeclarationRefusal::Dependency(refusal));
    } else if let Some(target) = target {
        work.charge(Dimension::DependencyEdges, 1)?;
        graph.insert(owner, target, reference);
    }
    Ok(())
}

struct Edge {
    owner: DeclarationId,
    target: DeclarationId,
    reference: usize,
}

// Only package declaration dependencies enter this graph. Repetition, node paths
// and model instance relationships are syntax for different later judgments.
struct Dependencies {
    edges: Vec<Edge>,
    outgoing: Vec<Vec<usize>>,
    incoming: Vec<Vec<usize>>,
}

impl Dependencies {
    fn new(declarations: usize) -> Self {
        Self {
            edges: Vec::new(),
            outgoing: vec![Vec::new(); declarations],
            incoming: vec![Vec::new(); declarations],
        }
    }

    fn insert(&mut self, owner: DeclarationId, target: DeclarationId, reference: usize) {
        let index = self.edges.len();
        self.edges.push(Edge {
            owner,
            target,
            reference,
        });
        self.outgoing[owner.0].push(index);
        self.incoming[target.0].push(index);
    }

    // Iterative Kosaraju: every edge occurrence is charged once in each pass,
    // even when its target has already been visited through a shared dependency.
    fn components(&self, work: &mut Work) -> Result<Vec<usize>, Exhaustion> {
        let mut visited = vec![false; self.outgoing.len()];
        let mut finished = Vec::new();
        for root in 0..self.outgoing.len() {
            if visited[root] {
                continue;
            }
            visited[root] = true;
            let mut stack = vec![(root, 0)];
            while let Some((node, cursor)) = stack.last_mut() {
                if let Some(edge) = self.outgoing[*node].get(*cursor) {
                    work.charge(Dimension::DependencyEdges, 1)?;
                    *cursor += 1;
                    let next = self.edges[*edge].target.0;
                    if !visited[next] {
                        visited[next] = true;
                        stack.push((next, 0));
                    }
                } else {
                    finished.push(*node);
                    stack.pop();
                }
            }
        }
        let mut components = vec![None; self.outgoing.len()];
        for root in finished.into_iter().rev() {
            if components[root].is_some() {
                continue;
            }
            components[root] = Some(root);
            let mut stack = vec![root];
            while let Some(node) = stack.pop() {
                for edge in &self.incoming[node] {
                    work.charge(Dimension::DependencyEdges, 1)?;
                    let next = self.edges[*edge].owner.0;
                    if components[next].is_none() {
                        components[next] = Some(root);
                        stack.push(next);
                    }
                }
            }
        }
        Ok(components
            .into_iter()
            .map(|component| component.expect("visited declaration"))
            .collect())
    }

    fn refuse_cycles(
        &self,
        namespace: &mut SyntaxNamespace,
        components: &[usize],
        work: &mut Work,
    ) -> Result<(), Exhaustion> {
        // Equal components suffice: every retained internal edge of a singleton
        // is a self edge, and each internal edge of a larger SCC lies on a cycle.
        for edge in &self.edges {
            work.charge(Dimension::DependencyEdges, 1)?;
            if components[edge.owner.0] == components[edge.target.0] {
                namespace.declarations[edge.owner.0]
                    .refusals
                    .push(DeclarationRefusal::Dependency(DependencyRefusal::Cycle {
                        reference: edge.reference,
                        target: edge.target,
                    }));
            }
        }
        Ok(())
    }

    fn propagate(
        &self,
        namespace: &mut SyntaxNamespace,
        work: &mut Work,
    ) -> Result<(), Exhaustion> {
        let mut refused: Vec<bool> = namespace
            .declarations
            .iter()
            .map(|entry| !entry.refusals.is_empty())
            .collect();
        let mut pending: VecDeque<usize> = refused
            .iter()
            .enumerate()
            .filter_map(|(index, refused)| refused.then_some(index))
            .collect();
        while let Some(target) = pending.pop_front() {
            for edge in &self.incoming[target] {
                work.charge(Dimension::DependencyEdges, 1)?;
                let edge = &self.edges[*edge];
                if !refused[edge.owner.0] {
                    refused[edge.owner.0] = true;
                    namespace.declarations[edge.owner.0].refusals.push(
                        DeclarationRefusal::Dependency(DependencyRefusal::RefusedTarget {
                            reference: edge.reference,
                            target: edge.target,
                        }),
                    );
                    pending.push_back(edge.owner.0);
                }
            }
        }
        Ok(())
    }
}
