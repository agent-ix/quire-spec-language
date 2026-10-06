// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-908 (FR-060-AC-5, FR-060-AC-6, FR-060-AC-7): `compile_package` emits the bytes and
//! `package_id` the spine emits, and refuses as the spine does. As an
//! integration test it reaches only the crate's public API.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_replay::spine::{self, CompileRefusal, ParseRequest, SpineLimits};
use qsl_replay::{
    compile_package, Code, DependencyInput, ReplayRefusal, ScalarLimits, SourceIdentity,
    StageLimits, SuppliedLibrary,
};
use qsl_semantics::model::intake::package_input;
use quire_exact::Cancel;

const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\n";
const LIST: &str = "record List { next: List?; }\n";
const TREE: &str = "record Tree { kids: Sequence<Tree>[0, 3]; }\n";

fn identity() -> SourceIdentity {
    SourceIdentity::new("a", "u", "git", "1")
}

fn limits(source_bytes: u64) -> StageLimits {
    let unbounded = ScalarLimits {
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
    StageLimits {
        s1: ScalarLimits {
            text_input_bytes: source_bytes,
            ..unbounded
        },
        s2: unbounded,
        s3: unbounded,
        s4: unbounded,
    }
}

/// The spine's own run over `text`: its package bytes and `package_id`, or
/// its refusal.
fn spine_run(
    text: &str,
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
) -> Result<(Vec<u8>, String), Box<CompileRefusal>> {
    let source = identity();
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let refusal = |failure| {
        spine::refusal_or_fault(failure).expect("the front end does not fault on this source")
    };
    let parsed = spine::parse(
        &ParseRequest {
            source: &source,
            path: "unit.native",
            bytes: text.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .map_err(refusal)?
    .into_value();
    let models = spine::select(&parsed, packages, limits.model, &cancel)
        .map_err(refusal)?
        .into_value();
    let checked = spine::check(
        &parsed,
        &models,
        &DependencyInput::default(),
        &spine::LockEvidence::default(),
        limits,
        &cancel,
    )
    .map_err(refusal)?
    .into_value();
    let emitted = spine::package(&checked, spine::PackageLimits::default(), &cancel)
        .map_err(refusal)?
        .into_value();
    Ok((
        emitted.package().bytes().to_vec(),
        emitted.package().package_id().hex(),
    ))
}

fn compile(text: &str, stages: StageLimits) -> Result<qsl_replay::CompiledPackage, ReplayRefusal> {
    compile_package(
        identity(),
        "unit.native",
        text.as_bytes(),
        [],
        &DependencyInput::default(),
        stages,
    )
}

/// FR-092's recursive `List` and `Tree` compile through the facade to the
/// spine's bytes and `package_id`.
#[trace("TC-908", "FR-060-AC-5")]
#[test]
fn recursive_records_compile_to_the_spine_bytes_and_package_id() {
    for record in [LIST, TREE] {
        let text = format!("{HEADER}{record}");
        let (bytes, package_id) =
            spine_run(&text, &BTreeMap::new()).expect("the spine compiles the record");
        let compiled = compile(&text, limits(1 << 20)).expect("the facade compiles the record");
        assert!(!bytes.is_empty());
        assert_eq!(compiled.bytes(), bytes.as_slice());
        assert_eq!(compiled.package_id().hex(), package_id);
    }
    let list = compile(&format!("{HEADER}{LIST}"), limits(1 << 20)).unwrap();
    let tree = compile(&format!("{HEADER}{TREE}"), limits(1 << 20)).unwrap();
    assert_ne!(list.package_id(), tree.package_id());
}

/// A malformed source refuses with the spine refusal's code and stage.
#[trace("TC-908", "FR-060-AC-6")]
#[test]
fn a_malformed_source_refuses_as_the_spine_does() {
    let text = format!("{HEADER}record {{ ");
    let expected = spine_run(&text, &BTreeMap::new()).expect_err("the spine refuses the source");
    let Err(ReplayRefusal::Recompile(refusal)) = compile(&text, limits(1 << 20)) else {
        panic!("the facade refuses the source at the recompile");
    };
    assert_eq!(refusal.code(), expected.code());
    assert_eq!(refusal.stage(), expected.stage());
    assert_eq!(refusal.code(), Code::InvalidSyntax);
}

/// A library whose source has the unit's owner is no dependency input of it.
#[trace("TC-908", "FR-060-AC-6")]
#[test]
fn a_library_with_the_units_owner_refuses() {
    let library = SuppliedLibrary {
        identity: "lib".to_owned(),
        source: SourceIdentity::new("a", "u", "git", "2"),
        path: "lib.native".to_owned(),
        bytes: HEADER.as_bytes().to_vec(),
    };
    let dependencies = DependencyInput::new([library]).unwrap();
    let refusal = compile_package(
        identity(),
        "unit.native",
        format!("{HEADER}{LIST}").as_bytes(),
        [],
        &dependencies,
        limits(1 << 20),
    );
    assert!(matches!(refusal, Err(ReplayRefusal::DependencyInput(_))));
}

/// The S1 byte limit stops a source above it, and a limit above the reader
/// limit is an invalid request.
#[trace("TC-908", "FR-060-AC-6")]
#[test]
fn stage_limits_refuse_as_the_replay_recompile_does() {
    let text = format!("{HEADER}{LIST}");
    let Err(ReplayRefusal::Recompile(refusal)) = compile(&text, limits(4)) else {
        panic!("a 4-byte S1 limit stops the source");
    };
    assert_eq!(refusal.code(), Code::StageLimitExceeded);
    assert!(matches!(
        compile(&text, limits(u64::MAX)),
        Err(ReplayRefusal::LimitAboveReader(_))
    ));
}

/// A unit selecting a domain package compiles through the facade to the
/// spine's bytes and `package_id` when the package is supplied, and refuses
/// at I1 as the spine does when it is not.
#[trace("TC-908", "FR-060-AC-7")]
#[test]
fn a_domain_package_is_the_i1_input_as_in_the_spine() {
    let document = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/fixtures/spine-model.semantic-ir.json"
    ))
    .unwrap();
    let packages = package_input([document.as_slice()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    let text = format!(
        "{HEADER}model M = \"acme/orders\" version \"1.0.0\" digest \"sha256-jcs:{hex}\";\n\
         function small using v(x: Int[0, 9]): Boolean pure {{ x < 5 }}\n"
    );
    let (bytes, package_id) = spine_run(&text, &packages).expect("the spine compiles the unit");
    let compiled = compile_package(
        identity(),
        "unit.native",
        text.as_bytes(),
        [document.as_slice()],
        &DependencyInput::default(),
        limits(1 << 20),
    )
    .expect("the facade compiles the unit");
    assert_eq!(compiled.bytes(), bytes.as_slice());
    assert_eq!(compiled.package_id().hex(), package_id);

    let expected = spine_run(&text, &BTreeMap::new()).expect_err("the spine refuses at I1");
    let Err(ReplayRefusal::Recompile(refusal)) = compile(&text, limits(1 << 20)) else {
        panic!("the facade refuses the unit at the recompile");
    };
    assert_eq!(refusal.code(), expected.code());
    assert_eq!(refusal.stage(), expected.stage());
    assert_eq!(refusal.code(), Code::MissingImport);
}
