// SPDX-License-Identifier: AGPL-3.0-or-later
//! The `analyze` operation (FR-281, ADR-029 OP-2): decide claims over every
//! behaviour of a model with the layer-A engines, route each item to an
//! engine by the engines' published manifests, and settle each result as one
//! FR-331 terminal record.
//!
//! [`analyze`] owns the operation, not the engines: each [`Engine`] is its
//! manifest ([`crate::model_check::manifest`], [`crate::statistical::manifest`])
//! and each item is the work an engine runs ([`AnalyzeItem`]). Routing reads
//! only the manifests, through FR-288's descriptor and `fsl-route`'s
//! registry; an item whose claim kind no engine advertises settles
//! `unsupported`, naming the kind.
//!
//! Settlement is the engine's own layer-6 map, never `analyze`'s opinion of
//! an engine's evidence: EN-1's results settle through `settle_model_check`
//! (its certificate checker and counterexample replay), EN-4's through
//! `check_statistical` and FR-192's map, and EN-5's through its solver,
//! qualified certificate checker and FR-203's map. The FR-280 provider
//! entry uses these same adapters, so `analyze` and the entry write one
//! record for one item. The item adapters ([`model_item`],
//! [`statistical_item`], [`exact_probabilistic_item`]) are the only
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

mod clause_item;
mod derived_item;
mod digital_item;
pub mod document;
mod exact;
mod finite_hyper_item;
mod hybrid_item;
mod hybrid_process;
mod hyper_route;
mod invariant_item;
mod items;
mod protocol_item;
mod reduction_item;
mod refinement_item;
pub(crate) mod request;
pub(crate) use request::dependency_packages;
mod schedule_item;
mod task_product_item;
pub use task_product_item::TaskProductReport;
pub(crate) mod simulation_item;
mod state_graph_item;
mod timed_item;
pub(crate) use request::analyze_bound;

pub use request::{
    analyze, ClaimItem, ClaimRefusal, FiniteHyperItem, HybridItem, ItemForm, ItemOptions,
    NegotiatedItem, ProbabilisticItem, ProbabilisticLimits, ProvisionRefusal, SourceProvision,
    TemporalItem,
};

use filament_exact::{Cancel, ObjectReference, ScalarLimits};
use filament_semantic_value::catalog::{CatalogCode, Category};
use fsl_eval::simulation::model::PopulationUniverse;
use fsl_eval::simulation::protocol::ProtocolTransition;
use fsl_foundation::diagnostic::category_of;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::{BackendId, RequestIndex};
use fsl_package::CheckedPackage;
use fsl_replay::statistical::StatisticalRecord;
use fsl_replay::{
    CounterexampleKind, CounterexampleWireError, OutcomeItemError, TemporalCounterexample,
};
use fsl_replay::{
    DeclineCode, IncompleteCause, ModelCheckRecord, Operation, OutcomeDiagnostic, OutcomeDocument,
    OutcomeItem, OutcomeStage, ProofRefusalCause, TerminalRecord, TerminalValue,
};
use fsl_route::routing::Disposition;
use fsl_route::{BackendDescriptor, Candidate, Mode, ProviderOrigin, RegistrationRefusal};
use fsl_route::{NegotiationCause, NegotiationOutcome};
use fsl_semantics::check::{Capability, DeclarationFamily};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::observation::{
    AdmissionFailure, DocumentProvision, DocumentRef, ObservationLimits,
};

pub use crate::contract_check::contract_item;
pub use crate::value_check::value_item;
pub(crate) use clause_item::{replay_protocol_over, ProtocolReplayInputs, ReplayShape};
pub use clause_item::{
    ClauseItemRefusal, ClauseLimits, ClauseSubject, PackageClauseSubject, CLAUSE_TRANSCRIPT,
};
pub use derived_item::{deadlock_model_item, PackageDeadlockSubject};
pub(crate) use derived_item::{derived_target, NoLetters};
pub use exact::{exact_probabilistic_item, ExactProbabilityEvidence};
#[cfg(feature = "test-support")]
pub use exact::{exact_probabilistic_item_tampered, exact_probabilistic_item_witness_tampered};
pub use invariant_item::{PackageInvariantSubject, PackageItemInputs};
pub(crate) use items::admission;
pub use items::{
    derived_model_item, derived_protocol_item, derived_zone_item, hyper_form_item, model_item,
    protocol_item, statistical_item, timed_refinement_item, zone_item, DerivedItemMismatch,
    ModelItem, ProtocolItem, ProtocolReplayFn, RefinementReplayFn, TimedItem, TimedRefinementItem,
};
pub use simulation_item::{SimulationItem, SimulationMode, SimulationSelection};
pub use state_graph_item::PackageStateGraphSubject;

use crate::provider::{
    ExternalProvider, ProviderOptions, ProviderRegistrationCause, ProviderRegistrationRefusal,
    ProviderReport,
};
use crate::zone_check::ZoneCheckRecord;
use crate::{contract_check, model_check, statistical, value_check, zone_check};

/// FSpec FR-331 "Backend tool": the closed `{name, version}` naming the code
/// that decides a backend's results. A linked FSL engine is a pure
/// in-process engine: its tool is the crate that implements it at its
/// package version. An external provider names the executable it fronts.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct BackendTool {
    name: String,
    version: String,
}

/// A [`BackendTool`] with an empty `name` or `version` (FSpec FR-331-AC-37).
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("a backend tool names a nonempty name and version")]
pub struct EmptyToolMember;

impl BackendTool {
    /// The tool `name` at `version`.
    ///
    /// # Errors
    ///
    /// [`EmptyToolMember`] when either is empty.
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
    ) -> Result<Self, EmptyToolMember> {
        let (name, version) = (name.into(), version.into());
        if name.is_empty() || version.is_empty() {
            return Err(EmptyToolMember);
        }
        Ok(Self { name, version })
    }

    /// The tool of an engine linked into this crate.
    fn linked() -> Self {
        Self {
            name: env!("CARGO_PKG_NAME").to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
        }
    }

    /// The tool's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The tool's version.
    pub fn version(&self) -> &str {
        &self.version
    }
}

/// One layer-A engine as `analyze` runs it: its backend identity and the
/// descriptor FR-288 converts its manifest into.
#[derive(Clone, Debug)]
pub struct Engine {
    descriptor: BackendDescriptor,
    /// The code that decides this engine's results (FSpec FR-331).
    tool: BackendTool,
    /// The external provider an [`Engine::external`] engine decides items
    /// with; absent for the in-process engines.
    provider: Option<std::sync::Arc<dyn ExternalProvider>>,
    /// The in-process hybrid provider an [`Engine::hybrid`] engine decides
    /// hybrid items with (FR-750).
    hybrid: Option<std::sync::Arc<dyn fsl_replay::HybridProvider>>,
    /// The byte exchange of an [`Engine::hybrid_process`] engine (FR-750).
    hybrid_exchange: Option<std::sync::Arc<dyn HybridProcessExchange>>,
}

/// The transport of one process hybrid provider (FR-750, FR-635): it takes
/// the FR-447 process request frame bytes and the run's cancellation token
/// and returns the provider's FSpec FR-331 result frame bytes or a typed
/// tool failure. FSL writes the request and reads the result; starting the
/// process, FR-462 framing, limits, the deadline and cancellation belong to
/// the implementor, the driver's plugin host. A deadline overrun or a
/// cancellation is returned as a result frame whose entry is `incomplete`
/// with cause `timed-out` or `cancelled` (FSpec FR-462).
pub trait HybridProcessExchange: std::fmt::Debug + Send + Sync {
    /// Exchange one request frame for its result frame.
    ///
    /// # Errors
    ///
    /// The [`fsl_replay::ToolFailure`] FSpec FR-462's failure table names.
    fn exchange(
        &self,
        request: &[u8],
        cancel: &filament_exact::Cancel,
    ) -> Result<Vec<u8>, fsl_replay::ToolFailure>;
}

impl Engine {
    /// The engine a descriptor describes: only the crate's own adapters name
    /// one.
    pub(crate) fn new(descriptor: BackendDescriptor) -> Self {
        Self {
            descriptor,
            tool: BackendTool::linked(),
            provider: None,
            hybrid: None,
            hybrid_exchange: None,
        }
    }

    /// An engine that decides hybrid items with `provider` (FR-750).
    ///
    /// # Errors
    ///
    /// [`NotHybrid`] when `descriptor`'s `dynamics` lacks `hybrid` or it does
    /// not advertise `temporal-satisfaction`; no engine is made and
    /// `provider` is never called.
    pub fn hybrid(
        descriptor: BackendDescriptor,
        provider: Box<dyn fsl_replay::HybridProvider>,
    ) -> Result<Self, NotHybrid> {
        let hybrid = descriptor
            .dynamics()
            .is_some_and(|kinds| kinds.contains(&fsl_route::DynamicsKind::Hybrid))
            && descriptor
                .advertises()
                .any(|(kind, _)| kind == Capability::TemporalSatisfaction);
        if !hybrid {
            return Err(NotHybrid {
                backend: descriptor.id().clone(),
            });
        }
        Ok(Self {
            descriptor,
            tool: BackendTool::linked(),
            provider: None,
            hybrid: Some(provider.into()),
            hybrid_exchange: None,
        })
    }

    /// The reference hybrid provider `fsl-hybrid-reference` (FR-1019), which
    /// `Engines::fsl()` leaves out.
    ///
    /// # Errors
    ///
    /// Only if the registry's admission rules refuse the provider's own
    /// descriptor.
    pub fn hybrid_reference() -> Result<Self, RegistrationRefusal> {
        let descriptor = hybrid_reference_descriptor()?;
        Ok(Self {
            descriptor,
            tool: BackendTool::linked(),
            provider: None,
            hybrid: Some(std::sync::Arc::new(fsl_hybrid_reference::ReferenceProvider)),
            hybrid_exchange: None,
        })
    }

    /// An engine whose hybrid provider runs in a process of its own
    /// (FR-750): `descriptor` is its `Process` descriptor, `tool` its FSpec
    /// FR-331 tool record and `exchange` the transport of its request and
    /// result frames. FSL writes the request, reads the result and settles it
    /// as it settles an in-process provider's answer, except that a `proved`
    /// entry is the plugin's `trusted` word unless FR-751 accepts its
    /// certificate.
    ///
    /// # Errors
    ///
    /// [`NotHybrid`] when `descriptor`'s `dynamics` lacks `hybrid` or it does
    /// not advertise `temporal-satisfaction`; no engine is made and `exchange`
    /// is never called.
    pub fn hybrid_process(
        descriptor: BackendDescriptor,
        tool: BackendTool,
        exchange: Box<dyn HybridProcessExchange>,
    ) -> Result<Self, NotHybrid> {
        let hybrid = descriptor
            .dynamics()
            .is_some_and(|kinds| kinds.contains(&fsl_route::DynamicsKind::Hybrid))
            && descriptor
                .advertises()
                .any(|(kind, _)| kind == Capability::TemporalSatisfaction);
        if !hybrid {
            return Err(NotHybrid {
                backend: descriptor.id().clone(),
            });
        }
        Ok(Self {
            descriptor,
            tool,
            provider: None,
            hybrid: None,
            hybrid_exchange: Some(exchange.into()),
        })
    }

    /// The process exchange this engine decides hybrid items with, if any.
    pub(crate) fn hybrid_exchange(&self) -> Option<&dyn HybridProcessExchange> {
        self.hybrid_exchange.as_deref()
    }

    /// The hybrid provider this engine decides items with, if any.
    pub(crate) fn hybrid_provider(&self) -> Option<&dyn fsl_replay::HybridProvider> {
        self.hybrid.as_deref()
    }

    /// An engine that decides its advertised items with `provider`, a
    /// registered decision procedure outside FSL (FR-968). A `refuted`
    /// verdict settles only after the item's counterexample replay reproduces.
    ///
    /// # Errors
    ///
    /// [`ProviderRegistrationRefusal`] when `descriptor` advertises no
    /// `(kind, mode)` pair, or advertises `value-validity` without the
    /// `unbounded` mode; no engine is made and `provider` is never called.
    /// `tool` is the provider's FSpec FR-331 tool record. The
    /// engine holds no limit: a run's FSpec FR-462 limits are the run's
    /// [`ProviderOptions`] (FR-964), which every request carries.
    pub fn external(
        descriptor: BackendDescriptor,
        tool: BackendTool,
        provider: Box<dyn ExternalProvider>,
    ) -> Result<Self, ProviderRegistrationRefusal> {
        let advertised: Vec<_> = descriptor.advertises().collect();
        let cause = if advertised.is_empty() {
            Some(ProviderRegistrationCause::AbsentKind)
        } else {
            let value_validity_modes: Vec<_> = advertised
                .iter()
                .filter_map(|(kind, mode)| (*kind == Capability::ValueValidity).then_some(*mode))
                .collect();
            (!value_validity_modes.is_empty() && !value_validity_modes.contains(&Mode::Unbounded))
                .then_some(ProviderRegistrationCause::ValueValidityNotUnbounded {
                    modes: value_validity_modes
                        .iter()
                        .map(|mode| mode.to_wire().to_owned())
                        .collect(),
                })
        };
        if let Some(cause) = cause {
            return Err(ProviderRegistrationRefusal {
                backend: descriptor.id().clone(),
                cause,
            });
        }
        Ok(Self {
            descriptor,
            provider: Some(provider.into()),
            hybrid: None,
            hybrid_exchange: None,
            tool,
        })
    }

    /// The external provider this engine decides items with, if any.
    pub(crate) fn provider(&self) -> Option<&dyn ExternalProvider> {
        self.provider.as_deref()
    }

    /// The descriptor this engine was registered with: for an external
    /// engine, the negotiated manifest entry its requests name.
    pub(crate) fn descriptor(&self) -> &BackendDescriptor {
        &self.descriptor
    }

    /// A probe engine `id` that advertises every capability kind in bounded
    /// mode: the engine [`AnalyzeItem::probe`] items run on, so a test sees
    /// which items `analyze` routes to an engine and which it refuses first.
    ///
    /// # Panics
    ///
    /// Never for a well-formed `id`.
    #[cfg(feature = "test-support")]
    pub fn probe(id: &str) -> Self {
        let advertisements = Capability::ALL
            .iter()
            .map(|kind| (Some(kind.to_wire()), Some(Mode::Bounded.to_wire())))
            .collect::<Vec<_>>();
        Self::new(
            fsl_route::descriptor_from_manifest(
                Candidate::new(BackendId::new(id)),
                ProviderOrigin::Linked,
                advertisements,
                fsl_route::ManifestMembers {
                    domains: Some(&[fsl_foundation::bound::DomainKind::Population.to_wire()]),
                    fairness_kinds: Some(fsl_route::FairnessKindsMember::List(&[])),
                    evidence_kinds: Some(&["statistical", "exact"]),
                    memory_models: None,
                    reductions: None,
                },
            )
            .expect("a well-formed probe engine that advertises every capability"),
        )
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

    /// EN-5, the exact probabilistic engine.
    ///
    /// # Errors
    ///
    /// Only if the registry's admission rules refuse EN-5's own manifest.
    pub fn exact_probabilistic() -> Result<Self, RegistrationRefusal> {
        crate::exact_probabilistic::manifest::descriptor(ProviderOrigin::Linked).map(Self::new)
    }

    /// EN-7, the closed-form schedulability engine (FR-247), which decides
    /// `schedulability` items over a checked task set (FR-633).
    ///
    /// # Errors
    ///
    /// Only if the registry's admission rules refuse EN-7's own manifest.
    pub fn schedulability() -> Result<Self, RegistrationRefusal> {
        crate::schedulability::manifest::descriptor(ProviderOrigin::Linked).map(Self::new)
    }

    /// The value-validity check arm (FSpec FR-483, FR-484).
    ///
    /// # Errors
    ///
    /// Only if the registry's admission rules refuse the arm's own manifest.
    pub fn value_check() -> Result<Self, RegistrationRefusal> {
        value_check::manifest::descriptor(ProviderOrigin::Linked).map(Self::new)
    }

    /// The operation-contract check arm (FSpec FR-483, FR-485).
    ///
    /// # Errors
    ///
    /// Only if the registry's admission rules refuse the arm's own manifest.
    pub fn contract_check() -> Result<Self, RegistrationRefusal> {
        contract_check::manifest::descriptor(ProviderOrigin::Linked).map(Self::new)
    }

    /// The protocol analysis engine: `local-projection` and
    /// `realizability` (ADR-036 PA-4).
    ///
    /// # Errors
    ///
    /// Only if the registry's admission rules refuse the engine's own
    /// manifest.
    pub fn protocol_analysis() -> Result<Self, RegistrationRefusal> {
        crate::protocol_analysis::manifest::descriptor(ProviderOrigin::Linked).map(Self::new)
    }

    /// The value-validity arm publishing `max_cases` as its default case
    /// budget (FSpec FR-483 "Descriptor modes, domains and registration").
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

    /// The modes the engine's manifest advertises for `kind`, in wire-name
    /// order: the descriptor's own iteration order varies between runs, and
    /// the diagnostic that carries the modes is written into documents that
    /// must be stable (FR-565).
    fn modes(&self, kind: Capability) -> Vec<Mode> {
        let mut modes: Vec<Mode> = self
            .descriptor
            .advertises()
            .filter_map(|(advertised, mode)| (advertised == kind).then_some(mode))
            .collect();
        modes.sort_by_key(|mode| mode.to_wire());
        modes
    }
}

/// [`Engine::hybrid`] was given a descriptor that is no hybrid provider's
/// (FR-750): `invalid_capability`/`invalid-dynamics`.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error(
    "invalid-dynamics: {backend:?} does not publish `hybrid` dynamics for temporal-satisfaction"
)]
pub struct NotHybrid {
    /// The backend whose descriptor was refused.
    pub backend: BackendId,
}

fn hybrid_reference_descriptor() -> Result<BackendDescriptor, RegistrationRefusal> {
    use fsl_hybrid_reference::{BACKEND, MAX_SOLVER_STEPS, MAX_SOLVER_TIME, OPTION_DEFAULTS};
    fsl_route::descriptor_from_manifest_with_dynamics(
        Candidate::new(BackendId::new(BACKEND)),
        ProviderOrigin::Linked,
        [Mode::Bounded, Mode::Unbounded].map(|mode| {
            (
                Some(Capability::TemporalSatisfaction.to_wire()),
                Some(mode.to_wire()),
            )
        }),
        fsl_route::ManifestMembers {
            domains: Some(&[fsl_foundation::bound::DomainKind::Integer.to_wire()]),
            fairness_kinds: Some(fsl_route::FairnessKindsMember::List(&[])),
            evidence_kinds: None,
            memory_models: None,
            reductions: None,
        },
        Some(fsl_route::DynamicsMember::List(&[
            fsl_route::DynamicsKind::Hybrid.to_wire(),
        ])),
        Some(fsl_route::SolverBounds {
            max_solver_steps: Some(MAX_SOLVER_STEPS),
            max_solver_time: Some(MAX_SOLVER_TIME),
        }),
    )
    .map(|descriptor| {
        descriptor.with_options(
            OPTION_DEFAULTS
                .iter()
                .map(|(name, default)| ((*name).to_owned(), (*default).to_owned())),
        )
    })
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
    /// The run's provider options (FR-964), which every provider request
    /// this run sends carries.
    options: ProviderOptions,
}

/// What every item of one analyze run reads of its package (FR-531,
/// FR-940): the checked package, and the model-check input the run builds
/// once, which each item's model system reads instead of building its own.
#[derive(Clone)]
pub struct RunInput<'a> {
    /// The checked package the run analyzes.
    pub package: &'a fsl_package::CheckedPackage,
    /// The model-check input every item's model system reads.
    pub model: fsl_eval::simulation::model::ModelCheckInput<'a>,
}

impl<'a> RunInput<'a> {
    /// The source route's run input (FR-531): the package and the input
    /// read from its `CheckedGraph`.
    pub fn source(package: &'a fsl_package::CheckedPackage) -> Self {
        Self {
            package,
            model: fsl_eval::simulation::model::ModelCheckInput::source(package),
        }
    }
}

/// FR-120's caller-supplied subject members other than the checked package.
///
/// The provider and analyze operations combine this value with the package
/// and [`AnalyzeLimits`] to admit one model system before running any item.
#[derive(Clone)]
pub struct SubjectBinding<'a> {
    /// FR-056's domain package input.
    pub packages: &'a fsl_semantics::model::intake::PackageInput,
    /// The snapshot documents selected by `initial`.
    pub snapshots: &'a DocumentProvision,
    /// Initial states selected from `snapshots`, in request order.
    pub initial: Vec<DocumentRef>,
    /// The finite key sets for the subject's populations.
    pub universes: Vec<PopulationUniverse>,
}

/// The caller's shared accounting, observation, normalization and deadline
/// limits for one analyze request (FR-277, FR-281).
///
/// The deadline must be stated explicitly; there is no implicit default.
/// ```compile_fail,E0277
/// use fsl_analyze::analyze::{AnalyzeLimits, DeadlineSupply};
/// let limits = AnalyzeLimits { ..Default::default() };
/// ```
///
/// ```
/// use fsl_analyze::analyze::{AnalyzeLimits, DeadlineSupply};
/// let limits = AnalyzeLimits {
///     deadline: DeadlineSupply::Unbounded,
///     ..AnalyzeLimits::unbounded()
/// };
/// assert_eq!(limits.deadline, DeadlineSupply::Unbounded);
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AnalyzeLimits {
    /// The scalar evaluation budget.
    pub accounting: ScalarLimits,
    /// The snapshot observation budgets.
    pub observation: ObservationLimits,
    /// The model normalization budgets.
    pub normalization: ModelNormalizationLimits,
    /// The operation deadline in milliseconds, if one was supplied.
    pub deadline: DeadlineSupply,
    /// The ceiling on the canonical bytes of each package emission the
    /// analysis makes (`emit.output_bytes`, FR-264 Behavior 5).
    pub emit: fsl_package::EmitLimits,
    /// The bound on each replay read the analysis starts
    /// (`replay.input_bytes`, FR-255; FR-940-AC-22).
    pub replay: fsl_replay::ReplayLimits,
}

impl AnalyzeLimits {
    /// Published default budgets with an explicitly unbounded deadline.
    pub fn unbounded() -> Self {
        Self {
            accounting: ScalarLimits {
                integer_bits: u64::MAX,
                decimal_digits: u64::MAX,
                scale_expansion: u64::MAX,
                text_input_bytes: u64::MAX,
                text_scalars: u64::MAX,
                normalized_scalars: u64::MAX,
                unit_edges: u64::MAX,
                value_occurrences: u64::MAX,
                work_units: 1_000_000,
                result_units: u64::MAX,
            },
            observation: ObservationLimits::default(),
            normalization: ModelNormalizationLimits::default(),
            deadline: DeadlineSupply::Unbounded,
            emit: fsl_package::EmitLimits::new(fsl_replay::spine::EMIT_OUTPUT_BYTES_DEFAULT),
            replay: fsl_replay::ReplayLimits::default(),
        }
    }
}

/// An admitted-for-execution request: one subject binding and its negotiated
/// items. Each item is already typed by its layer-A adapter and carries the
/// disposition established by the caller's negotiation.
///
/// Each claim is resolved in this package; no request member carries an
/// independently constructed model subject or replay request.
pub struct AnalyzeRequest<'a> {
    /// The shared subject binding.
    pub binding: SubjectBinding<'a>,
    /// The negotiated items, in request order.
    pub items: Vec<NegotiatedItem>,
    /// Source bytes under every label the checked package names.
    pub provision: SourceProvision,
}

/// The outcome of analyzing a request, preserving the typed result for each
/// item in request order.
#[derive(Debug)]
pub struct AnalyzeOutcome {
    /// The per-item outcomes.
    pub records: Vec<TerminalRecord>,
    /// Exactly the counterexamples whose native replay reproduced a refutation.
    pub counterexamples: Vec<ItemCounterexample>,
    /// Aggregate availability joined from the settled records to the
    /// request's required indices (FR-507-AC-5); a gap when the request
    /// repeats an index.
    pub aggregate: Result<Aggregate, AggregateGap>,
    /// What each record's engine reported beside its terminal record, in
    /// request order (FR-621 `exploration` and `certificate`).
    pub details: Vec<ItemDetail>,
    /// The FR-990 global-conformance output for each settled conformance item.
    pub protocol_conformance:
        std::collections::BTreeMap<RequestIndex, fsl_replay::ProtocolConformanceOutput>,
    /// EN-7's closed-form outcome, with its evidence, for each item it
    /// settled (FR-247, FR-633).
    pub closed_form: std::collections::BTreeMap<RequestIndex, fsl_replay::ClosedFormOutcome>,
}

/// What an item's engine reported beside its terminal record (FR-621).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ItemDetail {
    /// The item's request index.
    pub request_index: RequestIndex,
    /// The states, transitions and automaton states an explicit-state
    /// engine explored; `None` for an engine that explores no states.
    pub exploration: Option<fsl_replay::RunStats>,
    /// The qualified-core checker that accepted a proof's certificate.
    pub checker: Option<fsl_replay::CertificateChecker>,
    /// The steps of the supplied trace a `finite-replay` proof replayed.
    pub replayed_steps: Option<u64>,
    /// FR-235's liveness premise, when established by EN-1 or EN-6.
    pub non_zeno: Option<fsl_replay::NonZeno>,
    /// The memory bound a weak-memory protocol run used and whether it was
    /// reached (FSpec FR-439, FR-225); `None` under `sc` and for a model
    /// subject.
    pub memory_bound: Option<fsl_replay::StatedMemoryBound>,
    /// What a `finite-replay` item reports of a protocol claim beyond its
    /// terminal record (FR-941).
    pub finite_replay: Option<FiniteReplayDetail>,
    /// A hybrid entry's certificate and its outcome (FSpec FR-652).
    pub hybrid_certificate: Option<fsl_replay::CertificateRecord>,
    /// The FR-286 spelling of the provider's complete counterexample in a
    /// `hybrid-trace` disagreement (FR-450 "Settlement").
    pub hybrid_disagreement: Option<serde_json::Value>,
    /// A bounded task-product stop's FSpec FR-331 `task_search` frontier
    /// evidence, without its `identity`.
    pub task_search: Option<serde_json::Value>,
    /// The `task_product` part of the FSpec FR-418 certificate the checker
    /// accepted, for a proved task-product item.
    pub task_certificate: Option<serde_json::Value>,
}

/// FR-941: what a `finite-replay` item reports of a protocol claim beyond its
/// terminal record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FiniteReplayDetail {
    /// The deciding required clause's reading over the replayed behaviour,
    /// which writes the item's `disposition.temporal` (FSpec FR-352).
    pub evaluation:
        Option<Result<fsl_eval::temporal::TemporalOutcome, fsl_eval::temporal::TemporalRefusal>>,
    /// The `where` of a protocol expression undefined at a replayed state:
    /// `{position, state_key, locus}`.
    pub undefined_where: Option<serde_json::Value>,
}

impl ItemDetail {
    fn of(record: &AnalyzeRecord) -> Self {
        let model = record.model_check.as_deref();
        let protocol = record.protocol_check.as_deref();
        Self {
            request_index: record.request_index,
            exploration: model
                .and_then(|model| model.exploration)
                .or_else(|| protocol.and_then(|protocol| protocol.exploration))
                .or_else(|| record.state_graph.as_ref().map(|graph| graph.exploration)),
            checker: model
                .and_then(|model| model.checker)
                .or_else(|| {
                    record
                        .task_product
                        .as_deref()
                        .and_then(|report| report.proof.as_ref())
                        .filter(|proof| {
                            matches!(
                                proof.verdict,
                                task_product_item::ProofVerdict::Checked(
                                    fsl_replay::ZoneCertificateVerdict::Accepted
                                )
                            )
                        })
                        .map(|_| fsl_replay::CertificateChecker::Zone)
                })
                .or_else(|| record.state_graph.as_ref().and_then(|graph| graph.checker))
                .or_else(|| protocol.and_then(|protocol| protocol.checker)),
            replayed_steps: record.replayed_steps,
            non_zeno: record
                .non_zeno
                .or_else(|| record.zone_check.as_deref().and_then(|zone| zone.non_zeno)),
            memory_bound: protocol.and_then(|protocol| protocol.memory_bound),
            finite_replay: record.finite_replay.clone(),
            hybrid_certificate: record.hybrid_certificate.clone(),
            hybrid_disagreement: record.hybrid_disagreement.clone(),
            task_search: record
                .task_product
                .as_deref()
                .and_then(|report| report.task_search.clone()),
            task_certificate: record.task_product.as_deref().and_then(|report| {
                let proof = report.proof.as_ref()?;
                matches!(
                    proof.verdict,
                    task_product_item::ProofVerdict::Checked(
                        fsl_replay::ZoneCertificateVerdict::Accepted
                    )
                )
                .then_some(())?;
                proof.certificate.as_ref()?.get("task_product").cloned()
            }),
        }
    }
}

/// Why analyze could not admit a shared subject binding.
#[derive(Debug, thiserror::Error)]
pub enum AnalyzeRefusal {
    /// FR-120 did not admit the subject binding.
    #[error("the analyze subject binding did not admit: {0:?}")]
    Binding(AdmissionFailure),
    /// Source bytes named by the package were not provided.
    #[error("the analyze source provision did not admit: {0}")]
    Provision(ProvisionRefusal),
    /// The strict reader refused the package the run compiled: the reader's
    /// own code, cause and pointer, never an internal failure.
    #[error("the package the run compiled is not admitted: {}", .0.label())]
    Package(fsl_package::PackageAdmissionRefusal),
}

/// The deadline stated by the frontend, which owns its timer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeadlineSupply {
    /// No wall-clock deadline was requested.
    Unbounded,
    /// The frontend cancels the shared handle when these milliseconds elapse.
    Milliseconds(u64),
}

/// One replayed counterexample, associated with its original request item.
#[derive(Debug)]
pub struct ItemCounterexample {
    /// The item's request index.
    pub request_index: RequestIndex,
    /// The engine whose candidate was reproduced.
    pub backend: BackendId,
    /// The reproduced counterexample payload.
    pub counterexample: Counterexample,
    /// The counterexample in FR-286's wire spelling, written where the
    /// run's package is at hand.
    pub wire: Result<serde_json::Value, CounterexampleWireError>,
}

/// Two counterexamples are equal when they name the same item and engine and
/// spell the same wire (FR-286), the surface a comparison reads.
impl PartialEq for ItemCounterexample {
    fn eq(&self, other: &Self) -> bool {
        self.request_index == other.request_index
            && self.backend == other.backend
            && self.wire == other.wire
    }
}

/// The reproduced counterexample of a `refuted` item, one variant per kind
/// the in-process engines produce.
#[derive(Debug)]
pub enum Counterexample {
    /// A hyperproperty's reproduced tuple of model behaviours.
    Hyper(Box<fsl_replay::HyperCounterexample>),
    /// EN-1 and EN-6's temporal counterexample over a model subject, with
    /// the object its `over` parameter is bound to when the run named one.
    Temporal {
        /// The counterexample.
        counterexample: Box<TemporalCounterexample>,
        /// The object the clause's `over` parameter is bound to.
        over: Option<ObjectReference>,
    },
    /// EN-1's temporal counterexample over a protocol subject.
    Protocol {
        /// The counterexample.
        counterexample: Box<TemporalCounterexample<ProtocolTransition>>,
        /// The subject it was found over.
        subject: Box<fsl_replay::ProtocolWire>,
    },
    /// EN-5's replayed probabilistic witness.
    Probabilistic(Box<fsl_replay::ReproducedProbabilityWitness>),
    /// The value-validity arm's, or an external provider's, violating
    /// assignment.
    Assignments(Box<value_check::ValueCounterexample>),
    /// The operation-contract arm's counterexample.
    Contract(Box<fsl_replay::contract_check::ContractCounterexample>),
    /// A state-graph item's settled instance (FR-628): the record, its
    /// FSpec FR-393 refuting evidence when refuted and its witnesses when
    /// proved by one.
    StateGraph(Box<GraphReport>),
    /// A global-conformance recording prefix, already replayed in process.
    GlobalConformance(Box<serde_json::Value>),
    /// An EN-7 closed-form miss in FSpec FR-419's `schedulability-miss`
    /// spelling, typed and not replayed.
    ScheduleMiss(Box<serde_json::Value>),
    /// A hybrid provider's trace, reproduced by exact replay (FR-450).
    Hybrid(Box<fsl_replay::HybridCounterexample>),
    /// A task-product model trace accepted by FR-237 and spelled by FR-364.
    TaskProduct(Box<serde_json::Value>),
    /// FR-898's reproduced trace tuple and joint position.
    FiniteHyper(Box<serde_json::Value>),
}

/// What a state-graph engine run reports for one item (FR-628): the record of
/// the instance that decides it and the spelling of its evidence.
#[derive(Clone, Debug)]
pub struct GraphReport {
    /// The decisive instance's FR-169 record.
    pub record: fsl_replay::StateGraphRecord,
    /// The explored graph's counts (FR-621 `exploration`).
    pub exploration: fsl_replay::RunStats,
    /// A refuted record's evidence in FSpec FR-393's spelling.
    pub counterexample: Option<serde_json::Value>,
    /// A proved record's witnesses in FSpec FR-393's spelling, with the
    /// sampled ones.
    pub evidence: Option<serde_json::Value>,
    /// The accepted timed state-graph certificate's FR-712 document.
    pub certificate_document: Option<serde_json::Value>,
    /// The checker that accepted a timed state-graph proof.
    pub checker: Option<fsl_replay::CertificateChecker>,
}

/// A reproduced counterexample with its FR-286 spelling, written where the
/// engine's subject was at hand.
#[derive(Debug)]
pub struct Spelled<T> {
    /// The reproduced counterexample.
    pub found: Box<T>,
    /// Its wire spelling.
    pub wire: Result<serde_json::Value, CounterexampleWireError>,
}

impl<T> std::ops::Deref for Spelled<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.found
    }
}

impl AnalyzeRefusal {
    /// The FR-286 `analyze` document of a whole-request refusal: the
    /// admission failure's category, one diagnostic with its catalog code and
    /// cause, and no item, with the empty `aggregate` and `registry` FR-621
    /// requires of every `analyze` document.
    pub fn document(&self) -> OutcomeDocument {
        self.refusal_document().with_empty_analyze_members()
    }

    fn refusal_document(&self) -> OutcomeDocument {
        match self {
            Self::Binding(failure) => {
                refusal_document(Operation::Analyze, failure, failure.to_string())
                    .with_last_stage(Some(OutcomeStage::S4))
            }
            Self::Package(refusal) => {
                let mut diagnostic = OutcomeDiagnostic::new(
                    refusal.cause.as_deref(),
                    refusal.code.as_str(),
                    None,
                    self.to_string(),
                );
                if let Some(pointer) = &refusal.pointer {
                    diagnostic = diagnostic.with_details(Some(serde_json::Map::from_iter([(
                        "path".to_owned(),
                        serde_json::Value::String(pointer.clone()),
                    )])));
                }
                OutcomeDocument::new(
                    Operation::Analyze,
                    Some(OutcomeStage::S4),
                    refusal.code.category(),
                )
                .with_diagnostics(vec![diagnostic])
            }
            Self::Provision(refusal) => {
                let catalog = refusal.catalog_code();
                let category = category_of(&catalog).unwrap_or(Category::Refusal);
                OutcomeDocument::new(Operation::Analyze, Some(OutcomeStage::S4), category)
                    .with_diagnostics(vec![OutcomeDiagnostic::new(
                        Some(catalog.cause()),
                        catalog.code(),
                        None,
                        refusal.to_string(),
                    )])
            }
        }
    }
}

/// The FR-286 document of an `operation` refused by an admission `failure`:
/// the failure's category and one diagnostic with its catalog code and cause.
pub(crate) fn refusal_document(
    operation: Operation,
    failure: &AdmissionFailure,
    message: String,
) -> OutcomeDocument {
    let (catalog, category, details) = match failure {
        AdmissionFailure::Refused(record) | AdmissionFailure::Incomplete(record) => {
            let details = (!record.fields.is_empty()).then(|| {
                record
                    .fields
                    .iter()
                    .map(|(name, value)| ((*name).to_owned(), serde_json::json!(value)))
                    .collect()
            });
            (
                record.catalog,
                category_of(&record.catalog).unwrap_or(Category::Refusal),
                details,
            )
        }
        AdmissionFailure::Fault(fault) => (fault.catalog_code(), fault.category(), None),
    };
    OutcomeDocument::new(operation, None, category).with_diagnostics(vec![OutcomeDiagnostic::new(
        Some(catalog.cause()),
        catalog.code(),
        None,
        message,
    )
    .with_details(details)])
}

impl AnalyzeOutcome {
    /// The FR-286 `analyze` document of these records: one item per record in
    /// request order (FR-286-AC-3), a `refuted` item carrying its one
    /// counterexample.
    ///
    /// # Errors
    ///
    /// [`OutcomeItemError`] for a `refuted` record without its counterexample
    /// or whose counterexample has no wire spelling.
    pub fn document(&self) -> Result<OutcomeDocument, OutcomeItemError> {
        analyze_document_routed_with_conformance(
            &self.records,
            &self.counterexamples,
            &std::collections::BTreeMap::new(),
            &self.protocol_conformance,
        )
    }
}

/// The FR-286 `analyze` document of `records`, one item per record in
/// request order, each `refuted` item carrying its counterexample from
/// `counterexamples`. Its category is the most severe item's (ADR-029 CB-4).
/// `provider::run`'s results build the same document through this function,
/// so the two serialize to equal bytes (FR-286-AC-14).
///
/// # Errors
///
/// [`OutcomeItemError::MissingCounterexample`] for a `refuted` record with
/// none in `counterexamples`, and [`OutcomeItemError::Unspelled`] for one
/// whose counterexample has no wire spelling.
pub fn analyze_document(
    records: &[TerminalRecord],
    counterexamples: &[ItemCounterexample],
) -> Result<OutcomeDocument, OutcomeItemError> {
    analyze_document_routed(records, counterexamples, &std::collections::BTreeMap::new())
}

/// How negotiation routed one item, which FR-621 "Last stage" reads: an
/// item with no route settled before negotiation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ItemRoute {
    /// Negotiated to an FSL in-process engine (EN-1 to EN-7, the zone
    /// search), which settles it at `S6c`.
    InProcess,
    /// Settled at negotiation, `S6b`: negotiated to a registered provider
    /// outside FSL, or settled by its negotiation's own disposition.
    Negotiated,
}

/// [`analyze_document`] with each negotiated item's [`ItemRoute`] by
/// request index, so the document's `last_stage` (FR-621 "Last stage")
/// reads where each item was settled.
///
/// # Errors
///
/// As [`analyze_document`].
pub fn analyze_document_routed(
    records: &[TerminalRecord],
    counterexamples: &[ItemCounterexample],
    routes: &std::collections::BTreeMap<RequestIndex, ItemRoute>,
) -> Result<OutcomeDocument, OutcomeItemError> {
    analyze_document_routed_with_conformance(
        records,
        counterexamples,
        routes,
        &std::collections::BTreeMap::new(),
    )
}

fn analyze_document_routed_with_conformance(
    records: &[TerminalRecord],
    counterexamples: &[ItemCounterexample],
    routes: &std::collections::BTreeMap<RequestIndex, ItemRoute>,
    conformance: &std::collections::BTreeMap<RequestIndex, fsl_replay::ProtocolConformanceOutput>,
) -> Result<OutcomeDocument, OutcomeItemError> {
    let items = records
        .iter()
        .map(|record| {
            let item = analyze_item(record, counterexamples)?;
            Ok::<_, OutcomeItemError>(match conformance.get(&record.request_index()) {
                Some(output) => item.with_protocol_conformance(output.clone()),
                None => item,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    // FR-286: a cancelled request names no stage.
    let cancelled = records.iter().any(|record| {
        matches!(
            record.value(),
            TerminalValue::Incomplete(IncompleteCause::Cancelled { .. })
        )
    });
    let stage = records
        .iter()
        .map(|record| {
            item_stage(
                record,
                counterexamples,
                routes.get(&record.request_index()).copied(),
            )
        })
        .max_by_key(|stage| stage_rank(*stage))
        .unwrap_or(OutcomeStage::S4);
    Ok(OutcomeDocument::settled(
        Operation::Analyze,
        (!cancelled).then_some(stage),
        items,
    ))
}

fn stage_rank(stage: OutcomeStage) -> u8 {
    match stage {
        OutcomeStage::S4 => 4,
        OutcomeStage::S6b => 6,
        OutcomeStage::S6c => 7,
        OutcomeStage::S7 => 8,
        _ => 9,
    }
}

/// FR-621 "Last stage": the stage that settled `record`, negotiated by
/// `route` (none when it settled before negotiation); the document takes the
/// greatest of `S4` and its items'.
fn item_stage(
    record: &TerminalRecord,
    counterexamples: &[ItemCounterexample],
    route: Option<ItemRoute>,
) -> OutcomeStage {
    // A counterexample reaches the document only once its replay reproduced
    // it; the state-graph one is built as a typed witness and not replayed.
    if let Some(found) = counterexamples
        .iter()
        .find(|found| found.request_index == record.request_index())
    {
        return if matches!(
            found.counterexample,
            Counterexample::StateGraph(_) | Counterexample::ScheduleMiss(_)
        ) {
            OutcomeStage::S7
        } else {
            OutcomeStage::S8
        };
    }
    match route {
        // Negotiated to an outside provider, or settled by a disposition of
        // its negotiation (FSpec FR-331 `settled_by` `disposition`).
        Some(ItemRoute::Negotiated) => OutcomeStage::S6b,
        Some(ItemRoute::InProcess) if record.settled_by_disposition() => OutcomeStage::S6b,
        Some(ItemRoute::InProcess) => OutcomeStage::S6c,
        // A disposition before negotiation: an unknown claim, an
        // unregistered backend, a malformed member.
        None if record.settled_by_disposition() => OutcomeStage::S4,
        None => OutcomeStage::S6c,
    }
}

/// The FR-286 item of `record`, with its counterexample from
/// `counterexamples` when it is `refuted`: the one item `analyze` and
/// `prove` both write. A claim evaluation undefined at a state keeps its
/// `undefined-evaluation` cause beside the counterexample.
///
/// # Errors
///
/// As [`analyze_document`].
pub fn analyze_item(
    record: &TerminalRecord,
    counterexamples: &[ItemCounterexample],
) -> Result<OutcomeItem, OutcomeItemError> {
    let found = counterexamples
        .iter()
        .find(|found| found.request_index == record.request_index());
    if let Some(Counterexample::StateGraph(graph)) = found.map(|found| &found.counterexample) {
        return Ok(
            OutcomeItem::from_state_graph(record.request_index(), &graph.record)
                .with_graph_evidence(graph.counterexample.clone(), graph.evidence.clone())
                .with_graph_certificate_document(graph.certificate_document.clone()),
        );
    }
    let wire = found.map(|found| found.wire.clone()).transpose()?;
    let undefined = found.and_then(|found| found.counterexample.undefined_cause());
    OutcomeItem::from_analyze(record, wire, undefined)
}

impl Engines {
    /// The layer-A engines `analyze` has an item adapter for: EN-1, EN-4,
    /// EN-5, EN-6, the value-validity and operation-contract arms, and the
    /// protocol analysis engine.
    ///
    /// # Errors
    ///
    /// Only if the registry's admission rules refuse an engine's own
    /// manifest.
    pub fn fsl() -> Result<Self, RegistrationRefusal> {
        Ok(Self {
            options: ProviderOptions::default(),
            engines: vec![
                Engine::model_check()?,
                Engine::statistical()?,
                Engine::exact_probabilistic()?,
                Engine::zone_check()?,
                Engine::value_check()?,
                Engine::contract_check()?,
                Engine::protocol_analysis()?,
                Engine::schedulability()?,
            ],
        })
    }

    /// [`Engines::fsl`] with the reference hybrid provider added (FR-1019).
    ///
    /// # Errors
    ///
    /// Only if the registry's admission rules refuse an engine's own
    /// manifest.
    pub fn fsl_with_reference_hybrid() -> Result<Self, RegistrationRefusal> {
        let mut engines = Self::fsl()?;
        engines.engines.push(Engine::hybrid_reference()?);
        Ok(engines)
    }

    /// These engines under the run's provider `options` (FR-964): the
    /// FSpec FR-462 limits every provider request of the run carries. A
    /// provider-routing entry sets them once for the run; no engine holds
    /// one of its own.
    #[must_use]
    pub fn under(mut self, options: ProviderOptions) -> Self {
        self.options = options;
        self
    }

    /// The registry snapshot a process request carries (FSpec FR-331): the
    /// FR-331 descriptor of every registered engine, ordered by backend.
    pub(crate) fn manifest_wire(&self) -> Vec<serde_json::Value> {
        let mut snapshot: Vec<_> = self
            .engines
            .iter()
            .map(|engine| (engine.descriptor.id().as_str(), engine))
            .collect();
        snapshot.sort_by_key(|(id, _)| *id);
        snapshot
            .into_iter()
            .map(|(_, engine)| hybrid_process::descriptor_wire(&engine.descriptor, &engine.tool))
            .collect()
    }

    /// The run's provider options.
    pub fn options(&self) -> &ProviderOptions {
        &self.options
    }

    /// The exact provider descriptors owned by these linked engines.
    ///
    /// The returned descriptors are borrowed from their engine instances;
    /// callers cannot substitute a capability-only registration.
    pub fn registrations(&self) -> impl ExactSizeIterator<Item = &BackendDescriptor> {
        self.engines.iter().map(|engine| &engine.descriptor)
    }

    /// Each registered descriptor with its FSpec FR-331 tool record.
    pub fn tools(&self) -> impl ExactSizeIterator<Item = (&BackendDescriptor, &BackendTool)> {
        self.engines
            .iter()
            .map(|engine| (&engine.descriptor, &engine.tool))
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

    pub(crate) fn get(&self, backend: &BackendId) -> Option<&Engine> {
        self.engines.iter().find(|engine| engine.id() == backend)
    }

    /// Whether any registered engine is a candidate for a hybrid item
    /// (FR-075): it advertises `temporal-satisfaction` and its `dynamics`
    /// holds `hybrid`. FR-750 reserves `HybridProviderAbsent` for the case
    /// where none is.
    fn has_hybrid_candidate(&self) -> bool {
        self.engines.iter().any(|engine| {
            engine
                .descriptor()
                .dynamics()
                .is_some_and(|kinds| kinds.contains(&fsl_route::DynamicsKind::Hybrid))
                && engine
                    .descriptor()
                    .advertises()
                    .any(|(kind, _)| kind == Capability::TemporalSatisfaction)
        })
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
    /// An unbounded extent (FSpec FR-272): the kind, the candidate and the
    /// modes its manifest advertises for that kind.
    Unbounded {
        /// The item's kind.
        kind: Capability,
        /// The engine that refused the unbounded extent.
        candidate: BackendId,
        /// The modes its manifest advertises for the kind.
        modes: Vec<Mode>,
    },
    /// A finite trace population exceeds the bound declared by its hyper clause.
    FiniteHyperBound {
        /// The declared trace bound.
        bound: u64,
        /// The number of distinct traces supplied.
        found: usize,
    },
    /// A capability kind that does not apply to the checked claim family.
    Inapplicable {
        /// The request item's index.
        request_index: RequestIndex,
        /// The requested capability kind.
        capability: Capability,
        /// The claim's declaration family, absent for an unknown claim node.
        family: Option<DeclarationFamily>,
    },
    /// A published option whose value the provider refused (FSpec FR-779-AC-2):
    /// the item, the backend, the option and the value as the item wrote it.
    OptionValue {
        /// The request item's index.
        request_index: RequestIndex,
        /// The provider that refused the value.
        backend: BackendId,
        /// The option's name.
        option: String,
        /// The refused value.
        value: String,
    },
    /// A provider item whose checked package could not be emitted, so no
    /// request was sent (FSpec FR-331-AC-33): the kind and the backend.
    NotEmitted {
        /// The item's kind.
        kind: Capability,
        /// The provider the item was negotiated to.
        backend: BackendId,
    },
}

impl Payload {
    /// The payload as the FR-331 diagnostic `detail`, keyed by FR-272 payload
    /// names (`kind`, `candidate`, `modes`, `request_index`, `capability`,
    /// `family`).
    #[must_use]
    pub fn detail(&self) -> serde_json::Map<String, serde_json::Value> {
        use serde_json::{json, Value};
        let entries: Vec<(&str, Value)> = match self {
            Self::None => Vec::new(),
            Self::Kind(kind) => vec![("kind", json!(kind.to_wire()))],
            Self::Backend(backend) => vec![("backend", json!(backend.as_str()))],
            Self::Inconsistent { candidate, .. } => {
                vec![("candidates", json!([candidate.as_str()]))]
            }
            Self::Unbounded {
                kind,
                candidate,
                modes,
            } => vec![
                ("kind", json!(kind.to_wire())),
                ("candidate", json!(candidate.as_str())),
                (
                    "modes",
                    Value::Array(modes.iter().map(|mode| json!(mode.to_wire())).collect()),
                ),
            ],
            Self::OptionValue {
                request_index,
                backend,
                option,
                value,
            } => vec![
                ("request_index", json!(request_index.get())),
                ("backend", json!(backend.as_str())),
                ("option", json!(option)),
                ("value", json!(value)),
            ],
            Self::FiniteHyperBound { bound, found } => {
                vec![("bound", json!(bound)), ("found", json!(found))]
            }
            Self::NotEmitted { kind, backend } => vec![
                ("kind", json!(kind.to_wire())),
                ("backend", json!(backend.as_str())),
                ("reason", json!("package-not-emitted")),
            ],
            Self::Inapplicable {
                request_index,
                capability,
                family,
            } => {
                let mut entries = vec![
                    ("request_index", json!(request_index.get())),
                    ("capability", json!(capability.to_wire())),
                ];
                if let Some(family) = family {
                    entries.push(("family", json!(family_label(*family))));
                }
                entries
            }
        };
        entries
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect()
    }
}

/// The FR-290 declaration-family label.
const fn family_label(family: DeclarationFamily) -> &'static str {
    match family {
        DeclarationFamily::Value => "value",
        DeclarationFamily::StateModel => "state-model",
        DeclarationFamily::TemporalTrace => "temporal-trace",
        DeclarationFamily::RelationClaim => "relation-claim",
        DeclarationFamily::Protocol => "protocol",
        DeclarationFamily::Probabilistic => "probabilistic",
        DeclarationFamily::RealTime => "real-time",
    }
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

/// A record settled by a disposition (FSpec FR-331 `settled_by`
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
    /// EN-1's replay-gated hyperproperty settlement.
    Hyper(Box<fsl_replay::HyperRecord>),
    /// EN-1's record, settled by `settle_model_check`.
    Model(Box<ModelCheckRecord>),
    /// EN-1's record over an authored clause, with the object the clause's
    /// `over` parameter is bound to in its counterexample.
    Clause(
        Box<ModelCheckRecord>,
        Option<ObjectReference>,
        Option<fsl_replay::NonZeno>,
    ),
    /// EN-1's record over a refinement (FR-630), with the counterexample's
    /// per-position abstract images and FR-146 step records.
    Refinement(Box<ModelCheckRecord>, Option<serde_json::Value>),
    /// EN-1's record over a protocol subject, settled by
    /// `settle_protocol_check`.
    Protocol(
        Box<ModelCheckRecord<ProtocolTransition>>,
        Option<fsl_replay::ProtocolWire>,
    ),
    /// EN-4's record, settled by FR-192's map.
    Statistical(Box<StatisticalRecord>),
    /// The value-validity arm's finding.
    Value(Box<value_check::ValueReport>),
    /// The operation-contract arm's finding.
    Contract(Box<contract_check::ContractReport>),
    /// An external provider's verdict, its counterexample replayed.
    Provider(Box<ProviderReport>),
    /// EN-1's state-graph record (FR-169) for the item's decisive instance.
    StateGraph(Box<GraphReport>),
    /// EN-6's record, settled by FR-235's map, or a timed refinement's by
    /// FR-144's rows over EN-6's causes.
    Zone(Box<ZoneCheckRecord>, Option<ObjectReference>),
    /// EN-5's report, settled by FR-203 and the exact certificate check.
    Exact(Box<exact::ExactReport>),
    /// EN-7's outcome, settled by FR-248's map once its evidence was
    /// recomputed.
    ClosedForm(Box<schedule_item::ClosedFormReport>),
    /// EN-6's bounded search over an FR-249 task-derived timed subject.
    TaskProduct(Box<task_product_item::TaskProductReport>),
    /// The protocol analysis engine's report, settled by FR-573's map.
    ProtocolAnalysis(Box<crate::protocol_analysis::ProtocolAnalysisArtifact>),
    /// The protocol analysis engine's replay of a supplied finite trace.
    FiniteReplay(Box<crate::protocol_analysis::FiniteReplayReport>),
    /// FR-896's result with the typed FR-990 item output.
    GlobalConformance(Box<crate::protocol_analysis::conformance::ConformanceReport>),
    /// A hybrid provider's answer, settled by FR-250 and FR-450.
    Hybrid(Box<hybrid_item::HybridReport>),
    /// FR-898's finite trace-set verdict and reproduced tuple witness.
    FiniteHyper(Box<FiniteHyperReport>),
    /// The engine refused the item, and its settlement map gives the refusal
    /// no terminal record.
    Refused(Refusal),
}

/// A finite-hyperproperty result already checked against its request
/// population and checked body.
pub(crate) struct FiniteHyperReport {
    /// The evidence-backed terminal result.
    pub terminal: TerminalRecord,
    /// The FR-898 false-tuple witness, when refuted.
    pub counterexample: Option<serde_json::Value>,
}

/// The FR-281 "Engine refusals" table's rows.
pub(crate) enum Refusal {
    /// `declined`, with the refusal's cause, `DeclineCode` and full
    /// diagnostic.
    Declined(ProofRefusalCause, DeclineCode, CatalogCode),
    /// A declined request with structured detail naming the missing input.
    DeclinedWithDetail(
        ProofRefusalCause,
        DeclineCode,
        CatalogCode,
        serde_json::Map<String, serde_json::Value>,
    ),
    /// An unbounded extent: `unsupported` with `unbounded-extent`.
    RequiresBound,
    /// A finite trace population exceeds the bound declared by its hyper clause.
    RequiresFiniteHyperBound { bound: u64, found: usize },
    /// EN-4 `NotStatistical`: `invalid-request` with `not-statistical`.
    NotStatistical,
    /// A published option's value outside its domain (FSpec FR-779-AC-2):
    /// `invalid-request` with `invalid-option-value`, naming the option and
    /// the value.
    InvalidOptionValue {
        /// The option's name.
        option: String,
        /// The refused value.
        value: String,
    },
    /// `incomplete`: the request's subject exhausted a resource bound before
    /// the engine ran.
    Incomplete(fsl_replay::IncompleteCause),
    /// `unsupported` with the refusal's own diagnostic.
    Unsupported(CatalogCode),
    /// `unsupported` with structured details naming the unavailable package fact.
    UnsupportedWithDetail(CatalogCode, serde_json::Map<String, serde_json::Value>),
    /// `failed` with the refusal's diagnostic.
    Failed(CatalogCode),
    /// `failed` with the refusal's diagnostic and its detail (FR-750
    /// "Replay refusals": the invariant a checked package broke).
    FailedWithDetail(CatalogCode, serde_json::Map<String, serde_json::Value>),
    /// FR-750: `unsupported`, `unsupported_construct`/`expression-form`, the
    /// detail of a hybrid provider's construct outside its class, which the
    /// record completes with the backend.
    UnsupportedConstruct(serde_json::Map<String, serde_json::Value>),
    /// `unsupported` `unsupported-requested-capability`: the package of a
    /// provider item could not be emitted, so no request was sent (FSpec
    /// FR-331-AC-33).
    NotEmitted,
}

/// The work an engine runs for one item.
type Work<'a> = Box<dyn FnOnce(&Cancel, &Engine) -> EngineReport + 'a>;

/// What settles one item: an engine's own work, or an external provider's
/// verdict, which [`analyze_items`] obtains for every provider item of a
/// backend in one FSpec FR-331 request after routing and negotiation, and
/// the item then replays (FR-281 "External provider arm", FR-635).
pub(crate) enum ItemWork<'a> {
    /// The work an in-process engine runs.
    Engine(Work<'a>),
    /// The members a provider request is built from.
    Provider(Box<crate::provider::ProviderWork<'a>>),
}

/// One item to analyze: its request index, its claim, its settled
/// disposition and the work an engine runs for it, when the claim kind has an
/// item adapter.
pub struct AnalyzeItem<'a> {
    index: RequestIndex,
    kind: Capability,
    /// The checked claim's declaration family, read from its admitted claim
    /// identity before an engine item is built.
    family: Option<DeclarationFamily>,
    mode: Mode,
    disposition: Disposition,
    diagnostic: Option<Diagnostic>,
    work: Option<ItemWork<'a>>,
    /// The caller's requiredness for this request index (FR-507-AC-5).
    required: bool,
    /// The claim the item is read at, the locus of a counterexample that
    /// names an undefined evaluation.
    claim: Option<OccurrenceKey>,
    /// Whether the item is a hybrid item (FR-750), which settles
    /// `HybridProviderAbsent` or `tool-unavailable` where other items settle
    /// by their disposition.
    hybrid: bool,
}

impl<'a> AnalyzeItem<'a> {
    /// The provider work of an item that [`analyze_items`] will settle on
    /// the external engine its disposition names, with that engine: an item
    /// whose kind applies to its claim, that negotiation settled
    /// `Supported` on a registered engine advertising the item's kind and
    /// mode. Every other item settles without a provider request, as
    /// `run` states.
    pub(crate) fn provider_route<'e>(
        &self,
        engines: &'e Engines,
    ) -> Option<(&'e Engine, RequestIndex, &crate::provider::ProviderWork<'a>)> {
        let Some(ItemWork::Provider(work)) = &self.work else {
            return None;
        };
        if self
            .family
            .is_none_or(|family| !self.kind.applies_to_declaration(family))
        {
            return None;
        }
        let Disposition::Supported(candidate) = &self.disposition else {
            return None;
        };
        let engine = engines.get(candidate.id())?;
        engine
            .modes(self.kind)
            .contains(&self.mode)
            .then_some((engine, self.index, work.as_ref()))
    }

    /// An item that an external provider settles: `work` holds the members
    /// its FSpec FR-331 request is built from (FR-281 "External provider
    /// arm").
    pub(crate) fn new_with_provider(
        index: RequestIndex,
        family: Option<DeclarationFamily>,
        kind: Capability,
        backend: &str,
        work: crate::provider::ProviderWork<'a>,
    ) -> Self {
        Self {
            index,
            kind,
            family,
            mode: Mode::Unbounded,
            disposition: Disposition::Supported(Candidate::new(BackendId::new(backend))),
            diagnostic: None,
            work: Some(ItemWork::Provider(Box::new(work))),
            required: false,
            claim: None,
            hybrid: false,
        }
    }

    /// An item of claim kind `kind` and mode `mode` that `backend` runs as
    /// `work`: `Supported(backend)` unless [`Self::disposed`] says otherwise.
    pub(crate) fn new(
        index: RequestIndex,
        kind: Capability,
        family: Option<DeclarationFamily>,
        mode: Mode,
        backend: &str,
        work: impl FnOnce(&Cancel) -> EngineReport + 'a,
    ) -> Self {
        Self {
            index,
            kind,
            family,
            mode,
            disposition: Disposition::Supported(Candidate::new(BackendId::new(backend))),
            diagnostic: None,
            work: Some(ItemWork::Engine(Box::new(move |cancel, _| work(cancel)))),
            required: false,
            claim: None,
            hybrid: false,
        }
    }

    /// As [`Self::new`], for work that reads the engine's descriptor: the
    /// value-validity arm takes its `max_cases` default from it.
    pub(crate) fn new_with_engine(
        index: RequestIndex,
        kind: Capability,
        family: Option<DeclarationFamily>,
        mode: Mode,
        backend: &str,
        work: impl FnOnce(&Cancel, &Engine) -> EngineReport + 'a,
    ) -> Self {
        Self {
            index,
            kind,
            family,
            mode,
            disposition: Disposition::Supported(Candidate::new(BackendId::new(backend))),
            diagnostic: None,
            work: Some(ItemWork::Engine(Box::new(work))),
            required: false,
            claim: None,
            hybrid: false,
        }
    }

    /// An item of claim kind `kind` that no item adapter builds: no engine
    /// is given work for it, so it settles `unsupported` with
    /// `unsupported-requested-capability`, naming the kind.
    pub fn unadapted(
        index: RequestIndex,
        kind: Capability,
        family: Option<DeclarationFamily>,
    ) -> Self {
        Self {
            index,
            kind,
            family,
            mode: Mode::Bounded,
            disposition: Disposition::Unsupported,
            diagnostic: Some(unsupported_kind(kind)),
            work: None,
            required: false,
            claim: None,
            hybrid: false,
        }
    }

    /// A probe item of kind `kind` for [`Engine::probe`]`(backend)`, in no
    /// family until [`Self::at_claim`] reads one. Its work sets `ran` and
    /// settles `failed`, so a test sees whether `analyze` ran it.
    #[cfg(feature = "test-support")]
    pub fn probe(
        index: RequestIndex,
        kind: Capability,
        backend: &str,
        ran: &'a std::cell::Cell<bool>,
    ) -> Self {
        Self::new(index, kind, None, Mode::Bounded, backend, move |_| {
            ran.set(true);
            EngineReport::Refused(Refusal::Failed(CatalogCode::new(
                "runtime_invariant",
                "established-invariant-broken",
            )))
        })
    }

    /// This item at the claim `claim` of `package`, in the family that
    /// claim is in (FR-507): the family replaces the one the adapter gave
    /// the item, so an item built from an engine run, which holds no claim
    /// occurrence, is read at its request item's `item_kind` and claim.
    #[must_use]
    pub fn at_claim(
        mut self,
        package: &CheckedPackage,
        item_kind: fsl_route::request::ItemKind,
        claim: &OccurrenceKey,
    ) -> Self {
        self.family = crate::claim_family::claim_family(package, item_kind, claim);
        self.claimed(claim.clone())
    }

    /// This item at the claim `claim`, the locus of a counterexample that
    /// names an undefined evaluation.
    #[must_use]
    pub(crate) fn claimed(mut self, claim: OccurrenceKey) -> Self {
        self.claim = Some(claim);
        self
    }

    /// This item with the caller's requiredness for its request index. The
    /// item's identity and position are unchanged; an item is optional until
    /// this says otherwise.
    #[must_use]
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// The caller's requiredness for this item's request index, to be read
    /// before the item is consumed by [`analyze_items`] and joined to its
    /// records by [`aggregate`].
    pub fn requirement(&self) -> RequestRequirement {
        RequestRequirement {
            index: self.index,
            required: self.required,
        }
    }

    /// This item marked a hybrid item (FR-750).
    #[must_use]
    pub(crate) fn hybrid(mut self) -> Self {
        self.hybrid = true;
        self
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
        code: NegotiationOutcome::UnsupportedRequestedCapability.code(),
        payload: Payload::Kind(kind),
    }
}

/// One item's settlement.
#[derive(Debug)]
pub struct AnalyzeRecord {
    /// EN-1's hyperproperty record, retaining a reproduced trace tuple.
    pub hyper_check: Option<Box<fsl_replay::HyperRecord>>,
    /// The item's request index.
    pub request_index: RequestIndex,
    /// The item's claim, when the request named one.
    pub claim: Option<OccurrenceKey>,
    /// The item's settlement.
    pub settlement: Settlement,
    /// FR-235's digital-clock non-Zeno premise, when EN-1's SCC phase
    /// established one.
    pub non_zeno: Option<fsl_replay::NonZeno>,
    /// EN-1's full record, carrying the counterexample of a `refuted` item.
    pub model_check: Option<Box<ModelCheckRecord>>,
    /// EN-1's full record over a protocol subject, carrying the
    /// counterexample of a `refuted` item.
    pub protocol_check: Option<Box<ModelCheckRecord<ProtocolTransition>>>,
    /// EN-6's full record, carrying the counterexample of a `refuted` item.
    pub zone_check: Option<Box<ZoneCheckRecord>>,
    /// The value-validity arm's counterexample, carried by a `refuted` item.
    pub value_check: Option<Box<value_check::ValueCounterexample>>,
    /// The operation-contract arm's counterexample, carried by a `refuted`
    /// item.
    pub contract_check: Option<Spelled<fsl_replay::contract_check::ContractCounterexample>>,
    /// The certificate EN-5's checker accepted, carried by a `proved` item
    /// (FR-281-AC-1).
    pub exact_probabilistic: Option<Box<ExactProbabilityEvidence>>,
    /// The independently replayed EN-5 probabilistic counterexample.
    pub probabilistic_counterexample: Option<Spelled<fsl_replay::ReproducedProbabilityWitness>>,
    /// The object an EN-1 clause counterexample's `over` parameter is bound
    /// to, which only the run's subject can name (FSpec FR-364).
    pub over_reference: Option<ObjectReference>,
    /// A refinement counterexample's added members (FR-630 "Outputs"):
    /// `steps`, the abstract image of each position, and `step_simulations`,
    /// the FR-146 step records.
    pub refinement_steps: Option<serde_json::Value>,
    /// The protocol subject an EN-1 protocol counterexample was found over
    /// (FSpec FR-433's `subject`).
    pub protocol_subject: Option<fsl_replay::ProtocolWire>,
    /// The protocol analysis result: its key, and the witness or the
    /// counterexample with its FR-351 record (ADR-036 PA-24, PA-25).
    pub protocol_analysis: Option<Box<crate::protocol_analysis::ProtocolAnalysisArtifact>>,
    /// The FR-990 per-obligation, closure and global conformance result.
    pub protocol_conformance: Option<fsl_replay::ProtocolConformanceOutput>,
    /// FR-991's replayed protocol or recording counterexample spelling.
    pub protocol_conformance_counterexample: Option<serde_json::Value>,
    /// The state-graph engine's report (FR-628).
    pub state_graph: Option<Box<GraphReport>>,
    /// EN-7's closed-form outcome with its evidence (FR-247): the miss of a
    /// `refuted` item, the fixpoints or demand points of a `proved` one.
    pub closed_form: Option<Box<fsl_replay::ClosedFormOutcome>>,
    /// FSpec FR-419's `schedulability-miss` counterexample of a refuted
    /// EN-7 item (FSL FR-621): recomputed by the closed-form check, so it has
    /// no replay.
    pub schedule_miss: Option<Box<serde_json::Value>>,
    /// The replayed hybrid counterexample of a `refuted` hybrid item
    /// (FR-750), with its FR-286 spelling.
    pub hybrid_counterexample: Option<Spelled<fsl_replay::HybridCounterexample>>,
    /// The certificate of a hybrid provider's entry and what FR-751 did
    /// with it (FSpec FR-652).
    pub hybrid_certificate: Option<fsl_replay::CertificateRecord>,
    /// The FR-286 spelling of the provider's complete counterexample in a
    /// `hybrid-trace` disagreement (FR-450 "Settlement").
    pub hybrid_disagreement: Option<serde_json::Value>,
    /// FR-249's task-product search outcome, retained for FR-621 settlement.
    pub task_product: Option<Box<TaskProductReport>>,
    /// FR-364 task-product model trace, already replayed by the rebuilt plan.
    pub(crate) task_product_counterexample: Option<Box<serde_json::Value>>,
    /// FR-898's reproduced false-tuple witness.
    pub finite_hyper_counterexample: Option<serde_json::Value>,
    /// The number of steps of a supplied trace a `finite-replay` item
    /// replayed to its proof (FR-941).
    pub replayed_steps: Option<u64>,
    /// FR-941: what a `finite-replay` item reports of a protocol claim.
    pub finite_replay: Option<FiniteReplayDetail>,
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
    /// A record of `settlement` that holds no engine record.
    fn of(request_index: RequestIndex, settlement: Settlement) -> Self {
        Self {
            request_index,
            hyper_check: None,
            claim: None,
            settlement,
            non_zeno: None,
            model_check: None,
            protocol_check: None,
            zone_check: None,
            value_check: None,
            contract_check: None,
            exact_probabilistic: None,
            probabilistic_counterexample: None,
            over_reference: None,
            refinement_steps: None,
            protocol_subject: None,
            replayed_steps: None,
            finite_replay: None,
            protocol_analysis: None,
            protocol_conformance: None,
            protocol_conformance_counterexample: None,
            closed_form: None,
            schedule_miss: None,
            hybrid_counterexample: None,
            hybrid_certificate: None,
            hybrid_disagreement: None,
            task_product: None,
            task_product_counterexample: None,
            finite_hyper_counterexample: None,
            state_graph: None,
        }
    }

    /// The counterexample's FR-286 spelling, written where the run's
    /// `package` is at hand; `None` when the record holds none.
    fn spell(
        &self,
        package: Option<&CheckedPackage>,
    ) -> Option<Result<serde_json::Value, CounterexampleWireError>> {
        let claim = self.claim.as_ref();
        if let Some(found) = self
            .hyper_check
            .as_ref()
            .and_then(|record| record.counterexample.as_ref())
        {
            return Some(fsl_replay::hyper_counterexample_wire(found, claim));
        }
        if let Some(found) = self
            .model_check
            .as_ref()
            .and_then(|record| record.counterexample.as_ref())
            .or_else(|| {
                self.zone_check
                    .as_ref()
                    .and_then(|record| record.counterexample.as_ref())
            })
        {
            return Some(
                fsl_replay::temporal_wire(found, self.over_reference.as_ref(), claim).map(
                    |mut wire| {
                        if let (Some(serde_json::Value::Object(steps)), Some(members)) =
                            (&self.refinement_steps, wire.as_object_mut())
                        {
                            members.extend(steps.clone());
                        }
                        wire
                    },
                ),
            );
        }
        if let Some(found) = self.protocol_counterexample() {
            return Some(match &self.protocol_subject {
                Some(subject) => fsl_replay::protocol_temporal_wire(found, subject, claim),
                None => Err(CounterexampleWireError::Unspelled("protocol subject")),
            });
        }
        if let Some(found) = &self.protocol_conformance_counterexample {
            return Some(Ok(found.clone()));
        }
        if let Some(found) = &self.value_check {
            let Some(package) = package else {
                return Some(Err(CounterexampleWireError::Unspelled("value-check scope")));
            };
            let scope = package.graph().scope();
            return Some(fsl_replay::assignments_wire(
                &found.parameters,
                found.reproduced.assignment(),
                scope.types(),
                scope.variant_types(),
            ));
        }
        if let Some(found) = &self.contract_check {
            return Some(found.wire.clone());
        }
        if let Some(found) = &self.schedule_miss {
            // FSpec FR-331's counterexample entry around FR-419's miss object.
            let backend = self.terminal().and_then(|terminal| terminal.backend());
            return Some(Ok(serde_json::json!({
                "request_index": self.request_index.get(),
                "backend": backend.map(BackendId::as_str),
                "counterexample": (**found).clone(),
            })));
        }
        if let Some(found) = &self.hybrid_counterexample {
            return Some(found.wire.clone());
        }
        if let Some(found) = &self.task_product_counterexample {
            return Some(Ok(serde_json::json!({
                "request_index": self.request_index.get(),
                "backend": self.terminal().and_then(|terminal| terminal.backend()).map(BackendId::as_str),
                "counterexample": (**found).clone(),
            })));
        }
        if let Some(found) = &self.finite_hyper_counterexample {
            return Some(Ok(found.clone()));
        }
        self.probabilistic_counterexample
            .as_ref()
            .map(|found| found.wire.clone())
    }

    /// Move out the counterexample the engine that settled this record
    /// holds.
    fn take_counterexample(&mut self) -> Option<Counterexample> {
        if let Some(found) = self
            .hyper_check
            .as_mut()
            .and_then(|record| record.counterexample.take())
        {
            return Some(Counterexample::Hyper(Box::new(found)));
        }
        if let Some(found) = self
            .model_check
            .as_mut()
            .and_then(|record| record.counterexample.take())
            .or_else(|| {
                self.zone_check
                    .as_mut()
                    .and_then(|record| record.counterexample.take())
            })
        {
            return Some(Counterexample::Temporal {
                counterexample: Box::new(found),
                over: self.over_reference.take(),
            });
        }
        if let Some(found) = self.protocol_conformance_counterexample.take() {
            return Some(Counterexample::GlobalConformance(Box::new(found)));
        }
        if let Some(found) = self
            .protocol_check
            .as_mut()
            .and_then(|record| record.counterexample.take())
        {
            // Without a subject the item has no spelling; `spell` reports
            // that, and `split_records` settles the item `failed`.
            let subject = self.protocol_subject.take()?;
            return Some(Counterexample::Protocol {
                counterexample: Box::new(found),
                subject: Box::new(subject),
            });
        }
        if let Some(found) = self.value_check.take() {
            return Some(Counterexample::Assignments(found));
        }
        if let Some(found) = self.contract_check.take() {
            return Some(Counterexample::Contract(found.found));
        }
        if let Some(found) = self.schedule_miss.take() {
            return Some(Counterexample::ScheduleMiss(found));
        }
        if let Some(found) = self.hybrid_counterexample.take() {
            return Some(Counterexample::Hybrid(found.found));
        }
        if let Some(found) = self.task_product_counterexample.take() {
            return Some(Counterexample::TaskProduct(found));
        }
        if let Some(found) = self.finite_hyper_counterexample.take() {
            return Some(Counterexample::FiniteHyper(Box::new(found)));
        }
        self.probabilistic_counterexample
            .take()
            .map(|found| Counterexample::Probabilistic(found.found))
    }

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

    /// The outcome item (FR-286) this record writes, with its counterexample
    /// spelled over `package`: the item [`analyze_item`] writes for the same
    /// record, so `analyze` and `prove` agree. `None` for a record settled by
    /// a disposition, which the driver writes from its own disposition and
    /// diagnostic.
    pub fn outcome_item(
        &self,
        package: &CheckedPackage,
    ) -> Option<Result<OutcomeItem, OutcomeItemError>> {
        let terminal = self.terminal()?;
        let kind = self
            .counterexample()
            .map(|counterexample| &counterexample.kind)
            .or_else(|| {
                self.protocol_counterexample()
                    .map(|counterexample| &counterexample.kind)
            });
        let undefined = undefined_cause(kind, self.value_check.as_deref());
        let wire = self.spell(Some(package)).transpose();
        Some(
            wire.map_err(OutcomeItemError::from)
                .and_then(|wire| OutcomeItem::from_analyze(terminal, wire, undefined))
                .map(|item| match &self.protocol_conformance {
                    Some(output) => item.with_protocol_conformance(output.clone()),
                    None => item,
                }),
        )
    }
}

/// The cause of a claim evaluation undefined at a state: where and why. An
/// item so refuted is `refuted`, category violation, with cause
/// `undefined-evaluation`, never the `undefined` label (FR-281).
fn undefined_cause(
    kind: Option<&CounterexampleKind>,
    value: Option<&value_check::ValueCounterexample>,
) -> Option<(String, String)> {
    if let Some(undefined) = value.and_then(|found| found.reproduced.violation().undefined.as_ref())
    {
        return Some((
            format!(
                "node {}, occurrence {}/{}",
                undefined.locus.node(),
                undefined.locus.origin().role().as_str(),
                undefined.locus.origin().ordinal()
            ),
            undefined.reason.to_owned(),
        ));
    }
    match kind? {
        CounterexampleKind::UndefinedEvaluation(evaluation) => Some((
            format!(
                "position {}, state {}",
                evaluation.site.position,
                evaluation.site.state_key.hex()
            ),
            fsl_replay::undefined_reason(&evaluation.cause).to_owned(),
        )),
        CounterexampleKind::Formula
        | CounterexampleKind::Deadlock
        | CounterexampleKind::Race(_)
        | CounterexampleKind::TimeLock
        | CounterexampleKind::LoopProof(_) => None,
    }
}

impl Counterexample {
    /// Where and why this counterexample's claim evaluation was undefined,
    /// when it was.
    fn undefined_cause(&self) -> Option<(String, String)> {
        match self {
            Self::Temporal { counterexample, .. } => {
                undefined_cause(Some(&counterexample.kind), None)
            }
            Self::Protocol { counterexample, .. } => {
                undefined_cause(Some(&counterexample.kind), None)
            }
            Self::Assignments(found) => undefined_cause(None, Some(found)),
            Self::Probabilistic(_)
            | Self::Hyper(_)
            | Self::Contract(_)
            | Self::StateGraph(_)
            | Self::GlobalConformance(_)
            | Self::ScheduleMiss(_)
            | Self::Hybrid(_)
            | Self::TaskProduct(_)
            | Self::FiniteHyper(_) => None,
        }
    }
}

/// FR-281: run `items` on `engines` and settle each as one record, in
/// request order. Every item has a record: an item settled by a disposition
/// other than `supported` runs no engine, and an item whose `Supported`
/// candidate `engines` does not hold, or holds without advertising the
/// item's kind and mode, settles `invalid-request`.
pub fn analyze_items(
    engines: &Engines,
    items: Vec<AnalyzeItem<'_>>,
    cancel: &Cancel,
) -> Vec<AnalyzeRecord> {
    let mut settled = crate::provider::settle_items(engines, &items, cancel);
    items
        .into_iter()
        .map(|item| run(engines, item, cancel, &mut settled))
        .collect()
}

/// The caller's requiredness for one request index.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RequestRequirement {
    /// The request index.
    pub index: RequestIndex,
    /// Whether complete aggregate success needs this item.
    pub required: bool,
}

/// Whether complete aggregate success remains attainable (FR-507-AC-5).
/// Complete success itself is a downstream result and is never granted here.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Aggregate {
    /// Every required request index settled an FR-331 terminal record.
    Attainable,
    /// A required request index settled no terminal record (it was refused by
    /// a disposition, for example `invalid_capability`/`inapplicable-kind`),
    /// so complete success is out of reach. Names the first such index.
    Unavailable {
        /// The first required request index with no terminal record.
        request_index: RequestIndex,
    },
}

/// A request index the join of records to requirements cannot account for.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum AggregateGap {
    /// The same request index appears twice, among requirements or records.
    #[error("request index {0:?} appears more than once")]
    Duplicate(RequestIndex),
    /// A requirement has no record.
    #[error("request index {0:?} has no record")]
    Missing(RequestIndex),
    /// A record has no requirement.
    #[error("request index {0:?} was not requested")]
    Unexpected(RequestIndex),
}

/// Join `records` to the caller's `requirements` by request index and derive
/// aggregate availability from the settled records, not from any
/// pre-analysis disposition (FR-507-AC-5). An optional item never makes the
/// aggregate unavailable. A duplicate, missing or unexpected index yields no
/// aggregate.
pub fn aggregate(
    requirements: &[RequestRequirement],
    records: &[AnalyzeRecord],
) -> Result<Aggregate, AggregateGap> {
    let mut required = std::collections::BTreeMap::new();
    for requirement in requirements {
        if required
            .insert(requirement.index, requirement.required)
            .is_some()
        {
            return Err(AggregateGap::Duplicate(requirement.index));
        }
    }
    let mut settled = std::collections::BTreeMap::new();
    for record in records {
        if !required.contains_key(&record.request_index) {
            return Err(AggregateGap::Unexpected(record.request_index));
        }
        if settled
            .insert(record.request_index, record.terminal().is_some())
            .is_some()
        {
            return Err(AggregateGap::Duplicate(record.request_index));
        }
    }
    if let Some(index) = required.keys().find(|index| !settled.contains_key(index)) {
        return Err(AggregateGap::Missing(*index));
    }
    Ok(required
        .iter()
        .find(|(index, needed)| **needed && !settled[*index])
        .map_or(Aggregate::Attainable, |(index, _)| Aggregate::Unavailable {
            request_index: *index,
        }))
}

/// Analyze the negotiated items over one admitted checked-package subject
/// (FR-281). The binding is admitted before any engine item runs.
/// The FR-331 terminal record each of `records` settles, in request order: a
/// record settled by a disposition is written from that disposition and its
/// diagnostic, as FR-281 states.
pub fn terminal_records(records: Vec<AnalyzeRecord>) -> Vec<TerminalRecord> {
    records.into_iter().map(into_terminal).collect()
}

/// The terminal records of `records` and, for each `refuted` one, its one
/// counterexample (FR-281). A counterexample with no wire spelling is no
/// reproduced evidence of FR-286's form, so its item settles `failed`, an
/// engine fault, and the rest of the document is unaffected.
pub fn split_records(
    records: Vec<AnalyzeRecord>,
    package: &CheckedPackage,
) -> (Vec<TerminalRecord>, Vec<ItemCounterexample>) {
    split_records_over(records, Some(package))
}

/// [`split_records`] over the checked package `package` of the source route,
/// or over none on the package route (FR-850), whose items carry no
/// value-validity counterexample: one spelled against no package fails
/// unspelled.
pub(crate) fn split_records_over(
    records: Vec<AnalyzeRecord>,
    package: Option<&CheckedPackage>,
) -> (Vec<TerminalRecord>, Vec<ItemCounterexample>) {
    let mut terminals = Vec::with_capacity(records.len());
    let mut counterexamples = Vec::new();
    for mut record in records {
        if let Some(graph) = record.state_graph.take() {
            let terminal = into_terminal(record);
            if let Some(backend) = terminal.backend().cloned() {
                counterexamples.push(ItemCounterexample {
                    request_index: terminal.request_index(),
                    backend,
                    wire: Ok(graph
                        .counterexample
                        .clone()
                        .unwrap_or(serde_json::Value::Null)),
                    counterexample: Counterexample::StateGraph(graph),
                });
            }
            terminals.push(terminal);
            continue;
        }
        let wire = record.spell(package);
        let counterexample = record.take_counterexample();
        let mut terminal = into_terminal(record);
        if matches!(terminal.value(), TerminalValue::Refuted) {
            if let (Some(Err(_)), Some(backend)) = (&wire, terminal.backend()) {
                terminal = TerminalRecord::failed(terminal.request_index(), backend.clone());
            } else if let (Some(counterexample), Some(wire), Some(backend)) =
                (counterexample, wire, terminal.backend())
            {
                counterexamples.push(ItemCounterexample {
                    request_index: terminal.request_index(),
                    backend: backend.clone(),
                    wire,
                    counterexample,
                });
            }
        }
        terminals.push(terminal);
    }
    (terminals, counterexamples)
}

fn into_terminal(record: AnalyzeRecord) -> TerminalRecord {
    match record.settlement {
        Settlement::Terminal(terminal) => *terminal,
        Settlement::Disposed(disposed) => {
            let detail = disposed
                .diagnostic
                .as_ref()
                .map(|diagnostic| diagnostic.payload.detail())
                .unwrap_or_default();
            let diagnostic = disposed
                .diagnostic
                .map(|diagnostic| diagnostic.code)
                .unwrap_or(CatalogCode::new(
                    "runtime_invariant",
                    "established-invariant-broken",
                ));
            match disposed.kind {
                DispositionKind::RequiresBound | DispositionKind::Unsupported => {
                    TerminalRecord::disposition_unsupported(
                        record.request_index,
                        diagnostic,
                        detail,
                    )
                }
                DispositionKind::InvalidRequest => TerminalRecord::disposition_invalid_request(
                    record.request_index,
                    diagnostic,
                    detail,
                ),
            }
        }
    }
}

fn run(
    engines: &Engines,
    item: AnalyzeItem<'_>,
    cancel: &Cancel,
    settled: &mut crate::provider::Settled,
) -> AnalyzeRecord {
    let AnalyzeItem {
        index,
        kind,
        family,
        mode,
        disposition,
        diagnostic,
        work,
        required: _,
        claim,
        hybrid,
    } = item;
    let record = |settlement| AnalyzeRecord::of(index, settlement);
    let disposed = |kind, diagnostic| {
        record(Settlement::Disposed(Disposed {
            kind,
            backend: None,
            diagnostic,
        }))
    };
    if family.is_none_or(|family| !kind.applies_to_declaration(family)) {
        let diagnostic = Diagnostic {
            code: CatalogCode::new("invalid_capability", "inapplicable-kind"),
            payload: Payload::Inapplicable {
                request_index: index,
                capability: kind,
                family,
            },
        };
        return disposed(DispositionKind::InvalidRequest, Some(diagnostic));
    }
    if hybrid {
        match &disposition {
            Disposition::Unsupported if !engines.has_hybrid_candidate() => {
                return record(terminal(TerminalRecord::hybrid_provider_absent(index)));
            }
            Disposition::Supported(candidate)
                if engines.get(candidate.id()).is_none_or(|engine| {
                    engine.hybrid_provider().is_none() && engine.hybrid_exchange().is_none()
                }) =>
            {
                return record(terminal(TerminalRecord::hybrid_tool_unavailable(
                    index,
                    candidate.id().clone(),
                )));
            }
            _ => {}
        }
    }
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
                code: NegotiationCause::UnknownBackend(backend.clone()).code(),
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
    let report = match work {
        ItemWork::Engine(work) => work(cancel, engine),
        ItemWork::Provider(work) => {
            crate::provider::finish(*work, settled.remove(&index), cancel, engine)
        }
    };
    // An item whose charge saw the cancel was still open when the handle was
    // cancelled and settles `incomplete` with its cause (FR-276); a handle
    // cancelled after the item's last charge is not tripped and the verdict
    // stands.
    if let Some(source) = cancel.tripped() {
        return record(terminal(TerminalRecord::incomplete(
            index,
            backend,
            IncompleteCause::Cancelled { source },
        )));
    }
    AnalyzeRecord {
        claim,
        ..settle(index, kind, engine, report)
    }
}

fn terminal(record: TerminalRecord) -> Settlement {
    Settlement::Terminal(Box::new(record))
}

/// FR-941 "Formula claims": the terminal record the deciding required
/// clause's reading `evaluation` settles when it refutes nothing. A reading
/// that refutes carries its counterexample and settles through EN-1's record.
fn claim_terminal(
    evaluation: &Result<fsl_eval::temporal::TemporalOutcome, fsl_eval::temporal::TemporalRefusal>,
    index: RequestIndex,
    backend: BackendId,
    steps: u64,
) -> TerminalRecord {
    use fsl_eval::temporal::TemporalOutcome as Outcome;
    use fsl_replay::{InconclusiveCause, ModelCheckLimit, UnavailabilityCause};
    match evaluation {
        Ok(Outcome::Holds) => TerminalRecord::proved_by_replay(index, backend, steps),
        Ok(Outcome::Pending) => {
            TerminalRecord::inconclusive(index, backend, InconclusiveCause::TemporalPending)
        }
        Ok(Outcome::InsufficientData(short)) => TerminalRecord::inconclusive(
            index,
            backend,
            InconclusiveCause::InsufficientData {
                count: short.data.count,
                min_count: short.data.min_count,
            },
        ),
        Ok(Outcome::MissingFairnessPremise { constraint }) => TerminalRecord::unsupported(
            index,
            backend,
            UnavailabilityCause::MissingFairnessPremise {
                constraint: constraint.clone(),
            },
        ),
        Ok(Outcome::Incomplete(incomplete)) => TerminalRecord::incomplete(
            index,
            backend,
            IncompleteCause::LimitReached {
                limit: ModelCheckLimit::EvaluationMeter {
                    counter: incomplete.limit_kind,
                },
                value: incomplete.limit,
            },
        ),
        Ok(Outcome::WindowLimitReached(reached)) => TerminalRecord::incomplete(
            index,
            backend,
            IncompleteCause::LimitReached {
                limit: ModelCheckLimit::MaxWindowValues,
                value: reached.limit,
            },
        ),
        Err(refusal) => TerminalRecord::declined_with(
            index,
            backend,
            ProofRefusalCause::InvalidInput,
            DeclineCode::Fsl(fsl_foundation::Code::InvalidRuntimeInput),
            refusal.catalog_code(),
        ),
        // A violated or undefined reading carries its counterexample, which
        // settles through EN-1's record; one without it broke an invariant.
        Ok(Outcome::Violated { .. } | Outcome::Undefined(_)) => {
            TerminalRecord::failed(index, backend)
        }
    }
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
        model_check,
        ..AnalyzeRecord::of(index, settlement)
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
        EngineReport::Hyper(hyper) => {
            let terminal_record = TerminalRecord::of_hyper(index, backend.clone(), &hyper)
                .unwrap_or_else(|_| TerminalRecord::failed(index, backend));
            AnalyzeRecord {
                hyper_check: Some(hyper),
                ..AnalyzeRecord::of(index, terminal(terminal_record))
            }
        }
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
        EngineReport::Clause(model, over, non_zeno) => {
            let mut settled = settle(index, kind, engine, EngineReport::Model(model));
            settled.over_reference = over;
            settled.non_zeno = non_zeno;
            settled
        }
        EngineReport::Refinement(model, steps) => {
            let mut settled = settle(index, kind, engine, EngineReport::Model(model));
            settled.refinement_steps = steps;
            settled
        }
        EngineReport::Value(report) => {
            let (settled_record, counterexample) = value_check::settle(*report, index, backend);
            let mut settled = record(terminal(settled_record), None);
            settled.value_check = counterexample.map(Box::new);
            settled
        }
        EngineReport::Provider(report) => {
            let (settled_record, counterexample) =
                crate::provider::settle_record(*report, index, backend, engine.descriptor.origin());
            let mut settled = record(terminal(settled_record), None);
            settled.value_check = counterexample.map(Box::new);
            settled
        }
        EngineReport::Contract(report) => {
            let (settled_record, counterexample) = contract_check::settle(*report, index, backend);
            let mut settled = record(terminal(settled_record), None);
            settled.contract_check = counterexample.map(|(found, wire)| Spelled {
                found: Box::new(found),
                wire,
            });
            settled
        }
        EngineReport::Protocol(protocol, subject) => {
            // A refuted value without its reproduced counterexample is an
            // engine fault, never a verdict.
            let settlement = terminal(
                protocol
                    .to_terminal(index, backend.clone())
                    .unwrap_or_else(|_| TerminalRecord::failed(index, backend)),
            );
            AnalyzeRecord {
                protocol_check: Some(protocol),
                protocol_subject: subject,
                ..AnalyzeRecord::of(index, settlement)
            }
        }
        EngineReport::Statistical(statistical) => {
            record(terminal(statistical.into_terminal(index, backend)), None)
        }
        EngineReport::FiniteReplay(report) => {
            use crate::protocol_analysis::FiniteReplayReport;
            let mut replayed = None;
            let mut protocol_check = None;
            let mut protocol_subject = None;
            let mut finite_replay = None;
            let settled_record = match *report {
                FiniteReplayReport::Replayed(steps) => {
                    replayed = Some(steps);
                    TerminalRecord::proved_by_replay(index, backend, steps)
                }
                FiniteReplayReport::Declined {
                    cause,
                    position,
                    path,
                } => {
                    let declined = TerminalRecord::declined_with(
                        index,
                        backend,
                        ProofRefusalCause::InvalidInput,
                        DeclineCode::Fsl(fsl_foundation::Code::InvalidRuntimeInput),
                        CatalogCode::new("invalid_runtime_input", cause),
                    );
                    let detail: serde_json::Map<String, serde_json::Value> = position
                        .map(|position| ("position".to_owned(), serde_json::json!(position)))
                        .into_iter()
                        .chain(path.map(|path| ("path".to_owned(), serde_json::json!(path))))
                        .collect();
                    if detail.is_empty() {
                        declined
                    } else {
                        declined.with_diagnostic_detail(detail)
                    }
                }
                FiniteReplayReport::Claim(claim) => {
                    let crate::protocol_analysis::ClaimReport {
                        steps,
                        evaluation,
                        refuted,
                    } = *claim;
                    let terminal = match refuted {
                        Some(refuted) => {
                            let terminal = refuted
                                .record
                                .to_terminal(index, backend.clone())
                                .unwrap_or_else(|_| TerminalRecord::failed(index, backend));
                            protocol_check = Some(refuted.record);
                            protocol_subject = refuted.subject;
                            terminal
                        }
                        None => {
                            if matches!(&evaluation, Ok(fsl_eval::temporal::TemporalOutcome::Holds))
                            {
                                replayed = Some(steps);
                            }
                            claim_terminal(&evaluation, index, backend, steps)
                        }
                    };
                    finite_replay = Some(FiniteReplayDetail {
                        evaluation: Some(evaluation),
                        undefined_where: None,
                    });
                    terminal
                }
                FiniteReplayReport::UndefinedStep { refuted, at } => {
                    let terminal = refuted
                        .record
                        .to_terminal(index, backend.clone())
                        .unwrap_or_else(|_| TerminalRecord::failed(index, backend));
                    protocol_check = Some(refuted.record);
                    protocol_subject = refuted.subject;
                    finite_replay = Some(FiniteReplayDetail {
                        evaluation: None,
                        undefined_where: Some(at),
                    });
                    terminal
                }
            };
            AnalyzeRecord {
                replayed_steps: replayed,
                protocol_check,
                protocol_subject,
                finite_replay,
                ..record(terminal(settled_record), None)
            }
        }
        EngineReport::ProtocolAnalysis(report) => {
            let (settled_record, artifact) =
                crate::protocol_analysis::settle(*report, index, backend);
            let mut settled = record(terminal(settled_record), None);
            settled.protocol_analysis = Some(Box::new(artifact));
            settled
        }
        EngineReport::GlobalConformance(report) => AnalyzeRecord {
            protocol_conformance: Some(report.output),
            protocol_conformance_counterexample: report.counterexample,
            ..record(terminal(report.terminal), None)
        },
        EngineReport::FiniteHyper(report) => AnalyzeRecord {
            finite_hyper_counterexample: report.counterexample,
            ..record(terminal(report.terminal), None)
        },
        EngineReport::Exact(exact) => {
            let (settled_record, evidence, counterexample) = exact.settle(index, backend);
            let mut settled = record(terminal(settled_record), None);
            settled.exact_probabilistic = evidence;
            settled.probabilistic_counterexample =
                counterexample.map(|(found, wire)| Spelled { found, wire });
            settled
        }
        EngineReport::ClosedForm(report) => {
            let schedule_item::ClosedFormReport {
                value,
                outcome,
                miss,
            } = *report;
            // A refuted value without its miss evidence is an engine fault,
            // never a verdict.
            let settlement = terminal(
                TerminalRecord::of_closed_form(index, backend.clone(), value, &outcome)
                    .unwrap_or_else(|_| TerminalRecord::failed(index, backend)),
            );
            AnalyzeRecord {
                closed_form: Some(Box::new(outcome)),
                schedule_miss: miss.map(Box::new),
                ..AnalyzeRecord::of(index, settlement)
            }
        }
        EngineReport::Hybrid(report) => {
            let hybrid_item::HybridReport {
                value,
                counterexample,
                wire,
                certificate,
                disagreement,
            } = *report;
            // A refuted value without its replayed counterexample is an
            // engine fault, never a verdict.
            let settlement = terminal(
                TerminalRecord::of_hybrid(index, backend.clone(), value, counterexample.as_deref())
                    .unwrap_or_else(|_| TerminalRecord::failed(index, backend)),
            );
            AnalyzeRecord {
                hybrid_counterexample: counterexample.map(|found| Spelled { found, wire }),
                hybrid_certificate: certificate,
                hybrid_disagreement: disagreement,
                ..AnalyzeRecord::of(index, settlement)
            }
        }
        EngineReport::TaskProduct(report) => {
            let settlement = if report.counterexample.is_some() {
                TerminalRecord::of_model_settlement(
                    index,
                    backend.clone(),
                    TerminalValue::Refuted,
                    report.temporal_counterexample.as_deref(),
                )
                .unwrap_or_else(|_| TerminalRecord::failed(index, backend.clone()))
            } else if let Some(value) = report
                .proof
                .as_ref()
                .and_then(|proof| proof_value(proof, &report.outcome))
            {
                TerminalRecord::of_model_settlement(
                    index,
                    backend.clone(),
                    value,
                    report.temporal_counterexample.as_deref(),
                )
                .unwrap_or_else(|_| TerminalRecord::failed(index, backend.clone()))
            } else {
                // FSpec FR-235 V-7, FR-331: a work-budget stop is `limit-reached`
                // for `max_task_product_work`; an open choice with its frontier
                // evidence is `unresolved-dynamics`; a closed search with no
                // accepted FR-418 certificate is `certificate-unchecked`. A stop
                // that fits none of them is an engine fault, never a limit.
                let cause = match &report.outcome {
                    crate::zone_check::TaskProductSearchOutcome::Stopped {
                        cause: crate::zone_check::TaskProductSearchStop::WorkLimit(value),
                        ..
                    } => Some(fsl_replay::IncompleteCause::LimitReached {
                        limit: fsl_replay::ModelCheckLimit::TaskProductWork,
                        value: *value,
                    }),
                    // FSpec FR-331: unresolved dynamics with its frontier
                    // evidence is `bounded-search-incomplete`.
                    crate::zone_check::TaskProductSearchOutcome::Stopped {
                        cause:
                            crate::zone_check::TaskProductSearchStop::UnresolvedChoice
                            | crate::zone_check::TaskProductSearchStop::QueueOverflow,
                        ..
                    } if report.task_search.is_some() => {
                        Some(fsl_replay::IncompleteCause::UnresolvedDynamics)
                    }
                    crate::zone_check::TaskProductSearchOutcome::Stopped {
                        cause: crate::zone_check::TaskProductSearchStop::RegionClosureRequired,
                        ..
                    } => Some(fsl_replay::IncompleteCause::CertificateUnchecked),
                    crate::zone_check::TaskProductSearchOutcome::Stopped {
                        cause: crate::zone_check::TaskProductSearchStop::Cancelled(source),
                        ..
                    } => Some(fsl_replay::IncompleteCause::Cancelled { source: *source }),
                    // Missing evidence, or a refutation whose counterexample did
                    // not reproduce.
                    crate::zone_check::TaskProductSearchOutcome::Stopped {
                        cause:
                            crate::zone_check::TaskProductSearchStop::UnresolvedChoice
                            | crate::zone_check::TaskProductSearchStop::QueueOverflow,
                        ..
                    }
                    | crate::zone_check::TaskProductSearchOutcome::Refuted(_) => None,
                };
                cause.map_or_else(
                    || TerminalRecord::failed(index, backend.clone()),
                    |cause| TerminalRecord::incomplete(index, backend.clone(), cause),
                )
            };
            AnalyzeRecord {
                task_product_counterexample: report.counterexample.clone().map(Box::new),
                task_product: Some(report),
                ..AnalyzeRecord::of(index, terminal(settlement))
            }
        }
        EngineReport::StateGraph(graph) => {
            let settlement = terminal(TerminalRecord::of_state_graph(
                index,
                backend,
                &graph.record,
            ));
            AnalyzeRecord {
                state_graph: Some(graph),
                ..AnalyzeRecord::of(index, settlement)
            }
        }
        EngineReport::Zone(zone, zone_over) => {
            // A refuted value without its reproduced counterexample is an
            // engine fault, never a verdict.
            let settlement = terminal(
                TerminalRecord::of_model_settlement(
                    index,
                    backend.clone(),
                    zone.value.clone(),
                    zone.counterexample.as_ref(),
                )
                .unwrap_or_else(|_| TerminalRecord::failed(index, backend)),
            );
            AnalyzeRecord {
                zone_check: Some(zone),
                over_reference: zone_over,
                ..AnalyzeRecord::of(index, settlement)
            }
        }
        EngineReport::Refused(Refusal::Declined(cause, code, diagnostic)) => record(
            terminal(TerminalRecord::declined_with(
                index, backend, cause, code, diagnostic,
            )),
            None,
        ),
        EngineReport::Refused(Refusal::DeclinedWithDetail(cause, code, diagnostic, detail)) => {
            record(
                terminal(
                    TerminalRecord::declined_with(index, backend, cause, code, diagnostic)
                        .with_diagnostic_detail(detail),
                ),
                None,
            )
        }
        EngineReport::Refused(Refusal::Incomplete(cause)) => record(
            terminal(TerminalRecord::incomplete(index, backend, cause)),
            None,
        ),
        EngineReport::Refused(Refusal::Failed(code)) => record(
            terminal(TerminalRecord::failed_with(index, backend, code)),
            None,
        ),
        EngineReport::Refused(Refusal::FailedWithDetail(code, detail)) => record(
            terminal(
                TerminalRecord::failed_with(index, backend, code).with_diagnostic_detail(detail),
            ),
            None,
        ),
        EngineReport::Refused(Refusal::UnsupportedConstruct(mut detail)) => {
            // FSpec FR-272: the provider whose class excludes the construct
            // stands where the profile would.
            detail.insert("backend".to_owned(), backend.as_str().into());
            record(
                terminal(TerminalRecord::hybrid_unsupported_construct(
                    index, backend, detail,
                )),
                None,
            )
        }
        EngineReport::Refused(Refusal::Unsupported(code)) => {
            disposed(DispositionKind::Unsupported, Diagnostic::new(code), backend)
        }
        EngineReport::Refused(Refusal::UnsupportedWithDetail(code, detail)) => record(
            terminal(TerminalRecord::disposition_unsupported(index, code, detail)),
            None,
        ),
        EngineReport::Refused(Refusal::NotEmitted) => {
            let diagnostic = Diagnostic {
                code: CatalogCode::new(
                    "unsupported_projection",
                    "unsupported-requested-capability",
                ),
                payload: Payload::NotEmitted {
                    kind,
                    backend: backend.clone(),
                },
            };
            disposed(DispositionKind::Unsupported, diagnostic, backend)
        }
        EngineReport::Refused(Refusal::InvalidOptionValue { option, value }) => disposed(
            DispositionKind::InvalidRequest,
            Diagnostic {
                code: CatalogCode::new("invalid_capability", "invalid-option-value"),
                payload: Payload::OptionValue {
                    request_index: index,
                    backend: backend.clone(),
                    option,
                    value,
                },
            },
            backend,
        ),
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
        EngineReport::Refused(Refusal::RequiresFiniteHyperBound { bound, found }) => disposed(
            DispositionKind::RequiresBound,
            Diagnostic {
                code: CatalogCode::new("requires_bound", "requires-bound"),
                payload: Payload::FiniteHyperBound { bound, found },
            },
            backend,
        ),
    }
}

#[cfg(test)]
mod tests {
    use filament_exact::CancelCause;
    use fsl_foundation::bound::DomainKind;
    use fsl_route::descriptor_from_manifest;
    use ix_trace_rs::trace;

    use super::*;

    fn engine(id: &str, kind: Capability) -> Engine {
        Engine::new(
            descriptor_from_manifest(
                Candidate::new(BackendId::new(id)),
                ProviderOrigin::Linked,
                [(Some(kind.to_wire()), Some(Mode::Bounded.to_wire()))],
                fsl_route::ManifestMembers {
                    domains: Some(&[DomainKind::Population.to_wire()]),
                    fairness_kinds: Some(fsl_route::FairnessKindsMember::List(&[])),
                    evidence_kinds: None,
                    memory_models: None,
                    reductions: None,
                },
            )
            .expect("a well-formed test engine"),
        )
    }

    fn engine_all(id: &str) -> Engine {
        let advertisements = Capability::ALL
            .iter()
            .map(|kind| (Some(kind.to_wire()), Some(Mode::Bounded.to_wire())))
            .collect::<Vec<_>>();
        Engine::new(
            descriptor_from_manifest(
                Candidate::new(BackendId::new(id)),
                ProviderOrigin::Linked,
                advertisements,
                fsl_route::ManifestMembers {
                    domains: Some(&[DomainKind::Population.to_wire()]),
                    fairness_kinds: Some(fsl_route::FairnessKindsMember::List(&[])),
                    evidence_kinds: Some(&["statistical", "exact"]),
                    memory_models: None,
                    reductions: None,
                },
            )
            .expect("a well-formed test engine that advertises every capability"),
        )
    }

    #[trace("TC-2801", "FR-990-AC-1")]
    #[test]
    fn global_conformance_survives_analyze_document_settlement() {
        let index = RequestIndex::new(0);
        let terminal = TerminalRecord::proved(index, BackendId::new("conformance"), 1);
        let output = fsl_replay::ProtocolConformanceOutput {
            obligations: vec![fsl_replay::ConformanceObligationOutput {
                obligation: serde_json::json!({"kind": "closure"}),
                verdict: "satisfied".to_owned(),
                cause: None,
                basis: "explored".to_owned(),
                witness: None,
            }],
            closure: fsl_replay::ConformanceClosureOutput {
                state: "closed".to_owned(),
                premises: vec![serde_json::json!({"kind": "execution"})],
                producer: "engine".to_owned(),
            },
            global: fsl_replay::ConformanceGlobalOutput {
                label: "proved".to_owned(),
                basis: "closed-scope".to_owned(),
                complete: true,
                cause: None,
            },
            max_messages: None,
            projections: Some(vec![fsl_replay::ConformanceProjectionOutput {
                role: "Buyer".to_owned(),
                label: "proved".to_owned(),
                counterexample: None,
            }]),
            recording: None,
            replay: None,
        };
        let document = analyze_document_routed_with_conformance(
            &[terminal],
            &[],
            &std::collections::BTreeMap::new(),
            &[(index, output)].into_iter().collect(),
        )
        .expect("the settled item serializes");
        let wire: serde_json::Value =
            serde_json::from_slice(&document.to_bytes().expect("the document encodes"))
                .expect("the document is JSON");

        assert_eq!(wire["items"][0]["result"], "proved");
        assert_eq!(wire["items"][0]["obligations"][0]["verdict"], "satisfied");
        assert_eq!(wire["items"][0]["closure"]["state"], "closed");
        assert_eq!(wire["items"][0]["global"]["complete"], true);
        assert_eq!(wire["items"][0]["projections"][0]["role"], "Buyer");
    }

    fn item<'a>(
        backend: &str,
        kind: Capability,
        family: DeclarationFamily,
        ran: &'a std::cell::Cell<bool>,
    ) -> AnalyzeItem<'a> {
        AnalyzeItem::new(
            RequestIndex::new(0),
            kind,
            Some(family),
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

    #[trace("TC-1302", "FR-507-AC-10")]
    #[test]
    fn inapplicable_claim_kind_overrides_the_handed_disposition_without_running() {
        let dispositions = [
            (
                Disposition::Supported(Candidate::new(BackendId::new("test-all"))),
                None,
            ),
            (
                Disposition::Supported(Candidate::new(BackendId::new("test-missing"))),
                None,
            ),
            (
                Disposition::Supported(Candidate::new(BackendId::new("test-vv"))),
                None,
            ),
            (Disposition::RequiresBound, None),
            (Disposition::Unsupported, None),
            (
                Disposition::InvalidRequest,
                Some(Diagnostic::new(CatalogCode::new(
                    "invalid_capability",
                    "absent-extent",
                ))),
            ),
            (
                Disposition::InvalidRequest,
                Some(Diagnostic::new(NegotiationOutcome::AmbiguousBackend.code())),
            ),
        ];

        let mut engines = Engines::default();
        engines.with(engine_all("test-all")).expect("test-all");
        engines
            .with(engine("test-vv", Capability::ValueValidity))
            .expect("test-vv");
        for (disposition, handed_diagnostic) in dispositions {
            let ran = std::cell::Cell::new(false);
            let item = item(
                "test-all",
                Capability::FiniteReplay,
                DeclarationFamily::StateModel,
                &ran,
            )
            .disposed(disposition, handed_diagnostic);
            let record = analyze_items(&engines, vec![item], &Cancel::new())
                .into_iter()
                .next()
                .expect("one requested item has one result");
            let Settlement::Disposed(disposed) = record.settlement else {
                panic!("inapplicable item must settle by disposition");
            };
            assert_eq!(disposed.kind, DispositionKind::InvalidRequest);
            let diagnostic = disposed.diagnostic.expect("inapplicable-kind diagnostic");
            assert_eq!(
                diagnostic.code,
                CatalogCode::new("invalid_capability", "inapplicable-kind")
            );
            assert_eq!(
                diagnostic.payload,
                Payload::Inapplicable {
                    request_index: RequestIndex::new(0),
                    capability: Capability::FiniteReplay,
                    family: Some(DeclarationFamily::StateModel),
                }
            );
            assert!(!ran.get(), "an inapplicable item must not run an engine");
        }
    }

    #[trace("TC-1302", "FR-507-AC-1", "FR-057-AC-11")]
    #[test]
    fn every_capability_family_pair_is_admitted_or_refused_before_engine_work() {
        let families = [
            DeclarationFamily::Value,
            DeclarationFamily::StateModel,
            DeclarationFamily::TemporalTrace,
            DeclarationFamily::RelationClaim,
            DeclarationFamily::Protocol,
            DeclarationFamily::Probabilistic,
            DeclarationFamily::RealTime,
        ];
        let mut expected_runs = 0;
        let ran = std::cell::Cell::new(0);
        let mut index = 0;
        let items = Capability::ALL
            .iter()
            .copied()
            .flat_map(|kind| families.into_iter().map(move |family| (kind, family)))
            .map(|(kind, family)| {
                let request_index = RequestIndex::new(index);
                index += 1;
                if kind.applies_to_declaration(family) {
                    expected_runs += 1;
                }
                AnalyzeItem::new(
                    request_index,
                    kind,
                    Some(family),
                    Mode::Bounded,
                    "test-all",
                    |_| {
                        ran.set(ran.get() + 1);
                        EngineReport::Refused(Refusal::Failed(CatalogCode::new(
                            "runtime_invariant",
                            "established-invariant-broken",
                        )))
                    },
                )
            })
            .collect();
        let mut engines = Engines::default();
        engines.with(engine_all("test-all")).expect("one engine");
        let records = analyze_items(&engines, items, &Cancel::new());

        assert_eq!(records.len(), 12 * 7);
        assert_eq!(ran.get(), expected_runs);
        assert_eq!(expected_runs, 14);
        for record in &records {
            let Settlement::Disposed(disposed) = &record.settlement else {
                continue;
            };
            assert_eq!(disposed.kind, DispositionKind::InvalidRequest);
            let Some(Diagnostic {
                code,
                payload:
                    Payload::Inapplicable {
                        request_index,
                        capability,
                        family,
                    },
            }) = disposed.diagnostic.as_ref()
            else {
                panic!("inapplicable pair has its typed family diagnostic");
            };
            assert_eq!(*request_index, record.request_index);
            assert!(!capability
                .applies_to_declaration(family.expect("this matrix row has a known family")));
            assert_eq!(code.cause(), "inapplicable-kind");
        }
    }

    /// Items 0 (`operation-contract`, supported) and 1 (`finite-replay`,
    /// inapplicable to a state claim), with item 1's requiredness set.
    fn aggregate_pair<'a>(
        calls: &'a std::cell::Cell<u32>,
        second_required: bool,
    ) -> Vec<AnalyzeItem<'a>> {
        let work = move |_: &Cancel| {
            calls.set(calls.get() + 1);
            EngineReport::Refused(Refusal::Failed(CatalogCode::new(
                "runtime_invariant",
                "established-invariant-broken",
            )))
        };
        vec![
            AnalyzeItem::new(
                RequestIndex::new(0),
                Capability::OperationContract,
                Some(DeclarationFamily::StateModel),
                Mode::Bounded,
                "test-all",
                work,
            )
            .required(true),
            AnalyzeItem::new(
                RequestIndex::new(1),
                Capability::FiniteReplay,
                Some(DeclarationFamily::StateModel),
                Mode::Bounded,
                "test-all",
                work,
            )
            .required(second_required),
        ]
    }

    #[trace("TC-1302", "FR-507-AC-5")]
    #[test]
    fn an_inapplicable_required_item_keeps_its_position_and_skips_engine_work() {
        let calls = std::cell::Cell::new(0);
        let items = aggregate_pair(&calls, true);
        let requirements: Vec<_> = items.iter().map(AnalyzeItem::requirement).collect();
        let mut engines = Engines::default();
        engines.with(engine_all("test-all")).expect("one engine");

        let records = analyze_items(&engines, items, &Cancel::new());

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].request_index, RequestIndex::new(0));
        assert!(matches!(&records[0].settlement, Settlement::Terminal(_)));
        assert_eq!(records[1].request_index, RequestIndex::new(1));
        let Settlement::Disposed(disposed) = &records[1].settlement else {
            panic!("the required inapplicable item is declined");
        };
        assert_eq!(disposed.kind, DispositionKind::InvalidRequest);
        assert_eq!(
            disposed
                .diagnostic
                .as_ref()
                .map(|diagnostic| diagnostic.code),
            Some(CatalogCode::new("invalid_capability", "inapplicable-kind"))
        );
        assert_eq!(calls.get(), 1);
        assert_eq!(
            aggregate(&requirements, &records),
            Ok(Aggregate::Unavailable {
                request_index: RequestIndex::new(1)
            })
        );
    }

    #[trace("TC-1302", "FR-507-AC-5")]
    #[test]
    fn an_inapplicable_optional_item_leaves_the_aggregate_attainable() {
        let calls = std::cell::Cell::new(0);
        let items = aggregate_pair(&calls, false);
        let requirements: Vec<_> = items.iter().map(AnalyzeItem::requirement).collect();
        let mut engines = Engines::default();
        engines.with(engine_all("test-all")).expect("one engine");

        let records = analyze_items(&engines, items, &Cancel::new());

        assert_eq!(calls.get(), 1);
        assert_eq!(
            aggregate(&requirements, &records),
            Ok(Aggregate::Attainable)
        );
    }

    #[trace("TC-1302", "FR-507-AC-5")]
    #[test]
    fn the_aggregate_rejects_duplicate_missing_and_unexpected_indices() {
        let calls = std::cell::Cell::new(0);
        let items = aggregate_pair(&calls, false);
        let requirements: Vec<_> = items.iter().map(AnalyzeItem::requirement).collect();
        let mut engines = Engines::default();
        engines.with(engine_all("test-all")).expect("one engine");
        let mut records = analyze_items(&engines, items, &Cancel::new());
        let last = records.pop().expect("two records");

        assert_eq!(
            aggregate(&requirements, &records),
            Err(AggregateGap::Missing(RequestIndex::new(1)))
        );
        assert_eq!(
            aggregate(&requirements[..1], &[records.remove(0), last]),
            Err(AggregateGap::Unexpected(RequestIndex::new(1)))
        );
        let doubled = [requirements[0], requirements[0]];
        assert_eq!(
            aggregate(&doubled, &[]),
            Err(AggregateGap::Duplicate(RequestIndex::new(0)))
        );
    }

    #[trace("TC-1302", "FR-507-AC-3")]
    #[test]
    fn protocol_kinds_apply_only_to_protocol_claims() {
        let protocol_kinds = [
            Capability::FiniteReplay,
            Capability::GlobalConformance,
            Capability::Monitorability,
            Capability::LocalProjection,
            Capability::Refinement,
            Capability::Realizability,
            Capability::Composition,
        ];
        let families = [
            DeclarationFamily::Protocol,
            DeclarationFamily::StateModel,
            DeclarationFamily::Value,
            DeclarationFamily::TemporalTrace,
        ];
        let calls = std::cell::Cell::new(0);
        let mut request_index = 0;
        let items = protocol_kinds
            .into_iter()
            .flat_map(|kind| families.map(move |family| (kind, family)))
            .map(|(kind, family)| {
                let index = RequestIndex::new(request_index);
                request_index += 1;
                AnalyzeItem::new(index, kind, Some(family), Mode::Bounded, "test-all", |_| {
                    calls.set(calls.get() + 1);
                    EngineReport::Refused(Refusal::Failed(CatalogCode::new(
                        "runtime_invariant",
                        "established-invariant-broken",
                    )))
                })
            })
            .collect();
        let mut engines = Engines::default();
        engines.with(engine_all("test-all")).expect("one engine");

        let records = analyze_items(&engines, items, &Cancel::new());

        assert_eq!(records.len(), 7 * 4);
        assert_eq!(calls.get(), 7);
        for record in &records {
            let Settlement::Disposed(disposed) = &record.settlement else {
                continue;
            };
            assert_eq!(disposed.kind, DispositionKind::InvalidRequest);
            assert_eq!(
                disposed
                    .diagnostic
                    .as_ref()
                    .map(|diagnostic| diagnostic.code),
                Some(CatalogCode::new("invalid_capability", "inapplicable-kind"))
            );
        }
    }

    #[trace("TC-1302", "TC-115", "FR-507-AC-4", "FR-057-AC-11")]
    #[test]
    fn probabilistic_and_schedulability_kinds_apply_only_to_their_claim_families() {
        let cases = [
            (
                Capability::ProbabilisticSatisfaction,
                DeclarationFamily::Probabilistic,
            ),
            (
                Capability::ProbabilisticSatisfaction,
                DeclarationFamily::StateModel,
            ),
            (Capability::Schedulability, DeclarationFamily::RealTime),
            (Capability::Schedulability, DeclarationFamily::StateModel),
        ];
        let calls = std::cell::Cell::new(0);
        let items = cases
            .into_iter()
            .enumerate()
            .map(|(position, (kind, family))| {
                AnalyzeItem::new(
                    RequestIndex::new(position),
                    kind,
                    Some(family),
                    Mode::Bounded,
                    "test-all",
                    |_| {
                        calls.set(calls.get() + 1);
                        EngineReport::Refused(Refusal::Failed(CatalogCode::new(
                            "runtime_invariant",
                            "established-invariant-broken",
                        )))
                    },
                )
            })
            .collect();
        let mut engines = Engines::default();
        engines.with(engine_all("test-all")).expect("one engine");

        let records = analyze_items(&engines, items, &Cancel::new());

        assert_eq!(records.len(), cases.len());
        assert_eq!(calls.get(), 2);
        assert!(matches!(&records[0].settlement, Settlement::Terminal(_)));
        assert!(matches!(&records[2].settlement, Settlement::Terminal(_)));
        for index in [1, 3] {
            let Settlement::Disposed(disposed) = &records[index].settlement else {
                panic!("a state-model claim rejects this capability kind");
            };
            assert_eq!(disposed.kind, DispositionKind::InvalidRequest);
            assert_eq!(
                disposed
                    .diagnostic
                    .as_ref()
                    .map(|diagnostic| diagnostic.code),
                Some(CatalogCode::new("invalid_capability", "inapplicable-kind"))
            );
        }
    }

    #[trace("TC-1302", "FR-507-AC-9")]
    #[test]
    fn an_unresolved_claim_family_is_inapplicable_for_every_kind() {
        for kind in Capability::ALL.iter().copied() {
            let record = analyze_items(
                &Engines::default(),
                vec![AnalyzeItem::unadapted(RequestIndex::new(7), kind, None)],
                &Cancel::new(),
            )
            .remove(0);
            let Settlement::Disposed(disposed) = record.settlement else {
                panic!("a claim with no resolved family cannot run");
            };
            assert_eq!(disposed.kind, DispositionKind::InvalidRequest);
            assert_eq!(
                disposed.diagnostic.expect("typed refusal").payload,
                Payload::Inapplicable {
                    request_index: RequestIndex::new(7),
                    capability: kind,
                    family: None,
                }
            );
        }
    }

    fn disposed(records: &[AnalyzeRecord]) -> &Disposed {
        match &records[0].settlement {
            Settlement::Disposed(disposed) => disposed,
            Settlement::Terminal(terminal) => panic!("expected a disposition: {terminal:?}"),
        }
    }

    /// FR-489-AC-5: linked registrations contain exactly the descriptors
    /// owned by EN-1, EN-4, EN-5, EN-6, EN-7, value-checker,
    /// contract-checker and protocol-analysis, with each engine's identity
    /// and advertised kinds intact.
    #[trace("TC-1272", "FR-489-AC-5")]
    #[test]
    fn linked_engine_registrations_expose_the_owned_descriptors() {
        let engines = Engines::fsl().expect("FSL's linked engine descriptors are admitted");
        let registrations: Vec<_> = engines.registrations().collect();

        assert_eq!(
            registrations
                .iter()
                .map(|descriptor| descriptor.id().as_str())
                .collect::<Vec<_>>(),
            [
                "en-1",
                "en-4",
                "en-5",
                "en-6",
                "value-checker",
                "contract-checker",
                "protocol-analysis",
                "en-7"
            ]
        );
        let [en1, en4, en5, _en6, value, contract, protocol, en7] = registrations.as_slice() else {
            panic!("the linked engine set contains the eight FR-489-AC-5 descriptors");
        };
        assert_eq!(
            en7.advertises().collect::<Vec<_>>(),
            [(Capability::Schedulability, Mode::Unbounded)]
        );
        assert!(registrations
            .iter()
            .all(|descriptor| descriptor.origin() == ProviderOrigin::Linked));
        assert!(en1
            .advertises()
            .any(|(kind, _)| kind == Capability::TemporalSatisfaction));
        assert!(en4
            .advertises()
            .any(|(kind, _)| kind == Capability::ProbabilisticSatisfaction));
        assert!(en5
            .advertises()
            .any(|(kind, _)| kind == Capability::ProbabilisticSatisfaction));
        assert!(value
            .advertises()
            .any(|(kind, _)| kind == Capability::ValueValidity));
        assert!(contract
            .advertises()
            .any(|(kind, _)| kind == Capability::OperationContract));
        let mut kinds: Vec<_> = protocol.advertises().map(|(kind, _)| kind).collect();
        kinds.sort_by_key(|kind| kind.to_wire());
        assert_eq!(
            kinds,
            [
                Capability::FiniteReplay,
                Capability::GlobalConformance,
                Capability::LocalProjection,
                Capability::Realizability,
            ]
        );
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
        let records = analyze_items(
            &engines,
            vec![item(
                "test-a",
                Capability::OperationContract,
                DeclarationFamily::StateModel,
                &ran,
            )],
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
        let records = analyze_items(
            &engines,
            vec![item(
                "test-a",
                Capability::TemporalSatisfaction,
                DeclarationFamily::TemporalTrace,
                &ran,
            )],
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
        let records = analyze_items(
            &engines,
            vec![item(
                "test-c",
                Capability::ValueValidity,
                DeclarationFamily::Value,
                &ran,
            )],
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
        let ambiguous = Diagnostic::new(NegotiationOutcome::AmbiguousBackend.code());
        let records = analyze_items(
            &engines,
            vec![
                item(
                    "test-a",
                    Capability::ValueValidity,
                    DeclarationFamily::Value,
                    &ran,
                )
                .disposed(Disposition::InvalidRequest, Some(ambiguous.clone())),
                item(
                    "test-a",
                    Capability::ValueValidity,
                    DeclarationFamily::Value,
                    &ran,
                ),
            ],
            &Cancel::new(),
        );
        let settled = disposed(&records);
        assert_eq!(settled.kind, DispositionKind::InvalidRequest);
        assert_eq!(settled.diagnostic.as_ref(), Some(&ambiguous));
        assert!(records[1].terminal().is_some());
    }

    /// FR-286 and FSpec FR-331: an item a disposition settles carries its
    /// diagnostic as `{code, cause, detail}`, `detail` keyed by FR-272 payload
    /// names. An `unsupported` item has no `cause`; an `invalid-request` item
    /// has no `result` and no `cause` (FR-286, FR-621).
    #[trace("TC-763", "FR-281-AC-11")]
    #[test]
    fn dispositions_write_their_diagnostic_detail_into_the_item() {
        use serde_json::json;
        let mut engines = Engines::default();
        engines
            .with(engine("test-a", Capability::OperationContract))
            .expect("first");
        let ran = std::cell::Cell::new(false);
        let written = |item: AnalyzeItem<'_>| {
            let records = analyze_items(&engines, vec![item], &Cancel::new());
            let records = terminal_records(records);
            serde_json::to_value(OutcomeItem::from_terminal(&records[0])).expect("serializes")
        };

        let unbounded = written(refusing(|| Refusal::RequiresBound));
        assert_eq!(unbounded["result"], "unsupported");
        assert!(unbounded.get("cause").is_none(), "{unbounded}");
        assert_eq!(
            unbounded["diagnostic"],
            json!({
                "code": "unsupported_projection",
                "cause": "unbounded-extent",
                "detail": {
                    "kind": "operation-contract",
                    "candidate": "test-a",
                    "modes": ["bounded"],
                },
            })
        );

        let inapplicable = written(item(
            "test-a",
            Capability::FiniteReplay,
            DeclarationFamily::StateModel,
            &ran,
        ));
        assert!(inapplicable.get("result").is_none(), "{inapplicable}");
        assert!(inapplicable.get("cause").is_none(), "{inapplicable}");
        assert_eq!(
            inapplicable["diagnostic"],
            json!({
                "code": "invalid_capability",
                "cause": "inapplicable-kind",
                "detail": {
                    "request_index": 0,
                    "capability": "finite-replay",
                    "family": "state-model",
                },
            })
        );

        let inconsistent = written(item(
            "test-a",
            Capability::TemporalSatisfaction,
            DeclarationFamily::TemporalTrace,
            &ran,
        ));
        assert!(inconsistent.get("result").is_none(), "{inconsistent}");
        assert!(inconsistent.get("cause").is_none(), "{inconsistent}");
        assert_eq!(
            inconsistent["diagnostic"]["detail"],
            json!({ "candidates": ["test-a"] })
        );

        let unknown = written(item(
            "test-c",
            Capability::OperationContract,
            DeclarationFamily::StateModel,
            &ran,
        ));
        assert_eq!(
            unknown["diagnostic"]["detail"],
            json!({ "backend": "test-c" })
        );
        assert!(!ran.get());
    }

    fn refusing<'a>(refusal: impl Fn() -> Refusal + 'a) -> AnalyzeItem<'a> {
        AnalyzeItem::new(
            RequestIndex::new(0),
            Capability::OperationContract,
            Some(DeclarationFamily::StateModel),
            Mode::Bounded,
            "test-a",
            move |_| EngineReport::Refused(refusal()),
        )
    }

    /// FR-281-AC-11: each engine refusal row settles its record naming the
    /// engine; an unbounded extent carries FSpec FR-272's payload (kind,
    /// candidate, advertised modes).
    #[trace("TC-763", "FR-281-AC-11")]
    #[test]
    fn engine_refusals_settle_by_the_refusal_table() {
        let mut engines = Engines::default();
        engines
            .with(engine("test-a", Capability::OperationContract))
            .expect("first");
        let run = |refusal: fn() -> Refusal| {
            analyze_items(&engines, vec![refusing(refusal)], &Cancel::new())
        };

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
                DeclineCode::Fsl(fsl_foundation::Code::UnavailableObservation),
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
        let records = analyze_items(
            &engines,
            vec![refusing(move || Refusal::Failed(wrong))],
            &Cancel::new(),
        );
        let terminal = records[0].terminal().expect("a terminal record");
        assert_eq!(terminal.value(), &TerminalValue::Failed);
        assert_eq!(terminal.diagnostic(), Some(wrong));
        assert_eq!(terminal.backend(), Some(&BackendId::new("test-a")));
    }
    /// FR-276-AC-4: the test engine settles the first item, then blocks on
    /// the second until the handle is cancelled from another thread. The
    /// first record is unchanged, the item still open settles `incomplete`
    /// with the cancellation cause, and a later item runs no work.
    #[trace("TC-757", "FR-276-AC-4")]
    #[test]
    fn a_cancel_settles_the_item_still_open_incomplete_and_keeps_the_settled_one() {
        fn failed() -> EngineReport {
            EngineReport::Refused(Refusal::Failed(CatalogCode::new(
                "runtime_invariant",
                "established-invariant-broken",
            )))
        }
        fn item<'a>(
            index: usize,
            work: impl FnOnce(&Cancel) -> EngineReport + 'a,
        ) -> AnalyzeItem<'a> {
            AnalyzeItem::new(
                RequestIndex::new(index),
                Capability::OperationContract,
                Some(DeclarationFamily::StateModel),
                Mode::Bounded,
                "test-all",
                work,
            )
        }
        let mut engines = Engines::default();
        engines.with(engine_all("test-all")).expect("one engine");
        let uncancelled = analyze_items(&engines, vec![item(0, |_| failed())], &Cancel::new());

        let cancel = Cancel::new();
        let (blocked, wait_for_block) = std::sync::mpsc::channel();
        let canceller = {
            let cancel = cancel.clone();
            std::thread::spawn(move || {
                wait_for_block
                    .recv_timeout(std::time::Duration::from_secs(10))
                    .expect("the second item blocked");
                cancel.cancel(CancelCause::Requested);
            })
        };
        let third_ran = std::cell::Cell::new(false);
        let items = vec![
            item(0, |_| failed()),
            item(1, move |cancel| {
                blocked.send(()).expect("the canceller waits");
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
                while !cancel.poll() {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "the handle was never cancelled"
                    );
                    std::thread::yield_now();
                }
                failed()
            }),
            item(2, |_| {
                third_ran.set(true);
                failed()
            }),
        ];
        let records = analyze_items(&engines, items, &cancel);
        canceller.join().expect("the canceller finished");

        let cancelled = Some(&TerminalValue::Incomplete(IncompleteCause::Cancelled {
            source: CancelCause::Requested,
        }));
        assert_eq!(records.len(), 3);
        assert_eq!(records[0].terminal(), uncancelled[0].terminal());
        assert_eq!(records[1].terminal().map(TerminalRecord::value), cancelled);
        assert_eq!(records[2].terminal().map(TerminalRecord::value), cancelled);
        assert!(!third_ran.get());
    }
    /// FR-276 Behavior: the operation reads the handle's `tripped()` cause. A
    /// handle cancelled after an item's last charge is not tripped, so that
    /// item's verdict stands.
    #[trace("TC-757", "FR-276-AC-4")]
    #[test]
    fn a_cancel_after_the_last_charge_leaves_the_verdict_standing() {
        let failed = || {
            EngineReport::Refused(Refusal::Failed(CatalogCode::new(
                "runtime_invariant",
                "established-invariant-broken",
            )))
        };
        let mut engines = Engines::default();
        engines.with(engine_all("test-all")).expect("one engine");
        let cancel = Cancel::new();
        let item = AnalyzeItem::new(
            RequestIndex::new(0),
            Capability::OperationContract,
            Some(DeclarationFamily::StateModel),
            Mode::Bounded,
            "test-all",
            |cancel: &Cancel| {
                assert!(!cancel.poll(), "the item's last charge sees no cancel");
                cancel.cancel(CancelCause::Requested);
                failed()
            },
        );
        let records = analyze_items(&engines, vec![item], &cancel);

        assert_eq!(cancel.cause(), Some(CancelCause::Requested));
        assert_eq!(cancel.tripped(), None);
        let expected = analyze_items(
            &engines,
            vec![AnalyzeItem::new(
                RequestIndex::new(0),
                Capability::OperationContract,
                Some(DeclarationFamily::StateModel),
                Mode::Bounded,
                "test-all",
                |_| failed(),
            )],
            &Cancel::new(),
        );
        assert_eq!(records[0].terminal(), expected[0].terminal());
    }
}

/// FR-418 and FR-235: the terminal value a task-product proof attempt
/// settles. `proved` only when the checker accepted the emitted certificate.
fn proof_value(
    proof: &task_product_item::TaskProof,
    outcome: &crate::zone_check::TaskProductSearchOutcome,
) -> Option<TerminalValue> {
    use task_product_item::ProofVerdict;
    let closed = matches!(
        outcome,
        crate::zone_check::TaskProductSearchOutcome::Stopped {
            cause: crate::zone_check::TaskProductSearchStop::RegionClosureRequired,
            ..
        }
    );
    Some(match &proof.verdict {
        ProofVerdict::Checked(fsl_replay::ZoneCertificateVerdict::Accepted) => {
            TerminalValue::Proved {
                basis: fsl_replay::ProofBasis::ZoneCertified,
                certification: fsl_replay::Certification::Certified,
            }
        }
        ProofVerdict::Checked(fsl_replay::ZoneCertificateVerdict::Rejected(rejection)) => {
            TerminalValue::Inconclusive(fsl_replay::InconclusiveCause::CertificateRejected {
                rule: rejection.rule,
                at: rejection.at.clone(),
            })
        }
        ProofVerdict::Checked(fsl_replay::ZoneCertificateVerdict::Stopped(cause)) => {
            TerminalValue::Incomplete(cause.clone())
        }
        // The symbolic pass's own stops and faults settle only where the
        // concrete search did not decide, or closed.
        ProofVerdict::Stopped(cause) if closed => TerminalValue::Incomplete(cause.clone()),
        ProofVerdict::Fault if closed => TerminalValue::Failed,
        ProofVerdict::Stopped(_) | ProofVerdict::Fault | ProofVerdict::Uncertified => return None,
    })
}

#[cfg(test)]
mod proof_value_tests {
    use super::task_product_item::{ProofVerdict, TaskProof};
    use super::{proof_value, TerminalValue};
    use crate::zone_check::{TaskProductSearchOutcome, TaskProductSearchStop};
    use filament_exact::CancelCause;
    use fsl_replay::{
        CertificateLocus, CertificateRejection, CertificateRule, IncompleteCause,
        InconclusiveCause, ModelCheckLimit, ZoneCertificateVerdict,
    };

    fn proof(verdict: ProofVerdict) -> TaskProof {
        TaskProof {
            verdict,
            certificate: None,
        }
    }

    /// The concrete search's outcome: closed (`RegionClosureRequired`), or
    /// stopped at its work limit.
    fn outcome(closed: bool) -> TaskProductSearchOutcome {
        TaskProductSearchOutcome::Stopped {
            cause: if closed {
                TaskProductSearchStop::RegionClosureRequired
            } else {
                TaskProductSearchStop::WorkLimit(5)
            },
            frontier: Vec::new(),
            expanded: 0,
            transitions: 0,
        }
    }

    fn limit() -> IncompleteCause {
        IncompleteCause::LimitReached {
            limit: ModelCheckLimit::ZoneMaxCheckWork,
            value: 7,
        }
    }

    /// FR-418: `proved` only for an accepted certificate, whatever the
    /// concrete search did.
    #[test]
    fn an_accepted_certificate_settles_proved() {
        for closed in [true, false] {
            assert!(matches!(
                proof_value(
                    &proof(ProofVerdict::Checked(ZoneCertificateVerdict::Accepted)),
                    &outcome(closed)
                ),
                Some(TerminalValue::Proved { .. })
            ));
        }
    }

    /// A rejected certificate settles `inconclusive` with the checker's rule
    /// and locus; the checker's own stop settles `incomplete` with its cause.
    #[test]
    fn a_rejection_and_a_checker_stop_settle_with_the_checkers_cause() {
        let at = CertificateLocus::ZoneNode { node: 3 };
        let rejected = ZoneCertificateVerdict::Rejected(CertificateRejection {
            rule: CertificateRule::SuccessorMissing,
            at: at.clone(),
        });
        for closed in [true, false] {
            assert_eq!(
                proof_value(
                    &proof(ProofVerdict::Checked(rejected.clone())),
                    &outcome(closed)
                ),
                Some(TerminalValue::Inconclusive(
                    InconclusiveCause::CertificateRejected {
                        rule: CertificateRule::SuccessorMissing,
                        at: at.clone(),
                    }
                ))
            );
            assert_eq!(
                proof_value(
                    &proof(ProofVerdict::Checked(ZoneCertificateVerdict::Stopped(
                        limit()
                    ))),
                    &outcome(closed)
                ),
                Some(TerminalValue::Incomplete(limit()))
            );
        }
    }

    /// The symbolic pass's own stop (a work limit or a cancel) and its fault
    /// settle only once the concrete search has closed; before that the
    /// concrete search's own result stands.
    #[test]
    fn a_pass_stop_or_fault_settles_only_when_the_search_closed() {
        let cancelled = IncompleteCause::Cancelled {
            source: CancelCause::Requested,
        };
        for stop in [limit(), cancelled] {
            assert_eq!(
                proof_value(&proof(ProofVerdict::Stopped(stop.clone())), &outcome(true)),
                Some(TerminalValue::Incomplete(stop.clone()))
            );
            assert_eq!(
                proof_value(&proof(ProofVerdict::Stopped(stop)), &outcome(false)),
                None
            );
        }
        assert_eq!(
            proof_value(&proof(ProofVerdict::Fault), &outcome(true)),
            Some(TerminalValue::Failed)
        );
        assert_eq!(
            proof_value(&proof(ProofVerdict::Fault), &outcome(false)),
            None
        );
    }

    /// A set no certificate covers leaves the concrete search's result (the
    /// closed gate settles `certificate-unchecked` elsewhere).
    #[test]
    fn an_uncertified_set_leaves_the_concrete_result() {
        for closed in [true, false] {
            assert_eq!(
                proof_value(&proof(ProofVerdict::Uncertified), &outcome(closed)),
                None
            );
        }
    }
}
