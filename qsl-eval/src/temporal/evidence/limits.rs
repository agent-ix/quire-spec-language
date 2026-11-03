// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-523: the thirteen counters of one temporal evidence call, their
//! ceilings and the settings that set them (FR-255 `temporal_evidence.*`).

use qsl_foundation::diagnostic::LimitKind;
use qsl_foundation::{Setting, SettingLimits};

/// Whether a counter accumulates over the call or keeps its peak.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CounterKind {
    /// The sum of every charge (`positions`, `valuations`, `captures`,
    /// `evaluation_steps`, `visited`).
    Cumulative,
    /// The greatest value any one charge named.
    Peak,
}

/// One of the thirteen counters of a temporal evidence call (FR-523
/// "Counters"). `formula_depth` and `json_depth` are not counters: formula
/// nesting is not a limit (ADR-030 D-1) and a typed value has the nesting
/// its type gives.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EvidenceCounter {
    /// A distinct node of the clause's formula reachable from its root.
    FormulaNodes,
    /// One instance created from a delivery or an origin.
    Instances,
    /// One scan charge: one position read by one temporal operator node at
    /// one activation.
    Positions,
    /// One atomic `holds` lookup.
    Valuations,
    /// One capture record retained.
    Captures,
    /// A `(position, atom)` pair of retained or settled support.
    Support,
    /// The clause's greatest interval reach, in ticks.
    HistorySpan,
    /// One charge at the `temporal.visit` charge point.
    EvaluationSteps,
    /// The length of the result's chain of predecessors.
    Lineage,
    /// One unit of the usage of `formula_nodes`, `positions`, `valuations`
    /// and `captures`, and one evicted record consulted.
    Visited,
    /// One byte of the request's opaque byte members, in total.
    RequestBytes,
    /// One byte of the result's opaque byte members, in total.
    ResultBytes,
    /// One byte of the longest single opaque member.
    MemberBytes,
}

impl EvidenceCounter {
    /// Every counter, in FR-255's table order.
    pub const ALL: [Self; 13] = [
        Self::FormulaNodes,
        Self::Instances,
        Self::Positions,
        Self::Valuations,
        Self::Captures,
        Self::Support,
        Self::HistorySpan,
        Self::EvaluationSteps,
        Self::Lineage,
        Self::Visited,
        Self::RequestBytes,
        Self::ResultBytes,
        Self::MemberBytes,
    ];

    /// The counter's name, which is also its setting's last segment.
    pub const fn name(self) -> &'static str {
        match self {
            Self::FormulaNodes => "formula_nodes",
            Self::Instances => "instances",
            Self::Positions => "positions",
            Self::Valuations => "valuations",
            Self::Captures => "captures",
            Self::Support => "support",
            Self::HistorySpan => "history_span",
            Self::EvaluationSteps => "evaluation_steps",
            Self::Lineage => "lineage",
            Self::Visited => "visited",
            Self::RequestBytes => "request_bytes",
            Self::ResultBytes => "result_bytes",
            Self::MemberBytes => "member_bytes",
        }
    }

    /// The setting `temporal_evidence.<counter>` that sets the ceiling.
    pub const fn setting(self) -> Setting {
        match self {
            Self::FormulaNodes => Setting::TemporalEvidenceFormulaNodes,
            Self::Instances => Setting::TemporalEvidenceInstances,
            Self::Positions => Setting::TemporalEvidencePositions,
            Self::Valuations => Setting::TemporalEvidenceValuations,
            Self::Captures => Setting::TemporalEvidenceCaptures,
            Self::Support => Setting::TemporalEvidenceSupport,
            Self::HistorySpan => Setting::TemporalEvidenceHistorySpan,
            Self::EvaluationSteps => Setting::TemporalEvidenceEvaluationSteps,
            Self::Lineage => Setting::TemporalEvidenceLineage,
            Self::Visited => Setting::TemporalEvidenceVisited,
            Self::RequestBytes => Setting::TemporalEvidenceRequestBytes,
            Self::ResultBytes => Setting::TemporalEvidenceResultBytes,
            Self::MemberBytes => Setting::TemporalEvidenceMemberBytes,
        }
    }

    /// The counter whose setting is `setting`, when it is one of the
    /// thirteen.
    pub fn of_setting(setting: Setting) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|counter| counter.setting() == setting)
    }

    /// The limit kind a stop at this counter reports (FR-255).
    pub const fn limit_kind(self) -> LimitKind {
        self.setting().kind()
    }

    /// Whether the counter accumulates or keeps a peak (FR-523
    /// "Counters").
    pub const fn kind(self) -> CounterKind {
        match self {
            Self::Positions
            | Self::Valuations
            | Self::Captures
            | Self::EvaluationSteps
            | Self::Visited => CounterKind::Cumulative,
            Self::FormulaNodes
            | Self::Instances
            | Self::Support
            | Self::HistorySpan
            | Self::Lineage
            | Self::RequestBytes
            | Self::ResultBytes
            | Self::MemberBytes => CounterKind::Peak,
        }
    }
}

/// The thirteen ceilings of one temporal evidence call. Each is used as
/// given, above or below its default, and zero is a real ceiling (FR-523
/// "Counters"; FR-255).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TemporalEvidenceLimits {
    /// `temporal_evidence.formula_nodes`.
    pub formula_nodes: u64,
    /// `temporal_evidence.instances`.
    pub instances: u64,
    /// `temporal_evidence.positions`.
    pub positions: u64,
    /// `temporal_evidence.valuations`.
    pub valuations: u64,
    /// `temporal_evidence.captures`.
    pub captures: u64,
    /// `temporal_evidence.support`.
    pub support: u64,
    /// `temporal_evidence.history_span`.
    pub history_span: u64,
    /// `temporal_evidence.evaluation_steps`.
    pub evaluation_steps: u64,
    /// `temporal_evidence.lineage`.
    pub lineage: u64,
    /// `temporal_evidence.visited`.
    pub visited: u64,
    /// `temporal_evidence.request_bytes`.
    pub request_bytes: u64,
    /// `temporal_evidence.result_bytes`.
    pub result_bytes: u64,
    /// `temporal_evidence.member_bytes`.
    pub member_bytes: u64,
}

impl Default for TemporalEvidenceLimits {
    /// FR-255's defaults.
    fn default() -> Self {
        Self {
            formula_nodes: 100_000,
            instances: 10_000,
            positions: 1_000_000,
            valuations: 1_000_000,
            captures: 100_000,
            support: 1_000_000,
            history_span: u64::MAX,
            evaluation_steps: 1_000_000,
            lineage: 1_024,
            visited: 2_000_000,
            request_bytes: 8_388_608,
            result_bytes: 8_388_608,
            member_bytes: 1_048_576,
        }
    }
}

impl TemporalEvidenceLimits {
    /// The ceiling of `counter`.
    pub const fn ceiling(&self, counter: EvidenceCounter) -> u64 {
        match counter {
            EvidenceCounter::FormulaNodes => self.formula_nodes,
            EvidenceCounter::Instances => self.instances,
            EvidenceCounter::Positions => self.positions,
            EvidenceCounter::Valuations => self.valuations,
            EvidenceCounter::Captures => self.captures,
            EvidenceCounter::Support => self.support,
            EvidenceCounter::HistorySpan => self.history_span,
            EvidenceCounter::EvaluationSteps => self.evaluation_steps,
            EvidenceCounter::Lineage => self.lineage,
            EvidenceCounter::Visited => self.visited,
            EvidenceCounter::RequestBytes => self.request_bytes,
            EvidenceCounter::ResultBytes => self.result_bytes,
            EvidenceCounter::MemberBytes => self.member_bytes,
        }
    }

    /// These limits with `counter`'s ceiling set to `ceiling` and no other
    /// changed.
    #[must_use]
    pub const fn with_ceiling(mut self, counter: EvidenceCounter, ceiling: u64) -> Self {
        let field = match counter {
            EvidenceCounter::FormulaNodes => &mut self.formula_nodes,
            EvidenceCounter::Instances => &mut self.instances,
            EvidenceCounter::Positions => &mut self.positions,
            EvidenceCounter::Valuations => &mut self.valuations,
            EvidenceCounter::Captures => &mut self.captures,
            EvidenceCounter::Support => &mut self.support,
            EvidenceCounter::HistorySpan => &mut self.history_span,
            EvidenceCounter::EvaluationSteps => &mut self.evaluation_steps,
            EvidenceCounter::Lineage => &mut self.lineage,
            EvidenceCounter::Visited => &mut self.visited,
            EvidenceCounter::RequestBytes => &mut self.request_bytes,
            EvidenceCounter::ResultBytes => &mut self.result_bytes,
            EvidenceCounter::MemberBytes => &mut self.member_bytes,
        };
        *field = ceiling;
        self
    }
}

impl TemporalEvidenceLimits {
    /// The record with every member zero, which [`EvidenceUsage`] starts
    /// from.
    const ZERO: Self = Self {
        formula_nodes: 0,
        instances: 0,
        positions: 0,
        valuations: 0,
        captures: 0,
        support: 0,
        history_span: 0,
        evaluation_steps: 0,
        lineage: 0,
        visited: 0,
        request_bytes: 0,
        result_bytes: 0,
        member_bytes: 0,
    };
}

impl SettingLimits for TemporalEvidenceLimits {
    fn bounds(&self) -> Vec<(Setting, u64)> {
        EvidenceCounter::ALL
            .into_iter()
            .map(|counter| (counter.setting(), self.ceiling(counter)))
            .collect()
    }

    fn set_bound(&mut self, setting: Setting, bound: u64) -> bool {
        match EvidenceCounter::of_setting(setting) {
            Some(counter) => {
                *self = self.with_ceiling(counter, bound);
                true
            }
            None => false,
        }
    }
}

/// The measured value of each of the thirteen counters of a call (FR-516
/// `usage`).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EvidenceUsage(TemporalEvidenceLimits);

impl Default for EvidenceUsage {
    fn default() -> Self {
        Self(TemporalEvidenceLimits::ZERO)
    }
}

impl EvidenceUsage {
    /// The value of `counter`.
    pub const fn get(&self, counter: EvidenceCounter) -> u64 {
        self.0.ceiling(counter)
    }

    pub(crate) const fn with(self, counter: EvidenceCounter, value: u64) -> Self {
        Self(self.0.with_ceiling(counter, value))
    }

    /// Every counter with its value, in FR-255's table order.
    pub fn entries(&self) -> [(EvidenceCounter, u64); 13] {
        EvidenceCounter::ALL.map(|counter| (counter, self.get(counter)))
    }
}
