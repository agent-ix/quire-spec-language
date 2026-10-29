// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-090: compare real production results with independently composed byte vectors.

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
    for vector in package_vector_setup::cases(&models[0]) {
        let package = NativePackage::new(
            package_vector_setup::checked(&vector, &models),
            PackageLimits::default(),
        )
        .unwrap();
        assert_eq!(package.bytes(), vector.artifact, "{} artifact", vector.name);
        assert_eq!(package.digest(), ByteDigest::of(&vector.artifact));
        assert_eq!(
            package.canonical_identity().to_string(),
            vector.digest,
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
}
