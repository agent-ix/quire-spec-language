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
/// A charge belongs to binder B when it is made by reading B or by an
/// operation whose binder-derived operands all come from B (operand
/// provenance). An operation over operands from two binders, and a charge
/// made by a literal-only computation, belong to no binder and are recorded
/// as unattributed. A binder the expression read has an entry even when it
/// made no charge; a binder it never read has none.
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

/// Which binder, if any, a value or a charge is derived from.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Owner {
    /// Derived from no binder (a literal-only computation).
    None,
    /// Derived from the binder supplied at this root-frame source slot only.
    Binder(usize),
    /// Derived from more than one binder.
    Mixed,
}

impl Owner {
    /// The owner of a computation over operands owned by `self` and `other`:
    /// a binder-free operand adds nothing, and operands from two binders make
    /// the result belong to no single binder.
    pub(crate) fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::None, owner) | (owner, Self::None) => owner,
            (Self::Binder(left), Self::Binder(right)) if left == right => Self::Binder(left),
            (Self::Binder(_) | Self::Mixed, Self::Binder(_) | Self::Mixed) => Self::Mixed,
        }
    }
}

/// The owner of every stack value and frame slot, and of the step being run.
/// Its stacks mirror the machine's value and frame stacks.
#[derive(Default)]
pub(crate) struct Owners {
    values: Vec<Owner>,
    frames: Vec<Vec<Owner>>,
    step: Option<Owner>,
    popped: Vec<Owner>,
}

impl Owners {
    pub(crate) fn begin_step(&mut self) {
        self.step = Some(Owner::None);
    }

    pub(crate) fn step(&self) -> Owner {
        self.step.unwrap_or(Owner::None)
    }

    pub(crate) fn absorb(&mut self, owner: Owner) {
        self.step = Some(self.step().merge(owner));
    }

    pub(crate) fn push_value(&mut self) {
        self.values.push(self.step());
    }

    pub(crate) fn pop_value(&mut self) -> Option<()> {
        let owner = self.values.pop()?;
        self.absorb(owner);
        Some(())
    }

    pub(crate) fn pop_values(&mut self, count: usize) -> Option<()> {
        let start = self.values.len().checked_sub(count)?;
        self.popped = self.values.split_off(start);
        for owner in self.popped.clone() {
            self.absorb(owner);
        }
        Some(())
    }

    /// The owners of the values the last [`Self::pop_values`] took.
    pub(crate) fn popped(&self) -> &[Owner] {
        &self.popped
    }

    pub(crate) fn push_frame(&mut self, owners: Vec<Owner>) {
        self.frames.push(owners);
    }

    pub(crate) fn pop_frame(&mut self) -> Option<()> {
        self.frames.pop().map(|_| ())
    }

    pub(crate) fn set_frame(&mut self, owners: Vec<Owner>) -> Option<()> {
        *self.frames.last_mut()? = owners;
        Some(())
    }

    pub(crate) fn slot(&self, slot: usize) -> Owner {
        self.frames
            .last()
            .and_then(|frame| frame.get(slot))
            .copied()
            .unwrap_or(Owner::None)
    }

    pub(crate) fn set_slot(&mut self, slot: usize, owner: Owner) -> Option<()> {
        let frame = self.frames.last_mut()?;
        if frame.len() <= slot {
            frame.resize(slot + 1, Owner::None);
        }
        frame[slot] = owner;
        Some(())
    }
}

/// The routing state of one call: the binders the expression read and the
/// charges attributed so far.
pub(crate) struct ChargeRouting {
    charges: BinderCharges,
}

impl ChargeRouting {
    pub(crate) fn new() -> Self {
        Self {
            charges: BinderCharges {
                binders: BTreeMap::new(),
                unattributed: ChargeAmounts::default(),
            },
        }
    }

    /// A binder's supplied value is read: register it, so it has an entry
    /// even when it is charged nothing.
    pub(crate) fn register(&mut self, binder: &NodeKey) {
        self.charges.binders.entry(binder.clone()).or_default();
    }

    /// Drain the meter's interval onto `binder`, or onto no binder.
    /// `None` when the meter has charge accounting disabled.
    pub(crate) fn attribute(&mut self, meter: &mut Meter, binder: Option<&NodeKey>) -> Option<()> {
        let interval = meter.take_charge_accounting()?;
        let owner = match binder {
            Some(binder) => self.charges.binders.entry(binder.clone()).or_default(),
            None => &mut self.charges.unattributed,
        };
        owner.absorb_interval(&interval);
        Some(())
    }

    /// The finished charges.
    pub(crate) fn finish(self) -> BinderCharges {
        self.charges
    }
}
