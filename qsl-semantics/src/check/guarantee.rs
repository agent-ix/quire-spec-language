// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL FR-529, QSpec FR-490: the S3 check of an authored implementation
//! guarantee declaration, a `Relation` family declaration beside the
//! abstraction relation (ADR-012 §3, ADR-017 AR-1).
//!
//! [`CheckedGraph::check_declared_guarantees`] applies QSpec FR-490 "Static
//! checks" 2 to 6 in order and produces one [`CheckedImplementationGuarantee`]
//! per declaration, or the first refusal of each declaration that is
//! refused. The guarantee's constructor is private to this module, so the
//! checker is its only writer. [`CheckedImplementationGuarantee::slots`]
//! derives QSpec FR-490 "Derived items" in the order the spec gives.
//!
//! A model element the guarantee reads that the relation does not bind is
//! not a declaration refusal: it is the guarantee item's own scoped
//! refusal ([`CheckedImplementationGuarantee::item_refusal`], QSpec
//! FR-490-AC-6), and no other item of the package is affected.

use std::collections::{BTreeMap, BTreeSet};

use qsl_forms::{
    GuaranteeDeclarationForm, GuaranteeMeasureForm, GuaranteeMemberForm, GuaranteeSuppliesForm,
    NameForm, TypeFormHead,
};
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::{Code, Span};
use quire_checked::key::DeclarationKey;
use quire_exact::{EffectiveId, Identifier, NodeKey, ValueType};
use quire_semantic_value::catalog::InternalFault;
use quire_semantic_value::fairness::{
    FairnessConstraint, FairnessGranularity, FairnessKind, PropertyForm,
};
use quire_semantic_value::location::{Location, Origin};

use super::abstraction::{
    name_parts, AbstractionKey, CheckedAbstractionRelation, CheckedBinding, OperationKey, RustPath,
};
use super::capability::{Capability, DeclarationFamily};
use super::node_key::{
    EnvironmentEntry, ImplementationGuaranteeTerm, MeasureEntry, ProgressEntry, SuppliesEntry,
};
use super::profile::TemporalProfile;
use super::rust_syntax::path_defect;
use super::temporal_clause::{CheckedFormula, FormulaNode};
use super::CheckedGraph;
use super::TemporalClauseNode;

/// What a refused name was written for.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum GuaranteeName {
    /// The `of P` temporal clause.
    Clause,
    /// The `using A` abstraction relation.
    Relation,
    /// The operation a `progress` or scheduling `environment` member names.
    Operation,
    /// The predicate an `assumes` member names.
    Assumption,
}

/// Which Rust string held a malformed path.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum RustMember {
    /// The `initial to "f"` member.
    Initial,
    /// The `progress ... decreases "m"` member of this operation.
    Measure(String),
}

/// Why S3 refused an authored `guarantee` declaration (QSpec FR-490
/// "Static checks" 2 to 6). Every variant names its member by span.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum GuaranteeRefusal {
    /// Check 2 or 4: a name that resolves to nothing the member needs.
    MissingName {
        /// What the name was written for.
        what: GuaranteeName,
        /// The name as written.
        name: String,
        /// The span of the name.
        span: Span,
    },
    /// Check 3: `P` does not select `quire.temporal.infinite-trace/v1`.
    ClauseNotInfiniteTrace {
        /// The clause `P`.
        clause: String,
        /// The span of the name `P`.
        span: Span,
    },
    /// Check 3: `P` is not over a model subject.
    ClauseNotModelSubject {
        /// The clause `P`.
        clause: String,
        /// The span of the name `P`.
        span: Span,
    },
    /// Check 3: `P`'s model alias differs from `A`'s.
    ModelAliasMismatch {
        /// The clause `P`.
        clause: String,
        /// The model alias of `P`'s subject.
        clause_alias: String,
        /// The model aliases `A` binds.
        relation_aliases: Vec<String>,
        /// The span of the name `P`.
        span: Span,
    },
    /// Check 4: an `assumes` name that resolves to a declaration other than
    /// a `predicate`.
    AssumptionNotPredicate {
        /// The name as written.
        name: String,
        /// The span of the name.
        span: Span,
    },
    /// Check 5: a Rust string whose content is not a `rust-path`.
    RustPath {
        /// The member that held the string.
        member: RustMember,
        /// The content of the string.
        spelling: String,
        /// The first malformed segment.
        segment: String,
        /// The span of the malformed segment.
        span: Span,
    },
    /// Check 5: `bound 0`, or a bound that is not a positive `uint`.
    InvalidBound {
        /// The operation's name as written.
        operation: String,
        /// The integer as written.
        text: String,
        /// The span of the integer.
        span: Span,
    },
    /// Check 6: two `progress` members for one operation.
    ConflictingProgress {
        /// The operation's name as written.
        operation: String,
        /// The span of the first member.
        first: Span,
        /// The span of the second member.
        second: Span,
    },
    /// Check 6: two `environment` members of one name.
    ConflictingEnvironment {
        /// The shared name.
        name: String,
        /// The span of the first member.
        first: Span,
        /// The span of the second member.
        second: Span,
    },
    /// A `relation` target that is not an `abstraction_relation` node.
    NotAbstractionRelation {
        /// The span of the member that names it.
        span: Span,
    },
    /// The relation `A` checks to has no node to name.
    RelationNode {
        /// The span of the name `A`.
        span: Span,
        /// Why.
        reason: String,
    },
    /// A check invariant is broken.
    Internal(InternalFault),
}

impl GuaranteeRefusal {
    /// The catalog code.
    pub fn code(&self) -> Code {
        match self {
            Self::MissingName { .. } => Code::MissingDeclaration,
            Self::ClauseNotInfiniteTrace { .. }
            | Self::ClauseNotModelSubject { .. }
            | Self::ModelAliasMismatch { .. }
            | Self::AssumptionNotPredicate { .. }
            | Self::NotAbstractionRelation { .. }
            | Self::RustPath { .. }
            | Self::InvalidBound { .. }
            | Self::ConflictingProgress { .. }
            | Self::ConflictingEnvironment { .. }
            | Self::RelationNode { .. } => Code::InvalidModelBinding,
            Self::Internal(_) => Code::RuntimeInvariant,
        }
    }

    /// The catalog cause.
    pub fn cause(&self) -> &'static str {
        match self {
            Self::MissingName { .. } => "missing-name",
            Self::ClauseNotInfiniteTrace { .. }
            | Self::ClauseNotModelSubject { .. }
            | Self::ModelAliasMismatch { .. }
            | Self::AssumptionNotPredicate { .. }
            | Self::NotAbstractionRelation { .. }
            | Self::RustPath { .. }
            | Self::InvalidBound { .. }
            | Self::RelationNode { .. } => "malformed-declaration",
            Self::ConflictingProgress { .. } | Self::ConflictingEnvironment { .. } => {
                "conflicting-binding"
            }
            Self::Internal(_) => "established-invariant-broken",
        }
    }

    /// The span of the member the refusal names, when it has one.
    pub fn span(&self) -> Option<Span> {
        match self {
            Self::MissingName { span, .. }
            | Self::ClauseNotInfiniteTrace { span, .. }
            | Self::ClauseNotModelSubject { span, .. }
            | Self::ModelAliasMismatch { span, .. }
            | Self::AssumptionNotPredicate { span, .. }
            | Self::NotAbstractionRelation { span }
            | Self::RustPath { span, .. }
            | Self::InvalidBound { span, .. }
            | Self::RelationNode { span, .. } => Some(*span),
            Self::ConflictingProgress { second, .. }
            | Self::ConflictingEnvironment { second, .. } => Some(*second),
            Self::Internal(_) => None,
        }
    }
}

/// The scoped refusal of the guarantee item: a model element the guarantee
/// reads that the relation does not bind (QSpec FR-490 "Static checks",
/// FR-490-AC-6), `missing_declaration`/`missing-name`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuaranteeItemRefusal {
    /// The first unbound element, in element order.
    pub element: AbstractionKey,
    /// Every unbound element, in element order.
    pub unbound: Vec<AbstractionKey>,
}

impl GuaranteeItemRefusal {
    /// `missing_declaration`.
    pub fn code(&self) -> Code {
        Code::MissingDeclaration
    }

    /// `missing-name`.
    pub fn cause(&self) -> &'static str {
        "missing-name"
    }

    /// The identity of the domain package that owns the unbound element.
    pub fn package(&self) -> &str {
        self.element.package()
    }
}

/// A `progress` measure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProgressMeasure {
    /// `decreases "m"`.
    Decreases(RustPath),
    /// `bound N`, a positive count.
    Bound(u64),
}

/// One `progress` member (QSpec FR-490): the no-divergence premise of an
/// operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgressMember {
    /// The operation's declaring object type node.
    pub context: NodeKey,
    /// The operation.
    pub operation: Identifier,
    /// The measure.
    pub measure: ProgressMeasure,
    /// The span of the member.
    pub source: Span,
}

/// What an `environment` member supplies.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Supplies {
    /// A scheduling premise: the implementation executes the operation under
    /// this fairness.
    Fairness {
        /// The kind, `weak` when unwritten.
        kind: FairnessKind,
        /// The granularity, `whole` when unwritten.
        granularity: FairnessGranularity,
        /// The operation's declaring object type node.
        context: NodeKey,
        /// The operation.
        operation: Identifier,
    },
    /// An assumption premise: the `predicate` node that holds of the
    /// environment.
    Assumption(NodeKey),
}

/// One `environment` member (QSpec FR-490).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnvironmentMember {
    /// The premise name.
    pub name: Identifier,
    /// The supplier text.
    pub supplier: String,
    /// What the premise supplies.
    pub supplies: Supplies,
    /// The span of the member.
    pub source: Span,
}

/// An operation of the model subject, by its declaring object type node.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SubjectOperation {
    /// The declaring object type's `model`/`object_type` node.
    pub context: NodeKey,
    /// The operation.
    pub operation: Identifier,
}

/// One constraint of `F_P`, with its operation resolved to nodes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClauseConstraint {
    /// The checked constraint.
    pub constraint: FairnessConstraint,
    /// The operation's declaring object type node.
    pub context: NodeKey,
    /// The operation.
    pub operation: Identifier,
}

/// The checked facts of the clause `P` a guarantee is of (QSpec FR-490
/// "Declaration form", FR-363, FR-362).
#[derive(Clone, Debug, PartialEq)]
pub struct GuaranteeClause {
    /// The `temporal`/`temporal_clause` node of `P`.
    pub node: NodeKey,
    /// The clause name `P`.
    pub name: String,
    /// FR-363's property form of `P`.
    pub form: PropertyForm,
    /// `F_P`: the clause's resolved fairness set, in the clause's order.
    pub fairness: Vec<ClauseConstraint>,
    /// The model alias of `P`'s subject.
    pub alias: String,
    /// The `model`/`object_type` node of `P`'s model subject.
    pub subject: NodeKey,
    /// Every operation of the model subject, by operation name.
    pub operations: Vec<SubjectOperation>,
    /// `P`'s checked formula.
    pub formula: CheckedFormula,
}

impl GuaranteeClause {
    /// Whether FR-363 classifies `P` as `liveness`.
    pub fn is_liveness(&self) -> bool {
        self.form == PropertyForm::Liveness
    }
}

/// The spans of the members a declaration is made of.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GuaranteeSpans {
    /// The whole declaration.
    pub declaration: Span,
    /// The `of P` name.
    pub clause: Span,
    /// The `using A` name.
    pub relation: Span,
    /// The `initial to "f";` member.
    pub initial: Span,
}

/// A checked implementation guarantee declaration (QSpec FR-488, FR-490).
#[derive(Clone, Debug, PartialEq)]
pub struct CheckedImplementationGuarantee {
    name: String,
    key: NodeKey,
    package: String,
    spans: GuaranteeSpans,
    clause: GuaranteeClause,
    relation: NodeKey,
    initial: RustPath,
    progress: Vec<ProgressMember>,
    environment: Vec<EnvironmentMember>,
    refusal: Option<GuaranteeItemRefusal>,
}

/// The closed roles of QSpec FR-492's premise slots and items, spelled as the
/// spec spells them.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SlotRole {
    /// `guarantee`: the FR-490 implementation-guarantee item.
    Guarantee,
    /// `model-claim`.
    ModelClaim,
    /// `initialization`.
    Initialization,
    /// `transition-simulation`.
    TransitionSimulation,
    /// `frame`.
    Frame,
    /// `environment`.
    Environment,
    /// `no-divergence`.
    NoDivergence,
    /// `fairness-correspondence`.
    FairnessCorrespondence,
    /// `composition`: the FR-491 composition item.
    Composition,
    /// `component-guarantee`.
    ComponentGuarantee,
    /// `rely`.
    Rely,
    /// `liveness-dependency`.
    LivenessDependency,
    /// `fairness`.
    Fairness,
    /// `fixed-point`.
    FixedPoint,
}

impl SlotRole {
    /// The role's spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Guarantee => "guarantee",
            Self::ModelClaim => "model-claim",
            Self::Initialization => "initialization",
            Self::TransitionSimulation => "transition-simulation",
            Self::Frame => "frame",
            Self::Environment => "environment",
            Self::NoDivergence => "no-divergence",
            Self::FairnessCorrespondence => "fairness-correspondence",
            Self::Composition => "composition",
            Self::ComponentGuarantee => "component-guarantee",
            Self::Rely => "rely",
            Self::LivenessDependency => "liveness-dependency",
            Self::Fairness => "fairness",
            Self::FixedPoint => "fixed-point",
        }
    }
}

/// What backs a slot (QSpec FR-490 "Derived items", FR-492).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SlotBacking {
    /// A request item the declaration derives.
    Item,
    /// An FR-492 environment record fills the slot.
    Environment,
    /// The planner settles the slot itself.
    PlannerLocal,
    /// The declaration derives no item: FR-492 settles the slot `absent`.
    Unfilled,
}

/// An FR-331 temporal item kind a derived item carries.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ItemKind {
    /// `formula`.
    Formula,
}

/// An FR-331 extent classification.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ExtentClass {
    /// `bounded`.
    Bounded,
    /// `unbounded`.
    Unbounded,
}

/// The FR-331 `correspondence` member of an `operation-contract` item
/// (QSpec FR-490): `{role, operation}`, `operation` absent for
/// `initialization`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Correspondence {
    /// The role.
    pub role: SlotRole,
    /// The operation's declaring object type node and name.
    pub operation: Option<(NodeKey, Identifier)>,
}

/// The slot key of an item-backed slot (QSpec FR-492 "Slot key"): the item's
/// FR-201 obligation identity parts and the request members that widen it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SlotKey {
    /// The obligation's node: the declaration node for an
    /// `operation-contract` item, `P`'s node for the model claim.
    pub node: NodeKey,
    /// The checked package's identity text.
    pub package: String,
    /// The occurrence ordinal.
    pub ordinal: u32,
    /// The subject identity text.
    pub subject: String,
    /// The claim node (the item's anchor).
    pub claim: NodeKey,
    /// The FR-290 declaration family.
    pub family: DeclarationFamily,
    /// The FR-290 kind.
    pub kind: Capability,
    /// The FR-331 `item_kind`, for a temporal item.
    pub item_kind: Option<ItemKind>,
    /// The fairness set, sorted.
    pub fairness: Vec<FairnessConstraint>,
    /// Whether the item carries `stutter_closure`.
    pub stutter_closure: bool,
    /// The `correspondence` member, for an `operation-contract` item.
    pub correspondence: Option<Correspondence>,
    /// The extent classification.
    pub extent: ExtentClass,
}

/// One derived premise slot (QSpec FR-490 "Derived items").
#[derive(Clone, Debug, PartialEq)]
pub struct GuaranteeSlot {
    /// The slot's position in the derived order.
    pub index: usize,
    /// The role.
    pub role: SlotRole,
    /// The operation, for a role that is per operation.
    pub operation: Option<(NodeKey, Identifier)>,
    /// The `F_P` constraint, for a role that is per constraint.
    pub constraint: Option<FairnessConstraint>,
    /// The environment member's name, for a listed `environment` slot.
    pub environment: Option<Identifier>,
    /// What backs the slot.
    pub backing: SlotBacking,
    /// The slot key of an item-backed slot.
    pub key: Option<SlotKey>,
    /// The span of the member that produced the slot.
    pub source: Span,
}

impl CheckedImplementationGuarantee {
    /// The declared name `G`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The node `G` is emitted as (`correspondence`/`implementation_guarantee`).
    pub fn node(&self) -> &NodeKey {
        &self.key
    }

    /// The identity text of the checked package that carries `G`.
    pub fn package(&self) -> &str {
        &self.package
    }

    /// The spans of the declaration's members.
    pub fn spans(&self) -> GuaranteeSpans {
        self.spans
    }

    /// The checked facts of the clause `P`.
    pub fn clause(&self) -> &GuaranteeClause {
        &self.clause
    }

    /// The `correspondence`/`abstraction_relation` node of `A`.
    pub fn relation(&self) -> &NodeKey {
        &self.relation
    }

    /// The `initial` Rust function.
    pub fn initial(&self) -> &RustPath {
        &self.initial
    }

    /// The `progress` members in source order.
    pub fn progress(&self) -> &[ProgressMember] {
        &self.progress
    }

    /// The `environment` members in source order.
    pub fn environment(&self) -> &[EnvironmentMember] {
        &self.environment
    }

    /// `F_P`: the clause's resolved fairness set.
    pub fn fairness(&self) -> &[ClauseConstraint] {
        &self.clause.fairness
    }

    /// FR-363's form of `P`.
    pub fn form(&self) -> PropertyForm {
        self.clause.form
    }

    /// The guarantee item's scoped refusal, when the relation does not bind a
    /// model element the guarantee reads.
    pub fn item_refusal(&self) -> Option<&GuaranteeItemRefusal> {
        self.refusal.as_ref()
    }

    /// Every Rust path the declaration and `relation` name: the relation's
    /// object types, population collections and frame functions, then
    /// `initial` and each `decreases` measure.
    pub fn rust_paths(&self, relation: &CheckedAbstractionRelation) -> Vec<RustPath> {
        let mut paths = Vec::new();
        for (_, binding) in relation.iter() {
            match binding {
                CheckedBinding::Object(object) => paths.push(object.rust_type.clone()),
                CheckedBinding::Population(population) => {
                    paths.push(population.collection.clone());
                }
                CheckedBinding::Frame(frame) => paths.push(frame.binding().function.clone()),
            }
        }
        paths.push(self.initial.clone());
        for member in &self.progress {
            if let ProgressMeasure::Decreases(path) = &member.measure {
                paths.push(path.clone());
            }
        }
        paths
    }

    /// The first operator, in clause order, outside QSpec FR-380's
    /// stuttering-invariant fragment: an interval operator nested under an
    /// unbounded one, spelled `operator[lower,upper]`. The checked formula
    /// has no previous operator, so none is named.
    pub fn fragment_operator(&self) -> Option<String> {
        use super::temporal_clause::{BinaryTemporal, CheckedInterval, UnaryTemporal};
        use qsl_foundation::bound::TemporalUpper;
        let nodes = self.clause.formula.nodes();
        let mut under = vec![false; nodes.len()];
        for (index, node) in nodes.iter().enumerate() {
            let here = under.get(index).copied().unwrap_or(false);
            let (name, interval) = match node {
                FormulaNode::Unary {
                    operator, interval, ..
                } => (
                    match operator {
                        UnaryTemporal::Eventually => "eventually",
                        UnaryTemporal::Always => "always",
                        UnaryTemporal::Once => "once",
                        UnaryTemporal::Historically => "historically",
                    },
                    interval,
                ),
                FormulaNode::Binary {
                    operator, interval, ..
                } => (
                    match operator {
                        BinaryTemporal::Until => "until",
                        BinaryTemporal::Release => "release",
                        BinaryTemporal::Since => "since",
                        BinaryTemporal::Triggered => "triggered",
                    },
                    interval,
                ),
                _ => ("", &None),
            };
            let temporal = !name.is_empty();
            let open = match interval {
                None => true,
                Some(CheckedInterval::Steps(steps)) => steps.upper() == TemporalUpper::Open,
                Some(CheckedInterval::Timed(_)) => false,
            };
            let unbounded = temporal && open;
            if here && temporal && !open {
                let spelled = match interval {
                    Some(CheckedInterval::Steps(steps)) => match steps.upper() {
                        TemporalUpper::Finite(upper) => format!("[{},{}]", steps.lower(), upper),
                        TemporalUpper::Open => format!("[{},*]", steps.lower()),
                    },
                    Some(CheckedInterval::Timed(_)) | None => String::new(),
                };
                return Some(format!("{name}{spelled}"));
            }
            for operand in node.operands() {
                if let Some(slot) = under.get_mut(operand.index()) {
                    *slot = here || unbounded;
                }
            }
        }
        None
    }

    /// The premise slots QSpec FR-490 "Derived items" derives, in its order:
    /// the model claim, the initialization, the transition simulations, the
    /// frames, the environment slots (the listed members, then for a
    /// liveness `P` one unfilled slot per `F_P` constraint no scheduling
    /// member names), for a liveness `P` the no-divergence slots, then the
    /// fairness correspondences.
    pub fn slots(&self, package: &str) -> Vec<GuaranteeSlot> {
        let liveness = self.clause.is_liveness();
        let mut fairness: Vec<FairnessConstraint> = self
            .clause
            .fairness
            .iter()
            .map(|entry| entry.constraint.clone())
            .collect();
        fairness.sort();
        let subject = self.clause.subject.to_string();
        let contract = |correspondence: Correspondence| SlotKey {
            node: self.key.clone(),
            package: package.to_owned(),
            ordinal: 0,
            subject: format!("implementation of {}", self.relation),
            claim: self.key.clone(),
            family: DeclarationFamily::RelationClaim,
            kind: Capability::OperationContract,
            item_kind: None,
            fairness: Vec::new(),
            stutter_closure: false,
            correspondence: Some(correspondence),
            extent: ExtentClass::Unbounded,
        };
        let mut slots: Vec<GuaranteeSlot> = Vec::new();
        let mut push = |role, operation, constraint, environment, backing, key, source| {
            let index = slots.len();
            slots.push(GuaranteeSlot {
                index,
                role,
                operation,
                constraint,
                environment,
                backing,
                key,
                source,
            });
        };
        let operations: Vec<(NodeKey, Identifier)> = self
            .clause
            .operations
            .iter()
            .map(|entry| (entry.context.clone(), entry.operation.clone()))
            .collect();
        push(
            SlotRole::ModelClaim,
            None,
            None,
            None,
            SlotBacking::Item,
            Some(SlotKey {
                node: self.clause.node.clone(),
                package: package.to_owned(),
                ordinal: 0,
                subject,
                claim: self.clause.node.clone(),
                family: DeclarationFamily::TemporalTrace,
                kind: Capability::TemporalSatisfaction,
                item_kind: Some(ItemKind::Formula),
                fairness,
                stutter_closure: liveness,
                correspondence: None,
                extent: ExtentClass::Unbounded,
            }),
            self.spans.clause,
        );
        push(
            SlotRole::Initialization,
            None,
            None,
            None,
            SlotBacking::Item,
            Some(contract(Correspondence {
                role: SlotRole::Initialization,
                operation: None,
            })),
            self.spans.initial,
        );
        for role in [SlotRole::TransitionSimulation, SlotRole::Frame] {
            for operation in &operations {
                push(
                    role,
                    Some(operation.clone()),
                    None,
                    None,
                    SlotBacking::Item,
                    Some(contract(Correspondence {
                        role,
                        operation: Some(operation.clone()),
                    })),
                    self.spans.relation,
                );
            }
        }
        let mut scheduled: Vec<(&NodeKey, &Identifier)> = Vec::new();
        for member in &self.environment {
            if let Supplies::Fairness {
                context, operation, ..
            } = &member.supplies
            {
                scheduled.push((context, operation));
            }
            push(
                SlotRole::Environment,
                None,
                None,
                Some(member.name.clone()),
                SlotBacking::Environment,
                None,
                member.source,
            );
        }
        if liveness {
            for entry in &self.clause.fairness {
                if !scheduled.contains(&(&entry.context, &entry.operation)) {
                    push(
                        SlotRole::Environment,
                        None,
                        Some(entry.constraint.clone()),
                        None,
                        SlotBacking::Unfilled,
                        None,
                        self.spans.clause,
                    );
                }
            }
            for operation in &operations {
                let member = self.progress.iter().find(|member| {
                    (&member.context, &member.operation) == (&operation.0, &operation.1)
                });
                match member {
                    Some(member) => push(
                        SlotRole::NoDivergence,
                        Some(operation.clone()),
                        None,
                        None,
                        SlotBacking::Item,
                        Some(contract(Correspondence {
                            role: SlotRole::NoDivergence,
                            operation: Some(operation.clone()),
                        })),
                        member.source,
                    ),
                    None => push(
                        SlotRole::NoDivergence,
                        Some(operation.clone()),
                        None,
                        None,
                        SlotBacking::Unfilled,
                        None,
                        self.spans.declaration,
                    ),
                }
            }
            for entry in &self.clause.fairness {
                push(
                    SlotRole::FairnessCorrespondence,
                    None,
                    Some(entry.constraint.clone()),
                    None,
                    SlotBacking::PlannerLocal,
                    None,
                    self.spans.clause,
                );
            }
        }
        slots
    }

    /// The term the guarantee node's body is (QSpec FR-490 "Checked-package
    /// node"), its arrays in canonical order.
    pub fn term(&self) -> ImplementationGuaranteeTerm {
        let segments = |path: &RustPath| path.as_str().split("::").map(str::to_owned).collect();
        ImplementationGuaranteeTerm::new(
            self.clause.node.clone(),
            self.relation.clone(),
            segments(&self.initial),
            self.progress
                .iter()
                .map(|member| {
                    ProgressEntry::new(
                        member.context.clone(),
                        member.operation.as_str().to_owned(),
                        match &member.measure {
                            ProgressMeasure::Decreases(path) => {
                                MeasureEntry::Decreases(segments(path))
                            }
                            ProgressMeasure::Bound(count) => MeasureEntry::Bound(*count),
                        },
                    )
                })
                .collect(),
            self.environment
                .iter()
                .map(|member| {
                    EnvironmentEntry::new(
                        member.name.as_str().to_owned(),
                        member.supplier.clone(),
                        match &member.supplies {
                            Supplies::Fairness {
                                kind,
                                granularity,
                                context,
                                operation,
                            } => SuppliesEntry::Fairness {
                                kind: kind_spelling(*kind).to_owned(),
                                granularity: granularity_spelling(*granularity).to_owned(),
                                context: super::node_key::NodeRef::Admitted(context.clone()),
                                operation: operation.as_str().to_owned(),
                            },
                            Supplies::Assumption(node) => SuppliesEntry::Assumption(
                                super::node_key::NodeRef::Admitted(node.clone()),
                            ),
                        },
                    )
                })
                .collect(),
        )
    }
}

/// A fairness kind's spelling.
pub const fn kind_spelling(kind: FairnessKind) -> &'static str {
    match kind {
        FairnessKind::Weak => "weak",
        FairnessKind::Strong => "strong",
    }
}

/// A fairness granularity's spelling.
pub const fn granularity_spelling(granularity: FairnessGranularity) -> &'static str {
    match granularity {
        FairnessGranularity::Whole => "whole",
        FairnessGranularity::Each => "each",
    }
}

/// The member values checks 5 and 6 read: the same on the source path and on
/// the reader's.
pub struct MemberValues<'a> {
    /// The `initial` path and the span of its member.
    pub initial: (&'a str, Span),
    /// Each `progress` member in order: its operation as written, its
    /// measure and its span.
    pub progress: Vec<(&'a str, MeasureValue<'a>, Span)>,
    /// Each `environment` member in order: its name and its span.
    pub environment: Vec<(&'a str, Span)>,
}

/// A measure as checks 5 sees it.
pub enum MeasureValue<'a> {
    /// The content of a `decreases` Rust string.
    Decreases(&'a str),
    /// A `bound` integer as written.
    Bound(&'a str),
}

/// QSpec FR-490 "Static checks" 5 then 6 over the member values.
///
/// # Errors
///
/// The first malformed Rust string or bound, else the first repeated
/// operation or environment name.
pub fn check_member_values(values: &MemberValues<'_>) -> Result<(), GuaranteeRefusal> {
    let malformed = |member: RustMember, spelling: &str, span: Span| {
        path_defect(spelling).map_or(Ok(()), |defect| {
            Err(GuaranteeRefusal::RustPath {
                member,
                spelling: spelling.to_owned(),
                segment: defect.segment,
                span,
            })
        })
    };
    malformed(RustMember::Initial, values.initial.0, values.initial.1)?;
    for (operation, measure, span) in &values.progress {
        match measure {
            MeasureValue::Decreases(path) => {
                malformed(RustMember::Measure((*operation).to_owned()), path, *span)?;
            }
            MeasureValue::Bound(text) => {
                if text.parse::<u64>().map_or(true, |count| count == 0) {
                    return Err(GuaranteeRefusal::InvalidBound {
                        operation: (*operation).to_owned(),
                        text: (*text).to_owned(),
                        span: *span,
                    });
                }
            }
        }
    }
    let mut seen: BTreeMap<&str, Span> = BTreeMap::new();
    for (operation, _, span) in &values.progress {
        if let Some(first) = seen.insert(operation, *span) {
            return Err(GuaranteeRefusal::ConflictingProgress {
                operation: (*operation).to_owned(),
                first,
                second: *span,
            });
        }
    }
    let mut seen: BTreeMap<&str, Span> = BTreeMap::new();
    for (name, span) in &values.environment {
        if let Some(first) = seen.insert(name, *span) {
            return Err(GuaranteeRefusal::ConflictingEnvironment {
                name: (*name).to_owned(),
                first,
                second: *span,
            });
        }
    }
    Ok(())
}

/// The `Origin::TypeDeclaration` name a `guarantee` declaration is located
/// under (QSpec FR-451's pattern, as the abstraction relation's).
pub(crate) fn declaration_origin(name: &str) -> String {
    format!("guarantee {name}")
}

impl CheckedGraph {
    /// QSpec FR-490: the unit's checked `guarantee` declarations, in
    /// declaration order.
    pub fn implementation_guarantees(&self) -> &[CheckedImplementationGuarantee] {
        &self.guarantees
    }

    /// Where a refusal in the `index`th `guarantee` declaration is located.
    pub(super) fn guarantee_location(&self, index: usize) -> Location {
        Location::root(Origin::TypeDeclaration {
            name: self
                .guarantee_forms
                .get(index)
                .map(|declaration| declaration_origin(&declaration.name.name))
                .unwrap_or_default(),
        })
    }

    /// QSpec FR-490 "Static checks" 2 to 6 over every `guarantee`
    /// declaration. A declaration's first refusal is reported with its
    /// index; a declaration that checks is produced.
    ///
    /// # Errors
    ///
    /// Each declaration's first refusal, in declaration order.
    pub(super) fn check_declared_guarantees(
        &self,
    ) -> Result<Vec<CheckedImplementationGuarantee>, Vec<(usize, GuaranteeRefusal)>> {
        let mut checked = Vec::new();
        let mut refusals = Vec::new();
        for (index, form) in self.guarantee_forms.iter().enumerate() {
            match self.check_guarantee(form) {
                Ok(guarantee) => checked.push(guarantee),
                Err(refusal) => refusals.push((index, refusal)),
            }
        }
        if refusals.is_empty() {
            Ok(checked)
        } else {
            Err(refusals)
        }
    }

    fn check_guarantee(
        &self,
        form: &GuaranteeDeclarationForm,
    ) -> Result<CheckedImplementationGuarantee, GuaranteeRefusal> {
        let internal = |invariant| {
            GuaranteeRefusal::Internal(InternalFault::new("check.guarantee", invariant))
        };
        // Check 2.
        let clause_node = self
            .temporal_clauses
            .iter()
            .find(|node| node.clause().name() == form.clause.name)
            .ok_or_else(|| GuaranteeRefusal::MissingName {
                what: GuaranteeName::Clause,
                name: form.clause.name.clone(),
                span: form.clause.span,
            })?;
        let relation_form = self
            .abstractions
            .iter()
            .find(|declaration| declaration.name.name == form.relation.name)
            .ok_or_else(|| GuaranteeRefusal::MissingName {
                what: GuaranteeName::Relation,
                name: form.relation.name.clone(),
                span: form.relation.span,
            })?;
        // Check 3.
        let clause = &form.clause;
        if clause_node.clause().profile() != TemporalProfile::InfiniteTrace {
            return Err(GuaranteeRefusal::ClauseNotInfiniteTrace {
                clause: clause.name.clone(),
                span: clause.span,
            });
        }
        let alias = self
            .temporal_subject_aliases
            .get(&clause.name)
            .cloned()
            .ok_or_else(|| GuaranteeRefusal::ClauseNotModelSubject {
                clause: clause.name.clone(),
                span: clause.span,
            })?;
        let relation_aliases: BTreeSet<String> = relation_form
            .bindings
            .iter()
            .filter_map(|binding| {
                let name = match binding {
                    qsl_forms::AbstractionBindingForm::Object(form) => &form.model_type,
                    qsl_forms::AbstractionBindingForm::Population(form) => &form.population,
                    qsl_forms::AbstractionBindingForm::Frame(form) => &form.operation,
                };
                name.name.split("::").next().map(str::to_owned)
            })
            .collect();
        if !relation_aliases.contains(&alias) {
            return Err(GuaranteeRefusal::ModelAliasMismatch {
                clause: clause.name.clone(),
                clause_alias: alias,
                relation_aliases: relation_aliases.into_iter().collect(),
                span: clause.span,
            });
        }
        let facts = self.guarantee_clause(clause_node, alias).ok_or_else(|| {
            GuaranteeRefusal::ClauseNotModelSubject {
                clause: clause.name.clone(),
                span: clause.span,
            }
        })?;
        // Check 4.
        let mut progress = Vec::new();
        let mut environment = Vec::new();
        let mut written_progress: Vec<(&str, &GuaranteeMeasureForm, Span)> = Vec::new();
        let mut written_environment: Vec<(&str, Span)> = Vec::new();
        for member in &form.members {
            match member {
                GuaranteeMemberForm::Progress(member) => {
                    let (context, operation) =
                        self.resolve_subject_operation(&member.operation, &facts)?;
                    written_progress.push((&member.operation.name, &member.measure, member.span));
                    progress.push((context, operation, member));
                }
                GuaranteeMemberForm::Environment(member) => {
                    let name = Identifier::new(&member.name.name).map_err(|_| {
                        GuaranteeRefusal::MissingName {
                            what: GuaranteeName::Operation,
                            name: member.name.name.clone(),
                            span: member.name.span,
                        }
                    })?;
                    let supplies = match &member.supplies {
                        GuaranteeSuppliesForm::Fairness(constraint) => {
                            let (context, operation) =
                                self.resolve_subject_operation(&constraint.operation, &facts)?;
                            Supplies::Fairness {
                                kind: super::temporal_clause::checked_kind(constraint),
                                granularity: super::temporal_clause::checked_granularity(
                                    constraint,
                                ),
                                context,
                                operation,
                            }
                        }
                        GuaranteeSuppliesForm::Assumes(predicate) => {
                            Supplies::Assumption(self.resolve_predicate(predicate)?)
                        }
                    };
                    written_environment.push((&member.name.name, member.span));
                    environment.push(EnvironmentMember {
                        name,
                        supplier: member.supplier.text.clone(),
                        supplies,
                        source: member.span,
                    });
                }
            }
        }
        // Checks 5 and 6.
        check_member_values(&MemberValues {
            initial: (&form.initial.function.text, form.initial.span),
            progress: written_progress
                .iter()
                .map(|(operation, measure, span)| {
                    (
                        *operation,
                        match measure {
                            GuaranteeMeasureForm::Decreases(path) => {
                                MeasureValue::Decreases(&path.text)
                            }
                            GuaranteeMeasureForm::Bound { text, .. } => MeasureValue::Bound(text),
                        },
                        *span,
                    )
                })
                .collect(),
            environment: written_environment,
        })?;
        let progress = progress
            .into_iter()
            .map(|(context, operation, member)| {
                Ok(ProgressMember {
                    context,
                    operation,
                    measure: match &member.measure {
                        GuaranteeMeasureForm::Decreases(path) => {
                            ProgressMeasure::Decreases(RustPath::new(&path.text))
                        }
                        GuaranteeMeasureForm::Bound { text, .. } => ProgressMeasure::Bound(
                            text.parse()
                                .map_err(|_| internal("bound-is-a-positive-count"))?,
                        ),
                    },
                    source: member.span,
                })
            })
            .collect::<Result<Vec<_>, GuaranteeRefusal>>()?;
        let relation = self
            .abstraction
            .as_ref()
            .ok_or_else(|| internal("guarantee-relation-is-checked"))?;
        let relation_node = self
            .abstraction_nodes(relation)
            .map_err(|reason| GuaranteeRefusal::RelationNode {
                span: form.relation.span,
                reason: reason.to_string(),
            })?
            .relation()
            .key();
        let key = self
            .guarantee_node_key(&form.name.name)
            .ok_or_else(|| internal("guarantee-node-is-keyed"))?;
        let refusal = self.unbound_elements(clause_node, &facts, relation);
        Ok(CheckedImplementationGuarantee {
            name: form.name.name.clone(),
            key,
            package: self.package_name().to_owned(),
            spans: GuaranteeSpans {
                declaration: form.span,
                clause: form.clause.span,
                relation: form.relation.span,
                initial: form.initial.span,
            },
            clause: facts,
            relation: relation_node,
            initial: RustPath::new(&form.initial.function.text),
            progress,
            environment,
            refusal,
        })
    }

    /// The structural key of the node of the guarantee named `name`.
    pub(super) fn guarantee_node_key(&self, name: &str) -> Option<NodeKey> {
        super::derived_node_key(
            self.package_name(),
            super::node_key::NodeTag::Correspondence,
            "implementation_guarantee",
            vec![quire_exact::PathSegment::Text(name.to_owned())],
            0,
        )
        .ok()
    }

    /// The checked facts of the clause `node`, whose subject is in model
    /// alias `alias`: `None` for a clause that is not over a model subject.
    pub fn guarantee_clause(
        &self,
        node: &TemporalClauseNode,
        alias: String,
    ) -> Option<GuaranteeClause> {
        let checked = node.clause();
        let form = checked.property_form()?;
        let ValueType::Reference(subject) = &checked.over().1 else {
            return None;
        };
        let subject_node = self.object_node_of(subject)?;
        let mut fairness = Vec::new();
        for constraint in checked.fairness() {
            let (_, context) = self.context_of(&constraint.operation().declaring_type())?;
            fairness.push(ClauseConstraint {
                constraint: constraint.clone(),
                context,
                operation: Identifier::new(constraint.operation().name()).ok()?,
            });
        }
        let mut operations = Vec::new();
        for (declaring, operation) in self
            .scope
            .operations()
            .effective(self.scope.types(), subject.clone())
        {
            let (_, context) = self.context_of(&declaring)?;
            operations.push(SubjectOperation {
                context,
                operation: Identifier::new(operation.name()).ok()?,
            });
        }
        Some(GuaranteeClause {
            node: node.identity(),
            name: checked.name().to_owned(),
            form,
            fairness,
            alias,
            subject: subject_node,
            operations,
            formula: checked.formula().clone(),
        })
    }

    /// The declaration key and `model`/`object_type` node of the object type
    /// `id`.
    pub(super) fn context_of(&self, id: &EffectiveId) -> Option<(DeclarationKey, NodeKey)> {
        let (key, _) = self
            .object_types
            .iter()
            .find(|(_, effective)| *effective == id)?;
        Some((key.clone(), self.object_nodes.get(key)?.clone()))
    }

    fn object_node_of(&self, id: &EffectiveId) -> Option<NodeKey> {
        self.context_of(id).map(|(_, node)| node)
    }

    /// Check 4 for an operation name: it resolves to an operation of the
    /// model subject, as its declaring object type node and name.
    fn resolve_subject_operation(
        &self,
        name: &NameForm,
        facts: &GuaranteeClause,
    ) -> Result<(NodeKey, Identifier), GuaranteeRefusal> {
        let missing = || GuaranteeRefusal::MissingName {
            what: GuaranteeName::Operation,
            name: name.name.clone(),
            span: name.span,
        };
        let parts = name_parts(name, 3).ok_or_else(missing)?;
        let declaring = self
            .model_names
            .get(parts[0].as_str())
            .and_then(|model| model.export(&parts[1]))
            .ok_or_else(missing)?;
        let owner = self.object_types.get(declaring).ok_or_else(missing)?;
        let crate::model::operation::OperationLookup::Declared {
            declaring: resolved,
            operation,
        } = self
            .scope
            .operations()
            .resolve(self.scope.types(), owner.clone(), parts[2].as_str())
        else {
            return Err(missing());
        };
        let (_, context) = self.context_of(&resolved).ok_or_else(missing)?;
        let operation = Identifier::new(operation.name()).map_err(|_| missing())?;
        let wanted = SubjectOperation {
            context: context.clone(),
            operation: operation.clone(),
        };
        if facts.operations.contains(&wanted) {
            Ok((context, operation))
        } else {
            Err(missing())
        }
    }

    /// Check 4 for an `assumes` name: the node of the `predicate` it names.
    pub(super) fn resolve_predicate(&self, name: &NameForm) -> Result<NodeKey, GuaranteeRefusal> {
        if self.predicate_names.contains(&name.name) {
            return self.function_identity(&name.name).ok_or_else(|| {
                GuaranteeRefusal::MissingName {
                    what: GuaranteeName::Assumption,
                    name: name.name.clone(),
                    span: name.span,
                }
            });
        }
        if self.declares_function(&name.name) || self.type_spans.contains_key(&name.name) {
            return Err(GuaranteeRefusal::AssumptionNotPredicate {
                name: name.name.clone(),
                span: name.span,
            });
        }
        Err(GuaranteeRefusal::MissingName {
            what: GuaranteeName::Assumption,
            name: name.name.clone(),
            span: name.span,
        })
    }

    /// Whether `node` is the node of a `predicate` declaration of the unit.
    pub fn is_predicate_node(&self, node: &NodeKey) -> bool {
        self.predicate_names
            .iter()
            .filter_map(|name| self.function_identity(name))
            .any(|identity| identity == *node)
    }

    /// Whether `node` is the node of an object type some operation of the
    /// model subject of `facts` is declared by.
    pub fn is_subject_operation(
        &self,
        facts: &GuaranteeClause,
        context: &NodeKey,
        operation: &Identifier,
    ) -> bool {
        facts.operations.contains(&SubjectOperation {
            context: context.clone(),
            operation: operation.clone(),
        })
    }

    /// The model elements the guarantee reads that `relation` does not bind
    /// (QSpec FR-490-AC-6): the object types and populations `P` reads, and
    /// every operation of the model subject.
    fn unbound_elements(
        &self,
        clause: &TemporalClauseNode,
        facts: &GuaranteeClause,
        relation: &CheckedAbstractionRelation,
    ) -> Option<GuaranteeItemRefusal> {
        let occurrence = OccurrenceKey::new(
            super::claims::wire(clause.identity()),
            clause.claim().clone(),
        );
        let mut elements: BTreeSet<AbstractionKey> = self
            .item_elements(&occurrence)
            .and_then(Result::ok)
            .unwrap_or_default();
        for operation in &facts.operations {
            let declaring = self
                .object_types
                .keys()
                .find(|key| self.object_nodes.get(*key) == Some(&operation.context))?;
            elements.insert(AbstractionKey::Frame(OperationKey {
                declaring: declaring.clone(),
                operation: operation.operation.clone(),
            }));
        }
        let unbound: Vec<AbstractionKey> = elements
            .into_iter()
            .filter(|key| relation.get(key).is_none())
            .collect();
        unbound
            .first()
            .cloned()
            .map(|element| GuaranteeItemRefusal { element, unbound })
    }

    /// The name of a `TypeFormHead::Name` type, when `head` is one.
    pub(super) fn subject_alias_of(head: &TypeFormHead) -> Option<String> {
        match head {
            TypeFormHead::Name(name) => name.name.split("::").next().map(str::to_owned),
            _ => None,
        }
    }
}

/// One `progress` entry of a guarantee node as a reader found it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadProgress {
    /// The operation's declaring object type node.
    pub context: NodeKey,
    /// The operation name.
    pub operation: String,
    /// The measure.
    pub measure: MeasureEntry,
    /// The span of the member, from the node's source map.
    pub span: Span,
}

/// One `environment` entry of a guarantee node as a reader found it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadEnvironment {
    /// The premise name.
    pub name: String,
    /// The supplier text.
    pub supplier: String,
    /// What the entry supplies; every node reference is admitted.
    pub supplies: SuppliesEntry,
    /// The span of the member, from the node's source map.
    pub span: Span,
}

/// A guarantee node's body and source map as a reader found them, after the
/// reader's shape, identity and dependency checks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadGuarantee {
    /// The declared name, from the node's id.
    pub name: String,
    /// The `clause` target.
    pub clause: NodeKey,
    /// The `relation` target.
    pub relation: NodeKey,
    /// The `initial` Rust path's segments.
    pub initial: Vec<String>,
    /// The `progress` entries in the node's order.
    pub progress: Vec<ReadProgress>,
    /// The `environment` entries in the node's order.
    pub environment: Vec<ReadEnvironment>,
    /// The spans of the declaration's members.
    pub spans: GuaranteeSpans,
}

impl CheckedGraph {
    /// QSpec FR-490 "Checked-package node", the reader's steps after the
    /// shape, identity, term and canonical-order checks: each target is a
    /// node of the package of the kind its member needs, then static checks 4
    /// to 6 on the read values. `relation` is the package's checked
    /// abstraction relation.
    ///
    /// # Errors
    ///
    /// The first defect, in the reader's order.
    #[ix_trace_rs::string_edge]
    pub fn read_implementation_guarantee(
        &self,
        relation: Option<&CheckedAbstractionRelation>,
        read: &ReadGuarantee,
    ) -> Result<CheckedImplementationGuarantee, GuaranteeRefusal> {
        let internal = |invariant| {
            GuaranteeRefusal::Internal(InternalFault::new("check.guarantee-read", invariant))
        };
        let missing = |what, name: &str, span| GuaranteeRefusal::MissingName {
            what,
            name: name.to_owned(),
            span,
        };
        let key = self
            .guarantee_node_key(&read.name)
            .ok_or_else(|| internal("guarantee-node-is-keyed"))?;
        // The targets exist, and are of the kind their member needs.
        let clause_name = |node: &TemporalClauseNode| node.clause().name().to_owned();
        let Some(clause_node) = self
            .temporal_clauses
            .iter()
            .find(|node| node.identity() == read.clause)
        else {
            return Err(
                if self.semantic_graph().node(read.clause.clone()).is_some() {
                    GuaranteeRefusal::ClauseNotInfiniteTrace {
                        clause: read.clause.to_string(),
                        span: read.spans.clause,
                    }
                } else {
                    missing(
                        GuaranteeName::Clause,
                        &read.clause.to_string(),
                        read.spans.clause,
                    )
                },
            );
        };
        let name = clause_name(clause_node);
        let relation = relation.ok_or_else(|| {
            missing(
                GuaranteeName::Relation,
                &read.relation.to_string(),
                read.spans.relation,
            )
        })?;
        let relation_key = self
            .abstraction_nodes(relation)
            .map_err(|_| {
                missing(
                    GuaranteeName::Relation,
                    &read.relation.to_string(),
                    read.spans.relation,
                )
            })?
            .relation()
            .key();
        if relation_key != read.relation {
            return Err(
                if self.semantic_graph().node(read.relation.clone()).is_some() {
                    GuaranteeRefusal::NotAbstractionRelation {
                        span: read.spans.relation,
                    }
                } else {
                    missing(
                        GuaranteeName::Relation,
                        &read.relation.to_string(),
                        read.spans.relation,
                    )
                },
            );
        }
        if clause_node.clause().profile() != TemporalProfile::InfiniteTrace {
            return Err(GuaranteeRefusal::ClauseNotInfiniteTrace {
                clause: name,
                span: read.spans.clause,
            });
        }
        let alias = self
            .temporal_subject_aliases
            .get(&name)
            .cloned()
            .ok_or_else(|| GuaranteeRefusal::ClauseNotModelSubject {
                clause: name.clone(),
                span: read.spans.clause,
            })?;
        let facts = self.guarantee_clause(clause_node, alias).ok_or_else(|| {
            GuaranteeRefusal::ClauseNotModelSubject {
                clause: name.clone(),
                span: read.spans.clause,
            }
        })?;
        let in_package = |node: &NodeKey| self.object_nodes.values().any(|known| known == node);
        let mut progress = Vec::new();
        for entry in &read.progress {
            if !in_package(&entry.context) {
                return Err(missing(
                    GuaranteeName::Operation,
                    &entry.operation,
                    entry.span,
                ));
            }
            progress.push(ProgressMember {
                context: entry.context.clone(),
                operation: Identifier::new(&entry.operation)
                    .map_err(|_| missing(GuaranteeName::Operation, &entry.operation, entry.span))?,
                measure: match &entry.measure {
                    MeasureEntry::Decreases(segments) => {
                        ProgressMeasure::Decreases(RustPath::new(segments.join("::")))
                    }
                    MeasureEntry::Bound(count) => ProgressMeasure::Bound(*count),
                },
                source: entry.span,
            });
        }
        let mut environment = Vec::new();
        for entry in &read.environment {
            let supplies = match &entry.supplies {
                SuppliesEntry::Fairness {
                    kind,
                    granularity,
                    context,
                    operation,
                } => {
                    let context = context
                        .admitted()
                        .cloned()
                        .ok_or_else(|| internal("environment-context-is-admitted"))?;
                    if !in_package(&context) {
                        return Err(missing(GuaranteeName::Operation, operation, entry.span));
                    }
                    Supplies::Fairness {
                        kind: match kind.as_str() {
                            "weak" => FairnessKind::Weak,
                            "strong" => FairnessKind::Strong,
                            _ => return Err(internal("fairness-kind-is-spelled")),
                        },
                        granularity: match granularity.as_str() {
                            "whole" => FairnessGranularity::Whole,
                            "each" => FairnessGranularity::Each,
                            _ => return Err(internal("fairness-granularity-is-spelled")),
                        },
                        context,
                        operation: Identifier::new(operation).map_err(|_| {
                            missing(GuaranteeName::Operation, operation, entry.span)
                        })?,
                    }
                }
                SuppliesEntry::Assumption(node) => {
                    let node = node
                        .admitted()
                        .cloned()
                        .ok_or_else(|| internal("assumption-is-admitted"))?;
                    if self.is_predicate_node(&node) {
                        Supplies::Assumption(node)
                    } else if self.semantic_graph().node(node.clone()).is_some() {
                        return Err(GuaranteeRefusal::AssumptionNotPredicate {
                            name: node.to_string(),
                            span: entry.span,
                        });
                    } else {
                        return Err(missing(
                            GuaranteeName::Assumption,
                            &node.to_string(),
                            entry.span,
                        ));
                    }
                }
            };
            environment.push(EnvironmentMember {
                name: Identifier::new(&entry.name)
                    .map_err(|_| internal("environment-name-is-an-identifier"))?,
                supplier: entry.supplier.clone(),
                supplies,
                source: entry.span,
            });
        }
        // Check 4: every scheduling and progress operation is one of the
        // model subject's.
        let subject_operation = |context: &NodeKey, operation: &Identifier| {
            self.is_subject_operation(&facts, context, operation)
        };
        for member in &progress {
            if !subject_operation(&member.context, &member.operation) {
                return Err(missing(
                    GuaranteeName::Operation,
                    member.operation.as_str(),
                    member.source,
                ));
            }
        }
        for member in &environment {
            if let Supplies::Fairness {
                context, operation, ..
            } = &member.supplies
            {
                if !subject_operation(context, operation) {
                    return Err(missing(
                        GuaranteeName::Operation,
                        operation.as_str(),
                        member.source,
                    ));
                }
            }
        }
        // Checks 5 and 6.
        let split = |segments: &[String], span: Span| -> Result<String, GuaranteeRefusal> {
            match segments.iter().find(|segment| segment.contains("::")) {
                Some(segment) => Err(GuaranteeRefusal::RustPath {
                    member: RustMember::Initial,
                    spelling: segments.join("::"),
                    segment: segment.clone(),
                    span,
                }),
                None => Ok(segments.join("::")),
            }
        };
        let initial = split(&read.initial, read.spans.initial)?;
        let decreases: Vec<Option<String>> = read
            .progress
            .iter()
            .map(|entry| match &entry.measure {
                MeasureEntry::Decreases(segments) => split(segments, entry.span).map(Some),
                MeasureEntry::Bound(_) => Ok(None),
            })
            .collect::<Result<_, _>>()?;
        let bounds: Vec<Option<String>> = read
            .progress
            .iter()
            .map(|entry| match &entry.measure {
                MeasureEntry::Bound(count) => Some(count.to_string()),
                MeasureEntry::Decreases(_) => None,
            })
            .collect();
        check_member_values(&MemberValues {
            initial: (&initial, read.spans.initial),
            progress: read
                .progress
                .iter()
                .zip(decreases.iter().zip(&bounds))
                .map(|(entry, (path, bound))| {
                    (
                        entry.operation.as_str(),
                        match (path, bound) {
                            (Some(path), _) => MeasureValue::Decreases(path),
                            (None, Some(bound)) => MeasureValue::Bound(bound),
                            (None, None) => MeasureValue::Bound("0"),
                        },
                        entry.span,
                    )
                })
                .collect(),
            environment: read
                .environment
                .iter()
                .map(|entry| (entry.name.as_str(), entry.span))
                .collect(),
        })?;
        // Source order: the member spans give it back.
        progress.sort_by_key(|member| (member.source.start, member.source.end));
        environment.sort_by_key(|member| (member.source.start, member.source.end));
        let refusal = self.unbound_elements(clause_node, &facts, relation);
        Ok(CheckedImplementationGuarantee {
            name: read.name.clone(),
            key,
            package: self.package_name().to_owned(),
            spans: read.spans,
            clause: facts,
            relation: relation_key,
            initial: RustPath::new(initial),
            progress,
            environment,
            refusal,
        })
    }
}
