// SPDX-License-Identifier: AGPL-3.0-or-later
//! Complete-V1 exact scalar semantics (QSL #118, AD-005).
//!
//! This typed layer is separate from the legacy compatibility scalar types in
//! `native_model`, `checking` and `runtime::evaluation`, which it neither
//! widens nor reuses. Layers:
//!
//! 1. typed values: `quire_exact::Integer`,
//!    `quire_exact::IntegerInterval`/`quire_exact::BoundedInteger`,
//!    [`Rational`](quire_exact::Rational), [`Decimal`](quire_exact::Decimal),
//!    [`Text`](quire_exact::Text), [`EnumValue`] and FR-142
//!    `quire_exact::Quantity` values read against a [`UnitTable`] over an
//!    admitted [`UnitGraph`];
//! 2. explicit operation tables: [`evaluate_decimal`](quire_exact::evaluate_decimal)
//!    (FR-140), [`divide`] and [`modulo`] (FR-147),
//!    [`admit_text`](quire_exact::admit_text),
//!    [`compare_text`](quire_exact::compare_text) and [`compare_enum`]
//!    (FR-141), [`evaluate_quantity`] and [`convert_quantity`] (FR-142), after
//!    the type-checking [`IllTyped`](quire_exact::IllTyped) refusal;
//! 3. FR-143 records, tuples and finite recursive [`Value`]s over a
//!    [`TypeEnvironment`], the FR-149 equality matrix
//!    ([`TypeEnvironment::check_equality`]) and FR-144 bounded
//!    [`CollectionValue`]s ([`construct_collection`]) (QSL #119); FR-307
//!    library resolution relocated to the top-level `library` module
//!    (FR-087, #213 S-3a);
//! 4. [`order_numbers`](quire_exact::order_numbers) compares one
//!    [`OrderedOperands`](quire_exact::OrderedOperands)
//!    pair (`Integer`, `Rational` or `Decimal`) under one
//!    [`OrderingOperator`](quire_exact::OrderingOperator);
//!    [`evaluate_integer_arithmetic`](quire_exact::evaluate_integer_arithmetic)
//!    evaluates add/subtract/multiply/negate
//!    over [`IntegerArithmetic`](quire_exact::IntegerArithmetic) operands
//!    (integer division is rational division of `n`/`1`, so it has no
//!    `Divide` variant here) and
//!    [`evaluate_rational_arithmetic`](quire_exact::evaluate_rational_arithmetic)
//!    evaluates add/subtract/multiply/divide/negate over
//!    [`RationalArithmetic`](quire_exact::RationalArithmetic) operands;
//!    [`evaluate_boolean`](quire_exact::evaluate_boolean) evaluates the QSL
//!    connectives `and`/`or`/`implies`/`not` over
//!    [`BooleanConnective`](quire_exact::BooleanConnective) operands — all
//!    under `quire.value.accounting/v1` (QSL #119);
//! 5. the distinct evaluator [`Outcome`](quire_exact::Outcome) with typed
//!    [`Undefined`](quire_exact::Undefined), [`Refusal`](quire_exact::Refusal)
//!    and [`Incomplete`](quire_exact::Incomplete) reasons -- the kernel's own
//!    types (QSL-131 O2 deleted this module's byte-identical copy);
//! 6. `quire.value.accounting/v1` metering through
//!    [`Meter`](quire_exact::Meter).
//!
//! FR-148 IEEE binary32/binary64 profiles
//! ([`evaluate_ieee`](quire_exact::evaluate_ieee),
//! [`compare_ieee`](quire_exact::compare_ieee)) operate on exact bit patterns
//! with soft-float arithmetic over big integers. QSL-131 O3 deleted this
//! module's byte-identical `decimal`/`ieee`/`numeric`/`text` engine copies;
//! every one of the operation-table items above now names its `quire_exact`
//! definition directly, not a re-export from here.
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

// PR #282 review, F2: these nine submodules are `pub(crate)`, not private
// `mod`, so `check`'s tier-1/tier-2 imports (FR-068's Behavior section, "The
// layer-3 sibling imports `check.rs` keeps") can name them by their real,
// submodule-qualified crate-absolute path (`crate::value::quantity::
// UnitTable`, not the flat `crate::value::UnitTable`
// aggregate) -- FR-068:280-283 prescribes this form explicitly, and only
// this form keeps AC-6's two-tier allow-list legible to a textual scan of
// `check`'s own `use` lines (a flat import carries no tier information: it
// reads identically whether the item is tier 1, tier 2, or the forbidden
// tier 3). Every other `value::` submodule stays private; `check` imports
// nothing from them. QSL-131 O3 deleted `decimal`, `ieee`, `numeric` and
// `text` from this list; `check` now imports their former items from
// `quire_exact` directly.
mod application_key;
mod containment;
pub(crate) mod declaration;
pub(crate) mod definition;
pub(crate) mod enumeration;
mod expression;
mod key;
mod member;
mod model_query;
pub(crate) mod quantity;
mod reference;
pub(crate) mod semantic_node;
mod stop;
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
// QSL-131 V5: `Value`, `ValueType`, `OptionValue`, `CompositeValue`,
// `FieldValue`, `CollectionType` and `CollectionValue` are `quire_exact`'s
// own kernel types (ADR-011 §6.1's K row). QSL-131 V5b deleted the
// `value::composite` and `value::collection` modules that used to re-export
// them (both were empty once the kernel widened `form`, `form_grouped` and
// `member_equal` to `pub`, letting `value::expression::evaluate`'s `Machine`
// call the kernel directly), so every one of these seven is imported from
// `quire_exact` here with no intermediate module. `FieldDeclaration`,
// `Component`, `ConstructionCause` and `ConstructionRefusal` moved to
// `declaration` (QSL-131 V5b): they stay QSL's own, name-keyed types, since
// the kernel's own versions key a field by its opaque `MemberId` (ADR-013
// O-06), which needs the `check::CheckedGraph` member-identity resolution
// `value::member`'s own doc marks as not yet landed ("No production caller
// constructs a `Member` yet") -- adopting them here is remaining work,
// gated on that landing, not on this crate's own choice.
pub use containment::{GraphCause, GraphNode, GraphNodeId, GraphRefusal, GraphSlot, ValueGraph};
pub use quire_exact::{
    construct_collection, form_collection, CollectionType, CollectionValue, CompositeValue,
    Deferred, FieldValue, OptionValue, Value, ValueType,
};
// QSL-131 O3: `DecimalType`, `DecimalLoss`, `DecimalResult` and
// `evaluate_decimal` were this module's own `decimal` submodule, a
// byte-identical duplicate of `quire_exact`'s (V4 had already moved unit
// placement onto the kernel's `DecimalType::placement`; only the type and
// its evaluation engine were left). That submodule is deleted; every former
// consumer now imports `quire_exact::{DecimalType, DecimalLoss,
// DecimalResult, evaluate_decimal}` directly -- one definition, one import
// path, no re-export standing in for the deleted module.
// `declaration` owns the FR-143 registry and the FR-149 check-level
// equality layer; neither is a kernel type (ADR-011 §6.1). `FieldDeclaration`,
// `Component`, `ConstructionCause` and `ConstructionRefusal` moved here from
// the deleted `value::composite` (QSL-131 V5b, see the note above).
pub use declaration::{
    CheckedEquality, Component, CompositeDeclaration, CompositeShape, ConstructionCause,
    ConstructionRefusal, DeclarationCause, EqualityOperand, EqualityOperator, EqualitySchedule,
    FieldDeclaration, FieldExpression, InvalidDeclaration, ObjectTypeDeclaration, RecursionEdges,
    TypeEnvironment,
};
pub use definition::{
    divide, modulo, AdmittedIeeeProfile, AdmittedIntegerDivision, AdmittedSelection, CatalogEntry,
    CatalogRole, DefinitionLock, DefinitionReference, DefinitionRevision, PackageCause,
    PackageRefusal, PackageRefusalCode, SelectionRefusalCode, Trigger,
};
pub use enumeration::{
    compare_enum, mint_variant_id, EnumDeclaration, EnumDeclarationPreimage, EnumMemberIndex,
    EnumMemberPreimage, EnumValue,
};
// QSL-131 V5b deleted `value::equality`: `plan_pairs`/`PlannedPairs`/`Pair`
// (the last of its byte-identical copies of kernel logic, kept only because
// the kernel's own `plan_pairs` is `pub(crate)`) are gone entirely, not
// re-exported -- `value::expression::evaluate`'s `Machine` now gets a pair
// count and equality Boolean straight from the kernel's own
// `quire_exact::member_equal`, under the same `collection.member-walk`/
// `collection.member-test` charge points. `plan_equality` and `EqualityPlan`
// were already pure re-exports of identical kernel items, so they are
// imported from `quire_exact` directly with no intermediate module.
pub use quire_exact::{plan_equality, EqualityPlan};
// ADR-011 §7.3 M-5 (QSL-139/FR-068) relocated the checking half of
// `value::expression` to the layer-3 `check` module; `value`'s own
// aggregation path continues, only its source module changes
// (FR-068-CON-4: a re-export naming a new source module is not a second
// definition) -- `check` is these types' one remaining defining module.
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
// `CheckedPackageEvaluation` is the trait that carries `call`, `evaluate`
// and `emit_function_package_v2` over the foreign-to-`value`
// `CheckedPackage` typestate, since an inherent impl here would be E0116
// once `CheckedPackage` is `qsl-package`'s own type (X-7).
// `decode_function_package_v2` takes no package, so it is a free function
// beside the v2 codec it wraps.
pub use expression::{
    decode_function_package_v2, CallFailure, CheckedPackageEvaluation, DecodeV2Error, Evaluation,
    FamilyOutcome, FamilyResult, InputRefusal, InvalidQualifiedName, LocatedLoss, QualifiedName,
    ValueLoss,
};
// QSL-131 O3: `ExactScalar`, `IeeeOperand`, `IeeeProvenance`, `IeeeResult`,
// `IeeeExact`, `IeeeExactTarget`, the five entry points (`evaluate_ieee`/
// `compare_ieee`/`convert_ieee_width`/`ieee_to_exact`/`exact_to_ieee`) and
// the private rounding/arithmetic engine beneath them were this module's
// own `ieee` submodule, a byte-identical duplicate of `quire_exact`'s
// (`IeeeWidth`, `IeeeValue`, `IeeeFlag`, `IeeeFlags`, `IeeeComparison`,
// `IeeeOperationKind`, `ieee_intrinsic_identities`, `IeeeExactLoss`,
// `IeeeOperation` and `IEEE_DEFINITION` were already re-exported straight
// from `quire_exact`, not duplicated). That submodule is deleted; every
// former consumer now imports the whole set from `quire_exact` directly.
pub use member::Member;
// QSL-131 O3: `evaluate_boolean`, `evaluate_integer_arithmetic`,
// `evaluate_rational_arithmetic` and `order_numbers` were this module's own
// `numeric` submodule, a byte-identical duplicate of `quire_exact`'s
// (`ArithmeticOperator`, `OrderingOperator`, `OrderedOperands`,
// `IntegerArithmetic`, `RationalArithmetic` and `BooleanConnective` were
// already re-exported straight from `quire_exact`). That submodule is
// deleted; every former consumer now imports `quire_exact::{evaluate_boolean,
// evaluate_integer_arithmetic, evaluate_rational_arithmetic, order_numbers}`
// directly.
// QSL-131 O2: `Outcome`, `Undefined` and `Refusal` were this module's own
// `outcome` submodule, a byte-identical duplicate of `quire_exact`'s O-16
// kernel types (ADR-011 §6.1's K row), re-exported from here. That
// submodule is deleted; every former consumer now imports
// `quire_exact::{Outcome, Undefined, Refusal}` directly -- one definition,
// one import path, no re-export standing in for the deleted module.
// `PreconditionFailure` was `outcome`'s own non-kernel type (FR-151 dispatch
// vocabulary, ADR-013 T-6); it moved to `value::expression::causes`, the
// module that owns the `StateModel` undefined cause it is the payload of,
// narrowed to `pub(crate)` since its only consumers are inside
// `value::expression` (rust-review "narrow API" bar: `pub` only for what
// consumers outside the crate use).
pub use quantity::{
    compare_quantity, convert_quantity, evaluate_quantity, Conversion, ConvertedValue,
    QuantityOperation, QuantityTarget, QuantityUnit, UnitQuantity, UnitTable,
};
// QSL-131 V5b deleted `value::rational`, a pure re-export with nothing else
// in it (confirmed clean in the V5 measurement); every former consumer now
// imports these three from `quire_exact` directly, same as it already did
// for `Rational` itself.
pub use quire_exact::{NonPositiveDenominatorBound, RationalDomain, ZeroDenominator};
pub use reference::{
    ObjectEnvironment, ObjectEnvironmentCause, ObjectEnvironmentRefusal, PopulationConflict,
};
pub use semantic_node::{
    InvalidSemanticGraph, ModelSubject, NodeIdentityPreimage, NodeOwner, OwnerSelection,
    OwnerSubject, SemanticGraphCause,
};
// QSL-131 O3: `admit_text`, `compare_text`, `Text`, `TextPayload` and
// `InvalidTextLiteral` were this module's own `text` submodule, a
// byte-identical duplicate of `quire_exact`'s except for
// `TextPayload::from_source_literal`'s JSON-decode convenience, which the
// kernel deliberately excludes (a source-lexer concern, not a kernel one --
// `quire_exact::text`'s module doc). That submodule is deleted; every
// former consumer now imports `quire_exact::{admit_text, compare_text, Text,
// TextPayload}` directly. `InvalidTextLiteral` has no successor: nothing
// outside this crate's own now-deleted tests called
// `TextPayload::from_source_literal`'s fallible JSON decode, so
// `tests/it/text_enum_identity.rs` keeps that one decode step as a
// test-local helper, calling the kernel's new
// `quire_exact::TextPayload::from_source_literal(text, spelling)`
// constructor (infallible, over already-decoded text) to tag the result.
pub use unit::{
    CompoundUnit, CompoundUnitCause, CompoundUnitPreimage, Dimension, DimensionPreimage,
    InvalidCompoundUnit, NotAUnitKey, Unit, UnitEdge, UnitGraph, UnitPreimage,
};
