// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-026/027: local file orchestration around the existing native compiler and runtime.

mod compilation;
#[cfg(feature = "quire-extraction")]
pub mod extraction;
mod output;
mod projection_error;
mod source_package;
mod wire;

pub use output::NativeResult;
pub use projection_error::{ProjectionLocation, ProjectionSource};
pub use source_package::{resolve_parsed_source, SourcePackageRefusal};

use crate::formal_source::FormalSource;
use crate::lowering::{self, LoweringError, LoweringLimits, ProjectionTarget};
use crate::model_source::ModelSourceError;
use crate::package::{NativePackage, NativePackageRef, PackageError};
use crate::protocol_artifact::DOMAIN_PACKAGE_PROFILE;
use crate::runtime::{
    self, ArtifactLimits, ExecutionLimits, InputReadError, Invocation, RuntimeInput, Snapshot,
};
use qsl_foundation::serde_object::Object;
use qsl_foundation::{ByteDigest, Code, Diagnostic, Source};
use quire_contract_ir as ir;
use std::{
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};

type Result<T> = std::result::Result<T, RunCause>;
const REQUEST_BYTES: usize = 1_048_576;
const TOTAL_BYTES: usize = 8_388_608;
const FILES: usize = 64;

/// Selected file category charged by command preflight.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum FileCategory {
    /// Native program source.
    Program,
    /// Rule-model sources.
    Model,
    /// Snapshot artifacts.
    Snapshot,
    /// Invocation artifacts.
    Invocation,
    /// Selected compiled package artifacts.
    Package,
}

impl FileCategory {
    /// Stable category spelling in native intake diagnostics.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Program => "programs",
            Self::Model => "models",
            Self::Snapshot => "snapshots",
            Self::Invocation => "invocations",
            Self::Package => "packages",
        }
    }
}

struct FileCounts {
    programs: usize,
    models: usize,
    snapshots: usize,
    invocations: usize,
    packages: usize,
}

/// Command intake ceiling that stopped selected-file processing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum LimitKind {
    /// Per-file or remaining aggregate byte ceiling.
    FileBytes,
    /// Total selected file count.
    SelectedFiles {
        /// Group that could not be charged.
        category: FileCategory,
        /// Files requested by the group.
        requested: usize,
        /// Remaining slots before this group.
        remaining: usize,
        /// Effective total file ceiling.
        maximum: usize,
    },
}

impl LimitKind {
    /// Stable field spelling retained for native result compatibility.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FileBytes => "file bytes",
            Self::SelectedFiles { .. } => "selected files",
        }
    }
}

impl std::fmt::Display for LimitKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// JSON result of the local command, distinct from a portable evidence envelope.
#[derive(Debug)]
pub struct RunResult {
    /// FR-301's contract: 0 completed true, 10 completed false (logical
    /// violation), 20 refused, or 22 incomplete.
    pub exit_code: u8,
    /// Structured native-run-result/1 observations.
    pub value: NativeResult,
}

/// Original intake failure, without recovering typed data from display messages.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RunCause {
    /// Local file could not be opened or read.
    #[error("cannot read {path}: {error}")]
    Io {
        /// Actual selected filesystem path.
        path: PathBuf,
        /// Original operating-system error.
        #[source]
        error: io::Error,
    },
    /// Closed request decoding failed.
    #[error("invalid request: {0}")]
    Json(#[from] serde_json::Error),
    /// Typed output serialization failed.
    #[error("cannot serialize native result: {0}")]
    Output(#[source] serde_json::Error),
    /// Request format was not selected.
    #[error("unsupported native command format")]
    Format,
    /// Extraction selection, context or actual Quire/native compilation failure.
    #[cfg(feature = "quire-extraction")]
    #[error("{0}")]
    Extraction(#[from] Box<extraction::ExtractionError>),
    /// A command intake ceiling stopped the request.
    #[error("{0} limit exceeded")]
    Limit(LimitKind),
    /// Selected digest spelling is invalid.
    #[error("{0}")]
    Digest(#[from] qsl_foundation::digest::InvalidDigest),
    /// Existing IR identifier validation failed.
    #[error("invalid request identifier: {0:?}")]
    Identifier(ir::Diagnostic),
    /// Existing source/parser/profile failure with no linking or checking context.
    #[error("{0}")]
    Native(#[from] Box<Diagnostic>),
    /// Existing formal-environment linking failure.
    #[error("{0}")]
    Linking(#[from] Box<crate::linking::LinkingError>),
    /// Existing native constraint/proof checking failure.
    #[error("{0}")]
    Checking(#[from] Box<crate::checking::CheckingError>),
    /// Existing model frontend/admission failure.
    #[error("{0}")]
    Model(#[from] Box<ModelSourceError>),
    /// Existing native package failure.
    #[error("{0}")]
    Package(#[from] Box<PackageError>),
    /// Original lowering failure and its resolved native source context.
    #[error("{error}")]
    Lowering {
        /// Explicit target whose lowering failed.
        target: ProjectionTarget,
        /// Exact native package selected for projection.
        package: NativePackageRef,
        /// Program provenance without retained source text.
        program: Box<ProjectionSource>,
        /// Source coordinates resolved while the selected program was available.
        location: ProjectionLocation,
        /// Original classification, authored clause and IR diagnostics.
        #[source]
        error: Box<LoweringError>,
    },
    /// The selected package failed its verified reader; source compilation is not a fallback.
    #[error("selected package {file}: {error}")]
    SelectedPackage {
        /// Request-selected package file.
        file: String,
        /// Exact expected package byte reference.
        expected: NativePackageRef,
        /// Original reader failure.
        #[source]
        error: Box<PackageError>,
    },
    /// Existing selected runtime artifact failure.
    #[error("{0}")]
    Input(#[from] Box<InputReadError>),
    /// A `1-draft` program's request selects what spine compile does not
    /// take.
    #[error("a complete-V1 program compiles alone; the request selects {0}")]
    CompleteSelection(CompleteSelection),
    /// The request's `libraries` are refused before any library is read.
    #[error("the request's libraries are refused: {0}")]
    Libraries(LibrarySelection),
    /// Spine compile of a `1-draft` program refused at one stage.
    #[error("{}", .0.refusal)]
    Spine(Box<SpineFailure>),
    /// FR-100: a `1-draft` native-run/1 request carries a member the run
    /// command does not admit for it, or carries no `call`.
    #[error("a 1-draft program runs through the spine; the request selects {0}")]
    CompleteRunSelection(CompleteRunSelection),
    /// FR-100: a `0-draft` native-run/1 request carries a `1-draft`-only
    /// member, or is missing a native-only member FR-026 requires.
    #[error("a 0-draft program runs natively; {0}")]
    NativeRunSelection(NativeRunSelection),
    /// FR-100: the spine run entry refused, at its own stage and cause code.
    #[error("{0}")]
    SpineRun(Box<qsl_replay::spine::RunRefusal>),
    /// FR-026: a `0-draft` (or no-edition) program requires `clauses`,
    /// which a `1-draft` program never reaches this refusal for (`run_complete`/
    /// `complete` refuse it before native package construction is ever
    /// attempted).
    #[error("a 0-draft program requires clause bindings")]
    MissingClauses,
}

/// A request selection FR-100 does not admit for a `1-draft` program's
/// native-run/1 request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum CompleteRunSelection {
    /// The request selects an execution selection.
    Selection,
    /// The request selects a snapshot artifact.
    Snapshots,
    /// The request selects an invocation artifact.
    Invocations,
    /// The request selects a compiled package artifact.
    Package,
    /// The request sets a `validation_work` limit.
    ValidationWorkLimit,
    /// The request sets an `expression_steps` limit.
    ExpressionStepsLimit,
    /// The program carries clause bindings.
    Clauses,
    /// The program carries an extraction selection.
    #[cfg(feature = "quire-extraction")]
    Extraction,
    /// The request selects a model source in a native rule-model format.
    NativeModel,
    /// The request carries no `call`.
    NoCall,
}

impl std::fmt::Display for CompleteRunSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Selection => "an execution selection",
            Self::Snapshots => "a snapshot artifact",
            Self::Invocations => "an invocation artifact",
            Self::Package => "a compiled package artifact",
            Self::ValidationWorkLimit => "a validation_work limit",
            Self::ExpressionStepsLimit => "an expression_steps limit",
            Self::Clauses => "clause bindings",
            #[cfg(feature = "quire-extraction")]
            Self::Extraction => "an extraction selection",
            Self::NativeModel => "a native rule-model source",
            Self::NoCall => "no call",
        })
    }
}

/// Why a `0-draft` native-run/1 request refuses (FR-100).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum NativeRunSelection {
    /// The request carries a `call`, which only a `1-draft` program takes.
    Call,
    /// The request carries `libraries`, which only a `1-draft` program
    /// takes.
    Libraries,
    /// The request carries no execution selection.
    MissingSelection,
    /// The request carries no snapshot selections.
    MissingSnapshots,
    /// The request carries no invocation selections.
    MissingInvocations,
}

impl std::fmt::Display for NativeRunSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Call => "the request selects a call",
            Self::Libraries => "the request selects libraries",
            Self::MissingSelection => "the request selects no execution selection",
            Self::MissingSnapshots => "the request selects no snapshots",
            Self::MissingInvocations => "the request selects no invocations",
        })
    }
}

/// A spine refusal of a `1-draft` program, with the program it concerns.
#[derive(Debug)]
pub struct SpineFailure {
    /// The program's four source labels.
    pub source: qsl_foundation::SourceIdentity,
    /// The program's request-selected path.
    pub path: String,
    /// The refused region rendered over the program, when the stage records
    /// one.
    pub span: Option<qsl_foundation::LocatedSpan>,
    /// The stage's typed refusal.
    pub refusal: qsl_replay::spine::CompileRefusal,
}

impl From<ir::Diagnostic> for RunCause {
    fn from(value: ir::Diagnostic) -> Self {
        Self::Identifier(value)
    }
}

impl RunCause {
    /// Stable catalogued code for this cause, independent of its display message.
    pub fn code(&self) -> Code {
        match self {
            Self::Io { .. } => Code::IoError,
            Self::Json(_) => Code::InvalidRequest,
            Self::Output(_) => Code::OutputFailure,
            Self::Format => Code::UnknownWire,
            #[cfg(feature = "quire-extraction")]
            Self::Extraction(error) => error.code(),
            Self::Limit(_) => Code::ResourceExhausted,
            Self::Digest(_) => Code::InvalidDigest,
            Self::Identifier(_) => Code::InvalidIdentifier,
            Self::Native(error) => error.code,
            Self::Linking(error) => error.diagnostic.code,
            Self::Checking(error) => error.diagnostic.code,
            Self::Model(error) => error.code(),
            Self::Package(error) | Self::SelectedPackage { error, .. } => error.code,
            Self::Input(error) => error.code,
            Self::Lowering { error, .. } => error.code.code(),
            Self::CompleteSelection(_)
            | Self::Libraries(_)
            | Self::CompleteRunSelection(_)
            | Self::NativeRunSelection(_)
            | Self::MissingClauses => Code::InvalidRequest,
            Self::Spine(failure) => failure.refusal.code(),
            Self::SpineRun(refusal) => refusal.code(),
        }
    }

    /// Whether the retained cause prevented completion.
    pub fn is_incomplete(&self) -> bool {
        self.code().is_incomplete()
    }

    /// Native command exit status, on FR-301's six-code contract: writing
    /// the outcome out is a tool failure (30); every other disposition
    /// resolves through `Code::exit_code()`, the one place the
    /// unsupported(21)/incomplete(22)/invalid(20) ladder is written down.
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Output(_) => 30,
            _ => self.code().exit_code(),
        }
    }
}

/// Failed command retaining the exact request byte identity when it was read.
#[derive(Debug, thiserror::Error)]
#[error("{cause}")]
pub struct RunError {
    /// Exact request bytes; absent when bounded request intake itself failed.
    pub request_digest: Option<ByteDigest>,
    /// Original typed failure.
    #[source]
    pub cause: RunCause,
}

impl RunError {
    /// FR-301's six-code exit contract, derived from the cause.
    pub fn exit_code(&self) -> u8 {
        self.cause.exit_code()
    }

    /// JSON error with original stage/code and available source/reference details.
    pub fn value(&self) -> std::result::Result<serde_json::Value, serde_json::Error> {
        output::error(self)
    }
}

fn read_file(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let read = || -> io::Result<Vec<u8>> {
        let file = File::open(path)?;
        let mut bytes = Vec::new();
        // Limits are internal constants or their checked remainder, well below u64.
        let ceiling = u64::try_from(limit).map_err(io::Error::other)?;
        file.take(ceiling.saturating_add(1))
            .read_to_end(&mut bytes)?;
        Ok(bytes)
    };
    let bytes = read().map_err(|error| RunCause::Io {
        path: path.into(),
        error,
    })?;
    if bytes.len() > limit {
        return Err(RunCause::Limit(LimitKind::FileBytes));
    }
    Ok(bytes)
}

struct Intake<'a> {
    directory: &'a Path,
    remaining: usize,
}

impl Intake<'_> {
    fn file(&mut self, path: &str, ceiling: usize) -> Result<Vec<u8>> {
        let bytes = read_file(&self.directory.join(path), ceiling.min(self.remaining))?;
        self.remaining -= bytes.len();
        Ok(bytes)
    }

    fn source(&mut self, selected: &wire::SourceFile) -> Result<FormalSource> {
        let identity = selected.identity.bind()?;
        let expected = selected.digest.parse()?;
        let bytes = self.file(&selected.file, REQUEST_BYTES)?;
        let source = Source::read_verified(
            identity.native,
            &selected.file,
            &bytes,
            expected,
            REQUEST_BYTES,
        )?;
        Ok(FormalSource::new(source, identity.formal))
    }
}

/// Read a native-run/1 job and execute it locally with fresh, bounded stage state.
pub fn run(path: &Path) -> std::result::Result<RunResult, Box<RunError>> {
    with_request(path, run_bytes)
}

/// Compile a native-compile/1 source-only job (FR-027). The program source's
/// declared edition selects the compiler: `1-draft` compiles through the
/// spine into `quire.checked-package/v2` bytes, `0-draft` into the
/// native-linked-package/1 bytes. No runtime artifact is read or executed;
/// failures expose no partial artifact.
pub fn compile(path: &Path) -> std::result::Result<Vec<u8>, Box<RunError>> {
    with_request(path, |directory, bytes, _digest| {
        let request: wire::CompileRequest = request(bytes)?;
        compilation::source_only(&request.program)?;
        let mut intake = intake(
            directory,
            FileCounts {
                // A library source is a program source a `1-draft` program
                // imports, and counts toward the same file limits (FR-027).
                programs: request.libraries.len().saturating_add(1),
                models: request.models.len(),
                snapshots: 0,
                invocations: 0,
                packages: 0,
            },
        )?;
        // The edition is decided before any model is read: a `1-draft`
        // program takes domain packages only, and a request selecting a
        // native rule model for it refuses whatever state the model files
        // are in.
        let source = intake.source(&request.program.source)?;
        match Edition::of(source.source())? {
            Edition::Complete => complete(&request, &mut intake, source.source()),
            Edition::Native if !request.libraries.is_empty() => {
                Err(RunCause::Libraries(LibrarySelection::NativeProgram))
            }
            Edition::Native => {
                let models = compilation::models(&mut intake, &request.models)?;
                Ok(compilation::package_of(source, &request.program, &models)?
                    .bytes()
                    .to_vec())
            }
        }
    })
}

/// The compiler a program source's declared edition selects.
enum Edition {
    /// `1-draft`: the spine, S1 to S4 (ADR-011 §7.3 M-6a).
    Complete,
    /// `0-draft`, or a source whose header the native parser reports on.
    Native,
}

impl Edition {
    /// Read `source`'s header once. A declared edition neither compiler
    /// reads refuses as `unknown_edition` at its literal.
    fn of(source: &Source) -> Result<Self> {
        let Some(declared) = qsl_cst::declared_edition(source.text()) else {
            return Ok(Self::Native);
        };
        match declared.edition.as_str() {
            qsl_cst::EDITION => Ok(Self::Complete),
            crate::syntax::EDITION => Ok(Self::Native),
            other => Err(RunCause::Native(qsl_foundation::diagnostic::error(
                source,
                Code::UnknownEdition,
                qsl_foundation::Phase::Profile,
                declared.span.start,
                declared.span.end,
                format!(
                    "{} declares edition {other:?}; compile reads {:?} and {:?}",
                    source.path(),
                    qsl_cst::EDITION,
                    crate::syntax::EDITION
                ),
            ))),
        }
    }
}

/// Spine-compile a `1-draft` program. Each selected model is a domain
/// package document (format [`DOMAIN_PACKAGE_PROFILE`], the Semantic IR
/// contract it is admitted under), read under its source digest and handed
/// to the spine as FR-056's package input; the program's `model`
/// declarations select from it. Native rule models and clause bindings
/// select native constructs, which a complete-V1 program does not take. The
/// program's `document` and `formal_revision` were validated when intake
/// read it; the v2 wire has no member for them, so they do not reach the
/// bytes (FR-027 Outputs).
fn complete(
    request: &wire::CompileRequest,
    intake: &mut Intake<'_>,
    source: &Source,
) -> Result<Vec<u8>> {
    if request
        .models
        .iter()
        .any(|model| model.format != DOMAIN_PACKAGE_PROFILE)
    {
        return Err(RunCause::CompleteSelection(CompleteSelection::NativeModels));
    }
    if request.program.clauses.is_some() {
        return Err(RunCause::CompleteSelection(CompleteSelection::Clauses));
    }
    let documents = request
        .models
        .iter()
        .map(|model| intake.source(&model.source))
        .collect::<Result<Vec<_>>>()?;
    let packages = qsl_semantics::model::intake::package_input(
        documents
            .iter()
            .map(|document| document.source().text().as_bytes()),
    );
    let libraries = library_sources(request, intake)?;
    // A refusal is rendered over the source its region is in, by that
    // region's source digest: the program's or one library's, wherever in
    // the closure it arose. A refusal with no region names the program.
    let spine_failure = |refusal: qsl_replay::spine::CompileRefusal| {
        let located = refusal
            .region()
            .and_then(|region| {
                std::iter::once(source)
                    .chain(libraries.iter().map(|(_, read)| read.source()))
                    .find(|candidate| candidate.reference().digest() == region.source().digest())
            })
            .unwrap_or(source);
        RunCause::Spine(Box::new(SpineFailure {
            source: located.identity().clone(),
            path: located.path().to_owned(),
            span: refusal.region().and_then(|region| located.render(region)),
            refusal,
        }))
    };
    let dependencies =
        qsl_replay::spine::DependencyInput::new(libraries.iter().map(|(library, read)| {
            qsl_replay::spine::SuppliedLibrary {
                identity: library.identity.clone(),
                version: library.version.clone(),
                source: read.source().identity().clone(),
                path: read.source().path().to_owned(),
                bytes: read.source().text().as_bytes().to_vec(),
            }
        }))
        .map_err(|refusal| {
            spine_failure(qsl_replay::spine::CompileRefusal::DependencyInput(refusal))
        })?;
    qsl_replay::spine::compile(
        source.identity().clone(),
        source.path(),
        source.text().as_bytes(),
        &packages,
        &dependencies,
        qsl_replay::spine::SpineLimits::default(),
    )
    .map(|compiled| compiled.emitted.bytes().to_vec())
    .map_err(|refusal| spine_failure(*refusal))
}

/// The request's `libraries`, each with its source read under its source
/// digest (FR-027, ADR-015 D-1). An empty identity or version refuses as
/// `invalid-request` before any library file is read.
fn library_sources<'r>(
    request: &'r wire::CompileRequest,
    intake: &mut Intake<'_>,
) -> Result<Vec<(&'r wire::Library, FormalSource)>> {
    for library in &request.libraries {
        if library.identity.is_empty() {
            return Err(RunCause::Libraries(LibrarySelection::EmptyIdentity));
        }
        if library.version.is_empty() {
            return Err(RunCause::Libraries(LibrarySelection::EmptyVersion));
        }
    }
    request
        .libraries
        .iter()
        .map(|library| Ok((library, intake.source(&library.source)?)))
        .collect()
}

/// A request selection a complete-V1 (`1-draft`) program does not take.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum CompleteSelection {
    /// The request selects a model source in a native rule-model format.
    NativeModels,
    /// The program selects clause bindings.
    Clauses,
}

impl std::fmt::Display for CompleteSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NativeModels => "native rule-model sources",
            Self::Clauses => "clause bindings",
        })
    }
}

/// Why a native-compile/1 request's `libraries` refuse (FR-027).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum LibrarySelection {
    /// A `0-draft` program takes no library.
    NativeProgram,
    /// A library has an empty identity.
    EmptyIdentity,
    /// A library has an empty version.
    EmptyVersion,
}

impl std::fmt::Display for LibrarySelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NativeProgram => "a 0-draft program takes no library",
            Self::EmptyIdentity => "a library has an empty identity",
            Self::EmptyVersion => "a library has an empty version",
        })
    }
}

/// Compile native-compile/1 sources and export the existing Boolean IR projection.
/// Unsupported clauses refuse the complete export; no runtime inputs are needed.
pub fn lower(path: &Path) -> std::result::Result<Vec<u8>, Box<RunError>> {
    lower_for(path, ProjectionTarget::BooleanOracleV1)
}

/// Export a source-only job through an explicit IR lowering target (FR-033).
pub fn lower_for(
    path: &Path,
    target: ProjectionTarget,
) -> std::result::Result<Vec<u8>, Box<RunError>> {
    compile_with(path, |package| {
        lowering::lower_for(package, target, LoweringLimits::default())
            .map(|projection| projection.bytes().to_vec())
            .map_err(|error| {
                let program = &package.checked().bindings().source;
                RunCause::Lowering {
                    target,
                    package: NativePackageRef::new(package.digest()),
                    program: Box::new(program.into()),
                    location: ProjectionLocation::resolve(program.source(), error.source),
                    error,
                }
            })
    })
}

fn compile_with(
    path: &Path,
    export: impl FnOnce(&NativePackage<'_>) -> Result<Vec<u8>>,
) -> std::result::Result<Vec<u8>, Box<RunError>> {
    with_request(path, |directory, bytes, _digest| {
        let request: wire::CompileRequest = request(bytes)?;
        compilation::source_only(&request.program)?;
        let mut intake = intake(
            directory,
            FileCounts {
                programs: 1,
                models: request.models.len(),
                snapshots: 0,
                invocations: 0,
                packages: 0,
            },
        )?;
        let models = compilation::models(&mut intake, &request.models)?;
        let package = compilation::package(&mut intake, &request.program, &models)?;
        export(&package)
    })
}

// Both commands retain the same exact request identity on any later failure.
fn with_request<T>(
    path: &Path,
    action: impl FnOnce(&Path, &[u8], ByteDigest) -> Result<T>,
) -> std::result::Result<T, Box<RunError>> {
    let bytes = read_file(path, REQUEST_BYTES).map_err(|cause| {
        Box::new(RunError {
            request_digest: None,
            cause,
        })
    })?;
    let digest = ByteDigest::of(&bytes);
    action(path.parent().unwrap_or(Path::new(".")), &bytes, digest).map_err(|cause| {
        Box::new(RunError {
            request_digest: Some(digest),
            cause,
        })
    })
}

fn request<T: wire::RequestKind>(bytes: &[u8]) -> Result<T> {
    let Object(envelope): Object<wire::Envelope<'_>> = serde_json::from_slice(bytes)?;
    if envelope.format != T::FORMAT.as_str() {
        return Err(RunCause::Format);
    }
    let Object(request) = serde_json::from_str(envelope.request.get())?;
    Ok(request)
}

fn check_file_count(counts: FileCounts) -> Result<()> {
    let mut remaining_files = FILES;
    for (category, requested) in [
        (FileCategory::Program, counts.programs),
        (FileCategory::Model, counts.models),
        (FileCategory::Snapshot, counts.snapshots),
        (FileCategory::Invocation, counts.invocations),
        (FileCategory::Package, counts.packages),
    ] {
        remaining_files = remaining_files
            .checked_sub(requested)
            .ok_or(RunCause::Limit(LimitKind::SelectedFiles {
                category,
                requested,
                remaining: remaining_files,
                maximum: FILES,
            }))?;
    }
    Ok(())
}

fn intake(directory: &Path, counts: FileCounts) -> Result<Intake<'_>> {
    check_file_count(counts)?;
    Ok(Intake {
        directory,
        remaining: TOTAL_BYTES,
    })
}

fn run_bytes(directory: &Path, bytes: &[u8], digest: ByteDigest) -> Result<RunResult> {
    let request: wire::Request = request(bytes)?;
    let mut intake = intake(
        directory,
        FileCounts {
            // A library source is a program source a `1-draft` program
            // imports, and counts toward the same file limits (FR-100,
            // mirroring FR-027).
            programs: request.libraries.len().saturating_add(1),
            models: request.models.len(),
            snapshots: request.snapshots.as_ref().map_or(0, Vec::len),
            invocations: request.invocations.as_ref().map_or(0, Vec::len),
            packages: usize::from(request.package.is_some()),
        },
    )?;
    // The program source is read exactly once, regardless of extraction
    // (FND-004/FND-011): its declared edition decides the runner before any
    // model or library source is read (FR-100), with the same reader
    // `compile` uses. Extraction (FR-031) is a `run`-only, native-only
    // feature with its own source frontend (I3) over these same bytes
    // (`extraction::select`'s `Selected::compile` reads `program.source`
    // again itself); a `1-draft` source carrying `extraction` refuses
    // (`run_complete`'s `CompleteRunSelection::Extraction`) rather than
    // running natively.
    let source = intake.source(&request.program.source)?;
    match Edition::of(source.source())? {
        Edition::Native => run_native(&digest, &mut intake, &request, source),
        Edition::Complete => run_complete(&digest, &mut intake, &request, source),
    }
}

/// FR-026: run a `0-draft` (or no-edition) program natively, or (with the
/// `quire-extraction` feature) an extracted binding. `source` is the
/// program source `run_bytes` already read.
fn run_native(
    digest: &ByteDigest,
    intake: &mut Intake<'_>,
    request: &wire::Request,
    source: FormalSource,
) -> Result<RunResult> {
    if request.call.is_some() {
        return Err(RunCause::NativeRunSelection(NativeRunSelection::Call));
    }
    if !request.libraries.is_empty() {
        return Err(RunCause::NativeRunSelection(NativeRunSelection::Libraries));
    }
    let Some(selection) = &request.selection else {
        return Err(RunCause::NativeRunSelection(
            NativeRunSelection::MissingSelection,
        ));
    };
    let Some(snapshots) = &request.snapshots else {
        return Err(RunCause::NativeRunSelection(
            NativeRunSelection::MissingSnapshots,
        ));
    };
    let Some(invocations) = &request.invocations else {
        return Err(RunCause::NativeRunSelection(
            NativeRunSelection::MissingInvocations,
        ));
    };
    let selected = compilation::RunSelection::new(request)?;
    let models = compilation::models(intake, &request.models)?;
    // `source` (read once, above, for edition detection) is threaded
    // through to `RunSelection::compile`, so a selected package's own
    // `selected_package` does not read `program.source` a second time
    // (FND-011). The extraction arm ignores it: it reads its own source
    // through the I3 adapter instead.
    let package = selected.compile(intake, &models, source)?;
    let input = runtime_input(intake, snapshots, invocations)?;
    let bound_selection = selection.bind()?;
    let mut limits = ExecutionLimits::default();
    if let Some(value) = request.limits.validation_work {
        limits.validation.work = value;
    }
    if let Some(value) = request.limits.expression_steps {
        limits.evaluation.expression_steps = value;
    }
    let report = runtime::execute(package.native(), input, bound_selection, limits, || false);
    output::report(*digest, selection, &models, &package, &report)
}

/// FR-100: run a `1-draft` program's named function through the spine.
/// `source` is the program source, already read by `run_bytes`.
fn run_complete(
    digest: &ByteDigest,
    intake: &mut Intake<'_>,
    request: &wire::Request,
    source: FormalSource,
) -> Result<RunResult> {
    if request.selection.is_some() {
        return Err(RunCause::CompleteRunSelection(
            CompleteRunSelection::Selection,
        ));
    }
    if request.snapshots.is_some() {
        return Err(RunCause::CompleteRunSelection(
            CompleteRunSelection::Snapshots,
        ));
    }
    if request.invocations.is_some() {
        return Err(RunCause::CompleteRunSelection(
            CompleteRunSelection::Invocations,
        ));
    }
    if request.package.is_some() {
        return Err(RunCause::CompleteRunSelection(
            CompleteRunSelection::Package,
        ));
    }
    if request.limits.validation_work.is_some() {
        return Err(RunCause::CompleteRunSelection(
            CompleteRunSelection::ValidationWorkLimit,
        ));
    }
    if request.limits.expression_steps.is_some() {
        return Err(RunCause::CompleteRunSelection(
            CompleteRunSelection::ExpressionStepsLimit,
        ));
    }
    if request.program.clauses.is_some() {
        return Err(RunCause::CompleteRunSelection(
            CompleteRunSelection::Clauses,
        ));
    }
    #[cfg(feature = "quire-extraction")]
    if request.program.extraction.is_some() {
        return Err(RunCause::CompleteRunSelection(
            CompleteRunSelection::Extraction,
        ));
    }
    if request
        .models
        .iter()
        .any(|model| model.format != DOMAIN_PACKAGE_PROFILE)
    {
        return Err(RunCause::CompleteRunSelection(
            CompleteRunSelection::NativeModel,
        ));
    }
    let Some(call) = &request.call else {
        return Err(RunCause::CompleteRunSelection(CompleteRunSelection::NoCall));
    };
    let documents = request
        .models
        .iter()
        .map(|model| intake.source(&model.source))
        .collect::<Result<Vec<_>>>()?;
    let packages = qsl_semantics::model::intake::package_input(
        documents
            .iter()
            .map(|document| document.source().text().as_bytes()),
    );
    let libraries = request
        .libraries
        .iter()
        .map(|library| Ok((library, intake.source(&library.source)?)))
        .collect::<Result<Vec<(&wire::Library, FormalSource)>>>()?;
    let dependencies =
        qsl_replay::spine::DependencyInput::new(libraries.iter().map(|(library, read)| {
            qsl_replay::spine::SuppliedLibrary {
                identity: library.identity.clone(),
                version: library.version.clone(),
                source: read.source().identity().clone(),
                path: read.source().path().to_owned(),
                bytes: read.source().text().as_bytes().to_vec(),
            }
        }))
        .map_err(|refusal| {
            spine_run_failure(
                &source,
                &libraries,
                qsl_replay::spine::CompileRefusal::DependencyInput(refusal),
            )
        })?;
    let arguments = call
        .arguments
        .iter()
        .map(|argument| qsl_replay::spine::CallArgument {
            parameter: argument.parameter.clone(),
            value: argument.value,
        })
        .collect();
    let spine_call = qsl_replay::spine::Call {
        function: call.function.clone(),
        arguments,
        accounting: qsl_replay::spine::default_accounting(call.work_units.unwrap_or(1_000_000)),
    };
    let (package_id, outcome) = qsl_replay::spine::run(
        source.source().identity().clone(),
        source.source().path(),
        source.source().text().as_bytes(),
        &packages,
        &dependencies,
        qsl_replay::spine::SpineLimits::default(),
        &spine_call,
    )
    .map_err(|refusal| match *refusal {
        qsl_replay::spine::RunRefusal::Compile(compile_refusal) => {
            spine_run_failure(&source, &libraries, *compile_refusal)
        }
        other => RunCause::SpineRun(Box::new(other)),
    })?;
    output::spine_run_result(*digest, package_id, &source, &call.function, outcome)
}

/// A spine compile refusal from a `1-draft` `run`, located over the source
/// its region is in (the program's or one library's), the same rendering
/// `complete` uses for `compile` (FR-100 mirrors FR-027).
fn spine_run_failure(
    source: &FormalSource,
    libraries: &[(&wire::Library, FormalSource)],
    refusal: qsl_replay::spine::CompileRefusal,
) -> RunCause {
    let located = refusal
        .region()
        .and_then(|region| {
            std::iter::once(source)
                .chain(libraries.iter().map(|(_, read)| read))
                .find(|candidate| {
                    candidate.source().reference().digest() == region.source().digest()
                })
        })
        .unwrap_or(source);
    RunCause::Spine(Box::new(SpineFailure {
        source: located.source().identity().clone(),
        path: located.source().path().to_owned(),
        span: refusal
            .region()
            .and_then(|region| located.source().render(region)),
        refusal,
    }))
}

fn read_artifacts<T, U>(
    intake: &mut Intake<'_>,
    selected: &[wire::FileSelection<T>],
    limits: ArtifactLimits,
    read: impl Fn(&T, &[u8], ArtifactLimits) -> std::result::Result<U, Box<InputReadError>>,
) -> Result<Vec<U>> {
    selected
        .iter()
        .map(|selected| {
            let bytes = intake.file(&selected.file, limits.artifact_bytes)?;
            Ok(read(&selected.reference, &bytes, limits)?)
        })
        .collect()
}

fn runtime_input(
    intake: &mut Intake<'_>,
    snapshots: &[wire::FileSelection<runtime::SnapshotRef>],
    invocations: &[wire::FileSelection<runtime::InvocationRef>],
) -> Result<RuntimeInput> {
    let limits = ArtifactLimits::default();
    Ok(RuntimeInput {
        snapshots: read_artifacts(intake, snapshots, limits, Snapshot::read_verified)?,
        invocations: read_artifacts(intake, invocations, limits, Invocation::read_verified)?,
    })
}
