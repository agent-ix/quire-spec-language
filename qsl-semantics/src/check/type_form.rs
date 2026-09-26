// SPDX-License-Identifier: AGPL-3.0-or-later
//! Resolves a `qsl_forms::TypeForm` -- S2's syntactic declared type, with no
//! semantic identity (ADR-011 §1's stage table; §2.2 row E2) -- to the
//! kernel `ValueType` at E3 (ADR-013 O-14/C-26: "Checked type node ->
//! kernel `ValueType` (QSL checker)").
//!
//! Bound literals are read in the grammar's own spellings (`qsl-cst`'s
//! `RoundingMode` and `TextProfile` productions: `nearest-even`,
//! `unicode-scalars`, `nfc`, ...). Resolution reports a [`TypeFormError`]:
//! the typed [`TypeFormFault`] and the span of the type form it concerns.
//! The FR-091 assembler reports that as its own error; every other caller
//! goes through [`resolve_type_form`], which refuses through the existing
//! check diagnostics (`CheckCause::MissingName`/`AmbiguousName`, or
//! `CheckCause::IllTyped(IllTypedCause::TypeMismatch)` for a malformed
//! bound or argument), never a new catalog code.
//!
//! Names resolve through [`TypeNames`]: the package [`Scope`], or the
//! assembler's own table of the unit's declarations while it is still
//! building the package.

use super::check::Scope;
use super::refusal::{CheckCause, CheckRefusal, Location};
use qsl_forms::{BuiltinType, TypeForm, TypeFormHead};
use qsl_foundation::Span;
use quire_exact::{
    CardinalityBound, CollectionType, DecimalType, EffectiveId, FloatType, IeeeWidth,
    IllTypedCause, Integer, IntegerInterval, RationalDomain, RoundingMode, TextProfile, TextType,
    ValueType,
};

/// What a qualified type name resolves against.
pub(crate) trait TypeNames {
    /// Every type `name` binds, in resolution order. Empty when it binds
    /// none; more than one is ambiguous.
    fn named_types(&self, name: &str) -> &[ValueType];

    /// The object type named `name`, by its effective identity.
    fn object_type_named(&self, name: &str) -> Option<EffectiveId>;
}

impl TypeNames for Scope {
    fn named_types(&self, name: &str) -> &[ValueType] {
        Scope::named_types(self, name)
    }

    fn object_type_named(&self, name: &str) -> Option<EffectiveId> {
        Scope::object_type_named(self, name)
    }
}

/// Why a type form did not resolve.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TypeFormFault {
    /// The qualified name binds no type.
    MissingName(String),
    /// The qualified name binds more than one type.
    AmbiguousName(String),
    /// A bound is missing, is not a number or a grammar spelling, or the
    /// form has the wrong number of type arguments.
    Malformed,
    /// An `Int` or `Rational` interval whose lower bound is above its
    /// upper bound (`IntegerInterval`'s own refusal).
    EmptyInterval,
    /// A `Rational` denominator interval that reaches below one
    /// (`RationalDomain`'s own refusal).
    DenominatorBelowOne,
    /// A `Decimal` whose bounds or scales are out of order
    /// (`DecimalType`'s own refusal).
    MalformedDecimal,
    /// A `Text` minimum above its maximum (`TextType`'s own refusal).
    EmptyTextBounds,
    /// A collection cardinality minimum above its maximum
    /// (`CardinalityBound`'s own refusal).
    EmptyCardinality,
}

/// A [`TypeFormFault`] and the span of the type form it concerns.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeFormError {
    /// Why the form did not resolve.
    pub fault: TypeFormFault,
    /// The span of the form (or nested argument form) it concerns.
    pub span: Span,
}

impl TypeFormError {
    /// This error as a check refusal at `location`.
    fn into_refusal(self, location: &Location) -> CheckRefusal {
        let cause = match self.fault {
            TypeFormFault::MissingName(name) => CheckCause::MissingName(name),
            TypeFormFault::AmbiguousName(name) => CheckCause::AmbiguousName {
                name,
                loci: vec![location.clone()],
            },
            TypeFormFault::Malformed
            | TypeFormFault::EmptyInterval
            | TypeFormFault::DenominatorBelowOne
            | TypeFormFault::MalformedDecimal
            | TypeFormFault::EmptyTextBounds
            | TypeFormFault::EmptyCardinality => {
                return CheckRefusal::ill_typed(location, IllTypedCause::TypeMismatch)
            }
        };
        CheckRefusal {
            location: location.clone(),
            cause,
        }
    }
}

fn fault(form: &TypeForm, fault: TypeFormFault) -> TypeFormError {
    TypeFormError {
        fault,
        span: form.span,
    }
}

fn malformed(form: &TypeForm) -> TypeFormError {
    fault(form, TypeFormFault::Malformed)
}

/// Resolve a qualified type name against `names`: an alias, a record or
/// tuple, an enum, or a model object type.
fn named_type(
    names: &impl TypeNames,
    name: &str,
    form: &TypeForm,
) -> Result<ValueType, TypeFormError> {
    match names.named_types(name) {
        [] => Err(fault(form, TypeFormFault::MissingName(name.to_owned()))),
        [value_type] => Ok(value_type.clone()),
        _ => Err(fault(form, TypeFormFault::AmbiguousName(name.to_owned()))),
    }
}

/// Resolve a qualified type name against `scope`'s declared aliases,
/// composites, enums and object types: an alias, a record or tuple, an
/// enum, or a model object type. `allInstances<T>(p)`/`lookup<T>(p, r)`
/// (FR-153) name their queried type `T` this way (ADR-013 O-11: "name ->
/// node id by the checker only"), resolving to `ValueType::Reference`.
pub(crate) fn resolve_named_type(
    scope: &Scope,
    name: &str,
    location: &Location,
) -> Result<ValueType, CheckRefusal> {
    let form = TypeForm::name(name, Span { start: 0, end: 0 });
    named_type(scope, name, &form).map_err(|error| error.into_refusal(location))
}

fn bound_text(form: &TypeForm, index: usize) -> Result<&str, TypeFormError> {
    form.bounds
        .get(index)
        .map(String::as_str)
        .ok_or_else(|| malformed(form))
}

fn bound_integer(form: &TypeForm, index: usize) -> Result<Integer, TypeFormError> {
    bound_text(form, index)?
        .parse()
        .map_err(|_| malformed(form))
}

fn bound_u64(form: &TypeForm, index: usize) -> Result<u64, TypeFormError> {
    bound_text(form, index)?
        .parse()
        .map_err(|_| malformed(form))
}

/// A rounding mode as the grammar spells it (`qsl-cst` `RoundingMode`).
pub(crate) fn parse_rounding_mode(text: &str) -> Option<RoundingMode> {
    match text {
        "exact" => Some(RoundingMode::Exact),
        "toward-zero" => Some(RoundingMode::TowardZero),
        "toward-positive" => Some(RoundingMode::TowardPositive),
        "toward-negative" => Some(RoundingMode::TowardNegative),
        "nearest-even" => Some(RoundingMode::NearestEven),
        "nearest-away" => Some(RoundingMode::NearestAway),
        _ => None,
    }
}

/// A text profile as the grammar spells it (`qsl-cst` `TextProfile`).
fn parse_text_profile(text: &str) -> Option<TextProfile> {
    match text {
        "unicode-scalars" => Some(TextProfile::UnicodeScalars),
        "nfc" => Some(TextProfile::Nfc),
        "nfd" => Some(TextProfile::Nfd),
        "nfkc" => Some(TextProfile::Nfkc),
        "nfkd" => Some(TextProfile::Nfkd),
        "binary-utf8" => Some(TextProfile::BinaryUtf8),
        _ => None,
    }
}

fn only_argument(form: &TypeForm) -> Result<&TypeForm, TypeFormError> {
    match form.arguments.as_slice() {
        [argument] => Ok(argument),
        _ => Err(malformed(form)),
    }
}

fn interval(form: &TypeForm, lower: usize) -> Result<IntegerInterval, TypeFormError> {
    IntegerInterval::new(bound_integer(form, lower)?, bound_integer(form, lower + 1)?)
        .map_err(|_| fault(form, TypeFormFault::EmptyInterval))
}

/// `Float32[mode]`/`Float64[mode]`; the bare spelling is strict `exact`
/// (FR-091-OQ-4).
fn float_type(form: &TypeForm, width: IeeeWidth) -> Result<ValueType, TypeFormError> {
    let rounding = match form.bounds.first() {
        Some(text) => parse_rounding_mode(text).ok_or_else(|| malformed(form))?,
        None => RoundingMode::Exact,
    };
    Ok(ValueType::Float(FloatType::new(width, rounding)))
}

fn resolve_builtin(
    names: &impl TypeNames,
    builtin: BuiltinType,
    form: &TypeForm,
) -> Result<ValueType, TypeFormError> {
    match builtin {
        BuiltinType::Boolean => Ok(ValueType::Boolean),
        BuiltinType::Integer => Ok(ValueType::Integer),
        BuiltinType::Int => interval(form, 0).map(ValueType::Int),
        BuiltinType::Rational => RationalDomain::new(interval(form, 0)?, interval(form, 2)?)
            .map(ValueType::Rational)
            .map_err(|_| fault(form, TypeFormFault::DenominatorBelowOne)),
        BuiltinType::Decimal => {
            let lower = bound_integer(form, 0)?;
            let upper = bound_integer(form, 1)?;
            let min_scale = bound_u64(form, 2)?;
            let max_scale = bound_u64(form, 3)?;
            let rounding = match form.bounds.get(4) {
                Some(text) => parse_rounding_mode(text).ok_or_else(|| malformed(form))?,
                None => RoundingMode::default(),
            };
            DecimalType::new(lower, upper, min_scale, max_scale, rounding)
                .map(ValueType::Decimal)
                .map_err(|_| fault(form, TypeFormFault::MalformedDecimal))
        }
        BuiltinType::Float32 => float_type(form, IeeeWidth::Binary32),
        BuiltinType::Float64 => float_type(form, IeeeWidth::Binary64),
        BuiltinType::Text => {
            let min = bound_u64(form, 0)?;
            let max = bound_u64(form, 1)?;
            let profile = match form.bounds.get(2) {
                Some(text) => parse_text_profile(text).ok_or_else(|| malformed(form))?,
                None => TextProfile::UnicodeScalars,
            };
            TextType::new(min, max, profile)
                .map(ValueType::Text)
                .map_err(|_| fault(form, TypeFormFault::EmptyTextBounds))
        }
        BuiltinType::Option => resolve_form(names, only_argument(form)?).map(ValueType::option),
        BuiltinType::Reference => {
            let target = only_argument(form)?;
            let TypeFormHead::Name(name) = &target.head else {
                return Err(malformed(form));
            };
            names
                .object_type_named(name)
                .map(ValueType::Reference)
                .ok_or_else(|| fault(target, TypeFormFault::MissingName(name.clone())))
        }
    }
}

/// The object type `T` of a `Population<T>[N]` form, by its effective
/// identity, or `None` for any other form. FR-094 builds the population's
/// type node from `T`, which `ValueType::Population` does not carry; a `T`
/// that names no object type refuses `ill_typed`/`type-mismatch`.
pub(crate) fn population_target(
    scope: &Scope,
    form: &TypeForm,
    location: &Location,
) -> Result<Option<EffectiveId>, CheckRefusal> {
    if !matches!(form.head, TypeFormHead::Population) {
        return Ok(None);
    }
    let argument = only_argument(form).map_err(|error| error.into_refusal(location))?;
    match resolve_type_form(scope, argument, location)? {
        ValueType::Reference(target) => Ok(Some(target)),
        _ => Err(CheckRefusal::ill_typed(
            location,
            IllTypedCause::TypeMismatch,
        )),
    }
}

/// Resolve a syntactic `TypeForm` (S2) to the kernel `ValueType` (E3,
/// ADR-013 O-14/C-26) against `names`, reporting the fault and the span of
/// the form it concerns.
///
/// `Population<T>[N]` (FR-153) resolves to `ValueType::Population(N)`: the
/// kernel type carries only the declared maximum, and `allInstances<T>(p)`/
/// `lookup<T>(p, r)` name `T` at their own call site, so `T` is carried as
/// syntax but not resolved here.
pub(crate) fn resolve_form(
    names: &impl TypeNames,
    form: &TypeForm,
) -> Result<ValueType, TypeFormError> {
    match &form.head {
        TypeFormHead::Builtin(builtin) => resolve_builtin(names, *builtin, form),
        TypeFormHead::Collection(kind) => {
            let element = resolve_form(names, only_argument(form)?)?;
            let minimum = bound_u64(form, 0)?;
            let maximum = bound_u64(form, 1)?;
            let bound = CardinalityBound::new(minimum, maximum)
                .map_err(|_| fault(form, TypeFormFault::EmptyCardinality))?;
            Ok(ValueType::collection(CollectionType::new(
                *kind,
                element,
                Some(bound),
            )))
        }
        TypeFormHead::Population => Ok(ValueType::Population(Some(bound_u64(form, 0)?))),
        TypeFormHead::Name(name) => named_type(names, name, form),
    }
}

/// Resolve a syntactic `TypeForm` against the package `scope`, refusing at
/// `location` (see the module doc).
pub(crate) fn resolve_type_form(
    scope: &Scope,
    form: &TypeForm,
    location: &Location,
) -> Result<ValueType, CheckRefusal> {
    resolve_form(scope, form).map_err(|error| error.into_refusal(location))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::family::fixtures::{empty_scope, root_location, scope_with};
    use crate::value::declaration::{ObjectTypeDeclaration, TypeEnvironment};
    use quire_exact::{CollectionKind, EffectiveId};

    const SPAN: qsl_foundation::Span = qsl_foundation::Span { start: 0, end: 0 };

    /// FR-091-AC-11 (TC-398 step 3): the assembler's resolution `match` has
    /// no catch-all arm, neither `_` nor a bare binding.
    #[ix_trace_rs::trace("FR-091-AC-11", "TC-398")]
    #[test]
    fn resolve_form_has_no_catch_all_arm() {
        use syn::visit::Visit;
        struct Finder(Vec<usize>);
        impl<'ast> Visit<'ast> for Finder {
            fn visit_expr_match(&mut self, expression: &'ast syn::ExprMatch) {
                for arm in &expression.arms {
                    if matches!(arm.pat, syn::Pat::Wild(_) | syn::Pat::Ident(_)) {
                        self.0
                            .push(syn::spanned::Spanned::span(&arm.pat).start().line);
                    }
                }
                syn::visit::visit_expr_match(self, expression);
            }
        }
        let file = syn::parse_file(include_str!("type_form.rs")).expect("type_form.rs parses");
        let function = file
            .items
            .iter()
            .find_map(|item| match item {
                syn::Item::Fn(function) if function.sig.ident == "resolve_form" => Some(function),
                _ => None,
            })
            .expect("fn resolve_form exists");
        let mut finder = Finder(Vec::new());
        finder.visit_item_fn(function);
        assert!(
            finder.0.is_empty(),
            "resolve_form has a catch-all arm at lines {:?}",
            finder.0
        );
    }

    fn builtin(builtin: BuiltinType, bounds: &[&str]) -> TypeForm {
        TypeForm::builtin(builtin, SPAN)
            .with_bounds(bounds.iter().map(|bound| (*bound).to_owned()).collect())
    }

    fn resolve_in(scope: &Scope, form: &TypeForm) -> Result<ValueType, CheckRefusal> {
        resolve_type_form(scope, form, &root_location())
    }

    fn resolve(form: &TypeForm) -> Result<ValueType, CheckRefusal> {
        resolve_in(&empty_scope(), form)
    }

    fn assert_mismatch(form: &TypeForm) {
        let refusal = resolve(form).expect_err("the form is malformed");
        assert!(
            matches!(
                refusal.cause,
                CheckCause::IllTyped(IllTypedCause::TypeMismatch)
            ),
            "{form:?} refused with {:?}",
            refusal.cause
        );
    }

    fn object_type(name: &str) -> TypeEnvironment {
        TypeEnvironment::new(
            [],
            [ObjectTypeDeclaration::new(
                EffectiveId::from_digest([7; 32]),
                name,
                Vec::new(),
            )],
        )
        .expect("one object type admits")
    }

    /// FR-091-OQ-4 (TC-405): `Float32[mode]`/`Float64[mode]` resolve to a
    /// float type carrying the mode; a bare float is strict `exact`.
    #[ix_trace_rs::trace("FR-091-AC-19", "TC-405")]
    #[test]
    fn float_types_carry_their_rounding_mode() {
        let float = |form: &TypeForm| match resolve(form).expect("a float type resolves") {
            ValueType::Float(float) => (float.width(), float.rounding()),
            other => panic!("a float type, not {other:?}"),
        };
        assert_eq!(
            float(&builtin(BuiltinType::Float64, &["nearest-even"])),
            (IeeeWidth::Binary64, RoundingMode::NearestEven)
        );
        assert_eq!(
            float(&builtin(BuiltinType::Float32, &["toward-zero"])),
            (IeeeWidth::Binary32, RoundingMode::TowardZero)
        );
        assert_eq!(
            float(&builtin(BuiltinType::Float64, &[])),
            (IeeeWidth::Binary64, RoundingMode::Exact)
        );
        assert_mismatch(&builtin(BuiltinType::Float64, &["NearestEven"]));
    }

    /// Bound literals use the grammar's own spellings (`qsl-cst`
    /// `RoundingMode`, `TextProfile`); the Rust variant names are not
    /// language spellings and are refused.
    #[test]
    fn bounds_resolve_in_the_grammars_spellings_only() {
        let decimal = resolve(&builtin(
            BuiltinType::Decimal,
            &["0", "100", "0", "2", "nearest-even"],
        ))
        .expect("nearest-even is a grammar rounding mode");
        let ValueType::Decimal(decimal) = decimal else {
            panic!("a decimal type, not {decimal:?}");
        };
        assert_eq!(decimal.rounding(), RoundingMode::NearestEven);
        let text = resolve(&builtin(BuiltinType::Text, &["1", "5", "nfc"]))
            .expect("nfc is a grammar text profile");
        let ValueType::Text(text) = text else {
            panic!("a text type, not {text:?}");
        };
        assert_eq!(text.profile(), TextProfile::Nfc);
        assert_mismatch(&builtin(
            BuiltinType::Decimal,
            &["0", "100", "0", "2", "NearestEven"],
        ));
        assert_mismatch(&builtin(BuiltinType::Text, &["1", "5", "Nfc"]));
        assert_mismatch(&builtin(BuiltinType::Text, &["1", "5", "unicode"]));
    }

    /// Inverted, missing and non-numeric bounds are refused as a type
    /// mismatch, per builtin.
    #[test]
    fn malformed_bounds_are_refused() {
        assert_mismatch(&builtin(BuiltinType::Int, &["9", "0"]));
        assert_mismatch(&builtin(BuiltinType::Int, &["0"]));
        assert_mismatch(&builtin(BuiltinType::Int, &["zero", "9"]));
        assert_mismatch(&builtin(BuiltinType::Rational, &["0", "1", "0", "5"]));
        assert_mismatch(&builtin(BuiltinType::Decimal, &["0", "9", "3", "1"]));
        assert_mismatch(&builtin(BuiltinType::Text, &["5", "1", "nfc"]));
        assert_mismatch(&builtin(BuiltinType::Text, &["-1", "5"]));
        assert_mismatch(&TypeForm::new(TypeFormHead::Population, SPAN));
    }

    /// `Option`, a collection and `Reference` each take exactly one type
    /// argument; a collection also needs two cardinality bounds in order.
    #[test]
    fn argument_counts_and_cardinality_bounds_are_checked() {
        let integer = || TypeForm::builtin(BuiltinType::Integer, SPAN);
        assert_mismatch(&TypeForm::builtin(BuiltinType::Option, SPAN));
        assert_mismatch(
            &TypeForm::builtin(BuiltinType::Option, SPAN)
                .with_arguments(vec![integer(), integer()]),
        );
        let sequence = |arguments: Vec<TypeForm>, bounds: &[&str]| {
            TypeForm::collection(CollectionKind::Sequence, SPAN)
                .with_arguments(arguments)
                .with_bounds(bounds.iter().map(|bound| (*bound).to_owned()).collect())
        };
        assert_mismatch(&sequence(Vec::new(), &["0", "3"]));
        assert_mismatch(&sequence(vec![integer()], &["0"]));
        assert_mismatch(&sequence(vec![integer()], &["3", "1"]));
        assert!(resolve(&sequence(vec![integer()], &["0", "3"])).is_ok());
        assert_mismatch(&TypeForm::builtin(BuiltinType::Reference, SPAN));
        assert_mismatch(
            &TypeForm::builtin(BuiltinType::Reference, SPAN).with_arguments(vec![integer()]),
        );
    }

    /// `Reference<T>` names a declared object type; an undeclared one is a
    /// missing name.
    #[test]
    fn a_reference_to_an_undeclared_object_type_is_a_missing_name() {
        let reference = |name: &str| {
            TypeForm::builtin(BuiltinType::Reference, SPAN)
                .with_arguments(vec![TypeForm::name(name, SPAN)])
        };
        let scope = scope_with(object_type("M::A"), Vec::new());
        assert_eq!(
            resolve_in(&scope, &reference("M::A")).expect("M::A is declared"),
            ValueType::Reference(EffectiveId::from_digest([7; 32]))
        );
        let refusal = resolve_in(&scope, &reference("M::B")).unwrap_err();
        assert!(matches!(refusal.cause, CheckCause::MissingName(name) if name == "M::B"));
    }

    /// A name bound by both an alias and an object type is ambiguous; a
    /// name bound by nothing is missing.
    #[test]
    fn names_resolve_uniquely_or_are_refused() {
        let scope = scope_with(object_type("X"), vec![("X".to_owned(), ValueType::Boolean)]);
        let refusal = resolve_in(&scope, &TypeForm::name("X", SPAN)).unwrap_err();
        assert!(matches!(refusal.cause, CheckCause::AmbiguousName { name, .. } if name == "X"));
        let refusal = resolve(&TypeForm::name("Y", SPAN)).unwrap_err();
        assert!(matches!(refusal.cause, CheckCause::MissingName(name) if name == "Y"));
    }

    /// `Population<T>[N]` has its own head, so a user type named
    /// `Population` resolves as that user type, not as a population.
    #[test]
    fn a_user_type_named_population_is_not_shadowed() {
        let scope = scope_with(
            TypeEnvironment::default(),
            vec![("Population".to_owned(), ValueType::Boolean)],
        );
        assert_eq!(
            resolve_in(&scope, &TypeForm::name("Population", SPAN)).expect("the alias resolves"),
            ValueType::Boolean
        );
        assert_eq!(
            resolve_in(
                &scope,
                &TypeForm::new(TypeFormHead::Population, SPAN).with_bounds(vec!["3".to_owned()])
            )
            .expect("a population type resolves"),
            ValueType::Population(Some(3))
        );
    }
}
