// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-247's outcome of a closed-form analysis and the evidence it carries
//! (ADR-026 RT-7), the vocabulary EN-7 returns and the checker recomputes.

use quire_exact::{Identifier, Rational};
use serde::Serialize;

use crate::proof_result::IncompleteCause;

/// ADR-026 RT-7's analysis, QSpec FR-419's spelling.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClosedFormAnalysis {
    /// Fixed-priority response-time analysis.
    #[serde(rename = "fixed-priority")]
    FixedPriorityRta,
    /// The EDF processor-demand test with Quick Processor-demand Analysis.
    #[serde(rename = "edf")]
    EdfQpa,
    /// AMC-rtb.
    #[serde(rename = "amc-rtb")]
    AmcRtb,
}

/// The iterates of one job's recurrence: the start, then each step. A
/// satisfied job ends with its fixpoint repeated, as `5, 9, 12, 12`; a job
/// that misses ends with the first iterate whose response time exceeds its
/// bound.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JobIterates(pub Vec<Rational>);

impl JobIterates {
    /// The last iterate: the job's fixpoint when it is satisfied.
    pub fn last(&self) -> Option<&Rational> {
        self.0.last()
    }
}

/// One task's evidence: the iterates of each job of its level-`i` busy
/// period, job 0 first. A task whose busy period ends with its first job
/// holds one entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskFixpoint {
    /// The task.
    pub task: Identifier,
    /// Each job's iterates.
    pub jobs: Vec<JobIterates>,
}

/// The failing high-mode criterion of an AMC-rtb test: the iterates of a
/// `HI` task's high-mode recurrence, ending past its bound.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HighModeMiss {
    /// The `HI` task.
    pub task: Identifier,
    /// Its high-mode iterates.
    pub iterates: Vec<Rational>,
}

/// AMC-rtb's evidence (RT-6).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AmcEvidence {
    /// Every task's low-mode fixpoint.
    pub low: Vec<TaskFixpoint>,
    /// The `HI` tasks' high-mode fixpoints, up to the failing one.
    pub high: Vec<TaskFixpoint>,
    /// The high-mode criterion that failed; none when the set is schedulable.
    pub failing: Option<HighModeMiss>,
}

/// The evidence of a result that is not a miss (RT-7).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClosedFormEvidence {
    /// Fixed priority: each analysed task's fixpoints.
    FixedPoints(Vec<TaskFixpoint>),
    /// EDF: the synchronous busy-period length and QPA's check sequence.
    Demand {
        /// The busy-period length.
        busy_period: Rational,
        /// The `t` values QPA checked, in order.
        qpa_sequence: Vec<Rational>,
    },
    /// AMC-rtb.
    Amc(AmcEvidence),
}

/// An EDF demand miss: a `t` with `dbf(t) > t`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DemandWitness {
    /// The point.
    pub t: Rational,
    /// `dbf(t)`.
    pub demand: Rational,
}

/// The evidence of a miss (RT-7).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MissEvidence {
    /// A fixed-priority miss (low mode, for AMC): the task, the job `q` of
    /// its level-`i` busy period that misses, and that job's iterates up to
    /// the first whose response time `R + J_i - q * T_i` exceeds its bound.
    FixedPriority {
        /// The task.
        task: Identifier,
        /// The job `q`.
        job: u64,
        /// The iterates.
        iterates: Vec<Rational>,
    },
    /// An EDF demand miss.
    Edf(DemandWitness),
    /// Utilization above 1.
    Utilization(Rational),
}

/// The setting an EN-7 stop names (FR-247).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScheduleLimit {
    /// `max_demand_points`, with the value in force.
    MaxDemandPoints(u64),
}

/// EN-7's result for a claim (FR-247).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClosedFormOutcome {
    /// The claim holds.
    Schedulable(ClosedFormEvidence),
    /// The claim fails.
    Miss(MissEvidence),
    /// AMC-rtb's high-mode criterion failed: the sufficient test decides
    /// nothing.
    SufficientTestFailed(ClosedFormEvidence),
    /// A budget or the caller stopped the analysis.
    Stopped(IncompleteCause, ScheduleLimit),
}
