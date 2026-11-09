// SPDX-License-Identifier: AGPL-3.0-or-later
//! Source-level regressions for normalized dimension maps.

use crate::check::PackageDeclarations;
use crate::value::nominal_payload::NominalPayload;
use ix_trace_rs::trace;
use qsl_forms::build_unit;
use qsl_foundation::SourceIdentity;
use quire_semantic_value::checking::CheckingLimits;

/// A derived `L / L` dimension keeps its derived identity when its normalized
/// map is empty, while the base dimension remains non-dimensionless.
#[trace(
    "FR-091-AC-35",
    "FR-091-AC-45",
    "FR-091-AC-73",
    "FR-092-AC-25",
    "FR-093-AC-60",
    "TC-483",
    "QSpec-TC-187"
)]
#[test]
fn source_assembly_keeps_empty_derived_dimension_dimensionless() {
    let source = concat!(
        "language \"ix:native\" edition \"1-draft\";\n",
        "profile v = \"quire.value.complete/v1\";\n",
        "dimension L;\n",
        "dimension N = L / L;\n",
        "unit m : L = rational(1, 1);\n",
        "unit one : N = rational(1, 1);\n",
        "function dimensions using v(l: Quantity<m>, n: Quantity<one>): Boolean pure { true }\n",
    );
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "dimensionless-derived-unit"),
        "dimensionless.native",
        source.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("the source parses");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("the source forms build");
    let package = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "test/package".to_owned(),
        unit,
        crate::model::intake::SelectedModels::default(),
        Vec::new(),
    )
    .expect("the source declarations assemble");

    let units: Vec<_> = package.units.units().collect();
    assert_eq!(units.len(), 2);
    assert_eq!(
        units
            .iter()
            .filter(|unit| unit.dimension().is_dimensionless())
            .count(),
        1,
        "only the `L / L` unit is dimensionless"
    );
    assert_eq!(
        units
            .iter()
            .filter(|unit| !unit.dimension().is_dimensionless())
            .count(),
        1,
        "the base-dimension unit retains its dimension"
    );

    let checked = package
        .check(
            quire_semantic_value::checking::CheckMode::Linked,
            CheckingLimits::default(),
        )
        .expect("the assembled package checks");
    let graph = checked.semantic_graph();
    let dimension_of = |unit_name: &str| {
        let unit = graph
            .nodes()
            .find(|node| {
                node.semantic_form() == "unit"
                    && node.declaration().is_some_and(|segments| {
                        segments
                            .iter()
                            .map(|segment| segment.as_str())
                            .eq([unit_name])
                    })
            })
            .unwrap_or_else(|| panic!("the lowered unit {unit_name}"));
        graph
            .node(unit.semantic_type().expect("unit has its dimension type"))
            .expect("the unit's dimension is lowered")
    };
    let base = dimension_of("m");
    let derived = dimension_of("one");
    assert_ne!(base.key(), derived.key(), "L and N are distinct dimensions");
    let Some(NominalPayload::Dimension(base_payload)) = base.nominal_payload() else {
        panic!("the base dimension has a nominal dimension payload");
    };
    let Some(NominalPayload::Dimension(derived_payload)) = derived.nominal_payload() else {
        panic!("the derived dimension has a nominal dimension payload");
    };
    assert!(base_payload.is_base(), "L remains a base dimension");
    assert!(
        !derived_payload.is_base(),
        "N remains derived after L/L cancels"
    );
    assert!(
        derived_payload.terms().is_empty(),
        "L/L normalizes to no terms"
    );
}
