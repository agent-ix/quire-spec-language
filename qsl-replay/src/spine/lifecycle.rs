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
//! reached stage limit is [`StageFailure::Limit`] naming its setting
//! (FR-255), a reached ceiling of the caller's budget (model normalization,
//! intake) is that stage's `resource_exhausted` refusal (FR-255 Behavior 1),
//! and a broken invariant is [`StageFailure::Fault`] (FR-275). Each
//! output carries the work each stage did ([`Staged::work`], FR-275-AC-5). The
//! composition `parse`, `select`, `check`, `package` over a source returns
//! the same bytes, stage and cause code the CLI's `compile` writes for it.
//!
//! # Wrong-stage input does not compile
//!
//! Raw source bytes are not a [`ParsedUnitSource`], and the control with the
//! right type compiles:
//!
//! ```compile_fail
//! # use qsl_replay::spine::{check, AdmittedModels, DependencyInput, SpineLimits};
//! # use quire_exact::Cancel;
//! let _ = check(
//!     b"language \"ix:native\" edition \"1-draft\";",
//!     &AdmittedModels::default(),
//!     &DependencyInput::default(),
//!     SpineLimits::default(),
//!     &Cancel::new(),
//! );
//! ```
//!
//! ```
//! # use qsl_replay::spine::{
//! #     check, AdmittedModels, DependencyInput, ParsedUnitSource, SpineLimits,
//! # };
//! # use quire_exact::Cancel;
//! fn control(parsed: &ParsedUnitSource) {
//!     let _ = check(
//!         parsed,
//!         "fixture".to_owned(),
//!         &AdmittedModels::default(),
//!         &DependencyInput::default(),
//!         SpineLimits::default(),
//!         &Cancel::new(),
//!     );
//! }
//! ```
//!
//! A [`ParsedUnitSource`] is not a [`CheckedUnit`], so it cannot be emitted:
//!
//! ```compile_fail
//! # use qsl_replay::spine::{package, PackageLimits, ParsedUnitSource};
//! # use quire_exact::Cancel;
//! fn emit(parsed: &ParsedUnitSource) {
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
use qsl_eval::value::CheckedPackageEvaluation;
use qsl_forms::{parse_clause_text, FormsRefusal, ParsedUnit};
use qsl_foundation::diagnostic::{LimitExceeded, Locus, StageFailure, StageWork, Staged};
use qsl_foundation::selection::ImportSelection;
use qsl_foundation::source::provenance::{RawSourceRef, SourceRegion};
use qsl_foundation::source::Source;
use qsl_foundation::Setting;
use qsl_foundation::SyntaxLimit;
use qsl_foundation::{Code, SourceIdentity};
use qsl_package as pkg;
use qsl_package::{read_import_view, AdmittedPackages, Emission, Import, ImportViewRefusal};
use qsl_semantics::check::{
    resolve_profiles, AdmittedImport, AssemblyCause, CheckCause, CheckRefusal,
    CorrespondenceConflict, KeyFault, PackageDeclarations,
};
use qsl_semantics::library::{ImportView, LibraryName};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{
    admit_unit_with_cancel, SelectedModel, SelectedModels, UnitIntakeCause,
};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::value::identity_limit;
use quire_eval::{CallFailure, Evaluation};
use quire_exact::{Cancel, Meter, ScalarLimits, Value};
use quire_semantic_value::catalog::InternalFault;
use quire_semantic_value::semantic_node::IdentityRefusal;

use super::{
    region, CompileRefusal, DependencyInput, ImportRefusal, SpineLimits, SuppliedLibrary,
    UnitListRefusal,
};

/// What a front-end operation returns when it produces no output: the
/// stage's refusal, a reached limit, a cancellation or a fault.
pub type FrontEndFailure = StageFailure<Box<CompileRefusal>, Vec<super::ProtocolResult>>;

struct StageError {
    refusal: Box<CompileRefusal>,
    protocol_results: Option<Vec<super::ProtocolResult>>,
}

#[derive(Clone, Debug)]
enum ProtocolFailure {
    Source(qsl_cst::CompleteDiagnostic),
    Forms {
        failure: FormsRefusal,
        region: Option<SourceRegion>,
    },
    Check {
        refusals: Vec<CheckRefusal>,
        region: Option<SourceRegion>,
    },
}

impl ProtocolFailure {
    fn from_refusal(refusal: CompileRefusal) -> Self {
        match refusal {
            CompileRefusal::Source(diagnostic) => Self::Source(*diagnostic),
            CompileRefusal::Forms { failure, region } => Self::Forms { failure, region },
            CompileRefusal::Check { refusals, region } => Self::Check { refusals, region },
            other => Self::Check {
                refusals: Vec::new(),
                region: match other {
                    CompileRefusal::Limit(limit) => limit.locus().and_then(|locus| match locus {
                        Locus::Region(region) => Some(region.clone()),
                        _ => None,
                    }),
                    _ => None,
                },
            },
        }
    }

    fn to_refusal(&self) -> CompileRefusal {
        match self {
            Self::Source(diagnostic) => CompileRefusal::Source(Box::new(diagnostic.clone())),
            Self::Forms { failure, region } => CompileRefusal::Forms {
                failure: failure.clone(),
                region: region.clone(),
            },
            Self::Check { refusals, region } => CompileRefusal::Check {
                refusals: refusals.clone(),
                region: region.clone(),
            },
        }
    }
}

impl From<Box<CompileRefusal>> for StageError {
    fn from(refusal: Box<CompileRefusal>) -> Self {
        Self {
            refusal,
            protocol_results: None,
        }
    }
}

impl StageError {
    fn with_protocol_results(
        refusal: Box<CompileRefusal>,
        protocol_results: Vec<super::ProtocolResult>,
    ) -> Self {
        Self {
            refusal,
            protocol_results: Some(protocol_results),
        }
    }
}

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
/// reached limit is returned as the limit, naming its setting
/// (FR-255), and one that is a broken invariant as the fault (FR-275).
fn stage<T, E>(
    cancel: &Cancel,
    source_len: &dyn Fn(&SourceIdentity) -> Option<usize>,
    run: impl FnOnce(&mut StageWork) -> Result<T, E>,
) -> Result<Staged<T>, FrontEndFailure>
where
    E: Into<StageError>,
{
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
        Err(refusal) => Err(failure_of(refusal.into(), source_len)),
    }
}

/// `refusal` as the stage failure it is: a limit, a fault or a refusal.
/// `source_len` gives the byte length of the source an identity labels, for
/// the byte ceiling's counter.
fn failure_of(
    error: StageError,
    source_len: &dyn Fn(&SourceIdentity) -> Option<usize>,
) -> FrontEndFailure {
    let StageError {
        refusal,
        protocol_results,
    } = error;
    if let Some(limit) = limit_of(&refusal, source_len) {
        return match protocol_results {
            Some(results) => StageFailure::LimitWithPayload(limit, results),
            None => StageFailure::Limit(limit),
        };
    }
    if let Some(fault) = fault_of(&refusal) {
        return StageFailure::Fault(fault);
    }
    match protocol_results {
        Some(results) => StageFailure::RefusedWithPayload(Box::new(refusal), results),
        None => StageFailure::Refused(refusal),
    }
}

/// The reached limit `refusal` reports, with the setting that raises it
/// (FR-255), or `None` for a refusal that is not a limit.
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
            let (setting, configured, actual) = match diagnostic.limit()? {
                SyntaxLimit::SourceBytes { bound } => (
                    Setting::S1InputBytes,
                    bound,
                    source_len(&diagnostic.source).unwrap_or(bound + 1),
                ),
                SyntaxLimit::Tokens { bound } => (Setting::S1Tokens, bound, bound + 1),
                SyntaxLimit::Nodes { bound } => (Setting::S1Nodes, bound, bound + 1),
                // Bound and counter are both in parser steps: the unit's total
                // step budget, which `s1.work_units` sets per token.
                SyntaxLimit::Work { bound } => (Setting::S1WorkUnits, bound, bound + 1),
            };
            let limit = LimitExceeded::new(
                setting,
                u64::try_from(configured).unwrap_or(u64::MAX),
                u128::try_from(actual).unwrap_or(u128::MAX),
            );
            Some(at(limit, diagnostic.region.as_ref()))
        }
        // A `ModelNormalizationLimitsV1` ceiling and the intake byte limit
        // are the caller's budgets wherever they are read, so intake keeps
        // them as its `resource_exhausted` refusal (ADR-014 section 1,
        // FR-255 Behavior 1), not a stage limit.
        CompileRefusal::Intake { .. } => None,
        CompileRefusal::Check { refusals, region } => {
            if let Some((bound, required)) =
                refusals
                    .iter()
                    .find_map(|refusal| match refusal.cause.unwrapped() {
                        CheckCause::Identity(IdentityRefusal::InputBytes { bound, required }) => {
                            Some((*bound, *required))
                        }
                        _ => None,
                    })
            {
                return Some(at(identity_limit(bound, required), region.as_ref()));
            }
            let cause = refusals
                .iter()
                .find_map(|refusal| match refusal.cause.unwrapped() {
                    CheckCause::ResourceExhausted(cause) => Some(cause),
                    _ => None,
                })?;
            let locus = cause.region.as_ref().or(region.as_ref());
            Some(at(
                LimitExceeded::new(cause.kind.setting(), cause.limit, cause.actual),
                locus,
            ))
        }
        CompileRefusal::Assembly { refusal, .. } => {
            refusal.errors.iter().find_map(|error| match &error.cause {
                AssemblyCause::TypeLimit(limit)
                | AssemblyCause::DecimalScaleLimit(limit)
                | AssemblyCause::IdentityLimit(limit) => Some(limit.clone()),
                _ => None,
            })
        }
        CompileRefusal::Import {
            refusal: ImportRefusal::Limit(limit),
            region,
        } => Some(at(limit.clone(), region.as_ref())),
        CompileRefusal::Dependency { refusal, .. } => limit_of(refusal, source_len),
        CompileRefusal::Import {
            refusal: ImportRefusal::View { refusal, .. },
            ..
        } => match &**refusal {
            ImportViewRefusal::Read { refusal, .. } => match refusal.as_ref() {
                StageFailure::Limit(limit) => Some(limit.clone()),
                _ => None,
            },
            _ => None,
        },
        CompileRefusal::Limit(limit) => Some(limit.clone()),
        CompileRefusal::UnitList(_)
        | CompileRefusal::Forms { .. }
        | CompileRefusal::OperationClause { .. }
        | CompileRefusal::TypeInvariant { .. }
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
        CompileRefusal::Check { refusals, .. } => refusals.iter().find_map(|refusal| match refusal
            .cause
            .unwrapped()
        {
            CheckCause::InternalFault(fault) => {
                let named = match fault.as_ref() {
                    KeyFault::CorrespondenceConflict(conflict) => match conflict.as_ref() {
                        CorrespondenceConflict::NodeRebound {
                            node,
                            recorded,
                            offered,
                        } => vec![
                            ("node", format!("{node:?}")),
                            ("recorded", format!("{recorded:?}")),
                            ("offered", format!("{offered:?}")),
                        ],
                        CorrespondenceConflict::DeclarationRebound {
                            declaration,
                            recorded,
                            offered,
                        } => vec![
                            ("declaration", format!("{declaration:?}")),
                            ("recorded", format!("{recorded:?}")),
                            ("offered", format!("{offered:?}")),
                        ],
                    },
                    _ => Vec::new(),
                };
                Some(InternalFault::new("S3", fault.invariant()).with_named(named))
            }
            _ => None,
        }),
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
        StageFailure::RefusedWithPayload(refusal, _) => Ok(*refusal),
        StageFailure::Limit(limit) => Ok(Box::new(CompileRefusal::Limit(limit))),
        StageFailure::LimitWithPayload(limit, _) => Ok(Box::new(CompileRefusal::Limit(limit))),
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
pub struct ParsedUnitSource {
    syntax: qsl_cst::ParsedSource,
    unit: ParsedUnit,
    protocol_results: Vec<super::ProtocolResult>,
    protocol_forms_refusal: Option<FormsRefusal>,
}

/// S3-ready admitted source units. Units are held in FR-480 canonical owner
/// order; assembly and emission are deliberately left to the later stage.
#[derive(Clone, Debug)]
pub struct AdmittedUnitList {
    units: Vec<ParsedUnitSource>,
}

impl AdmittedUnitList {
    /// Parsed source units in ascending UTF-8 (`authority`, `identity`) order.
    pub fn units(&self) -> &[ParsedUnitSource] {
        &self.units
    }
}

/// FR-480 pre-parse checks. Call before parsing any request so empty lists,
/// repeated source owners and source/library owner collisions win over S1/S2
/// failures in the units.
pub fn preflight_unit_list(
    requests: &[ParseRequest<'_>],
    dependencies: &DependencyInput,
) -> Result<(), Box<CompileRefusal>> {
    if requests.is_empty() {
        return Err(unit_list_refusal(UnitListRefusal::Empty));
    }
    for (second, request) in requests.iter().enumerate() {
        if let Some(first) = requests[..second]
            .iter()
            .position(|earlier| same_owner(earlier.source, request.source))
        {
            return Err(unit_list_refusal(UnitListRefusal::DuplicateOwner {
                first,
                second,
                authority: request.source.authority.clone(),
                identity: request.source.identity.clone(),
            }));
        }
        for (library, supplied) in &dependencies.libraries {
            if same_owner(&supplied.source, request.source) {
                return Err(unit_list_refusal(UnitListRefusal::LibraryOwner {
                    unit: second,
                    library: library.to_string(),
                    authority: request.source.authority.clone(),
                    identity: request.source.identity.clone(),
                }));
            }
        }
    }
    Ok(())
}

/// FR-480 post-parse unit-list checks and canonical ordering. Parsing remains
/// the caller's responsibility and must be performed once per request in
/// request order after [`preflight_unit_list`].
pub fn admit_parsed_unit_list(
    mut units: Vec<ParsedUnitSource>,
    dependencies: &DependencyInput,
) -> Result<AdmittedUnitList, Box<CompileRefusal>> {
    if units.is_empty() {
        return Err(unit_list_refusal(UnitListRefusal::Empty));
    }
    let requests: Vec<_> = units
        .iter()
        .map(|unit| unit.source().identity().clone())
        .collect();
    // Keep the same source/library admission rule for callers that compose
    // this helper without first calling preflight_unit_list.
    for (unit, identity) in requests.iter().enumerate() {
        for (library, supplied) in &dependencies.libraries {
            if same_owner(&supplied.source, identity) {
                return Err(unit_list_refusal(UnitListRefusal::LibraryOwner {
                    unit,
                    library: library.to_string(),
                    authority: identity.authority.clone(),
                    identity: identity.identity.clone(),
                }));
            }
        }
    }
    for second in 0..units.len() {
        if let Some(first) =
            (0..second).find(|first| same_owner(&requests[*first], &requests[second]))
        {
            return Err(unit_list_refusal(UnitListRefusal::DuplicateOwner {
                first,
                second,
                authority: requests[second].authority.clone(),
                identity: requests[second].identity.clone(),
            }));
        }
    }
    let first_header = header_span(&units[0]);
    let first_profiles: Vec<_> = units[0]
        .unit()
        .selections()
        .profiles
        .iter()
        .map(|selection| selection.identity.as_str())
        .collect();
    for (index, unit) in units.iter().enumerate().skip(1) {
        let profiles: Vec<_> = unit
            .unit()
            .selections()
            .profiles
            .iter()
            .map(|selection| selection.identity.as_str())
            .collect();
        if !headers_match(
            units[0].unit().edition(),
            &first_profiles,
            unit.unit().edition(),
            &profiles,
        ) {
            let second_header = header_span(unit);
            return Err(unit_list_refusal(UnitListRefusal::HeaderConflict {
                first_unit: Box::new(requests[0].clone()),
                first_header,
                second_unit: Box::new(requests[index].clone()),
                second_header,
                region: region(unit.source().reference(), second_header),
            }));
        }
    }
    for (unit_index, unit) in units.iter().enumerate() {
        for import in &unit.unit().selections().imports {
            if let Some(sibling) = requests.iter().enumerate().find_map(|(index, source)| {
                (index != unit_index && source.identity == import.identity).then_some(index)
            }) {
                if !dependencies.libraries.contains_key(
                    &LibraryName::new(&import.identity)
                        .unwrap_or_else(|_| unreachable!("parsed import identities are validated")),
                ) {
                    return Err(unit_list_refusal(UnitListRefusal::SiblingImport {
                        identity: import.identity.clone(),
                        unit: Box::new(requests[sibling].clone()),
                        region: region(unit.source().reference(), import.identity_span),
                    }));
                }
            }
        }
    }
    let mut names: BTreeMap<&str, (usize, qsl_foundation::Span)> = BTreeMap::new();
    for (unit_index, unit) in units.iter().enumerate() {
        for form in unit.unit().forms() {
            let Some(name) = form.declared_name() else {
                continue;
            };
            let span = form.span();
            if let Some((first, first_span)) = names.insert(name, (unit_index, span)) {
                if first != unit_index {
                    return Err(unit_list_refusal(UnitListRefusal::DuplicateName {
                        name: name.to_owned(),
                        first_unit: Box::new(requests[first].clone()),
                        first_span,
                        second_unit: Box::new(requests[unit_index].clone()),
                        second_span: span,
                    }));
                }
            }
        }
    }
    let mut paired: Vec<_> = requests.into_iter().zip(units.drain(..)).collect();
    paired.sort_by(|(left, _), (right, _)| {
        left.authority
            .as_bytes()
            .cmp(right.authority.as_bytes())
            .then_with(|| left.identity.as_bytes().cmp(right.identity.as_bytes()))
    });
    Ok(AdmittedUnitList {
        units: paired.into_iter().map(|(_, unit)| unit).collect(),
    })
}

fn unit_list_refusal(refusal: UnitListRefusal) -> Box<CompileRefusal> {
    Box::new(CompileRefusal::UnitList(refusal))
}

fn same_owner(first: &SourceIdentity, second: &SourceIdentity) -> bool {
    first.authority == second.authority && first.identity == second.identity
}

fn header_span(unit: &ParsedUnitSource) -> qsl_foundation::Span {
    let end = unit
        .unit()
        .selections()
        .profiles
        .last()
        .map(|profile| profile.span.end)
        .or_else(|| qsl_cst::declared_edition(unit.source().text()).map(|edition| edition.span.end))
        .unwrap_or(0);
    qsl_foundation::Span { start: 0, end }
}

fn headers_match(
    first_edition: Option<&str>,
    first_profiles: &[&str],
    second_edition: Option<&str>,
    second_profiles: &[&str],
) -> bool {
    first_edition == second_edition && first_profiles == second_profiles
}

impl ParsedUnitSource {
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

    /// Per-protocol results retained through S2 for a caller that continues
    /// the unit after an attributable protocol refusal.
    pub fn protocol_results(&self) -> &[super::ProtocolResult] {
        &self.protocol_results
    }
}

/// The domain packages a unit's `model` declarations selected, admitted at
/// I1, with the package documents they were admitted from.
#[derive(Clone, Debug, Default)]
pub struct AdmittedModels {
    models: SelectedModels,
    packages: qsl_semantics::model::intake::PackageInput,
}

impl AdmittedModels {
    /// The admitted `model` declarations, in source order.
    pub fn models(&self) -> &[SelectedModel] {
        &self.models
    }

    /// The supplied domain package documents, by package identity.
    pub(crate) fn packages(&self) -> &qsl_semantics::model::intake::PackageInput {
        &self.packages
    }
}

/// A unit checked and linked: the in-process checked package and the
/// sources it was compiled from.
#[derive(Debug)]
pub struct CheckedUnit {
    package: Arc<pkg::CheckedPackage>,
    source: Source,
    sources: Vec<Source>,
    type_report: TypeReport,
    libraries: Vec<Source>,
    /// The library identity of each entry of `libraries`, in the same order.
    library_names: Vec<LibraryName>,
    /// The v2 bytes each resolved library was admitted from, emitted once
    /// during S4 for its import view and retained here by library identity.
    library_packages: Arc<BTreeMap<LibraryName, pkg::EmittedPackage>>,
    protocol_results: Vec<super::ProtocolResult>,
    protocol_failure: Option<ProtocolFailure>,
}

impl CheckedUnit {
    /// The shared checked package used by the evaluator's retained closure.
    pub(crate) fn shared_package(&self) -> &Arc<pkg::CheckedPackage> {
        &self.package
    }

    /// The S4 in-process package.
    pub fn package(&self) -> &pkg::CheckedPackage {
        self.package.as_ref()
    }

    /// Per-protocol results through S3, retained even when a sibling refused.
    pub fn protocol_results(&self) -> &[super::ProtocolResult] {
        &self.protocol_results
    }

    /// The unit's own source, exactly as S1 read it (FND-016: a caller
    /// resolving a locus over it reuses this rather than reading the same
    /// bytes again).
    pub fn source(&self) -> &Source {
        &self.source
    }

    /// The type evidence produced from this checked unit's source parse.
    /// The report carries no source identity; its spans resolve through
    /// [`Self::source`].
    pub fn type_report(&self) -> &TypeReport {
        &self.type_report
    }

    /// Every library the check actually resolved, transitively, each
    /// exactly as its own S1 read it. Never the full dependency input: a
    /// supplied library the unit's own imports never reached is not compiled
    /// and carries no source here.
    pub fn libraries(&self) -> &[Source] {
        &self.libraries
    }

    /// The retained v2 bytes of every resolved library, shared not copied.
    pub(crate) fn shared_library_packages(
        &self,
    ) -> &Arc<BTreeMap<LibraryName, pkg::EmittedPackage>> {
        &self.library_packages
    }

    /// The source `check` resolved for the library `identity`, if it
    /// resolved one.
    pub(crate) fn library_source(&self, identity: &LibraryName) -> Option<&Source> {
        self.library_names
            .iter()
            .position(|name| name == identity)
            .and_then(|index| self.libraries.get(index))
    }
}

/// Type evidence retained by S3 for one source unit.
#[derive(Clone, Debug, Default)]
pub struct TypeReport {
    declarations: Vec<TypeDeclaration>,
    requirements: Vec<TypeRequirement>,
    obligations: Vec<PendingObligation>,
}

impl TypeReport {
    /// Declarations owned by the checked unit, in ascending node order.
    pub fn declarations(&self) -> &[TypeDeclaration] {
        &self.declarations
    }

    /// Requirement records owned by the checked unit.
    pub fn requirements(&self) -> &[TypeRequirement] {
        &self.requirements
    }

    /// Claims retained for later analysis or replay.
    pub fn obligations(&self) -> &[PendingObligation] {
        &self.obligations
    }
}

/// One source-owned declaration and its checked value type, when present.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeDeclaration {
    node: quire_exact::NodeKey,
    kind: qsl_semantics::check::NodeTag,
    value_type: Option<quire_exact::NodeKey>,
}

impl TypeDeclaration {
    /// The declaration's structural node.
    pub fn node(&self) -> &quire_exact::NodeKey {
        &self.node
    }

    /// The checked graph's declaration kind.
    pub fn kind(&self) -> qsl_semantics::check::NodeTag {
        self.kind
    }

    /// The declaration's checked value type, if it has one.
    pub fn value_type(&self) -> Option<&quire_exact::NodeKey> {
        self.value_type.as_ref()
    }
}

/// A requirement occurrence retained with its checked record.
#[derive(Clone, Debug)]
pub struct TypeRequirement {
    occurrence: qsl_foundation::source::provenance::OccurrenceKey,
    record: qsl_semantics::check::RequirementRecord,
    span: Option<qsl_foundation::Span>,
}

impl TypeRequirement {
    /// The requirement's structural occurrence.
    pub fn occurrence(&self) -> &qsl_foundation::source::provenance::OccurrenceKey {
        &self.occurrence
    }

    /// The package's checked requirement record.
    pub fn record(&self) -> &qsl_semantics::check::RequirementRecord {
        &self.record
    }

    /// The requirement's source span, resolved through its CheckedUnit.
    pub fn span(&self) -> Option<qsl_foundation::Span> {
        self.span
    }
}

/// A checked claim that remains pending for analysis or replay.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingObligation {
    node: quire_exact::NodeKey,
    occurrence: qsl_foundation::source::provenance::OccurrenceKey,
    capability_kind: String,
    span: qsl_foundation::Span,
}

impl PendingObligation {
    /// The claim node.
    pub fn node(&self) -> &quire_exact::NodeKey {
        &self.node
    }

    /// The claim's checked occurrence.
    pub fn occurrence(&self) -> &qsl_foundation::source::provenance::OccurrenceKey {
        &self.occurrence
    }

    /// The claim form's capability kind.
    pub fn capability_kind(&self) -> &str {
        &self.capability_kind
    }

    /// The source span of this claim or protocol requirement.
    pub fn span(&self) -> qsl_foundation::Span {
        self.span
    }

    /// Every retained obligation is pending until analysis or replay.
    pub const fn state(&self) -> ObligationState {
        ObligationState::Pending
    }
}

/// The discharge state of a checked obligation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObligationState {
    /// The checked claim awaits analysis or replay.
    Pending,
}

fn type_report(graph: &qsl_semantics::check::CheckedGraph, source: &RawSourceRef) -> TypeReport {
    use qsl_semantics::check::Owner;

    let owner = qsl_semantics::check::SourceOwner::from(source);
    let owns = |node: &qsl_semantics::check::SemanticNode| {
        node.owner() == Some(&Owner::Source(owner.clone()))
    };
    let declarations = graph
        .semantic_graph()
        .nodes()
        .filter(|node| owns(node) && node.declaration().is_some())
        .map(|node| TypeDeclaration {
            node: node.key(),
            kind: node.node_tag(),
            value_type: node.semantic_type(),
        })
        .collect();
    let requirements = graph
        .requirements()
        .iter()
        .filter(|(occurrence, _)| {
            graph
                .semantic_graph()
                .node(occurrence.node().clone())
                .is_some_and(owns)
        })
        .map(|(occurrence, record)| {
            let span = graph
                .occurrence(occurrence.node().clone(), occurrence.origin())
                .and_then(|location| graph.region(location))
                .and_then(|region| {
                    Some(qsl_foundation::Span {
                        start: usize::try_from(region.start()).ok()?,
                        end: usize::try_from(region.end()).ok()?,
                    })
                });
            TypeRequirement {
                occurrence: occurrence.clone(),
                record: record.clone(),
                span,
            }
        })
        .collect();
    let mut obligations: Vec<_> = graph
        .requirements()
        .iter()
        .filter_map(|(occurrence, record)| {
            let node = graph.semantic_graph().node(occurrence.node().clone())?;
            if !owns(node) {
                return None;
            }
            let location = graph.occurrence(occurrence.node().clone(), occurrence.origin())?;
            let region = graph.region(location)?;
            Some(PendingObligation {
                node: occurrence.node().clone(),
                occurrence: occurrence.clone(),
                capability_kind: record.requirements().kind().to_wire().to_owned(),
                span: qsl_foundation::Span {
                    start: usize::try_from(region.start()).ok()?,
                    end: usize::try_from(region.end()).ok()?,
                },
            })
        })
        .collect();
    for protocol in graph.protocols() {
        for requirement in &protocol.requirements {
            let Some(temporal) = graph.temporal_clauses().get(requirement.temporal) else {
                continue;
            };
            let key = temporal.identity();
            let Some(node) = graph.semantic_graph().node(key.clone()) else {
                continue;
            };
            if !owns(node) {
                continue;
            }
            let occurrence = qsl_foundation::source::provenance::OccurrenceKey::new(
                key.clone(),
                temporal.claim().clone(),
            );
            obligations.push(PendingObligation {
                node: key,
                occurrence,
                capability_kind: qsl_semantics::check::Capability::TemporalSatisfaction
                    .to_wire()
                    .to_owned(),
                span: requirement.span,
            });
        }
    }
    obligations.sort_by(|left, right| {
        (&left.node, left.span, &left.capability_kind).cmp(&(
            &right.node,
            right.span,
            &right.capability_kind,
        ))
    });
    TypeReport {
        declarations,
        requirements,
        obligations,
    }
}

/// A checked package emitted as `quire.checked-package/v2` bytes, with the
/// sources another party needs to import or replay it.
#[derive(Clone, Debug)]
pub struct EmittedUnit {
    package: pkg::EmittedPackage,
    sources: Vec<Source>,
    evidence: quire_contract_model::CheckedPackageEvidence,
}

impl EmittedUnit {
    /// The v2 wire bytes for the compiled package.
    pub fn package(&self) -> &pkg::EmittedPackage {
        &self.package
    }

    /// The source provision: the unit's source and every resolved library's,
    /// each by its caller-named reference ([`Source::reference`]).
    pub fn sources(&self) -> &[Source] {
        &self.sources
    }

    /// The evidence Contract IR's reader needs to read [`Self::package`]'s
    /// bytes back.
    pub fn evidence(&self) -> &quire_contract_model::CheckedPackageEvidence {
        &self.evidence
    }
}

/// S1 and S2: read `request`'s bytes into syntax (FR-278). Refuses
/// `invalid_syntax` for a source the parser recovered from, and with S2's
/// own refusal for a unit no form represents.
pub fn parse(
    request: &ParseRequest<'_>,
    limits: SourceLimits,
    cancel: &Cancel,
) -> Result<Staged<ParsedUnitSource>, FrontEndFailure> {
    parse_operation(request, limits, cancel, false)
}

/// S1 and S2 for the compiler's continuation path: an attributable protocol
/// refusal stays in the parsed value so clean protocols can continue.
pub(crate) fn parse_for_continuation(
    request: &ParseRequest<'_>,
    limits: SourceLimits,
    cancel: &Cancel,
) -> Result<Staged<ParsedUnitSource>, FrontEndFailure> {
    parse_operation(request, limits, cancel, true)
}

fn parse_operation(
    request: &ParseRequest<'_>,
    limits: SourceLimits,
    cancel: &Cancel,
    continue_protocol_refusals: bool,
) -> Result<Staged<ParsedUnitSource>, FrontEndFailure> {
    let source_len =
        |identity: &SourceIdentity| (identity == request.source).then_some(request.bytes.len());
    stage(cancel, &source_len, |work| {
        let parsed = charged(cancel, &mut work.s1, || {
            parse_unit(
                request.source.clone(),
                request.path,
                request.bytes,
                limits,
                cancel,
            )
        })?;
        if !continue_protocol_refusals
            && parsed.protocol_results.iter().any(|result| {
                matches!(
                    result,
                    super::ProtocolResult::Refused { .. }
                        | super::ProtocolResult::Incomplete { .. }
                )
            })
        {
            let refusal = parsed
                .syntax
                .diagnostics()
                .first()
                .map(|diagnostic| Box::new(CompileRefusal::Source(Box::new(diagnostic.clone()))))
                .or_else(|| {
                    parsed.protocol_forms_refusal.clone().map(|failure| {
                        Box::new(CompileRefusal::Forms {
                            region: failure
                                .span
                                .and_then(|span| region(parsed.source().reference(), span)),
                            failure,
                        })
                    })
                })
                .ok_or_else(|| {
                    Box::new(CompileRefusal::Check {
                        refusals: Vec::new(),
                        region: None,
                    })
                })?;
            return Err(StageError::with_protocol_results(
                refusal,
                parsed.protocol_results.clone(),
            ));
        }
        Ok(parsed)
    })
}

/// I1: admit each domain package `parsed`'s `model` declarations select,
/// from `packages`, the supplied documents by package identity
/// (FR-056, FR-278). Refuses at the first `model` declaration that does not
/// admit.
pub fn select(
    parsed: &ParsedUnitSource,
    packages: &qsl_semantics::model::intake::PackageInput,
    limits: ModelNormalizationLimits,
    cancel: &Cancel,
) -> Result<Staged<AdmittedModels>, FrontEndFailure> {
    stage(cancel, &|_| None, |work| {
        let models = charged(cancel, &mut work.i1, || {
            select_models(parsed, packages, limits, cancel)
        })?;
        Ok::<_, Box<CompileRefusal>>(AdmittedModels {
            models,
            packages: packages.clone(),
        })
    })
}

/// I1 for an admitted source-unit list: admit every unit's model selections
/// together against `packages`, preserving all aliases and normalizing the
/// aggregate once. `source_limits` bounds model clause parsing after
/// admission; callers compose units only after choosing the limits for that
/// aggregate operation.
pub fn select_units(
    admitted: &AdmittedUnitList,
    packages: &qsl_semantics::model::intake::PackageInput,
    limits: ModelNormalizationLimits,
    source_limits: SourceLimits,
    cancel: &Cancel,
) -> Result<Staged<AdmittedModels>, FrontEndFailure> {
    let source_len = |identity: &SourceIdentity| {
        admitted
            .units()
            .iter()
            .find(|unit| unit.source().identity() == identity)
            .map(|unit| unit.source().text().len())
    };
    stage(cancel, &source_len, |work| {
        let selections: Vec<_> = admitted
            .units()
            .iter()
            .flat_map(|unit| unit.unit().selections().models.iter().cloned())
            .collect();
        let models = charged(cancel, &mut work.i1, || {
            admit_unit_with_cancel(&selections, packages, limits, cancel).map_err(|refusal| {
                let source = admitted
                    .units()
                    .iter()
                    .find(|unit| {
                        unit.unit().selections().models.iter().any(|selection| {
                            selection.alias == refusal.alias
                                && selection.identity == refusal.identity
                                && selection.span == refusal.span
                        })
                    })
                    .map(|unit| unit.source().reference());
                Box::new(CompileRefusal::Intake {
                    region: source.and_then(|source| region(source, refusal.span)),
                    refusal,
                })
            })
        })?;
        let models = parse_type_invariants(
            parse_operation_clauses(models, source_limits)?,
            source_limits,
        )?;
        Ok::<_, Box<CompileRefusal>>(AdmittedModels {
            models,
            packages: packages.clone(),
        })
    })
}

/// S3 and S4: check and link `parsed` against `models` and the dependency
/// input (ADR-015 D-1, FR-099, FR-278). The S4 source resolution compiles
/// each library the unit's `import`s reach, against the same package input,
/// dependency input and `limits`.
pub fn check(
    parsed: &ParsedUnitSource,
    package_name: String,
    models: &AdmittedModels,
    dependencies: &DependencyInput,
    limits: SpineLimits,
    cancel: &Cancel,
) -> Result<Staged<CheckedUnit>, FrontEndFailure> {
    check_with_refinement_internals(
        parsed,
        package_name,
        None,
        models,
        dependencies,
        limits,
        cancel,
    )
}

/// S3 and S4: assemble an admitted source-unit list as one package and
/// return one source-bound checked unit per canonical source. Every returned
/// unit shares the same checked package; its report and `source()` are bound
/// to its own parse. Imports are resolved over the combined list.
pub fn check_units(
    admitted: &AdmittedUnitList,
    package_name: String,
    models: &AdmittedModels,
    dependencies: &DependencyInput,
    limits: SpineLimits,
    cancel: &Cancel,
) -> Result<Staged<Vec<CheckedUnit>>, FrontEndFailure> {
    let source_len = |identity: &SourceIdentity| {
        admitted
            .units()
            .iter()
            .find(|unit| unit.source().identity() == identity)
            .map(|unit| unit.source().text().len())
            .or_else(|| {
                dependencies
                    .libraries
                    .values()
                    .find(|library| &library.source == identity)
                    .map(|library| library.bytes.len())
            })
    };
    stage(cancel, &source_len, |work| {
        for unit in admitted.units() {
            dependencies
                .check_unit_owner(unit.source().identity())
                .map_err(|refusal| Box::new(CompileRefusal::DependencyInput(refusal)))?;
        }
        let first = admitted
            .units()
            .first()
            .expect("an AdmittedUnitList is non-empty");
        let mut resolution = Resolution {
            dependencies,
            packages: &models.packages,
            continuation_source: first.source().reference().clone(),
            limits,
            cancel,
            temporal_selections: &[],
            active: Vec::new(),
            compiled: BTreeMap::new(),
            import_edges: 0,
            source_bytes: 0,
            admitted: AdmittedPackages::default(),
            work: StageWork::default(),
        };
        let package = resolution.check_unit_list(admitted, package_name, models.models.clone())?;
        resolution.check_unimported_refinement_values()?;
        let package = Arc::new(package);
        let sources: Vec<_> = admitted
            .units()
            .iter()
            .map(|unit| unit.source().clone())
            .collect();
        let mut library_packages = BTreeMap::new();
        let (library_names, libraries): (Vec<_>, Vec<_>) = resolution
            .compiled
            .into_iter()
            .map(|(name, library)| {
                library_packages.insert(name.clone(), library.emitted.clone());
                (name, library.source.clone())
            })
            .unzip();
        let mut checked_units = Vec::with_capacity(sources.len());
        for (parsed, source) in admitted.units().iter().zip(sources.iter().cloned()) {
            let (protocol_results, protocol_failure) =
                protocol_check_outcome(parsed, package.graph());
            if let Some(refusal) = protocol_failure.as_ref() {
                return Err(StageError::with_protocol_results(
                    Box::new(refusal.to_refusal()),
                    protocol_results,
                ));
            }
            checked_units.push(CheckedUnit {
                package: Arc::clone(&package),
                type_report: type_report(package.graph(), source.reference()),
                source,
                sources: sources.clone(),
                libraries: libraries.clone(),
                library_names: library_names.clone(),
                library_packages: Arc::new(library_packages.clone()),
                protocol_results,
                protocol_failure,
            });
        }
        *work = resolution.work;
        Ok(checked_units)
    })
}

/// S3 and S4 with the unit's unchecked internal-list value and each
/// dependency library's own value carried to its source owner (FR-147/FR-099).
pub fn check_with_refinement_internals(
    parsed: &ParsedUnitSource,
    package_name: String,
    refinement_internals: Option<serde_json::Value>,
    models: &AdmittedModels,
    dependencies: &DependencyInput,
    limits: SpineLimits,
    cancel: &Cancel,
) -> Result<Staged<CheckedUnit>, FrontEndFailure> {
    check_with_execution_points_and_refinement_internals(
        parsed,
        package_name,
        &BTreeMap::new(),
        refinement_internals,
        models,
        dependencies,
        limits,
        cancel,
    )
}

/// S3 and S4 with accepted execution points keyed by exact authored state
/// clause name (FR-104-AC-15). The map applies only to `parsed`, never to
/// independently compiled libraries.
pub fn check_with_execution_points(
    parsed: &ParsedUnitSource,
    package_name: String,
    execution_points: &super::StateClauseExecutionPoints,
    models: &AdmittedModels,
    dependencies: &DependencyInput,
    limits: SpineLimits,
    cancel: &Cancel,
) -> Result<Staged<CheckedUnit>, FrontEndFailure> {
    check_with_temporal_selections_and_execution_points(
        parsed,
        package_name,
        execution_points,
        &[],
        models,
        dependencies,
        limits,
        cancel,
    )
}

/// S3 and S4 with both the accepted execution points and unchecked
/// refinement-internal-list value carried to the unit owner (FR-147/FR-099).
#[allow(
    clippy::too_many_arguments,
    reason = "check inputs plus refinement internals"
)]
pub fn check_with_execution_points_and_refinement_internals(
    parsed: &ParsedUnitSource,
    package_name: String,
    execution_points: &super::StateClauseExecutionPoints,
    refinement_internals: Option<serde_json::Value>,
    models: &AdmittedModels,
    dependencies: &DependencyInput,
    limits: SpineLimits,
    cancel: &Cancel,
) -> Result<Staged<CheckedUnit>, FrontEndFailure> {
    check_with_context_and_temporal_selections(
        parsed,
        package_name,
        execution_points,
        refinement_internals,
        &[],
        models,
        dependencies,
        limits,
        cancel,
        false,
    )
}

/// S3 and S4 with explicit clock selections for configured temporal clauses.
/// Selections are partitioned by exact source reference before package emission.
#[allow(
    clippy::too_many_arguments,
    reason = "check inputs plus explicit selections"
)]
pub fn check_with_temporal_selections_and_execution_points(
    parsed: &ParsedUnitSource,
    package_name: String,
    execution_points: &super::StateClauseExecutionPoints,
    temporal_selections: &[qsl_package::TemporalClockSelection],
    models: &AdmittedModels,
    dependencies: &DependencyInput,
    limits: SpineLimits,
    cancel: &Cancel,
) -> Result<Staged<CheckedUnit>, FrontEndFailure> {
    check_with_context_and_temporal_selections(
        parsed,
        package_name,
        execution_points,
        None,
        temporal_selections,
        models,
        dependencies,
        limits,
        cancel,
        false,
    )
}

/// The per-unit inputs of [`check_for_continuation`].
pub(crate) struct ContinuationInputs<'a> {
    /// The package name the unit checks under.
    pub(crate) package_name: String,
    /// Accepted execution points keyed by authored state-clause name.
    pub(crate) execution_points: &'a super::StateClauseExecutionPoints,
    /// Explicit clock selections for configured temporal clauses.
    pub(crate) temporal_selections: &'a [qsl_package::TemporalClockSelection],
}

/// S3 continuation for the compiler: keep an attributable protocol refusal
/// in the checked graph so S4 can still emit clean siblings.
pub(crate) fn check_for_continuation(
    parsed: &ParsedUnitSource,
    inputs: ContinuationInputs<'_>,
    models: &AdmittedModels,
    dependencies: &DependencyInput,
    limits: SpineLimits,
    cancel: &Cancel,
) -> Result<Staged<CheckedUnit>, FrontEndFailure> {
    check_with_context_and_temporal_selections(
        parsed,
        inputs.package_name,
        inputs.execution_points,
        None,
        inputs.temporal_selections,
        models,
        dependencies,
        limits,
        cancel,
        true,
    )
}

#[allow(
    clippy::too_many_arguments,
    reason = "check inputs plus temporal selections and refinement internals"
)]
fn check_with_context_and_temporal_selections(
    parsed: &ParsedUnitSource,
    package_name: String,
    execution_points: &super::StateClauseExecutionPoints,
    refinement_internals: Option<serde_json::Value>,
    temporal_selections: &[qsl_package::TemporalClockSelection],
    models: &AdmittedModels,
    dependencies: &DependencyInput,
    limits: SpineLimits,
    cancel: &Cancel,
    continue_protocol_refusals: bool,
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
            continuation_source: parsed.source().reference().clone(),
            limits,
            cancel,
            temporal_selections,
            active: Vec::new(),
            compiled: BTreeMap::new(),
            import_edges: 0,
            source_bytes: 0,
            admitted: AdmittedPackages::default(),
            work: StageWork::default(),
        };
        let package = match resolution.check_unit(
            parsed,
            package_name.clone(),
            models.models.clone(),
            execution_points.clone(),
            refinement_internals,
        ) {
            Ok(package) => package,
            Err(refusal) => {
                return Err(attach_protocol_results(parsed, refusal, &source_len));
            }
        };
        if let Err(refusal) = resolution.check_unimported_refinement_values() {
            return Err(attach_protocol_results(parsed, refusal, &source_len));
        }
        *work = resolution.work;
        let mut library_packages = BTreeMap::new();
        let (library_names, libraries) = resolution
            .compiled
            .into_iter()
            .map(|(name, library)| {
                library_packages.insert(name.clone(), library.emitted.clone());
                (name, library.source.clone())
            })
            .unzip();
        let (protocol_results, protocol_failure) = protocol_check_outcome(parsed, package.graph());
        if !continue_protocol_refusals {
            if let Some(refusal) = protocol_failure.as_ref() {
                return Err(StageError::with_protocol_results(
                    Box::new(refusal.to_refusal()),
                    protocol_results,
                ));
            }
        }
        let package = Arc::new(package);
        let type_report = type_report(package.graph(), parsed.source().reference());
        let checked = CheckedUnit {
            package,
            source: parsed.source().clone(),
            sources: vec![parsed.source().clone()],
            type_report,
            libraries,
            library_names,
            library_packages: Arc::new(library_packages),
            protocol_results,
            protocol_failure,
        };
        Ok(checked)
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
    package_with_temporal_selections(checked, &[], limits, cancel)
}

/// E4 with the explicit clock configuration for each configured temporal
/// declaration, keyed by its checked source and exact locus.
pub fn package_with_temporal_selections(
    checked: &CheckedUnit,
    selections: &[qsl_package::TemporalClockSelection],
    limits: PackageLimits,
    cancel: &Cancel,
) -> Result<Staged<EmittedUnit>, FrontEndFailure> {
    let PackageLimits {} = limits;
    stage(
        cancel,
        &|_| None,
        |work| -> Result<EmittedUnit, StageError> {
            for selection in selections {
                let source_is_compiled = selection.source == *checked.source.reference()
                    || checked
                        .libraries
                        .iter()
                        .any(|source| selection.source == *source.reference());
                if !source_is_compiled {
                    return Err(Box::new(CompileRefusal::Emit(
                        qsl_package::EmitRefusal::SurplusTemporalClockSelection {
                            source_ref: selection.source.clone(),
                            span: selection.declaration_span,
                        },
                    ))
                    .into());
                }
            }
            let own: Vec<_> = selections
                .iter()
                .filter(|selection| {
                    checked
                        .sources
                        .iter()
                        .any(|source| source.reference() == &selection.source)
                })
                .cloned()
                .collect();
            let protocol_emission = charged(cancel, &mut work.e4, || {
                qsl_package::emit_checked_with_protocol_results_and_temporal_selections_and_cancel(
                    &checked.package,
                    &own,
                    cancel,
                )
            })
            .map_err(|refusal| Box::new(CompileRefusal::Emit(refusal)))?;
            let results = merge_emission_results(
                &checked.protocol_results,
                lifecycle_emission_results(&protocol_emission, &checked.source),
            );
            let emission = match protocol_emission.aggregate() {
                qsl_package::ProtocolEmissionAggregate::Emitted => {
                    if checked.protocol_failure.is_some()
                        || results
                            .iter()
                            .any(|result| matches!(result, super::ProtocolResult::Refused { .. }))
                    {
                        let refusal = checked.protocol_failure.as_ref().map_or_else(
                            || {
                                Box::new(CompileRefusal::Check {
                                    refusals: Vec::new(),
                                    region: None,
                                })
                            },
                            |failure| Box::new(failure.to_refusal()),
                        );
                        return Err(StageError::with_protocol_results(refusal, results));
                    }
                    protocol_emission.emission().ok_or_else(|| {
                        Box::new(CompileRefusal::Emit(
                            qsl_package::EmitRefusal::NothingToEmit {
                                omitted: Vec::new(),
                            },
                        ))
                    })?
                }
                qsl_package::ProtocolEmissionAggregate::Refused { primary } => {
                    let refusal = checked.protocol_failure.as_ref().map_or_else(
                        || {
                            Box::new(CompileRefusal::Check {
                                refusals: vec![primary.clone()],
                                region: None,
                            })
                        },
                        |failure| Box::new(failure.to_refusal()),
                    );
                    return Err(StageError::with_protocol_results(refusal, results));
                }
                qsl_package::ProtocolEmissionAggregate::Incomplete { primary } => {
                    let limit =
                        LimitExceeded::new(primary.kind.setting(), primary.limit, primary.actual)
                            .at(primary.region.clone().map(Locus::Region));
                    return Err(StageError::with_protocol_results(
                        Box::new(CompileRefusal::Limit(limit)),
                        results,
                    ));
                }
            };
            let sources = checked
                .sources
                .iter()
                .chain(&checked.libraries)
                .cloned()
                .collect();
            Ok(EmittedUnit {
                package: emission.package().clone(),
                sources,
                evidence: emission.evidence().clone(),
            })
        },
    )
}

fn merge_emission_results(
    checked: &[super::ProtocolResult],
    emitted: Vec<super::ProtocolResult>,
) -> Vec<super::ProtocolResult> {
    use super::ProtocolResult;
    let mut results = checked.to_vec();
    for emission in emitted {
        let declaration = match &emission {
            ProtocolResult::Cleared { declaration, .. }
            | ProtocolResult::Emitted { declaration, .. }
            | ProtocolResult::Refused { declaration, .. }
            | ProtocolResult::Incomplete { declaration, .. } => declaration,
        };
        if let Some(existing) = results.iter_mut().find(|existing| match existing {
            ProtocolResult::Cleared {
                declaration: name, ..
            }
            | ProtocolResult::Emitted {
                declaration: name, ..
            }
            | ProtocolResult::Refused {
                declaration: name, ..
            }
            | ProtocolResult::Incomplete {
                declaration: name, ..
            } => name == declaration,
        }) {
            if !matches!(existing, ProtocolResult::Refused { .. }) {
                *existing = emission;
            }
        } else {
            results.push(emission);
        }
    }
    results
}

fn lifecycle_emission_results(
    emission: &qsl_package::ProtocolEmission,
    source: &Source,
) -> Vec<super::ProtocolResult> {
    emission
        .results()
        .iter()
        .map(|result| match result {
            qsl_package::ProtocolEmissionResult::Emitted {
                declaration,
                clause_key,
                nodes,
            } => super::ProtocolResult::Emitted {
                declaration: declaration.clone(),
                clause_key: format!("{clause_key:?}"),
                nodes: nodes.clone(),
            },
            qsl_package::ProtocolEmissionResult::Refused {
                declaration,
                source_index,
                declaration_span,
                refusal,
            } => super::ProtocolResult::Refused {
                declaration: declaration.clone(),
                source_index: *source_index,
                declaration_span: *declaration_span,
                code: refusal.cause.code(),
                cause: refusal
                    .cause
                    .catalog_code()
                    .map(|code| code.cause().to_owned())
                    .unwrap_or_else(|| "unknown-cause".to_owned()),
                locus: region(source.reference(), *declaration_span).map(Locus::Region),
            },
            qsl_package::ProtocolEmissionResult::Incomplete { declaration, limit } => {
                super::ProtocolResult::Incomplete {
                    declaration: declaration.clone(),
                    limit: LimitExceeded::new(limit.kind.setting(), limit.limit, limit.actual)
                        .at(limit.region.clone().map(Locus::Region)),
                }
            }
        })
        .collect()
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
    /// The caller's supplied-value helper ceilings (FR-277).
    pub supplied: quire_semantic_value::supplied_stop::SuppliedLimits,
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
    // One run for the whole invocation: both helper budgets and the
    // caller's original handle, never a fresh one (FR-277).
    let mut run =
        quire_semantic_value::supplied_stop::SuppliedRun::new(request.supplied, cancel.clone())
            .map_err(CallFailure::Supplied)?;
    let evaluation = package.package.call_supplied(
        request.function,
        request.arguments.to_vec(),
        request.objects,
        &mut meter,
        &mut run,
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
) -> Result<ParsedUnitSource, StageError> {
    let parsed = qsl_cst::parse_with_cancel(source, path, bytes, limits, cancel)
        .map_err(|diagnostic| Box::new(CompileRefusal::Source(diagnostic)))?;
    let raw = parsed.source().reference().clone();
    let (unit, forms_refusals) = match qsl_forms::build_unit_with_protocol_results(&parsed) {
        Ok(built) => built,
        Err(failure) => {
            if let Some(diagnostic) = parsed.diagnostics().first() {
                let refusal = Box::new(CompileRefusal::Source(Box::new(diagnostic.clone())));
                if let Some(results) =
                    bounded_protocol_results_around_nonprotocol_diagnostic(&parsed, diagnostic)
                {
                    return Err(StageError::with_protocol_results(refusal, results));
                }
                return Err(refusal.into());
            }
            return Err(Box::new(CompileRefusal::Forms {
                region: failure.span.and_then(|span| region(&raw, span)),
                failure,
            })
            .into());
        }
    };
    let protocol_results = syntax_protocol_results(&parsed, parsed.diagnostics().first())
        .or_else(|| forms_protocol_results(&parsed, &forms_refusals))
        .unwrap_or_default();
    let protocol_forms_refusal = forms_refusals
        .first()
        .map(|failure| failure.refusal.clone());
    Ok(ParsedUnitSource {
        syntax: parsed,
        unit,
        protocol_results,
        protocol_forms_refusal,
    })
}

/// Clean, brace-bounded protocol declarations on both sides of a
/// non-protocol S1/S2 diagnostic have reached forms independently, even
/// when the unit-level forms builder must refuse the containing source.
/// Header errors, diagnostics inside a protocol, and unbounded EOF failures
/// remain payload-free.
fn bounded_protocol_results_around_nonprotocol_diagnostic(
    parsed: &qsl_cst::ParsedSource,
    diagnostic: &qsl_cst::CompleteDiagnostic,
) -> Option<Vec<super::ProtocolResult>> {
    use qsl_cst::TokenClass;

    let diagnostic_span = diagnostic.byte_span()?;
    if diagnostic_span.start >= parsed.source().text().len() {
        return None;
    }
    let protocols = bounded_protocol_clause_declarations(parsed)?;
    if protocols.is_empty()
        || protocols
            .iter()
            .any(|(_, span)| span.start < diagnostic_span.start && diagnostic_span.start < span.end)
    {
        return None;
    }

    let nonprotocol_declaration_heads = parsed.cst().tokens().iter().any(|token| {
        token.class() == TokenClass::Token
            && token.span().start <= diagnostic_span.start
            && is_nonprotocol_declaration_head(token.spelling())
            && !protocols
                .iter()
                .any(|(_, span)| span.start <= token.span().start && token.span().start < span.end)
    });
    if !nonprotocol_declaration_heads {
        return None;
    }

    Some(
        protocols
            .into_iter()
            .map(|(declaration, _)| super::ProtocolResult::Cleared {
                declaration,
                stage: super::SpineStage::Forms,
            })
            .collect(),
    )
}

/// Return only protocol declarations represented by actual top-level CST
/// nodes. A failed whole-unit parse has no declaration tree, so in that case
/// each declaration-shaped token range is parsed independently and accepted
/// only when the resulting CST contains a top-level `ProtocolClause`.
fn bounded_protocol_clause_declarations(
    parsed: &qsl_cst::ParsedSource,
) -> Option<Vec<(String, qsl_foundation::Span)>> {
    use qsl_cst::Production;

    let source_text = parsed.source().text();
    let cst = parsed.cst();
    let clauses: Vec<_> = cst
        .nodes()
        .iter()
        .filter(|node| node.production() == Production::ProtocolClause)
        .collect();
    if !clauses.is_empty() {
        return clauses
            .into_iter()
            .map(|clause| protocol_clause_declaration(cst, source_text, clause.span()))
            .collect();
    }

    use qsl_cst::TokenClass;
    let tokens: Vec<_> = cst
        .tokens()
        .iter()
        .filter(|token| token.class() == TokenClass::Token)
        .collect();
    let starts: Vec<usize> = (0..tokens.len())
        .filter(|index| is_protocol_declaration_start(&tokens, *index))
        .collect();
    let mut declarations = Vec::with_capacity(starts.len());
    for (ordinal, start) in starts.iter().copied().enumerate() {
        let boundary = starts.get(ordinal + 1).copied().unwrap_or(tokens.len());
        let mut depth = 0_usize;
        let mut declaration_end = None;
        for token in tokens.get(start..boundary)? {
            match token.spelling() {
                b"{" => depth = depth.checked_add(1)?,
                b"}" => {
                    depth = depth.checked_sub(1)?;
                    if depth == 0 {
                        declaration_end = Some(token.span().end);
                        break;
                    }
                }
                _ => {}
            }
        }
        let end = declaration_end?;
        let start_byte = tokens[start].span().start;
        let text = source_text.get(start_byte..end)?;
        let header = "language \"ix:native\" edition \"1-draft\";\n";
        let isolated_text = format!("{header}{text}");
        let isolated = qsl_cst::parse_with_cancel(
            parsed.source().identity().clone(),
            parsed.source().path(),
            isolated_text.as_bytes(),
            parsed.effective_limits(),
            &Cancel::new(),
        )
        .ok()?;
        if !isolated.is_admissible() {
            return None;
        }
        let clause = isolated
            .cst()
            .nodes()
            .iter()
            .find(|node| node.production() == Production::ProtocolClause)?;
        let name_span = clause.span();
        let declaration_name = isolated
            .source()
            .text()
            .get(name_span.start..name_span.end)?
            .strip_prefix("protocol")?
            .trim_start()
            .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
            .next()?
            .to_owned();
        let declaration = isolated.cst().nodes().iter().find(|node| {
            node.production() == Production::Declaration
                && node.span().start <= name_span.start
                && name_span.end <= node.span().end
        })?;
        let local_start = declaration.span().start.checked_sub(header.len())?;
        let local_end = declaration.span().end.checked_sub(header.len())?;
        declarations.push((
            declaration_name,
            qsl_foundation::Span {
                start: start_byte.checked_add(local_start)?,
                end: start_byte.checked_add(local_end)?,
            },
        ));
    }
    Some(declarations)
}

fn protocol_clause_declaration(
    cst: &qsl_cst::LosslessCst,
    source_text: &str,
    clause_span: qsl_foundation::Span,
) -> Option<(String, qsl_foundation::Span)> {
    use qsl_cst::Production;
    let declaration = cst.nodes().iter().find(|node| {
        node.production() == Production::Declaration
            && node.span().start <= clause_span.start
            && clause_span.end <= node.span().end
    })?;
    let name = source_text
        .get(clause_span.start..clause_span.end)?
        .strip_prefix("protocol")?
        .trim_start()
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .next()?
        .to_owned();
    Some((name, declaration.span()))
}

fn is_protocol_declaration_start(tokens: &[&qsl_cst::CstToken], index: usize) -> bool {
    use qsl_cst::TokenKind;
    let spelling_is = |offset: usize, spelling: &[u8]| {
        tokens
            .get(index + offset)
            .is_some_and(|token| token.kind() == TokenKind::Grammar && token.spelling() == spelling)
    };
    tokens
        .get(index)
        .is_some_and(|token| token.kind() == TokenKind::Grammar && token.spelling() == b"protocol")
        && tokens
            .get(index + 1)
            .is_some_and(|token| token.kind() == TokenKind::Identifier)
        && spelling_is(2, b"using")
        && tokens
            .get(index + 3)
            .is_some_and(|token| token.kind() == TokenKind::Identifier)
        && spelling_is(4, b"over")
}

fn is_nonprotocol_declaration_head(spelling: &[u8]) -> bool {
    matches!(
        spelling,
        b"dimension"
            | b"unit"
            | b"enum"
            | b"record"
            | b"tuple"
            | b"union"
            | b"type"
            | b"function"
            | b"predicate"
            | b"invariant"
            | b"constraint"
            | b"pre"
            | b"post"
            | b"body"
            | b"bound"
            | b"temporal"
            | b"terminal"
            | b"time"
            | b"urgent"
            | b"taskset"
            | b"schedulable"
            | b"response"
            | b"release"
            | b"random"
            | b"reward"
            | b"delay"
            | b"workload"
            | b"probabilistic"
            | b"possible"
            | b"always"
            | b"unique"
            | b"refinement"
            | b"relation"
            | b"hyper"
            | b"hybrid"
            | b"synthesis"
            | b"verify"
            | b"abstraction"
    )
}

fn forms_protocol_results(
    parsed: &qsl_cst::ParsedSource,
    refusals: &[qsl_forms::ProtocolFormsRefusal],
) -> Option<Vec<super::ProtocolResult>> {
    use qsl_cst::Production;
    let clauses: Vec<_> = parsed
        .cst()
        .nodes()
        .iter()
        .filter(|node| node.production() == Production::ProtocolClause)
        .collect();
    if clauses.is_empty() {
        return None;
    }
    clauses
        .into_iter()
        .enumerate()
        .map(|(source_index, node)| {
            let span = node.span();
            let declaration = parsed
                .source()
                .text()
                .get(span.start..span.end)?
                .strip_prefix("protocol")?
                .trim_start()
                .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
                .next()?
                .to_owned();
            let result = if let Some(failure) = refusals
                .iter()
                .find(|failure| failure.declaration == declaration)
            {
                super::ProtocolResult::Refused {
                    declaration,
                    source_index,
                    declaration_span: failure.declaration_span,
                    code: failure.refusal.catalog_code(),
                    cause: format!("{:?}", failure.refusal.cause),
                    locus: region(parsed.source().reference(), failure.declaration_span)
                        .map(Locus::Region),
                }
            } else {
                super::ProtocolResult::Cleared {
                    declaration,
                    stage: super::SpineStage::Forms,
                }
            };
            Some(result)
        })
        .collect()
}

fn protocol_check_outcome(
    parsed: &ParsedUnitSource,
    graph: &qsl_semantics::check::CheckedGraph,
) -> (Vec<super::ProtocolResult>, Option<ProtocolFailure>) {
    use qsl_cst::Production;
    let declarations: Vec<_> = parsed
        .syntax
        .cst()
        .nodes()
        .iter()
        .filter(|node| node.production() == Production::ProtocolClause)
        .filter_map(|node| {
            let span = node.span();
            let declaration = parsed
                .source()
                .text()
                .get(span.start..span.end)?
                .strip_prefix("protocol")?
                .trim_start()
                .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
                .next()?
                .to_owned();
            Some((declaration, span))
        })
        .collect();
    let mut results = Vec::with_capacity(declarations.len());
    let mut first_failure = None;
    for (declaration, declaration_span) in declarations {
        if let Some(existing) = parsed.protocol_results.iter().find(|result| {
            matches!(result, super::ProtocolResult::Refused { declaration: name, .. } if name == &declaration)
        }) {
            results.push(existing.clone());
            if first_failure.is_none() {
                first_failure = protocol_failure_from_parsed(parsed);
            }
            continue;
        }
        if let Some(refused) = graph
            .protocol_refusals()
            .iter()
            .find(|refusal| refusal.declaration == declaration)
        {
            let refusal = refused.refusals.first().cloned();
            if let Some(primary) = refusal.clone() {
                if first_failure.is_none() {
                    first_failure = Some(CompileRefusal::Check {
                        region: region(parsed.source().reference(), declaration_span),
                        refusals: refused.refusals.clone(),
                    });
                }
                if let Some(limit) =
                    refused
                        .refusals
                        .iter()
                        .find_map(|refusal| match refusal.cause.unwrapped() {
                            CheckCause::ResourceExhausted(cause) => Some(
                                LimitExceeded::new(cause.kind.setting(), cause.limit, cause.actual)
                                    .at(region(parsed.source().reference(), declaration_span)
                                        .map(Locus::Region)),
                            ),
                            _ => None,
                        })
                {
                    results.push(super::ProtocolResult::Incomplete { declaration, limit });
                    continue;
                }
                let catalog = primary.cause.catalog_code();
                results.push(super::ProtocolResult::Refused {
                    declaration,
                    source_index: refused.source_index,
                    declaration_span: refused.declaration_span,
                    code: primary.cause.code(),
                    cause: catalog
                        .map(|code| code.cause().to_owned())
                        .unwrap_or_else(|| "unknown-cause".to_owned()),
                    locus: region(parsed.source().reference(), refused.declaration_span)
                        .map(Locus::Region),
                });
            } else {
                results.push(super::ProtocolResult::Refused {
                    declaration,
                    source_index: refused.source_index,
                    declaration_span: refused.declaration_span,
                    code: Code::RuntimeInvariant,
                    cause: "protocol-refusal-without-cause".to_owned(),
                    locus: None,
                });
            }
        } else {
            results.push(super::ProtocolResult::Cleared {
                declaration,
                stage: super::SpineStage::Check,
            });
        }
    }
    if first_failure.is_none() && !graph.unit_refusals().is_empty() {
        first_failure = Some(CompileRefusal::Check {
            refusals: graph.unit_refusals().to_vec(),
            region: None,
        });
    }
    if first_failure.is_none() {
        first_failure = protocol_failure_from_parsed(parsed);
    }
    (results, first_failure.map(ProtocolFailure::from_refusal))
}

fn attach_protocol_results(
    parsed: &ParsedUnitSource,
    refusal: Box<CompileRefusal>,
    source_len: &dyn Fn(&SourceIdentity) -> Option<usize>,
) -> StageError {
    use qsl_cst::Production;
    let limit = limit_of(&refusal, source_len);
    let results = parsed
        .syntax
        .cst()
        .nodes()
        .iter()
        .filter(|node| node.production() == Production::ProtocolClause)
        .filter_map(|node| {
            let declaration_span = node.span();
            let declaration = parsed
                .source()
                .text()
                .get(declaration_span.start..declaration_span.end)?
                .strip_prefix("protocol")?
                .trim_start()
                .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
                .next()?
                .to_owned();
            let existing = parsed.protocol_results.iter().find(|result| match result {
                super::ProtocolResult::Cleared {
                    declaration: name, ..
                }
                | super::ProtocolResult::Emitted {
                    declaration: name, ..
                }
                | super::ProtocolResult::Refused {
                    declaration: name, ..
                }
                | super::ProtocolResult::Incomplete {
                    declaration: name, ..
                } => name == &declaration,
            });
            if let Some(super::ProtocolResult::Refused { .. }) = existing {
                return existing.cloned();
            }
            if let Some(limit) = &limit {
                return Some(super::ProtocolResult::Incomplete {
                    declaration,
                    limit: limit
                        .clone()
                        .at(region(parsed.source().reference(), declaration_span)
                            .map(Locus::Region)),
                });
            }
            Some(super::ProtocolResult::Cleared {
                declaration,
                stage: super::SpineStage::Check,
            })
        })
        .collect::<Vec<_>>();
    if results.is_empty() {
        StageError::from(refusal)
    } else {
        StageError::with_protocol_results(refusal, results)
    }
}

fn protocol_failure_from_parsed(parsed: &ParsedUnitSource) -> Option<CompileRefusal> {
    if let Some(diagnostic) = parsed.syntax.diagnostics().first() {
        return Some(CompileRefusal::Source(Box::new(diagnostic.clone())));
    }
    parsed
        .protocol_forms_refusal
        .clone()
        .map(|failure| CompileRefusal::Forms {
            region: failure
                .span
                .and_then(|span| region(parsed.source().reference(), span)),
            failure,
        })
}

/// The declarations delimited by S1 when one syntax diagnostic belongs to
/// exactly one protocol. Returns `None` for a unit-level error.
fn syntax_protocol_results(
    parsed: &qsl_cst::ParsedSource,
    diagnostic: Option<&qsl_cst::CompleteDiagnostic>,
) -> Option<Vec<super::ProtocolResult>> {
    use qsl_cst::Production;
    let diagnostic = diagnostic?;
    let diagnostic_span = diagnostic.byte_span()?;
    let clauses: Vec<_> = parsed
        .cst()
        .nodes()
        .iter()
        .filter(|node| node.production() == Production::ProtocolClause)
        .collect();
    let owner = clauses.iter().position(|node| {
        let span = node.span();
        span.start < diagnostic_span.start && diagnostic_span.start <= span.end
    })?;
    let source_text = parsed.source().text();
    let mut results = Vec::with_capacity(clauses.len());
    for (source_index, node) in clauses.iter().enumerate() {
        let span = node.span();
        let declaration = source_text
            .get(span.start..span.end)?
            .strip_prefix("protocol")?
            .trim_start()
            .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
            .next()?
            .to_owned();
        if source_index == owner {
            results.push(super::ProtocolResult::Refused {
                declaration,
                source_index,
                declaration_span: span,
                code: diagnostic.code,
                cause: format!("{:?}", diagnostic.cause),
                locus: region(parsed.source().reference(), span).map(Locus::Region),
            });
        } else {
            results.push(super::ProtocolResult::Cleared {
                declaration,
                stage: super::SpineStage::Forms,
            });
        }
    }
    Some(results)
}

/// I1 over one unit's `model` declarations.
fn select_models(
    parsed: &ParsedUnitSource,
    packages: &qsl_semantics::model::intake::PackageInput,
    limits: ModelNormalizationLimits,
    cancel: &Cancel,
) -> Result<SelectedModels, Box<CompileRefusal>> {
    let raw = parsed.source().reference();
    let models = admit_unit_with_cancel(&parsed.unit.selections().models, packages, limits, cancel)
        .map_err(|refusal| {
            Box::new(CompileRefusal::Intake {
                region: region(raw, refusal.span),
                refusal,
            })
        })?;
    let source_limits = parsed.syntax.effective_limits();
    parse_type_invariants(
        parse_operation_clauses(models, source_limits)?,
        source_limits,
    )
}

/// FR-366: parses the text of each type-level `Invariant` clause of `model`
/// with the unit clause production, and hands the parsed expressions to the
/// assembler on the model. The first clause that does not parse refuses,
/// located at its owner's `TypeDefinition.clauses[k]` member.
fn parse_type_invariants(
    models: SelectedModels,
    limits: SourceLimits,
) -> Result<SelectedModels, Box<CompileRefusal>> {
    models
        .with_parsed_type_invariants(|text| parse_clause_text(text, limits))
        .map_err(|failure| {
            Box::new(CompileRefusal::TypeInvariant {
                clause: failure.clause,
                member: failure.member,
                failure: failure.cause,
            })
        })
}

/// FR-103: parses each kept `quire`-language `requires` and `ensures` clause
/// of `model`'s operations with the unit clause production, and hands the
/// parsed expressions to the assembler on the model. The first clause that
/// does not parse refuses, located at the operation's `Operation.pre[k]` or
/// `Operation.post[k]` member.
fn parse_operation_clauses(
    models: SelectedModels,
    limits: SourceLimits,
) -> Result<SelectedModels, Box<CompileRefusal>> {
    models
        .with_parsed_operation_clauses(|text| parse_clause_text(text, limits))
        .map_err(|failure| {
            Box::new(CompileRefusal::OperationClause {
                operation: failure.operation,
                member: failure.member,
                failure: failure.cause,
            })
        })
}

/// E4 over one checked package, refusing a wire that would omit nodes: a
/// package missing part of the checked graph is partial output, which E4
/// never writes (ADR-011 §2.3).
fn emit_unit(
    package: &pkg::CheckedPackage,
    selections: &[qsl_package::TemporalClockSelection],
    cancel: &Cancel,
) -> Result<Emission, Box<CompileRefusal>> {
    let emission =
        qsl_package::emit_checked_with_temporal_selections_and_cancel(package, selections, cancel)
            .map_err(|refusal| Box::new(CompileRefusal::Emit(refusal)))?;
    if !emission.omitted().is_empty() {
        return Err(Box::new(CompileRefusal::Omitted(
            emission.omitted().to_vec(),
        )));
    }
    Ok(emission)
}

fn temporal_selections_for_source(
    selections: &[qsl_package::TemporalClockSelection],
    source: &RawSourceRef,
) -> Vec<qsl_package::TemporalClockSelection> {
    selections
        .iter()
        .filter(|selection| selection.source == *source)
        .cloned()
        .collect()
}

/// One unit through all four operations, over a handle nobody cancels: the
/// compile the spine's unit tests use to build their fixtures. Production
/// callers compose the operations themselves.
#[cfg(test)]
#[derive(Debug)]
pub(crate) struct ComposedUnit {
    /// The S4 in-process package.
    pub(crate) package: Arc<pkg::CheckedPackage>,
    /// The emitted wire bytes for the compiled package.
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
    packages: &qsl_semantics::model::intake::PackageInput,
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
        "fixture".to_owned(),
        &models,
        dependencies,
        limits,
        &cancel,
    )
    .map_err(refusal)?
    .into_value();
    let emitted = package(&checked, PackageLimits::default(), &cancel)
        .map_err(refusal)?
        .into_value();
    record_declarations(&checked.package);
    Ok(ComposedUnit {
        emitted: emitted.package().clone(),
        source: checked.source,
        package: checked.package,
    })
}

#[cfg(test)]
thread_local! {
    /// The `declaration` occurrence of each function of each composed
    /// package, by the callable's own node key: the key a `function` replay
    /// request carries for the predicate claim (FR-098).
    static DECLARATIONS: std::cell::RefCell<BTreeMap<quire_exact::NodeKey, qsl_foundation::source::provenance::OccurrenceKey>> =
        const { std::cell::RefCell::new(BTreeMap::new()) };
}

/// Remember the `declaration` occurrence of each function of `package`.
#[cfg(test)]
fn record_declarations(package: &pkg::CheckedPackage) {
    use qsl_foundation::source::provenance::OccurrenceKey;
    use quire_exact::{Origin, Role};
    let graph = package.graph();
    for state in (0..).map_while(|index| graph.function_state(index)) {
        if let Some(callable) = graph.callable(state.name) {
            let key = OccurrenceKey::new(
                callable.identity.clone(),
                Origin::new(Role::new("declaration"), 0),
            );
            DECLARATIONS.with(|declarations| {
                declarations
                    .borrow_mut()
                    .insert(callable.identity.clone(), key);
            });
        }
    }
}

/// The `declaration` occurrence of the callable `callable` in a package
/// [`compose`]d on this thread.
#[cfg(test)]
pub(crate) fn composed_declaration(
    callable: &quire_exact::NodeKey,
) -> Option<qsl_foundation::source::provenance::OccurrenceKey> {
    DECLARATIONS.with(|declarations| declarations.borrow().get(callable).cloned())
}

#[cfg(test)]
mod tests;

/// A library the S4 source resolution compiled: its checked package and
/// its import view.
#[derive(Debug)]
struct ResolvedLibrary {
    package: Arc<pkg::CheckedPackage>,
    view: ImportView,
    /// The v2 bytes S4 emitted for this library's import view.
    emitted: pkg::EmittedPackage,
    /// This library's own source, exactly as its own `qsl_cst::parse` read
    /// it (FND-016).
    source: Source,
}

/// One compile's S4 source resolution state (ADR-015 D-1).
struct Resolution<'a> {
    dependencies: &'a DependencyInput,
    packages: &'a qsl_semantics::model::intake::PackageInput,
    continuation_source: RawSourceRef,
    limits: SpineLimits,
    cancel: &'a Cancel,
    temporal_selections: &'a [qsl_package::TemporalClockSelection],
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
    /// The manifest's top-level package name for this unit.
    package_name: String,
    raw: RawSourceRef,
    unit: ParsedUnit,
    source_units: Vec<ParsedUnitSource>,
    models: SelectedModels,
    /// Accepted points supplied for the root unit. Library frames always
    /// carry an empty map because they compile independently.
    execution_points: super::StateClauseExecutionPoints,
    /// The unchecked refinement-internal-list value owned by this source.
    refinement_internals: Option<serde_json::Value>,
    /// The index of the import being resolved.
    next: usize,
    admitted: Vec<AdmittedImport>,
    links: Vec<Import>,
}

impl Frame {
    fn new(
        parsed: &ParsedUnitSource,
        package_name: String,
        models: SelectedModels,
        execution_points: super::StateClauseExecutionPoints,
        refinement_internals: Option<serde_json::Value>,
    ) -> Self {
        let imports = parsed.unit.selections().imports.len();
        Self {
            package_name,
            raw: parsed.source().reference().clone(),
            unit: parsed.unit.clone(),
            source_units: vec![parsed.clone()],
            models,
            execution_points,
            refinement_internals,
            next: 0,
            admitted: Vec::with_capacity(imports),
            links: Vec::with_capacity(imports),
        }
    }

    fn new_units(
        admitted: &AdmittedUnitList,
        package_name: String,
        models: SelectedModels,
    ) -> Self {
        let source_units = admitted.units.clone();
        let unit = ParsedUnit::combine(source_units.iter().map(|source| source.unit.clone()));
        let imports = unit.selections().imports.len();
        Self {
            package_name,
            raw: source_units[0].source().reference().clone(),
            unit,
            source_units,
            models,
            execution_points: super::StateClauseExecutionPoints::new(),
            refinement_internals: None,
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
    /// Compile supplied libraries with their own internal-list input even
    /// when the unit does not import them, so a malformed supplied value is
    /// located at its source owner and refuses the whole compile (FR-147-AC-10).
    fn check_unimported_refinement_values(&mut self) -> Result<(), Box<CompileRefusal>> {
        let pending: Vec<_> = self
            .dependencies
            .refinement_internals
            .keys()
            .filter(|identity| !self.compiled.contains_key(*identity))
            .cloned()
            .collect();
        for identity in pending {
            let Some(supplied) = self.dependencies.libraries.get(&identity) else {
                return Err(Box::new(CompileRefusal::DependencyInput(
                    super::DependencyInputRefusal::UnknownRefinementInternals {
                        identity: identity.as_str().to_owned(),
                    },
                )));
            };
            let (limits, cancel, packages) = (self.limits, self.cancel, self.packages);
            let parsed = charged(cancel, &mut self.work.s1, || {
                parse_unit(
                    supplied.source.clone(),
                    &supplied.path,
                    &supplied.bytes,
                    limits.source,
                    cancel,
                )
            })
            .map_err(|error| error.refusal)?;
            let models = charged(cancel, &mut self.work.i1, || {
                select_models(&parsed, packages, limits.model, cancel)
            })?;
            let refinement_internals = self.dependencies.refinement_internals(&identity).cloned();
            self.check_unit(
                &parsed,
                identity.as_str().to_owned(),
                models,
                super::StateClauseExecutionPoints::new(),
                refinement_internals,
            )
            .map_err(|refusal| wrap(&identity, refusal))?;
        }
        Ok(())
    }

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
        })
        .map_err(|error| error.refusal)?;
        let models = charged(cancel, &mut self.work.i1, || {
            select_models(&parsed, packages, limits.model, cancel)
        })?;
        // A library is compiled under its own identity, independently of
        // whoever imports it.
        Ok(Library {
            identity: identity.clone(),
            source: parsed.source().clone(),
            frame: Frame::new(
                &parsed,
                identity.as_str().to_owned(),
                models,
                super::StateClauseExecutionPoints::new(),
                self.dependencies.refinement_internals(&identity).cloned(),
            ),
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
        let source_units = frame.source_units.clone();
        let package_name = frame.package_name.clone();
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
        let execution_points = frame.execution_points.clone();
        let mut declarations = charged(cancel, &mut self.work.s3, || {
            if source_units.len() > 1 {
                PackageDeclarations::assemble_units(
                    package_name,
                    source_units
                        .iter()
                        .map(|source| (source.source().reference().clone(), source.unit().clone()))
                        .collect(),
                    models,
                    admitted,
                    limits.assembly,
                    cancel,
                )
            } else {
                PackageDeclarations::assemble_with_execution_points_and_cancel(
                    raw.clone(),
                    package_name,
                    unit,
                    models,
                    admitted,
                    limits.assembly,
                    qsl_semantics::check::AssemblyRun {
                        cancel,
                        execution_points,
                    },
                )
            }
        })
        .map_err(|refusal| {
            Box::new(CompileRefusal::Assembly {
                // A type-environment stage limit names no declaration,
                // so it has no region (FR-082, FR-096).
                region: refusal
                    .errors
                    .first()
                    .filter(|error| {
                        !matches!(
                            error.cause,
                            AssemblyCause::TypeLimit(_) | AssemblyCause::DecimalScaleLimit(_)
                        )
                    })
                    .and_then(|error| region(&raw, error.span)),
                refusal,
            })
        })?;
        if let Some(value) = frame.refinement_internals.clone() {
            declarations = declarations.with_refinement_internals(value);
        }
        // FR-302: link the dispatch table of each family a dispatched call's
        // member name may name, under the caller's model normalization
        // limits, before `check` types the calls against them. A family
        // that did not link is reported if the check cannot resolve a call
        // for want of it.
        let pending = charged(cancel, &mut self.work.s3, || {
            declarations.link_dispatch(limits.model)
        })
        .map_err(|refusals| {
            let regions = declarations.regions();
            Box::new(CompileRefusal::Check {
                region: refusals
                    .first()
                    .and_then(|refusal| regions.refusal_region(refusal)),
                refusals,
            })
        })?;
        let regions = declarations.regions();
        let graph = charged(cancel, &mut self.work.s3, || {
            if frame.raw == self.continuation_source {
                declarations.check_with_protocol_continuation_and_cancel(limits.checking, cancel)
            } else {
                declarations.check_with_cancel(limits.checking, cancel)
            }
        })
        .map_err(|refusals| {
            let refusals = pending.report(refusals);
            let region = refusals.first().and_then(|refusal| {
                regions
                    .refusal_region(refusal)
                    .or_else(|| match &refusal.cause {
                        CheckCause::MissingName(name)
                            if frame.execution_points.contains_key(name) =>
                        {
                            SourceRegion::new(frame.raw.clone(), 0, 0).ok()
                        }
                        _ => None,
                    })
            });
            Box::new(CompileRefusal::Check { region, refusals })
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
        parsed: &ParsedUnitSource,
        package_name: String,
        models: SelectedModels,
        execution_points: super::StateClauseExecutionPoints,
        refinement_internals: Option<serde_json::Value>,
    ) -> Result<pkg::CheckedPackage, Box<CompileRefusal>> {
        let root = Frame::new(
            parsed,
            package_name,
            models,
            execution_points,
            refinement_internals,
        );
        self.check_frame(root)
    }

    fn check_unit_list(
        &mut self,
        admitted: &AdmittedUnitList,
        package_name: String,
        models: SelectedModels,
    ) -> Result<pkg::CheckedPackage, Box<CompileRefusal>> {
        let root = Frame::new_units(admitted, package_name, models);
        self.check_frame(root)
    }

    fn check_frame(&mut self, mut root: Frame) -> Result<pkg::CheckedPackage, Box<CompileRefusal>> {
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
            let source = frame.raw.clone();
            let package = self
                .finish_unit(frame)
                .map_err(|refusal| unwind(&libraries, refusal))?;
            if libraries.is_empty() {
                return Ok(package);
            }
            let cancel = self.cancel;
            let selections = self.temporal_selections_for(&source);
            let emission = charged(cancel, &mut self.work.e4, || {
                emit_unit(&package, &selections, cancel)
            })
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
                self.limits.imports,
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
                emitted: emission.package().clone(),
                source: done.source,
            });
            self.compiled
                .insert(done.identity.clone(), Arc::clone(&library));
            current(&mut root, &mut libraries).push_import(done.identity, &library);
        }
    }

    fn temporal_selections_for(
        &self,
        source: &RawSourceRef,
    ) -> Vec<qsl_package::TemporalClockSelection> {
        temporal_selections_for_source(self.temporal_selections, source)
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
        let reached = |setting, bound: usize, actual: usize| {
            refuse(
                ImportRefusal::Limit(LimitExceeded::new(
                    setting,
                    u64::try_from(bound).unwrap_or(u64::MAX),
                    u128::try_from(actual).unwrap_or(u128::MAX),
                )),
                at_identity(),
            )
        };
        // Each import resolved is one edge of the import graph, charged
        // before it is resolved.
        self.import_edges = self.import_edges.saturating_add(1);
        if self.import_edges > limits.import_edges {
            return Err(reached(
                Setting::DependencyImportEdges,
                limits.import_edges,
                self.import_edges,
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
                Setting::DependencyLibraries,
                limits.libraries,
                libraries,
            ));
        }
        let source_bytes = self.source_bytes.saturating_add(supplied.bytes.len());
        if source_bytes > limits.source_bytes {
            return Err(reached(
                Setting::DependencySourceBytes,
                limits.source_bytes,
                source_bytes,
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
