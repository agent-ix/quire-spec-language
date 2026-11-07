// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-521 row 5: the public temporal surface kept until `quire-protocol` and
//! Contract IR repoint.
//!
//! The v1 evaluator is retired; its successors are specified by FR-327 to
//! FR-329 and FR-501 to FR-527 and implemented in `qsl-eval`. What remains is
//! [`evaluate_v2`] and its [`Report`], the [`Trace`] inputs and the
//! [`Closure`]/[`Completeness`]/[`Execution`] dimensions. The contract is in
//! [native-temporal-evaluation.md](https://github.com/agent-ix/quire-spec-language/blob/main/docs/native-temporal-evaluation.md).
//!
//! There is no public entry point accepting a freely constructed wire package.
//! Admission through [`crate::protocol_artifact::read`] is the constructor-private
//! evidence that the declaration was compiled rather than asserted.

mod owner;
mod owner_codec;
mod profile;
mod result;
mod trace;
mod valuation;
pub use owner::{
    evaluate_v2, CheckedSubject, NativeReportError, ObservationRef, Report, ReportOutcome,
    ReportPredecessorLimits, ReportPredecessorResource, ReportPredecessorUsage, ReportSubject,
    ReportSupport,
};
pub use owner_codec::{BindingDimension, ReportRefusal};

pub use profile::{EVENT_POSITION, FIXED_SAMPLE, TIMESTAMPED_WINDOW};
pub use result::{Closure, Completeness, Execution, Refusal};
pub use trace::{
    CaptureInput, ClockBinding, Eviction, Evidence, OrderKey, Position, Trace, Trigger,
};
pub use valuation::{
    MissingValuation, ValuationCause, ValuationEvidence, ValuationInstance, ValuationRow,
};
