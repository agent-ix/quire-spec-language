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

    /// `source` is the program source, already read by the caller (FND-011:
    /// a selected package's own construction does not read `program.source`
    /// a second time). The extraction arm ignores it: it reads its own
    /// source through the I3 adapter instead.
    pub fn compile<'model>(
        self,
        intake: &mut Intake<'_>,
        models: &'model [NativeModel],
        source: FormalSource,
    ) -> Result<RunPackage<'model>> {
        match self {
            Self::Native(request) => Ok(RunPackage::Native(Box::new(match &request.package {
                Some(selected) => {
                    selected_package(intake, selected, &request.program, models, source)?
                }
                None => package_of(source, &request.program, models)?,
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

pub(super) fn selected_package<'model>(
    intake: &mut Intake<'_>,
    selected: &wire::SelectedPackage,
    program: &wire::Program,
    models: &'model [NativeModel],
    source: FormalSource,
) -> Result<NativePackage<'model>> {
    let expected = NativePackageRef::new(selected.digest.parse()?);
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
