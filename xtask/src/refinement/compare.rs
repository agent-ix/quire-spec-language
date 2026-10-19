// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-343: compare the prior and superseding classes of one case into one
//! case result. A pure function of the two classes, their stages and the
//! case's selection kind.

use qsl_replay::spine::{ClauseRunSelection, ClauseRunStage, RefinementClass};
use serde::Serialize;

/// The kind of selection a case makes (FR-109, FR-115).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionKind {
    /// A state clause or a temporal clause.
    Clause,
    /// A Boolean function run as a claim.
    Function,
    /// An operation frame.
    Frame,
}

impl SelectionKind {
    /// The kind of `selection`.
    pub fn of(selection: &ClauseRunSelection) -> Self {
        match selection {
            // A temporal clause is a clause: one declared by name, whose
            // absence in the superseding revision drops a constraint.
            ClauseRunSelection::Clause(_) | ClauseRunSelection::Temporal { .. } => Self::Clause,
            ClauseRunSelection::Function { .. } => Self::Function,
            ClauseRunSelection::Frame { .. } => Self::Frame,
        }
    }
}

/// One side of a case: the class of its run and the stage it settled at.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Side {
    /// The FR-342 class.
    pub class: RefinementClass,
    /// The FR-109 stage.
    pub stage: ClauseRunStage,
}

/// FR-343's case result.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum CaseResult {
    /// The superseding revision admits what the prior admitted.
    #[serde(rename = "holds")]
    Holds,
    /// The prior revision admits and the superseding does not.
    #[serde(rename = "regression")]
    Regression,
    /// A QSL implementation gap stopped the comparison.
    #[serde(rename = "unresolved (unsupported)")]
    UnresolvedUnsupported,
    /// A limit or budget stopped the comparison.
    #[serde(rename = "unresolved (incomplete)")]
    UnresolvedIncomplete,
    /// The prior revision does not admit the case.
    #[serde(rename = "not applicable")]
    NotApplicable,
    /// The case's own setup is broken.
    #[serde(rename = "tool failure")]
    ToolFailure,
}

/// FR-343: the first row of the comparison table that matches.
pub fn compare(prior: Side, superseding: Side, kind: SelectionKind) -> CaseResult {
    use CaseResult::{
        Holds, NotApplicable, Regression, ToolFailure, UnresolvedIncomplete, UnresolvedUnsupported,
    };
    use RefinementClass::{Absent, Admitted, Incomplete, Prohibited, Refused, Unsupported};
    let prior_settled_early = matches!(
        prior.stage,
        ClauseRunStage::Compile | ClauseRunStage::Select
    );
    match (prior.class, superseding.class) {
        (RefinementClass::ToolFailure, _) | (_, RefinementClass::ToolFailure) => ToolFailure,
        (Incomplete, _) => UnresolvedIncomplete,
        (Unsupported, _) => UnresolvedUnsupported,
        // The prior revision must compile and select; the case is malformed.
        (Admitted | Refused | Prohibited | Absent, _) if prior_settled_early => ToolFailure,
        (Admitted, Admitted) => Holds,
        // The constraint was dropped; a frame exists only while a clause or
        // attempt names its operation (FR-105).
        (Admitted, Absent) => match kind {
            SelectionKind::Clause | SelectionKind::Frame => Holds,
            SelectionKind::Function => Regression,
        },
        (Admitted, Refused | Prohibited) => Regression,
        (Admitted, Unsupported) => UnresolvedUnsupported,
        (Admitted, Incomplete) => UnresolvedIncomplete,
        (Refused | Prohibited | Absent, _) => NotApplicable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    use CaseResult::{
        Holds as H, NotApplicable as N, Regression as R, ToolFailure as T,
        UnresolvedIncomplete as I, UnresolvedUnsupported as U,
    };

    /// The superseding classes, in the column order of [`TABLE`].
    const SUPERSEDING: [RefinementClass; 7] = [
        RefinementClass::Admitted,
        RefinementClass::Refused,
        RefinementClass::Prohibited,
        RefinementClass::Unsupported,
        RefinementClass::Incomplete,
        RefinementClass::ToolFailure,
        RefinementClass::Absent,
    ];

    /// How the prior stage splits the table: the prior revision must compile
    /// and select, so those two stages differ from `admit` and `evaluate`.
    #[derive(Clone, Copy)]
    enum Stages {
        Early,
        Late,
        Any,
    }

    /// The expected result, written out by hand from FR-343's rows. `None`
    /// in the last column of the `admitted` row means the result depends on
    /// the selection kind (see [`absent_superseding`]).
    type Row = (RefinementClass, Stages, [Option<CaseResult>; 7]);

    const fn s(result: CaseResult) -> Option<CaseResult> {
        Some(result)
    }

    const TABLE: [Row; 11] = [
        (
            RefinementClass::Admitted,
            Stages::Early,
            [s(T), s(T), s(T), s(T), s(T), s(T), s(T)],
        ),
        (
            RefinementClass::Admitted,
            Stages::Late,
            [s(H), s(R), s(R), s(U), s(I), s(T), None],
        ),
        (
            RefinementClass::Refused,
            Stages::Early,
            [s(T), s(T), s(T), s(T), s(T), s(T), s(T)],
        ),
        (
            RefinementClass::Refused,
            Stages::Late,
            [s(N), s(N), s(N), s(N), s(N), s(T), s(N)],
        ),
        (
            RefinementClass::Prohibited,
            Stages::Early,
            [s(T), s(T), s(T), s(T), s(T), s(T), s(T)],
        ),
        (
            RefinementClass::Prohibited,
            Stages::Late,
            [s(N), s(N), s(N), s(N), s(N), s(T), s(N)],
        ),
        (
            RefinementClass::Absent,
            Stages::Early,
            [s(T), s(T), s(T), s(T), s(T), s(T), s(T)],
        ),
        (
            RefinementClass::Absent,
            Stages::Late,
            [s(N), s(N), s(N), s(N), s(N), s(T), s(N)],
        ),
        (
            RefinementClass::Unsupported,
            Stages::Any,
            [s(U), s(U), s(U), s(U), s(U), s(T), s(U)],
        ),
        (
            RefinementClass::Incomplete,
            Stages::Any,
            [s(I), s(I), s(I), s(I), s(I), s(T), s(I)],
        ),
        (
            RefinementClass::ToolFailure,
            Stages::Any,
            [s(T), s(T), s(T), s(T), s(T), s(T), s(T)],
        ),
    ];

    const STAGES: [ClauseRunStage; 4] = [
        ClauseRunStage::Compile,
        ClauseRunStage::Select,
        ClauseRunStage::Admit,
        ClauseRunStage::Evaluate,
    ];

    const KINDS: [SelectionKind; 3] = [
        SelectionKind::Clause,
        SelectionKind::Function,
        SelectionKind::Frame,
    ];

    /// `admitted` against `absent`: a dropped clause or frame holds, a dropped
    /// function regresses.
    fn absent_superseding(kind: SelectionKind) -> CaseResult {
        match kind {
            SelectionKind::Clause | SelectionKind::Frame => H,
            SelectionKind::Function => R,
        }
    }

    fn applies(stages: Stages, stage: ClauseRunStage) -> bool {
        let early = matches!(stage, ClauseRunStage::Compile | ClauseRunStage::Select);
        match stages {
            Stages::Early => early,
            Stages::Late => !early,
            Stages::Any => true,
        }
    }

    /// FR-343-AC-1: every prior class and stage, superseding class and stage
    /// and selection kind gives the row the literal table states. The table
    /// is written here by hand and calls none of the gate's code.
    #[test]
    #[trace("TC-863", "FR-343-AC-1")]
    fn every_class_stage_and_kind_combination_gives_the_tables_result() {
        let mut combinations = 0_usize;
        for (prior_class, stages, expected) in TABLE {
            for prior_stage in STAGES.into_iter().filter(|stage| applies(stages, *stage)) {
                for (column, superseding_class) in SUPERSEDING.into_iter().enumerate() {
                    for superseding_stage in STAGES {
                        for kind in KINDS {
                            let want = expected[column].unwrap_or_else(|| absent_superseding(kind));
                            let got = compare(
                                Side {
                                    class: prior_class,
                                    stage: prior_stage,
                                },
                                Side {
                                    class: superseding_class,
                                    stage: superseding_stage,
                                },
                                kind,
                            );
                            assert_eq!(
                                got, want,
                                "{prior_class:?}/{prior_stage:?} against \
                                 {superseding_class:?}/{superseding_stage:?} for {kind:?}"
                            );
                            combinations += 1;
                        }
                    }
                }
            }
        }
        // 7 classes x 4 stages x 7 classes x 4 stages x 3 kinds.
        assert_eq!(combinations, 7 * 4 * 7 * 4 * 3);
    }
}
