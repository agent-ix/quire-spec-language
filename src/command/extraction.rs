// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-031: select one authored binding and invoke the actual Quire consumer.

use super::{wire, Intake, Result, RunCause};
use crate::formal_source::FormalSource;
use crate::native_model::NativeModel;
use crate::quire_source::{
    self, ExtractedPackage, Selection, CONTRACT_VERSION, SEMANTIC_CORE_VERSION,
};
use crate::Code;
use quire_rs::semantic::{read_semantic_block, BundleIndex, SemanticContext, SemanticFailure};
use serde_json::json;

/// Unsupported combinations at the extracted-command boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ExtractionMode {
    /// Source-only export does not admit an extracted program.
    #[error("extraction is supported by run only")]
    CompileCommand,
    /// Selected package bytes and extracted compilation cannot both supply a program.
    #[error("extraction cannot select a package artifact")]
    PackageSelected,
    /// Extracted compilation requires one authored binding.
    #[error("extraction requires one authored clause binding; found {actual}")]
    WrongClauseCount {
        /// Number of selected authored bindings.
        actual: usize,
    },
}

impl ExtractionMode {
    /// Stable catalogued code for the particular unsupported combination.
    pub fn code(self) -> Code {
        match self {
            Self::CompileCommand => Code::ExtractionRequiresRun,
            Self::PackageSelected => Code::ExtractionPackageConflict,
            Self::WrongClauseCount { .. } => Code::ExtractionClauseCount,
        }
    }
}

/// Extraction-specific failures behind the optional command seam.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ExtractionError {
    /// Explicitly unsupported request combination.
    #[error("{0}")]
    Mode(#[from] ExtractionMode),
    /// Pinned Quire rejected the constructed clause-only context.
    /// Its diagnostics remain Quire-owned; wire output includes the selected versions.
    #[error("Quire rejected the clause-only semantic context")]
    Context(Vec<SemanticFailure>),
    /// Actual Quire join or native compilation failure.
    #[error("{0}")]
    Compile(#[from] Box<quire_source::Error>),
}

impl ExtractionError {
    /// Existing native or selected command code, independent of display text.
    pub fn code(&self) -> Code {
        match self {
            Self::Mode(mode) => mode.code(),
            Self::Context(_) => Code::InvalidQuireContext,
            Self::Compile(error) => error.code(),
        }
    }
}

impl From<ExtractionMode> for RunCause {
    fn from(mode: ExtractionMode) -> Self {
        Self::Extraction(Box::new(ExtractionError::Mode(mode)))
    }
}

/// The one selected binding is retained here after preflight; no second count guard.
pub(super) struct Selected<'a> {
    source: &'a wire::SourceFile,
    binding: &'a wire::Binding,
    body: &'a wire::Identity,
}

pub(super) fn select<'a>(
    program: &'a wire::Program,
    extraction: &'a wire::Extraction,
) -> Result<Selected<'a>> {
    let [binding] = program.clauses.as_slice() else {
        return Err(ExtractionMode::WrongClauseCount {
            actual: program.clauses.len(),
        }
        .into());
    };
    Ok(Selected {
        source: &program.source,
        binding,
        body: &extraction.body,
    })
}

pub(super) struct ExtractedRun<'model> {
    pub package: ExtractedPackage<'model>,
    pub original: FormalSource,
}

impl Selected<'_> {
    pub(super) fn compile<'model>(
        self,
        intake: &mut Intake<'_>,
        models: &'model [NativeModel],
    ) -> Result<ExtractedRun<'model>> {
        let selection = Selection {
            binding: self.binding.bind()?,
            body: self.body.bind()?,
        };
        let original = intake.source(self.source)?;
        // Quire validates its own clause-only context; native imports retain model authority.
        let module = read_semantic_block(
            &json!({
                "contract_version": CONTRACT_VERSION, "semantic_core": SEMANTIC_CORE_VERSION,
                "package": selection.binding.requirement.package().as_str(),
                "exports": [], "targets": ["markdown"]
            }),
            &[],
            &|_| false,
        )
        .map_err(|failures| RunCause::Extraction(Box::new(ExtractionError::Context(failures))))?;
        let context =
            SemanticContext::new(module, original.source().path(), BundleIndex::default())
                .with_source_identity(original.source().identity().identity.clone());
        let package = quire_source::compile(
            original.source().clone(),
            &context,
            selection,
            models,
            quire_source::Limits::default(),
        )
        .map_err(|error| RunCause::Extraction(Box::new(ExtractionError::Compile(error))))?;
        Ok(ExtractedRun { package, original })
    }
}
