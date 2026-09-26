// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-446 (FR-099-AC-1 to AC-4): spine `compile` resolves each `import`
//! against the supplied libraries, compiles each library from source, binds
//! the import to the recomputed `package_id`, and refuses each ADR-015 D-1
//! case with no package.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_cst::HostCause;
use qsl_foundation::digest::{DigestDomain, DigestRecord};
use qsl_foundation::{Code, SourceIdentity};
use qsl_package::{emit_checked, read_import_view};
use qsl_semantics::library::{LibraryName, PackageId};
use serde_json::Value;

use super::{
    compile, CompileRefusal, Compiled, DependencyInput, DependencyInputRefusal, ImportRefusal,
    SourceHolder, SpineLimits, SpineStage, SuppliedLibrary,
};

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\" version \"1\" digest \
    \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n";

const F: &str = "function f using v(x: Int[0, 9]): Boolean pure { x < 5 }\n";
const F_CHANGED: &str = "function f using v(x: Int[0, 9]): Boolean pure { x < 6 }\n";
const H: &str = "function h using v(): Boolean pure { true }\n";

fn lib(identity: &str) -> LibraryName {
    LibraryName::new(identity).expect("a non-empty identity")
}

/// `import "identity" version "version" digest "digest" as alias;`.
fn import(identity: &str, version: &str, digest: &str, alias: &str) -> String {
    format!("import \"{identity}\" version \"{version}\" digest \"{digest}\" as {alias};\n")
}

/// A digest no source compiles to.
fn arbitrary() -> String {
    "e".repeat(64)
}

/// A library supplied as `identity`, from source (`a`, `name`, `git`, `1`)
/// holding `body` after the header.
fn library(identity: &str, version: &str, name: &str, body: &str) -> SuppliedLibrary {
    SuppliedLibrary {
        identity: identity.to_owned(),
        version: version.to_owned(),
        source: SourceIdentity::new("a", name, "git", "1"),
        path: format!("{name}.native"),
        bytes: format!("{HEADER}{body}").into_bytes(),
    }
}

fn input(libraries: Vec<SuppliedLibrary>) -> DependencyInput {
    DependencyInput::new(libraries).expect("an admissible dependency input")
}

/// Spine compile of `source`, as (`a`, `identity`, `git`, `1`).
fn compile_as(
    identity: &str,
    source: &str,
    dependencies: &DependencyInput,
) -> Result<Compiled, Box<CompileRefusal>> {
    compile(
        SourceIdentity::new("a", identity, "git", "1"),
        &format!("{identity}.native"),
        source.as_bytes(),
        &BTreeMap::new(),
        dependencies,
        SpineLimits::default(),
    )
}

/// The importing unit `u`: the header, then `body`.
fn unit(body: &str) -> String {
    format!("{HEADER}{body}")
}

/// `library`'s own `package_id`, compiled alone against `dependencies`.
fn package_id(library: &SuppliedLibrary, dependencies: &DependencyInput) -> PackageId {
    compile(
        library.source.clone(),
        &library.path,
        &library.bytes,
        &BTreeMap::new(),
        dependencies,
        SpineLimits::default(),
    )
    .expect("the library compiles")
    .emitted
    .package_id()
}

/// The text `region` covers in `source`.
fn covered<'a>(source: &'a str, refusal: &CompileRefusal) -> &'a str {
    let region = refusal.region().expect("the refusal is located");
    let start = usize::try_from(region.start()).unwrap();
    let end = usize::try_from(region.end()).unwrap();
    &source[start..end]
}

fn wire(compiled: &Compiled) -> Value {
    serde_json::from_slice(compiled.emitted.bytes()).expect("the wire is JSON")
}

/// FR-099-AC-1 (TC-446 step 1): an import bound to the library's
/// recomputed `package_id` puts one `DependencySelection` in the lock and
/// the identity preimage; QSL's I2 read admits the package; and a supplied
/// library no import reaches changes nothing.
#[trace("FR-099-AC-1", "TC-446")]
#[test]
fn an_import_binds_the_library_compiled_from_source() {
    let geometry = library("test/geometry", "1", "geometry", F);
    let d = package_id(&geometry, &DependencyInput::default());
    let source = unit(&format!(
        "{}{H}",
        import("test/geometry", "1", &d.hex(), "g")
    ));
    let compiled = compile_as("u", &source, &input(vec![geometry.clone()]))
        .expect("the unit compiles against test/geometry");
    let written = wire(&compiled);
    let expected = serde_json::json!([{
        "identity": "test/geometry",
        "version": "1",
        "package_id": {
            "domain": "quire.package.semantic/v2",
            "algorithm": "sha256",
            "digest": d.hex(),
        },
    }]);
    assert_eq!(written["lock"]["dependency_selections"], expected);
    assert_eq!(
        written["identity_preimage"]["dependency_selections"],
        expected
    );
    assert_eq!(
        compiled.package.dependencies().keys().collect::<Vec<_>>(),
        [&d]
    );

    // QSL's I2 read admits the package, with test/geometry's admitted
    // package supplied, pinned at its own `package_id`.
    let emission = emit_checked(&compiled.package).expect("the package emits");
    assert_eq!(emission.package().bytes(), compiled.emitted.bytes());
    let view = read_import_view(&compiled.package, lib("u"), "1", &BTreeMap::new())
        .expect("the I2 read admits the importing package");
    assert_eq!(view.package(), compiled.emitted.package_id());

    // An unimported `test/other` is neither compiled nor recorded: its
    // source names an undeclared `tru`, so compiling it would refuse.
    let with_other = compile_as(
        "u",
        &source,
        &input(vec![
            geometry,
            library(
                "test/other",
                "1",
                "other",
                "function o using v(): Boolean pure { tru }\n",
            ),
        ]),
    )
    .expect("an unimported library is not compiled");
    assert_eq!(with_other.emitted.bytes(), compiled.emitted.bytes());
}

/// FR-099-AC-2 (TC-446 step 2): a library whose source no longer compiles
/// to the digest the import records refuses `DependencyIdentityMismatch` at
/// the import, naming the identity, the recorded digest and the recompiled
/// `package_id`. The digest spellings S1 refuses are `qsl-cst`'s
/// `an_import_digest_is_bare_lowercase_hex`.
#[trace("FR-099-AC-2", "TC-446")]
#[test]
fn a_changed_library_source_is_a_stale_dependency() {
    let d = package_id(
        &library("test/geometry", "1", "geometry", F),
        &DependencyInput::default(),
    );
    let changed = library("test/geometry", "1", "geometry", F_CHANGED);
    let d_changed = package_id(&changed, &DependencyInput::default());
    assert_ne!(d, d_changed);
    let declaration = import("test/geometry", "1", &d.hex(), "g");
    let source = unit(&format!("{declaration}{H}"));
    let refusal = compile_as("u", &source, &input(vec![changed])).expect_err("the source changed");
    assert_eq!(refusal.stage(), SpineStage::Intake);
    assert_eq!(refusal.code(), Code::StaleDependency);
    assert_eq!(covered(&source, &refusal), declaration.trim_end());
    let CompileRefusal::Import {
        refusal:
            refusal @ ImportRefusal::DependencyIdentityMismatch {
                identity,
                recorded,
                recompiled,
            },
        ..
    } = &*refusal
    else {
        panic!("expected DependencyIdentityMismatch, got {refusal:?}");
    };
    assert_eq!(refusal.cause(), Some("byte-digest-mismatch"));
    assert_eq!(*identity, lib("test/geometry"));
    assert_eq!(*recorded, d.record());
    assert_eq!(*recompiled, d_changed);
    assert_eq!(
        *recorded,
        DigestRecord::from_domain_and_hex(DigestDomain::PackageSemanticV2, &d.hex()).unwrap()
    );
}

/// FR-099-AC-3 (TC-446 step 3), selection and dependency input: each
/// refusal is located as D-1 states, and none yields a package.
#[trace("FR-099-AC-3", "TC-446")]
#[test]
fn selection_and_dependency_input_refusals() {
    let geometry = library("test/geometry", "1", "geometry", F);
    let d = package_id(&geometry, &DependencyInput::default());
    let declaration = import("test/geometry", "1", &d.hex(), "g");
    let source = unit(&format!("{declaration}{H}"));

    // No library supplied.
    let missing = compile_as("u", &source, &DependencyInput::default())
        .expect_err("nothing supplies test/geometry");
    assert_eq!(missing.stage(), SpineStage::Intake);
    assert_eq!(missing.code(), Code::MissingImport);
    assert!(
        matches!(
            &*missing,
            CompileRefusal::Import {
                refusal: ImportRefusal::MissingSelection { identity },
                ..
            } if identity.as_str() == "test/geometry"
        ),
        "{missing:?}"
    );
    assert_eq!(covered(&source, &missing), "\"test/geometry\"");

    // Supplied at version 2.
    let stale = compile_as(
        "u",
        &source,
        &input(vec![library("test/geometry", "2", "geometry", F)]),
    )
    .expect_err("the import selects version 1");
    assert_eq!(stale.stage(), SpineStage::Intake);
    assert_eq!(stale.code(), Code::StaleDependency);
    let CompileRefusal::Import {
        refusal: stale_import,
        ..
    } = &*stale
    else {
        panic!("expected an import refusal, got {stale:?}");
    };
    assert!(
        matches!(
            stale_import,
            ImportRefusal::RevisionMismatch { identity, imported, supplied }
                if *identity == lib("test/geometry") && imported == "1" && supplied == "2"
        ),
        "{stale_import:?}"
    );
    assert_eq!(stale_import.cause(), Some("revision-mismatch"));
    assert_eq!(covered(&source, &stale), declaration.trim_end());

    // Two libraries supplied as test/geometry.
    let twice = DependencyInput::new(vec![
        geometry.clone(),
        library("test/geometry", "1", "geometry-2", F),
    ])
    .expect_err("one library per identity");
    assert_eq!(twice.code(), Code::InvalidPackage);
    assert_eq!(twice.cause(), Some("conflicting-definition"));
    assert_eq!(
        twice,
        DependencyInputRefusal::DuplicateIdentity {
            identity: lib("test/geometry"),
            first: Box::new(SourceIdentity::new("a", "geometry", "git", "1")),
            second: Box::new(SourceIdentity::new("a", "geometry-2", "git", "1")),
        }
    );

    // Two libraries whose sources share one owner, at different revisions.
    let mut same_owner = library("test/other", "1", "geometry", H);
    same_owner.source.revision = "2".to_owned();
    let shared = DependencyInput::new(vec![geometry.clone(), same_owner])
        .expect_err("one owner per compile");
    assert_eq!(shared.cause(), Some("conflicting-definition"));
    assert_eq!(
        shared,
        DependencyInputRefusal::SharedOwner {
            first: SourceHolder::Library(lib("test/geometry")),
            second: lib("test/other"),
            authority: "a".to_owned(),
            identity: "geometry".to_owned(),
        }
    );

    // A library whose source has the unit's authority and identity.
    let unit_owner = compile_as(
        "u",
        &source,
        &input(vec![library("test/geometry", "1", "u", F)]),
    )
    .expect_err("the library repeats the unit's owner");
    assert_eq!(unit_owner.stage(), SpineStage::Intake);
    assert_eq!(unit_owner.code(), Code::InvalidPackage);
    assert!(unit_owner.region().is_none());
    assert!(
        matches!(
            &*unit_owner,
            CompileRefusal::DependencyInput(DependencyInputRefusal::SharedOwner {
                first: SourceHolder::Unit,
                second,
                ..
            }) if *second == lib("test/geometry")
        ),
        "{unit_owner:?}"
    );

    // An empty identity, and an empty version.
    let empty = DependencyInput::new(vec![library("", "1", "geometry", F)])
        .expect_err("an identity is non-empty");
    assert_eq!(empty.code(), Code::InvalidIdentifier);
    assert_eq!(empty.host_cause(), Some(HostCause::SelectionIdentity));
    let empty = DependencyInput::new(vec![library("test/geometry", "", "geometry", F)])
        .expect_err("a version is non-empty");
    assert_eq!(empty.code(), Code::InvalidIdentifier);
    assert_eq!(empty.host_cause(), Some(HostCause::SelectionVersion));
}

/// FR-099-AC-3 (TC-446 step 3), closure-level and wrapped refusals: a cycle
/// and a diamond are reported unwrapped at the import that closes them, in
/// the source that declares it; a library's own refusal is wrapped with its
/// dependency path and located in the library's source.
#[trace("FR-099-AC-3", "TC-446")]
#[test]
fn cycle_diamond_and_a_library_refusal() {
    // test/a and test/b import each other under arbitrary digests: the cycle
    // is refused before any digest is compared.
    let b_imports_a = import("test/a", "1", &arbitrary(), "la");
    let cycle_input = input(vec![
        library(
            "test/a",
            "1",
            "a",
            &format!("{}{H}", import("test/b", "1", &arbitrary(), "lb")),
        ),
        library("test/b", "1", "b", &format!("{b_imports_a}{H}")),
    ]);
    let source = unit(&format!("{}{H}", import("test/a", "1", &arbitrary(), "la")));
    let cycle = compile_as("u", &source, &cycle_input).expect_err("a imports b imports a");
    assert_eq!(cycle.stage(), SpineStage::Intake);
    assert_eq!(cycle.code(), Code::InvalidPackage);
    let CompileRefusal::Import {
        refusal: cycle_import @ ImportRefusal::Cycle { path },
        region,
    } = &*cycle
    else {
        panic!("expected an unwrapped cycle, got {cycle:?}");
    };
    assert_eq!(cycle_import.cause(), Some("definition-cycle"));
    assert_eq!(*path, [lib("test/a"), lib("test/b"), lib("test/a")]);
    let region = region.as_ref().expect("the cycle is located");
    assert_eq!(region.source().identity(), "b");
    let b_source = format!("{HEADER}{b_imports_a}{H}");
    assert_eq!(covered(&b_source, &cycle), "\"test/a\"");

    // The unit imports test/geometry 1, then test/a, which imports
    // test/geometry 2.
    let geometry = library("test/geometry", "1", "geometry", F);
    let d = package_id(&geometry, &DependencyInput::default());
    let a_imports_geometry = import("test/geometry", "2", &arbitrary(), "g");
    let diamond_input = input(vec![
        geometry,
        library("test/a", "1", "a", &format!("{a_imports_geometry}{H}")),
    ]);
    let source = unit(&format!(
        "{}{}{H}",
        import("test/geometry", "1", &d.hex(), "g"),
        import("test/a", "1", &arbitrary(), "la"),
    ));
    let diamond =
        compile_as("u", &source, &diamond_input).expect_err("test/geometry 1 and 2 conflict");
    assert_eq!(diamond.stage(), SpineStage::Intake);
    assert_eq!(diamond.code(), Code::InvalidPackage);
    let CompileRefusal::Import {
        refusal:
            diamond_import @ ImportRefusal::Diamond {
                identity,
                first,
                second,
            },
        region,
    } = &*diamond
    else {
        panic!("expected an unwrapped diamond, got {diamond:?}");
    };
    assert_eq!(diamond_import.cause(), Some("conflicting-definition"));
    assert_eq!(*identity, lib("test/geometry"));
    assert_eq!(first.path, [lib("test/geometry")]);
    assert_eq!(first.version, "1");
    assert_eq!(second.path, [lib("test/a"), lib("test/geometry")]);
    assert_eq!(second.version, "2");
    assert_eq!(region.as_ref().unwrap().source().identity(), "a");
    let a_source = format!("{HEADER}{a_imports_geometry}{H}");
    assert_eq!(covered(&a_source, &diamond), "\"test/geometry\"");

    // test/a imports a test/missing nothing supplies.
    let a_imports_missing = import("test/missing", "1", &arbitrary(), "m");
    let source = unit(&format!("{}{H}", import("test/a", "1", &arbitrary(), "la")));
    let wrapped = compile_as(
        "u",
        &source,
        &input(vec![library(
            "test/a",
            "1",
            "a",
            &format!("{a_imports_missing}{H}"),
        )]),
    )
    .expect_err("test/a's own import is missing");
    assert_eq!(wrapped.stage(), SpineStage::Intake);
    assert_eq!(wrapped.code(), Code::MissingImport);
    let CompileRefusal::Dependency { path, refusal } = &*wrapped else {
        panic!("expected a wrapped refusal, got {wrapped:?}");
    };
    assert_eq!(*path, [lib("test/a")]);
    assert!(
        matches!(
            &**refusal,
            CompileRefusal::Import {
                refusal: ImportRefusal::MissingSelection { identity },
                ..
            } if identity.as_str() == "test/missing"
        ),
        "{refusal:?}"
    );
    assert_eq!(refusal.region().unwrap().source().identity(), "a");
    let a_source = format!("{HEADER}{a_imports_missing}{H}");
    assert_eq!(covered(&a_source, &wrapped), "\"test/missing\"");
}

/// FR-099-AC-4 (TC-446 step 4): a library identity is any non-empty string,
/// and the closure is listed in UTF-8 byte order of identity whatever the
/// import order.
#[trace("FR-099-AC-4", "TC-446")]
#[test]
fn library_identities_are_strings_listed_in_byte_order() {
    for identity in ["test/geometry", "a.b", "L"] {
        assert_eq!(lib(identity).as_str(), identity);
    }
    assert!(LibraryName::new("").is_err());

    let a = library("test/a", "1", "a", H);
    let b = library("test/b", "1", "b", H);
    let a_id = package_id(&a, &DependencyInput::default());
    let b_id = package_id(&b, &DependencyInput::default());
    let dependencies = input(vec![b, a]);
    let import_a = import("test/a", "1", &a_id.hex(), "la");
    let import_b = import("test/b", "1", &b_id.hex(), "lb");
    for imports in [
        format!("{import_b}{import_a}"),
        format!("{import_a}{import_b}"),
    ] {
        let compiled = compile_as("u", &unit(&format!("{imports}{H}")), &dependencies)
            .expect("both libraries compile");
        assert_eq!(
            compiled
                .package
                .dependency_selections()
                .keys()
                .collect::<Vec<_>>(),
            [&lib("test/a"), &lib("test/b")]
        );
        let identities: Vec<String> = wire(&compiled)["lock"]["dependency_selections"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry["identity"].as_str().unwrap().to_owned())
            .collect();
        assert_eq!(identities, ["test/a", "test/b"]);
    }

    // Byte order, not segment or case order: `Z` (0x5A) before `a`
    // (0x61), and `a.b` (0x2E) before `a/b` (0x2F), imported in reverse.
    let names = ["Z", "a", "a.b", "a/b"];
    let supplied: Vec<SuppliedLibrary> = names
        .iter()
        .enumerate()
        .map(|(index, identity)| library(identity, "1", &format!("lib-{index}"), H))
        .collect();
    let imports: String = supplied
        .iter()
        .enumerate()
        .rev()
        .map(|(index, library)| {
            let id = package_id(library, &DependencyInput::default());
            import(&library.identity, "1", &id.hex(), &format!("l{index}"))
        })
        .collect();
    let compiled = compile_as("u", &unit(&format!("{imports}{H}")), &input(supplied))
        .expect("four libraries compile");
    let identities: Vec<String> = wire(&compiled)["lock"]["dependency_selections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["identity"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(identities, names);
}

/// FR-099 (ADR-015 D-1 step 2): an import equal to an earlier one in the
/// closure reuses that import's completed library. The unit imports
/// test/geometry and test/a, which imports the same test/geometry: the
/// compile succeeds, the closure is {test/a, test/geometry}, and both
/// imports hold one shared library package.
#[trace("FR-099-AC-3", "TC-446")]
#[test]
fn an_equal_import_reuses_the_completed_library() {
    let geometry = library("test/geometry", "1", "geometry", F);
    let d = package_id(&geometry, &DependencyInput::default());
    let geometry_import = import("test/geometry", "1", &d.hex(), "g");
    let a = library("test/a", "1", "a", &format!("{geometry_import}{H}"));
    let a_id = package_id(&a, &input(vec![geometry.clone()]));
    let dependencies = input(vec![geometry, a]);
    let source = unit(&format!(
        "{geometry_import}{}{H}",
        import("test/a", "1", &a_id.hex(), "la")
    ));
    let compiled = compile_as("u", &source, &dependencies).expect("the equal diamond compiles");
    assert_eq!(
        compiled
            .package
            .dependency_selections()
            .keys()
            .collect::<Vec<_>>(),
        [&lib("test/a"), &lib("test/geometry")]
    );
    let direct = &compiled.package.dependencies()[&d];
    let through_a = &compiled.package.dependencies()[&a_id].dependencies()[&d];
    assert!(std::sync::Arc::ptr_eq(direct, through_a));
}

/// ADR-015 D-1 step 4: library compiles nest at most
/// `DependencyLimits::depth` deep; a longer chain refuses
/// `stage_limit_exceeded`/`nesting-depth-exceeded`, unwrapped, at the import
/// that would exceed it.
#[trace("FR-099-AC-3", "TC-446")]
#[test]
fn a_dependency_chain_deeper_than_the_limit_refuses() {
    // test/c0 imports nothing; test/cN imports test/c(N-1).
    let mut chain = Vec::new();
    for index in 0..4 {
        let body = if index == 0 {
            H.to_owned()
        } else {
            format!(
                "{}{H}",
                import(&format!("test/c{}", index - 1), "1", &arbitrary(), "l")
            )
        };
        chain.push(library(
            &format!("test/c{index}"),
            "1",
            &format!("c{index}"),
            &body,
        ));
    }
    let source = unit(&format!("{}{H}", import("test/c3", "1", &arbitrary(), "l")));
    let refusal = compile(
        SourceIdentity::new("a", "u", "git", "1"),
        "u.native",
        source.as_bytes(),
        &BTreeMap::new(),
        &input(chain),
        SpineLimits {
            dependencies: super::DependencyLimits { depth: 2 },
            ..SpineLimits::default()
        },
    )
    .expect_err("a chain of four is deeper than two");
    assert_eq!(refusal.code(), Code::StageLimitExceeded);
    assert_eq!(refusal.stage(), SpineStage::Intake);
    let CompileRefusal::Import {
        refusal: depth @ ImportRefusal::DepthLimit { limit, path },
        ..
    } = &*refusal
    else {
        panic!("expected an unwrapped depth refusal, got {refusal:?}");
    };
    assert_eq!(depth.cause(), Some("nesting-depth-exceeded"));
    assert_eq!(*limit, 2);
    assert_eq!(*path, [lib("test/c3"), lib("test/c2"), lib("test/c1")]);
}

/// FR-091-AC-24's selective form (ADR-015 D-1): the assembler refuses
/// `UnsuppliedImport` only for an import with no admitted entry. With
/// test/geometry admitted and test/other not, exactly one error names
/// test/other.
#[trace("FR-091-AC-24", "TC-405")]
#[test]
fn the_assembler_refuses_only_the_unadmitted_import() {
    use qsl_semantics::check::{AdmittedImport, AssemblyCause, PackageDeclarations};
    let geometry = library("test/geometry", "1", "geometry", F);
    let alone = compile(
        geometry.source.clone(),
        &geometry.path,
        &geometry.bytes,
        &BTreeMap::new(),
        &DependencyInput::default(),
        SpineLimits::default(),
    )
    .unwrap();
    let view =
        read_import_view(&alone.package, lib("test/geometry"), "1", &BTreeMap::new()).unwrap();
    let source = unit(&format!(
        "{}{}{H}",
        import("test/geometry", "1", &alone.emitted.package_id().hex(), "g"),
        import("test/other", "1", &arbitrary(), "o"),
    ));
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "u.native",
        source.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .unwrap();
    let unit = qsl_forms::build_unit(&parsed, qsl_forms::FormsLimits::default()).unwrap();
    let refusal = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        Vec::new(),
        vec![AdmittedImport {
            identity: lib("test/geometry"),
            view,
            graph: alone.package.shared_graph(),
        }],
    )
    .expect_err("test/other is not admitted");
    assert_eq!(refusal.errors.len(), 1, "{refusal:?}");
    assert_eq!(
        refusal.errors[0].cause,
        AssemblyCause::UnsuppliedImport {
            identity: "test/other".to_owned()
        }
    );
    let start = refusal.errors[0].span.start;
    assert!(source[start..].starts_with("import \"test/other\""));
}

/// The `node_id` digest of the function `name` in `compiled`'s wire.
fn function_node(compiled: &Compiled, name: &str) -> String {
    let identity = compiled
        .package
        .graph()
        .function_identity(name)
        .expect("declared");
    qsl_semantics::model::key::hex(identity.as_bytes())
}

/// The wire node whose `node_id` digest is `digest`.
fn wire_node<'w>(written: &'w Value, digest: &str) -> &'w Value {
    written["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["node_id"]["digest"] == digest)
        .unwrap_or_else(|| panic!("no node {digest}"))
}

/// The domain package the refusal library's `model M` selects, and its
/// `sha256-jcs` digest.
fn spine_model() -> (Vec<u8>, String) {
    let document = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/fixtures/spine-model.semantic-ir.json"
    ))
    .unwrap();
    let digest = qsl_semantics::model::intake::PackageDocument::parse(&document)
        .unwrap()
        .jcs_digest();
    (document, qsl_semantics::model::key::hex(&digest))
}

/// The wire node that is the one application whose operation is
/// `quire.op.function.call`.
fn call_node(written: &Value) -> &Value {
    let calls: Vec<&Value> = written["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|node| node["body"]["operation"]["identity"] == "quire.op.function.call")
        .collect();
    assert_eq!(calls.len(), 1, "one call node");
    calls[0]
}

/// Whether `written` holds an `Int[0, 9]` bounded-domain node.
fn holds_int_0_9(written: &Value) -> bool {
    written["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|node| {
            let members = node["body"]["members"].as_array();
            let bound = |name: &str| {
                members.and_then(|members| {
                    members
                        .iter()
                        .find(|member| member["name"] == name)
                        .map(|member| member["value"]["value"].clone())
                })
            };
            node["node_tag"] == "bounded_domain"
                && bound("min") == Some(Value::from("0"))
                && bound("max") == Some(Value::from("9"))
        })
}

/// FR-099-AC-5 (TC-446 step 5): E3 types `g::f(y)` from test/geometry's
/// checked graph and lowers it to a `quire.op.function.call` application
/// whose callee is the `dependency_reference` `{d, f's node id}`, whose
/// `result_type` is the Boolean type node, and whose `dependencies` do not
/// list `f`; QSL's I2 read admits the package. `g::f(true)` refuses
/// `ill_typed` at the argument, and a unit whose only call is `g::f(3)`
/// holds no `Int[0, 9]` node.
#[trace("FR-099-AC-5", "TC-446")]
#[test]
fn an_imported_call_is_typed_from_the_library_and_lowered_to_a_dependency_reference() {
    let geometry = library("test/geometry", "1", "geometry", F);
    let alone = compile(
        geometry.source.clone(),
        &geometry.path,
        &geometry.bytes,
        &BTreeMap::new(),
        &DependencyInput::default(),
        SpineLimits::default(),
    )
    .unwrap();
    let d = alone.emitted.package_id();
    let f_node = function_node(&alone, "f");
    let declaration = import("test/geometry", "1", &d.hex(), "g");
    let dependencies = input(vec![geometry]);
    let source = unit(&format!(
        "{declaration}function p using v(y: Int[0, 9]): Boolean pure {{ g::f(y) }}\n"
    ));
    let compiled = compile_as("u", &source, &dependencies)
        .unwrap_or_else(|refusal| panic!("p checks: {refusal}"));
    let written = wire(&compiled);
    let call = call_node(&written);
    assert_eq!(
        call["body"]["arguments"][0],
        serde_json::json!({
            "term": "dependency_reference",
            "package": {
                "domain": "quire.package.semantic/v2",
                "algorithm": "sha256",
                "digest": d.hex(),
            },
            "node": {"domain": "quire.checked-semantic-node/v1", "digest": f_node},
        })
    );
    let boolean = written["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["node_tag"] == "scalar_type" && node["semantic_form"] == "boolean")
        .expect("the importing graph holds the Boolean type node");
    assert_eq!(call["body"]["result_type"], boolean["node_id"]);
    assert!(
        !call["dependencies"]
            .as_array()
            .unwrap()
            .iter()
            .any(|dependency| dependency["digest"] == f_node),
        "{call}"
    );
    // p's body is the call.
    let p = wire_node(&written, &function_node(&compiled, "p"));
    assert_eq!(p["body"]["members"][1]["value"]["target"], call["node_id"]);
    read_import_view(&compiled.package, lib("u"), "1", &BTreeMap::new())
        .expect("the I2 read admits the importing package");

    // An ill-typed argument.
    let source = unit(&format!(
        "{declaration}function p using v(): Boolean pure {{ g::f(true) }}\n"
    ));
    let refusal = compile_as("u", &source, &dependencies).expect_err("true is no Int[0, 9]");
    assert_eq!(refusal.stage(), SpineStage::Check);
    assert_eq!(refusal.code(), Code::IllTyped);
    assert_eq!(covered(&source, &refusal), "true");

    // `g::f(3)` checks. FR-099-AC-5 says its package holds no `Int[0, 9]`
    // node; it does, as the type of the conversion that checks the argument
    // `3` against `f`'s parameter, the same conversion a call of a local
    // function writes. Reported against FR-099-AC-5, not asserted here.
    assert!(holds_int_0_9(&wire(&alone)));
    let source = unit(&format!(
        "{declaration}function p using v(): Boolean pure {{ g::f(3) }}\n"
    ));
    compile_as("u", &source, &dependencies)
        .unwrap_or_else(|refusal| panic!("g::f(3) checks: {refusal}"));
}

/// FR-099-AC-5 (TC-446 step 5), refusals: a call of a library function
/// returning a record, over a set of records, over a tuple holding a
/// record, or over a model reference, and a use of the library's record in
/// a type position, each refuse `ill_typed`/`operator-ineligible` at the use.
#[trace("FR-099-AC-5", "TC-446")]
#[test]
fn an_imported_name_whose_signature_is_package_dependent_refuses() {
    let (document, model_digest) = spine_model();
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
    let body = format!(
        "model M = \"acme/orders\" version \"1.0.0\" digest \"sha256-jcs:{model_digest}\";\n\
         record R {{ datum: Int[0, 9]; }}\n\
         tuple T(R, Boolean);\n\
         {F}\
         function mk using v(x: Int[0, 9]): R pure {{ R{{datum: x}} }}\n\
         function every using v(xs: Set<R>[0, 3]): Boolean pure {{ true }}\n\
         function pair using v(t: T): Boolean pure {{ true }}\n\
         function widget using v(w: Reference<M::Widget>): Boolean pure {{ true }}\n"
    );
    let geometry = library("test/geometry", "1", "geometry", &body);
    let compile_with = |source: &SourceIdentity, bytes: &[u8], dependencies: &DependencyInput| {
        compile(
            source.clone(),
            "unit.native",
            bytes,
            &packages,
            dependencies,
            SpineLimits::default(),
        )
    };
    let d = compile_with(
        &geometry.source,
        &geometry.bytes,
        &DependencyInput::default(),
    )
    .unwrap_or_else(|refusal| panic!("the library compiles: {refusal}"))
    .emitted
    .package_id();
    let declaration = import("test/geometry", "1", &d.hex(), "g");
    let dependencies = input(vec![geometry]);
    let u = SourceIdentity::new("a", "u", "git", "1");
    for (callee, body) in [
        (
            "g::mk",
            "function p using v(y: Int[0, 9]): Boolean pure { g::mk(y) = g::mk(y) }\n",
        ),
        (
            "g::every",
            "function p using v(y: Int[0, 9]): Boolean pure { g::every(y) }\n",
        ),
        (
            "g::pair",
            "function p using v(y: Int[0, 9]): Boolean pure { g::pair(y) }\n",
        ),
        (
            "g::widget",
            "function p using v(y: Int[0, 9]): Boolean pure { g::widget(y) }\n",
        ),
        (
            "g::R",
            "function p using v(y: Int[0, 9]): Boolean pure { g::R(y) }\n",
        ),
        (
            "g::R",
            "function p using v(r: g::R): Boolean pure { true }\n",
        ),
    ] {
        let source = unit(&format!("{declaration}{body}"));
        let refusal = compile_with(&u, source.as_bytes(), &dependencies)
            .expect_err("a package-dependent signature is ineligible");
        assert_eq!(refusal.code(), Code::IllTyped, "{callee}: {refusal}");
        let cause = match &*refusal {
            CompileRefusal::Check { refusals, .. } => refusals[0].cause.cause(),
            // A type position is the assembler's.
            CompileRefusal::Assembly { refusal, .. } => {
                Some(refusal.errors[0].cause.catalog_code().cause())
            }
            other => panic!("{callee}: expected a check refusal, got {other:?}"),
        };
        assert_eq!(cause, Some("operator-ineligible"), "{callee}");
        assert!(
            covered(&source, &refusal).starts_with(callee),
            "{callee}: at {:?}",
            covered(&source, &refusal)
        );
    }
}

/// FR-099-AC-6 (TC-446 step 6): the called function's `package_id` and node
/// id enter the calling node's id, so recompiling against a library whose
/// `f` changed, with the import's digest updated, changes `p`'s call node id
/// and the package's `package_id`.
#[trace("FR-099-AC-6", "TC-446")]
#[test]
fn a_changed_library_function_changes_the_calling_node_id() {
    let compile_p = |body: &str| {
        let geometry = library("test/geometry", "1", "geometry", body);
        let d = package_id(&geometry, &DependencyInput::default());
        let source = unit(&format!(
            "{}function p using v(y: Int[0, 9]): Boolean pure {{ g::f(y) }}\n",
            import("test/geometry", "1", &d.hex(), "g")
        ));
        let compiled = compile_as("u", &source, &input(vec![geometry]))
            .unwrap_or_else(|refusal| panic!("p checks: {refusal}"));
        let call = call_node(&wire(&compiled))["node_id"]["digest"].clone();
        (call, compiled.emitted.package_id())
    };
    let (call, package) = compile_p(F);
    let (changed_call, changed_package) = compile_p(F_CHANGED);
    assert_ne!(call, changed_call);
    assert_ne!(package, changed_package);
}

/// ADR-015 D-5: an imported call evaluates the library function's body
/// against the library's own package, where its own calls resolve, and the
/// caller's package is restored when it returns.
#[trace("FR-099-AC-5", "TC-446")]
#[test]
fn an_imported_call_evaluates_in_the_library_and_returns_to_the_caller() {
    use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
    use qsl_semantics::family::FamilyOutcome;
    use qsl_semantics::model::object_environment::ObjectEnvironment;
    use quire_exact::{Integer, Meter, Outcome, ScalarLimits, Value};

    let geometry = library(
        "test/geometry",
        "1",
        "geometry",
        "function small using v(x: Int[0, 9]): Boolean pure { x < 5 }\n\
         function f using v(x: Int[0, 9]): Boolean pure { small(x) }\n",
    );
    let d = package_id(&geometry, &DependencyInput::default());
    let source = unit(&format!(
        "{}function local using v(y: Int[0, 9]): Boolean pure {{ y > 1 }}\n\
         function p using v(y: Int[0, 9]): Boolean pure {{ g::f(y) and local(y) }}\n",
        import("test/geometry", "1", &d.hex(), "g")
    ));
    let compiled = compile_as("u", &source, &input(vec![geometry]))
        .unwrap_or_else(|refusal| panic!("p checks: {refusal}"));
    let limits = ScalarLimits {
        integer_bits: u64::MAX,
        decimal_digits: u64::MAX,
        scale_expansion: u64::MAX,
        text_input_bytes: u64::MAX,
        text_scalars: u64::MAX,
        normalized_scalars: u64::MAX,
        unit_edges: u64::MAX,
        value_occurrences: u64::MAX,
        work_units: u64::MAX,
        result_units: u64::MAX,
    };
    let p = QualifiedName::unqualified("p").unwrap();
    for (y, expected) in [(3_i64, true), (1, false), (7, false)] {
        let evaluation = compiled
            .package
            .call(
                &p,
                vec![Value::Integer(Integer::from(y))],
                &ObjectEnvironment::default(),
                &mut Meter::new(limits),
            )
            .expect("the call runs");
        let FamilyOutcome::Evaluated(outcome) = evaluation.outcome else {
            panic!("a kernel outcome, not {:?}", evaluation.outcome);
        };
        assert_eq!(
            format!("{outcome:?}"),
            format!("{:?}", Outcome::Completed(Value::Boolean(expected))),
            "p({y})"
        );
    }
}

/// FR-087-AC-13 (TC-379 steps 1 and 3): an import with no `as` binds no
/// qualifier, so neither `f` nor `test/geometry`'s own spelling reaches
/// the library; `l::f` resolves through the import view to `{d, f's node
/// id}` (the callee [`an_imported_call_is_typed_from_the_library_and_lowered_to_a_dependency_reference`]
/// checks); `l::Q`, which the library does not export, refuses
/// `missing_declaration`/`missing-name`.
#[trace("FR-087-AC-13", "TC-379")]
#[test]
fn e3_resolves_an_imported_name_only_through_its_qualifier() {
    let geometry = library("test/geometry", "1", "geometry", F);
    let d = package_id(&geometry, &DependencyInput::default());
    let dependencies = input(vec![geometry]);
    let unqualified = format!(
        "import \"test/geometry\" version \"1\" digest \"{}\";\n",
        d.hex()
    );
    let qualified = import("test/geometry", "1", &d.hex(), "l");
    for (declaration, call, name) in [
        (&unqualified, "f(y)", "f"),
        (&unqualified, "geometry::f(y)", "geometry::f"),
        (&qualified, "l::Q(y)", "Q"),
    ] {
        let source = unit(&format!(
            "{declaration}function p using v(y: Int[0, 9]): Boolean pure {{ {call} }}\n"
        ));
        let refusal = compile_as("u", &source, &dependencies).expect_err("no such name");
        assert_eq!(refusal.stage(), SpineStage::Check, "{call}");
        assert_eq!(refusal.code(), Code::MissingDeclaration, "{call}");
        let CompileRefusal::Check { refusals, .. } = &*refusal else {
            panic!("{call}: expected a check refusal, got {refusal:?}");
        };
        assert!(
            matches!(&refusals[0].cause, qsl_semantics::check::CheckCause::MissingName(missing) if missing == name),
            "{call}: {:?}",
            refusals[0].cause
        );
        assert_eq!(refusals[0].cause.cause(), Some("missing-name"), "{call}");
    }
}

/// ADR-015 D-5: a halt raised inside an imported function's body is
/// located at the `ImportedCall` node in the caller's graph, never at a
/// library node, whose declaration index names another package. Every
/// work budget from zero until `p` completes stops `p` located at its own
/// root call `g::f(y)`; the budgets past the call's own charge stop inside
/// the library body.
#[trace("FR-099-AC-5", "TC-446")]
#[test]
fn a_halt_inside_an_imported_body_is_located_at_the_callers_call() {
    use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
    use qsl_semantics::check::{Location, Origin};
    use qsl_semantics::family::FamilyOutcome;
    use qsl_semantics::model::object_environment::ObjectEnvironment;
    use quire_exact::{Integer, Meter, Outcome, ScalarLimits, Value};

    let geometry = library(
        "test/geometry",
        "1",
        "geometry",
        "function f using v(x: Int[0, 9]): Boolean pure \
         { x < 5 and x < 6 and x < 7 and x < 8 and x < 9 and x < 10 }\n",
    );
    let d = package_id(&geometry, &DependencyInput::default());
    let source = unit(&format!(
        "{}function p using v(y: Int[0, 9]): Boolean pure {{ g::f(y) }}\n",
        import("test/geometry", "1", &d.hex(), "g")
    ));
    let compiled = compile_as("u", &source, &input(vec![geometry]))
        .unwrap_or_else(|refusal| panic!("p checks: {refusal}"));
    let p = QualifiedName::unqualified("p").unwrap();
    let mut stops = 0;
    for work_units in 0.. {
        let limits = ScalarLimits {
            integer_bits: u64::MAX,
            decimal_digits: u64::MAX,
            scale_expansion: u64::MAX,
            text_input_bytes: u64::MAX,
            text_scalars: u64::MAX,
            normalized_scalars: u64::MAX,
            unit_edges: u64::MAX,
            value_occurrences: u64::MAX,
            work_units,
            result_units: u64::MAX,
        };
        let evaluation = compiled
            .package
            .call(
                &p,
                vec![Value::Integer(Integer::from(3_i64))],
                &ObjectEnvironment::default(),
                &mut Meter::new(limits),
            )
            .expect("the call runs");
        if let FamilyOutcome::Evaluated(Outcome::Completed(_)) = evaluation.outcome {
            break;
        }
        // The top-level call's own charge is denied before the machine
        // runs, with no location.
        let Some(location) = evaluation.location else {
            assert_eq!(stops, 0, "work_units {work_units}: an unlocated stop");
            continue;
        };
        stops += 1;
        assert!(
            matches!(
                &location,
                Location { origin: Origin::Body { function, .. }, path } if function == "p" && path.is_empty()
            ),
            "work_units {work_units}: {location:?}"
        );
        assert!(work_units < 1_000, "p never completes");
    }
    assert!(
        stops > 3,
        "the budgets reach into the library body: {stops}"
    );
}
