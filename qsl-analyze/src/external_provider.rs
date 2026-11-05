// SPDX-License-Identifier: AGPL-3.0-or-later
//! The external provider arm (FR-281 "External provider arm"): a registered
//! decision procedure outside QSL decides a `value-validity` item over
//! unbounded mathematical `Integer` operands, which no in-process engine
//! decides (FR-495).
//!
//! The provider's verdict is the provider's. QSL settles a `proved` verdict
//! as `proved` (uncertified: no core checker covers it) and an `unknown`
//! verdict as `inconclusive`, but a `refuted` verdict is only a candidate
//! violation: the counterexample is replayed through the native evaluator at
//! its own integers, and the item settles `refuted` only when the replay
//! reproduces the violation. Every type here is QSL's own, so QSL depends on
//! no CG crate for the seam (FR-280).

use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::{BackendId, RequestIndex};
use qsl_package::CheckedPackage;
use qsl_replay::value_check::{Selection, ValueTarget};
use qsl_replay::{Cancel, Certification, InconclusiveCause, TerminalRecord, Value};
use qsl_route::request::ItemKind;
use qsl_route::{Mode, ProviderOrigin};
use qsl_semantics::check::Capability;
use quire_semantic_value::catalog::InternalFault;

use crate::analyze::{AnalyzeItem, EngineReport, Refusal};
use crate::claim_family::claim_family;
use crate::value_check::{
    confirm_reported, manifest, ValueCounterexample, ValueRefusal, ValueReport,
};

/// What a provider is asked to decide: the item's claim target, whose
/// parameters, domains and claim a provider reads to build its query.
pub struct ProviderRequest<'t, 'p> {
    target: &'t ValueTarget<'p>,
}

impl<'t, 'p> ProviderRequest<'t, 'p> {
    /// The claim's target.
    pub fn target(&self) -> &'t ValueTarget<'p> {
        self.target
    }
}

/// What a provider reports for one item. Only `Refuted` carries evidence,
/// and QSL replays it before it counts (FR-281 "External provider arm").
#[derive(Clone, Debug)]
pub enum ProviderVerdict {
    /// The provider established that the claim holds for every assignment.
    Proved,
    /// The provider reports an assignment of the claim's parameters, in
    /// parameter order, at which the claim is violated.
    Refuted {
        /// The reported assignment.
        counterexample: Vec<Value>,
    },
    /// The provider neither proved nor refuted the claim.
    Unknown,
}

/// A decision procedure outside QSL that decides `value-validity` items over
/// unbounded `Integer` operands.
pub trait ExternalProvider: std::fmt::Debug + Send + Sync {
    /// Decide `request`, stopping early when `cancel` fires.
    fn check(&self, request: &ProviderRequest<'_, '_>, cancel: &Cancel) -> ProviderVerdict;
}

/// `Engine::external` was given a descriptor that does not advertise
/// `value-validity` in the `unbounded` mode.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("{backend:?} does not advertise value-validity in the unbounded mode")]
pub struct NotUnbounded {
    /// The descriptor's `BackendId`.
    pub backend: BackendId,
}

/// What the arm reports for one provider item.
pub(crate) enum ProviderReport {
    /// The provider proved the claim.
    Proved,
    /// The provider could not decide the claim.
    Unknown,
    /// The provider's counterexample, confirmed by replay or not.
    Replayed(ValueReport),
}

/// The `value-validity` item at `claim` of `package` that `backend`, an
/// engine built by `Engine::external`, decides, in the `unbounded` mode.
pub fn provider_item<'a>(
    index: RequestIndex,
    package: &CheckedPackage,
    claim: &OccurrenceKey,
    backend: &BackendId,
    selection: Result<ValueTarget<'a>, Selection>,
) -> AnalyzeItem<'a> {
    AnalyzeItem::new_with_engine(
        index,
        Capability::ValueValidity,
        claim_family(package, ItemKind::Formula, claim),
        Mode::Unbounded,
        backend.as_str(),
        move |cancel, engine| {
            let target = match selection {
                Ok(target) => target,
                Err(Selection::NoForm) => {
                    return EngineReport::Refused(crate::value_check::invalid_input())
                }
                Err(Selection::MissingNode) => {
                    return EngineReport::Refused(crate::value_check::incomplete_input())
                }
                Err(Selection::Fault(fault)) => {
                    return EngineReport::Refused(Refusal::Failed(fault.catalog_code()))
                }
            };
            let Some(provider) = engine.provider() else {
                return EngineReport::Refused(Refusal::Failed(
                    InternalFault::new("external-provider", "engine-holds-provider").catalog_code(),
                ));
            };
            let verdict = provider.check(&ProviderRequest { target: &target }, cancel);
            let budget = engine
                .max_cases()
                .unwrap_or(manifest::DEFAULT_MAX_CASES)
                .get();
            report(&target, verdict, cancel, budget)
        },
    )
}

/// The provider's verdict as the arm's report: a counterexample is replayed
/// before it is anything but a candidate.
fn report(
    target: &ValueTarget<'_>,
    verdict: ProviderVerdict,
    cancel: &Cancel,
    budget: u64,
) -> EngineReport {
    let counterexample = match verdict {
        ProviderVerdict::Proved => return EngineReport::Provider(Box::new(ProviderReport::Proved)),
        ProviderVerdict::Unknown => {
            return EngineReport::Provider(Box::new(ProviderReport::Unknown))
        }
        ProviderVerdict::Refuted { counterexample } => counterexample,
    };
    let parameters = target.parameters().iter().map(|p| p.wire()).collect();
    let claim = target.claim().clone();
    let replayed = |report| EngineReport::Provider(Box::new(ProviderReport::Replayed(report)));
    match confirm_reported(target, &counterexample, cancel, budget) {
        Ok((outcome, _)) => replayed(ValueReport::Found(outcome, parameters, claim)),
        Err(ValueRefusal::Unsupported(form, reason)) => {
            replayed(ValueReport::Unsupported(form, reason))
        }
        Err(ValueRefusal::Fault(fault)) => {
            EngineReport::Refused(Refusal::Failed(fault.catalog_code()))
        }
        // `confirm_reported` replays at the reported integers and never
        // asks for a bound.
        Err(ValueRefusal::RequiresBound) => EngineReport::Refused(Refusal::Failed(
            InternalFault::new("external-provider", "replay-needs-no-bound").catalog_code(),
        )),
    }
}

/// The certification of a provider's proof (QSpec FR-331, FR-354): a linked
/// first-party provider is `uncertified`, a plugin process `trusted`.
fn certification(origin: ProviderOrigin) -> Certification {
    match origin {
        ProviderOrigin::Linked => Certification::Uncertified,
        ProviderOrigin::Process => Certification::Trusted,
    }
}

/// Settle the arm's report as the item's FR-331 terminal record, with the
/// counterexample a `refuted` item keeps.
pub(crate) fn settle(
    report: ProviderReport,
    index: RequestIndex,
    backend: BackendId,
    origin: ProviderOrigin,
) -> (TerminalRecord, Option<ValueCounterexample>) {
    match report {
        ProviderReport::Proved => (
            TerminalRecord::proved_by_provider(index, backend, certification(origin)),
            None,
        ),
        ProviderReport::Unknown => (
            TerminalRecord::inconclusive(
                index,
                backend,
                InconclusiveCause::SolverInconclusive {
                    trace: None,
                    replay: None,
                },
            ),
            None,
        ),
        ProviderReport::Replayed(report) => crate::value_check::settle(report, index, backend),
    }
}
