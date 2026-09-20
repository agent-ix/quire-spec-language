// SPDX-License-Identifier: AGPL-3.0-or-later
//! `quire-exact`: the QSL kernel row (QSL#213 S-1, ADR-011 X-1, ADR-013 §7
//! S-1).
//!
//! This crate is the AD-016/ADR-011 module-DAG leaf layer `K`: checked
//! identity ([`NodeKey`] and the six opaque digest identities, such as
//! [`EffectiveId`]), provenance ([`Location`]), kernel outcomes and refusals
//! ([`Outcome`], [`Refusal`]), bounds and accounting ([`Meter`],
//! [`BoundedInteger`], [`CardinalityBound`]), and the exact semantic value
//! kernel ([`Value`]/[`ValueType`] and every value-family module it
//! composes: `numeric`, `rational`, `decimal`, `text`, `ieee`, `division`,
//! `comparison`, `equality`, `key`, `quantity`, `reference`). Every
//! submodule is private; this crate's public surface is exactly this page's
//! curated `pub use` facade (H-5, mirroring `src/value/mod.rs`'s own
//! private-submodules-behind-re-exports pattern), so the module names above
//! are plain text, not links.
//!
//! It depends on nothing else in the `quire-spec-language` workspace (ADR-011
//! §6.1, §7.1: every crate-DAG edge points *into* this crate, never out of
//! it), and on no wire format, hashing or JCS canonicalization crate: every
//! digest identity here ([`NodeKey`] and the six digest identities,
//! [`EffectiveId`], [`UniverseId`], [`ObjectId`], [`UnitId`], [`VariantId`],
//! [`MemberId`]) is minted by wrapping an already-computed digest through
//! its one public `from_digest` constructor (ADR-013 T-6), never by hashing
//! internally.
//!
//! Several real, deliberate capability losses at this kernel boundary are
//! documented where they occur rather than silently absorbed:
//! - [`Value`]/[`ValueType`]: `Value::Population` has no kernel payload at
//!   all (the `ValueType::Enum` shape, by contrast, carries its variant set
//!   inline per ADR-013 O-14, so it needs no declaration lookup and is not a
//!   capability loss).
//! - the `key` and `equality` modules: an enum pair keys and compares equal
//!   by raw digest, with no declaration-aware ordering or same-enum check.
//! - [`Quantity`]: no cross-unit arithmetic, comparison or equality; only
//!   same-unit operations.
//! - the `equality` module: the top-level text/enum/quantity schedule
//!   selection and the closed equality-conversion table are dropped along
//!   with the declaration registry and unit graph they need.
//!
//! **The ADR-011 §2.3 kernel proof gate does not exist yet.** This crate
//! ships with zero discharged propositions and no claimed-module list.
//! `cargo kani` cannot run against this workspace at all today: Kani 0.67.0's
//! bundled toolchain is `rustc 1.93.0-nightly`, while this workspace declares
//! `rust-version = "1.98"`, and `cargo kani` separately fails on a vendored
//! dependency's fixture `Cargo.toml`. Building the gate is tracked
//! separately (QSL-130) and left to ADR-011 §2.3's own named enforcer, #219.

#![forbid(unsafe_code)]

mod accounting;
mod collection;
mod comparison;
mod decimal;
mod division;
mod equality;
mod identity;
mod ieee;
mod integer;
mod key;
mod location;
mod node;
mod numeric;
mod outcome;
mod quantity;
mod rational;
mod reference;
mod text;
mod value;

// H-5: every submodule above is private and its public surface is exposed
// only through this curated facade, mirroring `src/value/mod.rs`'s pattern
// (private `mod`s behind selective `pub use` re-exports) rather than
// `pub mod` wholesale. `crate::key::compare_keys` and the five functions
// this PR's own REVISE round demoted to `pub(crate)`
// (`rational::divided_by_power_of_ten`, `decimal::DecimalRepresentation::
// to_rational`) are deliberately absent below: they are reachable only from
// inside this crate.
pub use accounting::{ChargePoint, Incomplete, InjectedDenial, LimitKind, Meter, ScalarLimits};
pub use collection::{
    construct_collection, form_collection, form_grouped, from_admitted, CardinalityBound,
    CollectionKind, CollectionType, CollectionValue, EmptyCardinalityBound,
};
pub use comparison::{ComparisonOperator, IllTyped, IllTypedCause};
pub use decimal::{
    evaluate_decimal, Decimal, DecimalLoss, DecimalOperation, DecimalRepresentation, DecimalResult,
    DecimalType, RoundingMode,
};
pub use division::{divide, modulo, DivisionProfile, QuotientRemainder};
pub use equality::{plan_equality, planned_equality, EqualityPlan};
pub use identity::{
    EffectiveId, MemberId, ObjectId, UnitId, UniverseId, VariantId, EFFECTIVE_ID_DOMAIN,
    MEMBER_ID_DOMAIN, OBJECT_ID_DOMAIN, UNIT_ID_DOMAIN, UNIVERSE_ID_DOMAIN, VARIANT_ID_DOMAIN,
};
pub use ieee::{
    compare_ieee, convert_ieee_width, evaluate_ieee, exact_to_ieee, ieee_intrinsic_identities,
    ieee_to_exact, ExactScalar, IeeeComparison, IeeeExact, IeeeExactLoss, IeeeExactTarget,
    IeeeFlag, IeeeFlags, IeeeOperand, IeeeOperation, IeeeOperationKind, IeeeProvenance, IeeeResult,
    IeeeValue, IeeeWidth, IEEE_DEFINITION,
};
pub use integer::{
    BoundedInteger, EmptyInterval, Integer, IntegerDomain, IntegerInterval, NonCanonicalInteger,
    OutOfDomain,
};
pub use location::{Location, Origin, Role};
pub use node::{NodeKey, NODE_KEY_DOMAIN};
pub use numeric::{
    evaluate_boolean, evaluate_integer_arithmetic, evaluate_rational_arithmetic, order_numbers,
    ArithmeticOperator, BooleanConnective, IntegerArithmetic, OrderedOperands, OrderingOperator,
    RationalArithmetic,
};
pub use outcome::{BoundViolation, Outcome, Refusal, Undefined};
pub use quantity::{compare_quantity, evaluate_quantity_arithmetic, Quantity, QuantityArithmetic};
pub use rational::{NonPositiveDenominatorBound, Rational, RationalDomain, ZeroDenominator};
pub use reference::ObjectReference;
pub use text::{
    admit_text, compare_text, EmptyTextBounds, InvalidUtf8, NormalizationForm, Text, TextPayload,
    TextProfile, TextProvenance, TextType, UNICODE_TEXT_DEFINITION, UNICODE_VERSION,
};
pub use value::{
    evaluate_record, evaluate_tuple, fill_slots, from_admitted_slots, record, tuple, Component,
    CompositeValue, ConstructionCause, ConstructionRefusal, Deferred, EnumShape, FieldDeclaration,
    FieldExpression, FieldValue, OptionValue, Presence, Value, ValueType,
};
