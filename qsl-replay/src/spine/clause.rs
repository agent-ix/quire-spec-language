// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-109 (QSL-278): `qsl_replay::spine::run_clause`, the spine's clause-run
//! entry beside [`super::call::run`]'s function-run entry (ADR-011 §5).
//!
//! Compiles a `1-draft` unit through the spine ([`super::compile`]),
//! resolves one state clause or one Boolean function run as a claim by
//! name, admits the supplied observations (FR-106,
//! `qsl_semantics::model::observation`), evaluates through S6a (FR-107's
//! `evaluate_clause`, or `CheckedPackage::call` for a `Function`
//! selection) and returns one [`ClauseRunReport`]: a typed disposition.
//! Reuses FR-100's outcome mapping ([`super::call::convert_outcome`]) by
//! reference, rather than restating it.

use std::collections::BTreeMap;

use qsl_foundation::diagnostic::InternalFault;
use qsl_foundation::source::Source;
use qsl_foundation::source_map::NativeLanguage;
use qsl_foundation::{ByteDigest, SourceIdentity};
use qsl_semantics::library::PackageId;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::observation::{
    admit_current_snapshot, admit_observations, population_universe_for, AdmissionFailure,
    AdmissionRecord, ClauseFacts, ClauseSelection, ClauseSelectionInput, DocumentRef,
    ObservationLimits, OperationFacts, Provisions,
};
use quire_exact::{Meter, ScalarLimits, Value, ValueType};

use super::call::{convert_call_failure, convert_outcome, select};
pub use super::call::{CallOutcome, CallValue, RunRefusal};
use super::{compile, CompileRefusal, DependencyInput, SpineLimits};

/// FR-109's `Function` selection argument value: FR-100's canonical
/// integer, or an object reference resolved in the selection's current
/// snapshot.
#[derive(Clone, Debug)]
pub enum ClauseArgumentValue {
    /// A canonical integer (FR-100's rule): the integer itself, or `0`/`1`
    /// for a `Boolean` parameter.
    Integer(i64),
    /// `{"reference": {population, key}}`.
    Reference {
        /// The population identity.
        population: String,
        /// The object's declared key.
        key: String,
    },
}

/// One `{parameter, value}` argument of a `Function` selection.
#[derive(Clone, Debug)]
pub struct ClauseArgument {
    /// The parameter's declared name.
    pub parameter: String,
    /// The argument's value.
    pub value: ClauseArgumentValue,
}

/// FR-109's selection: a state clause (FR-106's [`ClauseSelection`]), or a
/// Boolean function run as a claim.
#[derive(Clone, Debug)]
pub enum ClauseRunSelection {
    /// A state clause, admitted per FR-106.
    Clause(ClauseSelection),
    /// A function, called as a claim (FR-109's own `Function` selection).
    Function {
        /// The function's name (FR-100's `function` rule; one segment,
        /// resolved by `super::call::select`).
        name: String,
        /// The call's arguments, one per parameter, in any order.
        arguments: Vec<ClauseArgument>,
        /// The current snapshot each object argument resolves against.
        snapshot: DocumentRef,
    },
}

/// FR-109 Inputs' unit: the program source, or an I3 extracted source.
///
/// `#[non_exhaustive]`: the `Extracted` variant exists only under the
/// `quire-extraction` feature, and Cargo unifies features across a build,
/// so a downstream match must not rely on which variants it sees.
#[derive(Debug)]
#[non_exhaustive]
pub enum ClauseRunSource {
    /// The unit's source bytes and its four FR-001 labels.
    Program {
        /// The unit's four FR-001 labels.
        identity: SourceIdentity,
        /// The source's display path; only displayed, never opened.
        path: String,
        /// The unit's source bytes.
        bytes: Vec<u8>,
    },
    /// An I3 extracted source (ADR-011 I3, `qsl-source`): the verified body
    /// `qsl_source::extract` returned. Its body, with the body's own
    /// identity and path, is the unit `compile` reads; its original
    /// document's identity and digest are reported as
    /// [`ClauseRunProvenance::extraction`]. Its declared fence language
    /// must be `ix:native`, or the run reports
    /// [`ClauseDisposition::UnknownLanguage`] at stage `compile`.
    #[cfg(feature = "quire-extraction")]
    Extracted(qsl_source::ExtractedSource),
}

/// The unit a [`ClauseRunSource`] compiles, borrowed from it, and the
/// extraction's original document when I3 was used.
struct Unit<'a> {
    identity: &'a SourceIdentity,
    path: &'a str,
    bytes: &'a [u8],
    /// The I3 extracted clause's declared fence language; `None` for a
    /// program source, whose language is its own header's.
    language: Option<&'a str>,
    extraction: Option<ExtractionOrigin>,
}

impl ClauseRunSource {
    fn unit(&self) -> Unit<'_> {
        match self {
            Self::Program {
                identity,
                path,
                bytes,
            } => Unit {
                identity,
                path,
                bytes,
                language: None,
                extraction: None,
            },
            #[cfg(feature = "quire-extraction")]
            Self::Extracted(extracted) => {
                let map = extracted.map();
                let body = map.body();
                Unit {
                    identity: body.identity(),
                    path: body.path(),
                    bytes: body.text().as_bytes(),
                    language: Some(extracted.language()),
                    extraction: Some(ExtractionOrigin {
                        identity: map.original().identity().clone(),
                        digest: map.original().digest(),
                    }),
                }
            }
        }
    }
}

/// FR-109 Outputs: the I3 extraction's original document, "the
/// extraction's original identity and digest when I3 was used".
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtractionOrigin {
    /// The original document's four FR-001 labels.
    pub identity: SourceIdentity,
    /// The original document's byte digest.
    pub digest: ByteDigest,
}

/// FR-109 Inputs: one clause-run request.
pub struct ClauseRunRequest {
    /// The unit: program source bytes, or an I3 extracted source.
    pub source: ClauseRunSource,
    /// FR-056's package input.
    pub packages: BTreeMap<[u8; 32], Vec<u8>>,
    /// FR-099's dependency input.
    pub dependencies: DependencyInput,
    /// FR-106's snapshot provision.
    pub snapshots: BTreeMap<[u8; 32], Vec<u8>>,
    /// FR-106's invocation provision.
    pub invocations: BTreeMap<[u8; 32], Vec<u8>>,
    /// The selection.
    pub selection: ClauseRunSelection,
    /// An optional expected `package_id` (FR-098's stale package rule).
    pub expected_package_id: Option<PackageId>,
    /// The spine's stage limits.
    pub limits: SpineLimits,
    /// FR-106's observation limits.
    pub observation_limits: ObservationLimits,
    /// The limits admission re-normalizes the unit's domain packages under
    /// (see `qsl_semantics::model::observation`'s own module doc).
    pub model_limits: ModelNormalizationLimits,
    /// The evaluation meter's accounting limits (FR-100's `work_units`).
    pub accounting: ScalarLimits,
}

/// Why [`run_clause`] returned no report: a request that cannot be formed
/// (FR-109 Outputs). Every compile, selection, admission or evaluation
/// result is a report, never this.
#[derive(Debug, thiserror::Error)]
pub enum ClauseRunRefusal {
    /// The unit's source is empty.
    #[error("the unit source is empty")]
    EmptySource,
}

/// FR-109's disposition stage.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClauseRunStage {
    /// Compile (S1 to S4).
    Compile,
    /// Name resolution.
    Select,
    /// FR-106 admission.
    Admit,
    /// S6a evaluation.
    Evaluate,
}

/// FR-109's disposition: one record per non-`Evaluate` stage, or the S6a
/// outcome (FR-100's mapping, reused unchanged) for `Evaluate`.
#[derive(Debug)]
pub enum ClauseDisposition {
    /// Stage `compile`: the spine compile refused, naming its own cause
    /// code.
    Compile(Box<CompileRefusal>),
    /// Stage `compile`, category `refusal`, `unknown_language`: an I3
    /// extracted clause whose declared fence language is not `ix:native`
    /// (the same refusal the root crate's `mapped::compile` gives). Nothing
    /// is compiled.
    UnknownLanguage {
        /// The declared fence language.
        language: String,
    },
    /// Stage `compile`, category `refusal`, `stale_dependency`: the
    /// recompiled `package_id` differs from the request's expected one.
    StalePackage {
        /// The expected `package_id`.
        expected: PackageId,
        /// The recompiled `package_id`.
        actual: PackageId,
    },
    /// Stage `select`, `missing_declaration`/`missing-name`.
    MissingName {
        /// The selected name.
        name: String,
    },
    /// Stage `select`, `ill_typed`/`type-mismatch`: a `Function` selection
    /// names a function whose declared result is not `Boolean`.
    NotAPredicate {
        /// The selected name.
        name: String,
    },
    /// Stage `admit`: FR-106 admission's own failure, or (for a `Function`
    /// selection) the analogous snapshot-only admission failure.
    Admit(AdmissionFailure),
    /// Stage `admit`: a `Function` selection's argument refused before any
    /// call (FR-100's own refusal, reused).
    ArgumentRefusal(Box<RunRefusal>),
    /// Stage `evaluate`: the mapped S6a outcome.
    Evaluate(CallOutcome),
    /// Stage `evaluate`, category `internal-failure`: a broken S6a
    /// invariant reached outside the outcome mapping (FR-100's own
    /// internal-failure exit status).
    EvaluateFault(qsl_foundation::diagnostic::InternalFault),
}

impl ClauseDisposition {
    /// The FR-109 stage this disposition reports at.
    pub fn stage(&self) -> ClauseRunStage {
        match self {
            Self::Compile(_) | Self::UnknownLanguage { .. } | Self::StalePackage { .. } => {
                ClauseRunStage::Compile
            }
            Self::MissingName { .. } | Self::NotAPredicate { .. } => ClauseRunStage::Select,
            Self::Admit(_) | Self::ArgumentRefusal(_) => ClauseRunStage::Admit,
            Self::Evaluate(_) | Self::EvaluateFault(_) => ClauseRunStage::Evaluate,
        }
    }

    /// FR-109 Outputs' `category` (ADR-013 O-16, `qsl_foundation::
    /// diagnostic::Category`): `AdmissionFailure`'s own `Incomplete`/
    /// `Refused`/`Fault` split is read directly (FR-106's own Incomplete
    /// result is not the catalog's generic `incomplete_population`
    /// category row, which the catalog spells `Refusal` for every code but
    /// `cancelled`/`runtime_invariant` -- FR-106's Incomplete/Refused split
    /// is a QSL-specific distinction this method preserves, never
    /// `category_of`'s catalog-wide default).
    pub fn category(&self) -> qsl_foundation::diagnostic::Category {
        use qsl_foundation::diagnostic::Category;
        match self {
            Self::Compile(_)
            | Self::UnknownLanguage { .. }
            | Self::StalePackage { .. }
            | Self::MissingName { .. }
            | Self::NotAPredicate { .. }
            | Self::ArgumentRefusal(_) => Category::Refusal,
            Self::Admit(AdmissionFailure::Refused(_)) => Category::Refusal,
            Self::Admit(AdmissionFailure::Incomplete(_)) => Category::Incomplete,
            Self::Admit(AdmissionFailure::Fault(_)) | Self::EvaluateFault(_) => {
                Category::InternalFailure
            }
            Self::Evaluate(outcome) => match outcome {
                CallOutcome::Completed(CallValue::Boolean(true)) => Category::Success,
                CallOutcome::Completed(CallValue::Boolean(false)) => Category::Violation,
                CallOutcome::Completed(CallValue::Integer(_)) => Category::Success,
                CallOutcome::Refused(_) => Category::Refusal,
                CallOutcome::Undefined { .. } => Category::Undefined,
                CallOutcome::Incomplete { .. } => Category::Incomplete,
            },
        }
    }

    /// FR-109 Outputs' `truth`: only for `success` and `violation`.
    pub fn truth(&self) -> Option<bool> {
        match self {
            Self::Evaluate(CallOutcome::Completed(CallValue::Boolean(value))) => Some(*value),
            _ => None,
        }
    }
}

/// FR-109 Outputs' provenance: the compiled unit's identity, the I3
/// extraction's original document when one was used, the selections and
/// every admitted document's identity and digest. The unit's byte digest is
/// [`ClauseRunReport::source_digest`].
#[derive(Clone, Debug)]
pub struct ClauseRunProvenance {
    /// The compiled unit's four FR-001 labels: the program source's, or an
    /// I3 extracted body's own.
    pub source: SourceIdentity,
    /// The I3 extraction's original document, when the unit is an I3
    /// extracted source; `None` for a program source.
    pub extraction: Option<ExtractionOrigin>,
    /// Every model selection the compiled package resolved.
    pub model_selections: Vec<qsl_semantics::model::domain_package::DomainPackageRef>,
    /// The selection as given.
    pub selection: ClauseRunSelection,
    /// Every snapshot or invocation document admission actually read, in
    /// read order.
    pub documents: Vec<DocumentRef>,
}

/// FR-109 Outputs' usage: "the admission work and the evaluation meter
/// charges, separately" (`FR-109-run-a-state-clause-through-the-spine.
/// md:85`).
///
/// **Production-safe by construction (QSL-206).** `quire_exact::Meter`
/// deliberately keeps no per-charge log outside `test-support`
/// (`quire-exact/src/accounting.rs:518-529`: "a production meter holds only
/// fixed-size state... the meter never grows with the charge count"), and
/// `test-support` may only be a dev-dependency. `evaluation_charges` below is
/// therefore the meter's own fixed-size, always-available surface --
/// `admission_count()` and one `consumed(kind)` per `LimitKind::ALL` -- never
/// `Meter::admitted_charges()`'s per-event log. This still gives every
/// property AC-5 needs (equal reports on a repeated run) without adding
/// growing state to a production report.
///
/// Admission work is tracked by `ObservationLimits`' own four
/// document-shape ceilings, not `quire_exact`'s `LimitKind` (admission has
/// no analogue of `integer_bits`/`work_units`): `admission_consumed` is
/// `qsl_semantics::model::observation::AdmissionUsage`, the amount those
/// four ceilings actually consumed across every document admission read
/// (SR-751 FND-002 round 2), never pretended complete by reusing a
/// `LimitKind` that would misreport zeroes for kinds admission never
/// touches.
#[derive(Clone, Debug, Default)]
pub struct ClauseRunUsage {
    /// The admission work consumed.
    pub admission_consumed: qsl_semantics::model::observation::AdmissionUsage,
    /// How many charges the evaluation meter admitted in total.
    pub evaluation_admissions: u64,
    /// The evaluation meter's consumed total, one pair per `LimitKind::ALL`.
    pub evaluation_consumed: Vec<(quire_exact::LimitKind, u64)>,
}

impl ClauseRunUsage {
    fn from_meter(
        meter: &Meter,
        admission_consumed: qsl_semantics::model::observation::AdmissionUsage,
    ) -> Self {
        Self {
            admission_consumed,
            evaluation_admissions: meter.admission_count(),
            evaluation_consumed: quire_exact::LimitKind::ALL
                .iter()
                .map(|kind| (*kind, meter.consumed(*kind)))
                .collect(),
        }
    }
}

/// FR-109's `ClauseRunReport`: the disposition, with the source's own
/// digest and, where compile reached it, the compiled `package_id`.
#[derive(Debug)]
pub struct ClauseRunReport {
    /// The compiled unit's `sha256:` digest: the program source's bytes, or
    /// an I3 extracted body's.
    pub source_digest: String,
    /// The compiled `package_id`, when compile completed.
    pub package_id: Option<PackageId>,
    /// The disposition.
    pub disposition: ClauseDisposition,
    /// FR-109 Outputs' provenance.
    pub provenance: ClauseRunProvenance,
    /// FR-109 Outputs' usage.
    pub usage: ClauseRunUsage,
}

/// The `DocumentRef`s a selection names, in read order (before any are
/// necessarily admitted): `Current`'s snapshot, `Invocation`'s invocation,
/// or `Function`'s current snapshot.
fn selection_documents(selection: &ClauseRunSelection) -> Vec<DocumentRef> {
    match selection {
        ClauseRunSelection::Clause(clause) => match &clause.input {
            ClauseSelectionInput::Current { snapshot, .. } => vec![snapshot.clone()],
            ClauseSelectionInput::Invocation { invocation } => vec![invocation.clone()],
        },
        ClauseRunSelection::Function { snapshot, .. } => vec![snapshot.clone()],
    }
}

impl ClauseRunReport {
    /// FR-109: one total match over the stage and category, no `_` arm.
    pub fn exit_code(&self) -> u8 {
        use qsl_foundation::diagnostic::Code;
        match &self.disposition {
            ClauseDisposition::Compile(refusal) => refusal.code().exit_code(),
            ClauseDisposition::UnknownLanguage { .. } => Code::UnknownLanguage.exit_code(),
            ClauseDisposition::StalePackage { .. } => Code::StaleDependency.exit_code(),
            ClauseDisposition::MissingName { .. } => Code::MissingDeclaration.exit_code(),
            ClauseDisposition::NotAPredicate { .. } => Code::IllTyped.exit_code(),
            ClauseDisposition::Admit(AdmissionFailure::Fault(_)) => 30,
            ClauseDisposition::Admit(
                AdmissionFailure::Refused(record) | AdmissionFailure::Incomplete(record),
            ) => record.code.exit_code(),
            ClauseDisposition::ArgumentRefusal(refusal) => match refusal.as_ref() {
                RunRefusal::Fault(_) => 30,
                other => other.code().exit_code(),
            },
            ClauseDisposition::Evaluate(outcome) => evaluate_exit_code(outcome),
            ClauseDisposition::EvaluateFault(_) => 30,
        }
    }
}

fn evaluate_exit_code(outcome: &CallOutcome) -> u8 {
    use qsl_foundation::diagnostic::Code;
    match outcome {
        CallOutcome::Completed(CallValue::Boolean(true)) => 0,
        CallOutcome::Completed(CallValue::Boolean(false)) => 10,
        CallOutcome::Completed(CallValue::Integer(_)) => 0,
        CallOutcome::Refused(super::call::CallRefusal::Record { code, .. }) => {
            Code::from_code(code.code()).map_or(20, |c| c.exit_code())
        }
        CallOutcome::Refused(super::call::CallRefusal::Family { code, .. }) => {
            Code::from_code(code.code()).map_or(20, |c| c.exit_code())
        }
        CallOutcome::Undefined { .. } => 20,
        CallOutcome::Incomplete { .. } => 22,
    }
}

/// What every report of one [`run_clause`] call carries about its unit:
/// its byte digest, its identity and, for an I3 source, the original
/// document.
struct UnitProvenance {
    digest: String,
    source: SourceIdentity,
    extraction: Option<ExtractionOrigin>,
}

/// FR-109: `qsl_replay::spine::run_clause`.
pub fn run_clause(request: ClauseRunRequest) -> Result<ClauseRunReport, ClauseRunRefusal> {
    let unit = request.source.unit();
    if unit.bytes.is_empty() {
        return Err(ClauseRunRefusal::EmptySource);
    }
    let unit_provenance = UnitProvenance {
        digest: ByteDigest::of(unit.bytes).to_string(),
        source: unit.identity.clone(),
        extraction: unit.extraction,
    };
    let selection_documents = selection_documents(&request.selection);
    // Cloned once, up front, so `report` can hold the selection by value
    // without borrowing `request.selection` -- `request.selection` itself is
    // moved out below to be matched by variant.
    let selection_for_provenance = request.selection.clone();
    // `model_selections` is only known once compile has produced a package
    // graph; every pre-compile disposition reports it empty. `documents` is
    // FR-109's "every snapshot and invocation admission read" (`FR-109-run-
    // a-state-clause-through-the-spine.md:81-84`), never merely named by
    // the selection: every call site below passes `Vec::new()` until an
    // admission call has actually run, and `selection_documents.clone()`
    // from then on (FR-109-AC-2's own "no snapshot in its provenance" for
    // a compile-stage refusal).
    let report = |package_id,
                  disposition,
                  model_selections: Vec<qsl_semantics::model::domain_package::DomainPackageRef>,
                  documents: Vec<DocumentRef>| {
        ClauseRunReport {
            source_digest: unit_provenance.digest.clone(),
            package_id,
            disposition,
            provenance: ClauseRunProvenance {
                source: unit_provenance.source.clone(),
                extraction: unit_provenance.extraction.clone(),
                model_selections,
                selection: selection_for_provenance.clone(),
                documents,
            },
            usage: ClauseRunUsage::default(),
        }
    };

    if let Some(language) = unit.language {
        if NativeLanguage::of(language).is_none() {
            return Ok(report(
                None,
                ClauseDisposition::UnknownLanguage {
                    language: language.to_owned(),
                },
                Vec::new(),
                Vec::new(),
            ));
        }
    }

    let compiled = match compile(
        unit.identity.clone(),
        unit.path,
        unit.bytes,
        &request.packages,
        &request.dependencies,
        request.limits,
    ) {
        Ok(compiled) => compiled,
        Err(refusal) => {
            return Ok(report(
                None,
                ClauseDisposition::Compile(refusal),
                Vec::new(),
                Vec::new(),
            ))
        }
    };
    let package_id = compiled.emitted.package_id();
    let model_selections = compiled.package.graph().model_selections().to_vec();
    if let Some(expected) = request.expected_package_id {
        if package_id != expected {
            return Ok(report(
                Some(package_id),
                ClauseDisposition::StalePackage {
                    expected,
                    actual: package_id,
                },
                model_selections,
                Vec::new(),
            ));
        }
    }
    let package = &compiled.package;
    // FND-016: the unit's and every resolved library's source, reused
    // exactly as `compile` already read them (mirroring `call::run`'s own
    // `sources` build), so a locus resolves without a second read of
    // already-admitted bytes.
    let mut sources = Vec::with_capacity(1 + compiled.libraries.len());
    sources.push(compiled.source.clone());
    sources.extend(compiled.libraries.iter().cloned());

    match request.selection {
        ClauseRunSelection::Clause(selection) => {
            let Some(clause) = package.graph().state_clause(&selection.name) else {
                return Ok(report(
                    Some(package_id),
                    ClauseDisposition::MissingName {
                        name: selection.name,
                    },
                    model_selections,
                    Vec::new(),
                ));
            };
            let provisions = Provisions {
                snapshots: &request.snapshots,
                invocations: &request.invocations,
            };
            // The model -> check edge must stay empty (FR-074-AC-3):
            // `admit_observations` (FR-106) is a `model`-layer function, so
            // this caller reads the checked clause's own facts out of
            // `check::CheckedStateClause` here, into model-level
            // `ClauseFacts`/`OperationFacts`, rather than that module
            // importing `check` itself.
            let clause_facts = ClauseFacts {
                identity: clause.identity(),
                kind: clause.kind(),
                context: clause.context(),
                operation: clause.operation().map(|operation| OperationFacts {
                    declaring: operation.declaring,
                    declaration: operation.declaration.clone(),
                }),
            };
            let observations = match admit_observations(
                package.graph().model_selections(),
                package.graph().scope().types(),
                &clause_facts,
                &request.packages,
                request.model_limits,
                &provisions,
                &selection,
                request.observation_limits,
            ) {
                Ok(observations) => observations,
                Err(failure) => {
                    return Ok(report(
                        Some(package_id),
                        ClauseDisposition::Admit(failure),
                        model_selections,
                        selection_documents.clone(),
                    ))
                }
            };
            // FR-109 Outputs' provenance is "the identity and digest of
            // every snapshot and invocation admission read" -- for a
            // precondition/postcondition, that is the invocation plus its
            // pre and/or post snapshot, in the read order FR-106 check 1
            // itself states ("the invocation and then its pre and post
            // snapshots"), not merely the one document the selection named
            // (SR-751 FND-002 round 2).
            let mut admitted_documents = selection_documents.clone();
            if let Some(pre) = &observations.pre {
                admitted_documents.push(pre.identity.clone());
            }
            if let Some(post) = &observations.post {
                admitted_documents.push(post.identity.clone());
            }
            use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
            let Ok(qualified) = QualifiedName::unqualified(&selection.name) else {
                return Ok(report(
                    Some(package_id),
                    ClauseDisposition::MissingName {
                        name: selection.name,
                    },
                    model_selections,
                    selection_documents.clone(),
                ));
            };
            let mut meter = Meter::new(request.accounting);
            let disposition = match package.evaluate_clause(&qualified, &observations, &mut meter) {
                Ok(evaluation) => match convert_outcome(evaluation, package.graph(), &sources) {
                    Ok(outcome) => ClauseDisposition::Evaluate(outcome),
                    Err(refusal) => match *refusal {
                        RunRefusal::Fault(fault) => ClauseDisposition::EvaluateFault(fault),
                        other => ClauseDisposition::ArgumentRefusal(Box::new(other)),
                    },
                },
                Err(qsl_eval::value::CallFailure::Input(input)) => {
                    use qsl_eval::value::InputRefusal;
                    match input {
                        InputRefusal::UnknownClause(name) => {
                            ClauseDisposition::MissingName { name }
                        }
                        // Unreachable in this flow: this module resolves the
                        // named clause once (above) and admits `observations`
                        // against that same clause's identity, so
                        // `evaluate_clause`'s own name/identity checks can
                        // never disagree with it. Treated as a broken
                        // invariant rather than a document defect, never a
                        // wildcard arm (FR-090).
                        InputRefusal::ObservationsMismatch => ClauseDisposition::EvaluateFault(
                            InternalFault::new("call", "clause-observations-mismatch"),
                        ),
                        InputRefusal::UnknownFunction(_)
                        | InputRefusal::Arity { .. }
                        | InputRefusal::WrongValueKind { .. }
                        | InputRefusal::DanglingReference { .. } => {
                            ClauseDisposition::EvaluateFault(InternalFault::new(
                                "call",
                                "evaluate-clause-supplies-no-function-style-input",
                            ))
                        }
                    }
                }
                Err(qsl_eval::value::CallFailure::Fault(fault)) => {
                    ClauseDisposition::EvaluateFault(fault)
                }
            };
            let mut result = report(
                Some(package_id),
                disposition,
                model_selections,
                admitted_documents,
            );
            result.usage = ClauseRunUsage::from_meter(&meter, observations.usage);
            Ok(result)
        }
        ClauseRunSelection::Function {
            ref name,
            ref arguments,
            ref snapshot,
        } => run_function(
            &request.packages,
            request.model_limits,
            &request.snapshots,
            request.observation_limits,
            request.accounting,
            package,
            package_id,
            name,
            arguments,
            snapshot,
            &unit_provenance,
            &sources,
            model_selections,
            request.selection.clone(),
            selection_documents,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn run_function(
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    model_limits: ModelNormalizationLimits,
    snapshots: &BTreeMap<[u8; 32], Vec<u8>>,
    observation_limits: ObservationLimits,
    accounting: ScalarLimits,
    package: &qsl_package::CheckedPackage,
    package_id: PackageId,
    name: &str,
    arguments: &[ClauseArgument],
    snapshot: &DocumentRef,
    unit: &UnitProvenance,
    sources: &[Source],
    model_selections: Vec<qsl_semantics::model::domain_package::DomainPackageRef>,
    selection: ClauseRunSelection,
    selection_documents: Vec<DocumentRef>,
) -> Result<ClauseRunReport, ClauseRunRefusal> {
    use qsl_eval::value::CheckedPackageEvaluation;
    // Empty until `admit_current_snapshot` (below) actually reads
    // `selection_documents`' one snapshot; FR-109's own provenance is what
    // admission read, never merely what the selection named (FR-109-AC-2).
    let documents_read = std::cell::RefCell::new(Vec::<DocumentRef>::new());
    let report = |disposition| ClauseRunReport {
        source_digest: unit.digest.clone(),
        package_id: Some(package_id),
        disposition,
        provenance: ClauseRunProvenance {
            source: unit.source.clone(),
            extraction: unit.extraction.clone(),
            model_selections: model_selections.clone(),
            selection: selection.clone(),
            documents: documents_read.borrow().clone(),
        },
        usage: ClauseRunUsage::default(),
    };

    let selected = match select(package, name) {
        Ok(selected) => selected,
        Err(refusal) => {
            return Ok(match *refusal {
                RunRefusal::MissingDeclaration { function } => {
                    report(ClauseDisposition::MissingName { name: function })
                }
                RunRefusal::UnsupportedResult { function } => {
                    report(ClauseDisposition::NotAPredicate { name: function })
                }
                other => report(ClauseDisposition::ArgumentRefusal(Box::new(other))),
            })
        }
    };
    if !matches!(
        package
            .graph()
            .callable(name)
            .map(|callable| callable.result),
        Some(ValueType::Boolean)
    ) {
        return Ok(report(ClauseDisposition::NotAPredicate {
            name: name.to_owned(),
        }));
    }

    // Admission is about to read `selection_documents`' one snapshot,
    // whichever way it resolves -- set before the call, so both the
    // success and failure paths report it (FR-109-AC-2).
    *documents_read.borrow_mut() = selection_documents;
    let (environment, admission_usage) = match admit_current_snapshot(
        package.graph().model_selections(),
        package.graph().scope().types(),
        packages,
        model_limits,
        snapshots,
        snapshot,
        observation_limits,
    ) {
        Ok(admitted) => admitted,
        Err(failure) => return Ok(report(ClauseDisposition::Admit(failure))),
    };

    // Bind each argument to its parameter (FR-100's own name-binding
    // refusals), then convert to a value: a canonical integer as FR-100
    // does, or an object reference resolved in `environment`.
    let mut slots: Vec<Option<&ClauseArgumentValue>> = vec![None; selected.parameters.len()];
    for argument in arguments {
        let Some(position) = selected
            .parameters
            .iter()
            .position(|(parameter, _)| *parameter == argument.parameter)
        else {
            return Ok(report(ClauseDisposition::ArgumentRefusal(Box::new(
                RunRefusal::UnknownParameter {
                    parameter: argument.parameter.clone(),
                },
            ))));
        };
        if slots[position].replace(&argument.value).is_some() {
            return Ok(report(ClauseDisposition::ArgumentRefusal(Box::new(
                RunRefusal::DuplicateArgument {
                    parameter: argument.parameter.clone(),
                },
            ))));
        }
    }
    let mut bound = Vec::with_capacity(selected.parameters.len());
    for (position, (parameter, value_type)) in selected.parameters.iter().enumerate() {
        let Some(value) = slots[position] else {
            return Ok(report(ClauseDisposition::ArgumentRefusal(Box::new(
                RunRefusal::UnboundParameter {
                    parameter: parameter.clone(),
                },
            ))));
        };
        let converted = match (value, value_type) {
            (ClauseArgumentValue::Integer(0), ValueType::Boolean) => Value::Boolean(false),
            (ClauseArgumentValue::Integer(1), ValueType::Boolean) => Value::Boolean(true),
            (ClauseArgumentValue::Integer(value), ValueType::Integer | ValueType::Int(_)) => {
                Value::Integer(quire_exact::Integer::from(*value))
            }
            (
                ClauseArgumentValue::Reference { population, key },
                ValueType::Reference(type_identity),
            ) => {
                let universe = match population_universe_for(
                    package.graph().model_selections(),
                    packages,
                    model_limits,
                    *type_identity,
                ) {
                    Ok(universe) => universe,
                    Err(failure) => {
                        return Ok(report(ClauseDisposition::Admit(failure)));
                    }
                };
                let Some(reference) = environment.find(universe, key) else {
                    let mut fields = BTreeMap::new();
                    fields.insert("population", population.clone());
                    fields.insert("object", key.clone());
                    return Ok(report(ClauseDisposition::Admit(AdmissionFailure::Refused(
                        AdmissionRecord {
                            code: qsl_foundation::diagnostic::Code::InvalidRuntimeInput,
                            cause: "wrong-role-mapping",
                            fields,
                        },
                    ))));
                };
                Value::Reference(reference.clone())
            }
            _ => {
                return Ok(report(ClauseDisposition::ArgumentRefusal(Box::new(
                    RunRefusal::WrongValueKind { position },
                ))))
            }
        };
        bound.push(converted);
    }

    let mut meter = Meter::new(accounting);
    let disposition = match package.call(&selected.name, bound, &environment, &mut meter) {
        Ok(evaluation) => match convert_outcome(evaluation, package.graph(), sources) {
            Ok(outcome) => ClauseDisposition::Evaluate(outcome),
            Err(refusal) => match *refusal {
                RunRefusal::Fault(fault) => ClauseDisposition::EvaluateFault(fault),
                other => ClauseDisposition::ArgumentRefusal(Box::new(other)),
            },
        },
        Err(failure) => match *convert_call_failure(failure) {
            RunRefusal::Fault(fault) => ClauseDisposition::EvaluateFault(fault),
            other => ClauseDisposition::ArgumentRefusal(Box::new(other)),
        },
    };
    let mut result = report(disposition);
    result.usage = ClauseRunUsage::from_meter(&meter, admission_usage);
    Ok(result)
}

#[cfg(test)]
mod tests;
