// SPDX-License-Identifier: AGPL-3.0-or-later
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

pub(super) fn source_only(_program: &wire::Program) -> Result<()> {
    // A minimal build's closed Program decoder already rejects extraction fields.
    #[cfg(feature = "quire-extraction")]
    if _program.extraction.is_some() {
        return Err(super::extraction::ExtractionMode::CompileCommand.into());
    }
    Ok(())
}

pub(super) enum RunSelection<'a> {
    Native(&'a wire::Request),
    #[cfg(feature = "quire-extraction")]
    Extracted(super::extraction::Selected<'a>),
}

impl<'a> RunSelection<'a> {
    pub fn new(request: &'a wire::Request) -> Result<Self> {
        #[cfg(feature = "quire-extraction")]
        if let Some(descriptor) = &request.program.extraction {
            if request.package.is_some() {
                return Err(super::extraction::ExtractionMode::PackageSelected.into());
            }
            return Ok(Self::Extracted(super::extraction::select(
                &request.program,
                descriptor,
            )?));
        }
        Ok(Self::Native(request))
    }

    pub fn compile<'model>(
        self,
        intake: &mut Intake<'_>,
        models: &'model [NativeModel],
    ) -> Result<RunPackage<'model>> {
        match self {
            Self::Native(request) => Ok(RunPackage::Native(Box::new(match &request.package {
                Some(selected) => selected_package(intake, selected, &request.program, models)?,
                None => package(intake, &request.program, models)?,
            }))),
            #[cfg(feature = "quire-extraction")]
            Self::Extracted(selected) => Ok(RunPackage::Extracted(Box::new(
                selected.compile(intake, models)?,
            ))),
        }
    }
}

pub(super) enum RunPackage<'model> {
    Native(Box<NativePackage<'model>>),
    #[cfg(feature = "quire-extraction")]
    Extracted(Box<super::extraction::ExtractedRun<'model>>),
}

impl<'model> RunPackage<'model> {
    pub fn native(&self) -> &NativePackage<'model> {
        match self {
            Self::Native(package) => package,
            #[cfg(feature = "quire-extraction")]
            Self::Extracted(value) => value.package.mapped().native(),
        }
    }
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
