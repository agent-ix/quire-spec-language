// SPDX-License-Identifier: AGPL-3.0-or-later
//! Source-level regressions for normalized dimension maps.

use crate::check::PackageDeclarations;
use ix_trace_rs::trace;
use qsl_forms::build_unit;
use qsl_foundation::SourceIdentity;

/// A derived `L / L` dimension keeps its derived identity when its normalized
/// map is empty, while the base dimension remains non-dimensionless.
#[trace("FR-091-AC-35", "FR-091-AC-45", "QSpec-TC-187", "QSL-1738-AC-1")]
#[test]
fn source_assembly_keeps_empty_derived_dimension_dimensionless() {
    let source = concat!(
        "language \"ix:native\" edition \"1-draft\";\n",
        "profile v = \"quire.value.complete/v1\";\n",
        "dimension L;\n",
        "dimension N = L / L;\n",
        "unit m : L = rational(1, 1);\n",
        "unit one : N = rational(1, 1);\n",
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
}
