// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-227 reusable semantic library resolution over the real `value`
//! boundary (FR-307), following TC-227's own vectors L01-L08.
//!
//! A fixture package is a library identity, its imports, and its exports:
//! qualified names, each with the structural node key of the declaration it
//! names.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::library::{
    check_migration, resolve_libraries, ImportDeclaration, ImportView, LibraryCause,
    LibraryMigration, LibraryName, LibraryPackage, LibraryRefusal, RefusalClass,
};
use quire_exact::{NodeKey, PathSegment};

/// The exports declared by fixture package `label`.
fn exports_of(label: &str) -> &'static [&'static str] {
    match label {
        "L@1" | "L@1-other" | "L@2" | "L'@2" | "A@1" | "B@1" => &["R"],
        "L'@1" => &["R", "S"],
        _ => &[],
    }
}

/// The structural node key of fixture node `label`.
fn node_key(label: &str) -> NodeKey {
    NodeKey::new("lib", vec![PathSegment::Text(label.to_owned())], 0).unwrap()
}

fn name(library: &str) -> LibraryName {
    LibraryName::new(library).unwrap()
}

fn path(libraries: &[&str]) -> Vec<LibraryName> {
    libraries.iter().copied().map(name).collect()
}

fn import(library: &str, qualifier: Option<&str>) -> ImportDeclaration {
    ImportDeclaration {
        library: name(library),
        qualifier: qualifier.map(str::to_owned),
    }
}

/// Package `label` of `library`: one export per name `label` declares, each
/// keyed by its own node.
fn package(library: &str, label: &str, imports: Vec<ImportDeclaration>) -> LibraryPackage {
    LibraryPackage {
        library: name(library),
        imports,
        exports: exports_of(label)
            .iter()
            .map(|export| {
                (
                    (*export).to_owned(),
                    node_key(&format!("{label}::{export}")),
                )
            })
            .collect(),
    }
}

/// `L@1` exporting `R`.
fn library_l() -> LibraryPackage {
    package("L", "L@1", Vec::new())
}

/// Library `library@1` importing L as `l`.
fn over_l(library: &str) -> LibraryPackage {
    package(
        library,
        &format!("{library}@1"),
        vec![import("L", Some("l"))],
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

/// Assert a library refusal's complete record, code, cause and TC-282
/// classification (FR-087-AC-12): unlike `assert_library_refusal`, this also
/// asserts `LibraryRefusal::class()` itself.
fn assert_library_refusal_classified(
    outcome: Result<impl std::fmt::Debug, LibraryRefusal>,
    expected: &LibraryRefusal,
    code: Code,
    cause: LibraryCause,
    class: RefusalClass,
) {
    let refusal = outcome.unwrap_err();
    assert_eq!(&refusal, expected);
    assert_eq!((refusal.code(), refusal.cause()), (code, cause));
    assert_eq!(refusal.class(), class);
}

#[trace("QSpec-TC-227", "QSpec-FR-307-AC-1")]
#[test]
fn l01_an_import_selects_the_library() {
    let lock = resolve_libraries(&over_l("P"), &[library_l()]).unwrap();
    assert_eq!(lock.selections(), [name("L")]);
}

#[trace("QSpec-TC-227", "QSpec-FR-307-AC-1")]
#[test]
fn an_import_selects_the_supplied_library_of_its_identity() {
    let supplied = package("L", "L'@2", Vec::new());
    let lock = resolve_libraries(&over_l("P"), &[supplied]).unwrap();
    assert_eq!(lock.selections(), [name("L")]);
}

#[trace("QSpec-TC-227", "QSpec-FR-307-AC-2")]
#[test]
fn l02_an_import_without_a_qualifier_selects_its_library() {
    let root = package("P", "P@1", vec![import("L", None)]);
    let lock = resolve_libraries(&root, &[library_l()]).unwrap();
    assert_eq!(lock.selections(), [name("L")]);
}

fn diamond_root() -> LibraryPackage {
    package(
        "P",
        "P@1",
        vec![import("A", Some("a")), import("B", Some("b"))],
    )
}

#[trace("QSpec-TC-227", "QSpec-FR-307-AC-1")]
#[trace("QSpec-TC-227", "QSpec-FR-307-AC-2")]
#[test]
fn l04_a_diamond_selects_its_shared_library_once() {
    let root = diamond_root();
    let supplied = [over_l("A"), over_l("B"), library_l()];
    let lock = resolve_libraries(&root, &supplied).unwrap();
    let l_selections: Vec<_> = lock
        .selections()
        .into_iter()
        .filter(|library| *library == name("L"))
        .collect();
    assert_eq!(l_selections.len(), 1);
}

#[trace("QSpec-TC-227", "QSpec-FR-307-AC-2")]
#[trace("TC-282", "FR-087-AC-12")]
#[test]
fn l05_an_import_cycle_lists_its_dependency_edges() {
    let a = package("A", "A@1", vec![import("B", Some("b"))]);
    let b = package("B", "B@1", vec![import("A", Some("a"))]);
    // TC-282 (FR-087-AC-12): `ImportCycle` classifies to ADR-011 I2's third
    // rule.
    assert_library_refusal_classified(
        resolve_libraries(&a, &[a.clone(), b]),
        &LibraryRefusal::ImportCycle {
            cycle: path(&["A", "B", "A"]),
        },
        Code::InvalidPackage,
        LibraryCause::DefinitionCycle,
        RefusalClass::I2Rule(3),
    );
}

#[trace("QSpec-TC-227", "QSpec-FR-307-AC-1")]
#[test]
fn l06_the_lock_lists_selections_in_ascending_identity_order() {
    let root = package(
        "P",
        "P@1",
        vec![import("Z", Some("z")), import("A", Some("a"))],
    );
    let supplied = vec![
        package("Z", "Z@1", Vec::new()),
        package("A", "A@1", Vec::new()),
    ];
    let lock = resolve_libraries(&root, &supplied).unwrap();
    assert_eq!(lock.selections(), [name("A"), name("Z")]);
    let mut reversed = supplied;
    reversed.reverse();
    assert_eq!(resolve_libraries(&root, &reversed).unwrap(), lock);
}

#[trace("QSpec-TC-227", "QSpec-FR-307-AC-3")]
#[test]
fn l07_migration_is_a_changed_export_set_and_never_relabels_evidence() {
    let old_library = library_l();
    let new_library = package("L", "L'@2", Vec::new());
    let old_package = over_l("P");
    let new_package = package("P", "P'@2", vec![import("L", Some("l"))]);

    assert_eq!(
        check_migration(&old_library, &new_library),
        Ok(LibraryMigration::Migrated {
            from: name("L"),
            to: name("L"),
        })
    );
    assert_eq!(
        check_migration(&old_package, &new_package),
        Ok(LibraryMigration::Migrated {
            from: name("P"),
            to: name("P"),
        })
    );
    assert_eq!(
        check_migration(&old_library, &library_l()),
        Ok(LibraryMigration::Unchanged)
    );

    // The old lock still resolves against `old_library`, and the new lock
    // against `new_library`. Supplying both packages of `L` is a conflicting
    // definition, not a choice between them.
    let old_lock = resolve_libraries(&old_package, std::slice::from_ref(&old_library)).unwrap();
    assert_eq!(old_lock.root(), &old_package);
    assert_library_refusal(
        resolve_libraries(&new_package, &[old_library, new_library.clone()]),
        &LibraryRefusal::ConflictingDefinition { library: name("L") },
        Code::InvalidPackage,
        LibraryCause::ConflictingDefinition,
    );
    let new_lock = resolve_libraries(&new_package, &[new_library]).unwrap();
    assert_eq!(new_lock.selections(), [name("L")]);
}

#[trace("QSpec-TC-227", "QSpec-FR-307-AC-5")]
#[test]
fn an_import_view_lists_each_export_with_its_structural_key() {
    let view = ImportView::of(&library_l()).unwrap();
    let exports: Vec<_> = view.exports().collect();
    assert_eq!(exports, [("R", &node_key("L@1::R"))]);
}

#[trace("QSpec-TC-227", "QSpec-FR-307-AC-5")]
#[test]
fn an_export_name_that_is_not_a_qualified_name_is_invalid() {
    for export in ["", "1x", "A::", "::A", "A::1"] {
        let mut malformed = library_l();
        malformed.exports = vec![(export.to_owned(), node_key("L@1::R"))];
        let expected = LibraryRefusal::InvalidExportName {
            library: name("L"),
            export: export.to_owned(),
        };
        assert_library_refusal_classified(
            resolve_libraries(&over_l("P"), std::slice::from_ref(&malformed)),
            &expected,
            Code::InvalidPackage,
            LibraryCause::InvalidValue,
            RefusalClass::BindingCondition,
        );
        assert_library_refusal(
            ImportView::of(&malformed),
            &expected,
            Code::InvalidPackage,
            LibraryCause::InvalidValue,
        );
        assert_library_refusal(
            check_migration(&library_l(), &malformed),
            &expected,
            Code::InvalidPackage,
            LibraryCause::InvalidValue,
        );
    }
}

#[trace("QSpec-TC-227", "QSpec-FR-307-AC-5")]
#[test]
fn one_export_name_declared_twice_is_ambiguous() {
    let mut ambiguous = library_l();
    ambiguous.exports = vec![
        ("R".to_owned(), node_key("L@1::R")),
        ("R".to_owned(), node_key("L@1::S")),
    ];
    let expected = LibraryRefusal::AmbiguousDeclaration {
        library: name("L"),
        export: "R".to_owned(),
    };
    assert_library_refusal_classified(
        resolve_libraries(&over_l("P"), std::slice::from_ref(&ambiguous)),
        &expected,
        Code::AmbiguousDeclaration,
        LibraryCause::AmbiguousName,
        RefusalClass::BindingCondition,
    );
    assert_library_refusal(
        check_migration(&library_l(), &ambiguous),
        &expected,
        Code::AmbiguousDeclaration,
        LibraryCause::AmbiguousName,
    );
}

#[trace("QSpec-TC-227", "QSpec-FR-307-AC-1")]
#[test]
fn a_multi_segment_export_is_admitted_with_its_own_key() {
    let mut library = package("Example", "Example", Vec::new());
    library.exports = vec![
        ("Example::Status".to_owned(), node_key("Example::Status")),
        ("Example::metre".to_owned(), node_key("Example::metre")),
    ];
    let root = package("P", "P@1", vec![import("Example", Some("e"))]);
    resolve_libraries(&root, std::slice::from_ref(&library)).unwrap();
    let view = ImportView::of(&library).unwrap();
    assert_eq!(view.exports().count(), 2);
}

// TC-282 (FR-087-AC-12): every `LibraryRefusal` variant `resolve_libraries`
// can return classifies to exactly one of an ADR-011 I2 graph rule, a
// per-package admission check, or E3 name resolution. `ConflictingDefinition`
// (I2 rule 2) and `ImportCycle` (I2 rule 3) reuse l07's and l05's own
// fixtures above; the export-name refusals are classified above.

#[trace("TC-282", "FR-087-AC-12")]
#[test]
fn invalid_qualifier_classifies_to_e3_name_resolution() {
    let root = package("P", "P@1", vec![import("L", Some("1l"))]);
    assert_library_refusal_classified(
        resolve_libraries(&root, &[library_l()]),
        &LibraryRefusal::InvalidQualifier {
            path: path(&["P", "L"]),
        },
        Code::InvalidPackage,
        LibraryCause::InvalidValue,
        RefusalClass::E3NameResolution,
    );
}

#[trace("TC-282", "FR-087-AC-12")]
#[test]
fn conflicting_definition_classifies_to_i2_rule_2() {
    assert_library_refusal_classified(
        resolve_libraries(
            &over_l("P"),
            &[library_l(), package("L", "L'@2", Vec::new())],
        ),
        &LibraryRefusal::ConflictingDefinition { library: name("L") },
        Code::InvalidPackage,
        LibraryCause::ConflictingDefinition,
        RefusalClass::I2Rule(2),
    );
}

#[trace("TC-282", "FR-087-AC-12")]
#[test]
fn missing_import_classifies_to_i2_rule_1() {
    let root = package("P", "P@1", vec![import("Z", Some("z"))]);
    assert_library_refusal_classified(
        resolve_libraries(&root, &[]),
        &LibraryRefusal::MissingImport {
            path: path(&["P", "Z"]),
        },
        Code::MissingImport,
        LibraryCause::MissingSelection,
        RefusalClass::I2Rule(1),
    );
}
