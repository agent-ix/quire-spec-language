// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-502: the progress vocabulary of the spine facade. The ledger itself is
//! the evaluator's; these types are the facade's own spelling of its inputs and
//! outputs (FR-100-AC-8: no public `qsl_replay::spine` item names a `qsl_eval`
//! path), converted at the boundary.

use std::collections::{BTreeMap, BTreeSet};

use qsl_eval::temporal as eval;

/// Whether the input is complete, and through which tick.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProgressCompleteness {
    /// Every observation stamped at or before `through` has been supplied.
    Complete {
        /// The last tick the assertion covers.
        through: u64,
    },
    /// No completeness is asserted.
    Incomplete,
}

/// The closure of a decision scope or of the surrounding execution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProgressClosure {
    /// Nothing more will be added.
    Closed,
    /// More may still be added.
    Open,
}

/// Whether the first position of a progressed trace is the origin of the
/// history or only a cutoff within it (FR-504 Inputs).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProgressLowerBoundary {
    /// History before the first position is empty.
    AuthoritativeOrigin,
    /// History before the first position exists but was not supplied.
    HistoryCutoff,
}

/// One position of an admitted trace, by identity and stamp.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ObservedPosition {
    /// The position's identity: the identity of its admitted observation.
    pub identity: String,
    /// The position's stamp: its tick, or its coordinate under event-position
    /// and fixed-sample.
    pub stamp: u64,
}

/// The progress a trace asserts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProgressAssertion {
    /// The watermark `W`.
    pub watermark: u64,
    /// The completeness assertion.
    pub completeness: ProgressCompleteness,
    /// The decision scope's closure.
    pub scope: ProgressClosure,
    /// The surrounding execution's closure.
    pub execution: ProgressClosure,
}

/// The whole binding one progress assertion is made under (FR-502 Inputs).
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct LedgerBinding {
    /// The checked package identity.
    pub package: qsl_foundation::SourceIdentity,
    /// The checked clause identity: the declaration node, which is keyed by
    /// the clause's content and so is shared by two clauses of one body.
    pub clause: quire_exact::NodeKey,
    /// The occurrence of the declaration (its `claim` ordinal, the
    /// `occurrence` of its `temporal_bindings` row): two clauses with equal
    /// bodies and different names are two occurrences and keep separate
    /// progress.
    pub occurrence: u64,
    /// The clock binding name.
    pub clock: String,
    /// The selected profile identity.
    pub profile: String,
}

/// The dimension an assertion contradicts (FR-502 Outputs).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContradictedDimension {
    /// A watermark below the retained one.
    Watermark,
    /// A completeness assertion that retreats from the retained one, or a
    /// position the retained assertion did not cover.
    Completeness,
    /// A closure that reopens.
    Closure,
}

/// One side of a contradiction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProgressValue {
    /// A watermark.
    Watermark(u64),
    /// A completeness assertion.
    Completeness(ProgressCompleteness),
    /// The decision scope's closure.
    DecisionScope(ProgressClosure),
    /// The surrounding execution's closure.
    SurroundingExecution(ProgressClosure),
    /// A position the retained completeness assertion did not cover.
    Position(ObservedPosition),
}

/// An assertion that contradicts the retained progress (FR-502 Outputs).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgressContradiction {
    /// The contradicted dimension.
    pub dimension: ContradictedDimension,
    /// The binding's clock name.
    pub clock: String,
    /// The retained value.
    pub retained: ProgressValue,
    /// The asserted value.
    pub asserted: ProgressValue,
}

/// The covered-position set outgrew its capacity (FR-502, FR-255).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProgressLimitReached {
    /// The limit's kind, `node count`.
    pub limit: &'static str,
    /// The capacity in this run.
    pub value: u64,
    /// The setting that raises it, `progress.covered_positions`.
    pub setting: &'static str,
}

/// What one binding has retained.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgressLedgerEntry {
    assertion: ProgressAssertion,
    covered: BTreeSet<ObservedPosition>,
}

impl ProgressLedgerEntry {
    /// The retained watermark.
    pub fn watermark(&self) -> u64 {
        self.assertion.watermark
    }

    /// The retained completeness assertion.
    pub fn completeness(&self) -> ProgressCompleteness {
        self.assertion.completeness
    }

    /// The retained decision-scope closure.
    pub fn scope(&self) -> ProgressClosure {
        self.assertion.scope
    }

    /// The retained surrounding-execution closure.
    pub fn execution(&self) -> ProgressClosure {
        self.assertion.execution
    }

    /// The identity and stamp of every position an accepted request of this
    /// binding supplied, in identity-then-stamp order.
    pub fn covered(&self) -> impl Iterator<Item = &ObservedPosition> {
        self.covered.iter()
    }
}

/// Progress retained per binding across requests (FR-502).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProgressLedger {
    entries: BTreeMap<LedgerBinding, ProgressLedgerEntry>,
}

impl ProgressLedger {
    /// An empty ledger.
    pub fn new() -> Self {
        Self::default()
    }

    /// The entry retained under `binding`, where that binding has asserted
    /// any.
    pub fn entry(&self, binding: &LedgerBinding) -> Option<&ProgressLedgerEntry> {
        self.entries.get(binding)
    }

    /// Every binding with an entry, in binding order.
    pub fn bindings(&self) -> impl Iterator<Item = &LedgerBinding> {
        self.entries.keys()
    }

    /// How many bindings have an entry.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no binding has an entry.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Why admitting a request's progress returned no ledger.
pub(super) enum Refusal {
    Contradiction(ProgressContradiction),
    LimitReached(ProgressLimitReached),
}

impl ProgressLedger {
    /// FR-502: the evaluator's ledger admits `assertion` under `binding` over
    /// `positions` against `capacity`; this ledger is rebuilt from, and
    /// updated by, its answer.
    pub(super) fn admit(
        &self,
        binding: &LedgerBinding,
        assertion: ProgressAssertion,
        positions: &[ObservedPosition],
        capacity: u64,
    ) -> Result<Self, Refusal> {
        let mut carried = eval::ProgressLedger::new();
        for (held, entry) in &self.entries {
            let covered: Vec<eval::ObservedPosition> =
                entry.covered.iter().cloned().map(Into::into).collect();
            carried = carried
                .admit(
                    &held.clone().into(),
                    entry.assertion.into(),
                    &covered,
                    u64::MAX,
                )
                .map_err(Refusal::from)?;
        }
        let positions: Vec<eval::ObservedPosition> =
            positions.iter().cloned().map(Into::into).collect();
        carried
            .admit(
                &binding.clone().into(),
                assertion.into(),
                &positions,
                capacity,
            )
            .map(Self::from)
            .map_err(Refusal::from)
    }
}

impl From<eval::ProgressLedger> for ProgressLedger {
    fn from(ledger: eval::ProgressLedger) -> Self {
        let entries = ledger
            .bindings()
            .filter_map(|binding| {
                let entry = ledger.entry(binding)?;
                Some((
                    LedgerBinding::from(binding.clone()),
                    ProgressLedgerEntry {
                        assertion: ProgressAssertion {
                            watermark: entry.watermark(),
                            completeness: entry.completeness().into(),
                            scope: entry.scope().into(),
                            execution: entry.execution().into(),
                        },
                        covered: entry.covered().cloned().map(Into::into).collect(),
                    },
                ))
            })
            .collect();
        Self { entries }
    }
}

impl From<eval::LedgerRefusal> for Refusal {
    fn from(refusal: eval::LedgerRefusal) -> Self {
        match refusal {
            eval::LedgerRefusal::Contradiction(contradiction) => {
                Self::Contradiction(contradiction.into())
            }
            eval::LedgerRefusal::LimitReached(reached) => Self::LimitReached(reached.into()),
        }
    }
}

impl From<eval::CoveredPositionsLimitReached> for ProgressLimitReached {
    fn from(reached: eval::CoveredPositionsLimitReached) -> Self {
        Self {
            limit: reached.limit,
            value: reached.value,
            setting: reached.setting,
        }
    }
}

impl From<eval::ProgressContradiction> for ProgressContradiction {
    fn from(contradiction: eval::ProgressContradiction) -> Self {
        Self {
            dimension: match contradiction.dimension {
                eval::ContradictedDimension::Watermark => ContradictedDimension::Watermark,
                eval::ContradictedDimension::Completeness => ContradictedDimension::Completeness,
                eval::ContradictedDimension::Closure => ContradictedDimension::Closure,
            },
            clock: contradiction.clock,
            retained: contradiction.retained.into(),
            asserted: contradiction.asserted.into(),
        }
    }
}

impl From<eval::ProgressValue> for ProgressValue {
    fn from(value: eval::ProgressValue) -> Self {
        match value {
            eval::ProgressValue::Watermark(watermark) => Self::Watermark(watermark),
            eval::ProgressValue::Completeness(completeness) => {
                Self::Completeness(completeness.into())
            }
            eval::ProgressValue::DecisionScope(closure) => Self::DecisionScope(closure.into()),
            eval::ProgressValue::SurroundingExecution(closure) => {
                Self::SurroundingExecution(closure.into())
            }
            eval::ProgressValue::Position(position) => Self::Position(position.into()),
        }
    }
}

impl From<eval::ProgressCompleteness> for ProgressCompleteness {
    fn from(completeness: eval::ProgressCompleteness) -> Self {
        match completeness {
            eval::ProgressCompleteness::Complete { through } => Self::Complete { through },
            eval::ProgressCompleteness::Incomplete => Self::Incomplete,
        }
    }
}

impl From<ProgressCompleteness> for eval::ProgressCompleteness {
    fn from(completeness: ProgressCompleteness) -> Self {
        match completeness {
            ProgressCompleteness::Complete { through } => Self::Complete { through },
            ProgressCompleteness::Incomplete => Self::Incomplete,
        }
    }
}

impl From<eval::ProgressClosure> for ProgressClosure {
    fn from(closure: eval::ProgressClosure) -> Self {
        match closure {
            eval::ProgressClosure::Closed => Self::Closed,
            eval::ProgressClosure::Open => Self::Open,
        }
    }
}

impl From<ProgressClosure> for eval::ProgressClosure {
    fn from(closure: ProgressClosure) -> Self {
        match closure {
            ProgressClosure::Closed => Self::Closed,
            ProgressClosure::Open => Self::Open,
        }
    }
}

impl From<eval::ObservedPosition> for ObservedPosition {
    fn from(position: eval::ObservedPosition) -> Self {
        Self {
            identity: position.identity,
            stamp: position.stamp,
        }
    }
}

impl From<ObservedPosition> for eval::ObservedPosition {
    fn from(position: ObservedPosition) -> Self {
        Self {
            identity: position.identity,
            stamp: position.stamp,
        }
    }
}

impl From<ProgressAssertion> for eval::ProgressAssertion {
    fn from(assertion: ProgressAssertion) -> Self {
        Self {
            watermark: assertion.watermark,
            completeness: assertion.completeness.into(),
            scope: assertion.scope.into(),
            execution: assertion.execution.into(),
        }
    }
}

impl From<eval::LedgerBinding> for LedgerBinding {
    fn from(binding: eval::LedgerBinding) -> Self {
        Self {
            package: binding.package,
            clause: binding.clause,
            occurrence: binding.occurrence,
            clock: binding.clock,
            profile: binding.profile,
        }
    }
}

impl From<LedgerBinding> for eval::LedgerBinding {
    fn from(binding: LedgerBinding) -> Self {
        Self {
            package: binding.package,
            clause: binding.clause,
            occurrence: binding.occurrence,
            clock: binding.clock,
            profile: binding.profile,
        }
    }
}
