// SPDX-License-Identifier: AGPL-3.0-only
//! FR-026/027: local file orchestration around the existing native compiler and runtime.

mod output;
mod wire;

pub use output::NativeResult;

use crate::checking::{check, CheckBindings, CheckLimits};
use crate::formal_source::FormalSource;
use crate::model_source::{self, ModelSourceError, ModelSourceLimits};
use crate::native_model::{ModelLimits, NativeModel};
use crate::package::{NativePackage, PackageError, PackageLimits};
use crate::runtime::{
    self, ArtifactLimits, ExecutionLimits, InputReadError, Invocation, RuntimeInput, Snapshot,
};
use crate::serde_object::Object;
use crate::{ByteDigest, Code, Diagnostic, Limits, Source, SourceIdentity};
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

/// Command intake ceiling that stopped selected-file processing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum LimitKind {
    /// Per-file or remaining aggregate byte ceiling.
    FileBytes,
    /// Total selected file count.
    SelectedFiles,
}

impl LimitKind {
    /// Stable field spelling retained for native result compatibility.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FileBytes => "file bytes",
            Self::SelectedFiles => "selected files",
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
    /// 0 completed true, 1 completed false/refused, or 3 incomplete.
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
            Self::Limit(_) => Code::ResourceExhausted,
            Self::Digest(_) => Code::InvalidDigest,
            Self::Identifier(_) => Code::InvalidIdentifier,
            Self::Native(error) => error.code,
            Self::Model(error) => error.code(),
            Self::Package(error) => error.code,
            Self::Input(error) => error.code,
        }
    }

    /// Whether the retained cause prevented completion.
    pub fn is_incomplete(&self) -> bool {
        self.code().is_incomplete()
    }

    /// Native command exit status, with usage/I/O distinct from refusal.
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Io { .. }
            | Self::Json(_)
            | Self::Output(_)
            | Self::Digest(_)
            | Self::Identifier(_) => 2,
            Self::Format
            | Self::Limit(_)
            | Self::Native(_)
            | Self::Model(_)
            | Self::Package(_)
            | Self::Input(_) => {
                if self.is_incomplete() {
                    3
                } else {
                    1
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
    fn models(&mut self, selected: &[wire::Model]) -> Result<Vec<NativeModel>> {
        selected
            .iter()
            .map(|selected| {
                let source = self.source(&selected.source)?;
                Ok(
                    model_source::read(source, &selected.format, ModelSourceLimits::default())?
                        .admit(ModelLimits::default())?,
                )
            })
            .collect()
    }

    fn package<'model>(
        &mut self,
        program: &wire::Program,
        models: &'model [NativeModel],
    ) -> Result<NativePackage<'model>> {
        let source = self.source(&program.source)?;
        let unit = crate::parse_source(source.source().clone(), Limits::default())?;
        let linked = crate::link_native(unit, models, crate::LinkLimits::default())?;
        let clauses = program
            .clauses
            .iter()
            .map(wire::Binding::bind)
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let checked = check(
            linked,
            CheckBindings { source, clauses },
            CheckLimits::default(),
        )?;
        Ok(NativePackage::new(checked, PackageLimits::default())?)
    }

    fn file(&mut self, path: &str, ceiling: usize) -> Result<Vec<u8>> {
        let bytes = read_file(&self.directory.join(path), ceiling.min(self.remaining))?;
        self.remaining -= bytes.len();
        Ok(bytes)
    }

    fn source(&mut self, selected: &wire::SourceFile) -> Result<FormalSource> {
        let identity = ir::SourceIdentity::new(
            ir::SourceDocumentId::new(&selected.document)?,
            ir::SourceRevision::new(selected.formal_revision)?,
        );
        let expected = selected.digest.parse()?;
        let bytes = self.file(&selected.file, REQUEST_BYTES)?;
        let source = Source::read_verified(
            SourceIdentity {
                identity: selected.identity.clone(),
                revision: selected.revision.clone(),
            },
            &selected.file,
            &bytes,
            expected,
            REQUEST_BYTES,
        )?;
        Ok(FormalSource::new(source, identity))
    }
}

/// Read a native-run/1 job and execute it locally with fresh, bounded stage state.
pub fn run(path: &Path) -> std::result::Result<RunResult, Box<RunError>> {
    with_request(path, run_bytes)
}

/// Compile a native-compile/1 source-only job into the exact existing package bytes.
/// No runtime artifact is read or executed; failures expose no partial artifact.
pub fn compile(path: &Path) -> std::result::Result<Vec<u8>, Box<RunError>> {
    with_request(path, |directory, bytes, _digest| {
        let request: wire::CompileRequest = request(bytes, "native-compile/1")?;
        let mut intake = intake(directory, &[request.models.len()])?;
        let models = intake.models(&request.models)?;
        let package = intake.package(&request.program, &models)?;
        Ok(package.bytes().to_vec())
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

fn request<T: serde::de::DeserializeOwned>(bytes: &[u8], format: &str) -> Result<T> {
    let Object(envelope): Object<wire::Envelope<'_>> = serde_json::from_slice(bytes)?;
    if envelope.format != format {
        return Err(RunCause::Format);
    }
    let Object(request) = serde_json::from_str(envelope.request.get())?;
    Ok(request)
}

fn intake<'a>(directory: &'a Path, counts: &[usize]) -> Result<Intake<'a>> {
    let mut remaining_files = FILES - 1; // The selected native program also counts.
    for &count in counts {
        remaining_files = remaining_files
            .checked_sub(count)
            .ok_or(RunCause::Limit(LimitKind::SelectedFiles))?;
    }
    Ok(Intake {
        directory,
        remaining: TOTAL_BYTES,
    })
}

fn run_bytes(directory: &Path, bytes: &[u8], digest: ByteDigest) -> Result<RunResult> {
    let request: wire::Request = request(bytes, "native-run/1")?;
    let mut intake = intake(
        directory,
        &[
            request.models.len(),
            request.snapshots.len(),
            request.invocations.len(),
        ],
    )?;
    let models = intake.models(&request.models)?;
    let package = intake.package(&request.program, &models)?;
    let limits = ArtifactLimits::default();
    let snapshots = request
        .snapshots
        .iter()
        .map(|selected| {
            let bytes = intake.file(&selected.file, limits.artifact_bytes)?;
            Ok(Snapshot::read_verified(
                &selected.reference,
                &bytes,
                limits,
            )?)
        })
        .collect::<Result<Vec<_>>>()?;
    let invocations = request
        .invocations
        .iter()
        .map(|selected| {
            let bytes = intake.file(&selected.file, limits.artifact_bytes)?;
            Ok(Invocation::read_verified(
                &selected.reference,
                &bytes,
                limits,
            )?)
        })
        .collect::<Result<Vec<_>>>()?;
    let selection = request.selection.bind()?;
    let mut limits = ExecutionLimits::default();
    if let Some(value) = request.limits.validation_work {
        limits.validation.work = value;
    }
    if let Some(value) = request.limits.expression_steps {
        limits.evaluation.expression_steps = value;
    }
    let report = runtime::execute(
        &package,
        RuntimeInput {
            snapshots,
            invocations,
        },
        selection,
        limits,
        || false,
    );
    output::report(digest, &request.selection, &models, &report)
}
