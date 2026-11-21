// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared bytes-only directory admission (FR-850, FR-1039).

use super::{dependency_manifest::Manifest, Intake, Result, RunCause};
use filament_contract_model::CheckedPackageV2;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path},
    sync::Arc,
};

pub(super) struct Admitted {
    pub(super) package: CheckedPackageV2,
    pub(super) identities: fsl_package::ReadIdentityRows,
    pub(super) equality: Option<fsl_package::ReadEqualityRows>,
    pub(super) budget: filament_contract_model::NativeJoinBudget,
    pub(super) bytes: Vec<u8>,
    pub(super) identity_bytes: Vec<u8>,
    pub(super) equality_bytes: Option<Vec<u8>>,
    pub(super) dependencies: Vec<fsl_package::DependencyArtifact>,
}

pub(super) fn refusal(
    file: &str,
    cause: &str,
    pointer: Option<String>,
    detail: impl std::fmt::Display,
) -> RunCause {
    RunCause::AnalyzePackage {
        code: fsl_foundation::Code::InvalidPackage,
        cause: Some(cause.to_owned()),
        pointer,
        message: format!("invalid_package/{cause}: {file}: {detail}"),
    }
}

pub(super) fn read_file(
    intake: &mut Intake<'_>,
    directory: &str,
    relative: &str,
    additional: bool,
) -> Result<Vec<u8>> {
    if directory.contains('\\') {
        return Err(refusal(
            relative,
            "invalid-value",
            None,
            "backslash in artifact path",
        ));
    }
    let selected = Path::new(directory).join(relative);
    let mut current = intake.directory.to_path_buf();
    for component in selected.components() {
        let Component::Normal(component) = component else {
            return Err(refusal(
                relative,
                "invalid-value",
                None,
                "nonconfined artifact path",
            ));
        };
        current.push(component);
        let metadata = std::fs::symlink_metadata(&current).map_err(|_| {
            refusal(
                relative,
                "missing-file",
                None,
                "selected artifact is absent",
            )
        })?;
        if metadata.file_type().is_symlink()
            || (!metadata.is_dir() && current != intake.directory.join(&selected))
        {
            return Err(refusal(
                relative,
                "invalid-value",
                None,
                "unsafe artifact path",
            ));
        }
    }
    let metadata = std::fs::symlink_metadata(&current).map_err(|_| {
        refusal(
            relative,
            "missing-file",
            None,
            "selected artifact is absent",
        )
    })?;
    if !metadata.is_file() {
        return Err(refusal(
            relative,
            "invalid-value",
            None,
            "artifact is not a regular file",
        ));
    }
    if additional {
        intake.additional_file()?;
    }
    intake.file(&selected.to_string_lossy(), intake.remaining)
}

fn package_failure(file: &str, error: fsl_package::PackageAdmissionRefusal) -> RunCause {
    if let Some(limit) = &error.limit {
        return RunCause::AnalyzePackage {
            code: error.code,
            cause: error.cause,
            pointer: error.pointer,
            message: format!("{file}: {limit:?}"),
        };
    }
    RunCause::AnalyzePackage {
        code: error.code,
        cause: error.cause.clone(),
        pointer: error.pointer.clone(),
        message: format!("{file}: {}", error.label()),
    }
}

fn identities(
    bytes: &[u8],
    package: &CheckedPackageV2,
    budget: &mut filament_contract_model::NativeJoinBudget,
    file: &str,
) -> Result<fsl_package::ReadIdentityRows> {
    fsl_package::ReadIdentityRows::read(bytes, package, budget).map_err(|error| {
        let native = match &error {
            fsl_package::IdentityFileRefusal::Native(native) => {
                Some(fsl_package::NativeJoinFailure::from((**native).clone()))
            }
            fsl_package::IdentityFileRefusal::Rows(rows) => {
                Some(fsl_package::NativeJoinFailure::from(rows.clone()))
            }
            _ => None,
        };
        if let Some(denial) = native.and_then(|native| {
            fsl_package::NativeJoinLimits::from_budget(budget).refusal(native, budget)
        }) {
            return RunCause::AnalyzePackage {
                code: fsl_foundation::Code::StageLimitExceeded,
                cause: Some("insufficient-next-charge".to_owned()),
                pointer: None,
                message: format!("{file}: {denial:?}"),
            };
        }
        let cause = match &error {
            fsl_package::IdentityFileRefusal::Malformed { .. } => "malformed-file",
            fsl_package::IdentityFileRefusal::Format(_) => "unknown-format",
            fsl_package::IdentityFileRefusal::Unjoined { .. } => "unjoined-row",
            fsl_package::IdentityFileRefusal::Native(_)
            | fsl_package::IdentityFileRefusal::Rows(_)
            | fsl_package::IdentityFileRefusal::SourceObject(_)
            | fsl_package::IdentityFileRefusal::SourceOperation(_) => "native-witness",
        };
        refusal(file, cause, None, error)
    })
}

struct Pending {
    identity: String,
    directory: String,
    bytes: Vec<u8>,
    edges: Vec<String>,
}

// Lock inspection schedules strict admission only. No graph or semantic value
// is derived here; every package is subsequently validated by the IR reader.
fn edges(bytes: &[u8]) -> Vec<String> {
    serde_json::from_slice::<serde_json::Value>(bytes)
        .ok()
        .and_then(|value| {
            value
                .get("lock")?
                .get("dependency_selections")?
                .as_array()
                .cloned()
        })
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| {
            row.get("identity")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .collect()
}

pub(super) fn admit(
    intake: &mut Intake<'_>,
    directory: &str,
    bytes: Vec<u8>,
    supplied: &fsl_semantics::model::intake::PackageInput,
    limits: fsl_package::V2ReadLimits,
) -> Result<Admitted> {
    let manifest_bytes = read_file(intake, directory, "dependencies.json", true)?;
    let manifest = Manifest::read(&manifest_bytes).map_err(|error| {
        refusal(
            "dependencies.json",
            error.cause,
            Some(error.pointer),
            error.supplied_format.unwrap_or_default(),
        )
    })?;
    let mut pending = BTreeMap::new();
    for row in manifest.dependencies {
        let file = format!("{}/checked-package.json", row.directory);
        let bytes = read_file(intake, directory, &file, true)?;
        pending.insert(
            row.identity.clone(),
            Pending {
                identity: row.identity,
                directory: row.directory,
                edges: edges(&bytes),
                bytes,
            },
        );
    }
    let root_edges = edges(&bytes);
    // Detect cycles in recorded edge order before topological admission.
    let mut active = BTreeSet::new();
    let mut visited = BTreeSet::new();
    let mut stack: Vec<_> = root_edges
        .iter()
        .rev()
        .map(|identity| (identity.clone(), false))
        .collect();
    while let Some((identity, expanded)) = stack.pop() {
        if expanded {
            active.remove(&identity);
            visited.insert(identity);
            continue;
        }
        if visited.contains(&identity) {
            continue;
        }
        if !active.insert(identity.clone()) {
            return Err(refusal(
                "dependencies.json",
                "definition-cycle",
                None,
                format!("cycle through {identity}: {active:?}"),
            ));
        }
        stack.push((identity.clone(), true));
        if let Some(package) = pending.get(&identity) {
            for (index, child) in package.edges.iter().enumerate().rev() {
                if active.contains(child) {
                    return Err(refusal(
                        &format!("{}/checked-package.json", package.directory),
                        "definition-cycle",
                        Some(format!("/lock/dependency_selections/{index}")),
                        format!("{active:?} -> {child}"),
                    ));
                }
                stack.push((child.clone(), false));
            }
        }
    }
    let mut admitted = BTreeMap::new();
    let mut dependencies = Vec::new();
    let mut remaining = BTreeMap::new();
    let mut importers: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut ready = BTreeSet::new();
    for (identity, package) in &pending {
        let mut count = 0_usize;
        for edge in &package.edges {
            if pending.contains_key(edge) {
                count += 1;
                importers
                    .entry(edge.clone())
                    .or_default()
                    .push(identity.clone());
            }
        }
        remaining.insert(identity.clone(), count);
        if count == 0 {
            ready.insert(identity.clone());
        }
    }
    let mut budget = limits
        .native_join_limits()
        .budget()
        .with_read_storage_limit(u64::MAX);
    while !pending.is_empty() {
        let Some(identity) = ready.pop_first() else {
            return Err(refusal(
                "dependencies.json",
                "definition-cycle",
                None,
                "no acyclic dependency is ready",
            ));
        };
        let Some(package) = pending.remove(&identity) else {
            continue;
        };
        let own = package
            .edges
            .iter()
            .filter_map(|identity| {
                admitted
                    .get(identity)
                    .map(|package| (identity.clone(), Arc::clone(package)))
            })
            .collect();
        let file = format!("{}/checked-package.json", package.directory);
        let read = fsl_package::admit_checked_package_with_budget(
            &package.bytes,
            supplied,
            &own,
            limits,
            &mut budget,
        )
        .map_err(|error| package_failure(&file, error))?;
        if read.name() != package.identity {
            return Err(refusal(
                &file,
                "conflicting-definition",
                None,
                &package.identity,
            ));
        }
        let identity_file = format!("{}/identities.json", package.directory);
        let equality_file = format!("{}/equality.json", package.directory);
        let identity_bytes = read_file(intake, directory, &identity_file, true)?;
        let equality_bytes = read_file(intake, directory, &equality_file, true)?;
        let identities = identities(&identity_bytes, &read, &mut budget, &identity_file)?;
        let equality = fsl_package::ReadEqualityRows::read(&equality_bytes, &read)
            .map_err(|error| refusal(&equality_file, "refused-file", None, error))?;
        let read = Arc::new(read);
        admitted.insert(identity.clone(), Arc::clone(&read));
        for importer in importers.get(&identity).into_iter().flatten() {
            if let Some(count) = remaining.get_mut(importer) {
                *count -= 1;
                if *count == 0 {
                    ready.insert(importer.clone());
                }
            }
        }
        dependencies.push(fsl_package::DependencyArtifact {
            identity,
            package: read,
            bytes: package.bytes,
            identity_bytes,
            equality_bytes,
            identities,
            equality,
        });
    }
    let own = root_edges
        .iter()
        .filter_map(|identity| {
            admitted
                .get(identity)
                .map(|package| (identity.clone(), Arc::clone(package)))
        })
        .collect();
    let package =
        fsl_package::admit_checked_package_with_budget(&bytes, supplied, &own, limits, &mut budget)
            .map_err(|error| package_failure("checked-package.json", error))?;
    for dependency in &dependencies {
        if !visited.contains(&dependency.identity) {
            let index = dependencies
                .iter()
                .map(|entry| &entry.identity)
                .filter(|identity| *identity < &dependency.identity)
                .count();
            return Err(refusal(
                "dependencies.json",
                "invalid-value",
                Some(format!("/dependencies/{index}/identity")),
                "unreachable dependency artifact",
            ));
        }
    }
    for identity in supplied.keys() {
        if !package
            .selected_model_packages()
            .any(|selected| selected == identity)
            && !dependencies.iter().any(|dependency| {
                dependency
                    .package
                    .selected_model_packages()
                    .any(|selected| selected == identity)
            })
        {
            return Err(refusal("models", "conflicting-definition", None, identity));
        }
    }
    let identity_bytes = read_file(intake, directory, "identities.json", false)?;
    let identities = identities(&identity_bytes, &package, &mut budget, "identities.json")?;
    let equality_bytes = if std::fs::symlink_metadata(
        intake.directory.join(directory).join("equality.json"),
    )
    .is_ok()
    {
        Some(read_file(intake, directory, "equality.json", true)?)
    } else if package
        .checked_term_data()
        .is_some_and(|data| !data.equality_decisions.is_empty())
    {
        return Err(refusal(
            "equality.json",
            "missing-file",
            None,
            "required equality witness is absent",
        ));
    } else {
        None
    };
    let equality = equality_bytes
        .as_deref()
        .map(|bytes| {
            fsl_package::ReadEqualityRows::read(bytes, &package)
                .map_err(|error| refusal("equality.json", "refused-file", None, error))
        })
        .transpose()?;
    Ok(Admitted {
        package,
        identities,
        equality,
        budget,
        bytes,
        identity_bytes,
        equality_bytes,
        dependencies,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{checked_handoff, emit_checked_v2};
    use ix_trace_rs::trace;

    fn chain(directory: &Path) {
        let header = "language \"ix:native\" edition \"1-draft\"; profile v = \"filament.value.complete/v1\";";
        for (file, body) in [
            ("a.native", "function answer using v(): Integer pure { 42 }"),
            (
                "z.native",
                "import \"test/a\" as a; function answer using v(): Integer pure { a::answer() }",
            ),
            (
                "unit.native",
                "import \"test/z\" as z; function answer using v(): Integer pure { z::answer() }",
            ),
        ] {
            std::fs::write(directory.join(file), format!("{header}\n{body}\n")).unwrap();
        }
        checked_handoff::write(
            &checked_handoff::CheckedHandoffRequest {
                emit: emit_checked_v2::Request {
                    source: directory.join("unit.native"),
                    package_name: "root".to_owned(),
                    models: vec![],
                    libraries: vec![
                        ("test/a".to_owned(), directory.join("a.native")),
                        ("test/z".to_owned(), directory.join("z.native")),
                    ],
                    execution_points: None,
                    temporal_selections: vec![],
                    out: None,
                },
                corpus: checked_handoff::Corpus::None,
                native_join: fsl_package::NativeJoinLimits::new(1 << 40, 1 << 24),
            },
            &directory.join("handoff"),
        )
        .unwrap();
    }

    fn load(
        directory: &Path,
        limits: fsl_package::V2ReadLimits,
    ) -> Result<filament_contract_model::NativeJoinCharge> {
        let mut intake = super::super::intake_with_bytes(
            directory,
            super::super::FileCounts {
                programs: 2,
                ..Default::default()
            },
            1 << 28,
        )?;
        let bytes = read_file(&mut intake, "handoff", "checked-package.json", false)?;
        let mut loaded = admit(&mut intake, "handoff", bytes, &BTreeMap::new(), limits)?;
        let _view = fsl_package::BarePackage::new(&loaded.package)
            .with_dependency_artifacts(&loaded.dependencies, &mut loaded.budget)
            .and_then(|package| {
                package.with_supply(
                    fsl_package::BareSupply {
                        identities: &loaded.identities,
                        equality: loaded.equality.as_ref(),
                    },
                    &mut loaded.budget,
                )
            })
            .map_err(|error| RunCause::AnalyzePackage {
                code: fsl_foundation::Code::StageLimitExceeded,
                cause: None,
                pointer: None,
                message: format!("{error:?}"),
            })?;
        Ok(loaded.budget.consumed())
    }

    #[test]
    #[trace("FR-850-AC-15", "TC-2410")]
    fn strict_reads_and_own_witness_joins_share_exact_closure_allowances() {
        let directory = tempfile::tempdir().unwrap();
        chain(directory.path());
        let mut limits = fsl_package::V2ReadLimits::bounded(1 << 40);
        limits.work = 1 << 40;
        let measured =
            load(directory.path(), limits).expect("measure complete root and dependency admission");
        assert!(measured.work > 0 && measured.storage > 0);
        limits.work = measured.work;
        limits.storage = measured.storage;
        assert_eq!(load(directory.path(), limits).unwrap(), measured);
        for shortened in [
            fsl_package::V2ReadLimits {
                work: measured.work - 1,
                ..limits
            },
            fsl_package::V2ReadLimits {
                storage: measured.storage - 1,
                ..limits
            },
        ] {
            let refusal = load(directory.path(), shortened)
                .expect_err("one-short cumulative allowance must stop the pass");
            assert_eq!(
                refusal.code(),
                fsl_foundation::Code::StageLimitExceeded,
                "{refusal}"
            );
            assert!(
                refusal.to_string().contains("i2.") || refusal.to_string().contains("I2"),
                "the original setting is retained: {refusal}"
            );
        }
    }
}
