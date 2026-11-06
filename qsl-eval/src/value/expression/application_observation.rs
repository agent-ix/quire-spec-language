// SPDX-License-Identifier: AGPL-3.0-or-later
//! Owned checked-call evidence and bounded exact charge capture (FR-090).

use quire_eval::{CallFailure, Evaluation, FamilyOutcome};
use quire_exact::{
    ChargeAdmission, ChargeCapture, ChargeEvent, LimitKind, Meter, TruncatedChargeCapture, Value,
};
use quire_semantic_value::location::Location;
use quire_semantic_value::loss::LocatedLoss;

use super::ChargeAmounts;

/// Explicit whole-call limits for application observation.
#[derive(Clone, Copy, Debug)]
pub struct ApplicationObservationOptions {
    /// Maximum number of function bodies evaluated, including the root call.
    pub function_case_limit: u64,
    /// Maximum meter boundary events retained, including denied events.
    pub max_events: usize,
    /// Maximum size-request entries retained, including duplicates and denied requests.
    pub max_size_requests: usize,
}

/// An admitted meter request with the checked operation that issued it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationCharge {
    /// Actual checked operation location, absent for an unlocated operation.
    pub location: Option<Location>,
    /// Canonical exact payload, including ordered sizes and reservations.
    pub request: ChargeEvent,
}

/// The complete admitted charge sequence for one observing call.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OperationLedger {
    entries: Vec<OperationCharge>,
    amounts: ChargeAmounts,
}

impl OperationLedger {
    /// Admitted requests in execution order.
    pub fn entries(&self) -> &[OperationCharge] {
        &self.entries
    }

    /// Whole-evaluation interval amounts: cumulative sums and size maxima.
    pub fn interval_amounts(&self) -> &ChargeAmounts {
        &self.amounts
    }
}

/// Capture exceeded its whole-call capacity; no complete interval is available.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TruncatedOperationLedger {
    entries: Vec<OperationCharge>,
    admitted_prefix_amounts: ChargeAmounts,
}

impl TruncatedOperationLedger {
    /// Admitted requests before the first event that exceeded capacity.
    pub fn admitted_prefix(&self) -> &[OperationCharge] {
        &self.entries
    }

    /// Amounts of the retained admitted prefix only, never the whole evaluation.
    pub fn admitted_prefix_amounts(&self) -> &ChargeAmounts {
        &self.admitted_prefix_amounts
    }
}

/// Owned evidence returned even when a checked call fails.
///
/// The call detaches any unread caller capture and returns it in
/// `displaced_capture`. It uses the explicit capacities and leaves capture
/// disabled on every return. Callers must explicitly enable later captures.
pub struct ApplicationObservation {
    /// The actual result, without cloning or projecting family-owned causes.
    pub evaluation: Result<Evaluation, CallFailure>,
    /// Completed checked applications and their values, in execution order.
    pub observed: Vec<(Location, Value)>,
    /// A called dependency body was unavailable in this package.
    pub unavailable_imported_call: bool,
    /// Function bodies evaluated by this call and its nested calls.
    pub function_evaluations: u64,
    /// The case limit prevented a body evaluation.
    pub function_case_limit_reached: bool,
    /// Complete final global meter totals, including consumption before this call.
    pub meter_totals: ChargeAmounts,
    /// Complete ledger or a typed admitted prefix; truncation does not stop evaluation.
    pub ledger: Result<OperationLedger, TruncatedOperationLedger>,
    /// Unread caller capture displaced before the observing call started.
    pub displaced_capture: Option<Result<ChargeCapture, TruncatedChargeCapture>>,
}

impl ApplicationObservation {
    /// Borrow the actual outcome, absent when the call failed.
    pub fn outcome(&self) -> Option<&FamilyOutcome<Value>> {
        self.evaluation
            .as_ref()
            .ok()
            .map(|evaluation| &evaluation.outcome)
    }

    /// Borrow the actual ordered losses, absent when the call failed.
    pub fn losses(&self) -> Option<&[LocatedLoss]> {
        self.evaluation
            .as_ref()
            .ok()
            .map(|evaluation| evaluation.losses.as_slice())
    }

    /// Consume the report and return its actual evaluation or call failure.
    pub fn into_evaluation(self) -> Result<Evaluation, CallFailure> {
        self.evaluation
    }
}

/// Borrowed evidence sinks used only by the shared machine.
pub(super) struct ApplicationObserver<'a> {
    pub observed: &'a mut Vec<(Location, Value)>,
    pub unavailable_imported_call: &'a mut bool,
    pub function_case_limit: u64,
    pub function_evaluations: &'a mut u64,
    pub function_case_limit_reached: &'a mut bool,
    pub capture: &'a mut OperationCapture,
}

pub(super) struct OperationCapture {
    ledger: OperationLedger,
    remaining_events: usize,
    remaining_sizes: usize,
    truncated: bool,
}

impl OperationCapture {
    pub(super) fn new(options: ApplicationObservationOptions) -> Self {
        Self {
            ledger: OperationLedger::default(),
            remaining_events: options.max_events,
            remaining_sizes: options.max_size_requests,
            truncated: false,
        }
    }

    pub(super) fn start(&self, meter: &mut Meter) {
        meter.enable_charge_capture(self.remaining_events, self.remaining_sizes);
    }

    pub(super) fn drain(&mut self, meter: &mut Meter, location: Option<&Location>) {
        let Some(captured) = meter.disable_charge_capture() else {
            return;
        };
        let capture = match captured {
            Ok(capture) => capture,
            Err(truncated) => {
                self.truncated = true;
                truncated.prefix
            }
        };
        for event in capture.events() {
            // Count all boundaries before filtering admitted rows. Each drain
            // installs only the capacity remaining for the entire call.
            self.remaining_events -= 1;
            self.remaining_sizes -= event.sizes.len();
            if event.admission == ChargeAdmission::Admitted {
                self.ledger.amounts.absorb(
                    LimitKind::WorkUnits,
                    event
                        .work_units
                        .to_u64()
                        .expect("admitted work fits the meter's u64 counter"),
                );
                self.ledger.amounts.absorb(
                    LimitKind::ResultUnits,
                    event
                        .result_units
                        .to_u64()
                        .expect("admitted results fit the meter's u64 counter"),
                );
                for (kind, amount) in &event.sizes {
                    if !kind.is_cumulative() {
                        // Successful admissions fit the meter's u64 counters.
                        self.ledger.amounts.absorb(
                            *kind,
                            amount
                                .to_u64()
                                .expect("admitted sizes fit the meter's u64 counters"),
                        );
                    }
                }
                self.ledger.entries.push(OperationCharge {
                    location: location.cloned(),
                    request: event.clone(),
                });
            }
        }
        if !self.truncated {
            self.start(meter);
        }
    }

    pub(super) fn finish(
        mut self,
        meter: &mut Meter,
    ) -> Result<OperationLedger, TruncatedOperationLedger> {
        self.drain(meter, None);
        // drain may have started the next empty interval.
        meter.disable_charge_capture();
        if self.truncated {
            Err(TruncatedOperationLedger {
                entries: self.ledger.entries,
                admitted_prefix_amounts: self.ledger.amounts,
            })
        } else {
            Ok(self.ledger)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use quire_exact::{Charge, ChargePoint, Integer, ScalarLimits};

    #[trace("TC-1338", "FR-090-AC-24")]
    #[test]
    fn capture_preserves_ordered_duplicate_requests_and_exact_cumulative_amounts() {
        let limits = ScalarLimits {
            integer_bits: 100,
            decimal_digits: 100,
            scale_expansion: 100,
            text_input_bytes: 100,
            text_scalars: 100,
            normalized_scalars: 100,
            unit_edges: 100,
            value_occurrences: 100,
            work_units: 1000,
            result_units: 1000,
        };
        let mut meter = Meter::new(limits);
        let mut capture = OperationCapture::new(ApplicationObservationOptions {
            function_case_limit: 1,
            max_events: 1,
            max_size_requests: 3,
        });
        capture.start(&mut meter);
        meter
            .charge(
                Charge::new(ChargePoint::CollectionVisit)
                    .size(LimitKind::IntegerBits, 9)
                    .size(LimitKind::IntegerBits, 3)
                    .size(LimitKind::IntegerBits, 9)
                    .work(Integer::from(300_u64))
                    .results(200),
            )
            .expect("admitted request");
        capture.drain(&mut meter, None);
        let ledger = capture.finish(&mut meter).expect("complete capture");
        let event = &ledger.entries()[0].request;
        assert_eq!(
            event.sizes,
            [
                (LimitKind::IntegerBits, Integer::from(9_u64)),
                (LimitKind::IntegerBits, Integer::from(3_u64)),
                (LimitKind::IntegerBits, Integer::from(9_u64))
            ]
        );
        assert_eq!(event.work_units, Integer::from(300_u64));
        assert_eq!(event.result_units, Integer::from(200_u64));
        assert_eq!(ledger.interval_amounts().amount(LimitKind::IntegerBits), 9);
        assert_eq!(ledger.interval_amounts().amount(LimitKind::WorkUnits), 300);
        assert_eq!(
            ledger.interval_amounts().amount(LimitKind::ResultUnits),
            200
        );
    }
}
