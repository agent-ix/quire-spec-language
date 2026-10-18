// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-380 "The declaration": the `StateModel` family's S2 production for
//! `bound L on M::p [N]`, run on real complete-V1 source through
//! `qsl_cst::parse` and `build_unit` (TC-1054 row 1).

use ix_trace_rs::trace;
use qsl_forms::{build_unit, DeclarationForm, ParsedUnit, PopulationBoundForm};
use qsl_foundation::{SourceIdentity, Span};

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n\
    model Config = \"example/config-version\" version \"1\" digest \
    \"sha256-jcs:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n";

fn parse(declarations: &str) -> (String, qsl_cst::ParsedSource) {
    let text = format!("{HEADER}{declarations}\n");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    (text, parsed)
}

fn build(declarations: &str) -> (String, ParsedUnit) {
    let (text, parsed) = parse(declarations);
    assert!(
        parsed.is_admissible(),
        "{declarations}: {:?}",
        parsed.diagnostics()
    );
    let unit = build_unit(&parsed).unwrap_or_else(|failure| panic!("{declarations}: {failure:?}"));
    (text, unit)
}

fn population_bound(form: &DeclarationForm) -> &PopulationBoundForm {
    match form {
        DeclarationForm::PopulationBound(bound) => bound,
        other => panic!("a population bound form, not {other:?}"),
    }
}

/// one form, its label, alias, population and digits each
/// spelled and spanned, nothing resolved.
#[trace("TC-1054", "FR-380-AC-1")]
#[test]
fn a_bound_builds_one_form_with_every_part_spelled_and_spanned() {
    let (text, unit) = build("bound HistoryBound on Config::config_history [8]");
    let [parsed] = unit.forms() else {
        panic!("one declaration builds one form");
    };
    let form = population_bound(parsed.form());
    let slice = |span: Span| &text[span.start..span.end];
    assert_eq!(form.label.name, "HistoryBound");
    assert_eq!(slice(form.label.span), "HistoryBound");
    assert_eq!(form.model.name, "Config");
    assert_eq!(slice(form.model.span), "Config");
    assert_eq!(form.population.name, "config_history");
    assert_eq!(slice(form.population.span), "config_history");
    assert_eq!(form.maximum.name, "8");
    assert_eq!(slice(form.maximum.span), "8");
    assert_eq!(slice(form.bracketed), "[8]");
    assert_eq!(
        slice(form.span),
        "bound HistoryBound on Config::config_history [8]"
    );
    assert_eq!(slice(parsed.span()), slice(form.span));
    // `bound <label>` is no `verify` step's `bound <extent>`.
    assert_eq!(parsed.declared_extent(), None);
}

/// `0` is a valid maximum, and bounds keep their source order.
#[trace("TC-1054", "FR-380-AC-1")]
#[test]
fn a_zero_bound_builds_and_bounds_keep_source_order() {
    let (_, unit) = build(
        "bound Zero on Config::empty [0]\n\
         bound Eight on Config::config_history [8]",
    );
    let bounds: Vec<(&str, &str)> = unit
        .forms()
        .iter()
        .map(|form| {
            let bound = population_bound(form.form());
            (bound.label.name.as_str(), bound.maximum.name.as_str())
        })
        .collect();
    assert_eq!(bounds, [("Zero", "0"), ("Eight", "8")]);
}

/// a negative or non-numeric maximum refuses at S1, located at
/// the offending token.
#[trace("TC-1054", "FR-380-AC-1")]
#[test]
fn a_negative_or_non_numeric_maximum_refuses_at_s1() {
    for (declaration, offender) in [
        ("bound B on Config::p [-1]", "-1]"),
        ("bound B on Config::p [x]", "x]"),
    ] {
        let (text, parsed) = parse(declaration);
        assert!(!parsed.is_admissible(), "{declaration} must refuse");
        let at = text.find(offender).expect("the offending token");
        let first = parsed.diagnostics().first().expect("a diagnostic");
        assert_eq!(
            first.byte_span().expect("located").start,
            at,
            "{declaration}"
        );
    }
}

/// `bound` heads a declaration only; the word stays usable as a
/// member name in an expression.
#[trace("TC-1054", "FR-380-AC-1")]
#[test]
fn bound_stays_a_member_name_in_an_expression() {
    let (_, unit) = build(
        "function f using v(b: Config::Thing): Integer pure { b.bound }\n\
         bound B on Config::p [3]",
    );
    assert_eq!(unit.forms().len(), 2);
    assert!(matches!(
        unit.forms()[1].form(),
        DeclarationForm::PopulationBound(_)
    ));
}
