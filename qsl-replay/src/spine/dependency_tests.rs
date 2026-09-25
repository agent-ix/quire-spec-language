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

    // QSL's I2 read admits the package: the emission `compile` wrote, read
    // back pinned at its own `package_id`.
    let emission = emit_checked(&compiled.package).expect("the package emits");
    assert_eq!(emission.package().bytes(), compiled.emitted.bytes());
    let view = read_import_view(&emission, lib("u"), "1", &BTreeMap::new())
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
            } if identity == "test/geometry"
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
            first: SourceIdentity::new("a", "geometry", "git", "1"),
            second: SourceIdentity::new("a", "geometry-2", "git", "1"),
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
            } if identity == "test/missing"
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
}
