// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-277's supplied-value phase budgets (the vocabulary the S6a checked call
//! and the replay facade share): the caller-owned
//! `supplied.admission_work_units` (A) and `supplied.conversion_work_units`
//! (H), the numeric magnitude bound `supplied.numeric_magnitude_bits` (B),
//! the phase-marked pre-call stop, and the run that carries both
//! budgets and the original [`Cancel`] through every supplied argument.
//!
//! The units are logical helper events, not evaluator charges: a pre-call
//! stop reports zero evaluation consumption and has no kernel charge point
//! (FR-277 "Projection into existing QSL carriers"). Membership events are
//! QSV's (`quire_semantic_value::supplied_membership`); conversion events
//! are QSL's (FR-098 "Conversion helper event units"). Each event has exactly
//! one initiating-phase owner.

use quire_exact::{Cancel, NumericMembershipOperand, Value, ValueType};
use quire_semantic_value::call::InputRefusal;
use quire_semantic_value::declaration::{StorageRequest, StorageUnit, TypeEnvironment};
use quire_semantic_value::supplied_membership::{
    BoundedSuppliedAdmissionError, InvalidMembershipCause, MembershipEvent, MembershipLocus,
    MembershipPart, MembershipPathArena, PathArenaCapacity, PathArenaCapacityUnit,
    SuppliedAdmissionError, SuppliedMembershipBudget,
};
use quire_semantic_value::supplied_stop::{
    AttemptedCount, SuppliedCause, SuppliedContext, SuppliedEvent, SuppliedFailure, SuppliedLimit,
    SuppliedLimitKind, SuppliedLocus, SuppliedNumericOperand, SuppliedPhase, SuppliedStop,
    SuppliedStorage, SuppliedStorageUnit, UnsupportedHelperStop,
};

/// FR-277: the published default of each supplied helper ceiling.
pub const DEFAULT_SUPPLIED_WORK_UNITS: u64 = 16_777_216;

/// FR-277: the published default of B, the numeric magnitude bound in bits.
pub const DEFAULT_NUMERIC_MAGNITUDE_BITS: u64 = 4096;

/// The typed supplied-value limits (FR-277, QSpec FR-323 `limits.supplied`):
/// two independent caller work ceilings, each defaulting to
/// [`DEFAULT_SUPPLIED_WORK_UNITS`], and the numeric magnitude bound B,
/// defaulting to [`DEFAULT_NUMERIC_MAGNITUDE_BITS`]. They are not
/// setting-table rows and never travel in a replay request's `stage_limits`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SuppliedLimits {
    admission_work_units: u64,
    conversion_work_units: u64,
    numeric_magnitude_bits: u64,
}

impl Default for SuppliedLimits {
    fn default() -> Self {
        Self {
            admission_work_units: DEFAULT_SUPPLIED_WORK_UNITS,
            conversion_work_units: DEFAULT_SUPPLIED_WORK_UNITS,
            numeric_magnitude_bits: DEFAULT_NUMERIC_MAGNITUDE_BITS,
        }
    }
}

impl SuppliedLimits {
    /// These limits with A set to `units`; H is unchanged.
    #[must_use]
    pub const fn with_admission_work_units(mut self, units: u64) -> Self {
        self.admission_work_units = units;
        self
    }

    /// These limits with H set to `units`; A is unchanged.
    #[must_use]
    pub const fn with_conversion_work_units(mut self, units: u64) -> Self {
        self.conversion_work_units = units;
        self
    }

    /// These limits with B set to `bits`; A and H are unchanged.
    #[must_use]
    pub const fn with_numeric_magnitude_bits(mut self, bits: u64) -> Self {
        self.numeric_magnitude_bits = bits;
        self
    }

    /// B: `supplied.numeric_magnitude_bits`.
    #[must_use]
    pub const fn numeric_magnitude_bits(&self) -> u64 {
        self.numeric_magnitude_bits
    }

    /// A: `supplied.admission_work_units`.
    #[must_use]
    pub const fn admission_work_units(&self) -> u64 {
        self.admission_work_units
    }

    /// H: `supplied.conversion_work_units`.
    #[must_use]
    pub const fn conversion_work_units(&self) -> u64 {
        self.conversion_work_units
    }

    /// The ceiling of `phase`'s helper budget.
    #[must_use]
    pub const fn of(&self, phase: SuppliedPhase) -> u64 {
        match phase {
            SuppliedPhase::Admission => self.admission_work_units,
            SuppliedPhase::Conversion => self.conversion_work_units,
        }
    }
}

/// The event a membership event is charged as.
const fn event_of_membership(event: MembershipEvent) -> SuppliedEvent {
    match event {
        MembershipEvent::ValueVisit => SuppliedEvent::ValueVisit,
        MembershipEvent::TypeLinkComparison => SuppliedEvent::TypeComparison,
        MembershipEvent::ChildScheduling => SuppliedEvent::ChildScheduling,
        MembershipEvent::DescriptorQuery => SuppliedEvent::DescriptorQuery,
    }
}

/// The event a membership locus part is charged as.
const fn event_of_part(part: MembershipPart) -> SuppliedEvent {
    match part {
        MembershipPart::Value => SuppliedEvent::ValueVisit,
        MembershipPart::TypeLink(_) => SuppliedEvent::TypeComparison,
        MembershipPart::Child(_) => SuppliedEvent::ChildScheduling,
        MembershipPart::Descriptor(_) => SuppliedEvent::DescriptorQuery,
    }
}

/// The measured storage request of a denied or overflowing reservation.
fn storage_of_request(request: StorageRequest) -> SuppliedStorage {
    SuppliedStorage {
        requested: u64::try_from(request.amount()).unwrap_or(u64::MAX),
        unit: match request.unit() {
            StorageUnit::Bytes => SuppliedStorageUnit::Bytes,
            StorageUnit::AdditionalElements => SuppliedStorageUnit::AdditionalElements,
        },
    }
}

/// The measured arena-identifier request of an unrepresentable path arena.
fn storage_of_arena(capacity: PathArenaCapacity) -> SuppliedStorage {
    let PathArenaCapacityUnit::ArenaIdentifiers = capacity.unit;
    SuppliedStorage {
        requested: u64::try_from(capacity.requested).unwrap_or(u64::MAX),
        unit: SuppliedStorageUnit::ArenaIdentifiers,
    }
}

/// Both phase budgets, the rooted-path arena and the original [`Cancel`] of
/// one supplied invocation (FR-277). One run covers every supplied argument
/// and nested helper: nothing resets between arguments, and the handle is
/// never replaced.
#[derive(Debug)]
pub struct SuppliedRun {
    numeric_magnitude_bits: u64,
    admission: SuppliedMembershipBudget,
    conversion: SuppliedMembershipBudget,
    arena: MembershipPathArena,
    cancel: Cancel,
    stopped: Option<SuppliedFailure>,
    #[cfg(feature = "test-support")]
    permit: Option<ReservationPermit>,
}

/// The test-only answer to each measured admission reservation.
#[cfg(feature = "test-support")]
struct ReservationPermit(Box<dyn Fn(StorageRequest) -> bool>);

#[cfg(feature = "test-support")]
impl core::fmt::Debug for ReservationPermit {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("ReservationPermit")
    }
}

impl SuppliedRun {
    /// A run at `limits` over the caller's original `cancel`.
    ///
    /// # Errors
    ///
    /// A capacity stop when the process has no path-arena identifier left.
    pub fn new(limits: SuppliedLimits, cancel: Cancel) -> Result<Self, Box<SuppliedStop>> {
        let arena = MembershipPathArena::new().map_err(|capacity| {
            Box::new(SuppliedStop {
                context: SuppliedContext {
                    phase: SuppliedPhase::Admission,
                    parameter: None,
                    event: None,
                    locus: None,
                    consumed: 0,
                },
                cause: SuppliedCause::Capacity(storage_of_arena(capacity)),
            })
        })?;
        Ok(Self {
            numeric_magnitude_bits: limits.numeric_magnitude_bits,
            admission: SuppliedMembershipBudget::new(limits.admission_work_units),
            conversion: SuppliedMembershipBudget::new(limits.conversion_work_units),
            arena,
            cancel,
            stopped: None,
            #[cfg(feature = "test-support")]
            permit: None,
        })
    }

    /// Answer every admission reservation with `permit` instead of the global
    /// reservations: a `false` answer is a denied reservation. Public only
    /// under `test-support`; the production path always uses the global
    /// reservations.
    #[cfg(feature = "test-support")]
    #[must_use]
    pub fn with_reservation_permit(
        mut self,
        permit: impl Fn(StorageRequest) -> bool + 'static,
    ) -> Self {
        self.permit = Some(ReservationPermit(Box::new(permit)));
        self
    }

    /// B: the numeric magnitude bound every Integer and Rational operand of
    /// this run is checked against.
    #[must_use]
    pub const fn numeric_magnitude_bits(&self) -> u64 {
        self.numeric_magnitude_bits
    }

    /// The `limit` stop of a numeric operand over B found by a conversion
    /// domain comparison at the witness node `locus`, charged to the
    /// conversion event `event` (FR-277: the stop is on the charged event).
    pub fn conversion_magnitude_stop(
        &mut self,
        parameter: usize,
        event: SuppliedEvent,
        locus: SuppliedLocus,
        operand: NumericMembershipOperand,
        measured_bits: u64,
    ) -> SuppliedFailure {
        let failure = SuppliedStop {
            context: SuppliedContext {
                phase: SuppliedPhase::Conversion,
                parameter: Some(parameter),
                event: Some(event),
                locus: Some(locus),
                consumed: 0,
            },
            cause: SuppliedCause::Limit(SuppliedLimit {
                limit_kind: SuppliedLimitKind::NumericMagnitudeBits,
                limit: self.numeric_magnitude_bits,
                next_charge: measured_bits,
                counter: AttemptedCount::Count(measured_bits),
                operand: Some(SuppliedNumericOperand::Value(operand)),
            }),
        }
        .into();
        self.remember(failure)
    }

    /// The first stop of this run with its full phase context, whichever
    /// cause: the diagnostic context that accompanies the existing
    /// invalid-input and cancellation failures of the checked call, which
    /// keep their own arms (FR-277 "Projection into existing QSL carriers").
    #[must_use]
    pub const fn failure(&self) -> Option<&SuppliedFailure> {
        self.stopped.as_ref()
    }

    fn remember(&mut self, failure: SuppliedFailure) -> SuppliedFailure {
        if self.stopped.is_none() {
            self.stopped = Some(failure.clone());
        }
        failure
    }

    /// The original cancellation handle.
    #[must_use]
    pub const fn cancel(&self) -> &Cancel {
        &self.cancel
    }

    /// A's successful cumulative spend.
    #[must_use]
    pub const fn admission_spent(&self) -> u64 {
        self.admission.spent()
    }

    /// H's successful cumulative spend.
    #[must_use]
    pub const fn conversion_spent(&self) -> u64 {
        self.conversion.spent()
    }

    /// The successful cumulative spend of `phase`'s budget.
    #[must_use]
    pub const fn spent(&self, phase: SuppliedPhase) -> u64 {
        match phase {
            SuppliedPhase::Admission => self.admission.spent(),
            SuppliedPhase::Conversion => self.conversion.spent(),
        }
    }

    /// Admit `value` against `expected` for parameter `parameter` under the
    /// initiating `phase`'s cumulative budget, one QSV supplied-membership
    /// event at a time.
    ///
    /// # Errors
    ///
    /// The typed stop of the first denied, cancelled, invalid, unreservable
    /// or unqualified event.
    pub fn admit(
        &mut self,
        phase: SuppliedPhase,
        parameter: usize,
        types: &TypeEnvironment,
        expected: &ValueType,
        value: &Value,
    ) -> Result<(), SuppliedFailure> {
        let at = MembershipLocus {
            path: self.arena.root(parameter),
            part: MembershipPart::Value,
        };
        let cancel = self.cancel.clone();
        let result = {
            let Self {
                admission,
                conversion,
                arena,
                numeric_magnitude_bits,
                #[cfg(feature = "test-support")]
                permit,
                ..
            } = self;
            let numeric_magnitude_bits = *numeric_magnitude_bits;
            let budget = match phase {
                SuppliedPhase::Admission => admission,
                SuppliedPhase::Conversion => conversion,
            };
            #[cfg(feature = "test-support")]
            let injected = permit.as_ref();
            #[cfg(feature = "test-support")]
            if let Some(ReservationPermit(permit)) = injected {
                types.admit_supplied_bounded_numeric_with_reservation::<InvalidMembershipCause>(
                    expected,
                    value,
                    arena,
                    at,
                    budget,
                    &cancel,
                    numeric_magnitude_bits,
                    permit,
                )
            } else {
                types.admit_supplied_bounded_numeric::<InvalidMembershipCause>(
                    expected,
                    value,
                    arena,
                    at,
                    budget,
                    &cancel,
                    numeric_magnitude_bits,
                )
            }
            #[cfg(not(feature = "test-support"))]
            types.admit_supplied_bounded_numeric::<InvalidMembershipCause>(
                expected,
                value,
                arena,
                at,
                budget,
                &cancel,
                numeric_magnitude_bits,
            )
        };
        let outcome = match result {
            Ok(()) => return Ok(()),
            Err(error) => self.bounded_failure_of(phase, parameter, error),
        };
        Err(self.remember(outcome))
    }

    /// The typed stop of one bounded-admission error, every arm named.
    fn bounded_failure_of(
        &self,
        phase: SuppliedPhase,
        parameter: usize,
        error: BoundedSuppliedAdmissionError<InvalidMembershipCause>,
    ) -> SuppliedFailure {
        match error {
            BoundedSuppliedAdmissionError::Admission(error) => {
                self.failure_of(phase, parameter, error)
            }
            BoundedSuppliedAdmissionError::NumericSize {
                operand,
                measured_bits,
                locus,
                ..
            } => self.magnitude_stop(
                phase,
                parameter,
                SuppliedEvent::ValueVisit,
                SuppliedNumericOperand::Value(operand),
                measured_bits,
                locus,
            ),
            BoundedSuppliedAdmissionError::DecimalSize {
                operand,
                measured_bits,
                locus,
                ..
            } => self.magnitude_stop(
                phase,
                parameter,
                SuppliedEvent::ValueVisit,
                SuppliedNumericOperand::Decimal(operand),
                measured_bits,
                locus,
            ),
            BoundedSuppliedAdmissionError::DecimalCapacity {
                magnitude_bits,
                locus,
            } => self.storage_stop(
                phase,
                parameter,
                locus,
                SuppliedCause::Capacity(SuppliedStorage {
                    requested: magnitude_bits,
                    unit: SuppliedStorageUnit::MagnitudeBits,
                }),
            ),
            BoundedSuppliedAdmissionError::DecimalAllocation {
                layout_bytes,
                locus,
                ..
            } => self.storage_stop(
                phase,
                parameter,
                locus,
                SuppliedCause::Storage(SuppliedStorage {
                    requested: u64::try_from(layout_bytes).unwrap_or(u64::MAX),
                    unit: SuppliedStorageUnit::Bytes,
                }),
            ),
            BoundedSuppliedAdmissionError::NumericTypeSize {
                position,
                measured_bits,
                locus,
                ..
            } => self.magnitude_stop(
                phase,
                parameter,
                SuppliedEvent::TypeComparison,
                SuppliedNumericOperand::TypeLink(position),
                measured_bits,
                locus,
            ),
        }
    }

    /// Charge one QSL-owned conversion event to the conversion budget before
    /// its work, polling the original [`Cancel`].
    ///
    /// # Errors
    ///
    /// The limit or cancellation stop of the event, with `event` and `locus`
    /// as its context.
    pub fn charge_conversion(
        &mut self,
        parameter: usize,
        event: SuppliedEvent,
        locus: SuppliedLocus,
    ) -> Result<(), Box<SuppliedStop>> {
        // QSV's budget is the one counter for conversion work. Its event
        // enum names only QSV's own four events, so a QSL-owned event is
        // charged as a value-visit-shaped unit and named back here: the
        // label is never read from the budget's reply.
        let at = MembershipLocus {
            path: self.arena.root(parameter),
            part: MembershipPart::Value,
        };
        let cancel = self.cancel.clone();
        let spent = self.conversion.spent();
        let ceiling = self.conversion.ceiling();
        let context = |consumed| SuppliedContext {
            phase: SuppliedPhase::Conversion,
            parameter: Some(parameter),
            event: Some(event),
            locus: Some(locus.clone()),
            consumed,
        };
        let charged = self
            .conversion
            .charge::<InvalidMembershipCause>(MembershipEvent::ValueVisit, at, &cancel)
            .map_err(|error| {
                Box::new(match error {
                    SuppliedAdmissionError::Cancelled { cause, spent, .. } => SuppliedStop {
                        context: context(spent),
                        cause: SuppliedCause::Cancelled(cause),
                    },
                    // `charge` produces only a limit or a cancellation; any
                    // other reply is read as the limit it can only mean.
                    SuppliedAdmissionError::Limit { .. }
                    | SuppliedAdmissionError::Invalid { .. }
                    | SuppliedAdmissionError::UnsupportedHelper { .. }
                    | SuppliedAdmissionError::Allocation { .. }
                    | SuppliedAdmissionError::Capacity { .. } => SuppliedStop {
                        context: context(spent),
                        cause: SuppliedCause::Limit(SuppliedLimit {
                            limit_kind: SuppliedLimitKind::ConversionWorkUnits,
                            limit: ceiling,
                            next_charge: 1,
                            counter: spent
                                .checked_add(1)
                                .map_or(AttemptedCount::Overflow, AttemptedCount::Count),
                            operand: None,
                        }),
                    },
                })
            });
        charged.inspect_err(|stop| {
            self.remember(SuppliedFailure::Stop(stop.clone()));
        })
    }

    /// Stop with the original cancellation cause if the caller cancelled,
    /// outside any scheduled event (so with no event named).
    ///
    /// # Errors
    ///
    /// The cancellation stop of `phase`'s parameter.
    pub fn check_cancelled(
        &mut self,
        phase: SuppliedPhase,
        parameter: usize,
    ) -> Result<(), SuppliedFailure> {
        let Some(cause) = self.cancel.cause() else {
            return Ok(());
        };
        let stop = SuppliedStop {
            context: SuppliedContext {
                phase,
                parameter: Some(parameter),
                event: None,
                locus: None,
                consumed: self.spent(phase),
            },
            cause: SuppliedCause::Cancelled(cause),
        };
        Err(self.remember(SuppliedFailure::from(stop)))
    }

    /// The owned locus of a membership-event locus.
    fn locus_of(&self, locus: MembershipLocus) -> Option<SuppliedLocus> {
        let resolved = self.arena.resolve(locus)?;
        Some(SuppliedLocus::Membership {
            argument: resolved.argument(),
            steps_from_leaf: resolved.steps_from_leaf().collect(),
            part: resolved.part(),
        })
    }

    /// The `limit` stop of one numeric operand over B: B is a per-operand
    /// bound with no cumulative counter, so the owning counter's successful
    /// prefix is zero and the denied amount is the operand's bit length.
    fn magnitude_stop(
        &self,
        phase: SuppliedPhase,
        parameter: usize,
        event: SuppliedEvent,
        operand: SuppliedNumericOperand,
        measured_bits: u64,
        locus: MembershipLocus,
    ) -> SuppliedFailure {
        SuppliedStop {
            context: SuppliedContext {
                phase,
                parameter: Some(parameter),
                event: Some(event),
                locus: self.locus_of(locus),
                consumed: 0,
            },
            cause: SuppliedCause::Limit(SuppliedLimit {
                limit_kind: SuppliedLimitKind::NumericMagnitudeBits,
                limit: self.numeric_magnitude_bits,
                next_charge: measured_bits,
                counter: AttemptedCount::Count(measured_bits),
                operand: Some(operand),
            }),
        }
        .into()
    }

    /// A Decimal helper's storage or capacity stop at the value visit that
    /// requested it.
    fn storage_stop(
        &self,
        phase: SuppliedPhase,
        parameter: usize,
        locus: MembershipLocus,
        cause: SuppliedCause,
    ) -> SuppliedFailure {
        SuppliedStop {
            context: SuppliedContext {
                phase,
                parameter: Some(parameter),
                event: Some(SuppliedEvent::ValueVisit),
                locus: self.locus_of(locus),
                consumed: self.spent(phase),
            },
            cause,
        }
        .into()
    }

    fn failure_of(
        &self,
        phase: SuppliedPhase,
        parameter: usize,
        error: SuppliedAdmissionError<InvalidMembershipCause>,
    ) -> SuppliedFailure {
        let spent = self.spent(phase);
        let context =
            |event: Option<SuppliedEvent>, locus: MembershipLocus, consumed: u64| SuppliedContext {
                phase,
                parameter: Some(parameter),
                event,
                locus: self.locus_of(locus),
                consumed,
            };
        match error {
            SuppliedAdmissionError::Invalid { cause, locus } => SuppliedStop {
                context: context(Some(event_of_part(locus.part)), locus, spent),
                cause: SuppliedCause::Input {
                    refusal: InputRefusal::WrongValueKind { parameter },
                    membership: Some(cause),
                },
            }
            .into(),
            SuppliedAdmissionError::UnsupportedHelper { family, locus } => {
                SuppliedFailure::Unsupported(Box::new(UnsupportedHelperStop {
                    context: context(Some(event_of_part(locus.part)), locus, spent),
                    family,
                }))
            }
            SuppliedAdmissionError::Limit {
                ceiling,
                spent,
                requested,
                event,
                locus,
            } => SuppliedStop {
                context: context(Some(event_of_membership(event)), locus, spent),
                cause: SuppliedCause::Limit(SuppliedLimit {
                    limit_kind: match phase {
                        SuppliedPhase::Admission => SuppliedLimitKind::AdmissionWorkUnits,
                        SuppliedPhase::Conversion => SuppliedLimitKind::ConversionWorkUnits,
                    },
                    limit: ceiling,
                    next_charge: requested,
                    counter: spent
                        .checked_add(requested)
                        .map_or(AttemptedCount::Overflow, AttemptedCount::Count),
                    operand: None,
                }),
            }
            .into(),
            SuppliedAdmissionError::Cancelled {
                cause,
                spent,
                event,
                locus,
            } => SuppliedStop {
                context: context(Some(event_of_membership(event)), locus, spent),
                cause: SuppliedCause::Cancelled(cause),
            }
            .into(),
            SuppliedAdmissionError::Allocation { request, locus } => SuppliedStop {
                context: context(Some(event_of_part(locus.part)), locus, spent),
                cause: SuppliedCause::Storage(storage_of_request(request)),
            }
            .into(),
            SuppliedAdmissionError::Capacity { request, locus } => SuppliedStop {
                context: context(Some(event_of_part(locus.part)), locus, spent),
                cause: SuppliedCause::Capacity(storage_of_request(request)),
            }
            .into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn run() -> SuppliedRun {
        SuppliedRun::new(SuppliedLimits::default(), Cancel::new()).expect("the run")
    }

    fn locus(run: &SuppliedRun) -> MembershipLocus {
        MembershipLocus {
            path: run.arena.root(0),
            part: MembershipPart::Value,
        }
    }

    fn stop(failure: SuppliedFailure) -> SuppliedStop {
        match failure {
            SuppliedFailure::Stop(stop) => *stop,
            other => panic!("a supplied-phase stop, got {other:?}"),
        }
    }

    /// FR-277-AC-5 (TC-906): every storage-bearing arm of QSV's bounded
    /// admission error maps to its own typed cause: an allocator denial is
    /// `Storage`, a size that cannot be represented is `Capacity`, and neither
    /// is a type refusal or a limit.
    #[trace("TC-906", "FR-277-AC-5")]
    #[test]
    fn storage_and_capacity_arms_map_to_their_own_typed_causes() {
        let request = StorageRequest::new(usize::MAX, StorageUnit::AdditionalElements);
        let run = run();
        let at = locus(&run);
        let cases = [
            (
                BoundedSuppliedAdmissionError::Admission(SuppliedAdmissionError::Allocation {
                    request,
                    locus: at,
                }),
                false,
                SuppliedStorageUnit::AdditionalElements,
            ),
            (
                BoundedSuppliedAdmissionError::Admission(SuppliedAdmissionError::Capacity {
                    request,
                    locus: at,
                }),
                true,
                SuppliedStorageUnit::AdditionalElements,
            ),
            (
                BoundedSuppliedAdmissionError::DecimalAllocation {
                    additional_elements: 3,
                    element_width: 4,
                    layout_bytes: 12,
                    locus: at,
                },
                false,
                SuppliedStorageUnit::Bytes,
            ),
            (
                BoundedSuppliedAdmissionError::DecimalCapacity {
                    magnitude_bits: 77,
                    locus: at,
                },
                true,
                SuppliedStorageUnit::MagnitudeBits,
            ),
        ];
        for (error, capacity, unit) in cases {
            let stop = stop(run.bounded_failure_of(SuppliedPhase::Admission, 0, error));
            let (SuppliedCause::Storage(storage) | SuppliedCause::Capacity(storage)) = &stop.cause
            else {
                panic!("a storage-bearing cause, got {:?}", stop.cause);
            };
            assert_eq!(
                matches!(stop.cause, SuppliedCause::Capacity(_)),
                capacity,
                "{error:?}"
            );
            assert_eq!(storage.unit, unit);
            assert!(stop.limit().is_none());
            assert_eq!(stop.context.consumed, 0);
        }
    }
}
