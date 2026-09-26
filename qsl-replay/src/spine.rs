// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-027 and FR-098: spine compile (ADR-011 §5, §7.3 M-6a; ADR-013 O-26).
//! Complete-V1 source enters S1 and goes through the stage APIs in DAG
//! order: S1 (`qsl_cst::parse`), S2 (`qsl_forms::build_unit`), I1
//! (`model::intake::admit_unit`, the unit's `model` declarations against
//! the supplied domain packages, FR-056), the S4 source resolution of the
//! unit's `import`s against the dependency input (ADR-015 D-1, FR-099), the
//! FR-091 assembler, S3 (`PackageDeclarations::check`), E4
//! (`CheckedPackage::link_with`) and the v2 emitter
//! (`qsl_package::emit_checked`). This module calls them and makes no
//! semantic decision.
//!
//! It lives in layer 6 `replay` because both of its callers are layer 6:
//! `command`'s CLI `compile`, which writes the emitted bytes, and this
//! crate's replay executor ([`crate::replay`]), which recompiles a replay
//! request's digest-addressed source and keeps the in-process
//! [`CheckedPackage`] (ADR-011 §2.1 E9). No layer below 6 may depend on
//! S1 and S2 together (ADR-011 §6.1), and `replay` may not depend on
//! `command`, so this is the one place the chain can be written once.

use std::collections::BTreeMap;
use std::sync::Arc;

use qsl_cst::{CompleteDiagnostic, HostCause};
use qsl_forms::{build_unit, FormsCause, FormsFailure, FormsLimits};
use qsl_foundation::digest::DigestRecord;
use qsl_foundation::selection::ImportSelection;
use qsl_foundation::source::provenance::{RawSourceRef, SourceRegion};
use qsl_foundation::{Code, SourceIdentity, Span};
use qsl_package::{
    emit_checked, read_import_view, AdmittedPackages, CheckedPackage, Emission, EmitRefusal,
    EmittedPackage, Import, ImportViewRefusal, LinkRefusal, OmittedNode,
};
use qsl_semantics::check::{
    AdmittedImport, AssemblyCause, AssemblyRefusal, CheckCause, CheckRefusal, CheckingLimits,
    PackageDeclarations,
};
use qsl_semantics::library::{ImportView, LibraryName, PackageId};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, UnitIntakeCause, UnitIntakeRefusal};

/// Why spine [`compile`] produced no checked package. Each
/// variant is the stage that refused, with its typed cause and, where the
/// stage records one, the region of the unit it concerns.
#[derive(Debug, thiserror::Error)]
pub enum CompileRefusal {
    /// E1: S1 refused the bytes, or the CST carries a diagnostic or a
    /// recovery, which E2 does not admit. Holds the first diagnostic.
    #[error("{}", .0.message)]
    Source(Box<CompleteDiagnostic>),
    /// E2: the forms stage refused the unit or reached its depth limit.
    #[error("the forms stage refused the unit: {}", forms_message(.failure))]
    Forms {
        /// The S2 failure.
        failure: FormsFailure,
        /// The region it concerns, when it concerns one.
        region: Option<SourceRegion>,
    },
    /// I1: a `model` declaration's domain package did not admit.
    #[error("domain package intake refused `model {}`: {}", .refusal.alias, intake_message(&.refusal.cause))]
    Intake {
        /// The intake refusal.
        refusal: UnitIntakeRefusal,
        /// The region of the `model` declaration.
        region: Option<SourceRegion>,
    },
    /// E3: the FR-091 assembler refused the unit.
    #[error("the assembler refused the unit: {}", assembly_message(.refusal))]
    Assembly {
        /// Every assembly error, never empty.
        refusal: AssemblyRefusal,
        /// The region of the first error.
        region: Option<SourceRegion>,
    },
    /// E3: the checker refused the package.
    #[error("the checker refused the package: {}", check_message(.refusals))]
    Check {
        /// Every check refusal, never empty.
        refusals: Vec<CheckRefusal>,
        /// The region of the first refusal, when it names one.
        region: Option<SourceRegion>,
    },
    /// ADR-015 D-1: the dependency input refused. Closure-level: reported
    /// unwrapped, with no source region.
    #[error("the dependency input refused: {0}")]
    DependencyInput(DependencyInputRefusal),
    /// ADR-015 D-1: the S4 source resolution refused one of this unit's own
    /// `import`s.
    #[error("{refusal}")]
    Import {
        /// The resolution's refusal.
        refusal: ImportRefusal,
        /// The region of the import, or of its identity string, in the
        /// source that declares it.
        region: Option<SourceRegion>,
    },
    /// ADR-015 D-1: a library's own refusal, raised while resolving or
    /// compiling it. It reports the library's own stage and region, located
    /// in the library's source.
    #[error("the library {} refused: {refusal}", display_path(.path))]
    Dependency {
        /// The library identities from the unit's import down to the
        /// library that refused, that library last.
        path: Vec<LibraryName>,
        /// The library's own refusal; never itself a `Dependency`.
        refusal: Box<CompileRefusal>,
    },
    /// E4: the link step refused the unit's dependency closure.
    #[error("{0}")]
    Link(LinkRefusal),
    /// E4: the v2 emitter wrote no bytes.
    #[error("{0}")]
    Emit(EmitRefusal),
    /// E4: the v2 emitter would omit these nodes. A package missing part of
    /// the checked graph is partial output, which E4 never writes
    /// (ADR-011 §2.3).
    #[error("the checked-package/v2 wire would omit {} node(s)", .0.len())]
    Omitted(Vec<OmittedNode>),
}

impl CompileRefusal {
    /// The catalog code of this refusal: the stage cause's own.
    pub fn code(&self) -> Code {
        match self {
            Self::Source(diagnostic) => diagnostic.code,
            Self::Forms { failure, .. } => failure.catalog_code(),
            Self::Intake { refusal, .. } => refusal.cause.code(),
            Self::Assembly { refusal, .. } => refusal
                .errors
                .first()
                .map_or(Code::RuntimeInvariant, |error| error.cause.code()),
            Self::Check { refusals, .. } => refusals
                .first()
                .map_or(Code::RuntimeInvariant, |refusal| refusal.cause.code()),
            Self::DependencyInput(refusal) => refusal.code(),
            Self::Import { refusal, .. } => refusal.code(),
            Self::Dependency { refusal, .. } => refusal.code(),
            Self::Link(refusal) => refusal.code(),
            Self::Emit(refusal) => refusal.code(),
            // An emission path IR's pinned v2 vocabulary does not hold yet.
            Self::Omitted(_) => Code::UnsupportedProjection,
        }
    }

    /// The stage that refused.
    pub fn stage(&self) -> SpineStage {
        match self {
            Self::Source(_) => SpineStage::Source,
            Self::Forms { .. } => SpineStage::Forms,
            Self::Intake { .. } | Self::DependencyInput(_) | Self::Import { .. } => {
                SpineStage::Intake
            }
            Self::Dependency { refusal, .. } => refusal.stage(),
            Self::Assembly { .. } => SpineStage::Assembly,
            Self::Check { .. } => SpineStage::Check,
            Self::Link(_) | Self::Emit(_) | Self::Omitted(_) => SpineStage::Emit,
        }
    }

    /// The region of the unit this refusal concerns, when its stage records
    /// one.
    pub fn region(&self) -> Option<&SourceRegion> {
        match self {
            Self::Source(diagnostic) => diagnostic.region.as_ref(),
            Self::Forms { region, .. }
            | Self::Intake { region, .. }
            | Self::Assembly { region, .. }
            | Self::Import { region, .. }
            | Self::Check { region, .. } => region.as_ref(),
            Self::Dependency { refusal, .. } => refusal.region(),
            Self::DependencyInput(_) | Self::Link(_) | Self::Emit(_) | Self::Omitted(_) => None,
        }
    }

    /// Whether this refusal is closure-level (ADR-015 D-1): the top-level
    /// compile reports it unwrapped, wherever in the closure it arose.
    fn is_closure_level(&self) -> bool {
        matches!(
            self,
            Self::DependencyInput(_)
                | Self::Import {
                    refusal: ImportRefusal::Cycle { .. }
                        | ImportRefusal::Diamond { .. }
                        | ImportRefusal::DepthLimit { .. },
                    ..
                }
        )
    }
}

/// The spine stage a [`CompileRefusal`] comes from.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpineStage {
    /// S1: `qsl_cst::parse`.
    Source,
    /// S2: `qsl_forms::build_unit`.
    Forms,
    /// I1: `model::intake::admit_unit`.
    Intake,
    /// The FR-091 assembler.
    Assembly,
    /// S3: `PackageDeclarations::check`.
    Check,
    /// S4: the v2 emitter.
    Emit,
}

impl SpineStage {
    /// The stage's name in the command-error envelope.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Forms => "forms",
            Self::Intake => "intake",
            Self::Assembly => "assembly",
            Self::Check => "check",
            Self::Emit => "emit",
        }
    }
}

/// A readable account of an S2 failure.
fn forms_message(failure: &FormsFailure) -> String {
    match failure {
        FormsFailure::Refused(refusal) => match &refusal.cause {
            FormsCause::RecoveringCst => {
                "the source has a syntax error the parser recovered from".to_owned()
            }
            FormsCause::DiagnosedSource(code) => {
                format!("the source carries a {} diagnostic", code.as_str())
            }
            FormsCause::NoDispatchEntry { spelling } => {
                format!("no form reads a `{spelling}` declaration")
            }
            FormsCause::UnrepresentedConstruct { .. } => {
                "no form represents this construct".to_owned()
            }
            FormsCause::UnexpectedShape { .. } => {
                "a node does not have the shape of its grammar rule".to_owned()
            }
        },
        FormsFailure::Limit { limit, .. } => format!(
            "{} (bound {}, reached {})",
            limit.kind().catalog_cause(),
            limit.configured_bound(),
            limit.actual()
        ),
    }
}

/// A readable account of an I1 refusal.
fn intake_message(cause: &UnitIntakeCause) -> String {
    match cause {
        UnitIntakeCause::ArtifactDigest => {
            "a `sha256:` digest selects a compiled-model artifact, not a domain package".to_owned()
        }
        UnitIntakeCause::Refused(refusals) => {
            let first = &refusals[0];
            with_more(
                format!(
                    "{} ({}): {}",
                    first.code.as_str(),
                    first.cause.as_str(),
                    first.detail
                ),
                refusals.len(),
            )
        }
        UnitIntakeCause::Invariant => "the record reader refused with no refusal".to_owned(),
        UnitIntakeCause::Limit(incomplete) => format!(
            "model normalization reached its `{}` limit ({})",
            incomplete.limit_kind.as_str(),
            incomplete.limit
        ),
    }
}

/// A readable account of the assembler's first error, and how many more.
fn assembly_message(refusal: &AssemblyRefusal) -> String {
    let Some(first) = refusal.errors.first() else {
        return "no error recorded".to_owned();
    };
    let message = match &first.cause {
        AssemblyCause::UnresolvedTypeName { name } => format!("no declaration is named `{name}`"),
        AssemblyCause::ImportedTypeName { name } => {
            format!("`{name}` names an imported declaration, which stands only as a callee")
        }
        AssemblyCause::AmbiguousTypeName { name, .. } => {
            format!("`{name}` names more than one declaration")
        }
        AssemblyCause::IllFormedBounds(_) => "a type's declared bounds are ill-formed".to_owned(),
        AssemblyCause::FloatingType { .. } => "a floating type is not admitted".to_owned(),
        AssemblyCause::AliasCycle { edges } => format!(
            "the alias `{}` reaches itself",
            edges.first().map_or("", |(alias, _)| alias.as_str())
        ),
        AssemblyCause::UndeclaredAlias { alias } => {
            format!("`using {alias}` names no profile selection")
        }
        AssemblyCause::DuplicateAlias { alias, .. } => {
            format!("the selection alias `{alias}` is declared more than once")
        }
        AssemblyCause::InvalidTypeDeclaration(_) => {
            "the records and tuples are not an admitted declaration set".to_owned()
        }
        AssemblyCause::TypeLimit(limit) => format!(
            "{} (bound {}, reached {})",
            limit.kind().catalog_cause(),
            limit.configured_bound(),
            limit.actual()
        ),
        AssemblyCause::Handle(_) => "a declared type's handle could not be encoded".to_owned(),
        AssemblyCause::UnsuppliedImport { identity } => {
            format!("`import \"{identity}\"` names a library no dependency input supplies")
        }
        AssemblyCause::UnadmittedModel { alias } => {
            format!("`model {alias}` names no admitted domain package")
        }
        AssemblyCause::UnsupportedModelMember { alias, node } => format!(
            "the domain package of `model {alias}` declares `{node}`, which no type environment \
             entry represents yet"
        ),
        AssemblyCause::ModelType { alias, node } => {
            format!("the domain package of `model {alias}` has no effective type for `{node}`")
        }
    };
    with_more(message, refusal.errors.len())
}

/// A readable account of the checker's first refusal, and how many more.
fn check_message(refusals: &[CheckRefusal]) -> String {
    let Some(first) = refusals.first() else {
        return "no refusal recorded".to_owned();
    };
    let mut message = first.cause.code().as_str().to_owned();
    if let Some(cause) = first.cause.cause() {
        message = format!("{message} ({cause})");
    }
    match &first.cause {
        CheckCause::MissingName(name) | CheckCause::AmbiguousName { name, .. } => {
            message = format!("{message}: `{name}`");
        }
        _ => {}
    }
    with_more(message, refusals.len())
}

/// `message`, noting the `total - 1` further errors it does not describe.
fn with_more(message: String, total: usize) -> String {
    match total.saturating_sub(1) {
        0 => message,
        more => format!("{message}, and {more} more"),
    }
}

/// `span` as a region of `source`.
fn region(source: &RawSourceRef, span: Span) -> Option<SourceRegion> {
    let start = u64::try_from(span.start).ok()?;
    let end = u64::try_from(span.end).ok()?;
    SourceRegion::new(source.clone(), start, end).ok()
}

/// A dependency path written `a -> b -> c`.
fn display_path(path: &[LibraryName]) -> String {
    path.iter()
        .map(LibraryName::as_str)
        .collect::<Vec<_>>()
        .join(" -> ")
}

/// One supplied library as its supplier names it (ADR-015 D-1; QSpec
/// FR-307): its identity, its version and its source unit. It carries no
/// `package_id`: a library's `package_id` is the one its own compile yields
/// (ADR-013 O-02).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SuppliedLibrary {
    /// The library identity, a non-empty string (ADR-015 D-3).
    pub identity: String,
    /// The library's version, a non-empty string.
    pub version: String,
    /// The source's four FR-001 labels.
    pub source: SourceIdentity,
    /// The source's display path; only displayed, never opened.
    pub path: String,
    /// The source bytes.
    pub bytes: Vec<u8>,
}

/// The dependency input of one compile (ADR-015 D-1): at most one supplied
/// library per library identity, each source with its own owner.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DependencyInput {
    libraries: BTreeMap<LibraryName, SuppliedLibrary>,
}

/// Which source of a compile a [`DependencyInputRefusal`] names.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceHolder {
    /// The unit being compiled.
    Unit,
    /// The supplied library of this identity.
    Library(LibraryName),
}

/// Why a dependency input was refused (ADR-015 D-1), naming both offending
/// libraries or the empty field. Closure-level: the compile reports it
/// unwrapped and at no source region.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum DependencyInputRefusal {
    /// A library is supplied under the empty identity
    /// (`invalid_identifier`, [`HostCause::SelectionIdentity`]).
    #[error("invalid_identifier: a library from source {} has an empty identity", .labels.identity)]
    EmptyIdentity {
        /// The library's source labels.
        labels: Box<SourceIdentity>,
    },
    /// A library is supplied with an empty version (`invalid_identifier`,
    /// [`HostCause::SelectionVersion`]).
    #[error("invalid_identifier: the library {identity} has an empty version")]
    EmptyVersion {
        /// The library identity.
        identity: LibraryName,
    },
    /// Two libraries are supplied under one identity
    /// (`invalid_package`/`conflicting-definition`).
    #[error(
        "invalid_package/conflicting-definition: {identity} is supplied from {} and from {}",
        .first.identity,
        .second.identity
    )]
    DuplicateIdentity {
        /// The identity supplied twice.
        identity: LibraryName,
        /// The first library's source labels.
        first: Box<SourceIdentity>,
        /// The second library's source labels.
        second: Box<SourceIdentity>,
    },
    /// A library's source has the authority and identity of the unit's or
    /// of another library's source: one owner per compile (ADR-013 O-04)
    /// (`invalid_package`/`conflicting-definition`).
    #[error(
        "invalid_package/conflicting-definition: the library {second} has the source owner {authority}/{identity} of {}",
        match .first { SourceHolder::Unit => "the unit".to_owned(), SourceHolder::Library(name) => format!("the library {name}") }
    )]
    SharedOwner {
        /// The source that holds the owner first.
        first: SourceHolder,
        /// The library whose source repeats it.
        second: LibraryName,
        /// The shared source authority.
        authority: String,
        /// The shared source identity.
        identity: String,
    },
}

impl DependencyInputRefusal {
    /// The catalog code.
    pub fn code(&self) -> Code {
        match self {
            Self::EmptyIdentity { .. } | Self::EmptyVersion { .. } => Code::InvalidIdentifier,
            Self::DuplicateIdentity { .. } | Self::SharedOwner { .. } => Code::InvalidPackage,
        }
    }

    /// The host cause of an `invalid_identifier` refusal.
    pub fn host_cause(&self) -> Option<HostCause> {
        match self {
            Self::EmptyIdentity { .. } => Some(HostCause::SelectionIdentity),
            Self::EmptyVersion { .. } => Some(HostCause::SelectionVersion),
            Self::DuplicateIdentity { .. } | Self::SharedOwner { .. } => None,
        }
    }

    /// The catalog cause of an `invalid_package` refusal.
    pub fn cause(&self) -> Option<&'static str> {
        match self {
            Self::DuplicateIdentity { .. } | Self::SharedOwner { .. } => {
                Some("conflicting-definition")
            }
            Self::EmptyIdentity { .. } | Self::EmptyVersion { .. } => None,
        }
    }
}

impl DependencyInput {
    /// The dependency input holding `libraries`, refusing, in supply order,
    /// an empty identity or version, a second library under one identity,
    /// and a library whose source repeats another library's owner.
    pub fn new(
        libraries: impl IntoIterator<Item = SuppliedLibrary>,
    ) -> Result<Self, DependencyInputRefusal> {
        let mut held: BTreeMap<LibraryName, SuppliedLibrary> = BTreeMap::new();
        for library in libraries {
            let Ok(identity) = LibraryName::new(library.identity.as_str()) else {
                return Err(DependencyInputRefusal::EmptyIdentity {
                    labels: Box::new(library.source),
                });
            };
            if library.version.is_empty() {
                return Err(DependencyInputRefusal::EmptyVersion { identity });
            }
            if let Some(first) = held.get(&identity) {
                return Err(DependencyInputRefusal::DuplicateIdentity {
                    identity,
                    first: Box::new(first.source.clone()),
                    second: Box::new(library.source),
                });
            }
            if let Some((first, _)) = held
                .iter()
                .find(|(_, other)| same_owner(&other.source, &library.source))
            {
                return Err(DependencyInputRefusal::SharedOwner {
                    first: SourceHolder::Library(first.clone()),
                    second: identity,
                    authority: library.source.authority,
                    identity: library.source.identity,
                });
            }
            held.insert(identity, library);
        }
        Ok(Self { libraries: held })
    }

    /// Refuse a library whose source has the owner of `unit`, the unit
    /// being compiled against this input (ADR-013 O-04).
    pub(crate) fn check_unit_owner(
        &self,
        unit: &SourceIdentity,
    ) -> Result<(), DependencyInputRefusal> {
        match self
            .libraries
            .iter()
            .find(|(_, library)| same_owner(&library.source, unit))
        {
            Some((identity, _)) => Err(DependencyInputRefusal::SharedOwner {
                first: SourceHolder::Unit,
                second: identity.clone(),
                authority: unit.authority.clone(),
                identity: unit.identity.clone(),
            }),
            None => Ok(()),
        }
    }
}

/// Whether two sources have one owner: the same authority and identity
/// (ADR-013 O-04).
fn same_owner(first: &SourceIdentity, second: &SourceIdentity) -> bool {
    first.authority == second.authority && first.identity == second.identity
}

/// One import's selection as the S4 source resolution first visited it: the
/// version and digest it records and the dependency path that reached it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VisitedImport {
    /// The library identities from the unit's import down to this library.
    pub path: Vec<LibraryName>,
    /// The version the import names.
    pub version: String,
    /// The digest the import records.
    pub digest: DigestRecord,
}

/// Why the S4 source resolution refused an `import` (ADR-015 D-1).
#[derive(Debug, thiserror::Error)]
pub enum ImportRefusal {
    /// Step 1: the import names a library whose compile is in progress
    /// (`invalid_package`/`definition-cycle`). Closure-level.
    #[error("invalid_package/definition-cycle: {}", display_path(.path))]
    Cycle {
        /// The identity path, from the library the cycle returns to, back
        /// to it.
        path: Vec<LibraryName>,
    },
    /// Step 2: an earlier import in the closure selects the same identity
    /// with another version or digest
    /// (`invalid_package`/`conflicting-definition`, QSpec FR-307's diamond
    /// rule). Closure-level.
    #[error(
        "invalid_package/conflicting-definition: {} and {} select {identity} differently",
        display_path(&.first.path),
        display_path(&.second.path)
    )]
    Diamond {
        /// The library identity.
        identity: LibraryName,
        /// The earlier import.
        first: Box<VisitedImport>,
        /// The later, conflicting import.
        second: Box<VisitedImport>,
    },
    /// Step 3: no library is supplied under the import's identity
    /// (`missing_import`/`missing-selection`).
    #[error("missing_import/missing-selection: no library is supplied as {identity}")]
    MissingSelection {
        /// The identity the import names.
        identity: LibraryName,
    },
    /// Step 4: compiling the library would nest library compiles deeper
    /// than [`DependencyLimits::depth`] (`stage_limit_exceeded`/
    /// `nesting-depth-exceeded`). Closure-level.
    #[error("stage_limit_exceeded/nesting-depth-exceeded: {} nests library compiles deeper than {limit}", display_path(.path))]
    DepthLimit {
        /// The configured ceiling.
        limit: usize,
        /// The dependency path whose compile would exceed it.
        path: Vec<LibraryName>,
    },
    /// An import names the empty identity, which the parser never admits:
    /// a broken invariant (`runtime_invariant`).
    #[error("runtime_invariant: an admitted import names the empty library identity")]
    UnnamedImport,
    /// Step 3: the library supplied under the import's identity has another
    /// version (`stale_dependency`/`revision-mismatch`).
    #[error(
        "stale_dependency/revision-mismatch: {identity} is imported at version {imported} but supplied at {supplied}"
    )]
    RevisionMismatch {
        /// The library identity.
        identity: LibraryName,
        /// The version the import names.
        imported: String,
        /// The version the library is supplied at.
        supplied: String,
    },
    /// Step 5: the library's recomputed `package_id` is not the digest the
    /// import records (`stale_dependency`/`byte-digest-mismatch`,
    /// ADR-011 §4).
    #[error(
        "stale_dependency/byte-digest-mismatch: {identity} is recorded as {} but its source compiles to {}",
        .recorded.hex(),
        .recompiled.hex()
    )]
    DependencyIdentityMismatch {
        /// The library identity.
        identity: LibraryName,
        /// The digest the import records.
        recorded: DigestRecord,
        /// The library's recomputed `package_id`.
        recompiled: PackageId,
    },
    /// Step 6: the I2 read of the library's emitted bytes built no import
    /// view.
    #[error("the I2 read of {identity} refused: {refusal}")]
    View {
        /// The library identity.
        identity: LibraryName,
        /// The read's refusal or reached ceiling.
        refusal: Box<ImportViewRefusal>,
    },
}

impl ImportRefusal {
    /// The catalog code.
    pub fn code(&self) -> Code {
        match self {
            Self::Cycle { .. } | Self::Diamond { .. } => Code::InvalidPackage,
            Self::MissingSelection { .. } => Code::MissingImport,
            Self::DepthLimit { .. } => Code::StageLimitExceeded,
            Self::UnnamedImport => Code::RuntimeInvariant,
            Self::RevisionMismatch { .. } | Self::DependencyIdentityMismatch { .. } => {
                Code::StaleDependency
            }
            Self::View { refusal, .. } => refusal.code(),
        }
    }

    /// The catalog cause, where the catalog names one.
    pub fn cause(&self) -> Option<&'static str> {
        match self {
            Self::Cycle { .. } => Some("definition-cycle"),
            Self::Diamond { .. } => Some("conflicting-definition"),
            Self::MissingSelection { .. } => Some("missing-selection"),
            Self::DepthLimit { .. } => Some("nesting-depth-exceeded"),
            Self::UnnamedImport => None,
            Self::RevisionMismatch { .. } => Some("revision-mismatch"),
            Self::DependencyIdentityMismatch { .. } => Some("byte-digest-mismatch"),
            Self::View { .. } => None,
        }
    }
}

/// The stage limits one spine compile runs under: S1's, S2's, I1's and
/// S3's own limits types, each defaulting to that stage's published
/// default. The v2 emitter takes none.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SpineLimits {
    /// S1: the lexer and parser ceilings.
    pub source: qsl_cst::Limits,
    /// S2: the forms depth ceiling.
    pub forms: FormsLimits,
    /// I1: domain package normalization ceilings.
    pub model: ModelNormalizationLimits,
    /// S3: the checker's ceilings.
    pub checking: CheckingLimits,
    /// The S4 source resolution's ceilings (ADR-015 D-1).
    pub dependencies: DependencyLimits,
}

/// The S4 source resolution's ceilings. A library compile is charged the
/// full S1 to S4 limits as its own unit; this bounds how deeply library
/// compiles nest, since each nested compile holds its caller's state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DependencyLimits {
    /// The most library compiles in progress at once: the longest
    /// dependency path from the unit. Defaults to 64.
    pub depth: usize,
}

impl Default for DependencyLimits {
    fn default() -> Self {
        Self { depth: 64 }
    }
}

/// One unit compiled through S1 to S4: the in-process checked package and
/// its emitted `quire.checked-package/v2` bytes with their `package_id`.
#[derive(Debug)]
pub struct Compiled {
    /// The S4 in-process package.
    pub package: CheckedPackage,
    /// The S4 wire output.
    pub emitted: EmittedPackage,
}

/// Compile complete-V1 `bytes`, labelled `source` and displayed as `path`,
/// through S1 to S4 under `limits`. `packages` is FR-056's package input,
/// the supplied domain package documents by their `sha256-jcs` digest
/// (`model::intake::package_input`): I1 admits each `model` declaration of
/// the unit against it. `dependencies` is the dependency input (ADR-015
/// D-1): the S4 source resolution compiles each library the unit's
/// `import`s reach from its source, against the same package input,
/// dependency input and limits. No lock file or request file is read
/// (ADR-011 §5), and `path` is only displayed, never opened.
pub fn compile(
    source: SourceIdentity,
    path: &str,
    bytes: &[u8],
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    dependencies: &DependencyInput,
    limits: SpineLimits,
) -> Result<Compiled, Box<CompileRefusal>> {
    dependencies
        .check_unit_owner(&source)
        .map_err(|refusal| Box::new(CompileRefusal::DependencyInput(refusal)))?;
    let mut resolution = Resolution {
        dependencies,
        packages,
        limits,
        active: Vec::new(),
        visited: BTreeMap::new(),
        compiled: BTreeMap::new(),
        admitted: AdmittedPackages::default(),
    };
    let (package, emission) = resolution.compile_unit(source, path, bytes)?;
    Ok(Compiled {
        package,
        emitted: emission.package().clone(),
    })
}

/// A library the S4 source resolution compiled: its checked package and
/// its import view.
#[derive(Debug)]
struct ResolvedLibrary {
    package: Arc<CheckedPackage>,
    view: ImportView,
}

/// One compile's S4 source resolution state (ADR-015 D-1).
struct Resolution<'a> {
    dependencies: &'a DependencyInput,
    packages: &'a BTreeMap<[u8; 32], Vec<u8>>,
    limits: SpineLimits,
    /// The libraries whose compile is in progress, outermost first.
    active: Vec<LibraryName>,
    /// Each identity's first import in the closure.
    visited: BTreeMap<LibraryName, VisitedImport>,
    /// Each library whose compile completed. A library is compiled at most
    /// once per compile, so the number of library compiles is at most the
    /// number of supplied libraries.
    compiled: BTreeMap<LibraryName, Arc<ResolvedLibrary>>,
    /// IR's admitted package of each library read so far, which a later
    /// library's view read takes for its closure instead of reading again.
    admitted: AdmittedPackages,
}

impl Resolution<'_> {
    /// One unit through S1 to S4, resolving its imports depth first.
    fn compile_unit(
        &mut self,
        source: SourceIdentity,
        path: &str,
        bytes: &[u8],
    ) -> Result<(CheckedPackage, Emission), Box<CompileRefusal>> {
        let limits = self.limits;
        let parsed = qsl_cst::parse(source, path, bytes, limits.source)
            .map_err(|diagnostic| Box::new(CompileRefusal::Source(diagnostic)))?;
        if let Some(first) = parsed.diagnostics().first() {
            return Err(Box::new(CompileRefusal::Source(Box::new(first.clone()))));
        }
        let raw = parsed.source().reference().clone();
        let unit = build_unit(&parsed, limits.forms).map_err(|failure| {
            let span = match &failure {
                FormsFailure::Refused(refusal) => refusal.span,
                FormsFailure::Limit { span, .. } => Some(*span),
            };
            Box::new(CompileRefusal::Forms {
                region: span.and_then(|span| region(&raw, span)),
                failure,
            })
        })?;
        let models = admit_unit(&unit.selections().models, self.packages, limits.model).map_err(
            |refusal| {
                Box::new(CompileRefusal::Intake {
                    region: region(&raw, refusal.span),
                    refusal,
                })
            },
        )?;
        let mut admitted = Vec::with_capacity(unit.selections().imports.len());
        let mut links = Vec::with_capacity(unit.selections().imports.len());
        for import in &unit.selections().imports {
            let (identity, library) = self.resolve(&raw, import)?;
            admitted.push(AdmittedImport {
                identity: identity.clone(),
                view: library.view.clone(),
                graph: library.package.shared_graph(),
            });
            links.push(Import {
                identity,
                version: import.version.clone(),
                digest: import.digest,
                package: Arc::clone(&library.package),
            });
        }
        let declarations = PackageDeclarations::assemble(raw.clone(), unit, models, admitted)
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
        let regions = declarations.regions();
        let graph = declarations.check(limits.checking).map_err(|refusals| {
            Box::new(CompileRefusal::Check {
                region: refusals
                    .first()
                    .and_then(|refusal| regions.refusal_region(refusal)),
                refusals,
            })
        })?;
        let package = CheckedPackage::link_with(graph, links)
            .map_err(|refusal| Box::new(CompileRefusal::Link(refusal)))?;
        let emission =
            emit_checked(&package).map_err(|refusal| Box::new(CompileRefusal::Emit(refusal)))?;
        if !emission.omitted().is_empty() {
            return Err(Box::new(CompileRefusal::Omitted(
                emission.omitted().to_vec(),
            )));
        }
        Ok((package, emission))
    }

    /// ADR-015 D-1's six steps for one `import` of the unit `raw` names:
    /// cycle, diamond, selection, compile, identity, view.
    fn resolve(
        &mut self,
        raw: &RawSourceRef,
        import: &ImportSelection,
    ) -> Result<(LibraryName, Arc<ResolvedLibrary>), Box<CompileRefusal>> {
        let at_identity = || region(raw, import.identity_span);
        let at_import = || region(raw, import.span);
        let refuse = |refusal, region| Box::new(CompileRefusal::Import { refusal, region });
        // The parser admits no empty identity.
        let Ok(identity) = LibraryName::new(import.identity.as_str()) else {
            return Err(refuse(ImportRefusal::UnnamedImport, at_identity()));
        };
        // 1. Cycle, before any digest is compared.
        if let Some(start) = self.active.iter().position(|active| *active == identity) {
            let mut path = self.active[start..].to_vec();
            path.push(identity);
            return Err(refuse(ImportRefusal::Cycle { path }, at_identity()));
        }
        let mut path = self.active.clone();
        path.push(identity.clone());
        let visit = VisitedImport {
            path,
            version: import.version.clone(),
            digest: import.digest,
        };
        // 2. Diamond; an equal earlier import reuses its completed library.
        if let Some(first) = self.visited.get(&identity) {
            if first.version != visit.version || first.digest != visit.digest {
                return Err(refuse(
                    ImportRefusal::Diamond {
                        identity,
                        first: Box::new(first.clone()),
                        second: Box::new(visit),
                    },
                    at_identity(),
                ));
            }
            if let Some(library) = self.compiled.get(&identity) {
                return Ok((identity, Arc::clone(library)));
            }
        }
        self.visited.insert(identity.clone(), visit);
        // 3. Selection.
        let dependencies = self.dependencies;
        let Some(supplied) = dependencies.libraries.get(&identity) else {
            return Err(refuse(
                ImportRefusal::MissingSelection { identity },
                at_identity(),
            ));
        };
        if supplied.version != import.version {
            return Err(refuse(
                ImportRefusal::RevisionMismatch {
                    identity,
                    imported: import.version.clone(),
                    supplied: supplied.version.clone(),
                },
                at_import(),
            ));
        }
        // 4. Compile, by this same resolution, within the depth ceiling.
        if self.active.len() >= self.limits.dependencies.depth {
            let mut path = self.active.clone();
            path.push(identity);
            return Err(refuse(
                ImportRefusal::DepthLimit {
                    limit: self.limits.dependencies.depth,
                    path,
                },
                at_identity(),
            ));
        }
        self.active.push(identity.clone());
        let compiled = self.compile_unit(supplied.source.clone(), &supplied.path, &supplied.bytes);
        self.active.pop();
        let (package, emission) = compiled.map_err(|refusal| wrap(&identity, refusal))?;
        // 5. Identity.
        let recompiled = emission.package().package_id();
        if !recompiled.matches(&import.digest) {
            return Err(refuse(
                ImportRefusal::DependencyIdentityMismatch {
                    identity,
                    recorded: import.digest,
                    recompiled,
                },
                at_import(),
            ));
        }
        // 6. View.
        let view = read_import_view(
            &package,
            &emission,
            identity.clone(),
            &import.version,
            self.packages,
            &mut self.admitted,
        )
        .map_err(|refusal| {
            refuse(
                ImportRefusal::View {
                    identity: identity.clone(),
                    refusal: Box::new(refusal),
                },
                at_import(),
            )
        })?;
        let library = Arc::new(ResolvedLibrary {
            package: Arc::new(package),
            view,
        });
        self.compiled.insert(identity.clone(), Arc::clone(&library));
        Ok((identity, library))
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

#[cfg(test)]
mod dependency_tests;

#[cfg(test)]
mod tests {
    use super::{compile, CompileRefusal, DependencyInput, ImportRefusal, SpineLimits, SpineStage};
    use ix_trace_rs::trace;
    use qsl_foundation::{Code, SourceIdentity};
    use qsl_semantics::check::CheckingLimits;
    use std::collections::BTreeMap;

    /// FR-096 at the CLI's compile: `Typer`'s depth stop on
    /// `not not not true` under depth 3 is reported at the region of
    /// `true`, not the whole body.
    #[trace("TC-378", "FR-096-AC-11")]
    #[test]
    fn a_family_depth_stop_is_reported_at_the_node_that_failed() {
        const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
            profile v = \"quire.value.complete/v1\" version \"1\" digest \
            \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n\
            function f using v(): Boolean pure { not not not true }\n";
        let refusal = compile(
            SourceIdentity::new("a", "u", "git", "1"),
            "unit.native",
            UNIT.as_bytes(),
            &BTreeMap::new(),
            &DependencyInput::default(),
            SpineLimits {
                checking: CheckingLimits::new(u64::MAX, 3).unwrap(),
                ..SpineLimits::default()
            },
        )
        .expect_err("depth 3 stops a body four deep");
        assert_eq!(refusal.stage(), SpineStage::Check);
        assert_eq!(refusal.code(), Code::StageLimitExceeded);
        let region = refusal.region().expect("the stop is located");
        let start = usize::try_from(region.start()).unwrap();
        let end = usize::try_from(region.end()).unwrap();
        assert_eq!(&UNIT[start..end], "true");
        assert_eq!(start, UNIT.rfind("true").unwrap());
    }

    /// FR-091-AC-24 (ADR-015 D-1): with no library supplied, the S4 source
    /// resolution refuses an `import` at stage `intake`, before assembly, as
    /// `missing_import`/`missing-selection` at the import's identity string,
    /// rather than drop the import from the package it emits.
    #[trace("TC-405", "FR-091-AC-24")]
    #[test]
    fn an_import_no_dependency_input_supplies_refuses() {
        let unit = format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             profile v = \"quire.value.complete/v1\" version \"1\" digest \
             \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n\
             import \"test/units\" version \"2\" digest \"{}\" as u;\n\
             function f using v(): Boolean pure {{ true }}\n",
            "b".repeat(64)
        );
        let refusal = compile(
            SourceIdentity::new("a", "u", "git", "1"),
            "unit.native",
            unit.as_bytes(),
            &BTreeMap::new(),
            &DependencyInput::default(),
            SpineLimits::default(),
        )
        .expect_err("no dependency input supplies test/units");
        assert_eq!(refusal.stage(), SpineStage::Intake);
        assert_eq!(refusal.code(), Code::MissingImport);
        let CompileRefusal::Import {
            refusal: import, ..
        } = &*refusal
        else {
            panic!("expected an import refusal, got {refusal:?}");
        };
        assert!(
            matches!(import, ImportRefusal::MissingSelection { identity } if identity.as_str() == "test/units"),
            "{import:?}"
        );
        assert_eq!(import.cause(), Some("missing-selection"));
        let region = refusal.region().expect("the refusal is located");
        let start = usize::try_from(region.start()).unwrap();
        let end = usize::try_from(region.end()).unwrap();
        assert_eq!(&unit[start..end], "\"test/units\"");
    }

    /// FR-027-AC-8 (TC-435 step 6): an emission that would omit part of the
    /// checked graph refuses at the emit stage as `unsupported_projection`.
    /// No complete-V1 source reaches it through the CLI yet, since every
    /// construct S3 checks today has a v2 form.
    #[trace("TC-435", "FR-027-AC-8")]
    #[test]
    fn an_omitting_emission_refuses_at_the_emit_stage() {
        let refusal = CompileRefusal::Omitted(Vec::new());
        assert_eq!(refusal.stage(), SpineStage::Emit);
        assert_eq!(refusal.code(), Code::UnsupportedProjection);
        assert!(refusal.region().is_none());
    }
}
