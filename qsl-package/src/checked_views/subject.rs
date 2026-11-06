// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-515 "The temporal subject": the view of one temporal clause of an
//! admitted checked package.

use qsl_foundation::source::provenance::OccurrenceKey;
use quire_contract_model::CheckedPackageReadLimits;
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

/// The most formula applications one view expands: the reader's own
/// `i2.work_units` default. A formula node shared by several parents is read
/// once per parent, so the expansion of a package of `n` nodes can exceed `n`
/// exponentially; the view refuses past this count instead.
///
/// The bound is linear, not small: each visit keeps one `OperatorUse` (a
/// borrowed identity and an interval clone, on the order of 100 bytes) and
/// each `holds` visit one `CheckedNodeId` clone (a path vector of about seven
/// segments, a few hundred bytes), so a formula at the limit holds on the
/// order of a hundred megabytes. The spec's `&[CheckedNodeId]` accessors need
/// owned ids, so the per-visit cost is not reduced here.
pub(super) const FORMULA_VISITS: u64 = CheckedPackageReadLimits::bounded().work;

/// The temporal operation a checked formula node applies (FR-337). A package
/// spells it as an operation identity; [`FormulaOperation::of`] is the one
/// conversion of that spelling, so no other code compares it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FormulaOperation {
    /// `quire.op.temporal.true`.
    True,
    /// `quire.op.temporal.false`.
    False,
    /// `quire.op.temporal.holds`: an atom.
    Holds,
    /// `quire.op.temporal.aggregate`: a window aggregate atom.
    Aggregate,
    /// `quire.op.temporal.not`.
    Not,
    /// `quire.op.temporal.and`.
    And,
    /// `quire.op.temporal.or`.
    Or,
    /// `quire.op.temporal.implies`.
    Implies,
    /// `quire.op.temporal.eventually`.
    Eventually,
    /// `quire.op.temporal.always`.
    Always,
    /// `quire.op.temporal.once`.
    Once,
    /// `quire.op.temporal.historically`.
    Historically,
    /// `quire.op.temporal.until`.
    Until,
    /// `quire.op.temporal.release`.
    Release,
    /// `quire.op.temporal.since`.
    Since,
    /// `quire.op.temporal.triggered`.
    Triggered,
}

impl FormulaOperation {
    /// The operation an identity names, `None` for an identity that is no
    /// temporal operation.
    #[ix_trace_rs::string_edge]
    pub fn of(identity: &str) -> Option<Self> {
        match identity {
            "quire.op.temporal.true" => Some(Self::True),
            "quire.op.temporal.false" => Some(Self::False),
            "quire.op.temporal.holds" => Some(Self::Holds),
            "quire.op.temporal.aggregate" => Some(Self::Aggregate),
            "quire.op.temporal.not" => Some(Self::Not),
            "quire.op.temporal.and" => Some(Self::And),
            "quire.op.temporal.or" => Some(Self::Or),
            "quire.op.temporal.implies" => Some(Self::Implies),
            "quire.op.temporal.eventually" => Some(Self::Eventually),
            "quire.op.temporal.always" => Some(Self::Always),
            "quire.op.temporal.once" => Some(Self::Once),
            "quire.op.temporal.historically" => Some(Self::Historically),
            "quire.op.temporal.until" => Some(Self::Until),
            "quire.op.temporal.release" => Some(Self::Release),
            "quire.op.temporal.since" => Some(Self::Since),
            "quire.op.temporal.triggered" => Some(Self::Triggered),
            _ => None,
        }
    }

    /// Whether this operation is an atom of a formula.
    pub fn is_atom(self) -> bool {
        matches!(self, Self::Holds | Self::Aggregate)
    }

    /// The direction of a bounded temporal operator; `None` for `holds`, the
    /// connectives and the constants.
    pub fn direction(self) -> Option<OperatorDirection> {
        match self {
            Self::Eventually | Self::Always | Self::Until | Self::Release => {
                Some(OperatorDirection::Future)
            }
            Self::Once | Self::Historically | Self::Since | Self::Triggered => {
                Some(OperatorDirection::Past)
            }
            Self::True
            | Self::False
            | Self::Holds
            | Self::Aggregate
            | Self::Not
            | Self::And
            | Self::Or
            | Self::Implies => None,
        }
    }
}

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

/// The arena handle of one occurrence of a [`TemporalSubjectView`]'s
/// declaration (FR-515-AC-29). It is opaque and only the view's
/// [`TemporalSubjectView::arena_of`] makes one: it names the declaration node
/// and the slot of the occurrence in the view's occurrence order, and is
/// derived from no formula bytes, digest or hash.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArenaHandle {
    declaration: CheckedNodeId,
    slot: usize,
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

/// Whether a bounded temporal operator looks forward or back (QSpec FR-092).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperatorDirection {
    /// `eventually`, `always`, `until` or `release`.
    Future,
    /// `once`, `historically`, `since` or `triggered`.
    Past,
}

/// One formula node of a clause with the positions of its operands.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormulaNodeUse<'p> {
    /// The checked node this entry is a use of. Equal terms share one node
    /// (FR-337), so two entries with one `node` are two uses of one formula
    /// node, which FR-532 counts as one atom or one operator.
    pub node: &'p CheckedNodeId,
    /// The `quire.op.temporal.*` operation identity.
    pub operation: &'p str,
    /// The interval member, `None` for an unbounded or interval-free node.
    pub interval: Option<CheckedOperationInterval>,
    /// The checked aggregate argument of an aggregate atom, including its
    /// bindings. `None` for every other formula operation.
    pub aggregate: Option<&'p CheckedTerm>,
    /// The positions, in [`TemporalSubjectView::formula_graph`], of the
    /// node's operand entries.
    pub operands: Vec<usize>,
}

impl FormulaNodeUse<'_> {
    /// The temporal operation of the node, `None` for an identity that is no
    /// temporal operation.
    pub fn kind(&self) -> Option<FormulaOperation> {
        FormulaOperation::of(self.operation)
    }

    /// The direction of the node when it is a bounded temporal operator;
    /// `None` for `holds`, the connectives and the constants.
    pub fn direction(&self) -> Option<OperatorDirection> {
        self.kind().and_then(FormulaOperation::direction)
    }
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
    activation: Activation,
    captures: Vec<CheckedNodeId>,
    capture_names: Vec<String>,
    formula: Vec<OperatorUse<'p>>,
    formula_graph: Vec<FormulaNodeUse<'p>>,
    predicate_leaves: Vec<CheckedNodeId>,
}

impl<'p> TemporalSubjectView<'p> {
    /// The package this view was read from.
    pub(super) fn package(&self) -> &'p CheckedPackageV2 {
        self.package
    }

    /// The view with `profile` as its profile row, so a test can present a
    /// selection the reader would not admit.
    #[cfg(test)]
    pub(crate) fn with_profile(mut self, profile: &'p CheckedSelection) -> Self {
        self.profile = profile;
        self
    }

    /// The clause node.
    pub fn declaration(&self) -> &'p CheckedNodeId {
        self.declaration
    }

    /// One entry per declaration of the clause node, in occurrence order.
    pub fn occurrences(&self) -> &[ClauseOccurrence] {
        &self.occurrences
    }

    /// The declared temporal-clause name for a uniquely named occurrence.
    /// A view with several occurrences has no single name; a missing
    /// `temporal_clause_names` row remains absent rather than being inferred.
    pub fn name(&self) -> Option<&str> {
        let [occurrence] = self.occurrences.as_slice() else {
            return None;
        };
        self.name_of(occurrence)
    }

    /// The declared temporal-clause name for this exact occurrence. The
    /// declaration, claim role and producer-assigned ordinal all participate
    /// in the lookup; an occurrence from another node or an absent ordinal
    /// has no name.
    pub fn name_of(&self, occurrence: &ClauseOccurrence) -> Option<&str> {
        if occurrence.node != *self.declaration
            || occurrence.role != CheckedOccurrenceRole::Claim
            || !self.occurrences.contains(occurrence)
        {
            return None;
        }
        self.package
            .temporal_clause_names()?
            .iter()
            .find(|row| {
                row.declaration == *self.declaration && row.occurrence == occurrence.ordinal
            })
            .map(|row| row.name.as_ref())
    }

    /// The arena handle of `occurrence`, the one handle the view's
    /// declaration-to-arena relation holds for it (FR-515-AC-29). The key is
    /// a lookup key, accepted only when it equals an occurrence the view
    /// declares.
    ///
    /// # Errors
    ///
    /// [`ViewRefusal::OccurrenceNotDeclared`] for a key the declaration does
    /// not hold.
    pub fn arena_of(&self, occurrence: &ClauseOccurrence) -> Result<ArenaHandle, ViewRefusal> {
        let slot = self
            .occurrences
            .iter()
            .position(|declared| declared == occurrence)
            .ok_or(ViewRefusal::OccurrenceNotDeclared)?;
        Ok(ArenaHandle {
            declaration: self.declaration.clone(),
            slot,
        })
    }

    /// The occurrence `handle` is the arena handle of (FR-515-AC-29).
    ///
    /// # Errors
    ///
    /// [`ViewRefusal::ArenaHandleNotDeclared`] for a handle no row of this
    /// view declares.
    pub fn occurrence_of(&self, handle: ArenaHandle) -> Result<ClauseOccurrence, ViewRefusal> {
        if handle.declaration != *self.declaration {
            return Err(ViewRefusal::ArenaHandleNotDeclared);
        }
        self.occurrences
            .get(handle.slot)
            .cloned()
            .ok_or(ViewRefusal::ArenaHandleNotDeclared)
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

    /// The `temporal_bindings` row of the node's only occurrence. `None`
    /// when the lock selects no clock configuration for it and `None` for a
    /// node with several occurrences, whose rows [`Self::clock_of`] returns.
    pub fn clock(&self) -> Option<&'p TemporalBinding> {
        match self.occurrences.as_slice() {
            [only] => self.clock_of(only),
            _ => None,
        }
    }

    /// The `temporal_bindings` row keyed by `occurrence`'s `{ declaration,
    /// role, ordinal }`, `None` when the lock selects no clock configuration
    /// for it. The row is never paired by the position of the occurrence or
    /// of the row: producer-assigned ordinals need not run consecutively
    /// from zero, and the node's occurrence order need not match the lock's.
    pub fn clock_of(&self, occurrence: &ClauseOccurrence) -> Option<&'p TemporalBinding> {
        if occurrence.role != CheckedOccurrenceRole::Claim {
            return None;
        }
        self.package
            .temporal_bindings()?
            .iter()
            .find(|row| row.declaration == occurrence.node && row.occurrence == occurrence.ordinal)
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

    /// The declared name of each capture, in source order.
    pub fn capture_names(&self) -> &[String] {
        &self.capture_names
    }

    /// Every formula node the clause reads in pre-order, with the positions
    /// in this list of its operand entries: the temporal applications, the
    /// `holds` applications, the connectives and the constants. A `holds`
    /// atom and a constant have no operands. A node shared by several parents
    /// is listed once per use, like [`Self::formula`], which lists the same
    /// nodes without their operand edges.
    pub fn formula_graph(&self) -> &[FormulaNodeUse<'p>] {
        &self.formula_graph
    }

    /// The Boolean operand node of every `holds` atom in formula pre-order.
    /// The guard is a node of the activation argument; equal terms share one
    /// node, so a guard equal to a `holds` operand is among these entries.
    pub fn predicate_leaves(&self) -> &[CheckedNodeId] {
        &self.predicate_leaves
    }

    /// The structural key of the clause node (FR-502 `clause`).
    pub fn declaration_key(&self) -> Option<quire_exact::NodeKey> {
        wire_graph(self.package).ok()?.key(self.declaration).ok()
    }

    /// The declared value type of each capture, in source order, projected
    /// from the typed result type of its initializer term as FR-390
    /// projects a binder type (FR-532 `captures`). The actual checked package
    /// is the projection authority; caller-created maps cannot supply types.
    /// All captures charge the same caller-owned native `budget`.
    ///
    /// # Errors
    ///
    /// The original owner projection refusal, including a denied native charge.
    /// A map containing owner results from another package is not an input:
    ///
    /// ```compile_fail,E0308
    /// fn foreign_results(
    ///     view: &qsl_package::checked_views::TemporalSubjectView<'_>,
    ///     foreign: &std::collections::BTreeMap<
    ///         quire_contract_model::CheckedNodeId,
    ///         Result<quire_contract_model::ExactTypeProjection,
    ///             quire_contract_model::ExactTypeProjectionError>>,
    ///     budget: &mut quire_contract_model::NativeJoinBudget,
    /// ) {
    ///     let _ = view.capture_value_types(foreign, budget);
    /// }
    /// ```
    pub fn capture_value_types(
        &self,
        identities: &quire_contract_model::SelectedExactIdentities,
        budget: &mut quire_contract_model::NativeJoinBudget,
    ) -> Result<Vec<quire_exact::ValueType>, quire_contract_model::ExactTypeProjectionError> {
        self.captures
            .iter()
            .map(|initializer| {
                self.package
                    .project_exact_type(initializer, identities, budget)
                    .map(|projected| projected.value_type().clone())
            })
            .collect()
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
/// states, and [`ViewRefusal::FormulaTooLarge`] for a formula whose per-use
/// expansion exceeds the reader's work-unit default; no refusal leaves a
/// partial view.
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
    subject_in(&Graph::new(package), clause, FORMULA_VISITS)
}

/// [`temporal_subject`] with an explicit formula-visit budget, so a test can
/// read the [`ViewRefusal::FormulaTooLarge`] path without a package of a
/// million expansions.
#[cfg(test)]
pub(crate) fn temporal_subject_within<'p>(
    package: &'p CheckedPackageV2,
    clause: &CheckedNodeId,
    visits: u64,
) -> Result<TemporalSubjectView<'p>, ViewRefusal> {
    subject_in(&Graph::new(package), clause, visits)
}

/// The view of `clause` over a graph index the caller built once.
pub(super) fn subject_in<'p>(
    graph: &Graph<'p>,
    clause: &CheckedNodeId,
    visits: u64,
) -> Result<TemporalSubjectView<'p>, ViewRefusal> {
    let package = graph.package();
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
    let over = parameter_reference(graph, over)?;
    let formula_at = reference_position(graph, formula)?;
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
    let (formula_graph, predicate_leaves) = formula_reads(graph, formula_at, visits)?;
    let formula = formula_graph
        .iter()
        .map(|node| OperatorUse {
            operation: node.operation,
            interval: node.interval.clone(),
        })
        .collect();
    Ok(TemporalSubjectView {
        package,
        declaration,
        occurrences,
        over,
        requirements: requirements(graph, declaration),
        source_span,
        profile,
        activation: read_activation(activation)?,
        captures: read_captures(captures)?,
        capture_names: read_capture_names(captures)?,
        formula,
        formula_graph,
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

fn read_capture_names(term: &CheckedTerm) -> Result<Vec<String>, ViewRefusal> {
    members(term)?
        .iter()
        .map(|member| match member {
            CheckedTerm::Binding { name, .. } => match name.kind() {
                CheckedBindingNameKind::Other(name) => Ok(name.to_owned()),
                _ => Err(ViewRefusal::UnresolvedClause),
            },
            _ => Err(ViewRefusal::UnresolvedClause),
        })
        .collect()
}

/// The declared name of the `value`/`parameter` node `term` is the body of.
pub(super) fn parameter_name(term: &CheckedTerm) -> Option<&str> {
    let CheckedTerm::Aggregate { members } = term else {
        return None;
    };
    match binding(members, "name")?.literal_read()?.value {
        CheckedLiteralValue::Text(name) => Some(name),
        _ => None,
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

/// The formula nodes rooted at `root` in pre-order with their operand
/// positions, and the Boolean operand of each `holds` among them.
fn formula_reads<'p>(
    graph: &Graph<'p>,
    root: usize,
    visits: u64,
) -> Result<(Vec<FormulaNodeUse<'p>>, Vec<CheckedNodeId>), ViewRefusal> {
    let mut uses: Vec<FormulaNodeUse<'p>> = Vec::new();
    let mut leaves = Vec::new();
    // Each pending entry is a node and the position of the parent that
    // listed it; operands pop in operand order, so they append in that order.
    let mut pending: Vec<(usize, Option<usize>)> = vec![(root, None)];
    while let Some((at, parent)) = pending.pop() {
        if u64::try_from(uses.len()).map_or(true, |read| read >= visits) {
            return Err(ViewRefusal::FormulaTooLarge);
        }
        if graph.kind(at) != Some(CheckedNodeKind::Temporal(TemporalForm::Formula)) {
            return Err(ViewRefusal::UnresolvedClause);
        }
        let operation = graph.operation(at).ok_or(ViewRefusal::UnresolvedClause)?;
        let interval = match graph.member(at) {
            Some(CheckedOperationMember::TemporalInterval { interval }) => interval.clone(),
            _ => None,
        };
        let arguments = graph.arguments(at).ok_or(ViewRefusal::UnresolvedClause)?;
        let aggregate = if FormulaOperation::of(operation) == Some(FormulaOperation::Aggregate) {
            let [CheckedTerm::Aggregate { .. }] = arguments else {
                return Err(ViewRefusal::UnresolvedClause);
            };
            Some(&arguments[0])
        } else {
            None
        };
        let position = uses.len();
        if let Some(parent) = parent {
            uses.get_mut(parent)
                .ok_or(ViewRefusal::UnresolvedClause)?
                .operands
                .push(position);
        }
        uses.push(FormulaNodeUse {
            node: graph
                .node(at)
                .map(|node| &node.node_id)
                .ok_or(ViewRefusal::UnresolvedClause)?,
            operation,
            interval,
            aggregate,
            operands: Vec::new(),
        });
        if FormulaOperation::of(operation) == Some(FormulaOperation::Holds) {
            let [operand] = arguments else {
                return Err(ViewRefusal::UnresolvedClause);
            };
            leaves.push(
                reference_target(operand)
                    .cloned()
                    .ok_or(ViewRefusal::UnresolvedClause)?,
            );
        } else if aggregate.is_none() {
            for operand in arguments.iter().rev() {
                pending.push((reference_position(graph, operand)?, Some(position)));
            }
        }
    }
    Ok((uses, leaves))
}

/// The package's FR-449 activation map: each protocol clause with the
/// temporal clause its `requirements` name, in the package's row order.
pub fn activation_relation(package: &CheckedPackageV2) -> Vec<(CheckedNodeId, CheckedNodeId)> {
    package
        .protocol_activation_relation()
        .into_iter()
        .map(|row| (row.clause, row.required_temporal))
        .collect()
}

#[cfg(test)]
mod formula_operation_tests {
    use super::*;

    #[test]
    fn identities_decode_to_operations_with_their_direction() {
        assert_eq!(
            FormulaOperation::of("quire.op.temporal.holds"),
            Some(FormulaOperation::Holds)
        );
        assert!(FormulaOperation::Holds.is_atom());
        assert!(FormulaOperation::Aggregate.is_atom());
        assert!(!FormulaOperation::And.is_atom());
        assert_eq!(
            FormulaOperation::of("quire.op.temporal.until").and_then(FormulaOperation::direction),
            Some(OperatorDirection::Future)
        );
        assert_eq!(
            FormulaOperation::of("quire.op.temporal.triggered")
                .and_then(FormulaOperation::direction),
            Some(OperatorDirection::Past)
        );
        assert_eq!(FormulaOperation::And.direction(), None);
        assert_eq!(FormulaOperation::of("quire.op.integer.add"), None);
    }
}
