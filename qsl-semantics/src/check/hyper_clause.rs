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

use std::collections::BTreeMap;

use qsl_forms::{
    BinaryOperator, ExprId, ExprNode, ExprRef, Expression, ExpressionSpans, HyperBehavioursForm,
    HyperFiniteForm, IntervalUpper, ModelRelationForm, NameForm, QuantifierForm, QuantifierKind,
    SpanId, StepLabelForm, TemporalFormulaForm, TemporalNodeForm, TemporalOperator, TypeFormHead,
};
use qsl_foundation::bound::{DomainKey, DomainKind};
use qsl_foundation::diagnostic::{Code, StageFailure, Staged};
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::{InternalFault, Span};
use quire_exact::{EffectiveId, Identifier, ValueType};
use quire_semantic_value::fairness::{FairnessConstraint, FairnessOperation};
use quire_semantic_value::location::{Location, Origin};

use super::abstraction::{AbstractionKey, CheckedAbstractionRelation, OperationKey};
use super::check::Signatures;
use super::hyper_typing::{type_condition, BodyFault, Env, ExecutionRoot, Root, Site, Ty};
use super::profile::TemporalProfile;
use super::refusal::{CheckCause, CheckRefusal, TemporalCause};
use super::state_clause::{population_of, PopulationDomain};
use super::temporal_clause::{
    build_formula, checked_constraint, checked_operators, in_safety_fragment, CheckedFormula,
    CheckedOperator, TemporalClaim, TemporalDeclarations, TemporalSelection,
};
#[cfg(any(test, feature = "test-support"))]
use super::Scope;
use super::{AdmittedModel, Capability};
use crate::family::requirements::{classify_domains, ClaimExtent, ClassifyFailure, Requirements};
use crate::family::{CheckContext, CheckOutcome, FamilyContract};
use crate::model::operation::OperationLookup;

/// The stage name this module's faults report.
const STAGE: &str = "check.hyper";

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
    /// The parameter `p`.
    pub parameter: String,
    /// The parameter's type `T`.
    pub parameter_type: ValueType,
}

impl ArgumentRead {
    /// The type of the read: `Option<T>`, present exactly when the step is an
    /// `O` step.
    pub fn read_type(&self) -> ValueType {
        ValueType::Option(std::sync::Arc::new(self.parameter_type.clone()))
    }
}

/// One top-level conjunct of a `match` block (ADR-023 HM-3).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedStepPredicate {
    root: ExprId,
    span: Span,
    variables: Vec<String>,
    reads: Vec<StepRead>,
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

/// Which rule of ADR-023 section 3 places a clause in HP-4.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
}

/// A `relation` over model executions, checked (FR-172 "Outputs").
#[derive(Clone, Debug)]
pub struct CheckedModelRelation {
    name: String,
    span: Span,
    executions: Vec<(String, OperationKey)>,
    /// The typed body as S2 built it. FR-172 names a `CheckedExpression`;
    /// lowering the typed body to the checked node is S4 work and remains.
    body: Expression,
    claim: TemporalClaim,
    code_domains: BTreeMap<(usize, Vec<u32>), DomainKind>,
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
    pub fn requirements(&self, roots: &[WireNodeId]) -> Result<Requirements, InternalFault> {
        let mut domains = BTreeMap::new();
        for ((index, path), kind) in &self.domains {
            let node = roots
                .get(*index)
                .ok_or_else(|| InternalFault::new(STAGE, "root-node-keyed"))?;
            domains.insert(
                DomainKey::Node {
                    node: *node,
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

    /// The relation's body, every read of which was typed under the value
    /// profile: a Boolean over each execution's arguments, result and
    /// receiver's pre-state and post-state. S4 lowers it to FR-322 terms.
    pub fn body(&self) -> &Expression {
        &self.body
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
}

/// A clause checked by [`HyperTraceFamily::check`].
#[derive(Clone, Debug)]
pub enum CheckedHyper {
    /// A hyper clause over `behaviours`.
    Behaviours(Box<CheckedHyperClause>),
    /// A relation over model executions.
    Relation(CheckedModelRelation),
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
        check_declaration(form, cx.declarations()).map(Staged::new)
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
            Self::UnboundParameterAlias { .. }
            | Self::ParameterNotAnObject { .. }
            | Self::UnindexedRead { .. }
            | Self::BodyIllTyped { .. }
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
        }
    }
}

/// A refusal found while checking one clause, before the clause's earliest is
/// reported.
enum Found {
    Hyper(HyperCause),
    Temporal(TemporalCause),
}

impl Found {
    fn start(&self) -> usize {
        match self {
            Self::Hyper(cause) => cause.span().map_or(0, |span| span.start),
            Self::Temporal(cause) => cause.span().start,
        }
    }

    fn into_failure(self) -> StageFailure<CheckRefusal> {
        let cause = match self {
            Self::Hyper(cause) => CheckCause::Hyper(Box::new(cause)),
            Self::Temporal(cause) => CheckCause::Temporal(Box::new(cause)),
        };
        StageFailure::Refused(CheckRefusal {
            // A clause has no expression tree for a `Location` path to walk;
            // `region::DeclarationRegions::refusal_region` reads the cause's
            // own span, so this `Location` is never resolved.
            location: Location::root(Origin::Expression),
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

fn check_declaration(
    declaration: &HyperDeclaration,
    declarations: &HyperDeclarations<'_>,
) -> Result<CheckedHyper, StageFailure<CheckRefusal>> {
    match declaration {
        HyperDeclaration::Behaviours(form) => check_behaviours(form, declarations)
            .map(|clause| CheckedHyper::Behaviours(Box::new(clause))),
        HyperDeclaration::Finite(form) => Err(check_finite(form)),
        HyperDeclaration::Relation(form) => {
            check_relation(form, declarations).map(CheckedHyper::Relation)
        }
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

fn check_behaviours(
    form: &HyperBehavioursForm,
    declarations: &HyperDeclarations<'_>,
) -> Result<CheckedHyperClause, StageFailure<CheckRefusal>> {
    let temporal = &declarations.temporal;
    let scope = temporal.scope;
    let profile = match temporal.selections {
        [only] => only.profile,
        found => return Err(profile_selection(found, form.span)),
    };
    let mut found: Vec<Found> = Vec::new();

    let quantifiers = checked_quantifiers(form, declarations, &mut found)?;
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
    let operators = checked_operators(&form.body, false, &mut temporal_causes)?;
    found.extend(temporal_causes.into_iter().map(Found::Temporal));
    let atom_roots = || -> BTreeMap<&str, Root> {
        parameters
            .iter()
            .map(|(parameter, object)| (parameter.name.as_str(), Root::Object(*object)))
            .collect()
    };
    let no_label = |_: &str, _: &StepLabelForm| None;
    for (_, node) in form.body.iter() {
        if let TemporalNodeForm::Holds { condition, .. } = node {
            check_atom(
                &condition.expression,
                &condition.spans,
                &parameters,
                &variables,
                &mut found,
            );
            let env = Env {
                scope,
                signatures: declarations.signatures,
                roots: atom_roots(),
                site: Site::Body,
                label: &no_label,
            };
            found.extend(
                type_condition(&env, &condition.expression, &condition.spans)
                    .into_iter()
                    .map(body_fault),
            );
        }
    }

    let (match_expression, mu_universal, mu_existential) = match &form.match_block {
        Some(block) => {
            let split = check_match(block, &variables, declarations, &mut found)?;
            let label = |trace: &str, label: &StepLabelForm| match step_read(
                trace,
                label,
                &variables,
                declarations,
                root_span(&block.spans),
            ) {
                Ok(StepRead::Operation { .. } | StepRead::Stutter) => Some(Ty::Label),
                Ok(StepRead::Receiver { .. }) => Some(Ty::Receiver),
                Ok(StepRead::Argument(argument)) => Some(Ty::Argument(argument.parameter_type)),
                Err(_) => None,
            };
            let env = Env {
                scope,
                signatures: declarations.signatures,
                roots: BTreeMap::new(),
                site: Site::Match,
                label: &label,
            };
            found.extend(
                type_condition(&env, &block.expression, &block.spans)
                    .into_iter()
                    .map(body_fault),
            );
            (Some(block.expression.clone()), split.0, split.1)
        }
        None => (None, Vec::new(), Vec::new()),
    };

    let align_skip = checked_align_skip(form, &quantifiers, &aliases, declarations, &mut found);
    earliest(std::mem::take(&mut found))?;

    let intervals: BTreeMap<_, _> = operators
        .iter()
        .map(|operator| (operator.node, operator.interval))
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

    let roots: Vec<ValueType> = parameters
        .iter()
        .map(|(_, object)| ValueType::Reference(*object))
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
        claim: TemporalClaim::new(parameter_domains, profile == TemporalProfile::InfiniteTrace),
        identity,
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
fn checked_quantifiers(
    form: &HyperBehavioursForm,
    declarations: &HyperDeclarations<'_>,
    found: &mut Vec<Found>,
) -> Result<Vec<CheckedQuantifier>, StageFailure<CheckRefusal>> {
    let mut checked = Vec::with_capacity(form.quantifiers.len());
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
        checked.push(CheckedQuantifier {
            kind: quantifier.kind,
            variable: quantifier.variable.name.clone(),
            alias: ModelAlias(alias.name.clone()),
            fairness: checked_fairness(quantifier, &alias.name, declarations, found),
        });
    }
    Ok(checked)
}

/// A quantifier's fairness set (FR-172 "Fairness and profile"): each
/// constraint names an operation of the quantifier's own model.
fn checked_fairness(
    quantifier: &QuantifierForm,
    alias: &str,
    declarations: &HyperDeclarations<'_>,
    found: &mut Vec<Found>,
) -> Vec<FairnessConstraint> {
    let mut set: Vec<FairnessConstraint> = Vec::new();
    for written in &quantifier.fairness {
        match resolve_operation(declarations, alias, &written.operation) {
            Ok((operation, _)) => {
                let constraint = checked_constraint(written, operation);
                if !set.contains(&constraint) {
                    set.push(constraint);
                }
            }
            Err(cause) => found.push(Found::Temporal(cause)),
        }
    }
    set
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
        let alias = written.split("::").next().unwrap_or_default();
        if !aliases.iter().any(|bound| bound.as_str() == alias) {
            found.push(Found::Hyper(HyperCause::UnboundParameterAlias {
                parameter: name,
                span,
            }));
            continue;
        }
        let Some(object) = scope.object_type_named(written) else {
            found.push(Found::Hyper(HyperCause::ParameterNotAnObject {
                parameter: name,
                span,
            }));
            continue;
        };
        match population_of(declarations.models, object, scope.types()) {
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

/// A body fault as the cause it is reported as.
fn body_fault(fault: BodyFault) -> Found {
    Found::Hyper(match fault {
        BodyFault::Missing { name, span } => HyperCause::BodyMissing { name, span },
        BodyFault::IllTyped { span } => HyperCause::BodyIllTyped { span },
        BodyFault::Unsupported { span } => HyperCause::BodyUnsupported { span },
        BodyFault::Shadows { name, span } => HyperCause::BodyShadows { name, span },
    })
}

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
            ExprNode::Name(name) => return Some(name),
            other => node = node.at(chain_operand(other)?),
        }
    }
}

/// FR-172 "Indexed atoms and step labels": every read of an object
/// parameter's member in a body atom carries exactly one trace variable, bound
/// to the model that declares the parameter.
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
    walk(expression, spans, |visit| {
        let Some(span) = visit.span else {
            return;
        };
        match visit.node.node() {
            ExprNode::StepLabel { .. } | ExprNode::Stutter => {
                found.push(Found::Hyper(HyperCause::StepLabelInBody { span }));
            }
            ExprNode::Indexed { read, trace } => {
                let Some(alias) = variables.alias_of(trace) else {
                    found.push(Found::Hyper(HyperCause::UnknownTrace {
                        name: trace.clone(),
                        span,
                    }));
                    return;
                };
                let owner = read_root(visit.node.at(*read)).and_then(parameter);
                if owner.is_none_or(|owner| &owner.alias != alias) {
                    found.push(Found::Hyper(HyperCause::MismatchedTrace { span }));
                }
            }
            other => {
                let is_read = chain_operand(other).is_some()
                    && read_root(visit.node).and_then(parameter).is_some();
                let continues = visit.parent.is_some_and(|parent| {
                    chain_operand(parent.node()) == Some(visit.node.id())
                        || matches!(parent.node(), ExprNode::Indexed { read, .. } if *read == visit.node.id())
                });
                if is_read && !continues {
                    found.push(Found::Hyper(HyperCause::UnindexedRead { span }));
                }
            }
        }
    });
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
) -> Result<(Vec<CheckedStepPredicate>, Vec<CheckedStepPredicate>), StageFailure<CheckRefusal>> {
    let expression = &block.expression;
    let mut conjuncts: Vec<(ExprRef<'_>, Option<SpanId>)> = Vec::new();
    let mut pending = vec![(expression.root(), Some(block.spans.root()))];
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
    let mut universal = Vec::new();
    let mut existential = Vec::new();
    for (conjunct, located) in conjuncts {
        let span = located
            .and_then(|id| block.spans.span(id))
            .ok_or_else(|| fault("conjunct-has-span"))?;
        let mut reads = Vec::new();
        let mut variable_names: Vec<String> = Vec::new();
        let mut subtree = vec![(conjunct, located)];
        while let Some((node, located)) = subtree.pop() {
            let at = located.and_then(|id| block.spans.span(id)).unwrap_or(span);
            match node.node() {
                ExprNode::StepLabel { trace, label } => {
                    note_variable(&mut variable_names, trace);
                    match step_read(trace, label, variables, declarations, at) {
                        Ok(read) => reads.push(read),
                        Err(cause) => found.push(Found::Hyper(cause)),
                    }
                }
                ExprNode::Stutter => reads.push(StepRead::Stutter),
                ExprNode::Boolean(_)
                | ExprNode::Integer(_)
                | ExprNode::Rational(..)
                | ExprNode::Quantity(_)
                | ExprNode::String(_)
                | ExprNode::Binary { .. }
                | ExprNode::Not(_)
                | ExprNode::Negate(_)
                | ExprNode::If { .. }
                | ExprNode::Present(_)
                | ExprNode::Value(_) => {}
                ExprNode::Case(_)
                | ExprNode::Name(_)
                | ExprNode::Let { .. }
                | ExprNode::Field { .. }
                | ExprNode::Deref(_)
                | ExprNode::Call { .. }
                | ExprNode::Record { .. }
                | ExprNode::Collection { .. }
                | ExprNode::Convert { .. }
                | ExprNode::Query { .. }
                | ExprNode::Flatten(_)
                | ExprNode::Accumulate { .. }
                | ExprNode::Count { .. }
                | ExprNode::Sum { .. }
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
        let existential_conjunct = variable_names
            .iter()
            .any(|name| variables.kind_of(name) == Some(QuantifierKind::Exists));
        let predicate = CheckedStepPredicate {
            root: conjunct.id(),
            span,
            variables: variable_names,
            reads,
        };
        if existential_conjunct {
            existential.push(predicate);
        } else {
            universal.push(predicate);
        }
    }
    Ok((universal, existential))
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
                span,
            };
            let (resolved, declaration) =
                resolve_operation(declarations, alias.as_str(), &name).map_err(|_| unresolved())?;
            let (_, value_type) = declaration
                .parameters()
                .iter()
                .find(|(declared, _)| declared == parameter)
                .ok_or_else(unresolved)?;
            Ok(StepRead::Argument(Box::new(ArgumentRead {
                trace: trace.to_owned(),
                operation: operation_key(declarations.models, alias.as_str(), &resolved)
                    .ok_or_else(unresolved)?,
                parameter: parameter.clone(),
                parameter_type: value_type.clone(),
            })))
        }
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
fn node_bytes(tag: &[u8], parts: &[Vec<u8>]) -> Vec<u8> {
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
struct Swap<'s>(Option<(&'s str, &'s str)>);

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
        | ExprNode::AllInstances { .. }
        | ExprNode::Lookup { .. }
        | ExprNode::Dispatch { .. }
        | ExprNode::Quantity(_)
        | ExprNode::Reaches { .. }
        | ExprNode::ProtocolReference { .. } => true,
        ExprNode::Boolean(_)
        | ExprNode::Integer(_)
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
        ExprNode::Name(name) => (b"name".to_vec(), text_segment(name)),
        ExprNode::Boolean(value) => (b"bool".to_vec(), vec![u8::from(*value)]),
        ExprNode::Integer(value) => (b"int".to_vec(), text_segment(&value.to_string())),
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
        ExprNode::Call { name, .. } => (b"call".to_vec(), text_segment(name)),
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
            TemporalNodeForm::Operator(operator) => {
                let operands: Vec<Encoded> = operator
                    .operands
                    .iter()
                    .filter_map(|operand| encoded.get_mut(operand.index()).and_then(Option::take))
                    .collect();
                let tag = temporal_tag(operator.operator);
                let interval = operator.interval.map_or_else(Vec::new, |interval| {
                    let upper = match interval.upper {
                        IntervalUpper::Finite(upper) => upper.to_be_bytes().to_vec(),
                        IntervalUpper::Open => b"*".to_vec(),
                    };
                    let mut bytes = interval.lower.to_be_bytes().to_vec();
                    bytes.extend(upper);
                    bytes
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
    let (plain_body, _) = encode_formula(body, Swap(None));
    let (plain_mu, _) = encode_conjuncts(match_expression, mu_universal, Swap(None));
    let swappable = |first: &CheckedQuantifier, second: &CheckedQuantifier| {
        if first.alias != second.alias || !same_set(&first.fairness, &second.fairness) {
            return false;
        }
        let swap = Swap(Some((first.variable.as_str(), second.variable.as_str())));
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
    let mut classes: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    for (position, variable) in universal.iter().enumerate() {
        classes
            .entry(class_of[position])
            .or_default()
            .push(variable.variable.clone());
    }
    CopyGroup {
        classes: classes
            .into_values()
            .filter(|class| class.len() >= 2)
            .collect(),
    }
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
        mu_universal: encode_conjuncts(match_expression, mu_universal, Swap(None)).0,
        mu_existential: encode_conjuncts(match_expression, mu_existential, Swap(None)).0,
        align_skip: align_skip.to_vec(),
        body: encode_formula(body, Swap(None)).0,
    }
}

// ---------------------------------------------------------------------------
// Relations (FR-172 "Relations over model executions", FR-180)
// ---------------------------------------------------------------------------

fn check_relation(
    form: &ModelRelationForm,
    declarations: &HyperDeclarations<'_>,
) -> Result<CheckedModelRelation, StageFailure<CheckRefusal>> {
    let temporal = &declarations.temporal;
    let scope = temporal.scope;
    let mut found: Vec<Found> = Vec::new();
    let mut executions = Vec::with_capacity(form.executions.len());
    let mut roots: Vec<ValueType> = Vec::new();
    let mut body_roots: BTreeMap<&str, Root> = BTreeMap::new();
    for binding in &form.executions {
        if body_roots.contains_key(binding.variable.name.as_str()) {
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
        body_roots.insert(
            &binding.variable.name,
            Root::Execution(Box::new(ExecutionRoot {
                declaring: operation.declaring_type(),
                parameters: declaration.parameters().to_vec(),
                result: declaration.result().cloned(),
            })),
        );
        executions.push((binding.variable.name.clone(), key));
    }
    let no_label = |_: &str, _: &StepLabelForm| None;
    let env = Env {
        scope,
        signatures: declarations.signatures,
        roots: body_roots,
        site: Site::Body,
        label: &no_label,
    };
    found.extend(
        type_condition(&env, &form.body.expression, &form.body.spans)
            .into_iter()
            .map(body_fault),
    );
    earliest(found)?;
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
        body: form.body.expression.clone(),
        claim: TemporalClaim::new(BTreeMap::new(), true),
        code_domains,
    })
}

/// The refusal for a clause that checks but that no S4 emission writes yet.
pub(crate) fn not_emitted(name: String, span: Span) -> CheckRefusal {
    CheckRefusal {
        location: Location::root(Origin::Expression),
        cause: CheckCause::Temporal(Box::new(TemporalCause::NotEmitted { name, span })),
    }
}

impl CheckedHyper {
    /// The declared name and span of the clause.
    pub(crate) fn name_and_span(&self) -> (String, Span) {
        match self {
            Self::Behaviours(clause) => (clause.name.clone(), clause.span),
            Self::Relation(relation) => (relation.name.clone(), relation.span),
        }
    }
}

/// Every `hyper` clause and `relation` of `package`, checked as the
/// assembler's check would (test-support only: the production check refuses a
/// clause that checks until S4 emits it).
#[cfg(any(test, feature = "test-support"))]
pub fn check_hyper_clauses(
    package: &crate::check::PackageDeclarations,
) -> Vec<Result<CheckedHyper, StageFailure<CheckRefusal>>> {
    let scope = Scope::new(
        package.types.clone(),
        package.operations.clone(),
        package.enums.clone(),
        package.aliases.clone(),
        Vec::new(),
        None,
        Vec::new(),
    );
    let declarations = HyperDeclarations {
        temporal: TemporalDeclarations {
            scope: &scope,
            selections: &package.temporal_selections,
            source: &package.source,
            embedding: None,
            position_limit: 10_000,
        },
        models: &package.models,
        // The helper checks clauses over no declared function.
        signatures: &Signatures::default(),
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
