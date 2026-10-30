// SPDX-License-Identifier: AGPL-3.0-or-later
//! The facade's compile entry: QSL source to the checked-package bytes QSL
//! emits (FR-060 T12-A). A consumer that reaches QSL only through this crate
//! compiles through [`compile_package`] and never names [`crate::spine`].

use qsl_foundation::SourceIdentity;
use qsl_semantics::library::LibraryName;
use qsl_semantics::model::intake::package_input;

use crate::bounds::ReplayLimits;
use crate::execute::{request_limits, run_spine_with_execution_points, Recompiled, ReplayRefusal};
use crate::request::StageLimits;
use crate::spine::{self, DependencyInput};

/// One source unit compiled and emitted: its `quire.checked-package/v2`
/// bytes.
#[derive(Clone, Debug)]
pub struct CompiledPackage {
    emitted: spine::EmittedUnit,
}

impl CompiledPackage {
    pub(crate) fn new(emitted: spine::EmittedUnit) -> Self {
        Self { emitted }
    }

    /// The emitted `quire.checked-package/v2` bytes.
    pub fn bytes(&self) -> &[u8] {
        self.emitted.package().bytes()
    }

}

/// Compile `bytes`, displayed as `path` and labelled `source`, with
/// `packages` -- each a domain package document, keyed by its own
/// package identity as [`crate::call_site`] and `replay` key theirs -- as
/// I1's package input and `dependencies` as the dependency input, through the spine's parse, select, check and emit
/// stages that [`crate::replay`] recompiles through, and return the emitted
/// package.
///
/// `package_name` is the package manifest's top-level name, the library
/// name the unit's nodes are keyed under; it has no default.
///
/// `limits` are read as a replay request's stage limits are: each setting
/// at its entry or its published default, with the default reader limit
/// bounding the source bytes. `replay_limits` carries `replay.input_bytes`
/// as [`crate::replay`]'s does, so a caller can raise it.
/// The refusals are the replay recompile's: [`ReplayRefusal::Recompile`]
/// with the stage's catalog code for a source that does not compile,
/// [`ReplayRefusal::DependencyInput`] for a library that shares the unit's
/// owner, and [`ReplayRefusal::LimitAboveReader`] for an S1 limit above the
/// reader limit.
pub fn compile_package<'a>(
    source: SourceIdentity,
    package_name: String,
    path: &str,
    bytes: &[u8],
    packages: impl IntoIterator<Item = &'a [u8]>,
    dependencies: &DependencyInput,
    limits: StageLimits,
    replay_limits: ReplayLimits,
) -> Result<CompiledPackage, ReplayRefusal> {
    compile_package_with_execution_points(
        source,
        package_name,
        path,
        bytes,
        packages,
        dependencies,
        &spine::StateClauseExecutionPoints::new(),
        limits,
        replay_limits,
    )
}

/// [`compile_package`] with accepted state-clause execution points keyed by
/// exact authored state clause name (FR-104-AC-15), checked through
/// `spine::check_with_execution_points`, so the emitted clauses carry them.
/// A point for no authored clause, or one the clause kind does not accept,
/// is the checker's refusal and no package is emitted.
#[allow(
    clippy::too_many_arguments,
    reason = "compile_package's arguments plus the accepted points"
)]
pub fn compile_package_with_execution_points<'a>(
    source: SourceIdentity,
    package_name: String,
    path: &str,
    bytes: &[u8],
    packages: impl IntoIterator<Item = &'a [u8]>,
    dependencies: &DependencyInput,
    execution_points: &spine::StateClauseExecutionPoints,
    limits: StageLimits,
    replay_limits: ReplayLimits,
) -> Result<CompiledPackage, ReplayRefusal> {
    let compiled = recompile_unit_with_execution_points(
        source,
        package_name,
        path,
        bytes,
        packages,
        dependencies,
        execution_points,
        limits,
        replay_limits,
    )?;
    Ok(CompiledPackage::new(compiled.emitted))
}

/// The one compile [`compile_package`] and [`crate::compile_admitted`] both
/// run: the spine over `bytes`, keeping the checked unit.
pub(crate) fn recompile_unit<'a>(
    source: SourceIdentity,
    package_name: String,
    path: &str,
    bytes: &[u8],
    packages: impl IntoIterator<Item = &'a [u8]>,
    dependencies: &DependencyInput,
    limits: StageLimits,
    replay_limits: ReplayLimits,
) -> Result<Recompiled, ReplayRefusal> {
    recompile_unit_with_execution_points(
        source,
        package_name,
        path,
        bytes,
        packages,
        dependencies,
        &spine::StateClauseExecutionPoints::new(),
        limits,
        replay_limits,
    )
}

#[allow(
    clippy::too_many_arguments,
    reason = "recompile_unit's arguments plus the accepted points"
)]
fn recompile_unit_with_execution_points<'a>(
    source: SourceIdentity,
    package_name: String,
    path: &str,
    bytes: &[u8],
    packages: impl IntoIterator<Item = &'a [u8]>,
    dependencies: &DependencyInput,
    execution_points: &spine::StateClauseExecutionPoints,
    limits: StageLimits,
    replay_limits: ReplayLimits,
) -> Result<Recompiled, ReplayRefusal> {
    let limits = request_limits(
        &limits,
        crate::limits::AccountingLimits::default().0,
        replay_limits,
    )?;
    dependencies
        .check_unit_owner(&source)
        .map_err(ReplayRefusal::DependencyInput)?;
    let package = LibraryName::new(package_name).map_err(ReplayRefusal::InvalidPackageName)?;
    run_spine_with_execution_points(
        &source,
        &package,
        path,
        bytes,
        &package_input(packages)
            .map_err(|refusal| ReplayRefusal::DomainPackages(Box::new(refusal)))?,
        dependencies,
        execution_points,
        limits.spine,
    )
}
