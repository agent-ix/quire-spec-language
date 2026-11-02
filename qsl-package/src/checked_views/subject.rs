// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-515 "The temporal subject": the view of one temporal clause of an
//! admitted checked package.

use qsl_foundation::source::provenance::OccurrenceKey;
use quire_contract_model::{
    CheckedBindingNameKind, CheckedLiteralValue, CheckedNodeId, CheckedNodeKind,
    CheckedOccurrenceRole, CheckedOperationInterval, CheckedOperationMember, CheckedPackageV2,
    CheckedSelection, CheckedSelectionRole, CheckedSemanticNodeV2, CheckedSourceMapEntry,
    CheckedTerm, LawRole, TemporalBinding, TemporalForm, ValueForm,
};
use quire_exact::{Origin, Role};

use super::graph::Graph;
use super::ViewRefusal;
use crate::checked_v2::{resolve_clause_root, role_spelling, wire_graph};

/// The operation identity of a `holds` atom.
const HOLDS: &str = "quire.op.temporal.holds";

/// One declaration of a clause node: FR-337's identity triple.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClauseOccurrence {
    /// The clause node.
    pub node: CheckedNodeId,
    /// The occurrence role, `claim` for every declaration.
    pub role: CheckedOccurrenceRole,
    /// The occurrence ordinal.
    pub ordinal: u64,
}

/// How a clause is activated, read from its activation argument.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Activation {
    /// `on origin`.
    Origin,
    /// `on each (q: T) [when (g)]`.
    Each {
        /// The `value`/`parameter` node of the trigger binder.
        trigger: CheckedNodeId,
        /// The guard's Boolean node, when one is written.
        guard: Option<CheckedNodeId>,
    },
}

/// One temporal operator application of a clause's formula.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperatorUse<'p> {
    /// The `quire.op.temporal.*` operation identity.
    pub operation: &'p str,
    /// The interval member, `None` for an unbounded or interval-free operator.
    pub interval: Option<CheckedOperationInterval>,
}

/// The temporal clause `declaration` of one admitted checked package.
///
/// The view borrows the package, is built only by [`temporal_subject`] and
/// evaluates nothing. It carries no digest, document bytes or identity label:
/// the package binding is its lifetime and [`Self::belongs_to`].
///
/// A view cannot be built outside this module:
///
/// ```compile_fail,E0451
/// use qsl_package::checked_views::TemporalSubjectView;
/// fn construct(package: &quire_contract_model::CheckedPackageV2) -> TemporalSubjectView<'_> {
///     TemporalSubjectView { package }
/// }
/// ```
///
/// It has none of `digest`, `bytes`, `identity` or `package_digest`, and no
/// method that takes a closure. Each call below fails for the missing method
/// alone; the control call shows the same shape compiles with a real accessor.
///
/// ```
/// fn call(view: &qsl_package::checked_views::TemporalSubjectView<'_>) {
///     let _ = (view.declaration(), view.occurrences(), view.formula());
/// }
/// ```
///
/// ```compile_fail,E0599
/// fn call(view: &qsl_package::checked_views::TemporalSubjectView<'_>) {
///     let _ = view.digest();
/// }
/// ```
///
/// ```compile_fail,E0599
/// fn call(view: &qsl_package::checked_views::TemporalSubjectView<'_>) {
///     let _ = view.bytes();
/// }
/// ```
///
/// ```compile_fail,E0599
/// fn call(view: &qsl_package::checked_views::TemporalSubjectView<'_>) {
///     let _ = view.identity();
/// }
/// ```
///
/// ```compile_fail,E0599
/// fn call(view: &qsl_package::checked_views::TemporalSubjectView<'_>) {
///     let _ = view.package_digest();
/// }
/// ```
///
/// ```compile_fail,E0599
/// fn call(view: &qsl_package::checked_views::TemporalSubjectView<'_>) {
///     let _ = view.evaluate(|_| true);
/// }
/// ```
#[derive(Clone, Debug)]
pub struct TemporalSubjectView<'p> {
    package: &'p CheckedPackageV2,
    declaration: &'p CheckedNodeId,
    occurrences: Vec<ClauseOccurrence>,
    over: &'p CheckedNodeId,
    requirements: Vec<&'p CheckedNodeId>,
    source_span: &'p CheckedSourceMapEntry,
    profile: &'p CheckedSelection,
    clock: Option<&'p TemporalBinding>,
    activation: Activation,
    captures: Vec<CheckedNodeId>,
    formula: Vec<OperatorUse<'p>>,
    predicate_leaves: Vec<CheckedNodeId>,
}

impl<'p> TemporalSubjectView<'p> {
    /// The clause node.
    pub fn declaration(&self) -> &'p CheckedNodeId {
        self.declaration
    }

    /// One entry per declaration of the clause node, in occurrence order.
    pub fn occurrences(&self) -> &[ClauseOccurrence] {
        &self.occurrences
    }

    /// The `value`/`parameter` node the clause is `over`.
    pub fn over(&self) -> &'p CheckedNodeId {
        self.over
    }

    /// The protocol clauses whose `requirements` name this clause, in
    /// FR-449's row order.
    pub fn requirements(&self) -> &[&'p CheckedNodeId] {
        &self.requirements
    }

    /// The package source map entry of the clause node.
    pub fn source_span(&self) -> &'p CheckedSourceMapEntry {
        self.source_span
    }

    /// The lock's `profile_selections` row the clause's `temporal_profile` law selects.
    pub fn profile(&self) -> &'p CheckedSelection {
        self.profile
    }

    /// The `temporal_bindings` row of this declaration, `None` when the lock
    /// selects no clock configuration for it.
    pub fn clock(&self) -> Option<&'p TemporalBinding> {
        self.clock
    }

    /// The clause's activation.
    pub fn activation(&self) -> &Activation {
        &self.activation
    }

    /// The initializer node of each capture, in source order.
    pub fn captures(&self) -> &[CheckedNodeId] {
        &self.captures
    }

    /// The `quire.op.temporal.*` applications of the formula in pre-order.
    pub fn formula(&self) -> &[OperatorUse<'p>] {
        &self.formula
    }

    /// The Boolean operand node of every `holds` atom in formula pre-order.
    /// The activation guard is read from [`Self::activation`], not from here.
    pub fn predicate_leaves(&self) -> &[CheckedNodeId] {
        &self.predicate_leaves
    }

    /// Whether `package` is the package this view was read from.
    pub fn belongs_to(&self, package: &CheckedPackageV2) -> bool {
        std::ptr::eq(self.package, package)
    }
}

/// Read the temporal clause `clause` of `package`.
///
/// Refuses [`ViewRefusal::UnresolvedClause`] for a node the package does not
/// hold, one that is no clause root, or one whose body lacks a fact FR-337
/// states; no refusal leaves a partial view.
///
/// The view's lifetime is the package's borrow: the package cannot be moved
/// or dropped while the view is in use.
///
/// ```
/// use qsl_package::checked_views::temporal_subject;
/// fn read(package: quire_contract_model::CheckedPackageV2, id: quire_contract_model::CheckedNodeId) {
///     let view = temporal_subject(&package, &id).unwrap();
///     let _ = view.declaration();
///     drop(package);
/// }
/// ```
///
/// ```compile_fail,E0505
/// use qsl_package::checked_views::temporal_subject;
/// fn read(package: quire_contract_model::CheckedPackageV2, id: quire_contract_model::CheckedNodeId) {
///     let view = temporal_subject(&package, &id).unwrap();
///     drop(package);
///     let _ = view.declaration();
/// }
/// ```
pub fn temporal_subject<'p>(
    package: &'p CheckedPackageV2,
    clause: &CheckedNodeId,
) -> Result<TemporalSubjectView<'p>, ViewRefusal> {
    let graph = Graph::new(package);
    let at = graph
        .position(clause)
        .ok_or(ViewRefusal::UnresolvedClause)?;
    let (node, kind) = graph
        .node(at)
        .zip(graph.kind(at))
        .ok_or(ViewRefusal::UnresolvedClause)?;
    if kind != CheckedNodeKind::Temporal(TemporalForm::TemporalClause) {
        return Err(ViewRefusal::UnresolvedClause);
    }
    let declaration = &node.node_id;
    let occurrences = occurrences(package, node)?;
    let operation = match graph.term(at) {
        Some(CheckedTerm::Application { operation, .. }) => operation
            .read()
            .map_err(|_| ViewRefusal::UnresolvedClause)?,
        _ => return Err(ViewRefusal::UnresolvedClause),
    };
    let [over, _clock, activation, captures, _fairness, formula] =
        graph.arguments(at).ok_or(ViewRefusal::UnresolvedClause)?
    else {
        return Err(ViewRefusal::UnresolvedClause);
    };
    let over = parameter_reference(&graph, over)?;
    let formula_at = reference_position(&graph, formula)?;
    let profile = operation
        .laws
        .iter()
        .find(|law| law.role == LawRole::TemporalProfile)
        .and_then(|law| {
            package.lock().profile_selections.iter().find(|selection| {
                selection.role == CheckedSelectionRole::TemporalProfile
                    && selection.definition == law.definition
            })
        })
        .ok_or(ViewRefusal::UnresolvedClause)?;
    let source_span = package
        .source_map()
        .iter()
        .find(|entry| entry.node_id == *declaration && entry.role == CheckedOccurrenceRole::Claim)
        .ok_or(ViewRefusal::UnresolvedClause)?;
    let (formula, predicate_leaves) = formula_reads(&graph, formula_at)?;
    Ok(TemporalSubjectView {
        package,
        declaration,
        occurrences,
        over,
        requirements: requirements(&graph, declaration),
        source_span,
        profile,
        clock: package
            .temporal_bindings()
            .and_then(|rows| rows.iter().find(|row| row.declaration == *declaration)),
        activation: read_activation(activation)?,
        captures: read_captures(captures)?,
        formula,
        predicate_leaves,
    })
}

/// The `claim` occurrences of the clause node that FR-337's lookup resolves.
fn occurrences(
    package: &CheckedPackageV2,
    node: &CheckedSemanticNodeV2,
) -> Result<Vec<ClauseOccurrence>, ViewRefusal> {
    let declaration = &node.node_id;
    let wire = wire_graph(package).map_err(|_| ViewRefusal::UnresolvedClause)?;
    let key = wire
        .key(declaration)
        .map_err(|_| ViewRefusal::UnresolvedClause)?;
    let claims = node
        .occurrences
        .iter()
        .filter(|occurrence| occurrence.role == CheckedOccurrenceRole::Claim);
    let found: Vec<ClauseOccurrence> = claims
        .filter(|occurrence| {
            let triple = OccurrenceKey::new(
                key.clone(),
                Origin::new(
                    Role::new(role_spelling(&CheckedOccurrenceRole::Claim)),
                    occurrence.ordinal,
                ),
            );
            resolve_clause_root(package, &triple).is_ok()
        })
        .map(|occurrence| ClauseOccurrence {
            node: declaration.clone(),
            role: CheckedOccurrenceRole::Claim,
            ordinal: occurrence.ordinal,
        })
        .collect();
    if found.is_empty() {
        return Err(ViewRefusal::UnresolvedClause);
    }
    Ok(found)
}

/// The protocol clauses whose requirements name `declaration`, from the
/// package's own FR-449 relation.
fn requirements<'p>(graph: &Graph<'p>, declaration: &CheckedNodeId) -> Vec<&'p CheckedNodeId> {
    graph
        .package()
        .protocol_activation_relation()
        .into_iter()
        .filter(|row| row.required_temporal == *declaration)
        .filter_map(|row| {
            let (node, _) = graph.resolve(&row.clause)?;
            Some(&node.node_id)
        })
        .collect()
}

/// The target of the reference term `term`.
fn reference_target(term: &CheckedTerm) -> Option<&CheckedNodeId> {
    match term {
        CheckedTerm::Reference { target } => Some(target),
        _ => None,
    }
}

fn reference_position(graph: &Graph<'_>, term: &CheckedTerm) -> Result<usize, ViewRefusal> {
    reference_target(term)
        .and_then(|target| graph.position(target))
        .ok_or(ViewRefusal::UnresolvedClause)
}

/// The id of the `value`/`parameter` node `term` references.
fn parameter_reference<'p>(
    graph: &Graph<'p>,
    term: &'p CheckedTerm,
) -> Result<&'p CheckedNodeId, ViewRefusal> {
    let at = reference_position(graph, term)?;
    match graph.kind(at) {
        Some(CheckedNodeKind::Value(ValueForm::Parameter)) => graph
            .node(at)
            .map(|node| &node.node_id)
            .ok_or(ViewRefusal::UnresolvedClause),
        _ => Err(ViewRefusal::UnresolvedClause),
    }
}

/// The members of the aggregate `term`.
fn members(term: &CheckedTerm) -> Result<&[CheckedTerm], ViewRefusal> {
    match term {
        CheckedTerm::Aggregate { members } => Ok(members),
        _ => Err(ViewRefusal::UnresolvedClause),
    }
}

/// The value of the binding named `name` among `members`.
fn binding<'t>(members: &'t [CheckedTerm], name: &str) -> Option<&'t CheckedTerm> {
    members.iter().find_map(|member| match member {
        CheckedTerm::Binding { name: bound, value }
            if bound.kind() == CheckedBindingNameKind::Other(name) =>
        {
            Some(&**value)
        }
        _ => None,
    })
}

fn read_activation(term: &CheckedTerm) -> Result<Activation, ViewRefusal> {
    let members = members(term)?;
    let kind = binding(members, "kind")
        .and_then(CheckedTerm::literal_read)
        .map(|literal| literal.value);
    match kind {
        Some(CheckedLiteralValue::Text("origin")) => Ok(Activation::Origin),
        Some(CheckedLiteralValue::Text("each")) => {
            let id = |term: &CheckedTerm| {
                reference_target(term)
                    .cloned()
                    .ok_or(ViewRefusal::UnresolvedClause)
            };
            Ok(Activation::Each {
                trigger: binding(members, "trigger")
                    .ok_or(ViewRefusal::UnresolvedClause)
                    .and_then(id)?,
                guard: binding(members, "guard").map(id).transpose()?,
            })
        }
        _ => Err(ViewRefusal::UnresolvedClause),
    }
}

fn read_captures(term: &CheckedTerm) -> Result<Vec<CheckedNodeId>, ViewRefusal> {
    members(term)?
        .iter()
        .map(|member| match member {
            CheckedTerm::Binding { value, .. } => reference_target(value)
                .cloned()
                .ok_or(ViewRefusal::UnresolvedClause),
            _ => Err(ViewRefusal::UnresolvedClause),
        })
        .collect()
}

/// The operator applications of the formula rooted at `root` in pre-order,
/// and the Boolean operand of each `holds` among them.
fn formula_reads<'p>(
    graph: &Graph<'p>,
    root: usize,
) -> Result<(Vec<OperatorUse<'p>>, Vec<CheckedNodeId>), ViewRefusal> {
    let mut uses = Vec::new();
    let mut leaves = Vec::new();
    let mut pending = vec![root];
    while let Some(at) = pending.pop() {
        if graph.kind(at) != Some(CheckedNodeKind::Temporal(TemporalForm::Formula)) {
            return Err(ViewRefusal::UnresolvedClause);
        }
        let operation = graph.operation(at).ok_or(ViewRefusal::UnresolvedClause)?;
        let interval = match graph.member(at) {
            Some(CheckedOperationMember::TemporalInterval { interval }) => interval.clone(),
            _ => None,
        };
        uses.push(OperatorUse {
            operation,
            interval,
        });
        let arguments = graph.arguments(at).ok_or(ViewRefusal::UnresolvedClause)?;
        if operation == HOLDS {
            let [operand] = arguments else {
                return Err(ViewRefusal::UnresolvedClause);
            };
            leaves.push(
                reference_target(operand)
                    .cloned()
                    .ok_or(ViewRefusal::UnresolvedClause)?,
            );
        } else {
            for operand in arguments.iter().rev() {
                pending.push(reference_position(graph, operand)?);
            }
        }
    }
    Ok((uses, leaves))
}
