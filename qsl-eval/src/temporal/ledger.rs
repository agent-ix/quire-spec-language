// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-502: the spine's progress ledger.
//!
//! A caller that evaluates one temporal clause repeatedly as observation
//! arrives carries one [`ProgressLedger`]. For each [`LedgerBinding`] the ledger
//! retains what that binding has already asserted about its watermark,
//! completeness and closure, and the identities of the positions it covered.
//! [`ProgressLedger::admit`] refuses an assertion that contradicts the retained
//! entry and returns the updated ledger otherwise; the ledger it was called on
//! is never changed, so a refusal rolls back nothing and rewrites no earlier
//! result.
//!
//! The ledger does not verify or repair an assertion. It only keeps one
//! binding from asserting less than it already asserted.

use std::collections::{BTreeMap, BTreeSet};

/// FR-255's setting that raises the covered-position capacity.
pub const COVERED_POSITIONS_SETTING: &str = "progress.covered_positions";

/// FR-255's default for [`COVERED_POSITIONS_SETTING`], in positions.
pub const DEFAULT_COVERED_POSITIONS: u64 = 1_000_000;

/// The kind of limit [`COVERED_POSITIONS_SETTING`] names (FR-255).
const COVERED_POSITIONS_LIMIT: &str = "node count";

/// The whole binding one progress assertion is made under (FR-502 Inputs).
/// An assertion under any other package, clause, clock binding name or
/// profile identity is progress for a different binding.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct LedgerBinding {
    /// The checked package identity.
    pub package: qsl_foundation::SourceIdentity,
    /// The checked clause identity.
    pub clause: quire_exact::NodeKey,
    /// The clock binding name.
    pub clock: String,
    /// The selected profile identity.
    pub profile: String,
}

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

/// What one binding has retained.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LedgerEntry {
    watermark: u64,
    completeness: ProgressCompleteness,
    scope: ProgressClosure,
    execution: ProgressClosure,
    covered: BTreeSet<String>,
}

impl LedgerEntry {
    /// The retained watermark.
    pub fn watermark(&self) -> u64 {
        self.watermark
    }

    /// The retained completeness assertion.
    pub fn completeness(&self) -> ProgressCompleteness {
        self.completeness
    }

    /// The retained decision-scope closure.
    pub fn scope(&self) -> ProgressClosure {
        self.scope
    }

    /// The retained surrounding-execution closure.
    pub fn execution(&self) -> ProgressClosure {
        self.execution
    }

    /// The identities of every position an accepted request of this binding
    /// supplied, in identity order.
    pub fn covered(&self) -> impl Iterator<Item = &str> {
        self.covered.iter().map(String::as_str)
    }
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
pub struct CoveredPositionsLimitReached {
    /// The limit's kind, `node count`.
    pub limit: &'static str,
    /// The capacity in this run.
    pub value: u64,
    /// The setting that raises it, `progress.covered_positions`.
    pub setting: &'static str,
}

/// Why [`ProgressLedger::admit`] returned no ledger.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LedgerRefusal {
    /// The assertion contradicts the retained progress.
    Contradiction(ProgressContradiction),
    /// Recording the positions would exceed the ledger's capacity.
    LimitReached(CoveredPositionsLimitReached),
}

/// Progress retained per binding across requests (FR-502).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProgressLedger {
    entries: BTreeMap<LedgerBinding, LedgerEntry>,
}

impl ProgressLedger {
    /// An empty ledger.
    pub fn new() -> Self {
        Self::default()
    }

    /// The entry retained under `binding`, where that binding has asserted
    /// any. Progress under another binding is never reported here.
    pub fn entry(&self, binding: &LedgerBinding) -> Option<&LedgerEntry> {
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

    /// Admit one request's progress: `assertion` under `binding` over the
    /// request trace's `positions`, against a covered-position `capacity`.
    ///
    /// Returns the updated ledger, or the refusal with `self` unchanged.
    /// Dimensions are checked in the order watermark, completeness, closure.
    /// Positions the retained entry covered stay covered when a later trace
    /// omits them.
    pub fn admit(
        &self,
        binding: &LedgerBinding,
        assertion: ProgressAssertion,
        positions: &[ObservedPosition],
        capacity: u64,
    ) -> Result<Self, LedgerRefusal> {
        let mut covered = self
            .entries
            .get(binding)
            .map(|entry| entry.covered.clone())
            .unwrap_or_default();
        if let Some(retained) = self.entries.get(binding) {
            check_progress(binding, retained, assertion, positions)
                .map_err(LedgerRefusal::Contradiction)?;
        }
        covered.extend(positions.iter().map(|position| position.identity.clone()));
        let others: usize = self
            .entries
            .iter()
            .filter(|(key, _)| *key != binding)
            .map(|(_, entry)| entry.covered.len())
            .sum();
        let total = u64::try_from(others)
            .ok()
            .zip(u64::try_from(covered.len()).ok())
            .and_then(|(others, own)| others.checked_add(own));
        if total.is_none_or(|total| total > capacity) {
            return Err(LedgerRefusal::LimitReached(CoveredPositionsLimitReached {
                limit: COVERED_POSITIONS_LIMIT,
                value: capacity,
                setting: COVERED_POSITIONS_SETTING,
            }));
        }
        let mut entries = self.entries.clone();
        entries.insert(
            binding.clone(),
            LedgerEntry {
                watermark: assertion.watermark,
                completeness: assertion.completeness,
                scope: assertion.scope,
                execution: assertion.execution,
                covered,
            },
        );
        Ok(Self { entries })
    }
}

/// The first contradiction of `retained` by `assertion` over `positions`.
fn check_progress(
    binding: &LedgerBinding,
    retained: &LedgerEntry,
    assertion: ProgressAssertion,
    positions: &[ObservedPosition],
) -> Result<(), ProgressContradiction> {
    let contradiction =
        |dimension, retained: ProgressValue, asserted: ProgressValue| ProgressContradiction {
            dimension,
            clock: binding.clock.clone(),
            retained,
            asserted,
        };
    if assertion.watermark < retained.watermark {
        return Err(contradiction(
            ContradictedDimension::Watermark,
            ProgressValue::Watermark(retained.watermark),
            ProgressValue::Watermark(assertion.watermark),
        ));
    }
    if let ProgressCompleteness::Complete { through } = retained.completeness {
        let retreats = match assertion.completeness {
            ProgressCompleteness::Complete { through: asserted } => asserted < through,
            ProgressCompleteness::Incomplete => true,
        };
        if retreats {
            return Err(contradiction(
                ContradictedDimension::Completeness,
                ProgressValue::Completeness(retained.completeness),
                ProgressValue::Completeness(assertion.completeness),
            ));
        }
        if let Some(position) = positions.iter().find(|position| {
            position.stamp <= through && !retained.covered.contains(&position.identity)
        }) {
            return Err(contradiction(
                ContradictedDimension::Completeness,
                ProgressValue::Completeness(retained.completeness),
                ProgressValue::Position(position.clone()),
            ));
        }
    }
    if retained.scope == ProgressClosure::Closed && assertion.scope == ProgressClosure::Open {
        return Err(contradiction(
            ContradictedDimension::Closure,
            ProgressValue::DecisionScope(retained.scope),
            ProgressValue::DecisionScope(assertion.scope),
        ));
    }
    if retained.execution == ProgressClosure::Closed && assertion.execution == ProgressClosure::Open
    {
        return Err(contradiction(
            ContradictedDimension::Closure,
            ProgressValue::SurroundingExecution(retained.execution),
            ProgressValue::SurroundingExecution(assertion.execution),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    const CAPACITY: u64 = DEFAULT_COVERED_POSITIONS;

    fn binding() -> LedgerBinding {
        LedgerBinding {
            package: package_identity("pkg", "1"),
            clause: clause_identity("Refund", 0),
            clock: "ms".to_owned(),
            profile: "finite-window".to_owned(),
        }
    }

    fn package_identity(identity: &str, revision: &str) -> qsl_foundation::SourceIdentity {
        qsl_foundation::SourceIdentity {
            authority: "test".to_owned(),
            identity: identity.to_owned(),
            revision_namespace: "git".to_owned(),
            revision: revision.to_owned(),
        }
    }

    fn clause_identity(name: &str, ordinal: u64) -> quire_exact::NodeKey {
        quire_exact::NodeKey::new(
            "pkg",
            vec![quire_exact::PathSegment::Text(name.to_owned())],
            ordinal,
        )
        .expect("test clause identity is valid")
    }

    fn at(identity: &str, stamp: u64) -> ObservedPosition {
        ObservedPosition {
            identity: identity.to_owned(),
            stamp,
        }
    }

    fn assertion(watermark: u64, completeness: ProgressCompleteness) -> ProgressAssertion {
        ProgressAssertion {
            watermark,
            completeness,
            scope: ProgressClosure::Open,
            execution: ProgressClosure::Open,
        }
    }

    fn complete(through: u64) -> ProgressCompleteness {
        ProgressCompleteness::Complete { through }
    }

    fn zero_to_three() -> Vec<ObservedPosition> {
        (0..4).map(|n| at(&format!("p{n}"), n)).collect()
    }

    fn admit(
        ledger: &ProgressLedger,
        binding: &LedgerBinding,
        assertion: ProgressAssertion,
        positions: &[ObservedPosition],
    ) -> Result<ProgressLedger, LedgerRefusal> {
        ledger.admit(binding, assertion, positions, CAPACITY)
    }

    fn contradiction(refusal: LedgerRefusal) -> ProgressContradiction {
        match refusal {
            LedgerRefusal::Contradiction(contradiction) => contradiction,
            LedgerRefusal::LimitReached(reached) => panic!("unexpected limit {reached:?}"),
        }
    }

    fn retaining(assertion: ProgressAssertion, positions: &[ObservedPosition]) -> ProgressLedger {
        admit(&ProgressLedger::new(), &binding(), assertion, positions).expect("first assertion")
    }

    #[trace("FR-502-AC-1", "TC-1296")]
    #[test]
    fn a_regressing_watermark_is_refused_and_the_ledger_keeps_the_retained_one() {
        let ledger = retaining(assertion(100, ProgressCompleteness::Incomplete), &[]);
        let refusal = admit(
            &ledger,
            &binding(),
            assertion(60, ProgressCompleteness::Incomplete),
            &[],
        )
        .expect_err("regression");
        assert_eq!(
            contradiction(refusal),
            ProgressContradiction {
                dimension: ContradictedDimension::Watermark,
                clock: "ms".to_owned(),
                retained: ProgressValue::Watermark(100),
                asserted: ProgressValue::Watermark(60),
            }
        );
        assert_eq!(
            ledger.entry(&binding()).map(LedgerEntry::watermark),
            Some(100)
        );
        let same = admit(
            &ledger,
            &binding(),
            assertion(100, ProgressCompleteness::Incomplete),
            &[],
        )
        .expect("equal watermark");
        let later = admit(
            &same,
            &binding(),
            assertion(130, ProgressCompleteness::Incomplete),
            &[],
        )
        .expect("later watermark");
        assert_eq!(
            later.entry(&binding()).map(LedgerEntry::watermark),
            Some(130)
        );
    }

    #[trace("FR-502-AC-2", "TC-1296")]
    #[test]
    fn a_retreating_completeness_or_an_uncovered_position_is_refused() {
        let ledger = retaining(assertion(100, complete(100)), &zero_to_three());
        for (asserted, positions, value) in [
            (
                complete(60),
                Vec::new(),
                ProgressValue::Completeness(complete(60)),
            ),
            (
                ProgressCompleteness::Incomplete,
                Vec::new(),
                ProgressValue::Completeness(ProgressCompleteness::Incomplete),
            ),
            (
                complete(100),
                vec![at("late", 80)],
                ProgressValue::Position(at("late", 80)),
            ),
        ] {
            let refusal = admit(&ledger, &binding(), assertion(100, asserted), &positions)
                .expect_err("completeness contradiction");
            let found = contradiction(refusal);
            assert_eq!(found.dimension, ContradictedDimension::Completeness);
            assert_eq!(found.retained, ProgressValue::Completeness(complete(100)));
            assert_eq!(found.asserted, value);
        }
        assert_eq!(
            ledger.entry(&binding()).map(LedgerEntry::completeness),
            Some(complete(100))
        );
        let same = admit(&ledger, &binding(), assertion(100, complete(100)), &[]).expect("same");
        let later = admit(&same, &binding(), assertion(100, complete(140)), &[]).expect("later");
        assert_eq!(
            later.entry(&binding()).map(LedgerEntry::completeness),
            Some(complete(140))
        );
    }

    #[trace("FR-502-AC-2", "TC-1296")]
    #[test]
    fn an_incomplete_assertion_after_an_incomplete_one_is_accepted() {
        let ledger = retaining(assertion(5, ProgressCompleteness::Incomplete), &[]);
        let next = admit(
            &ledger,
            &binding(),
            assertion(5, ProgressCompleteness::Incomplete),
            &[],
        )
        .expect("incomplete after incomplete");
        assert_eq!(
            next.entry(&binding()).map(LedgerEntry::completeness),
            Some(ProgressCompleteness::Incomplete)
        );
    }

    #[trace("FR-502-AC-3", "TC-1296")]
    #[test]
    fn a_closed_scope_or_execution_does_not_reopen_and_closure_is_independent_of_completeness() {
        let closed = ProgressAssertion {
            scope: ProgressClosure::Closed,
            execution: ProgressClosure::Closed,
            ..assertion(10, ProgressCompleteness::Incomplete)
        };
        let ledger = retaining(closed, &[]);
        let entry = ledger.entry(&binding()).expect("entry");
        assert_eq!(entry.completeness(), ProgressCompleteness::Incomplete);
        assert_eq!(entry.scope(), ProgressClosure::Closed);

        let reopen_scope = ProgressAssertion {
            scope: ProgressClosure::Open,
            ..closed
        };
        let found =
            contradiction(admit(&ledger, &binding(), reopen_scope, &[]).expect_err("scope reopen"));
        assert_eq!(found.dimension, ContradictedDimension::Closure);
        assert_eq!(
            found.retained,
            ProgressValue::DecisionScope(ProgressClosure::Closed)
        );
        assert_eq!(
            found.asserted,
            ProgressValue::DecisionScope(ProgressClosure::Open)
        );

        let reopen_execution = ProgressAssertion {
            execution: ProgressClosure::Open,
            ..closed
        };
        let found = contradiction(
            admit(&ledger, &binding(), reopen_execution, &[]).expect_err("execution reopen"),
        );
        assert_eq!(found.dimension, ContradictedDimension::Closure);
        assert_eq!(
            found.asserted,
            ProgressValue::SurroundingExecution(ProgressClosure::Open)
        );

        let completed = retaining(assertion(100, complete(100)), &[]);
        let entry = completed.entry(&binding()).expect("entry");
        assert_eq!(entry.completeness(), complete(100));
        assert_eq!(entry.scope(), ProgressClosure::Open);
    }

    #[trace("FR-502-AC-4", "TC-1296")]
    #[test]
    fn a_foreign_binding_is_its_own_entry_and_leaves_the_binding_unchanged() {
        let base = retaining(assertion(100, complete(100)), &zero_to_three());
        let before = base.entry(&binding()).cloned();
        let variants = [
            LedgerBinding {
                package: package_identity("other", "1"),
                ..binding()
            },
            LedgerBinding {
                clause: clause_identity("Other", 0),
                ..binding()
            },
            LedgerBinding {
                clock: "us".to_owned(),
                ..binding()
            },
            LedgerBinding {
                profile: "other".to_owned(),
                ..binding()
            },
        ];
        let mut ledger = base.clone();
        for variant in &variants {
            // A lower watermark and Incomplete would contradict `binding()`,
            // so acceptance shows the variant is a different key.
            ledger = admit(
                &ledger,
                variant,
                assertion(1, ProgressCompleteness::Incomplete),
                &[],
            )
            .expect("foreign binding");
        }
        assert_eq!(ledger.len(), 5);
        assert_eq!(ledger.entry(&binding()).cloned(), before);
        for variant in &variants {
            assert_eq!(ledger.entry(variant).map(LedgerEntry::watermark), Some(1));
        }
    }

    #[trace("FR-502-AC-6", "TC-1296")]
    #[test]
    fn the_returned_ledger_replays_the_same_sequence_and_a_fresh_ledger_accepts_a_lower_watermark()
    {
        let first = assertion(100, ProgressCompleteness::Incomplete);
        let second = assertion(60, ProgressCompleteness::Incomplete);
        let empty = ProgressLedger::new();
        // With no carried ledger, each call stands alone.
        assert!(admit(&empty, &binding(), first, &[]).is_ok());
        assert!(admit(&empty, &binding(), second, &[]).is_ok());
        // With a carried ledger the second is refused.
        let after_first = admit(&empty, &binding(), first, &[]).expect("first");
        assert!(admit(&after_first, &binding(), second, &[]).is_err());
        // The same sequence from the same initial ledger gives the same outcomes.
        let sequence = [
            first,
            assertion(130, ProgressCompleteness::Incomplete),
            second,
        ];
        let run = || {
            let mut ledger = ProgressLedger::new();
            let mut outcomes = Vec::new();
            for step in sequence {
                match admit(&ledger, &binding(), step, &[]) {
                    Ok(next) => {
                        outcomes.push(Ok(step.watermark));
                        ledger = next;
                    }
                    Err(refusal) => outcomes.push(Err(refusal)),
                }
            }
            (outcomes, ledger)
        };
        assert_eq!(run(), run());
        assert_eq!(
            run().1.entry(&binding()).map(LedgerEntry::watermark),
            Some(130)
        );
    }

    #[trace("FR-502-AC-7", "TC-1296")]
    #[test]
    fn a_new_position_inside_the_retained_extent_is_refused_and_one_beyond_it_is_covered() {
        let ledger = retaining(assertion(100, complete(100)), &zero_to_three());
        let found = contradiction(
            admit(
                &ledger,
                &binding(),
                assertion(100, complete(100)),
                &[at("p80", 80)],
            )
            .expect_err("uncovered position at 80"),
        );
        assert_eq!(found.dimension, ContradictedDimension::Completeness);
        let next = admit(
            &ledger,
            &binding(),
            assertion(120, complete(120)),
            &[at("p120", 120)],
        )
        .expect("position at 120");
        let covered: Vec<&str> = next.entry(&binding()).expect("entry").covered().collect();
        assert_eq!(covered, ["p0", "p1", "p120", "p2", "p3"]);
    }

    #[trace("FR-502-AC-8", "TC-1296")]
    #[test]
    fn the_covered_set_is_bounded_and_a_later_trace_may_omit_covered_positions() {
        let ledger = ProgressLedger::new();
        let over = ledger
            .admit(&binding(), assertion(3, complete(3)), &zero_to_three(), 3)
            .expect_err("four positions over a capacity of three");
        assert_eq!(
            over,
            LedgerRefusal::LimitReached(CoveredPositionsLimitReached {
                limit: "node count",
                value: 3,
                setting: "progress.covered_positions",
            })
        );
        assert!(ledger.is_empty());

        let three: Vec<ObservedPosition> = (0..3).map(|n| at(&format!("p{n}"), n)).collect();
        let held = ledger
            .admit(&binding(), assertion(3, complete(3)), &three, 4)
            .expect("three positions");
        let next = held
            .admit(
                &binding(),
                assertion(9, complete(3)),
                &[at("p0", 0), at("far", 9)],
                4,
            )
            .expect("omits p1 and p2, adds one beyond through");
        let covered: Vec<&str> = next.entry(&binding()).expect("entry").covered().collect();
        assert_eq!(covered, ["far", "p0", "p1", "p2"]);
        let refused = next
            .admit(
                &binding(),
                assertion(10, complete(10)),
                &[at("more", 10)],
                4,
            )
            .expect_err("fifth covered position");
        assert!(matches!(refused, LedgerRefusal::LimitReached(_)));
        assert_eq!(next.entry(&binding()).expect("entry").covered().count(), 4);
    }
}
