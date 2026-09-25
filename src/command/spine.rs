// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-027: spine `compile` (ADR-011 §5, §7.3 M-6a). Complete-V1 source
//! enters S1 and goes through the stage APIs in DAG order: S1
//! (`qsl_cst::parse`), S2 (`qsl_forms::build_unit`), the FR-091 assembler,
//! S3 (`PackageDeclarations::check`), S4 (`CheckedPackage::link`) and the v2
//! emitter (`qsl_package::emit_checked`). This module calls them and makes
//! no semantic decision.

use qsl_cst::CompleteDiagnostic;
use qsl_forms::{build_unit, FormsFailure, FormsLimits};
use qsl_foundation::source::provenance::{RawSourceRef, SourceRegion};
use qsl_foundation::{Code, SourceIdentity, Span};
use qsl_package::{emit_checked, CheckedPackage, EmitRefusal, OmittedNode};
use qsl_semantics::check::{AssemblyRefusal, CheckRefusal, CheckingLimits, PackageDeclarations};

/// Why spine `compile` wrote no `quire.checked-package/v2` bytes. Each
/// variant is the stage that refused, with its typed cause and, where the
/// stage records one, the region of the unit it concerns.
#[derive(Debug, thiserror::Error)]
pub enum CompileRefusal {
    /// E1: S1 refused the bytes, or the CST carries a diagnostic or a
    /// recovery, which E2 does not admit. Holds the first diagnostic.
    #[error("{}", .0.message)]
    Source(Box<CompleteDiagnostic>),
    /// E2: the forms stage refused the unit or reached its depth limit.
    #[error("the forms stage refused the unit: {failure:?}")]
    Forms {
        /// The S2 failure.
        failure: FormsFailure,
        /// The region it concerns, when it concerns one.
        region: Option<SourceRegion>,
    },
    /// E3: the FR-091 assembler refused the unit.
    #[error("the assembler refused the unit: {refusal:?}")]
    Assembly {
        /// Every assembly error, never empty.
        refusal: AssemblyRefusal,
        /// The region of the first error.
        region: Option<SourceRegion>,
    },
    /// E3: the checker refused the package.
    #[error("the checker refused the package: {refusals:?}")]
    Check {
        /// Every check refusal, never empty.
        refusals: Vec<CheckRefusal>,
        /// The region of the first refusal, when it names one.
        region: Option<SourceRegion>,
    },
    /// E4: the v2 emitter wrote no bytes.
    #[error("{0}")]
    Emit(EmitRefusal),
    /// E4: the v2 emitter would omit these nodes. A package missing part of
    /// the checked graph is partial output, which E4 never writes
    /// (ADR-011 §2.3).
    #[error("the checked-package/v2 wire would omit {} node(s): {:?}", .0.len(), .0)]
    Omitted(Vec<OmittedNode>),
}

impl CompileRefusal {
    /// The catalog code of this refusal: the stage cause's own.
    pub fn code(&self) -> Code {
        match self {
            Self::Source(diagnostic) => diagnostic.code,
            Self::Forms { failure, .. } => failure.catalog_code(),
            Self::Assembly { refusal, .. } => refusal
                .errors
                .first()
                .and_then(|error| Code::from_code(error.cause.catalog_code().code()))
                // Every assembler cause's code is catalogued (FR-091-AC-21);
                // one that is not is a broken invariant, not a refusal.
                .unwrap_or(Code::RuntimeInvariant),
            Self::Check { refusals, .. } => refusals
                .first()
                .map_or(Code::RuntimeInvariant, |refusal| refusal.cause.code()),
            Self::Emit(refusal) => refusal.code(),
            // An emission path IR's pinned v2 vocabulary does not hold yet.
            Self::Omitted(_) => Code::UnsupportedProjection,
        }
    }

    /// The stage that refused: `source`, `forms`, `assembly`, `check` or
    /// `emit`.
    pub fn stage(&self) -> &'static str {
        match self {
            Self::Source(_) => "source",
            Self::Forms { .. } => "forms",
            Self::Assembly { .. } => "assembly",
            Self::Check { .. } => "check",
            Self::Emit(_) | Self::Omitted(_) => "emit",
        }
    }

    /// The region of the unit this refusal concerns, when its stage records
    /// one.
    pub fn region(&self) -> Option<&SourceRegion> {
        match self {
            Self::Source(diagnostic) => diagnostic.region.as_ref(),
            Self::Forms { region, .. }
            | Self::Assembly { region, .. }
            | Self::Check { region, .. } => region.as_ref(),
            Self::Emit(_) | Self::Omitted(_) => None,
        }
    }
}

/// `span` as a region of `source`.
fn region(source: &RawSourceRef, span: Span) -> Option<SourceRegion> {
    let start = u64::try_from(span.start).ok()?;
    let end = u64::try_from(span.end).ok()?;
    SourceRegion::new(source.clone(), start, end).ok()
}

/// Compile complete-V1 `bytes`, labelled `source` and displayed as `path`,
/// through S1 to S4 into `quire.checked-package/v2` bytes. No lock file or
/// request file is read (ADR-011 §5). It returns the emitted bytes, not the
/// `EmittedPackage`: only the stage constructors hand back a stage type
/// (FR-087-AC-2).
pub fn compile(
    source: SourceIdentity,
    path: &str,
    bytes: &[u8],
) -> Result<Vec<u8>, Box<CompileRefusal>> {
    let parsed = qsl_cst::parse(source, path, bytes, qsl_cst::Limits::default())
        .map_err(|diagnostic| Box::new(CompileRefusal::Source(diagnostic)))?;
    if let Some(first) = parsed.diagnostics().first() {
        return Err(Box::new(CompileRefusal::Source(Box::new(first.clone()))));
    }
    let raw = parsed.source().reference().clone();
    let unit = build_unit(&parsed, FormsLimits::default()).map_err(|failure| {
        let span = match &failure {
            FormsFailure::Refused(refusal) => refusal.span,
            FormsFailure::Limit { span, .. } => Some(*span),
        };
        Box::new(CompileRefusal::Forms {
            region: span.and_then(|span| region(&raw, span)),
            failure,
        })
    })?;
    let declarations = PackageDeclarations::assemble(raw.clone(), unit).map_err(|refusal| {
        Box::new(CompileRefusal::Assembly {
            region: refusal
                .errors
                .first()
                .and_then(|error| region(&raw, error.span)),
            refusal,
        })
    })?;
    let regions = declarations.clone();
    let graph = declarations
        .check(CheckingLimits::default())
        .map_err(|refusals| {
            Box::new(CompileRefusal::Check {
                region: refusals
                    .first()
                    .and_then(|refusal| regions.region(&refusal.location)),
                refusals,
            })
        })?;
    let emission = emit_checked(&CheckedPackage::link(graph))
        .map_err(|refusal| Box::new(CompileRefusal::Emit(refusal)))?;
    if !emission.omitted().is_empty() {
        return Err(Box::new(CompileRefusal::Omitted(
            emission.omitted().to_vec(),
        )));
    }
    Ok(emission.package().bytes().to_vec())
}

#[cfg(test)]
mod tests {
    use super::CompileRefusal;
    use ix_trace_rs::trace;
    use qsl_foundation::Code;

    /// FR-027-AC-8 (TC-435 step 6): an emission that would omit part of the
    /// checked graph refuses at the emit stage as `unsupported_projection`.
    /// No complete-V1 source reaches it through the CLI yet, since every
    /// construct S3 checks today has a v2 form.
    #[trace("TC-435", "FR-027-AC-8")]
    #[test]
    fn an_omitting_emission_refuses_at_the_emit_stage() {
        let refusal = CompileRefusal::Omitted(Vec::new());
        assert_eq!(refusal.stage(), "emit");
        assert_eq!(refusal.code(), Code::UnsupportedProjection);
        assert!(refusal.region().is_none());
    }
}
