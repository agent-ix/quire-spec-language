// SPDX-License-Identifier: AGPL-3.0-or-later
//! EN-4's FR-331 provider entry (FR-280): the items of a provider request
//! routed to EN-4, run over the run's `CheckedPackage` and settled by
//! FR-192's one map.
//!
//! The entry reads the checked package a [`StatisticalRequest`]'s subject
//! carries and takes no `EmittedPackage`: S4 refuses to emit a package that
//! holds a probabilistic claim (FR-185), so an entry that read an emitted
//! package could serve no EN-4 item. `analyze` settles an EN-4 item through
//! the same two calls, [`check_statistical`] and FR-192's settlement, so the
//! record of an item is the same through either path (ADR-029 OP-2).

use qsl_foundation::{BackendId, RequestIndex};
use qsl_replay::statistical::{
    settle_statistical, settle_statistical_refusal, StatisticalRecord, SubjectLimits,
    UnsettledRefusal,
};
use qsl_replay::TerminalRecord;
use quire_exact::Cancel;

use super::manifest::BACKEND_ID;
use super::{check_statistical, StatisticalRequest};

/// One item routed to EN-4: its position in the provider request and the
/// statistical request built from its claim and the request's settings.
pub struct RoutedItem<'a> {
    /// The item's position in the provider request.
    pub index: RequestIndex,
    /// The item's request; its subject holds the run's `CheckedPackage`.
    pub request: StatisticalRequest<'a>,
}

/// FR-280: settle one EN-4 request over the checked package its subject
/// holds.
///
/// # Errors
///
/// [`UnsettledRefusal`] for a refusal FR-192 settles with no terminal record
/// (`NotStatistical`, `RequiresBound`, `Admission`, `Symmetry`,
/// `PredicateRefused`): the caller reports it.
pub fn serve_request(
    request: StatisticalRequest<'_>,
    cancel: &Cancel,
) -> Result<StatisticalRecord, UnsettledRefusal> {
    let simulation = &request.subject.simulation;
    let expansion = simulation.expansion;
    let meter = simulation.meter.clone();
    match check_statistical(request, cancel) {
        Ok(outcome) => Ok(settle_statistical(
            &outcome,
            SubjectLimits {
                expansion,
                meter: &meter,
            },
        )),
        Err(refusal) => settle_statistical_refusal(&refusal),
    }
}

/// FR-280: the entry for the items of a provider request routed to EN-4. It
/// returns, for each item in order, its FR-331 terminal record naming EN-4,
/// or the unsettled refusal for an item FR-192 gives no record.
pub fn serve_items(
    items: Vec<RoutedItem<'_>>,
    cancel: &Cancel,
) -> Vec<Result<TerminalRecord, UnsettledRefusal>> {
    items
        .into_iter()
        .map(|item| {
            serve_request(item.request, cancel)
                .map(|record| record.into_terminal(item.index, BackendId::new(BACKEND_ID)))
        })
        .collect()
}
