// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-344: the report of a refinement gate. It lists every result in its
//! group order, gives the run one verdict and maps that verdict to the exit
//! code (QSpec FR-301). [`Verdict::exit_code`] is the one place a refinement
//! gate's exit code is chosen.

use qsl_foundation::source::provenance::RawSourceRef;
use qsl_foundation::{Setting, SettingLimits};
use qsl_replay::spine::{
    Classification, ClauseRunStage, ReachedLimit, RefinementClass, RefusalCause,
};
use qsl_replay::ModelNormalizationLimits;
use serde::Serialize;

use super::compare::CaseResult;

/// The verdict of one run: the highest-severity result present (QSpec FR-301's
/// order, lowest first).
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum Verdict {
    /// Every result holds or does not apply.
    #[serde(rename = "success")]
    Success,
    /// A regression is present.
    #[serde(rename = "violation")]
    Violation,
    /// An unresolved result stopped at a limit or budget.
    #[serde(rename = "incomplete")]
    Incomplete,
    /// An unresolved result stopped at a QSL implementation gap.
    #[serde(rename = "unsupported")]
    Unsupported,
    /// A tool failure is present.
    #[serde(rename = "tool failure")]
    ToolFailure,
}

impl Verdict {
    /// The verdict a case result gives.
    pub const fn of(result: CaseResult) -> Self {
        match result {
            CaseResult::Holds | CaseResult::NotApplicable => Self::Success,
            CaseResult::Regression => Self::Violation,
            CaseResult::UnresolvedIncomplete => Self::Incomplete,
            CaseResult::UnresolvedUnsupported => Self::Unsupported,
            CaseResult::ToolFailure => Self::ToolFailure,
        }
    }

    /// QSpec FR-301's exit code for this verdict: the only code a refinement
    /// gate exits with.
    pub const fn exit_code(self) -> u8 {
        match self {
            Self::Success => 0,
            Self::Violation => 10,
            Self::Unsupported => 21,
            Self::Incomplete => 22,
            Self::ToolFailure => 30,
        }
    }
}

/// How a versioning case is named: the pair's two sources and its selection.
/// Ordered by the prior then the superseding reference, then the selection.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct CaseName {
    /// The prior revision's source reference.
    pub prior: RawSourceRef,
    /// The superseding revision's source reference.
    pub superseding: RawSourceRef,
    /// The case's selection.
    pub selection: SelectionName,
}

/// The selection of a case, by the members that name what it selects.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionName {
    /// A state clause, over the observation the case binds to `observation`.
    Clause {
        /// The clause name.
        name: String,
        /// The case's key of the observation the clause reads.
        observation: String,
    },
    /// A function run as a claim.
    Function {
        /// The function name.
        name: String,
        /// The case's key of the snapshot the function reads.
        snapshot: String,
    },
    /// An operation frame.
    Frame {
        /// The operation `M::T::op`.
        operation: String,
        /// The case's key of the invocation document.
        invocation: String,
    },
}

/// The case member that raises a limit (FR-340 Inputs).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum LimitMember {
    /// `limits`: the spine stage limits.
    #[serde(rename = "limits")]
    Limits,
    /// `observation_limits`.
    #[serde(rename = "observation_limits")]
    ObservationLimits,
    /// `model_limits`.
    #[serde(rename = "model_limits")]
    ModelLimits,
    /// `accounting`.
    #[serde(rename = "accounting")]
    Accounting,
}

impl LimitMember {
    /// The member that raises `limit`: an accounting counter has no stage
    /// setting; the three observation settings belong to `observation_limits`
    /// and the model normalization settings to `model_limits`.
    pub fn of(limit: &ReachedLimit) -> Self {
        let Some(setting) = Setting::from_name(limit.name) else {
            return Self::Accounting;
        };
        let observation = matches!(
            setting,
            Setting::ObservationInputBytes
                | Setting::ObservationObjects
                | Setting::ObservationValues
        );
        let model = ModelNormalizationLimits::default()
            .bounds()
            .iter()
            .any(|(named, _)| *named == setting);
        if observation {
            Self::ObservationLimits
        } else if model {
            Self::ModelLimits
        } else {
            Self::Limits
        }
    }
}

/// Which side of a case a limit was reached on.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SideName {
    /// The prior revision's run.
    Prior,
    /// The superseding revision's run.
    Superseding,
}

/// A limit reached by an `incomplete` run, its value and the member that
/// raises it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct LimitReport {
    /// The side whose run reached it.
    pub side: SideName,
    /// The limit's name.
    pub name: &'static str,
    /// The limit's configured value.
    pub bound: u64,
    /// The case member that raises it.
    pub member: LimitMember,
}

/// One cause of a refusal, by catalog code and cause.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CodeReport {
    /// The catalog code.
    pub code: &'static str,
    /// The catalog cause under the code, where it names one.
    pub cause: Option<&'static str>,
}

/// One side of a case: its class, stage and codes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SideReport {
    /// The FR-342 class.
    pub class: &'static str,
    /// The FR-109 stage.
    pub stage: &'static str,
    /// Every cause's code, in order.
    pub codes: Vec<CodeReport>,
}

impl SideReport {
    /// The report of a run that settled at `stage` as `classification`.
    pub fn new(stage: ClauseRunStage, classification: &Classification) -> Self {
        Self {
            class: class_name(classification.class),
            stage: stage_name(stage),
            codes: classification.causes.iter().map(code_report).collect(),
        }
    }
}

fn code_report(cause: &RefusalCause) -> CodeReport {
    CodeReport {
        code: cause.code.as_str(),
        cause: cause.cause,
    }
}

/// The FR-342 class spelled as ADR-017 RF-2 spells it.
pub const fn class_name(class: RefinementClass) -> &'static str {
    match class {
        RefinementClass::Admitted => "admitted",
        RefinementClass::Refused => "refused",
        RefinementClass::Prohibited => "prohibited",
        RefinementClass::Unsupported => "unsupported",
        RefinementClass::Incomplete => "incomplete",
        RefinementClass::ToolFailure => "tool failure",
        RefinementClass::Absent => "absent",
    }
}

/// The FR-109 stage spelled as FR-109 spells it.
pub const fn stage_name(stage: ClauseRunStage) -> &'static str {
    match stage {
        ClauseRunStage::Compile => "compile",
        ClauseRunStage::Select => "select",
        ClauseRunStage::Admit => "admit",
        ClauseRunStage::Evaluate => "evaluate",
    }
}

/// The limits one side's `incomplete` class reached.
pub fn limit_reports(side: SideName, classification: &Classification) -> Vec<LimitReport> {
    classification
        .limits()
        .iter()
        .map(|limit| LimitReport {
            side,
            name: limit.name,
            bound: limit.bound,
            member: LimitMember::of(limit),
        })
        .collect()
}

/// A case result: the case's name, each side, any limit reached and the
/// result.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CaseEntry {
    /// The case's name.
    pub name: CaseName,
    /// The prior revision's run; absent when the case could not be formed
    /// and nothing ran.
    pub prior: Option<SideReport>,
    /// The superseding revision's run; absent when nothing ran.
    pub superseding: Option<SideReport>,
    /// Each limit an `incomplete` side reached.
    pub limits: Vec<LimitReport>,
    /// Why the case could not be formed (a tool failure of the case).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defect: Option<String>,
    /// The result.
    pub result: CaseResult,
}

/// A tool failure of the corpus as a whole (FR-340): it cannot be read as a
/// directory, or holds no pair.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CorpusFailure {
    /// The corpus path as given.
    pub corpus: String,
    /// The defect.
    pub defect: String,
    /// Always `tool failure`.
    pub result: CaseResult,
}

impl CorpusFailure {
    /// A corpus-level tool failure.
    pub fn new(corpus: impl Into<String>, defect: impl Into<String>) -> Self {
        Self {
            corpus: corpus.into(),
            defect: defect.into(),
            result: CaseResult::ToolFailure,
        }
    }
}

/// A tool failure of one pair (FR-340): its `pair.json` is malformed.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PairFailure {
    /// The `pair.json` path relative to the corpus.
    pub pair: String,
    /// The defect.
    pub defect: String,
    /// Always `tool failure`.
    pub result: CaseResult,
}

impl PairFailure {
    /// A pair-level tool failure.
    pub fn new(pair: impl Into<String>, defect: impl Into<String>) -> Self {
        Self {
            pair: pair.into(),
            defect: defect.into(),
            result: CaseResult::ToolFailure,
        }
    }
}

/// One entry of the report's `cases`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Entry {
    /// A corpus-level tool failure.
    Corpus(CorpusFailure),
    /// A pair-level tool failure.
    Pair(PairFailure),
    /// A case result.
    Case(Box<CaseEntry>),
}

impl Entry {
    fn verdict(&self) -> Verdict {
        match self {
            Self::Corpus(_) | Self::Pair(_) => Verdict::ToolFailure,
            Self::Case(case) => Verdict::of(case.result),
        }
    }
}

/// FR-344's report: every result, the verdict and its exit code.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Report {
    /// Every result: corpus-level tool failures, pair-level tool failures,
    /// then case results; each group ordered by name.
    pub cases: Vec<Entry>,
    /// The highest-severity result present.
    pub verdict: Verdict,
    /// The verdict's exit code.
    pub exit: u8,
}

impl Report {
    /// Order every result by group and by name, and give the verdict.
    pub fn new(
        mut corpus: Vec<CorpusFailure>,
        mut pairs: Vec<PairFailure>,
        mut cases: Vec<CaseEntry>,
    ) -> Self {
        corpus.sort_by(|a, b| a.corpus.cmp(&b.corpus));
        pairs.sort_by(|a, b| a.pair.cmp(&b.pair));
        cases.sort_by(|a, b| a.name.cmp(&b.name));
        let entries: Vec<Entry> = corpus
            .into_iter()
            .map(Entry::Corpus)
            .chain(pairs.into_iter().map(Entry::Pair))
            .chain(cases.into_iter().map(|case| Entry::Case(Box::new(case))))
            .collect();
        let verdict = entries
            .iter()
            .map(Entry::verdict)
            .max()
            .unwrap_or(Verdict::Success);
        Self {
            cases: entries,
            verdict,
            exit: verdict.exit_code(),
        }
    }

    /// The report as JSON text: byte-equal for equal results.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let mut text = serde_json::to_string_pretty(self)?;
        text.push('\n');
        Ok(text)
    }
}
