// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-277's supplied-value phase budgets (the vocabulary the S6a checked call
//! and the replay facade share): the caller-owned
//! `supplied.admission_work_units` (A) and `supplied.conversion_work_units`
//! (H), the phase-marked pre-call stop, and the run that carries both
//! budgets and the original [`Cancel`] through every supplied argument.
//!
//! The units are logical helper events, not evaluator charges: a pre-call
//! stop reports zero evaluation consumption and has no kernel charge point
//! (FR-277 "Projection into existing QSL carriers"). Membership events are
//! QSV's (`quire_semantic_value::supplied_membership`); conversion events
//! are QSL's (FR-098 "Conversion helper event units"). Each event has exactly
//! one initiating-phase owner.

use qsl_foundation::diagnostic::InternalFault;
use quire_exact::{Cancel, CancelCause, Value, ValueType};
use quire_semantic_value::call::InputRefusal;
use quire_semantic_value::declaration::{
    EnvironmentFailure, StorageRequest, StorageUnit, TypeEnvironment,
};
use quire_semantic_value::supplied_membership::{
    InvalidMembershipCause, MembershipEvent, MembershipLocus, MembershipPart, MembershipPathArena,
    PathArenaCapacity, PathArenaCapacityUnit, PathStep, SuppliedAdmissionError,
    SuppliedMembershipBudget, UnsupportedHelperFamily,
};

/// FR-277: the published default of each supplied helper ceiling.
pub const DEFAULT_SUPPLIED_WORK_UNITS: u64 = 16_777_216;

/// The setting name of A, FR-277's caller-owned admission ceiling.
pub const ADMISSION_WORK_UNITS: &str = "supplied.admission_work_units";

/// The setting name of H, FR-277's caller-owned conversion ceiling.
pub const CONVERSION_WORK_UNITS: &str = "supplied.conversion_work_units";

/// The typed supplied-value limits (FR-277, QSpec FR-323 `limits.supplied`):
/// two independent caller ceilings, each defaulting to
/// [`DEFAULT_SUPPLIED_WORK_UNITS`]. They are not setting-table rows and
/// never travel in a replay request's `stage_limits`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SuppliedLimits {
    admission_work_units: u64,
    conversion_work_units: u64,
}

impl Default for SuppliedLimits {
    fn default() -> Self {
        Self {
            admission_work_units: DEFAULT_SUPPLIED_WORK_UNITS,
            conversion_work_units: DEFAULT_SUPPLIED_WORK_UNITS,
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

/// The phase that initiated a supplied-value event and owns its budget.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SuppliedPhase {
    /// Direct or final checked-call membership, charged to A.
    Admission,
    /// Replay witness conversion, charged to H.
    Conversion,
}

impl SuppliedPhase {
    /// The wire spelling of the phase.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Admission => "admission",
            Self::Conversion => "conversion",
        }
    }

    /// The caller field that bounds this phase's helper work.
    #[must_use]
    pub const fn field(self) -> &'static str {
        match self {
            Self::Admission => ADMISSION_WORK_UNITS,
            Self::Conversion => CONVERSION_WORK_UNITS,
        }
    }
}

/// The actual logical event a supplied-phase stop happened at (QSpec FR-323
/// "Pre-call result context").
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SuppliedEvent {
    /// QSV: one runtime occurrence visit.
    ValueVisit,
    /// QSV: one compared pair of type-chain links.
    TypeComparison,
    /// QSV: one child scheduling operation.
    ChildScheduling,
    /// QSV or conversion: one descriptor query.
    DescriptorQuery,
    /// Conversion: reading and locally validating one witness node.
    WitnessPreflight,
    /// Conversion: constructing one converted kernel node.
    WitnessMaterialization,
    /// FR-098's per-argument converted-node counter C.
    ConvertedNode,
    /// The per-argument semantic `value_occurrences` counter.
    ValueOccurrence,
}

impl SuppliedEvent {
    /// The wire spelling of the event.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ValueVisit => "value-visit",
            Self::TypeComparison => "type-comparison",
            Self::ChildScheduling => "child-scheduling",
            Self::DescriptorQuery => "descriptor-query",
            Self::WitnessPreflight => "witness-preflight",
            Self::WitnessMaterialization => "witness-materialization",
            Self::ConvertedNode => "converted-node",
            Self::ValueOccurrence => "value-occurrence",
        }
    }

    const fn of_membership(event: MembershipEvent) -> Self {
        match event {
            MembershipEvent::ValueVisit => Self::ValueVisit,
            MembershipEvent::TypeLinkComparison => Self::TypeComparison,
            MembershipEvent::ChildScheduling => Self::ChildScheduling,
            MembershipEvent::DescriptorQuery => Self::DescriptorQuery,
        }
    }

    const fn of_part(part: MembershipPart) -> Self {
        match part {
            MembershipPart::Value => Self::ValueVisit,
            MembershipPart::TypeLink(_) => Self::TypeComparison,
            MembershipPart::Child(_) => Self::ChildScheduling,
            MembershipPart::Descriptor(_) => Self::DescriptorQuery,
        }
    }
}

/// Where in a supplied argument a stop happened, in its producer's own
/// typed terms. No fabricated empty path is ever carried: an unavailable
/// locus is `None` on the context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SuppliedLocus {
    /// A membership event: the zero-based supplied argument, the retained
    /// child edges from the current occurrence back to that argument's root
    /// (leaf first), and the local part of the occurrence.
    Membership {
        /// The zero-based supplied argument at the path root.
        argument: usize,
        /// Child edges from the current occurrence to the root.
        steps_from_leaf: Vec<PathStep>,
        /// The final event part.
        part: MembershipPart,
    },
    /// A conversion event: the zero-based argument and the zero-based
    /// preorder position of the witness node within it.
    Witness {
        /// The zero-based supplied argument.
        argument: usize,
        /// The node's preorder position in that argument's witness.
        position: u64,
    },
}

/// Which counter a limit stop denied.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SuppliedLimitKind {
    /// A: cumulative admission helper events.
    AdmissionWorkUnits,
    /// H: cumulative conversion helper events.
    ConversionWorkUnits,
    /// FR-098's converted nodes C, bounded by `work_units`.
    WorkUnits,
    /// The semantic occurrence count, bounded by `value_occurrences`.
    ValueOccurrences,
    /// A decimal quantity magnitude's power-of-ten expansion, bounded by
    /// `scale_expansion`.
    ScaleExpansion,
}

impl SuppliedLimitKind {
    /// The caller field that raises this bound.
    #[must_use]
    pub const fn field(self) -> &'static str {
        match self {
            Self::AdmissionWorkUnits => ADMISSION_WORK_UNITS,
            Self::ConversionWorkUnits => CONVERSION_WORK_UNITS,
            Self::WorkUnits => "work_units",
            Self::ValueOccurrences => "value_occurrences",
            Self::ScaleExpansion => "scale_expansion",
        }
    }
}

/// The checked count a denied event would have reached.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AttemptedCount {
    /// The exact attempted count.
    Count(u64),
    /// The counter's representation overflowed; nothing wrapped.
    Overflow,
}

/// The pre-call limit record (FR-277 "Projection into existing QSL
/// carriers"): the numerical meanings of `Incomplete` without a charge point.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SuppliedLimit {
    /// The denied counter.
    pub limit_kind: SuppliedLimitKind,
    /// The configured ceiling.
    pub limit: u64,
    /// The exact denied amount (one for A and H).
    pub next_charge: u64,
    /// The checked attempted count.
    pub counter: AttemptedCount,
}

impl SuppliedLimit {
    /// The caller field that raises this bound.
    #[must_use]
    pub const fn field(&self) -> &'static str {
        self.limit_kind.field()
    }
}

/// The measured unit of a failed reservation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SuppliedStorageUnit {
    /// Native bytes.
    Bytes,
    /// Additional collection elements.
    AdditionalElements,
    /// Distinct path-arena identifiers.
    ArenaIdentifiers,
}

/// The measured request of a storage or capacity stop, in its native unit.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SuppliedStorage {
    /// The exact amount submitted to the reservation.
    pub requested: u64,
    /// Its native unit.
    pub unit: SuppliedStorageUnit,
}

impl SuppliedStorage {
    fn of_request(request: StorageRequest) -> Self {
        Self {
            requested: u64::try_from(request.amount()).unwrap_or(u64::MAX),
            unit: match request.unit() {
                StorageUnit::Bytes => SuppliedStorageUnit::Bytes,
                StorageUnit::AdditionalElements => SuppliedStorageUnit::AdditionalElements,
            },
        }
    }

    fn of_arena(capacity: PathArenaCapacity) -> Self {
        let PathArenaCapacityUnit::ArenaIdentifiers = capacity.unit;
        Self {
            requested: u64::try_from(capacity.requested).unwrap_or(u64::MAX),
            unit: SuppliedStorageUnit::ArenaIdentifiers,
        }
    }
}

/// The five closed causes of a supplied-phase stop (QSpec FR-323).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SuppliedCause {
    /// A helper or counter ceiling denied the next event.
    Limit(SuppliedLimit),
    /// The input failed membership; the original refusal and, where QSV
    /// reported it, the membership cause.
    Input {
        /// The original input refusal.
        refusal: InputRefusal,
        /// QSV's membership cause, when membership found the fault.
        membership: Option<InvalidMembershipCause>,
    },
    /// The original caller [`Cancel`] stopped the phase.
    Cancelled(CancelCause),
    /// A fallible reservation was denied by the allocator.
    Storage(SuppliedStorage),
    /// A reservation size could not be represented.
    Capacity(SuppliedStorage),
}

/// The context every supplied-phase stop carries (QSpec FR-323):
/// `{ phase, parameter, event, locus, consumed, cause }` with the cause in
/// [`SuppliedStop::cause`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SuppliedContext {
    /// The actual initiating owner.
    pub phase: SuppliedPhase,
    /// The zero-based declared parameter position, when known.
    pub parameter: Option<usize>,
    /// The actual owning event; `None` for a cancellation outside a
    /// scheduled event.
    pub event: Option<SuppliedEvent>,
    /// The producer's typed locus, when available.
    pub locus: Option<SuppliedLocus>,
    /// The successful prefix of the owning counter: cumulative for A and H,
    /// per argument for C and `value_occurrences`. A denied step is never in
    /// it.
    pub consumed: u64,
}

/// A phase-marked pre-call stop: no evaluation ran, nothing was charged to
/// the evaluator's meter and no partial value exists.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SuppliedStop {
    /// The phase context.
    pub context: SuppliedContext,
    /// Why the phase stopped.
    pub cause: SuppliedCause,
}

impl SuppliedStop {
    /// The limit record, when a ceiling denied the event.
    #[must_use]
    pub const fn limit(&self) -> Option<&SuppliedLimit> {
        match &self.cause {
            SuppliedCause::Limit(limit) => Some(limit),
            SuppliedCause::Input { .. }
            | SuppliedCause::Cancelled(_)
            | SuppliedCause::Storage(_)
            | SuppliedCause::Capacity(_) => None,
        }
    }
}

/// A supplied-admission walk reached a helper family whose supplied-value
/// contract QSV has not qualified (IR-714): the walk stopped before that
/// helper ran. It is distinct from invalid input, a limit, a cancellation,
/// storage and capacity, and never a [`SuppliedCause`] arm.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnsupportedHelperStop {
    /// The phase context at the stop.
    pub context: SuppliedContext,
    /// The helper family that remains an implementation dependency.
    pub family: UnsupportedHelperFamily,
}

/// Why a supplied phase stopped before any call.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SuppliedFailure {
    /// A limit, input, cancellation, storage or capacity stop.
    Stop(Box<SuppliedStop>),
    /// An unqualified helper family.
    Unsupported(Box<UnsupportedHelperStop>),
    /// A broken kernel contract: an environment ceiling or declaration
    /// refusal answering a membership query.
    Fault(InternalFault),
}

impl From<SuppliedStop> for SuppliedFailure {
    fn from(stop: SuppliedStop) -> Self {
        Self::Stop(Box::new(stop))
    }
}

/// Both phase budgets, the rooted-path arena and the original [`Cancel`] of
/// one supplied invocation (FR-277). One run covers every supplied argument
/// and nested helper: nothing resets between arguments, and the handle is
/// never replaced.
#[derive(Debug)]
pub struct SuppliedRun {
    admission: SuppliedMembershipBudget,
    conversion: SuppliedMembershipBudget,
    arena: MembershipPathArena,
    cancel: Cancel,
    stopped: Option<SuppliedFailure>,
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
                cause: SuppliedCause::Capacity(SuppliedStorage::of_arena(capacity)),
            })
        })?;
        Ok(Self {
            admission: SuppliedMembershipBudget::new(limits.admission_work_units),
            conversion: SuppliedMembershipBudget::new(limits.conversion_work_units),
            arena,
            cancel,
            stopped: None,
        })
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
                ..
            } = self;
            let budget = match phase {
                SuppliedPhase::Admission => admission,
                SuppliedPhase::Conversion => conversion,
            };
            types.admit_supplied::<InvalidMembershipCause>(
                expected, value, arena, at, budget, &cancel,
            )
        };
        let outcome = match result {
            Ok(()) => return Ok(()),
            Err(error) => self.failure_of(phase, parameter, error),
        };
        Err(self.remember(outcome))
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
                context: context(Some(SuppliedEvent::of_part(locus.part)), locus, spent),
                cause: SuppliedCause::Input {
                    refusal: InputRefusal::WrongValueKind { parameter },
                    membership: Some(cause),
                },
            }
            .into(),
            SuppliedAdmissionError::UnsupportedHelper { family, locus } => {
                SuppliedFailure::Unsupported(Box::new(UnsupportedHelperStop {
                    context: context(Some(SuppliedEvent::of_part(locus.part)), locus, spent),
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
                context: context(Some(SuppliedEvent::of_membership(event)), locus, spent),
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
                }),
            }
            .into(),
            SuppliedAdmissionError::Cancelled {
                cause,
                spent,
                event,
                locus,
            } => SuppliedStop {
                context: context(Some(SuppliedEvent::of_membership(event)), locus, spent),
                cause: SuppliedCause::Cancelled(cause),
            }
            .into(),
            SuppliedAdmissionError::Allocation { request, locus } => SuppliedStop {
                context: context(Some(SuppliedEvent::of_part(locus.part)), locus, spent),
                cause: SuppliedCause::Storage(SuppliedStorage::of_request(request)),
            }
            .into(),
            SuppliedAdmissionError::Capacity { request, locus } => SuppliedStop {
                context: context(Some(SuppliedEvent::of_part(locus.part)), locus, spent),
                cause: SuppliedCause::Capacity(SuppliedStorage::of_request(request)),
            }
            .into(),
        }
    }
}
