// SPDX-License-Identifier: AGPL-3.0-only
//! FR-025: source-aware rule-model intake and native admission.

mod decode;
mod lower;
mod wire;

use crate::formal_source::FormalSource;
use crate::native_model::{ModelLimits, NativeModel, NativeRoles};
use crate::{located_json, Code, Diagnostic};
use decode::decode_model;
use lower::lower_model;
use quire_contract_ir as ir;

/// Category charged against the model source's shared entry ceiling.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntryKind {
    /// A top-level scalar, record, enum, value, object or operation.
    Declaration,
    /// A record field.
    Field,
    /// An enumeration variant.
    Variant,
    /// An operation parameter.
    Parameter,
    /// A field named in an operation's effect frame.
    FrameField,
    /// An object type allowed to be created.
    Created,
    /// An object type allowed to be deleted.
    Deleted,
}

/// Explicit authoring profile for the existing rule-model JSON syntax.
pub const FORMAT: &str = "native-rule-model/1";

/// Inclusive frontend limits, independently clamped to their defaults.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelSourceLimits {
    /// Original source UTF-8 bytes, at most 1 MiB.
    pub source_bytes: usize,
    /// Declaration, field, variant, parameter and frame entries, at most 10,000.
    pub entries: usize,
    /// Active nested type levels, at most 64.
    pub type_depth: usize,
}

impl Default for ModelSourceLimits {
    fn default() -> Self {
        Self {
            source_bytes: 1_048_576,
            entries: 10_000,
            type_depth: 64,
        }
    }
}

impl ModelSourceLimits {
    /// Effective limits after applying the implementation ceilings.
    pub fn bounded(self) -> Self {
        let hard = Self::default();
        Self {
            source_bytes: self.source_bytes.min(hard.source_bytes),
            entries: self.entries.min(hard.entries),
            type_depth: self.type_depth.min(hard.type_depth),
        }
    }
}

/// Original frontend or admission failure; no message parsing determines its kind.
#[derive(Debug, thiserror::Error)]
pub enum ModelSourceCause {
    /// JSON decoding or original occurrence correspondence failed.
    #[error("{0}")]
    Decode(#[from] located_json::Error),
    /// Existing IR constructors rejected the declarations.
    #[error("formal model construction failed: {0:?}")]
    Formal(Vec<ir::Diagnostic>),
    /// Two scalar declarations have the same validated name.
    #[error("duplicate scalar identity: {}", .name.as_str())]
    DuplicateScalar {
        /// Repeated scalar name.
        name: ir::SymbolName,
    },
    /// A scalar use has no corresponding declaration.
    #[error("unknown scalar identity: {}", .name.as_str())]
    UnknownScalar {
        /// Undeclared scalar name.
        name: ir::SymbolName,
    },
    /// Original bytes exceeded the effective source ceiling before decoding.
    #[error("model source byte limit exceeded")]
    SourceBytes {
        /// Original source length.
        actual: usize,
        /// Effective source ceiling.
        maximum: usize,
    },
    /// An entry group exceeded the remaining budget before value decoding.
    #[error("model {kind:?} entry limit exceeded")]
    Entries {
        /// Entry group that could not be charged.
        kind: EntryKind,
        /// Entries requested by this group.
        requested: usize,
        /// Entries remaining before this group.
        remaining: usize,
        /// Effective total entry ceiling.
        maximum: usize,
    },
    /// A nested type exceeded the effective depth ceiling.
    #[error("model type depth limit exceeded")]
    TypeDepth {
        /// Required active depth, counting the root as one.
        actual: usize,
        /// Effective type-depth ceiling.
        maximum: usize,
    },
    /// The caller selected an unknown source profile.
    #[error("unsupported rule-model source profile")]
    UnknownFormat,
    /// Actual native role admission failed.
    #[error("{0}")]
    Admission(#[from] Box<Diagnostic>),
}

impl From<ir::Diagnostic> for ModelSourceCause {
    fn from(error: ir::Diagnostic) -> Self {
        Self::Formal(vec![error])
    }
}

impl From<Vec<ir::Diagnostic>> for ModelSourceCause {
    fn from(errors: Vec<ir::Diagnostic>) -> Self {
        Self::Formal(errors)
    }
}

/// A refused or incomplete source read retains the exact original document.
#[derive(Debug, thiserror::Error)]
#[error("{cause}")]
pub struct ModelSourceError {
    binding: FormalSource,
    source_limits: ModelSourceLimits,
    /// Original typed failure.
    #[source]
    pub cause: ModelSourceCause,
}

impl ModelSourceError {
    /// Effective frontend limits, also retained when native admission fails.
    pub fn source_limits(&self) -> ModelSourceLimits {
        self.source_limits
    }
    /// Exact original native/formal source, including retained bytes.
    pub fn source(&self) -> &FormalSource {
        &self.binding
    }
    /// Stable native classification for the actual failing stage.
    pub fn code(&self) -> Code {
        match &self.cause {
            ModelSourceCause::Decode(located_json::Error::Source(error))
            | ModelSourceCause::Admission(error) => error.code,
            ModelSourceCause::Decode(located_json::Error::ForeignOccurrence) => {
                Code::InvalidSourceMap
            }
            ModelSourceCause::SourceBytes { .. }
            | ModelSourceCause::Entries { .. }
            | ModelSourceCause::TypeDepth { .. }
            | ModelSourceCause::Decode(located_json::Error::ByteLimit { .. }) => {
                Code::ResourceExhausted
            }
            ModelSourceCause::UnknownFormat => Code::UnknownWire,
            ModelSourceCause::Formal(errors)
                if errors.iter().any(|error| {
                    matches!(
                        error.code,
                        ir::DiagnosticCode::SemanticInputTooLarge
                            | ir::DiagnosticCode::CanonicalizationResourceExhausted
                    )
                }) =>
            {
                Code::ResourceExhausted
            }
            ModelSourceCause::Decode(located_json::Error::Json(_))
            | ModelSourceCause::Formal(_)
            | ModelSourceCause::DuplicateScalar { .. }
            | ModelSourceCause::UnknownScalar { .. } => Code::InvalidModelBinding,
        }
    }
    /// A source, lowering or admission budget prevented completion.
    pub fn is_incomplete(&self) -> bool {
        self.code() == Code::ResourceExhausted
    }
}

type Result<T> = std::result::Result<T, ModelSourceCause>;

/// Source-derived inputs to native admission, not an already admitted model.
#[derive(Debug)]
pub struct ModelDraft {
    /// Immutable original document and explicitly supplied formal identity.
    pub source: FormalSource,
    /// Existing IR declarations constructed from this source.
    pub environment: ir::DeclarationEnvironment,
    /// Explicit source-derived nominal, object and operation roles.
    pub roles: NativeRoles,
    /// Original declared license string, retained as metadata without interpretation.
    pub declared_license: String,
    /// Effective frontend ceilings used to produce this draft.
    pub source_limits: ModelSourceLimits,
}

impl ModelDraft {
    /// Apply the existing native admission checks under independent limits.
    pub fn admit(
        self,
        limits: ModelLimits,
    ) -> std::result::Result<NativeModel, Box<ModelSourceError>> {
        let binding = self.source.clone();
        NativeModel::new(self.source, self.environment, self.roles, limits).map_err(|cause| {
            Box::new(ModelSourceError {
                binding,
                source_limits: self.source_limits,
                cause: ModelSourceCause::Admission(cause),
            })
        })
    }
}

/// Decode and lower the selected rule-model profile with exact original loci.
pub fn read(
    source: FormalSource,
    format: &str,
    limits: ModelSourceLimits,
) -> std::result::Result<ModelDraft, Box<ModelSourceError>> {
    let limits = limits.bounded();
    let lower = || {
        if format != FORMAT {
            return Err(ModelSourceCause::UnknownFormat);
        }
        if source.source().text().len() > limits.source_bytes {
            return Err(ModelSourceCause::SourceBytes {
                actual: source.source().text().len(),
                maximum: limits.source_bytes,
            });
        }
        lower_model(decode_model(&source, limits)?, limits.type_depth)
    };
    match lower() {
        Ok((environment, roles, declared_license)) => Ok(ModelDraft {
            source,
            environment,
            roles,
            declared_license,
            source_limits: limits,
        }),
        Err(cause) => Err(Box::new(ModelSourceError {
            binding: source,
            source_limits: limits,
            cause,
        })),
    }
}
