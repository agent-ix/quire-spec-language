// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-172, FR-173, FR-174, FR-180: the `TemporalTrace` family's S3 check of a
//! `hyper` clause over `behaviours` and of a `relation` over model
//! executions (ADR-023 sections 1, 3, 14 and 16).
//!
//! A `behaviours` clause binds each trace variable to a model alias, each
//! object parameter to a population of a bound alias, types its indexed atoms
//! and step labels, splits its `match` block into the conjuncts that read
//! universal variables only and the rest, checks each variable's fairness set
//! and the `align skip` list, and is classified into one [`HyperForm`]. A
//! clause of the universal forms also carries the [`CopyGroup`] of its
//! swappable universal variables. A `relation` resolves each execution
//! binding to a model operation; when every bound operation has an
//! abstraction-relation binding it also has a second requirement, the code
//! claim.
//!
//! A clause that checks is refused as not emitted until S4 writes its node,
//! as a `temporal` clause is (`temporal_clause::not_emitted`).

use qsl_foundation::source::node_table::NodeKeyDebug;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use qsl_forms::{
    BinaryOperator, ExprId, ExprNode, ExprRef, Expression, ExpressionSpans, HyperBehavioursForm,
    HyperFiniteForm, IntervalUpper, ModelRelationForm, NameForm, OperatorInterval, QuantifierForm,
    QuantifierKind, SpanId, StepLabelForm, TemporalFormulaForm, TemporalNodeForm, TemporalOperator,
    TypeFormHead,
};
use qsl_foundation::bound::{DomainKey, DomainKind};
use qsl_foundation::diagnostic::{Code, StageFailure, Staged};
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::Span;
use quire_exact::{EffectiveId, Identifier, NodeKey, ValueType};
use quire_semantic_value::catalog::InternalFault;
use quire_semantic_value::fairness::{FairnessConstraint, FairnessOperation};
use quire_semantic_value::location::{Location, Origin};

use super::abstraction::{AbstractionKey, CheckedAbstractionRelation, OperationKey};
use super::check::{HyperTyping, Signatures, Typer};
use super::ir::{CheckedBody, CheckedNode};
use super::profile::TemporalProfile;
use super::refusal::{CheckCause, CheckRefusal, Obligation, TemporalCause};
use super::state_clause::{population_of, PopulationDomain};
use super::temporal_clause::FormulaNodeId;
use super::temporal_clause::{
    build_formula, checked_constraint, checked_operators, in_safety_fragment, CheckedFormula,
    CheckedOperator, IntervalReading, TemporalClaim, TemporalDeclarations, TemporalSelection,
};
use super::Scope;
use super::{AdmittedModel, Capability};
use crate::family::requirements::{classify_domains, ClaimExtent, ClassifyFailure, Requirements};
use crate::family::{CheckContext, CheckOutcome, FamilyContract};
use crate::model::operation::OperationLookup;

pub(crate) mod emission;
pub(crate) mod lowering;

pub use lowering::{operation_text, AtomSlot, STUTTER_TEXT};

/// The stage name this module's faults report.
const STAGE: &str = "check.hyper";

pub(super) fn declaration_has_ieee_literal(declaration: &HyperDeclaration) -> bool {
    match declaration {
        HyperDeclaration::Behaviours(form) => {
            form.match_block
                .as_ref()
                .is_some_and(|block| block.expression.contains_ieee_literal())
                || form.body.nodes().iter().any(|node| match node {
                    qsl_forms::TemporalNodeForm::Holds { condition, .. } => {
                        condition.expression.contains_ieee_literal()
                    }
                    qsl_forms::TemporalNodeForm::AggregateAtom(atom) => {
                        atom.term
                            .value
                            .as_ref()
                            .is_some_and(|value| value.expression.contains_ieee_literal())
                            || atom
                                .term
                                .filter
                                .as_ref()
                                .is_some_and(|filter| filter.expression.contains_ieee_literal())
                            || atom.threshold.expression.contains_ieee_literal()
                    }
                    qsl_forms::TemporalNodeForm::Constant { .. }
                    | qsl_forms::TemporalNodeForm::Operator(_) => false,
                })
        }
        HyperDeclaration::Finite(form) => form.body.expression.contains_ieee_literal(),
        HyperDeclaration::Relation(form) => form.body.expression.contains_ieee_literal(),
    }
}

/// A `hyper` clause or `relation` as the assembler hands it to the check.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HyperDeclaration {
    /// A `hyper` clause over `behaviours`.
    Behaviours(HyperBehavioursForm),
    /// A `hyper` clause over a declared finite trace domain.
    Finite(HyperFiniteForm),
    /// A `relation` over named executions.
    Relation(ModelRelationForm),
}

impl HyperDeclaration {
    /// The `using` alias the declaration selects its profile by.
    pub(crate) fn profile(&self) -> &qsl_forms::UsingAlias {
        match self {
            Self::Behaviours(form) => &form.profile,
            Self::Finite(form) => &form.profile,
            Self::Relation(form) => &form.profile,
        }
    }
}

/// What [`HyperTraceFamily::check`] reads of its unit: the type scope, the
/// unit's temporal profile selections, the source its regions resolve in and
/// the admitted models.
pub struct HyperDeclarations<'a> {
    pub(crate) temporal: TemporalDeclarations<'a>,
    pub(crate) models: &'a [AdmittedModel],
    /// The unit's declared functions, which a body's calls resolve against.
    pub(crate) signatures: &'a Signatures,
    /// The unit's dispatch tables, which a condition's definedness reads.
    pub(crate) dispatch_tables: &'a [super::DispatchTable],
    /// The unit's checking limits, which a condition is typed under.
    pub(crate) checking_limits: quire_semantic_value::checking::CheckingLimits,
    /// The caller's cancellation handle, polled at every node charge.
    pub(crate) cancel: Option<&'a quire_exact::Cancel>,
    /// The package's running expression-node total: the total before the
    /// clause checked next, and the total after the clause checked last.
    pub(crate) nodes_used: std::cell::Cell<u64>,
    /// How many `behaviours` clauses have been checked: the index of the one
    /// checked next among the unit's hyper clauses over `behaviours`.
    pub(crate) behaviours_checked: std::cell::Cell<usize>,
}

/// The `TemporalTrace` family's `hyper` and `relation` productions (ADR-023
/// DS-1).
pub struct HyperTraceFamily;

/// A model alias a `hyper` clause binds a trace variable to.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ModelAlias(String);

impl ModelAlias {
    /// The alias as written.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One object parameter `v: V::T`, bound to a population of its alias's model
/// (FR-172 "Object parameters").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectParameter {
    /// The parameter's name.
    pub name: String,
    /// The alias whose model declares the population.
    pub alias: ModelAlias,
    /// The population the parameter's objects are drawn from.
    pub population: PopulationDomain,
}

/// One checked quantifier: its kind, variable, model alias and fairness set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedQuantifier {
    /// Universal or existential.
    pub kind: QuantifierKind,
    /// The trace variable.
    pub variable: String,
    /// The model whose behaviours the variable ranges over.
    pub alias: ModelAlias,
    /// The variable's fairness set, each constraint once, in source order of
    /// first occurrence.
    pub fairness: Vec<FairnessConstraint>,
}

impl CheckedQuantifier {
    /// Whether the quantifier is `forall`.
    pub fn is_universal(&self) -> bool {
        self.kind == QuantifierKind::Forall
    }
}

/// One step label a `match` conjunct reads, typed by its trace's model
/// (FR-172 "Indexed atoms and step labels").
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StepRead {
    /// `t.step.op`: the operation of the step into the current position, or
    /// the stutter label.
    Operation {
        /// The trace variable.
        trace: String,
    },
    /// `t.step.receiver`: the step's receiver key.
    Receiver {
        /// The trace variable.
        trace: String,
        /// The one object type `T` that declares every operation of the
        /// trace's model that has a receiver: the key is `Reference<T>`.
        object: EffectiveId,
    },
    /// `t.step.O.p`: the argument `p` of operation `O`, of type `Option<T>`
    /// for `p: T`.
    Argument(Box<ArgumentRead>),
    /// `stutter`.
    Stutter,
}

/// The argument of a step a `match` conjunct reads.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArgumentRead {
    /// The trace variable.
    pub trace: String,
    /// The operation `O`.
    pub operation: OperationKey,
    /// The operation as the label writes it.
    pub written: String,
    /// The parameter `p`.
    pub parameter: String,
    /// The parameter's index among the operation's parameters.
    pub index: usize,
    /// The parameter's type `T`.
    pub parameter_type: ValueType,
}

impl StepRead {
    /// The name the read's slot is bound under in a conjunct's condition;
    /// `None` for `stutter`, which is a literal.
    pub(crate) fn slot_name(&self) -> Option<String> {
        let (trace, label) = match self {
            Self::Operation { trace } => (trace, StepLabelForm::Operation),
            Self::Receiver { trace, .. } => (trace, StepLabelForm::Receiver),
            Self::Argument(argument) => (
                &argument.trace,
                StepLabelForm::Argument {
                    operation: argument.written.clone(),
                    parameter: argument.parameter.clone(),
                },
            ),
            Self::Stutter => return None,
        };
        Some(lowering::step_slot_name(trace, &label))
    }

    /// The type of the read's slot: a `String` for an operation, the
    /// receiver key or the argument `Option<T>` otherwise (FR-487 "Lowered
    /// form").
    pub(crate) fn slot_type(&self) -> Option<ValueType> {
        match self {
            Self::Operation { .. } => Some(ValueType::String),
            Self::Receiver { object, .. } => Some(ValueType::Option(std::sync::Arc::new(
                ValueType::Reference(object.clone()),
            ))),
            Self::Argument(argument) => Some(argument.read_type()),
            Self::Stutter => None,
        }
    }
}

impl ArgumentRead {
    /// The type of the read: `Option<T>`, present exactly when the step is an
    /// `O` step.
    pub fn read_type(&self) -> ValueType {
        ValueType::Option(std::sync::Arc::new(self.parameter_type.clone()))
    }
}

/// One top-level conjunct of a `match` block (ADR-023 HM-3).
#[derive(Clone, Debug, PartialEq)]
pub struct CheckedStepPredicate {
    root: ExprId,
    span: Span,
    variables: Vec<String>,
    reads: Vec<StepRead>,
    slots: Vec<StepRead>,
    condition: lowering::ConjunctCondition,
}

impl CheckedStepPredicate {
    /// The conjunct's root node in [`CheckedHyperClause::match_expression`].
    pub fn root(&self) -> ExprId {
        self.root
    }

    /// The conjunct's span.
    pub fn span(&self) -> Span {
        self.span
    }

    /// The trace variables the conjunct reads, in order of first read.
    pub fn variables(&self) -> &[String] {
        &self.variables
    }

    /// The typed step-label reads of the conjunct, in source order.
    pub fn reads(&self) -> &[StepRead] {
        &self.reads
    }

    /// The conjunct's distinct step-label reads, in order of first read;
    /// slot `k` of its condition is entry `k` (FR-487). `stutter` is not a
    /// read.
    pub fn slots(&self) -> &[StepRead] {
        &self.slots
    }

    /// The conjunct's checked condition and its slot count.
    pub fn condition(&self) -> (CheckedNode<'_>, usize) {
        (self.condition.body.root(), self.condition.slots)
    }

    /// The conjunct's lowered condition with the names of its slots.
    pub(crate) fn lowered(&self) -> &lowering::ConjunctCondition {
        &self.condition
    }
}

/// The form ADR-023 section 3 classifies a clause into.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HyperForm {
    /// HP-1: a `relation` over model executions.
    StepRelation,
    /// HP-2: every quantifier universal, aligned in lockstep.
    Universal,
    /// HP-3: universal quantifiers, then existential ones, over a safety body.
    ForallExistsSafety,
    /// HP-4: a form negotiation settles V-8.
    Other {
        /// The rule that placed the clause here.
        reason: HyperUnsupported,
    },
    /// HP-5: exactly one existential quantifier.
    SingleExistential,
    /// HP-6: every quantifier universal, aligned on a projection.
    ProjectionAligned,
}

impl HyperForm {
    /// The HP-4 classification reason, if this form is unsupported by every
    /// negotiation arm (FR-396, FR-399).
    pub const fn unsupported_reason(&self) -> Option<HyperUnsupported> {
        match self {
            Self::Other { reason } => Some(*reason),
            Self::StepRelation
            | Self::Universal
            | Self::ForallExistsSafety
            | Self::SingleExistential
            | Self::ProjectionAligned => None,
        }
    }
}

/// Which rule of ADR-023 section 3 places a clause in HP-4.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum HyperUnsupported {
    /// An `exists` before a `forall`.
    ExistentialBeforeUniversal,
    /// An `exists` beside another quantifier, over a body outside the safety
    /// fragment or with a non-empty fairness set.
    ExistentialOutsideSafety,
    /// `align skip` with an `exists`, or over a body outside the safety
    /// fragment.
    ProjectionOutsideSafety,
}

/// The classes of universal variables that can be swapped without changing the
/// clause (FR-174 "Copy-swap detection"): empty when none can.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CopyGroup {
    /// Each class of two or more swappable variables, in quantifier order.
    pub classes: Vec<Vec<String>>,
    /// One generator for each pair of consecutive variables of a class, which
    /// together generate the symmetric group of the class. EN-1 permutes the
    /// states of the body's automaton with them (FR-174 "Canonicalisation").
    pub generators: Vec<CopyGenerator>,
}

/// The exchange of two universal variables, as it acts on the body's atoms.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CopyGenerator {
    /// The position in quantifier order of the first variable.
    pub first: usize,
    /// The position in quantifier order of the second variable.
    pub second: usize,
    /// For each `holds` atom of the body, in arena order, the atom whose
    /// normal form is its normal form with the two variables exchanged. An
    /// involution.
    pub atoms: Vec<usize>,
}

/// Everything that changes a clause's meaning (ADR-023 HV-7): the subject
/// aliases in order, the object parameters, the quantifier prefix with each
/// variable's kind, alias and fairness set, `match`, `align skip` and the body.
/// Operands of `and`, `or`, `=` and `!=` are in the normal form of FR-174.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HyperIdentity {
    aliases: Vec<ModelAlias>,
    parameters: Vec<ObjectParameter>,
    prefix: Vec<(QuantifierKind, ModelAlias, Vec<FairnessConstraint>)>,
    mu_universal: Vec<u8>,
    mu_existential: Vec<u8>,
    align_skip: Vec<OperationKey>,
    body: Vec<u8>,
}

/// A hyper clause over `behaviours`, checked (FR-172 "Outputs").
#[derive(Clone, Debug)]
pub struct CheckedHyperClause {
    name: String,
    span: Span,
    profile: TemporalProfile,
    aliases: Vec<ModelAlias>,
    parameters: Vec<ObjectParameter>,
    quantifiers: Vec<CheckedQuantifier>,
    match_expression: Option<Expression>,
    mu_universal: Vec<CheckedStepPredicate>,
    mu_existential: Vec<CheckedStepPredicate>,
    align_skip: Vec<OperationKey>,
    operators: Vec<CheckedOperator>,
    body: CheckedFormula,
    form: Option<HyperForm>,
    copy_group: CopyGroup,
    claim: TemporalClaim,
    identity: HyperIdentity,
    atom_slots: Vec<AtomSlot>,
    atoms: BTreeMap<usize, lowering::AtomCondition>,
    /// The object type each object parameter declares, in source order.
    parameter_objects: Vec<EffectiveId>,
    /// The span of each quantifier's written constraint that resolved to
    /// each member of its fairness set, in fairness-set order.
    fairness_spans: Vec<Vec<Span>>,
    /// The `claim` occurrence of the clause's `hyperproperty` node, set when
    /// the package is assembled (FR-488 "Outputs"); `None` for a clause no
    /// package assembled.
    occurrence: Option<OccurrenceKey>,
}

#[cfg(any(test, feature = "test-support"))]
impl CheckedHyperClause {
    /// The clause with every object parameter bound to the model alias
    /// `alias` after S3 (test-support only): a clause no well-formed request
    /// carries when `alias` is no alias of the clause.
    #[must_use]
    pub fn with_parameter_alias_after_s3(mut self, alias: &str) -> Self {
        for parameter in &mut self.parameters {
            parameter.alias = ModelAlias(alias.to_owned());
        }
        self
    }
}

impl CheckedHyperClause {
    /// The declared clause name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The span of the clause's declaration.
    pub fn span(&self) -> Span {
        self.span
    }

    /// The one temporal profile the clause was checked under. A clause under
    /// any profile but infinite-trace records it and no form (ADR-023 HS-7).
    pub fn profile(&self) -> TemporalProfile {
        self.profile
    }

    /// The distinct subject aliases, in order of first use: the request
    /// supplies one model subject for each (ADR-023 HM-6).
    pub fn aliases(&self) -> &[ModelAlias] {
        &self.aliases
    }

    /// The object parameters, in source order.
    pub fn parameters(&self) -> &[ObjectParameter] {
        &self.parameters
    }

    /// The quantifier prefix, in source order.
    pub fn quantifiers(&self) -> &[CheckedQuantifier] {
        &self.quantifiers
    }

    /// How many of the quantifier prefix's leading variables are universal.
    /// Every existential variable follows them: an `exists` before a
    /// `forall` is HP-4 (FR-173).
    pub fn universals(&self) -> usize {
        self.quantifiers
            .iter()
            .take_while(|quantifier| quantifier.kind == QuantifierKind::Forall)
            .count()
    }

    /// The `match` block's expression; `None` when there is no block.
    pub fn match_expression(&self) -> Option<&Expression> {
        self.match_expression.as_ref()
    }

    /// The `match` conjuncts that read universal variables only (`mu_U`).
    pub fn mu_universal(&self) -> &[CheckedStepPredicate] {
        &self.mu_universal
    }

    /// The `match` conjuncts that read an existential variable (`mu_E`).
    pub fn mu_existential(&self) -> &[CheckedStepPredicate] {
        &self.mu_existential
    }

    /// The operations whose steps the traces skip when they align; empty for
    /// lockstep.
    pub fn align_skip(&self) -> &[OperationKey] {
        &self.align_skip
    }

    /// Every operator node of the body, connectives included, in arena order.
    pub fn operators(&self) -> &[CheckedOperator] {
        &self.operators
    }

    /// The checked body, over indexed atoms.
    pub fn body(&self) -> &CheckedFormula {
        &self.body
    }

    /// The clause's form (FR-173); `None` under a profile other than
    /// infinite-trace.
    pub fn form(&self) -> Option<&HyperForm> {
        self.form.as_ref()
    }

    /// The swappable classes of universal variables (FR-174); empty unless
    /// the clause is HP-2 or HP-6 and has a symmetry.
    pub fn copy_group(&self) -> &CopyGroup {
        &self.copy_group
    }

    /// The clause's identity members (ADR-023 HV-7).
    pub fn identity(&self) -> &HyperIdentity {
        &self.identity
    }

    /// One entry per atom slot, in slot order (FR-487 "Atom slots"): every
    /// atom of the clause has the same slot list.
    pub fn atom_slots(&self) -> &[AtomSlot] {
        &self.atom_slots
    }

    /// The checked condition of the `holds` atom `atom` and its slot count;
    /// `None` for a node that is not a `holds` atom of the body.
    pub fn condition(&self, atom: FormulaNodeId) -> Option<(CheckedNode<'_>, usize)> {
        let condition = self.atoms.get(&atom.index())?;
        Some((condition.root(), condition.slots))
    }

    /// The component (trace variable index, quantifier order) that each model
    /// read of the atom `atom` reads, keyed by the read's location.
    pub fn component_reads(&self, atom: FormulaNodeId) -> Option<&BTreeMap<Location, usize>> {
        self.atoms.get(&atom.index()).map(|atom| &atom.components)
    }

    /// Both parts' `match` conjuncts in source order, as `(part, index in
    /// that part)`, ordered by [`CheckedStepPredicate::span`] start (FR-488
    /// "Inputs"). The package keeps no part or index, so the split between
    /// `mu_U` and `mu_E` is recomputed from the checked clause.
    pub fn conjuncts(&self) -> Vec<(MatchPart, usize)> {
        let mut ordered: Vec<(usize, MatchPart, usize)> = self
            .mu_universal
            .iter()
            .enumerate()
            .map(|(index, conjunct)| (conjunct.span.start, MatchPart::Universal, index))
            .chain(
                self.mu_existential
                    .iter()
                    .enumerate()
                    .map(|(index, conjunct)| (conjunct.span.start, MatchPart::Existential, index)),
            )
            .collect();
        ordered.sort_by_key(|(start, _, _)| *start);
        ordered
            .into_iter()
            .map(|(_, part, index)| (part, index))
            .collect()
    }

    /// The `claim` occurrence `[node, "claim", ordinal]` (ADR-013 O-07) of
    /// the clause's `hyperproperty` node: the `clause` member of FR-315's
    /// `HyperProductIdentity`. The emitter assigns it when the package is
    /// assembled, so a clause that was checked and not assembled has none.
    /// A consumer reads clauses from `CheckedGraph::hyper_clauses()`, which
    /// are all assembled, never from `HyperTraceFamily::check`.
    pub fn assembled_occurrence(&self) -> Option<&OccurrenceKey> {
        self.occurrence.as_ref()
    }

    /// The same clause, assembled into a package at `occurrence`.
    pub(crate) fn assembled_at(mut self, occurrence: OccurrenceKey) -> Self {
        self.occurrence = Some(occurrence);
        self
    }

    /// The object type each object parameter declares (`v: V::T`), in
    /// source order.
    pub(crate) fn parameter_objects(&self) -> &[EffectiveId] {
        &self.parameter_objects
    }

    /// The span of each quantifier's written fairness constraints, one per
    /// member of its fairness set.
    pub(crate) fn fairness_spans(&self) -> &[Vec<Span>] {
        &self.fairness_spans
    }

    /// The checked condition of each `holds` atom, by arena index.
    pub(crate) fn atom_conditions(&self) -> &BTreeMap<usize, lowering::AtomCondition> {
        &self.atoms
    }

    /// Hand the compound units typing formed to lowering, which keys them.
    pub(crate) fn take_formed_units(&mut self) -> Vec<quire_semantic_value::quantity::UnitTable> {
        let mut units = Vec::new();
        for atom in self.atoms.values_mut() {
            units.push(std::mem::take(&mut atom.formed_units));
        }
        for conjunct in self.mu_universal.iter_mut().chain(&mut self.mu_existential) {
            units.push(std::mem::take(&mut conjunct.condition.formed_units));
        }
        units
    }
}

/// Which part of a `match` block a conjunct belongs to (FR-172, ADR-023
/// HM-3).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MatchPart {
    /// `mu_U`: the conjuncts that read universal variables only.
    Universal,
    /// `mu_E`: the conjuncts that read an existential variable.
    Existential,
}

/// A `relation` over model executions, checked (FR-172 "Outputs").
#[derive(Clone)]
pub struct CheckedModelRelation {
    name: String,
    span: Span,
    executions: Vec<(String, OperationKey)>,
    /// The selected model alias for each execution, aligned with
    /// `executions`.
    execution_aliases: Vec<String>,
    /// The source body, retained for diagnostics and source maps.
    body: Expression,
    /// The body expression's source spans for S4 source-map entries.
    body_spans: ExpressionSpans,
    /// The typed value body that S4 lowers, with one local slot for every
    /// execution's self, result (when present) and parameters.
    typed_body: CheckedBody,
    /// The body binder names and types in the slot order above.
    body_parameters: Vec<(String, ValueType)>,
    claim: TemporalClaim,
    code_domains: BTreeMap<(usize, Vec<u32>), DomainKind>,
    /// The S4 step-relation node key, once assembled.
    identity: Option<quire_exact::NodeKey>,
    /// Its `claim` occurrence, once assembled.
    occurrence: Option<OccurrenceKey>,
}

impl std::fmt::Debug for CheckedModelRelation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            name,
            span,
            executions,
            execution_aliases,
            body,
            body_spans,
            typed_body,
            body_parameters,
            claim,
            code_domains,
            identity,
            occurrence,
        } = self;
        f.debug_struct("CheckedModelRelation")
            .field("name", &name)
            .field("span", &span)
            .field("executions", &executions)
            .field("execution_aliases", &execution_aliases)
            .field("body", &body)
            .field("body_spans", &body_spans)
            .field("typed_body", &typed_body)
            .field("body_parameters", &body_parameters)
            .field("claim", &claim)
            .field("code_domains", &code_domains)
            .field("identity", &identity.as_ref().map(NodeKeyDebug))
            .field("occurrence", &occurrence)
            .finish()
    }
}

/// The `operation-contract` requirement of a step relation's code claim
/// (FR-180 "Hand-over"), before the checked nodes it is keyed by exist.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationContractClaim {
    domains: BTreeMap<(usize, Vec<u32>), DomainKind>,
}

impl OperationContractClaim {
    /// The capability kind the claim requests.
    pub fn kind(&self) -> Capability {
        Capability::OperationContract
    }

    /// The claim's requirement record, keyed by one node per root: for each
    /// execution in order, the receiver's pre-state, then each parameter.
    ///
    /// # Errors
    ///
    /// An internal fault when `roots` has fewer nodes than the claim has
    /// roots.
    pub fn requirements(&self, roots: &[NodeKey]) -> Result<Requirements, InternalFault> {
        let mut domains = BTreeMap::new();
        for ((index, path), kind) in &self.domains {
            let node = roots
                .get(*index)
                .ok_or_else(|| InternalFault::new(STAGE, "root-node-keyed"))?;
            domains.insert(
                DomainKey::Node {
                    node: node.clone(),
                    path: path.clone(),
                },
                *kind,
            );
        }
        Ok(Requirements::new(
            Capability::OperationContract,
            ClaimExtent::from_domains(domains),
        ))
    }
}

impl CheckedModelRelation {
    /// The declared relation name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The span of the relation's declaration.
    pub fn span(&self) -> Span {
        self.span
    }

    /// Each execution variable and the model operation it ranges over, in
    /// order.
    pub fn executions(&self) -> &[(String, OperationKey)] {
        &self.executions
    }

    /// The selected model alias for each execution, in source order.
    pub(crate) fn execution_aliases(&self) -> &[String] {
        &self.execution_aliases
    }

    /// The relation's body, every read of which was typed under the value
    /// profile: a Boolean over each execution's arguments, result and
    /// receiver's pre-state and post-state. S4 lowers it to FR-322 terms.
    pub fn body(&self) -> &Expression {
        &self.body
    }

    /// The source spans of the checked relation body.
    pub(crate) fn body_spans(&self) -> &ExpressionSpans {
        &self.body_spans
    }

    /// The typed S3 body whose nodes S4 lowers.
    pub(crate) fn typed_body(&self) -> &CheckedBody {
        &self.typed_body
    }

    /// The typed body binders, in the order their local slots name them.
    pub(crate) fn body_parameters(&self) -> &[(String, ValueType)] {
        &self.body_parameters
    }

    /// The relation's form: always HP-1 (FR-173).
    pub fn form(&self) -> HyperForm {
        HyperForm::StepRelation
    }

    /// The model claim: `temporal-satisfaction` over the model's behaviours.
    pub fn model_claim(&self) -> &TemporalClaim {
        &self.claim
    }

    /// The code claim (ADR-023 XC-1), present only when every execution's
    /// operation is bound to an implementation by `abstraction`.
    pub fn code_claim(
        &self,
        abstraction: &CheckedAbstractionRelation,
    ) -> Option<OperationContractClaim> {
        self.executions
            .iter()
            .all(|(_, operation)| {
                abstraction
                    .get(&AbstractionKey::Frame(operation.clone()))
                    .is_some()
            })
            .then(|| OperationContractClaim {
                domains: self.code_domains.clone(),
            })
    }

    /// The step-relation node key after S4 assembly.
    pub fn identity(&self) -> Option<quire_exact::NodeKey> {
        self.identity.clone()
    }

    /// The node's `claim` occurrence after S4 assembly.
    pub fn occurrence(&self) -> Option<&OccurrenceKey> {
        self.occurrence.as_ref()
    }

    pub(crate) fn assembled_at(
        mut self,
        identity: quire_exact::NodeKey,
        occurrence: OccurrenceKey,
    ) -> Self {
        self.identity = Some(identity);
        self.occurrence = Some(occurrence);
        self
    }
}

/// A clause checked by [`HyperTraceFamily::check`].
#[derive(Clone, Debug)]
pub enum CheckedHyper {
    /// A hyper clause over `behaviours`.
    Behaviours(Box<CheckedHyperClause>),
    /// A relation over model executions.
    Relation(Box<CheckedModelRelation>),
}

impl FamilyContract for HyperTraceFamily {
    type Form = HyperDeclaration;
    type Checked = CheckedHyper;
    type Cause = CheckRefusal;
    type Declarations<'a> = HyperDeclarations<'a>;
    type Claim = TemporalClaim;

    fn check<'a>(
        form: &HyperDeclaration,
        cx: &mut CheckContext<'a, HyperDeclarations<'a>>,
    ) -> CheckOutcome<CheckedHyper, CheckRefusal> {
        check_declaration(form, cx).map(Staged::new)
    }

    fn requirements(checked: &CheckedHyper) -> Vec<TemporalClaim> {
        match checked {
            CheckedHyper::Behaviours(clause) => vec![clause.claim.clone()],
            CheckedHyper::Relation(relation) => vec![relation.claim.clone()],
        }
    }
}

/// Why a `hyper` clause or `relation` refused.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum HyperCause {
    /// A quantifier of a `behaviours` clause names no model alias with `of`.
    /// `unsupported_construct`/`expression-form`, at the quantifier.
    QuantifierWithoutAlias {
        /// The quantifier's span.
        span: Span,
    },
    /// A quantifier of a finite trace domain clause names a model alias.
    /// `unsupported_construct`/`expression-form`, at the quantifier.
    QuantifierWithAlias {
        /// The quantifier's span.
        span: Span,
    },
    /// A clause over a finite trace domain, whose quantifiers are
    /// well-formed, has no S3 check here. `unsupported_construct`/
    /// `expression-form`, at the clause.
    FiniteDomain {
        /// The clause's span.
        span: Span,
    },
    /// An `of` alias, or an object parameter's alias, names no model of the
    /// unit. `missing_declaration`/`missing-name`, at the alias.
    UnresolvedAlias {
        /// The alias as written.
        alias: String,
        /// The alias's span.
        span: Span,
    },
    /// An object parameter names a population of an alias the clause does not
    /// bind. `ill_typed`/`type-mismatch`, at the parameter.
    UnboundParameterAlias {
        /// The parameter's name.
        parameter: String,
        /// The parameter's span.
        span: Span,
    },
    /// An object parameter's type is not an object type of a model.
    /// `ill_typed`/`type-mismatch`, at the parameter.
    ParameterNotAnObject {
        /// The parameter's name.
        parameter: String,
        /// The parameter's span.
        span: Span,
    },
    /// No population of the alias's model has the parameter's object type as
    /// its element type. `missing_declaration`/`missing-name`, at the
    /// parameter.
    UnresolvedPopulation {
        /// The parameter's name.
        parameter: String,
        /// The parameter's span.
        span: Span,
    },
    /// More than one population has the parameter's object type as its element
    /// type. `ambiguous_declaration`/`ambiguous-name`, at the parameter.
    AmbiguousPopulation {
        /// The parameter's name.
        parameter: String,
        /// The parameter's span.
        span: Span,
    },
    /// A model member read in the body carries no trace variable.
    /// `ill_typed`/`type-mismatch`, at the read.
    UnindexedRead {
        /// The read's span.
        span: Span,
    },
    /// An indexed read names a trace bound to a model other than its member's,
    /// or reads no member of an object parameter. `ill_typed`/`type-mismatch`,
    /// at the read.
    MismatchedTrace {
        /// The indexed read's span.
        span: Span,
    },
    /// A read or label names a trace variable the clause does not quantify.
    /// `missing_declaration`/`missing-name`, at the read.
    UnknownTrace {
        /// The trace variable as written.
        name: String,
        /// The read's span.
        span: Span,
    },
    /// A step label names no operation, or no parameter of it, in its trace's
    /// model. `missing_declaration`/`missing-name`, at the label.
    UnresolvedStepLabel {
        /// The label's operation as written.
        operation: String,
        /// The label's span.
        span: Span,
    },
    /// A step label in the body. `unsupported_construct`/`expression-form`,
    /// at the label.
    StepLabelInBody {
        /// The label's span.
        span: Span,
    },
    /// A `match` conjunct reads state or a form `match` does not admit.
    /// `unsupported_construct`/`expression-form`, at the read.
    MatchReadsState {
        /// The offending node's span.
        span: Span,
    },
    /// An `align skip` list with no entry. `unsupported_construct`/
    /// `expression-form`, at the list.
    AlignSkipEmpty {
        /// The list's span.
        span: Span,
    },
    /// An `align skip` entry naming no operation of a bound alias.
    /// `unsupported_construct`/`expression-form`, at the entry.
    AlignSkipUnbound {
        /// The entry's span.
        span: Span,
    },
    /// An operation named twice in an `align skip` list.
    /// `unsupported_construct`/`expression-form`, at the second entry.
    AlignSkipRepeated {
        /// The second entry's span.
        span: Span,
    },
    /// A fairness constraint on a variable of an `align skip` clause.
    /// `unsupported_construct`/`expression-form`, at the constraint.
    AlignSkipFairness {
        /// The constraint's span.
        span: Span,
    },
    /// A single-existential clause with a `match` block.
    /// `unsupported_construct`/`expression-form`, at the block.
    SingleExistentialMatch {
        /// The block's span.
        span: Span,
    },
    /// An execution binding names no operation of a model alias.
    /// `unsupported_construct`/`expression-form`, at the binding.
    ExecutionNotAnOperation {
        /// The binding's qualified name as written.
        name: String,
        /// The name's span.
        span: Span,
    },
    /// A name used twice among a clause's trace variables, object parameters
    /// or execution variables. `conflicting_declaration`/`duplicate-declaration`,
    /// at the second name.
    DuplicateName {
        /// The name as written.
        name: String,
        /// The second name's span.
        span: Span,
    },
    /// A body names a variable or member that is declared nowhere.
    /// `missing_declaration`/`missing-name`, at the read.
    BodyMissing {
        /// The name as written.
        name: String,
        /// The read's span.
        span: Span,
    },
    /// A body operand of the wrong type, or a body that is not Boolean.
    /// `ill_typed`/`type-mismatch`, at the operand.
    BodyIllTyped {
        /// The operand's span.
        span: Span,
    },
    /// A contextual rational literal has no expected type.
    BodyAmbiguousLiteral {
        /// The literal's span.
        span: Span,
    },
    /// A value expression has an obligation the checker cannot prove.
    BodyUnproved {
        /// The failed obligation.
        obligation: Obligation,
        /// The obligation's expression span.
        span: Span,
    },
    /// A `let` binder that names a binder in scope or a variable of the
    /// clause, as the value typer refuses it. `ambiguous_declaration`/
    /// `ambiguous-name`, at the `let`.
    BodyShadows {
        /// The binder as written.
        name: String,
        /// The `let`'s span.
        span: Span,
    },
    /// A body form that atoms and relation bodies do not admit.
    /// `unsupported_construct`/`expression-form`, at the form.
    BodyUnsupported {
        /// The form's span.
        span: Span,
    },
    /// An indexed read or step label outside a `hyper` clause over
    /// `behaviours`. `unsupported_construct`/`expression-form`.
    OutsideHyperClause,
}

impl HyperCause {
    /// The refusal's own span; `None` for a cause located by the refusal's
    /// `Location`.
    pub fn span(&self) -> Option<Span> {
        match self {
            Self::QuantifierWithoutAlias { span }
            | Self::QuantifierWithAlias { span }
            | Self::FiniteDomain { span }
            | Self::UnresolvedAlias { span, .. }
            | Self::UnboundParameterAlias { span, .. }
            | Self::ParameterNotAnObject { span, .. }
            | Self::UnresolvedPopulation { span, .. }
            | Self::AmbiguousPopulation { span, .. }
            | Self::UnindexedRead { span }
            | Self::MismatchedTrace { span }
            | Self::UnknownTrace { span, .. }
            | Self::UnresolvedStepLabel { span, .. }
            | Self::StepLabelInBody { span }
            | Self::MatchReadsState { span }
            | Self::AlignSkipEmpty { span }
            | Self::AlignSkipUnbound { span }
            | Self::AlignSkipRepeated { span }
            | Self::AlignSkipFairness { span }
            | Self::SingleExistentialMatch { span }
            | Self::ExecutionNotAnOperation { span, .. }
            | Self::DuplicateName { span, .. }
            | Self::BodyMissing { span, .. }
            | Self::BodyIllTyped { span }
            | Self::BodyAmbiguousLiteral { span }
            | Self::BodyUnproved { span, .. }
            | Self::BodyShadows { span, .. }
            | Self::BodyUnsupported { span } => Some(*span),
            Self::OutsideHyperClause => None,
        }
    }

    /// This cause's catalog code.
    pub fn code(&self) -> Code {
        match self {
            Self::QuantifierWithoutAlias { .. }
            | Self::QuantifierWithAlias { .. }
            | Self::FiniteDomain { .. }
            | Self::StepLabelInBody { .. }
            | Self::MatchReadsState { .. }
            | Self::AlignSkipEmpty { .. }
            | Self::AlignSkipUnbound { .. }
            | Self::AlignSkipRepeated { .. }
            | Self::AlignSkipFairness { .. }
            | Self::SingleExistentialMatch { .. }
            | Self::ExecutionNotAnOperation { .. }
            | Self::BodyUnsupported { .. }
            | Self::OutsideHyperClause => Code::UnsupportedConstruct,
            Self::DuplicateName { .. } => Code::ConflictingDeclaration,
            Self::UnresolvedAlias { .. }
            | Self::UnresolvedPopulation { .. }
            | Self::UnknownTrace { .. }
            | Self::BodyMissing { .. }
            | Self::UnresolvedStepLabel { .. } => Code::MissingDeclaration,
            Self::AmbiguousPopulation { .. } | Self::BodyShadows { .. } => {
                Code::AmbiguousDeclaration
            }
            Self::BodyUnproved { .. } => Code::UndefinedExpression,
            Self::UnboundParameterAlias { .. }
            | Self::ParameterNotAnObject { .. }
            | Self::UnindexedRead { .. }
            | Self::BodyIllTyped { .. }
            | Self::BodyAmbiguousLiteral { .. }
            | Self::MismatchedTrace { .. } => Code::IllTyped,
        }
    }

    /// This cause's catalog tag.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::QuantifierWithoutAlias { .. }
            | Self::QuantifierWithAlias { .. }
            | Self::FiniteDomain { .. }
            | Self::StepLabelInBody { .. }
            | Self::MatchReadsState { .. }
            | Self::AlignSkipEmpty { .. }
            | Self::AlignSkipUnbound { .. }
            | Self::AlignSkipRepeated { .. }
            | Self::AlignSkipFairness { .. }
            | Self::SingleExistentialMatch { .. }
            | Self::ExecutionNotAnOperation { .. }
            | Self::BodyUnsupported { .. }
            | Self::OutsideHyperClause => "expression-form",
            Self::DuplicateName { .. } => "duplicate-declaration",
            Self::UnresolvedAlias { .. }
            | Self::UnresolvedPopulation { .. }
            | Self::UnknownTrace { .. }
            | Self::BodyMissing { .. }
            | Self::UnresolvedStepLabel { .. } => "missing-name",
            Self::AmbiguousPopulation { .. } | Self::BodyShadows { .. } => "ambiguous-name",
            Self::UnboundParameterAlias { .. }
            | Self::ParameterNotAnObject { .. }
            | Self::UnindexedRead { .. }
            | Self::BodyIllTyped { .. }
            | Self::MismatchedTrace { .. } => "type-mismatch",
            Self::BodyAmbiguousLiteral { .. } => "ambiguous-literal",
            Self::BodyUnproved { obligation, .. } => match obligation {
                Obligation::Nonzero => "unproved-nonzero",
                Obligation::Presence => "unproved-presence",
                Obligation::Range { .. }
                | Obligation::RationalRange
                | Obligation::NonemptyReduction { .. } => "unproved-range",
            },
        }
    }
}

/// A refusal found while checking one clause, before the clause's earliest is
/// reported.
enum Found {
    Hyper(HyperCause),
    Temporal(TemporalCause),
    /// A value-typer refusal with its original declaration location.
    Typed(HyperCause, Location),
}

impl Found {
    fn start(&self) -> usize {
        match self {
            Self::Hyper(cause) => cause.span().map_or(0, |span| span.start),
            Self::Temporal(cause) => cause.span().start,
            Self::Typed(cause, _) => cause.span().map_or(0, |span| span.start),
        }
    }

    fn into_failure(self) -> StageFailure<CheckRefusal> {
        let (cause, location) = match self {
            Self::Hyper(cause) => (
                CheckCause::Hyper(Box::new(cause)),
                Location::root(Origin::Expression),
            ),
            Self::Temporal(cause) => (
                CheckCause::Temporal(Box::new(cause)),
                Location::root(Origin::Expression),
            ),
            Self::Typed(cause, location) => (CheckCause::Hyper(Box::new(cause)), location),
        };
        StageFailure::Refused(CheckRefusal {
            // Synthesized clause diagnostics carry their source span in the
            // cause; value-typer refusals retain their original declaration
            // location so package region lookup can resolve the source node.
            location,
            cause,
        })
    }
}

fn fault(invariant: &'static str) -> StageFailure<CheckRefusal> {
    StageFailure::Fault(InternalFault::new(STAGE, invariant))
}

/// The earliest refusal of `found`, as a failure; `Ok` when there is none.
fn earliest(found: Vec<Found>) -> Result<(), StageFailure<CheckRefusal>> {
    match found.into_iter().min_by_key(Found::start) {
        Some(first) => Err(first.into_failure()),
        None => Ok(()),
    }
}

fn check_declaration<'a>(
    declaration: &HyperDeclaration,
    cx: &mut CheckContext<'a, HyperDeclarations<'a>>,
) -> Result<CheckedHyper, StageFailure<CheckRefusal>> {
    let declarations = cx.declarations();
    match declaration {
        HyperDeclaration::Behaviours(form) => check_behaviours(form, declarations, cx)
            .map(|clause| CheckedHyper::Behaviours(Box::new(clause))),
        HyperDeclaration::Finite(form) => Err(check_finite(form)),
        HyperDeclaration::Relation(form) => check_relation(form, declarations, cx)
            .map(|relation| CheckedHyper::Relation(Box::new(relation))),
    }
}

/// A clause over a finite trace domain: a quantifier with `of` refuses at the
/// quantifier (ADR-023 HS-2); one without has no check at S3.
fn check_finite(form: &HyperFiniteForm) -> StageFailure<CheckRefusal> {
    let found: Vec<Found> = form
        .quantifiers
        .iter()
        .filter(|quantifier| quantifier.alias.is_some())
        .map(|quantifier| {
            Found::Hyper(HyperCause::QuantifierWithAlias {
                span: quantifier.span,
            })
        })
        .collect();
    match earliest(found) {
        Err(failure) => failure,
        Ok(()) => Found::Hyper(HyperCause::FiniteDomain { span: form.span }).into_failure(),
    }
}

/// The trace variables of a clause, by name.
struct Variables<'f> {
    by_name: BTreeMap<&'f str, &'f CheckedQuantifier>,
}

impl Variables<'_> {
    fn alias_of(&self, name: &str) -> Option<&ModelAlias> {
        self.by_name.get(name).map(|quantifier| &quantifier.alias)
    }

    fn kind_of(&self, name: &str) -> Option<QuantifierKind> {
        self.by_name.get(name).map(|quantifier| quantifier.kind)
    }
}

fn check_behaviours<'a>(
    form: &HyperBehavioursForm,
    declarations: &HyperDeclarations<'a>,
    cx: &mut CheckContext<'a, HyperDeclarations<'a>>,
) -> Result<CheckedHyperClause, StageFailure<CheckRefusal>> {
    let temporal = &declarations.temporal;
    let scope = temporal.scope;
    let clause_index = declarations.behaviours_checked.get();
    declarations.behaviours_checked.set(clause_index + 1);
    let profile = match temporal.selections {
        [only] => only.profile,
        found => return Err(profile_selection(found, form.span)),
    };
    let mut found: Vec<Found> = Vec::new();

    let (quantifiers, fairness_spans) = checked_quantifiers(form, declarations, &mut found)?;
    let aliases = distinct_aliases(&quantifiers);
    let variables = Variables {
        by_name: quantifiers
            .iter()
            .map(|quantifier| (quantifier.variable.as_str(), quantifier))
            .collect(),
    };
    let parameters = checked_parameters(form, declarations, &aliases, &mut found);

    // The body: operators and intervals as an infinite-trace clause checks
    // them. A clause under another profile records its profile and no form,
    // so no bounded-profile rule applies to it here.
    let mut temporal_causes: Vec<TemporalCause> = Vec::new();
    let reading = if profile == TemporalProfile::Timed {
        IntervalReading::TimedUnread
    } else {
        IntervalReading::Steps { bounded: false }
    };
    let (operators, _) = checked_operators(&form.body, &reading, &mut temporal_causes)?;
    found.extend(temporal_causes.into_iter().map(Found::Temporal));
    // A condition is lowered only when the clause's quantifiers and object
    // parameters resolved, and it checked clean before.
    let resolved = found.is_empty();
    let typing = lowering::Typing {
        scope,
        signatures: declarations.signatures,
        dispatch_tables: declarations.dispatch_tables,
        models: declarations.models,
        limits: declarations.checking_limits,
        cancel: declarations.cancel,
    };
    let trace_names: Vec<&str> = quantifiers
        .iter()
        .map(|quantifier| quantifier.variable.as_str())
        .collect();
    let parameter_names: Vec<&str> = parameters
        .iter()
        .map(|(parameter, _)| parameter.name.as_str())
        .collect();
    let slots = lowering::AtomSlots::of(
        &quantifiers
            .iter()
            .map(|quantifier| (quantifier.variable.as_str(), quantifier.alias.as_str()))
            .collect::<Vec<_>>(),
        &parameters
            .iter()
            .map(|(parameter, object)| {
                (
                    parameter.name.as_str(),
                    parameter.alias.as_str(),
                    object.clone(),
                )
            })
            .collect::<Vec<_>>(),
    );
    let first_trace = |parameter: &str| {
        let alias = parameters
            .iter()
            .find(|(candidate, _)| candidate.name == parameter)
            .map(|(candidate, _)| &candidate.alias)?;
        quantifiers
            .iter()
            .position(|quantifier| &quantifier.alias == alias)
    };
    let mut nodes = declarations.nodes_used.get();
    let mut atoms: BTreeMap<usize, lowering::AtomCondition> = BTreeMap::new();
    for (id, node) in form.body.iter() {
        if let TemporalNodeForm::Holds { condition, .. } = node {
            let mark = found.len();
            check_atom(
                &condition.expression,
                &condition.spans,
                &parameters,
                &variables,
                &mut found,
            );
            if resolved && found.len() == mark {
                let location =
                    lowering::hyper_location(clause_index, lowering::Site::Atom(id.index()));
                let locate = |at: &Location| lowering::span_at(&condition.spans, at);
                match lowering::lower_atom(
                    &typing,
                    &mut cx.reborrow(scope),
                    &mut nodes,
                    &location,
                    &slots,
                    (&parameter_names, &trace_names),
                    &first_trace,
                    (&condition.expression, &locate),
                ) {
                    Ok(lowered) => {
                        atoms.insert(id.index(), lowered);
                    }
                    Err(refused) => found.push(body_refusal_at(
                        *refused.refusal,
                        refused.span,
                        &condition.expression,
                        &condition.spans,
                        &parameters
                            .iter()
                            .map(|(parameter, object)| (parameter.name.clone(), object.clone()))
                            .collect(),
                        &BTreeMap::new(),
                        scope,
                    )),
                }
            }
        }
    }

    let match_mark = found.len();
    let (match_expression, drafts) = match &form.match_block {
        Some(block) => {
            let split = check_match(block, &variables, declarations, &mut found)?;
            (Some(block.expression.clone()), split)
        }
        None => (None, (Vec::new(), Vec::new())),
    };
    let (universal_drafts, existential_drafts) = drafts;
    let mut lowered_universal = Vec::new();
    let mut lowered_existential = Vec::new();
    if let (true, true, Some(block)) = (resolved, found.len() == match_mark, &form.match_block) {
        for (existential, drafts, lowered) in [
            (false, &universal_drafts, &mut lowered_universal),
            (true, &existential_drafts, &mut lowered_existential),
        ] {
            for (index, draft) in drafts.iter().enumerate() {
                let site = if existential {
                    lowering::Site::Existential(index)
                } else {
                    lowering::Site::Universal(index)
                };
                let location = lowering::hyper_location(clause_index, site);
                // A conjunct refuses at its own span, the span the first
                // typing pass names.
                let locate = |_: &Location| draft.span;
                match lowering::lower_conjunct(
                    &typing,
                    &mut cx.reborrow(scope),
                    &mut nodes,
                    &location,
                    draft.root,
                    (&draft.slots, draft.labels.clone()),
                    (&block.expression, &locate),
                ) {
                    Ok(condition) => lowered.push(condition),
                    Err(refused) => found.push(body_refusal_at(
                        *refused.refusal,
                        refused.span,
                        &block.expression,
                        &block.spans,
                        &BTreeMap::new(),
                        &BTreeMap::new(),
                        scope,
                    )),
                }
            }
        }
    }
    declarations.nodes_used.set(nodes);

    let align_skip = checked_align_skip(form, &quantifiers, &aliases, declarations, &mut found);
    earliest(std::mem::take(&mut found))?;
    let predicates = |drafts: Vec<ConjunctDraft>,
                      lowered: Vec<lowering::ConjunctCondition>|
     -> Result<Vec<CheckedStepPredicate>, StageFailure<CheckRefusal>> {
        if drafts.len() != lowered.len() {
            return Err(fault("conjuncts-lowered"));
        }
        Ok(drafts
            .into_iter()
            .zip(lowered)
            .map(|(draft, condition)| CheckedStepPredicate {
                root: draft.root,
                span: draft.span,
                variables: draft.variables,
                reads: draft.reads,
                slots: draft.slots,
                condition,
            })
            .collect())
    };
    let mu_universal = predicates(universal_drafts, lowered_universal)?;
    let mu_existential = predicates(existential_drafts, lowered_existential)?;

    let intervals: BTreeMap<_, _> = operators
        .iter()
        .map(|operator| (operator.node, operator.interval.clone()))
        .collect();
    let body = build_formula(&form.body, &intervals)?;

    let form_class = (profile == TemporalProfile::InfiniteTrace).then(|| {
        classify(
            &quantifiers,
            form.align_skip.is_some(),
            in_safety_fragment(&body),
        )
    });
    if let (Some(HyperForm::SingleExistential), Some(block)) = (&form_class, &form.match_block) {
        let span = form.match_span.unwrap_or(root_span(&block.spans));
        return Err(Found::Hyper(HyperCause::SingleExistentialMatch { span }).into_failure());
    }

    let copy_group = match form_class {
        Some(HyperForm::Universal | HyperForm::ProjectionAligned) => copy_group(
            &quantifiers,
            &form.body,
            match_expression.as_ref(),
            &mu_universal,
        ),
        _ => CopyGroup::default(),
    };

    let parameter_objects: Vec<EffectiveId> = parameters
        .iter()
        .map(|(_, object)| object.clone())
        .collect();
    let roots: Vec<ValueType> = parameters
        .iter()
        .map(|(_, object)| ValueType::Reference(object.clone()))
        .collect();
    let root_refs: Vec<&ValueType> = roots.iter().collect();
    let parameter_domains = classify_domains(&root_refs, scope.types(), temporal.position_limit)
        .map_err(|failure| match failure {
            ClassifyFailure::Limit(limit) => {
                super::temporal_clause::located(limit, temporal, form.span)
            }
            ClassifyFailure::Fault(fault) => StageFailure::Fault(fault),
        })?;
    let identity = identity(
        &aliases,
        &parameters
            .iter()
            .map(|(parameter, _)| parameter.clone())
            .collect::<Vec<_>>(),
        &quantifiers,
        &form.body,
        match_expression.as_ref(),
        (&mu_universal, &mu_existential),
        &align_skip,
    );
    Ok(CheckedHyperClause {
        name: form.name.name.clone(),
        span: form.span,
        profile,
        aliases,
        parameters: parameters
            .into_iter()
            .map(|(parameter, _)| parameter)
            .collect(),
        quantifiers,
        match_expression,
        mu_universal,
        mu_existential,
        align_skip,
        operators,
        body,
        form: form_class,
        copy_group,
        claim: TemporalClaim::new(
            parameter_domains,
            profile == TemporalProfile::InfiniteTrace,
            true,
        ),
        identity,
        atom_slots: slots.slots().to_vec(),
        atoms,
        parameter_objects,
        fairness_spans,
        occurrence: None,
    })
}

fn profile_selection(found: &[TemporalSelection], span: Span) -> StageFailure<CheckRefusal> {
    Found::Temporal(TemporalCause::ProfileSelection {
        found: found
            .iter()
            .map(|selection| selection.profile.identity())
            .collect(),
        span,
    })
    .into_failure()
}

/// The model of `alias`, when the unit selects one.
fn model_of<'m>(models: &'m [AdmittedModel], alias: &str) -> Option<&'m AdmittedModel> {
    models.iter().find(|model| model.alias() == Some(alias))
}

/// The quantifiers, each bound to its alias and its fairness set. A quantifier
/// with no alias, one naming no model and a constraint naming an operation of
/// another model push their refusals onto `found`.
/// The checked quantifiers and, per quantifier, the span of each written
/// fairness constraint that resolved.
type QuantifiersWithSpans = (Vec<CheckedQuantifier>, Vec<Vec<Span>>);

fn checked_quantifiers(
    form: &HyperBehavioursForm,
    declarations: &HyperDeclarations<'_>,
    found: &mut Vec<Found>,
) -> Result<QuantifiersWithSpans, StageFailure<CheckRefusal>> {
    let mut checked = Vec::with_capacity(form.quantifiers.len());
    let mut spans = Vec::with_capacity(form.quantifiers.len());
    let mut seen: Vec<&str> = Vec::new();
    for quantifier in &form.quantifiers {
        if seen.contains(&quantifier.variable.name.as_str()) {
            found.push(Found::Hyper(HyperCause::DuplicateName {
                name: quantifier.variable.name.clone(),
                span: quantifier.variable.span,
            }));
            continue;
        }
        seen.push(&quantifier.variable.name);
        let Some(alias) = &quantifier.alias else {
            found.push(Found::Hyper(HyperCause::QuantifierWithoutAlias {
                span: quantifier.span,
            }));
            continue;
        };
        if model_of(declarations.models, &alias.name).is_none() {
            found.push(Found::Hyper(HyperCause::UnresolvedAlias {
                alias: alias.name.clone(),
                span: alias.span,
            }));
            continue;
        }
        let (fairness, fairness_spans) =
            checked_fairness(quantifier, &alias.name, declarations, found);
        checked.push(CheckedQuantifier {
            kind: quantifier.kind,
            variable: quantifier.variable.name.clone(),
            alias: ModelAlias(alias.name.clone()),
            fairness,
        });
        spans.push(fairness_spans);
    }
    Ok((checked, spans))
}

/// A quantifier's fairness set (FR-172 "Fairness and profile"): each
/// constraint names an operation of the quantifier's own model.
fn checked_fairness(
    quantifier: &QuantifierForm,
    alias: &str,
    declarations: &HyperDeclarations<'_>,
    found: &mut Vec<Found>,
) -> (Vec<FairnessConstraint>, Vec<Span>) {
    let mut set: Vec<FairnessConstraint> = Vec::new();
    let mut spans: Vec<Span> = Vec::new();
    for written in &quantifier.fairness {
        match resolve_operation(declarations, alias, &written.operation) {
            Ok((operation, _)) => {
                let constraint = checked_constraint(written, operation);
                if !set.contains(&constraint) {
                    set.push(constraint);
                    spans.push(written.span);
                }
            }
            Err(cause) => found.push(Found::Temporal(cause)),
        }
    }
    (set, spans)
}

/// The operation `name` (`Alias::Type::operation`) names in the model of
/// `alias`, with its declaration. A name whose alias is another model's
/// resolves nothing.
fn resolve_operation<'s>(
    declarations: &HyperDeclarations<'s>,
    alias: &str,
    name: &NameForm,
) -> Result<
    (
        FairnessOperation,
        &'s crate::model::operation::OperationDeclaration,
    ),
    TemporalCause,
> {
    let scope = declarations.temporal.scope;
    let unresolved = || TemporalCause::UnresolvedFairnessOperation {
        name: name.name.clone(),
        span: name.span,
    };
    let segments: Vec<&str> = name.name.split("::").collect();
    let [written_alias, object, operation] = segments[..] else {
        return Err(unresolved());
    };
    if written_alias != alias {
        return Err(unresolved());
    }
    let object = scope
        .object_type_named(&format!("{written_alias}::{object}"))
        .ok_or_else(unresolved)?;
    match scope.operations().resolve(scope.types(), object, operation) {
        OperationLookup::Declared {
            declaring,
            operation,
        } => Ok((
            FairnessOperation::new(declaring, operation.name().to_owned()),
            operation,
        )),
        OperationLookup::Missing => Err(unresolved()),
        OperationLookup::Ambiguous(_) => Err(TemporalCause::AmbiguousFairnessOperation {
            name: name.name.clone(),
            span: name.span,
        }),
    }
}

/// The distinct aliases of `quantifiers`, in order of first use.
fn distinct_aliases(quantifiers: &[CheckedQuantifier]) -> Vec<ModelAlias> {
    let mut aliases: Vec<ModelAlias> = Vec::new();
    for quantifier in quantifiers {
        if !aliases.contains(&quantifier.alias) {
            aliases.push(quantifier.alias.clone());
        }
    }
    aliases
}

/// The operation key of `operation` in the model of `alias`.
fn operation_key(
    models: &[AdmittedModel],
    alias: &str,
    operation: &FairnessOperation,
) -> Option<OperationKey> {
    let model = model_of(models, alias)?;
    Some(OperationKey {
        declaring: model.declaration_key(operation.declaring_type())?.clone(),
        operation: Identifier::new(operation.name()).ok()?,
    })
}

/// Each object parameter bound to a population of a bound alias (FR-172
/// "Object parameters"), with the parameter's object type.
fn checked_parameters(
    form: &HyperBehavioursForm,
    declarations: &HyperDeclarations<'_>,
    aliases: &[ModelAlias],
    found: &mut Vec<Found>,
) -> Vec<(ObjectParameter, EffectiveId)> {
    let scope = declarations.temporal.scope;
    let mut checked = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    for parameter in &form.parameters {
        let name = parameter.name.name.clone();
        if seen.contains(&name.as_str()) {
            found.push(Found::Hyper(HyperCause::DuplicateName {
                name,
                span: parameter.name.span,
            }));
            continue;
        }
        seen.push(&parameter.name.name);
        let span = Span {
            start: parameter.name.span.start,
            end: parameter.value_type.span.end,
        };
        let TypeFormHead::Name(written) = &parameter.value_type.head else {
            found.push(Found::Hyper(HyperCause::ParameterNotAnObject {
                parameter: name,
                span,
            }));
            continue;
        };
        let alias = written
            .segments
            .first()
            .map(String::as_str)
            .unwrap_or_default();
        if !aliases.iter().any(|bound| bound.as_str() == alias) {
            found.push(Found::Hyper(HyperCause::UnboundParameterAlias {
                parameter: name,
                span,
            }));
            continue;
        }
        let Some(object) = scope.object_type_named(&written.name) else {
            found.push(Found::Hyper(HyperCause::ParameterNotAnObject {
                parameter: name,
                span,
            }));
            continue;
        };
        match population_of(declarations.models, object.clone(), scope.types()) {
            Ok(Some(population)) => checked.push((
                ObjectParameter {
                    name,
                    alias: ModelAlias(alias.to_owned()),
                    population,
                },
                object,
            )),
            Ok(None) => found.push(Found::Hyper(HyperCause::UnresolvedPopulation {
                parameter: name,
                span,
            })),
            Err(_) => found.push(Found::Hyper(HyperCause::AmbiguousPopulation {
                parameter: name,
                span,
            })),
        }
    }
    checked
}

/// One node of an expression with the span of its CST node and its parent.
pub(crate) struct Visit<'e> {
    pub(crate) node: ExprRef<'e>,
    pub(crate) span: Option<Span>,
    pub(crate) parent: Option<ExprRef<'e>>,
}

/// The span of the whole expression `spans` locates.
fn root_span(spans: &ExpressionSpans) -> Span {
    spans
        .span(spans.root())
        .unwrap_or(Span { start: 0, end: 0 })
}

/// Map a value-typer refusal to the source span it identifies and preserve
/// the hyper clause's public refusal categories.
fn body_refusal(
    refusal: CheckRefusal,
    expression: &Expression,
    spans: &ExpressionSpans,
    roots: &BTreeMap<String, EffectiveId>,
    execution_fields: &BTreeMap<String, BTreeSet<String>>,
    scope: &Scope,
) -> Found {
    let span = spans
        .at(&refusal.location.path())
        .or_else(|| spans.span(spans.root()))
        .unwrap_or(Span { start: 0, end: 0 });
    body_refusal_at(
        refusal,
        span,
        expression,
        spans,
        roots,
        execution_fields,
        scope,
    )
}

fn body_refusal_at(
    refusal: CheckRefusal,
    mut span: Span,
    expression: &Expression,
    spans: &ExpressionSpans,
    roots: &BTreeMap<String, EffectiveId>,
    execution_fields: &BTreeMap<String, BTreeSet<String>>,
    scope: &Scope,
) -> Found {
    let location = refusal.location.clone();
    let path = location.path();
    let node_spans = spans.of_nodes(expression);
    let mut field = None;
    for depth in (0..=path.len()).rev() {
        let Some(id) = lowering::node_at(expression, &path[..depth]) else {
            continue;
        };
        if let ExprNode::Field {
            operand,
            field: name,
        } = expression.at(id).node()
        {
            field = Some((id, *operand, name.as_str()));
            break;
        }
    }
    let missing_member = field.and_then(|(id, operand, name)| {
        let mut operand = expression.at(operand);
        while let ExprNode::Field { operand: inner, .. } = operand.node() {
            operand = expression.at(*inner);
        }
        let ExprNode::Name(receiver) = operand.node() else {
            return None;
        };
        let object = roots.get(&receiver.name)?;
        if execution_fields
            .get(&receiver.name)
            .is_some_and(|fields| fields.contains(name))
        {
            return None;
        }
        let exists = scope.types().attribute(object.clone(), name).is_some();
        let legacy_execution_root = matches!(&refusal.cause, CheckCause::MissingName(_));
        (!exists || legacy_execution_root).then(|| {
            span = node_spans
                .as_ref()
                .and_then(|spans| spans.get(id.index()))
                .copied()
                .unwrap_or(span);
            name.to_owned()
        })
    });
    let cause = match refusal.cause {
        CheckCause::MissingName(_) if missing_member.is_some() => HyperCause::BodyMissing {
            name: missing_member.expect("checked above"),
            span,
        },
        CheckCause::IllTyped(_) if missing_member.is_some() => HyperCause::BodyMissing {
            name: missing_member.expect("checked above"),
            span,
        },
        CheckCause::StateModel(_) if missing_member.is_some() => HyperCause::BodyMissing {
            name: missing_member.expect("checked above"),
            span,
        },
        CheckCause::MissingName(name) => HyperCause::BodyMissing { name, span },
        CheckCause::MissingQualifiedName(name) => HyperCause::BodyMissing {
            name: name.name,
            span,
        },
        CheckCause::MissingSegment { name, .. } => HyperCause::BodyMissing { name, span },
        CheckCause::AmbiguousName { name, .. } => HyperCause::BodyShadows { name, span },
        CheckCause::IllTyped(quire_exact::IllTypedCause::AmbiguousLiteral) => {
            HyperCause::BodyAmbiguousLiteral { span }
        }
        CheckCause::Unproved(obligation) => HyperCause::BodyUnproved { obligation, span },
        CheckCause::Hyper(_) => HyperCause::BodyUnsupported { span },
        CheckCause::UnsupportedNavigation(_)
        | CheckCause::ObjectReadInValueInvariant
        | CheckCause::PopulationFormOutsideRefinement
        | CheckCause::PopulationFormNotChecked { .. } => HyperCause::BodyUnsupported { span },
        CheckCause::ProfileProhibitedOperator(_) => HyperCause::BodyIllTyped { span },
        _ => HyperCause::BodyIllTyped { span },
    };
    Found::Typed(cause, location)
}

/// A body fault as the cause it is reported as.
/// Visit every node of `expression` in source order, on an explicit stack.
pub(crate) fn walk<'e>(
    expression: &'e Expression,
    spans: &ExpressionSpans,
    mut visit: impl FnMut(&Visit<'e>),
) {
    let mut pending: Vec<(ExprRef<'e>, Option<SpanId>, Option<ExprRef<'e>>)> =
        vec![(expression.root(), Some(spans.root()), None)];
    while let Some((node, located, parent)) = pending.pop() {
        visit(&Visit {
            node,
            span: located.and_then(|id| spans.span(id)),
            parent,
        });
        for (index, child) in node.children().into_iter().enumerate().rev() {
            pending.push((
                child,
                located.and_then(|id| spans.child(id, index)),
                Some(node),
            ));
        }
    }
}

/// The operand a member-read link continues the chain through.
fn chain_operand(node: &ExprNode) -> Option<ExprId> {
    match node {
        ExprNode::Field { operand, .. } | ExprNode::Deref(operand) => Some(*operand),
        ExprNode::Dispatch { receiver, .. } => Some(*receiver),
        _ => None,
    }
}

/// The name a member-read chain starts at, when it is one.
fn read_root<'e>(mut node: ExprRef<'e>) -> Option<&'e str> {
    loop {
        match node.node() {
            ExprNode::Name(name) => return Some(name.name.as_str()),
            other => node = node.at(chain_operand(other)?),
        }
    }
}

/// FR-172 "Indexed atoms and step labels": every model member read in a body
/// atom is enclosed by exactly one trace variable, `e @ t`, and `@ t` scopes
/// over every read of `e` that no inner `@` encloses. A member read is the
/// field selection of a chain whose root is an object parameter, and a field
/// selection on a `deref`, whatever its root. The trace variable is bound to
/// the model that declares the parameter the read is rooted at.
fn check_atom(
    expression: &Expression,
    spans: &ExpressionSpans,
    parameters: &[(ObjectParameter, EffectiveId)],
    variables: &Variables<'_>,
    found: &mut Vec<Found>,
) {
    let parameter = |name: &str| {
        parameters
            .iter()
            .map(|(parameter, _)| parameter)
            .find(|parameter| parameter.name == name)
    };
    let enclosing = lowering::enclosing_reads(expression);
    let mut located: Vec<Option<Span>> = vec![None; expression.len()];
    walk(expression, spans, |visit| {
        if let Some(slot) = located.get_mut(visit.node.id().index()) {
            *slot = visit.span;
        }
    });
    let mut indexed: Vec<ExprId> = Vec::new();
    let mut touched: Vec<ExprId> = Vec::new();
    walk(expression, spans, |visit| {
        let Some(span) = visit.span else {
            return;
        };
        let id = visit.node.id();
        let around = enclosing.get(id.index()).copied().flatten();
        match visit.node.node() {
            ExprNode::StepLabel { .. } | ExprNode::Stutter => {
                found.push(Found::Hyper(HyperCause::StepLabelInBody { span }));
            }
            ExprNode::Indexed { trace, .. } => {
                if variables.alias_of(trace).is_none() {
                    found.push(Found::Hyper(HyperCause::UnknownTrace {
                        name: trace.clone(),
                        span,
                    }));
                    return;
                }
                indexed.push(id);
            }
            ExprNode::Name(name) if parameter(&name.name).is_some() => {
                touched.extend(around);
            }
            other => {
                let root_parameter = read_root(visit.node).and_then(parameter);
                let reads_member = match other {
                    ExprNode::Field { operand, .. } => {
                        root_parameter.is_some()
                            || matches!(visit.node.at(*operand).node(), ExprNode::Deref(_))
                    }
                    ExprNode::Deref(_) => true,
                    _ => false,
                };
                let continues = visit.parent.is_some_and(|parent| {
                    chain_operand(parent.node()) == Some(id)
                        || matches!(parent.node(), ExprNode::Indexed { read, .. } if *read == id)
                });
                if !reads_member {
                    return;
                }
                match around {
                    None if !continues => {
                        found.push(Found::Hyper(HyperCause::UnindexedRead { span }));
                    }
                    None => {}
                    Some(read) => {
                        touched.push(read);
                        let trace_alias = match expression.at(read).node() {
                            ExprNode::Indexed { trace, .. } => variables.alias_of(trace),
                            _ => None,
                        };
                        if let (Some(owner), Some(alias)) = (root_parameter, trace_alias) {
                            if &owner.alias != alias {
                                let at = located.get(read.index()).copied().flatten();
                                found.push(Found::Hyper(HyperCause::MismatchedTrace {
                                    span: at.unwrap_or(span),
                                }));
                            }
                        }
                    }
                }
            }
        }
    });
    for read in indexed {
        if !touched.contains(&read) {
            if let Some(span) = located.get(read.index()).copied().flatten() {
                found.push(Found::Hyper(HyperCause::MismatchedTrace { span }));
            }
        }
    }
}

/// One top-level conjunct of a `match` block, checked for its reads and not
/// yet lowered.
struct ConjunctDraft {
    root: ExprId,
    span: Span,
    variables: Vec<String>,
    reads: Vec<StepRead>,
    slots: Vec<StepRead>,
    /// The operation text of each label constant the conjunct writes.
    labels: BTreeMap<String, String>,
}

/// What a qualified name in a `match` conjunct resolves to.
enum Qualified {
    /// An operation of the model of the alias written in the name.
    Operation(String, OperationKey),
    /// A member of an object type of a model: a state read.
    Member,
    /// A name of a bound model that names neither.
    Nothing,
    /// Not a name of that form.
    Other,
}

/// What `name` (`Alias::Type::member`) resolves to (FR-172 "Match").
fn qualify(declarations: &HyperDeclarations<'_>, name: &NameForm) -> Qualified {
    let scope = declarations.temporal.scope;
    let segments: Vec<&str> = name.name.split("::").collect();
    let [alias, object, member] = segments[..] else {
        return Qualified::Other;
    };
    if model_of(declarations.models, alias).is_none() {
        return Qualified::Other;
    }
    if let Ok((operation, _)) = resolve_operation(declarations, alias, name) {
        return match operation_key(declarations.models, alias, &operation) {
            Some(key) => Qualified::Operation(alias.to_owned(), key),
            None => Qualified::Nothing,
        };
    }
    let is_member = scope
        .object_type_named(&format!("{alias}::{object}"))
        .is_some_and(|object| scope.types().attribute(object, member).is_some());
    if is_member {
        Qualified::Member
    } else {
        Qualified::Nothing
    }
}

/// The top-level conjuncts of a `match` block, left to right, each with the
/// span node that locates it: the one split `check_match` and
/// [`conjunct_spans`] share.
fn split_conjuncts(block: &qsl_forms::ExpressionForm) -> Vec<(ExprRef<'_>, Option<SpanId>)> {
    let mut conjuncts = Vec::new();
    let mut pending = vec![(block.expression.root(), Some(block.spans.root()))];
    while let Some((node, located)) = pending.pop() {
        match node.node() {
            ExprNode::Binary {
                operator: BinaryOperator::And,
                left,
                right,
            } => {
                pending.push((
                    node.at(*right),
                    located.and_then(|id| block.spans.child(id, 1)),
                ));
                pending.push((
                    node.at(*left),
                    located.and_then(|id| block.spans.child(id, 0)),
                ));
            }
            _ => conjuncts.push((node, located)),
        }
    }
    conjuncts
}

/// Whether the conjunct rooted at `node` reads the step of a trace variable
/// for which `existential` holds (ADR-023 HM-3): the one classification that
/// splits `mu` into `mu_U` and `mu_E`.
fn reads_existential(node: ExprRef<'_>, existential: impl Fn(&str) -> bool) -> bool {
    let mut subtree = vec![node];
    while let Some(current) = subtree.pop() {
        if let ExprNode::StepLabel { trace, .. } = current.node() {
            if existential(trace) {
                return true;
            }
        }
        subtree.extend(current.children());
    }
    false
}

/// The spans of the `match` conjuncts of `form`: the universal ones and the
/// existential ones, each in source order, split and classified as
/// [`check_match`] does.
pub(crate) fn conjunct_spans(form: &qsl_forms::HyperBehavioursForm) -> (Vec<Span>, Vec<Span>) {
    let Some(block) = &form.match_block else {
        return (Vec::new(), Vec::new());
    };
    let (mut universal, mut existential) = (Vec::new(), Vec::new());
    for (node, located) in split_conjuncts(block) {
        let Some(span) = located.and_then(|id| block.spans.span(id)) else {
            continue;
        };
        let exists = reads_existential(node, |name| {
            form.quantifiers.iter().any(|quantifier| {
                quantifier.kind == QuantifierKind::Exists && quantifier.variable.name == name
            })
        });
        if exists {
            existential.push(span);
        } else {
            universal.push(span);
        }
    }
    (universal, existential)
}

/// The expression span trees of the `match` conjuncts of `form`, rooted at
/// each conjunct and classified as [`conjunct_spans`] does: the universal
/// ones and the existential ones, each in source order.
pub(crate) fn conjunct_trees(
    form: &qsl_forms::HyperBehavioursForm,
) -> (
    Vec<qsl_forms::ExpressionSpans>,
    Vec<qsl_forms::ExpressionSpans>,
) {
    let Some(block) = &form.match_block else {
        return (Vec::new(), Vec::new());
    };
    let (mut universal, mut existential) = (Vec::new(), Vec::new());
    for (node, located) in split_conjuncts(block) {
        let Some(tree) = located.and_then(|id| block.spans.subtree(id)) else {
            continue;
        };
        let exists = reads_existential(node, |name| {
            form.quantifiers.iter().any(|quantifier| {
                quantifier.kind == QuantifierKind::Exists && quantifier.variable.name == name
            })
        });
        if exists {
            existential.push(tree);
        } else {
            universal.push(tree);
        }
    }
    (universal, existential)
}

/// FR-172 "Match": `mu` reads step labels only, and splits into its top-level
/// conjuncts (ADR-023 HM-3): the ones that read universal variables only, and
/// the ones that read an existential variable.
#[allow(clippy::type_complexity)]
fn check_match(
    block: &qsl_forms::ExpressionForm,
    variables: &Variables<'_>,
    declarations: &HyperDeclarations<'_>,
    found: &mut Vec<Found>,
) -> Result<(Vec<ConjunctDraft>, Vec<ConjunctDraft>), StageFailure<CheckRefusal>> {
    let expression = &block.expression;
    let conjuncts = split_conjuncts(block);
    let mut parents: Vec<Option<ExprId>> = vec![None; expression.len()];
    for node in expression.iter() {
        for child in node.node().children() {
            if let Some(slot) = parents.get_mut(child.index()) {
                *slot = Some(node.id());
            }
        }
    }
    let mut universal = Vec::new();
    let mut existential = Vec::new();
    for (conjunct, located) in conjuncts {
        let span = located
            .and_then(|id| block.spans.span(id))
            .ok_or_else(|| fault("conjunct-has-span"))?;
        let mut reads = Vec::new();
        let mut variable_names: Vec<String> = Vec::new();
        let mut labels: BTreeMap<String, String> = BTreeMap::new();
        let mut subtree = vec![(conjunct, located)];
        let found_before = found.len();
        while let Some((node, located)) = subtree.pop() {
            let at = located.and_then(|id| block.spans.span(id)).unwrap_or(span);
            match node.node() {
                ExprNode::StepLabel { trace, label } => {
                    note_variable(&mut variable_names, trace);
                    match step_read(trace, label, variables, declarations, at) {
                        Ok(read) => reads.push(read),
                        Err(cause) => found.push(Found::Hyper(cause)),
                    }
                    if matches!(label, StepLabelForm::Operation)
                        && !label_compared(node, trace, &parents, variables, declarations)
                    {
                        // The operation is misused by the comparison it
                        // sits in, so the conjunct is the refused span.
                        found.push(Found::Hyper(HyperCause::BodyIllTyped { span }));
                    }
                }
                ExprNode::Stutter => reads.push(StepRead::Stutter),
                ExprNode::Name(name) => match qualify(declarations, name) {
                    Qualified::Operation(alias, key) => {
                        if label_constant_of(node, &alias, &parents, variables) {
                            labels.insert(name.name.clone(), operation_text(&key));
                        } else {
                            found.push(Found::Hyper(HyperCause::BodyIllTyped { span: at }));
                        }
                    }
                    Qualified::Nothing => found.push(Found::Hyper(HyperCause::BodyMissing {
                        name: name.name.clone(),
                        span: at,
                    })),
                    Qualified::Member | Qualified::Other => {
                        found.push(Found::Hyper(HyperCause::MatchReadsState { span: at }));
                    }
                },
                ExprNode::Boolean(_)
                | ExprNode::Absent
                | ExprNode::Integer(_)
                | ExprNode::Float32(_)
                | ExprNode::Float64(_)
                | ExprNode::Rational(..)
                | ExprNode::Quantity(_)
                | ExprNode::String(_)
                | ExprNode::Binary { .. }
                | ExprNode::Not(_)
                | ExprNode::Negate(_)
                | ExprNode::If { .. }
                | ExprNode::Present(_)
                | ExprNode::Value(_)
                | ExprNode::Call { .. } => {}
                ExprNode::Case(_)
                | ExprNode::Let { .. }
                | ExprNode::Field { .. }
                | ExprNode::Deref(_)
                | ExprNode::Record { .. }
                | ExprNode::Collection { .. }
                | ExprNode::Convert { .. }
                | ExprNode::Query { .. }
                | ExprNode::Flatten(_)
                | ExprNode::Accumulate { .. }
                | ExprNode::Count { .. }
                | ExprNode::Sum { .. }
                | ExprNode::Population { .. }
                | ExprNode::Seq { .. }
                | ExprNode::Size(_)
                | ExprNode::Contains { .. }
                | ExprNode::AllInstances { .. }
                | ExprNode::Lookup { .. }
                | ExprNode::Dispatch { .. }
                | ExprNode::Pre(_)
                | ExprNode::SelfRef
                | ExprNode::Result
                | ExprNode::Reaches { .. }
                | ExprNode::Indexed { .. }
                | ExprNode::ProtocolReference { .. } => {
                    found.push(Found::Hyper(HyperCause::MatchReadsState { span: at }));
                }
                #[cfg(seam_probe)]
                ExprNode::__SeamProbe => {}
            }
            for (index, child) in node.children().into_iter().enumerate().rev() {
                subtree.push((child, located.and_then(|id| block.spans.child(id, index))));
            }
        }
        // A label misused by the form around it is `ill_typed`; the state
        // reads of that same form are not reported beside it.
        if found[found_before..]
            .iter()
            .any(|f| matches!(f, Found::Hyper(HyperCause::BodyIllTyped { .. })))
        {
            let mut index = 0;
            found.retain(|f| {
                let keep = index < found_before
                    || !matches!(f, Found::Hyper(HyperCause::MatchReadsState { .. }));
                index += 1;
                keep
            });
        }
        let existential_conjunct = reads_existential(conjunct, |name| {
            variables.kind_of(name) == Some(QuantifierKind::Exists)
        });
        let mut slots: Vec<StepRead> = Vec::new();
        for read in &reads {
            if !matches!(read, StepRead::Stutter) && !slots.contains(read) {
                slots.push(read.clone());
            }
        }
        let draft = ConjunctDraft {
            root: conjunct.id(),
            span,
            variables: variable_names,
            reads,
            slots,
            labels,
        };
        if existential_conjunct {
            existential.push(draft);
        } else {
            universal.push(draft);
        }
    }
    Ok((universal, existential))
}

/// Whether the operation label `node` of `trace` is compared as FR-172
/// "Match" admits: by `=` or `!=` with the label of a trace variable over the
/// same model, `stutter`, or a label constant of that model.
fn label_compared(
    node: ExprRef<'_>,
    trace: &str,
    parents: &[Option<ExprId>],
    variables: &Variables<'_>,
    declarations: &HyperDeclarations<'_>,
) -> bool {
    let Some(alias) = variables.alias_of(trace) else {
        return true;
    };
    let Some(parent) = parents.get(node.id().index()).copied().flatten() else {
        return false;
    };
    let ExprNode::Binary {
        operator: BinaryOperator::Equal | BinaryOperator::NotEqual,
        left,
        right,
    } = node.tree().at(parent).node()
    else {
        return false;
    };
    let other = if *left == node.id() { *right } else { *left };
    match node.tree().at(other).node() {
        ExprNode::Stutter => true,
        ExprNode::StepLabel {
            trace: other,
            label: StepLabelForm::Operation,
        } => variables.alias_of(other) == Some(alias),
        // A name that is no operation is refused as the state read or the
        // missing name it is, not as a misused label.
        ExprNode::Name(name) => match qualify(declarations, name) {
            Qualified::Operation(written, _) => written == alias.as_str(),
            Qualified::Nothing | Qualified::Member | Qualified::Other => true,
        },
        _ => false,
    }
}

/// Whether the label constant `node` of the model `alias` is written as
/// FR-172 "Match" admits: an operand of `=` or `!=` whose other operand is
/// the operation label of a trace variable bound to that alias.
fn label_constant_of(
    node: ExprRef<'_>,
    alias: &str,
    parents: &[Option<ExprId>],
    variables: &Variables<'_>,
) -> bool {
    let Some(parent) = parents.get(node.id().index()).copied().flatten() else {
        return false;
    };
    let ExprNode::Binary {
        operator: BinaryOperator::Equal | BinaryOperator::NotEqual,
        left,
        right,
    } = node.tree().at(parent).node()
    else {
        return false;
    };
    let other = if *left == node.id() { *right } else { *left };
    matches!(
        node.tree().at(other).node(),
        ExprNode::StepLabel { trace, label: StepLabelForm::Operation }
            if variables.alias_of(trace).is_some_and(|bound| bound.as_str() == alias)
    )
}

fn note_variable(names: &mut Vec<String>, name: &str) {
    if !names.iter().any(|known| known == name) {
        names.push(name.to_owned());
    }
}

/// The typed read of a step label (FR-172 "Indexed atoms and step labels").
fn step_read(
    trace: &str,
    label: &StepLabelForm,
    variables: &Variables<'_>,
    declarations: &HyperDeclarations<'_>,
    span: Span,
) -> Result<StepRead, HyperCause> {
    let alias = variables
        .alias_of(trace)
        .ok_or_else(|| HyperCause::UnknownTrace {
            name: trace.to_owned(),
            span,
        })?;
    match label {
        StepLabelForm::Operation => Ok(StepRead::Operation {
            trace: trace.to_owned(),
        }),
        StepLabelForm::Receiver => Ok(StepRead::Receiver {
            trace: trace.to_owned(),
            object: receiver_object(declarations, alias.as_str())
                .ok_or(HyperCause::MatchReadsState { span })?,
        }),
        StepLabelForm::Argument {
            operation,
            parameter,
        } => {
            let unresolved = || HyperCause::UnresolvedStepLabel {
                operation: operation.clone(),
                span,
            };
            let name = NameForm {
                name: format!("{}::{operation}", alias.as_str()),
                segments: vec![alias.as_str().to_owned(), operation.clone()].into(),
                span,
            };
            let (resolved, declaration) =
                resolve_operation(declarations, alias.as_str(), &name).map_err(|_| unresolved())?;
            let index = declaration
                .parameters()
                .iter()
                .position(|(declared, _)| declared == parameter)
                .ok_or_else(unresolved)?;
            let (_, value_type) = &declaration.parameters()[index];
            if !decodable_argument(value_type) {
                return Err(HyperCause::BodyUnsupported { span });
            }
            Ok(StepRead::Argument(Box::new(ArgumentRead {
                trace: trace.to_owned(),
                operation: operation_key(declarations.models, alias.as_str(), &resolved)
                    .ok_or_else(unresolved)?,
                written: operation.clone(),
                parameter: parameter.clone(),
                index,
                parameter_type: value_type.clone(),
            })))
        }
    }
}

/// Whether the evaluator decodes a step argument of type `value_type` from
/// the step's canonical argument text (FR-181): Booleans, integers, strings,
/// references and options of them. The one statement of that list: S3 refuses
/// a conjunct reading any other type, and the evaluator's decoder admits
/// exactly these types.
pub fn decodable_argument(value_type: &ValueType) -> bool {
    match value_type {
        ValueType::Boolean
        | ValueType::Integer
        | ValueType::Int(_)
        | ValueType::String
        | ValueType::Reference(_) => true,
        ValueType::Option(payload) => decodable_argument(payload),
        _ => false,
    }
}

/// The one object type that declares every operation of the model of `alias`
/// that has a receiver (QSpec FR-395 "Step labels"); `None` when those
/// operations are declared by more than one type, or by none.
fn receiver_object(declarations: &HyperDeclarations<'_>, alias: &str) -> Option<EffectiveId> {
    let model = model_of(declarations.models, alias)?;
    let objects: Vec<&quire_checked::key::DeclarationKey> =
        model.object_types().map(|(key, _)| key).collect();
    let mut declaring: Vec<EffectiveId> = Vec::new();
    for operation in model.model_index().operations() {
        if !objects.contains(&&operation.owner) {
            continue;
        }
        let owner = model.effective_id(&operation.owner)?;
        if !declaring.contains(&owner) {
            declaring.push(owner);
        }
    }
    match declaring.as_slice() {
        [only] => Some(only.clone()),
        _ => None,
    }
}

/// FR-172 "Align skip": each entry names an operation of a bound alias's
/// model, once; the list is not empty; and no variable of the clause has a
/// fairness constraint (ADR-023 PA-1, PA-3).
fn checked_align_skip(
    form: &HyperBehavioursForm,
    quantifiers: &[CheckedQuantifier],
    aliases: &[ModelAlias],
    declarations: &HyperDeclarations<'_>,
    found: &mut Vec<Found>,
) -> Vec<OperationKey> {
    let Some(skip) = &form.align_skip else {
        return Vec::new();
    };
    if skip.operations.is_empty() {
        found.push(Found::Hyper(HyperCause::AlignSkipEmpty { span: skip.span }));
    }
    let mut keys: Vec<OperationKey> = Vec::new();
    for entry in &skip.operations {
        let alias = entry.name.split("::").next().unwrap_or_default();
        if !aliases.iter().any(|bound| bound.as_str() == alias) {
            found.push(Found::Hyper(HyperCause::AlignSkipUnbound {
                span: entry.span,
            }));
            continue;
        }
        let key = resolve_operation(declarations, alias, entry)
            .ok()
            .and_then(|(operation, _)| operation_key(declarations.models, alias, &operation));
        match key {
            None => found.push(Found::Hyper(HyperCause::AlignSkipUnbound {
                span: entry.span,
            })),
            Some(key) if keys.contains(&key) => {
                found.push(Found::Hyper(HyperCause::AlignSkipRepeated {
                    span: entry.span,
                }));
            }
            Some(key) => keys.push(key),
        }
    }
    if quantifiers
        .iter()
        .any(|quantifier| !quantifier.fairness.is_empty())
    {
        for written in form
            .quantifiers
            .iter()
            .flat_map(|quantifier| &quantifier.fairness)
        {
            found.push(Found::Hyper(HyperCause::AlignSkipFairness {
                span: written.span,
            }));
        }
    }
    keys
}

/// The form of a clause over `behaviours` under infinite-trace (ADR-023 section
/// 3, in the order FR-173 states).
fn classify(quantifiers: &[CheckedQuantifier], aligned: bool, safety: bool) -> HyperForm {
    let exists = |quantifier: &&CheckedQuantifier| quantifier.kind == QuantifierKind::Exists;
    let any_exists = quantifiers
        .iter()
        .any(|quantifier| quantifier.kind == QuantifierKind::Exists);
    if let [only] = quantifiers {
        if only.kind == QuantifierKind::Exists && !aligned {
            return HyperForm::SingleExistential;
        }
    }
    let first_exists = quantifiers
        .iter()
        .position(|quantifier| quantifier.kind == QuantifierKind::Exists);
    let last_forall = quantifiers
        .iter()
        .rposition(|quantifier| quantifier.kind == QuantifierKind::Forall);
    if first_exists
        .zip(last_forall)
        .is_some_and(|(exists, forall)| exists < forall)
    {
        return HyperForm::Other {
            reason: HyperUnsupported::ExistentialBeforeUniversal,
        };
    }
    let existential_fairness = quantifiers
        .iter()
        .filter(exists)
        .any(|quantifier| !quantifier.fairness.is_empty());
    if any_exists && quantifiers.len() >= 2 && (!safety || existential_fairness) {
        return HyperForm::Other {
            reason: HyperUnsupported::ExistentialOutsideSafety,
        };
    }
    if aligned && (any_exists || !safety) {
        return HyperForm::Other {
            reason: HyperUnsupported::ProjectionOutsideSafety,
        };
    }
    match (any_exists, aligned) {
        (false, true) => HyperForm::ProjectionAligned,
        (false, false) => HyperForm::Universal,
        (true, _) => HyperForm::ForallExistsSafety,
    }
}

// ---------------------------------------------------------------------------
// Normal form, copy-swap and identity (FR-174, ADR-023 HV-7)
// ---------------------------------------------------------------------------

/// The encoding of one subtree in the normal form of FR-174: the operands of
/// `and`, `or`, `=` and `!=` sorted by their encodings, nested `and` and `or`
/// flattened, and every other node in place. `Associative` keeps its operands
/// apart so a parent of the same operator can flatten them.
#[derive(Clone)]
enum Encoded {
    Plain(Vec<u8>),
    Associative(u8, Vec<Vec<u8>>),
}

impl Encoded {
    fn into_bytes(self) -> Vec<u8> {
        match self {
            Self::Plain(bytes) => bytes,
            Self::Associative(tag, operands) => node_bytes(&[tag], &operands),
        }
    }
}

/// `tag`, then each of `parts` length-prefixed.
pub(super) fn node_bytes(tag: &[u8], parts: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    push_segment(&mut out, tag);
    out.extend_from_slice(&u64::try_from(parts.len()).unwrap_or(u64::MAX).to_be_bytes());
    for part in parts {
        push_segment(&mut out, part);
    }
    out
}

fn push_segment(out: &mut Vec<u8>, segment: &[u8]) {
    out.extend_from_slice(
        &u64::try_from(segment.len())
            .unwrap_or(u64::MAX)
            .to_be_bytes(),
    );
    out.extend_from_slice(segment);
}

fn text_segment(text: &str) -> Vec<u8> {
    text.as_bytes().to_vec()
}

/// The trace variables `a` and `b` exchanged, or none.
#[derive(Clone, Copy)]
struct Swap<'s>(Option<(&'s str, &'s str)>, Option<&'s str>);

impl Swap<'_> {
    fn apply(&self, trace: &str) -> String {
        match self.0 {
            Some((first, second)) if trace == first => second.to_owned(),
            Some((first, second)) if trace == second => first.to_owned(),
            _ => trace.to_owned(),
        }
    }
}

/// The nodes whose encoding cannot be compared under a swap: they bind names or
/// carry a payload this normal form leaves in place, so a trace variable
/// inside one makes the clause not provably symmetric.
fn is_opaque(node: &ExprNode) -> bool {
    match node {
        ExprNode::Case(_)
        | ExprNode::Let { .. }
        | ExprNode::Record { .. }
        | ExprNode::Collection { .. }
        | ExprNode::Convert { .. }
        | ExprNode::Query { .. }
        | ExprNode::Flatten(_)
        | ExprNode::Accumulate { .. }
        | ExprNode::Count { .. }
        | ExprNode::Sum { .. }
        | ExprNode::Population { .. }
        | ExprNode::Seq { .. }
        | ExprNode::AllInstances { .. }
        | ExprNode::Lookup { .. }
        | ExprNode::Dispatch { .. }
        | ExprNode::Quantity(_)
        | ExprNode::Reaches { .. }
        | ExprNode::ProtocolReference { .. } => true,
        ExprNode::Boolean(_)
        | ExprNode::Absent
        | ExprNode::Integer(_)
        | ExprNode::Float32(_)
        | ExprNode::Float64(_)
        | ExprNode::Rational(..)
        | ExprNode::String(_)
        | ExprNode::Name(_)
        | ExprNode::If { .. }
        | ExprNode::Binary { .. }
        | ExprNode::Negate(_)
        | ExprNode::Not(_)
        | ExprNode::Field { .. }
        | ExprNode::Present(_)
        | ExprNode::Value(_)
        | ExprNode::Deref(_)
        | ExprNode::Call { .. }
        | ExprNode::Size(_)
        | ExprNode::Contains { .. }
        | ExprNode::Pre(_)
        | ExprNode::SelfRef
        | ExprNode::Result
        | ExprNode::Indexed { .. }
        | ExprNode::StepLabel { .. }
        | ExprNode::Stutter => false,
        #[cfg(seam_probe)]
        ExprNode::__SeamProbe => false,
    }
}

/// The normal form of `expression` under `swap`: the encoding of its root and
/// of each node in `keep`, and whether an opaque node reads a trace variable.
/// Each node's children precede it in the arena, so one forward pass
/// encodes every node from its children's encodings.
struct Encodings {
    root: Vec<u8>,
    kept: Vec<Vec<u8>>,
    opaque_mention: bool,
}

fn encode_expression(expression: &Expression, swap: Swap<'_>, keep: &[ExprId]) -> Encodings {
    let mut encoded: Vec<Option<Encoded>> = Vec::with_capacity(expression.len());
    let mut mentions: Vec<bool> = Vec::with_capacity(expression.len());
    let mut kept: Vec<Vec<u8>> = vec![Vec::new(); keep.len()];
    let mut opaque_mention = false;
    for node in expression.iter() {
        let children = node.node().children();
        let operands: Vec<Encoded> = children
            .iter()
            .filter_map(|child| {
                let slot = encoded.get_mut(child.index())?;
                if keep.contains(child) {
                    slot.clone()
                } else {
                    slot.take()
                }
            })
            .collect();
        let child_mentions = children
            .iter()
            .any(|child| mentions.get(child.index()).copied().unwrap_or(false));
        if is_opaque(node.node()) && child_mentions {
            opaque_mention = true;
        }
        let value = match node.node() {
            ExprNode::Binary { operator, .. } if operands.len() == 2 => {
                let (tag, commutes) = binary_tag(*operator);
                match commutes {
                    Commutes::Associative => Encoded::Associative(tag, flatten(tag, operands)),
                    Commutes::Symmetric => {
                        let mut operands: Vec<Vec<u8>> =
                            operands.into_iter().map(Encoded::into_bytes).collect();
                        operands.sort();
                        Encoded::Plain(node_bytes(&[tag], &operands))
                    }
                    Commutes::No => Encoded::Plain(node_bytes(
                        &[tag],
                        &operands
                            .into_iter()
                            .map(Encoded::into_bytes)
                            .collect::<Vec<_>>(),
                    )),
                }
            }
            other => {
                let (tag, payload) = leaf_payload(other, swap);
                let mut parts = vec![payload];
                parts.extend(operands.into_iter().map(Encoded::into_bytes));
                Encoded::Plain(node_bytes(&tag, &parts))
            }
        };
        if let Some(position) = keep.iter().position(|id| *id == node.id()) {
            if let Some(slot) = kept.get_mut(position) {
                *slot = value.clone().into_bytes();
            }
        }
        mentions.push(
            child_mentions
                || matches!(
                    node.node(),
                    ExprNode::Indexed { .. } | ExprNode::StepLabel { .. }
                ),
        );
        encoded.push(Some(value));
    }
    let root = encoded
        .pop()
        .flatten()
        .map(Encoded::into_bytes)
        .unwrap_or_default();
    Encodings {
        root,
        kept,
        opaque_mention,
    }
}

/// The operands of an associative node of `tag`, flattening operands of the
/// same tag, sorted.
fn flatten(tag: u8, operands: Vec<Encoded>) -> Vec<Vec<u8>> {
    let mut flattened: Vec<Vec<u8>> = Vec::new();
    for operand in operands {
        match operand {
            Encoded::Associative(inner, inner_operands) if inner == tag => {
                flattened.extend(inner_operands);
            }
            other => flattened.push(other.into_bytes()),
        }
    }
    flattened.sort();
    flattened
}

/// How a binary operator's operands may be reordered.
enum Commutes {
    Associative,
    Symmetric,
    No,
}

fn binary_tag(operator: BinaryOperator) -> (u8, Commutes) {
    match operator {
        BinaryOperator::And => (1, Commutes::Associative),
        BinaryOperator::Or => (2, Commutes::Associative),
        BinaryOperator::Equal => (3, Commutes::Symmetric),
        BinaryOperator::NotEqual => (4, Commutes::Symmetric),
        BinaryOperator::Implies => (5, Commutes::No),
        BinaryOperator::Less => (6, Commutes::No),
        BinaryOperator::LessOrEqual => (7, Commutes::No),
        BinaryOperator::Greater => (8, Commutes::No),
        BinaryOperator::GreaterOrEqual => (9, Commutes::No),
        BinaryOperator::Add => (10, Commutes::No),
        BinaryOperator::Subtract => (11, Commutes::No),
        BinaryOperator::Multiply => (12, Commutes::No),
        BinaryOperator::Divide => (13, Commutes::No),
        BinaryOperator::IntegerDiv => (14, Commutes::No),
        BinaryOperator::IntegerRem => (15, Commutes::No),
        BinaryOperator::IntegerMod => (16, Commutes::No),
    }
}

/// A node's tag and its own payload, without its operands. A node outside the
/// normal form's reach is kept in place by its debug rendering, which names
/// its operands by arena position: equal for equal clauses.
fn leaf_payload(node: &ExprNode, swap: Swap<'_>) -> (Vec<u8>, Vec<u8>) {
    match node {
        // A claim's `over` binder is not content: it encodes as a binder.
        ExprNode::Name(name) if swap.1 == Some(name.name.as_str()) => {
            (b"binder".to_vec(), Vec::new())
        }
        ExprNode::Name(name) => (b"name".to_vec(), text_segment(&name.name)),
        ExprNode::Boolean(value) => (b"bool".to_vec(), vec![u8::from(*value)]),
        ExprNode::Absent => (b"absent".to_vec(), Vec::new()),
        ExprNode::Integer(value) => (b"int".to_vec(), text_segment(&value.to_string())),
        ExprNode::Float32(bits) => (b"float32".to_vec(), text_segment(&format!("{bits:08x}"))),
        ExprNode::Float64(bits) => (b"float64".to_vec(), text_segment(&format!("{bits:016x}"))),
        ExprNode::String(text) => (b"str".to_vec(), text_segment(text)),
        ExprNode::Field { field, .. } => (b"field".to_vec(), text_segment(field)),
        ExprNode::Indexed { trace, .. } => (b"indexed".to_vec(), text_segment(&swap.apply(trace))),
        ExprNode::StepLabel { trace, label } => {
            let mut payload = text_segment(&swap.apply(trace));
            match label {
                StepLabelForm::Operation => payload.extend_from_slice(b"|op"),
                StepLabelForm::Receiver => payload.extend_from_slice(b"|receiver"),
                StepLabelForm::Argument {
                    operation,
                    parameter,
                } => {
                    payload.extend_from_slice(b"|arg|");
                    payload.extend_from_slice(operation.as_bytes());
                    payload.push(b'|');
                    payload.extend_from_slice(parameter.as_bytes());
                }
            }
            (b"step".to_vec(), payload)
        }
        ExprNode::Stutter => (b"stutter".to_vec(), Vec::new()),
        ExprNode::Not(_) => (b"not".to_vec(), Vec::new()),
        ExprNode::Negate(_) => (b"neg".to_vec(), Vec::new()),
        ExprNode::Present(_) => (b"present".to_vec(), Vec::new()),
        ExprNode::Value(_) => (b"value".to_vec(), Vec::new()),
        ExprNode::Deref(_) => (b"deref".to_vec(), Vec::new()),
        ExprNode::Pre(_) => (b"pre".to_vec(), Vec::new()),
        ExprNode::Size(_) => (b"size".to_vec(), Vec::new()),
        ExprNode::Contains { .. } => (b"contains".to_vec(), Vec::new()),
        ExprNode::If { .. } => (b"if".to_vec(), Vec::new()),
        ExprNode::SelfRef => (b"self".to_vec(), Vec::new()),
        ExprNode::Result => (b"result".to_vec(), Vec::new()),
        ExprNode::Call { name, .. } => (b"call".to_vec(), text_segment(&name.name)),
        ExprNode::Rational(numerator, denominator) => (
            b"rational".to_vec(),
            text_segment(&format!("{numerator}/{denominator}")),
        ),
        ExprNode::Binary { .. }
        | ExprNode::Quantity(_)
        | ExprNode::Case(_)
        | ExprNode::Let { .. }
        | ExprNode::Record { .. }
        | ExprNode::Collection { .. }
        | ExprNode::Convert { .. }
        | ExprNode::Query { .. }
        | ExprNode::Flatten(_)
        | ExprNode::Accumulate { .. }
        | ExprNode::Count { .. }
        | ExprNode::Sum { .. }
        | ExprNode::Population { .. }
        | ExprNode::Seq { .. }
        | ExprNode::AllInstances { .. }
        | ExprNode::Lookup { .. }
        | ExprNode::Dispatch { .. }
        | ExprNode::Reaches { .. }
        | ExprNode::ProtocolReference { .. } => {
            (b"opaque".to_vec(), text_segment(&format!("{node:?}")))
        }
        #[cfg(seam_probe)]
        ExprNode::__SeamProbe => (b"probe".to_vec(), Vec::new()),
    }
}

/// The normal form of a temporal formula under `swap`: its operands after it,
/// so the arena is encoded from the last node to the first. Returns the root's
/// bytes and whether an opaque node reads a trace variable.
fn encode_formula(formula: &TemporalFormulaForm, swap: Swap<'_>) -> (Vec<u8>, bool) {
    let nodes = formula.nodes();
    let mut encoded: Vec<Option<Encoded>> = (0..nodes.len()).map(|_| None).collect();
    let mut opaque_mention = false;
    for (index, node) in nodes.iter().enumerate().rev() {
        let value = match node {
            TemporalNodeForm::Constant { value, .. } => {
                Encoded::Plain(node_bytes(b"constant", &[vec![u8::from(*value)]]))
            }
            TemporalNodeForm::Holds { condition, .. } => {
                let encodings = encode_expression(&condition.expression, swap, &[]);
                opaque_mention |= encodings.opaque_mention;
                Encoded::Plain(node_bytes(b"holds", &[encodings.root]))
            }
            TemporalNodeForm::AggregateAtom(atom) => {
                // This encoder is used for relation identity, not aggregate
                // checking. Preserve distinct authored sites conservatively;
                // FR-194's monitor checker owns aggregate semantics.
                opaque_mention = true;
                Encoded::Plain(node_bytes(
                    b"aggregate-unchecked",
                    &[
                        atom.span.start.to_be_bytes().to_vec(),
                        atom.span.end.to_be_bytes().to_vec(),
                    ],
                ))
            }
            TemporalNodeForm::Operator(operator) => {
                let operands: Vec<Encoded> = operator
                    .operands
                    .iter()
                    .filter_map(|operand| encoded.get_mut(operand.index()).and_then(Option::take))
                    .collect();
                let tag = temporal_tag(operator.operator);
                let interval = operator
                    .interval
                    .as_ref()
                    .map_or_else(Vec::new, |interval| match interval {
                        OperatorInterval::Steps(interval) => {
                            let upper = match interval.upper {
                                IntervalUpper::Finite(upper) => upper.to_be_bytes().to_vec(),
                                IntervalUpper::Open => b"*".to_vec(),
                            };
                            let mut bytes = interval.lower.to_be_bytes().to_vec();
                            bytes.extend(upper);
                            bytes
                        }
                        // A timed interval is refused when the clause is checked;
                        // it never reaches a comparison as a step interval.
                        OperatorInterval::Timed(_) => b"timed".to_vec(),
                    });
                match operator.operator {
                    TemporalOperator::And | TemporalOperator::Or => {
                        Encoded::Associative(tag, flatten(tag, operands))
                    }
                    _ => {
                        let mut parts = vec![interval];
                        parts.extend(operands.into_iter().map(Encoded::into_bytes));
                        Encoded::Plain(node_bytes(&[tag], &parts))
                    }
                }
            }
        };
        if let Some(slot) = encoded.get_mut(index) {
            *slot = Some(value);
        }
    }
    let root = encoded
        .get_mut(formula.root().index())
        .and_then(Option::take)
        .map(Encoded::into_bytes)
        .unwrap_or_default();
    (root, opaque_mention)
}

fn temporal_tag(operator: TemporalOperator) -> u8 {
    match operator {
        TemporalOperator::Not => 20,
        TemporalOperator::And => 1,
        TemporalOperator::Or => 2,
        TemporalOperator::Implies => 21,
        TemporalOperator::Eventually => 22,
        TemporalOperator::Always => 23,
        TemporalOperator::Once => 24,
        TemporalOperator::Historically => 25,
        TemporalOperator::Until => 26,
        TemporalOperator::Release => 27,
        TemporalOperator::Since => 28,
        TemporalOperator::Triggered => 29,
    }
}

/// The normal form of a probabilistic claim's state predicate (FR-174's
/// encoding of one expression), for the claim's normal form (FR-186). The
/// `over` binder, when named, is encoded as a binder, not as its name.
pub(super) fn encode_predicate(expression: &Expression, binder: Option<&str>) -> Vec<u8> {
    encode_expression(expression, Swap(None, binder), &[]).root
}

/// The normal form of a probabilistic claim's temporal event, for its
/// normal form (FR-186).
pub(super) fn encode_event(formula: &TemporalFormulaForm, binder: Option<&str>) -> Vec<u8> {
    encode_formula(formula, Swap(None, binder)).0
}

/// The encoding of the `match` conjuncts `predicates` of `expression`, as one
/// conjunction under `swap`, and whether an opaque node reads a trace variable.
fn encode_conjuncts(
    expression: Option<&Expression>,
    predicates: &[CheckedStepPredicate],
    swap: Swap<'_>,
) -> (Vec<u8>, bool) {
    let Some(expression) = expression else {
        return (Vec::new(), false);
    };
    let roots: Vec<ExprId> = predicates.iter().map(|predicate| predicate.root).collect();
    let encodings = encode_expression(expression, swap, &roots);
    let mut operands = encodings.kept;
    operands.sort();
    (
        node_bytes(b"conjunction", &operands),
        encodings.opaque_mention,
    )
}

/// FR-174 "Copy-swap detection": the classes of universal variables of one
/// alias with equal fairness sets whose exchange leaves `mu_U` and the body
/// equal in the normal form.
fn copy_group(
    quantifiers: &[CheckedQuantifier],
    body: &TemporalFormulaForm,
    match_expression: Option<&Expression>,
    mu_universal: &[CheckedStepPredicate],
) -> CopyGroup {
    let universal: Vec<&CheckedQuantifier> = quantifiers
        .iter()
        .filter(|quantifier| quantifier.kind == QuantifierKind::Forall)
        .collect();
    let (plain_body, _) = encode_formula(body, Swap(None, None));
    let (plain_mu, _) = encode_conjuncts(match_expression, mu_universal, Swap(None, None));
    let (plain_atoms, _) = atom_encodings(body, Swap(None, None));
    let swappable = |first: &CheckedQuantifier, second: &CheckedQuantifier| {
        if first.alias != second.alias || !same_set(&first.fairness, &second.fairness) {
            return false;
        }
        let swap = Swap(
            Some((first.variable.as_str(), second.variable.as_str())),
            None,
        );
        let (swapped_body, body_opaque) = encode_formula(body, swap);
        let (swapped_mu, mu_opaque) = encode_conjuncts(match_expression, mu_universal, swap);
        !body_opaque && !mu_opaque && swapped_body == plain_body && swapped_mu == plain_mu
    };
    // Classes by the equivalence "swappable": a variable joins the class of
    // the first earlier variable it can be swapped with.
    let mut class_of: Vec<usize> = (0..universal.len()).collect();
    for (later, second) in universal.iter().enumerate() {
        for (earlier, first) in universal.iter().enumerate().take(later) {
            if swappable(first, second) {
                let root = class_of[earlier];
                let previous = class_of[later];
                for slot in &mut class_of {
                    if *slot == previous {
                        *slot = root;
                    }
                }
                break;
            }
        }
    }
    let mut classes: BTreeMap<usize, Vec<&str>> = BTreeMap::new();
    for (position, variable) in universal.iter().enumerate() {
        classes
            .entry(class_of[position])
            .or_default()
            .push(variable.variable.as_str());
    }
    // A generator for each pair of consecutive variables of a class. A class
    // whose pair cannot be matched atom for atom is left out, so the group
    // stays inside the symmetries the test proved.
    let position = |name: &str| {
        quantifiers
            .iter()
            .position(|quantifier| quantifier.variable == name)
    };
    let mut kept: Vec<Vec<String>> = Vec::new();
    let mut generators = Vec::new();
    for class in classes.into_values().filter(|class| class.len() >= 2) {
        let pairs: Option<Vec<CopyGenerator>> = class
            .windows(2)
            .map(|pair| {
                let (swapped, _) = atom_encodings(body, Swap(Some((pair[0], pair[1])), None));
                Some(CopyGenerator {
                    first: position(pair[0])?,
                    second: position(pair[1])?,
                    atoms: atom_exchange(&plain_atoms, &swapped)?,
                })
            })
            .collect();
        if let Some(pairs) = pairs {
            generators.extend(pairs);
            kept.push(class.into_iter().map(str::to_owned).collect());
        }
    }
    CopyGroup {
        classes: kept,
        generators,
    }
}

/// The normal-form encoding of each `holds` atom of `body` under `swap`, in
/// arena order.
fn atom_encodings(body: &TemporalFormulaForm, swap: Swap<'_>) -> (Vec<Vec<u8>>, bool) {
    let mut opaque_mention = false;
    let encodings = body
        .nodes()
        .iter()
        .filter_map(|node| match node {
            TemporalNodeForm::Holds { condition, .. } => {
                let encodings = encode_expression(&condition.expression, swap, &[]);
                opaque_mention |= encodings.opaque_mention;
                Some(encodings.root)
            }
            _ => None,
        })
        .collect();
    (encodings, opaque_mention)
}

/// For each atom, the atom whose encoding in `plain` is its encoding in
/// `swapped`, equal encodings paired in arena order; `None` unless that is an
/// involution.
fn atom_exchange(plain: &[Vec<u8>], swapped: &[Vec<u8>]) -> Option<Vec<usize>> {
    let mut candidates: BTreeMap<&[u8], VecDeque<usize>> = BTreeMap::new();
    for (index, encoding) in plain.iter().enumerate() {
        candidates.entry(encoding).or_default().push_back(index);
    }
    let exchange: Vec<usize> = swapped
        .iter()
        .map(|encoding| candidates.get_mut(encoding.as_slice())?.pop_front())
        .collect::<Option<_>>()?;
    (0..exchange.len())
        .all(|atom| exchange[exchange[atom]] == atom)
        .then_some(exchange)
}

/// Whether two fairness sets hold the same constraints.
fn same_set(first: &[FairnessConstraint], second: &[FairnessConstraint]) -> bool {
    first.iter().all(|constraint| second.contains(constraint))
        && second.iter().all(|constraint| first.contains(constraint))
}

fn identity(
    aliases: &[ModelAlias],
    parameters: &[ObjectParameter],
    quantifiers: &[CheckedQuantifier],
    body: &TemporalFormulaForm,
    match_expression: Option<&Expression>,
    (mu_universal, mu_existential): (&[CheckedStepPredicate], &[CheckedStepPredicate]),
    align_skip: &[OperationKey],
) -> HyperIdentity {
    HyperIdentity {
        aliases: aliases.to_vec(),
        parameters: parameters.to_vec(),
        prefix: quantifiers
            .iter()
            .map(|quantifier| {
                let mut fairness = quantifier.fairness.clone();
                fairness.sort();
                (quantifier.kind, quantifier.alias.clone(), fairness)
            })
            .collect(),
        mu_universal: encode_conjuncts(match_expression, mu_universal, Swap(None, None)).0,
        mu_existential: encode_conjuncts(match_expression, mu_existential, Swap(None, None)).0,
        align_skip: align_skip.to_vec(),
        body: encode_formula(body, Swap(None, None)).0,
    }
}

// ---------------------------------------------------------------------------
// Relations (FR-172 "Relations over model executions", FR-180)
// ---------------------------------------------------------------------------

fn check_relation(
    form: &ModelRelationForm,
    declarations: &HyperDeclarations<'_>,
    cx: &mut CheckContext<'_, HyperDeclarations<'_>>,
) -> Result<CheckedModelRelation, StageFailure<CheckRefusal>> {
    let temporal = &declarations.temporal;
    let scope = temporal.scope;
    let mut found: Vec<Found> = Vec::new();
    let mut executions = Vec::with_capacity(form.executions.len());
    let mut execution_aliases = Vec::with_capacity(form.executions.len());
    let mut execution_fields = BTreeMap::new();
    let mut body_parameters = Vec::new();
    let mut roots: Vec<ValueType> = Vec::new();
    for binding in &form.executions {
        if execution_fields.contains_key(binding.variable.name.as_str()) {
            found.push(Found::Hyper(HyperCause::DuplicateName {
                name: binding.variable.name.clone(),
                span: binding.variable.span,
            }));
            continue;
        }
        let alias = binding
            .operation
            .name
            .split("::")
            .next()
            .unwrap_or_default();
        let resolved = model_of(declarations.models, alias)
            .and_then(|_| resolve_operation(declarations, alias, &binding.operation).ok());
        let Some((operation, declaration)) = resolved else {
            found.push(Found::Hyper(HyperCause::ExecutionNotAnOperation {
                name: binding.operation.name.clone(),
                span: binding.operation.span,
            }));
            continue;
        };
        let Some(key) = operation_key(declarations.models, alias, &operation) else {
            return Err(fault("execution-operation-keyed"));
        };
        roots.push(ValueType::Reference(operation.declaring_type()));
        roots.extend(
            declaration
                .parameters()
                .iter()
                .map(|(_, value_type)| value_type.clone()),
        );
        let mut fields = BTreeMap::new();
        let self_slot = body_parameters.len();
        let self_type = ValueType::Reference(operation.declaring_type());
        fields.insert("self".to_owned(), (self_slot, self_type.clone()));
        body_parameters.push((format!("{}.self", binding.variable.name), self_type));
        if let Some(result) = declaration.result() {
            let slot = body_parameters.len();
            fields.insert("result".to_owned(), (slot, result.clone()));
            body_parameters.push((format!("{}.result", binding.variable.name), result.clone()));
        }
        for (name, value_type) in declaration.parameters() {
            let slot = body_parameters.len();
            fields.insert(name.clone(), (slot, value_type.clone()));
            body_parameters.push((
                format!("{}.{}", binding.variable.name, name),
                value_type.clone(),
            ));
        }
        execution_fields.insert(binding.variable.name.clone(), fields);
        executions.push((binding.variable.name.clone(), key));
        execution_aliases.push(alias.to_owned());
    }
    earliest(found)?;
    let mut nodes_used = declarations.nodes_used.get();
    let execution_roots = execution_fields
        .iter()
        .filter_map(|(name, fields)| {
            let (_, value_type) = fields.get("self")?;
            let ValueType::Reference(object) = value_type else {
                return None;
            };
            Some((name.clone(), object.clone()))
        })
        .collect();
    let execution_member_names = execution_fields
        .iter()
        .map(|(name, fields)| {
            (
                name.clone(),
                fields.keys().cloned().collect::<BTreeSet<_>>(),
            )
        })
        .collect();
    let hyper_typing = HyperTyping {
        reserved_roots: form
            .executions
            .iter()
            .map(|binding| binding.variable.name.clone())
            .collect(),
        executions: execution_fields,
        ..HyperTyping::default()
    };
    let body_location = Location::root(Origin::TypeDeclaration {
        name: format!("relation {} body", form.name.name),
    });
    let typed_body = Typer::new(
        scope,
        declarations.signatures,
        declarations.checking_limits,
        &mut nodes_used,
        qsl_forms::ClauseKind::Invariant,
    )
    // FR-180 relation bodies read the receiver state through execution
    // binders; they are checked in the model profile even though they are not
    // a state-clause body.
    .with_model_family(true)
    .with_cancel(declarations.cancel)
    .with_context(cx.reborrow(scope))
    .with_hyper(hyper_typing)
    .check_as(&form.body.expression, &ValueType::Boolean, &body_location)
    .map_err(|refusal| {
        body_refusal(
            refusal,
            &form.body.expression,
            &form.body.spans,
            &execution_roots,
            &execution_member_names,
            scope,
        )
        .into_failure()
    })?;
    declarations.nodes_used.set(nodes_used);
    let root_refs: Vec<&ValueType> = roots.iter().collect();
    let code_domains = classify_domains(&root_refs, scope.types(), temporal.position_limit)
        .map_err(|failure| match failure {
            ClassifyFailure::Limit(limit) => {
                super::temporal_clause::located(limit, temporal, form.span)
            }
            ClassifyFailure::Fault(fault) => StageFailure::Fault(fault),
        })?;
    Ok(CheckedModelRelation {
        name: form.name.name.clone(),
        span: form.span,
        executions,
        execution_aliases,
        body: form.body.expression.clone(),
        body_spans: form.body.spans.clone(),
        typed_body,
        body_parameters,
        claim: TemporalClaim::new(BTreeMap::new(), true, true),
        code_domains,
        identity: None,
        occurrence: None,
    })
}

/// Every `hyper` clause and `relation` of `package`, checked as the
/// assembler's check would (test-support only).
#[cfg(any(test, feature = "test-support"))]
pub fn check_hyper_clauses(
    package: &crate::check::PackageDeclarations,
) -> Vec<Result<CheckedHyper, StageFailure<CheckRefusal>>> {
    let complete_value = package
        .function_selections
        .values()
        .any(super::state_clause::is_complete_value)
        || package
            .state_clauses
            .iter()
            .any(|clause| super::state_clause::is_complete_value(&clause.selection));
    let mut scope = Scope::new(
        package.types.clone(),
        package.operations.clone(),
        package.enums.clone(),
        package.aliases.clone(),
        package.model_operations.clone(),
        package.ieee_profile.clone(),
        package.dispatch_operations.clone(),
    )
    .with_integer_division(package.integer_division)
    .with_complete_value(complete_value)
    .with_known_populations(super::refinement::population_names(&package.models))
    .with_complete_model(!package.models.is_empty())
    .with_model_selections(
        package
            .models
            .iter()
            .filter_map(|model| Some((model.alias()?.to_owned(), model.selection().clone())))
            .collect(),
    )
    .with_declared_units(&package.units);
    // The declared functions' signatures, which a body's calls resolve
    // against; a function that does not resolve is left out.
    let signatures = Signatures::new(
        package
            .functions
            .iter()
            .filter_map(|function| {
                let location = super::root_location();
                let (parameters, result) =
                    super::resolve_signature(&scope, function, &location).ok()?;
                Some(super::check::Signature {
                    name: function.name.clone(),
                    parameters,
                    result,
                    callable_by_name: function.callable_by_name(),
                })
            })
            .collect(),
    );
    let has_ieee_literal = package
        .hyper_clauses
        .iter()
        .any(declaration_has_ieee_literal);
    scope
        .select_declared_ieee_profile(signatures.as_slice(), has_ieee_literal)
        .ok();
    let declarations = HyperDeclarations {
        temporal: TemporalDeclarations {
            scope: &scope,
            selections: &package.temporal_selections,
            source: &package.source,
            package_name: &package.package_name,
            embedding: None,
            position_limit: 10_000,
            models: &package.models,
            protocols: &[],
            // A hyper body reads step intervals only.
            timed: super::temporal_clause::TimedContext {
                units: &super::time_units::TimeUnits::default(),
                time_sources: &[],
                rational_bits: 0,
            },
        },
        models: &package.models,
        signatures: &signatures,
        dispatch_tables: &package.dispatch_tables,
        checking_limits: quire_semantic_value::checking::CheckingLimits::default(),
        cancel: None,
        nodes_used: std::cell::Cell::new(0),
        behaviours_checked: std::cell::Cell::new(0),
    };
    package
        .hyper_clauses
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
            HyperTraceFamily::check(clause, &mut cx).map(Staged::into_value)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A step argument is decoded for the types the evaluator decodes, and
    /// the conjunct that reads any other is refused at S3.
    #[test]
    fn only_the_types_the_evaluator_decodes_are_readable_arguments() {
        assert!(decodable_argument(&ValueType::Boolean));
        assert!(decodable_argument(&ValueType::String));
        assert!(decodable_argument(&ValueType::Integer));
        assert!(!decodable_argument(&ValueType::Uuid));
        assert!(decodable_argument(&ValueType::Option(std::sync::Arc::new(
            ValueType::Boolean
        ))));
        assert!(!decodable_argument(&ValueType::Option(
            std::sync::Arc::new(ValueType::Timestamp)
        )));
    }
}
