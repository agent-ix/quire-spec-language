// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-026/027: source-model admission and the shared static compiler pipeline.

use super::{wire, Intake, Result, RunCause};
use crate::checking::{check, CheckBindings, CheckLimits};
use crate::formal_source::FormalSource;
use crate::model_source::{self, ModelSourceLimits};
use crate::native_model::{ModelLimits, NativeModel};
use crate::package::{NativePackage, PackageLimits};
use crate::Limits;

pub(super) fn source_only(_program: &wire::Program) -> Result<()> {
    // A minimal build's closed Program decoder already rejects extraction fields.
    #[cfg(feature = "quire-extraction")]
    if let Some(wire::Extraction { body }) = _program.extraction.as_ref() {
        // The retired selection remains closed and fully decoded before the
        // command refuses it; retain the body identity's wire validation.
        let _ = body;
        return Err(super::extraction::ExtractionMode::CompileCommand.into());
    }
    Ok(())
}
pub(super) fn models(
    intake: &mut Intake<'_>,
    selected: &[wire::Model],
) -> Result<Vec<NativeModel>> {
    selected
        .iter()
        .map(|selected| {
            let source = intake.source(&selected.source)?;
            Ok(
                model_source::read(source, &selected.format, ModelSourceLimits::default())?
                    .admit(ModelLimits::default())?,
            )
        })
        .collect()
}

pub(super) fn package<'model>(
    intake: &mut Intake<'_>,
    program: &wire::Program,
    models: &'model [NativeModel],
) -> Result<NativePackage<'model>> {
    let source = intake.source(&program.source)?;
    package_of(source, program, models)
}

/// The native package of `program`, whose `source` intake has already read.
pub(super) fn package_of<'model>(
    source: FormalSource,
    program: &wire::Program,
    models: &'model [NativeModel],
) -> Result<NativePackage<'model>> {
    let unit = crate::parse_source(source.source().clone(), Limits::default())?;
    let linked = crate::link_native(unit, models, crate::LinkLimits::default())?;
    let checked = check(linked, bindings(source, program)?, CheckLimits::default())?;
    Ok(NativePackage::new(checked, PackageLimits::default())?)
}

fn bindings(source: FormalSource, program: &wire::Program) -> Result<CheckBindings> {
    // FR-026: a `0-draft` program's `clauses` is required (a `1-draft`
    // program never reaches native package construction: `run_complete`/
    // `complete` refuse it before either ever calls this).
    let Some(clauses) = &program.clauses else {
        return Err(RunCause::MissingClauses);
    };
    let clauses = clauses
        .iter()
        .map(wire::Binding::bind)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(CheckBindings { source, clauses })
}
