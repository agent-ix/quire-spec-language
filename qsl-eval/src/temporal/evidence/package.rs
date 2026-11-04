// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-515 "The in-process package" and FR-516: the temporal evidence call
//! over a checked package.
//!
//! An [`EvidencePackage`] is the evaluation package of one emission. It is
//! built from the [`Emission`] the emitter returned ([`EvidencePackage::from_emission`])
//! or from the emitted bytes read back through the reader
//! ([`EvidencePackage::from_read`]). Each holds from construction the
//! `profile_selections` rows, the `temporal_bindings` rows (one per clause
//! occurrence) and the `CheckedNodeId` of every clause occurrence, takes the
//! clause from the checked graph by position, and answers the subject views
//! and the call by the one construction both routes share: the clause's
//! occurrences, formula, instance-bound atoms and activation map are read
//! from the package's `temporal_subject` view.

use std::collections::BTreeSet;

use qsl_package::checked_views::{
    activation_relation, instance_bound_atoms, temporal_subject, ClauseOccurrence, FormulaNodeUse,
    TemporalSubjectView, ViewRefusal,
};
use qsl_package::{
    AdmittedPackageRead, CheckedNodeId, CheckedOccurrence, CheckedSelection, Emission,
    TemporalBinding, TemporalClauseRef,
};
use qsl_semantics::check::{CheckedFormula, FormulaNode};
use quire_eval::CallFailure;
use quire_exact::Meter;

use super::call::{evaluate_temporal_evidence, refusal, select_occurrence};
use super::request::{
    ActivationRow, EvidenceRequest, EvidenceResult, EvidenceSubject, SubjectOccurrence,
};
use super::work::ClauseFormula;
use crate::temporal::ledger::LedgerBinding;
use crate::temporal::DEFAULT_COVERED_POSITIONS;

/// The rows an evaluation package holds from construction (FR-515): the
/// clause occurrences with their ids, the lock's `profile_selections` and the
/// package's `temporal_bindings`, exactly as the emitter wrote them or the
/// reader returned them. A package recomputes none of them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageRows {
    clauses: Vec<TemporalClauseRef>,
    profile_selections: Vec<CheckedSelection>,
    bindings: Vec<TemporalBinding>,
}

impl PackageRows {
    /// The rows the emitter wrote (FR-337).
    pub fn of_emission(emission: &Emission) -> Self {
        let rows = emission.temporal_rows();
        Self {
            clauses: rows.clauses().to_vec(),
            profile_selections: rows.profile_selections().to_vec(),
            bindings: rows.bindings().to_vec(),
        }
    }

    /// The rows of the package read back: each `temporal_clause_names` row
    /// names its clause by `{ declaration, occurrence }`, and the checked
    /// graph's clause of that name is the in-process clause. A name row of a
    /// declaration that is no temporal clause of the graph (a hyper or
    /// hybrid clause) has no clause here.
    ///
    /// # Errors
    ///
    /// [`PackageMismatch::AmbiguousName`] when two clauses of the checked
    /// graph carry one name or two name rows pair to one clause: the pairing
    /// by name would name a clause the bytes do not.
    pub fn of_read(read: &AdmittedPackageRead<'_>) -> Result<Self, PackageMismatch> {
        let package = read.admitted();
        let graph = read.source().graph();
        let mut clauses: Vec<TemporalClauseRef> = Vec::new();
        for row in package.temporal_clause_names().unwrap_or_default() {
            let mut named = graph
                .temporal_clauses()
                .iter()
                .enumerate()
                .filter(|(_, node)| node.clause().name() == row.name.as_ref());
            let Some((clause, _)) = named.next() else {
                continue;
            };
            if named.next().is_some() || clauses.iter().any(|found| found.clause == clause) {
                return Err(PackageMismatch::AmbiguousName);
            }
            clauses.push(TemporalClauseRef {
                declaration: row.declaration.clone(),
                occurrence: row.occurrence,
                clause,
            });
        }
        Ok(Self {
            clauses,
            profile_selections: package.lock().profile_selections.clone(),
            bindings: package.temporal_bindings().unwrap_or_default().to_vec(),
        })
    }

    /// One reference per clause occurrence.
    pub fn clauses(&self) -> &[TemporalClauseRef] {
        &self.clauses
    }

    /// The lock's `profile_selections` rows.
    pub fn profile_selections(&self) -> &[CheckedSelection] {
        &self.profile_selections
    }

    /// The `temporal_bindings` rows, one per clause occurrence.
    pub fn bindings(&self) -> &[TemporalBinding] {
        &self.bindings
    }
}

/// The rows are not the package's: an emission that is not the one the
/// package was read from, or names that cannot pair the package's clauses to
/// the checked graph's.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PackageMismatch {
    /// An emission clause occurrence is no `temporal_clause_names` row of
    /// the package, or the clause at its position carries another name.
    #[error("a clause reference of the emission is no name row of the package")]
    ClauseRow,
    /// The emission's `profile_selections` differ from the package lock's.
    #[error("the emission's profile selections differ from the package's")]
    ProfileSelections,
    /// The emission's `temporal_bindings` differ from the package's.
    #[error("the emission's temporal bindings differ from the package's")]
    Bindings,
    /// Two clauses of the checked graph carry one name, or two name rows pair
    /// to one clause.
    #[error("a clause name does not pair one name row to one clause")]
    AmbiguousName,
}

/// The evaluation package of one emission, whichever route built it.
///
/// ```compile_fail,E0599
/// // The instance-bound atoms are derived from the package; no caller
/// // supplies them.
/// fn call(package: &qsl_eval::temporal::evidence::EvidencePackage<'_>) {
///     let _ = package.with_instance_bound([0_u64]);
/// }
/// ```
pub struct EvidencePackage<'p> {
    read: &'p AdmittedPackageRead<'p>,
    rows: PackageRows,
    activation_map: Vec<ActivationRow>,
    progress_capacity: u64,
}

impl<'p> EvidencePackage<'p> {
    /// The package constructed from `emission`: its rows and clause ids are
    /// the emitter's own. `read` is the package read back from the same
    /// emission's bytes, the reader's answer for the clause nodes.
    ///
    /// # Errors
    ///
    /// [`PackageMismatch`] when `emission` is not the one `read` was read
    /// from.
    pub fn from_emission(
        emission: &Emission,
        read: &'p AdmittedPackageRead<'p>,
    ) -> Result<Self, PackageMismatch> {
        let rows = PackageRows::of_emission(emission);
        let package = read.admitted();
        let names = package.temporal_clause_names().unwrap_or_default();
        let graph = read.source().graph();
        if !rows.clauses.iter().all(|found| {
            let held = graph
                .temporal_clauses()
                .get(found.clause)
                .map(|node| node.clause().name());
            names.iter().any(|row| {
                row.declaration == found.declaration
                    && row.occurrence == found.occurrence
                    && held == Some(row.name.as_ref())
            })
        }) {
            return Err(PackageMismatch::ClauseRow);
        }
        if rows.profile_selections != package.lock().profile_selections {
            return Err(PackageMismatch::ProfileSelections);
        }
        if rows.bindings != package.temporal_bindings().unwrap_or_default() {
            return Err(PackageMismatch::Bindings);
        }
        Ok(Self::with_rows(read, rows))
    }

    /// The package constructed from the emitted bytes read back, paired with
    /// the in-process package the read admitted them for.
    ///
    /// # Errors
    ///
    /// [`PackageMismatch::AmbiguousName`] as [`PackageRows::of_read`] states.
    pub fn from_read(read: &'p AdmittedPackageRead<'p>) -> Result<Self, PackageMismatch> {
        Ok(Self::with_rows(read, PackageRows::of_read(read)?))
    }

    fn with_rows(read: &'p AdmittedPackageRead<'p>, rows: PackageRows) -> Self {
        let activation_map = activation_relation(read.admitted())
            .into_iter()
            .map(|(control, clause)| ActivationRow { control, clause })
            .collect();
        Self {
            read,
            rows,
            activation_map,
            progress_capacity: DEFAULT_COVERED_POSITIONS,
        }
    }

    /// The package with `progress.covered_positions` (FR-255) set to
    /// `positions`.
    #[must_use]
    pub fn with_progress_capacity(mut self, positions: u64) -> Self {
        self.progress_capacity = positions;
        self
    }

    /// The rows the package holds.
    pub fn rows(&self) -> &PackageRows {
        &self.rows
    }

    /// The temporal subject view of `clause`, read from the package.
    ///
    /// # Errors
    ///
    /// [`ViewRefusal`] as `temporal_subject` states.
    pub fn temporal_subject(
        &self,
        clause: &CheckedNodeId,
    ) -> Result<TemporalSubjectView<'p>, ViewRefusal> {
        temporal_subject(self.read.admitted(), clause)
    }

    /// The held `profile_selections` row `view`'s clause selects.
    pub fn profile(&self, view: &TemporalSubjectView<'_>) -> Option<&CheckedSelection> {
        self.rows.profile_selections.iter().find(|row| {
            row.role == view.profile().role && row.definition == view.profile().definition
        })
    }

    /// The held `temporal_bindings` row of the node's only occurrence;
    /// `None` when the lock selects no clock configuration for it and `None`
    /// for a node with several occurrences, which [`Self::clock_of`] answers.
    pub fn clock(&self, view: &TemporalSubjectView<'_>) -> Option<&TemporalBinding> {
        match view.occurrences() {
            [only] => self.clock_of(only),
            _ => None,
        }
    }

    /// The held `temporal_bindings` row keyed by `occurrence`'s
    /// `{ declaration, role, ordinal }`, never by position.
    pub fn clock_of(&self, occurrence: &ClauseOccurrence) -> Option<&TemporalBinding> {
        self.rows
            .bindings
            .iter()
            .find(|row| row.declaration == occurrence.node && row.occurrence == occurrence.ordinal)
    }

    /// The facts the call reads about `clause`'s occurrence `occurrence`
    /// (FR-515): the held rows, the clause by its position in the checked
    /// graph, the formula built from the view's `formula_graph()` and the
    /// instance-bound atoms derived from its `binders()`.
    ///
    /// # Errors
    ///
    /// `Err(CallFailure::Admission)` with cause `clause-root-unresolved` for
    /// a clause the package does not hold or that no clause of the checked
    /// graph answers, `occurrence-required` or `occurrence-unknown` as FR-516
    /// states, and `clause-unreadable` for a clause whose atoms cannot be
    /// mapped to the evaluator's.
    pub fn subject(
        &self,
        clause: &CheckedNodeId,
        occurrence: Option<&CheckedOccurrence>,
    ) -> Result<EvidenceSubject<'_>, CallFailure> {
        let unresolved = || refusal("clause-root-unresolved", "clause", &[]);
        let unreadable = |reason: &'static str| {
            refusal(
                "clause-unreadable",
                "clause",
                &[("reason", reason.to_owned())],
            )
        };
        let view = self.temporal_subject(clause).map_err(|_| unresolved())?;
        let held: Vec<(CheckedOccurrence, Option<&TemporalBinding>)> = view
            .occurrences()
            .iter()
            .map(|found| {
                (
                    CheckedOccurrence {
                        role: found.role.clone(),
                        ordinal: found.ordinal,
                    },
                    self.clock_of(found),
                )
            })
            .collect();
        let occurrences: Vec<SubjectOccurrence<'_>> = held
            .into_iter()
            .map(|(occurrence, clock)| SubjectOccurrence { occurrence, clock })
            .collect();
        let chosen = select_occurrence(&occurrences, occurrence)?
            .occurrence
            .ordinal;
        let found = self
            .rows
            .clauses
            .iter()
            .find(|found| found.declaration == *clause && found.occurrence == chosen)
            .ok_or_else(unresolved)?;
        let node = self
            .read
            .source()
            .graph()
            .temporal_clauses()
            .get(found.clause)
            .ok_or_else(unresolved)?;
        let checked = node.clause();
        let profile = self
            .profile(&view)
            .ok_or_else(|| unreadable("profile-selection"))?;

        let graph = view.formula_graph();
        if !same_formula(graph, checked.formula()) {
            return Err(unreadable("formula-differs"));
        }
        let reads = instance_bound_atoms(&view).map_err(|_| unreadable("atom-count"))?;
        let instance_bound = instance_bound_indices(checked.formula(), &reads, graph.len())
            .ok_or_else(|| unreadable("atom-count"))?;

        let source = self.read.source().graph().source();
        Ok(EvidenceSubject {
            declaration: &found.declaration,
            profile,
            occurrences,
            clause: checked,
            formula: ClauseFormula::of_graph(graph),
            instance_bound,
            activation_map: &self.activation_map,
            progress: LedgerBinding {
                package: qsl_foundation::SourceIdentity::new(source.authority(), source.identity()),
                clause: node.identity(),
                occurrence: chosen,
                clock: checked.clock().to_owned(),
                profile: checked.profile().identity().to_owned(),
            },
            progress_capacity: self.progress_capacity,
        })
    }

    /// Evaluate the temporal clause `clause` over `request` (FR-516): the
    /// call of [`evaluate_temporal_evidence`] over [`Self::subject`], with
    /// the request's `occurrence` selecting the clause occurrence and its
    /// clock row by key.
    ///
    /// # Errors
    ///
    /// As [`Self::subject`] and [`evaluate_temporal_evidence`] state.
    pub fn evaluate_temporal_evidence(
        &self,
        clause: &CheckedNodeId,
        request: &EvidenceRequest,
        meter: &mut Meter,
    ) -> Result<EvidenceResult, CallFailure> {
        let subject = self.subject(clause, request.occurrence.as_ref())?;
        evaluate_temporal_evidence(&subject, request, meter)
    }
}

/// The evaluator's atom indices of the instance-bound `holds` atoms of
/// `formula`, given for each `holds` atom of the package's `formula_graph()`
/// in formula pre-order whether its operand reads an instance binder
/// (`reads`, FR-515 `instance_bound_atoms`).
///
/// The `k`-th entry of `reads` belongs to the `k`-th `holds` node of
/// `formula` in pre-order (one entry per use), whose arena index is the atom
/// index the evaluator reads valuations by. `None` when the count of `holds`
/// nodes differs from `reads.len()` or the walk would visit more than
/// `nodes` nodes (the package's formula has that many entries), so that the
/// clause is unreadable (FR-516 `clause-unreadable`).
pub fn instance_bound_indices(
    formula: &CheckedFormula,
    reads: &[bool],
    nodes: usize,
) -> Option<BTreeSet<u64>> {
    let mut atoms = Vec::new();
    let mut visited = 0_usize;
    let mut pending = vec![formula.root()];
    while let Some(id) = pending.pop() {
        visited += 1;
        if visited > nodes {
            return None;
        }
        let node = formula.nodes().get(id.index())?;
        if matches!(node, FormulaNode::Holds { .. }) {
            atoms.push(u64::try_from(id.index()).ok()?);
        }
        let operands: Vec<_> = node.operands().collect();
        pending.extend(operands.into_iter().rev());
    }
    (atoms.len() == reads.len()).then(|| {
        atoms
            .into_iter()
            .zip(reads)
            .filter_map(|(atom, bound)| bound.then_some(atom))
            .collect()
    })
}

/// The part of an interval the two formulas compare.
#[derive(Clone, Debug, Eq, PartialEq)]
enum IntervalKey {
    Steps {
        lower: u64,
        upper: Option<u64>,
    },
    Timed {
        lower: (String, String),
        upper: Option<(String, String)>,
        closed: (bool, bool),
    },
}

/// The operation identity of `node` and its interval, `None` for a node the
/// package route does not read (a window aggregate).
fn operation_of(node: &FormulaNode) -> Option<(&'static str, Option<IntervalKey>)> {
    use qsl_foundation::bound::TemporalUpper;
    use qsl_semantics::check::{BinaryTemporal, CheckedInterval, UnaryTemporal};
    let key = |interval: &Option<CheckedInterval>| {
        interval.as_ref().map(|interval| match interval {
            CheckedInterval::Steps(steps) => IntervalKey::Steps {
                lower: steps.lower(),
                upper: match steps.upper() {
                    TemporalUpper::Finite(upper) => Some(upper),
                    TemporalUpper::Open => None,
                },
            },
            // A finite-window clause holds only step intervals (FR-326): a
            // timed interval is the timed profile's, in its rational form.
            CheckedInterval::Timed(timed) => IntervalKey::Timed {
                lower: (
                    timed.lower().numerator().to_string(),
                    timed.lower().denominator().to_string(),
                ),
                upper: Some((
                    timed.upper().numerator().to_string(),
                    timed.upper().denominator().to_string(),
                )),
                closed: (timed.lower_closed(), timed.upper_closed()),
            },
        })
    };
    Some(match node {
        FormulaNode::Constant(true) => ("quire.op.temporal.true", None),
        FormulaNode::Constant(false) => ("quire.op.temporal.false", None),
        FormulaNode::Holds { .. } => ("quire.op.temporal.holds", None),
        FormulaNode::Not(_) => ("quire.op.temporal.not", None),
        FormulaNode::And(..) => ("quire.op.temporal.and", None),
        FormulaNode::Or(..) => ("quire.op.temporal.or", None),
        FormulaNode::Implies(..) => ("quire.op.temporal.implies", None),
        FormulaNode::Unary {
            operator, interval, ..
        } => (
            match operator {
                UnaryTemporal::Eventually => "quire.op.temporal.eventually",
                UnaryTemporal::Always => "quire.op.temporal.always",
                UnaryTemporal::Once => "quire.op.temporal.once",
                UnaryTemporal::Historically => "quire.op.temporal.historically",
            },
            key(interval),
        ),
        FormulaNode::Binary {
            operator, interval, ..
        } => (
            match operator {
                BinaryTemporal::Until => "quire.op.temporal.until",
                BinaryTemporal::Release => "quire.op.temporal.release",
                BinaryTemporal::Since => "quire.op.temporal.since",
                BinaryTemporal::Triggered => "quire.op.temporal.triggered",
            },
            key(interval),
        ),
        FormulaNode::Aggregate { .. } => return None,
    })
}

/// The interval of a package entry as [`operation_of`] keys the checked one.
fn graph_key(node: &FormulaNodeUse<'_>) -> Option<Option<IntervalKey>> {
    use qsl_package::CheckedIntervalEnd::Closed;
    use qsl_package::CheckedOperationInterval as Interval;
    let pair = |bound: &qsl_package::CheckedOperationRational| {
        (bound.numerator.to_string(), bound.denominator.to_string())
    };
    Some(match node.interval.as_ref() {
        None => None,
        Some(Interval::Integer { lower, upper }) => Some(IntervalKey::Steps {
            lower: u64::try_from(lower).ok()?,
            upper: match upper {
                None => None,
                Some(upper) => Some(u64::try_from(upper).ok()?),
            },
        }),
        Some(Interval::Timed {
            lower,
            upper,
            lower_end,
            upper_end,
        }) => {
            let integral = |bound: &qsl_package::CheckedOperationRational| {
                u64::try_from(&bound.denominator).is_ok_and(|denominator| denominator == 1)
            };
            match upper {
                Some(upper)
                    if *lower_end == Closed
                        && *upper_end == Closed
                        && integral(lower)
                        && integral(upper) =>
                {
                    Some(IntervalKey::Steps {
                        lower: u64::try_from(&lower.numerator).ok()?,
                        upper: Some(u64::try_from(&upper.numerator).ok()?),
                    })
                }
                _ => Some(IntervalKey::Timed {
                    lower: pair(lower),
                    upper: upper.as_ref().map(pair),
                    closed: (*lower_end == Closed, *upper_end == Closed),
                }),
            }
        }
    })
}

/// Whether the package's `graph` and the in-process `formula` are the same
/// formula: walked in pre-order, one entry per use of a node, each entry has
/// the same operation, interval and operand positions, and the two have the
/// same length. The evaluator's truth is computed on the in-process formula
/// and its work counters and atom mapping on the package's, so the two must
/// agree for the call to assess the clause the package holds (FR-515 "One
/// evaluator formula").
fn same_formula(graph: &[FormulaNodeUse<'_>], formula: &CheckedFormula) -> bool {
    let mut position = 0_usize;
    let mut operands: Vec<Vec<usize>> = vec![Vec::new(); graph.len()];
    let mut pending = vec![(formula.root(), None::<usize>)];
    while let Some((id, parent)) = pending.pop() {
        let (Some(entry), Some(node)) = (graph.get(position), formula.nodes().get(id.index()))
        else {
            return false;
        };
        let Some((operation, interval)) = operation_of(node) else {
            return false;
        };
        if entry.operation != operation || graph_key(entry) != Some(interval) {
            return false;
        }
        if let Some(parent) = parent {
            let Some(listed) = operands.get_mut(parent) else {
                return false;
            };
            listed.push(position);
        }
        let own: Vec<_> = node.operands().collect();
        pending.extend(
            own.into_iter()
                .rev()
                .map(|operand| (operand, Some(position))),
        );
        position += 1;
    }
    position == graph.len()
        && graph
            .iter()
            .zip(&operands)
            .all(|(entry, listed)| entry.operands == *listed)
}
