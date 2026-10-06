// SPDX-License-Identifier: AGPL-3.0-or-later
//! The facade's compile entry: QSL source to the checked-package bytes QSL
//! emits (FR-060 T12-A). A consumer that reaches QSL only through this crate
//! compiles through [`compile_package`] and never names [`crate::spine`].

use qsl_foundation::SourceIdentity;
use qsl_semantics::model::intake::package_input;

use crate::bounds::ReplayLimits;
use crate::execute::{request_limits, run_spine, ReplayRefusal};
use crate::request::StageLimits;
use crate::spine::{self, DependencyInput};
use crate::DigestRecord;

/// One source unit compiled and emitted: its `quire.checked-package/v2`
/// bytes and the `package_id` they declare.
#[derive(Clone, Debug)]
pub struct CompiledPackage {
    emitted: spine::EmittedUnit,
}

impl CompiledPackage {
    /// The emitted `quire.checked-package/v2` bytes.
    pub fn bytes(&self) -> &[u8] {
        self.emitted.package().bytes()
    }

    /// The `quire.package.semantic/v2` `package_id` the bytes declare.
    pub fn package_id(&self) -> DigestRecord {
        self.emitted.package().package_id().record()
    }
}

/// Compile `bytes`, displayed as `path` and labelled `source`, with
/// `packages` -- each a domain package document, keyed by its own
/// `sha256-jcs` digest as [`crate::call_site`] and `replay` key theirs -- as
/// I1's package input and `dependencies` as the dependency input, through the spine's parse, select, check and emit
/// stages that [`crate::replay`] recompiles through, and return the emitted
/// package.
///
/// `limits` are read as a replay request's stage limits are: each setting
/// at its entry or its published default, with the default reader limit
/// bounding the source bytes.
/// The refusals are the replay recompile's: [`ReplayRefusal::Recompile`]
/// with the stage's catalog code for a source that does not compile,
/// [`ReplayRefusal::DependencyInput`] for a library that shares the unit's
/// owner, and [`ReplayRefusal::LimitAboveReader`] for an S1 limit above the
/// reader limit.
pub fn compile_package<'a>(
    source: SourceIdentity,
    path: &str,
    bytes: &[u8],
    packages: impl IntoIterator<Item = &'a [u8]>,
    dependencies: &DependencyInput,
    limits: StageLimits,
) -> Result<CompiledPackage, ReplayRefusal> {
    let limits = request_limits(&limits, ReplayLimits::default())?;
    dependencies
        .check_unit_owner(&source)
        .map_err(ReplayRefusal::DependencyInput)?;
    let compiled = run_spine(
        &source,
        path,
        bytes,
        &package_input(packages),
        dependencies,
        limits.spine,
    )?;
    Ok(CompiledPackage {
        emitted: compiled.emitted,
    })
}
