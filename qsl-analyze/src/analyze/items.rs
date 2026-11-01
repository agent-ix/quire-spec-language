// SPDX-License-Identifier: AGPL-3.0-or-later
//! The item adapters of the engines `analyze` runs: each builds the
//! [`AnalyzeItem`] whose work calls the engine and hands its result to the
//! engine's layer-6 settlement map, or classifies its refusal by FR-281's
//! "Engine refusals" table.

use qsl_eval::simulation::model::{ClauseOutcome, DEFAULT_MAX_CLOCKS};
use qsl_eval::simulation::protocol::ProtocolTransition;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::RequestIndex;
use qsl_package::CheckedPackage;
use qsl_replay::statistical::{settle_statistical, settle_statistical_refusal, SubjectLimits};
use qsl_replay::{
    check_zone_certificate, settle_hyper_form, CertificateCheckLimits, CertificateRequest,
    DeclineCode, ModelTraceRefusal, ModelTraceReplayResult, ProofRefusalCause,
    RefinementTraceSettlement, ReplayRefusal, SubjectRefusal, TemporalCounterexample,
    TerminalValue, UnavailabilityCause, ZoneCertificateRequest,
};
use qsl_replay::{ModelCheckItem, ModelCheckLimits, ProtocolCertificateRequest, SubjectIdentity};
use qsl_route::deadlock::{ItemObligation, ObligationKind};
use qsl_route::request::{DerivedLimit, DerivedLimits, ItemKind, RequestItem, RequirementItem};
use qsl_route::Mode;
use qsl_semantics::check::{Capability, HyperForm, TerminalKeys, TerminalMismatch};
use qsl_semantics::family::UnsupportedItem;
use qsl_semantics::model::observation::AdmissionFailure;
use quire_semantic_value::catalog::CatalogCode;

use super::{AnalyzeItem, EngineReport, Refusal};
use crate::model_check::{
    check_model, check_protocol, manifest as en1, HyperRefusal, LivenessRefusal, ModelCheckRefusal,
    ModelCheckRequest, ProtocolCheckRequest,
};
use crate::statistical::{check_statistical, manifest as en4, StatisticalRequest};
use crate::zone_check::{
    certificate_item, check_timed_model, check_timed_refinement, manifest as en6,
    settle_timed_refinement, settle_zone_check, TimedRefinementRefusal, TimedRefinementRequest,
    ZoneCheckItem, ZoneCheckLimits, ZoneCheckRefusal, ZoneCheckRequest, ZoneItem,
};

/// The FR-396/FR-399 negotiation seam for a routed hyper item. HP-4 calls
/// [`settle_hyper_form`] before an engine adapter is built, then becomes the
/// existing unsupported-requested-capability disposition. The route writer
/// supplies the matching typed cause and an empty candidate set; admitted
/// forms and mismatched request items return `None` for their ordinary path.
pub fn hyper_form_item(item: &RequirementItem, form: &HyperForm) -> Option<AnalyzeItem<'static>> {
    if item.kind() != Capability::TemporalSatisfaction {
        return None;
    }
    let UnsupportedItem::HyperForm { reason } = item.unsupported()? else {
        return None;
    };
    let Some(TerminalValue::Unsupported(UnavailabilityCause::UnsupportedHyperForm {
        reason: settled_reason,
    })) = settle_hyper_form(form)
    else {
        return None;
    };
    (*reason == settled_reason).then(|| AnalyzeItem::unadapted(item.index(), item.kind()))
}

/// `runtime_invariant`/`established-invariant-broken`: the diagnostic of a
/// refusal whose own type names no code.
const BROKEN: CatalogCode = CatalogCode::new("runtime_invariant", "established-invariant-broken");
const NOT_YET: CatalogCode = CatalogCode::new("unsupported_construct", "not-yet-implemented");
const EXPRESSION_FORM: CatalogCode = CatalogCode::new("unsupported_construct", "expression-form");
const INVALID_VALUE: CatalogCode = CatalogCode::new("invalid_runtime_input", "invalid-value");

/// An EN-1 item (FR-281 Inputs): the run, the certificate request that
/// re-admits its subject for the closure checker, and the replay of a
/// counterexample (FR-128), which in production is `replay_model_trace` over
/// the same source.
pub struct ModelItem<'a> {
    /// The EN-1 run.
    pub request: ModelCheckRequest<'a>,
    /// The closure checker's request over the same subject and item.
    pub certificate: CertificateRequest<'a>,
    /// Replays the run's counterexample.
    pub replay: Box<CounterexampleReplayFn<'a>>,
}

/// The replay of an EN-1 counterexample (FR-128).
pub type CounterexampleReplayFn<'a> =
    dyn FnOnce(&TemporalCounterexample) -> Result<ModelTraceReplayResult, ReplayRefusal> + 'a;

/// The `temporal-satisfaction` item that runs `item` on EN-1 and settles it
/// by `settle_model_check` (FR-127). It is `Supported(EN-1)` until the
/// caller's negotiation says otherwise ([`AnalyzeItem::disposed`]).
///
/// # Errors
///
/// [`DerivedItemMismatch::DerivedRun`] when `item` runs a derived item
/// (deadlock or race freedom): a derived item settles only through
/// [`derived_model_item`] or [`derived_protocol_item`], which join its
/// verdict to the request item it answers (FR-411, QSpec FR-368).
pub fn model_item<'a>(
    index: RequestIndex,
    item: ModelItem<'a>,
) -> Result<AnalyzeItem<'a>, DerivedItemMismatch> {
    match item.request.item {
        ModelCheckItem::Clause { .. } => Ok(run_model_item(index, item)),
        ModelCheckItem::DeadlockFreedom | ModelCheckItem::RaceFreedom => {
            Err(DerivedItemMismatch::DerivedRun { index })
        }
    }
}

/// [`model_item`] with no check of what `item` runs: the derived adapters'
/// path once they have joined the run to its request item.
fn run_model_item(index: RequestIndex, item: ModelItem<'_>) -> AnalyzeItem<'_> {
    AnalyzeItem::new(
        index,
        Capability::TemporalSatisfaction,
        Mode::Bounded,
        en1::BACKEND_ID,
        move |cancel| {
            let ModelItem {
                request,
                certificate,
                replay,
            } = item;
            let limits = request.limits;
            match check_model(request, cancel) {
                Ok(outcome) => EngineReport::Model(Box::new(qsl_replay::settle_model_check(
                    &outcome,
                    &limits,
                    &certificate,
                    cancel,
                    replay,
                ))),
                Err(refusal) => EngineReport::Refused(model_refusal(&refusal)),
            }
        },
    )
}

/// An EN-1 item over a protocol subject (FR-215): the run, the certificate
/// request that re-admits its subject for the protocol closure checker, and the replay of a counterexample
/// (FR-217), which in production is `replay_protocol_trace` over the same
/// source.
pub struct ProtocolItem<'a> {
    /// The EN-1 run over the protocol subject.
    pub request: ProtocolCheckRequest<'a>,
    /// The protocol closure checker's request over the same subject and
    /// item.
    pub certificate: ProtocolCertificateRequest<'a>,
    /// Replays the run's counterexample.
    pub replay: Box<ProtocolReplayFn<'a>>,
}

/// The replay of an EN-1 counterexample over a protocol subject (FR-217).
pub type ProtocolReplayFn<'a> = dyn FnOnce(
        &TemporalCounterexample<ProtocolTransition>,
    ) -> Result<ModelTraceReplayResult, ReplayRefusal>
    + 'a;

/// The `temporal-satisfaction` item that runs `item` on EN-1 over its
/// protocol subject and settles it by `settle_protocol_check` (FR-215,
/// FR-217). It is `Supported(EN-1)` until the caller's negotiation says
/// otherwise ([`AnalyzeItem::disposed`]).
///
/// # Errors
///
/// [`DerivedItemMismatch::DerivedRun`] when `item` runs a derived item, as
/// [`model_item`] refuses it.
pub fn protocol_item<'a>(
    index: RequestIndex,
    item: ProtocolItem<'a>,
) -> Result<AnalyzeItem<'a>, DerivedItemMismatch> {
    match item.request.item {
        ModelCheckItem::Clause { .. } => Ok(run_protocol_item(index, item)),
        ModelCheckItem::DeadlockFreedom | ModelCheckItem::RaceFreedom => {
            Err(DerivedItemMismatch::DerivedRun { index })
        }
    }
}

/// [`protocol_item`] with no check of what `item` runs.
fn run_protocol_item(index: RequestIndex, item: ProtocolItem<'_>) -> AnalyzeItem<'_> {
    AnalyzeItem::new(
        index,
        Capability::TemporalSatisfaction,
        Mode::Bounded,
        en1::BACKEND_ID,
        move |cancel| {
            let ProtocolItem {
                request,
                certificate,
                replay,
            } = item;
            let limits = request.limits;
            match check_protocol(request, cancel) {
                Ok(outcome) => EngineReport::Protocol(Box::new(qsl_replay::settle_protocol_check(
                    &outcome,
                    &limits,
                    &certificate,
                    cancel,
                    replay,
                ))),
                Err(refusal) => EngineReport::Refused(model_refusal(&refusal)),
            }
        },
    )
}

/// The `probabilistic-satisfaction` item that runs `request` on EN-4 and
/// settles it by `check_statistical` and FR-192's map, the two calls the
/// FR-280 provider entry makes.
pub fn statistical_item<'a>(
    index: RequestIndex,
    request: StatisticalRequest<'a>,
) -> AnalyzeItem<'a> {
    AnalyzeItem::new(
        index,
        Capability::ProbabilisticSatisfaction,
        Mode::Bounded,
        en4::BACKEND_ID,
        move |cancel| {
            let simulation = &request.subject.simulation;
            let expansion = simulation.expansion;
            let meter = simulation.meter;
            match check_statistical(request, cancel) {
                Ok(outcome) => EngineReport::Statistical(Box::new(settle_statistical(
                    &outcome,
                    SubjectLimits {
                        expansion,
                        meter: &meter,
                    },
                ))),
                Err(refusal) => statistical_refusal(&refusal),
            }
        },
    )
}

/// An EN-6 item over a timed subject (FR-243): the zone search, the
/// checker's request that re-admits the subject for FR-245's certificate
/// check, and the replay of a
/// counterexample (FR-237), which in production is
/// `replay_timed_counterexample` over the same source.
pub struct TimedItem<'a, S> {
    /// The EN-6 run.
    pub request: ZoneCheckRequest<'a, 'a, 'a, S>,
    /// The checker's request over the same subject and item.
    pub certificate: ZoneCertificateRequest<'a>,
    /// The checker's budgets.
    pub certificate_limits: CertificateCheckLimits,
    /// Replays the run's counterexample.
    pub replay: Box<CounterexampleReplayFn<'a>>,
}

/// The `temporal-satisfaction` item that runs `item` on EN-6 and settles it
/// by `settle_zone_check` (FR-235), its certificate checked by FR-245's
/// `check_zone_certificate`. It is `Supported(EN-6)` until the caller's
/// negotiation says otherwise ([`AnalyzeItem::disposed`]).
///
/// # Errors
///
/// [`DerivedItemMismatch::DerivedRun`] when `item` runs a derived item
/// (deadlock or time-lock freedom): it settles only through
/// [`derived_zone_item`] (FR-411, QSpec FR-368).
pub fn zone_item<'a, S: Clone + Eq + 'a>(
    index: RequestIndex,
    item: TimedItem<'a, S>,
) -> Result<AnalyzeItem<'a>, DerivedItemMismatch> {
    match item.request.item {
        ZoneCheckItem::Clause { .. } => Ok(run_zone_item(index, item)),
        ZoneCheckItem::DeadlockFreedom | ZoneCheckItem::TimeLockFreedom => {
            Err(DerivedItemMismatch::DerivedRun { index })
        }
    }
}

/// [`zone_item`] with no check of what `item` runs.
fn run_zone_item<'a, S: Clone + Eq + 'a>(
    index: RequestIndex,
    item: TimedItem<'a, S>,
) -> AnalyzeItem<'a> {
    AnalyzeItem::new(
        index,
        Capability::TemporalSatisfaction,
        Mode::Bounded,
        en6::BACKEND_ID,
        move |cancel| {
            let TimedItem {
                request,
                certificate,
                certificate_limits,
                replay,
            } = item;
            let obligation = request.obligation.clone();
            match check_timed_model(request, cancel) {
                Ok(outcome) => EngineReport::Zone(Box::new(settle_zone_check(
                    ZoneItem::Outcome(&outcome),
                    cancel,
                    |proof| {
                        check_zone_certificate(
                            certificate,
                            &obligation,
                            certificate_item,
                            proof,
                            certificate_limits,
                            cancel,
                        )
                    },
                    replay,
                ))),
                Err(refusal) => EngineReport::Refused(zone_refusal(&refusal)),
            }
        },
    )
}

/// Why a derived item adapter built no item: the engine run is not the
/// derived request item's (QSpec FR-368).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum DerivedItemMismatch {
    /// The request item is not a derived item of a kind the engine decides.
    #[error("request item {index:?} is no derived item this engine decides")]
    NotDerived {
        /// The request item's index.
        index: RequestIndex,
    },
    /// The engine run or its certificate check decides another item than
    /// the request item's kind, or is over another subject, a protocol
    /// subject's resolved models included (QSpec FR-425), than the request
    /// item's obligation.
    #[error("the run decides another item than request item {index:?}'s {kind:?}")]
    OtherItem {
        /// The request item's index.
        index: RequestIndex,
        /// The request item's kind.
        kind: ItemKind,
    },
    /// The timed subject, or its certificate check, bounds the live clocks
    /// by another value than the request item's `max_clocks`: the caller's
    /// setting, or EN-6's published default when it has none (QSpec FR-368
    /// "Negotiation", FR-415).
    #[error("request item {index:?} bounds the live clocks by {expected}, the run by {run}")]
    ClockBound {
        /// The request item's index.
        index: RequestIndex,
        /// The request item's `max_clocks`.
        expected: u64,
        /// The bound the run or its check holds.
        run: u64,
    },
    /// A derived item's run handed to [`model_item`], [`protocol_item`] or
    /// [`zone_item`], which settle it with no join to its request item: a
    /// derived item settles only through its derived adapter (FR-411).
    #[error("request item {index:?}'s run is a derived item's; it settles only through its derived adapter")]
    DerivedRun {
        /// The request item's index.
        index: RequestIndex,
    },
    /// A run or certificate check is over a package that holds no
    /// `model_root` anchor equal to the derived request item's claim, the
    /// node of its obligation identity (QSpec FR-368, ADR-013 O-09): the
    /// run answers another subject's item.
    #[error("stale_dependency/content-mismatch: no model_root of the run's package anchors request item {index:?} at {occurrence}")]
    OtherAnchor {
        /// The request item's index.
        index: RequestIndex,
        /// The request item's anchor occurrence.
        occurrence: OccurrenceKey,
    },
    /// A model subject's deadlock-freedom run or its certificate check is
    /// over a package whose terminal declaration differs from the one the
    /// request item was written under (FR-411 "Deadlock-freedom verdict
    /// join"); no verdict is settled.
    #[error("stale_dependency/content-mismatch: request item {index:?}'s {mismatch}")]
    TerminalDeclaration {
        /// The request item's index.
        index: RequestIndex,
        /// The first model alias whose declaration differs, and the two
        /// terminal node keys.
        mismatch: TerminalMismatch,
    },
}

/// QSpec FR-368 "Claim and anchor": every package a run or a certificate
/// check of `derived` is over holds the `model_root` node `derived` is
/// anchored at, with its `generated` occurrence 0 (FR-411
/// "Deadlock-freedom verdict join": the node of the obligation identity).
fn join_anchor<'p>(
    derived: &RequestItem,
    packages: impl IntoIterator<Item = &'p CheckedPackage>,
) -> Result<(), DerivedItemMismatch> {
    let claim = derived.occurrence();
    for package in packages {
        let anchored = package
            .graph()
            .model_roots()
            .any(|(_, root)| OccurrenceKey::generated(root) == *claim);
        if !anchored {
            return Err(DerivedItemMismatch::OtherAnchor {
                index: derived.index(),
                occurrence: claim.clone(),
            });
        }
    }
    Ok(())
}

/// FR-411 "Deadlock-freedom verdict join": the terminal node keys `derived`
/// was written under, compared with those of each package a run or a
/// certificate check of it is over.
fn join_terminals<'p>(
    derived: &RequestItem,
    packages: impl IntoIterator<Item = &'p CheckedPackage>,
) -> Result<TerminalKeys, DerivedItemMismatch> {
    let index = derived.index();
    let expected = derived
        .derived_terminals()
        .ok_or(DerivedItemMismatch::NotDerived { index })?;
    for package in packages {
        if let Some(mismatch) = expected.mismatch(&package.graph().terminal_keys()) {
            return Err(DerivedItemMismatch::TerminalDeclaration { index, mismatch });
        }
    }
    Ok(expected.clone())
}

/// `replay`, refusing `stale_dependency`/`content-mismatch` when the
/// recompiled package it replayed over declares other terminal states than
/// `expected` (FR-411 "Deadlock-freedom verdict join"), so a refutation
/// found under one declaration never settles the item under another.
fn joined_replay<'a>(
    replay: Box<CounterexampleReplayFn<'a>>,
    expected: TerminalKeys,
) -> Box<CounterexampleReplayFn<'a>> {
    Box::new(move |payload| {
        let replayed = replay(payload)?;
        match expected.mismatch(&replayed.terminals) {
            None => Ok(replayed),
            Some(mismatch) => Err(ReplayRefusal::ModelTrace(Box::new(
                ModelTraceRefusal::TerminalDeclaration(mismatch),
            ))),
        }
    })
}

/// The run's limits under a derived item's caller settings (QSpec FR-368
/// "Negotiation of a derived or refinement item"): each limit the caller set
/// replaces `default`'s, the routed provider's published value, and every
/// other limit keeps it.
fn derived_limit(limits: &DerivedLimits, limit: DerivedLimit, default: u64) -> u64 {
    limits.get(limit).unwrap_or(default)
}

/// EN-1's explored-graph limits under a derived item's caller settings:
/// `max_states` and `max_transitions`, each over EN-1's default when unset.
fn derived_graph_limits(limits: &DerivedLimits) -> qsl_eval::simulation::Limits {
    let defaults = ModelCheckLimits::default().limits;
    let widen = |value: usize| u64::try_from(value).unwrap_or(u64::MAX);
    let narrow = |value: u64| usize::try_from(value).unwrap_or(usize::MAX);
    let limit = |limit, default| narrow(derived_limit(limits, limit, widen(default)));
    qsl_eval::simulation::Limits {
        max_states: limit(DerivedLimit::MaxStates, defaults.max_states),
        max_transitions: limit(DerivedLimit::MaxTransitions, defaults.max_transitions),
    }
}

/// The EN-1 item that runs the derived `deadlock-freedom` request item
/// `derived` (FR-124): [`model_item`] at the request item's index, with the
/// request item's caller-set `max_states` and `max_transitions` over EN-1's
/// defaults, and no `max_depth`, since a derived item is an exhaustive
/// search (QSpec FR-368-AC-20).
///
/// `obligation` is `derived`'s obligation identity, as the written request
/// gives it: the run and its certificate check must be over its subject.
///
/// # Errors
///
/// [`DerivedItemMismatch`] when `derived` is no `deadlock-freedom` item, or
/// `item` runs another item or is over another subject than `obligation`'s,
/// or its run or certificate check is over a package with another terminal
/// declaration than `derived` was written under (FR-411). A replay over
/// such a package refuses `stale_dependency`/`content-mismatch`, so the
/// item settles `inconclusive`, never `refuted`.
/// A run or check over a package that does not hold `derived`'s anchor
/// refuses [`DerivedItemMismatch::OtherAnchor`].
pub fn derived_model_item<'a>(
    derived: &RequestItem,
    obligation: &ItemObligation<SubjectIdentity>,
    mut item: ModelItem<'a>,
) -> Result<AnalyzeItem<'a>, DerivedItemMismatch> {
    let index = derived.index();
    let (Some(ItemKind::DeadlockFreedom), Some(limits)) =
        (derived.item_kind(), derived.derived_limits())
    else {
        return Err(DerivedItemMismatch::NotDerived { index });
    };
    // A subject other than the one the request was written over.
    let over = |subject: &qsl_eval::model_subject::ModelSubject<'_>| {
        SubjectIdentity::of(subject) == obligation.subject
    };
    if !matches!(item.request.item, ModelCheckItem::DeadlockFreedom)
        || !matches!(item.certificate.item, ModelCheckItem::DeadlockFreedom)
        || obligation.kind != ObligationKind::DeadlockFreedom
        || !over(&item.request.subject)
        || !over(&item.certificate.subject)
    {
        return Err(DerivedItemMismatch::OtherItem {
            index,
            kind: ItemKind::DeadlockFreedom,
        });
    }
    let packages = [
        item.request.subject.simulation.package,
        item.certificate.subject.simulation.package,
    ];
    join_anchor(derived, packages)?;
    let expected = join_terminals(derived, packages)?;
    item.replay = joined_replay(item.replay, expected);
    item.request.limits.limits = derived_graph_limits(limits);
    item.request.max_depth = None;
    Ok(run_model_item(index, item))
}

/// The EN-1 item that runs the derived `deadlock-freedom` (QSpec FR-429) or
/// `race-freedom` (QSpec FR-438) request item `derived` over a protocol
/// subject: [`protocol_item`] at the request item's index, with no
/// `max_depth`, since a derived item is an exhaustive search, and the
/// request item's caller-set limits over EN-1's defaults (QSpec FR-368
/// "Negotiation of a derived or refinement item"): `max_states` and
/// `max_transitions`, and `max_store_buffer`, `max_messages` and
/// `max_live_instances` for both the run and its certificate check, an
/// unset one keeping EN-1's default.
///
/// `obligation` is `derived`'s obligation identity, as the written request
/// gives it: its resolved models are part of the protocol subject (QSpec
/// FR-425), so the run and its certificate check must be over exactly
/// those models.
///
/// # Errors
///
/// [`DerivedItemMismatch`] when `derived` is no `deadlock-freedom` or
/// `race-freedom` item, or `item`'s run or its certificate check decides
/// another item or is over other resolved models than `obligation`'s.
pub fn derived_protocol_item<'a, S>(
    derived: &RequestItem,
    obligation: &ItemObligation<S>,
    mut item: ProtocolItem<'a>,
) -> Result<AnalyzeItem<'a>, DerivedItemMismatch> {
    let index = derived.index();
    let (Some(kind), Some(limits)) = (derived.item_kind(), derived.derived_limits()) else {
        return Err(DerivedItemMismatch::NotDerived { index });
    };
    let decides: fn(&ModelCheckItem<'_>) -> bool = match kind {
        ItemKind::DeadlockFreedom => |run| matches!(run, ModelCheckItem::DeadlockFreedom),
        ItemKind::RaceFreedom => |run| matches!(run, ModelCheckItem::RaceFreedom),
        ItemKind::Formula | ItemKind::TimeLockFreedom | ItemKind::Refinement => {
            return Err(DerivedItemMismatch::NotDerived { index })
        }
    };
    let models = match (&obligation.kind, kind) {
        (ObligationKind::ProtocolDeadlockFreedom { models }, ItemKind::DeadlockFreedom)
        | (ObligationKind::RaceFreedom { models }, ItemKind::RaceFreedom) => models,
        _ => return Err(DerivedItemMismatch::OtherItem { index, kind }),
    };
    let over = |memory: &qsl_eval::simulation::protocol::ResolvedMemory| {
        memory.parallels().len() == models.len()
            && memory
                .parallels()
                .iter()
                .zip(models)
                .all(|(resolved, (parallel, model))| {
                    resolved.parallel == *parallel && resolved.model == *model
                })
    };
    if !decides(&item.request.item)
        || !decides(&item.certificate.item)
        || !over(&item.request.subject.memory)
        || !over(&item.certificate.subject.memory)
    {
        return Err(DerivedItemMismatch::OtherItem { index, kind });
    }
    item.request.limits.limits = derived_graph_limits(limits);
    item.request.max_depth = None;
    let max_store_buffer = limits.get(DerivedLimit::MaxStoreBuffer);
    let max_messages = limits.get(DerivedLimit::MaxMessages);
    let max_live_instances = limits.get(DerivedLimit::MaxLiveInstances);
    item.request.max_store_buffer = max_store_buffer;
    item.request.max_messages = max_messages;
    item.request.max_live_instances = max_live_instances;
    item.certificate.max_store_buffer = max_store_buffer;
    item.certificate.max_messages = max_messages;
    item.certificate.max_live_instances = max_live_instances;
    Ok(run_protocol_item(index, item))
}

/// The EN-6 item that runs the derived `deadlock-freedom` or
/// `time-lock-freedom` request item `derived` over a timed subject
/// (FR-232, FR-243): [`zone_item`] at the request item's index, its run's
/// obligation the item's kind, with the request item's caller-set
/// `max_states` and `max_transitions` bounding the symbolic states and
/// edges over EN-6's defaults.
///
/// # Errors
///
/// [`DerivedItemMismatch`] when `derived` is no such item, `item` runs
/// another item, or `item`'s timed subject or certificate check bounds the
/// live clocks by another value than the item's `max_clocks`, or, for a
/// `deadlock-freedom` item, its run or its certificate check is over a
/// package with another terminal declaration than `derived` was written
/// under (FR-411);
/// its replay then refuses as [`derived_model_item`]'s does. A run or check
/// over a package that does not hold `derived`'s anchor refuses
/// [`DerivedItemMismatch::OtherAnchor`].
pub fn derived_zone_item<'a, S: Clone + Eq + 'a>(
    derived: &RequestItem,
    mut item: TimedItem<'a, S>,
) -> Result<AnalyzeItem<'a>, DerivedItemMismatch> {
    let index = derived.index();
    let (Some(kind), Some(limits)) = (derived.item_kind(), derived.derived_limits()) else {
        return Err(DerivedItemMismatch::NotDerived { index });
    };
    let runs = match kind {
        ItemKind::DeadlockFreedom => {
            matches!(item.request.item, ZoneCheckItem::DeadlockFreedom)
                && item.request.obligation.kind == ObligationKind::DeadlockFreedom
        }
        ItemKind::TimeLockFreedom => {
            matches!(item.request.item, ZoneCheckItem::TimeLockFreedom)
                && item.request.obligation.kind == ObligationKind::TimeLockFreedom
        }
        ItemKind::Formula | ItemKind::RaceFreedom | ItemKind::Refinement => {
            return Err(DerivedItemMismatch::NotDerived { index })
        }
    };
    if !runs {
        return Err(DerivedItemMismatch::OtherItem { index, kind });
    }
    // The run's own package as well as the check's: a symmetry-reduced
    // deadlock-freedom run settles `Proved{Reduced, Uncertified}` with no
    // certificate check (FR-160), so the run alone can decide the verdict.
    let packages = [
        item.request.subject.system().package(),
        item.certificate.simulation.package,
    ];
    join_anchor(derived, packages)?;
    if kind == ItemKind::DeadlockFreedom {
        let expected = join_terminals(derived, packages)?;
        item.replay = joined_replay(item.replay, expected);
    }
    // The clock bound is fixed when the timed subject is built, so the
    // adapter checks it rather than setting it.
    let expected = derived_limit(limits, DerivedLimit::MaxClocks, DEFAULT_MAX_CLOCKS.get());
    for run in [
        item.request.subject.max_clocks().get(),
        item.certificate.max_clocks.get(),
    ] {
        if run != expected {
            return Err(DerivedItemMismatch::ClockBound {
                index,
                expected,
                run,
            });
        }
    }
    let defaults = ZoneCheckLimits::default();
    item.request.limits.max_symbolic_states = derived_limit(
        limits,
        DerivedLimit::MaxStates,
        defaults.max_symbolic_states,
    );
    item.request.limits.max_symbolic_transitions = derived_limit(
        limits,
        DerivedLimit::MaxTransitions,
        defaults.max_symbolic_transitions,
    );
    Ok(run_zone_item(index, item))
}

/// An EN-6 timed refinement item (QSpec FR-430, FR-144): the safety half's
/// run, the checker's request that recompiles both subjects for FR-245's
/// check of the refinement form, and the replay of its counterexample
/// (FR-145 over FR-241's delays), which in production is
/// `replay_timed_refinement_trace` over the same subjects.
pub struct TimedRefinementItem<'a, S> {
    /// The EN-6 run.
    pub request: TimedRefinementRequest<'a, S>,
    /// The checker's request over the same refinement and subjects.
    pub certificate: ZoneCertificateRequest<'a>,
    /// The checker's budgets.
    pub certificate_limits: CertificateCheckLimits,
    /// Replays the run's counterexample.
    pub replay: Box<RefinementReplayFn<'a>>,
}

/// The replay of a timed refinement counterexample (FR-145).
pub type RefinementReplayFn<'a> =
    dyn FnOnce(&TemporalCounterexample) -> Result<RefinementTraceSettlement, ReplayRefusal> + 'a;

/// The `temporal-satisfaction` refinement item over a `time dense`
/// concrete subject, run on EN-6 and settled by `settle_timed_refinement`.
/// A safety half that holds settles `proved` only when FR-245's
/// `check_zone_certificate` accepts its certificate (FR-144,
/// FR-430-AC-8); the item is then its liveness half's, when `F_A` is
/// non-empty.
pub fn timed_refinement_item<'a, S: Clone + Eq + 'a>(
    index: RequestIndex,
    item: TimedRefinementItem<'a, S>,
) -> AnalyzeItem<'a> {
    AnalyzeItem::new(
        index,
        Capability::TemporalSatisfaction,
        Mode::Bounded,
        en6::BACKEND_ID,
        move |cancel| {
            let TimedRefinementItem {
                request,
                certificate,
                certificate_limits,
                replay,
            } = item;
            let obligation = request.obligation.clone();
            match check_timed_refinement(request, cancel) {
                Ok(outcome) => EngineReport::Zone(Box::new(settle_timed_refinement(
                    &outcome,
                    cancel,
                    |proof| {
                        check_zone_certificate(
                            certificate,
                            &obligation,
                            certificate_item,
                            proof,
                            certificate_limits,
                            cancel,
                        )
                    },
                    replay,
                ))),
                Err(refusal) => EngineReport::Refused(timed_refinement_refusal(&refusal)),
            }
        },
    )
}

/// The FR-281 "Engine refusals" row of an EN-6 refusal.
pub(super) fn zone_refusal(refusal: &ZoneCheckRefusal) -> Refusal {
    match refusal {
        ZoneCheckRefusal::RequiresBound(_) => Refusal::RequiresBound,
        ZoneCheckRefusal::Symmetry(_) => symmetry(),
        // The forms EN-6 does not decide yet.
        ZoneCheckRefusal::UnsupportedForm | ZoneCheckRefusal::CarriedClocks => {
            Refusal::Unsupported(NOT_YET)
        }
        // A request defect: an obligation that is not the item's.
        ZoneCheckRefusal::ObligationMismatch => Refusal::Failed(INVALID_VALUE),
        // Engine defects, and refusals whose types name no catalog code:
        // `failed` with the broken-invariant diagnostic.
        ZoneCheckRefusal::Extent(_)
        | ZoneCheckRefusal::Atom(_)
        | ZoneCheckRefusal::Automaton(_)
        | ZoneCheckRefusal::Fault(_) => Refusal::Failed(BROKEN),
    }
}

/// The FR-281 "Engine refusals" row of an EN-6 timed refinement refusal.
fn timed_refinement_refusal(refusal: &TimedRefinementRefusal) -> Refusal {
    match refusal {
        TimedRefinementRefusal::Zone(zone) => zone_refusal(zone),
        TimedRefinementRefusal::Subject(SubjectRefusal::Admission(failure)) => admission(failure),
        // The subjects EN-6 does not refine yet.
        TimedRefinementRefusal::Subject(
            SubjectRefusal::ProtocolSubject | SubjectRefusal::ProtocolAbstractSide,
        ) => Refusal::Unsupported(NOT_YET),
        // A request defect: a refinement with no model, a universe the
        // request does not give, or a concrete subject that is not timed.
        TimedRefinementRefusal::Subject(SubjectRefusal::NoModels | SubjectRefusal::NoUniverse)
        | TimedRefinementRefusal::Timed(_) => Refusal::Failed(INVALID_VALUE),
    }
}

fn admission(failure: &AdmissionFailure) -> Refusal {
    let declined = |cause, record: &qsl_semantics::model::observation::AdmissionRecord| {
        Refusal::Declined(
            cause,
            DeclineCode::Qsl(
                qsl_foundation::diagnostic::Code::from_code(record.catalog.code())
                    .unwrap_or(qsl_foundation::diagnostic::Code::RuntimeInvariant),
            ),
            record.catalog,
        )
    };
    match failure {
        AdmissionFailure::Refused(record) => declined(ProofRefusalCause::InvalidInput, record),
        AdmissionFailure::Incomplete(record) => {
            declined(ProofRefusalCause::IncompleteInput, record)
        }
        AdmissionFailure::Fault(fault) => Refusal::Failed(fault.catalog_code()),
    }
}

fn symmetry() -> Refusal {
    Refusal::Declined(
        ProofRefusalCause::InvalidInput,
        DeclineCode::Qsl(qsl_foundation::Code::InvalidRuntimeInput),
        INVALID_VALUE,
    )
}

/// The FR-281 "Engine refusals" row of an EN-1 refusal: `unsupported` only
/// for a refusal whose own FR-272 code is `unsupported_construct` or
/// `unsupported_projection`, `failed` for every other one.
pub(super) fn model_refusal(refusal: &ModelCheckRefusal) -> Refusal {
    match refusal {
        ModelCheckRefusal::Admission(failure) => admission(failure),
        ModelCheckRefusal::RequiresBound(_) => Refusal::RequiresBound,
        ModelCheckRefusal::Symmetry(_) => symmetry(),
        ModelCheckRefusal::UnknownConstraint { .. } => Refusal::Failed(INVALID_VALUE),
        ModelCheckRefusal::Fault(fault) => Refusal::Failed(fault.catalog_code()),
        // A refused `holds` evaluation settles as FR-203 and FR-192 settle
        // the same outcome: `failed` with the refusal's own code and cause,
        // and a broken invariant for a refusal that names none or an outcome
        // that is no refusal (QSL-1039).
        ModelCheckRefusal::AtomRefused(ClauseOutcome::KernelRefused(refusal)) => {
            match (refusal.code(), refusal.cause()) {
                (Some(code), Some(cause)) => Refusal::Failed(CatalogCode::new(code, cause)),
                _ => Refusal::Failed(BROKEN),
            }
        }
        ModelCheckRefusal::AtomRefused(ClauseOutcome::FamilyRefused(code)) => {
            Refusal::Failed(*code)
        }
        ModelCheckRefusal::AtomRefused(_) => Refusal::Failed(BROKEN),
        // An unobservable state settles by the stop's own cause, which is
        // the broken invariant for an internal failure.
        ModelCheckRefusal::StateUnobservable(stop) => Refusal::Failed(stop.cause),
        // The forms and subjects EN-1 does not decide yet.
        ModelCheckRefusal::ProtocolRefinement
        | ModelCheckRefusal::FormNotDecided { .. }
        | ModelCheckRefusal::CaptureRead { .. }
        | ModelCheckRefusal::WeakLiveness
        | ModelCheckRefusal::ReducedLiveness
        | ModelCheckRefusal::ProtocolAbstractSide
        | ModelCheckRefusal::Hyper(
            HyperRefusal::FormNotDecided { .. } | HyperRefusal::NotSingleExistential { .. },
        ) => Refusal::Unsupported(NOT_YET),
        // Clauses and operations the engine cannot read: unsupported forms.
        ModelCheckRefusal::Translation(_) | ModelCheckRefusal::Footprint(_) => {
            Refusal::Unsupported(EXPRESSION_FORM)
        }
        // A request defect: an operation or subject count the request names
        // wrongly, or the race-freedom item over a model subject.
        ModelCheckRefusal::RaceOverModel
        | ModelCheckRefusal::Liveness(LivenessRefusal::UnknownOperation { .. })
        | ModelCheckRefusal::Hyper(
            HyperRefusal::UnknownOperation { .. } | HyperRefusal::SubjectCount { .. },
        ) => Refusal::Failed(INVALID_VALUE),
        // Engine defects, and refusals whose types name no catalog code
        // (a spec point for STD-444's follow-up): `failed` with the
        // broken-invariant diagnostic.
        ModelCheckRefusal::Liveness(LivenessRefusal::Automaton(_))
        | ModelCheckRefusal::Hyper(HyperRefusal::Automaton(_) | HyperRefusal::NoPartnerPath)
        | ModelCheckRefusal::Extent(_)
        | ModelCheckRefusal::Protocol(_)
        | ModelCheckRefusal::Encoding(_)
        | ModelCheckRefusal::Key(_)
        | ModelCheckRefusal::Item(_) => Refusal::Failed(BROKEN),
    }
}

/// An EN-4 refusal: FR-192's map settles most; the rest are the "Engine
/// refusals" rows.
pub(super) fn statistical_refusal(
    refusal: &qsl_replay::statistical::StatisticalRefusal,
) -> EngineReport {
    use qsl_replay::statistical::StatisticalRefusal as R;
    EngineReport::Refused(match refusal {
        R::Admission(failure) => admission(failure),
        R::Symmetry(_) => symmetry(),
        R::RequiresBound(_) => Refusal::RequiresBound,
        R::NotStatistical { .. } => Refusal::NotStatistical,
        // FR-192's map settles the rest, a refused predicate by FR-192-AC-7's
        // rows (its own code, or `incomplete`/`limit-reached`).
        settled => match settle_statistical_refusal(settled) {
            Ok(record) => return EngineReport::Statistical(Box::new(record)),
            Err(_) => Refusal::Failed(BROKEN),
        },
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use ix_trace_rs::trace;
    use qsl_foundation::Code;
    use qsl_replay::statistical::StatisticalRefusal;
    use qsl_replay::{IncompleteCause, TerminalValue};
    use qsl_semantics::model::observation::AdmissionRecord;

    use super::*;

    fn record(code: Code, cause: &'static str) -> AdmissionRecord {
        AdmissionRecord {
            catalog: CatalogCode::new(code.as_str(), cause),
            fields: BTreeMap::new(),
            path: Vec::new(),
            kernel: None,
        }
    }

    fn code(refusal: &Refusal) -> Option<String> {
        match refusal {
            Refusal::Declined(_, _, code) | Refusal::Unsupported(code) | Refusal::Failed(code) => {
                Some(code.to_string())
            }
            Refusal::RequiresBound | Refusal::NotStatistical => None,
        }
    }

    /// FR-281-AC-11: a refused admission is `declined` `invalid-input` and an
    /// incomplete one `incomplete-input`, each carrying the record's full
    /// code and cause, and an admission fault is `failed`.
    #[trace("TC-763", "FR-281-AC-11")]
    #[test]
    fn admission_declines_with_the_records_full_diagnostic() {
        let refused = AdmissionFailure::Refused(Box::new(record(
            Code::InvalidRuntimeInput,
            "wrong-role-mapping",
        )));
        let Refusal::Declined(cause, decline, diagnostic) = admission(&refused) else {
            panic!("expected declined");
        };
        assert_eq!(cause, ProofRefusalCause::InvalidInput);
        assert_eq!(decline, DeclineCode::Qsl(Code::InvalidRuntimeInput));
        assert_eq!(
            diagnostic.to_string(),
            "invalid_runtime_input/wrong-role-mapping"
        );

        let incomplete = AdmissionFailure::Incomplete(Box::new(record(
            Code::UnavailableObservation,
            "missing-required-artifact",
        )));
        let Refusal::Declined(cause, _, diagnostic) = admission(&incomplete) else {
            panic!("expected declined");
        };
        assert_eq!(cause, ProofRefusalCause::IncompleteInput);
        assert_eq!(
            diagnostic.to_string(),
            "unavailable_observation/missing-required-artifact"
        );
    }

    /// FR-281-AC-11: a malformed symmetry declaration is `declined`,
    /// `invalid-input`, `invalid_runtime_input`/`invalid-value`.
    #[trace("TC-763", "FR-281-AC-11")]
    #[test]
    fn symmetry_declines_invalid_value() {
        let Refusal::Declined(cause, decline, diagnostic) = symmetry() else {
            panic!("expected declined");
        };
        assert_eq!(cause, ProofRefusalCause::InvalidInput);
        assert_eq!(decline, DeclineCode::Qsl(Code::InvalidRuntimeInput));
        assert_eq!(diagnostic, INVALID_VALUE);
    }

    /// QSL-1039: a refused `holds` atom settles `failed` with the refusal's
    /// own code and cause as FR-203 and FR-192 settle the same outcome, a
    /// refusal naming none and a non-refusal settle the broken invariant,
    /// and an unobservable state settles by its stop's own cause.
    #[trace("TC-763")]
    #[test]
    fn a_refused_holds_atom_and_an_unobservable_state_settle_failed_with_their_own_code() {
        use quire_exact::{BoundViolation, CardinalityBound, CollectionKind, Refusal as Kernel};

        let failed = |refusal: ModelCheckRefusal| match model_refusal(&refusal) {
            Refusal::Failed(code) => code,
            _ => panic!("expected failed"),
        };
        let above = Kernel::CardinalityOutOfBound {
            violation: BoundViolation::AboveMaximum,
            kind: CollectionKind::Set,
            bound: CardinalityBound::new(0, 1).expect("a bound"),
            count: 2,
        };
        assert_eq!(
            failed(ModelCheckRefusal::AtomRefused(
                ClauseOutcome::KernelRefused(above)
            )),
            CatalogCode::new("cardinality_out_of_bound", "above-maximum")
        );
        let family = CatalogCode::new("integer_out_of_domain", "outside-domain");
        assert_eq!(
            failed(ModelCheckRefusal::AtomRefused(
                ClauseOutcome::FamilyRefused(family)
            )),
            family
        );
        assert_eq!(
            failed(ModelCheckRefusal::AtomRefused(ClauseOutcome::NotBoolean)),
            BROKEN
        );
        let exhausted = CatalogCode::new("resource_exhausted", "insufficient-next-charge");
        assert_eq!(
            failed(ModelCheckRefusal::StateUnobservable(
                qsl_eval::simulation::ExpansionStop {
                    cause: exhausted,
                    limit: None
                }
            )),
            exhausted
        );
        assert_eq!(
            failed(ModelCheckRefusal::StateUnobservable(
                qsl_eval::simulation::ExpansionStop {
                    cause: BROKEN,
                    limit: None
                }
            )),
            BROKEN
        );
    }

    /// FR-281's table: only a refusal whose own code is `unsupported_*` is
    /// `unsupported`; liveness and hyper request or engine defects are
    /// `failed`.
    #[trace("TC-763", "FR-281-AC-11")]
    #[test]
    fn model_refusals_are_unsupported_only_for_unsupported_codes() {
        let unsupported = |refusal: ModelCheckRefusal| match model_refusal(&refusal) {
            Refusal::Unsupported(code) => Some(code.to_string()),
            _ => None,
        };
        let failed = |refusal: ModelCheckRefusal| match model_refusal(&refusal) {
            Refusal::Failed(code) => Some(code.to_string()),
            _ => None,
        };
        let not_yet = Some("unsupported_construct/not-yet-implemented".to_owned());
        assert_eq!(unsupported(ModelCheckRefusal::ReducedLiveness), not_yet);
        assert_eq!(
            unsupported(ModelCheckRefusal::Hyper(HyperRefusal::FormNotDecided {
                form: None
            })),
            not_yet
        );
        assert_eq!(
            unsupported(ModelCheckRefusal::Hyper(
                HyperRefusal::NotSingleExistential { form: None }
            )),
            not_yet
        );
        assert_eq!(
            failed(ModelCheckRefusal::Liveness(
                LivenessRefusal::UnknownOperation {
                    name: "op".to_owned()
                }
            )),
            Some("invalid_runtime_input/invalid-value".to_owned())
        );
        assert_eq!(
            failed(ModelCheckRefusal::Hyper(HyperRefusal::SubjectCount {
                expected: 2,
                found: 1
            })),
            Some("invalid_runtime_input/invalid-value".to_owned())
        );
        assert_eq!(
            failed(ModelCheckRefusal::Hyper(HyperRefusal::UnknownOperation {
                name: "op".to_owned()
            })),
            Some("invalid_runtime_input/invalid-value".to_owned())
        );
        assert_eq!(
            failed(ModelCheckRefusal::Hyper(HyperRefusal::NoPartnerPath)),
            Some("runtime_invariant/established-invariant-broken".to_owned())
        );
        assert_eq!(
            failed(ModelCheckRefusal::UnknownConstraint {
                name: "c".to_owned()
            }),
            Some("invalid_runtime_input/invalid-value".to_owned())
        );
        assert!(matches!(
            model_refusal(&ModelCheckRefusal::Admission(Box::new(
                AdmissionFailure::Refused(Box::new(record(
                    Code::InvalidRuntimeInput,
                    "wrong-role-mapping",
                )))
            ))),
            Refusal::Declined(..)
        ));
    }

    /// FR-281's table for EN-4: a refused predicate is not intercepted, so
    /// `analyze` settles it as FR-192-AC-7's rows do (a family refusal by its
    /// own code, an exhausted meter `incomplete`/`limit-reached`, anything else
    /// broken), equal to the provider entry's record (FR-280); `NotStatistical`
    /// is a disposition; the rest settle through FR-192's map.
    #[trace("TC-763", "FR-281-AC-11")]
    #[test]
    fn statistical_refusals_follow_the_table() {
        use quire_exact::{ChargePoint, Incomplete, Integer, LimitKind};

        let settled = |refusal: StatisticalRefusal| match statistical_refusal(&refusal) {
            EngineReport::Statistical(record) => {
                assert_eq!(
                    *record,
                    settle_statistical_refusal(&refusal).expect("a terminal record")
                );
                *record
            }
            _ => panic!("expected the map's record"),
        };
        let outside = CatalogCode::new("integer_out_of_domain", "outside-domain");
        let family = settled(StatisticalRefusal::PredicateRefused(
            ClauseOutcome::FamilyRefused(outside),
        ));
        assert_eq!(family.value, TerminalValue::Failed);
        assert_eq!(family.diagnostic, Some(outside));

        let exhausted = settled(StatisticalRefusal::PredicateRefused(
            ClauseOutcome::Incomplete(Incomplete {
                limit_kind: LimitKind::WorkUnits,
                limit: 9,
                consumed: 9,
                next_charge: Integer::from(1_i64),
                charge_point: ChargePoint::TextInputBytes,
            }),
        ));
        assert!(matches!(
            exhausted.value,
            TerminalValue::Incomplete(IncompleteCause::LimitReached { value: 9, .. })
        ));

        let broken = settled(StatisticalRefusal::PredicateRefused(
            ClauseOutcome::NotBoolean,
        ));
        assert_eq!(broken.value, TerminalValue::Failed);
        assert_eq!(broken.diagnostic, Some(BROKEN));

        let class = |refusal: StatisticalRefusal| match statistical_refusal(&refusal) {
            EngineReport::Refused(refusal) => code(&refusal),
            _ => None,
        };
        assert_eq!(
            class(StatisticalRefusal::Admission(Box::new(
                AdmissionFailure::Refused(Box::new(record(
                    Code::InvalidRuntimeInput,
                    "wrong-role-mapping",
                )))
            ))),
            Some("invalid_runtime_input/wrong-role-mapping".to_owned())
        );
    }
}
