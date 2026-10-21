// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-136 "Selecting the concrete protocol" and "Coverage over a protocol
//! subject", FR-214: the S3 check of a refinement whose step rows select a
//! protocol clause of the unit.
//!
//! The rows select the protocol (`step <C>::<protocol>::<name>`, a keyworded
//! form or `activate <C>::<protocol>`), one protocol per refinement. What
//! depends on the models only is checked with the rest of the declaration
//! (`classify`): which rows name a protocol, the selected clause and the
//! refusals of a second one. What is drawn from the clause is checked once
//! the clause is (`ProtocolRefinement::complete`): the rows resolve against
//! its nodes, templates, channels, roles, trigger and memory models, and
//! every step class has exactly one row. A refused clause is reported by the
//! clause's own checks alone.

use std::collections::{BTreeMap, BTreeSet};

use qsl_forms::{
    ChannelDelivery, ProtocolDeclarationForm, ProtocolNodeKind, RefinementForm, RefinementRowForm,
    RoleLifetime, StepClassKeyword, StepRowForm, StepSourceForm, StepTargetForm,
};
use qsl_foundation::{MemoryModelName, Span};
use quire_exact::EffectiveId;

use super::{
    CheckedRefinement, Checker, Findings, NameSegments, ObjectType, RefinementCause,
    RefinementRefusal, RowClass, Side, StepClass, StepRow, StepTarget, UnsupportedFeature,
};
use crate::check::check::ScopedHistory;
use crate::check::lowering::AdmittedModel;
use crate::check::protocol_clause::{BoundAttempt, CheckedProtocol, ProtocolNodeId};
use crate::check::protocol_obligation::CheckedActivation;
use crate::check::refusal::{CheckCause, CheckRefusal};
use crate::check::state_clause::ClauseDeclarations;
use crate::family::CheckContext;
use crate::model::key::DeclarationKey;
use crate::model::operation::OperationDeclaration;

/// A step of a protocol node (ADR-027 ST-2 to ST-9) that has no operation
/// and so needs a row of its own.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ProtocolStepKind {
    /// An `effect`, a domain `event` or a `commit` (ST-2).
    Event,
    /// A `send` (ST-3).
    Send,
    /// A `receive` (ST-4).
    Receive,
    /// The `fork` of a `parallel` (ST-6).
    Fork,
    /// The `join` of a `parallel` (ST-7).
    Join,
    /// The `timeout` of an `await` (ST-8).
    Timeout,
    /// The `finish` node (ST-9).
    Finish,
}

/// A step of a channel (ADR-027 ST-5).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ChannelStepKind {
    /// `duplicate`.
    Duplicate,
    /// `lose`.
    Lose,
}

/// A step of a replicated role (ADR-027 ST-14, ST-15).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RoleStepKind {
    /// `spawn`.
    Spawn,
    /// `retire`.
    Retire,
}

/// An internal step of a memory model (ADR-027 ST-10, SE-1 d).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MemoryStepKind {
    /// `tso`'s `flush` of a store buffer (ADR-025 TSO-4). `sc` and `ra` have
    /// no internal step.
    Flush,
}

impl MemoryStepKind {
    /// The kind's spelling in a `memory <kind>` row.
    pub fn label(self) -> &'static str {
        match self {
            Self::Flush => "flush",
        }
    }

    /// The internal step kinds of `model`.
    fn of_model(model: MemoryModelName) -> &'static [Self] {
        match model {
            MemoryModelName::Sc | MemoryModelName::Ra => &[],
            MemoryModelName::Tso => &[Self::Flush],
        }
    }
}

/// The concrete protocol clause the rows of a refinement select: its index
/// among the unit's protocol declarations and the first row that names it.
#[derive(Clone, Copy, Debug)]
pub(super) struct Selection {
    pub(super) protocol: usize,
    pub(super) first: Span,
}

/// What a `step` row's left side is, before S3 resolves it further.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RowSort {
    /// A row of a concrete operation.
    Operation,
    /// A row naming the selected protocol clause.
    Protocol,
    /// A `memory <kind>` row.
    Memory,
    /// A row whose middle segment is no type of the concrete model and no
    /// protocol clause; `classify` settles it once it knows the subject.
    Unresolved,
    /// A row already refused.
    Refused,
}

/// The `step` rows of a declaration, sorted, and the protocol they select.
pub(super) struct Classified<'f> {
    /// Every `step` row with its position among the declaration's rows.
    pub(super) rows: Vec<(usize, &'f StepRowForm, RowSort)>,
    /// The selected clause; `None` for a model subject.
    pub(super) selection: Option<Selection>,
}

/// Whether the roles of `protocol` act on the model `alias`: some role is
/// `on` or `each` an object type of it.
fn acts_on(protocol: &ProtocolDeclarationForm, alias: &str) -> bool {
    protocol
        .roles
        .iter()
        .any(|role| role.context.name.split("::").next() == Some(alias))
}

/// Sorts the `step` rows of `form`, selects the concrete protocol and
/// records the refusals that depend on the unit's protocol names only
/// (FR-136 "Selecting the concrete protocol"):
///
/// - a segment after `<C>::` is resolved among the concrete model's object
///   types first, a type making an operation row, then among `protocols`;
/// - the clause named by the first row that names one is selected, and the
///   first row that names each other clause refuses `conflicting-binding`;
/// - a name that is no protocol refuses `missing-name` at that segment, and
///   a clause whose roles do not act on `<C>` `malformed-declaration`.
pub(super) fn classify<'f>(
    form: &'f RefinementForm,
    protocols: &[ProtocolDeclarationForm],
    concrete_types: &[ObjectType],
    findings: &mut Findings,
) -> Classified<'f> {
    let concrete = form.concrete.name.as_str();
    let mut selection: Option<Selection> = None;
    // The clauses already refused, each once: a second selected-against
    // protocol, or one that does not act on `<C>`.
    let mut reported: BTreeSet<usize> = BTreeSet::new();
    let mut rows = Vec::new();
    // The rows that name no protocol, by their position in `rows`, with the
    // segment and its span.
    let mut unresolved: Vec<(usize, &str, Span)> = Vec::new();
    for (index, row) in form.rows.iter().enumerate() {
        let RefinementRowForm::Step(row) = row else {
            continue;
        };
        let sort = match &row.source {
            StepSourceForm::Memory(_) => RowSort::Memory,
            StepSourceForm::Named { kind, name } => {
                let segments = NameSegments::of(name);
                let names_protocol = match (kind, segments.parts()) {
                    (None, [alias, ty, _]) if *alias == concrete => {
                        let qualified = format!("{alias}::{ty}");
                        (!concrete_types.iter().any(|object| object.name == qualified)).then_some(1)
                    }
                    (None, _) => None,
                    (Some(StepClassKeyword::Activate), [alias, _]) if *alias == concrete => Some(1),
                    (Some(StepClassKeyword::Activate), _) => {
                        findings.unresolved(&name.name, name.span, None);
                        rows.push((index, row, RowSort::Refused));
                        continue;
                    }
                    (Some(_), [alias, _, _]) if *alias == concrete => Some(1),
                    (Some(_), _) => {
                        findings.unresolved(&name.name, name.span, None);
                        rows.push((index, row, RowSort::Refused));
                        continue;
                    }
                };
                match names_protocol {
                    None => RowSort::Operation,
                    Some(position) => {
                        let protocol_name = segments.parts()[position];
                        let span = segments.span(position);
                        let found = protocols
                            .iter()
                            .position(|protocol| protocol.name.name == protocol_name);
                        match found {
                            None => {
                                unresolved.push((rows.len(), protocol_name, span));
                                RowSort::Unresolved
                            }
                            Some(protocol) if !acts_on(&protocols[protocol], concrete) => {
                                if reported.insert(protocol) {
                                    findings.malformed(protocol_name, span);
                                }
                                RowSort::Refused
                            }
                            Some(protocol) => match selection {
                                None => {
                                    selection = Some(Selection {
                                        protocol,
                                        first: row.span,
                                    });
                                    RowSort::Protocol
                                }
                                Some(selected) if selected.protocol == protocol => {
                                    RowSort::Protocol
                                }
                                Some(selected) => {
                                    if reported.insert(protocol) {
                                        findings.refuse(
                                            row.span,
                                            RefinementCause::Conflicting {
                                                name: protocol_name.to_owned(),
                                                first: selected.first,
                                            },
                                        );
                                    }
                                    RowSort::Refused
                                }
                            },
                        }
                    }
                }
            }
        };
        rows.push((index, row, sort));
    }
    // Under a protocol subject a name that is no clause refuses at that
    // segment (FR-136 "Selecting the concrete protocol"). Under a model
    // subject a row refuses at its span, as FR-136 "Coverage over a model
    // subject" states: an unkeyworded name is read as an operation's, and a
    // keyworded row names none.
    for (position, segment, span) in unresolved {
        let (_, row, sort) = &mut rows[position];
        match (selection, &row.source) {
            (Some(_), _) => {
                findings.unresolved(segment, span, None);
                *sort = RowSort::Refused;
            }
            (None, StepSourceForm::Named { kind: None, .. }) => *sort = RowSort::Operation,
            (None, StepSourceForm::Named { name, .. }) => {
                findings.unresolved(&name.name, name.span, None);
                *sort = RowSort::Refused;
            }
            (None, StepSourceForm::Memory(_)) => *sort = RowSort::Memory,
        }
    }
    Classified { rows, selection }
}

/// A refinement over a concrete protocol subject whose rows are not all
/// resolved: the declaration's header, population, object, fairness and
/// history rows and its operation rows are checked, and the rows that name
/// the protocol clause wait for the checked clause (FR-136 "Order of the
/// checks").
pub(crate) struct ProtocolRefinement<'f> {
    pub(super) checked: CheckedRefinement,
    /// The operation rows that checked, with their position among the
    /// declaration's rows.
    pub(super) operations: Vec<(usize, StepRow)>,
    /// The operation of every operation row written.
    pub(super) written: Vec<DeclarationKey>,
    pub(super) selection: Selection,
    /// The rows that name the protocol clause or a memory step kind.
    pub(super) rows: Vec<(usize, &'f StepRowForm)>,
    /// The profile the declaration selects, which a refusal names.
    pub(super) profile: String,
    /// Every refusal the declaration's other checks found.
    pub(super) refusals: Vec<CheckRefusal>,
    /// The span of the refinement's name, where a missing row is reported.
    pub(super) name_span: Span,
    /// What the rows of the declaration were typed with, which an
    /// application on an attempt or `cattempt` row is typed with too.
    pub(super) typing: Typing,
}

/// The state of the first pass that typing a row reads.
pub(super) struct Typing {
    pub(super) lifting: BTreeMap<EffectiveId, EffectiveId>,
    pub(super) maps_complete: bool,
    pub(super) history: Vec<ScopedHistory>,
}

/// The concrete protocol clause of a refinement, as the unit checked it.
pub(crate) struct ProtocolClause<'a> {
    pub(crate) form: &'a ProtocolDeclarationForm,
    pub(crate) checked: &'a CheckedProtocol,
    /// The attempts' operations, as `protocol_clause::check` bound them.
    pub(crate) attempts: &'a [BoundAttempt],
}

/// The protocol a checked refinement is over, for the pipeline stage that
/// mints the clause's node key.
#[derive(Clone, Debug)]
pub(crate) struct SelectedProtocol {
    protocol: usize,
    /// The first row that names the clause.
    first: Span,
    refinement: String,
}

impl SelectedProtocol {
    /// The index of the clause among the unit's protocol declarations.
    pub(crate) fn protocol(&self) -> usize {
        self.protocol
    }

    /// `refusal`, that the clause has no node to record, at the refinement's
    /// first row that names the clause.
    pub(crate) fn unkeyed(&self, refusal: CheckRefusal) -> CheckRefusal {
        CheckRefusal {
            location: refusal.location,
            cause: CheckCause::Refinement(Box::new(RefinementRefusal {
                refinement: self.refinement.clone(),
                span: self.first,
                cause: RefinementCause::Check(refusal.cause),
            })),
        }
    }
}

/// How a step class is written as a row, which S3 resolves a row's left side
/// against.
struct Entry {
    class: StepClass,
    keyword: Option<StepClassKeyword>,
    name: String,
    /// The kind of row a refusal for a missing row names.
    row: RowClass,
    need: Need,
}

/// What covers a class.
enum Need {
    /// A row of its own.
    Row,
    /// A row of its own, or the row of this concrete operation.
    OrOperation(Option<DeclarationKey>),
}

fn node_entries(clause: &ProtocolClause<'_>, concrete: &AdmittedModel, entries: &mut Vec<Entry>) {
    let tree = &clause.checked.controls;
    for control in &tree.controls {
        let node = control.node;
        let name = control.name.clone();
        let row = |kind| StepClass::ProtocolNode { kind, node };
        let mut plain = |class: StepClass, keyword, row, need| {
            entries.push(Entry {
                class,
                keyword,
                name: name.clone(),
                row,
                need,
            });
        };
        match control.kind {
            ProtocolNodeKind::Attempt => {
                let operation = clause
                    .attempts
                    .iter()
                    .find(|bound| bound.declaration == node.0)
                    .and_then(|bound| concrete.operation_key(&bound.operation))
                    .cloned();
                plain(
                    StepClass::AttemptNode(node),
                    None,
                    RowClass::Node,
                    Need::OrOperation(operation),
                );
            }
            ProtocolNodeKind::Effect | ProtocolNodeKind::Event | ProtocolNodeKind::Commit => plain(
                row(ProtocolStepKind::Event),
                None,
                RowClass::Node,
                Need::Row,
            ),
            ProtocolNodeKind::Send => {
                plain(row(ProtocolStepKind::Send), None, RowClass::Node, Need::Row);
            }
            ProtocolNodeKind::Receive => plain(
                row(ProtocolStepKind::Receive),
                None,
                RowClass::Node,
                Need::Row,
            ),
            ProtocolNodeKind::Await => plain(
                row(ProtocolStepKind::Timeout),
                None,
                RowClass::Node,
                Need::Row,
            ),
            ProtocolNodeKind::Parallel => {
                plain(
                    row(ProtocolStepKind::Fork),
                    Some(StepClassKeyword::Fork),
                    RowClass::Node,
                    Need::Row,
                );
                plain(
                    row(ProtocolStepKind::Join),
                    Some(StepClassKeyword::Join),
                    RowClass::Node,
                    Need::Row,
                );
            }
            // Structure, which has no step (ST-1 to ST-9). A `fence` has one
            // (ADR-027 SE-5), but FR-136's coverage lists no class for it.
            ProtocolNodeKind::Sequence
            | ProtocolNodeKind::Choice
            | ProtocolNodeKind::Repeat
            | ProtocolNodeKind::Branch
            | ProtocolNodeKind::Case
            | ProtocolNodeKind::Check
            | ProtocolNodeKind::Fence
            | ProtocolNodeKind::Finish
            | ProtocolNodeKind::CompensateTemplate => {}
        }
    }
    if let Some((node, name)) = &clause.checked.finish {
        entries.push(Entry {
            class: StepClass::ProtocolNode {
                kind: ProtocolStepKind::Finish,
                node: *node,
            },
            keyword: None,
            name: name.clone(),
            row: RowClass::Node,
            need: Need::Row,
        });
    }
}

/// The outermost `parallel` controls of the tree: those inside no other.
fn outermost_parallels(clause: &ProtocolClause<'_>) -> Vec<ProtocolNodeId> {
    let tree = &clause.checked.controls;
    let mut found = Vec::new();
    let Some(root) = tree.root() else {
        return found;
    };
    let mut pending = vec![root.node];
    while let Some(node) = pending.pop() {
        let Some(control) = tree.control(node) else {
            continue;
        };
        if control.kind == ProtocolNodeKind::Parallel {
            found.push(node);
        } else {
            pending.extend(control.children.iter().rev());
        }
    }
    found
}

/// Every step class of the clause, in the order a missing row is reported:
/// the controls in tree order, then the `finish` node, the templates, the
/// channels, the replicated roles, the trigger and the memory steps.
fn entries(clause: &ProtocolClause<'_>, concrete: &AdmittedModel) -> Vec<Entry> {
    let mut entries = Vec::new();
    node_entries(clause, concrete, &mut entries);
    let obligations = &clause.checked.obligations;
    for template in &obligations.compensations {
        let operation = concrete.operation_key(&template.operation).cloned();
        entries.push(Entry {
            class: StepClass::CompensationAttempt(template.name.clone()),
            keyword: None,
            name: template.name.clone(),
            row: RowClass::Template,
            need: Need::OrOperation(operation),
        });
        entries.push(Entry {
            class: StepClass::CompensationEnd(template.name.clone()),
            keyword: Some(StepClassKeyword::Cend),
            name: template.name.clone(),
            row: RowClass::Template,
            need: Need::Row,
        });
    }
    for channel in &clause.checked.channels {
        let admits = |kind| match (kind, channel.delivery) {
            (
                ChannelStepKind::Duplicate,
                ChannelDelivery::Unknown | ChannelDelivery::AtLeastOnce,
            )
            | (ChannelStepKind::Lose, ChannelDelivery::Unknown | ChannelDelivery::AtMostOnce) => {
                true
            }
            (_, ChannelDelivery::ExactlyOncePremise)
            | (ChannelStepKind::Duplicate, ChannelDelivery::AtMostOnce)
            | (ChannelStepKind::Lose, ChannelDelivery::AtLeastOnce) => false,
        };
        for (kind, keyword) in [
            (ChannelStepKind::Duplicate, StepClassKeyword::Duplicate),
            (ChannelStepKind::Lose, StepClassKeyword::Lose),
        ] {
            if admits(kind) {
                entries.push(Entry {
                    class: StepClass::Channel {
                        kind,
                        channel: channel.name.clone(),
                    },
                    keyword: Some(keyword),
                    name: channel.name.clone(),
                    row: RowClass::Channel,
                    need: Need::Row,
                });
            }
        }
    }
    for (role, form) in clause.checked.roles.iter().zip(&clause.form.roles) {
        let Some(replicated) = &form.replicated else {
            continue;
        };
        let retires = matches!(replicated.lifetime, RoleLifetime::Until(_));
        for (kind, keyword, wanted) in [
            (RoleStepKind::Spawn, StepClassKeyword::Spawn, true),
            (RoleStepKind::Retire, StepClassKeyword::Retire, retires),
        ] {
            if wanted {
                entries.push(Entry {
                    class: StepClass::Role {
                        kind,
                        role: role.name.clone(),
                    },
                    keyword: Some(keyword),
                    name: role.name.clone(),
                    row: RowClass::Role,
                    need: Need::Row,
                });
            }
        }
    }
    if matches!(obligations.activation, CheckedActivation::OnEach { .. }) {
        entries.push(Entry {
            class: StepClass::Trigger,
            keyword: Some(StepClassKeyword::Activate),
            name: clause.form.name.name.clone(),
            row: RowClass::Trigger,
            need: Need::Row,
        });
    }
    // One `memory <kind>` row covers a kind for every outermost `parallel`,
    // since the row names no node (FR-136).
    let memory = &clause.checked.memory.parallels;
    let kinds: BTreeSet<MemoryStepKind> = outermost_parallels(clause)
        .into_iter()
        .filter_map(|node| memory.iter().find(|parallel| parallel.node == node))
        .flat_map(|parallel| MemoryStepKind::of_model(parallel.memory).iter().copied())
        .collect();
    entries.extend(kinds.into_iter().map(|kind| Entry {
        class: StepClass::MemoryStep(kind),
        keyword: None,
        name: kind.label().to_owned(),
        row: RowClass::Memory,
        need: Need::Row,
    }));
    entries
}

/// The spelling of a row for `entry`, as a refusal names it.
fn spelled(entry: &Entry, concrete: &str, protocol: &str) -> String {
    let keyword = entry.keyword.map(StepClassKeyword::as_str);
    let name = match entry.class {
        StepClass::MemoryStep(kind) => return format!("memory {}", kind.label()),
        StepClass::Trigger => format!("{concrete}::{protocol}"),
        _ => format!("{concrete}::{protocol}::{}", entry.name),
    };
    match keyword {
        Some(keyword) => format!("{keyword} {name}"),
        None => name,
    }
}

impl ProtocolRefinement<'_> {
    /// The index of the selected clause among the unit's protocols.
    pub(crate) fn protocol(&self) -> usize {
        self.selection.protocol
    }

    /// Whether a refusal found so far is a resource limit, which stops the
    /// unit.
    pub(crate) fn exhausted(&self) -> bool {
        self.refusals
            .iter()
            .any(|refusal| matches!(refusal.cause.unwrapped(), CheckCause::ResourceExhausted(_)))
    }

    /// The refusals found so far.
    pub(crate) fn refusals(&self) -> &[CheckRefusal] {
        &self.refusals
    }

    /// The refusals found so far, by value.
    pub(crate) fn into_refusals(self) -> Vec<CheckRefusal> {
        self.refusals
    }

    /// Resolves the rows that name the clause, requires one row for every
    /// step class of it and yields the checked refinement, or every
    /// refusal of the declaration. With `clause` absent, the clause was
    /// refused: the declaration's other refusals stand and no coverage is
    /// reported (FR-136 "Order of the checks").
    ///
    /// # Errors
    ///
    /// Every refusal of the declaration, in the order found.
    pub(crate) fn complete<'a>(
        self,
        clause: Option<&ProtocolClause<'_>>,
        declarations: &ClauseDeclarations<'a>,
        cx: &mut CheckContext<'a, ClauseDeclarations<'a>>,
    ) -> (
        Result<(CheckedRefinement, SelectedProtocol), Vec<CheckRefusal>>,
        u64,
    ) {
        let Self {
            mut checked,
            operations,
            written,
            selection,
            rows,
            profile,
            refusals,
            name_span,
            typing,
        } = self;
        let mut nodes = declarations.nodes_used;
        let findings = Findings {
            refinement: checked.name.clone(),
            refusals,
        };
        let Some(clause) = clause else {
            return (Err(findings.refusals), nodes);
        };
        let models = declarations.models;
        let (abstract_alias, concrete_alias) = (
            checked.abstract_model.clone(),
            checked.concrete_model.clone(),
        );
        let (Some(abstract_side), Some(concrete_side)) = (
            side_of(models, &abstract_alias),
            side_of(models, &concrete_alias),
        ) else {
            let mut findings = findings;
            findings.fault("the-models-of-a-refinement-are-admitted");
            return (Err(findings.refusals), nodes);
        };
        let mut checker = Checker {
            declarations,
            protocols: &[],
            abstract_side,
            concrete_side,
            findings,
            nodes: &mut nodes,
            lifting: typing.lifting,
            maps_complete: typing.maps_complete,
            baseline: 0,
            populations: None,
            history: typing.history,
        };
        let outcome = checker.complete_rows(
            clause,
            ProtocolRows {
                checked: &mut checked,
                operations,
                written,
                selection,
                rows,
                profile,
                name_span,
            },
            cx,
        );
        let refusals = checker.findings.refusals;
        let result = match outcome {
            Some(selected) if refusals.is_empty() => Ok((checked, selected)),
            _ => Err(refusals),
        };
        (result, nodes)
    }
}

/// The model the unit gives `alias`, with that alias.
fn side_of<'s>(models: &'s [AdmittedModel], alias: &'s str) -> Option<Side<'s>> {
    models
        .iter()
        .find(|model| model.alias() == Some(alias))
        .map(|model| Side { alias, model })
}

/// What `complete` hands the row checks.
struct ProtocolRows<'r, 'f> {
    checked: &'r mut CheckedRefinement,
    operations: Vec<(usize, StepRow)>,
    written: Vec<DeclarationKey>,
    selection: Selection,
    rows: Vec<(usize, &'f StepRowForm)>,
    profile: String,
    name_span: Span,
}

impl<'a> Checker<'_, 'a> {
    /// Resolves the rows that name the clause and requires one for every step
    /// class of it. The refinement's steps are set when it checks; the result
    /// is the selected protocol then, and the refusals are in `findings`.
    fn complete_rows(
        &mut self,
        clause: &ProtocolClause<'_>,
        input: ProtocolRows<'_, '_>,
        cx: &mut CheckContext<'a, ClauseDeclarations<'a>>,
    ) -> Option<SelectedProtocol> {
        let ProtocolRows {
            checked,
            operations,
            written,
            selection,
            rows,
            profile,
            name_span,
        } = input;
        let concrete = self.concrete_side.model;
        let abstract_types = {
            let found = self.abstract_side.object_types(self.types());
            self.or_fault(found)
        };
        let concrete_types = {
            let found = self.concrete_side.object_types(self.types());
            self.or_fault(found)
        };
        let entries = entries(clause, concrete);
        let concrete_alias = checked.concrete_model.clone();
        let protocol = clause.form.name.name.clone();
        let mut resolved: Vec<(usize, StepRow)> = Vec::new();
        let mut first_rows: Vec<(StepClass, Span)> = Vec::new();
        for (index, row) in rows {
            let entry: &Entry = match &row.source {
                StepSourceForm::Memory(kind) => {
                    let mut kinds = entries.iter().filter(|entry| {
                        matches!(entry.class, StepClass::MemoryStep(_)) && entry.name == kind.name
                    });
                    let Some(entry) = kinds.next() else {
                        self.findings.unresolved(&kind.name, kind.span, None);
                        continue;
                    };
                    entry
                }
                StepSourceForm::Named { kind, name } => {
                    let segments = NameSegments::of(name);
                    let (position, wanted) = match (kind, segments.parts()) {
                        (Some(StepClassKeyword::Activate), [_, _]) => (1, protocol.as_str()),
                        (_, [_, _, node]) => (2, *node),
                        _ => {
                            self.findings.fault("a-protocol-row-has-a-classified-shape");
                            continue;
                        }
                    };
                    let mut named = entries.iter().filter(|entry| {
                        entry.keyword == *kind
                            && entry.name == wanted
                            && !matches!(entry.class, StepClass::MemoryStep(_))
                    });
                    match (named.next(), named.next()) {
                        (Some(entry), None) => entry,
                        (None, _) => {
                            self.findings
                                .unresolved(wanted, segments.span(position), None);
                            continue;
                        }
                        (Some(_), Some(_)) => {
                            self.findings.refuse(
                                name.span,
                                RefinementCause::Ambiguous {
                                    name: name.name.clone(),
                                },
                            );
                            continue;
                        }
                    }
                }
            };
            if let Some((_, first)) = first_rows.iter().find(|(class, _)| *class == entry.class) {
                self.findings.refuse(
                    row.span,
                    RefinementCause::Conflicting {
                        name: spelled(entry, &concrete_alias, &protocol),
                        first: *first,
                    },
                );
                continue;
            }
            first_rows.push((entry.class.clone(), row.span));
            let target = match &row.target {
                StepTargetForm::Stutter { .. } => StepTarget::Stutter,
                StepTargetForm::Any { .. } => StepTarget::Any,
                StepTargetForm::Node { node, .. } => {
                    // While the abstract side is a model subject, a right
                    // side naming a protocol node is malformed (FR-147).
                    self.findings.malformed(&node.name, row.span);
                    continue;
                }
                StepTargetForm::Apply {
                    span,
                    operation,
                    arguments,
                } => {
                    // FR-136 "Right sides": the receiver and arguments are
                    // typed over the concrete step, which an attempt or a
                    // `cattempt` has (it applies an operation). A step with no
                    // operation has no receiver the requirement names.
                    let applied = applied_operation(&entry.class, clause).and_then(|applied| {
                        let object = concrete_types
                            .iter()
                            .find(|object| object.id == applied.0)?;
                        Some((object, applied.1))
                    });
                    match applied {
                        Some((object, concrete_operation)) => {
                            match self.application(
                                row.span,
                                operation,
                                arguments,
                                (object, &concrete_operation),
                                &abstract_types,
                                cx,
                            ) {
                                Some(target) => target,
                                None => continue,
                            }
                        }
                        None => {
                            self.findings.refuse(
                                *span,
                                RefinementCause::Unsupported {
                                    feature: UnsupportedFeature::ProtocolStepApplication,
                                    profile: profile.clone(),
                                    subject: None,
                                },
                            );
                            continue;
                        }
                    }
                }
            };
            resolved.push((
                index,
                StepRow {
                    class: entry.class.clone(),
                    target,
                },
            ));
        }
        for entry in &entries {
            let covered = first_rows.iter().any(|(class, _)| *class == entry.class);
            let by_operation = match &entry.need {
                Need::Row => false,
                Need::OrOperation(operation) => operation
                    .as_ref()
                    .is_some_and(|operation| written.contains(operation)),
            };
            if !covered && !by_operation {
                self.findings.refuse(
                    name_span,
                    RefinementCause::MissingRow {
                        class: entry.row,
                        name: spelled(entry, &concrete_alias, &protocol),
                    },
                );
            }
        }
        if !self.findings.refusals.is_empty() {
            return None;
        }
        let mut steps = operations;
        steps.extend(resolved);
        steps.sort_by_key(|(index, _)| *index);
        checked.steps = steps.into_iter().map(|(_, row)| row).collect();
        let refinement = checked.name.clone();
        Some(SelectedProtocol {
            protocol: selection.protocol,
            first: selection.first,
            refinement,
        })
    }
}

/// The receiver's type and the operation the concrete step of an attempt
/// node or a template's `cattempt` applies; `None` for a step with no
/// operation.
fn applied_operation(
    class: &StepClass,
    clause: &ProtocolClause<'_>,
) -> Option<(EffectiveId, OperationDeclaration)> {
    match class {
        StepClass::AttemptNode(node) => {
            let bound = clause
                .attempts
                .iter()
                .find(|bound| bound.declaration == node.0)?;
            Some((bound.receiver, bound.operation.declaration.clone()))
        }
        StepClass::CompensationAttempt(name) => {
            let template = clause
                .checked
                .obligations
                .compensations
                .iter()
                .find(|template| template.name == *name)?;
            Some((
                template.operation.declaring,
                template.operation.declaration.clone(),
            ))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests;
