// SPDX-License-Identifier: AGPL-3.0-or-later
//! The vendored complete-value inputs are the exact bytes of the pinned QSpec
//! revision, and package selection follows its canonical vectors.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use ix_trace_rs::trace;
use quire_spec_language::value::{
    CatalogRole, DefinitionLock, LockError, SelectionRefusalCode, Trigger, PINNED_LOCK_BYTES,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};

const DEFINITIONS: &str =
    "resources/complete-value/quire-specification/proposals/quire-v1/definitions";

/// SHA-256 of the non-catalog inputs, read from QSpec
/// `5aa00f35056c65948de93ad339540974d35c368a` with `git show`.
const PINNED_NON_CATALOG: [(&str, &str); 8] = [
    (
        "proposals/quire-v1/definitions/complete-value-lock.json",
        "87b60efd54f2883fb2458d76e02921e119640c987515f435f09bb840384da117",
    ),
    (
        "proposals/quire-v1/definitions/complete-value-selection-vectors.json",
        "16b9774a05da385a838d89daba939f8b594493fc670148a6e7d24423ddaea282",
    ),
    (
        "spec/test-cases/TC-185-exact-decimal-semantics.md",
        "b822e091441b59fc6bd4993b8112aefa51ef12d2d4499cca8c4d99a7c124e3d1",
    ),
    (
        "spec/test-cases/TC-192-integer-division-profiles.md",
        "e7e636a17d943f600ea7f707a4af377d954942c491561b984f3696535e11c1cf",
    ),
    (
        "spec/test-cases/TC-187-quantity-and-unit-conversion.md",
        "73ef551a68fc90472149f923a378fe027585806df8fd63c04da9bab2be5c3628",
    ),
    (
        "spec/test-cases/TC-186-text-and-enum-identity-semantics.md",
        "ec66db8182cb304dcb907a5dda536ce5fe437608f57713ebcea7ff3572de041e",
    ),
    (
        "proposals/checked-package-v2/node-identity-vectors.json",
        "635717d735883c4f33440758376d4958e43340a801ac3b7be055d156f2096244",
    ),
    (
        "proposals/checked-package-v2/node-identity-preimage.schema.json",
        "978f63b9094189a4f480326658a9cc1c009239c5df770f0dc118b632e0b02501",
    ),
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/complete-value/quire-specification")
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[trace("Task-048")]
#[test]
fn vendored_catalog_artifacts_match_the_pinned_lock_digests() {
    let lock = DefinitionLock::pinned().unwrap();
    assert_eq!(lock.revision(), "1-draft.1");
    let definitions = Path::new(env!("CARGO_MANIFEST_DIR")).join(DEFINITIONS);
    assert_eq!(
        std::fs::read(definitions.join("complete-value-lock.json")).unwrap(),
        PINNED_LOCK_BYTES
    );
    let roles: BTreeSet<_> = lock.catalog().iter().map(|entry| entry.role).collect();
    assert_eq!(roles, CatalogRole::ALL.into_iter().collect());
    for entry in lock.catalog() {
        let bytes = std::fs::read(definitions.join(&entry.artifact_path))
            .unwrap_or_else(|error| panic!("{}: {error}", entry.artifact_path));
        assert_eq!(
            sha256_hex(&bytes),
            entry.definition.digest,
            "{} differs from the pinned lock",
            entry.artifact_path
        );
        assert_eq!(entry.definition.digest_domain, "quire.definition.bytes/v1");
    }
    for (path, digest) in PINNED_NON_CATALOG {
        assert_eq!(
            sha256_hex(&std::fs::read(root().join(path)).unwrap()),
            digest,
            "{path}"
        );
    }
}

#[trace("Task-048")]
#[test]
fn lock_admission_refuses_malformed_or_non_closed_locks() {
    let text = std::str::from_utf8(PINNED_LOCK_BYTES).unwrap();
    let mutations = [
        (
            text.replacen("\"lock_version\"", "\"lock_versions\"", 1),
            "unknown member",
        ),
        (
            text.replacen(
                "quire.value.definition-lock/v1",
                "quire.value.definition-lock/v2",
                1,
            ),
            "header",
        ),
        (
            text.replacen("[\"ieee_operation\", ", "[", 1),
            "trigger vocabulary",
        ),
        (
            text.replacen("\"role\": \"root\"", "\"role\": \"edition\"", 1),
            "duplicate role",
        ),
        (
            text.replacen(
                "\"always\": [\"edition\", \"root\",",
                "\"always\": [\"edition\", \"root\", \"text_profile\",",
                1,
            ),
            "two selection slots",
        ),
        (
            text.replacen("d70a50262b334c6b", "D70A50262B334C6B", 1),
            "digest spelling",
        ),
    ];
    let outcomes: Vec<_> = mutations
        .iter()
        .map(|(bytes, name)| {
            assert_ne!(bytes.as_str(), text, "mutation {name} did not apply");
            DefinitionLock::parse(bytes.as_bytes()).unwrap_err()
        })
        .collect();
    assert!(matches!(outcomes[0], LockError::Malformed { .. }));
    assert_eq!(outcomes[1], LockError::UnsupportedHeader);
    assert_eq!(outcomes[2], LockError::VocabularyMismatch);
    assert_eq!(outcomes[3], LockError::CatalogRole(CatalogRole::Edition));
    assert_eq!(
        outcomes[4],
        LockError::SelectionSlot(CatalogRole::TextProfile)
    );
    assert_eq!(outcomes[5], LockError::InvalidDigest(CatalogRole::Edition));
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SelectionVectors {
    version: String,
    lock: LockRef,
    always_roles: Vec<String>,
    accepted: Vec<Accepted>,
    refused: Vec<Refused>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LockRef {
    lock_version: String,
    revision: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Accepted {
    name: String,
    triggers: Vec<String>,
    additional_roles: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Refused {
    name: String,
    triggers: Vec<String>,
    additional_roles: Vec<String>,
    omitted_always_roles: Vec<String>,
    expected_code: String,
}

fn selection_vectors() -> SelectionVectors {
    let bytes = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(DEFINITIONS)
            .join("complete-value-selection-vectors.json"),
    )
    .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn strs(values: &[String]) -> Vec<&str> {
    values.iter().map(String::as_str).collect()
}

#[trace("TC-192")]
#[test]
fn canonical_package_selection_vectors_admit_and_refuse_exactly() {
    let lock = DefinitionLock::pinned().unwrap();
    let vectors = selection_vectors();
    assert_eq!(vectors.version, "quire.value.package-selection-vectors/v1");
    assert_eq!(vectors.lock.lock_version, "quire.value.definition-lock/v1");
    assert_eq!(vectors.lock.revision, lock.revision());
    let always: Vec<_> = lock
        .always_roles()
        .iter()
        .map(|role| role.as_str())
        .collect();
    assert_eq!(strs(&vectors.always_roles), always);

    let accepted: BTreeSet<_> = vectors.accepted.iter().map(|v| v.name.as_str()).collect();
    assert_eq!(
        accepted,
        BTreeSet::from([
            "no-trigger",
            "text-bearing",
            "ieee-operation",
            "div-rem-euclidean",
            "div-rem-floor",
            "div-rem-truncating",
            "every-trigger",
        ])
    );
    for vector in &vectors.accepted {
        let mut roles = always.clone();
        roles.extend(strs(&vector.additional_roles));
        let admitted = lock
            .admit_selection(&strs(&vector.triggers), &roles)
            .unwrap_or_else(|code| panic!("{}: {code:?}", vector.name));
        let triggers: BTreeSet<_> = vector
            .triggers
            .iter()
            .map(|code| Trigger::from_code(code).unwrap())
            .collect();
        assert_eq!(admitted.triggers(), &triggers, "{}", vector.name);
        assert_eq!(admitted.roles().len(), roles.len(), "{}", vector.name);
        assert_eq!(
            admitted.division_profile().is_some(),
            triggers.contains(&Trigger::IntegerDivRem),
            "{}",
            vector.name
        );
    }

    let refused: BTreeSet<_> = vectors.refused.iter().map(|v| v.name.as_str()).collect();
    assert_eq!(
        refused,
        BTreeSet::from([
            "two-division-profiles",
            "division-missing-for-div-rem",
            "division-without-trigger",
            "ieee-without-trigger",
            "unicode-missing-for-text-bearing",
            "always-role-missing",
            "unknown-trigger",
            "duplicate-trigger",
            "unknown-trigger-before-duplicate-trigger",
            "duplicate-trigger-before-unknown-role",
            "unknown-role",
            "duplicate-role",
        ])
    );
    let mut codes = BTreeSet::new();
    for vector in &vectors.refused {
        let mut roles: Vec<_> = always
            .iter()
            .copied()
            .filter(|role| {
                !vector
                    .omitted_always_roles
                    .iter()
                    .any(|omitted| omitted == role)
            })
            .collect();
        roles.extend(strs(&vector.additional_roles));
        let expected = SelectionRefusalCode::from_code(&vector.expected_code).unwrap();
        assert_eq!(
            lock.admit_selection(&strs(&vector.triggers), &roles),
            Err(expected),
            "{}",
            vector.name
        );
        codes.insert(expected);
    }
    assert_eq!(codes, SelectionRefusalCode::ALL.into_iter().collect());
}
