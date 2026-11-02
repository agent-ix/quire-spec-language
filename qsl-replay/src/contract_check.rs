// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-483 and FR-485: the native evaluation of one case for the
//! `operation-contract` check arm, and the replay of the arm's
//! counterexample.
//!
//! The arm decides a claim over a checked package's model by evaluating its
//! form at every *case*: an initial state for the initial-state obligations,
//! and a pre-state of the declared types for the others (FR-485 "Common
//! premises"). This module owns that evaluation, so the arm in `qsl-analyze`
//! and the replay of the arm's counterexample run one code path:
//!
//! * [`select_contract`] reads the claim's form from its node and
//!   occurrence (FR-485 "Forms") and refuses a claim that selects none, or a
//!   bound item that reads an element with no abstraction binding;
//! * [`ContractTarget::evaluate`] evaluates one [`ContractCase`];
//! * [`ContractTarget::replay`] re-evaluates a counterexample's case and
//!   returns a [`ReproducedContractViolation`] only when the same violation
//!   recurs, the only evidence
//!   [`crate::TerminalRecord::refuted_by_contract`] accepts.
//!
//! The arm holds no implementation engine (QSpec FR-485 "The oracle"): an
//! item whose operation has an FR-353 implementation binding is not run, and
//! a `frame` item with none has no contract-level reading. Every
//! counterexample it writes has `bound` `false` and is replayed on the
//! recorded values by the native evaluator. A step relation's
//! operation-contract item exists only when every execution is bound
//! (ADR-023 XC-1), so the `step-relation` form always settles
//! `no-implementation-engine`.

use qsl_eval::simulation::model::{
    ClauseOutcome, ContractClause, ContractClauseKind, DeclaredStates, ModelFinding, ModelSimState,
    ModelSystem, OperationCases,
};
use qsl_eval::simulation::ExpansionStop;
use qsl_eval::temporal::UndefinedCause;
use qsl_forms::StateClauseKind;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_package::CheckedPackage;
use qsl_semantics::check::{
    AbstractionKey, Capability, CheckedAbstractionRelation, CheckedRefinement, OperationKey,
};
use qsl_semantics::model::state::ModelState;
use quire_contract_model::CheckedOccurrenceRole;
use quire_exact::{EffectiveId, Identifier, NodeKey, Value};
use quire_semantic_value::catalog::InternalFault;
use quire_semantic_value::model_transition::ModelTransition;
use quire_semantic_value::step_simulation::StepObligation;

use crate::refinement::{
    check_initial, check_step, initial_history, map_state, ConcreteStep, MappingFailure,
    RefinementPosition, StepVerdict, Undetermined,
};

/// The claim form a target selects (QSpec FR-485 "Forms").
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ContractForm {
    /// An invariant clause: `initial-state` and `preservation`.
    Invariant,
    /// A `pre` or `post` clause.
    Clause,
    /// An operation's frame.
    Frame,
    /// A step relation every execution of which is bound.
    StepRelation,
    /// A refinement's per-step simulation obligation.
    Simulation,
}

impl ContractForm {
    /// The form's wire spelling (QSpec FR-485 "Forms").
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Invariant => "invariant",
            Self::Clause => "clause",
            Self::Frame => "frame",
            Self::StepRelation => "step-relation",
            Self::Simulation => "simulation",
        }
    }
}

/// Why the arm does not decide an item (QSpec FR-483 `unsupported`, whose
/// `detail.reason` names it).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ContractCheckUnsupported {
    /// The item's operation has an implementation binding, and the arm
    /// holds no implementation engine (FR-485 "Clause", "Frame").
    NoImplementationEngine,
    /// A `frame` item whose operation has no implementation binding: the
    /// contract relation assumes the frame (FR-485 "Frame"); or a
    /// `simulation` item over a refinement that declares a hidden field, a
    /// history field or an `any` row, which has no per-step reading
    /// (FR-485 "Simulation", FR-485-AC-14).
    NoContractLevelReading,
    /// The model's pre-states hold a relationship extent, which the arm does
    /// not range over.
    RelationshipState,
    /// The model holds a clock member, which has no value domain here.
    ClockMember,
    /// A field of the model has no finite domain.
    FieldDomain,
}

impl ContractCheckUnsupported {
    /// The reason's wire spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoImplementationEngine => "no-implementation-engine",
            Self::NoContractLevelReading => "no-contract-level-reading",
            Self::RelationshipState => "relationship-state-not-enumerable",
            Self::ClockMember => "clock-member-not-enumerable",
            Self::FieldDomain => "field-domain-not-enumerable",
        }
    }
}

/// Why a claim selects no checkable target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContractSelection {
    /// The claim's node and occurrence select none of the forms, or a node
    /// the package does not hold: `declined` `invalid-input` (QSpec FR-483
    /// "The check input").
    NoForm,
    /// The package lacks a node the form requires: `declined`
    /// `incomplete-input`.
    MissingNode,
    /// A bound item reads an element the FR-353 abstraction relation does
    /// not bind: `declined` `incomplete-input`, `missing_declaration`/
    /// `missing-name` naming the element (QSpec FR-485 "The oracle").
    Unbound(AbstractionKey),
    /// A broken invariant of the checked package; the item settles `failed`.
    Fault(InternalFault),
}

/// What a claim selected: the form's target in the package.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Target {
    /// An invariant clause.
    Invariant {
        clause: NodeKey,
        context: EffectiveId,
    },
    /// A precondition: its definedness under premises 1 to 3.
    Precondition {
        clause: NodeKey,
        context: EffectiveId,
    },
    /// A postcondition: its definedness under the pre-state and frame
    /// premises.
    Postcondition {
        clause: NodeKey,
        context: EffectiveId,
        operation: (EffectiveId, String),
    },
    /// An operation's frame.
    Frame,
    /// A rule of a refinement that declares a hidden field, a history field
    /// or an `any` row: it has no per-step reading (QSpec FR-485
    /// "Simulation", FR-485-AC-14).
    NotStepSimulable,
    /// A bound step relation; it is always bound, so never evaluated.
    StepRelation,
    /// A refinement's initial-state rule (ordinal 0) or step row `source`.
    Simulation {
        refinement: usize,
        row: Option<usize>,
    },
}

/// A claim's form and target over one checked package (QSpec FR-485
/// "Forms").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractClaim {
    claim: OccurrenceKey,
    form: ContractForm,
    target: Target,
    /// Whether the item's operation has an implementation binding.
    bound: bool,
}

impl ContractClaim {
    /// The claim's `{node, occurrence}`, the locus of every `where`.
    pub fn claim(&self) -> &OccurrenceKey {
        &self.claim
    }

    /// The claim's form.
    pub fn form(&self) -> ContractForm {
        self.form
    }

    /// Whether the item's operation has an FR-353 implementation binding.
    pub fn bound(&self) -> bool {
        self.bound
    }

    /// Why the arm does not decide the claim, when its form alone says so:
    /// a bound item (no implementation engine); an unbound frame, or a rule
    /// of a refinement that declares a hidden field, a history field or an
    /// `any` row (no contract-level reading, QSpec FR-485-AC-6 and
    /// FR-485-AC-14). Read before any model system is opened, so no check
    /// runs and no counterexample is recorded.
    pub fn refusal(&self) -> Option<ContractCheckUnsupported> {
        if self.bound {
            return Some(ContractCheckUnsupported::NoImplementationEngine);
        }
        match self.target {
            Target::Frame => Some(ContractCheckUnsupported::NoContractLevelReading),
            Target::NotStepSimulable => Some(ContractCheckUnsupported::NoContractLevelReading),
            _ => None,
        }
    }
}

fn fault(invariant: &'static str) -> InternalFault {
    InternalFault::new("contract-check", invariant)
}

/// The FR-353 key of the operation `name` that `declaring` declares.
fn operation_key(
    package: &CheckedPackage,
    declaring: EffectiveId,
    name: &str,
) -> Result<OperationKey, ContractSelection> {
    let graph = package.graph();
    Ok(OperationKey {
        declaring: graph
            .object_type_key(declaring)
            .cloned()
            .ok_or_else(|| ContractSelection::Fault(fault("operation-owner-is-admitted")))?,
        operation: Identifier::new(name)
            .map_err(|_| ContractSelection::Fault(fault("operation-name-is-an-identifier")))?,
    })
}

/// Whether `relation` binds the operation `key`, and, when it does, the
/// first of `elements` it leaves unbound.
fn binding(
    relation: Option<&CheckedAbstractionRelation>,
    key: &OperationKey,
    elements: impl IntoIterator<Item = AbstractionKey>,
) -> Result<bool, ContractSelection> {
    let Some(relation) = relation else {
        return Ok(false);
    };
    if relation.get(&AbstractionKey::Frame(key.clone())).is_none() {
        return Ok(false);
    }
    for element in elements {
        if relation.get(&element).is_none() {
            return Err(ContractSelection::Unbound(element));
        }
    }
    Ok(true)
}

/// The form and target of the claim at `claim` over `package` (QSpec
/// FR-485 "Forms").
///
/// # Errors
///
/// [`ContractSelection`] names why the claim selects no checkable target.
pub fn select_contract(
    package: &CheckedPackage,
    claim: &OccurrenceKey,
) -> Result<ContractClaim, ContractSelection> {
    let graph = package.graph();
    let role = qsl_package::occurrence_role(claim.origin().role().as_str())
        .ok_or(ContractSelection::NoForm)?;
    // A refinement node is minted beside the semantic graph (FR-375), so a
    // `generated` claim is matched by occurrence, not resolved.
    let node = if role == CheckedOccurrenceRole::Generated {
        None
    } else {
        Some(
            graph
                .semantic_graph()
                .node(claim.node().clone())
                .ok_or(ContractSelection::NoForm)?
                .key(),
        )
    };
    let relation = package.abstraction();
    let selected = |form, target, bound| ContractClaim {
        claim: claim.clone(),
        form,
        target,
        bound,
    };
    match (role, node.as_ref()) {
        (CheckedOccurrenceRole::Claim, Some(node))
            if graph
                .step_relations()
                .iter()
                .any(|relation| relation.identity() == Some(node.clone())) =>
        {
            step_relation(package, claim, node.clone())
        }
        (CheckedOccurrenceRole::Claim, Some(node)) => {
            let clause = graph
                .state_clauses()
                .iter()
                .find(|clause| clause.identity() == *node && clause.claim() == claim.origin())
                .ok_or(ContractSelection::NoForm)?;
            let context = clause.context();
            if clause.kind() == StateClauseKind::Constraint {
                return Err(ContractSelection::NoForm);
            }
            if clause.kind() == StateClauseKind::Invariant {
                return Ok(selected(
                    ContractForm::Invariant,
                    Target::Invariant {
                        clause: node.clone(),
                        context,
                    },
                    false,
                ));
            }
            let operation = clause.operation().ok_or(ContractSelection::MissingNode)?;
            let name = operation.declaration.name().to_owned();
            let key = operation_key(package, operation.declaring.clone(), &name)?;
            let elements = graph
                .item_elements(claim)
                .transpose()
                .map_err(ContractSelection::Fault)?
                .unwrap_or_default();
            let bound = binding(relation, &key, elements)?;
            let target = if clause.kind() == StateClauseKind::Precondition {
                Target::Precondition {
                    clause: node.clone(),
                    context,
                }
            } else {
                Target::Postcondition {
                    clause: node.clone(),
                    context,
                    operation: (operation.declaring.clone(), name),
                }
            };
            Ok(selected(ContractForm::Clause, target, bound))
        }
        (CheckedOccurrenceRole::Anchor, Some(node)) => {
            let frame = graph
                .operation_frame_by_anchor(node.clone())
                .ok_or(ContractSelection::NoForm)?;
            if claim.origin().ordinal() != 0 {
                return Err(ContractSelection::NoForm);
            }
            let operation = frame.operation();
            if operation.declaration.effect().is_empty() {
                return Err(ContractSelection::NoForm);
            }
            let key = operation_key(
                package,
                operation.declaring.clone(),
                operation.declaration.name(),
            )?;
            let elements = [AbstractionKey::Object(key.declaring.clone())];
            let bound = binding(relation, &key, elements)?;
            Ok(selected(ContractForm::Frame, Target::Frame, bound))
        }
        (CheckedOccurrenceRole::Generated, _) => {
            let entry = graph
                .refinements()
                .iter()
                .enumerate()
                .find_map(|(index, refinement)| {
                    refinement
                        .step_simulations()
                        .iter()
                        .find(|entry| entry.occurrence() == claim)
                        .map(|entry| (index, entry))
                });
            let Some((refinement, entry)) = entry else {
                not_step_simulable(package, claim)?;
                return Ok(selected(
                    ContractForm::Simulation,
                    Target::NotStepSimulable,
                    false,
                ));
            };
            let row = match entry.record().obligation {
                StepObligation::Initial => None,
                StepObligation::Step { row } => {
                    // A row is named by its declaration position.
                    let source = usize::try_from(row.index())
                        .ok()
                        .filter(|source| *source < graph.refinements()[refinement].steps().len())
                        .ok_or(ContractSelection::MissingNode)?;
                    Some(source)
                }
            };
            Ok(selected(
                ContractForm::Simulation,
                Target::Simulation { refinement, row },
                false,
            ))
        }
        _ => Err(ContractSelection::NoForm),
    }
}

/// The `step-relation` form of `claim` on the step-relation node `node`
/// (QSpec FR-485 "Forms"): the node's operation-contract code claim, which
/// QSL writes only when every execution's operation is bound (ADR-023 XC-1).
/// The node's model claim, a `temporal-satisfaction` item, selects no form;
/// a bound item that reads an element with no abstraction binding is
/// [`ContractSelection::Unbound`].
fn step_relation(
    package: &CheckedPackage,
    claim: &OccurrenceKey,
    node: NodeKey,
) -> Result<ContractClaim, ContractSelection> {
    let graph = package.graph();
    let relation = graph
        .step_relations()
        .iter()
        .find(|relation| relation.identity() == Some(node.clone()))
        .ok_or(ContractSelection::NoForm)?;
    let code = graph
        .requirements()
        .get(claim)
        .is_some_and(|record| record.requirements().kind() == Capability::OperationContract);
    if relation.occurrence() == Some(claim) || !code {
        return Err(ContractSelection::NoForm);
    }
    let elements = graph
        .item_elements(claim)
        .transpose()
        .map_err(ContractSelection::Fault)?
        .unwrap_or_default();
    let relation_bindings = package.abstraction();
    let mut bound = true;
    for (_, operation) in relation.executions() {
        bound &= binding(relation_bindings, operation, elements.iter().cloned())?;
    }
    if !bound {
        // QSL writes the code claim only over bound executions.
        return Err(ContractSelection::Fault(fault(
            "a-step-relation-code-claim-is-bound",
        )));
    }
    Ok(ContractClaim {
        claim: claim.clone(),
        form: ContractForm::StepRelation,
        target: Target::StepRelation,
        bound,
    })
}

/// A `generated` claim with no step-simulation entry: `Ok` when it names a
/// rule of a refinement that is not proof-eligible (ordinal 0, or k + 1 for
/// one of its step rows), else no form.
fn not_step_simulable(
    package: &CheckedPackage,
    claim: &OccurrenceKey,
) -> Result<(), ContractSelection> {
    let ordinal = claim.origin().ordinal();
    let names = package.graph().refinements().iter().any(|refinement| {
        !refinement.proof_eligible()
            && usize::try_from(ordinal).is_ok_and(|ordinal| ordinal <= refinement.steps().len())
            && refinement
                .v2_emission()
                .is_ok_and(|emission| emission.identity == *claim.node())
    });
    if names {
        Ok(())
    } else {
        Err(ContractSelection::NoForm)
    }
}

/// One case the arm evaluates (QSpec FR-485 "Common premises").
#[derive(Clone, Debug)]
pub enum ContractCase {
    /// An initial state of the subject, at its index in the subject's list.
    Initial(usize, ModelSimState),
    /// A pre-state of the declared types.
    Declared(ModelSimState),
}

impl ContractCase {
    fn state(&self) -> &ModelSimState {
        match self {
            Self::Initial(_, state) | Self::Declared(state) => state,
        }
    }
}

/// The obligation a violation names (QSpec FR-485 "Counterexample").
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ContractObligation {
    /// An invariant false or undefined in an initial state.
    InitialState,
    /// An invariant false or undefined after an operation of its context.
    Preservation,
    /// A clause undefined under its premises.
    Definedness,
    /// A refinement's initial-state rule.
    SimulationInitial,
    /// A refinement's step-row rule.
    SimulationStep,
}

impl ContractObligation {
    /// The obligation's wire spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InitialState => "initial-state",
            Self::Preservation => "preservation",
            Self::Definedness => "definedness",
            Self::SimulationInitial => "simulation-initial",
            Self::SimulationStep => "simulation-step",
        }
    }
}

/// One execution of a `contract-counterexample` (QSpec FR-485
/// "Counterexample").
#[derive(Clone, Debug)]
pub struct ContractExecution {
    /// The operation's `state`/`operation_anchor` node; `None` for an
    /// initial-state violation, or an operation no clause names.
    pub operation: Option<NodeKey>,
    /// The application's transition identity; `None` for an initial-state
    /// violation.
    pub transition: Option<ModelTransition>,
    /// The pre-state.
    pub pre_state: ModelState,
    /// The arguments, in parameter order.
    pub arguments: Vec<Value>,
    /// The result, when the operation has one.
    pub result: Option<Value>,
    /// The post-state; `None` when the violated obligation reads none.
    pub post_state: Option<ModelState>,
}

/// A `contract-counterexample` (QSpec FR-485 "Counterexample").
#[derive(Clone, Debug)]
pub struct ContractCounterexample {
    /// The violated obligation.
    pub obligation: ContractObligation,
    /// The `{node, occurrence}` of the clause or rule that is false or
    /// undefined: the item's claim.
    pub violated: OccurrenceKey,
    /// Whether the item's oracle is a bound implementation. The arm's own
    /// counterexamples are always `false`; a received one may not be.
    pub bound: bool,
    /// One entry per execution.
    pub executions: Vec<ContractExecution>,
    /// The cause, when the goal was undefined rather than false.
    pub undefined: Option<UndefinedCause>,
}

/// A violation the native replay reproduced: the only evidence of a
/// `refuted` arm outcome.
#[derive(Clone, Debug)]
pub struct ReproducedContractViolation {
    counterexample: ContractCounterexample,
}

impl ReproducedContractViolation {
    /// The reproduced counterexample.
    pub fn counterexample(&self) -> &ContractCounterexample {
        &self.counterexample
    }
}

/// What evaluating one case showed.
#[derive(Clone, Debug)]
pub enum CaseVerdict {
    /// The premises held and the goal held at every check of the case.
    Holds,
    /// The premises held at no check of the case: it makes zero checks.
    Inapplicable,
    /// The goal is false or undefined at a check of the case.
    Violated(Box<ContractCounterexample>),
    /// A premise or the goal decided no Boolean, and no violation was found.
    Undecided,
    /// A budget or the meter stopped the evaluation.
    Stopped(ExpansionStop),
}

/// What reading a received counterexample gave (QSpec FR-485 "Replay").
#[derive(Clone, Debug)]
pub enum ReadCounterexample {
    /// The replay reproduced it.
    Reproduced(Box<ReproducedContractViolation>),
    /// The replay did not reproduce it: `inconclusive` `replay-parity`.
    NotReproduced,
    /// It is bound and this reader holds no implementation engine:
    /// `inconclusive` `replay-refused`, never `refuted`.
    ReplayRefused,
}

impl ReadCounterexample {
    /// The `inconclusive` cause a read that does not refute settles with
    /// (QSpec FR-483 "Replay before refuted", FR-485 "Replay"):
    /// `replay-refused` for a bound counterexample this reader cannot
    /// replay, `replay-parity` for one the replay did not reproduce; `None`
    /// for a reproduced violation, which settles `refuted`.
    pub fn inconclusive_cause(&self) -> Option<crate::InconclusiveCause> {
        use quire_semantic_value::catalog::Category;
        match self {
            Self::Reproduced(_) => None,
            Self::NotReproduced => Some(crate::InconclusiveCause::ReplayParity(
                crate::DisagreementCause::Verdicts {
                    proved: crate::Verdict::from_category(Category::Violation),
                    replayed: crate::Verdict::from_category(Category::Success),
                },
            )),
            Self::ReplayRefused => Some(crate::InconclusiveCause::ReplayRefused(
                qsl_foundation::diagnostic::Code::UnsupportedProjection,
            )),
        }
    }
}

/// The violations, checks and open readings of one case.
#[derive(Default)]
struct Evaluated {
    violations: Vec<ContractCounterexample>,
    checks: usize,
    undecided: bool,
}

/// A claim's target over a concrete model system and, for a simulation
/// item, the refinement's abstract system.
pub struct ContractTarget<'s, 'a> {
    claim: ContractClaim,
    package: &'s CheckedPackage,
    concrete: &'s ModelSystem<'a>,
    abstract_system: Option<&'s ModelSystem<'a>>,
}

/// A clause outcome as a goal reading: `Ok(Some(cause))` undefined,
/// `Ok(None)` refused or not Boolean, `Err` a meter stop.
fn undefined_of(outcome: &ClauseOutcome) -> Option<UndefinedCause> {
    outcome.undefined_cause()
}

impl<'s, 'a> ContractTarget<'s, 'a> {
    /// The target of `claim` over `concrete` and, for a simulation item, the
    /// refinement's `abstract_system`.
    ///
    /// # Errors
    ///
    /// [`ContractCheckUnsupported`] when the arm does not decide the item:
    /// it is bound (no implementation engine), a frame item has no
    /// implementation binding, or an invariant's context has a bound
    /// operation. An [`InternalFault`] when a simulation item has no
    /// abstract system.
    pub fn new(
        claim: ContractClaim,
        package: &'s CheckedPackage,
        concrete: &'s ModelSystem<'a>,
        abstract_system: Option<&'s ModelSystem<'a>>,
    ) -> Result<Result<Self, ContractCheckUnsupported>, InternalFault> {
        if let Some(reason) = claim.refusal() {
            return Ok(Err(reason));
        }
        match &claim.target {
            Target::Simulation { refinement, .. }
                if !package
                    .graph()
                    .refinements()
                    .get(*refinement)
                    .is_some_and(CheckedRefinement::proof_eligible) =>
            {
                return Ok(Err(ContractCheckUnsupported::NoContractLevelReading))
            }
            Target::Invariant { context, .. } => {
                let relation = package.abstraction();
                for (declaring, name) in concrete.context_operations(context.clone()) {
                    let Ok(key) = operation_key(package, declaring, &name) else {
                        return Err(fault("operation-owner-is-admitted"));
                    };
                    if relation
                        .is_some_and(|relation| relation.get(&AbstractionKey::Frame(key)).is_some())
                    {
                        return Ok(Err(ContractCheckUnsupported::NoImplementationEngine));
                    }
                }
            }
            Target::Simulation { .. } if abstract_system.is_none() => {
                return Err(fault("a-simulation-item-has-its-abstract-system"));
            }
            _ => {}
        }
        Ok(Ok(Self {
            claim,
            package,
            concrete,
            abstract_system,
        }))
    }

    /// The claim's form.
    pub fn form(&self) -> ContractForm {
        self.claim.form
    }

    /// The claim's `{node, occurrence}`.
    pub fn claim(&self) -> &OccurrenceKey {
        &self.claim.claim
    }

    /// The initial-state cases the form reads: every initial state of the
    /// subject for an invariant and a simulation's initial-state rule, none
    /// otherwise.
    pub fn initial_cases(&self) -> Vec<ContractCase> {
        let reads = match &self.claim.target {
            Target::Invariant { .. } => true,
            Target::Simulation { row, .. } => row.is_none(),
            _ => false,
        };
        if !reads {
            return Vec::new();
        }
        self.concrete
            .initial_states()
            .iter()
            .cloned()
            .enumerate()
            .map(|(index, state)| ContractCase::Initial(index, state))
            .collect()
    }

    /// The pre-states of the declared types the form reads; `None` when it
    /// reads none (a simulation's initial-state rule).
    ///
    /// # Errors
    ///
    /// [`ContractCheckUnsupported`] when the model's pre-states cannot be
    /// enumerated.
    pub fn declared_cases(
        &self,
    ) -> Result<Option<DeclaredStates<'s, 'a>>, ContractCheckUnsupported> {
        if matches!(self.claim.target, Target::Simulation { row: None, .. }) {
            return Ok(None);
        }
        self.concrete
            .declared_states()
            .map(Some)
            .map_err(|refusal| {
                use qsl_eval::simulation::model::DeclaredStatesRefusal as Refusal;
                match refusal {
                    Refusal::Relationship => ContractCheckUnsupported::RelationshipState,
                    Refusal::ClockMember => ContractCheckUnsupported::ClockMember,
                    Refusal::Field { .. } => ContractCheckUnsupported::FieldDomain,
                }
            })
    }

    /// Evaluate the claim at `case`.
    ///
    /// # Errors
    ///
    /// An [`InternalFault`] when the evaluator breaks one of its invariants.
    pub fn evaluate(&self, case: &ContractCase) -> Result<CaseVerdict, InternalFault> {
        let evaluated = match self.violations(case) {
            Ok(evaluated) => evaluated,
            Err(Stop::Expansion(stop)) if stop.is_internal_failure() => {
                return Err(fault("contract-evaluation-invariant"))
            }
            Err(Stop::Expansion(stop)) => return Ok(CaseVerdict::Stopped(stop)),
            Err(Stop::Fault(fault)) => return Err(fault),
        };
        let Evaluated {
            mut violations,
            checks,
            undecided,
        } = evaluated;
        Ok(if !violations.is_empty() {
            CaseVerdict::Violated(Box::new(violations.remove(0)))
        } else if undecided {
            CaseVerdict::Undecided
        } else if checks == 0 {
            CaseVerdict::Inapplicable
        } else {
            CaseVerdict::Holds
        })
    }

    /// Replay `counterexample` through the native evaluator: the violation
    /// it reproduces, or `None` when the evaluator does not find the same
    /// violation at the recorded case (QSpec FR-485 "Replay": the premises
    /// and the obligation are evaluated on the recorded values).
    ///
    /// # Errors
    ///
    /// An [`InternalFault`] when the evaluator breaks one of its invariants.
    pub fn replay(
        &self,
        counterexample: &ContractCounterexample,
    ) -> Result<Option<ReproducedContractViolation>, InternalFault> {
        if counterexample.bound || counterexample.violated != self.claim.claim {
            return Ok(None);
        }
        let [execution] = counterexample.executions.as_slice() else {
            return Ok(None);
        };
        let case = match counterexample.obligation {
            ContractObligation::InitialState | ContractObligation::SimulationInitial => {
                let Some(index) = self.initial_index(&execution.pre_state)? else {
                    return Ok(None);
                };
                ContractCase::Initial(index, self.concrete.initial_states()[index].clone())
            }
            ContractObligation::Preservation
            | ContractObligation::Definedness
            | ContractObligation::SimulationStep => ContractCase::Declared(
                self.concrete
                    .pre_state(execution.pre_state.clone(), self.concrete.declared_anchor()),
            ),
        };
        let evaluated = match self.violations(&case) {
            Ok(evaluated) => evaluated,
            Err(Stop::Expansion(_)) => return Ok(None),
            Err(Stop::Fault(fault)) => return Err(fault),
        };
        for found in evaluated.violations {
            if self.same(&found, counterexample)? {
                return Ok(Some(ReproducedContractViolation {
                    counterexample: found,
                }));
            }
        }
        Ok(None)
    }

    /// Read a received `contract-counterexample` (QSpec FR-485 "Replay"):
    /// a bound one cannot be replayed without an implementation engine,
    /// which this reader does not hold; an unbound one is replayed.
    ///
    /// # Errors
    ///
    /// As [`Self::replay`].
    pub fn read(
        &self,
        counterexample: &ContractCounterexample,
    ) -> Result<ReadCounterexample, InternalFault> {
        if counterexample.bound {
            return Ok(ReadCounterexample::ReplayRefused);
        }
        Ok(match self.replay(counterexample)? {
            Some(reproduced) => ReadCounterexample::Reproduced(Box::new(reproduced)),
            None => ReadCounterexample::NotReproduced,
        })
    }

    /// The index of the subject's initial state equal to `state`, by state
    /// key.
    fn initial_index(&self, state: &ModelState) -> Result<Option<usize>, InternalFault> {
        let key = self.key(state)?;
        for (index, initial) in self.concrete.initial_states().iter().enumerate() {
            if self.key(initial.state())? == key {
                return Ok(Some(index));
            }
        }
        Ok(None)
    }

    fn key(&self, state: &ModelState) -> Result<Vec<u8>, InternalFault> {
        self.concrete
            .state_key_of(state)
            .map(|(key, _)| key)
            .map_err(|_| fault("state-encodes"))
    }

    /// The FR-101 value keys of `values`, which compare as the values do.
    fn values(&self, values: &[Value]) -> Result<Vec<Vec<u8>>, InternalFault> {
        values
            .iter()
            .map(|value| {
                self.concrete
                    .value_key(value)
                    .map_err(|_| fault("value-encodes"))
            })
            .collect()
    }

    /// Whether `found` is the violation `recorded` names: the same
    /// obligation, transition, pre-state, post-state and cause.
    fn same(
        &self,
        found: &ContractCounterexample,
        recorded: &ContractCounterexample,
    ) -> Result<bool, InternalFault> {
        if found.obligation != recorded.obligation || found.undefined != recorded.undefined {
            return Ok(false);
        }
        let ([found], [recorded]) = (found.executions.as_slice(), recorded.executions.as_slice())
        else {
            return Ok(false);
        };
        if found.transition != recorded.transition
            || self.values(&found.arguments)? != self.values(&recorded.arguments)?
            || self.values(found.result.as_slice())? != self.values(recorded.result.as_slice())?
            || self.key(&found.pre_state)? != self.key(&recorded.pre_state)?
        {
            return Ok(false);
        }
        Ok(match (&found.post_state, &recorded.post_state) {
            (None, None) => true,
            (Some(left), Some(right)) => self.key(left)? == self.key(right)?,
            _ => false,
        })
    }

    /// Every violation at `case`, with the checks it made and whether a
    /// reading was open.
    fn violations(&self, case: &ContractCase) -> Result<Evaluated, Stop> {
        match &self.claim.target {
            Target::Invariant { clause, context } => {
                self.invariant(case, clause.clone(), context.clone())
            }
            Target::Precondition { clause, context } => {
                self.precondition(case.state(), clause.clone(), context.clone())
            }
            Target::Postcondition {
                clause,
                context,
                operation,
            } => self.postcondition(case.state(), clause.clone(), context.clone(), operation),
            Target::Frame | Target::NotStepSimulable | Target::StepRelation => {
                Err(Stop::Fault(fault("an-unsupported-item-is-not-evaluated")))
            }
            Target::Simulation { refinement, row } => {
                let refinement = self
                    .package
                    .graph()
                    .refinements()
                    .get(*refinement)
                    .ok_or_else(|| Stop::Fault(fault("selected-refinement-exists")))?;
                match (row, case) {
                    (None, ContractCase::Initial(index, state)) => {
                        self.simulation_initial(refinement, *index, state)
                    }
                    (Some(source), ContractCase::Declared(state)) => {
                        self.simulation_step(refinement, *source, state)
                    }
                    _ => Ok(Evaluated::default()),
                }
            }
        }
    }

    fn clause(&self, identity: NodeKey) -> Result<ContractClause, Stop> {
        self.concrete
            .contract_clauses()
            .into_iter()
            .find(|clause| clause.identity == identity)
            .ok_or_else(|| Stop::Fault(fault("the-claim-clause-is-evaluated")))
    }

    /// QSpec FR-485 "Common premises" 1 and 3 at `state`: every unit bound
    /// holds and every invariant whose context `holds_for` admits holds.
    /// `Ok(Some(false))` when one is false or undefined, `Ok(None)` when one
    /// decided nothing else.
    fn premises(
        &self,
        state: &ModelSimState,
        holds_for: impl Fn(&ContractClause) -> bool,
    ) -> Result<Option<bool>, Stop> {
        let clauses = self.concrete.contract_clauses();
        let mut open = false;
        for finding in self.concrete.invariant_findings_at(state)? {
            let (name, outcome) = match &finding {
                ModelFinding::InvariantViolated { clause, .. } => (clause, None),
                ModelFinding::InvariantUndetermined {
                    clause, evaluation, ..
                } => (clause, Some(evaluation)),
                _ => continue,
            };
            let premise = match clauses.iter().find(|clause| &clause.name == name) {
                Some(clause) => holds_for(clause),
                // A unit bound (FR-154): a declared bound of a field.
                None => true,
            };
            if !premise {
                continue;
            }
            match outcome {
                None => return Ok(Some(false)),
                Some(outcome) if undefined_of(outcome).is_some() => return Ok(Some(false)),
                Some(_) => open = true,
            }
        }
        Ok(if open { None } else { Some(true) })
    }

    fn execution(
        &self,
        operation: Option<(EffectiveId, &str)>,
        transition: Option<ModelTransition>,
        pre: &ModelSimState,
        arguments: Vec<Value>,
        result: Option<Value>,
        post: Option<ModelState>,
    ) -> ContractExecution {
        ContractExecution {
            operation: operation.and_then(|(declaring, name)| {
                self.package.graph().operation_anchor_of(declaring, name)
            }),
            transition,
            pre_state: pre.state().clone(),
            arguments,
            result,
            post_state: post,
        }
    }

    fn violation(
        &self,
        obligation: ContractObligation,
        execution: ContractExecution,
        undefined: Option<UndefinedCause>,
    ) -> ContractCounterexample {
        ContractCounterexample {
            obligation,
            violated: self.claim.claim.clone(),
            bound: false,
            executions: vec![execution],
            undefined,
        }
    }

    /// The invariant `name`'s reading at `state` over every object of its
    /// context: `Ok(Some(None))` false, `Ok(Some(Some(cause)))` undefined,
    /// `Ok(None)` true, `Err(())` decided nothing.
    fn goal(
        &self,
        state: &ModelSimState,
        name: &str,
    ) -> Result<Result<Option<Option<UndefinedCause>>, ()>, Stop> {
        let mut open = false;
        for finding in self.concrete.invariant_findings_at(state)? {
            match finding {
                ModelFinding::InvariantViolated { clause, .. } if clause == name => {
                    return Ok(Ok(Some(None)))
                }
                ModelFinding::InvariantUndetermined {
                    clause, evaluation, ..
                } if clause == name => match undefined_of(&evaluation) {
                    Some(cause) => return Ok(Ok(Some(Some(cause)))),
                    None => open = true,
                },
                _ => {}
            }
        }
        Ok(if open { Err(()) } else { Ok(None) })
    }

    fn invariant(
        &self,
        case: &ContractCase,
        clause: NodeKey,
        context: EffectiveId,
    ) -> Result<Evaluated, Stop> {
        let name = self.clause(clause)?.name;
        let mut evaluated = Evaluated::default();
        match case {
            ContractCase::Initial(_, state) => {
                evaluated.checks = 1;
                match self.goal(state, &name)? {
                    Ok(None) => {}
                    Ok(Some(undefined)) => evaluated.violations.push(self.violation(
                        ContractObligation::InitialState,
                        self.execution(None, None, state, Vec::new(), None, None),
                        undefined,
                    )),
                    Err(()) => evaluated.undecided = true,
                }
            }
            ContractCase::Declared(state) => {
                match self.premises(state, |premise| premise.context == context)? {
                    Some(true) => {}
                    Some(false) => return Ok(evaluated),
                    None => {
                        evaluated.undecided = true;
                        return Ok(evaluated);
                    }
                }
                for (declaring, operation) in self.concrete.context_operations(context.clone()) {
                    let cases = self.concrete.operation_cases(
                        state,
                        declaring.clone(),
                        &operation,
                        context.clone(),
                        &|_| true,
                    )?;
                    open_contracts(&cases, &mut evaluated);
                    for step in cases.steps {
                        evaluated.checks += 1;
                        match self.goal(&step.post, &name)? {
                            Ok(None) => {}
                            Ok(Some(undefined)) => evaluated.violations.push(self.violation(
                                ContractObligation::Preservation,
                                self.execution(
                                    Some((declaring.clone(), &operation)),
                                    Some(step.transition),
                                    state,
                                    step.arguments,
                                    step.result,
                                    Some(step.post.state().clone()),
                                ),
                                undefined,
                            )),
                            Err(()) => evaluated.undecided = true,
                        }
                    }
                }
            }
        }
        Ok(evaluated)
    }

    fn precondition(
        &self,
        state: &ModelSimState,
        clause: NodeKey,
        context: EffectiveId,
    ) -> Result<Evaluated, Stop> {
        let mut evaluated = Evaluated::default();
        match self.premises(state, |premise| premise.context == context)? {
            Some(true) => {}
            Some(false) => return Ok(evaluated),
            None => {
                evaluated.undecided = true;
                return Ok(evaluated);
            }
        }
        let operation = self.clause(clause.clone())?.operation;
        for case in self.concrete.precondition_cases(state, clause)? {
            evaluated.checks += 1;
            let Some(outcome) = case.undecided else {
                continue;
            };
            match undefined_of(&outcome) {
                Some(cause) => evaluated.violations.push(
                    self.violation(
                        ContractObligation::Definedness,
                        self.execution(
                            operation
                                .as_ref()
                                .map(|(declaring, name)| (declaring.clone(), name.as_str())),
                            Some(case.transition),
                            state,
                            case.arguments,
                            None,
                            None,
                        ),
                        Some(cause),
                    ),
                ),
                None => evaluated.undecided = true,
            }
        }
        Ok(evaluated)
    }

    fn postcondition(
        &self,
        state: &ModelSimState,
        clause: NodeKey,
        context: EffectiveId,
        (declaring, operation): &(EffectiveId, String),
    ) -> Result<Evaluated, Stop> {
        let mut evaluated = Evaluated::default();
        match self.premises(state, |premise| premise.context == context)? {
            Some(true) => {}
            Some(false) => return Ok(evaluated),
            None => {
                evaluated.undecided = true;
                return Ok(evaluated);
            }
        }
        let name = self.clause(clause.clone())?.name;
        let clauses = self.concrete.contract_clauses();
        let select = |identity: NodeKey| {
            identity == clause
                || clauses.iter().any(|known| {
                    known.identity == identity && known.kind == ContractClauseKind::Precondition
                })
        };
        let cases =
            self.concrete
                .operation_cases(state, declaring.clone(), operation, context, &select)?;
        evaluated.checks = cases.enabled;
        for contract in cases.undecided {
            let undefined = undefined_of(&contract.evaluation);
            match (contract.clause == name, contract.post, undefined) {
                (true, Some((post, result)), Some(cause)) => {
                    evaluated.violations.push(self.violation(
                        ContractObligation::Definedness,
                        self.execution(
                            Some((declaring.clone(), operation)),
                            Some(contract.transition),
                            state,
                            contract.arguments,
                            result,
                            Some(post),
                        ),
                        Some(cause),
                    ));
                }
                // An undefined precondition is a premise that does not hold.
                (_, _, Some(_)) => {}
                (_, _, None) => evaluated.undecided = true,
            }
        }
        Ok(evaluated)
    }

    fn systems(&self) -> Result<&'s ModelSystem<'a>, Stop> {
        self.abstract_system
            .ok_or_else(|| Stop::Fault(fault("a-simulation-item-has-its-abstract-system")))
    }

    fn simulation_initial(
        &self,
        refinement: &CheckedRefinement,
        index: usize,
        state: &ModelSimState,
    ) -> Result<Evaluated, Stop> {
        let abstract_system = self.systems()?;
        let mut evaluated = Evaluated::default();
        match self.premises(state, |_| true)? {
            Some(true) => {}
            Some(false) => return Ok(evaluated),
            None => {
                evaluated.undecided = true;
                return Ok(evaluated);
            }
        }
        evaluated.checks = 1;
        let verdict = check_initial(
            refinement,
            self.concrete,
            abstract_system,
            index,
            state.state(),
        );
        self.judge(
            verdict,
            ContractObligation::SimulationInitial,
            || self.execution(None, None, state, Vec::new(), None, None),
            &mut evaluated,
        )?;
        Ok(evaluated)
    }

    fn simulation_step(
        &self,
        refinement: &CheckedRefinement,
        source: usize,
        state: &ModelSimState,
    ) -> Result<Evaluated, Stop> {
        let abstract_system = self.systems()?;
        let mut evaluated = Evaluated::default();
        let (declaring, operation) = refinement
            .steps()
            .get(source)
            .and_then(qsl_semantics::check::StepRow::concrete_operation)
            .ok_or_else(|| Stop::Fault(fault("a-simulation-row-is-an-operation-row")))?;
        match self.premises(state, |_| true)? {
            Some(true) => {}
            Some(false) => return Ok(evaluated),
            None => {
                evaluated.undecided = true;
                return Ok(evaluated);
            }
        }
        let cases = self.concrete.operation_cases(
            state,
            declaring.clone(),
            operation,
            declaring.clone(),
            &|_| true,
        )?;
        open_contracts(&cases, &mut evaluated);
        if cases.steps.is_empty() {
            return Ok(evaluated);
        }
        let history = initial_history(refinement, self.concrete, state.state());
        let position = match map_state(
            refinement,
            self.concrete,
            abstract_system,
            state.state(),
            &history,
        ) {
            Ok(mapped) => RefinementPosition {
                concrete: state.state().clone(),
                history,
                abstract_states: vec![mapped.visible],
                abstract_protocol_states: Vec::new(),
            },
            Err(failure) => {
                evaluated.checks += 1;
                let undefined = match failure {
                    MappingFailure::Undefined { cause, .. } => Some(*cause),
                    MappingFailure::Undetermined(_) => None,
                    MappingFailure::WrongSystem(_) | MappingFailure::Fault(_) => {
                        return Err(Stop::Fault(fault("the-refinements-own-systems-map")))
                    }
                };
                evaluated.violations.push(self.violation(
                    ContractObligation::SimulationStep,
                    self.execution(None, None, state, Vec::new(), None, None),
                    undefined,
                ));
                return Ok(evaluated);
            }
        };
        for step in cases.steps {
            evaluated.checks += 1;
            let verdict = check_step(
                refinement,
                self.concrete,
                abstract_system,
                &position,
                &ConcreteStep {
                    position: 1,
                    transition: Some(&step.transition),
                    post: &step.post,
                },
            )
            .verdict;
            let OperationStepParts {
                transition,
                arguments,
                result,
                post,
            } = OperationStepParts::from(step);
            self.judge(
                verdict,
                ContractObligation::SimulationStep,
                || {
                    self.execution(
                        Some((declaring.clone(), operation)),
                        Some(transition),
                        state,
                        arguments,
                        result,
                        Some(post),
                    )
                },
                &mut evaluated,
            )?;
        }
        Ok(evaluated)
    }

    /// Record what a refinement step verdict says about the rule (QSpec
    /// FR-485 "Simulation": a violation is `refuted`, never `inconclusive`;
    /// a row that is undefined or undetermined is a violation).
    fn judge(
        &self,
        verdict: StepVerdict,
        obligation: ContractObligation,
        execution: impl FnOnce() -> ContractExecution,
        evaluated: &mut Evaluated,
    ) -> Result<(), Stop> {
        match verdict {
            StepVerdict::Passes { .. } => {}
            StepVerdict::Fails(_) | StepVerdict::Undetermined(Undetermined::Mapping(_)) => {
                evaluated
                    .violations
                    .push(self.violation(obligation, execution(), None));
            }
            StepVerdict::Undefined(undefined) => {
                evaluated.violations.push(self.violation(
                    obligation,
                    execution(),
                    Some(undefined.cause),
                ));
            }
            StepVerdict::Undetermined(Undetermined::UndecidedSuccessor) => {
                evaluated.undecided = true;
            }
            StepVerdict::Undetermined(Undetermined::ProtocolAssumptionNotEstablished(_)) => {
                evaluated.undecided = true;
            }
            StepVerdict::ProtocolContractUndetermined { .. } => {
                evaluated.undecided = true;
            }
            StepVerdict::Stopped(_) => {
                return Err(Stop::Expansion(ExpansionStop {
                    cause: quire_semantic_value::catalog::CatalogCode::new(
                        "resource_exhausted",
                        "insufficient-next-charge",
                    ),
                    limit: None,
                }))
            }
            StepVerdict::Fault(fault) => return Err(Stop::Fault(fault)),
        }
        Ok(())
    }
}

/// The parts of an operation step a counterexample keeps.
struct OperationStepParts {
    transition: ModelTransition,
    arguments: Vec<Value>,
    result: Option<Value>,
    post: ModelState,
}

impl From<qsl_eval::simulation::model::OperationStep> for OperationStepParts {
    fn from(step: qsl_eval::simulation::model::OperationStep) -> Self {
        Self {
            transition: step.transition,
            arguments: step.arguments,
            result: step.result,
            post: step.post.state().clone(),
        }
    }
}

/// A contract clause of `cases` that decided no Boolean for another reason
/// than undefinedness leaves the premise open.
fn open_contracts(cases: &OperationCases, evaluated: &mut Evaluated) {
    if cases
        .undecided
        .iter()
        .any(|contract| undefined_of(&contract.evaluation).is_none())
    {
        evaluated.undecided = true;
    }
}

/// Why an evaluation stopped before a verdict.
enum Stop {
    Expansion(ExpansionStop),
    Fault(InternalFault),
}

impl From<ExpansionStop> for Stop {
    fn from(stop: ExpansionStop) -> Self {
        Self::Expansion(stop)
    }
}
