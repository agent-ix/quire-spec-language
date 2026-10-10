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
    let original = emitted_once(source.clone(), path, bytes, packages, dependencies)?;
    assert_format_identity(&original, source, path, bytes, packages, dependencies);
    Ok(original)
}

/// Trace: FR-003-AC-9
/// Every successful fixture using this actual S1–S4 seam also checks its
/// formatted bytes under the same model and dependency selections.
fn assert_format_identity(
    original: &EmittedUnit,
    source: SourceIdentity,
    path: &str,
    bytes: &[u8],
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    dependencies: &DependencyInput,
) {
    let parsed = qsl_cst::parse(source.clone(), path, bytes, qsl_cst::Limits::default()).unwrap();
    let formatted = qsl_cst::format::format(&parsed).unwrap();
    let mut formatted_source = source;
    formatted_source.revision.push_str("-formatted");
    let reparsed = qsl_cst::parse(
        formatted_source.clone(),
        path,
        formatted.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .unwrap();
    assert_eq!(
        qsl_cst::format::format(&reparsed).unwrap(),
        formatted,
        "{path}: second pass"
    );
    let checked = emitted_once(
        formatted_source,
        path,
        formatted.as_bytes(),
        packages,
        dependencies,
    )
    .unwrap_or_else(|refusal| panic!("{path}: formatted fixture must check: {refusal}"));
    assert_eq!(
        original.sources()[0].text().as_bytes(),
        bytes,
        "{path}: original source retained"
    );
    assert_eq!(
        checked.sources()[0].text(),
        formatted,
        "{path}: formatted source retained"
    );
    assert_eq!(
        checked.package().package_id(),
        original.package().package_id(),
        "{path}: formatting changed checked package identity"
    );
}

fn emitted_once(
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
    let models = select(&parsed, packages, limits.model, &cancel)
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
