// SPDX-License-Identifier: AGPL-3.0-or-later
//! The spine's four front-end operations composed over one source with the
//! default limits and a handle nobody cancels, for the tests that compare a
//! command's output with the library's.

use std::collections::BTreeMap;

use qsl_foundation::SourceIdentity;
use qsl_replay::spine::{
    check, package, parse, select, CompileRefusal, DependencyInput, EmittedUnit, LockEvidence,
    PackageLimits, ParseRequest, SpineLimits,
};
use quire_exact::Cancel;

/// `bytes`, labelled `source` and displayed as `path`, through `parse`,
/// `select`, `check` and `package`.
pub fn emitted(
    source: SourceIdentity,
    path: &str,
    bytes: &[u8],
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    dependencies: &DependencyInput,
) -> Result<EmittedUnit, Box<CompileRefusal>> {
    let cancel = Cancel::new();
    let limits = SpineLimits::default();
    let refusal = |failure| {
        qsl_replay::spine::refusal_or_fault(failure)
            .unwrap_or_else(|fault| panic!("the front end faulted: {fault:?}"))
    };
    let parsed = parse(
        &ParseRequest {
            source: &source,
            path,
            bytes,
        },
        limits.source,
        &cancel,
    )
    .map_err(refusal)?
    .into_value();
    let models = select(
        &parsed,
        packages,
        limits.model,
        &cancel,
    )
    .map_err(refusal)?
    .into_value();
    let checked = check(
        &parsed,
        &models,
        dependencies,
        &LockEvidence::default(),
        limits,
        &cancel,
    )
    .map_err(refusal)?
    .into_value();
    Ok(package(&checked, PackageLimits::default(), &cancel)
        .map_err(refusal)?
        .into_value())
}
