// SPDX-License-Identifier: AGPL-3.0-or-later
//! The verification planner's native settlement (QSL FR-529, QSpec FR-488,
//! FR-490, FR-492): read one `quire.planner-settlement/v1` request for an
//! implementation-guarantee item, admit it, and settle the item from the
//! premise results it carries.
//!
//! The module is layer 6 `qsl-replay` (ADR-011 section 6.1), because it reads
//! FR-331 result entries into [`TerminalRecord`](crate::TerminalRecord)s,
//! which this crate owns (ADR-013 O-24). It reads no package: the front end
//! builds a [`PlannerDeclaration`] from the checked package, and
//! [`read_settlement_request`] admits a request against it.
//!
//! - [`read_result_entry`] reads one FR-331 result entry in the order QSpec
//!   FR-331 "Result entries" gives; a plugin's entry goes through FR-290's
//!   readers.
//! - [`read_premise_item`] reads the slot-key members of a premise's request
//!   item.
//! - [`read_settlement_request`] applies QSpec FR-492 "Admission".
//! - [`settle()`](settle()) applies QSpec FR-492 "Slot outcome" and "Decision
//!   order", QSpec FR-488 "Settlement" for a guarantee and QSpec FR-489
//!   "Discharge", "Cycles" and "Settlement" for an `assume-guarantee`
//!   composition, whose premise discharge record is
//!   [`PlannerResult::discharge`].

mod compose;
mod declaration;
mod entry;
mod from_package;
mod item;
mod json;
mod key;
mod request;
mod settle;

#[cfg(test)]
mod composition_tests;
#[cfg(test)]
mod tests;

pub use from_package::{subject_text, DeclarationRefusal};
pub use compose::{DischargeEntry, DischargeRecord, DischargeSource, Relation};
pub use declaration::{
    ClauseFacts, ClauseForm, DerivedSlot, EnvSupplies, EnvironmentMember, ItemRole,
    PlannerDeclaration, SchedulingPremise, SlotBacking, SlotRole,
};
pub use entry::{
    read_result_entry, EntryCause, EntryOrigin, LimitFacts, ResultEntry, ResultEntryRefusal,
};
pub use item::{read_premise_item, PremiseItem};
pub use json::{Cause, Fault};
pub use key::{
    ClaimKind, Correspondence, CorrespondenceRole, ExtentClass, FairnessKey, ItemKey, KeyWireError,
    LocalKey, OperationRef, PremiseKey, SlotKey, TemporalItemKind, TemporalKey,
};
pub use request::{
    read_settlement_request, Discharge, Discharger, EnvironmentRecord, ItemRef, PremiseResult,
    ReadLimits, RecordSupplies, RequestRefusal, RequestSlot, SettlementRequest, SlotFill,
    DEFAULT_MAX_WORK, FORMAT,
};
pub use settle::{
    settle, Budget, Conclusion, ConclusionExtent, PlannerCause, PlannerResult, PlannerValue,
    SettleRefusal, SlotOutcome, SlotValue,
};
