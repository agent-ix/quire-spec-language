// SPDX-License-Identifier: AGPL-3.0-or-later
//! `quire-exact`: the QSL kernel row (QSL#213 S-1, ADR-011 X-1, ADR-013 §7
//! S-1).
//!
//! This crate is the AD-016/ADR-011 module-DAG leaf layer `K`: checked
//! identity ([`NodeKey`] and the seven opaque digest identities, such as
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
//! digest identity here ([`NodeKey`] and the seven digest identities,
//! [`EffectiveId`], [`UniverseId`], [`ObjectId`], [`UnitId`], [`VariantId`],
//! [`MemberId`], [`PopulationId`]) is minted by wrapping an already-computed
//! digest -- through its one public `from_digest` constructor, or for the
//! two-domain [`UnitId`] through its one constructor per domain (ADR-013
//! T-6, QC-22) -- never by hashing internally.
//!
//! [`Value`]/[`ValueType`]: `ValueType::admits` never pairs
//! `ValueType::Population(u64)` with `Value::Population(PopulationId)`
//! (ADR-013 O-13 Population row, QC-21, FR-089). This is not a capability
//! loss: FR-089-AC-5's declared-maximum comparison is a QSL-layer check --
//! the model/evaluator resolves a `PopulationId` to its binding and compares
//! the binding's own declared maximum there, work this leaf crate has no way
//! to do -- so kernel `admits` refuses every population pair outright,
//! falling through to its catch-all and returning `false` (the
//! `ValueType::Enum` shape, by contrast, carries its variant set inline per
//! ADR-013 O-14, so it needs no declaration lookup at all).
//!
//! Several real, deliberate capability losses at this kernel boundary are
//! documented where they occur rather than silently absorbed:
//! - the `key` and `equality` modules: `Value::Population` has no key and
//!   compares under neither, matching QSL's own `value::equality`/
//!   `value::key`, which refuse a population as an equality operand or key
//!   participant today -- a population binding is a direct operand of
//!   `allInstances`/`lookup` only, never an equality or key operand. (A
//!   same-enum check *is* still available: `ValueType::Enum(EnumShape)`'s
//!   admission already guarantees both operands share one enum's variant
//!   set before either module ever runs, per ADR-013 O-14.)
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
//! `rust-version = "1.98"`, and `cargo kani` separately fails on a
//! dependency's own fixture `Cargo.toml`. Building the gate is tracked
//! separately (QSL-130) and left to ADR-011 §2.3's own named enforcer, #219.
//!
//! **H-9: no acceptance criterion exists for most of this crate's own test
//! suite.** The exception is FR-089-AC-6 (TC-297): the three kernel
//! population-pair refusal tests in `value`, `equality` and `key` carry the
//! two-argument `#[trace("TC-297", "FR-089-AC-6")]` form. Every other
//! `#[trace("TC-3NN")]` tag here is the bare one-argument form,
//! against the repo's two-argument `#[trace("TC-NNN", "FR-NNN-AC-n")]`
//! convention, because there is no `FR-NNN-AC-n` to name: no `spec/`
//! functional requirement or acceptance criterion, no `spec/test-cases/
//! TC-3NN-*.md` file and no `spec/tests.md`/subsystem `tests.md` test
//! matrix row exists for the rest of `quire-exact`'s value-kernel behavior as of this
//! PR. This is stated here rather than left silent, and rather than bound
//! to an approximate existing FR (every FR found under `spec/functional/`
//! that mentions ADR-011/ADR-013 is about package/capability admission,
//! not value-kernel semantics -- binding these tests to one of those
//! would misrepresent what they actually verify). Authoring a real FR/AC
//! set and test matrix for this crate is a QSpec decision -- which
//! subsystem directory it belongs to, and whether criteria are authored
//! before or after the code they describe -- not something this PR
//! decides for itself.
//!
//! This crate's ids run `TC-300` to `TC-356`, but that range names 57 ids
//! for 56 tests: **`TC-343` is retired, not reused.** It named
//! `text::tests::tc_343_unquoted_literal_is_refused`, which tested only
//! `TextPayload::from_source_literal`'s malformed-input path; M-6 removed
//! `from_source_literal` from the kernel entirely (with no in-crate caller
//! outside that test), and the test went with it rather than being
//! repointed at unrelated behavior. Do not mint a new `TC-343` to fill the
//! hole -- an id that once named one thing should not silently come to
//! name another.

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
// `pub mod` wholesale. `crate::key::compare_keys` is deliberately absent
// below: it is reachable only from inside this crate. `rational::
// divided_by_power_of_ten`/`divided_by_power_of_two`, `decimal::
// DecimalRepresentation::to_rational`, `decimal::compare_shifted`,
// `decimal::power_of_ten_bits`/`sbits`/`sdigits`, `decimal::DecimalType::
// placement` with `Placement`/`Placed`/`Admitted`, `numeric::rational_arithmetic_bits`,
// `decimal::DecimalLoss::exact`/`exact_denominator` and
// `equality::plan_equality` are all `pub`
// and exported below: each is unmetered exact arithmetic or comparison --
// the same discipline `Integer`'s own arithmetic carries -- so a caller
// charges or bounds its inputs before calling any of them.
//
// QSL-146 moves the ledger the other direction: 17 `Integer`/
// `IntegerInterval` inherent methods (`integer.rs`'s own module doc names
// them) go from `pub(crate)` to `pub`, each verified against a real
// cross-crate call site in `quire_spec_language` (revert each in isolation,
// recompile `--workspace --all-targets --all-features`, confirm a genuine
// `E0624` at that method's own real callers -- 151 across the 17, not the
// 137 a first, non-isolated pass under-counted by masking 14 real call
// sites across six methods behind a cascading error earlier in the same
// expression (`add` +5, `neg` +3, `sub` +2, `mul` +2, `exact_div` +1,
// `shifted_left` +1). `Integer`
// and `IntegerInterval` were already exported below (this facade only
// gates the type; an inherent method's own `pub`/`pub(crate)` is not listed
// here separately) -- what changed is that their methods are now reachable
// through that existing export, not merely from inside this crate.
pub use accounting::{
    length_amount, Charge, ChargePoint, Incomplete, InjectedDenial, LimitKind, Meter, ScalarLimits,
};
pub use collection::{
    construct_collection, form_collection, form_grouped, from_admitted, CardinalityBound,
    CollectionKind, CollectionType, CollectionValue, EmptyCardinalityBound,
};
pub use comparison::{ComparisonOperator, IllTyped, IllTypedCause};
pub use decimal::{
    compare_shifted, evaluate_decimal, power_of_ten_bits, sbits, sdigits, Admitted, Decimal,
    DecimalLoss, DecimalOperation, DecimalRepresentation, DecimalResult, DecimalType, Placed,
    Placement, RoundingMode,
};
pub use division::{divide, modulo, DivisionProfile, QuotientRemainder};
pub use equality::{plan_equality, planned_equality, EqualityPlan};
pub use identity::{
    EffectiveId, EmptyObjectIdentity, MemberId, ObjectId, PopulationId, UnitDomain, UnitId,
    UniverseId, VariantId, COMPOUND_UNIT_DOMAIN, EFFECTIVE_ID_DOMAIN, MEMBER_ID_DOMAIN,
    POPULATION_ID_DOMAIN, UNIVERSE_ID_DOMAIN, VARIANT_ID_DOMAIN,
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
pub use node::{is_identifier, Identifier, InvalidIdentifier, NodeKey, NODE_KEY_DOMAIN};
pub use numeric::{
    evaluate_boolean, evaluate_integer_arithmetic, evaluate_rational_arithmetic, order_numbers,
    rational_arithmetic_bits, retain_boolean, ArithmeticOperator, BooleanConnective,
    IntegerArithmetic, OrderedOperands, OrderingOperator, RationalArithmetic,
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
    CompositeValue, ConstructionCause, ConstructionRefusal, Deferred, EnumMember, EnumShape,
    FieldDeclaration, FieldExpression, FieldValue, OptionValue, Presence, Value, ValueType,
};
