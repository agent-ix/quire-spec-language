// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-275, FR-276, FR-278 (ADR-029 OP-1): the spine's front end as typed
//! library operations. [`parse`] reads source bytes into syntax (S1, S2),
//! [`select`] admits the domain packages a unit's `model` declarations name
//! (I1), [`check`] checks and links (S3, S4) and [`package`] emits the
//! checked package (E4). [`execute`] calls a checked function (S6a); its
//! signature names `qsl_eval` types, which the spine's public surface may
//! not (FR-100-AC-8), so it is crate-internal.
//!
//! Each operation takes its predecessor's own type, so a program that passes
//! another stage's value, raw bytes or an unchecked value does not compile.
//! Each takes a caller-owned [`Cancel`] handle: a handle already cancelled
//! returns before any charge, and a handle cancelled during the operation
//! stops it at its next meter charge, with no output (FR-276). The library
//! reads no clock.
//!
//! [`CheckedUnit`] and [`EmittedUnit`] hold the stage outputs
//! `qsl_package::CheckedPackage` and `qsl_package::EmittedPackage` with the
//! sources the unit was compiled from, which the package's replay needs and
//! the stage types do not carry.
//!
//! A stage refusal is the refusing stage's [`CompileRefusal`], boxed. A
//! reached limit is [`StageFailure::Limit`] naming the caller's limits field
//! (FR-277), and a broken invariant is [`StageFailure::Fault`] (FR-275). Each
//! output carries the work each stage did ([`Staged::work`], FR-275-AC-5). The
//! composition `parse`, `select`, `check`, `package` over a source returns
//! the same bytes, stage and cause code the CLI's `compile` writes for it.
//!
//! # Wrong-stage input does not compile
//!
//! Raw source bytes are not a [`ParsedSource`], and the control with the
//! right type compiles:
//!
//! ```compile_fail
//! # use qsl_replay::spine::{check, AdmittedModels, DependencyInput, LockEvidence, SpineLimits};
//! # use quire_exact::Cancel;
//! let _ = check(
//!     b"language \"ix:native\" edition \"1-draft\";",
//!     &AdmittedModels::default(),
//!     &DependencyInput::default(),
//!     &LockEvidence::default(),
//!     SpineLimits::default(),
//!     &Cancel::new(),
//! );
//! ```
//!
//! ```
//! # use qsl_replay::spine::{
//! #     check, AdmittedModels, DependencyInput, LockEvidence, ParsedSource, SpineLimits,
//! # };
//! # use quire_exact::Cancel;
//! fn control(parsed: &ParsedSource) {
//!     let _ = check(
//!         parsed,
//!         &AdmittedModels::default(),
//!         &DependencyInput::default(),
//!         &LockEvidence::default(),
//!         SpineLimits::default(),
//!         &Cancel::new(),
//!     );
//! }
//! ```
//!
//! A [`ParsedSource`] is not a [`CheckedUnit`], so it cannot be emitted:
//!
//! ```compile_fail
//! # use qsl_replay::spine::{package, PackageLimits, ParsedSource};
//! # use quire_exact::Cancel;
//! fn emit(parsed: &ParsedSource) {
//!     let _ = package(parsed, PackageLimits::default(), &Cancel::new());
//! }
//! ```
//!
//! ```
//! # use qsl_replay::spine::{package, CheckedUnit, PackageLimits};
//! # use quire_exact::Cancel;
//! fn control(checked: &CheckedUnit) {
//!     let _ = package(checked, PackageLimits::default(), &Cancel::new());
//! }
//! ```

use std::collections::BTreeMap;
use std::sync::Arc;

use qsl_cst::Limits as SourceLimits;
use qsl_eval::value::{CallFailure, CheckedPackageEvaluation, Evaluation};
use qsl_forms::{build_unit, ParsedUnit};
use qsl_foundation::diagnostic::{
    InternalFault, LimitExceeded, LimitKind as FoundationKind, LimitsField, Locus, StageFailure,
    StageWork, Staged,
};
use qsl_foundation::selection::ImportSelection;
use qsl_foundation::source::provenance::{RawSourceRef, SourceRegion};
use qsl_foundation::source::Source;
use qsl_foundation::SourceIdentity;
use qsl_foundation::SyntaxLimit;
use qsl_package as pkg;
use qsl_package::{emit_checked_with_cancel, read_import_view, AdmittedPackages, Emission, Import};
use qsl_semantics::check::{
    resolve_profiles, AdmittedImport, AssemblyCause, AssemblyLimits, CheckCause, CheckingLimitKind,
    LockEvidence, PackageDeclarations,
};
use qsl_semantics::library::{ImportView, LibraryName};
use qsl_semantics::model::accounting::LimitKind as ModelKind;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit_with_cancel, SelectedModel, UnitIntakeCause};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::model::refusal::ModelRefusalCause;
use quire_exact::{Cancel, Meter, ScalarLimits, Value};

use super::{region, CompileRefusal, DependencyInput, ImportRefusal, SpineLimits, SuppliedLibrary};

/// What a front-end operation returns when it produces no output: the
/// stage's refusal, a reached limit, a cancellation or a fault.
pub type FrontEndFailure = StageFailure<Box<CompileRefusal>>;

/// The charges `run` made against `cancel`, added to `counter`: the work of
/// one stage (FR-275-AC-5).
fn charged<T>(cancel: &Cancel, counter: &mut u64, run: impl FnOnce() -> T) -> T {
    let before = cancel.charges();
    let output = run();
    *counter = counter.saturating_add(u64::from(cancel.charges().saturating_sub(before)));
    output
}

/// The one place an operation's outcome is decided. A handle already
/// cancelled returns before `run` makes any charge. Otherwise a charge that
/// saw the cancellation makes `run` stop early, and its result, whatever
/// the denial became on its way up, is replaced by the cancellation: an
/// operation that was cancelled returns no output. A refusal that is a
/// reached limit is returned as the limit, naming the caller's limits field
/// (FR-277), and one that is a broken invariant as the fault (FR-275).
fn stage<T>(
    cancel: &Cancel,
    source_len: &dyn Fn(&SourceIdentity) -> Option<usize>,
    run: impl FnOnce(&mut StageWork) -> Result<T, Box<CompileRefusal>>,
) -> Result<Staged<T>, FrontEndFailure> {
    if let Some(cause) = cancel.cause() {
        return Err(StageFailure::Cancelled(cause));
    }
    // The stages' charges are counted while the operation runs; an
    // evaluation that counts nothing polls with a load alone.
    let _counting = cancel.count_charges();
    let mut work = StageWork::default();
    let result = run(&mut work);
    if let Some(cause) = cancel.tripped() {
        return Err(StageFailure::Cancelled(cause));
    }
    match result {
        Ok(value) => Ok(Staged::new(value).with_work(work)),
        Err(refusal) => Err(failure_of(refusal, source_len)),
    }
}

/// `refusal` as the stage failure it is: a limit, a fault or a refusal.
/// `source_len` gives the byte length of the source an identity labels, for
/// the byte ceiling's counter.
fn failure_of(
    refusal: Box<CompileRefusal>,
    source_len: &dyn Fn(&SourceIdentity) -> Option<usize>,
) -> FrontEndFailure {
    if let Some(limit) = limit_of(&refusal, source_len) {
        return StageFailure::Limit(limit);
    }
    if let Some(fault) = fault_of(&refusal) {
        return StageFailure::Fault(fault);
    }
    StageFailure::Refused(refusal)
}

/// The reached limit `refusal` reports, with the caller's limits field that
/// raises it (FR-277), or `None` for a refusal that is not a limit.
fn limit_of(
    refusal: &CompileRefusal,
    source_len: &dyn Fn(&SourceIdentity) -> Option<usize>,
) -> Option<LimitExceeded> {
    let at = |limit: LimitExceeded, region: Option<&SourceRegion>| {
        limit.at(region.cloned().map(Locus::Region))
    };
    match refusal {
        CompileRefusal::Source(diagnostic) => {
            // The counter at the failed charge: the source's real length for
            // the byte ceiling, and the first value past the bound for the
            // charges that stop at it (a token, a node, a parser step).
            let (kind, configured, actual, field) = match diagnostic.limit()? {
                SyntaxLimit::SourceBytes { bound } => (
                    FoundationKind::InputBytes,
                    bound,
                    source_len(&diagnostic.source).unwrap_or(bound + 1),
                    LimitsField::SourceBytes,
                ),
                SyntaxLimit::Tokens { bound } => (
                    FoundationKind::TokenCount,
                    bound,
                    bound + 1,
                    LimitsField::SourceTokens,
                ),
                SyntaxLimit::Nodes { bound } => (
                    FoundationKind::NodeCount,
                    bound,
                    bound + 1,
                    LimitsField::SourceNodes,
                ),
                // Bound and counter are both in parser steps: the unit's total
                // step budget, which `source.work_units` sets per token.
                SyntaxLimit::Work { bound } => (
                    FoundationKind::WorkBudget,
                    bound,
                    bound + 1,
                    LimitsField::SourceWorkUnits,
                ),
            };
            let limit = LimitExceeded::new(
                kind,
                u64::try_from(configured).unwrap_or(u64::MAX),
                u128::try_from(actual).unwrap_or(u128::MAX),
            )
            .named(field);
            Some(at(limit, diagnostic.region.as_ref()))
        }
        CompileRefusal::Intake { refusal, region } => {
            let incomplete = match &refusal.cause {
                UnitIntakeCause::Limit(incomplete) => incomplete,
                // The two edge ceilings refuse as model causes; the
                // counter is the edge count the denied charge would have
                // reached, the first value past the bound.
                UnitIntakeCause::Refused(refusals) => {
                    let (kind, bound, field) =
                        refusals.iter().find_map(|refusal| match refusal.cause {
                            ModelRefusalCause::AncestorSteps { limit, .. } => Some((
                                FoundationKind::EdgeCount,
                                limit,
                                LimitsField::ModelAncestorSteps,
                            )),
                            ModelRefusalCause::FamilySteps { limit, .. } => Some((
                                FoundationKind::EdgeCount,
                                limit,
                                LimitsField::ModelFamilySteps,
                            )),
                            _ => None,
                        })?;
                    let limit = LimitExceeded::new(kind, bound, u128::from(bound) + 1).named(field);
                    return Some(at(limit, region.as_ref()));
                }
                _ => return None,
            };
            let (kind, field) = match incomplete.limit_kind {
                ModelKind::DeclarationRecords => (
                    FoundationKind::OccurrenceCount,
                    LimitsField::ModelDeclarationRecords,
                ),
                ModelKind::DerivationFacts => {
                    (FoundationKind::EdgeCount, LimitsField::ModelDerivationFacts)
                }
                ModelKind::EffectiveDeclarations => (
                    FoundationKind::NodeCount,
                    LimitsField::ModelEffectiveDeclarations,
                ),
                ModelKind::DispatchCandidates => (
                    FoundationKind::EdgeCount,
                    LimitsField::ModelDispatchCandidates,
                ),
                ModelKind::HashedBytes => {
                    (FoundationKind::InputBytes, LimitsField::ModelHashedBytes)
                }
                ModelKind::WorkUnits => (FoundationKind::WorkBudget, LimitsField::ModelWorkUnits),
            };
            let limit = LimitExceeded::new(
                kind,
                incomplete.limit,
                u128::from(incomplete.consumed).saturating_add(u128::from(incomplete.next_charge)),
            )
            .named(field);
            Some(at(limit, region.as_ref()))
        }
        CompileRefusal::Check { refusals, region } => {
            let cause = refusals.iter().find_map(|refusal| match &refusal.cause {
                CheckCause::ResourceExhausted(cause) => Some(cause),
                _ => None,
            })?;
            let (kind, field) = match cause.kind {
                CheckingLimitKind::Nodes => (FoundationKind::NodeCount, LimitsField::CheckingNodes),
                CheckingLimitKind::InputBytes => {
                    (FoundationKind::InputBytes, LimitsField::CheckingInputBytes)
                }
                CheckingLimitKind::WorkBudget => {
                    (FoundationKind::WorkBudget, LimitsField::CheckingWorkBudget)
                }
            };
            let locus = cause.region.as_ref().or(region.as_ref());
            Some(at(
                LimitExceeded::new(kind, cause.limit, cause.actual).named(field),
                locus,
            ))
        }
        CompileRefusal::Assembly { refusal, .. } => {
            refusal.errors.iter().find_map(|error| match &error.cause {
                AssemblyCause::TypeLimit(limit) => Some(match limit.kind() {
                    FoundationKind::EdgeCount => {
                        limit.clone().named(LimitsField::EnvironmentAncestorSteps)
                    }
                    _ => limit.clone().named(LimitsField::EnvironmentWorkUnits),
                }),
                _ => None,
            })
        }
        CompileRefusal::Import {
            refusal: ImportRefusal::Limit(limit),
            region,
        } => Some(at(limit.clone(), region.as_ref())),
        CompileRefusal::Dependency { refusal, .. } => limit_of(refusal, source_len),
        CompileRefusal::Limit(limit) => Some(limit.clone()),
        CompileRefusal::Forms { .. }
        | CompileRefusal::Profile { .. }
        | CompileRefusal::DependencyInput(_)
        | CompileRefusal::Import { .. }
        | CompileRefusal::Link(_)
        | CompileRefusal::Emit(_)
        | CompileRefusal::Omitted(_) => None,
    }
}

/// The internal fault `refusal` reports (FR-275), or `None` for a refusal of
/// the input.
fn fault_of(refusal: &CompileRefusal) -> Option<InternalFault> {
    match refusal {
        CompileRefusal::Check { refusals, .. } => {
            refusals.iter().find_map(|refusal| match &refusal.cause {
                CheckCause::InternalFault(fault) => {
                    Some(InternalFault::new("S3", fault.invariant()))
                }
                _ => None,
            })
        }
        CompileRefusal::Intake { refusal, .. }
            if matches!(refusal.cause, UnitIntakeCause::Invariant) =>
        {
            Some(InternalFault::new(
                "I1",
                "record-reader-refused-with-no-refusal",
            ))
        }
        CompileRefusal::Dependency { refusal, .. } => fault_of(refusal),
        _ => None,
    }
}

/// The stage refusal of `failure`, for a caller that made its own [`Cancel`]
/// handle and shares it with nobody, so it is never cancelled. A reached
/// limit is the refusal [`CompileRefusal::Limit`]. A failure that is not a
/// refusal or a limit, a fault or a cancellation of a handle nobody shares,
/// is returned as the fault it is.
pub fn refusal_or_fault(failure: FrontEndFailure) -> Result<Box<CompileRefusal>, InternalFault> {
    match failure {
        StageFailure::Refused(refusal) => Ok(refusal),
        StageFailure::Limit(limit) => Ok(Box::new(CompileRefusal::Limit(limit))),
        StageFailure::Fault(fault) => Err(fault),
        StageFailure::Cancelled(_) => Err(InternalFault::new(
            "spine",
            "cancelled-without-a-shared-handle",
        )),
    }
}

/// The request [`parse`] takes: the unit's source bytes, the identity they
/// are labelled with and the path they are displayed as. The path is only
/// displayed, never opened.
#[derive(Clone, Copy, Debug)]
pub struct ParseRequest<'a> {
    /// The source's identity.
    pub source: &'a SourceIdentity,
    /// The path the source is displayed as.
    pub path: &'a str,
    /// The source bytes.
    pub bytes: &'a [u8],
}

/// A unit read into syntax (S1) and forms (S2).
#[derive(Clone, Debug)]
pub struct ParsedSource {
    syntax: qsl_cst::ParsedSource,
    unit: ParsedUnit,
}

impl ParsedSource {
    /// The unit's source, exactly as S1 read it.
    pub fn source(&self) -> &Source {
        self.syntax.source()
    }

    /// S1's parse, whose [`LosslessCst`](qsl_cst::LosslessCst) tooling reads.
    pub fn syntax(&self) -> &qsl_cst::ParsedSource {
        &self.syntax
    }

    /// S2's forms.
    pub fn unit(&self) -> &ParsedUnit {
        &self.unit
    }
}

/// The domain packages a unit's `model` declarations selected, admitted at
/// I1, with the package documents they were admitted from.
#[derive(Clone, Debug, Default)]
pub struct AdmittedModels {
    models: Vec<SelectedModel>,
    packages: BTreeMap<[u8; 32], Vec<u8>>,
}

impl AdmittedModels {
    /// The admitted `model` declarations, in source order.
    pub fn models(&self) -> &[SelectedModel] {
        &self.models
    }
}

/// A unit checked and linked: the in-process checked package and the
/// sources it was compiled from.
#[derive(Debug)]
pub struct CheckedUnit {
    package: pkg::CheckedPackage,
    source: Source,
    libraries: Vec<Source>,
}

impl CheckedUnit {
    /// The S4 in-process package.
    pub fn package(&self) -> &pkg::CheckedPackage {
        &self.package
    }

    /// The unit's own source, exactly as S1 read it (FND-016: a caller
    /// resolving a locus over it reuses this rather than reading the same
    /// bytes again).
    pub fn source(&self) -> &Source {
        &self.source
    }

    /// Every library the check actually resolved, transitively, each
    /// exactly as its own S1 read it. Never the full dependency input: a
    /// supplied library the unit's own imports never reached is not compiled
    /// and carries no source here.
    pub fn libraries(&self) -> &[Source] {
        &self.libraries
    }
}

/// A checked package emitted as `quire.checked-package/v2` bytes, with the
/// digest-addressed sources another party needs to import or replay it.
#[derive(Clone, Debug)]
pub struct EmittedUnit {
    package: pkg::EmittedPackage,
    sources: Vec<Source>,
}

impl EmittedUnit {
    /// The v2 wire bytes with their `package_id`.
    pub fn package(&self) -> &pkg::EmittedPackage {
        &self.package
    }

    /// The source provision: the unit's source and every resolved library's,
    /// each by its own digest ([`Source::reference`]).
    pub fn sources(&self) -> &[Source] {
        &self.sources
    }
}

/// S1 and S2: read `request`'s bytes into syntax (FR-278). Refuses
/// `invalid_syntax` for a source the parser recovered from, and with S2's
/// own refusal for a unit no form represents.
pub fn parse(
    request: &ParseRequest<'_>,
    limits: SourceLimits,
    cancel: &Cancel,
) -> Result<Staged<ParsedSource>, FrontEndFailure> {
    let source_len =
        |identity: &SourceIdentity| (identity == request.source).then_some(request.bytes.len());
    stage(cancel, &source_len, |work| {
        charged(cancel, &mut work.s1, || {
            parse_unit(
                request.source.clone(),
                request.path,
                request.bytes,
                limits,
                cancel,
            )
        })
    })
}

/// I1: admit each domain package `parsed`'s `model` declarations select,
/// from `packages`, the supplied documents by their `sha256-jcs` digest
/// (FR-056, FR-278). Refuses at the first `model` declaration that does not
/// admit.
pub fn select(
    parsed: &ParsedSource,
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    limits: ModelNormalizationLimits,
    cancel: &Cancel,
) -> Result<Staged<AdmittedModels>, FrontEndFailure> {
    stage(cancel, &|_| None, |work| {
        let models = charged(cancel, &mut work.i1, || {
            select_models(parsed, packages, limits, cancel)
        })?;
        Ok(AdmittedModels {
            models,
            packages: packages.clone(),
        })
    })
}

/// S3 and S4: check and link `parsed` against `models`, the dependency
/// input (ADR-015 D-1, FR-099) and the package's lock evidence (ADR-011
/// §2.4, FR-278). The S4 source resolution compiles
/// each library the unit's `import`s reach, against the same package input,
/// dependency input and `limits`.
pub fn check(
    parsed: &ParsedSource,
    models: &AdmittedModels,
    dependencies: &DependencyInput,
    lock: &LockEvidence,
    limits: SpineLimits,
    cancel: &Cancel,
) -> Result<Staged<CheckedUnit>, FrontEndFailure> {
    // The byte length of the unit's own source and of each supplied library.
    let source_len = |identity: &SourceIdentity| {
        if identity == parsed.source().identity() {
            return Some(parsed.source().text().len());
        }
        dependencies
            .libraries
            .values()
            .find(|library| &library.source == identity)
            .map(|library| library.bytes.len())
    };
    stage(cancel, &source_len, |work| {
        dependencies
            .check_unit_owner(parsed.source().identity())
            .map_err(|refusal| Box::new(CompileRefusal::DependencyInput(refusal)))?;
        let mut resolution = Resolution {
            dependencies,
            packages: &models.packages,
            limits,
            cancel,
            active: Vec::new(),
            compiled: BTreeMap::new(),
            import_edges: 0,
            source_bytes: 0,
            admitted: AdmittedPackages::default(),
            work: StageWork::default(),
        };
        let package = resolution.check_unit(parsed, models.models.clone(), lock)?;
        *work = resolution.work;
        Ok(CheckedUnit {
            package,
            source: parsed.source().clone(),
            libraries: resolution
                .compiled
                .into_values()
                .map(|library| library.source.clone())
                .collect(),
        })
    })
}

/// The limits value [`package`] takes (FR-275, FR-277). E4 enforces no bound
/// of its own yet, so it has no field; a bound E4 gains is a field here.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct PackageLimits {}

/// E4: emit `checked` as `quire.checked-package/v2` bytes with its source
/// provision (FR-278). It runs no stage up to S4.
pub fn package(
    checked: &CheckedUnit,
    limits: PackageLimits,
    cancel: &Cancel,
) -> Result<Staged<EmittedUnit>, FrontEndFailure> {
    let PackageLimits {} = limits;
    stage(cancel, &|_| None, |work| {
        let emission = charged(cancel, &mut work.e4, || emit_unit(&checked.package, cancel))?;
        let sources = std::iter::once(&checked.source)
            .chain(&checked.libraries)
            .cloned()
            .collect();
        Ok(EmittedUnit {
            package: emission.package().clone(),
            sources,
        })
    })
}

/// The request [`execute`] takes: the function, its admitted argument
/// values and the object environment it runs in.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ExecuteRequest<'a> {
    /// The function to call.
    pub function: &'a qsl_eval::value::QualifiedName,
    /// Its argument values, in parameter order.
    pub arguments: &'a [Value],
    /// The object environment.
    pub objects: &'a ObjectEnvironment,
}

/// S6a: call a checked function under `accounting` (FR-275). It runs no
/// stage up to S4. A cancelled handle returns
/// [`CallFailure::Cancelled`] and no value.
pub(crate) fn execute(
    package: &CheckedUnit,
    request: &ExecuteRequest<'_>,
    accounting: ScalarLimits,
    cancel: &Cancel,
) -> Result<Evaluation, CallFailure> {
    if let Some(cause) = cancel.cause() {
        return Err(CallFailure::Cancelled(cause));
    }
    let mut meter = Meter::new(accounting).with_cancel(cancel.clone());
    let evaluation = package.package.call(
        request.function,
        request.arguments.to_vec(),
        request.objects,
        &mut meter,
    );
    if let Some(cause) = cancel.tripped() {
        return Err(CallFailure::Cancelled(cause));
    }
    evaluation
}

/// S1 then S2 over one unit's bytes: the part of every unit's compile that
/// does not depend on its imports.
fn parse_unit(
    source: SourceIdentity,
    path: &str,
    bytes: &[u8],
    limits: SourceLimits,
    cancel: &Cancel,
) -> Result<ParsedSource, Box<CompileRefusal>> {
    let parsed = qsl_cst::parse_with_cancel(source, path, bytes, limits, cancel)
        .map_err(|diagnostic| Box::new(CompileRefusal::Source(diagnostic)))?;
    if let Some(first) = parsed.diagnostics().first() {
        return Err(Box::new(CompileRefusal::Source(Box::new(first.clone()))));
    }
    let raw = parsed.source().reference().clone();
    let unit = build_unit(&parsed).map_err(|failure| {
        Box::new(CompileRefusal::Forms {
            region: failure.span.and_then(|span| region(&raw, span)),
            failure,
        })
    })?;
    Ok(ParsedSource {
        syntax: parsed,
        unit,
    })
}

/// I1 over one unit's `model` declarations.
fn select_models(
    parsed: &ParsedSource,
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    limits: ModelNormalizationLimits,
    cancel: &Cancel,
) -> Result<Vec<SelectedModel>, Box<CompileRefusal>> {
    let raw = parsed.source().reference();
    admit_unit_with_cancel(&parsed.unit.selections().models, packages, limits, cancel).map_err(
        |refusal| {
            Box::new(CompileRefusal::Intake {
                region: region(raw, refusal.span),
                refusal,
            })
        },
    )
}

/// E4 over one checked package, refusing a wire that would omit nodes: a
/// package missing part of the checked graph is partial output, which E4
/// never writes (ADR-011 §2.3).
fn emit_unit(
    package: &pkg::CheckedPackage,
    cancel: &Cancel,
) -> Result<Emission, Box<CompileRefusal>> {
    let emission = emit_checked_with_cancel(package, cancel)
        .map_err(|refusal| Box::new(CompileRefusal::Emit(refusal)))?;
    if !emission.omitted().is_empty() {
        return Err(Box::new(CompileRefusal::Omitted(
            emission.omitted().to_vec(),
        )));
    }
    Ok(emission)
}

/// One unit through all four operations, over a handle nobody cancels: the
/// compile the spine's unit tests use to build their fixtures. Production
/// callers compose the operations themselves.
#[cfg(test)]
#[derive(Debug)]
pub(crate) struct ComposedUnit {
    /// The S4 in-process package.
    pub(crate) package: pkg::CheckedPackage,
    /// The emitted wire bytes with their `package_id`.
    pub(crate) emitted: pkg::EmittedPackage,
    /// The unit's own source.
    pub(crate) source: Source,
}

/// [`ComposedUnit`] of `bytes` under `limits`.
#[cfg(test)]
pub(crate) fn compose(
    source: SourceIdentity,
    path: &str,
    bytes: &[u8],
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    dependencies: &DependencyInput,
    limits: SpineLimits,
) -> Result<ComposedUnit, Box<CompileRefusal>> {
    let cancel = Cancel::new();
    let refusal = |failure| match refusal_or_fault(failure) {
        Ok(refusal) => refusal,
        Err(fault) => panic!("the front end faulted: {fault:?}"),
    };
    let parsed = parse(
        &ParseRequest {
            source: &source,
            path,
            bytes,
        },
        limits.source,
        &cancel,
    )
    .map_err(refusal)?
    .into_value();
    let models = select(&parsed, packages, limits.model, &cancel)
        .map_err(refusal)?
        .into_value();
    let checked = check(
        &parsed,
        &models,
        dependencies,
        &LockEvidence::default(),
        limits,
        &cancel,
    )
    .map_err(refusal)?
    .into_value();
    let emitted = package(&checked, PackageLimits::default(), &cancel)
        .map_err(refusal)?
        .into_value();
    Ok(ComposedUnit {
        emitted: emitted.package().clone(),
        source: checked.source,
        package: checked.package,
    })
}

#[cfg(test)]
mod tests;

/// A library the S4 source resolution compiled: its checked package and
/// its import view.
#[derive(Debug)]
struct ResolvedLibrary {
    package: Arc<pkg::CheckedPackage>,
    view: ImportView,
    /// This library's own source, exactly as its own `qsl_cst::parse` read
    /// it (FND-016).
    source: Source,
}

/// One compile's S4 source resolution state (ADR-015 D-1).
struct Resolution<'a> {
    dependencies: &'a DependencyInput,
    packages: &'a BTreeMap<[u8; 32], Vec<u8>>,
    limits: SpineLimits,
    cancel: &'a Cancel,
    /// The libraries whose compile is in progress, outermost first.
    active: Vec<LibraryName>,
    /// Each library whose compile completed. A library is compiled at most
    /// once per compile, so the number of library compiles is at most the
    /// number of supplied libraries.
    compiled: BTreeMap<LibraryName, Arc<ResolvedLibrary>>,
    /// The imports resolved so far across the closure
    /// (`dependency.import_edges`).
    import_edges: usize,
    /// The summed input bytes of the library sources compiled so far
    /// (`dependency.source_bytes`).
    source_bytes: usize,
    /// IR's admitted package of each library read so far, which a later
    /// library's view read takes for its closure instead of reading again.
    admitted: AdmittedPackages,
    /// The charges each stage made, over the unit and every library it
    /// compiled.
    work: StageWork,
}

/// One unit whose imports the resolution is walking: the unit itself, or a
/// library whose compile is in progress.
struct Frame {
    raw: RawSourceRef,
    unit: ParsedUnit,
    models: Vec<SelectedModel>,
    lock: LockEvidence,
    /// The index of the import being resolved.
    next: usize,
    admitted: Vec<AdmittedImport>,
    links: Vec<Import>,
}

impl Frame {
    fn new(parsed: &ParsedSource, models: Vec<SelectedModel>, lock: LockEvidence) -> Self {
        let imports = parsed.unit.selections().imports.len();
        Self {
            raw: parsed.source().reference().clone(),
            unit: parsed.unit.clone(),
            models,
            lock,
            next: 0,
            admitted: Vec::with_capacity(imports),
            links: Vec::with_capacity(imports),
        }
    }

    /// Records the resolved `library` for the import being resolved and
    /// moves to the next import.
    fn push_import(&mut self, identity: LibraryName, library: &ResolvedLibrary) {
        self.admitted.push(AdmittedImport {
            identity: identity.clone(),
            view: library.view.clone(),
            graph: library.package.shared_graph(),
        });
        self.links.push(Import {
            identity,
            package: Arc::clone(&library.package),
        });
        self.next += 1;
    }
}

/// A library whose compile is in progress on the resolution's stack.
struct Library {
    identity: LibraryName,
    /// This library's own source, exactly as its own `qsl_cst::parse` read
    /// it (FND-016).
    source: Source,
    frame: Frame,
}

/// What ADR-015 D-1's steps 1 to 3 decide for one import.
enum Selected<'a> {
    /// A library compiled earlier in this closure, reused.
    Reused(LibraryName, Arc<ResolvedLibrary>),
    /// The supplied library to compile.
    Compile(LibraryName, &'a SuppliedLibrary),
}

/// The frame the walk is in: the innermost library, or the unit itself when
/// no library compile is in progress.
fn current<'a>(root: &'a mut Frame, libraries: &'a mut [Library]) -> &'a mut Frame {
    match libraries.last_mut() {
        Some(library) => &mut library.frame,
        None => root,
    }
}

/// `refusal`, raised while the libraries on the stack are being compiled, as
/// the unit reports it: wrapped under each library's dependency path,
/// innermost first (ADR-015 D-1).
fn unwind(libraries: &[Library], refusal: Box<CompileRefusal>) -> Box<CompileRefusal> {
    libraries
        .iter()
        .rev()
        .fold(refusal, |refusal, library| wrap(&library.identity, refusal))
}

impl<'a> Resolution<'a> {
    /// One library through S1 and I1, ready to resolve its own imports.
    fn begin_library(
        &mut self,
        identity: LibraryName,
        supplied: &SuppliedLibrary,
    ) -> Result<Library, Box<CompileRefusal>> {
        let (limits, cancel, packages) = (self.limits, self.cancel, self.packages);
        let parsed = charged(cancel, &mut self.work.s1, || {
            parse_unit(
                supplied.source.clone(),
                &supplied.path,
                &supplied.bytes,
                limits.source,
                cancel,
            )
        })?;
        let models = charged(cancel, &mut self.work.i1, || {
            select_models(&parsed, packages, limits.model, cancel)
        })?;
        // A library is compiled independently of whoever imports it, so its
        // recomputed `package_id` cannot depend on the importer: it is
        // checked under the default lock evidence, never the importer's.
        Ok(Library {
            identity,
            source: parsed.source().clone(),
            frame: Frame::new(&parsed, models, LockEvidence::default()),
        })
    }

    /// One unit through S3 and S4 once its imports are resolved.
    fn finish_unit(
        &mut self,
        frame: &mut Frame,
    ) -> Result<pkg::CheckedPackage, Box<CompileRefusal>> {
        let limits = self.limits;
        let raw = frame.raw.clone();
        let unit = frame.unit.clone();
        let models = std::mem::take(&mut frame.models);
        let admitted = std::mem::take(&mut frame.admitted);
        let links = std::mem::take(&mut frame.links);
        resolve_profiles(&unit.selections().profiles).map_err(|refusals| {
            Box::new(CompileRefusal::Profile {
                region: refusals
                    .first()
                    .and_then(|refusal| region(&raw, refusal.identity_span)),
                refusals,
            })
        })?;
        let cancel = self.cancel;
        let mut declarations = charged(cancel, &mut self.work.s3, || {
            PackageDeclarations::assemble_with_cancel(
                raw.clone(),
                unit,
                models,
                admitted,
                AssemblyLimits {
                    environment: limits.environment,
                    ..AssemblyLimits::default()
                },
                cancel,
            )
        })
        .map_err(|refusal| {
            Box::new(CompileRefusal::Assembly {
                // A type-environment stage limit names no declaration,
                // so it has no region (FR-082, FR-096).
                region: refusal
                    .errors
                    .first()
                    .filter(|error| !matches!(error.cause, AssemblyCause::TypeLimit(_)))
                    .and_then(|error| region(&raw, error.span)),
                refusal,
            })
        })?;
        declarations.lock_evidence = frame.lock.clone();
        let regions = declarations.regions();
        let graph = charged(cancel, &mut self.work.s3, || {
            declarations.check_with_cancel(limits.checking, cancel)
        })
        .map_err(|refusals| {
            Box::new(CompileRefusal::Check {
                region: refusals
                    .first()
                    .and_then(|refusal| regions.refusal_region(refusal)),
                refusals,
            })
        })?;
        charged(cancel, &mut self.work.s4, || {
            pkg::CheckedPackage::link_with(graph, links)
        })
        .map_err(|refusal| Box::new(CompileRefusal::Link(refusal)))
    }

    /// One parsed unit through S3 and S4, resolving its imports depth first
    /// over an explicit stack of the library compiles in progress, so a
    /// dependency chain of any length compiles on a constant native stack
    /// (ADR-030 D-1).
    fn check_unit(
        &mut self,
        parsed: &ParsedSource,
        models: Vec<SelectedModel>,
        lock: &LockEvidence,
    ) -> Result<pkg::CheckedPackage, Box<CompileRefusal>> {
        let mut root = Frame::new(parsed, models, lock.clone());
        let mut libraries: Vec<Library> = Vec::new();
        loop {
            let frame = current(&mut root, &mut libraries);
            if let Some(import) = frame.unit.selections().imports.get(frame.next).cloned() {
                let raw = frame.raw.clone();
                match self
                    .select_import(&raw, &import)
                    .map_err(|refusal| unwind(&libraries, refusal))?
                {
                    Selected::Reused(identity, library) => {
                        current(&mut root, &mut libraries).push_import(identity, &library);
                    }
                    Selected::Compile(identity, supplied) => {
                        // 4. Compile, by this same resolution.
                        let library = self
                            .begin_library(identity.clone(), supplied)
                            .map_err(|refusal| unwind(&libraries, wrap(&identity, refusal)))?;
                        self.active.push(identity);
                        libraries.push(library);
                    }
                }
                continue;
            }
            let package = self
                .finish_unit(frame)
                .map_err(|refusal| unwind(&libraries, refusal))?;
            if libraries.is_empty() {
                return Ok(package);
            }
            let cancel = self.cancel;
            let emission = charged(cancel, &mut self.work.e4, || emit_unit(&package, cancel))
                .map_err(|refusal| unwind(&libraries, refusal))?;
            let Some(done) = libraries.pop() else {
                return Ok(package);
            };
            self.active.pop();
            // 5. View, in the importing unit's context.
            let parent = current(&mut root, &mut libraries);
            let at_import = parent
                .unit
                .selections()
                .imports
                .get(parent.next)
                .and_then(|import| region(&parent.raw, import.span));
            let view = read_import_view(
                &package,
                &emission,
                done.identity.clone(),
                self.packages,
                &mut self.admitted,
            )
            .map_err(|refusal| {
                unwind(
                    &libraries,
                    Box::new(CompileRefusal::Import {
                        refusal: ImportRefusal::View {
                            identity: done.identity.clone(),
                            refusal: Box::new(refusal),
                        },
                        region: at_import,
                    }),
                )
            })?;
            let library = Arc::new(ResolvedLibrary {
                package: Arc::new(package),
                view,
                source: done.source,
            });
            self.compiled
                .insert(done.identity.clone(), Arc::clone(&library));
            current(&mut root, &mut libraries).push_import(done.identity, &library);
        }
    }

    /// ADR-015 D-1's steps for one `import` of the unit `raw` names:
    /// cycle, reuse, selection.
    fn select_import(
        &mut self,
        raw: &RawSourceRef,
        import: &ImportSelection,
    ) -> Result<Selected<'a>, Box<CompileRefusal>> {
        let at_identity = || region(raw, import.identity_span);
        let refuse = |refusal, region| Box::new(CompileRefusal::Import { refusal, region });
        let limits = self.limits.dependencies;
        let reached = |kind, bound: usize, actual: usize, field| {
            refuse(
                ImportRefusal::Limit(
                    LimitExceeded::new(
                        kind,
                        u64::try_from(bound).unwrap_or(u64::MAX),
                        u128::try_from(actual).unwrap_or(u128::MAX),
                    )
                    .named(field),
                ),
                at_identity(),
            )
        };
        // Each import resolved is one edge of the import graph, charged
        // before it is resolved.
        self.import_edges = self.import_edges.saturating_add(1);
        if self.import_edges > limits.import_edges {
            return Err(reached(
                FoundationKind::EdgeCount,
                limits.import_edges,
                self.import_edges,
                LimitsField::DependencyImportEdges,
            ));
        }
        // The parser admits no empty identity.
        let Ok(identity) = LibraryName::new(import.identity.as_str()) else {
            return Err(refuse(ImportRefusal::UnnamedImport, at_identity()));
        };
        // 1. Cycle.
        if let Some(start) = self.active.iter().position(|active| *active == identity) {
            let mut path = self.active[start..].to_vec();
            path.push(identity);
            return Err(refuse(ImportRefusal::Cycle { path }, at_identity()));
        }
        // 2. A library compiled earlier in this closure is reused: one
        // supplied library per identity makes every import of it the same.
        if let Some(library) = self.compiled.get(&identity) {
            return Ok(Selected::Reused(identity, Arc::clone(library)));
        }
        // 3. Selection, by identity alone.
        let dependencies = self.dependencies;
        let Some(supplied) = dependencies.libraries.get(&identity) else {
            return Err(refuse(
                ImportRefusal::MissingSelection { identity },
                at_identity(),
            ));
        };
        // A library compile is one node of the closure, and its source's
        // bytes are summed, each charged before the compile starts.
        let libraries = self
            .compiled
            .len()
            .saturating_add(self.active.len())
            .saturating_add(1);
        if libraries > limits.libraries {
            return Err(reached(
                FoundationKind::NodeCount,
                limits.libraries,
                libraries,
                LimitsField::DependencyLibraries,
            ));
        }
        let source_bytes = self.source_bytes.saturating_add(supplied.bytes.len());
        if source_bytes > limits.source_bytes {
            return Err(reached(
                FoundationKind::InputBytes,
                limits.source_bytes,
                source_bytes,
                LimitsField::DependencySourceBytes,
            ));
        }
        self.source_bytes = source_bytes;
        Ok(Selected::Compile(identity, supplied))
    }
}

/// `refusal`, raised compiling the library `identity`, as the importing
/// unit reports it (ADR-015 D-1): a closure-level refusal unwrapped, and
/// any other as the library's own under its dependency path.
fn wrap(identity: &LibraryName, refusal: Box<CompileRefusal>) -> Box<CompileRefusal> {
    if refusal.is_closure_level() {
        return refusal;
    }
    match *refusal {
        CompileRefusal::Dependency { mut path, refusal } => {
            path.insert(0, identity.clone());
            Box::new(CompileRefusal::Dependency { path, refusal })
        }
        refusal => Box::new(CompileRefusal::Dependency {
            path: vec![identity.clone()],
            refusal: Box::new(refusal),
        }),
    }
}
