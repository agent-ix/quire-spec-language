// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1088 (FR-394-AC-1 to FR-394-AC-5): `compile_admitted` returns the
//! emitted package with the admitted enum key table and refuses as typed. As
//! an integration test it reaches the facade through the crate root; only the
//! oracle that produces the expected keys runs the spine directly.

use ix_trace_rs::trace;
use qsl_foundation::Setting;
use qsl_replay::spine::{self, ParseRequest, SpineLimits};
use qsl_replay::{
    compile_admitted, compile_package, AdmittedCompilation, AdmittedLookupRefusal, AdmittedRefusal,
    AdmittedResource, DependencyInput, Identifier, NodeKey, ReplayLimits, SourceIdentity,
    StageLimits, SuppliedLibrary, WireNodeId,
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

/// An enum a dependency admitted is `Foreign`, naming that dependency; the
/// unit's own enum is found.
#[trace("TC-1088", "FR-394-AC-3")]
#[test]
fn a_dependencys_enum_is_foreign() {
    let shades = library("test/shades", "shades", SHADES);
    let own_context = compile_admitted(
        shades.source.clone(),
        &shades.path,
        &shades.bytes,
        [],
        &DependencyInput::default(),
        limits(1 << 20),
        ReplayLimits::default(),
    )
    .expect("the library compiles");
    let shade = own_context.context().enums().next().expect("an enum");
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
            package: own_context.context().package_id(),
        })
    );
    let color = context.enums().next().expect("the unit's enum");
    assert!(context.enum_declaration(wire(color.key())).is_ok());
    assert_eq!(context.enums().count(), 1);
}

/// A source or dependency input that does not compile is a package refusal;
/// a stage limit or an S1 limit above the reader is a resource refusal.
#[trace("TC-1088", "FR-394-AC-4")]
#[test]
fn compile_failures_are_package_or_resource_refusals() {
    let none = DependencyInput::default();
    let defaults = ReplayLimits::default;

    let malformed = format!("{HEADER}record {{ ");
    assert!(matches!(
        admit(&malformed, &none, limits(1 << 20), defaults()),
        Err(AdmittedRefusal::Package(_))
    ));

    let owned = SuppliedLibrary {
        source: SourceIdentity::new("a", "u", "git", "2"),
        ..library("lib", "lib", "")
    };
    let text = format!("{HEADER}{COLORS}");
    assert!(matches!(
        admit(
            &text,
            &DependencyInput::new([owned]).unwrap(),
            limits(1 << 20),
            defaults()
        ),
        Err(AdmittedRefusal::Package(_))
    ));

    assert!(matches!(
        admit(&text, &none, limits(4), defaults()),
        Err(AdmittedRefusal::Resource(AdmittedResource::StageLimit(_)))
    ));
    assert!(matches!(
        admit(&text, &none, limits(u64::MAX), defaults()),
        Err(AdmittedRefusal::Resource(AdmittedResource::AboveReader(_)))
    ));
}

/// This file names only the crate root: no `qsl_replay::spine` item is
/// needed to look a key up, only to produce the oracle above.
#[trace("TC-1088", "FR-394-AC-5")]
#[test]
fn a_consumer_looks_keys_up_through_the_crate_root() {
    let admitted = admit_ok(&format!("{HEADER}{COLORS}"));
    let (package, context) = admitted.into_parts();
    let color = context.enums().next().expect("an enum");
    let qualified: Vec<_> = color
        .qualified_name()
        .iter()
        .map(Identifier::as_str)
        .collect();
    assert_eq!(qualified, ["Color"]);
    assert!(!package.bytes().is_empty());
}
