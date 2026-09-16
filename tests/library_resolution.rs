// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-227 reusable semantic library resolution over the real `value`
//! boundary (FR-307), following the L01–L07 vectors of the FR-307 amendment in
//! QSpec PR #74.
//!
//! `package_id` values and export node keys are opaque fixture digests:
//! CheckedPackage V2 is not available to the value layer (SPEC-GAP(119-21)).

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use quire_spec_language::diagnostic::Code;
use quire_spec_language::value::{
    check_migration, resolve_libraries, Export, ExportIdentity, ImportDeclaration, LibraryName,
    LibraryPackage, LibraryRefusal, MigrationRefusal, NameReference, NameRefusal, NodeKey,
    PackageId, Selection,
};
use sha2::{Digest, Sha256};

fn digest(label: &str) -> [u8; 32] {
    Sha256::digest(label.as_bytes()).into()
}

fn id(label: &str) -> PackageId {
    PackageId(digest(label))
}

fn node(label: &str) -> NodeKey {
    let hex: String = digest(label)
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
    package: &str,
    qualifier: Option<&str>,
) -> ImportDeclaration {
    ImportDeclaration {
        library: name(library),
        version: version.to_owned(),
        package_id: id(package),
        qualifier: qualifier.map(str::to_owned),
    }
}

fn package(
    library: &str,
    version: &str,
    package: &str,
    imports: Vec<ImportDeclaration>,
    exports: &[&str],
) -> LibraryPackage {
    LibraryPackage {
        library: name(library),
        version: version.to_owned(),
        package_id: id(package),
        imports,
        exports: exports
            .iter()
            .map(|export| Export {
                name: (*export).to_owned(),
                node: node(&format!("{package}::{export}")),
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

/// A library `library` importing `L@1` with `package_id` `l_package` as `l`.
fn over_l(library: &str, l_version: &str, l_package: &str) -> LibraryPackage {
    package(
        library,
        "1",
        &format!("{library}@1"),
        vec![import("L", l_version, l_package, Some("l"))],
        &[],
    )
}

#[trace("TC-227", "FR-307-AC-1")]
#[test]
fn a_compatible_diamond_resolves_one_export_identity_and_a_reproducible_lock() {
    // L04 and L06: P imports Z then A; both import the same `L@1`.
    let root = package(
        "P",
        "1",
        "P@1",
        vec![
            import("Z", "1", "Z@1", Some("z")),
            import("A", "1", "A@1", Some("a")),
        ],
        &[],
    );
    let supplied = vec![
        over_l("Z", "1", "L@1"),
        over_l("A", "1", "L@1"),
        library_l(),
    ];
    let lock = resolve_libraries(&root, &supplied).unwrap();

    let selection = |version: &str, package: &str| Selection {
        version: version.to_owned(),
        package_id: id(package),
    };
    assert_eq!(
        lock.selections(),
        vec![
            (name("A"), selection("1", "A@1")),
            (name("L"), selection("1", "L@1")),
            (name("Z"), selection("1", "Z@1")),
        ]
    );

    // L01: the export identity is (L package key, `R` node key) on every path.
    let identity = ExportIdentity {
        package: id("L@1"),
        node: node("L@1::R"),
    };
    for user in ["A", "Z"] {
        assert_eq!(
            lock.resolve_name(&name(user), &qualified("l", "R")),
            Ok(identity)
        );
    }

    // The lock is reproducible from any supplied order.
    let mut reversed = supplied.clone();
    reversed.reverse();
    assert_eq!(resolve_libraries(&root, &reversed).unwrap(), lock);

    // L01: an import digest that is not the library's `package_id`.
    let stale = over_l("P", "1", "L-raw-source-bytes");
    let refusal = resolve_libraries(&stale, &[library_l()]).unwrap_err();
    assert_eq!(
        refusal,
        LibraryRefusal::StaleDependency {
            path: path(&["P", "L"]),
            import: import("L", "1", "L-raw-source-bytes", Some("l")),
        }
    );
    assert_eq!(refusal.code(), Code::StaleDependency);
    let refusal = resolve_libraries(&over_l("P", "2", "L@1"), &[library_l()]).unwrap_err();
    assert_eq!(refusal.code(), Code::StaleDependency);

    let refusal = resolve_libraries(&over_l("P", "1", "L@1"), &[]).unwrap_err();
    assert_eq!(
        refusal,
        LibraryRefusal::MissingImport {
            path: path(&["P", "L"]),
        }
    );
    assert_eq!(refusal.code(), Code::MissingImport);
}

#[trace("TC-227", "FR-307-AC-2")]
#[test]
fn conflicts_cycles_and_ambiguous_or_unqualified_names_refuse_with_paths() {
    // L02: an import without `as`.
    let unqualified_import = package("P", "1", "P@1", vec![import("L", "1", "L@1", None)], &[]);
    let refusal = resolve_libraries(&unqualified_import, &[library_l()]).unwrap_err();
    assert_eq!(
        refusal,
        LibraryRefusal::MissingQualifier {
            path: path(&["P", "L"]),
        }
    );
    assert_eq!(refusal.code(), Code::InvalidPackage);

    // L02: an unqualified use of an imported name.
    let lock = resolve_libraries(&over_l("P", "1", "L@1"), &[library_l()]).unwrap();
    let unqualified = NameReference::Unqualified("R".to_owned());
    let refusal = lock.resolve_name(&name("P"), &unqualified).unwrap_err();
    assert_eq!(refusal, NameRefusal::MissingDeclaration(unqualified));
    assert_eq!(refusal.code(), Code::MissingDeclaration);

    // L03: two libraries imported `as a`.
    let shared = package(
        "P",
        "1",
        "P@1",
        vec![
            import("A", "1", "A@1", Some("a")),
            import("B", "1", "B@1", Some("a")),
        ],
        &[],
    );
    let lock = resolve_libraries(
        &shared,
        &[
            package("A", "1", "A@1", Vec::new(), &["R"]),
            package("B", "1", "B@1", Vec::new(), &["R"]),
        ],
    )
    .unwrap();
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
    assert_eq!(refusal.code(), Code::AmbiguousDeclaration);

    // L04: B reaches L with a different `package_id`, then a different version.
    let root = package(
        "P",
        "1",
        "P@1",
        vec![
            import("A", "1", "A@1", Some("a")),
            import("B", "1", "B@1", Some("b")),
        ],
        &[],
    );
    for (version, l_package) in [("1", "L@1-other"), ("2", "L@2")] {
        let supplied = [
            over_l("A", "1", "L@1"),
            over_l("B", version, l_package),
            library_l(),
            package("L", version, l_package, Vec::new(), &["R"]),
        ];
        let refusal = resolve_libraries(&root, &supplied).unwrap_err();
        assert_eq!(
            refusal,
            LibraryRefusal::ConflictingDefinition {
                library: name("L"),
                paths: [path(&["P", "A", "L"]), path(&["P", "B", "L"])],
            }
        );
        assert_eq!(refusal.code(), Code::InvalidPackage);
    }

    // L05: A imports B and B imports A.
    let a = package(
        "A",
        "1",
        "A@1",
        vec![import("B", "1", "B@1", Some("b"))],
        &[],
    );
    let b = package(
        "B",
        "1",
        "B@1",
        vec![import("A", "1", "A@1", Some("a"))],
        &[],
    );
    let refusal = resolve_libraries(&a, &[a.clone(), b]).unwrap_err();
    assert_eq!(
        refusal,
        LibraryRefusal::ImportCycle {
            cycle: path(&["A", "B", "A"]),
        }
    );
    assert_eq!(refusal.code(), Code::InvalidPackage);
}

#[trace("TC-227", "FR-307-AC-3")]
#[test]
fn migration_creates_new_identities_and_never_relabels_evidence() {
    // L07: L migrates to L', and P (importing L) to P' (importing L').
    let old_library = library_l();
    let new_library = package("L", "2", "L'@2", Vec::new(), &["R"]);
    let old_package = over_l("P", "1", "L@1");
    let mut new_package = over_l("P", "2", "L'@2");
    new_package.package_id = id("P'@2");

    let library_migration = check_migration(&old_library, &new_library).unwrap();
    let package_migration = check_migration(&old_package, &new_package).unwrap();
    assert_eq!(library_migration.from, (name("L"), id("L@1")));
    assert_eq!(library_migration.to, (name("L"), id("L'@2")));
    assert_ne!(package_migration.from.1, package_migration.to.1);

    let old_lock = resolve_libraries(&old_package, std::slice::from_ref(&old_library)).unwrap();
    let old_identity = old_lock
        .resolve_name(&name("P"), &qualified("l", "R"))
        .unwrap();
    let evidence = BTreeMap::from([(old_identity, "checked against P@1")]);

    let new_lock =
        resolve_libraries(&new_package, &[old_library.clone(), new_library.clone()]).unwrap();
    let new_identity = new_lock
        .resolve_name(&name("P"), &qualified("l", "R"))
        .unwrap();
    assert_ne!(new_identity, old_identity);
    assert_eq!(new_identity.package, id("L'@2"));
    assert_eq!(evidence.get(&old_identity), Some(&"checked against P@1"));
    assert_eq!(evidence.get(&new_identity), None);
    assert_eq!(
        old_lock
            .resolve_name(&name("P"), &qualified("l", "R"))
            .unwrap(),
        old_identity
    );

    // The successor must not reuse the migrated `package_id`.
    let reused = package("L", "2", "L@1", Vec::new(), &["R"]);
    let refusal = check_migration(&old_library, &reused).unwrap_err();
    assert_eq!(refusal, MigrationRefusal::PackageIdReused);
    assert_eq!(refusal.code(), Code::InvalidPackage);
}
