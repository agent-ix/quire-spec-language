// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1088 (FR-394-AC-1 to FR-394-AC-4): `compile_admitted` returns the
//! emitted package with the admitted enum key table and refuses as typed. As
//! an integration test it reaches the facade through the crate root; only the
//! oracle that produces the expected keys runs the spine directly.
//! FR-394-AC-5 (the crate root alone) is `admitted_root_only.rs`.

use ix_trace_rs::trace;
use qsl_foundation::Setting;
use qsl_replay::spine::{self, ParseRequest, SpineLimits};
use qsl_replay::{
    compile_admitted, compile_package, AdmittedCompilation, AdmittedLookupRefusal, AdmittedRefusal,
    AdmittedResource, Code, DependencyInput, Identifier, NodeKey, ReplayLimits, ReplayRefusal,
    SourceIdentity, StageLimits, SuppliedLibrary, WireNodeId,
};
use qsl_semantics::check::NominalNode;
use quire_exact::Cancel;

const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\n";
// `GREEN` is declared and used by no function.
const COLORS: &str = "enum Color { BLUE, GREEN, RED }\n\
     function is_blue using v(c: Color): Boolean pure { false }\n";
const SHADES: &str = "enum Shade { DARK, LIGHT }\n";

fn unit_identity() -> SourceIdentity {
    SourceIdentity::new("a", "u", "git", "1")
}

fn limits(source_bytes: u64) -> StageLimits {
    [(Setting::S1InputBytes, source_bytes)]
        .into_iter()
        .collect()
}

fn id(text: &str) -> Identifier {
    Identifier::new(text).expect("an identifier")
}

fn wire(key: NodeKey) -> WireNodeId {
    WireNodeId::from_digest(*key.as_bytes())
}

fn library(identity: &str, name: &str, body: &str) -> SuppliedLibrary {
    SuppliedLibrary {
        identity: identity.to_owned(),
        source: SourceIdentity::new("a", name, "git", "1"),
        path: format!("{name}.native"),
        bytes: format!("{HEADER}{body}").into_bytes(),
    }
}

fn admit(
    text: &str,
    dependencies: &DependencyInput,
    stages: StageLimits,
    replay: ReplayLimits,
) -> Result<AdmittedCompilation, AdmittedRefusal> {
    compile_admitted(
        unit_identity(),
        "unit.native",
        text.as_bytes(),
        [],
        dependencies,
        stages,
        replay,
    )
}

fn admit_ok(text: &str) -> AdmittedCompilation {
    admit(
        text,
        &DependencyInput::default(),
        limits(1 << 20),
        ReplayLimits::default(),
    )
    .expect("the unit compiles")
}

/// The spine's own checked graph: every enum declaration key with its case
/// names and member keys, from the nominal nodes the checker built.
fn spine_enums(text: &str) -> Vec<(NodeKey, Vec<(String, NodeKey)>)> {
    let source = unit_identity();
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let parsed = spine::parse(
        &ParseRequest {
            source: &source,
            path: "unit.native",
            bytes: text.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .expect("parses")
    .into_value();
    let models = spine::select(&parsed, &Default::default(), limits.model, &cancel)
        .expect("selects")
        .into_value();
    let checked = spine::check(
        &parsed,
        &models,
        &DependencyInput::default(),
        &spine::LockEvidence::default(),
        limits,
        &cancel,
    )
    .expect("checks")
    .into_value();
    let graph = checked.package().graph().semantic_graph();
    let mut enums = Vec::new();
    for node in graph.nodes() {
        let Some(NominalNode::EnumDeclaration(preimage)) = node.nominal() else {
            continue;
        };
        let cases = preimage
            .members()
            .iter()
            .map(|case| {
                let member = graph
                    .nodes()
                    .find(|member| {
                        matches!(member.nominal(), Some(NominalNode::EnumMember { declaration, case: name })
                            if *declaration == node.key() && name == case)
                    })
                    .expect("a member node per case");
                (case.clone(), member.key())
            })
            .collect();
        enums.push((node.key(), cases));
    }
    enums
}

/// The context holds the package the compile emitted and, for the declared
/// enum, every case including the unused one, with the spine's keys.
#[trace("TC-1088", "FR-394-AC-1")]
#[test]
fn the_context_holds_every_case_with_the_keys_the_check_minted() {
    let text = format!("{HEADER}{COLORS}");
    let admitted = admit_ok(&text);
    let plain = compile_package(
        unit_identity(),
        "unit.native",
        text.as_bytes(),
        [],
        &DependencyInput::default(),
        limits(1 << 20),
        ReplayLimits::default(),
    )
    .expect("compile_package compiles the unit");
    assert_eq!(admitted.package().bytes(), plain.bytes());
    assert_eq!(admitted.context().package_id().record(), plain.package_id());

    let expected = spine_enums(&text);
    let [(declaration, cases)] = expected.as_slice() else {
        panic!("one enum declared");
    };
    let table: Vec<_> = admitted.context().enums().collect();
    let [color] = table.as_slice() else {
        panic!("one enum admitted");
    };
    assert_eq!(color.key(), *declaration);
    assert!(!color.is_ordered());
    let names: Vec<_> = color.cases().iter().map(|c| c.case().as_str()).collect();
    assert_eq!(names, ["BLUE", "GREEN", "RED"]);
    let keys: Vec<_> = color
        .cases()
        .iter()
        .map(|c| (c.case().as_str().to_owned(), c.key()))
        .collect();
    assert_eq!(&keys, cases);
}

/// A declaration and a case are found by wire node id; an id nobody admitted
/// and a case the enum lacks are `Unknown`.
#[trace("TC-1088", "FR-394-AC-2")]
#[test]
fn lookups_find_admitted_keys_and_refuse_unknown_ones() {
    let admitted = admit_ok(&format!("{HEADER}{COLORS}"));
    let context = admitted.context();
    let color = context.enums().next().expect("an enum");
    let declaration = wire(color.key());

    let found = context.enum_declaration(declaration).expect("found");
    assert_eq!(found.key(), color.key());
    let green = context.enum_case(declaration, &id("GREEN")).expect("found");
    assert_eq!(Some(green), color.case(&id("GREEN")));

    let nobody = WireNodeId::from_digest([7; 32]);
    assert_eq!(
        context.enum_declaration(nobody),
        Err(AdmittedLookupRefusal::Unknown {
            declaration: nobody,
            case: None
        })
    );
    assert_eq!(
        context.enum_case(declaration, &id("PURPLE")),
        Err(AdmittedLookupRefusal::Unknown {
            declaration,
            case: Some(id("PURPLE"))
        })
    );
}

/// A library compiled alone: its context names the package id and
/// the wire id of its one enum.
fn alone(library: &SuppliedLibrary, dependencies: &DependencyInput) -> AdmittedCompilation {
    compile_admitted(
        library.source.clone(),
        &library.path,
        &library.bytes,
        [],
        dependencies,
        limits(1 << 20),
        ReplayLimits::default(),
    )
    .expect("the library compiles")
}

/// An enum a direct dependency admitted is `Foreign`, naming that dependency;
/// its own enum is found. A unit cannot name an imported enum as a type
/// (the assembler refuses `ImportedTypeName`), so the import alone makes it
/// foreign.
#[trace("TC-1088", "FR-394-AC-3")]
#[test]
fn a_dependencys_enum_is_foreign() {
    let shades = library("test/shades", "shades", SHADES);
    let alone_shades = alone(&shades, &DependencyInput::default());
    let shade = alone_shades.context().enums().next().expect("an enum");
    let shade_id = wire(shade.key());

    let text = format!("{HEADER}import \"test/shades\" as s;\n{COLORS}");
    let admitted = admit(
        &text,
        &DependencyInput::new([shades]).expect("a dependency input"),
        limits(1 << 20),
        ReplayLimits::default(),
    )
    .expect("the unit compiles against the library");
    let context = admitted.context();
    assert_eq!(
        context.enum_declaration(shade_id),
        Err(AdmittedLookupRefusal::Foreign {
            declaration: shade_id,
            package: alone_shades.context().package_id(),
        })
    );
    let color = context.enums().next().expect("the unit's enum");
    assert!(context.enum_declaration(wire(color.key())).is_ok());
    assert_eq!(context.enums().count(), 1);
}

/// An enum a transitive dependency admitted is `Foreign`, naming the package
/// that declares it: unit imports `mid`, `mid` imports `shades`.
#[trace("TC-1088", "FR-394-AC-3")]
#[test]
fn a_transitive_dependencys_enum_is_foreign() {
    let shades = library("test/shades", "shades", SHADES);
    let alone_shades = alone(&shades, &DependencyInput::default());
    let shade_id = wire(
        alone_shades
            .context()
            .enums()
            .next()
            .expect("an enum")
            .key(),
    );
    let mid = library(
        "test/mid",
        "mid",
        "import \"test/shades\" as s;\nfunction h using v(): Boolean pure { true }\n",
    );

    let text = format!("{HEADER}import \"test/mid\" as m;\n{COLORS}");
    let admitted = admit(
        &text,
        &DependencyInput::new([shades, mid]).expect("a dependency input"),
        limits(1 << 20),
        ReplayLimits::default(),
    )
    .expect("the unit compiles against the libraries");
    assert_eq!(
        admitted.context().enum_declaration(shade_id),
        Err(AdmittedLookupRefusal::Foreign {
            declaration: shade_id,
            package: alone_shades.context().package_id(),
        })
    );
}

/// A source or dependency input that does not compile is a package refusal;
/// a stage limit or an S1 limit above the reader is a resource refusal.
#[trace("TC-1088", "FR-394-AC-4")]
#[test]
fn compile_failures_are_package_or_resource_refusals() {
    let none = DependencyInput::default();
    let defaults = ReplayLimits::default;

    let malformed = format!("{HEADER}record {{ ");
    let Err(AdmittedRefusal::Package(refusal)) =
        admit(&malformed, &none, limits(1 << 20), defaults())
    else {
        panic!("a malformed source is a package refusal");
    };
    assert_eq!(refusal.code(), Code::InvalidSyntax);

    let owned = SuppliedLibrary {
        source: SourceIdentity::new("a", "u", "git", "2"),
        ..library("lib", "lib", "")
    };
    let text = format!("{HEADER}{COLORS}");
    let owned = DependencyInput::new([owned]).unwrap();
    let Err(ReplayRefusal::DependencyInput(expected)) = compile_package(
        unit_identity(),
        "unit.native",
        text.as_bytes(),
        [],
        &owned,
        limits(1 << 20),
        defaults(),
    ) else {
        panic!("compile_package refuses the owned library as a dependency input");
    };
    let expected = ReplayRefusal::DependencyInput(expected);
    let Err(AdmittedRefusal::Package(refusal)) = admit(&text, &owned, limits(1 << 20), defaults())
    else {
        panic!("an owned library is a package refusal");
    };
    assert_eq!(refusal.code(), expected.code());

    assert!(matches!(
        admit(&text, &none, limits(4), defaults()),
        Err(AdmittedRefusal::Resource(AdmittedResource::StageLimit(_)))
    ));
    assert!(matches!(
        admit(&text, &none, limits(u64::MAX), defaults()),
        Err(AdmittedRefusal::Resource(AdmittedResource::AboveReader(_)))
    ));
}
