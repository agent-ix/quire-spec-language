// SPDX-License-Identifier: AGPL-3.0-or-later
//! A runnable `quire.checked-package/v2` producer for downstream readers.
//!
//! [`emit`] reads a complete-V1 (`1-draft`) QSL source file, the domain
//! package documents its `model` declarations select and the library sources
//! its `import`s name, and returns the bytes
//! [`qsl_replay::compile_package`] emits for them. It adds no emit logic of
//! its own, so the bytes follow the producer as it changes; nothing here
//! freezes a shape or a digest. The `emit_checked_v2` example is a thin
//! caller of [`run`].
//!
//! Every failure (usage, unreadable file, write failure, refusal) exits with
//! the refusal category's code, 20, as `quire-spec` does, unless the
//! producer's catalog code belongs to another category (for example 22 for
//! an incomplete result); the stderr diagnostic tells them apart. No bytes
//! are written on failure.
//!
//! The emitted bytes embed the source path exactly as given: the label
//! `source:<path>` feeds the source digest labels and so the `package_id`.
//! A caller that needs stable bytes passes an identical, fixed,
//! cwd-relative path on every invocation. The same holds for each
//! `--library` path.

use std::ffi::OsString;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use qsl_foundation::source::MAX_SOURCE_BYTES;
use qsl_foundation::{Code, SourceIdentity};
use qsl_replay::{
    compile_package_with_execution_points,
    spine::{DependencyInput, DependencyInputRefusal, StateClauseExecutionPoints, SuppliedLibrary},
    ReplayLimits, ReplayRefusal, StageLimits,
};
use quire_contract_model::ExecutionPoint;
use quire_semantic_value::catalog::Category;

/// The usage line, shown on a malformed command line.
pub const USAGE: &str = "usage: emit_checked_v2 [--model <domain-package-file>]... \
[--library <identity>=<source-file>]... [--execution-points <json-file>] \
[--out <path>] <source-file>";

/// Why no bytes were emitted.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The command line is malformed.
    #[error("{USAGE}")]
    Usage,
    /// A named file could not be read.
    #[error("cannot read {path}: {source}")]
    Read {
        /// The file.
        path: String,
        /// The I/O failure.
        source: std::io::Error,
    },
    /// The output path could not be written.
    #[error("cannot write {path}: {source}")]
    Write {
        /// The file.
        path: String,
        /// The I/O failure.
        source: std::io::Error,
    },
    /// The library selection is invalid.
    #[error("{0}")]
    Dependencies(DependencyInputRefusal),
    /// A domain package or library file is over the source-size ceiling.
    #[error("{path} is over the {limit}-byte file ceiling; it is not truncated")]
    TooLarge {
        /// The file.
        path: String,
        /// The ceiling in bytes.
        limit: usize,
    },
    /// An execution-points file is not a JSON object from authored state
    /// clause name to execution point.
    #[error("execution points {path}: {message}")]
    Points {
        /// The file.
        path: String,
        /// What is wrong with it.
        message: String,
    },
    /// The producer refused the source.
    #[error("{}", without_limit_hint(.0))]
    Producer(Box<ReplayRefusal>),
}

impl Error {
    /// The process exit code: the refusing code's category for a producer or
    /// dependency refusal, the refusal category for a usage or file error.
    pub fn exit_code(&self) -> u8 {
        let code: Option<Code> = match self {
            Self::Producer(refusal) => Some(refusal.code()),
            Self::Dependencies(refusal) => Some(refusal.code()),
            Self::Usage
            | Self::Read { .. }
            | Self::Write { .. }
            | Self::TooLarge { .. }
            | Self::Points { .. } => None,
        };
        code.map_or(Category::Refusal.exit_code(), |code| {
            code.category().exit_code()
        })
    }
}

/// One parsed command line.
#[derive(Debug, Eq, PartialEq)]
pub struct Request {
    /// The `1-draft` source file.
    pub source: PathBuf,
    /// Domain package documents, in the order given.
    pub models: Vec<PathBuf>,
    /// Library sources: the identity an `import` names, and the file.
    pub libraries: Vec<(String, PathBuf)>,
    /// A JSON file of accepted state-clause execution points (FR-104-AC-15):
    /// an object from exact authored clause name to an execution point such
    /// as `{"kind": "post", "operation": "set"}`.
    pub execution_points: Option<PathBuf>,
    /// Where to write the bytes instead of stdout.
    pub out: Option<PathBuf>,
}

impl Request {
    /// Parse `arguments` (without the program name).
    #[ix_trace_rs::string_edge]
    pub fn parse(arguments: &[OsString]) -> Result<Self, Error> {
        let mut models = Vec::new();
        let mut libraries = Vec::new();
        let mut out = None;
        let mut execution_points = None;
        let mut source = None;
        let mut rest = arguments.iter();
        while let Some(argument) = rest.next() {
            match argument.to_str() {
                Some("--model") => models.push(PathBuf::from(rest.next().ok_or(Error::Usage)?)),
                Some("--library") => {
                    let pair = rest.next().and_then(|pair| pair.to_str());
                    let (identity, file) =
                        pair.and_then(|p| p.split_once('=')).ok_or(Error::Usage)?;
                    libraries.push((identity.to_owned(), PathBuf::from(file)));
                }
                Some("--execution-points") if execution_points.is_none() => {
                    execution_points = Some(PathBuf::from(rest.next().ok_or(Error::Usage)?));
                }
                Some("--out") if out.is_none() => {
                    out = Some(PathBuf::from(rest.next().ok_or(Error::Usage)?));
                }
                Some(option) if option.starts_with("--") => return Err(Error::Usage),
                _ if source.is_none() => source = Some(PathBuf::from(argument)),
                _ => return Err(Error::Usage),
            }
        }
        Ok(Self {
            source: source.ok_or(Error::Usage)?,
            models,
            libraries,
            execution_points,
            out,
        })
    }
}

/// The producer's refusal text without its trailing "raise it with
/// `--limit ...`" hint, which names a flag this tool does not have. The
/// refusal itself, its code and its category are unchanged.
fn without_limit_hint(refusal: &ReplayRefusal) -> String {
    let text = refusal.to_string();
    match text.find("; raise it with") {
        Some(at) => text[..at].to_owned(),
        None => text,
    }
}

/// Read at most the source ceiling plus one byte, as `quire-spec` does, so an
/// oversized file reaches the producer's own size refusal without an
/// unbounded read.
///
/// With `refuse_over_cap`, a file over the ceiling is an explicit
/// [`Error::TooLarge`] rather than truncated bytes (a domain package or
/// library would otherwise fail later as a digest or import refusal); the
/// source passes `false` and leaves the refusal to the producer.
fn read(path: &Path, refuse_over_cap: bool) -> Result<Vec<u8>, Error> {
    let failed = |source| Error::Read {
        path: path.display().to_string(),
        source,
    };
    let file = std::fs::File::open(path).map_err(failed)?;
    let ceiling = u64::try_from(MAX_SOURCE_BYTES).unwrap_or(u64::MAX);
    let mut bytes = Vec::new();
    file.take(ceiling.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(failed)?;
    if refuse_over_cap && bytes.len() > MAX_SOURCE_BYTES {
        return Err(Error::TooLarge {
            path: path.display().to_string(),
            limit: MAX_SOURCE_BYTES,
        });
    }
    Ok(bytes)
}

/// The labels a file is compiled under: its own path names it, so a caller
/// supplies no identity of its own.
fn identity(kind: &str, path: &Path) -> SourceIdentity {
    SourceIdentity::new("local", format!("{kind}:{}", path.display()), "file", "1")
}

/// The points file's object, read so a repeated clause name is detected
/// rather than silently last-wins.
struct PointsFile(Vec<(String, serde_json::Value)>);

impl<'de> serde::Deserialize<'de> for PointsFile {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = PointsFile;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("an object from authored clause name to execution point")
            }
            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                mut map: M,
            ) -> Result<PointsFile, M::Error> {
                let mut entries: Vec<(String, serde_json::Value)> = Vec::new();
                while let Some((name, point)) = map.next_entry::<String, serde_json::Value>()? {
                    if entries.iter().any(|(seen, _)| *seen == name) {
                        return Err(serde::de::Error::custom(format!(
                            "clause {name:?} is given more than once"
                        )));
                    }
                    entries.push((name, point));
                }
                Ok(PointsFile(entries))
            }
        }
        deserializer.deserialize_map(Visitor)
    }
}

/// The accepted execution points `path` holds, keyed by authored clause name.
/// An empty object, a repeated clause name, an unknown `kind` or a member the
/// point does not define is an [`Error::Points`] refusal; whether a clause
/// exists, accepts its point and the anchor name is well formed is the
/// checker's, reported as [`Error::Producer`].
fn execution_points(path: &Path) -> Result<StateClauseExecutionPoints, Error> {
    let bad = |message: String| Error::Points {
        path: path.display().to_string(),
        message,
    };
    let bytes = read(path, true)?;
    let PointsFile(entries) = serde_json::from_slice(&bytes).map_err(|e| bad(e.to_string()))?;
    if entries.is_empty() {
        return Err(bad("no execution points".to_owned()));
    }
    let mut points = StateClauseExecutionPoints::new();
    for (clause, wire) in entries {
        let point: ExecutionPoint =
            serde_json::from_value(wire).map_err(|e| bad(format!("{clause}: {e}")))?;
        points.insert(clause, point);
    }
    Ok(points)
}

/// Emit the `quire.checked-package/v2` bytes of `request`'s source through
/// [`qsl_replay::compile_package`].
pub fn emit(request: &Request) -> Result<Vec<u8>, Error> {
    let source = read(&request.source, false)?;
    let models = request
        .models
        .iter()
        .map(|path| read(path, true))
        .collect::<Result<Vec<_>, _>>()?;
    let libraries = request
        .libraries
        .iter()
        .map(|(library, path)| {
            Ok(SuppliedLibrary {
                identity: library.clone(),
                source: identity("library", path),
                path: path.display().to_string(),
                bytes: read(path, true)?,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let dependencies = DependencyInput::new(libraries).map_err(Error::Dependencies)?;
    let points = request
        .execution_points
        .as_deref()
        .map(execution_points)
        .transpose()?
        .unwrap_or_default();
    let compiled = compile_package_with_execution_points(
        identity("source", &request.source),
        &request.source.display().to_string(),
        &source,
        models.iter().map(Vec::as_slice),
        &dependencies,
        &points,
        StageLimits::default(),
        ReplayLimits::default(),
    )
    .map_err(|refusal| Error::Producer(Box::new(refusal)))?;
    Ok(compiled.bytes().to_vec())
}

/// Parse `arguments`, emit, and write the bytes to `--out` or `stdout`.
pub fn run(arguments: &[OsString], stdout: &mut impl std::io::Write) -> Result<(), Error> {
    let request = Request::parse(arguments)?;
    let bytes = emit(&request)?;
    match &request.out {
        Some(path) => std::fs::write(path, &bytes).map_err(|source| Error::Write {
            path: path.display().to_string(),
            source,
        }),
        None => stdout.write_all(&bytes).map_err(|source| Error::Write {
            path: "<stdout>".to_owned(),
            source,
        }),
    }
}
