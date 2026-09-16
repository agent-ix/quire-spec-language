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
//! 3. FR-143 records, tuples and finite recursive [`Value`]s over a
//!    [`TypeEnvironment`], and the FR-149 equality matrix
//!    [`evaluate_equality`], FR-144 bounded [`CollectionValue`]s and the
//!    FR-145 collection queries [`map`], [`filter`], [`flatten`], [`count`],
//!    [`fold`], [`reduce`] and [`convert`], FR-146 total pure functions
//!    [`check_functions`] and FR-307 library resolution
//!    [`resolve_libraries`] (QSL #119);
//! 4. the distinct evaluator [`Outcome`] with typed [`Undefined`], [`Refusal`]
//!    and [`Incomplete`] reasons;
//! 5. `quire.value.accounting/v1` metering through [`Meter`].
//!
//! FR-148 IEEE binary32/binary64 profiles ([`evaluate_ieee`], [`compare_ieee`])
//! operate on exact bit patterns with soft-float arithmetic over big integers.
//!
//! Profile selection and misuse are refused at semantic admission by
//! [`DefinitionLock`]. No host floating-point arithmetic or narrowing integer
//! conversion is used by any semantic path.

mod accounting;
mod collection;
mod collection_query;
mod comparison;
mod composite;
mod containment;
mod decimal;
mod definition;
mod division;
mod enumeration;
mod equality;
mod function;
mod ieee;
mod integer;
mod library;
mod node;
mod outcome;
mod quantity;
mod rational;
mod reference;
mod text;
mod unit;

pub use accounting::{ChargePoint, Incomplete, InjectedDenial, LimitKind, Meter, ScalarLimits};
pub use collection::{
    CanonicalCollection, CanonicalEntry, CardinalityBound, CardinalityViolation, CollectionKind,
    CollectionValue, EmptyCardinalityBound, NoTotalElementKey,
};
pub use collection_query::{
    convert, count, filter, flatten, fold, map, reduce, AlgebraicProperties, CollectionConversion,
    CollectionLoss, FoldFunction, ValueFunction,
};
pub use comparison::{ComparisonOperator, IllTyped, IllTypedCause};
pub use composite::{
    Component, CompositeDeclaration, CompositeShape, CompositeValue, ConstructionCause,
    ConstructionRefusal, ConstructorDeclaration, DeclarationCause, FieldDeclaration,
    FieldExpression, FieldValue, InvalidDeclaration, OptionValue, Presence, TypeEnvironment, Value,
    ValueType,
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
pub use equality::{convert_for_equality, evaluate_equality, plan_equality, EqualityPlan};
pub use function::{
    check_functions, ArithmeticOperator, CallRefusal, CheckedFoldFunction, CheckedFunction,
    CheckedFunctions, CheckedValueFunction, DecreaseObligation, DischargedPrecondition, Effect,
    ElementRelation, Expression, FunctionCause, FunctionDeclaration, FunctionLimits,
    FunctionRefusal, IntegerComparison, Location, MeasureElement, ParameterDeclaration, PathStep,
    Precondition, PreconditionEvidence, Termination,
};
pub use ieee::{
    compare_ieee, convert_ieee_width, evaluate_ieee, exact_to_ieee, ieee_intrinsic_identities,
    ieee_to_exact, negotiate_ieee, AdmittedIeeeProfile, ExactScalar, IeeeBackendCapabilities,
    IeeeComparison, IeeeDisposition, IeeeExact, IeeeExactTarget, IeeeFlag, IeeeFlags,
    IeeeItemRequirement, IeeeOperand, IeeeOperation, IeeeOperationKind, IeeeProvenance, IeeeResult,
    IeeeUnsupportedCause, IeeeValue, IeeeWidth, IEEE_DEFINITION,
};
pub use integer::{
    BoundedInteger, EmptyInterval, Integer, IntegerDomain, IntegerInterval, NonCanonicalInteger,
    OutOfDomain,
};
pub use library::{
    check_migration, resolve_libraries, Export, ExportIdentity, ImportDeclaration, ImportPath,
    InvalidLibraryName, LibraryLock, LibraryMigration, LibraryName, LibraryPackage, LibraryRefusal,
    MigrationRefusal, NameReference, NameRefusal, PackageId, Selection,
};
pub use node::{
    InvalidSemanticGraph, ModelSubject, NodeKey, NodeOwner, OwnerSelection, OwnerSubject,
    SemanticGraphCause, NODE_KEY_DOMAIN,
};
pub use outcome::{Outcome, Refusal, Undefined};
pub use quantity::{
    compare_quantity, convert_quantity, evaluate_quantity, Conversion, ConvertedValue, Quantity,
    QuantityOperation, QuantityTarget, QuantityUnit,
};
pub use rational::{NonPositiveDenominatorBound, Rational, RationalDomain, ZeroDenominator};
pub use reference::{
    InvalidObjectIdentity, ObjectEnvironment, ObjectEnvironmentCause, ObjectEnvironmentRefusal,
    ObjectIdentity, ObjectReference,
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
