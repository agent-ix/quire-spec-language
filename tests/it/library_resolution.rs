// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-227 reusable semantic library resolution over the real `value`
//! boundary (FR-307), following TC-227's own vectors L01-L08.
//!
//! Each fixture package's identity preimage is a structurally valid
//! `quire.checked-package-id/v2` JCS object whose `identity_projection` holds
//! one nominal node per export, and its `package_id` is the SHA-256 of those
//! bytes. Export node keys are derived from the projection nodes.

use ix_trace_rs::trace;
use quire_spec_language::diagnostic::Code;
use quire_spec_language::digest::WireNodeId;
use quire_spec_language::library::{
    check_migration, resolve_libraries, ImportDeclaration, LibraryCause, LibraryMigration,
    LibraryName, LibraryPackage, LibraryRefusal, NodeDefect, PackageId, PreimageDefect, Selection,
    StaleCause,
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

fn wire_node(label: &str) -> WireNodeId {
    WireNodeId::from_hex(&hex(label)).unwrap()
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
        node["declaration"] = json!({"qualified_name": [declaration]});
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
#[test]
fn l01_an_import_binds_the_library_package_id() {
    resolve_libraries(&over_l("P", "1", id("L@1")), &[library_l()]).unwrap();

    let raw_source = PackageId::of_preimage(b"library L version 1 { R }");
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
#[test]
fn l02_an_import_without_a_qualifier_selects_its_library() {
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
#[test]
fn l04_a_diamond_unifies_only_one_version_and_package_id() {
    let root = diamond_root();
    let supplied = [
        over_l("A", "1", id("L@1")),
        over_l("B", "1", id("L@1")),
        library_l(),
    ];
    let lock = resolve_libraries(&root, &supplied).unwrap();
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

    // A migrated library is a new identity (a different `package_id`), never
    // a relabelling of the old one: the old lock still resolves against
    // `old_library`'s own `package_id`, and the new lock's selection for `L`
    // names `new_library`'s `package_id`, not the old one.
    let old_lock = resolve_libraries(&old_package, std::slice::from_ref(&old_library)).unwrap();
    assert_eq!(
        old_lock.selections(),
        [(
            name("L"),
            Selection {
                version: "1".to_owned(),
                package_id: id("L@1"),
            }
        )]
    );
    let new_lock = resolve_libraries(&new_package, &[old_library, new_library]).unwrap();
    assert_eq!(
        new_lock.selections(),
        [(
            name("L"),
            Selection {
                version: "2".to_owned(),
                package_id: id("L'@2"),
            }
        )]
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
    let repeated = preimage_value(vec![node_r.clone(), node_r.clone()]);
    let mut spaced = jcs(&valid).into_vec();
    spaced.insert(1, b' ');

    let mut unknown_member = valid.clone();
    unknown_member["surplus"] = json!(0);
    let mut wrong_edition_type = valid.clone();
    wrong_edition_type["edition"] = json!([]);
    let mut wrong_selection_type = valid.clone();
    wrong_selection_type["model_selections"] = json!({});
    let mut unknown_node_member = valid.clone();
    unknown_node_member["identity_projection"][0]["surplus"] = json!(0);
    let mut wrong_schema_version = valid.clone();
    wrong_schema_version["identity_projection"][0]["schema_version"] =
        json!("quire.checked-semantic-graph/v1");
    let mut wrong_declaration = valid.clone();
    wrong_declaration["identity_projection"][0]["nominal_identity_preimage"]
        ["qualified_declaration"] = json!([]);
    let not_an_object = jcs(&json!([]));
    let not_json: Box<[u8]> = Box::from(&b"{"[..]);
    // Two distinct node ids declaring one name `R`, in ascending id order.
    let mut second_r = projection_node("L@1::S", Some("R"));
    second_r["nominal_identity_preimage"]["qualified_declaration"] = json!(["R"]);
    let (first_declaration, second_declaration) = if hex("L@1::R") < hex("L@1::S") {
        (node_r.clone(), second_r)
    } else {
        (second_r, node_r.clone())
    };
    let ambiguous_declaration = preimage_value(vec![first_declaration, second_declaration]);
    let scalar_node = preimage_value(vec![json!(0)]);

    // The top-level `declaration` disagrees with the nominal
    // `qualified_declaration` (`declaration-nominal-mismatch`).
    let mut mismatched_declaration = valid.clone();
    mismatched_declaration["identity_projection"][0]["declaration"] =
        json!({"qualified_name": ["S"]});
    // `declaration` is absent although the node is nominal.
    let mut missing_declaration = valid.clone();
    missing_declaration["identity_projection"][0]
        .as_object_mut()
        .unwrap()
        .remove("declaration");
    // `declaration` is not `{qualified_name}`: wrong type.
    let mut declaration_wrong_type = valid.clone();
    declaration_wrong_type["identity_projection"][0]["declaration"] = json!(42);
    // `declaration` is not `{qualified_name}`: an extra member.
    let mut declaration_extra_member = valid.clone();
    declaration_extra_member["identity_projection"][0]["declaration"] =
        json!({"qualified_name": ["R"], "surplus": 0});

    let cases: [MalformedCase; 19] = [
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
        (not_an_object, &[], PreimageDefect::NotObject),
        (not_json, &[], PreimageDefect::NotObject),
        (
            jcs(&unknown_member),
            &["R"],
            PreimageDefect::UnknownMember("surplus".to_owned()),
        ),
        (
            jcs(&wrong_edition_type),
            &["R"],
            PreimageDefect::MemberType("edition"),
        ),
        (
            jcs(&wrong_selection_type),
            &["R"],
            PreimageDefect::MemberType("model_selections"),
        ),
        (
            jcs(&unknown_node_member),
            &["R"],
            PreimageDefect::Node {
                index: 0,
                defect: NodeDefect::UnknownMember("surplus".to_owned()),
            },
        ),
        (
            jcs(&scalar_node),
            &[],
            PreimageDefect::Node {
                index: 0,
                defect: NodeDefect::NotObject,
            },
        ),
        (
            jcs(&wrong_schema_version),
            &["R"],
            PreimageDefect::Node {
                index: 0,
                defect: NodeDefect::SchemaVersion,
            },
        ),
        (
            jcs(&wrong_declaration),
            &[],
            PreimageDefect::Node {
                index: 0,
                defect: NodeDefect::Declaration,
            },
        ),
        (
            jcs(&declaration_wrong_type),
            &[],
            PreimageDefect::Node {
                index: 0,
                defect: NodeDefect::Declaration,
            },
        ),
        (
            jcs(&declaration_extra_member),
            &[],
            PreimageDefect::Node {
                index: 0,
                defect: NodeDefect::Declaration,
            },
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

    // Two distinct node ids declaring one name `R`: `ambiguous-name`, a
    // different `Code`/`LibraryCause` family than the loop above, so it is
    // asserted separately with both node keys in ascending digest order.
    let ambiguous = with_preimage(jcs(&ambiguous_declaration), &[]);
    let (first_node, second_node) = if hex("L@1::R") < hex("L@1::S") {
        (wire_node("L@1::R"), wire_node("L@1::S"))
    } else {
        (wire_node("L@1::S"), wire_node("L@1::R"))
    };
    let expected_ambiguous = LibraryRefusal::InvalidPreimage {
        library: name("L"),
        defect: PreimageDefect::AmbiguousDeclaration {
            name: "R".to_owned(),
            nodes: [first_node, second_node],
        },
    };
    assert_eq!(expected_ambiguous.member_path(), Some("/identity_preimage"));
    assert_library_refusal(
        resolve_libraries(&importer, std::slice::from_ref(&ambiguous)),
        &expected_ambiguous,
        Code::AmbiguousDeclaration,
        LibraryCause::AmbiguousName,
    );
    assert_library_refusal(
        check_migration(&library_l(), &ambiguous),
        &expected_ambiguous,
        Code::AmbiguousDeclaration,
        LibraryCause::AmbiguousName,
    );

    // A node's top-level `declaration.qualified_name` disagrees with its
    // nominal `qualified_declaration` (`declaration-nominal-mismatch`): a
    // different `Code`/`LibraryCause` family than the loop above, so it is
    // asserted separately with the node key and both names.
    let mismatched = with_preimage(jcs(&mismatched_declaration), &[]);
    let expected_mismatched = LibraryRefusal::InvalidPreimage {
        library: name("L"),
        defect: PreimageDefect::DeclarationNominalMismatch {
            node: wire_node("L@1::R"),
            declared: Some("S".to_owned()),
            nominal: "R".to_owned(),
        },
    };
    assert_eq!(
        expected_mismatched.member_path(),
        Some("/identity_preimage")
    );
    assert_library_refusal(
        resolve_libraries(&importer, std::slice::from_ref(&mismatched)),
        &expected_mismatched,
        Code::InvalidPackage,
        LibraryCause::DeclarationNominalMismatch,
    );
    assert_library_refusal(
        check_migration(&library_l(), &mismatched),
        &expected_mismatched,
        Code::InvalidPackage,
        LibraryCause::DeclarationNominalMismatch,
    );

    // `declaration` is absent although the node is nominal: also
    // `declaration-nominal-mismatch`, with no declared name to retain.
    let missing = with_preimage(jcs(&missing_declaration), &[]);
    let expected_missing = LibraryRefusal::InvalidPreimage {
        library: name("L"),
        defect: PreimageDefect::DeclarationNominalMismatch {
            node: wire_node("L@1::R"),
            declared: None,
            nominal: "R".to_owned(),
        },
    };
    assert_eq!(expected_missing.member_path(), Some("/identity_preimage"));
    assert_library_refusal(
        resolve_libraries(&importer, std::slice::from_ref(&missing)),
        &expected_missing,
        Code::InvalidPackage,
        LibraryCause::DeclarationNominalMismatch,
    );
    assert_library_refusal(
        check_migration(&library_l(), &missing),
        &expected_missing,
        Code::InvalidPackage,
        LibraryCause::DeclarationNominalMismatch,
    );

    assert_eq!(
        with_preimage(jcs(&valid), &["R"]),
        library_l(),
        "the valid fixture is the supplied L@1"
    );
}

#[trace("TC-227", "FR-307-AC-5")]
#[test]
fn l08_b_schema_refusals_rank_before_a_mismatch_at_an_earlier_node() {
    // Four nodes; the lower-indexed node (1) has a declaration-nominal
    // mismatch and the higher-indexed node (3) has an unknown member. Schema
    // refusals (pass 1) run to completion across all nodes before mismatch
    // checks (pass 2) begin, so the unknown member is reported, never the
    // earlier-indexed mismatch.
    let mut labeled: Vec<(String, Value)> = ["W", "X", "Y", "Z"]
        .iter()
        .map(|export| {
            let node_label = format!("L@1::{export}");
            (hex(&node_label), projection_node(&node_label, Some("R")))
        })
        .collect();
    labeled.sort_by(|left, right| left.0.cmp(&right.0));
    labeled[1].1["declaration"] = json!({"qualified_name": ["M"]});
    labeled[3].1["surplus"] = json!(0);
    let nodes: Vec<Value> = labeled.into_iter().map(|(_, node)| node).collect();
    let malformed = with_preimage(jcs(&preimage_value(nodes)), &[]);
    let importer = over_l("P", "1", id("L@1"));
    let expected = LibraryRefusal::InvalidPreimage {
        library: name("L"),
        defect: PreimageDefect::Node {
            index: 3,
            defect: NodeDefect::UnknownMember("surplus".to_owned()),
        },
    };
    assert_library_refusal(
        resolve_libraries(&importer, std::slice::from_ref(&malformed)),
        &expected,
        Code::InvalidPackage,
        LibraryCause::InvalidValue,
    );
}

#[trace("TC-227", "FR-307-AC-5")]
#[test]
fn l08_c_a_mismatch_ranks_before_an_ambiguity_at_an_earlier_node() {
    // Three nodes; the two lower-indexed nodes (0, 1) both declare "R" (an
    // ambiguity) and the higher-indexed node (2) has a declaration-nominal
    // mismatch. Mismatch checks (pass 2) run to completion across all nodes
    // before ambiguity checks (pass 3) begin, so the mismatch is reported,
    // never the earlier-indexed ambiguity.
    let mut labeled: Vec<(String, Value)> = ["W", "X", "Y"]
        .iter()
        .map(|export| {
            let node_label = format!("L@1::{export}");
            (hex(&node_label), projection_node(&node_label, Some("R")))
        })
        .collect();
    labeled.sort_by(|left, right| left.0.cmp(&right.0));
    let mismatch_key = WireNodeId::from_hex(&labeled[2].0).unwrap();
    labeled[2].1["declaration"] = json!({"qualified_name": ["T"]});
    let nodes: Vec<Value> = labeled.into_iter().map(|(_, node)| node).collect();
    let malformed = with_preimage(jcs(&preimage_value(nodes)), &[]);
    let importer = over_l("P", "1", id("L@1"));
    let expected = LibraryRefusal::InvalidPreimage {
        library: name("L"),
        defect: PreimageDefect::DeclarationNominalMismatch {
            node: mismatch_key,
            declared: Some("T".to_owned()),
            nominal: "R".to_owned(),
        },
    };
    assert_library_refusal(
        resolve_libraries(&importer, std::slice::from_ref(&malformed)),
        &expected,
        Code::InvalidPackage,
        LibraryCause::DeclarationNominalMismatch,
    );
}

#[trace("TC-227", "FR-307-AC-1")]
#[test]
fn l01_every_declared_export_must_derive_from_a_checked_package_v2_identity_projection() {
    // A self-built two-export identity projection (this test's own fixture,
    // not read from any external file). `declaration`/`nominal_identity_preimage`
    // need each name segment separately (schema `QualifiedName` = one or
    // more bare identifiers, `is_qualified_name`), so this builds the node
    // directly rather than through `projection_node`, which only ever takes
    // one segment.
    //
    // FR-087 (#213 S-3a) removed `resolve_name`/`ExportIdentity` from
    // `library` (name resolution is E3's own, per ADR-013 R-06), so this
    // test's remaining externally-observable claim is `resolve_libraries`'
    // own admission: it succeeds when every declared export is spelled by a
    // nominal `qualified_declaration` in the projection, and refuses
    // `UndeclaredExport` when one is not -- `library` no longer exposes a
    // per-name lookup to prove which node id a name derives.
    fn two_segment_node(node_label: &str, segments: [&str; 2]) -> Value {
        let reference =
            json!({"digest": hex(node_label), "domain": "quire.checked-semantic-node/v1"});
        json!({
            "body": {"members": [], "term": "aggregate"},
            "dependencies": [],
            "node_id": reference,
            "node_tag": "scalar_type",
            "schema_version": "quire.checked-semantic-graph/v2",
            "semantic_form": "enum",
            "semantic_type": reference,
            "nominal_identity_preimage": {
                "members": ["READY"],
                "ordered": true,
                "owner": {"authority": "agent-ix", "identity": "library", "kind": "definition"},
                "qualified_declaration": segments,
                "version": "quire.enum-declaration-node/v1",
            },
            "declaration": {"qualified_name": segments},
        })
    }
    let exports = ["Example::Status", "Example::metre"];
    let segments = [["Example", "Status"], ["Example", "metre"]];
    let mut nodes: Vec<(String, Value)> = exports
        .iter()
        .zip(segments)
        .map(|(export, segments)| (hex(export), two_segment_node(export, segments)))
        .collect();
    nodes.sort_by(|left, right| left.0.cmp(&right.0));
    let preimage_bytes = jcs(&preimage_value(
        nodes.into_iter().map(|(_, node)| node).collect(),
    ));
    let claimed = PackageId::of_preimage(&preimage_bytes);
    let valid_library = LibraryPackage {
        library: name("Example"),
        version: "1".to_owned(),
        package_id: claimed,
        identity_preimage: preimage_bytes.clone(),
        imports: Vec::new(),
        exports: exports.iter().map(|export| (*export).to_owned()).collect(),
    };
    let root = package(
        "P",
        "1",
        "P@1",
        vec![import("Example", "1", claimed, Some("e"))],
    );
    resolve_libraries(&root, &[valid_library]).unwrap();

    let unexported_library = LibraryPackage {
        library: name("Example"),
        version: "1".to_owned(),
        package_id: claimed,
        identity_preimage: preimage_bytes,
        imports: Vec::new(),
        exports: vec!["Example::Length".to_owned()],
    };
    assert_library_refusal(
        resolve_libraries(&root, &[unexported_library]),
        &LibraryRefusal::UndeclaredExport {
            library: name("Example"),
            export: "Example::Length".to_owned(),
        },
        Code::MissingDeclaration,
        LibraryCause::UndeclaredExport,
    );
}

/// A projection node with digest `hex(label)` of `node_tag`/`semantic_form`
/// with no nominal identity preimage, whose body binds `name`.
fn unnamed_node(label: &str, tag: &str, form: &str, name: &str) -> Value {
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
fn l09_only_a_nominal_qualified_declaration_names_an_export() {
    let nodes = ascending(vec![
        projection_node("K::Status", Some("Status")),
        unnamed_node("K::Pair", "composite_type", "record", "Pair"),
        unnamed_node("K::limit", "value", "literal", "limit"),
        unnamed_node("K::twice", "function", "pure_function", "twice"),
    ]);
    let bytes = jcs(&preimage_value(nodes.clone()));
    let supply = |bytes: Box<[u8]>, export: &str| {
        let library = LibraryPackage {
            library: name("K"),
            version: "1".to_owned(),
            package_id: PackageId::of_preimage(&bytes),
            identity_preimage: bytes,
            imports: Vec::new(),
            exports: vec![export.to_owned()],
        };
        let root = package(
            "P",
            "1",
            "P@1",
            vec![import("K", "1", library.package_id, Some("k"))],
        );
        (root, library)
    };

    let (root, library) = supply(bytes.clone(), "Status");
    // `Status` is spelled by a nominal `qualified_declaration`, so
    // `resolve_libraries` admits it (FR-087 removed the `resolve_name` path
    // that used to prove the specific derived node id here; the remaining
    // externally-observable claim is admission itself).
    resolve_libraries(&root, std::slice::from_ref(&library)).unwrap();

    // A type, constant or function node without a nominal
    // `qualified_declaration` has no explicit name, and neither has a name
    // no node carries: none is guessed from a binding body.
    for export in ["Pair", "limit", "twice", "absent"] {
        let (root, library) = supply(bytes.clone(), export);
        let expected = LibraryRefusal::UndeclaredExport {
            library: name("K"),
            export: export.to_owned(),
        };
        assert_eq!(expected.member_path(), None);
        assert_library_refusal(
            resolve_libraries(&root, &[library]),
            &expected,
            Code::MissingDeclaration,
            LibraryCause::UndeclaredExport,
        );
        assert_eq!(LibraryCause::UndeclaredExport.as_str(), "undeclared-export");
    }

    let index = nodes
        .iter()
        .position(|node| node["node_id"]["digest"] == json!(hex("K::limit")))
        .unwrap();
    let mut unknown_tag = nodes.clone();
    unknown_tag[index]["node_tag"] = json!("constant");
    let (root, library) = supply(jcs(&preimage_value(unknown_tag)), "Status");
    assert_library_refusal(
        resolve_libraries(&root, &[library]),
        &LibraryRefusal::InvalidPreimage {
            library: name("K"),
            defect: PreimageDefect::Node {
                index,
                defect: NodeDefect::NodeTag,
            },
        },
        Code::InvalidPackage,
        LibraryCause::InvalidValue,
    );
}

// TC-282 (FR-087-AC-12): every `LibraryRefusal` variant `resolve_libraries`
// can return classifies to exactly one of an ADR-011 I2 graph rule, the §4
// binding's condition 2 or 3, or E3 name resolution -- with
// `DuplicatePackageId` the one named exception. One fixture per row of the
// classification table (Description, item 3, owner ruling (e)); ten rows,
// `StaleDependency`'s two causes counted separately.

#[trace("TC-282", "FR-087-AC-12")]
#[test]
fn package_id_mismatch_classifies_to_4_condition_2() {
    let mut reused = package("L", "1", "L'@1", Vec::new());
    reused.package_id = id("L@1");
    assert_library_refusal(
        resolve_libraries(&over_l("P", "1", id("L@1")), std::slice::from_ref(&reused)),
        &LibraryRefusal::PackageIdMismatch {
            library: name("L"),
            claimed: id("L@1"),
            recomputed: id("L'@1"),
        },
        Code::InvalidPackage,
        LibraryCause::InvalidValue,
    );
}

#[trace("TC-282", "FR-087-AC-12")]
#[test]
fn invalid_preimage_classifies_alongside_4_condition_2() {
    let mut wrong_version = preimage_value(projection("L@1"));
    wrong_version["version"] = json!("quire.checked-package-id/v1");
    let malformed = with_preimage(jcs(&wrong_version), &["R"]);
    assert_library_refusal(
        resolve_libraries(
            &over_l("P", "1", malformed.package_id),
            std::slice::from_ref(&malformed),
        ),
        &LibraryRefusal::InvalidPreimage {
            library: name("L"),
            defect: PreimageDefect::Version,
        },
        Code::InvalidPackage,
        LibraryCause::InvalidValue,
    );
}

#[trace("TC-282", "FR-087-AC-12")]
#[test]
fn undeclared_export_classifies_alongside_4_condition_2() {
    let malformed = with_preimage(preimage("L@1"), &["Missing"]);
    assert_library_refusal(
        resolve_libraries(
            &over_l("P", "1", malformed.package_id),
            std::slice::from_ref(&malformed),
        ),
        &LibraryRefusal::UndeclaredExport {
            library: name("L"),
            export: "Missing".to_owned(),
        },
        Code::MissingDeclaration,
        LibraryCause::UndeclaredExport,
    );
}

#[trace("TC-282", "FR-087-AC-12")]
#[test]
fn invalid_qualifier_classifies_to_e3_name_resolution() {
    let root = package("P", "1", "P@1", vec![import("L", "1", id("L@1"), Some("1l"))]);
    assert_library_refusal(
        resolve_libraries(&root, &[library_l()]),
        &LibraryRefusal::InvalidQualifier {
            path: path(&["P", "L"]),
        },
        Code::InvalidPackage,
        LibraryCause::InvalidValue,
    );
}

#[trace("TC-282", "FR-087-AC-12")]
#[test]
fn conflicting_definition_classifies_to_i2_rule_2() {
    let root = diamond_root();
    let supplied = [
        over_l("A", "1", id("L@1")),
        over_l("B", "2", id("L@2")),
        library_l(),
        package("L", "2", "L@2", Vec::new()),
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

#[trace("TC-282", "FR-087-AC-12")]
#[test]
fn import_cycle_classifies_to_i2_rule_3() {
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

#[trace("TC-282", "FR-087-AC-12")]
#[test]
fn stale_dependency_revision_mismatch_classifies_to_4_condition_3() {
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

#[trace("TC-282", "FR-087-AC-12")]
#[test]
fn stale_dependency_byte_digest_mismatch_classifies_to_i2_rule_1() {
    let raw_source = PackageId::of_preimage(b"library L version 1 { R }");
    assert_library_refusal(
        resolve_libraries(&over_l("P", "1", raw_source), &[library_l()]),
        &LibraryRefusal::StaleDependency {
            path: path(&["P", "L"]),
            import: import("L", "1", raw_source, Some("l")),
            cause: StaleCause::ByteDigestMismatch,
        },
        Code::StaleDependency,
        LibraryCause::ByteDigestMismatch,
    );
}

#[trace("TC-282", "FR-087-AC-12")]
#[test]
fn missing_import_classifies_to_i2_rule_1() {
    let root = package("P", "1", "P@1", vec![import("Z", "1", id("Z@1"), Some("z"))]);
    assert_library_refusal(
        resolve_libraries(&root, &[]),
        &LibraryRefusal::MissingImport {
            path: path(&["P", "Z"]),
        },
        Code::MissingImport,
        LibraryCause::MissingSelection,
    );
}

#[trace("TC-282", "FR-087-AC-12")]
#[test]
fn duplicate_package_id_is_the_named_exception_outside_all_four() {
    // Two packages sharing one byte-identical `identity_preimage` (hence one
    // recomputed `package_id`, each independently passing its own digest
    // check), differing only in `version` -- a field the preimage excludes.
    let root = package("P", "1", "P@1", Vec::new());
    let first = package("L", "1", "L@1", Vec::new());
    let second = package("L", "2", "L@1", Vec::new());
    assert_library_refusal(
        resolve_libraries(&root, &[first, second]),
        &LibraryRefusal::DuplicatePackageId(id("L@1")),
        Code::InvalidPackage,
        LibraryCause::InvalidValue,
    );
}
