// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-326: the `TemporalTrace` family's S3 check of one `temporal` clause
//! (ADR-012 §12.2 Check row; ADR-014 §3 TR-3 and TR-4, §5 A-2 and A-3).
//!
//! A clause is checked under the one temporal profile its unit selects
//! (`profile.rs`'s [`TemporalProfile`] catalog). Each temporal operator's
//! interval is built into a validated [`TemporalInterval`]:
//!
//! - under a bounded profile every operator needs a closed interval, so an
//!   operator with none refuses `unsupported_construct`/`expression-form`
//!   at the operator and an `[a,*]` interval refuses the same way at the
//!   interval; the clause then has a horizon, the greatest sum of interval
//!   upper bounds along a path of the formula, computed with checked
//!   arithmetic;
//! - under `quire.temporal.infinite-trace/v1` an operator with no interval is
//!   unbounded (`None`), a closed or `[a,*]` interval is admitted, and the
//!   clause has no horizon;
//! - under every profile `a > b` refuses `ill_typed`/`type-mismatch` at the
//!   interval.
//!
//! The clause's requirement record is `temporal-satisfaction` over its
//! extent: the `over` parameter's, each `on each` parameter's and each
//! capture's type positions, plus, under infinite-trace, the formula itself
//! as one infinite-trace domain no finite bound can stand for. A clause
//! over a model object type also carries that type's population as a
//! `DomainKey::Population` domain unless the unit bounds it (FR-336-AC-1);
//! the record is computed from checked types alone, so no universe a
//! request supplies reaches it.
//!
//! FR-123 adds the fairness set and the property form. Each fairness
//! constraint is checked into one shared `FairnessConstraint` (kind,
//! resolved operation, granularity); a constraint under a bounded profile
//! refuses `unsupported_construct`/`expression-form`, and one naming no
//! operation of an admitted model refuses `missing_declaration`/`missing-name`.
//! A clause over a model subject is classified into one `PropertyForm`.
//!
//! A `holds` condition is typed by [`condition::check_conditions`] once the
//! package's signatures are known, and the clause is emitted as FR-337's
//! v2 temporal clause node. The activation's `when` and a capture's value
//! are not checked: [`not_emitted`] refuses a clause that activates `on
//! each` or declares a capture, so it is never compiled into a package
//! that silently lacks them.

use std::collections::BTreeMap;

use qsl_forms::{
    ActivationForm, FairnessConstraintForm, FairnessGranularity as FormGranularity,
    FairnessKind as FormKind, IntervalForm, IntervalUpper, ParameterForm, TemporalClauseForm,
    TemporalNodeForm, TemporalNodeId, TemporalOperator,
};
use qsl_foundation::bound::{DomainKey, DomainKind, IntervalKey, TemporalInterval, TemporalUpper};
use qsl_foundation::diagnostic::{LimitExceeded, Locus, StageFailure, Staged};
use qsl_foundation::digest::{ByteDigest, WireNodeId};
use qsl_foundation::selection::DefinitionRef;
use qsl_foundation::source::provenance::RawSourceRef;
use qsl_foundation::source_map::SourceMap;
use qsl_foundation::{InternalFault, Setting, Span};
use quire_exact::ValueType;
use quire_semantic_value::fairness::{
    FairnessConstraint, FairnessGranularity, FairnessKind, FairnessOperation, PropertyForm,
};
use quire_semantic_value::location::{Location, Origin};

use super::lowering::AdmittedModel;
use super::profile::TemporalProfile;
use super::refusal::{CheckCause, CheckRefusal, TemporalCause};
use super::state_clause::{population_of, PopulationDomain};
use super::Scope;
use crate::check::Capability;
use crate::family::requirements::{classify_domains, ClassifyFailure};
use crate::family::requirements::{ClaimExtent, Requirements};
use crate::family::{CheckContext, CheckOutcome, FamilyContract};
use crate::model::operation::OperationLookup;

pub(crate) mod condition;

/// The stage name this module's faults report.
const STAGE: &str = "check.temporal";

/// One header profile the `TemporalTrace` catalog holds (FR-110).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TemporalSelection {
    /// The profile the header selected.
    pub profile: TemporalProfile,
    /// The span of the whole `profile` declaration.
    pub span: Span,
}

/// A `temporal` clause as the assembler hands it to the check: its form and
/// the span of the whole declaration, which a clause-level refusal names.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemporalDeclaration {
    /// The clause as S2 built it.
    pub form: TemporalClauseForm,
    /// The span of the whole declaration.
    pub span: Span,
}

/// What [`TemporalTraceFamily::check`] reads of its unit (ADR-012 §2
/// `Declarations`): the type scope, the unit's temporal profile selections
/// and the source its regions resolve in.
pub struct TemporalDeclarations<'a> {
    pub(crate) scope: &'a Scope,
    pub(crate) selections: &'a [TemporalSelection],
    pub(crate) source: &'a RawSourceRef,
    pub(crate) embedding: Option<&'a SourceMap>,
    /// The type positions a requirement's extent walk may visit.
    pub(crate) position_limit: u64,
    /// The admitted models whose populations the `over` object type's
    /// population is found in (FR-336-AC-1).
    pub(crate) models: &'a [AdmittedModel],
}

/// The `TemporalTrace` family's `temporal` clause production (ADR-012
/// §15.2, FR-326).
pub struct TemporalTraceFamily;

/// One temporal operator of a checked formula.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CheckedOperator {
    /// The operator's node in the clause's formula.
    pub node: TemporalNodeId,
    /// Which operator.
    pub operator: TemporalOperator,
    /// Its interval: `Some` under a bounded profile and on an interval
    /// operator under infinite-trace, `None` on an unbounded operator and on
    /// a connective (ADR-014 TR-3).
    pub interval: Option<TemporalInterval>,
}

/// A node of a [`CheckedFormula`], by its index in the formula's arena.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FormulaNodeId(usize);

impl FormulaNodeId {
    /// The node's index in the formula's arena.
    pub fn index(self) -> usize {
        self.0
    }
}

/// A unary temporal operator (ADR-014 TR-3).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnaryTemporal {
    /// `eventually`.
    Eventually,
    /// `always`.
    Always,
    /// `once`.
    Once,
    /// `historically`.
    Historically,
}

/// A binary temporal operator (ADR-014 TR-3).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryTemporal {
    /// `until`.
    Until,
    /// `release`.
    Release,
    /// `since`.
    Since,
    /// `triggered`.
    Triggered,
}

/// One node of a [`CheckedFormula`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FormulaNode {
    /// `true` or `false`.
    Constant(bool),
    /// A `holds(e)` atom; its condition is FR-123's, so the node keeps the
    /// span of the whole `holds(e)`.
    Holds {
        /// The span of the whole `holds(e)`.
        span: Span,
    },
    /// `not`.
    Not(FormulaNodeId),
    /// `and`.
    And(FormulaNodeId, FormulaNodeId),
    /// `or`.
    Or(FormulaNodeId, FormulaNodeId),
    /// `implies`.
    Implies(FormulaNodeId, FormulaNodeId),
    /// A unary temporal operator.
    Unary {
        /// Which operator.
        operator: UnaryTemporal,
        /// Its interval; `None` on an unbounded operator.
        interval: Option<TemporalInterval>,
        /// Its operand.
        operand: FormulaNodeId,
    },
    /// A binary temporal operator.
    Binary {
        /// Which operator.
        operator: BinaryTemporal,
        /// Its interval; `None` on an unbounded operator.
        interval: Option<TemporalInterval>,
        /// Its left operand.
        left: FormulaNodeId,
        /// Its right operand.
        right: FormulaNodeId,
    },
}

impl FormulaNode {
    /// The node's operands in source order.
    pub fn operands(&self) -> impl Iterator<Item = FormulaNodeId> {
        let (first, second) = match self {
            Self::Constant(_) | Self::Holds { .. } => (None, None),
            Self::Not(only) | Self::Unary { operand: only, .. } => (Some(*only), None),
            Self::And(left, right)
            | Self::Or(left, right)
            | Self::Implies(left, right)
            | Self::Binary { left, right, .. } => (Some(*left), Some(*right)),
        };
        first.into_iter().chain(second)
    }
}

/// A checked temporal formula: an arena in which an operator precedes its
/// operands, so no walk of it recurses (ADR-030 D-4.2).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedFormula {
    nodes: Vec<FormulaNode>,
    root: FormulaNodeId,
}

impl CheckedFormula {
    /// The formula's root node.
    pub fn root(&self) -> FormulaNodeId {
        self.root
    }

    /// Every node, in arena order.
    pub fn nodes(&self) -> &[FormulaNode] {
        &self.nodes
    }

    /// The id of the node at arena `index`, when the arena has one.
    pub fn node_id(&self, index: usize) -> Option<FormulaNodeId> {
        (index < self.nodes.len()).then_some(FormulaNodeId(index))
    }
}

/// How a clause activates (FR-325 "Outputs").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Activation {
    /// `on origin`: the clause is evaluated at position 0.
    Origin,
    /// `on each`: the clause is evaluated at every activating position.
    Each {
        /// Whether the activation has a `when` condition.
        guarded: bool,
    },
}

/// The `temporal-satisfaction` claim of a clause (ADR-014 §4), before the
/// checked nodes it is keyed by exist.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemporalClaim {
    kind: Capability,
    /// The unbounded type positions of the clause's parameters, by the
    /// parameter's index (`over` first) and the child-index path.
    parameter_domains: BTreeMap<(usize, Vec<u32>), DomainKind>,
    /// Whether the formula is itself an infinite-trace domain.
    infinite_trace: bool,
    /// The population of the `over` object type when the unit declares no
    /// maximum for it: an unbounded domain of the record (FR-336-AC-1).
    population: Option<PopulationDomain>,
}

impl TemporalClaim {
    /// The capability kind the claim requests.
    pub fn kind(&self) -> Capability {
        self.kind
    }

    /// The unbounded population the clause ranges over: the `over` object
    /// type's population, when the unit declares no maximum for it. Its key
    /// is [`CheckedGraph::population_key`](crate::check::CheckedGraph::population_key).
    pub fn population(&self) -> Option<PopulationDomain> {
        self.population
    }

    /// The claim's requirement record (FR-326 "Requirement record"), keyed
    /// by the clause's checked node and each parameter's, in the order the
    /// clause declares them. `population` is the key of
    /// [`Self::population`]; the record takes no universe (FR-336-AC-1).
    pub fn requirements(
        &self,
        clause: WireNodeId,
        parameters: &[WireNodeId],
        population: Option<DomainKey>,
    ) -> Result<Requirements, InternalFault> {
        let mut domains = BTreeMap::new();
        for ((index, path), kind) in &self.parameter_domains {
            let node = parameters
                .get(*index)
                .ok_or_else(|| InternalFault::new(STAGE, "parameter-node-keyed"))?;
            domains.insert(
                DomainKey::Node {
                    node: *node,
                    path: path.clone(),
                },
                *kind,
            );
        }
        if self.infinite_trace {
            domains.insert(
                DomainKey::Node {
                    node: clause,
                    path: Vec::new(),
                },
                DomainKind::InfiniteTrace,
            );
        }
        if self.population.is_some() {
            let key =
                population.ok_or_else(|| InternalFault::new(STAGE, "population-domain-keyed"))?;
            domains.insert(key, DomainKind::Population);
        }
        Ok(Requirements::new(
            self.kind,
            ClaimExtent::from_domains(domains),
        ))
    }
}

/// A temporal clause checked by [`TemporalTraceFamily::check`] (FR-326
/// "Outputs").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedTemporalClause {
    name: String,
    span: Span,
    profile: TemporalProfile,
    profile_ref: DefinitionRef,
    /// The `over` parameter's name and resolved type.
    over: (String, ValueType),
    /// The clock role, as written.
    clock: String,
    clock_binding: WireNodeId,
    operators: Vec<CheckedOperator>,
    horizon: Option<u64>,
    claim: TemporalClaim,
    formula: CheckedFormula,
    activation: Activation,
    fairness: Vec<FairnessConstraint>,
    /// The index among the written constraints of each fairness set
    /// member's first occurrence, in set order.
    fairness_sources: Vec<usize>,
    property_form: Option<PropertyForm>,
}

impl CheckedTemporalClause {
    /// The declared clause name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The span of the clause's declaration.
    pub fn span(&self) -> Span {
        self.span
    }

    /// The one profile the clause was checked under.
    pub fn profile(&self) -> TemporalProfile {
        self.profile
    }

    /// The `over` parameter's name and resolved type.
    pub fn over(&self) -> &(String, ValueType) {
        &self.over
    }

    /// The clock role, as written (`clock "model-steps"` is `model-steps`).
    pub fn clock(&self) -> &str {
        &self.clock
    }

    /// The index among the written fairness constraints of each fairness
    /// set member's first occurrence, in set order.
    pub(crate) fn fairness_sources(&self) -> &[usize] {
        &self.fairness_sources
    }

    /// Every operator node of the formula (connectives included), in arena
    /// order, each with its interval when it is a temporal operator.
    pub fn operators(&self) -> &[CheckedOperator] {
        &self.operators
    }

    /// The formula's horizon (ADR-014 TR-4): `Some` under a bounded
    /// profile, `None` under infinite-trace.
    pub fn horizon(&self) -> Option<u64> {
        self.horizon
    }

    /// The checked formula: its nodes in arena order (an operator precedes
    /// its operands), each temporal operator holding its validated interval.
    pub fn formula(&self) -> &CheckedFormula {
        &self.formula
    }

    /// How the clause activates (FR-325 "Outputs").
    pub fn activation(&self) -> Activation {
        self.activation
    }

    /// The clause's fairness set, each constraint once, in source order of
    /// first occurrence (FR-123).
    pub fn fairness(&self) -> &[FairnessConstraint] {
        &self.fairness
    }

    /// The clause's property form (FR-123 "Property form"); `None` for a
    /// clause that is not over a model subject, or that selects the
    /// fixed-sample or timestamped-event profile.
    pub fn property_form(&self) -> Option<PropertyForm> {
        self.property_form
    }

    /// The QSpec FR-255 key of the interval `node` carries, under the
    /// clause's profile and clock binding; `None` when `node` carries no
    /// finite interval.
    pub fn interval_key(&self, node: TemporalNodeId) -> Option<IntervalKey> {
        let operator = self
            .operators
            .iter()
            .find(|operator| operator.node == node)?;
        operator
            .interval?
            .key(self.profile_ref.clone(), self.clock_binding)
    }
}

impl FamilyContract for TemporalTraceFamily {
    type Form = TemporalDeclaration;
    type Checked = CheckedTemporalClause;
    type Cause = CheckRefusal;
    type Declarations<'a> = TemporalDeclarations<'a>;
    type Claim = TemporalClaim;

    fn check<'a>(
        form: &TemporalDeclaration,
        cx: &mut CheckContext<'a, TemporalDeclarations<'a>>,
    ) -> CheckOutcome<CheckedTemporalClause, CheckRefusal> {
        check_clause(form, cx.declarations()).map(Staged::new)
    }

    fn requirements(checked: &CheckedTemporalClause) -> Vec<TemporalClaim> {
        vec![checked.claim.clone()]
    }
}

fn refuse(cause: TemporalCause) -> StageFailure<CheckRefusal> {
    StageFailure::Refused(refusal(cause))
}

fn refusal(cause: TemporalCause) -> CheckRefusal {
    CheckRefusal {
        // A clause has no expression tree for a `Location` path to walk;
        // `region::DeclarationRegions::refusal_region` reads a `Temporal`
        // cause's own span directly, so this `Location` is never resolved.
        location: Location::root(Origin::Expression),
        cause: CheckCause::Temporal(Box::new(cause)),
    }
}

fn fault(invariant: &'static str) -> StageFailure<CheckRefusal> {
    StageFailure::Fault(InternalFault::new(STAGE, invariant))
}

/// Whether `operator` is a temporal operator, which an interval may
/// qualify, rather than a connective.
const fn is_temporal(operator: TemporalOperator) -> bool {
    match operator {
        TemporalOperator::Eventually
        | TemporalOperator::Always
        | TemporalOperator::Once
        | TemporalOperator::Historically
        | TemporalOperator::Until
        | TemporalOperator::Release
        | TemporalOperator::Since
        | TemporalOperator::Triggered => true,
        TemporalOperator::Not
        | TemporalOperator::And
        | TemporalOperator::Or
        | TemporalOperator::Implies => false,
    }
}

/// The clock binding's node id. Recorded exception to keying on the string
/// itself: `IntervalKey::clock_binding` is a `WireNodeId` (QSpec FR-255's
/// structural clock-binding node, fixed by ADR-014 and `qsl_foundation::bound`),
/// a 32-byte id that cannot hold the `clock` string, so the id is the digest of
/// the string's length-prefixed bytes. Equal strings bind equally. FR-233's
/// `model-steps`/`model-time` binding node replaces it.
fn clock_binding(text: &str) -> WireNodeId {
    let mut preimage = Vec::with_capacity(text.len() + 8);
    preimage.extend_from_slice(&u64::try_from(text.len()).unwrap_or(u64::MAX).to_be_bytes());
    preimage.extend_from_slice(text.as_bytes());
    WireNodeId::from_digest(ByteDigest::of(&preimage).as_bytes())
}

/// A temporal operator's interval built from its form: the validated
/// interval, or the refusal that `a > b` gives under every profile.
fn build_interval(interval: &IntervalForm) -> Result<TemporalInterval, TemporalCause> {
    let upper = match interval.upper {
        IntervalUpper::Finite(upper) => TemporalUpper::Finite(upper),
        IntervalUpper::Open => TemporalUpper::Open,
    };
    TemporalInterval::new(interval.lower, upper).map_err(|inverted| {
        TemporalCause::InvertedInterval {
            lower: inverted.lower,
            upper: inverted.upper,
            span: interval.span,
        }
    })
}

/// The parameters whose types the clause's extent ranges over: `over`, the
/// `on each` parameter and each capture, in that order.
fn parameters(form: &TemporalClauseForm) -> Vec<&ParameterForm> {
    let mut parameters = vec![&form.over];
    if let ActivationForm::Each { parameter, .. } = &form.activation {
        parameters.push(parameter);
    }
    parameters.extend(form.captures.iter().map(|capture| &capture.parameter));
    parameters
}

/// The region of `span` as a located stage limit.
fn located(
    limit: LimitExceeded,
    declarations: &TemporalDeclarations<'_>,
    span: Span,
) -> StageFailure<CheckRefusal> {
    let region = super::region::region(declarations.source, declarations.embedding, span);
    StageFailure::Limit(limit.at(region.map(Locus::Region)))
}

fn check_clause(
    declaration: &TemporalDeclaration,
    declarations: &TemporalDeclarations<'_>,
) -> Result<CheckedTemporalClause, StageFailure<CheckRefusal>> {
    let form = &declaration.form;
    let profile = match declarations.selections {
        [only] => only.profile,
        found => {
            return Err(refuse(TemporalCause::ProfileSelection {
                found: found
                    .iter()
                    .map(|selection| selection.profile.identity())
                    .collect(),
                span: declaration.span,
            }))
        }
    };
    let profile_ref =
        DefinitionRef::new(super::profile::CLAUSE_PROFILE_AUTHORITY, profile.identity())
            .map_err(|_| fault("profile-identity-valid"))?;

    let mut types = Vec::new();
    for parameter in parameters(form) {
        let resolved = super::type_form::resolve_form(declarations.scope, &parameter.value_type)
            .map_err(|error| {
                refuse(TemporalCause::ParameterType {
                    parameter: parameter.name.name.clone(),
                    fault: error.fault,
                    span: error.span,
                })
            })?;
        types.push(resolved);
    }

    let bounded = profile.is_bounded();
    let mut operators = Vec::new();
    let mut refusals: Vec<TemporalCause> = Vec::new();
    for (id, node) in form.formula.iter() {
        let TemporalNodeForm::Operator(operator) = node else {
            continue;
        };
        let interval = match (&operator.interval, is_temporal(operator.operator)) {
            (Some(_), false) => return Err(fault("connective-carries-no-interval")),
            (None, true) if bounded => {
                refusals.push(TemporalCause::MissingInterval {
                    span: operator.operator_span,
                });
                None
            }
            (None, _) => None,
            (Some(written), true) => match build_interval(written) {
                Err(cause) => {
                    refusals.push(cause);
                    None
                }
                Ok(interval) => {
                    if bounded && interval.upper() == TemporalUpper::Open {
                        refusals.push(TemporalCause::OpenInterval { span: written.span });
                    }
                    Some(interval)
                }
            },
        };
        operators.push(CheckedOperator {
            node: id,
            operator: operator.operator,
            interval,
        });
    }
    let (fairness, fairness_sources) =
        checked_fairness(form, bounded, declarations, types.first(), &mut refusals);
    if let Some(first) = refusals.into_iter().min_by_key(|cause| cause.span().start) {
        return Err(refuse(first));
    }

    let horizon = if bounded {
        Some(formula_horizon(declaration, declarations)?)
    } else {
        None
    };

    let roots: Vec<&ValueType> = types.iter().collect();
    let parameter_domains = classify_domains(
        &roots,
        declarations.scope.types(),
        declarations.position_limit,
    )
    .map_err(|failure| match failure {
        ClassifyFailure::Limit(limit) => located(limit, declarations, declaration.span),
        ClassifyFailure::Fault(fault) => StageFailure::Fault(fault),
    })?;

    let intervals: BTreeMap<TemporalNodeId, Option<TemporalInterval>> = operators
        .iter()
        .map(|operator| (operator.node, operator.interval))
        .collect();
    let formula = build_formula(form, &intervals)?;
    let model_subject = matches!(types.first(), Some(ValueType::Reference(_)));
    let population = match types.first() {
        Some(ValueType::Reference(object)) => {
            population_of(declarations.models, *object, declarations.scope.types())
                .map_err(|populations| {
                    refuse(TemporalCause::AmbiguousPopulation {
                        populations,
                        span: declaration.span,
                    })
                })?
                .filter(|domain| domain.maximum.is_none())
        }
        _ => None,
    };
    let property_form = property_form(profile, model_subject, &formula);
    let over_type = types
        .into_iter()
        .next()
        .ok_or_else(|| fault("over-type-resolved"))?;

    Ok(CheckedTemporalClause {
        name: form.name.name.clone(),
        span: declaration.span,
        profile,
        profile_ref,
        over: (form.over.name.name.clone(), over_type),
        clock: form.clock.text.clone(),
        clock_binding: clock_binding(&form.clock.text),
        operators,
        horizon,
        claim: TemporalClaim {
            kind: Capability::TemporalSatisfaction,
            parameter_domains,
            infinite_trace: !bounded,
            population,
        },
        formula,
        activation: match &form.activation {
            ActivationForm::Origin => Activation::Origin,
            ActivationForm::Each { when, .. } => Activation::Each {
                guarded: when.is_some(),
            },
        },
        fairness,
        fairness_sources,
        property_form,
    })
}

/// The checked fairness set (FR-123, FR-129): each constraint once, in
/// source order of first occurrence. An unmarked kind is `weak` and an
/// unmarked granularity is `whole` (ADR-018 FA-6). A constraint under a
/// bounded profile, or naming no operation of an admitted model, pushes its
/// refusal onto `refusals` and joins no set. Each member comes with the
/// index of the written constraint it was first read from.
fn checked_fairness(
    form: &TemporalClauseForm,
    bounded: bool,
    declarations: &TemporalDeclarations<'_>,
    over: Option<&ValueType>,
    refusals: &mut Vec<TemporalCause>,
) -> (Vec<FairnessConstraint>, Vec<usize>) {
    let mut set: Vec<FairnessConstraint> = Vec::new();
    let mut sources = Vec::new();
    for (index, written) in form.fairness.iter().enumerate() {
        if bounded {
            refusals.push(TemporalCause::FairnessUnderBoundedProfile { span: written.span });
            continue;
        }
        let operation = match resolve_fairness_operation(written, declarations.scope, over) {
            Ok(operation) => operation,
            Err(cause) => {
                refusals.push(cause);
                continue;
            }
        };
        let constraint = FairnessConstraint::new(
            match written.kind {
                None | Some(FormKind::Weak) => FairnessKind::Weak,
                Some(FormKind::Strong) => FairnessKind::Strong,
            },
            operation,
            match written.granularity {
                None | Some(FormGranularity::Whole) => FairnessGranularity::Whole,
                Some(FormGranularity::Each) => FairnessGranularity::Each,
            },
        );
        if !set.contains(&constraint) {
            set.push(constraint);
            sources.push(index);
        }
    }
    (set, sources)
}

/// The operation a fairness constraint names, resolved in an admitted
/// model object type's effective view (FR-103): `Model::Type::operation`
/// names the type, and an operation written alone is read in the view of the
/// clause's `over` object type. The operation keeps its declaring type, so
/// same-named operations of different types stay distinct.
fn resolve_fairness_operation(
    written: &FairnessConstraintForm,
    scope: &Scope,
    over: Option<&ValueType>,
) -> Result<FairnessOperation, TemporalCause> {
    let name = &written.operation;
    let (object, operation) = match name.name.rsplit_once("::") {
        Some((object, operation)) => (scope.object_type_named(object), operation),
        None => (
            match over {
                Some(ValueType::Reference(object)) => Some(*object),
                _ => None,
            },
            name.name.as_str(),
        ),
    };
    let unresolved = || TemporalCause::UnresolvedFairnessOperation {
        name: name.name.clone(),
        span: name.span,
    };
    let object = object.ok_or_else(unresolved)?;
    match scope.operations().resolve(scope.types(), object, operation) {
        OperationLookup::Declared {
            declaring,
            operation,
        } => Ok(FairnessOperation::new(
            declaring,
            operation.name().to_owned(),
        )),
        OperationLookup::Missing => Err(unresolved()),
        OperationLookup::Ambiguous(_) => Err(TemporalCause::AmbiguousFairnessOperation {
            name: name.name.clone(),
            span: name.span,
        }),
    }
}

/// The clause's property form (FR-123 "Property form", ADR-018 TP-1 to
/// TP-4), in the order the requirement states; `None` for a clause that is
/// not over a model subject and for the fixed-sample and timestamped-event
/// profiles, which negotiation settles unsupported.
fn property_form(
    profile: TemporalProfile,
    model_subject: bool,
    formula: &CheckedFormula,
) -> Option<PropertyForm> {
    if !model_subject {
        return None;
    }
    match profile {
        TemporalProfile::FixedSampleFalseExtension
        | TemporalProfile::TimestampedEventFiniteWindow => return None,
        TemporalProfile::EventPositionFalseExtension | TemporalProfile::InfiniteTrace => {}
    }
    if is_reachable_invariant(formula) {
        Some(PropertyForm::ReachableInvariant)
    } else if profile == TemporalProfile::EventPositionFalseExtension {
        Some(PropertyForm::BoundedMltl)
    } else if in_safety_fragment(formula) {
        Some(PropertyForm::Safety)
    } else {
        Some(PropertyForm::Liveness)
    }
}

/// Whether the formula is `always holds(I)` with no interval (TP-1).
fn is_reachable_invariant(formula: &CheckedFormula) -> bool {
    let nodes = formula.nodes();
    let Some(FormulaNode::Unary {
        operator: UnaryTemporal::Always,
        interval: None,
        operand,
    }) = nodes.get(formula.root().index())
    else {
        return false;
    };
    matches!(nodes.get(operand.index()), Some(FormulaNode::Holds { .. }))
}

/// Whether the interval is closed: the one an interval operator needs to
/// stay within the safety fragment when its operator is `eventually` or
/// `until` (ADR-018 IV-4).
fn is_closed(interval: Option<TemporalInterval>) -> bool {
    matches!(
        interval.map(|interval| interval.upper()),
        Some(TemporalUpper::Finite(_))
    )
}

/// Whether the formula, in negation normal form, is in ADR-014 A-4's safety
/// fragment as ADR-018 IV-4 amends it (TP-3): atoms, `and`, `or`, `always`,
/// `release`, the past operators, and `eventually` and `until` only with a
/// closed interval. A top-down pass tracks each node's polarity, so `not`
/// and `implies` are read as negation normal form would write them: a
/// negated `eventually` is an `always`, a negated `until` a `release`, and
/// the reverse. The arena puts an operator before its operands, so one pass
/// reaches every node after its parent.
fn in_safety_fragment(formula: &CheckedFormula) -> bool {
    let nodes = formula.nodes();
    let mut negated = vec![false; nodes.len()];
    for (index, node) in nodes.iter().enumerate() {
        let here = negated.get(index).copied().unwrap_or(false);
        let (admitted, first, second) = match node {
            FormulaNode::Constant(_)
            | FormulaNode::Holds { .. }
            | FormulaNode::And(..)
            | FormulaNode::Or(..) => (true, here, here),
            FormulaNode::Not(_) | FormulaNode::Implies(..) => (true, !here, here),
            FormulaNode::Unary {
                operator, interval, ..
            } => {
                let admitted = match operator {
                    UnaryTemporal::Once | UnaryTemporal::Historically => true,
                    UnaryTemporal::Always => !here || is_closed(*interval),
                    UnaryTemporal::Eventually => here || is_closed(*interval),
                };
                (admitted, here, here)
            }
            FormulaNode::Binary {
                operator, interval, ..
            } => {
                let admitted = match operator {
                    BinaryTemporal::Since | BinaryTemporal::Triggered => true,
                    BinaryTemporal::Release => !here || is_closed(*interval),
                    BinaryTemporal::Until => here || is_closed(*interval),
                };
                (admitted, here, here)
            }
        };
        if !admitted {
            return false;
        }
        for (operand, polarity) in node.operands().zip([first, second]) {
            if let Some(slot) = negated.get_mut(operand.index()) {
                *slot = polarity;
            }
        }
    }
    true
}

/// The checked formula of `form`: the arena order is kept, so an operator
/// still precedes its operands.
fn build_formula(
    form: &TemporalClauseForm,
    intervals: &BTreeMap<TemporalNodeId, Option<TemporalInterval>>,
) -> Result<CheckedFormula, StageFailure<CheckRefusal>> {
    let node = |id: &TemporalNodeId| FormulaNodeId(id.index());
    let mut nodes = Vec::with_capacity(form.formula.nodes().len());
    for (id, written) in form.formula.iter() {
        let checked = match written {
            TemporalNodeForm::Constant { value, .. } => FormulaNode::Constant(*value),
            TemporalNodeForm::Holds { span, .. } => FormulaNode::Holds { span: *span },
            TemporalNodeForm::Operator(operator) => {
                let interval = *intervals
                    .get(&id)
                    .ok_or_else(|| fault("operator-has-checked-interval"))?;
                match (operator.operator, operator.operands.as_slice()) {
                    (TemporalOperator::Not, [only]) => FormulaNode::Not(node(only)),
                    (TemporalOperator::And, [left, right]) => {
                        FormulaNode::And(node(left), node(right))
                    }
                    (TemporalOperator::Or, [left, right]) => {
                        FormulaNode::Or(node(left), node(right))
                    }
                    (TemporalOperator::Implies, [left, right]) => {
                        FormulaNode::Implies(node(left), node(right))
                    }
                    (TemporalOperator::Eventually, [operand]) => {
                        unary(UnaryTemporal::Eventually, interval, node(operand))
                    }
                    (TemporalOperator::Always, [operand]) => {
                        unary(UnaryTemporal::Always, interval, node(operand))
                    }
                    (TemporalOperator::Once, [operand]) => {
                        unary(UnaryTemporal::Once, interval, node(operand))
                    }
                    (TemporalOperator::Historically, [operand]) => {
                        unary(UnaryTemporal::Historically, interval, node(operand))
                    }
                    (TemporalOperator::Until, [left, right]) => {
                        binary(BinaryTemporal::Until, interval, node(left), node(right))
                    }
                    (TemporalOperator::Release, [left, right]) => {
                        binary(BinaryTemporal::Release, interval, node(left), node(right))
                    }
                    (TemporalOperator::Since, [left, right]) => {
                        binary(BinaryTemporal::Since, interval, node(left), node(right))
                    }
                    (TemporalOperator::Triggered, [left, right]) => {
                        binary(BinaryTemporal::Triggered, interval, node(left), node(right))
                    }
                    (
                        TemporalOperator::Not
                        | TemporalOperator::And
                        | TemporalOperator::Or
                        | TemporalOperator::Implies
                        | TemporalOperator::Eventually
                        | TemporalOperator::Always
                        | TemporalOperator::Once
                        | TemporalOperator::Historically
                        | TemporalOperator::Until
                        | TemporalOperator::Release
                        | TemporalOperator::Since
                        | TemporalOperator::Triggered,
                        _,
                    ) => return Err(fault("operator-arity")),
                }
            }
        };
        nodes.push(checked);
    }
    let root = FormulaNodeId(form.formula.root().index());
    for (index, node) in nodes.iter().enumerate() {
        if node
            .operands()
            .any(|operand| operand.0 <= index || operand.0 >= nodes.len())
        {
            return Err(fault("operands-follow-operator"));
        }
    }
    if root.0 >= nodes.len() {
        return Err(fault("root-in-arena"));
    }
    Ok(CheckedFormula { nodes, root })
}

fn unary(
    operator: UnaryTemporal,
    interval: Option<TemporalInterval>,
    operand: FormulaNodeId,
) -> FormulaNode {
    FormulaNode::Unary {
        operator,
        interval,
        operand,
    }
}

fn binary(
    operator: BinaryTemporal,
    interval: Option<TemporalInterval>,
    left: FormulaNodeId,
    right: FormulaNodeId,
) -> FormulaNode {
    FormulaNode::Binary {
        operator,
        interval,
        left,
        right,
    }
}

/// A bounded formula's horizon (ADR-014 TR-4): the greatest sum of interval
/// upper bounds along a path from the root. Each node's operands follow it
/// in the arena, so one reverse pass computes every reach from its
/// operands'. A sum past `u64` is a work-budget stage limit located at the
/// operator whose interval overflowed.
fn formula_horizon(
    declaration: &TemporalDeclaration,
    declarations: &TemporalDeclarations<'_>,
) -> Result<u64, StageFailure<CheckRefusal>> {
    let formula = &declaration.form.formula;
    let nodes = formula.nodes();
    let mut reach = vec![0_u64; nodes.len()];
    for (index, node) in nodes.iter().enumerate().rev() {
        let TemporalNodeForm::Operator(operator) = node else {
            continue;
        };
        let mut deepest = 0_u64;
        for operand in &operator.operands {
            if operand.index() <= index {
                return Err(fault("operands-follow-operator"));
            }
            deepest = deepest.max(
                *reach
                    .get(operand.index())
                    .ok_or_else(|| fault("operand-in-arena"))?,
            );
        }
        let own = match operator.interval.map(|interval| interval.upper) {
            Some(IntervalUpper::Finite(upper)) => upper,
            Some(IntervalUpper::Open) | None => 0,
        };
        let Some(total) = deepest.checked_add(own) else {
            let limit = LimitExceeded::new(
                Setting::S3WorkUnits,
                u64::MAX,
                u128::from(deepest) + u128::from(own),
            );
            return Err(located(limit, declarations, operator.operator_span));
        };
        *reach
            .get_mut(index)
            .ok_or_else(|| fault("operator-in-arena"))? = total;
    }
    reach
        .get(formula.root().index())
        .copied()
        .ok_or_else(|| fault("root-in-arena"))
}

/// Whether S4 emits `form`'s clause (FR-337): one that activates `on
/// origin` and declares no capture. The `on each` activation and the
/// captures are not checked, so a clause with either is [`not_emitted`].
pub(crate) fn is_emitted(form: &TemporalClauseForm) -> bool {
    matches!(form.activation, ActivationForm::Origin) && form.captures.is_empty()
}

/// The refusal for a clause that checked but that no S4 emission writes
/// (an `on each` activation or a capture, [`is_emitted`]): refused rather
/// than compiled and silently missing from the package.
pub fn not_emitted(checked: &CheckedTemporalClause) -> CheckRefusal {
    refusal(TemporalCause::NotEmitted {
        name: checked.name.clone(),
        span: checked.span,
    })
}

/// Every `temporal` clause of `package`, checked by the family alone, as
/// the package check runs it before its `holds` conditions are typed
/// (test-support only).
#[cfg(any(test, feature = "test-support"))]
pub fn check_clauses(
    package: &crate::check::PackageDeclarations,
) -> Vec<Result<CheckedTemporalClause, StageFailure<CheckRefusal>>> {
    let scope = Scope::new(
        package.types.clone(),
        package.operations.clone(),
        package.enums.clone(),
        package.aliases.clone(),
        Vec::new(),
        None,
        Vec::new(),
    );
    let declarations = TemporalDeclarations {
        scope: &scope,
        selections: &package.temporal_selections,
        source: &package.source,
        embedding: None,
        position_limit: 10_000,
        models: &package.models,
    };
    package
        .temporal_clauses
        .iter()
        .map(|clause| {
            let mut meter = quire_exact::Meter::new(crate::check::family::SCALAR_LIMITS_UNLIMITED);
            let mut diagnostics = crate::family::DiagnosticSink::default();
            let mut scopes = crate::family::ScopeStack::default();
            let mut cx = CheckContext::new(
                &declarations,
                crate::family::StageLimits {
                    input_bytes: u64::MAX,
                    node_count: u64::MAX,
                },
                &mut meter,
                &mut diagnostics,
                &mut scopes,
            );
            TemporalTraceFamily::check(clause, &mut cx).map(Staged::into_value)
        })
        .collect()
}

#[cfg(test)]
mod tests;
