// SPDX-License-Identifier: AGPL-3.0-or-later
//! Incremental selection charges on the caller's actual preparation budget.

use qsl_replay::MonitorSubject;
use qsl_semantics::check::wire_id::{try_structural_id, StructuralIdMaterializationError};
use quire_contract_model::{
    CheckedNodeId, CheckedNodePathSegment, NativeJoinBudget, NativeJoinChargeKind,
    NativeJoinDeniedCharge, NativeJoinLocus,
};
use quire_exact::{NodeKey, PathSegmentView};

use super::PlanAuthorityRefusal;

/// Borrows the original ledger; owns neither allowance nor shadow counters.
pub(in crate::monitor_root) struct Charge<'budget> {
    pub(in crate::monitor_root) budget: &'budget mut NativeJoinBudget,
    pub(in crate::monitor_root) locus: NativeJoinLocus,
}

impl Charge<'_> {
    fn pay(
        &mut self,
        kind: NativeJoinChargeKind,
        amount: u128,
    ) -> Result<(), PlanAuthorityRefusal> {
        self.budget
            .accounted_charge(kind, amount, self.locus)
            .map_err(|denied| self.denial(denied))
    }

    /// The `Budget` refusal of a strict-read denial: the budgeted reader's own
    /// denied resource and full amount, the read's configured bound, and the
    /// caller budget's actual consumption and first-unpaid locus.
    pub(in crate::monitor_root) fn read_denial(
        &self,
        resource: quire_contract_model::PackageReadResource,
        amount: u128,
        configured_bound: u64,
    ) -> PlanAuthorityRefusal {
        PlanAuthorityRefusal::Budget {
            denied: NativeJoinDeniedCharge {
                kind: match resource {
                    quire_contract_model::PackageReadResource::Work => NativeJoinChargeKind::Work,
                    quire_contract_model::PackageReadResource::ReadStorage => {
                        NativeJoinChargeKind::Storage
                    }
                },
                amount,
            },
            locus: self.budget.first_unpaid_locus().or(Some(self.locus)),
            configured_bound,
            consumed: self.budget.consumed(),
        }
    }

    fn denial(&self, denied: NativeJoinDeniedCharge) -> PlanAuthorityRefusal {
        PlanAuthorityRefusal::Budget {
            denied,
            locus: self.budget.first_unpaid_locus(),
            configured_bound: match denied.kind {
                NativeJoinChargeKind::Work => self.budget.work_limit(),
                NativeJoinChargeKind::Storage => self.budget.storage_limit(),
            },
            consumed: self.budget.consumed(),
        }
    }

    pub(in crate::monitor_root) fn node_id(
        &mut self,
        key: &NodeKey,
    ) -> Result<CheckedNodeId, PlanAuthorityRefusal> {
        match try_structural_id(key, self.budget, self.locus) {
            Ok(id) => Ok(id),
            Err(StructuralIdMaterializationError::Budget(denied)) => Err(self.denial(denied)),
            Err(StructuralIdMaterializationError::Allocation(error)) => {
                Err(PlanAuthorityRefusal::Allocation(error))
            }
        }
    }

    pub(in crate::monitor_root) fn work(
        &mut self,
        amount: u128,
    ) -> Result<(), PlanAuthorityRefusal> {
        self.pay(NativeJoinChargeKind::Work, amount)
    }

    pub(in crate::monitor_root) fn storage(
        &mut self,
        amount: u128,
    ) -> Result<(), PlanAuthorityRefusal> {
        self.pay(NativeJoinChargeKind::Storage, amount)
    }

    pub(in crate::monitor_root) fn text(
        &mut self,
        left: &str,
        right: &str,
    ) -> Result<bool, PlanAuthorityRefusal> {
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

    pub(in crate::monitor_root) fn node(
        &mut self,
        left: &NodeKey,
        right: &NodeKey,
    ) -> Result<bool, PlanAuthorityRefusal> {
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

    /// Compare the original admitted identities without invoking an uncharged
    /// recursive Eq or manufacturing another encoded identity.
    pub(in crate::monitor_root) fn checked_node(
        &mut self,
        left: &CheckedNodeId,
        right: &CheckedNodeId,
    ) -> Result<bool, PlanAuthorityRefusal> {
        self.storage(2)?;
        let mut pending = Vec::new();
        pending.try_reserve_exact(1)?;
        pending.push((left, right));
        loop {
            self.work(1)?;
            let Some((left, right)) = pending.pop() else {
                return Ok(true);
            };
            if !self.text(&left.package, &right.package)? {
                return Ok(false);
            }
            self.work(4)?;
            if left.ordinal != right.ordinal || left.path.len() != right.path.len() {
                return Ok(false);
            }
            for (left, right) in left.path.iter().zip(&right.path) {
                self.work(2)?;
                match (left, right) {
                    (CheckedNodePathSegment::Name(left), CheckedNodePathSegment::Name(right)) => {
                        if !self.text(left, right)? {
                            return Ok(false);
                        }
                    }
                    (CheckedNodePathSegment::Index(left), CheckedNodePathSegment::Index(right)) => {
                        self.work(2)?;
                        if left != right {
                            return Ok(false);
                        }
                    }
                    (CheckedNodePathSegment::Node(left), CheckedNodePathSegment::Node(right)) => {
                        // Reserve the actual additional stack slot before growing it.
                        if pending.len() == pending.capacity() {
                            self.storage(1)?;
                            pending.try_reserve_exact(1)?;
                        }
                        pending.push((left.as_ref(), right.as_ref()));
                    }
                    _ => return Ok(false),
                }
            }
        }
    }

    pub(in crate::monitor_root) fn subject(
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
