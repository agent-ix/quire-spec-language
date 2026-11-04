// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-520-AC-11's per-binder charge log for one protocol-slot call.
//!
//! The quire-exact `Meter` captures fixed-size interval aggregates of its ten
//! counters. This module owns the QSL side: which binder an interval belongs
//! to, the registration of binders the expression actually read, and the
//! version of those attribution rules.

use std::collections::BTreeMap;

use quire_exact::{ChargeAccounting, LimitKind, Meter, NodeKey};

/// The version of the attribution rules in [`BinderCharges`]. It changes
/// whenever the rule that decides which binder a charge belongs to changes.
pub const BINDER_ACCOUNTING_VERSION: &str = "qsl.protocol-slot.binder-charges/1";

/// The ten FR-255 counters accumulated over one attribution scope: a sum for
/// the cumulative `work_units` and `result_units`, a maximum for the eight
/// high-water counters.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ChargeAmounts {
    amounts: [u64; LimitKind::ALL.len()],
}

impl ChargeAmounts {
    /// The accumulated amount of one counter.
    pub fn amount(&self, kind: LimitKind) -> u64 {
        self.amounts[position(kind)]
    }

    fn absorb_interval(&mut self, interval: &ChargeAccounting) {
        for kind in LimitKind::ALL {
            self.absorb(kind, interval.amount(kind));
        }
    }

    fn absorb(&mut self, kind: LimitKind, amount: u64) {
        let held = &mut self.amounts[position(kind)];
        *held = match kind {
            // Interval sums never exceed the meter's checked global totals, so
            // the saturation is unreachable and only keeps the add total.
            LimitKind::WorkUnits | LimitKind::ResultUnits => held.saturating_add(amount),
            LimitKind::IntegerBits
            | LimitKind::DecimalDigits
            | LimitKind::ScaleExpansion
            | LimitKind::TextInputBytes
            | LimitKind::TextScalars
            | LimitKind::NormalizedScalars
            | LimitKind::UnitEdges
            | LimitKind::ValueOccurrences => (*held).max(amount),
        };
    }

    fn combine(&mut self, other: &Self) {
        for kind in LimitKind::ALL {
            self.absorb(kind, other.amount(kind));
        }
    }
}

fn position(kind: LimitKind) -> usize {
    match kind {
        LimitKind::IntegerBits => 0,
        LimitKind::DecimalDigits => 1,
        LimitKind::ScaleExpansion => 2,
        LimitKind::TextInputBytes => 3,
        LimitKind::TextScalars => 4,
        LimitKind::NormalizedScalars => 5,
        LimitKind::UnitEdges => 6,
        LimitKind::ValueOccurrences => 7,
        LimitKind::WorkUnits => 8,
        LimitKind::ResultUnits => 9,
    }
}

/// The charges of one protocol-slot call, attributed per binder.
///
/// A charge belongs to the binder whose supplied value the evaluation read
/// most recently; charges made before the first binder read belong to no
/// binder. A binder the expression read has an entry even when it made no
/// charge; a binder it never read has none.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BinderCharges {
    binders: BTreeMap<NodeKey, ChargeAmounts>,
    unattributed: ChargeAmounts,
}

impl BinderCharges {
    /// The attribution version these charges follow
    /// ([`BINDER_ACCOUNTING_VERSION`]).
    pub fn accounting_version(&self) -> &'static str {
        BINDER_ACCOUNTING_VERSION
    }

    /// The charges attributed to `binder`, absent when it was never read.
    pub fn binder(&self, binder: &NodeKey) -> Option<&ChargeAmounts> {
        self.binders.get(binder)
    }

    /// Every binder that was read, in binder identity order, with its charges.
    pub fn binders(&self) -> impl Iterator<Item = (&NodeKey, &ChargeAmounts)> {
        self.binders.iter()
    }

    /// The charges made while no binder read was current.
    pub fn unattributed(&self) -> &ChargeAmounts {
        &self.unattributed
    }

    /// The call's total: every binder entry combined with the unattributed
    /// entry.
    pub fn total(&self) -> ChargeAmounts {
        let mut total = self.unattributed;
        for amounts in self.binders.values() {
            total.combine(amounts);
        }
        total
    }
}

/// The routing state of one call: which binder's interval is open.
pub(crate) struct ChargeRouting {
    current: Option<NodeKey>,
    charges: BinderCharges,
}

impl ChargeRouting {
    pub(crate) fn new() -> Self {
        Self {
            current: None,
            charges: BinderCharges {
                binders: BTreeMap::new(),
                unattributed: ChargeAmounts::default(),
            },
        }
    }

    /// A binder's supplied value is read: close the open interval onto the
    /// binder that owned it, register the reader and open the next interval.
    /// `None` when the meter has charge accounting disabled.
    pub(crate) fn read(&mut self, meter: &mut Meter, binder: &NodeKey) -> Option<()> {
        if self.current.as_ref() == Some(binder) {
            return Some(());
        }
        self.close(meter)?;
        self.charges.binders.entry(binder.clone()).or_default();
        self.current = Some(binder.clone());
        Some(())
    }

    /// Close the last interval and return the finished charges.
    pub(crate) fn finish(mut self, meter: &mut Meter) -> Option<BinderCharges> {
        self.close(meter)?;
        Some(self.charges)
    }

    fn close(&mut self, meter: &mut Meter) -> Option<()> {
        let interval = meter.take_charge_accounting()?;
        let owner = match &self.current {
            Some(binder) => self.charges.binders.entry(binder.clone()).or_default(),
            None => &mut self.charges.unattributed,
        };
        owner.absorb_interval(&interval);
        Some(())
    }
}
