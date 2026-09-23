// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-180 K5: resolves a
//! `forms::TypeForm` -- S2's syntactic declared type, with no semantic
//! identity (ADR-011 §1's stage table; §2.2 row E2) -- to the kernel
//! `ValueType` at E3 (ADR-013 O-14/C-26: "Checked type node -> kernel
//! `ValueType` (QSL checker)"). This is the same E2/E3 split
//! `forms::syntax`'s own `Accumulate::accumulator_type`/`Count::result_type`/
//! `Sum::result_type` already draw with a qualified-name `String`, resolved
//! at E3 by [`resolve_named_type`] -- the exact function
//! [`resolve_type_form`]'s own [`TypeFormHead::Name`] arm calls for every
//! name that is not the reserved `"Population"` spelling (see that arm's own
//! doc).
//!
//! An unresolvable form is refused through this crate's existing check
//! diagnostics (`CheckCause::MissingName`/`AmbiguousName`, or
//! `CheckCause::IllTyped(IllTypedCause::TypeMismatch)` for a malformed
//! bound, argument count or keyword), never a new catalog code.

use super::check::Scope;
use super::refusal::{CheckCause, CheckRefusal, Location};
use crate::forms::{TypeForm, TypeFormHead};
use crate::value::collection::CollectionType;
use crate::value::comparison::IllTypedCause;
use crate::value::composite::ValueType;
use crate::value::decimal::DecimalType;
use crate::value::rational::RationalDomain;
use crate::value::text::{TextProfile, TextType};
use qsl_cst::token::Kind;
use quire_exact::{CardinalityBound, IeeeWidth, Integer, IntegerInterval, RoundingMode};

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
/// composites, enums and (QSL-180 K5) object types: an alias, a record or
/// tuple, an enum, or a model object type. The last is new here --
/// `allInstances<T>(p)`/`lookup<T>(p, r)` (FR-153) name their queried type
/// `T` this way (ADR-013 O-11: "the parsed-forms stage produces qualified
/// names, and the check stage resolves them... name -> node id by the
/// checker only") -- resolving to `ValueType::Reference`, exactly as a
/// record or tuple name resolves to `ValueType::Composite`.
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

fn bound_integer(
    form: &TypeForm,
    index: usize,
    location: &Location,
) -> Result<Integer, CheckRefusal> {
    let text = form.bounds.get(index).ok_or_else(|| mismatch(location))?;
    parse_integer(text, location)
}

fn bound_u64(form: &TypeForm, index: usize, location: &Location) -> Result<u64, CheckRefusal> {
    let text = form.bounds.get(index).ok_or_else(|| mismatch(location))?;
    parse_u64(text, location)
}

fn parse_rounding_mode(text: &str, location: &Location) -> Result<RoundingMode, CheckRefusal> {
    match text {
        "Exact" => Ok(RoundingMode::Exact),
        "TowardZero" => Ok(RoundingMode::TowardZero),
        "TowardPositive" => Ok(RoundingMode::TowardPositive),
        "TowardNegative" => Ok(RoundingMode::TowardNegative),
        "NearestEven" => Ok(RoundingMode::NearestEven),
        "NearestAway" => Ok(RoundingMode::NearestAway),
        _ => Err(mismatch(location)),
    }
}

fn parse_text_profile(text: &str, location: &Location) -> Result<TextProfile, CheckRefusal> {
    match text {
        "UnicodeScalars" => Ok(TextProfile::UnicodeScalars),
        "Nfc" => Ok(TextProfile::Nfc),
        "Nfd" => Ok(TextProfile::Nfd),
        "Nfkc" => Ok(TextProfile::Nfkc),
        "Nfkd" => Ok(TextProfile::Nfkd),
        "BinaryUtf8" => Ok(TextProfile::BinaryUtf8),
        _ => Err(mismatch(location)),
    }
}

/// Resolve one builtin keyword type. `form` supplies the keyword's own
/// arguments/bounds; `kind` is `form.head`'s already-destructured
/// [`TypeFormHead::Keyword`] payload.
fn resolve_keyword(
    scope: &Scope,
    kind: &Kind,
    form: &TypeForm,
    location: &Location,
) -> Result<ValueType, CheckRefusal> {
    match kind {
        Kind::BooleanType => Ok(ValueType::Boolean),
        Kind::IntegerType => Ok(ValueType::Integer),
        Kind::IntType => {
            let lower = bound_integer(form, 0, location)?;
            let upper = bound_integer(form, 1, location)?;
            IntegerInterval::new(lower, upper)
                .map(ValueType::Int)
                .map_err(|_| mismatch(location))
        }
        Kind::RationalType => {
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
        Kind::DecimalType => {
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
        Kind::Float32Type => Ok(ValueType::Float(IeeeWidth::Binary32)),
        Kind::Float64Type => Ok(ValueType::Float(IeeeWidth::Binary64)),
        Kind::TextType => {
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
        Kind::OptionType => {
            let payload = form.arguments.first().ok_or_else(|| mismatch(location))?;
            resolve_type_form(scope, payload, location).map(ValueType::option)
        }
        Kind::ReferenceType => {
            let argument = form.arguments.first().ok_or_else(|| mismatch(location))?;
            let TypeFormHead::Name(name) = &argument.head else {
                return Err(mismatch(location));
            };
            scope
                .types
                .object_types()
                .find(|declaration| declaration.name() == name)
                .map(|declaration| ValueType::Reference(declaration.key()))
                .ok_or_else(|| missing(name, location))
        }
        _ => Err(mismatch(location)),
    }
}

/// Resolve a syntactic `TypeForm` (S2) to the kernel `ValueType` (E3,
/// ADR-013 O-14/C-26). `check::check::Typer::resolve_type` is this
/// function's one production caller.
///
/// [`TypeFormHead::Name`]'s `"Population"` spelling is reserved: FR-153's
/// `Population<T>[N]` parameter type has no kernel `T` (`ValueType::
/// Population` carries only the declared maximum `N` -- `allInstances<T>(p)`/
/// `lookup<T>(p, r)` name `T` separately, at their own call site, per
/// FR-153's own table), so `T` here is carried as syntax
/// (`form.arguments`) but never resolved or read back: only the one bound
/// literal `N` is.
pub(crate) fn resolve_type_form(
    scope: &Scope,
    form: &TypeForm,
    location: &Location,
) -> Result<ValueType, CheckRefusal> {
    match &form.head {
        TypeFormHead::Name(name) if name == "Population" => {
            let maximum = bound_u64(form, 0, location)?;
            Ok(ValueType::Population(maximum))
        }
        TypeFormHead::Name(name) => resolve_named_type(scope, name, location),
        TypeFormHead::Keyword(kind) => resolve_keyword(scope, kind, form, location),
        TypeFormHead::Collection(kind) => {
            let element = form.arguments.first().ok_or_else(|| mismatch(location))?;
            let element = resolve_type_form(scope, element, location)?;
            let minimum = bound_u64(form, 0, location)?;
            let maximum = bound_u64(form, 1, location)?;
            let bound = CardinalityBound::new(minimum, maximum).map_err(|_| mismatch(location))?;
            Ok(ValueType::collection(CollectionType::new(
                *kind, element, bound,
            )))
        }
    }
}
