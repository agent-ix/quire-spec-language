// SPDX-License-Identifier: AGPL-3.0-only
//! FR-026/027: source-model admission and the shared static compiler pipeline.

use super::{wire, Intake, Result, RunCause};
use crate::checking::{check, CheckBindings, CheckLimits};
use crate::formal_source::FormalSource;
use crate::model_source::{self, ModelSourceLimits};
use crate::native_model::{ModelLimits, NativeModel};
use crate::package::{
    NativePackage, NativePackageRef, PackageLimits, PackageReadLimits, PackageSupport,
};
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
    let checked = check(linked, bindings(source, program)?, CheckLimits::default())?;
    Ok(NativePackage::new(checked, PackageLimits::default())?)
}

fn bindings(source: FormalSource, program: &wire::Program) -> Result<CheckBindings> {
    let clauses = program
        .clauses
        .iter()
        .map(wire::Binding::bind)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(CheckBindings { source, clauses })
}

pub(super) fn selected_package<'model>(
    intake: &mut Intake<'_>,
    selected: &wire::SelectedPackage,
    program: &wire::Program,
    models: &'model [NativeModel],
) -> Result<NativePackage<'model>> {
    let expected = NativePackageRef::new(selected.digest.parse()?);
    let source = intake.source(&program.source)?;
    let bindings = bindings(source, program)?;
    let limits = PackageReadLimits::default();
    let bytes = intake.file(&selected.file, limits.package.artifact_bytes)?;
    NativePackage::read_verified(
        &bytes,
        expected,
        bindings,
        models,
        &PackageSupport::default(),
        limits,
    )
    .map_err(|error| RunCause::SelectedPackage {
        file: selected.file.clone(),
        expected,
        error,
    })
}
