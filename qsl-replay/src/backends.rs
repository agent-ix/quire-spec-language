// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-296: running a function call on a caller-supplied execution backend.
//!
//! `qsl_replay::spine`'s public surface names no `qsl_eval` path
//! (FR-100-AC-8), so the one entry that takes `qsl_eval`'s backend types
//! lives here, beside the spine rather than in it.

use qsl_eval::execution::{PrepareLimits, SuppliedBackends};
use qsl_foundation::SourceIdentity;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_exact::Cancel;

use crate::spine::{Call, CallOutcome, DependencyInput, RunRefusal, SpineLimits};

/// [`crate::spine::run`] with the execution backends the caller supplies
/// (FR-296) and the `prepare` limits the chosen backend's `prepare` runs
/// under (FR-279, FR-294): `call.backend` names which backend runs the call.
/// A backend that is not supplied refuses before any stage as
/// [`RunRefusal::Unsupported`].
#[allow(
    clippy::too_many_arguments,
    reason = "run's signature (FR-100) plus the caller's backends and prepare limits"
)]
pub fn run_with_backends(
    source: SourceIdentity,
    package_name: String,
    path: &str,
    bytes: &[u8],
    packages: &qsl_semantics::model::intake::PackageInput,
    dependencies: &DependencyInput,
    limits: SpineLimits,
    call: &Call,
    objects: &ObjectEnvironment,
    backends: &SuppliedBackends<'_>,
    prepare: &PrepareLimits,
    cancel: &Cancel,
) -> Result<CallOutcome, Box<RunRefusal>> {
    crate::spine::run_reading(
        source,
        package_name,
        path,
        bytes,
        packages,
        dependencies,
        limits,
        call,
        objects,
        backends,
        prepare,
        cancel,
        |_, _, _| Ok(()),
    )
    .map(|(outcome, ())| outcome)
}
