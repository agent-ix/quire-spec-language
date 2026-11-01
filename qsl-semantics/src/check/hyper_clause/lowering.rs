// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-487: the checked Boolean condition of each `holds` atom and `match`
//! conjunct of a `hyper` clause over `behaviours`.
//!
//! An atom is typed with the value typer over the clause's atom slots, one
//! per (trace variable, object parameter) pair of one alias, as FR-337
//! types a temporal atom over its `over` slot. A name of an object parameter
//! is respelled to its slot before typing: the slot of the trace variable of
//! the innermost indexed read around it, else of the first trace variable of
//! its alias. Typing then reads `e @ t` as `e`. The component each model
//! read takes is recorded by the read's location, and definedness keeps each
//! component a separate observation.
//!
//! A conjunct is typed over its distinct step-label reads, each a slot; a
//! label is a `String` there, `stutter` and a label constant are `String`
//! literals.

use quire_checked::key::DeclarationKey;
use std::collections::{BTreeMap, BTreeSet};

use qsl_forms::{ExprId, ExprNode, Expression, ExpressionSpans, StepLabelForm};
use qsl_foundation::Span;
use quire_exact::{Cancel, EffectiveId, ValueType};
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::location::{Location, Origin};
use quire_semantic_value::quantity::UnitTable;

use crate::check::abstraction::OperationKey;
use crate::check::check::{bind_parameters, HyperTyping, NamedScope, Scope, StateContext, Typer};
use crate::check::facts::Definedness;
use crate::check::ir::{CheckedBody, CheckedNode};
use crate::check::observation::Observations;
use crate::check::refusal::{CheckCause, CheckRefusal};
use crate::check::{AdmittedModel, DispatchTable, Signatures};
use crate::family::CheckContext;
use crate::model::clause_observation::Observation;

use super::StepRead;

/// The text a `String` label holds at a stutter step, and `stutter` lowers to.
pub const STUTTER_TEXT: &str = "stutter";

/// A slot no local carries: a condition has no `self`.
const NO_SLOT: usize = usize::MAX;

/// The name a conjunct's slot for the step label `label` of `trace` is bound
/// under. It holds a `.`, which no identifier does, so it names no object
/// parameter or `let`.
pub(crate) fn step_slot_name(trace: &str, label: &StepLabelForm) -> String {
    match label {
        StepLabelForm::Operation => format!("{trace}.step.op"),
        StepLabelForm::Receiver => format!("{trace}.step.receiver"),
        StepLabelForm::Argument {
            operation,
            parameter,
        } => format!("{trace}.step.{operation}.{parameter}"),
    }
}

/// The name an atom slot is bound under: the object parameter `parameter` in
/// the component of `trace`. It holds an `@`, which no identifier does.
fn atom_slot_name(parameter: &str, trace: &str) -> String {
    format!("{parameter}@{trace}")
}

/// The operation text a transition of `operation` carries
/// (`ModelTransition::operation`): the RFC 8785 text of the identity of the
/// operation declaration, which `node` of its declaring type extends with the
/// operation's name.
pub fn operation_text(operation: &OperationKey) -> String {
    crate::model::key::declaration_identity_text(&DeclarationKey {
        package: operation.declaring.package.clone(),
        node: format!(
            "{}/{}",
            operation.declaring.node,
            operation.operation.as_str()
        ),
    })
}

/// One slot of every atom of a clause (FR-487 "Atom slots"): the object
/// parameter `parameter` in the component of the trace variable `trace`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AtomSlot {
    /// The trace variable's index in quantifier order.
    pub trace: usize,
    /// The object parameter's index in source order.
    pub parameter: usize,
}

/// The checked condition of one `holds` atom and what it reads.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AtomCondition {
    pub(crate) body: CheckedBody,
    pub(crate) slots: usize,
    /// The name each slot of the condition was bound under.
    pub(crate) slot_names: Vec<String>,
    /// The compound units typing formed, which lowering keys.
    pub(crate) formed_units: UnitTable,
    /// The span of each model read of the condition, by the read's location.
    pub(crate) read_spans: BTreeMap<Location, Span>,
    /// The component each model read of the condition reads, by the read's
    /// location.
    pub(crate) components: BTreeMap<Location, usize>,
}

impl AtomCondition {
    pub(crate) fn root(&self) -> CheckedNode<'_> {
        self.body.root()
    }
}

/// The checked condition of one `match` conjunct.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ConjunctCondition {
    pub(crate) body: CheckedBody,
    pub(crate) slots: usize,
    /// The name each slot of the condition was bound under.
    pub(crate) slot_names: Vec<String>,
    /// The compound units typing formed, which lowering keys.
    pub(crate) formed_units: UnitTable,
}

/// What a condition is typed against.
pub(crate) struct Typing<'a> {
    pub(crate) scope: &'a Scope,
    pub(crate) signatures: &'a Signatures,
    pub(crate) dispatch_tables: &'a [DispatchTable],
    pub(crate) models: &'a [AdmittedModel],
    pub(crate) limits: CheckingLimits,
    pub(crate) cancel: Option<&'a Cancel>,
}

/// A part of a clause a condition is located at (FR-487 "Lowering a holds
/// atom", "Lowering a match conjunct").
#[derive(Clone, Copy, Debug)]
pub(crate) enum Site {
    /// The `holds` atom at this formula arena index.
    Atom(usize),
    /// The universal conjunct at this index.
    Universal(usize),
    /// The existential conjunct at this index.
    Existential(usize),
    /// The clause's declared name: its `hyperproperty` `claim` occurrence.
    Claim,
    /// The body wrapper's `claim` occurrence (FR-488 "The body").
    Body,
    /// The formula node at this arena index.
    Node(usize),
    /// The atom slot at this index.
    Slot(usize),
    /// The slot at the second index of the universal conjunct at the first.
    UniversalSlot(usize, usize),
    /// The slot at the second index of the existential conjunct at the first.
    ExistentialSlot(usize, usize),
    /// The `align skip` operation at this index.
    AlignSkip(usize),
    /// A relation claim, indexed among model relations.
    Relation(usize),
    /// A model relation's body.
    RelationBody(usize),
    /// An execution entry of a model relation.
    RelationExecution(usize, usize),
}

/// The `Origin::TypeDeclaration` name of `site` of the hyper clause at
/// `clause`, its index among the unit's hyper clauses over `behaviours`.
/// Spaced, so it names no declared type.
pub(crate) fn hyper_site(clause: usize, site: Site) -> String {
    match site {
        Site::Atom(node) => format!("hyper {clause} holds {node}"),
        Site::Universal(index) => format!("hyper {clause} match {index}"),
        Site::Existential(index) => format!("hyper {clause} match exists {index}"),
        Site::Claim => format!("hyper {clause}"),
        Site::Body => format!("hyper {clause} body"),
        Site::Node(node) => format!("hyper {clause} node {node}"),
        Site::Slot(slot) => format!("hyper {clause} slot {slot}"),
        Site::UniversalSlot(index, slot) => format!("hyper {clause} match {index} slot {slot}"),
        Site::ExistentialSlot(index, slot) => {
            format!("hyper {clause} match exists {index} slot {slot}")
        }
        Site::AlignSkip(index) => format!("hyper {clause} align {index}"),
        Site::Relation(index) => format!("relation {index}"),
        Site::RelationBody(index) => format!("relation {index} body"),
        Site::RelationExecution(index, execution) => {
            format!("relation {index} execution {execution}")
        }
    }
}

/// The root location of `site` of the hyper clause at `clause`.
pub(crate) fn hyper_location(clause: usize, site: Site) -> Location {
    Location::root(Origin::TypeDeclaration {
        name: hyper_site(clause, site),
    })
}

/// The expression nodes' enclosing indexed read: for each node, the indexed
/// node `e @ t` whose operand contains it and which no inner indexed read
/// separates from it. `None` for a node outside every `@`.
pub(crate) fn enclosing_reads(expression: &Expression) -> Vec<Option<ExprId>> {
    let mut enclosing = vec![None; expression.len()];
    let mut pending = vec![(expression.root_id(), None)];
    while let Some((id, around)) = pending.pop() {
        if let Some(slot) = enclosing.get_mut(id.index()) {
            *slot = around;
        }
        let node = expression.at(id);
        let inside = match node.node() {
            ExprNode::Indexed { .. } => Some(id),
            _ => around,
        };
        pending.extend(
            node.node()
                .children()
                .into_iter()
                .map(|child| (child, inside)),
        );
    }
    enclosing
}

/// The node `path` reaches from the root of `expression`.
pub(crate) fn node_at(expression: &Expression, path: &[usize]) -> Option<ExprId> {
    let mut id = expression.root_id();
    for index in path {
        id = *expression.at(id).node().children().get(*index)?;
    }
    Some(id)
}

/// The clause's atom slots and the type of each (FR-487 "Atom slots").
pub(crate) struct AtomSlots {
    slots: Vec<AtomSlot>,
    names: Vec<String>,
    types: Vec<ValueType>,
}

impl AtomSlots {
    /// The slots of `trace_aliases` (each trace variable's alias, in
    /// quantifier order) and `parameters` (each object parameter's name,
    /// alias and object type, in source order).
    pub(crate) fn of(traces: &[(&str, &str)], parameters: &[(&str, &str, EffectiveId)]) -> Self {
        let mut slots = Vec::new();
        let mut names = Vec::new();
        let mut types = Vec::new();
        for (trace, (variable, alias)) in traces.iter().enumerate() {
            for (parameter, (name, parameter_alias, object)) in parameters.iter().enumerate() {
                if parameter_alias == alias {
                    slots.push(AtomSlot { trace, parameter });
                    names.push(atom_slot_name(name, variable));
                    types.push(ValueType::Reference(object.clone()));
                }
            }
        }
        Self {
            slots,
            names,
            types,
        }
    }

    pub(crate) fn slots(&self) -> &[AtomSlot] {
        &self.slots
    }

    fn bound(&self) -> Vec<(String, ValueType)> {
        self.names
            .iter()
            .cloned()
            .zip(self.types.iter().cloned())
            .collect()
    }

    fn has(&self, name: &str) -> bool {
        self.names.iter().any(|known| known == name)
    }
}

/// A refusal of a condition, with the span of the expression it is at.
pub(crate) struct Refused {
    pub(crate) refusal: Box<CheckRefusal>,
    pub(crate) span: Span,
}

/// The span of the expression node a location of a condition reaches.
pub(crate) type Locate<'a> = &'a dyn Fn(&Location) -> Span;

/// The span of the node `location` reaches in the expression `spans` locates.
pub(crate) fn span_at(spans: &ExpressionSpans, location: &Location) -> Span {
    spans.at(&location.path()).unwrap_or_else(|| {
        spans
            .span(spans.root())
            .unwrap_or(Span { start: 0, end: 0 })
    })
}

/// A typer over `typing` that has bound `bound` as its first slots.
fn typer<'a>(
    typing: &Typing<'a>,
    cx: &'a mut CheckContext<'_, Scope>,
    nodes: &'a mut u64,
    bound: &[(String, ValueType)],
    hyper: HyperTyping,
    location: &Location,
) -> Result<Typer<'a>, CheckRefusal> {
    let named = NamedScope::of(typing.models, typing.scope);
    let mut typer = Typer::new(
        typing.scope,
        typing.signatures,
        typing.limits,
        nodes,
        qsl_forms::ClauseKind::Invariant,
    )
    .with_cancel(typing.cancel)
    .with_context(cx.reborrow(typing.scope))
    .with_named_populations(named)
    .with_hyper(hyper);
    bind_parameters(&mut typer, bound, location)?;
    typer.enter_state_clause(StateContext {
        parent_projections: BTreeMap::new(),
        kind: qsl_forms::StateClauseKind::Invariant,
        operation: None,
        self_slot: NO_SLOT,
        result_slot: None,
    });
    Ok(typer)
}

/// The condition of the `holds` atom whose condition is `expression`, typed
/// over `slots`, with `trace_names` the trace variables in quantifier order
/// and `first_trace` the first trace variable of each object parameter's
/// alias (FR-487 "Lowering a holds atom").
#[allow(clippy::too_many_arguments)]
pub(crate) fn lower_atom<'a>(
    typing: &Typing<'a>,
    cx: &'a mut CheckContext<'_, Scope>,
    nodes: &'a mut u64,
    location: &Location,
    slots: &AtomSlots,
    (parameters, traces): (&[&str], &[&str]),
    first_trace: &dyn Fn(&str) -> Option<usize>,
    (expression, locate): (&Expression, Locate<'_>),
) -> Result<AtomCondition, Refused> {
    let refused = |refusal: CheckRefusal| Refused {
        span: locate(&refusal.location),
        refusal: Box::new(refusal),
    };
    let enclosing = enclosing_reads(expression);
    let respelled = expression.respell_names(|id, spellings| {
        let Some(name) = spellings.reference else {
            return;
        };
        if !parameters.contains(&name.name.as_str()) {
            return;
        }
        let around = enclosing
            .get(id.index())
            .copied()
            .flatten()
            .and_then(|read| match expression.at(read).node() {
                ExprNode::Indexed { trace, .. } => Some(trace.as_str()),
                _ => None,
            });
        let chosen = around
            .filter(|trace| slots.has(&atom_slot_name(&name.name, trace)))
            .or_else(|| {
                first_trace(&name.name)
                    .and_then(|index| traces.get(index))
                    .copied()
            });
        if let Some(trace) = chosen {
            let spelled = atom_slot_name(&name.name, trace);
            name.segments = vec![spelled.clone()].into();
            name.name = spelled;
        }
    });
    let bound = slots.bound();
    let reserved_roots = traces
        .iter()
        .chain(parameters.iter())
        .map(|name| (*name).to_owned())
        .collect::<BTreeSet<_>>();
    let mut typer = typer(
        typing,
        cx,
        nodes,
        &bound,
        HyperTyping {
            reserved_roots,
            ..HyperTyping::default()
        },
        location,
    )
    .map_err(refused)?;
    let body = typer
        .infer(&respelled, Some(&ValueType::Boolean), location)
        .map_err(refused)?;
    if body.root().value_type() != &ValueType::Boolean {
        return Err(refused(CheckRefusal {
            location: location.clone(),
            cause: CheckCause::NonBooleanRoot,
        }));
    }
    let slot_count = typer.slots();
    let slot_names = typer.slot_names().to_vec();
    let formed_units = typer.into_formed_units();
    let observations = Observations::of(body.root(), Observation::Current, bound.len());
    let mut components = BTreeMap::new();
    let mut read_spans = BTreeMap::new();
    for (read, _) in observations.iter() {
        let component = node_at(expression, &read.path())
            .and_then(|id| enclosing.get(id.index()).copied().flatten())
            .and_then(|indexed| match expression.at(indexed).node() {
                ExprNode::Indexed { trace, .. } => traces.iter().position(|name| name == trace),
                _ => None,
            });
        match component {
            Some(component) => {
                components.insert(read.clone(), component);
                read_spans.insert(read.clone(), locate(read));
            }
            None => {
                return Err(Refused {
                    refusal: Box::new(CheckRefusal {
                        location: read.clone(),
                        cause: CheckCause::Hyper(Box::new(super::HyperCause::UnindexedRead {
                            span: locate(read),
                        })),
                    }),
                    span: locate(read),
                });
            }
        }
    }
    Definedness::new(
        bound.len(),
        typing.dispatch_tables,
        &typing.scope.dispatch_operations,
    )
    .with_observations(&observations)
    .with_components(&components)
    .check(body.root())
    .map_err(refused)?;
    Ok(AtomCondition {
        body,
        slots: slot_count,
        slot_names,
        formed_units,
        read_spans,
        components,
    })
}

/// The condition of the `match` conjunct rooted at `root` of `expression`,
/// typed over its step-label `reads` (FR-487 "Lowering a match conjunct").
/// `labels` maps each label constant the conjunct writes to its operation
/// text.
pub(crate) fn lower_conjunct<'a>(
    typing: &Typing<'a>,
    cx: &'a mut CheckContext<'_, Scope>,
    nodes: &'a mut u64,
    location: &Location,
    root: ExprId,
    (slots, labels): (&[StepRead], BTreeMap<String, String>),
    (expression, locate): (&Expression, Locate<'_>),
) -> Result<ConjunctCondition, Refused> {
    let refused = |refusal: CheckRefusal| Refused {
        span: locate(&refusal.location),
        refusal: Box::new(CheckRefusal {
            location: location.clone(),
            cause: refusal.cause,
        }),
    };
    let bound: Vec<(String, ValueType)> = slots
        .iter()
        .filter_map(|read| Some((read.slot_name()?, read.slot_type()?)))
        .collect();
    let mut typer = typer(
        typing,
        cx,
        nodes,
        &bound,
        HyperTyping {
            labels,
            ..HyperTyping::default()
        },
        location,
    )
    .map_err(refused)?;
    let body = typer
        .infer_subtree(expression, root, Some(&ValueType::Boolean), location)
        .map_err(refused)?;
    if body.root().value_type() != &ValueType::Boolean {
        return Err(refused(CheckRefusal {
            location: location.clone(),
            cause: CheckCause::NonBooleanRoot,
        }));
    }
    let slot_count = typer.slots();
    let slot_names = typer.slot_names().to_vec();
    let formed_units = typer.into_formed_units();
    Definedness::new(
        bound.len(),
        typing.dispatch_tables,
        &typing.scope.dispatch_operations,
    )
    .check(body.root())
    .map_err(refused)?;
    Ok(ConjunctCondition {
        body,
        slots: slot_count,
        slot_names,
        formed_units,
    })
}
