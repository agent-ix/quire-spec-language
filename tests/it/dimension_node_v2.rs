// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1339: base versus derived dimension identity from source text, through
//! assembly, admission, lowering and the replay facade (FR-091-AC-73,
//! FR-094-AC-14).

use ix_trace_rs::trace;
use qsl_foundation::Setting;
use qsl_replay::spine::{self, ParseRequest, SpineLimits};
use qsl_replay::{compile_admitted, DependencyInput, ReplayLimits, SourceIdentity, StageLimits};
use qsl_semantics::value::nominal_payload::NominalPayload;
use quire_exact::Cancel;

const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n\
    dimension L;\n\
    dimension N = L / L;\n\
    dimension M = L * L;\n\
    unit m : L = rational(1, 1);\n\
    unit one : N = rational(1, 1);\n\
    unit m2 : M = rational(1, 1);\n\
    function f using v(a: Quantity<m>, b: Quantity<one>, c: Quantity<m2>): Boolean pure { true }\n";

/// FR-091-AC-73 and FR-094-AC-14 (TC-1339): `dimension N = L / L;` stays a
/// derived dimension with no terms in the lowered `dimension` node's payload
/// and in the replay facade, while `L` is base and `M = L * L` is derived
/// with one term.
#[trace("FR-091-AC-73", "FR-094-AC-14", "TC-1339")]
#[test]
fn an_empty_derived_dimension_stays_derived_through_lowering_and_replay() {
    let limits: StageLimits = [(Setting::S1InputBytes, 1 << 20)].into_iter().collect();
    let admitted = compile_admitted(
        qsl_replay::CompileUnit {
            source: SourceIdentity::new("a", "u"),
            package_name: "fixture".to_owned(),
            path: "unit.native",
            bytes: SOURCE.as_bytes(),
        },
        [],
        &DependencyInput::default(),
        limits,
        ReplayLimits::default(),
    )
    .expect("the unit admits");

    let context = admitted.context();
    let facade = |name: &str| {
        context
            .dimensions()
            .find(|d| d.qualified_name().last().map(|n| n.as_str()) == Some(name))
            .expect("the dimension is listed")
    };
    assert!(!facade("N").is_base() && facade("N").terms().is_empty());
    assert!(facade("L").is_base() && facade("L").terms().is_empty());
    assert!(!facade("M").is_base() && facade("M").terms().len() == 1);

    let source = SourceIdentity::new("a", "u");
    let spine_limits = SpineLimits::default();
    let cancel = Cancel::new();
    let parsed = spine::parse(
        &ParseRequest {
            source: &source,
            path: "unit.native",
            bytes: SOURCE.as_bytes(),
        },
        spine_limits.source,
        &cancel,
    )
    .expect("parses")
    .into_value();
    let models = spine::select(&parsed, &Default::default(), spine_limits.model, &cancel)
        .expect("selects")
        .into_value();
    let checked = spine::check(
        &parsed,
        "fixture".to_owned(),
        &models,
        &DependencyInput::default(),
        spine_limits,
        &cancel,
    )
    .expect("checks")
    .into_value();
    let lowered: Vec<(Vec<String>, bool, usize)> = checked
        .package()
        .graph()
        .semantic_graph()
        .nodes()
        .filter_map(|node| match node.nominal_payload() {
            Some(NominalPayload::Dimension(dimension)) => Some((
                node.declaration()
                    .unwrap_or_default()
                    .iter()
                    .map(|part| part.as_str().to_owned())
                    .collect(),
                dimension.is_base(),
                dimension.terms().len(),
            )),
            _ => None,
        })
        .collect();
    let find = |name: &str| {
        lowered
            .iter()
            .find(|(path, _, _)| path.last().map(String::as_str) == Some(name))
            .unwrap_or_else(|| panic!("no lowered dimension node {name}: {lowered:?}"))
    };
    assert_eq!((find("N").1, find("N").2), (false, 0));
    assert_eq!((find("L").1, find("L").2), (true, 0));
    assert_eq!((find("M").1, find("M").2), (false, 1));
}
