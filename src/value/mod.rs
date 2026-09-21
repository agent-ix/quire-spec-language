// SPDX-License-Identifier: AGPL-3.0-or-later
//! Complete-V1 exact scalar semantics (QSL #118, AD-005).
//!
//! This typed layer is separate from the legacy compatibility scalar types in
//! `native_model`, `checking` and `runtime::evaluation`, which it neither
//! widens nor reuses. Layers:
//!
//! 1. typed values: `quire_exact::Integer`,
//!    `quire_exact::IntegerInterval`/`quire_exact::BoundedInteger`,
//!    [`Rational`], [`Decimal`], [`Text`], [`EnumValue`] and FR-142
//!    [`Quantity`] over an admitted [`UnitGraph`];
//! 2. explicit operation tables: [`evaluate_decimal`] (FR-140), [`divide`] and
//!    [`modulo`] (FR-147), [`admit_text`], [`compare_text`] and [`compare_enum`]
//!    (FR-141), [`evaluate_quantity`] and [`convert_quantity`] (FR-142), after the type-checking [`IllTyped`] refusal;
//! 3. FR-143 records, tuples and finite recursive [`Value`]s over a
//!    [`TypeEnvironment`], the FR-149 equality matrix
//!    ([`TypeEnvironment::check_equality`]), FR-144 bounded
//!    [`CollectionValue`]s ([`construct_collection`]) and FR-307 library
//!    resolution [`resolve_libraries`] (QSL #119);
//! 4. [`order_numbers`] compares one [`OrderedOperands`] pair (`Integer`,
//!    `Rational` or `Decimal`) under one [`OrderingOperator`];
//!    [`evaluate_integer_arithmetic`] evaluates add/subtract/multiply/negate
//!    over [`IntegerArithmetic`] operands (integer division is rational
//!    division of `n`/`1`, so it has no `Divide` variant here) and
//!    [`evaluate_rational_arithmetic`] evaluates add/subtract/multiply/divide/
//!    negate over [`RationalArithmetic`] operands; [`evaluate_boolean`]
//!    evaluates the QSL connectives `and`/`or`/`implies`/`not` over
//!    [`BooleanConnective`] operands — all under `quire.value.accounting/v1`
//!    (QSL #119);
//! 5. the distinct evaluator [`Outcome`] with typed [`Undefined`], [`Refusal`]
//!    and [`Incomplete`] reasons;
//! 6. `quire.value.accounting/v1` metering through [`Meter`].
//!
//! FR-148 IEEE binary32/binary64 profiles ([`evaluate_ieee`], [`compare_ieee`])
//! operate on exact bit patterns with soft-float arithmetic over big integers.
//!
//! Profile selection and misuse are refused at semantic admission by
//! [`DefinitionLock`]. No host floating-point arithmetic or narrowing integer
//! conversion is used by any semantic path.

mod accounting;
mod collection;
mod comparison;
mod composite;
mod containment;
mod decimal;
mod definition;
mod division;
mod enumeration;
mod equality;
mod expression;
mod ieee;
mod key;
mod library;
mod member;
mod model_query;
mod node;
mod numeric;
mod outcome;
mod package_identity;
mod quantity;
mod rational;
mod reference;
mod text;
mod unit;

pub use accounting::{ChargePoint, Incomplete, InjectedDenial, LimitKind, Meter, ScalarLimits};
// `crate::model` reuses this crate-wide `usize -> u64` persistence/wire
// conversion (PR #140 F7) rather than a bare `as u64` at its own charge sites.
// FR-153's `crate::model::population` charges `lookup.*`, `population.visit`
// and `collection.*` directly against this crate's own `ScalarLimitsV1`
// meter (`quire.value.accounting/v1` is the one schedule those charge points
// belong to), so `Charge` itself is exposed crate-wide the same way.
pub(crate) use accounting::{length_amount, Charge};
pub use collection::{
    construct_collection, form_collection, CardinalityBound, CollectionType, CollectionValue,
    EmptyCardinalityBound,
};
pub use comparison::{ComparisonOperator, IllTyped, IllTypedCause};
pub use composite::{
    Component, CompositeDeclaration, CompositeShape, CompositeValue, ConstructionCause,
    ConstructionRefusal, DeclarationCause, Deferred, FieldDeclaration, FieldExpression, FieldValue,
    InvalidDeclaration, ObjectTypeDeclaration, OptionValue, Presence, RecursionEdges,
    TypeEnvironment, Value, ValueType,
};
pub use containment::{GraphCause, GraphNode, GraphNodeId, GraphRefusal, GraphSlot, ValueGraph};
pub use decimal::{
    evaluate_decimal, Decimal, DecimalLoss, DecimalOperation, DecimalRepresentation, DecimalResult,
    DecimalType, RoundingMode,
};
pub use definition::{
    AdmittedIntegerDivision, AdmittedSelection, CatalogEntry, CatalogRole, DefinitionLock,
    DefinitionReference, DefinitionRevision, LockError, PackageCause, PackageRefusal,
    PackageRefusalCode, SelectionRefusalCode, Trigger, PINNED_LOCK_BYTES,
};
pub use division::{
    divide, modulo, negotiate_integer_division, DivisionProfile, IntegerDivisionBounds,
    IntegerDivisionConsumer, IntegerDivisionDisposition, QuotientRemainder,
};
pub use enumeration::{
    compare_enum, EnumDeclaration, EnumDeclarationPreimage, EnumMemberPreimage, EnumValue,
};
pub use equality::{
    admits_equality_conversion, plan_equality, CheckedEquality, EqualityOperand, EqualityOperator,
    EqualityPlan, EqualitySchedule,
};
// ADR-011 §7.3 M-5 (QSL-139/FR-068) relocated the checking half of
// `value::expression` to the layer-3 `check` module; `value`'s own
// aggregation path continues, only its source module changes
// (FR-068-CON-4, the same "re-export naming a new source module is not a
// second definition" pattern FR-067-AC-9 already established for the
// `forms` move) -- `check` is these types' one remaining defining module.
pub use crate::check::{
    CheckCause, CheckMode, CheckRefusal, CheckedExpression, CheckedPackage, CheckingLimitKind,
    CheckingLimits, CheckingStage, CollectionLoss, CollectionProperty, DepthAboveMaximum,
    DispatchCandidate, DispatchFunctionRole, DispatchOperation, DispatchTable, EnumBinding,
    InvalidDispatchDeclaration, Location, MeasureObligation, Obligation, Origin,
    PackageDeclarations, ProvedInterval, WrongSnapshotCause, MAX_CHECKING_DEPTH,
};
// The evaluation half stays at layer 5, in `value::expression` itself.
pub use expression::{
    DecodeV2Error, Evaluation, InputRefusal, InvalidQualifiedName, LocatedLoss, QualifiedName,
    ValueLoss,
};
// The S2 parsed-form types (ADR-011 §6.2 module map: `value::expression::syntax`
// moves to layer-2 `forms`, M-3a). Re-exported here, not re-defined: `forms`
// is their one defining module (FR-067-AC-9). `value::Expression` etc. were
// already this module's own aggregation path before the move (this file
// re-exports dozens of other types the same way, from their own owning
// submodules); the move changes which module they aggregate from, not
// whether `value` aggregates them.
pub use crate::forms::{
    Accumulation, BinaryOperator, BinderQuery, ClauseKind, DeclaredClauseKind, Expression,
    FieldInitializer, FunctionDeclaration,
};
pub use ieee::{
    compare_ieee, convert_ieee_width, evaluate_ieee, exact_to_ieee, ieee_intrinsic_identities,
    ieee_to_exact, negotiate_ieee, AdmittedIeeeProfile, ExactScalar, IeeeBackendCapabilities,
    IeeeComparison, IeeeDisposition, IeeeExact, IeeeExactLoss, IeeeExactTarget, IeeeFlag,
    IeeeFlags, IeeeItemRequirement, IeeeOperand, IeeeOperation, IeeeOperationKind, IeeeProvenance,
    IeeeResult, IeeeUnsupportedCause, IeeeValue, IeeeWidth, IEEE_DEFINITION,
};
pub use library::{
    check_migration, resolve_libraries, ExportIdentity, ImportDeclaration, ImportPath,
    InvalidLibraryName, LibraryCause, LibraryLock, LibraryMigration, LibraryName, LibraryPackage,
    LibraryRefusal, NameReference, NameRefusal, PackageId, Selection, StaleCause,
    IDENTITY_PREIMAGE_PATH, PACKAGE_ID_PATH,
};
pub use member::{Identifier, InvalidIdentifier, Member};
pub use node::{
    InvalidSemanticGraph, ModelSubject, NodeKey, NodeOwner, OwnerSelection, OwnerSubject,
    SemanticGraphCause, NODE_KEY_DOMAIN,
};
pub use numeric::{
    evaluate_boolean, evaluate_integer_arithmetic, evaluate_rational_arithmetic, order_numbers,
    BooleanConnective, IntegerArithmetic, OrderedOperands, OrderingOperator, RationalArithmetic,
};
// `crate::check`'s tier-1 K-designated import (FR-068's Behavior section,
// "The layer-3 sibling imports `check.rs` keeps"): `value`'s own submodules
// are private (`mod numeric;`), so a crate-absolute `crate::value::numeric::
// ArithmeticOperator` path -- reachable pre-move only because `check.rs` was
// a descendant of `value` -- no longer resolves from `check`, a top-level
// sibling. Exposed crate-internal-only here, the same `pub(crate) use`
// pattern this file already uses for `length_amount`/`Charge`.
pub(crate) use numeric::ArithmeticOperator;
pub use outcome::{BoundViolation, Outcome, PreconditionFailure, Refusal, Undefined};
pub use package_identity::{NodeDefect, PreimageDefect};
pub use quantity::{
    compare_quantity, convert_quantity, evaluate_quantity, Conversion, ConvertedValue, Quantity,
    QuantityOperation, QuantityTarget, QuantityUnit,
};
// `crate::check`'s tier-2 declared interim edge (FR-068's Behavior section,
// same subsection; FR-068-AC-6): exposed crate-internal-only for the same
// reason as `ArithmeticOperator` above.
pub(crate) use quantity::{check_comparable, result_unit, UnitOperation};
pub use rational::{NonPositiveDenominatorBound, Rational, RationalDomain, ZeroDenominator};
pub use reference::{
    InvalidObjectIdentity, ObjectEnvironment, ObjectEnvironmentCause, ObjectEnvironmentRefusal,
    ObjectIdentity, ObjectReference, UniverseIdentity,
};
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
