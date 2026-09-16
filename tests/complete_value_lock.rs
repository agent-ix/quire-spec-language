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
/// `7d7943ab1482e091f6d126401ada38957c4a1ccf` with `git show`.
const PINNED_NON_CATALOG: [(&str, &str); 20] = [
    (
        "spec/functional/expressions/FR-146-check-total-pure-functions.md",
        "4d21ad026fdf637f9eea9ec9455c678a4e78a8d4f6b87fd9446e7cde31556d29",
    ),
    (
        "spec/functional/tooling/FR-307-package-reusable-semantic-libraries.md",
        "d287d75f670ae46cbc6d2fbe895f13c5eb29a52af59b6569b0e2c1d03ea7f03a",
    ),
    (
        "spec/test-cases/TC-191-total-pure-function-checking.md",
        "24441875a3ef17a4aa4ce4f86c901709d0cf1424b05c01392a3190f4e2ed3295",
    ),
    (
        "spec/test-cases/TC-227-reusable-semantic-library-resolution.md",
        "bf1d59228323f6325afa0fc8b35791a12db9263c67585f9bd590d62947425674",
    ),
    (
        "spec/functional/type-model/FR-144-preserve-complete-collection-algebra.md",
        "a1319ba750bdef42df93627e07601a808f998a4ba5f83cde02104edea6ee8951",
    ),
    (
        "spec/functional/expressions/FR-145-convert-and-query-collections.md",
        "eb3b4c8ebad6ca616ba0fb166663f4163d55418294ba864b471f10b6f2c74c93",
    ),
    (
        "spec/test-cases/TC-189-collection-kind-algebra.md",
        "df2a5768489ea1432c7a1587ac59da7536377a9c16942b58a7073c7686924cf6",
    ),
    (
        "spec/test-cases/TC-190-collection-query-and-conversion-algebra.md",
        "080d2801ca78999a2b8437974efc3e0b00b9abd02482b065f5851a8b2659b3d8",
    ),
    (
        "spec/functional/type-model/FR-143-evaluate-record-tuple-recursive-values.md",
        "cfa98d995dad5939c065fc1fca581e1524dc05c01d3596198f9f0c8bc8aa5313",
    ),
    (
        "spec/test-cases/TC-188-record-tuple-and-recursive-values.md",
        "011ff0d8450ac876e63f078cc8328f84c180c50278e5a6f49612bcbeceaa499a",
    ),
    (
        "spec/test-cases/TC-194-complete-equality-matrix.md",
        "e7a261755c7b7552bf3a5fc516c2c57dcbc8f20e321d9ad339cdf3549c7a3b48",
    ),
    (
        "proposals/quire-v1/definitions/complete-value-lock.json",
        "8892527d03064c1c1c06fbe7ec2ebaee0b5f33d0934c90af12ca02edb9580aef",
    ),
    (
        "proposals/quire-v1/definitions/complete-value-selection-vectors.json",
        "16b9774a05da385a838d89daba939f8b594493fc670148a6e7d24423ddaea282",
    ),
    (
        "spec/test-cases/TC-185-exact-decimal-semantics.md",
        "7000e35969abefab803c1ee1eaf170ff6729909c25492b22648eb7c21cbccbed",
    ),
    (
        "spec/test-cases/TC-192-integer-division-profiles.md",
        "e7e636a17d943f600ea7f707a4af377d954942c491561b984f3696535e11c1cf",
    ),
    (
        "spec/test-cases/TC-187-quantity-and-unit-conversion.md",
        "8c006cd22741b28fbe20f4d10ffca761aa563c77c4b18ea507168a9278e2edb4",
    ),
    (
        "spec/test-cases/TC-186-text-and-enum-identity-semantics.md",
        "e8e0c72a476d66777c18fdcab1f4537ffa3d66f50aaedf1c1bb9db8da196494f",
    ),
    (
        "proposals/checked-package-v2/node-identity-vectors.json",
        "635717d735883c4f33440758376d4958e43340a801ac3b7be055d156f2096244",
    ),
    (
        "proposals/checked-package-v2/node-identity-preimage.schema.json",
        "978f63b9094189a4f480326658a9cc1c009239c5df770f0dc118b632e0b02501",
    ),
    (
        "spec/test-cases/TC-193-ieee-exceptional-and-rounding-profiles.md",
        "708cb6e97b88a00ea3636bf2d9594a3174ab9bfb495699dc71681d5f3aeda868",
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
