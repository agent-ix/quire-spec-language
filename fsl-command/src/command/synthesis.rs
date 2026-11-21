// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-754: bounded candidate enumeration for value synthesis.

use filament_exact::{Integer, IntegerInterval, ValueType};
use fsl_foundation::Code;
use serde::Serialize;
use serde_json::Value;
use std::path::Path;
use std::time::{Duration, Instant};

const DEFAULT_MAX_CANDIDATES: u64 = 10_000_000;
const DEFAULT_MAX_WORK: u64 = 100_000_000;

/// The two independent limits applied while enumerating a bounded integer
/// domain. Each candidate consumes one unit from each counter.
#[derive(Clone, Copy, Debug)]
pub(super) struct SearchLimits {
    pub(super) max_candidates: Option<u64>,
    pub(super) max_work: Option<u64>,
    deadline: Option<Instant>,
}

impl Default for SearchLimits {
    fn default() -> Self {
        Self {
            max_candidates: Some(DEFAULT_MAX_CANDIDATES),
            max_work: Some(DEFAULT_MAX_WORK),
            deadline: None,
        }
    }
}

impl SearchLimits {
    fn deadline_passed(&self) -> bool {
        self.deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
    }

    /// The limit that stops the run once `work` units are spent, before any
    /// further evaluation.
    fn work_stop(&self, work: u64) -> Option<StopCause> {
        if let Some(maximum) = self.max_work.filter(|maximum| work >= *maximum) {
            return Some(StopCause::Work(maximum));
        }
        self.deadline_passed().then_some(StopCause::Deadline)
    }

    /// The limit that stops the search before the next candidate is evaluated.
    fn stop_before(&self, visited: u64, work: u64) -> Option<StopCause> {
        if let Some(maximum) = self.max_candidates.filter(|maximum| visited >= *maximum) {
            return Some(StopCause::Candidates(maximum));
        }
        self.work_stop(work)
    }
}

/// Why a bounded run stopped before it finished.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum StopCause {
    /// `synthesis.max_candidates` candidates were enumerated.
    Candidates(u64),
    /// `synthesis.max_work` evaluations were spent.
    Work(u64),
    /// The request deadline passed.
    Deadline,
    /// A counter of the request's `accounting` meter ran out.
    Accounting { limit: &'static str, value: u64 },
}

impl StopCause {
    /// FSpec FR-823's `cause` member: FR-801's `limit-reached` or the deadline.
    fn into_value(self) -> Value {
        let limit_reached = |limit: &str, value: u64, setting: String| {
            serde_json::json!({
                "kind": "limit-reached",
                "limit": limit,
                "value": value,
                "setting": setting,
            })
        };
        match self {
            Self::Candidates(value) => limit_reached(
                "max_candidates",
                value,
                "synthesis.max_candidates".to_owned(),
            ),
            Self::Work(value) => limit_reached("max_work", value, "synthesis.max_work".to_owned()),
            Self::Deadline => serde_json::json!({"kind": "deadline"}),
            Self::Accounting { limit, value } => {
                limit_reached(limit, value, format!("accounting.{limit}"))
            }
        }
    }
}

/// What stopped one evaluation.
pub(super) enum Halt {
    /// A limit stopped the run: it reports `incomplete` with this cause.
    Stop(StopCause),
    /// The request cannot be run: it is refused.
    Refuse(super::RunCause),
}

/// The disposition of a bounded integer search.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum SearchDisposition {
    /// The first candidate accepted by the checked satisfaction expression.
    Candidate(Integer),
    /// Every value in the finite domain was visited without a match.
    NoCandidate,
    /// A request limit stopped the search before its finite domain was done.
    Incomplete(StopCause),
}

/// One bounded integer search result, including the number of visited values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SearchResult {
    pub(super) disposition: SearchDisposition,
    pub(super) visited: u64,
    pub(super) work: u64,
}

/// The synthesis outcome's always-present `explored` object (FSpec FR-823).
#[derive(Clone, Debug, Serialize)]
struct Explored {
    visited: u64,
    work: u64,
}

/// The synthesis outcome's always-present `limits` object (FSpec FR-823).
#[derive(Clone, Debug, Serialize)]
struct OutcomeLimits {
    #[serde(rename = "synthesis.max_candidates")]
    max_candidates: u64,
    #[serde(rename = "synthesis.max_work")]
    max_work: u64,
}

/// A lineage entry identifying one candidate and both in-process engines.
#[derive(Clone, Debug, Serialize)]
struct Lineage {
    node: Value,
    index: u64,
    symmetry: String,
    synthesizer: &'static str,
    checker: &'static str,
}

/// The strict value-synthesis outcome document. Optional members serialize as
/// `null`, so each status keeps FR-823's one closed key set.
#[derive(Clone, Debug, Serialize)]
pub(super) struct SynthesisOutcomeDocument {
    format: &'static str,
    package: String,
    declaration: String,
    status: String,
    check: Option<String>,
    candidate: Option<Value>,
    objective: Option<Value>,
    optimal: Option<bool>,
    explored: Explored,
    lineage: Option<Lineage>,
    counterexample: Option<Value>,
    limits: OutcomeLimits,
    cause: Option<Value>,
}

impl SynthesisOutcomeDocument {
    /// Build the common outcome envelope, including the complete limit set.
    pub(super) fn new(
        package: impl Into<String>,
        declaration: impl Into<String>,
        status: impl Into<String>,
        visited: u64,
        work: u64,
        max_candidates: u64,
        max_work: u64,
    ) -> Self {
        Self {
            format: "filament.synthesis-outcome/v1",
            package: package.into(),
            declaration: declaration.into(),
            status: status.into(),
            check: None,
            candidate: None,
            objective: None,
            optimal: None,
            explored: Explored { visited, work },
            lineage: None,
            counterexample: None,
            limits: OutcomeLimits {
                max_candidates,
                max_work,
            },
            cause: None,
        }
    }

    /// Attach the independent check result and its typed candidate value.
    #[cfg(test)]
    pub(super) fn with_candidate(
        mut self,
        value: Value,
        check: impl Into<String>,
        optimal: Option<bool>,
    ) -> Self {
        self.candidate = Some(value);
        self.check = Some(check.into());
        self.optimal = optimal;
        self
    }

    /// Attach the expression and evaluated value for an authored objective.
    pub(super) fn with_objective(mut self, objective: Value) -> Self {
        self.objective = Some(objective);
        self
    }

    /// Attach the source node and deterministic candidate lineage.
    #[cfg(test)]
    pub(super) fn with_lineage(
        mut self,
        node: Value,
        index: u64,
        symmetry: impl Into<String>,
    ) -> Self {
        self.lineage = Some(Lineage {
            node,
            index,
            symmetry: symmetry.into(),
            synthesizer: "filament.synthesis.enumerate/v1",
            checker: "filament.native.value-validity/v1",
        });
        self
    }

    /// Attach a checker's replayable counterexample.
    pub(super) fn with_counterexample(mut self, value: Value) -> Self {
        self.counterexample = Some(value);
        self
    }

    /// Attach a limit, cancellation, or engine cause.
    pub(super) fn with_cause(mut self, value: Value) -> Self {
        self.cause = Some(value);
        self
    }

    /// Serialize the closed document to its JSON value.
    pub(super) fn into_value(self) -> Value {
        serde_json::to_value(self).expect("the synthesis outcome is serializable")
    }
}

/// FSpec FR-194's objective direction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Direction {
    Minimize,
    Maximize,
}

impl Direction {
    pub(super) fn parse(name: &str) -> Option<Self> {
        match name {
            "minimize" => Some(Self::Minimize),
            "maximize" => Some(Self::Maximize),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Minimize => "minimize",
            Self::Maximize => "maximize",
        }
    }
}

/// The authored objective of a value synthesis.
pub(super) struct ObjectiveSpec<'a> {
    pub(super) direction: Direction,
    /// The objective expression's source spelling (`objective.text`).
    pub(super) text: &'a str,
    /// Whether the objective is the candidate itself, so the candidate's own
    /// value order is the cost order and no objective is evaluated to rank.
    pub(super) is_candidate: bool,
}

/// The two evaluations of a checked value synthesis on one candidate.
pub(super) trait CandidateEvaluator {
    /// The `satisfies` block's value.
    fn block(&mut self, candidate: &Integer) -> Result<filament_exact::Value, Halt>;
    /// The objective expression's value.
    fn objective(&mut self, candidate: &Integer) -> Result<filament_exact::Value, Halt>;
}

/// The domain values in ascending order.
fn ascending(domain: &IntegerInterval) -> Box<dyn Iterator<Item = Integer> + '_> {
    let upper = domain.upper().clone();
    Box::new(
        std::iter::successors(Some(domain.lower().clone()), move |value| {
            Some(value.add(&Integer::one()))
        })
        .take_while(move |value| *value <= upper),
    )
}

/// The domain values in descending order.
fn descending(domain: &IntegerInterval) -> Box<dyn Iterator<Item = Integer> + '_> {
    let lower = domain.lower().clone();
    Box::new(
        std::iter::successors(Some(domain.upper().clone()), move |value| {
            Some(value.sub(&Integer::one()))
        })
        .take_while(move |value| *value >= lower),
    )
}

/// Visit `order` and evaluate each value once, stopping at the first value the
/// caller's native checked-expression path accepts. `spent` is the work already
/// consumed before the first candidate. Limits stop before the next candidate
/// is evaluated.
pub(super) fn search_integer_domain(
    order: impl IntoIterator<Item = Integer>,
    limits: SearchLimits,
    spent: u64,
    mut satisfies: impl FnMut(&Integer) -> Result<bool, Halt>,
) -> Result<SearchResult, super::RunCause> {
    let mut visited = 0_u64;
    let result = |disposition, visited: u64| -> Result<SearchResult, super::RunCause> {
        Ok(SearchResult {
            disposition,
            visited,
            work: spent.saturating_add(visited),
        })
    };
    for candidate in order {
        if let Some(cause) = limits.stop_before(visited, spent.saturating_add(visited)) {
            return result(SearchDisposition::Incomplete(cause), visited);
        }
        visited = visited.saturating_add(1);
        let accepted = match satisfies(&candidate) {
            Ok(accepted) => accepted,
            Err(Halt::Stop(cause)) => return result(SearchDisposition::Incomplete(cause), visited),
            Err(Halt::Refuse(refusal)) => return Err(refusal),
        };
        if limits.deadline_passed() {
            return result(SearchDisposition::Incomplete(StopCause::Deadline), visited);
        }
        if accepted {
            return result(SearchDisposition::Candidate(candidate), visited);
        }
    }
    result(SearchDisposition::NoCandidate, visited)
}

/// A domain value with the objective value that ranks it.
struct Ranked {
    candidate: Integer,
    key: Integer,
}

/// The domain in FSpec FR-194's cost order, or the limit that stopped the
/// ranking after `work` objective evaluations.
enum Ranking {
    Ordered(Vec<Ranked>),
    Stopped { cause: StopCause, work: u64 },
}

/// Evaluate the objective on every domain value and order the values by it:
/// nondecreasing for `minimize`, nonincreasing for `maximize`, ties by the
/// canonical value order. Each evaluation is one unit of work and one
/// enumerated candidate.
fn rank_by_objective(
    domain: &IntegerInterval,
    direction: Direction,
    limits: SearchLimits,
    evaluator: &mut impl CandidateEvaluator,
) -> Result<Ranking, super::RunCause> {
    let mut ranked = Vec::new();
    for candidate in ascending(domain) {
        let done = u64::try_from(ranked.len()).unwrap_or(u64::MAX);
        if let Some(cause) = limits.stop_before(done, done) {
            return Ok(Ranking::Stopped { cause, work: done });
        }
        match evaluator.objective(&candidate) {
            Ok(filament_exact::Value::Integer(key)) => ranked.push(Ranked { candidate, key }),
            Ok(other) => {
                return Err(not_routed(
                    "/request/synthesis/objective",
                    format!(
                        "ranking candidates by the objective value {other:?} is not implemented yet"
                    ),
                ));
            }
            Err(Halt::Stop(cause)) => {
                return Ok(Ranking::Stopped {
                    cause,
                    work: done.saturating_add(1),
                });
            }
            Err(Halt::Refuse(refusal)) => return Err(refusal),
        }
    }
    ranked.sort_by(|left, right| match direction {
        Direction::Minimize => left.key.cmp(&right.key),
        Direction::Maximize => right.key.cmp(&left.key),
    });
    Ok(Ranking::Ordered(ranked))
}

/// One value synthesis to run, whichever route read it.
pub(super) struct ValueRun<'a> {
    pub(super) package: &'a str,
    pub(super) declaration: &'a str,
    pub(super) node: Value,
    pub(super) domain: &'a IntegerInterval,
    pub(super) limits: SearchLimits,
    pub(super) objective: Option<ObjectiveSpec<'a>>,
}

/// An `incomplete` best candidate whose independent check did not run.
fn unchecked(
    mut document: SynthesisOutcomeDocument,
    cause: StopCause,
) -> (SynthesisOutcomeDocument, u8) {
    document.status = "incomplete".to_owned();
    document.check = Some("unchecked".to_owned());
    document.optimal = Some(false);
    document = document.with_cause(cause.into_value());
    (document, 22)
}

/// Run the bounded search in cost order through `searcher`, check the reported
/// candidate through `checker`, and finish FSpec FR-823's outcome document and
/// exit code. Both the source and the package route end here, so one request
/// has one outcome whichever route read it. On the source route `checker` is
/// the emitted package's strict read; on the package route there is no other
/// route, so it is a second strict read of the node with its own meter, a
/// re-read and re-evaluation check. A candidate is `validated` only when the
/// two evaluations agree on it.
pub(super) fn run_value_synthesis(
    run: ValueRun<'_>,
    searcher: &mut impl CandidateEvaluator,
    checker: &mut impl CandidateEvaluator,
) -> super::Result<(SynthesisOutcomeDocument, u8)> {
    let limits = run.limits;
    let mut document = SynthesisOutcomeDocument::new(
        run.package,
        run.declaration,
        "",
        0,
        0,
        limits.max_candidates.unwrap_or(DEFAULT_MAX_CANDIDATES),
        limits.max_work.unwrap_or(DEFAULT_MAX_WORK),
    );
    let ranked = match &run.objective {
        Some(objective) if !objective.is_candidate => {
            match rank_by_objective(run.domain, objective.direction, limits, searcher)? {
                Ranking::Ordered(ranked) => ranked,
                Ranking::Stopped { cause, work } => {
                    document.explored.work = work;
                    document.status = "incomplete".to_owned();
                    document = document.with_cause(cause.into_value());
                    return Ok((document, 22));
                }
            }
        }
        _ => Vec::new(),
    };
    let spent = u64::try_from(ranked.len()).unwrap_or(u64::MAX);
    let order: Box<dyn Iterator<Item = Integer> + '_> = match &run.objective {
        Some(objective) if !objective.is_candidate => {
            Box::new(ranked.iter().map(|entry| entry.candidate.clone()))
        }
        Some(ObjectiveSpec {
            direction: Direction::Maximize,
            ..
        }) => descending(run.domain),
        _ => ascending(run.domain),
    };
    let result = search_integer_domain(order, limits, spent, |candidate| {
        match searcher.block(candidate)? {
            filament_exact::Value::Boolean(holds) => Ok(holds),
            other => Err(Halt::Refuse(refused(
                "/request/synthesis",
                format!("satisfies expression returned {other:?}"),
            ))),
        }
    })?;
    document.explored.visited = result.visited;
    document.explored.work = result.work;
    let value = match result.disposition {
        SearchDisposition::NoCandidate => {
            document.status = "no-candidate".to_owned();
            return Ok((document, 10));
        }
        SearchDisposition::Incomplete(cause) => {
            document.status = "incomplete".to_owned();
            document = document.with_cause(cause.into_value());
            return Ok((document, 22));
        }
        SearchDisposition::Candidate(value) => value,
    };
    document.status = "candidate".to_owned();
    document.candidate = Some(integer_value(&value));
    document.lineage = Some(Lineage {
        node: run.node,
        index: result.visited.saturating_sub(1),
        symmetry: "identity".to_owned(),
        synthesizer: "filament.synthesis.enumerate/v1",
        checker: "filament.native.value-validity/v1",
    });
    let searched_objective = run.objective.as_ref().map(|_| {
        ranked
            .iter()
            .find(|entry| entry.candidate == value)
            .map_or_else(|| value.clone(), |entry| entry.key.clone())
    });
    if let (Some(objective), Some(key)) = (&run.objective, &searched_objective) {
        document = document.with_objective(serde_json::json!({
            "direction": objective.direction.as_str(),
            "expression": objective.text,
            "value": integer_value(key),
        }));
    }
    if limits.deadline_passed() {
        return Ok(unchecked(document, StopCause::Deadline));
    }
    let verdict = match check_candidate(checker, &value, searched_objective.as_ref()) {
        Ok(verdict) => verdict,
        Err(cause) => return Ok(unchecked(document, cause)),
    };
    if limits.deadline_passed() {
        return Ok(unchecked(document, StopCause::Deadline));
    }
    match verdict {
        Verdict::Agrees => {
            document.check = Some("validated".to_owned());
            if run.objective.is_some() {
                document.optimal = Some(true);
            }
            Ok((document, 0))
        }
        Verdict::Disagrees(record) => {
            document.check = Some("rejected".to_owned());
            document = document.with_counterexample(record);
            Ok((document, 30))
        }
    }
}

/// What the independent check found.
enum Verdict {
    /// The checker's evaluation agrees with the search's on every value.
    Agrees,
    /// The two evaluations differ; the record holds both values.
    Disagrees(Value),
}

/// Evaluate the `satisfies` block, and the objective when the search ranked by
/// one, on `candidate` through `checker` and compare with what the search
/// found. A limit stopping the checker is the only `Err`: an undefined or
/// failing checker evaluation is a disagreement with the search's defined one.
fn check_candidate(
    checker: &mut impl CandidateEvaluator,
    candidate: &Integer,
    searched_objective: Option<&Integer>,
) -> Result<Verdict, StopCause> {
    let defined = |evaluated: Result<filament_exact::Value, Halt>| match evaluated {
        Ok(value) => Ok(Ok(value)),
        Err(Halt::Stop(cause)) => Err(cause),
        Err(Halt::Refuse(refusal)) => Ok(Err(refusal.to_string())),
    };
    let block = defined(checker.block(candidate))?;
    let objective = searched_objective
        .map(|_| defined(checker.objective(candidate)))
        .transpose()?;
    let holds = matches!(block, Ok(filament_exact::Value::Boolean(true)));
    let same_objective = match (searched_objective, &objective) {
        (None, _) => true,
        (Some(searched), Some(Ok(filament_exact::Value::Integer(checked)))) => searched == checked,
        (Some(_), _) => false,
    };
    if holds && same_objective {
        return Ok(Verdict::Agrees);
    }
    let seen = |evaluated: &Result<filament_exact::Value, String>| match evaluated {
        Ok(filament_exact::Value::Integer(value)) => integer_value(value),
        Ok(filament_exact::Value::Boolean(holds)) => serde_json::json!(holds),
        Ok(other) => serde_json::json!({"value": format!("{other:?}")}),
        Err(fault) => serde_json::json!({"undefined": fault}),
    };
    Ok(Verdict::Disagrees(serde_json::json!({
        "kind": "checker-disagrees",
        "search": {
            "satisfies": true,
            "objective": searched_objective.map(integer_value),
        },
        "checker": {
            "satisfies": seen(&block),
            "objective": objective.as_ref().map(seen),
        },
    })))
}

fn integer_value(value: &Integer) -> Value {
    serde_json::json!({"type": "integer", "value": value.to_string()})
}

/// One evaluation's value, or the limit or fault that stopped it.
fn completed(
    evaluation: Result<filament_eval::FamilyOutcome<filament_exact::Value>, String>,
) -> Result<filament_exact::Value, Halt> {
    match evaluation {
        Err(fault) => Err(Halt::Refuse(refused("/request/synthesis", fault))),
        Ok(filament_eval::FamilyOutcome::Evaluated(filament_exact::Outcome::Completed(value))) => {
            Ok(value)
        }
        Ok(filament_eval::FamilyOutcome::Evaluated(filament_exact::Outcome::Incomplete(
            incomplete,
        ))) => Err(Halt::Stop(StopCause::Accounting {
            limit: incomplete.limit_kind.as_str(),
            value: incomplete.limit,
        })),
        Ok(outcome) => Err(Halt::Refuse(refused(
            "/request/synthesis",
            format!("checked synthesis expression did not complete: {outcome:?}"),
        ))),
    }
}

struct Selection {
    limits: SearchLimits,
    accounting: super::wire::Accounting,
}

/// The `synthesis` member as the caller sent it. Only the declaration is
/// read before the package is checked: the selected declaration decides
/// whether the rest is a value synthesis's members or a plan request's
/// (FSpec FR-805).
struct Requested {
    declaration: Option<String>,
    member: Value,
    started: Instant,
}

impl Requested {
    fn read(member: Value, started: Instant) -> super::Result<Self> {
        let Some(object) = member.as_object() else {
            return Err(wrong_kind("/request/synthesis", "an object"));
        };
        let declaration = read_declaration(object)?;
        let requested = Self {
            declaration,
            member,
            started,
        };
        // A member without `initial` is a value synthesis's: its refusals
        // come before the package is read. A plan request always names its
        // initial snapshot.
        if !requested.has_initial() {
            requested.value()?;
        }
        Ok(requested)
    }

    /// Whether the member names an initial snapshot, a plan request's file.
    fn has_initial(&self) -> bool {
        self.member.get("initial").is_some()
    }

    /// The members of a value synthesis, with its deadline counted from the
    /// start of the run.
    fn value(&self) -> super::Result<Selection> {
        let (mut selection, deadline_ms) = read_selection(self.member.clone())?;
        selection.limits.deadline = deadline_ms.and_then(|milliseconds| {
            self.started
                .checked_add(Duration::from_millis(milliseconds))
        });
        Ok(selection)
    }
}

/// Run the native-run/1 synthesis selection when present.
pub(super) fn run_bytes(
    directory: &Path,
    bytes: &[u8],
    run_limits: super::RunLimits,
    _overrides: &super::RequestOverrides<'_>,
) -> super::Result<Option<super::RunResult>> {
    let started = Instant::now();
    let mut document: Value = serde_json::from_slice(bytes).map_err(super::RunCause::Json)?;
    if document.get("format").and_then(Value::as_str)
        != Some(fsl_foundation::wire_format::WireFormat::RunRequest.as_str())
    {
        return Ok(None);
    }
    let Some(body) = document.get_mut("request").and_then(Value::as_object_mut) else {
        return Ok(None);
    };
    let Some(member) = body.remove("synthesis") else {
        return Ok(None);
    };
    let requested = Requested::read(member, started)?;
    let request_bytes = serde_json::to_vec(&document).map_err(super::RunCause::Output)?;
    let request: super::wire::Request = super::request(&request_bytes)?;
    if request.package.is_some() {
        return run_package_synthesis(directory, &request, &requested, run_limits.dependent_bytes)
            .map(Some);
    }
    exclusive_member(&request)?;
    if request.integer_division.is_some()
        || request.libraries.iter().any(|library| {
            library.integer_division.is_some() || library.refinement_internals.is_some()
        })
    {
        return Err(refused(
            "/request/synthesis",
            "this synthesis runner does not admit caller-selected arithmetic laws or refinement internals",
        ));
    }
    if !request.extensions.is_empty() {
        return Err(refused(
            "/request/extensions",
            "synthesis extension inputs are not supported",
        ));
    }
    #[cfg(feature = "quire-extraction")]
    if request
        .program
        .as_ref()
        .is_some_and(|program| program.extraction.is_some())
    {
        return Err(refused(
            "/request/program/extraction",
            "synthesis requires a source program",
        ));
    }
    let program = super::required_program(&request)?;
    let counts = super::FileCounts {
        programs: request.libraries.len().saturating_add(1),
        models: request.models.len(),
        observations: usize::from(requested.has_initial()),
        extensions: 0,
    };
    let mut intake = super::intake_with_bytes(directory, counts, run_limits.dependent_bytes)?;
    let source = intake.source(&program.source, "/request/program/source")?;
    super::require_native_header(source.source())?;
    let model_sources = request
        .models
        .iter()
        .map(|model| intake.source(&model.source, "/request/models"))
        .collect::<super::Result<Vec<_>>>()?;
    let library_sources = request
        .libraries
        .iter()
        .map(|library| intake.source(&library.source, "/request/libraries"))
        .collect::<super::Result<Vec<_>>>()?;
    let models = model_sources
        .iter()
        .map(|model| model.source().text().as_bytes().to_vec())
        .collect();
    let libraries = request
        .libraries
        .iter()
        .zip(&library_sources)
        .map(|(library, source)| {
            crate::emit_checked_v2::supplied_library(
                library.identity.clone(),
                &directory.join(&library.source.file),
                source.source().text().as_bytes().to_vec(),
            )
        })
        .collect();
    let package_name = super::package_name(source.source().identity());
    let compiled = crate::emit_checked_v2::compile_inputs(crate::emit_checked_v2::Inputs {
        source_path: directory.join(&program.source.file),
        source_identity: Some(source.source().identity().clone()),
        source: source.source().text().as_bytes().to_vec(),
        models,
        libraries,
        package_name: package_name.clone(),
        points: Default::default(),
        temporal_selections: Vec::new(),
    })
    .map_err(|error| match error {
        crate::emit_checked_v2::Error::Producer(producer) => match *producer {
            fsl_replay::ReplayRefusal::Recompile { refusal, .. } => {
                super::spine_run_failure(&source, library_sources.iter(), *refusal)
            }
            other => refused("/request/synthesis", other.to_string()),
        },
        other => refused("/request/synthesis", other.to_string()),
    })?;
    let model_documents =
        super::package_input_from_selected_models(&request.models, &model_sources)?;
    let read_limits =
        fsl_package::V2ReadLimits::bounded(fsl_replay::spine::I2_NATIVE_JOIN_STORAGE_DEFAULT);
    let admitted = fsl_package::admit_checked_package(
        compiled.emitted.package().bytes(),
        &model_documents,
        read_limits,
    )
    .map_err(|failure| {
        refused(
            "/request/synthesis",
            format!("checked package admission failed: {failure:?}"),
        )
    })?;
    let claim = select_claim(&admitted, requested.declaration.as_deref())?;
    let graph = compiled.checked.package().graph();
    let checked = graph
        .syntheses()
        .iter()
        .find(|synthesis| {
            synthesis.name() == claim.name.as_ref()
                && u64::try_from(synthesis.span().start).ok()
                    == Some(claim.declaration_region.start)
                && u64::try_from(synthesis.span().end).ok() == Some(claim.declaration_region.end)
        })
        .ok_or_else(|| missing_name("the declaration is not a checked synthesis"))?;
    if matches!(checked.grammar(), ValueType::Reference(_)) {
        let (exit_code, document) = super::plan_synthesis::run(
            super::plan_synthesis::PlanRun {
                package_name: &package_name,
                declaration: claim.name.as_ref(),
                package: compiled.checked.package(),
                checked,
                member: &requested.member,
                packages: &model_documents,
                intake: &mut intake,
            },
            super::call_accounting(&super::wire::Accounting::default()),
            requested.started,
        )?;
        return Ok(Some(super::RunResult {
            exit_code,
            value: super::output::NativeResult::synthesis_document(serde_json::json!({
                "synthesis": document
            })),
            outcome: None,
            requirements: None,
            sources: crate::Sources::from_sources(intake.sources.iter().cloned()),
        }));
    }
    let selection = requested.value()?;
    let ValueType::Int(domain) = checked.domain() else {
        return Err(refused(
            "/request/synthesis/declaration",
            "the value synthesis runner currently requires a finite integer domain",
        ));
    };
    if checked.slot_names().len() != 1 {
        return Err(refused(
            "/request/synthesis/declaration",
            "the checked synthesis candidate has no single evaluation slot",
        ));
    }
    let node = serde_json::to_value(&claim.declaration).map_err(super::RunCause::Output)?;
    let objective = match checked.objective_direction() {
        None => None,
        Some(name) => Some(ObjectiveSpec {
            direction: Direction::parse(name)
                .ok_or_else(|| refused("/request/synthesis/objective", "unknown direction"))?,
            text: checked.objective_source().unwrap_or(""),
            is_candidate: checked.objective_is_candidate(),
        }),
    };
    let mut budget = read_limits.native_join_limits().budget();
    let emission = compiled.emitted.emission();
    let identities = emission
        .identity_rows(&admitted, &mut budget)
        .map_err(|failure| refused("/request/synthesis", format!("identity rows: {failure:?}")))?;
    let equality = emission
        .equality_witness(&admitted, &mut budget)
        .map_err(|failure| refused("/request/synthesis", format!("equality rows: {failure:?}")))?;
    let bare = fsl_package::BarePackage::new(&admitted)
        .with_supply(
            fsl_package::BareSupply {
                identities: &identities,
                equality: Some(&equality),
            },
            &mut budget,
        )
        .map_err(|failure| {
            refused(
                "/request/synthesis",
                format!("package admission: {failure:?}"),
            )
        })?;
    let (check_world, check_projected) = read_value_synthesis(&bare, &claim.declaration)?;
    let mut searcher = SourceEvaluator {
        package: compiled.checked.package(),
        checked,
        meter: filament_exact::Meter::new(super::call_accounting(&selection.accounting)),
    };
    let mut checker = PackageEvaluator {
        world: &check_world,
        projected: &check_projected,
        meter: filament_exact::Meter::new(super::call_accounting(&selection.accounting)),
    };
    let (outcome, exit_code) = run_value_synthesis(
        ValueRun {
            package: &package_name,
            declaration: claim.name.as_ref(),
            node,
            domain,
            limits: selection.limits,
            objective,
        },
        &mut searcher,
        &mut checker,
    )?;
    Ok(Some(super::RunResult {
        exit_code,
        value: super::output::NativeResult::synthesis_document(serde_json::json!({
            "synthesis": outcome.into_value()
        })),
        outcome: None,
        requirements: None,
        sources: crate::Sources::from_sources(intake.sources.iter().cloned()),
    }))
}

/// The source route's evaluations over the checked package.
struct SourceEvaluator<'a> {
    package: &'a fsl_package::CheckedPackage,
    checked: &'a fsl_semantics::check::CheckedSynthesis,
    meter: filament_exact::Meter,
}

impl SourceEvaluator<'_> {
    fn evaluate(
        &mut self,
        body: &fsl_semantics::check::CheckedBody,
        candidate: &Integer,
    ) -> Result<filament_exact::Value, Halt> {
        completed(
            fsl_eval::value::evaluate_protocol_slot(
                self.package,
                body,
                vec![Some(filament_exact::Value::Integer(candidate.clone()))],
                &fsl_semantics::model::object_environment::ObjectEnvironment::default(),
                &mut self.meter,
            )
            .map(|evaluation| evaluation.outcome)
            .map_err(|fault| format!("checked synthesis expression fault: {fault:?}")),
        )
    }
}

impl CandidateEvaluator for SourceEvaluator<'_> {
    fn block(&mut self, candidate: &Integer) -> Result<filament_exact::Value, Halt> {
        let body = self.checked.satisfies();
        self.evaluate(body, candidate)
    }

    fn objective(&mut self, candidate: &Integer) -> Result<filament_exact::Value, Halt> {
        let body = self
            .checked
            .objective()
            .ok_or_else(|| Halt::Refuse(refused("/request/synthesis/objective", "no objective")))?;
        self.evaluate(body, candidate)
    }
}

fn run_package_synthesis(
    directory: &Path,
    request: &super::wire::Request,
    requested: &Requested,
    dependent_bytes: usize,
) -> super::Result<super::RunResult> {
    exclusive_member(request)?;
    if !request.extensions.is_empty() {
        return Err(refused(
            "/request/extensions",
            "synthesis extension inputs are not supported",
        ));
    }
    let package_selection = request
        .package
        .as_ref()
        .ok_or_else(|| refused("/request/package", "expected a package selection"))?;
    super::package_run::validate_request(request)?;
    let admitted =
        super::package_run::admit(directory, request, package_selection, dependent_bytes, None)?;
    let mut budget = admitted.budget;
    let identities = admitted.identities.unwrap_or_else(|| {
        fsl_package::ReadIdentityRows::of_rows(
            &admitted.package,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
    });
    let bare = fsl_package::BarePackage::new(&admitted.package)
        .with_supply(
            fsl_package::BareSupply {
                identities: &identities,
                equality: admitted.equality.as_ref(),
            },
            &mut budget,
        )
        .map_err(|failure| {
            refused(
                "/request/synthesis",
                format!("package admission: {failure:?}"),
            )
        })?;
    let claim = select_claim(&admitted.package, requested.declaration.as_deref())?;
    if fsl_package::is_plan_synthesis(&admitted.package, &claim.declaration) {
        return Err(plan_not_routed());
    }
    let selection = requested.value()?;
    let (world, projected) = read_value_synthesis(&bare, &claim.declaration)?;
    let (check_world, check_projected) = read_value_synthesis(&bare, &claim.declaration)?;
    let package_name = admitted.package.name().to_owned();
    let node = serde_json::to_value(&claim.declaration).map_err(super::RunCause::Output)?;
    let objective = match projected.objective() {
        None => None,
        Some(objective) => Some(ObjectiveSpec {
            direction: Direction::parse(objective.direction())
                .ok_or_else(|| refused("/request/synthesis/objective", "unknown direction"))?,
            text: objective.text(),
            is_candidate: objective.is_candidate(),
        }),
    };
    let mut searcher = PackageEvaluator {
        world: &world,
        projected: &projected,
        meter: filament_exact::Meter::new(super::call_accounting(&selection.accounting)),
    };
    let mut checker = PackageEvaluator {
        world: &check_world,
        projected: &check_projected,
        meter: filament_exact::Meter::new(super::call_accounting(&selection.accounting)),
    };
    let (outcome, exit_code) = run_value_synthesis(
        ValueRun {
            package: &package_name,
            declaration: claim.name.as_ref(),
            node,
            domain: projected.domain(),
            limits: selection.limits,
            objective,
        },
        &mut searcher,
        &mut checker,
    )?;
    Ok(super::RunResult {
        exit_code,
        value: super::output::NativeResult::synthesis_document(serde_json::json!({
            "synthesis": outcome.into_value()
        })),
        outcome: None,
        requirements: None,
        sources: crate::Sources::from_sources(admitted.models),
    })
}

/// A strict read of one value synthesis, ready to evaluate.
fn read_value_synthesis(
    bare: &fsl_package::BarePackage<'_>,
    declaration: &filament_contract_model::CheckedNodeId,
) -> super::Result<(fsl_package::BareWorld, fsl_package::BareSynthesis)> {
    let projected = bare
        .project_value_synthesis(declaration)
        .map_err(|failure| {
            refused(
                "/request/synthesis",
                format!("checked synthesis read: {failure:?}"),
            )
        })?;
    let world = bare
        .world()
        .map_err(|failure| refused("/request/synthesis", format!("package world: {failure:?}")))?;
    Ok((world, projected))
}

/// The package route's evaluations over the admitted package's world.
struct PackageEvaluator<'a> {
    world: &'a fsl_package::BareWorld,
    projected: &'a fsl_package::BareSynthesis,
    meter: filament_exact::Meter,
}

impl PackageEvaluator<'_> {
    fn evaluate(
        &mut self,
        expression: &fsl_package::ReaderProtocolProjection,
        candidate: &Integer,
    ) -> Result<filament_exact::Value, Halt> {
        let slot = self.projected.candidate_slot();
        let mut frame = vec![None; expression.frame_slots().max(slot.saturating_add(1))];
        frame[slot] = Some(filament_exact::Value::Integer(candidate.clone()));
        completed(
            fsl_eval::bare::evaluate_bare_expression(
                self.world,
                expression,
                frame,
                &mut self.meter,
            )
            .map(|evaluation| evaluation.outcome)
            .map_err(|fault| format!("checked synthesis expression fault: {fault:?}")),
        )
    }
}

impl CandidateEvaluator for PackageEvaluator<'_> {
    fn block(&mut self, candidate: &Integer) -> Result<filament_exact::Value, Halt> {
        let projected = self.projected;
        self.evaluate(projected.satisfies(), candidate)
    }

    fn objective(&mut self, candidate: &Integer) -> Result<filament_exact::Value, Halt> {
        let projected = self.projected;
        let objective = projected
            .objective()
            .ok_or_else(|| Halt::Refuse(refused("/request/synthesis/objective", "no objective")))?;
        self.evaluate(objective.expression(), candidate)
    }
}

fn read_declaration(object: &serde_json::Map<String, Value>) -> super::Result<Option<String>> {
    match object.get("declaration") {
        None => Ok(None),
        Some(Value::String(name)) if !name.is_empty() => Ok(Some(name.clone())),
        Some(Value::String(_)) => Err(invalid_value(
            "/request/synthesis/declaration",
            "expected a non-empty name",
        )),
        Some(_) => Err(wrong_kind("/request/synthesis/declaration", "a name")),
    }
}

/// The members of a value synthesis (FSpec FR-823) and its deadline.
fn read_selection(member: Value) -> super::Result<(Selection, Option<u64>)> {
    let Some(object) = member.as_object() else {
        return Err(wrong_kind("/request/synthesis", "an object"));
    };
    if let Some(key) = object
        .keys()
        .find(|key| !["declaration", "limits", "accounting", "deadline_ms"].contains(&key.as_str()))
    {
        return Err(unknown_member(&format!("/request/synthesis/{key}")));
    }
    read_declaration(object)?;
    let mut limits = SearchLimits::default();
    if let Some(value) = object.get("limits") {
        let Some(object) = value.as_object() else {
            return Err(wrong_kind("/request/synthesis/limits", "an object"));
        };
        if let Some(key) = object
            .keys()
            .find(|key| !["max_candidates", "max_work"].contains(&key.as_str()))
        {
            return Err(unknown_member(&format!("/request/synthesis/limits/{key}")));
        }
        limits.max_candidates = Some(read_limit(
            object,
            "max_candidates",
            DEFAULT_MAX_CANDIDATES,
        )?);
        limits.max_work = Some(read_limit(object, "max_work", DEFAULT_MAX_WORK)?);
    }
    let accounting = object
        .get("accounting")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|error| {
            invalid_value(
                "/request/synthesis/accounting",
                format!("invalid accounting object: {error}"),
            )
        })?
        .unwrap_or_default();
    let deadline_ms = match object.get("deadline_ms") {
        None => None,
        Some(value) => Some(
            value
                .as_u64()
                .filter(|value| *value <= 9_007_199_254_740_991)
                .ok_or_else(|| {
                    invalid_value(
                        "/request/synthesis/deadline_ms",
                        "expected an integer from 0 to 9007199254740991",
                    )
                })?,
        ),
    };
    Ok((Selection { limits, accounting }, deadline_ms))
}

fn read_limit(
    object: &serde_json::Map<String, Value>,
    name: &str,
    default: u64,
) -> super::Result<u64> {
    let pointer = format!("/request/synthesis/limits/{name}");
    match object.get(name) {
        None => Ok(default),
        Some(value) => match value.as_u64() {
            Some(0) => Err(invalid_value(&pointer, "expected a positive integer")),
            Some(limit) => Ok(limit),
            None => Err(wrong_kind(&pointer, "a positive integer")),
        },
    }
}

fn select_claim<'p>(
    package: &'p filament_contract_model::CheckedPackageV2,
    requested: Option<&str>,
) -> super::Result<&'p filament_contract_model::ClaimNameRow> {
    let rows = package.claim_names().unwrap_or_default();
    let graph = package.resolved_graph();
    let declarations: std::collections::BTreeSet<_> = graph
        .nodes
        .iter()
        .filter(|node| {
            node.node_tag.as_ref() == "claim" && node.semantic_form.as_ref() == "synthesis_request"
        })
        .map(|node| &node.node_id)
        .collect();
    select_claim_rows(
        rows.iter()
            .filter(|row| declarations.contains(&row.declaration)),
        requested,
    )
}

fn select_claim_rows<'p>(
    rows: impl Iterator<Item = &'p filament_contract_model::ClaimNameRow>,
    requested: Option<&str>,
) -> super::Result<&'p filament_contract_model::ClaimNameRow> {
    let rows: Vec<_> = rows.collect();
    let unique_declarations = |matching: &[&'p filament_contract_model::ClaimNameRow]| {
        let mut declarations = Vec::new();
        for row in matching {
            if !declarations
                .iter()
                .any(|known: &&filament_contract_model::ClaimNameRow| {
                    known.declaration == row.declaration
                })
            {
                declarations.push(*row);
            }
        }
        declarations
    };
    let selected = match requested {
        Some(name) => {
            let path: Vec<_> = name.split("::").collect();
            if path.iter().any(|segment| segment.is_empty()) {
                return Err(missing_name("the declaration name has an empty segment"));
            }
            let matching: Vec<_> = rows
                .iter()
                .copied()
                .filter(|row| {
                    if path.len() == 1 {
                        row.name.as_ref() == name
                    } else {
                        row.qualified_name
                            .iter()
                            .map(|segment| segment.as_ref())
                            .eq(path.iter().copied())
                    }
                })
                .collect();
            match unique_declarations(&matching).as_slice() {
                [row] => *row,
                [] => return Err(missing_name("no synthesis declaration has this name")),
                _ => {
                    return Err(request_refusal(
                        Code::AmbiguousDeclaration,
                        "ambiguous-name",
                        "/request/synthesis/declaration",
                        "the name matches more than one synthesis declaration",
                    ));
                }
            }
        }
        None => match unique_declarations(&rows).as_slice() {
            [row] => *row,
            [] => return Err(missing_name("the package holds no synthesis declaration")),
            _ => {
                return Err(request_refusal(
                    Code::InvalidRuntimeInput,
                    "missing-member",
                    "/request/synthesis/declaration",
                    "the package holds several synthesis declarations; name one",
                ));
            }
        },
    };
    Ok(selected)
}

/// A refusal of the synthesis run that has no catalogued cause of its own.
fn refused(member: &str, reason: impl Into<String>) -> super::RunCause {
    super::RunCause::ClauseRequest {
        member: "synthesis".to_owned(),
        reason: format!("{}: {}", member, reason.into()),
    }
}

/// A `synthesis` member refusal with its catalog code and FR-805 cause.
fn request_refusal(
    code: Code,
    cause: &'static str,
    pointer: &str,
    reason: impl Into<String>,
) -> super::RunCause {
    super::RunCause::SynthesisRequest {
        path: pointer.to_owned(),
        code,
        cause,
        reason: reason.into(),
    }
}

fn unknown_member(pointer: &str) -> super::RunCause {
    request_refusal(
        Code::InvalidRuntimeInput,
        "unknown-member",
        pointer,
        "the member is not admitted here",
    )
}

fn wrong_kind(pointer: &str, expected: &str) -> super::RunCause {
    request_refusal(
        Code::InvalidRuntimeInput,
        "wrong-value-kind",
        pointer,
        format!("expected {expected}"),
    )
}

fn invalid_value(pointer: &str, reason: impl Into<String>) -> super::RunCause {
    request_refusal(Code::InvalidRuntimeInput, "invalid-value", pointer, reason)
}

fn missing_name(reason: &str) -> super::RunCause {
    request_refusal(
        Code::MissingDeclaration,
        "missing-name",
        "/request/synthesis/declaration",
        reason,
    )
}

/// A plan synthesis selected through a package: the planner judges the
/// checked source's bodies, which a package does not carry, and a plan never
/// runs through the value enumerator.
fn plan_not_routed() -> super::RunCause {
    not_routed(
        "/request/synthesis/declaration",
        "a plan synthesis needs the checked source; select it with a program, not a package",
    )
}

fn not_routed(pointer: &str, reason: impl Into<String>) -> super::RunCause {
    request_refusal(
        Code::UnsupportedConstruct,
        "not-yet-implemented",
        pointer,
        reason,
    )
}

/// FR-1037: the `synthesis` member is the request's only run member.
fn exclusive_member(request: &super::wire::Request) -> super::Result<()> {
    if request.call.is_some()
        || request.clause.is_some()
        || request.items.is_some()
        || request.tl_lowering.is_some()
        || request.verification_plan.is_some()
    {
        return Err(refused(
            "/request/synthesis",
            "is mutually exclusive with call, clause, items, tl_lowering and verification_plan",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        ascending, run_value_synthesis, search_integer_domain, select_claim_rows,
        CandidateEvaluator, Direction, Halt, ObjectiveSpec, SearchDisposition, SearchLimits,
        SearchResult, StopCause, SynthesisOutcomeDocument, ValueRun,
    };
    use filament_exact::{Integer, IntegerInterval};
    use ix_trace_rs::trace;
    use serde_json::Value;

    fn digit_domain() -> IntegerInterval {
        IntegerInterval::new(Integer::from(0_i64), Integer::from(9_i64)).expect("digit range")
    }

    fn claim_row(scope: &str) -> filament_contract_model::ClaimNameRow {
        serde_json::from_value(serde_json::json!({
            "declaration": {
                "package": "pkg",
                "path": [scope, "Pick"],
                "ordinal": 0
            },
            "occurrence": {"role": "declaration", "ordinal": 0},
            "name": "Pick",
            "qualified_name": [scope, "Pick"],
            "declaration_region": {
                "source": {"authority": "test", "identity": scope},
                "start": 0,
                "end": 4
            }
        }))
        .expect("a valid claim-name row")
    }

    /// FSpec FR-823-AC-3: a bare name matching two declarations is
    /// ambiguous; a fully qualified name selects exactly one row.
    #[test]
    #[trace("TC-3017", "FR-1037-AC-2")]
    #[trace("FSpec-TC-2038", "FSpec-FR-823-AC-3")]
    fn claim_selection_refuses_ambiguous_bare_names_and_accepts_qualified_names() {
        let a = claim_row("A");
        let b = claim_row("B");
        let ambiguous = select_claim_rows([&a, &b].into_iter(), Some("Pick"))
            .expect_err("two matching declarations are ambiguous");
        assert_eq!(ambiguous.code(), fsl_foundation::Code::AmbiguousDeclaration);
        assert!(format!("{ambiguous:?}").contains("ambiguous-name"));

        let selected = select_claim_rows([&a, &b].into_iter(), Some("A::Pick"))
            .expect("qualified name selects one declaration");
        assert_eq!(
            selected
                .qualified_name
                .iter()
                .map(|segment| segment.as_ref())
                .collect::<Vec<_>>(),
            ["A", "Pick"]
        );
    }

    fn search(
        limits: SearchLimits,
        satisfies: impl FnMut(&Integer) -> Result<bool, Halt>,
    ) -> SearchResult {
        search_integer_domain(ascending(&digit_domain()), limits, 0, satisfies)
            .expect("the search is not refused")
    }

    #[test]
    #[trace("TC-2104", "FR-754-AC-3")]
    fn visits_values_through_the_first_satisfying_integer() {
        let result = search(SearchLimits::default(), |value| {
            Ok(value == &Integer::from(7_i64))
        });
        assert_eq!(
            result.disposition,
            SearchDisposition::Candidate(Integer::from(7_i64))
        );
        assert_eq!(result.visited, 8);
    }

    #[test]
    #[trace("TC-2104", "FR-754-AC-3")]
    fn exhausts_a_finite_integer_domain_before_reporting_no_candidate() {
        let result = search(SearchLimits::default(), |_| Ok(false));
        assert_eq!(result.disposition, SearchDisposition::NoCandidate);
        assert_eq!(result.visited, 10);
    }

    #[test]
    #[trace("TC-2104", "FR-754-AC-3")]
    fn request_limits_stop_before_evaluating_the_next_value() {
        let result = search(
            SearchLimits {
                max_candidates: Some(3),
                max_work: None,
                deadline: None,
            },
            |_| Ok(false),
        );
        assert_eq!(
            result.disposition,
            SearchDisposition::Incomplete(StopCause::Candidates(3))
        );
        assert_eq!(result.visited, 3);
    }

    /// FR-823: an elapsed request deadline stops before another candidate is
    /// evaluated and produces an incomplete result.
    #[test]
    #[trace("FSpec-TC-2038", "FSpec-FR-823-AC-2")]
    fn elapsed_deadline_stops_before_the_first_candidate() {
        let result = search(
            SearchLimits {
                deadline: Some(std::time::Instant::now() - std::time::Duration::from_millis(1)),
                ..SearchLimits::default()
            },
            |_| panic!("no candidate is evaluated after the deadline"),
        );
        assert_eq!(
            result.disposition,
            SearchDisposition::Incomplete(StopCause::Deadline)
        );
        assert_eq!(result.visited, 0);
        assert_eq!(result.work, 0);
    }

    /// FR-823: an evaluation that runs beyond the request deadline is counted
    /// as consumed work, but cannot produce a candidate result.
    #[test]
    #[trace("FSpec-TC-2038", "FSpec-FR-823-AC-2")]
    fn deadline_expiring_during_evaluation_reports_consumed_work() {
        let result = search(
            SearchLimits {
                deadline: Some(std::time::Instant::now() + std::time::Duration::from_millis(1)),
                ..SearchLimits::default()
            },
            |_| {
                std::thread::sleep(std::time::Duration::from_millis(10));
                Ok(true)
            },
        );
        assert_eq!(
            result.disposition,
            SearchDisposition::Incomplete(StopCause::Deadline)
        );
        assert_eq!(result.visited, 1);
        assert_eq!(result.work, 1);
    }

    /// An evaluator over small integers: the block and the objective are
    /// plain functions of the candidate.
    #[derive(Clone, Copy)]
    struct Fake {
        block: fn(i64) -> bool,
        objective: fn(i64) -> i64,
        meter_stop_at: Option<i64>,
    }

    fn number(candidate: &Integer) -> i64 {
        candidate.to_string().parse().expect("a small integer")
    }

    impl CandidateEvaluator for Fake {
        fn block(&mut self, candidate: &Integer) -> Result<filament_exact::Value, Halt> {
            if self.meter_stop_at == Some(number(candidate)) {
                return Err(Halt::Stop(StopCause::Accounting {
                    limit: "work_units",
                    value: 5,
                }));
            }
            let block = self.block;
            Ok(filament_exact::Value::Boolean(block(number(candidate))))
        }

        fn objective(&mut self, candidate: &Integer) -> Result<filament_exact::Value, Halt> {
            let objective = self.objective;
            Ok(filament_exact::Value::Integer(Integer::from(objective(
                number(candidate),
            ))))
        }
    }

    fn pick(block: fn(i64) -> bool, objective: fn(i64) -> i64, meter_stop_at: Option<i64>) -> Fake {
        Fake {
            block,
            objective,
            meter_stop_at,
        }
    }

    /// Run `Pick` with `searcher` finding and `checker` checking.
    fn run_checked(
        limits: SearchLimits,
        direction: Option<(Direction, bool)>,
        searcher: &mut Fake,
        checker: &mut Fake,
    ) -> (Value, u8) {
        let domain = digit_domain();
        let (document, exit) = run_value_synthesis(
            ValueRun {
                package: "pkg",
                declaration: "Pick",
                node: serde_json::json!({"package": "pkg", "path": ["Pick"], "ordinal": 0}),
                domain: &domain,
                limits,
                objective: direction.map(|(direction, is_candidate)| ObjectiveSpec {
                    direction,
                    text: "Pick",
                    is_candidate,
                }),
            },
            searcher,
            checker,
        )
        .expect("the run is not refused");
        (document.into_value(), exit)
    }

    /// Run `Pick` with a checker that evaluates exactly as the searcher does.
    fn run_pick(
        limits: SearchLimits,
        direction: Option<(Direction, bool)>,
        evaluator: &mut Fake,
    ) -> (Value, u8) {
        let mut checker = Fake {
            meter_stop_at: None,
            ..*evaluator
        };
        run_checked(limits, direction, evaluator, &mut checker)
    }

    fn limited(max_work: u64) -> SearchLimits {
        SearchLimits {
            max_work: Some(max_work),
            ..SearchLimits::default()
        }
    }

    /// FSpec FR-194-AC-7: `minimize` takes the first satisfying value in
    /// nondecreasing order and `maximize` the first in nonincreasing order.
    #[test]
    #[trace("TC-2104", "FR-754-AC-3")]
    #[trace("FSpec-TC-1602", "FSpec-FR-194-AC-7")]
    fn the_objective_orders_the_search() {
        let mut evaluator = pick(|c| c >= 3, |c| c, None);
        let (minimum, exit) = run_pick(
            SearchLimits::default(),
            Some((Direction::Minimize, true)),
            &mut evaluator,
        );
        assert_eq!(exit, 0);
        assert_eq!(minimum["candidate"]["value"], "3");
        assert_eq!(minimum["explored"]["visited"], 4);
        assert_eq!(minimum["explored"]["work"], 4);
        assert_eq!(minimum["optimal"], true);
        assert_eq!(minimum["objective"]["value"]["value"], "3");
        let (maximum, exit) = run_pick(
            SearchLimits::default(),
            Some((Direction::Maximize, true)),
            &mut evaluator,
        );
        assert_eq!(exit, 0);
        assert_eq!(maximum["candidate"]["value"], "9");
        assert_eq!(maximum["explored"]["visited"], 1);
        assert_eq!(maximum["lineage"]["index"], 0);
        assert_eq!(maximum["optimal"], true);
        assert_eq!(maximum["objective"]["direction"], "maximize");
        assert_eq!(maximum["objective"]["expression"], "Pick");
        assert_eq!(maximum["objective"]["value"]["value"], "9");
    }

    /// FSpec FR-194: an objective that is not the candidate ranks every value
    /// by it, ties broken by the canonical value order.
    #[test]
    #[trace("FSpec-TC-1602", "FSpec-FR-194-AC-7")]
    fn a_computed_objective_ranks_the_domain_with_ties_by_value() {
        let mut evaluator = pick(|c| c >= 7, |c| (c - 5) * (c - 5), None);
        let (minimum, exit) = run_pick(
            SearchLimits::default(),
            Some((Direction::Minimize, false)),
            &mut evaluator,
        );
        assert_eq!(exit, 0);
        // The order is 5, 4, 6, 3, 7: 7 is the fifth value and costs 4.
        assert_eq!(minimum["candidate"]["value"], "7");
        assert_eq!(minimum["explored"]["visited"], 5);
        assert_eq!(minimum["lineage"]["index"], 4);
        assert_eq!(minimum["objective"]["value"]["value"], "4");
        assert_eq!(minimum["explored"]["work"], 15);
        let (maximum, exit) = run_pick(
            SearchLimits::default(),
            Some((Direction::Maximize, false)),
            &mut evaluator,
        );
        assert_eq!(exit, 0);
        // The order is 0, 1, 9, 2, 8, ...: 9 is the third value and costs 16.
        assert_eq!(maximum["candidate"]["value"], "9");
        assert_eq!(maximum["explored"]["visited"], 3);
        assert_eq!(maximum["objective"]["value"]["value"], "16");
        assert_eq!(maximum["optimal"], true);
    }

    #[test]
    #[trace("TC-2104", "FR-754-AC-3")]
    fn outcome_carries_typed_candidate_and_limit_cause() {
        let mut evaluator = pick(|c| c == 7, |c| c, None);
        let (candidate, exit) = run_pick(SearchLimits::default(), None, &mut evaluator);
        assert_eq!(exit, 0);
        assert_eq!(candidate["status"], "candidate");
        assert_eq!(
            candidate["candidate"],
            serde_json::json!({"type": "integer", "value": "7"})
        );
        assert_eq!(candidate["check"], "validated");
        assert_eq!(candidate["explored"]["work"], 8);
        assert_eq!(candidate["lineage"]["index"], 7);
        assert_eq!(candidate["lineage"]["symmetry"], "identity");
        assert_eq!(candidate["optimal"], Value::Null);

        let (limited, exit) = run_pick(
            SearchLimits {
                max_candidates: Some(3),
                ..SearchLimits::default()
            },
            None,
            &mut pick(|_| false, |c| c, None),
        );
        assert_eq!(exit, 22);
        assert_eq!(limited["status"], "incomplete");
        assert_eq!(limited["explored"]["visited"], 3);
        assert_eq!(limited["explored"]["work"], 3);
        assert_eq!(
            limited["cause"],
            serde_json::json!({
                "kind": "limit-reached", "limit": "max_candidates", "value": 3,
                "setting": "synthesis.max_candidates"
            })
        );
    }

    /// FSpec FR-823: the independent check runs under the request's
    /// `accounting`, not under the search limits, so a work limit spent on the
    /// last candidate still leaves it `validated`.
    #[test]
    #[trace("TC-2104", "FR-754-AC-3")]
    #[trace("FSpec-TC-2038", "FSpec-FR-823-AC-5")]
    fn the_check_is_not_charged_to_the_search_work_limit() {
        let mut evaluator = pick(|c| c == 7, |c| c, None);
        let (outcome, exit) = run_pick(
            limited(8),
            Some((Direction::Minimize, true)),
            &mut evaluator,
        );
        assert_eq!(exit, 0);
        assert_eq!(outcome["status"], "candidate");
        assert_eq!(outcome["check"], "validated");
        assert_eq!(outcome["explored"]["work"], 8);
        assert_eq!(outcome["objective"]["value"]["value"], "7");
        let (stopped, exit) = run_pick(limited(7), None, &mut evaluator);
        assert_eq!(exit, 22);
        assert_eq!(stopped["status"], "incomplete");
        assert_eq!(stopped["candidate"], Value::Null);
    }

    /// FSpec FR-823: a checker that disagrees with the search rejects the
    /// candidate with both evaluations in the record, exit 30.
    #[test]
    #[trace("FSpec-TC-2038", "FSpec-FR-823-AC-5")]
    fn a_checker_that_disagrees_rejects_the_candidate() {
        let mut searcher = pick(|c| c >= 3, |c| c, None);
        let mut checker = pick(|c| c >= 4, |c| c, None);
        let (outcome, exit) =
            run_checked(SearchLimits::default(), None, &mut searcher, &mut checker);
        assert_eq!(exit, 30);
        assert_eq!(outcome["status"], "candidate");
        assert_eq!(outcome["check"], "rejected");
        assert_eq!(outcome["optimal"], Value::Null);
        assert_eq!(
            outcome["counterexample"],
            serde_json::json!({
                "kind": "checker-disagrees",
                "search": {"satisfies": true, "objective": null},
                "checker": {"satisfies": false, "objective": null},
            })
        );
        let mut checker = pick(|c| c >= 3, |c| c + 1, None);
        let (outcome, exit) = run_checked(
            SearchLimits::default(),
            Some((Direction::Minimize, true)),
            &mut searcher,
            &mut checker,
        );
        assert_eq!(exit, 30);
        assert_eq!(outcome["check"], "rejected");
        assert_eq!(
            outcome["counterexample"]["search"]["objective"]["value"],
            "3"
        );
        assert_eq!(
            outcome["counterexample"]["checker"]["objective"]["value"],
            "4"
        );
        assert_eq!(outcome["counterexample"]["checker"]["satisfies"], true);
    }

    /// FSpec FR-823: a limit stopping the checker leaves the best candidate
    /// `unchecked` and the result `incomplete`.
    #[test]
    #[trace("FSpec-TC-2038", "FSpec-FR-823-AC-2")]
    fn a_checker_stop_leaves_the_candidate_unchecked() {
        let mut searcher = pick(|c| c == 7, |c| c, None);
        let mut checker = pick(|c| c == 7, |c| c, Some(7));
        let (outcome, exit) = run_checked(
            SearchLimits::default(),
            Some((Direction::Minimize, true)),
            &mut searcher,
            &mut checker,
        );
        assert_eq!(exit, 22);
        assert_eq!(outcome["status"], "incomplete");
        assert_eq!(outcome["check"], "unchecked");
        assert_eq!(outcome["optimal"], false);
        assert_eq!(outcome["objective"]["value"]["value"], "7");
        assert_eq!(outcome["cause"]["setting"], "accounting.work_units");
    }

    /// FSpec FR-823: a meter limit reached while evaluating a candidate is an
    /// `incomplete` result naming the accounting counter, not a refusal.
    #[test]
    #[trace("FSpec-TC-2038", "FSpec-FR-823-AC-2")]
    fn an_accounting_stop_reports_incomplete_with_its_counter() {
        let (outcome, exit) = run_pick(
            SearchLimits::default(),
            None,
            &mut pick(|c| c == 7, |c| c, Some(2)),
        );
        assert_eq!(exit, 22);
        assert_eq!(outcome["status"], "incomplete");
        assert_eq!(outcome["candidate"], Value::Null);
        assert_eq!(outcome["explored"]["visited"], 3);
        assert_eq!(
            outcome["cause"],
            serde_json::json!({
                "kind": "limit-reached", "limit": "work_units", "value": 5,
                "setting": "accounting.work_units"
            })
        );
    }

    #[test]
    #[trace("TC-3017", "FR-1037-AC-1")]
    #[trace("FSpec-FR-823-AC-1")]
    fn candidate_outcome_serializes_every_fr823_member() {
        let outcome = SynthesisOutcomeDocument::new(
            "pkg",
            "Pick",
            "candidate",
            8,
            8,
            10_000_000,
            100_000_000,
        )
        .with_candidate(
            serde_json::json!({"type": "integer", "value": "7"}),
            "validated",
            None,
        )
        .with_lineage(
            serde_json::json!({"package": "pkg", "path": ["Pick"], "ordinal": 0}),
            7,
            "identity",
        )
        .into_value();
        assert_eq!(
            outcome
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            [
                "candidate",
                "cause",
                "check",
                "counterexample",
                "declaration",
                "explored",
                "format",
                "limits",
                "lineage",
                "objective",
                "optimal",
                "package",
                "status"
            ]
        );
        assert_eq!(outcome["format"], "filament.synthesis-outcome/v1");
        assert_eq!(
            outcome["explored"],
            serde_json::json!({"visited": 8, "work": 8})
        );
        assert_eq!(outcome["lineage"]["symmetry"], "identity");
        assert_eq!(outcome["lineage"]["index"], 7);
        assert_eq!(outcome["optimal"], Value::Null);
        assert_eq!(outcome["objective"], Value::Null);
        assert_eq!(outcome["counterexample"], Value::Null);
        assert_eq!(outcome["cause"], Value::Null);
    }
}
