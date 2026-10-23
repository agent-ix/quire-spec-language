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
//! [`crate::statistical::provider::serve_request`], the same call the FR-280
//! provider entry makes, so `analyze` and the entry write one record for one
//! item. The item adapters ([`model_item`], [`statistical_item`]) are the
//! only constructors of an engine's work, so no caller can hand `analyze` a
//! pre-built verdict, a checker of its own choosing or a replay outcome. An
//! engine with no adapter has no way to report a proof or a counterexample
//! until its checker and replay exist in the core (EN-5: QSL-574..576).
//!
//! A cancelled [`Cancel`] settles every item not yet run `incomplete`, with
//! the cancellation cause kept. An item an engine refuses with no FR-331
//! record (EN-1's refusals, FR-192's unsettled refusals) is reported as
//! [`Unsettled`], never given a made-up verdict.

mod items;

use qsl_foundation::{BackendId, RequestIndex};
use qsl_replay::statistical::StatisticalRecord;
use qsl_replay::statistical::UnsettledRefusal;
use qsl_replay::CounterexampleKind;
use qsl_replay::{
    IncompleteCause, ModelCheckRecord, OutcomeItem, TerminalRecord, TerminalValue,
    UnavailabilityCause, UndefinedEvaluation,
};
use qsl_route::{
    BackendDescriptor, CandidateOutcome, ItemEvidence, ManifestDigest, ProviderOrigin,
    RegistrationRefusal, Registry,
};
use qsl_semantics::check::Capability;
use quire_exact::Cancel;

pub use items::{model_item, statistical_item, ModelItem};

use crate::{model_check, statistical};

/// One layer-A engine as `analyze` routes to it: its backend identity and
/// the descriptor FR-288 converts its manifest into.
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
        model_check::manifest::descriptor(ProviderOrigin::Linked, linked_digest()).map(Self::new)
    }

    /// EN-4, the statistical engine.
    ///
    /// # Errors
    ///
    /// Only if the registry's admission rules refuse EN-4's own manifest.
    pub fn statistical() -> Result<Self, RegistrationRefusal> {
        statistical::manifest::descriptor(ProviderOrigin::Linked, linked_digest()).map(Self::new)
    }

    /// The engine's backend identity.
    pub fn id(&self) -> &BackendId {
        self.descriptor.id()
    }
}

/// A linked engine publishes its manifest as data of its own module and has
/// no separate manifest bytes to digest; the registry needs a digest only to
/// tell two registrations of one identity apart.
fn linked_digest() -> ManifestDigest {
    ManifestDigest::from_digest([0; 32])
}

/// The layer-A engines `analyze` has an item adapter for: EN-1 and EN-4.
///
/// # Errors
///
/// Only if the registry's admission rules refuse an engine's own manifest.
pub fn layer_a_engines() -> Result<Vec<Engine>, RegistrationRefusal> {
    Ok(vec![Engine::model_check()?, Engine::statistical()?])
}

/// What an engine reports for one item. Only the crate's item adapters
/// construct one, from the engine's own layer-6 settlement.
pub(crate) enum EngineReport {
    /// EN-1's record, settled by `settle_model_check`.
    Model(Box<ModelCheckRecord>),
    /// EN-4's record, settled by FR-192's map.
    Statistical(Box<StatisticalRecord>),
    /// The engine refused the item, and no layer-6 map gives the refusal a
    /// terminal record.
    Refused(Unsettled),
}

/// A refusal that has no FR-331 terminal record.
#[derive(Debug)]
pub enum Unsettled {
    /// EN-1 explored nothing.
    Model(Box<model_check::ModelCheckRefusal>),
    /// A refusal FR-192's map gives no terminal record.
    Statistical(UnsettledRefusal),
}

/// The work an engine runs for one item.
type Work<'a> = Box<dyn FnOnce(&Cancel) -> EngineReport + 'a>;

/// One item to analyze: its request index, its claim and the work an engine
/// runs for it, when the claim kind has an item adapter.
pub struct AnalyzeItem<'a> {
    index: RequestIndex,
    kind: Capability,
    evidence: ItemEvidence,
    work: Option<Work<'a>>,
}

impl<'a> AnalyzeItem<'a> {
    /// An item of claim kind `kind` and probabilistic evidence `evidence`
    /// (ignored for a kind that is not probabilistic) whose engine runs
    /// `work`.
    pub(crate) fn new(
        index: RequestIndex,
        kind: Capability,
        evidence: ItemEvidence,
        work: impl FnOnce(&Cancel) -> EngineReport + 'a,
    ) -> Self {
        Self {
            index,
            kind,
            evidence,
            work: Some(Box::new(work)),
        }
    }

    /// An item of claim kind `kind` that no item adapter builds: it settles
    /// `unsupported`, naming the kind, even when an engine advertises it,
    /// since there is no work to hand the engine.
    pub fn unadapted(index: RequestIndex, kind: Capability) -> Self {
        Self {
            index,
            kind,
            evidence: ItemEvidence::Absent,
            work: None,
        }
    }
}

/// One item's settlement.
#[derive(Debug)]
pub struct AnalyzeRecord {
    /// The item's request index.
    pub request_index: RequestIndex,
    /// The item's FR-331 terminal record, or the refusal that has none.
    pub result: Result<TerminalRecord, Unsettled>,
    /// EN-1's full record, carrying the counterexample of a `refuted` item.
    pub model_check: Option<Box<ModelCheckRecord>>,
}

impl AnalyzeRecord {
    /// The outcome item (FR-286) this record writes. An item refuted by a
    /// claim evaluation undefined at a state is `refuted`, category
    /// violation, with cause `undefined-evaluation` naming that state and
    /// the undefined reason, and never the `undefined` label (FR-281).
    /// `None` for an item with no terminal record.
    pub fn outcome_item(&self) -> Option<OutcomeItem> {
        let terminal = self.result.as_ref().ok()?;
        let undefined = self
            .model_check
            .as_ref()
            .and_then(|record| record.counterexample.as_ref())
            .and_then(|counterexample| match &counterexample.kind {
                CounterexampleKind::UndefinedEvaluation(evaluation) => Some(evaluation),
                CounterexampleKind::Formula | CounterexampleKind::Deadlock => None,
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

/// Why `analyze` ran no item.
#[derive(Debug, thiserror::Error)]
pub enum AnalyzeRefusal {
    /// The engines' manifests do not register as one registry.
    #[error("the engines' manifests do not register: {0:?}")]
    Registration(Vec<RegistrationRefusal>),
    /// The manifests give an item more than one candidate. FR-290 settles
    /// an item `supported` only with exactly one, and routing never chooses.
    #[error("item {index:?} has more than one candidate engine: {candidates:?}")]
    Ambiguous {
        /// The item.
        index: RequestIndex,
        /// Its candidate engines, in FR-290's order.
        candidates: Vec<BackendId>,
    },
}

/// FR-281: run `items` on the engines `engines` route them to and settle each
/// as one record, in request order.
///
/// # Errors
///
/// [`AnalyzeRefusal`] when the engines' manifests do not register as one
/// registry, or route an item to more than one engine; no item has run then.
pub fn analyze(
    engines: &[Engine],
    items: Vec<AnalyzeItem<'_>>,
    cancel: &Cancel,
) -> Result<Vec<AnalyzeRecord>, AnalyzeRefusal> {
    let mut registry = Registry::new();
    let mut refusals = Vec::new();
    for engine in engines {
        if let Err(refused) = registry.register(engine.descriptor.clone()) {
            refusals.extend(refused);
        }
    }
    if !refusals.is_empty() {
        return Err(AnalyzeRefusal::Registration(refusals));
    }
    let mut routed = Vec::with_capacity(items.len());
    for item in items {
        let candidates = match registry.candidates_for_evidence(item.kind, &item.evidence, None) {
            CandidateOutcome::Candidates(set) => set
                .candidates()
                .iter()
                .map(|candidate| candidate.id().clone())
                .collect::<Vec<_>>(),
            CandidateOutcome::UnknownBackend(_) => Vec::new(),
        };
        match <[BackendId; 1]>::try_from(candidates) {
            Ok([backend]) => routed.push((item, Some(backend))),
            Err(candidates) if candidates.is_empty() => routed.push((item, None)),
            Err(candidates) => {
                return Err(AnalyzeRefusal::Ambiguous {
                    index: item.index,
                    candidates,
                })
            }
        }
    }
    Ok(routed
        .into_iter()
        .map(|(item, backend)| run(item, backend, cancel))
        .collect())
}

/// Run one routed item: `unsupported` when no engine took it, `incomplete`
/// when the handle is already cancelled, the engine's settlement otherwise.
fn run(item: AnalyzeItem<'_>, backend: Option<BackendId>, cancel: &Cancel) -> AnalyzeRecord {
    let AnalyzeItem {
        index, kind, work, ..
    } = item;
    let record = |terminal| AnalyzeRecord {
        request_index: index,
        result: Ok(terminal),
        model_check: None,
    };
    let (Some(backend), Some(work)) = (backend, work) else {
        return record(TerminalRecord::unrouted(
            index,
            UnavailabilityCause::ClaimKind { kind },
        ));
    };
    if let Some(source) = cancel.cause() {
        return record(TerminalRecord::incomplete(
            index,
            backend,
            IncompleteCause::Cancelled { source },
        ));
    }
    settle(index, backend, work(cancel))
}

fn settle(index: RequestIndex, backend: BackendId, report: EngineReport) -> AnalyzeRecord {
    match report {
        EngineReport::Model(record) => AnalyzeRecord {
            request_index: index,
            // A refuted value without its reproduced counterexample is an
            // engine fault, never a verdict.
            result: Ok(record
                .to_terminal(index, backend.clone())
                .unwrap_or_else(|_| TerminalRecord::failed(index, backend))),
            model_check: Some(record),
        },
        EngineReport::Statistical(record) => AnalyzeRecord {
            request_index: index,
            result: Ok(record.into_terminal(index, backend)),
            model_check: None,
        },
        EngineReport::Refused(refusal) => AnalyzeRecord {
            request_index: index,
            result: Err(refusal),
            model_check: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use qsl_foundation::bound::DomainKind;
    use qsl_route::{descriptor_from_manifest, Candidate, Mode};

    use super::*;

    fn engine(id: &str, kind: Capability) -> Engine {
        Engine::new(
            descriptor_from_manifest(
                Candidate::new(BackendId::new(id)),
                ProviderOrigin::Linked,
                ManifestDigest::from_digest([9; 32]),
                [(Some(kind.to_wire()), Some(Mode::Bounded.to_wire()))],
                Some(&[DomainKind::Population.to_wire()]),
                Some(qsl_route::FairnessKindsMember::List(&[])),
                None,
            )
            .expect("a well-formed test engine"),
        )
    }

    /// Two engines advertising one claim kind give an item two candidates,
    /// which routing never chooses between: `analyze` refuses before
    /// running it.
    #[test]
    fn two_engines_for_one_kind_are_refused_not_chosen_between() {
        let engines = [
            engine("test-a", Capability::OperationContract),
            engine("test-b", Capability::OperationContract),
        ];
        let items = vec![AnalyzeItem::new(
            RequestIndex::new(0),
            Capability::OperationContract,
            ItemEvidence::Absent,
            |_| panic!("an ambiguous item runs nothing"),
        )];
        let refusal = analyze(&engines, items, &Cancel::new()).expect_err("an ambiguous route");
        assert!(
            matches!(&refusal, AnalyzeRefusal::Ambiguous { candidates, .. } if candidates.len() == 2),
            "{refusal:?}"
        );
    }

    /// An item an engine advertises but no adapter builds settles
    /// `unsupported`, naming the kind, and runs nothing.
    #[test]
    fn an_unadapted_item_settles_unsupported_even_when_an_engine_advertises_it() {
        let engines = [engine("test-a", Capability::OperationContract)];
        let items = vec![AnalyzeItem::unadapted(
            RequestIndex::new(0),
            Capability::OperationContract,
        )];
        let records = analyze(&engines, items, &Cancel::new()).expect("registers");
        let terminal = records[0].result.as_ref().expect("a record");
        assert_eq!(
            terminal.value(),
            &TerminalValue::Unsupported(UnavailabilityCause::ClaimKind {
                kind: Capability::OperationContract
            })
        );
    }
}
