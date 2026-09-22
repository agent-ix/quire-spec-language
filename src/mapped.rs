// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-022: compile one mapped native clause without losing its original source.

use quire_contract_ir as ir;

use crate::checking::{check, CheckBindings, CheckLimits, ClauseBinding};
use crate::formal_source::FormalSource;
use crate::native_model::NativeModel;
use crate::package::{NativePackage, PackageCause, PackageError, PackageLimits};
use crate::source_map::SourceMap;
use crate::{
    link_native, parse_source, Code, Diagnostic, Limits, LinkLimits, LocatedSpan, Phase, Span,
};

/// Existing stage budgets; each stage retains its own implementation ceilings.
#[derive(Clone, Copy, Debug, Default)]
pub struct CompileLimits {
    /// Source, token and native syntax limits.
    pub syntax: Limits,
    /// Model admission and linking limits.
    pub linking: LinkLimits,
    /// Native typing and definedness limits.
    pub checking: CheckLimits,
    /// Native artifact construction limits.
    pub package: PackageLimits,
}

/// Original stage failure, preserved without parsing human-readable messages.
#[derive(Debug, thiserror::Error)]
pub enum CompileCause {
    /// Original syntax, profile, linking or checking diagnostic.
    #[error("{0}")]
    Native(#[from] Box<Diagnostic>),
    /// Original package stage, path, counters and nested cause.
    #[error("{0}")]
    Package(#[from] Box<PackageError>),
}

impl From<Box<crate::linking::LinkingError>> for CompileCause {
    fn from(error: Box<crate::linking::LinkingError>) -> Self {
        Self::Native(Box::new(Diagnostic::from(*error)))
    }
}

impl From<Box<crate::checking::CheckingError>> for CompileCause {
    fn from(error: Box<crate::checking::CheckingError>) -> Self {
        Self::Native(Box::new(Diagnostic::from(*error)))
    }
}

/// A failed mapped request retains both source authorities and authored selection.
#[derive(Debug, thiserror::Error)]
#[error("{cause}")]
pub struct CompileError {
    binding: ClauseBinding,
    mapping: SourceMap,
    /// The actual failed stage; no partial package accompanies this error.
    #[source]
    pub cause: CompileCause,
}

impl CompileError {
    /// Supplied authored identity and intended native clause name.
    pub fn binding(&self) -> &ClauseBinding {
        &self.binding
    }

    /// Exact verified original/body source correspondence from this request.
    pub fn mapping(&self) -> &SourceMap {
        &self.mapping
    }

    /// Stable code from the stage that actually failed.
    pub fn code(&self) -> Code {
        match &self.cause {
            CompileCause::Native(error) => error.code,
            CompileCause::Package(error) => error.code,
        }
    }

    /// Original native diagnostic when the failed stage has a body location.
    pub fn native_diagnostic(&self) -> Option<&Diagnostic> {
        match &self.cause {
            CompileCause::Native(error) => Some(error),
            CompileCause::Package(error) => match &error.cause {
                Some(PackageCause::Native(error)) => Some(error),
                _ => None,
            },
        }
    }

    /// Exact original regions for a native diagnostic; encoding paths have none.
    /// Foreign or inconsistent native coordinates are refused, never guessed.
    pub fn original_spans(&self) -> Result<Option<Vec<LocatedSpan>>, Box<Diagnostic>> {
        let Some(diagnostic) = self.native_diagnostic() else {
            return Ok(None);
        };
        let body = self.mapping.body();
        let span = Span {
            start: diagnostic.span.start.byte,
            end: diagnostic.span.end.byte,
        };
        if &diagnostic.source != body.identity()
            || diagnostic.path != body.path()
            || body.locate(span) != Some(diagnostic.span)
        {
            return Err(crate::diagnostic::error(
                body,
                Code::InvalidSourceMap,
                Phase::SourceMap,
                0,
                0,
                "diagnostic does not belong to the mapped body",
            ));
        }
        self.mapping.map_span(body, span).map(Some)
    }
}

/// A checked native package attached to its exact original document mapping.
#[derive(Debug)]
pub struct MappedPackage<'model> {
    native: NativePackage<'model>,
    mapping: SourceMap,
}

impl<'model> MappedPackage<'model> {
    /// Complete native artifact for existing validation, evaluation and lowering.
    pub fn native(&self) -> &NativePackage<'model> {
        &self.native
    }

    /// Immutable original document, extracted body and checked byte segments.
    pub fn mapping(&self) -> &SourceMap {
        &self.mapping
    }

    /// Resolve a byte span from this package's body into exact original regions.
    pub fn original_spans(&self, span: Span) -> Result<Vec<LocatedSpan>, Box<Diagnostic>> {
        self.mapping
            .map_span(self.native.checked().linked().unit().source(), span)
    }
}

/// Compile an adapter-selected complete native unit containing exactly one clause.
///
/// The caller selects and verifies the original document/digest/region before
/// constructing `mapping`. This function neither extracts Markdown nor changes
/// the adapter's availability metadata. A language tag alone grants no success.
pub fn compile<'model>(
    mapping: SourceMap,
    language: &str,
    binding: ClauseBinding,
    formal_identity: ir::SourceIdentity,
    models: &'model [NativeModel],
    limits: CompileLimits,
) -> Result<MappedPackage<'model>, Box<CompileError>> {
    let compile_native = || -> Result<NativePackage<'model>, CompileCause> {
        let body = mapping.body();
        if language != "ix:native" {
            return Err(crate::diagnostic::error(
                body,
                Code::UnknownLanguage,
                Phase::Profile,
                0,
                0,
                "mapped clause must declare ix:native",
            )
            .into());
        }
        let unit = parse_source(body.clone(), limits.syntax)?;
        let [clause] = unit.clauses() else {
            return Err(crate::diagnostic::error(
                body,
                Code::InvalidModelBinding,
                Phase::Check,
                0,
                0,
                "mapped native unit must contain exactly one clause",
            )
            .into());
        };
        if clause.name.value != binding.name {
            return Err(crate::diagnostic::error(
                body,
                Code::InvalidModelBinding,
                Phase::Check,
                clause.name.span.start,
                clause.name.span.end,
                "mapped native clause name differs from the supplied binding",
            )
            .into());
        }
        let bindings = CheckBindings {
            source: FormalSource::new(body.clone(), formal_identity),
            clauses: vec![binding.clone()],
        };
        let checked = check(
            link_native(unit, models, limits.linking)?,
            bindings,
            limits.checking,
        )?;
        Ok(NativePackage::new(checked, limits.package)?)
    };
    match compile_native() {
        Ok(native) => Ok(MappedPackage { native, mapping }),
        Err(cause) => Err(Box::new(CompileError {
            binding,
            mapping,
            cause,
        })),
    }
}
