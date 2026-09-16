// SPDX-License-Identifier: AGPL-3.0-or-later
//! Complete-V1 exact scalar semantics (QSL #118, AD-005).
//!
//! This typed layer is separate from the legacy compatibility scalar types in
//! `native_model`, `checking` and `runtime::evaluation`, which it neither
//! widens nor reuses. Layers:
//!
//! 1. typed values: [`Integer`], [`IntegerInterval`]/[`BoundedInteger`],
//!    [`Rational`], [`Decimal`], [`Text`], [`EnumValue`] and FR-142
//!    [`Quantity`] over an admitted [`UnitGraph`];
//! 2. explicit operation tables: [`evaluate_decimal`] (FR-140), [`divide`] and
//!    [`modulo`] (FR-147), [`admit_text`], [`compare_text`] and [`compare_enum`]
//!    (FR-141), [`evaluate_quantity`] and [`convert_quantity`] (FR-142), after the type-checking [`IllTyped`] refusal;
//! 3. the distinct evaluator [`Outcome`] with typed [`Undefined`], [`Refusal`]
//!    and [`Incomplete`] reasons;
//! 4. `quire.value.accounting/v1` metering through [`Meter`].
//!
//! Profile selection and misuse are refused at semantic admission by
//! [`DefinitionLock`]. No host floating-point arithmetic or narrowing integer
//! conversion is used by any semantic path.

mod accounting;
mod comparison;
mod decimal;
mod definition;
mod division;
mod enumeration;
mod integer;
mod node;
mod outcome;
mod quantity;
mod rational;
mod text;
mod unit;

pub use accounting::{ChargePoint, Incomplete, InjectedDenial, LimitKind, Meter, ScalarLimits};
pub use comparison::{ComparisonOperator, IllTyped, IllTypedCause};
pub use decimal::{
    evaluate_decimal, Decimal, DecimalLoss, DecimalOperation, DecimalRepresentation, DecimalResult,
    DecimalType, RoundingMode,
};
pub use definition::{
    AdmittedIntegerDivision, AdmittedSelection, CatalogEntry, CatalogRole, DefinitionLock,
    DefinitionReference, DefinitionRevision, LockError, PackageCause, PackageRefusal,
    PackageRefusalCode, SelectionRefusalCode, Trigger, PINNED_LOCK_BYTES,
};
pub use division::{divide, modulo, DivisionProfile, QuotientRemainder};
pub use enumeration::{
    compare_enum, EnumDeclaration, EnumDeclarationPreimage, EnumMemberPreimage, EnumValue,
};
pub use integer::{
    BoundedInteger, EmptyInterval, Integer, IntegerDomain, IntegerInterval, NonCanonicalInteger,
    OutOfDomain,
};
pub use node::{
    InvalidSemanticGraph, ModelSubject, NodeKey, NodeOwner, OwnerSelection, OwnerSubject,
    SemanticGraphCause, NODE_KEY_DOMAIN,
};
pub use outcome::{Outcome, Refusal, Undefined};
pub use quantity::{
    convert_quantity, evaluate_quantity, Conversion, ConvertedValue, Quantity, QuantityOperation,
    QuantityTarget, QuantityUnit,
};
pub use rational::{Rational, ZeroDenominator};
pub use text::{
    admit_text, compare_text, EmptyTextBounds, InvalidTextLiteral, InvalidUtf8, NormalizationForm,
    Text, TextPayload, TextProfile, TextProvenance, TextType, UNICODE_TEXT_DEFINITION,
    UNICODE_VERSION,
};
pub use unit::{
    CompoundUnit, CompoundUnitCause, CompoundUnitIdentity, CompoundUnitPreimage, Dimension,
    DimensionPreimage, InvalidCompoundUnit, Unit, UnitEdge, UnitGraph, UnitPreimage,
    COMPOUND_UNIT_DOMAIN,
};
