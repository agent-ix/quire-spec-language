// SPDX-License-Identifier: AGPL-3.0-or-later
//! The four distinct evaluator outcomes of AD-005.
//!
//! A false Boolean is a completed value, never a refusal. `requires-bound` and
//! `unsupported` are per-item I13 provider dispositions, not evaluator outcomes,
//! so they have no variant here.

use super::ieee::IeeeFlags;
use super::reference::ObjectReference;
use crate::check::WrongSnapshotCause;
use qsl_foundation::diagnostic::{
    CatalogCode, CatalogCoded, UndefinedCoded, UndefinedReason, UndefinedRecord,
};
use quire_exact::{CardinalityBound, CollectionKind, Incomplete};
use std::collections::BTreeMap;

/// Exactly one of a completed value, undefined, refused or incomplete.
#[derive(Clone, Debug, Eq, PartialEq)]
#[must_use]
pub enum Outcome<T> {
    /// A completed value with its provenance and any typed loss.
    Completed(T),
    /// The operation has no mathematical value.
    Undefined(Undefined),
    /// The operation is defined but its result is not admitted.
    Refused(Refusal),
    /// A named charge was unavailable; no partial value exists.
    Incomplete(Incomplete),
}

impl<T> Outcome<T> {
    /// The completed value, if any.
    pub fn completed(self) -> Option<T> {
        match self {
            Self::Completed(value) => Some(value),
            Self::Undefined(_) | Self::Refused(_) | Self::Incomplete(_) => None,
        }
    }
}

impl<T> Outcome<T> {
    /// FR-090-AC-1: this QSL kernel-copy outcome, mapped onto the real
    /// `quire_exact::Outcome<T>` `FamilyOutcome::Evaluated`/`EvalOutcome::
    /// Kernel` carry (ADR-013 O-16). A total, one-to-one re-tagging, not a
    /// migration: since `ProtocolClause`'s `wrong_snapshot`, the
    /// model-query refusal, `precondition-false` and `absent-key` are all
    /// intercepted earlier (`Halt::Family`, `EvalHalt::Family`) and never
    /// reach this copy's `Refused`/`Undefined` any more (ADR-013 T-6), the
    /// two enums are already structurally identical to their kernel
    /// counterparts, member for member -- QSL-131's remaining work is
    /// deleting this copy in favor of the kernel type directly, not
    /// reconciling a shape difference.
    pub(crate) fn into_kernel(self) -> quire_exact::Outcome<T> {
        match self {
            Self::Completed(value) => quire_exact::Outcome::Completed(value),
            Self::Undefined(undefined) => quire_exact::Outcome::Undefined(undefined.into_kernel()),
            Self::Refused(refusal) => quire_exact::Outcome::Refused(refusal.into_kernel()),
            Self::Incomplete(record) => quire_exact::Outcome::Incomplete(record),
        }
    }
}

impl Undefined {
    fn into_kernel(self) -> quire_exact::Undefined {
        match self {
            Self::DivisionByZero => quire_exact::Undefined::DivisionByZero,
            Self::IeeeNotFinite => quire_exact::Undefined::IeeeNotFinite,
            Self::EmptyReduction => quire_exact::Undefined::EmptyReduction,
            Self::NoneValue => quire_exact::Undefined::NoneValue,
        }
    }
}

impl Refusal {
    fn into_kernel(self) -> quire_exact::Refusal {
        match self {
            Self::InexactDecimal => quire_exact::Refusal::InexactDecimal,
            Self::DecimalOutOfDomain => quire_exact::Refusal::DecimalOutOfDomain,
            Self::DivisionPairOutOfDomain {
                quotient_admitted,
                remainder_admitted,
            } => quire_exact::Refusal::DivisionPairOutOfDomain {
                quotient_admitted,
                remainder_admitted,
            },
            Self::ModuloOutOfDomain => quire_exact::Refusal::ModuloOutOfDomain,
            Self::TextLengthOutOfDomain => quire_exact::Refusal::TextLengthOutOfDomain,
            Self::IntegerOutOfDomain => quire_exact::Refusal::IntegerOutOfDomain,
            Self::RationalOutOfDomain => quire_exact::Refusal::RationalOutOfDomain,
            Self::IeeeNotExact { would_be } => quire_exact::Refusal::IeeeNotExact { would_be },
            Self::IeeeNanPayloadNotRepresentable => {
                quire_exact::Refusal::IeeeNanPayloadNotRepresentable
            }
            Self::IeeeRationalOutOfDomain => quire_exact::Refusal::IeeeRationalOutOfDomain,
            Self::ForeignReference => quire_exact::Refusal::ForeignReference,
            Self::CardinalityOutOfBound {
                violation,
                kind,
                bound,
                count,
            } => quire_exact::Refusal::CardinalityOutOfBound {
                violation: violation.into_kernel(),
                kind,
                bound,
                count,
            },
            Self::CheckedInvariant => quire_exact::Refusal::CheckedInvariant,
        }
    }
}

impl BoundViolation {
    fn into_kernel(self) -> quire_exact::BoundViolation {
        match self {
            Self::BelowMinimum => quire_exact::BoundViolation::BelowMinimum,
            Self::AboveMaximum => quire_exact::BoundViolation::AboveMaximum,
        }
    }
}

impl<T> Outcome<T> {
    pub(crate) fn from_stop(result: Result<T, Stop>) -> Self {
        match result {
            Ok(value) => Self::Completed(value),
            Err(Stop::Undefined(reason)) => Self::Undefined(reason),
            Err(Stop::Refused(reason)) => Self::Refused(reason),
            Err(Stop::Incomplete(record)) => Self::Incomplete(record),
        }
    }

    pub(crate) fn into_stop(self) -> Result<T, Stop> {
        match self {
            Self::Completed(value) => Ok(value),
            Self::Undefined(reason) => Err(Stop::Undefined(reason)),
            Self::Refused(reason) => Err(Stop::Refused(reason)),
            Self::Incomplete(record) => Err(Stop::Incomplete(record)),
        }
    }
}

/// Why an operation is undefined.
///
/// **`AbsentKey` and `PreconditionFalse` are removed (ADR-013 T-6,
/// FR-090-AC-11/AC-12).** Both were family-owned undefined causes sitting
/// in the kernel copy's own closed set; O-16 keeps model/dispatch
/// vocabulary out of the kernel entirely. `StateModelUndefined` (this
/// module) is their real replacement, carried in `FamilyResult::Undefined`
/// through `CatalogCoded`'s undefined-side counterpart, `UndefinedCoded`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Undefined {
    /// A divisor is (normalized) zero.
    DivisionByZero,
    /// An IEEE NaN or infinity has no exact value.
    IeeeNotFinite,
    /// FR-145: `reduce` over an empty collection has no value. Only direct
    /// kernel evaluation of an unlinked expression can meet it.
    EmptyReduction,
    /// `value(e)` of `none`. Only direct kernel evaluation of an unlinked
    /// expression can meet it.
    NoneValue,
}

/// The `precondition-false` payload (`native-diagnostics.md`): the called
/// effective operation, the selected method's effective identity, the
/// receiver reference and the call locus. The call locus is the evaluator's
/// own [`crate::value::Evaluation::location`], not
/// repeated here.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PreconditionFailure {
    /// The called effective operation's unqualified member name.
    pub operation: String,
    /// The selected method's effective identity: the redefinition candidate
    /// the receiver's most-specific runtime type actually linked to.
    pub selected: String,
    /// The receiver reference the call was made on.
    pub receiver: ObjectReference,
}

/// Why a defined result is refused. Refusals never carry the refused value.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Refusal {
    /// Strict `exact` rounding would discard a nonzero digit.
    InexactDecimal,
    /// The normalized decimal coefficient is outside the target domain.
    DecimalOutOfDomain,
    /// At least one member of a quotient/remainder pair is outside the consumer
    /// domain; neither member is exposed.
    DivisionPairOutOfDomain {
        /// Whether the quotient is a domain member.
        quotient_admitted: bool,
        /// Whether the remainder is a domain member.
        remainder_admitted: bool,
    },
    /// The Euclidean `mod` remainder is outside the consumer domain.
    ModuloOutOfDomain,
    /// The profile length (scalars, or bytes for `binary-utf8`) is outside the
    /// declared `Text[min,max; profile]` bounds.
    TextLengthOutOfDomain,
    /// An exact conversion or arithmetic result is outside the target integer
    /// domain.
    IntegerOutOfDomain,
    /// An exact rational arithmetic result is outside its `Rational[..]`
    /// result domain.
    RationalOutOfDomain,
    /// Strict IEEE `exact` found an inexact, overflowing or tiny-and-inexact
    /// result; only its would-be flags are reported, never rounded bits.
    IeeeNotExact {
        /// The flags the rounded result would have raised.
        would_be: IeeeFlags,
    },
    /// A NaN payload does not fit the explicit conversion's target width.
    IeeeNanPayloadNotRepresentable,
    /// An exact rational converted from an IEEE value is outside the
    /// `Rational[..]` target domain.
    IeeeRationalOutOfDomain,
    /// An FR-149 comparison met two references of different universes.
    ForeignReference,
    /// A formed collection's bound count is outside its declared bound; no
    /// collection is materialized.
    CardinalityOutOfBound {
        /// Which side of the bound is violated.
        violation: BoundViolation,
        /// The collection kind of the declared type.
        kind: CollectionKind,
        /// The declared inclusive bound.
        bound: CardinalityBound,
        /// The formed bound count: occurrences for a sequence or bag, members
        /// for a set or ordered set.
        count: u64,
    },
    /// A checked-program invariant failed during evaluation; unreachable for
    /// an admitted program.
    CheckedInvariant,
    // `WrongSnapshot(WrongSnapshotCause)` and `Model(ModelQueryRefusal)` are
    // removed (ADR-013 T-6, FR-090-AC-7/AC-8). Both were evaluation-time
    // causes wrapped in the kernel-shaped `Refused` outcome; O-16 keeps
    // family evaluation causes out of the kernel entirely; ADR-013 T-6
    // states the removal explicitly ("`WrongSnapshotCause` leaves the
    // kernel `Refusal`... it becomes an evaluation cause of
    // `ProtocolClause`"). `ProtocolClauseSnapshot` (this module) and
    // `crate::model::normalize::ModelRefusal`'s own `CatalogCoded` impl are
    // their real replacements, carried in `FamilyResult::Refused`.
    //
    // FR-089-AC-4/AC-5's `UnresolvedPopulation`/`PopulationMaximumMismatch`
    // variants are also deleted (FR-090-AC-10, ADR-013 T-4): a consumed
    // `Value::Population(population_id)` that names no recorded binding, or
    // whose resolved binding's declared maximum differs from the checked
    // parameter's, is refused at admission (`CallFailure::Input`,
    // `CheckedPackage::call`/`evaluate`'s own `validate`) before S6a ever
    // runs; meeting either condition inside S6a (`Machine::
    // resolve_population`, `expression/evaluate.rs`) is now an
    // `InternalFault`, never a kernel `Refused` outcome -- so this public
    // `Refusal` enum has no variant left for any of the three cases.
}

impl Refusal {
    /// The closed `refused { code }` spelling, where the language defines one.
    pub fn code(self) -> Option<&'static str> {
        match self {
            Self::IeeeNanPayloadNotRepresentable => Some("ieee_nan_payload_not_representable"),
            Self::IeeeRationalOutOfDomain => Some("ieee_rational_out_of_domain"),
            Self::ForeignReference => Some("foreign_reference"),
            Self::CardinalityOutOfBound { .. } => Some("cardinality_out_of_bound"),
            Self::InexactDecimal
            | Self::DecimalOutOfDomain
            | Self::DivisionPairOutOfDomain { .. }
            | Self::ModuloOutOfDomain
            | Self::TextLengthOutOfDomain
            | Self::IntegerOutOfDomain
            | Self::RationalOutOfDomain
            | Self::IeeeNotExact { .. }
            | Self::CheckedInvariant => None,
        }
    }

    /// The closed FR-272 `cause` tag, where the code has one.
    pub fn cause(self) -> Option<&'static str> {
        match self {
            Self::CardinalityOutOfBound { violation, .. } => Some(violation.as_str()),
            Self::InexactDecimal
            | Self::DecimalOutOfDomain
            | Self::DivisionPairOutOfDomain { .. }
            | Self::ModuloOutOfDomain
            | Self::TextLengthOutOfDomain
            | Self::IntegerOutOfDomain
            | Self::RationalOutOfDomain
            | Self::IeeeNotExact { .. }
            | Self::IeeeNanPayloadNotRepresentable
            | Self::IeeeRationalOutOfDomain
            | Self::ForeignReference
            | Self::CheckedInvariant => None,
        }
    }
}

/// The side of a cardinality bound a formed collection violates.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BoundViolation {
    /// `below-minimum`.
    BelowMinimum,
    /// `above-maximum`.
    AboveMaximum,
}

impl BoundViolation {
    /// The FR-272 cause tag.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BelowMinimum => "below-minimum",
            Self::AboveMaximum => "above-maximum",
        }
    }
}

/// ADR-013 T-6: `ProtocolClause`'s own evaluation-time refusal cause,
/// carrying [`WrongSnapshotCause`] -- `ProtocolClause` is the family that
/// owns `Pre` (ADR-012 §4.3; FR-091-AC-8). Defined here, beside
/// [`crate::value::expression::evaluate::Machine::select_anchor`], the one
/// evaluation-time production site: only [`WrongSnapshotCause::WrongAnchor`]
/// is reachable there, since [`WrongSnapshotCause::ForbiddenPreRead`] is a
/// checking-time-only cause (`crate::check::check`). `catalog_code()`
/// (TC-387) is exhaustive over both variants regardless, matching O-17's
/// "one exhaustive `catalog_code()`" for the whole cause type.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ProtocolClauseSnapshot(pub(crate) WrongSnapshotCause);

impl CatalogCoded for ProtocolClauseSnapshot {
    fn catalog_code(&self) -> CatalogCode {
        match self.0 {
            WrongSnapshotCause::WrongAnchor => CatalogCode::new("wrong_snapshot", "wrong-anchor"),
            WrongSnapshotCause::ForbiddenPreRead => {
                CatalogCode::new("wrong_snapshot", "forbidden-pre-read")
            }
        }
    }
}

/// The identity string a catalog payload field carries for a member: every
/// real population member's own identity is a JSON string
/// (`crate::value::model_query`'s own doc), rendered lossily only for a
/// malformed reference no admitted program produces.
pub(crate) fn identity_string(identity: &[u8]) -> String {
    String::from_utf8_lossy(identity).into_owned()
}

/// ADR-013 O-16: the `StateModel` family's evaluation-time undefined cause
/// (FR-090-AC-11/AC-12). ADR-012 assigns dispatch, dispatch preconditions
/// and population lookup to `StateModel` (ADR-012 §1, §3, §4.3), even
/// though both evaluators run inside `Value`'s own
/// [`crate::value::expression::evaluate::Machine`] -- layer 5
/// `value::expression` holds every family's evaluator (ADR-011 §6.1).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum StateModelUndefined {
    /// FR-151 (TC-196 D06): a dispatched `receiver.member(args)` call's
    /// selected method's effective precondition evaluated to `false`.
    PreconditionFalse(PreconditionFailure),
    /// FR-153: a `lookup<T>(p, r) absent undefined` query's reference `r`
    /// names no member of the population bound to `p`.
    AbsentKey {
        /// The population binding `p` is bound to, rendered for the
        /// catalog payload.
        binding: String,
        /// The requested reference key `r`, rendered for the catalog
        /// payload.
        key: String,
    },
}

impl UndefinedCoded for StateModelUndefined {
    fn undefined_record(&self) -> UndefinedRecord {
        match self {
            Self::PreconditionFalse(failure) => UndefinedRecord {
                reason: UndefinedReason::PreconditionFalse,
                fields: BTreeMap::from([
                    ("operation", failure.operation.clone()),
                    ("selected", failure.selected.clone()),
                    (
                        "receiver",
                        identity_string(failure.receiver.identity().as_bytes()),
                    ),
                ]),
            },
            Self::AbsentKey { binding, key } => UndefinedRecord {
                reason: UndefinedReason::AbsentKey,
                fields: BTreeMap::from([("binding", binding.clone()), ("key", key.clone())]),
            },
        }
    }
}

/// The evaluator's own early-exit carrier, extended with the family-owned
/// evaluation-time result a handful of `value/*.rs` computations produce
/// (FR-090-AC-7/AC-8/AC-11/AC-12): [`Stop`] alone cannot carry a
/// `FamilyResult`, since it stays shared by roughly two dozen kernel-shaped
/// computations that all convert through [`Outcome::from_stop`] and must
/// never gain a non-kernel-shaped arm (see [`Stop`]'s own doc). `Machine`'s
/// crate-private `Halt` converts an `EvalHalt::Family` the same way it
/// converts an S6a invariant break: intercepted before anything calls
/// `Outcome::from_stop`.
pub(crate) enum EvalHalt {
    /// An ordinary evaluator stop, to be converted to an `Outcome` as usual.
    Stop(Stop),
    /// A family-owned evaluation-time refusal or undefined result.
    Family(crate::family::FamilyResult),
}

impl From<Stop> for EvalHalt {
    fn from(stop: Stop) -> Self {
        Self::Stop(stop)
    }
}

impl From<Incomplete> for EvalHalt {
    fn from(record: Incomplete) -> Self {
        Self::Stop(Stop::Incomplete(record))
    }
}

/// Internal early-exit carrier, always converted into an [`Outcome`] by
/// [`Outcome::from_stop`]. An S6a invariant break (FR-090-AC-3/AC-10,
/// ADR-013 T-4) is never a `Stop`: this type is shared by roughly two dozen
/// `value/*.rs` computations that all convert through `from_stop`, so a
/// fault variant here would be reachable from every one of them with no
/// compiler-checked guarantee that `Machine::run` intercepts it first (PR
/// #334 review round 2, finding N1) -- `Machine`'s own crate-private `Halt`
/// (`value::expression::evaluate.rs`) carries a fault instead, and no
/// `From<Halt> for Stop`/`Outcome` conversion exists, so a fault is
/// unrepresentable here by construction, not by convention.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Stop {
    Undefined(Undefined),
    Refused(Refusal),
    Incomplete(Incomplete),
}

impl From<Incomplete> for Stop {
    fn from(record: Incomplete) -> Self {
        Self::Incomplete(record)
    }
}
