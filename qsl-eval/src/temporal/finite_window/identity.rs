// SPDX-License-Identifier: AGPL-3.0-or-later
//! The result identity (FR-501 "Result report"; FR-043-AC-13): a SHA-256
//! content identity over the RFC 8785 form of everything an evaluation read.

use quire_canonical::{Encode, Error, Limits, Sink, Writer};
use quire_exact::ScalarLimits;
use quire_semantic_value::catalog::InternalFault;

use super::cover::ClockCover;
use super::lift::Cell;
use super::outcome::ResultIdentity;
use super::trace::{LowerBoundary, TimestampedTrace};
use crate::temporal::ledger::{ProgressClosure, ProgressCompleteness};
use crate::temporal::plan::fault;

/// What one evaluation read, in a fixed order.
pub(super) struct Preimage<'a> {
    pub(super) trace: &'a TimestampedTrace,
    pub(super) profile: &'static str,
    /// The atoms read, as `(position, atom node, valuation)`, sorted.
    pub(super) readings: &'a [(usize, usize, Cell)],
    pub(super) ceilings: &'a ScalarLimits,
}

/// A decimal string: an integer past 2^53 has no RFC 8785 number form.
fn count<S: Sink + ?Sized>(writer: &mut Writer<'_, S>, value: u128) -> Result<(), Error> {
    writer.string(&value.to_string())
}

fn closure(value: ProgressClosure) -> &'static str {
    match value {
        ProgressClosure::Closed => "closed",
        ProgressClosure::Open => "open",
    }
}

fn tag(cell: Cell) -> &'static str {
    match cell {
        Cell::True => "true",
        Cell::False => "false",
        Cell::Missing => "missing",
        Cell::Conflicting => "conflicting",
    }
}

impl Encode for Preimage<'_> {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        let trace = self.trace;
        writer.begin_array()?;
        writer.string("quire.qsl.finite-window-result/v1")?;
        writer.string(&trace.clause_identity)?;
        writer.string(self.profile)?;
        writer.string(&trace.clock.unit)?;
        match trace.clock.cover {
            ClockCover::Exact => writer.string("exact")?,
            ClockCover::Counter { uncertainty } => {
                writer.string("counter")?;
                count(writer, u128::from(uncertainty))?;
            }
        }
        writer.begin_array()?;
        for position in &trace.positions {
            writer.begin_array()?;
            count(writer, u128::from(position.stamp))?;
            match &position.order_key {
                Some(key) => {
                    writer.string(&key.authority)?;
                    writer.integer(i128::from(key.key))?;
                }
                None => writer.null()?,
            }
            writer.string(&position.observation)?;
            writer.end_array()?;
        }
        writer.end_array()?;
        writer.begin_array()?;
        for (position, node, cell) in self.readings {
            writer.begin_array()?;
            count(writer, *position as u128)?;
            count(writer, *node as u128)?;
            writer.string(tag(*cell))?;
            writer.end_array()?;
        }
        writer.end_array()?;
        let progress = &trace.asserted;
        writer.integer(progress.watermark.inclusive())?;
        match progress.completeness {
            ProgressCompleteness::Complete { through } => {
                writer.string("complete")?;
                count(writer, u128::from(through))?;
            }
            ProgressCompleteness::Incomplete => writer.string("incomplete")?,
        }
        writer.string(closure(progress.scope))?;
        writer.string(closure(progress.execution))?;
        writer.string(match progress.lower_boundary {
            LowerBoundary::AuthoritativeOrigin => "authoritative-origin",
            LowerBoundary::HistoryCutoff => "history-cutoff",
        })?;
        let ceilings = self.ceilings;
        writer.begin_array()?;
        for ceiling in [
            ceilings.integer_bits,
            ceilings.decimal_digits,
            ceilings.scale_expansion,
            ceilings.text_input_bytes,
            ceilings.text_scalars,
            ceilings.normalized_scalars,
            ceilings.unit_edges,
            ceilings.value_occurrences,
            ceilings.work_units,
            ceilings.result_units,
        ] {
            count(writer, u128::from(ceiling))?;
        }
        writer.end_array()?;
        writer.end_array()
    }
}

impl Preimage<'_> {
    /// The identity of this preimage.
    pub(super) fn identity(&self) -> Result<ResultIdentity, InternalFault> {
        let digest = quire_canonical::sha256(self, Limits::new(u64::MAX))
            .map_err(|_| fault("result-identity-encodes"))?;
        let mut text = String::from("sha256:");
        for byte in digest.as_bytes() {
            text.push_str(&format!("{byte:02x}"));
        }
        Ok(ResultIdentity(text))
    }
}
