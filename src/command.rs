// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-026/027: local file orchestration around the existing native compiler and runtime.

mod compilation;
#[cfg(feature = "quire-extraction")]
mod extraction;
mod output;
mod projection_error;
mod wire;

#[cfg(feature = "quire-extraction")]
pub use extraction::{ExtractionError, ExtractionMode};
pub use output::NativeResult;
pub use projection_error::{ProjectionLocation, ProjectionSource};

use crate::formal_source::FormalSource;
use crate::lowering::{self, LoweringError, LoweringLimits, ProjectionTarget};
use crate::model_source::ModelSourceError;
use crate::package::{NativePackage, NativePackageRef, PackageError};
use crate::runtime::{
    self, ArtifactLimits, ExecutionLimits, InputReadError, Invocation, RuntimeInput, Snapshot,
};
use crate::serde_object::Object;
use crate::{ByteDigest, Code, Diagnostic, Source};
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
    Extraction(#[from] Box<ExtractionError>),
    /// A command intake ceiling stopped the request.
    #[error("{0} limit exceeded")]
    Limit(LimitKind),
    /// Selected digest spelling is invalid.
    #[error("{0}")]
    Digest(#[from] crate::digest::InvalidDigest),
    /// Existing IR identifier validation failed.
    #[error("invalid request identifier: {0:?}")]
    Identifier(ir::Diagnostic),
    /// Existing source/parser/linker/checker failure.
    #[error("{0}")]
    Native(#[from] Box<Diagnostic>),
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
            Self::Model(error) => error.code(),
            Self::Package(error) | Self::SelectedPackage { error, .. } => error.code,
            Self::Input(error) => error.code,
            Self::Lowering { error, .. } => error.code.code(),
        }
    }

    /// Whether the retained cause prevented completion.
    pub fn is_incomplete(&self) -> bool {
        self.code().is_incomplete()
    }

    /// Native command exit status, on FR-301's six-code contract: request
    /// intake and identifier/digest failures are invalid input (20); a
    /// disposition's incomplete/exhausted classification promotes it to
    /// incomplete (22) in place of invalid input (20); writing the outcome
    /// out is a tool failure (30).
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Output(_) => 30,
            Self::Io { .. } | Self::Json(_) | Self::Digest(_) | Self::Identifier(_) => 20,
            Self::Format
            | Self::Limit(_)
            | Self::Native(_)
            | Self::Model(_)
            | Self::Package(_)
            | Self::SelectedPackage { .. }
            | Self::Lowering { .. }
            | Self::Input(_) => {
                if self.is_incomplete() {
                    22
                } else {
                    20
                }
            }
            #[cfg(feature = "quire-extraction")]
            Self::Extraction(error) => {
                if error.code().is_incomplete() {
                    22
                } else {
                    20
                }
            }
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
    /// Existing usage/refusal/incomplete exit convention, derived from the cause.
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

/// Compile a native-compile/1 source-only job into the exact existing package bytes.
/// No runtime artifact is read or executed; failures expose no partial artifact.
pub fn compile(path: &Path) -> std::result::Result<Vec<u8>, Box<RunError>> {
    compile_with(path, |package| Ok(package.bytes().to_vec()))
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
    let selected = compilation::RunSelection::new(&request)?;
    let mut intake = intake(
        directory,
        FileCounts {
            programs: 1,
            models: request.models.len(),
            snapshots: request.snapshots.len(),
            invocations: request.invocations.len(),
            packages: usize::from(request.package.is_some()),
        },
    )?;
    let models = compilation::models(&mut intake, &request.models)?;
    let package = selected.compile(&mut intake, &models)?;
    let input = runtime_input(&mut intake, &request)?;
    let selection = request.selection.bind()?;
    let mut limits = ExecutionLimits::default();
    if let Some(value) = request.limits.validation_work {
        limits.validation.work = value;
    }
    if let Some(value) = request.limits.expression_steps {
        limits.evaluation.expression_steps = value;
    }
    let report = runtime::execute(package.native(), input, selection, limits, || false);
    output::report(digest, &request.selection, &models, &package, &report)
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

fn runtime_input(intake: &mut Intake<'_>, request: &wire::Request) -> Result<RuntimeInput> {
    let limits = ArtifactLimits::default();
    Ok(RuntimeInput {
        snapshots: read_artifacts(intake, &request.snapshots, limits, Snapshot::read_verified)?,
        invocations: read_artifacts(
            intake,
            &request.invocations,
            limits,
            Invocation::read_verified,
        )?,
    })
}
