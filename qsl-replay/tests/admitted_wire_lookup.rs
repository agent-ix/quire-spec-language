// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1090 (FR-396-AC-10): the by-wire lookups take the structural wire id
//! and answer as the NodeKey lookups do, with the same refusals.

use ix_trace_rs::trace;
use qsl_foundation::Setting;
use qsl_replay::{
    compile_admitted, AdmittedCompilation, AdmittedLookupRefusal, DependencyInput, Identifier,
    NodeKey, ReplayLimits, SourceIdentity, StageLimits, SuppliedLibrary,
};
use quire_contract_model::{CheckedNodeId, CheckedNodePathSegment};
use quire_exact::{InvalidNodeKey, PathSegment};

const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\n";
const UNIT: &str = "record Left { a: Boolean; b: Boolean; }\n\
     enum Color { BLUE, RED }\n\
     function is_blue using v(c: Color): Boolean pure { false }\n";
const LIBRARY: &str = "enum Shade { DARK, LIGHT }\nrecord Point { x: Boolean; }\n";

fn limits() -> StageLimits {
    [(Setting::S1InputBytes, 1 << 20)].into_iter().collect()
}

fn id(text: &str) -> Identifier {
    Identifier::new(text).expect("an identifier")
}

/// The structural wire id of `key`, as the emitted package spells it.
fn wire(key: &NodeKey) -> CheckedNodeId {
    let text = qsl_foundation::node_wire::node_key_to_text(key).expect("a node text");
    serde_json::from_str(&text).expect("a wire id")
}

fn library() -> SuppliedLibrary {
    SuppliedLibrary {
        identity: "test/lib".to_owned(),
        source: SourceIdentity::new("a", "lib", "git", "1"),
        path: "lib.native".to_owned(),
        bytes: format!("{HEADER}{LIBRARY}").into_bytes(),
    }
}

fn admit(text: &str, dependencies: &DependencyInput) -> AdmittedCompilation {
    compile_admitted(
        qsl_replay::CompileUnit {
            source: SourceIdentity::new("a", "u", "git", "1"),
            package_name: "fixture".to_owned(),
            path: "unit.native",
            bytes: text.as_bytes(),
        },
        [],
        dependencies,
        limits(),
        ReplayLimits::default(),
    )
    .expect("the unit compiles")
}

fn own() -> AdmittedCompilation {
    admit(&format!("{HEADER}{UNIT}"), &DependencyInput::default())
}

/// A unit importing the library, and the library's own context.
fn importing() -> (AdmittedCompilation, AdmittedCompilation) {
    let lib = library();
    let alone = compile_admitted(
        qsl_replay::CompileUnit {
            source: lib.source.clone(),
            package_name: lib.identity.clone(),
            path: &lib.path,
            bytes: &lib.bytes,
        },
        [],
        &DependencyInput::default(),
        limits(),
        ReplayLimits::default(),
    )
    .expect("the library compiles");
    let text = format!("{HEADER}import \"test/lib\" as d;\n{UNIT}");
    let unit = admit(
        &text,
        &DependencyInput::new([lib]).expect("a dependency input"),
    );
    (unit, alone)
}

fn nobody() -> NodeKey {
    NodeKey::new("fixture", vec![PathSegment::Text("7".to_owned())], 0).expect("fixture node")
}

#[trace("TC-1090", "FR-396-AC-10")]
#[test]
fn enum_by_wire_is_found() {
    let admitted = own();
    let context = admitted.context();
    let color = context.enums().next().expect("the enum");
    let key = wire(color.key());
    assert_eq!(context.enum_declaration_by_wire(&key), Ok(color));
    assert_eq!(
        context.enum_case_by_wire(&key, &id("RED")),
        Ok(color.case(&id("RED")).expect("the case"))
    );
}

#[trace("TC-1090", "FR-396-AC-10")]
#[test]
fn enum_by_wire_refuses_foreign() {
    let (unit, lib) = importing();
    let shade = lib.context().enums().next().expect("the library enum");
    let declaration = shade.key().clone();
    let foreign = Err(AdmittedLookupRefusal::Foreign {
        declaration,
        package: lib.context().package_name().clone(),
    });
    assert_eq!(
        unit.context().enum_declaration_by_wire(&wire(shade.key())),
        foreign
    );
    assert_eq!(
        unit.context()
            .enum_case_by_wire(&wire(shade.key()), &id("DARK"))
            .map(|_| ()),
        foreign.map(|_| ())
    );
}

#[trace("TC-1090", "FR-396-AC-10")]
#[test]
fn enum_by_wire_refuses_unknown() {
    let admitted = own();
    let context = admitted.context();
    let nobody = nobody();
    assert_eq!(
        context.enum_declaration_by_wire(&wire(&nobody)),
        Err(AdmittedLookupRefusal::Unknown {
            declaration: nobody.clone(),
            case: None
        })
    );
    let color = context.enums().next().expect("the enum").key().clone();
    assert_eq!(
        context.enum_case_by_wire(&wire(&color), &id("PURPLE")),
        Err(AdmittedLookupRefusal::Unknown {
            declaration: color,
            case: Some(id("PURPLE"))
        })
    );
}

#[trace("TC-1090", "FR-396-AC-10")]
#[test]
fn enum_by_wire_never_refuses_wrong_kind() {
    let admitted = own();
    let context = admitted.context();
    let record = context.composites().next().expect("the record").node();
    assert_eq!(
        context.enum_declaration_by_wire(&wire(record)),
        Err(AdmittedLookupRefusal::Unknown {
            declaration: record.clone(),
            case: None
        })
    );
}

#[trace("TC-1090", "FR-396-AC-10")]
#[test]
fn composite_by_wire_is_found() {
    let admitted = own();
    let context = admitted.context();
    let left = context.composites().next().expect("the record");
    assert_eq!(
        context.composite_declaration_by_wire(&wire(left.node())),
        Ok(left)
    );
}

#[trace("TC-1090", "FR-396-AC-10")]
#[test]
fn composite_by_wire_refuses_foreign() {
    let (unit, lib) = importing();
    let point = lib
        .context()
        .composites()
        .next()
        .expect("the library record");
    assert_eq!(
        unit.context()
            .composite_declaration_by_wire(&wire(point.node())),
        Err(AdmittedLookupRefusal::Foreign {
            declaration: point.node().clone(),
            package: lib.context().package_name().clone(),
        })
    );
}

#[trace("TC-1090", "FR-396-AC-10")]
#[test]
fn composite_by_wire_refuses_unknown() {
    let admitted = own();
    let nobody = nobody();
    assert_eq!(
        admitted
            .context()
            .composite_declaration_by_wire(&wire(&nobody)),
        Err(AdmittedLookupRefusal::Unknown {
            declaration: nobody,
            case: None
        })
    );
}

#[trace("TC-1090", "FR-396-AC-10")]
#[test]
fn composite_by_wire_refuses_wrong_kind() {
    let admitted = own();
    let context = admitted.context();
    let color = context.enums().next().expect("the enum").key().clone();
    assert_eq!(
        context.composite_declaration_by_wire(&wire(&color)),
        Err(AdmittedLookupRefusal::WrongKind { declaration: color })
    );
}

/// A wire id whose path nests another node maps to the NodeKey with that
/// nested node, and an id with no path names no node.
#[trace("TC-1090", "FR-396-AC-10")]
#[test]
fn nested_and_malformed_wire_ids() {
    let admitted = own();
    let context = admitted.context();
    let inner = nobody();
    let nested_key = NodeKey::new(
        "fixture",
        vec![
            PathSegment::Text("derived".to_owned()),
            PathSegment::Node(inner),
        ],
        1,
    )
    .expect("a nested node");
    assert_eq!(
        context.composite_declaration_by_wire(&wire(&nested_key)),
        Err(AdmittedLookupRefusal::Unknown {
            declaration: nested_key,
            case: None
        })
    );

    let empty = CheckedNodeId {
        package: "fixture".into(),
        path: Vec::<CheckedNodePathSegment>::new(),
        ordinal: 0,
    };
    assert_eq!(
        context.composite_declaration_by_wire(&empty),
        Err(AdmittedLookupRefusal::MalformedWire {
            cause: InvalidNodeKey::EmptyPath
        })
    );
}
