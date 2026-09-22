// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-253 (FR-087-AC-3) at the `library` level: the ADR-011 §4 verified
//! binding (`verify_binding`) over a hand-built candidate, for the steps the
//! wire path cannot reach with `library`'s own named cause. On the wire path
//! IR refuses a `package_id` that does not recompute before `library` ever
//! sees it, and names it `stale_dependency` (IR-238 item 3), so condition 2's
//! `PackageIdMismatch` and the digest non-substitution cases (steps 3 and
//! 5-7) are exercised here, where `verify_package` is the refusing check.
//!
//! Steps 5-7 are weakly discriminating on their own: any wrong `package_id`
//! refuses. Each case therefore makes the substitute digest the value both
//! the candidate claims *and* the pin selects, so an implementation that
//! trusted an agreeing claim and pin instead of recomputing from the
//! identity preimage would admit it and fail the test.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use super::package_identity::fixtures::{hex, one_node_preimage};
use super::{
    resolve_libraries, verify_binding, ConflictingPin, ImportDeclaration, LibraryName,
    LibraryPackage, LibraryRefusal, PackageId, PackageNodeKey, PinMismatch, PinnedRequest,
    Selection, StaleCause, StalePin, SupportedV2Wire, VerifiedPackage,
};
use qsl_foundation::digest::WireNodeId;

fn identity(label: &str) -> LibraryName {
    LibraryName::new(vec![label.to_owned()]).unwrap()
}

fn preimage() -> Vec<u8> {
    one_node_preimage(b"pkg::R", &["R"])
}

fn recomputed() -> PackageId {
    PackageId::of_preimage(&preimage())
}

/// `pkg@1` exporting `R`, claiming `package_id`.
fn candidate(package_id: PackageId) -> LibraryPackage {
    LibraryPackage {
        library: identity("pkg"),
        version: "1".to_owned(),
        package_id,
        identity_preimage: preimage().into_boxed_slice(),
        imports: Vec::new(),
        exports: vec!["R".to_owned()],
    }
}

fn selection(version: &str, package_id: PackageId) -> Selection {
    Selection {
        version: version.to_owned(),
        package_id,
    }
}

fn pin(version: &str, package_id: PackageId) -> PinnedRequest {
    PinnedRequest::new([(identity("pkg"), selection(version, package_id))])
        .expect("one entry never conflicts")
}

fn bind(
    candidate: LibraryPackage,
    pinned: &PinnedRequest,
) -> Result<VerifiedPackage, LibraryRefusal> {
    verify_binding(SupportedV2Wire::attest_ir_admitted_v2(), candidate, pinned)
}

/// Condition 2 refuses `claimed` with `library`'s own named cause, naming
/// both the claimed and the recomputed `package_id`.
fn assert_package_id_mismatch(result: Result<VerifiedPackage, LibraryRefusal>, claimed: PackageId) {
    assert_eq!(
        result,
        Err(LibraryRefusal::PackageIdMismatch {
            library: identity("pkg"),
            claimed,
            recomputed: recomputed(),
        })
    );
}

/// TC-253 step 1: all three conditions hold, so the binding admits the
/// candidate, and its view carries `R` at the node the preimage declares.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn admits_when_all_three_conditions_hold() {
    let verified =
        bind(candidate(recomputed()), &pin("1", recomputed())).expect("all three conditions hold");
    assert_eq!(verified.package_id(), recomputed());
    let view = verified.into_import_view();
    let node = WireNodeId::from_hex(&hex(b"pkg::R")).unwrap();
    assert_eq!(
        view.exports().collect::<Vec<_>>(),
        vec![("R", PackageNodeKey::new(recomputed(), node))]
    );
}

/// TC-253 step 3: a declared `package_id` that is not the digest of the
/// identity preimage refuses with the named `PackageIdMismatch`, even when
/// the pin agrees with the wrong claim.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn refuses_a_package_id_that_does_not_recompute() {
    let claimed = PackageId::of_preimage(b"not-the-preimage");
    assert_package_id_mismatch(bind(candidate(claimed), &pin("1", claimed)), claimed);
}

/// The envelope bytes a v2 file carrying `preimage()` would hold.
fn file_bytes() -> Vec<u8> {
    let preimage: Value = serde_json::from_slice(&preimage()).unwrap();
    serde_json::to_vec(&json!({
        "contract_version": "quire.checked-package/v2",
        "identity_preimage": preimage,
    }))
    .unwrap()
}

/// TC-253 step 5: the digest of the file bytes never substitutes for the
/// identity-preimage digest.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn refuses_a_file_byte_digest_in_place_of_the_package_id() {
    let file_digest = PackageId::of_preimage(&file_bytes());
    assert_package_id_mismatch(
        bind(candidate(file_digest), &pin("1", file_digest)),
        file_digest,
    );
}

/// TC-253 step 6: the digest of the lock member's JCS bytes never
/// substitutes.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn refuses_a_lock_digest_in_place_of_the_package_id() {
    let lock = serde_json::to_vec(&json!({
        "definition_selections": [],
        "dependency_selections": [],
        "sources": [{"identity": "src", "digest": hex(b"src")}],
    }))
    .unwrap();
    let lock_digest = PackageId::of_preimage(&lock);
    assert_package_id_mismatch(
        bind(candidate(lock_digest), &pin("1", lock_digest)),
        lock_digest,
    );
}

/// TC-253 step 7: the lock's source digest (`sha256("src")`) never
/// substitutes.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn refuses_a_source_digest_in_place_of_the_package_id() {
    let source_digest = PackageId::of_preimage(b"src");
    assert_package_id_mismatch(
        bind(candidate(source_digest), &pin("1", source_digest)),
        source_digest,
    );
}

/// TC-253 step 8: conditions 2 and 3 failing together (a wrong claim, and
/// no pin at all) still refuse cleanly with a named cause and no
/// `VerifiedPackage`.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn refuses_when_conditions_2_and_3_both_fail() {
    let claimed = PackageId::of_preimage(b"not-the-preimage");
    assert_package_id_mismatch(bind(candidate(claimed), &PinnedRequest::default()), claimed);
}

/// TC-253 step 8: condition 3's two halves failing together (another
/// `package_id` *and* another version pinned) refuse on the digest.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn refuses_when_the_pinned_id_and_version_both_disagree() {
    let other = PackageId::of_preimage(b"other");
    assert_eq!(
        bind(candidate(recomputed()), &pin("2", other)),
        Err(LibraryRefusal::StaleDependency {
            path: vec![identity("pkg")],
            pin: StalePin::Pinned(Box::new(PinMismatch {
                pinned: selection("2", other),
                presented: selection("1", recomputed()),
            })),
            cause: StaleCause::ByteDigestMismatch,
        })
    );
}

/// PR #340 review F8: a pinned request holds one selection per identity.
/// Two different selections for one identity cannot be built, in either
/// order, so condition 3's verdict can never depend on entry order.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn conflicting_pins_for_one_identity_cannot_be_built() {
    let good = selection("1", recomputed());
    let stale = selection("1", PackageId::of_preimage(b"stale"));
    for (first, second) in [(&good, &stale), (&stale, &good)] {
        assert_eq!(
            PinnedRequest::new([
                (identity("pkg"), first.clone()),
                (identity("pkg"), second.clone()),
            ]),
            Err(ConflictingPin {
                library: identity("pkg"),
                selections: Box::new([first.clone(), second.clone()]),
            })
        );
    }
    let repeated =
        PinnedRequest::new([(identity("pkg"), good.clone()), (identity("pkg"), good)]).unwrap();
    assert!(bind(candidate(recomputed()), &repeated).is_ok());
}

/// Condition 3 reads a resolved `LibraryLock` as its pinned request: a lock
/// selecting `pkg@1` at the recomputed id admits the candidate, and a lock
/// selecting another version of the same id refuses it as a revision
/// mismatch.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn a_resolved_library_lock_is_a_pinned_request() {
    let root_preimage = one_node_preimage(b"root::Q", &["Q"]);
    let root = |version: &str| LibraryPackage {
        library: identity("root"),
        version: "1".to_owned(),
        package_id: PackageId::of_preimage(&root_preimage),
        identity_preimage: root_preimage.clone().into_boxed_slice(),
        imports: vec![ImportDeclaration {
            library: identity("pkg"),
            version: version.to_owned(),
            package_id: recomputed(),
            qualifier: None,
        }],
        exports: vec!["Q".to_owned()],
    };
    let lock = resolve_libraries(&root("1"), &[candidate(recomputed())]).unwrap();
    assert!(bind(candidate(recomputed()), &PinnedRequest::from(&lock)).is_ok());

    let mut other_version = candidate(recomputed());
    other_version.version = "2".to_owned();
    let lock = resolve_libraries(&root("2"), &[other_version]).unwrap();
    assert_eq!(
        bind(candidate(recomputed()), &PinnedRequest::from(&lock))
            .unwrap_err()
            .cause(),
        super::LibraryCause::RevisionMismatch
    );
}

/// A preimage declaring `nodes` (seed, qualified name, node tag) with no
/// nominal identity, in the ascending node-id order the reader requires.
fn preimage_declaring(nodes: &[(&str, &str, &str)]) -> Vec<u8> {
    let mut sorted = nodes.to_vec();
    sorted.sort_by_key(|(seed, _, _)| hex(seed.as_bytes()));
    let projection: Vec<Value> = sorted
        .iter()
        .map(|(seed, name, tag)| {
            let reference =
                json!({"digest": hex(seed.as_bytes()), "domain": "quire.checked-semantic-node/v1"});
            json!({
                "body": {},
                "declaration": {"qualified_name": [name]},
                "dependencies": [],
                "node_id": reference,
                "node_tag": tag,
                "schema_version": "quire.checked-semantic-graph/v2",
                "semantic_form": "declared",
                "semantic_type": reference,
            })
        })
        .collect();
    let mut preimage: Value = serde_json::from_slice(&preimage()).unwrap();
    preimage["identity_projection"] = Value::Array(projection);
    serde_json::to_vec(&preimage).unwrap()
}

/// FR-087-AC-4, TC-254 step 1's separate case: two exports of different
/// node kinds, at different `WireNodeId`s, both reach the `ImportView`, each
/// exported name mapped to its own node id.
#[trace("TC-254", "FR-087-AC-4")]
#[test]
fn import_view_carries_exports_of_different_kinds() {
    let bytes = preimage_declaring(&[
        ("pkg::Flag", "Flag", "scalar_type"),
        ("pkg::go", "go", "function"),
    ]);
    let package_id = PackageId::of_preimage(&bytes);
    let package = LibraryPackage {
        library: identity("pkg"),
        version: "1".to_owned(),
        package_id,
        identity_preimage: bytes.into_boxed_slice(),
        imports: Vec::new(),
        exports: vec!["Flag".to_owned(), "go".to_owned()],
    };
    let view = bind(package, &pin("1", package_id))
        .expect("all three conditions hold")
        .into_import_view();
    let key = |seed: &str| {
        PackageNodeKey::new(
            package_id,
            WireNodeId::from_hex(&hex(seed.as_bytes())).unwrap(),
        )
    };
    assert_eq!(
        view.exports().collect::<Vec<_>>(),
        vec![("Flag", key("pkg::Flag")), ("go", key("pkg::go"))]
    );
}
