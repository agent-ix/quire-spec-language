// SPDX-License-Identifier: AGPL-3.0-only
//! TC-090: compare real production results with frozen independent byte vectors.

use sha2::{Digest, Sha256};

use super::*;

#[test]
#[trace("TC-090", "FR-021-AC-1", "FR-021-AC-6")]
#[trace("TC-078", "FR-019-AC-1", "FR-019-AC-8", "FR-019-AC-9")]
fn fixed_minimal_control_and_multiple_clause_vectors_match_the_public_producer() {
    let models = [vectors::model()];
    assert_eq!(
        models[0].source().source().text(),
        include_str!("../fixtures/native-package/model-source.json")
    );
    assert_eq!(
        models[0].artifact_bytes(),
        include_bytes!("../fixtures/native-package/model-artifact.json")
    );
    for vector in package_vector_setup::cases() {
        let package = NativePackage::new(
            package_vector_setup::checked(&vector, &models),
            PackageLimits::default(),
        )
        .unwrap();
        assert_eq!(package.bytes(), vector.artifact, "{} artifact", vector.name);
        assert_eq!(package.digest(), ByteDigest::of(vector.artifact));
        let mut hash = Sha256::new();
        hash.update(b"quire-spec-language\0quire.native.bound-package/v1\0linked-package\0");
        hash.update(vector.canonical);
        let digest = vector
            .digest
            .strip_suffix('\n')
            .expect("digest text has one final newline");
        assert_eq!(
            format!("{:x}", hash.finalize()),
            digest,
            "{} independent hash",
            vector.name
        );
        assert_eq!(
            package.canonical_identity().to_string(),
            digest,
            "{} production hash",
            vector.name
        );
        assert_eq!(package.checked().clauses().len(), vector.clauses.len());
        assert_eq!(
            package.usage().canonical.unwrap().output_bytes,
            vector.canonical.len()
        );
        assert_eq!(
            package.usage().encode.unwrap().output_bytes,
            vector.artifact.len()
        );
    }
    // The recipe is an independently maintained oracle, and ordinary tests
    // compare it with the frozen files without rewriting those files.
    let multiple = vectors::expected_multiple(&models[0], r#""test:multiple""#, r#""draft:1""#, 1);
    assert_eq!(
        multiple.canonical.as_bytes(),
        include_bytes!("../fixtures/native-package/multiple.canonical.json")
    );
    assert_eq!(
        multiple.artifact.as_bytes(),
        include_bytes!("../fixtures/native-package/multiple.package.json")
    );
}
