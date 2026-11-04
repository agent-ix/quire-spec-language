// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-515 "The in-process package", FR-516 and FR-532: the temporal evidence
//! call over a checked package.
//!
//! An [`EvidencePackage`] is the evaluation package of one checked package.
//! It is built from the [`Emission`] the emitter returned
//! ([`EvidencePackage::from_emission`]), from the emitted bytes read back
//! through the reader ([`EvidencePackage::from_read`]) or from a package read
//! from its bytes alone ([`EvidencePackage::from_package`], FR-531). On every
//! route the evaluator's [`ClauseFacts`] are built from the package's own
//! nodes and rows (FR-532): the two routes that hold a source keep it for
//! the cross-check of FR-515 that the package's `formula_graph()` equals the
//! source clause, and read nothing else from it.

use std::collections::BTreeSet;

use qsl_package::checked_views::{
    activation_relation, instance_bound_atoms as view_bound_atoms, temporal_subject,
    Activation as WireActivation, ClauseOccurrence, FormulaNodeUse, TemporalSubjectView,
    ViewRefusal,
};
use qsl_package::{
    AdmittedPackageRead, BarePackage, CheckedNodeId, CheckedOccurrence, CheckedPackageV2,
    CheckedSelection, Emission, SelectedExactIdentities, TemporalBinding, TemporalClauseRef,
};
use qsl_semantics::check::{Activation, CheckedFormula, FormulaNode, TemporalProfile};
use quire_eval::CallFailure;
use quire_exact::Meter;

use super::call::{evaluate_temporal_evidence, refusal, select_occurrence};
use super::request::{
    ActivationRow, EvidenceRequest, EvidenceResult, EvidenceSubject, SubjectOccurrence,
};
use crate::temporal::facts::{atom_numbers, ClauseFacts, ClauseFormula};
use crate::temporal::ledger::LedgerBinding;
use crate::temporal::DEFAULT_COVERED_POSITIONS;

/// The declared name of one clause occurrence: a `temporal_clause_names` row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClauseName {
    /// The clause's declaration node.
    pub declaration: CheckedNodeId,
    /// The occurrence's ordinal.
    pub occurrence: u64,
    /// The declared name.
    pub name: Box<str>,
}

/// The rows an evaluation package holds from construction (FR-515): the
/// `temporal_clause_names` rows, the lock's `profile_selections` and the
/// package's `temporal_bindings`, exactly as the package carries them. A
/// package recomputes none of them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageRows {
    names: Vec<ClauseName>,
    profile_selections: Vec<CheckedSelection>,
    bindings: Vec<TemporalBinding>,
}

impl PackageRows {
    /// The rows of `package`.
    ///
    /// # Errors
    ///
    /// [`PackageMismatch::AmbiguousName`] when two `temporal_clause_names`
    /// rows pair to one `{ declaration, occurrence }`.
    pub fn of_package(package: &CheckedPackageV2) -> Result<Self, PackageMismatch> {
        let mut names: Vec<ClauseName> = Vec::new();
        for row in package.temporal_clause_names().unwrap_or_default() {
            if names.iter().any(|held| {
                held.declaration == row.declaration && held.occurrence == row.occurrence
            }) {
                return Err(PackageMismatch::AmbiguousName);
            }
            names.push(ClauseName {
                declaration: row.declaration.clone(),
                occurrence: row.occurrence,
                name: row.name.clone(),
            });
        }
        Ok(Self {
            names,
            profile_selections: package.lock().profile_selections.clone(),
            bindings: package.temporal_bindings().unwrap_or_default().to_vec(),
        })
    }

    /// One row per named clause occurrence.
    pub fn names(&self) -> &[ClauseName] {
        &self.names
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
    /// to one clause occurrence.
    #[error("a clause name does not pair one name row to one clause")]
    AmbiguousName,
}

/// The source a route that holds one keeps for the cross-check of FR-515:
/// the admitted read and, for each clause occurrence, the position of its
/// clause in the source graph.
struct SourceCheck<'p> {
    read: &'p AdmittedPackageRead<'p>,
    clauses: Vec<TemporalClauseRef>,
}

/// The evaluation package of one checked package, whichever route built it.
///
/// ```compile_fail,E0599
/// // The instance-bound atoms are derived from the package; no caller
/// // supplies them.
/// fn call(package: &qsl_eval::temporal::evidence::EvidencePackage<'_>) {
///     let _ = package.with_instance_bound([0_u64]);
/// }
/// ```
pub struct EvidencePackage<'p> {
    package: &'p CheckedPackageV2,
    source: Option<SourceCheck<'p>>,
    /// The selected model identity rows a declaration-bearing capture type
    /// needs: the source unit's on the routes that hold one, none on the bare
    /// route.
    identities: SelectedExactIdentities,
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
        let rows = PackageRows::of_package(read.admitted())?;
        let held = emission.temporal_rows();
        let graph = read.source().graph();
        if !held.clauses().iter().all(|found| {
            let source_name = graph
                .temporal_clauses()
                .get(found.clause)
                .map(|node| node.clause().name());
            rows.names.iter().any(|row| {
                row.declaration == found.declaration
                    && row.occurrence == found.occurrence
                    && source_name == Some(row.name.as_ref())
            })
        }) {
            return Err(PackageMismatch::ClauseRow);
        }
        if held.profile_selections() != rows.profile_selections {
            return Err(PackageMismatch::ProfileSelections);
        }
        if held.bindings() != rows.bindings {
            return Err(PackageMismatch::Bindings);
        }
        let clauses = held.clauses().to_vec();
        Ok(Self::with_rows(
            read.admitted(),
            Some(SourceCheck { read, clauses }),
            rows,
        ))
    }

    /// The package constructed from the emitted bytes read back, paired with
    /// the in-process package the read admitted them for. Each
    /// `temporal_clause_names` row names its clause by `{ declaration,
    /// occurrence }`, and the checked graph's clause of that name is the
    /// clause the cross-check compares.
    ///
    /// # Errors
    ///
    /// [`PackageMismatch::AmbiguousName`] when two clauses of the checked
    /// graph carry one name or two name rows pair to one occurrence.
    pub fn from_read(read: &'p AdmittedPackageRead<'p>) -> Result<Self, PackageMismatch> {
        let rows = PackageRows::of_package(read.admitted())?;
        let graph = read.source().graph();
        let mut clauses: Vec<TemporalClauseRef> = Vec::new();
        for row in &rows.names {
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
        Ok(Self::with_rows(
            read.admitted(),
            Some(SourceCheck { read, clauses }),
            rows,
        ))
    }

    /// The package constructed from a package read from its bytes alone
    /// (FR-531, FR-532). It holds no source, so no clause is cross-checked:
    /// the package is evaluated as admitted. The temporal call reads nothing
    /// but the package; the supply's identity rows, when the consumer gave
    /// them, only let a record-typed capture project its type.
    ///
    /// # Errors
    ///
    /// [`PackageMismatch::AmbiguousName`] when two `temporal_clause_names`
    /// rows pair to one `{ declaration, occurrence }`.
    pub fn from_package(bare: &'p BarePackage<'p>) -> Result<Self, PackageMismatch> {
        let rows = PackageRows::of_package(bare.package())?;
        let mut package = Self::with_rows(bare.package(), None, rows);
        // A declaration-bearing capture type needs the selected identity
        // rows; the supply's rows serve where the consumer gave them.
        if let Some(supply) = bare.supply() {
            if let Ok(identities) = SelectedExactIdentities::from_stage_rows(
                bare.package(),
                supply.identities.declarations().to_vec(),
                supply.identities.fields().to_vec(),
            ) {
                package.identities = identities;
            }
        }
        Ok(package)
    }

    fn with_rows(
        package: &'p CheckedPackageV2,
        source: Option<SourceCheck<'p>>,
        rows: PackageRows,
    ) -> Self {
        let identities = source
            .as_ref()
            .and_then(|held| {
                held.read
                    .source()
                    .graph()
                    .selected_exact_identities(held.read.admitted())
                    .ok()
            })
            .unwrap_or_else(SelectedExactIdentities::empty);
        let activation_map = activation_relation(package)
            .into_iter()
            .map(|(control, clause)| ActivationRow { control, clause })
            .collect();
        Self {
            package,
            source,
            identities,
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
        temporal_subject(self.package, clause)
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
    /// (FR-515, FR-532): the held rows, the [`ClauseFacts`] built from the
    /// package's nodes and rows and the instance-bound atoms derived from the
    /// view's `binders()`.
    ///
    /// # Errors
    ///
    /// `Err(CallFailure::Admission)` with cause `clause-root-unresolved` for
    /// a clause the package does not hold, `occurrence-required` or
    /// `occurrence-unknown` as FR-516 states, and `clause-unreadable` for a
    /// clause whose name, profile, captures or atoms cannot be read.
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
        let chosen = select_occurrence(&occurrences, occurrence)?;
        let ordinal = chosen.occurrence.ordinal;
        let clock_name: Box<str> = chosen
            .clock
            .map(|row| row.clock_name.clone())
            .unwrap_or_default();
        let named = self
            .rows
            .names
            .iter()
            .find(|row| row.declaration == *clause && row.occurrence == ordinal)
            .ok_or_else(|| unreadable("clause-name"))?;
        let profile_row = self
            .profile(&view)
            .ok_or_else(|| unreadable("profile-selection"))?;
        let profile = finite_window_profile(profile_row.definition.identity.as_ref())
            .ok_or_else(|| unreadable("profile-unknown"))?;
        let graph = view.formula_graph();
        let formula = ClauseFormula::of_graph(graph);
        let reads = view_bound_atoms(&view).map_err(|_| unreadable("atom-count"))?;
        let instance_bound =
            bound_atom_indices(graph, &reads).ok_or_else(|| unreadable("atom-count"))?;
        let captures: Vec<(Box<str>, quire_exact::ValueType)> = view
            .capture_value_types(&self.identities)
            .ok_or_else(|| unreadable("capture-type"))?
            .into_iter()
            .zip(view.capture_names())
            .map(|(declared, name)| (name.as_str().into(), declared))
            .collect();
        if let Some(source) = &self.source {
            let found = source
                .clauses
                .iter()
                .find(|found| found.declaration == *clause && found.occurrence == ordinal)
                .ok_or_else(unresolved)?;
            let node = source
                .read
                .source()
                .graph()
                .temporal_clauses()
                .get(found.clause)
                .ok_or_else(unresolved)?;
            if !same_formula(graph, node.clause().formula()) {
                return Err(unreadable("formula-differs"));
            }
        }
        let activation = match view.activation() {
            WireActivation::Origin => Activation::Origin,
            WireActivation::Each { guard, .. } => Activation::Each {
                guarded: guard.is_some(),
            },
        };
        let facts = ClauseFacts::new(
            named.name.clone(),
            activation,
            profile,
            clock_name.clone(),
            captures,
            formula,
        );
        let key = view.declaration_key().ok_or_else(unresolved)?;
        Ok(EvidenceSubject {
            declaration: &named.declaration,
            profile: profile_row,
            occurrences,
            facts,
            instance_bound,
            activation_map: &self.activation_map,
            progress: LedgerBinding {
                package: self.package.name().to_owned(),
                clause: key,
                occurrence: ordinal,
                clock: clock_name.to_string(),
                profile: profile.identity().to_owned(),
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

/// The evaluator profile a lock row's definition identity names. The timed
/// profile is named too: the evaluator refuses it as the in-process route
/// does (`not-finite-window-profile`).
fn finite_window_profile(identity: &str) -> Option<TemporalProfile> {
    TemporalProfile::from_identity(identity)
}

/// The atom indices of the instance-bound atoms (FR-532 "Atom index"), given
/// for each `holds` use of `graph` in pre-order whether its operand reads an
/// instance binder (`reads`, FR-515 `instance_bound_atoms`). A shared atom
/// node has one index. `None` when `reads` and the `holds` entries differ in
/// number (FR-516 `clause-unreadable` `atom-count`).
pub fn bound_atom_indices(graph: &[FormulaNodeUse<'_>], reads: &[bool]) -> Option<BTreeSet<u64>> {
    let holds = graph
        .iter()
        .filter(|entry| entry.operation == "quire.op.temporal.holds")
        .count();
    if holds != reads.len() {
        return None;
    }
    let numbers = atom_numbers(graph);
    Some(
        graph
            .iter()
            .zip(numbers)
            .filter(|(entry, _)| entry.operation == "quire.op.temporal.holds")
            .zip(reads)
            .filter_map(|((_, number), bound)| bound.then_some(number).flatten())
            .collect(),
    )
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
            CheckedInterval::Timed(timed) => {
                // Closed ends and integral bounds count ticks, as the work
                // counters read them (`ClauseWork`), and are the integer
                // interval the package lists.
                match timed.closed_integral_bounds() {
                    Some((lower, upper)) => IntervalKey::Steps {
                        lower,
                        upper: Some(upper),
                    },
                    None => IntervalKey::Timed {
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
                }
            }
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

#[cfg(test)]
mod tests {
    use super::finite_window_profile;
    use qsl_semantics::check::TemporalProfile;

    /// FR-532 `profile`, FR-531-AC-6: the closed table of the finite-window
    /// profile identities and the infinite-trace profile; any other identity
    /// is `profile-unknown`.
    #[test]
    fn only_the_finite_window_and_infinite_trace_identities_name_a_profile() {
        for (identity, profile) in [
            (
                "quire.temporal.event-position.false-extension/v1",
                TemporalProfile::EventPositionFalseExtension,
            ),
            (
                "quire.temporal.fixed-sample.false-extension/v1",
                TemporalProfile::FixedSampleFalseExtension,
            ),
            (
                "quire.temporal.timestamped-event.finite-window/v1",
                TemporalProfile::TimestampedEventFiniteWindow,
            ),
            (
                "quire.temporal.infinite-trace/v1",
                TemporalProfile::InfiniteTrace,
            ),
        ] {
            assert_eq!(finite_window_profile(identity), Some(profile));
        }
        assert_eq!(
            finite_window_profile("quire.temporal.timed/v1"),
            Some(TemporalProfile::Timed)
        );
        for unknown in ["quire.temporal.event-position.false-extension/v2", ""] {
            assert_eq!(finite_window_profile(unknown), None, "{unknown}");
        }
    }
}
