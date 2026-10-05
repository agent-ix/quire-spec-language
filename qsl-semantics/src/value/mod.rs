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
//!    [`Text`](quire_exact::Text), `quire_semantic_value::enumeration::EnumValue` and FR-142
//!    `quire_exact::Quantity` values read against a
//!    `quire_semantic_value::quantity::UnitTable` over an admitted
//!    `quire_semantic_value::unit::UnitGraph`;
//! 2. explicit operation tables: [`evaluate_decimal`](quire_exact::evaluate_decimal)
//!    (FR-140), [`divide`] and [`modulo`] (FR-147),
//!    [`admit_text`](quire_exact::admit_text),
//!    [`compare_text`](quire_exact::compare_text) and `quire_semantic_value::enumeration::compare_enum`
//!    (FR-141), `quire_semantic_value::quantity`'s `evaluate_quantity` and
//!    `convert_quantity` (FR-142), after
//!    the type-checking [`IllTyped`](quire_exact::IllTyped) refusal;
//! 3. FR-143 records, tuples and finite recursive
//!    [`Value`](quire_exact::Value)s over a
//!    `quire_semantic_value::declaration::TypeEnvironment`, the FR-149 equality matrix
//!    (`TypeEnvironment::check_equality`) and FR-144 bounded
//!    [`CollectionValue`](quire_exact::CollectionValue)s
//!    ([`construct_collection`](quire_exact::construct_collection)) (QSL #119); FR-307
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
//!    types (this module's byte-identical copy is deleted);
//! 6. `quire.value.accounting/v1` metering through
//!    [`Meter`](quire_exact::Meter).
//!
//! FR-148 IEEE binary32/binary64 profiles
//! ([`evaluate_ieee`](quire_exact::evaluate_ieee),
//! [`compare_ieee`](quire_exact::compare_ieee)) operate on exact bit patterns
//! with soft-float arithmetic over big integers. This
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
//! The RT capability-negotiation copies `negotiate_ieee`,
//! `negotiate_integer_division` and their supporting types from `ieee` and
//! `division` (FR-078-AC-1, FR-078-AC-2); only the evaluation functions
//! listed above remain. Each removed name is gone from this crate, not
//! renamed or gated: importing it from `value` is `E0432`, an unresolved
//! import, not a type or borrow error.
//!
//! ```compile_fail,E0432
//! use qsl_semantics::value::negotiate_ieee;
//! ```
//! ```compile_fail,E0432
//! use qsl_semantics::value::negotiate_integer_division;
//! ```
//! ```compile_fail,E0432
//! use qsl_semantics::value::IeeeBackendCapabilities;
//! ```
//! ```compile_fail,E0432
//! use qsl_semantics::value::IeeeItemRequirement;
//! ```
//! ```compile_fail,E0432
//! use qsl_semantics::value::IeeeUnsupportedCause;
//! ```
//! ```compile_fail,E0432
//! use qsl_semantics::value::IeeeDisposition;
//! ```
//! ```compile_fail,E0432
//! use qsl_semantics::value::IntegerDivisionBounds;
//! ```
//! ```compile_fail,E0432
//! use qsl_semantics::value::IntegerDivisionConsumer;
//! ```
//! ```compile_fail,E0432
//! use qsl_semantics::value::IntegerDivisionDisposition;
//! ```
//!
//! A positive control: the module path itself still resolves, and a
//! surviving name imports cleanly, so the failures above are each the named
//! removed item, not a broken crate path.
//!
//! ```
//! use qsl_semantics::value::AdmittedIeeeProfile;
//! fn _use(_: AdmittedIeeeProfile) {}
//! ```

// ADR-011 §6.1 layer 3: the §6.2 `semantic_value` submodules and
// `value::model_query` (§6.2: `model`), with the flat re-exports of their own items. The layer-5 S6a
// evaluator, `value::expression`, is in the `qsl-eval` crate's own `value`
// module and imports these by their
// `qsl_semantics::value::<submodule>` path. The `pub` submodules are those
// it imports by submodule path: `enumeration`, `environment_stage` and
// `model_query`; `declaration`, `containment`, `quantity` and `stop` are in
// the `quire-semantic-value` crate (ADR-011 layer SV). `definition` and `semantic_node` stay
// `pub(crate)`: their consumers outside `value` are `check`, `model` and
// `library`, all in this crate. `decimal`, `ieee`,
// `numeric` and `text` are deleted; their former items are imported from `quire_exact`
// directly.

pub(crate) mod definition;
mod diagnostics_catalog;
pub mod enumeration;
pub mod environment_stage;
pub(crate) mod member;
pub mod model_query;
pub(crate) mod semantic_node;
pub(crate) mod unit;

// `ChargePoint`, `Incomplete`, `InjectedDenial`, `LimitKind`,
// `Meter`, `ScalarLimits`, `Charge` and `length_amount` were this module's
// own `accounting` submodule, re-exported from here. That submodule
// duplicated `quire_exact`'s accounting types byte-for-byte and is deleted;
// every former consumer (`crate::model::population` included, which reused
// this crate-wide re-export the same way FR-153 reuses this crate's own
// `quire.value.accounting/v1` meter for `lookup.*`/`population.visit`/
// `collection.*` charges) now imports `quire_exact::{..}` directly. One
// definition, one import path -- no re-export shim stands in for the
// deleted module.
// `CardinalityBound`/`EmptyCardinalityBound` are `quire_exact`'s own types; this
// module does not re-export them, so consumers import them from
// `quire_exact` directly.
// The kernel's `Value`, `ValueType`, collection, equality and rational items
// (ADR-011 §6.1's K row) are not re-exported here: every consumer imports
// them from `quire_exact`, the one definition (ADR-011 §7.2).
// `DecimalType`, `DecimalLoss`, `DecimalResult` and
// `evaluate_decimal` were this module's own `decimal` submodule, a
// byte-identical duplicate of `quire_exact`'s (V4 had already moved unit
// placement onto the kernel's `DecimalType::placement`; only the type and
// its evaluation engine were left). That submodule is deleted; every former
// consumer now imports `quire_exact::{DecimalType, DecimalLoss,
// DecimalResult, evaluate_decimal}` directly -- one definition, one import
// path, no re-export standing in for the deleted module.
// `enumeration` and `environment_stage` are `pub` modules, so their
// items have one public path, the submodule one
// (`value::environment_stage::stage_failure`); this module does not re-export
// them flat as well (the same one-path rule as for the kernel's items).
// `SelectionRefusalCode` and the `PackageRefusal` vocabulary are
// `quire_semantic_value::definition`'s (ADR-011 §6.1 layer SV); consumers
// import them from there, not through a re-export here.
pub use definition::{
    divide, modulo, AdmittedIeeeProfile, AdmittedIntegerDivision, AdmittedSelection, CatalogEntry,
    CatalogRole, DefinitionLock, DefinitionReference, Trigger,
};
pub use diagnostics_catalog::{native_diagnostics_catalog, NATIVE_DIAGNOSTICS_IDENTITY};
// `ExactScalar`, `IeeeOperand`, `IeeeProvenance`, `IeeeResult`,
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
// `evaluate_boolean`, `evaluate_integer_arithmetic`,
// `evaluate_rational_arithmetic` and `order_numbers` were this module's own
// `numeric` submodule, a byte-identical duplicate of `quire_exact`'s
// (`ArithmeticOperator`, `OrderingOperator`, `OrderedOperands`,
// `IntegerArithmetic`, `RationalArithmetic` and `BooleanConnective` were
// already re-exported straight from `quire_exact`). That submodule is
// deleted; every former consumer now imports `quire_exact::{evaluate_boolean,
// evaluate_integer_arithmetic, evaluate_rational_arithmetic, order_numbers}`
// directly.
// `Outcome`, `Undefined` and `Refusal` were this module's own
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
// `InvalidSemanticGraph` and `SemanticGraphCause` are
// `quire_semantic_value::semantic_node`'s (ADR-011 §6.1 layer SV); consumers
// import them from there, not through a re-export here.
pub use semantic_node::{
    identity_limit, ModelSubject, NodeIdentityPreimage, NodeOwner, NominalRefusal, OwnerSelection,
    OwnerSubject,
};
// `admit_text`, `compare_text`, `Text`, `TextPayload` and
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
// `value::unit` keeps the compile-side half of the unit graph: the preimage
// readers and admission. The runtime half (`Dimension`, `Unit`, `UnitEdge`,
// `UnitGraph`, `CompoundUnit`, `compound_unit_id`, `InvalidCompoundUnit`,
// `CompoundUnitCause`, `NotAUnitKey`) is `quire_semantic_value::unit`, which
// every caller names directly.
pub use unit::{admit_unit_graph, CompoundUnitPreimage, DimensionPreimage, UnitPreimage};
