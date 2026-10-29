// SPDX-License-Identifier: AGPL-3.0-or-later
//! The `analyze` operation (FR-281, ADR-029 OP-2): decide claims over every
//! behaviour of a model with the layer-A engines, route each item to an
//! engine by the engines' published manifests, and settle each result as one
//! FR-331 terminal record.
//!
//! [`analyze`] owns the operation, not the engines: each [`Engine`] is its
//! manifest ([`crate::model_check::manifest`], [`crate::statistical::manifest`])
//! and each item is the work an engine runs ([`AnalyzeItem`]). Routing reads
//! only the manifests, through FR-288's descriptor and `qsl-route`'s
//! registry; an item whose claim kind no engine advertises settles
//! `unsupported`, naming the kind.
//!
//! Settlement is the engine's own layer-6 map, never `analyze`'s opinion of
//! an engine's evidence: EN-1's results settle through `settle_model_check`
//! (its certificate checker and counterexample replay), EN-4's through
//! `check_statistical` and FR-192's map, the two calls the FR-280 provider
//! entry makes, so `analyze` and the entry write one record for one item. The
//! item adapters ([`model_item`], [`statistical_item`]) are the only
//! constructors of an engine's work, so no caller can hand `analyze` a
//! pre-built verdict, a checker or a replay outcome.
//!
//! `analyze` does not negotiate and reads no manifest to route: the caller
//! settles each item's [`Disposition`] (CG's `negotiate_*`) and `analyze` runs
//! a `Supported` item on that one candidate and settles any other disposition
//! as it is, without running an engine. Every item has a record, so no
//! refusal ends the request: an engine refusal that its settlement map gives
//! no terminal record is settled by the FR-281 "Engine refusals" table.
//!
//! A cancelled [`Cancel`] settles every item that would run `incomplete`,
//! with the cancellation cause kept.

mod items;

use qsl_eval::simulation::protocol::ProtocolTransition;
use qsl_foundation::{BackendId, RequestIndex};
use qsl_replay::statistical::StatisticalRecord;
use qsl_replay::{CounterexampleKind, TemporalCounterexample};
use qsl_replay::{
    DeclineCode, IncompleteCause, ModelCheckRecord, OutcomeItem, ProofRefusalCause, TerminalRecord,
    TerminalValue, UndefinedEvaluation,
};
use qsl_route::routing::Disposition;
use qsl_route::{BackendDescriptor, Candidate, Mode, ProviderOrigin, RegistrationRefusal};
use qsl_semantics::check::Capability;
use quire_exact::Cancel;
use quire_semantic_value::catalog::CatalogCode;

pub use crate::value_check::value_item;
pub use items::{
    derived_model_item, derived_protocol_item, derived_zone_item, hyper_form_item, model_item,
    protocol_item, statistical_item, timed_refinement_item, zone_item, DerivedItemMismatch,
    ModelItem, ProtocolItem, ProtocolReplayFn, RefinementReplayFn, TimedItem, TimedRefinementItem,
};

use crate::zone_check::ZoneCheckRecord;
use crate::{model_check, statistical, value_check, zone_check};

/// One layer-A engine as `analyze` runs it: its backend identity and the
/// descriptor FR-288 converts its manifest into.
#[derive(Clone, Debug)]
pub struct Engine {
    descriptor: BackendDescriptor,
}

impl Engine {
    /// The engine a descriptor describes: only the crate's own adapters name
    /// one.
    pub(crate) fn new(descriptor: BackendDescriptor) -> Self {
        Self { descriptor }
    }

    /// EN-1, the explicit-state model checker.
    ///
    /// # Errors
    ///
    /// Only if the registry's admission rules refuse EN-1's own manifest.
    pub fn model_check() -> Result<Self, RegistrationRefusal> {
        model_check::manifest::descriptor(ProviderOrigin::Linked).map(Self::new)
    }

    /// EN-6, the zone engine, which decides items over a `time dense`
    /// subject (FR-243).
    ///
    /// # Errors
    ///
    /// Only if the registry's admission rules refuse EN-6's own manifest.
    pub fn zone_check() -> Result<Self, RegistrationRefusal> {
        zone_check::manifest::descriptor(ProviderOrigin::Linked).map(Self::new)
    }

    /// EN-4, the statistical engine.
    ///
    /// # Errors
    ///
    /// Only if the registry's admission rules refuse EN-4's own manifest.
    pub fn statistical() -> Result<Self, RegistrationRefusal> {
        statistical::manifest::descriptor(ProviderOrigin::Linked).map(Self::new)
    }

    /// The value-validity check arm (QSpec FR-483, FR-484).
    ///
    /// # Errors
    ///
    /// Only if the registry's admission rules refuse the arm's own manifest.
    pub fn value_check() -> Result<Self, RegistrationRefusal> {
        value_check::manifest::descriptor(ProviderOrigin::Linked).map(Self::new)
    }

    /// The value-validity arm publishing `max_cases` as its default case
    /// budget (QSpec FR-483 "Descriptor modes, domains and registration").
    ///
    /// # Errors
    ///
    /// Only if the registry's admission rules refuse the arm's manifest.
    pub fn value_check_with_max_cases(
        max_cases: std::num::NonZeroU64,
    ) -> Result<Self, RegistrationRefusal> {
        value_check::manifest::descriptor_with(ProviderOrigin::Linked, max_cases).map(Self::new)
    }

    /// The default `max_cases` this engine's descriptor publishes.
    pub(crate) fn max_cases(&self) -> Option<std::num::NonZeroU64> {
        self.descriptor.max_cases()
    }

    /// The engine's backend identity.
    pub fn id(&self) -> &BackendId {
        self.descriptor.id()
    }

    /// The modes the engine's manifest advertises for `kind`.
    fn modes(&self, kind: Capability) -> Vec<Mode> {
        self.descriptor
            .advertises()
            .filter_map(|(advertised, mode)| (advertised == kind).then_some(mode))
            .collect()
    }
}

/// `Engines::with` was given an engine whose `BackendId` the set holds
/// (FR-281): `invalid_capability`/`duplicate-backend`.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("duplicate-backend: the set already holds {backend:?}")]
pub struct DuplicateEngine {
    /// The `BackendId` the set holds.
    pub backend: BackendId,
}

impl DuplicateEngine {
    /// The refusal's catalog code.
    pub const fn catalog_code(&self) -> CatalogCode {
        CatalogCode::new("invalid_capability", "duplicate-backend")
    }
}

/// The engines `analyze` may run, at most one per `BackendId`, so a
/// candidate names at most one engine (FR-281).
#[derive(Clone, Debug, Default)]
pub struct Engines {
    engines: Vec<Engine>,
}

impl Engines {
    /// The layer-A engines `analyze` has an item adapter for: EN-1, EN-4 and
    /// EN-6.
    ///
    /// # Errors
    ///
    /// Only if the registry's admission rules refuse an engine's own
    /// manifest.
    pub fn qsl() -> Result<Self, RegistrationRefusal> {
        Ok(Self {
            engines: vec![
                Engine::model_check()?,
                Engine::statistical()?,
                Engine::zone_check()?,
                Engine::value_check()?,
            ],
        })
    }

    /// The exact provider descriptors owned by these linked engines.
    ///
    /// The returned descriptors are borrowed from their engine instances;
    /// callers cannot substitute a capability-only registration.
    pub fn registrations(&self) -> impl ExactSizeIterator<Item = &BackendDescriptor> {
        self.engines.iter().map(|engine| &engine.descriptor)
    }

    /// Add `engine`.
    ///
    /// # Errors
    ///
    /// [`DuplicateEngine`] when the set holds an engine of that `BackendId`;
    /// the set is unchanged.
    pub fn with(&mut self, engine: Engine) -> Result<(), DuplicateEngine> {
        if self.get(engine.id()).is_some() {
            return Err(DuplicateEngine {
                backend: engine.id().clone(),
            });
        }
        self.engines.push(engine);
        Ok(())
    }

    fn get(&self, backend: &BackendId) -> Option<&Engine> {
        self.engines.iter().find(|engine| engine.id() == backend)
    }
}

/// What a diagnostic's FR-272 payload names.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Payload {
    /// The diagnostic carries no payload.
    None,
    /// The item's claim kind.
    Kind(Capability),
    /// A backend: the unknown candidate.
    Backend(BackendId),
    /// A `Supported` candidate whose manifest does not advertise the item's
    /// kind and mode.
    Inconsistent {
        /// The candidate.
        candidate: BackendId,
        /// The item's kind.
        kind: Capability,
        /// The item's mode.
        mode: Mode,
    },
    /// An unbounded extent (QSpec FR-272): the kind, the candidate and the
    /// modes its manifest advertises for that kind.
    Unbounded {
        /// The item's kind.
        kind: Capability,
        /// The engine that refused the unbounded extent.
        candidate: BackendId,
        /// The modes its manifest advertises for the kind.
        modes: Vec<Mode>,
    },
}

/// The FR-272 diagnostic of a record settled by a disposition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    /// The code and cause.
    pub code: CatalogCode,
    /// The payload.
    pub payload: Payload,
}

impl Diagnostic {
    /// A diagnostic with no payload.
    pub fn new(code: CatalogCode) -> Self {
        Self {
            code,
            payload: Payload::None,
        }
    }
}

/// A disposition other than `supported`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DispositionKind {
    /// `requires-bound`.
    RequiresBound,
    /// `unsupported`.
    Unsupported,
    /// `invalid-request`.
    InvalidRequest,
}

/// A record settled by a disposition (QSpec FR-331 `settled_by`
/// `disposition`): no result, the disposition and its diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Disposed {
    /// The disposition.
    pub kind: DispositionKind,
    /// The engine that refused the item, when one ran; none for an item that
    /// ran no engine.
    pub backend: Option<BackendId>,
    /// The disposition's diagnostic.
    pub diagnostic: Option<Diagnostic>,
}

/// What an engine reports for one item. Only the crate's item adapters
/// construct one, from the engine's own layer-6 settlement.
pub(crate) enum EngineReport {
    /// EN-1's record, settled by `settle_model_check`.
    Model(Box<ModelCheckRecord>),
    /// EN-1's record over a protocol subject, settled by
    /// `settle_protocol_check`.
    Protocol(Box<ModelCheckRecord<ProtocolTransition>>),
    /// EN-4's record, settled by FR-192's map.
    Statistical(Box<StatisticalRecord>),
    /// The value-validity arm's finding.
    Value(Box<value_check::ValueReport>),
    /// EN-6's record, settled by FR-235's map, or a timed refinement's by
    /// FR-144's rows over EN-6's causes.
    Zone(Box<ZoneCheckRecord>),
    /// The engine refused the item, and its settlement map gives the refusal
    /// no terminal record.
    Refused(Refusal),
}

/// The FR-281 "Engine refusals" table's rows.
pub(crate) enum Refusal {
    /// `declined`, with the refusal's cause, `DeclineCode` and full
    /// diagnostic.
    Declined(ProofRefusalCause, DeclineCode, CatalogCode),
    /// An unbounded extent: `unsupported` with `unbounded-extent`.
    RequiresBound,
    /// EN-4 `NotStatistical`: `invalid-request` with `not-statistical`.
    NotStatistical,
    /// `unsupported` with the refusal's own diagnostic.
    Unsupported(CatalogCode),
    /// `failed` with the refusal's diagnostic.
    Failed(CatalogCode),
}

/// The work an engine runs for one item.
type Work<'a> = Box<dyn FnOnce(&Cancel, &Engine) -> EngineReport + 'a>;

/// One item to analyze: its request index, its claim, its settled
/// disposition and the work an engine runs for it, when the claim kind has an
/// item adapter.
pub struct AnalyzeItem<'a> {
    index: RequestIndex,
    kind: Capability,
    mode: Mode,
    disposition: Disposition,
    diagnostic: Option<Diagnostic>,
    work: Option<Work<'a>>,
}

impl<'a> AnalyzeItem<'a> {
    /// An item of claim kind `kind` and mode `mode` that `backend` runs as
    /// `work`: `Supported(backend)` unless [`Self::disposed`] says otherwise.
    pub(crate) fn new(
        index: RequestIndex,
        kind: Capability,
        mode: Mode,
        backend: &str,
        work: impl FnOnce(&Cancel) -> EngineReport + 'a,
    ) -> Self {
        Self {
            index,
            kind,
            mode,
            disposition: Disposition::Supported(Candidate::new(BackendId::new(backend))),
            diagnostic: None,
            work: Some(Box::new(move |cancel, _| work(cancel))),
        }
    }

    /// As [`Self::new`], for work that reads the engine's descriptor: the
    /// value-validity arm takes its `max_cases` default from it.
    pub(crate) fn new_with_engine(
        index: RequestIndex,
        kind: Capability,
        mode: Mode,
        backend: &str,
        work: impl FnOnce(&Cancel, &Engine) -> EngineReport + 'a,
    ) -> Self {
        Self {
            index,
            kind,
            mode,
            disposition: Disposition::Supported(Candidate::new(BackendId::new(backend))),
            diagnostic: None,
            work: Some(Box::new(work)),
        }
    }

    /// An item of claim kind `kind` that no item adapter builds: no engine
    /// is given work for it, so it settles `unsupported` with
    /// `unsupported-requested-capability`, naming the kind.
    pub fn unadapted(index: RequestIndex, kind: Capability) -> Self {
        Self {
            index,
            kind,
            mode: Mode::Bounded,
            disposition: Disposition::Unsupported,
            diagnostic: Some(unsupported_kind(kind)),
            work: None,
        }
    }

    /// This item with the disposition the caller's negotiation settled it
    /// (FR-281 "Negotiated items") and, for `Unsupported` and
    /// `InvalidRequest`, its diagnostic.
    #[must_use]
    pub fn disposed(mut self, disposition: Disposition, diagnostic: Option<Diagnostic>) -> Self {
        self.disposition = disposition;
        self.diagnostic = diagnostic;
        self
    }
}

fn unsupported_kind(kind: Capability) -> Diagnostic {
    Diagnostic {
        code: CatalogCode::new("unsupported_projection", "unsupported-requested-capability"),
        payload: Payload::Kind(kind),
    }
}

/// One item's settlement.
#[derive(Debug)]
pub struct AnalyzeRecord {
    /// The item's request index.
    pub request_index: RequestIndex,
    /// The item's settlement.
    pub settlement: Settlement,
    /// EN-1's full record, carrying the counterexample of a `refuted` item.
    pub model_check: Option<Box<ModelCheckRecord>>,
    /// EN-1's full record over a protocol subject, carrying the
    /// counterexample of a `refuted` item.
    pub protocol_check: Option<Box<ModelCheckRecord<ProtocolTransition>>>,
    /// EN-6's full record, carrying the counterexample of a `refuted` item.
    pub zone_check: Option<Box<ZoneCheckRecord>>,
    /// The value-validity arm's counterexample, carried by a `refuted` item.
    pub value_check: Option<Box<value_check::ValueCounterexample>>,
}

/// How an item settled.
#[derive(Debug)]
pub enum Settlement {
    /// An FR-331 terminal record.
    Terminal(Box<TerminalRecord>),
    /// A record settled by a disposition.
    Disposed(Disposed),
}

impl AnalyzeRecord {
    /// The terminal record, when the item settled one.
    pub fn terminal(&self) -> Option<&TerminalRecord> {
        match &self.settlement {
            Settlement::Terminal(terminal) => Some(terminal.as_ref()),
            Settlement::Disposed(_) => None,
        }
    }

    /// The counterexample a `refuted` item carries, from whichever engine
    /// settled it.
    pub fn counterexample(&self) -> Option<&TemporalCounterexample> {
        self.model_check
            .as_ref()
            .and_then(|record| record.counterexample.as_ref())
            .or_else(|| {
                self.zone_check
                    .as_ref()
                    .and_then(|record| record.counterexample.as_ref())
            })
    }

    /// The counterexample a `refuted` item over a protocol subject carries.
    pub fn protocol_counterexample(&self) -> Option<&TemporalCounterexample<ProtocolTransition>> {
        self.protocol_check
            .as_ref()
            .and_then(|record| record.counterexample.as_ref())
    }

    /// The outcome item (FR-286) this record writes. An item refuted by a
    /// claim evaluation undefined at a state is `refuted`, category
    /// violation, with cause `undefined-evaluation` naming that state and
    /// the undefined reason, and never the `undefined` label (FR-281).
    /// `None` for a record settled by a disposition, which the driver writes
    /// from its own disposition and diagnostic.
    pub fn outcome_item(&self) -> Option<OutcomeItem> {
        let terminal = self.terminal()?;
        if let (Some(value), TerminalValue::Refuted) = (&self.value_check, terminal.value()) {
            if let Some(undefined) = &value.reproduced.violation().undefined {
                return Some(OutcomeItem::undefined_evaluation(
                    self.request_index,
                    format!(
                        "node {}, occurrence {}/{}",
                        undefined.locus.node(),
                        undefined.locus.origin().role().as_str(),
                        undefined.locus.origin().ordinal()
                    ),
                    undefined.reason,
                ));
            }
        }
        let kind = self
            .counterexample()
            .map(|counterexample| &counterexample.kind)
            .or_else(|| {
                self.protocol_counterexample()
                    .map(|counterexample| &counterexample.kind)
            });
        let undefined = kind.and_then(|kind| match kind {
            CounterexampleKind::UndefinedEvaluation(evaluation) => Some(evaluation),
            CounterexampleKind::Formula
            | CounterexampleKind::Deadlock
            | CounterexampleKind::Race(_)
            | CounterexampleKind::TimeLock => None,
        });
        Some(match undefined {
            Some(evaluation) if matches!(terminal.value(), TerminalValue::Refuted) => {
                undefined_item(self.request_index, evaluation)
            }
            Some(_) | None => OutcomeItem::from_terminal(terminal),
        })
    }
}

fn undefined_item(index: RequestIndex, evaluation: &UndefinedEvaluation) -> OutcomeItem {
    OutcomeItem::undefined_evaluation(
        index,
        format!(
            "position {}, state {}",
            evaluation.site.position,
            evaluation.site.state_key.hex()
        ),
        qsl_replay::undefined_reason(&evaluation.cause),
    )
}

/// FR-281: run `items` on `engines` and settle each as one record, in
/// request order. Every item has a record: an item settled by a disposition
/// other than `supported` runs no engine, and an item whose `Supported`
/// candidate `engines` does not hold, or holds without advertising the
/// item's kind and mode, settles `invalid-request`.
pub fn analyze(
    engines: &Engines,
    items: Vec<AnalyzeItem<'_>>,
    cancel: &Cancel,
) -> Vec<AnalyzeRecord> {
    items
        .into_iter()
        .map(|item| run(engines, item, cancel))
        .collect()
}

fn run(engines: &Engines, item: AnalyzeItem<'_>, cancel: &Cancel) -> AnalyzeRecord {
    let AnalyzeItem {
        index,
        kind,
        mode,
        disposition,
        diagnostic,
        work,
    } = item;
    let record = |settlement| AnalyzeRecord {
        request_index: index,
        settlement,
        model_check: None,
        protocol_check: None,
        zone_check: None,
        value_check: None,
    };
    let disposed = |kind, diagnostic| {
        record(Settlement::Disposed(Disposed {
            kind,
            backend: None,
            diagnostic,
        }))
    };
    let candidate = match disposition {
        Disposition::Supported(candidate) => candidate,
        Disposition::RequiresBound => return disposed(DispositionKind::RequiresBound, diagnostic),
        Disposition::Unsupported => return disposed(DispositionKind::Unsupported, diagnostic),
        Disposition::InvalidRequest => {
            return disposed(DispositionKind::InvalidRequest, diagnostic)
        }
    };
    let backend = candidate.id().clone();
    let Some(engine) = engines.get(&backend) else {
        return disposed(
            DispositionKind::InvalidRequest,
            Some(Diagnostic {
                code: CatalogCode::new("invalid_capability", "unknown-backend"),
                payload: Payload::Backend(backend),
            }),
        );
    };
    if !engine.modes(kind).contains(&mode) {
        return disposed(
            DispositionKind::InvalidRequest,
            Some(Diagnostic {
                code: CatalogCode::new("invalid_capability", "inconsistent-candidates"),
                payload: Payload::Inconsistent {
                    candidate: backend,
                    kind,
                    mode,
                },
            }),
        );
    }
    let Some(work) = work else {
        return disposed(DispositionKind::Unsupported, Some(unsupported_kind(kind)));
    };
    if let Some(source) = cancel.cause() {
        return record(terminal(TerminalRecord::incomplete(
            index,
            backend,
            IncompleteCause::Cancelled { source },
        )));
    }
    settle(index, kind, engine, work(cancel, engine))
}

fn terminal(record: TerminalRecord) -> Settlement {
    Settlement::Terminal(Box::new(record))
}

/// The record names `engine`, the one `Engines` holds for the item's
/// candidate. An item's adapter and that candidate agree by construction
/// (`AnalyzeItem::new` takes the adapter's own `BackendId`); a future
/// `Engine` trait must report the backend from the engine that ran, not from
/// the item.
fn settle(
    index: RequestIndex,
    kind: Capability,
    engine: &Engine,
    report: EngineReport,
) -> AnalyzeRecord {
    let backend = engine.id().clone();
    let record = |settlement, model_check| AnalyzeRecord {
        request_index: index,
        settlement,
        model_check,
        protocol_check: None,
        zone_check: None,
        value_check: None,
    };
    let disposed = |kind, diagnostic: Diagnostic, backend| {
        record(
            Settlement::Disposed(Disposed {
                kind,
                backend: Some(backend),
                diagnostic: Some(diagnostic),
            }),
            None,
        )
    };
    match report {
        EngineReport::Model(model) => record(
            // A refuted value without its reproduced counterexample is an
            // engine fault, never a verdict.
            terminal(
                model
                    .to_terminal(index, backend.clone())
                    .unwrap_or_else(|_| TerminalRecord::failed(index, backend)),
            ),
            Some(model),
        ),
        EngineReport::Value(report) => {
            let (settled_record, counterexample) = value_check::settle(*report, index, backend);
            let mut settled = record(terminal(settled_record), None);
            settled.value_check = counterexample.map(Box::new);
            settled
        }
        EngineReport::Protocol(protocol) => AnalyzeRecord {
            request_index: index,
            // A refuted value without its reproduced counterexample is an
            // engine fault, never a verdict.
            settlement: terminal(
                protocol
                    .to_terminal(index, backend.clone())
                    .unwrap_or_else(|_| TerminalRecord::failed(index, backend)),
            ),
            model_check: None,
            protocol_check: Some(protocol),
            zone_check: None,
            value_check: None,
        },
        EngineReport::Statistical(statistical) => {
            record(terminal(statistical.into_terminal(index, backend)), None)
        }
        EngineReport::Zone(zone) => AnalyzeRecord {
            request_index: index,
            // A refuted value without its reproduced counterexample is an
            // engine fault, never a verdict.
            settlement: terminal(
                TerminalRecord::of_model_settlement(
                    index,
                    backend.clone(),
                    zone.value.clone(),
                    zone.counterexample.as_ref(),
                )
                .unwrap_or_else(|_| TerminalRecord::failed(index, backend)),
            ),
            model_check: None,
            protocol_check: None,
            zone_check: Some(zone),
            value_check: None,
        },
        EngineReport::Refused(Refusal::Declined(cause, code, diagnostic)) => record(
            terminal(TerminalRecord::declined_with(
                index, backend, cause, code, diagnostic,
            )),
            None,
        ),
        EngineReport::Refused(Refusal::Failed(code)) => record(
            terminal(TerminalRecord::failed_with(index, backend, code)),
            None,
        ),
        EngineReport::Refused(Refusal::Unsupported(code)) => {
            disposed(DispositionKind::Unsupported, Diagnostic::new(code), backend)
        }
        EngineReport::Refused(Refusal::NotStatistical) => disposed(
            DispositionKind::InvalidRequest,
            Diagnostic::new(CatalogCode::new("invalid_capability", "not-statistical")),
            backend,
        ),
        EngineReport::Refused(Refusal::RequiresBound) => {
            let diagnostic = Diagnostic {
                code: CatalogCode::new("unsupported_projection", "unbounded-extent"),
                payload: Payload::Unbounded {
                    kind,
                    candidate: backend.clone(),
                    modes: engine.modes(kind),
                },
            };
            disposed(DispositionKind::Unsupported, diagnostic, backend)
        }
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use qsl_foundation::bound::DomainKind;
    use qsl_route::descriptor_from_manifest;

    use super::*;

    fn engine(id: &str, kind: Capability) -> Engine {
        Engine::new(
            descriptor_from_manifest(
                Candidate::new(BackendId::new(id)),
                ProviderOrigin::Linked,
                [(Some(kind.to_wire()), Some(Mode::Bounded.to_wire()))],
                qsl_route::ManifestMembers {
                    domains: Some(&[DomainKind::Population.to_wire()]),
                    fairness_kinds: Some(qsl_route::FairnessKindsMember::List(&[])),
                    evidence_kinds: None,
                    memory_models: None,
                    reductions: None,
                },
            )
            .expect("a well-formed test engine"),
        )
    }

    fn item<'a>(
        backend: &str,
        kind: Capability,
        ran: &'a std::cell::Cell<bool>,
    ) -> AnalyzeItem<'a> {
        AnalyzeItem::new(
            RequestIndex::new(0),
            kind,
            Mode::Bounded,
            backend,
            move |_| {
                ran.set(true);
                EngineReport::Refused(Refusal::Failed(CatalogCode::new(
                    "runtime_invariant",
                    "established-invariant-broken",
                )))
            },
        )
    }

    fn disposed(records: &[AnalyzeRecord]) -> &Disposed {
        match &records[0].settlement {
            Settlement::Disposed(disposed) => disposed,
            Settlement::Terminal(terminal) => panic!("expected a disposition: {terminal:?}"),
        }
    }

    /// FR-489-AC-5: linked registrations are borrowed from the EN-1, EN-4 and
    /// EN-6 engines and retain each engine's own identity and advertised kind.
    #[trace("TC-1272", "FR-489-AC-5")]
    #[test]
    fn linked_engine_registrations_expose_the_owned_descriptors() {
        let engines = Engines::qsl().expect("QSL's linked engine descriptors are admitted");
        let registrations: Vec<_> = engines.registrations().collect();

        assert_eq!(
            registrations
                .iter()
                .map(|descriptor| descriptor.id().as_str())
                .collect::<Vec<_>>(),
            ["en-1", "en-4", "en-6", "value-checker"]
        );
        let [en1, en4, _en6, value] = registrations.as_slice() else {
            panic!("the linked engine set contains EN-1, EN-4, EN-6 and the value arm");
        };
        assert!(registrations
            .iter()
            .all(|descriptor| descriptor.origin() == ProviderOrigin::Linked));
        assert!(en1
            .advertises()
            .any(|(kind, _)| kind == Capability::TemporalSatisfaction));
        assert!(en4
            .advertises()
            .any(|(kind, _)| kind == Capability::ProbabilisticSatisfaction));
        assert!(value
            .advertises()
            .any(|(kind, _)| kind == Capability::ValueValidity));
    }

    /// FR-281-AC-10: a second engine of one `BackendId` is refused as
    /// `duplicate-backend` and the set still runs the first.
    #[trace("TC-762", "FR-281-AC-10")]
    #[test]
    fn a_duplicate_backend_is_refused_and_the_set_is_unchanged() {
        let mut engines = Engines::default();
        engines
            .with(engine("test-a", Capability::OperationContract))
            .expect("first");
        let refusal = engines
            .with(engine("test-a", Capability::ValueValidity))
            .expect_err("a duplicate");
        assert_eq!(refusal.backend, BackendId::new("test-a"));
        assert_eq!(refusal.catalog_code().cause(), "duplicate-backend");
        let ran = std::cell::Cell::new(false);
        let records = analyze(
            &engines,
            vec![item("test-a", Capability::OperationContract, &ran)],
            &Cancel::new(),
        );
        assert!(ran.get());
        assert!(records[0].terminal().is_some());
    }

    /// FR-281-AC-10: a `Supported` candidate that does not advertise the
    /// item's kind and mode settles `inconsistent-candidates` and does not
    /// run; one the set does not hold settles `unknown-backend`.
    #[trace("TC-762", "FR-281-AC-10")]
    #[test]
    fn an_inconsistent_or_unknown_candidate_settles_invalid_request_and_runs_nothing() {
        let mut engines = Engines::default();
        engines
            .with(engine("test-a", Capability::ValueValidity))
            .expect("first");
        let ran = std::cell::Cell::new(false);
        let records = analyze(
            &engines,
            vec![item("test-a", Capability::TemporalSatisfaction, &ran)],
            &Cancel::new(),
        );
        let settled = disposed(&records);
        assert_eq!(settled.kind, DispositionKind::InvalidRequest);
        let diagnostic = settled.diagnostic.as_ref().expect("a diagnostic");
        assert_eq!(diagnostic.code.cause(), "inconsistent-candidates");
        assert_eq!(
            diagnostic.payload,
            Payload::Inconsistent {
                candidate: BackendId::new("test-a"),
                kind: Capability::TemporalSatisfaction,
                mode: Mode::Bounded,
            }
        );
        let records = analyze(
            &engines,
            vec![item("test-c", Capability::ValueValidity, &ran)],
            &Cancel::new(),
        );
        let diagnostic = disposed(&records).diagnostic.clone().expect("a diagnostic");
        assert_eq!(diagnostic.code.cause(), "unknown-backend");
        assert_eq!(
            diagnostic.payload,
            Payload::Backend(BackendId::new("test-c"))
        );
        assert!(!ran.get());
    }

    /// FR-281-AC-8: an item settled `InvalidRequest` (an ambiguous route, for
    /// one) keeps its caller-given diagnostic and runs no engine; its
    /// neighbours still settle.
    #[trace("TC-762", "FR-281-AC-8")]
    #[test]
    fn a_disposition_settles_its_item_alone_and_runs_nothing() {
        let mut engines = Engines::default();
        engines
            .with(engine("test-a", Capability::ValueValidity))
            .expect("first");
        let ran = std::cell::Cell::new(false);
        let ambiguous =
            Diagnostic::new(CatalogCode::new("invalid_capability", "ambiguous-backend"));
        let records = analyze(
            &engines,
            vec![
                item("test-a", Capability::ValueValidity, &ran)
                    .disposed(Disposition::InvalidRequest, Some(ambiguous.clone())),
                item("test-a", Capability::ValueValidity, &ran),
            ],
            &Cancel::new(),
        );
        let settled = disposed(&records);
        assert_eq!(settled.kind, DispositionKind::InvalidRequest);
        assert_eq!(settled.diagnostic.as_ref(), Some(&ambiguous));
        assert!(records[1].terminal().is_some());
    }

    fn refusing<'a>(refusal: impl Fn() -> Refusal + 'a) -> AnalyzeItem<'a> {
        AnalyzeItem::new(
            RequestIndex::new(0),
            Capability::OperationContract,
            Mode::Bounded,
            "test-a",
            move |_| EngineReport::Refused(refusal()),
        )
    }

    /// FR-281-AC-11: each engine refusal row settles its record naming the
    /// engine; an unbounded extent carries QSpec FR-272's payload (kind,
    /// candidate, advertised modes).
    #[trace("TC-763", "FR-281-AC-11")]
    #[test]
    fn engine_refusals_settle_by_the_refusal_table() {
        let mut engines = Engines::default();
        engines
            .with(engine("test-a", Capability::OperationContract))
            .expect("first");
        let run =
            |refusal: fn() -> Refusal| analyze(&engines, vec![refusing(refusal)], &Cancel::new());

        let records = run(|| Refusal::RequiresBound);
        let settled = disposed(&records);
        assert_eq!(settled.kind, DispositionKind::Unsupported);
        assert_eq!(settled.backend, Some(BackendId::new("test-a")));
        let diagnostic = settled.diagnostic.as_ref().expect("a diagnostic");
        assert_eq!(
            diagnostic.code.to_string(),
            "unsupported_projection/unbounded-extent"
        );
        assert_eq!(
            diagnostic.payload,
            Payload::Unbounded {
                kind: Capability::OperationContract,
                candidate: BackendId::new("test-a"),
                modes: vec![Mode::Bounded],
            }
        );

        let records = run(|| Refusal::NotStatistical);
        assert_eq!(disposed(&records).kind, DispositionKind::InvalidRequest);
        assert_eq!(
            disposed(&records)
                .diagnostic
                .as_ref()
                .map(|d| d.code.to_string()),
            Some("invalid_capability/not-statistical".to_owned())
        );

        let records = run(|| {
            Refusal::Unsupported(CatalogCode::new(
                "unsupported_construct",
                "not-yet-implemented",
            ))
        });
        assert_eq!(disposed(&records).kind, DispositionKind::Unsupported);

        let records = run(|| {
            Refusal::Declined(
                ProofRefusalCause::IncompleteInput,
                DeclineCode::Qsl(qsl_foundation::Code::UnavailableObservation),
                CatalogCode::new("unavailable_observation", "missing-required-artifact"),
            )
        });
        assert!(matches!(
            records[0].terminal().map(TerminalRecord::value),
            Some(TerminalValue::Declined {
                cause: ProofRefusalCause::IncompleteInput,
                ..
            })
        ));

        let wrong = CatalogCode::new("invalid_runtime_input", "wrong-value-kind");
        let records = analyze(
            &engines,
            vec![refusing(move || Refusal::Failed(wrong))],
            &Cancel::new(),
        );
        let terminal = records[0].terminal().expect("a terminal record");
        assert_eq!(terminal.value(), &TerminalValue::Failed);
        assert_eq!(terminal.diagnostic(), Some(wrong));
        assert_eq!(terminal.backend(), Some(&BackendId::new("test-a")));
    }
}
