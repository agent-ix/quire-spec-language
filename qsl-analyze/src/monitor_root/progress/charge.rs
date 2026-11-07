// SPDX-License-Identifier: AGPL-3.0-or-later
//! Incremental selection charges on the caller's actual preparation budget.

use qsl_replay::MonitorSubject;
use quire_contract_model::{NativeJoinBudget, NativeJoinChargeKind, NativeJoinLocus};
use quire_exact::{NodeKey, PathSegmentView};

use super::PlanAuthorityRefusal;

/// Borrows the original ledger; owns neither allowance nor shadow counters.
pub(super) struct Charge<'budget> {
    pub(super) budget: &'budget mut NativeJoinBudget,
    pub(super) locus: NativeJoinLocus,
}

impl Charge<'_> {
    fn pay(
        &mut self,
        kind: NativeJoinChargeKind,
        amount: u128,
    ) -> Result<(), PlanAuthorityRefusal> {
        self.budget
            .accounted_charge(kind, amount, self.locus)
            .map_err(|denied| PlanAuthorityRefusal::Budget {
                denied,
                locus: self.budget.first_unpaid_locus(),
                configured_bound: match denied.kind {
                    NativeJoinChargeKind::Work => self.budget.work_limit(),
                    NativeJoinChargeKind::Storage => self.budget.storage_limit(),
                },
                consumed: self.budget.consumed(),
            })
    }

    pub(super) fn work(&mut self, amount: u128) -> Result<(), PlanAuthorityRefusal> {
        self.pay(NativeJoinChargeKind::Work, amount)
    }

    fn storage(&mut self, amount: u128) -> Result<(), PlanAuthorityRefusal> {
        self.pay(NativeJoinChargeKind::Storage, amount)
    }

    pub(super) fn text(&mut self, left: &str, right: &str) -> Result<bool, PlanAuthorityRefusal> {
        let (mut left, mut right) = (left.bytes(), right.bytes());
        loop {
            self.work(2)?;
            match (left.next(), right.next()) {
                (None, None) => return Ok(true),
                (Some(left), Some(right)) if left == right => {}
                _ => return Ok(false),
            }
        }
    }

    fn node(&mut self, left: &NodeKey, right: &NodeKey) -> Result<bool, PlanAuthorityRefusal> {
        self.storage(1)?;
        let mut pending = Vec::new();
        self.storage(1)?;
        pending.try_reserve(1)?;
        pending.push((left.view(), right.view()));
        loop {
            self.work(1)?;
            let Some((left, right)) = pending.pop() else {
                return Ok(true);
            };
            if !self.text(left.package(), right.package())? {
                return Ok(false);
            }
            self.work(2)?;
            if left.ordinal() != right.ordinal() {
                return Ok(false);
            }
            let (mut left, mut right) = (left.path(), right.path());
            loop {
                self.work(2)?;
                match (left.next(), right.next()) {
                    (None, None) => break,
                    (Some(PathSegmentView::Text(left)), Some(PathSegmentView::Text(right))) => {
                        if !self.text(left, right)? {
                            return Ok(false);
                        }
                    }
                    (Some(PathSegmentView::Node(left)), Some(PathSegmentView::Node(right))) => {
                        self.storage(1)?;
                        pending.try_reserve(1)?;
                        pending.push((left, right));
                    }
                    _ => return Ok(false),
                }
            }
        }
    }

    pub(super) fn subject(
        &mut self,
        left: &MonitorSubject,
        right: &MonitorSubject,
    ) -> Result<bool, PlanAuthorityRefusal> {
        if !self.text(left.package().as_str(), right.package().as_str())?
            || !self.node(left.clause_node(), right.clause_node())?
            || !self.node(left.occurrence().node(), right.occurrence().node())?
            || !self.text(
                left.occurrence().origin().role().as_str(),
                right.occurrence().origin().role().as_str(),
            )?
        {
            return Ok(false);
        }
        self.work(2)?;
        if left.occurrence().origin().ordinal() != right.occurrence().origin().ordinal() {
            return Ok(false);
        }
        self.text(left.clause_name(), right.clause_name())
    }
}
