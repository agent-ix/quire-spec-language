// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-180 K5 test support: encodes a plain `ValueType` shape as the
//! `forms::TypeForm` syntax `check::Typer::resolve_type` resolves back to
//! that exact `ValueType` -- the exact inverse of that resolution. Lets a
//! fixture keep describing its intended type by the `ValueType` shape it
//! wants, while still handing the checker only syntax (ADR-011 §1: S2 has
//! no semantic identity).
//!
//! `ValueType::Composite`/`Enum`/`Reference` carry only a `NodeKey` digest,
//! with no name to recover -- a fixture that wants one of those builds the
//! matching `TypeForm::name(name, SPAN)` directly, with the same name
//! string it used to derive that `NodeKey` (see each declared type's own
//! registration), rather than through [`type_form`] here.

use quire_spec_language::value::{CollectionType, IeeeWidth, TypeForm, ValueType};

/// A placeholder span: `TypeForm`'s own span carries no identity
/// (ADR-011 §2.2 row E2, "identity: none: forms carry position only"), so
/// every fixture built through this module uses the same one.
pub const SPAN: qsl_foundation::Span = qsl_foundation::Span { start: 0, end: 0 };

fn rounding_mode_text(mode: quire_exact::RoundingMode) -> &'static str {
    match mode {
        quire_exact::RoundingMode::Exact => "Exact",
        quire_exact::RoundingMode::TowardZero => "TowardZero",
        quire_exact::RoundingMode::TowardPositive => "TowardPositive",
        quire_exact::RoundingMode::TowardNegative => "TowardNegative",
        quire_exact::RoundingMode::NearestEven => "NearestEven",
        quire_exact::RoundingMode::NearestAway => "NearestAway",
    }
}

fn text_profile_text(profile: quire_spec_language::value::TextProfile) -> &'static str {
    use quire_spec_language::value::TextProfile;
    match profile {
        TextProfile::UnicodeScalars => "UnicodeScalars",
        TextProfile::Nfc => "Nfc",
        TextProfile::Nfd => "Nfd",
        TextProfile::Nfkc => "Nfkc",
        TextProfile::Nfkd => "Nfkd",
        TextProfile::BinaryUtf8 => "BinaryUtf8",
    }
}

fn collection_type_form(collection: &CollectionType) -> TypeForm {
    TypeForm::collection(collection.kind(), SPAN)
        .with_arguments(vec![type_form(collection.element())])
        .with_bounds(vec![
            collection.bound().minimum().to_string(),
            collection.bound().maximum().to_string(),
        ])
}

/// The exact inverse of `check::Typer::resolve_type` for every `ValueType`
/// shape that carries no name to recover (everything but
/// `Composite`/`Enum`/`Reference` -- see this module's own doc; those
/// panic).
pub fn type_form(value_type: &ValueType) -> TypeForm {
    match value_type {
        ValueType::Boolean => TypeForm::keyword(qsl_cst::token::Kind::BooleanType, SPAN),
        ValueType::Integer => TypeForm::keyword(qsl_cst::token::Kind::IntegerType, SPAN),
        ValueType::Int(interval) => TypeForm::keyword(qsl_cst::token::Kind::IntType, SPAN)
            .with_bounds(vec![
                interval.lower().to_string(),
                interval.upper().to_string(),
            ]),
        ValueType::Rational(domain) => TypeForm::keyword(qsl_cst::token::Kind::RationalType, SPAN)
            .with_bounds(vec![
                domain.numerator().lower().to_string(),
                domain.numerator().upper().to_string(),
                domain.denominator().lower().to_string(),
                domain.denominator().upper().to_string(),
            ]),
        ValueType::Decimal(decimal) => TypeForm::keyword(qsl_cst::token::Kind::DecimalType, SPAN)
            .with_bounds(vec![
                decimal.lower().to_string(),
                decimal.upper().to_string(),
                decimal.min_scale().to_string(),
                decimal.max_scale().to_string(),
                rounding_mode_text(decimal.rounding()).to_owned(),
            ]),
        ValueType::Float(width) => TypeForm::keyword(
            match width {
                IeeeWidth::Binary32 => qsl_cst::token::Kind::Float32Type,
                IeeeWidth::Binary64 => qsl_cst::token::Kind::Float64Type,
            },
            SPAN,
        ),
        ValueType::Text(text) => TypeForm::keyword(qsl_cst::token::Kind::TextType, SPAN)
            .with_bounds(vec![
                text.min().to_string(),
                text.max().to_string(),
                text_profile_text(text.profile()).to_owned(),
            ]),
        ValueType::Option(payload) => TypeForm::keyword(qsl_cst::token::Kind::OptionType, SPAN)
            .with_arguments(vec![type_form(payload)]),
        ValueType::Collection(collection) => collection_type_form(collection),
        ValueType::Population(maximum) => {
            TypeForm::name("Population", SPAN).with_bounds(vec![maximum.to_string()])
        }
        ValueType::Composite(_) | ValueType::Enum(_) | ValueType::Reference(_) => {
            panic!(
                "type_form: {value_type:?} carries a NodeKey with no recoverable name -- build \
                 a TypeForm::name(...) directly, with the same name string used to derive the \
                 key, instead of calling this function"
            )
        }
        ValueType::Quantity(_) => panic!(
            "type_form: {value_type:?} has no declared-type syntax -- qsl_cst's keyword \
             vocabulary has no QuantityType token (see forms/syntax.rs's own K5 doc), so no \
             fixture can build this TypeForm today; a caller reaching this arm wants \
             `value::equality::EqualityOperand`'s own `ValueType`-based API instead, not a \
             `TypeForm`"
        ),
    }
}

/// [`TypeForm::name`] at [`SPAN`], for a declared record, tuple, enum or
/// model object type's own name (see this module's own doc).
pub fn named_type_form(name: impl Into<String>) -> TypeForm {
    TypeForm::name(name, SPAN)
}
