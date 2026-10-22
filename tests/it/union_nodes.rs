// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-320 (TC-827, TC-828) through the public spine: a unit holding a union,
//! a construction and a `case` is compiled and emitted, recompiled from its
//! retained source, and its `case` arm's claim is a requested item of the
//! emitted package.

use std::collections::{BTreeMap, BTreeSet};

use ix_trace_rs::trace;
use qsl_foundation::bound::DomainKind;
use qsl_foundation::digest::ByteDigest;
use qsl_foundation::{BackendId, Setting, SourceIdentity};
use qsl_replay::spine::DependencyInput;
use qsl_replay::{compile_package, ReplayLimits, StageLimits};
use qsl_route::request::items_from_requirements;
use qsl_route::{BackendDescriptor, Candidate, ManifestDigest, Mode, ProviderOrigin, Registry};
use qsl_semantics::check::Capability;
use serde_json::Value;

use crate::support::front_end::emitted;

const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n\
    union Shape { Empty, Circle(Integer), Rect(Integer, Integer) }\n\
    function area using v(s: Shape): Integer pure \
    { case s { Empty: 0; Circle(r): r; Rect(w, h): w * h; } }\n\
    function rect using v(): Shape pure { Shape::Rect(2, 3) }\n";

fn identity(revision: &str) -> SourceIdentity {
    SourceIdentity::new("test", "union-nodes", "fixture", revision)
}

/// The package's `semantic_graph` nodes.
fn nodes(wire: &Value) -> &[Value] {
    wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("a node list")
}

/// TC-827 step 2: the retained source recompiles, through the facade replay
/// recompiles with, to the emitted `package_id`.
#[trace("FR-320-AC-1", "TC-827")]
#[test]
fn a_union_unit_recompiles_to_its_emitted_package_id() {
    let original = emitted(
        identity("1"),
        "unit.native",
        UNIT.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
    )
    .expect("the unit compiles and emits");
    let wire: Value = serde_json::from_slice(original.package().bytes()).expect("a JSON wire");
    let forms: BTreeSet<&str> = nodes(&wire)
        .iter()
        .filter_map(|node| node["semantic_form"].as_str())
        .collect();
    for form in ["union", "union_value", "case"] {
        assert!(forms.contains(form), "the emitted package holds a {form}");
    }

    let stages: StageLimits = [(Setting::S1InputBytes, 1 << 20)].into_iter().collect();
    let recompiled = compile_package(
        identity("2"),
        "unit.native",
        original.sources()[0].text().as_bytes(),
        [],
        &DependencyInput::default(),
        stages,
        ReplayLimits::default(),
    )
    .expect("the retained source recompiles");
    assert_eq!(recompiled.bytes(), original.package().bytes());
    assert_eq!(
        recompiled.package_id(),
        original.package().package_id().record()
    );
}

/// TC-828 step 1: `area`'s one claim, the multiplication in the `Rect` arm,
/// is a `value-validity` item of the emitted package, and the `case` node
/// names it. The IR and CG arms that settle such an item `unsupported` are
/// not part of this repository, so QSL's side is that it requests the item
/// and records no settlement of its own.
#[trace("FR-320-AC-3", "TC-828")]
#[test]
fn a_claim_inside_a_case_arm_is_requested_as_a_value_validity_item() {
    let original = emitted(
        identity("1"),
        "unit.native",
        UNIT.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
    )
    .expect("the unit compiles and emits");
    let wire: Value = serde_json::from_slice(original.package().bytes()).expect("a JSON wire");

    let descriptor = BackendDescriptor::new(
        Candidate::new(BackendId::new("kani")),
        ProviderOrigin::Linked,
        ManifestDigest::from_digest(ByteDigest::of(b"kani").as_bytes()),
        [(Capability::ValueValidity, Mode::Bounded)],
        Some(BTreeSet::from([DomainKind::Collection])),
        None,
    )
    .expect("a well-formed descriptor");
    let mut registry = Registry::new();
    registry.register(descriptor).expect("one registration");

    // The checked package the emission was written from.
    let parsed = qsl_cst::parse(
        identity("1"),
        "unit.native",
        UNIT.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let graph = qsl_semantics::check::PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the unit assembles")
    .check(quire_semantic_value::checking::CheckingLimits::default())
    .expect("the unit checks");
    let items = items_from_requirements(graph.requirements(), &registry, None);
    let [item] = items.as_slice() else {
        panic!("area's multiplication is its one claim: {items:#?}");
    };
    assert_eq!(item.kind(), Capability::ValueValidity);

    let node = item.node().to_string();
    let written = nodes(&wire)
        .iter()
        .find(|candidate| candidate["node_id"]["digest"] == node.as_str())
        .expect("the item's node is written to the package");
    assert_eq!(written["semantic_form"], "binary");
    let case = nodes(&wire)
        .iter()
        .find(|candidate| candidate["semantic_form"] == "case")
        .expect("the package holds the case");
    let names_it = case["dependencies"]
        .as_array()
        .expect("dependencies")
        .iter()
        .any(|dependency| dependency["digest"] == node.as_str());
    assert!(names_it, "the case names its arm's multiplication");
}
