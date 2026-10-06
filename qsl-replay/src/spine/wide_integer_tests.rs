// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-091-AC-36 to AC-38 (TC-909): integer bounds and literals up to i128
//! check, the directly negated 2^127 folds to the one literal `i128::MIN`,
//! and every other integer outside i128 refuses `IntegerOutsideI128`. Each
//! case compiles real source through the spine.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_foundation::SourceIdentity;
use qsl_semantics::check::{
    AssemblyCause, CheckCause, I128Limit, IntegerOutsideI128, IntegerSite, TypeFormFault,
};
use quire_exact::{Integer, IntegerInterval, ValueType};

use super::{compose, CompileRefusal, ComposedUnit, DependencyInput, SpineLimits, SpineStage};

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
                      profile v = \"quire.value.complete/v1\";\n";

const TWO_POW_127: &str = "170141183460469231731687303715884105728";
const I128_MAX: &str = "170141183460469231731687303715884105727";
const I128_MIN: &str = "-170141183460469231731687303715884105728";
/// FR-092 vector L7: the folded `i128::MIN` literal.
const L7_KEY: &str = "c3aa30b239d70dea497badfb4fd426fb7b57e441aeb22d2eb450a19ad6617c5b";

/// `function u using v(x: <parameter>): Boolean pure { <body> }`.
fn unit(parameter: &str, body: &str) -> String {
    format!("{HEADER}function u using v(x: {parameter}): Boolean pure {{ {body} }}\n")
}

fn compile(source: &str) -> Result<ComposedUnit, Box<CompileRefusal>> {
    compose(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        source.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
        SpineLimits::default(),
    )
}

fn int(lower: impl Into<Integer>, upper: impl Into<Integer>) -> ValueType {
    ValueType::Int(IntegerInterval::new(lower.into(), upper.into()).expect("a nonempty interval"))
}

fn big(text: &str) -> Integer {
    text.parse().expect("a canonical integer")
}

/// The type of `u`'s parameter `x` in `compiled`.
fn parameter_type(compiled: &ComposedUnit) -> ValueType {
    compiled
        .package
        .graph()
        .callable("u")
        .expect("`u` is declared")
        .parameters[0]
        .1
        .clone()
}

/// The text `refusal`'s region covers in `source`.
fn covered<'a>(source: &'a str, refusal: &CompileRefusal) -> &'a str {
    let region = refusal.region().expect("the refusal is located");
    let start = usize::try_from(region.start()).unwrap();
    let end = usize::try_from(region.end()).unwrap();
    &source[start..end]
}

/// The one assembler error of `refusal`: its cause and the text it covers.
fn assembly_error<'a>(source: &'a str, refusal: &CompileRefusal) -> (AssemblyCause, &'a str) {
    let CompileRefusal::Assembly { refusal, .. } = refusal else {
        panic!("not an assembler refusal: {refusal}");
    };
    let [error] = refusal.errors.as_slice() else {
        panic!("not one assembler error: {:?}", refusal.errors);
    };
    let start = usize::try_from(error.span.start).unwrap();
    let end = usize::try_from(error.span.end).unwrap();
    (error.cause.clone(), &source[start..end])
}

/// FR-091-AC-36 (TC-909 steps 1 and 2): bounds across the whole i128 range
/// resolve to `ValueType::Int` with exactly the written bounds.
#[trace("FR-091-AC-36", "TC-909")]
#[test]
fn bounds_up_to_i128_resolve_exactly() {
    let cases = [
        (
            "Int[0, 18446744073709551615]",
            "x >= 0",
            int(0_i64, u64::MAX),
        ),
        (
            "Int[0, 9223372036854775808]",
            "x >= 0",
            int(0_i64, 9_223_372_036_854_775_808_u64),
        ),
        (
            "Int[0, 18446744073709551616]",
            "x >= 0",
            int(0_i64, 18_446_744_073_709_551_616_i128),
        ),
        (
            "Int[-170141183460469231731687303715884105728, 170141183460469231731687303715884105727]",
            "x <= 170141183460469231731687303715884105727",
            int(i128::MIN, i128::MAX),
        ),
    ];
    for (parameter, body, expected) in cases {
        let compiled = compile(&unit(parameter, body))
            .unwrap_or_else(|refusal| panic!("{parameter}: {refusal}"));
        assert_eq!(parameter_type(&compiled), expected, "{parameter}");
    }
}

/// FR-091-AC-37 (TC-909 step 3): a bound one past either i128 limit refuses
/// `IntegerOutsideI128` at the type form, `ill_typed`/`type-mismatch`.
#[trace("FR-091-AC-37", "TC-909")]
#[test]
fn a_bound_outside_i128_refuses_at_its_type_form() {
    let cases = [
        (
            format!("Int[0, {TWO_POW_127}]"),
            IntegerSite::Upper,
            TWO_POW_127.to_owned(),
            I128Limit::Max,
        ),
        (
            "Int[-170141183460469231731687303715884105729, 0]".to_owned(),
            IntegerSite::Lower,
            "-170141183460469231731687303715884105729".to_owned(),
            I128Limit::Min,
        ),
    ];
    for (parameter, site, value, limit) in cases {
        let source = unit(&parameter, "x >= 0");
        let refusal = compile(&source).expect_err("the bound is outside i128");
        let (cause, covered) = assembly_error(&source, &refusal);
        assert_eq!(
            cause,
            AssemblyCause::IllFormedBounds(TypeFormFault::IntegerOutsideI128(IntegerOutsideI128 {
                site,
                value,
                limit,
            }))
        );
        assert_eq!(covered, parameter);
        assert_eq!(refusal.stage(), SpineStage::Assembly);
        let catalog = match &refusal.as_ref() {
            CompileRefusal::Assembly { refusal, .. } => refusal.errors[0].cause.catalog_code(),
            _ => unreachable!("asserted above"),
        };
        assert_eq!(catalog.to_string(), "ill_typed/type-mismatch");
    }
}

/// The literals of the compiled package, as `(value, key)`.
fn integer_literals(compiled: &ComposedUnit) -> Vec<(String, String)> {
    compiled
        .package
        .graph()
        .semantic_graph()
        .nodes()
        .filter(|node| node.semantic_form() == "literal")
        .filter_map(|node| {
            let preimage: serde_json::Value = serde_json::from_slice(node.preimage()).ok()?;
            (preimage["body"]["value_kind"] == "integer").then(|| {
                (
                    preimage["body"]["value"].as_str().unwrap().to_owned(),
                    node.key().to_string(),
                )
            })
        })
        .collect()
}

/// Whether the compiled package holds a negate application.
fn has_negate(compiled: &ComposedUnit) -> bool {
    compiled
        .package
        .graph()
        .semantic_graph()
        .nodes()
        .any(|node| {
            let preimage = String::from_utf8_lossy(node.preimage());
            preimage.contains("quire.op.integer.negate")
        })
}

/// FR-091-AC-38 (TC-909 steps 4 and 5): a directly negated 2^127 is the one
/// literal `i128::MIN`, keyed as L7 with no negation; `-(2^127 - 1)` keeps
/// its `Negate`.
#[trace("FR-091-AC-38", "TC-909")]
#[test]
fn a_directly_negated_two_pow_127_is_one_literal() {
    let parameter = format!("Int[{I128_MIN}, 0]");
    for body in [
        format!("x >= {I128_MIN}"),
        // Only layout stands between the `-` and the literal.
        format!("x >= - {TWO_POW_127}"),
        format!("x >= -/* the minimum */{TWO_POW_127}"),
    ] {
        let compiled =
            compile(&unit(&parameter, &body)).unwrap_or_else(|refusal| panic!("{body}: {refusal}"));
        assert!(!has_negate(&compiled), "{body}: no Negate node");
        assert!(
            integer_literals(&compiled).contains(&(I128_MIN.to_owned(), L7_KEY.to_owned())),
            "{body}: the literal is L7"
        );
    }

    let compiled = compile(&unit(&parameter, "x >= -170141183460469231731687303715884105727"))
        .expect("2^127 - 1 negated checks");
    assert!(has_negate(&compiled), "a negation of 2^127 - 1 stays a Negate");
    assert!(integer_literals(&compiled).iter().any(|(value, _)| value == I128_MAX));
}

/// FR-091-AC-38 (TC-909 step 6): every other integer outside i128 refuses
/// `IntegerOutsideI128` at the literal's span, site `literal`, the value as
/// written, limit `i128::MAX`, `ill_typed`/`type-mismatch`.
#[trace("FR-091-AC-38", "TC-909")]
#[test]
fn a_literal_outside_i128_refuses_at_the_literal() {
    let parameter = format!("Int[{I128_MIN}, 0]");
    let cases = [
        (format!("x < {TWO_POW_127}"), TWO_POW_127),
        (format!("x >= -({TWO_POW_127})"), TWO_POW_127),
        (format!("x >= - ({TWO_POW_127})"), TWO_POW_127),
        (
            "x >= -170141183460469231731687303715884105729".to_owned(),
            "170141183460469231731687303715884105729",
        ),
    ];
    for (body, literal) in cases {
        let source = unit(&parameter, &body);
        let refusal = compile(&source).expect_err("the literal is outside i128");
        let CompileRefusal::Check { refusals, .. } = refusal.as_ref() else {
            panic!("{body}: not a check refusal: {refusal}");
        };
        let [only] = refusals.as_slice() else {
            panic!("{body}: not one refusal: {refusals:?}");
        };
        assert_eq!(
            only.cause,
            CheckCause::IntegerOutsideI128(IntegerOutsideI128 {
                site: IntegerSite::Literal,
                value: literal.to_owned(),
                limit: I128Limit::Max,
            }),
            "{body}"
        );
        assert_eq!(covered(&source, &refusal), literal, "{body}");
        assert_eq!(refusal.code().as_str(), "ill_typed");
        assert_eq!(refusal.cause(), Some("type-mismatch"));
    }
}
