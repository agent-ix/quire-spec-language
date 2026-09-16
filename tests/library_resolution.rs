// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-227 reusable semantic library resolution over the real `value`
//! boundary (FR-307), following vectors L01–L08 of the vendored TC-227.
//!
//! Each fixture package's identity preimage is a structurally valid
//! `quire.checked-package-id/v2` JCS object whose `identity_projection` holds
//! one nominal node per export (L09 adds function, type and constant nodes
//! declared by `binding` bodies), and its `package_id` is the SHA-256 of those
//! bytes. Export node keys are derived from the projection nodes.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use quire_spec_language::diagnostic::Code;
use quire_spec_language::value::{
    check_migration, resolve_libraries, ExportIdentity, ImportDeclaration, LibraryCause,
    LibraryMigration, LibraryName, LibraryPackage, LibraryRefusal, NameReference, NameRefusal,
    NodeDefect, NodeKey, PackageId, PreimageDefect, Selection, StaleCause,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// The exports declared by fixture package `label`.
fn exports_of(label: &str) -> &'static [&'static str] {
    match label {
        "L@1" | "L@1-other" | "L@2" | "L'@2" | "A@1" | "B@1" => &["R"],
        "L'@1" => &["R", "S"],
        _ => &[],
    }
}

fn node(label: &str) -> NodeKey {
    NodeKey::from_hex(&hex(label)).unwrap()
}

fn hex(label: &str) -> String {
    Sha256::digest(label.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn selection(identity: &str) -> Value {
    json!({
        "definition": {
            "authority": "agent-ix",
            "digest": hex(identity),
            "digest_domain": "quire.definition.bytes/v1",
            "identity": identity,
            "revision": {"namespace": "semver", "value": "1"},
        },
        "role": "edition",
    })
}

/// A projection node with digest `hex(label)`, declaring `declaration` when
/// given.
fn projection_node(label: &str, declaration: Option<&str>) -> Value {
    let reference = json!({"digest": hex(label), "domain": "quire.checked-semantic-node/v1"});
    let mut node = json!({
        "body": {"members": [], "term": "aggregate"},
        "dependencies": [],
        "node_id": reference,
        "node_tag": "scalar_type",
        "schema_version": "quire.checked-semantic-graph/v2",
        "semantic_form": "enum",
        "semantic_type": reference,
    });
    if let Some(declaration) = declaration {
        node["nominal_identity_preimage"] = json!({
            "members": ["READY"],
            "ordered": true,
            "owner": {"authority": "agent-ix", "identity": "library", "kind": "definition"},
            "qualified_declaration": [declaration],
            "version": "quire.enum-declaration-node/v1",
        });
    }
    node
}

/// The identity preimage value of fixture package `label` with projection
/// `nodes`.
fn preimage_value(nodes: Vec<Value>) -> Value {
    json!({
        "definition_selections": [],
        "dependency_selections": [],
        "edition": selection("quire-edition"),
        "identity_projection": nodes,
        "model_selections": [],
        "profile_selections": [],
        "required_features": ["quire.value.complete/v1"],
        "version": "quire.checked-package-id/v2",
    })
}

/// The projection of fixture package `label`: one node per export, or one
/// undeclared node, ascending by node id.
fn projection(label: &str) -> Vec<Value> {
    let mut nodes: Vec<(String, Value)> = exports_of(label)
        .iter()
        .map(|export| {
            let node_label = format!("{label}::{export}");
            (hex(&node_label), projection_node(&node_label, Some(export)))
        })
        .collect();
    if nodes.is_empty() {
        nodes.push((hex(label), projection_node(label, None)));
    }
    nodes.sort_by(|left, right| left.0.cmp(&right.0));
    nodes.into_iter().map(|(_, node)| node).collect()
}

fn jcs(value: &Value) -> Box<[u8]> {
    serde_json::to_vec(value).unwrap().into_boxed_slice()
}

/// The JCS identity preimage bytes of fixture package `label`.
fn preimage(label: &str) -> Box<[u8]> {
    jcs(&preimage_value(projection(label)))
}

fn id(label: &str) -> PackageId {
    PackageId::of_preimage(&preimage(label))
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
) -> LibraryPackage {
    LibraryPackage {
        library: name(library),
        version: version.to_owned(),
        package_id: id(label),
        identity_preimage: preimage(label),
        imports,
        exports: exports_of(label)
            .iter()
            .map(|export| (*export).to_owned())
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
    package("L", "1", "L@1", Vec::new())
}

/// Library `library@1` importing L at `l_version` with `l_package` as `l`.
fn over_l(library: &str, l_version: &str, l_package: PackageId) -> LibraryPackage {
    package(
        library,
        "1",
        &format!("{library}@1"),
        vec![import("L", l_version, l_package, Some("l"))],
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
    let root = package("P", "1", "P@1", vec![import("L", "1", id("L@1"), None)]);
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
    );
    let lock = resolve_libraries(
        &root,
        &[
            package("A", "1", "A@1", Vec::new()),
            package("B", "1", "B@1", Vec::new()),
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
            package("L", version, label, Vec::new()),
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
    );
    let b = package(
        "B",
        "1",
        "B@1",
        vec![import("A", "1", id("A@1"), Some("a"))],
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
    );
    let supplied = vec![
        package("Z", "1", "Z@1", Vec::new()),
        package("A", "1", "A@1", Vec::new()),
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
    let new_library = package("L", "2", "L'@2", Vec::new());
    let old_package = over_l("P", "1", id("L@1"));
    let new_package = package(
        "P",
        "2",
        "P'@2",
        vec![import("L", "2", id("L'@2"), Some("l"))],
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
    let mut reused = package("L", "1", "L'@1", Vec::new());
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

/// Preimage bytes, exported names and the expected defect.
type MalformedCase = (Box<[u8]>, &'static [&'static str], PreimageDefect);

/// A package of `L@1` whose identity preimage is `bytes`, claiming their
/// digest, and exporting `exports`.
fn with_preimage(bytes: Box<[u8]>, exports: &[&str]) -> LibraryPackage {
    LibraryPackage {
        library: name("L"),
        version: "1".to_owned(),
        package_id: PackageId::of_preimage(&bytes),
        identity_preimage: bytes,
        imports: Vec::new(),
        exports: exports.iter().map(|export| (*export).to_owned()).collect(),
    }
}

#[trace("TC-227", "FR-307-AC-5")]
#[test]
fn l08_a_structurally_malformed_identity_preimage_is_invalid_before_resolution() {
    let valid = preimage_value(projection("L@1"));
    let node_r = projection_node("L@1::R", Some("R"));
    let node_s = projection_node("L@1::S", Some("S"));
    let (low, high) = if hex("L@1::R") < hex("L@1::S") {
        (node_r.clone(), node_s)
    } else {
        (node_s, node_r.clone())
    };

    let mut wrong_version = valid.clone();
    wrong_version["version"] = json!("quire.checked-package-id/v1");
    let mut missing_member = valid.clone();
    missing_member
        .as_object_mut()
        .unwrap()
        .remove("model_selections");
    let mut missing_node_member = valid.clone();
    missing_node_member["identity_projection"][0]
        .as_object_mut()
        .unwrap()
        .remove("semantic_type");
    let mut wrong_domain = valid.clone();
    wrong_domain["identity_projection"][0]["node_id"]["domain"] = json!("quire.other/v1");
    let empty = preimage_value(Vec::new());
    let descending = preimage_value(vec![high, low]);
    let repeated = preimage_value(vec![node_r.clone(), node_r]);
    let mut spaced = jcs(&valid).into_vec();
    spaced.insert(1, b' ');

    let cases: [MalformedCase; 9] = [
        (jcs(&wrong_version), &["R"], PreimageDefect::Version),
        (
            jcs(&missing_member),
            &["R"],
            PreimageDefect::MissingMember("model_selections"),
        ),
        (
            jcs(&missing_node_member),
            &["R"],
            PreimageDefect::Node {
                index: 0,
                defect: NodeDefect::MissingMember("semantic_type"),
            },
        ),
        (
            jcs(&wrong_domain),
            &["R"],
            PreimageDefect::Node {
                index: 0,
                defect: NodeDefect::NodeId,
            },
        ),
        (jcs(&empty), &[], PreimageDefect::EmptyProjection),
        (
            jcs(&descending),
            &[],
            PreimageDefect::NodeOrder { index: 1 },
        ),
        (jcs(&repeated), &[], PreimageDefect::NodeOrder { index: 1 }),
        (
            spaced.into_boxed_slice(),
            &["R"],
            PreimageDefect::NonCanonical,
        ),
        (
            jcs(&valid),
            &["S"],
            PreimageDefect::UndeclaredExport("S".to_owned()),
        ),
    ];
    let importer = over_l("P", "1", id("L@1"));
    for (bytes, exports, defect) in cases {
        let malformed = with_preimage(bytes, exports);
        let expected = LibraryRefusal::InvalidPreimage {
            library: name("L"),
            defect,
        };
        assert_eq!(expected.member_path(), Some("/identity_preimage"));
        assert_library_refusal(
            resolve_libraries(&importer, std::slice::from_ref(&malformed)),
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
    assert_eq!(
        with_preimage(jcs(&valid), &["R"]),
        library_l(),
        "the valid fixture is the supplied L@1"
    );
}

#[trace("TC-227", "FR-307-AC-1")]
#[test]
fn l01_export_node_keys_derive_from_a_checked_package_v2_identity_projection() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../resources/complete-value/quire-specification/proposals/checked-package-v2/fixtures/positive-nominal-identities.json"
    ))
    .unwrap();
    let digest = fixture["package_id"]["digest"].as_str().unwrap();
    let claimed = PackageId(*NodeKey::from_hex(digest).unwrap().as_bytes());
    let library = LibraryPackage {
        library: name("Example"),
        version: "1".to_owned(),
        package_id: claimed,
        identity_preimage: jcs(&fixture["identity_preimage"]),
        imports: Vec::new(),
        exports: vec!["Example::Status".to_owned(), "Example::metre".to_owned()],
    };
    let root = package(
        "P",
        "1",
        "P@1",
        vec![import("Example", "1", claimed, Some("e"))],
    );
    let lock = resolve_libraries(&root, &[library]).unwrap();
    for (export, node_id) in [
        (
            "Example::Status",
            "7928f1e1b570335b404c8d21c66da8a3b8e37e434b0ebc622f80285488811562",
        ),
        (
            "Example::metre",
            "79637623a46d29e884b62c6fa292aeb29d41e4ecc4e800b4d7ee910a3eaf23a4",
        ),
    ] {
        assert_eq!(
            lock.resolve_name(&name("P"), &qualified("e", export)),
            Ok(ExportIdentity {
                package: claimed,
                node: NodeKey::from_hex(node_id).unwrap(),
            })
        );
    }
    let unexported = qualified("e", "Example::Length");
    assert_eq!(
        lock.resolve_name(&name("P"), &unexported),
        Err(NameRefusal::MissingDeclaration(unexported))
    );
}

/// A projection node with digest `hex(label)` of `node_tag`/`semantic_form`
/// whose body binds `name`.
fn bound_node(label: &str, tag: &str, form: &str, name: &str) -> Value {
    let reference = json!({"digest": hex(label), "domain": "quire.checked-semantic-node/v1"});
    json!({
        "body": {
            "name": name,
            "term": "binding",
            "value": {"term": "literal", "value": true, "value_kind": "boolean"},
        },
        "dependencies": [],
        "node_id": reference,
        "node_tag": tag,
        "schema_version": "quire.checked-semantic-graph/v2",
        "semantic_form": form,
        "semantic_type": reference,
    })
}

fn ascending(mut nodes: Vec<Value>) -> Vec<Value> {
    nodes.sort_by(|left, right| {
        left["node_id"]["digest"]
            .as_str()
            .cmp(&right["node_id"]["digest"].as_str())
    });
    nodes
}

#[trace("TC-227", "FR-307-AC-1")]
#[test]
fn l09_functions_types_and_constants_export_from_their_projection_nodes() {
    let nodes = ascending(vec![
        projection_node("K::Status", Some("Status")),
        bound_node("K::Pair", "composite_type", "record", "Pair"),
        bound_node("K::Small", "bounded_domain", "integer_range", "Small"),
        bound_node("K::limit", "value", "literal", "limit"),
        bound_node("K::twice", "function", "pure_function", "twice"),
        bound_node("K::local", "expression", "let", "local"),
    ]);
    let bytes = jcs(&preimage_value(nodes.clone()));
    let exports = ["Status", "Pair", "Small", "limit", "twice"];
    let library = LibraryPackage {
        library: name("K"),
        version: "1".to_owned(),
        package_id: PackageId::of_preimage(&bytes),
        identity_preimage: bytes.clone(),
        imports: Vec::new(),
        exports: exports.iter().map(|export| (*export).to_owned()).collect(),
    };
    let root = package(
        "P",
        "1",
        "P@1",
        vec![import("K", "1", library.package_id, Some("k"))],
    );
    let lock = resolve_libraries(&root, std::slice::from_ref(&library)).unwrap();
    for export in exports {
        assert_eq!(
            lock.resolve_name(&name("P"), &qualified("k", export)),
            Ok(ExportIdentity {
                package: library.package_id,
                node: node(&format!("K::{export}")),
            }),
            "{export}"
        );
    }

    let refuse = |bytes: Box<[u8]>, export: &str, defect: PreimageDefect| {
        let supplied = LibraryPackage {
            package_id: PackageId::of_preimage(&bytes),
            identity_preimage: bytes,
            exports: vec![export.to_owned()],
            ..library.clone()
        };
        let importer = package(
            "P",
            "1",
            "P@1",
            vec![import("K", "1", supplied.package_id, Some("k"))],
        );
        assert_library_refusal(
            resolve_libraries(&importer, &[supplied]),
            &LibraryRefusal::InvalidPreimage {
                library: name("K"),
                defect,
            },
            Code::InvalidPackage,
            LibraryCause::InvalidValue,
        );
    };
    // A name no projection node declares, and a name bound only by an
    // expression node, are both undeclared exports.
    for export in ["absent", "local"] {
        refuse(
            bytes.clone(),
            export,
            PreimageDefect::UndeclaredExport(export.to_owned()),
        );
    }
    let index_of = |label: &str| {
        nodes
            .iter()
            .position(|node| node["node_id"]["digest"] == json!(hex(label)))
            .unwrap()
    };
    let malformed = |label: &str, edit: &dyn Fn(&mut Value)| {
        let mut edited = nodes.clone();
        edit(&mut edited[index_of(label)]);
        jcs(&preimage_value(edited))
    };
    let node_defect = |label: &str, defect: NodeDefect| PreimageDefect::Node {
        index: index_of(label),
        defect,
    };
    refuse(
        malformed("K::twice", &|node| {
            node["body"]["name"] = json!("not a name")
        }),
        "twice",
        node_defect("K::twice", NodeDefect::Declaration),
    );
    refuse(
        malformed("K::Status", &|node| {
            node["body"] = json!({"name": "Other", "term": "binding", "value": {"members": [], "term": "aggregate"}});
        }),
        "Status",
        node_defect("K::Status", NodeDefect::Declaration),
    );
    refuse(
        malformed("K::limit", &|node| node["node_tag"] = json!("constant")),
        "limit",
        node_defect("K::limit", NodeDefect::NodeTag),
    );
    let mut duplicate = nodes.clone();
    duplicate[index_of("K::limit")]["body"]["name"] = json!("twice");
    let later = index_of("K::limit").max(index_of("K::twice"));
    refuse(
        jcs(&preimage_value(duplicate)),
        "twice",
        PreimageDefect::DuplicateDeclaration { index: later },
    );
}
