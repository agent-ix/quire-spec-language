// SPDX-License-Identifier: AGPL-3.0-or-later
//! Resolves a `forms::TypeForm` -- S2's syntactic declared type, with no
//! semantic identity (ADR-011 §1's stage table; §2.2 row E2) -- to the
//! kernel `ValueType` at E3 (ADR-013 O-14/C-26: "Checked type node ->
//! kernel `ValueType` (QSL checker)").
//!
//! Bound literals are read in the grammar's own spellings (`qsl-cst`'s
//! `RoundingMode` and `TextProfile` productions: `nearest-even`,
//! `unicode-scalars`, `nfc`, ...). An unresolvable form is refused through
//! the existing check diagnostics (`CheckCause::MissingName`/
//! `AmbiguousName`, or `CheckCause::IllTyped(IllTypedCause::TypeMismatch)`
//! for a malformed bound or argument), never a new catalog code.

use super::check::Scope;
use super::refusal::{CheckCause, CheckRefusal, Location};
use crate::forms::{BuiltinType, TypeForm, TypeFormHead};
use crate::value::collection::CollectionType;
use crate::value::composite::ValueType;
use crate::value::decimal::DecimalType;
use crate::value::rational::RationalDomain;
use quire_exact::{
    CardinalityBound, IeeeWidth, IllTypedCause, Integer, IntegerInterval, RoundingMode,
    TextProfile, TextType,
};

fn mismatch(location: &Location) -> CheckRefusal {
    CheckRefusal::ill_typed(location, IllTypedCause::TypeMismatch)
}

fn missing(name: &str, location: &Location) -> CheckRefusal {
    CheckRefusal {
        location: location.clone(),
        cause: CheckCause::MissingName(name.to_owned()),
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
    let mut candidates: Vec<ValueType> = scope
        .aliases
        .iter()
        .filter(|(alias, _)| alias == name)
        .map(|(_, value_type)| value_type.clone())
        .collect();
    candidates.extend(
        scope
            .types
            .composites()
            .filter(|declaration| declaration.name() == name)
            .map(|declaration| ValueType::Composite(declaration.key())),
    );
    candidates.extend(
        scope
            .enums
            .iter()
            .filter(|binding| binding.name == name)
            .map(|binding| ValueType::Enum(binding.declaration.key())),
    );
    candidates.extend(
        scope
            .types
            .object_types()
            .filter(|declaration| declaration.name() == name)
            .map(|declaration| ValueType::Reference(declaration.key())),
    );
    match candidates.len() {
        0 => Err(missing(name, location)),
        1 => Ok(candidates.remove(0)),
        _ => Err(CheckRefusal {
            location: location.clone(),
            cause: CheckCause::AmbiguousName {
                name: name.to_owned(),
                loci: vec![location.clone()],
            },
        }),
    }
}

fn parse_integer(text: &str, location: &Location) -> Result<Integer, CheckRefusal> {
    text.parse().map_err(|_| mismatch(location))
}

fn parse_u64(text: &str, location: &Location) -> Result<u64, CheckRefusal> {
    text.parse().map_err(|_| mismatch(location))
}

fn bound_text<'f>(
    form: &'f TypeForm,
    index: usize,
    location: &Location,
) -> Result<&'f str, CheckRefusal> {
    form.bounds
        .get(index)
        .map(String::as_str)
        .ok_or_else(|| mismatch(location))
}

fn bound_integer(
    form: &TypeForm,
    index: usize,
    location: &Location,
) -> Result<Integer, CheckRefusal> {
    parse_integer(bound_text(form, index, location)?, location)
}

fn bound_u64(form: &TypeForm, index: usize, location: &Location) -> Result<u64, CheckRefusal> {
    parse_u64(bound_text(form, index, location)?, location)
}

/// A rounding mode as the grammar spells it (`qsl-cst` `RoundingMode`).
fn parse_rounding_mode(text: &str, location: &Location) -> Result<RoundingMode, CheckRefusal> {
    match text {
        "exact" => Ok(RoundingMode::Exact),
        "toward-zero" => Ok(RoundingMode::TowardZero),
        "toward-positive" => Ok(RoundingMode::TowardPositive),
        "toward-negative" => Ok(RoundingMode::TowardNegative),
        "nearest-even" => Ok(RoundingMode::NearestEven),
        "nearest-away" => Ok(RoundingMode::NearestAway),
        _ => Err(mismatch(location)),
    }
}

/// A text profile as the grammar spells it (`qsl-cst` `TextProfile`).
fn parse_text_profile(text: &str, location: &Location) -> Result<TextProfile, CheckRefusal> {
    match text {
        "unicode-scalars" => Ok(TextProfile::UnicodeScalars),
        "nfc" => Ok(TextProfile::Nfc),
        "nfd" => Ok(TextProfile::Nfd),
        "nfkc" => Ok(TextProfile::Nfkc),
        "nfkd" => Ok(TextProfile::Nfkd),
        "binary-utf8" => Ok(TextProfile::BinaryUtf8),
        _ => Err(mismatch(location)),
    }
}

fn only_argument<'f>(
    form: &'f TypeForm,
    location: &Location,
) -> Result<&'f TypeForm, CheckRefusal> {
    match form.arguments.as_slice() {
        [argument] => Ok(argument),
        _ => Err(mismatch(location)),
    }
}

fn resolve_builtin(
    scope: &Scope,
    builtin: BuiltinType,
    form: &TypeForm,
    location: &Location,
) -> Result<ValueType, CheckRefusal> {
    match builtin {
        BuiltinType::Boolean => Ok(ValueType::Boolean),
        BuiltinType::Integer => Ok(ValueType::Integer),
        BuiltinType::Int => {
            let lower = bound_integer(form, 0, location)?;
            let upper = bound_integer(form, 1, location)?;
            IntegerInterval::new(lower, upper)
                .map(ValueType::Int)
                .map_err(|_| mismatch(location))
        }
        BuiltinType::Rational => {
            let numerator = IntegerInterval::new(
                bound_integer(form, 0, location)?,
                bound_integer(form, 1, location)?,
            )
            .map_err(|_| mismatch(location))?;
            let denominator = IntegerInterval::new(
                bound_integer(form, 2, location)?,
                bound_integer(form, 3, location)?,
            )
            .map_err(|_| mismatch(location))?;
            RationalDomain::new(numerator, denominator)
                .map(ValueType::Rational)
                .map_err(|_| mismatch(location))
        }
        BuiltinType::Decimal => {
            let lower = bound_integer(form, 0, location)?;
            let upper = bound_integer(form, 1, location)?;
            let min_scale = bound_u64(form, 2, location)?;
            let max_scale = bound_u64(form, 3, location)?;
            let rounding = match form.bounds.get(4) {
                Some(text) => parse_rounding_mode(text, location)?,
                None => RoundingMode::default(),
            };
            DecimalType::new(lower, upper, min_scale, max_scale, rounding)
                .map(ValueType::Decimal)
                .map_err(|_| mismatch(location))
        }
        BuiltinType::Float32 => Ok(ValueType::Float(IeeeWidth::Binary32)),
        BuiltinType::Float64 => Ok(ValueType::Float(IeeeWidth::Binary64)),
        BuiltinType::Text => {
            let min = bound_u64(form, 0, location)?;
            let max = bound_u64(form, 1, location)?;
            let profile = match form.bounds.get(2) {
                Some(text) => parse_text_profile(text, location)?,
                None => TextProfile::UnicodeScalars,
            };
            TextType::new(min, max, profile)
                .map(ValueType::Text)
                .map_err(|_| mismatch(location))
        }
        BuiltinType::Option => resolve_type_form(scope, only_argument(form, location)?, location)
            .map(ValueType::option),
        BuiltinType::Reference => {
            let TypeFormHead::Name(name) = &only_argument(form, location)?.head else {
                return Err(mismatch(location));
            };
            scope
                .types
                .object_types()
                .find(|declaration| declaration.name() == name)
                .map(|declaration| ValueType::Reference(declaration.key()))
                .ok_or_else(|| missing(name, location))
        }
    }
}

/// Resolve a syntactic `TypeForm` (S2) to the kernel `ValueType` (E3,
/// ADR-013 O-14/C-26).
///
/// `Population<T>[N]` (FR-153) resolves to `ValueType::Population(N)`: the
/// kernel type carries only the declared maximum, and `allInstances<T>(p)`/
/// `lookup<T>(p, r)` name `T` at their own call site, so `T` is carried as
/// syntax but not resolved here.
pub(crate) fn resolve_type_form(
    scope: &Scope,
    form: &TypeForm,
    location: &Location,
) -> Result<ValueType, CheckRefusal> {
    match &form.head {
        TypeFormHead::Builtin(builtin) => resolve_builtin(scope, *builtin, form, location),
        TypeFormHead::Collection(kind) => {
            let element = resolve_type_form(scope, only_argument(form, location)?, location)?;
            let minimum = bound_u64(form, 0, location)?;
            let maximum = bound_u64(form, 1, location)?;
            let bound = CardinalityBound::new(minimum, maximum).map_err(|_| mismatch(location))?;
            Ok(ValueType::collection(CollectionType::new(
                *kind, element, bound,
            )))
        }
        TypeFormHead::Population => Ok(ValueType::Population(bound_u64(form, 0, location)?)),
        TypeFormHead::Name(name) => resolve_named_type(scope, name, location),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::family::checking_tests::{empty_scope, root_location};
    use crate::value::declaration::{ObjectTypeDeclaration, TypeEnvironment};
    use quire_exact::{CollectionKind, NodeKey};

    const SPAN: qsl_foundation::Span = qsl_foundation::Span { start: 0, end: 0 };

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

    fn scope_with_object_type(name: &str) -> Scope {
        let mut scope = empty_scope();
        scope.types = TypeEnvironment::new(
            [],
            [ObjectTypeDeclaration::new(
                NodeKey::from_digest([7; 32]),
                name,
                Vec::new(),
            )],
        )
        .expect("one object type admits");
        scope
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
        let scope = scope_with_object_type("M::A");
        assert_eq!(
            resolve_in(&scope, &reference("M::A")).expect("M::A is declared"),
            ValueType::Reference(NodeKey::from_digest([7; 32]))
        );
        let refusal = resolve_in(&scope, &reference("M::B")).unwrap_err();
        assert!(matches!(refusal.cause, CheckCause::MissingName(name) if name == "M::B"));
    }

    /// A name bound by both an alias and an object type is ambiguous; a
    /// name bound by nothing is missing.
    #[test]
    fn names_resolve_uniquely_or_are_refused() {
        let mut scope = scope_with_object_type("X");
        scope.aliases.push(("X".to_owned(), ValueType::Boolean));
        let refusal = resolve_in(&scope, &TypeForm::name("X", SPAN)).unwrap_err();
        assert!(matches!(refusal.cause, CheckCause::AmbiguousName { name, .. } if name == "X"));
        let refusal = resolve(&TypeForm::name("Y", SPAN)).unwrap_err();
        assert!(matches!(refusal.cause, CheckCause::MissingName(name) if name == "Y"));
    }

    /// `Population<T>[N]` has its own head, so a user type named
    /// `Population` resolves as that user type, not as a population.
    #[test]
    fn a_user_type_named_population_is_not_shadowed() {
        let mut scope = empty_scope();
        scope
            .aliases
            .push(("Population".to_owned(), ValueType::Boolean));
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
            ValueType::Population(3)
        );
    }
}
