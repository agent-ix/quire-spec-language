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
//!    ([`TypeEnvironment::check_equality`]) and FR-144 bounded
//!    [`CollectionValue`]s ([`construct_collection`]) (QSL #119); FR-307
//!    library resolution relocated to the top-level `library` module
//!    (FR-087, #213 S-3a);
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
//!    and [`Incomplete`](quire_exact::Incomplete) reasons;
//! 6. `quire.value.accounting/v1` metering through
//!    [`Meter`](quire_exact::Meter).
//!
//! FR-148 IEEE binary32/binary64 profiles ([`evaluate_ieee`], [`compare_ieee`])
//! operate on exact bit patterns with soft-float arithmetic over big integers.
//!
//! Profile selection and misuse are refused at semantic admission by
//! [`DefinitionLock`]. No host floating-point arithmetic or narrowing integer
//! conversion is used by any semantic path.
//!
//! ## FR-078: no `negotiate_*` copies (TC-201)
//!
//! QSL-131 removed the RT capability-negotiation copies `negotiate_ieee`,
//! `negotiate_integer_division` and their supporting types from `ieee` and
//! `division` (FR-078-AC-1, FR-078-AC-2); only the evaluation functions
//! listed above remain. Each removed name is gone from this crate, not
//! renamed or gated: importing it from `value` is `E0432`, an unresolved
//! import, not a type or borrow error.
//!
//! ```compile_fail,E0432
//! use quire_spec_language::value::negotiate_ieee;
//! ```
//! ```compile_fail,E0432
//! use quire_spec_language::value::negotiate_integer_division;
//! ```
//! ```compile_fail,E0432
//! use quire_spec_language::value::IeeeBackendCapabilities;
//! ```
//! ```compile_fail,E0432
//! use quire_spec_language::value::IeeeItemRequirement;
//! ```
//! ```compile_fail,E0432
//! use quire_spec_language::value::IeeeUnsupportedCause;
//! ```
//! ```compile_fail,E0432
//! use quire_spec_language::value::IeeeDisposition;
//! ```
//! ```compile_fail,E0432
//! use quire_spec_language::value::IntegerDivisionBounds;
//! ```
//! ```compile_fail,E0432
//! use quire_spec_language::value::IntegerDivisionConsumer;
//! ```
//! ```compile_fail,E0432
//! use quire_spec_language::value::IntegerDivisionDisposition;
//! ```
//!
//! A positive control: the module path itself still resolves, and a
//! surviving name imports cleanly, so the failures above are each the named
//! removed item, not a broken crate path.
//!
//! ```
//! use quire_spec_language::value::AdmittedIeeeProfile;
//! fn _use(_: AdmittedIeeeProfile) {}
//! ```

// PR #282 review, F2: these eleven submodules are `pub(crate)`, not private
// `mod`, so `check`'s tier-1/tier-2 imports (FR-068's Behavior section, "The
// layer-3 sibling imports `check.rs` keeps") can name them by their real,
// submodule-qualified crate-absolute path (`crate::value::numeric::
// ArithmeticOperator`, not the flat `crate::value::ArithmeticOperator`
// aggregate) -- FR-068:280-283 prescribes this form explicitly, and only
// this form keeps AC-6's two-tier allow-list legible to a textual scan of
// `check`'s own `use` lines (a flat import carries no tier information: it
// reads identically whether the item is tier 1, tier 2, or the forbidden
// tier 3). Every other `value::` submodule stays private; `check` imports
// nothing from them.
pub(crate) mod collection;
pub(crate) mod comparison;
pub(crate) mod composite;
mod containment;
pub(crate) mod decimal;
mod definition;
mod division;
pub(crate) mod enumeration;
pub(crate) mod equality;
mod expression;
pub(crate) mod ieee;
mod key;
mod member;
mod model_query;
pub(crate) mod node;
pub(crate) mod numeric;
mod outcome;
pub(crate) mod quantity;
pub(crate) mod rational;
mod reference;
pub(crate) mod text;
mod unit;

// QSL-166: `ChargePoint`, `Incomplete`, `InjectedDenial`, `LimitKind`,
// `Meter`, `ScalarLimits`, `Charge` and `length_amount` were this module's
// own `accounting` submodule, re-exported from here. That submodule
// duplicated `quire-exact/src/accounting.rs` byte-for-byte and is deleted;
// every former consumer (`crate::model::population` included, which reused
// this crate-wide re-export the same way FR-153 reuses this crate's own
// `quire.value.accounting/v1` meter for `lookup.*`/`population.visit`/
// `collection.*` charges) now imports `quire_exact::{..}` directly. One
// definition, one import path -- no re-export shim stands in for the
// deleted module.
// `CardinalityBound`/`EmptyCardinalityBound` are `quire_exact`'s own types; this
// module does not re-export them (QSL-131 S-1b), so consumers import them from
// `quire_exact` directly.
pub use collection::{construct_collection, form_collection, CollectionType, CollectionValue};
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
    DefinitionReference, DefinitionRevision, PackageCause, PackageRefusal, PackageRefusalCode,
    SelectionRefusalCode, Trigger,
};
pub use division::{divide, modulo, DivisionProfile, QuotientRemainder};
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
    CheckCause, CheckMode, CheckRefusal, CheckedExpression, CheckedGraph, CheckingLimitKind,
    CheckingLimits, CheckingStage, CollectionLoss, CollectionProperty, DepthAboveMaximum,
    DispatchCandidate, DispatchFunctionRole, DispatchOperation, DispatchTable, EnumBinding,
    InvalidDispatchDeclaration, Location, MeasureObligation, Obligation, Origin,
    PackageDeclarations, ProvedInterval, WrongSnapshotCause, MAX_CHECKING_DEPTH,
};
// ADR-013 T-1 (FR-087, QSL-158 S-3a): `CheckedPackage` (S4 in-process, the
// S3 `CheckedGraph` above plus the checked dependency closure) is a
// different, canonical type, defined in layer-4 `checked_package`, not
// `check`. Re-exported through `expression` (which already re-exports it
// from `crate::checked_package` as the S6a entry point, FR-087-AC-9/TC-256)
// rather than a second, independent
// `pub use crate::checked_package::CheckedPackage;` line, so there is
// exactly one re-export source for `value` to track.
pub use expression::CheckedPackage;
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
    ieee_to_exact, AdmittedIeeeProfile, ExactScalar, IeeeComparison, IeeeExact, IeeeExactLoss,
    IeeeExactTarget, IeeeFlag, IeeeFlags, IeeeOperand, IeeeOperation, IeeeOperationKind,
    IeeeProvenance, IeeeResult, IeeeValue, IeeeWidth, IEEE_DEFINITION,
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
pub use outcome::{BoundViolation, Outcome, PreconditionFailure, Refusal, Undefined};
pub use quantity::{
    compare_quantity, convert_quantity, evaluate_quantity, Conversion, ConvertedValue, Quantity,
    QuantityOperation, QuantityTarget, QuantityUnit,
};
pub use rational::{NonPositiveDenominatorBound, Rational, RationalDomain, ZeroDenominator};
pub use reference::{
    InvalidObjectIdentity, ObjectEnvironment, ObjectEnvironmentCause, ObjectEnvironmentRefusal,
    ObjectIdentity, ObjectReference, PopulationConflict, UniverseIdentity,
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
