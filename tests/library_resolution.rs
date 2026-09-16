// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-227 reusable semantic library resolution over the real `value`
//! boundary (FR-307), following vectors L01–L08 of the vendored TC-227.
//!
//! Each fixture package's identity preimage is a small JCS object labelled by
//! the package, and its `package_id` is the SHA-256 of those bytes, as for a
//! `quire.package.semantic/v2` digest. The value layer hashes the preimage
//! without reading it, so the fixture preimages carry no semantic graph. Export
//! node keys are opaque fixture digests.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use quire_spec_language::diagnostic::Code;
use quire_spec_language::value::{
    check_migration, resolve_libraries, Export, ExportIdentity, ImportDeclaration, LibraryCause,
    LibraryMigration, LibraryName, LibraryPackage, LibraryRefusal, NameReference, NameRefusal,
    NodeKey, PackageId, Selection, StaleCause,
};
use sha2::{Digest, Sha256};

/// The JCS identity preimage bytes of fixture package `label`.
fn preimage(label: &str) -> Box<[u8]> {
    format!(r#"{{"identity_projection":["{label}"],"version":"quire.checked-package-id/v2"}}"#)
        .into_bytes()
        .into_boxed_slice()
}

fn id(label: &str) -> PackageId {
    PackageId::of_preimage(&preimage(label))
}

fn node(label: &str) -> NodeKey {
    let hex: String = Sha256::digest(label.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    NodeKey::from_hex(&hex).unwrap()
}

fn name(library: &str) -> LibraryName {
    LibraryName::new(vec![library.to_owned()]).unwrap()
}

fn path(libraries: &[&str]) -> Vec<LibraryName> {
    libraries.iter().copied().map(name).collect()
}

fn import(
    library: &str,
    version: &str,
    package_id: PackageId,
    qualifier: Option<&str>,
) -> ImportDeclaration {
    ImportDeclaration {
        library: name(library),
        version: version.to_owned(),
        package_id,
        qualifier: qualifier.map(str::to_owned),
    }
}

/// Package `label` of `library` at `version`, whose `package_id` is the digest
/// of its own preimage.
fn package(
    library: &str,
    version: &str,
    label: &str,
    imports: Vec<ImportDeclaration>,
    exports: &[&str],
) -> LibraryPackage {
    LibraryPackage {
        library: name(library),
        version: version.to_owned(),
        package_id: id(label),
        identity_preimage: preimage(label),
        imports,
        exports: exports
            .iter()
            .map(|export| Export {
                name: (*export).to_owned(),
                node: node(&format!("{label}::{export}")),
            })
            .collect(),
    }
}

fn qualified(qualifier: &str, export: &str) -> NameReference {
    NameReference::Qualified {
        qualifier: qualifier.to_owned(),
        name: export.to_owned(),
    }
}

/// `L@1` exporting `R`.
fn library_l() -> LibraryPackage {
    package("L", "1", "L@1", Vec::new(), &["R"])
}

/// Library `library@1` importing L at `l_version` with `l_package` as `l`.
fn over_l(library: &str, l_version: &str, l_package: PackageId) -> LibraryPackage {
    package(
        library,
        "1",
        &format!("{library}@1"),
        vec![import("L", l_version, l_package, Some("l"))],
        &[],
    )
}

/// Assert a library refusal's complete record, code and cause.
fn assert_library_refusal(
    outcome: Result<impl std::fmt::Debug, LibraryRefusal>,
    expected: &LibraryRefusal,
    code: Code,
    cause: LibraryCause,
) {
    let refusal = outcome.unwrap_err();
    assert_eq!(&refusal, expected);
    assert_eq!((refusal.code(), refusal.cause()), (code, cause));
}

#[trace("TC-227", "FR-307-AC-1")]
#[trace("TC-227", "FR-307-AC-4")]
#[test]
fn l01_an_import_binds_the_library_package_id() {
    let lock = resolve_libraries(&over_l("P", "1", id("L@1")), &[library_l()]).unwrap();
    assert_eq!(
        lock.resolve_name(&name("P"), &qualified("l", "R")),
        Ok(ExportIdentity {
            package: id("L@1"),
            node: node("L@1::R"),
        })
    );

    let raw_source = PackageId(Sha256::digest(b"library L version 1 { R }").into());
    let by_bytes = over_l("P", "1", raw_source);
    assert_library_refusal(
        resolve_libraries(&by_bytes, &[library_l()]),
        &LibraryRefusal::StaleDependency {
            path: path(&["P", "L"]),
            import: import("L", "1", raw_source, Some("l")),
            cause: StaleCause::ByteDigestMismatch,
        },
        Code::StaleDependency,
        LibraryCause::ByteDigestMismatch,
    );

    assert_library_refusal(
        resolve_libraries(&over_l("P", "2", id("L@1")), &[library_l()]),
        &LibraryRefusal::StaleDependency {
            path: path(&["P", "L"]),
            import: import("L", "2", id("L@1"), Some("l")),
            cause: StaleCause::RevisionMismatch,
        },
        Code::StaleDependency,
        LibraryCause::RevisionMismatch,
    );
}

#[trace("TC-227", "FR-307-AC-2")]
#[trace("TC-227", "FR-307-AC-4")]
#[test]
fn l02_an_import_without_a_qualifier_binds_no_name() {
    let root = package(
        "P",
        "1",
        "P@1",
        vec![import("L", "1", id("L@1"), None)],
        &[],
    );
    let lock = resolve_libraries(&root, &[library_l()]).unwrap();
    assert_eq!(
        lock.selections(),
        [(
            name("L"),
            Selection {
                version: "1".to_owned(),
                package_id: id("L@1"),
            }
        )]
    );
    for reference in [
        NameReference::Unqualified("R".to_owned()),
        qualified("L", "R"),
    ] {
        let refusal = lock.resolve_name(&name("P"), &reference).unwrap_err();
        assert_eq!(refusal, NameRefusal::MissingDeclaration(reference));
        assert_eq!(
            (refusal.code(), refusal.cause()),
            (Code::MissingDeclaration, LibraryCause::MissingName)
        );
    }
}

#[trace("TC-227", "FR-307-AC-2")]
#[trace("TC-227", "FR-307-AC-4")]
#[test]
fn l03_a_shared_qualifier_is_ambiguous_at_each_use() {
    let root = package(
        "P",
        "1",
        "P@1",
        vec![
            import("A", "1", id("A@1"), Some("a")),
            import("B", "1", id("B@1"), Some("a")),
        ],
        &[],
    );
    let lock = resolve_libraries(
        &root,
        &[
            package("A", "1", "A@1", Vec::new(), &["R"]),
            package("B", "1", "B@1", Vec::new(), &["R"]),
        ],
    )
    .unwrap();
    for _use in 0..2 {
        let refusal = lock
            .resolve_name(&name("P"), &qualified("a", "R"))
            .unwrap_err();
        assert_eq!(
            refusal,
            NameRefusal::AmbiguousDeclaration {
                qualifier: "a".to_owned(),
                paths: vec![path(&["P", "A"]), path(&["P", "B"])],
            }
        );
        assert_eq!(
            (refusal.code(), refusal.cause()),
            (Code::AmbiguousDeclaration, LibraryCause::AmbiguousName)
        );
    }
}

fn diamond_root() -> LibraryPackage {
    package(
        "P",
        "1",
        "P@1",
        vec![
            import("A", "1", id("A@1"), Some("a")),
            import("B", "1", id("B@1"), Some("b")),
        ],
        &[],
    )
}

#[trace("TC-227", "FR-307-AC-1")]
#[trace("TC-227", "FR-307-AC-2")]
#[trace("TC-227", "FR-307-AC-4")]
#[test]
fn l04_a_diamond_unifies_only_one_version_and_package_id() {
    let root = diamond_root();
    let supplied = [
        over_l("A", "1", id("L@1")),
        over_l("B", "1", id("L@1")),
        library_l(),
    ];
    let lock = resolve_libraries(&root, &supplied).unwrap();
    let identity = ExportIdentity {
        package: id("L@1"),
        node: node("L@1::R"),
    };
    for user in ["A", "B"] {
        assert_eq!(
            lock.resolve_name(&name(user), &qualified("l", "R")),
            Ok(identity)
        );
    }
    let l_selections: Vec<_> = lock
        .selections()
        .into_iter()
        .filter(|(library, _)| *library == name("L"))
        .collect();
    assert_eq!(l_selections.len(), 1);

    for (version, label) in [("1", "L@1-other"), ("2", "L@2")] {
        let supplied = [
            over_l("A", "1", id("L@1")),
            over_l("B", version, id(label)),
            library_l(),
            package("L", version, label, Vec::new(), &["R"]),
        ];
        assert_library_refusal(
            resolve_libraries(&root, &supplied),
            &LibraryRefusal::ConflictingDefinition {
                library: name("L"),
                paths: [path(&["P", "A", "L"]), path(&["P", "B", "L"])],
            },
            Code::InvalidPackage,
            LibraryCause::ConflictingDefinition,
        );
    }
}

#[trace("TC-227", "FR-307-AC-2")]
#[trace("TC-227", "FR-307-AC-4")]
#[test]
fn l05_an_import_cycle_lists_its_dependency_edges() {
    let a = package(
        "A",
        "1",
        "A@1",
        vec![import("B", "1", id("B@1"), Some("b"))],
        &[],
    );
    let b = package(
        "B",
        "1",
        "B@1",
        vec![import("A", "1", id("A@1"), Some("a"))],
        &[],
    );
    assert_library_refusal(
        resolve_libraries(&a, &[a.clone(), b]),
        &LibraryRefusal::ImportCycle {
            cycle: path(&["A", "B", "A"]),
        },
        Code::InvalidPackage,
        LibraryCause::DefinitionCycle,
    );
}

#[trace("TC-227", "FR-307-AC-1")]
#[test]
fn l06_the_lock_lists_selections_in_ascending_identity_order() {
    let root = package(
        "P",
        "1",
        "P@1",
        vec![
            import("Z", "1", id("Z@1"), Some("z")),
            import("A", "1", id("A@1"), Some("a")),
        ],
        &[],
    );
    let supplied = vec![
        package("Z", "1", "Z@1", Vec::new(), &[]),
        package("A", "1", "A@1", Vec::new(), &[]),
    ];
    let lock = resolve_libraries(&root, &supplied).unwrap();
    let selection = |label: &str| Selection {
        version: "1".to_owned(),
        package_id: id(label),
    };
    assert_eq!(
        lock.selections(),
        [(name("A"), selection("A@1")), (name("Z"), selection("Z@1"))]
    );
    let mut reversed = supplied;
    reversed.reverse();
    assert_eq!(resolve_libraries(&root, &reversed).unwrap(), lock);
}

#[trace("TC-227", "FR-307-AC-3")]
#[test]
fn l07_migration_creates_new_identities_and_never_relabels_evidence() {
    let old_library = library_l();
    let new_library = package("L", "2", "L'@2", Vec::new(), &["R"]);
    let old_package = over_l("P", "1", id("L@1"));
    let new_package = package(
        "P",
        "2",
        "P'@2",
        vec![import("L", "2", id("L'@2"), Some("l"))],
        &[],
    );

    assert_eq!(
        check_migration(&old_library, &new_library),
        Ok(LibraryMigration::Migrated {
            from: (name("L"), id("L@1")),
            to: (name("L"), id("L'@2")),
        })
    );
    assert_eq!(
        check_migration(&old_package, &new_package),
        Ok(LibraryMigration::Migrated {
            from: (name("P"), id("P@1")),
            to: (name("P"), id("P'@2")),
        })
    );
    assert_eq!(
        check_migration(&old_library, &library_l()),
        Ok(LibraryMigration::Unchanged)
    );

    let old_lock = resolve_libraries(&old_package, std::slice::from_ref(&old_library)).unwrap();
    let old_identity = old_lock
        .resolve_name(&name("P"), &qualified("l", "R"))
        .unwrap();
    let evidence = BTreeMap::from([(old_identity, "checked against P@1")]);

    let new_lock = resolve_libraries(&new_package, &[old_library, new_library]).unwrap();
    let new_identity = new_lock
        .resolve_name(&name("P"), &qualified("l", "R"))
        .unwrap();
    assert_ne!(new_identity, old_identity);
    assert_eq!(new_identity.package, id("L'@2"));
    assert_eq!(evidence.get(&old_identity), Some(&"checked against P@1"));
    assert_eq!(evidence.get(&new_identity), None);
    assert_eq!(
        old_lock.resolve_name(&name("P"), &qualified("l", "R")),
        Ok(old_identity)
    );
}

#[trace("TC-227", "FR-307-AC-3")]
#[trace("TC-227", "FR-307-AC-5")]
#[test]
fn l08_a_migrated_package_reusing_its_package_id_is_invalid_at_package_id() {
    let old_library = library_l();
    // L' changes an export, so its preimage differs, but claims L's id.
    let mut reused = package("L", "1", "L'@1", Vec::new(), &["R", "S"]);
    reused.package_id = id("L@1");
    let mismatch = LibraryRefusal::PackageIdMismatch {
        library: name("L"),
        claimed: id("L@1"),
        recomputed: id("L'@1"),
    };

    assert_library_refusal(
        check_migration(&old_library, &reused),
        &mismatch,
        Code::InvalidPackage,
        LibraryCause::InvalidValue,
    );
    assert_eq!(mismatch.member_path(), Some("/package_id"));

    let importer = over_l("P", "1", id("L@1"));
    assert_library_refusal(
        resolve_libraries(&importer, std::slice::from_ref(&reused)),
        &mismatch,
        Code::InvalidPackage,
        LibraryCause::InvalidValue,
    );
}
