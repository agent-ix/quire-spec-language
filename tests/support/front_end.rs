// SPDX-License-Identifier: AGPL-3.0-or-later
//! The spine's four front-end operations composed over one source with the
//! default limits and a handle nobody cancels, for the tests that compare a
//! command's output with the library's.

use std::collections::BTreeMap;

use qsl_foundation::SourceIdentity;
use qsl_replay::spine::{
    check, package, parse, select, CheckedUnit, CompileRefusal, DependencyInput, EmittedUnit,
    LockEvidence, PackageLimits, ParseRequest, SpineLimits, SuppliedLibrary,
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
    emitted_with_authority(
        source,
        path,
        bytes,
        packages,
        dependencies,
        SpineLimits::default(),
        &LockEvidence::default(),
    )
}

/// The same identity oracle with the owning fixture's limits and lock evidence.
// Keep source, selected packages, dependencies, limits and lock evidence explicit
// so callers retain each owning fixture's independent admission authority.
#[allow(clippy::too_many_arguments)]
pub fn emitted_with_authority(
    source: SourceIdentity,
    path: &str,
    bytes: &[u8],
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    dependencies: &DependencyInput,
    limits: SpineLimits,
    lock: &LockEvidence,
) -> Result<EmittedUnit, Box<CompileRefusal>> {
    let original = emitted_once(
        source.clone(),
        path,
        bytes,
        packages,
        dependencies,
        limits,
        lock,
    )?;
    assert_format_identity(
        &original,
        source,
        path,
        bytes,
        packages,
        dependencies,
        limits,
        lock,
    );
    Ok(original)
}

/// Trace: FR-003-AC-9
/// Every successful fixture using this actual S1–S4 seam also checks its
/// formatted bytes under the same model and dependency selections.
// The original result and each admission input remain separate so the round trip
// reuses the fixture's actual authority without substituting helper defaults.
#[allow(clippy::too_many_arguments)]
fn assert_format_identity(
    original: &EmittedUnit,
    source: SourceIdentity,
    path: &str,
    bytes: &[u8],
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    dependencies: &DependencyInput,
    limits: SpineLimits,
    lock: &LockEvidence,
) {
    let parsed = qsl_cst::parse(source.clone(), path, bytes, limits.source).unwrap();
    // AC-9 isolates semantic identity; the AC-4/5/6 tests exercise exhaustion.
    // Give a fixture with an exact original S1 byte bound enough bytes for
    // its formatted layout, retaining its model/dependency/checking authority.
    let format_limits = qsl_cst::format::FormatLimits::default().with_output_bytes(usize::MAX);
    let formatted = qsl_cst::format::format_with_limits(&parsed, format_limits).unwrap();
    let mut formatted_limits = limits;
    formatted_limits.source.source_bytes =
        formatted_limits.source.source_bytes.max(formatted.len());
    let mut formatted_source = source;
    formatted_source.revision.push_str("-formatted");
    let reparsed = qsl_cst::parse(
        formatted_source.clone(),
        path,
        formatted.as_bytes(),
        formatted_limits.source,
    )
    .unwrap();
    assert_eq!(
        qsl_cst::format::format_with_limits(&reparsed, format_limits).unwrap(),
        formatted,
        "{path}: second pass"
    );
    let checked = emitted_once(
        formatted_source,
        path,
        formatted.as_bytes(),
        packages,
        dependencies,
        formatted_limits,
        lock,
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

// Forward the same independent admission inputs to checking before packaging;
// grouping or defaulting them here would obscure the owning fixture's choices.
#[allow(clippy::too_many_arguments)]
fn emitted_once(
    source: SourceIdentity,
    path: &str,
    bytes: &[u8],
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    dependencies: &DependencyInput,
    limits: SpineLimits,
    lock: &LockEvidence,
) -> Result<EmittedUnit, Box<CompileRefusal>> {
    let checked = checked_once(&source, path, bytes, packages, dependencies, limits, lock)?;
    Ok(package(&checked, PackageLimits::default(), &Cancel::new())
        .map_err(refusal)?
        .into_value())
}

fn refusal(failure: qsl_replay::spine::FrontEndFailure) -> Box<CompileRefusal> {
    qsl_replay::spine::refusal_or_fault(failure)
        .unwrap_or_else(|fault| panic!("the front end faulted: {fault:?}"))
}

// This seam composes parse/select/check with the caller's exact source, models,
// dependencies, limits and lock; each input stays explicit for fixture custody.
#[allow(clippy::too_many_arguments)]
fn checked_once(
    source: &SourceIdentity,
    path: &str,
    bytes: &[u8],
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    dependencies: &DependencyInput,
    limits: SpineLimits,
    lock: &LockEvidence,
) -> Result<CheckedUnit, Box<CompileRefusal>> {
    let cancel = Cancel::new();
    let parsed = parse(
        &ParseRequest {
            source,
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
    Ok(check(&parsed, &models, dependencies, lock, limits, &cancel)
        .map_err(refusal)?
        .into_value())
}

/// Formats every source in an actual dependency fixture, then checks each
/// reached package's own emitted identity, including transitive libraries.
pub fn dependency_fixture_identity(
    source: SourceIdentity,
    path: &str,
    text: &str,
    libraries: Vec<SuppliedLibrary>,
) {
    let format = |mut source: SourceIdentity, path: &str, bytes: &[u8]| {
        let parsed =
            qsl_cst::parse(source.clone(), path, bytes, qsl_cst::Limits::default()).unwrap();
        let formatted = qsl_cst::format::format(&parsed).unwrap();
        source.revision.push_str("-formatted");
        let reparsed = qsl_cst::parse(
            source,
            path,
            formatted.as_bytes(),
            qsl_cst::Limits::default(),
        )
        .unwrap();
        assert_eq!(
            qsl_cst::format::format(&reparsed).unwrap(),
            formatted,
            "{path}: dependency second pass"
        );
        formatted
    };
    let formatted_libraries: Vec<_> = libraries
        .iter()
        .map(|library| {
            let mut source = library.source.clone();
            source.revision.push_str("-formatted");
            SuppliedLibrary {
                identity: library.identity.clone(),
                source,
                path: library.path.clone(),
                bytes: format(library.source.clone(), &library.path, &library.bytes).into_bytes(),
            }
        })
        .collect();
    let original_input = DependencyInput::new(libraries).unwrap();
    let formatted_input = DependencyInput::new(formatted_libraries).unwrap();
    let formatted_text = format(source.clone(), path, text.as_bytes());
    let packages = BTreeMap::new();
    let limits = SpineLimits::default();
    let lock = LockEvidence::default();
    let original = checked_once(
        &source,
        path,
        text.as_bytes(),
        &packages,
        &original_input,
        limits,
        &lock,
    )
    .expect("original fixture and its actual dependency closure check");
    let mut formatted_source = source;
    formatted_source.revision.push_str("-formatted");
    let formatted = checked_once(
        &formatted_source,
        path,
        formatted_text.as_bytes(),
        &packages,
        &formatted_input,
        limits,
        &lock,
    )
    .expect("formatted fixture and its actual dependency closure check");
    let mut pending = vec![(original.package(), formatted.package())];
    let mut visited = std::collections::BTreeSet::new();
    while let Some((original, formatted)) = pending.pop() {
        let original_emission = qsl_package::emit_checked(original).unwrap();
        assert!(
            original_emission.omitted().is_empty(),
            "{path}: original dependency emission omitted {:?}",
            original_emission.omitted()
        );
        let formatted_emission = qsl_package::emit_checked(formatted).unwrap();
        assert!(
            formatted_emission.omitted().is_empty(),
            "{path}: formatted dependency emission omitted {:?}",
            formatted_emission.omitted()
        );
        let original_id = original_emission.package().package_id();
        let formatted_id = formatted_emission.package().package_id();
        assert_eq!(
            original_id, formatted_id,
            "{path}: formatting changed checked dependency package identity"
        );
        if !visited.insert(original_id) {
            continue;
        }
        assert_eq!(
            original.dependency_selections(),
            formatted.dependency_selections()
        );
        for (identity, original) in original.dependencies() {
            pending.push((
                original,
                formatted
                    .dependencies()
                    .get(identity)
                    .expect("same checked dependency"),
            ));
        }
    }
}
