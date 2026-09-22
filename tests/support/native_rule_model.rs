// SPDX-License-Identifier: AGPL-3.0-or-later
//! IT-005 / FR-025: fixture setup through the public model-source frontend.

use qsl_foundation::{Diagnostic, Source, SourceIdentity};
use quire_contract_ir as ir;
use quire_spec_language::formal_source::FormalSource;
use quire_spec_language::native_model::{ModelLimits, NativeModel, NativeRoles};

/// Separately authored, licensed native rule-model input.
pub const FIXTURE: &str = include_str!("../fixtures/native-rule-model.json");

/// Fixture setup failure, distinct from a native model or checker judgment.
#[derive(Debug, thiserror::Error)]
pub enum FixtureError {
    /// Original source could not be admitted within its input contract.
    #[error("fixture source intake failed: {0}")]
    Source(#[from] Box<Diagnostic>),
    /// Actual public model-source frontend refusal.
    #[error("model source setup failed: {0}")]
    Model(#[from] Box<quire_spec_language::model_source::ModelSourceError>),
    /// Existing IR constructors rejected the supplied declarations.
    #[error("formal fixture construction failed: {0:?}")]
    Formal(Vec<ir::Diagnostic>),
    /// Fixture-specific identity or bounded-construction rules failed.
    #[error("invalid rule-model fixture: {0}")]
    Invalid(&'static str),
}

impl From<ir::Diagnostic> for FixtureError {
    fn from(error: ir::Diagnostic) -> Self {
        Self::Formal(vec![error])
    }
}

impl From<Vec<ir::Diagnostic>> for FixtureError {
    fn from(errors: Vec<ir::Diagnostic>) -> Self {
        Self::Formal(errors)
    }
}

type Result<T> = std::result::Result<T, FixtureError>;

/// Construct a known test symbol; fallible producer input uses try_symbol.
pub fn symbol(name: &str) -> ir::SymbolName {
    try_symbol(name).expect("valid test symbol")
}

fn try_symbol(name: &str) -> Result<ir::SymbolName> {
    Ok(ir::SymbolName::new(name)?)
}

/// Independently produced input to actual native model admission.
pub struct Parts {
    /// Immutable original fixture and explicit formal source identity.
    pub source: FormalSource,
    /// Declarations produced through the existing IR constructors.
    pub environment: ir::DeclarationEnvironment,
    /// Explicit nominal, object and operation roles from the same input.
    pub roles: NativeRoles,
}

impl Parts {
    /// Admit known valid fixture setup; adverse model tests call the API directly.
    pub fn model(self) -> NativeModel {
        NativeModel::new(
            self.source,
            self.environment,
            self.roles,
            ModelLimits::default(),
        )
        .expect("qualified rule-model setup")
    }
}

/// Read the separately authored fixture; setup failures cannot become judgments.
pub fn parts() -> Parts {
    from_text(FIXTURE, "native-rule-model.json", "draft:1")
        .expect("valid source-derived rule-model fixture")
}

/// Orchestrate source intake, occurrence-aware decoding and typed lowering.
pub fn from_text(text: &str, path: &str, revision: &str) -> Result<Parts> {
    from_source(bind_source(text, path, revision)?)
}

/// Lower an explicitly identified original source, including multi-model fixtures.
pub fn from_source(source: FormalSource) -> Result<Parts> {
    let draft = quire_spec_language::model_source::read(
        source,
        quire_spec_language::model_source::FORMAT,
        quire_spec_language::model_source::ModelSourceLimits::default(),
    )?;
    if draft.declared_license != "AGPL-3.0-only" {
        return Err(FixtureError::Invalid("unexpected fixture license"));
    }
    let quire_spec_language::model_source::ModelDraft {
        source,
        environment,
        roles,
        ..
    } = draft;
    Ok(Parts {
        source,
        environment,
        roles,
    })
}

fn bind_source(text: &str, path: &str, revision: &str) -> Result<FormalSource> {
    let native = Source::read(
        SourceIdentity {
            identity: "test:rule-model".into(),
            revision: revision.into(),
        },
        path,
        text.as_bytes(),
        qsl_foundation::source::MAX_SOURCE_BYTES,
    )?;
    let formal = ir::SourceIdentity::new(
        ir::SourceDocumentId::new("RuleModelSource")?,
        ir::SourceRevision::new(1)?,
    );
    Ok(FormalSource::new(native, formal))
}
