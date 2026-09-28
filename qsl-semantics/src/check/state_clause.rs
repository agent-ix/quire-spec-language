// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-104: the `ProtocolClause` family's S3 check of a state clause
//! (ADR-012 §2, §15.7), and its `requirements` hook.
//!
//! The FR-091 assembler resolves a [`StateClauseForm`]'s profile alias, its
//! context type `M::T` and, for a `pre` or `post` clause, its operation in
//! `M::T`'s effective view (FR-103), and refuses an unresolved name at its
//! span, into a [`StateClauseDeclaration`]. [`ProtocolClauseFamily::check`]
//! then types the body as a Boolean under its clause kind, with `self`,
//! `result` and the operation's parameters bound, gives every model read
//! its observation ([`Observations`]) and runs the static definedness check
//! over observation-keyed facts. [`PackageDeclarations::check`] lowers each
//! checked clause after every function (a clause body may call one), which
//! mints its node identity and records its `claim` occurrence, and keys one
//! `operation-contract` record per clause and one per frame of an operation
//! a `pre` or `post` clause names.
//!
//! [`StateClauseForm`]: qsl_forms::StateClauseForm
//! [`PackageDeclarations::check`]: super::PackageDeclarations::check

use std::collections::{BTreeMap, BTreeSet};

use qsl_forms::{ClauseKind, DeclarationSpans, Expression, StateClauseKind};
use qsl_foundation::bound::DomainKey;
use qsl_foundation::diagnostic::{StageFailure, Staged};
use qsl_foundation::selection::ProfileSelection;
use quire_exact::{EffectiveId, NodeKey, Origin, ValueType};

use super::check::{bind_parameters, Signatures, StateContext, Typer};
use super::claims::{wire, RequirementRecord};
use super::facts::Definedness;
use super::ir::{DispatchTable, Node, NodeKind, Observation};
use super::lowering::{AdmittedModel, LoweredClause};
use super::observation::Observations;
use super::refusal::{CheckCause, CheckRefusal, KeyFault, Location};
use super::{Capability, CheckingLimits, Scope};
use crate::family::{
    classify_domains, classify_extent, CheckContext, CheckOutcome, ClaimExtent, ClassifyFailure,
    DomainKind, FamilyContract, Requirements,
};
use crate::model::key::DeclarationKey;
use crate::value::declaration::{OperationDeclaration, TypeEnvironment};

/// The operation a `pre` or `post` clause names, resolved in its context
/// type's effective view (FR-103).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClauseOperation {
    /// The object type that declares the operation: the context type, or
    /// the ancestor the context type inherits it from.
    pub declaring: EffectiveId,
    /// The operation.
    pub declaration: OperationDeclaration,
}

/// One protocol `attempt` the FR-091 assembler admitted (FR-114 "Behavior",
/// QSL-309): its operation resolved exactly as a `pre`/`post` clause's own
/// (FR-104), by the same assembler pass that resolves
/// [`StateClauseDeclaration`]s, so an attempt's operation and a clause's
/// operation naming the same `M::T::op` always agree on identity. The
/// `contracts` list stays on the attempt's own S2 form
/// (`qsl_forms::AttemptForm::contracts`, unresolved): checking it needs the
/// unit's other state clauses, which are checked at S3, not the assembler
/// (`check::protocol_clause` binds it, QSL-309).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttemptDeclaration {
    /// The index into the owning `ProtocolDeclarationForm::declarations` of
    /// this attempt's own static declaration (`ProtocolNodeKind::Attempt`),
    /// the same index [`qsl_forms::AttemptForm::declaration`] carries.
    pub declaration: usize,
    /// The resolved operation `on M::T::op` names.
    pub operation: ClauseOperation,
    /// The population domain of the type declaring the operation, resolved
    /// the same way a state clause's own `frame_population` is
    /// (FR-104 "Requirements"): `None` when the declaring type shares no
    /// population.
    pub frame_population: Option<PopulationDomain>,
}

/// One state clause the FR-091 assembler admitted (FR-104 "Inputs"): every
/// name of its header resolved, its body as S2 built it.
#[derive(Clone, Debug)]
pub struct StateClauseDeclaration {
    /// Invariant, precondition or postcondition.
    pub kind: StateClauseKind,
    /// The declared clause name.
    pub name: String,
    /// The profile selection the clause's `using` alias resolved to.
    pub selection: ProfileSelection,
    /// The context object type `M::T`, by its effective identity.
    pub context: EffectiveId,
    /// The operation a `pre` or `post` clause names; `None` for an
    /// invariant.
    pub operation: Option<ClauseOperation>,
    /// The context's population domain, the assembler's own resolution
    /// (FR-104 "Resolution", "Requirements"): the one population with no
    /// maximum whose member types cover `context`, by conformance
    /// (FR-084's `allInstances<T>`); `None` when none does.
    pub context_population: Option<PopulationDomain>,
    /// The population domain of the type declaring the operation a `pre` or
    /// `post` clause names, resolved the same way; `None` for an invariant,
    /// or when the declaring type shares no population.
    pub frame_population: Option<PopulationDomain>,
    /// The body.
    pub body: Expression,
    /// The form's spans: the declaration's, and one per body node.
    pub spans: DeclarationSpans,
}

impl StateClauseDeclaration {
    /// The clause kind as the typer reads it: `pre(e)` is legal only in a
    /// postcondition.
    fn clause_kind(&self) -> ClauseKind {
        match self.kind {
            StateClauseKind::Invariant => ClauseKind::Invariant,
            StateClauseKind::Precondition => ClauseKind::Precondition,
            StateClauseKind::Postcondition => ClauseKind::Postcondition,
        }
    }

    /// The observation the clause's own reads take (FR-104).
    fn observation(&self) -> Observation {
        match self.kind {
            StateClauseKind::Invariant => Observation::Current,
            StateClauseKind::Precondition => Observation::Pre,
            StateClauseKind::Postcondition => Observation::Post,
        }
    }

    /// `self`, then `result` in a postcondition of an operation that
    /// declares one, then the operation's parameters of a `pre` or `post`
    /// clause, each typed as FR-103 declares it: the slots the body is
    /// typed over, in order. An invariant binds `self` alone.
    fn parameters(&self) -> (Vec<(String, ValueType)>, bool) {
        let mut parameters = vec![("self".to_owned(), ValueType::Reference(self.context))];
        let Some(operation) = &self.operation else {
            return (parameters, false);
        };
        let result = match (self.kind, operation.declaration.result()) {
            (StateClauseKind::Postcondition, Some(result)) => {
                parameters.push(("result".to_owned(), result.clone()));
                true
            }
            _ => false,
        };
        parameters.extend(operation.declaration.parameters().iter().cloned());
        (parameters, result)
    }
}

/// The read-only declarations a state clause checks against.
///
/// The type itself stays `pub`: it is `FamilyContract::Declarations`
/// (`family/contract.rs`), a trait `qsl-eval` and `qsl-package` also
/// implement/consume, so E0446 ("private in public") refuses a
/// `pub(crate)` type here. Its fields are `pub(crate)` (SR-750 FND-012):
/// only `check::mod`'s own state-clause checking constructs or reads one,
/// never a caller outside `qsl-semantics`.
pub struct ClauseDeclarations<'a> {
    pub(crate) scope: &'a Scope,
    pub(crate) signatures: &'a Signatures,
    pub(crate) dispatch_tables: &'a [DispatchTable],
    pub(crate) models: &'a [AdmittedModel],
    pub(crate) checking_limits: CheckingLimits,
    /// The clause body's root location.
    pub(crate) location: &'a Location,
    /// The package's running expression-node total before this clause.
    pub(crate) nodes_used: u64,
}

/// One population domain of a requested item (FR-104 "Requirements"): the
/// population's own declared member object type (never the clause's
/// context, which may be one of its proper subtypes, SR-736 FND-010) and
/// its ordinal among its package's population declarations in ascending
/// `DeclarationKey` order.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PopulationDomain {
    pub(crate) object: EffectiveId,
    pub(crate) ordinal: usize,
}

/// What an `operation-contract` claim is about.
///
/// `pub`, not narrowed: an enum's own variants cannot carry a narrower
/// visibility than the enum itself, and this enum is `ClauseClaim::subject`
/// (below)'s field type, which must stay reachable wherever `ClauseClaim`
/// is (SR-750 FND-012).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClaimSubject {
    /// The clause itself, keyed by its `claim` occurrence.
    Clause,
    /// The frame of the operation a `pre` or `post` clause names, keyed by
    /// the frame node's own occurrence.
    Frame,
}

/// One `operation-contract` claim of a checked state clause, before
/// lowering keys it (ADR-012 §13.5).
///
/// The type itself stays `pub` (`FamilyContract::Claim`, same reasoning as
/// [`ClauseDeclarations`]); its own field is `pub(crate)` (SR-750 FND-012).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClauseClaim {
    pub(crate) subject: ClaimSubject,
    /// The population domains the item ranges over.
    pub(crate) populations: Vec<PopulationDomain>,
}

/// A state clause checked by [`ProtocolClauseFamily::check`], before
/// lowering keys it.
#[derive(Debug)]
pub struct TypedStateClause {
    pub(crate) parameters: Vec<(String, ValueType)>,
    pub(crate) body: Node,
    pub(crate) slots: usize,
    pub(crate) slot_names: Vec<String>,
    pub(crate) observations: Observations,
    pub(crate) formed_units: crate::value::quantity::UnitTable,
    pub nodes_used: u64,
    /// The object types whose population domains the claims name, the
    /// context's first.
    pub(crate) population_types: Vec<EffectiveId>,
    pub(crate) claims: Vec<ClauseClaim>,
    /// Whether slot 1 is `result` (FR-104 "Behavior": a postcondition of an
    /// operation with a result binds it there; nothing else does). Carried
    /// as its own typed field, not re-derived by name from `parameters` at
    /// the S6a seam (QSL-278, FR-064's string-edge rule: dispatch on a
    /// parameter's name, not on a typed field the checker already knows,
    /// is exactly the kind of comparison a `#[string_edge]` reader marks,
    /// never an interior evaluator decision).
    pub(crate) has_result: bool,
}

/// The `ProtocolClause` family's state clause production (ADR-012 §15.2,
/// FR-104).
pub struct ProtocolClauseFamily;

/// The one population with no maximum that covers `object`, by conformance
/// (FR-104, FR-084's `allInstances<T>`: a member type or a proper supertype
/// of it): `Ok(None)` when none does, and the populations' keys in ascending
/// `DeclarationKey` order when several do. A domain package population
/// declares no maximum, so every one counts.
pub(crate) fn population_of(
    models: &[AdmittedModel],
    object: EffectiveId,
    types: &TypeEnvironment,
) -> Result<Option<PopulationDomain>, Vec<DeclarationKey>> {
    let found: Vec<(usize, &DeclarationKey, EffectiveId)> = models
        .iter()
        .flat_map(|model| model.populations_of(object, |sub, sup| types.conforms(sub, sup)))
        .collect();
    match found.as_slice() {
        [] => Ok(None),
        // The domain names the population's own covering member type, not
        // `object` (SR-736 FND-010): `Sub` and `ConfigVersion` clauses over
        // `config_history` key the same `PopulationDomain`.
        [(ordinal, _, member)] => Ok(Some(PopulationDomain {
            object: *member,
            ordinal: *ordinal,
        })),
        many => {
            let mut keys: Vec<DeclarationKey> =
                many.iter().map(|(_, key, _)| (*key).clone()).collect();
            keys.sort();
            Err(keys)
        }
    }
}

/// The ambiguous-population refusal at `location`: `object`'s population
/// cannot be named exactly once (FR-104 "Requirements").
fn ambiguous_population(
    scope: &Scope,
    object: EffectiveId,
    populations: &[DeclarationKey],
    location: &Location,
) -> CheckRefusal {
    let name = scope
        .types()
        .object_type(object)
        .map_or_else(String::new, |declaration| declaration.name().to_owned());
    let populations: Vec<&str> = populations.iter().map(|key| key.node.as_str()).collect();
    CheckRefusal {
        location: location.clone(),
        cause: CheckCause::AmbiguousName {
            name: format!("{name} ({})", populations.join(", ")),
            loci: vec![location.clone()],
        },
    }
}

impl FamilyContract for ProtocolClauseFamily {
    type Form = StateClauseDeclaration;
    type Checked = TypedStateClause;
    type Cause = CheckRefusal;
    type Declarations<'a> = ClauseDeclarations<'a>;
    type Claim = ClauseClaim;

    fn check<'a>(
        form: &StateClauseDeclaration,
        cx: &mut CheckContext<'a, ClauseDeclarations<'a>>,
    ) -> CheckOutcome<TypedStateClause, CheckRefusal> {
        let declarations = cx.declarations();
        check_clause(form, declarations)
            .map(Staged::new)
            .map_err(StageFailure::Refused)
    }

    fn requirements(checked: &TypedStateClause) -> Vec<ClauseClaim> {
        checked.claims.clone()
    }
}

/// Type `form`'s body as a Boolean under its clause kind, give each model
/// read its observation, and check its definedness (FR-104 "Typing",
/// "Observations of reads", "Definedness facts").
fn check_clause(
    form: &StateClauseDeclaration,
    input: &ClauseDeclarations<'_>,
) -> Result<TypedStateClause, CheckRefusal> {
    let location = input.location;
    let mut nodes = input.nodes_used;
    let (parameters, has_result) = form.parameters();
    let mut typer = Typer::new(
        input.scope,
        input.signatures,
        input.checking_limits,
        &mut nodes,
        form.clause_kind(),
    );
    bind_parameters(&mut typer, &parameters, location)?;
    typer.enter_state_clause(StateContext {
        kind: form.kind,
        operation: form
            .operation
            .as_ref()
            .map(|operation| operation.declaration.name().to_owned()),
        self_slot: 0,
        result_slot: has_result.then_some(1),
    });
    let body = typer.infer(&form.body, Some(&ValueType::Boolean), location)?;
    if body.value_type() != &ValueType::Boolean {
        return Err(CheckRefusal {
            location: location.clone(),
            cause: CheckCause::NonBooleanRoot,
        });
    }
    let slots = typer.slots();
    let slot_names = typer.slot_names().to_vec();
    let formed_units = typer.into_formed_units();
    let observations = Observations::of(&body, form.observation(), parameters.len());
    Definedness::new(
        parameters.len(),
        input.dispatch_tables,
        &input.scope.dispatch_operations,
    )
    .with_observations(&observations)
    .check(&body)?;

    // The populations the clause ranges over: its context's, already
    // resolved by the assembler (FR-104 "Resolution"), then each one a
    // `reaches` walks -- unknown until the body is typed, so resolved here
    // (FR-104 "Requirements").
    let mut clause_populations = BTreeSet::new();
    clause_populations.extend(form.context_population);
    for node in body.descendants() {
        if let NodeKind::Reaches { source, .. } = node.kind() {
            if let ValueType::Reference(object) = source.value_type() {
                let walked =
                    population_of(input.models, *object, input.scope.types()).map_err(|keys| {
                        ambiguous_population(input.scope, *object, &keys, node.location())
                    })?;
                clause_populations.extend(walked);
            }
        }
    }
    let mut claims = vec![ClauseClaim {
        subject: ClaimSubject::Clause,
        populations: clause_populations.into_iter().collect(),
    }];
    if form.operation.is_some() {
        claims.push(ClauseClaim {
            subject: ClaimSubject::Frame,
            populations: form.frame_population.into_iter().collect(),
        });
    }
    let population_types = claims
        .iter()
        .flat_map(|claim| claim.populations.iter().map(|domain| domain.object))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    Ok(TypedStateClause {
        parameters,
        body,
        slots,
        slot_names,
        observations,
        formed_units,
        nodes_used: nodes,
        population_types,
        claims,
        has_result,
    })
}

/// One state clause S3 checked (FR-104 "Outputs"): its kind, context,
/// operation, checked Boolean body with each model read's observation, its
/// node identity and its `claim` occurrence.
#[derive(Debug)]
pub struct CheckedStateClause {
    pub(crate) name: String,
    pub(crate) kind: StateClauseKind,
    pub(crate) context: EffectiveId,
    pub(crate) operation: Option<ClauseOperation>,
    pub(crate) parameters: Vec<(String, ValueType)>,
    pub(crate) body: Node,
    pub(crate) slots: usize,
    pub(crate) observations: Observations,
    pub(crate) identity: NodeKey,
    pub(crate) claim: Origin,
    pub(crate) spans: DeclarationSpans,
    pub(crate) has_result: bool,
}

impl CheckedStateClause {
    /// The declared name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Invariant, precondition or postcondition.
    pub fn kind(&self) -> StateClauseKind {
        self.kind
    }

    /// The context object type `M::T`, by its effective identity.
    pub fn context(&self) -> EffectiveId {
        self.context
    }

    /// The operation a `pre` or `post` clause names.
    pub fn operation(&self) -> Option<&ClauseOperation> {
        self.operation.as_ref()
    }

    /// `self`, then `result` when bound, then the operation's parameters,
    /// each with its type, in slot order.
    pub fn parameters(&self) -> &[(String, ValueType)] {
        &self.parameters
    }

    /// Whether slot 1 is `result` (a postcondition of an operation with a
    /// declared result; never true for an invariant or a precondition, or
    /// a postcondition of an operation with none). FR-104 "Behavior": the
    /// evaluator (S6a) reads this typed field to decide whether to bind
    /// `result`, instead of comparing a parameter's name.
    pub fn binds_result(&self) -> bool {
        self.has_result
    }

    /// The checked Boolean body.
    pub fn body(&self) -> &Node {
        &self.body
    }

    /// The evaluation slot count.
    pub fn slots(&self) -> usize {
        self.slots
    }

    /// Every model read of the body and the observation it reads at, by
    /// the read's location.
    pub fn reads(&self) -> impl Iterator<Item = (&Location, Observation)> {
        self.observations.iter()
    }

    /// The clause's node identity: its `state_clause` node's key. Two
    /// clauses of equal kind, anchor and body share it (FR-104-AC-6).
    pub fn identity(&self) -> NodeKey {
        self.identity
    }

    /// The clause's `claim` occurrence of its node (ADR-013 O-07).
    pub fn claim(&self) -> &Origin {
        &self.claim
    }
}

/// The `operation-contract` requirement record of one claim (FR-104
/// "Requirements"): its extent over the item's `roots` (each a checked
/// parameter node's key and type, the domains of whose type positions
/// ADR-014 §4 classifies, FR-097) and its population domains, each keyed
/// by its member object type's model node and its ordinal. The record's
/// result is the clause's `Boolean`.
pub(crate) fn record(
    roots: &[(NodeKey, &ValueType)],
    root_prefix: Option<NodeKey>,
    populations: &[(PopulationDomain, NodeKey)],
    boolean: NodeKey,
    types: &TypeEnvironment,
    position_limit: u64,
) -> Result<RequirementRecord, ClassifyFailure> {
    let mut domains = BTreeMap::new();
    match root_prefix {
        // The ordinary case: each root already has its own node, exactly
        // what `classify_extent` keys by, so it is reused rather than
        // reimplemented.
        None => {
            let wired: Vec<(qsl_foundation::digest::WireNodeId, &ValueType)> = roots
                .iter()
                .map(|(node, value_type)| (wire(*node), *value_type))
                .collect();
            if let ClaimExtent::Unbounded(unbounded) =
                classify_extent(&wired, types, position_limit)?
            {
                domains.extend(unbounded.iter().map(|(key, kind)| (key.clone(), kind)));
            }
        }
        // A frame has no parameter node of its own: its roots' domains are
        // keyed by the frame node, each under its root's position, so this
        // walks `classify_domains` directly instead.
        Some(frame) => {
            let root_types: Vec<&ValueType> =
                roots.iter().map(|(_, value_type)| *value_type).collect();
            for ((root, path), kind) in classify_domains(&root_types, types, position_limit)? {
                let position = u32::try_from(root).map_err(|_| {
                    ClassifyFailure::Fault(qsl_foundation::InternalFault::new(
                        "check.requirements",
                        "type-position-index-past-u32",
                    ))
                })?;
                let mut prefixed = vec![position];
                prefixed.extend(path);
                domains.insert(DomainKey::new(wire(frame), prefixed), kind);
            }
        }
    }
    for (domain, object) in populations {
        let ordinal = u32::try_from(domain.ordinal).map_err(|_| {
            ClassifyFailure::Fault(qsl_foundation::InternalFault::new(
                "check.requirements",
                "population-ordinal-past-u32",
            ))
        })?;
        domains.insert(
            DomainKey::new(wire(*object), vec![ordinal]),
            DomainKind::Population,
        );
    }
    Ok(RequirementRecord::unguarded(
        Requirements::new(
            Capability::OperationContract,
            ClaimExtent::from_domains(domains),
        ),
        ValueType::Boolean,
        wire(boolean),
    ))
}

/// A lowered state clause's claims, each with the node and the `Origin` of
/// the occurrence that keys its record: the clause's own `state_clause`
/// node at its `claim` occurrence, or its operation's `frame` node at that
/// operation's own occurrence (FR-104-AC-5: two operations never share one,
/// even when their frame nodes' content coincides -- see
/// [`super::lowering::Lowering::frame_occurrence`]).
pub(crate) struct KeyedClaim {
    pub(crate) node: NodeKey,
    pub(crate) origin: Origin,
    pub(crate) record: RequirementRecord,
}

/// The records of `clause`'s claims, once `lowered` has keyed its nodes.
/// `claim` is the clause's own `claim` occurrence (ADR-013 O-07), reused for
/// every `ClaimSubject::Clause` record.
#[allow(clippy::too_many_arguments)]
pub(crate) fn clause_records(
    clause: &TypedStateClause,
    claims: &[ClauseClaim],
    form: &StateClauseDeclaration,
    lowered: &LoweredClause,
    claim: &Origin,
    types: &TypeEnvironment,
    position_limit: u64,
    location: &Location,
) -> Result<Vec<KeyedClaim>, CheckRefusal> {
    let object_of = |object: EffectiveId| {
        clause
            .population_types
            .iter()
            .position(|named| *named == object)
            .and_then(|at| lowered.population_objects.get(at).copied())
    };
    let unkeyable = || CheckRefusal {
        location: location.clone(),
        cause: CheckCause::InternalFault(Box::new(KeyFault::UnkeyableRequirements)),
    };
    // `limit_cause` is `check`'s one mapping from a stage limit to a
    // checking cause (QSL-236); reused here rather than hand-built, so a
    // state clause's resource-exhausted refusal carries the same kind and
    // region a function's would.
    let classify = |failure: ClassifyFailure| match failure {
        ClassifyFailure::Limit(exceeded) => CheckRefusal {
            location: location.clone(),
            cause: super::limit_cause(&exceeded),
        },
        ClassifyFailure::Fault(fault) => CheckRefusal {
            location: location.clone(),
            cause: CheckCause::InternalFault(Box::new(KeyFault::UnclassifiedExtent(fault))),
        },
    };
    let mut keyed = Vec::with_capacity(claims.len());
    for entry in claims {
        let populations: Vec<(PopulationDomain, NodeKey)> = entry
            .populations
            .iter()
            .map(|domain| object_of(domain.object).map(|object| (*domain, object)))
            .collect::<Option<_>>()
            .ok_or_else(unkeyable)?;
        match entry.subject {
            ClaimSubject::Clause => {
                let roots: Vec<(NodeKey, &ValueType)> = lowered
                    .parameters
                    .iter()
                    .copied()
                    .zip(clause.parameters.iter().map(|(_, value_type)| value_type))
                    .collect();
                let record = record(
                    &roots,
                    None,
                    &populations,
                    lowered.boolean,
                    types,
                    position_limit,
                )
                .map_err(classify)?;
                keyed.push(KeyedClaim {
                    node: lowered.key,
                    origin: claim.clone(),
                    record,
                });
            }
            ClaimSubject::Frame => {
                let (Some((frame, frame_origin)), Some(operation)) =
                    (lowered.frame.clone(), &form.operation)
                else {
                    return Err(unkeyable());
                };
                // The frame's own roots: the declaring type's `self`, the
                // operation's result and its parameters.
                let receiver = ValueType::Reference(operation.declaring);
                let mut roots: Vec<(NodeKey, &ValueType)> = vec![(frame, &receiver)];
                roots.extend(operation.declaration.result().map(|result| (frame, result)));
                roots.extend(
                    operation
                        .declaration
                        .parameters()
                        .iter()
                        .map(|(_, value_type)| (frame, value_type)),
                );
                let record = record(
                    &roots,
                    Some(frame),
                    &populations,
                    lowered.boolean,
                    types,
                    position_limit,
                )
                .map_err(classify)?;
                keyed.push(KeyedClaim {
                    node: frame,
                    origin: frame_origin,
                    record,
                });
            }
        }
    }
    Ok(keyed)
}
