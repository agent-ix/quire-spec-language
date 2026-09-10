// SPDX-License-Identifier: AGPL-3.0-only
//! FR-026/027: source-model admission and the shared static compiler pipeline.

use super::{wire, Intake, Result};
use crate::checking::{check, CheckBindings, CheckLimits};
use crate::model_source::{self, ModelSourceLimits};
use crate::native_model::{ModelLimits, NativeModel};
use crate::package::{NativePackage, PackageLimits};
use crate::Limits;

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
