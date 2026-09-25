// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-027: spine `compile` (ADR-011 §5, §7.3 M-6a). Complete-V1 source
//! enters S1 and goes through the stage APIs in DAG order: S1
//! (`qsl_cst::parse`), S2 (`qsl_forms::build_unit`), the FR-091 assembler,
//! S3 (`PackageDeclarations::check`), S4 (`CheckedPackage::link`) and the v2
//! emitter (`qsl_package::emit_checked`). This module calls them and makes
//! no semantic decision.

use qsl_cst::CompleteDiagnostic;
use qsl_forms::{build_unit, FormsCause, FormsFailure, FormsLimits};
use qsl_foundation::source::provenance::{RawSourceRef, SourceRegion};
use qsl_foundation::{Code, SourceIdentity, Span};
use qsl_package::{emit_checked, CheckedPackage, EmitRefusal, OmittedNode};
use qsl_semantics::check::{
    AssemblyCause, AssemblyRefusal, CheckCause, CheckRefusal, CheckingLimits, PackageDeclarations,
};

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
    #[error("the forms stage refused the unit: {}", forms_message(.failure))]
    Forms {
        /// The S2 failure.
        failure: FormsFailure,
        /// The region it concerns, when it concerns one.
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
            Self::Assembly { refusal, .. } => refusal
                .errors
                .first()
                .map_or(Code::RuntimeInvariant, |error| error.cause.code()),
            Self::Check { refusals, .. } => refusals
                .first()
                .map_or(Code::RuntimeInvariant, |refusal| refusal.cause.code()),
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
            Self::Assembly { .. } => SpineStage::Assembly,
            Self::Check { .. } => SpineStage::Check,
            Self::Emit(_) | Self::Omitted(_) => SpineStage::Emit,
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

/// The spine stage a [`CompileRefusal`] comes from.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpineStage {
    /// S1: `qsl_cst::parse`.
    Source,
    /// S2: `qsl_forms::build_unit`.
    Forms,
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

/// A readable account of the assembler's first error, and how many more.
fn assembly_message(refusal: &AssemblyRefusal) -> String {
    let Some(first) = refusal.errors.first() else {
        return "no error recorded".to_owned();
    };
    let message = match &first.cause {
        AssemblyCause::UnresolvedTypeName { name } => format!("no declaration is named `{name}`"),
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
    compile_under(source, path, bytes, CheckingLimits::default())
}

/// [`compile`] with S3 checking under `limits`.
pub(crate) fn compile_under(
    source: SourceIdentity,
    path: &str,
    bytes: &[u8],
    limits: CheckingLimits,
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
            // A type-environment stage limit names no declaration, so it
            // has no region (FR-082, FR-096).
            region: refusal
                .errors
                .first()
                .filter(|error| !matches!(error.cause, AssemblyCause::TypeLimit(_)))
                .and_then(|error| region(&raw, error.span)),
            refusal,
        })
    })?;
    let regions = declarations.regions();
    let graph = declarations.check(limits).map_err(|refusals| {
        Box::new(CompileRefusal::Check {
            region: refusals
                .first()
                .and_then(|refusal| regions.refusal_region(refusal)),
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
    use super::{compile_under, CompileRefusal, SpineStage};
    use ix_trace_rs::trace;
    use qsl_foundation::{Code, SourceIdentity};
    use qsl_semantics::check::CheckingLimits;

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
        let refusal = compile_under(
            SourceIdentity::new("a", "u", "git", "1"),
            "unit.native",
            UNIT.as_bytes(),
            CheckingLimits::new(u64::MAX, 3).unwrap(),
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
