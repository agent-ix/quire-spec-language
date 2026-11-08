// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL FR-530, QSpec FR-491: the S3 check of an authored component
//! composition declaration, a `Relation` family declaration beside the
//! implementation guarantee (ADR-012 §3, ADR-017 AR-1).
//!
//! [`CheckedGraph::check_declared_compositions`] applies QSpec FR-491 "Static
//! checks" 2 to 5 in order and produces one [`CheckedComposition`] per
//! declaration, or the first refusal of each declaration that is refused. The
//! composition's constructor is private to this module, so the checker and the
//! node reader are its only writers.
//!
//! A component is the subject `M` of an imported checked package: its clauses
//! are read from the import's checked graph ([`AdmittedImport::graph`]), the
//! same graph a refinement's abstract side is read from. The dependency graph
//! and its cycles are derived here, and a cycle is not a declaration refusal:
//! it is the composition item's own refusal
//! ([`CheckedComposition::item_refusal`]) unless the unit selects a profile
//! that declares FR-176's lattice, when it derives a `fixed-point` slot.
//!
//! [`AdmittedImport::graph`]: super::AdmittedImport

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use qsl_forms::{
    ComponentClauseForm, CompositionDeclarationForm, CompositionMemberForm, CompositionRuleForm,
    GuaranteeSuppliesForm, NameForm,
};
use qsl_foundation::{Code, Span};
use quire_exact::{Identifier, NodeKey};
use quire_semantic_value::catalog::InternalFault;
use quire_semantic_value::fairness::{
    FairnessConstraint, FairnessGranularity, FairnessKind, PropertyForm,
};
use quire_semantic_value::location::{Location, Origin};

use super::abstraction::name_parts;
use super::capability::{Capability, DeclarationFamily};
use super::guarantee::{
    granularity_spelling, kind_spelling, ClauseConstraint, EnvironmentMember, ExtentClass,
    ItemKind, ReadEnvironment, SlotBacking, SlotKey, SlotRole, SubjectOperation, Supplies,
};
use super::node_key::{
    ClauseEntry, ComponentEntry, CompositionRule, CompositionTerm, EnvironmentEntry, NodeRef,
    PremiseEntry, SuppliesEntry, SupplyEntry,
};
use super::CheckedGraph;

/// What a refused name was written for.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CompositionName {
    /// The `from I` import alias, or a package a node names.
    Import,
    /// The `using M` model alias, or a subject a node names.
    Model,
    /// The component `C` of a `C::G`.
    Component,
    /// The clause `G` of a `C::G`.
    Clause,
    /// The operation a `supplies` or scheduling `environment` member names.
    Operation,
    /// The predicate an `assumes` member names.
    Assumption,
}

/// Why a member is malformed (QSpec FR-491 "Static checks" 3 and 4).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MalformedMember {
    /// Check 3: the `C::G` of a `rely` or `depends` is not a declared
    /// `guarantee`.
    UndeclaredGuarantee,
    /// Check 3: the `C` of a `rely` or `depends` equals its `D`.
    SelfComponent,
    /// Check 4: a `rely` whose `H` FR-363 classifies as `liveness`.
    RelyOnLiveness,
    /// Check 4: a `depends` whose `G` or `H` FR-363 does not classify as
    /// `liveness`.
    DependsNotLiveness,
    /// An `assumes` name that resolves to a declaration other than a
    /// `predicate`.
    AssumptionNotPredicate,
}

/// Why S3 refused an authored `compose` declaration (QSpec FR-491 "Static
/// checks" 2 to 5). Every variant names its member by span.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum CompositionRefusal {
    /// Check 2 or 5: a name that resolves to nothing the member needs.
    MissingName {
        /// What the name was written for.
        what: CompositionName,
        /// The name as written.
        name: String,
        /// The span of the name.
        span: Span,
    },
    /// Check 2: two `component` members of one name.
    ConflictingComponent {
        /// The shared name.
        name: String,
        /// The span of the first member.
        first: Span,
        /// The span of the second member.
        second: Span,
    },
    /// Check 2: two `component` members over one subject identity.
    ConflictingSubject {
        /// The shared subject identity.
        subject: String,
        /// The first component's name.
        first_component: String,
        /// The second component's name.
        second_component: String,
        /// The span of the first member.
        first: Span,
        /// The span of the second member.
        second: Span,
    },
    /// Two members that state one premise, guarantee, supply or environment
    /// name.
    ConflictingMember {
        /// The member as written.
        member: String,
        /// The span of the first member.
        first: Span,
        /// The span of the second member.
        second: Span,
    },
    /// Check 3 or 4: a member the declaration cannot hold.
    Malformed {
        /// Why.
        reason: MalformedMember,
        /// The member as written.
        member: String,
        /// The span of the member.
        span: Span,
    },
    /// A check invariant is broken.
    Internal(InternalFault),
}

impl CompositionRefusal {
    /// The catalog code.
    pub fn code(&self) -> Code {
        match self {
            Self::MissingName { .. } => Code::MissingDeclaration,
            Self::ConflictingComponent { .. }
            | Self::ConflictingSubject { .. }
            | Self::ConflictingMember { .. }
            | Self::Malformed { .. } => Code::InvalidModelBinding,
            Self::Internal(_) => Code::RuntimeInvariant,
        }
    }

    /// The catalog cause.
    pub fn cause(&self) -> &'static str {
        match self {
            Self::MissingName { .. } => "missing-name",
            Self::ConflictingComponent { .. }
            | Self::ConflictingSubject { .. }
            | Self::ConflictingMember { .. } => "conflicting-binding",
            Self::Malformed { .. } => "malformed-declaration",
            Self::Internal(_) => "established-invariant-broken",
        }
    }

    /// The span of the member the refusal names, when it has one.
    pub fn span(&self) -> Option<Span> {
        match self {
            Self::MissingName { span, .. } | Self::Malformed { span, .. } => Some(*span),
            Self::ConflictingComponent { second, .. }
            | Self::ConflictingSubject { second, .. }
            | Self::ConflictingMember { second, .. } => Some(*second),
            Self::Internal(_) => None,
        }
    }
}

/// A component's subject identity: the identity of the imported package and
/// of the model `M` selects within it (QSpec FR-491 "Declaration form").
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ComponentSubject {
    /// The imported package's identity.
    pub package: String,
    /// The identity of the domain package the model alias selects.
    pub model: String,
}

impl fmt::Display for ComponentSubject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} using {}", self.package, self.model)
    }
}

/// One checked `component C from I using M;` member.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedComponent {
    /// The component name `C`.
    pub name: String,
    /// The component's subject identity.
    pub subject: ComponentSubject,
    /// The subject's node in the imported package: the first object type the
    /// model exports.
    pub subject_node: NodeKey,
    /// The span of the member.
    pub source: Span,
}

/// A component's temporal clause, `C::G`.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ClauseRef {
    /// The component name `C`.
    pub component: String,
    /// The clause name `G`.
    pub name: String,
    /// The clause's `temporal`/`temporal_clause` node, in the component's
    /// package.
    pub node: NodeKey,
}

impl fmt::Display for ClauseRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}::{}", self.component, self.name)
    }
}

/// The checked facts of a clause a composition names (QSpec FR-363, FR-362).
#[derive(Clone, Debug, PartialEq)]
pub struct CompositionClauseFacts {
    /// The clause.
    pub reference: ClauseRef,
    /// FR-363's property form of the clause.
    pub form: PropertyForm,
    /// The clause's resolved fairness set, in the clause's order.
    pub fairness: Vec<ClauseConstraint>,
}

impl CompositionClauseFacts {
    /// Whether FR-363 classifies the clause as `liveness`.
    pub fn is_liveness(&self) -> bool {
        self.form == PropertyForm::Liveness
    }
}

/// One `guarantee C::G;` member.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionGuarantee {
    /// The guarantee.
    pub reference: ClauseRef,
    /// The span of the member.
    pub source: Span,
}

/// One `rely C::G on D::H;` or `depends C::G on D::H;` member.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionPremise {
    /// The guarantee that carries the premise.
    pub guarantee: ClauseRef,
    /// The peer guarantee the premise is on.
    pub on: ClauseRef,
    /// The span of the member.
    pub source: Span,
}

/// One `supplies fair ...;` member.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionSupply {
    /// The kind, `weak` when unwritten.
    pub kind: FairnessKind,
    /// The granularity, `whole` when unwritten.
    pub granularity: FairnessGranularity,
    /// The operation's declaring object type node, in a component's package.
    pub context: NodeKey,
    /// The operation.
    pub operation: Identifier,
    /// The span of the member.
    pub source: Span,
}

/// The spans of the members a declaration is made of.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompositionSpans {
    /// The whole declaration.
    pub declaration: Span,
    /// The rule spelling.
    pub rule: Span,
}

/// A cycle of the dependency graph: its guarantees in edge order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionCycle {
    /// The cycle's guarantees, each the tail of one of the cycle's edges, in
    /// the order the `rely` and `depends` members that make the edges are
    /// written.
    pub guarantees: Vec<ClauseRef>,
    /// The span of the first member that makes one of the cycle's edges.
    pub source: Span,
}

/// The composition item's scoped refusal: the dependency graph has a cycle
/// and no profile the unit selects declares FR-176's lattice (QSpec FR-491
/// "Dependency graph and cycles"), `invalid_model_binding`/
/// `malformed-declaration`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionItemRefusal {
    /// Every cycle, with its guarantees in edge order.
    pub cycles: Vec<Vec<ClauseRef>>,
}

impl CompositionItemRefusal {
    /// `invalid_model_binding`.
    pub fn code(&self) -> Code {
        Code::InvalidModelBinding
    }

    /// `malformed-declaration`.
    pub fn cause(&self) -> &'static str {
        "malformed-declaration"
    }
}

/// A checked component composition declaration (QSpec FR-489, FR-491).
#[derive(Clone, Debug, PartialEq)]
pub struct CheckedComposition {
    name: String,
    key: NodeKey,
    package: String,
    rule: CompositionRule,
    spans: CompositionSpans,
    components: Vec<CheckedComponent>,
    guarantees: Vec<CompositionGuarantee>,
    relies: Vec<CompositionPremise>,
    liveness_dependencies: Vec<CompositionPremise>,
    supplies: Vec<CompositionSupply>,
    environment: Vec<EnvironmentMember>,
    clauses: Vec<CompositionClauseFacts>,
    cycles: Vec<CompositionCycle>,
    lattice: Option<String>,
    refusal: Option<CompositionItemRefusal>,
}

/// The slot key of a `component-guarantee` slot (QSpec FR-491 "Derived item
/// and slots"): the item of `G` over `C`'s subject, with the FR-176
/// assumption set it is checked under.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentGuaranteeKey {
    /// The item's FR-492 slot key without the assumption set.
    pub key: SlotKey,
    /// The peer guarantees `G` may assume: the `H` of each `rely` and
    /// `depends` member `G` carries, sorted by node.
    pub assumptions: Vec<NodeKey>,
}

/// The slot key of an item-backed composition slot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompositionSlotKey {
    /// A `component-guarantee` slot.
    ComponentGuarantee(ComponentGuaranteeKey),
    /// A `fixed-point` slot: one FR-331 `composition` item anchored at the
    /// declaration's node, whose premises are the cycle's component-guarantee
    /// keys and whose options name the declaring profile.
    FixedPoint {
        /// The declaration's node.
        node: NodeKey,
        /// The checked package's identity text.
        package: String,
        /// The cycle's component-guarantee slot keys, in the cycle's edge
        /// order.
        premises: Vec<ComponentGuaranteeKey>,
        /// The profile that declares FR-176's lattice.
        profile: String,
    },
}

/// One derived premise slot (QSpec FR-491 "Derived item and slots").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionSlot {
    /// The slot's position in the derived order.
    pub index: usize,
    /// The role.
    pub role: SlotRole,
    /// What backs the slot.
    pub backing: SlotBacking,
    /// The slot key of an item-backed slot.
    pub key: Option<CompositionSlotKey>,
    /// The guarantee the slot concerns, for every role but `environment`.
    pub guarantee: Option<ClauseRef>,
    /// The peer guarantee, for a `rely` and a `liveness-dependency` slot.
    pub on: Option<ClauseRef>,
    /// The constraint, for a `fairness` slot.
    pub constraint: Option<FairnessConstraint>,
    /// The environment member's name, for an `environment` slot.
    pub environment: Option<Identifier>,
    /// The span of the member that produced the slot.
    pub source: Span,
}

/// Where the composition item is settled (QSpec FR-491 "Derived item and
/// slots").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ItemPlacement {
    /// `assume-guarantee`: one planner-local item.
    PlannerLocal,
    /// The other three rules: one FR-331 `composition` request item.
    Request,
}

/// The item a checked declaration derives.
#[derive(Clone, Debug, PartialEq)]
pub struct CompositionItem {
    /// Who settles the item.
    pub placement: ItemPlacement,
    /// The selected rule.
    pub rule: CompositionRule,
    /// The declaration's node: the item's anchor.
    pub node: NodeKey,
    /// The checked package's identity text.
    pub package: String,
    /// Every component subject identity, by component name.
    pub subjects: Vec<ComponentSubject>,
    /// The item's `premises` member: its slots in derived order. A `Request`
    /// item holds the `component-guarantee`, `rely`, `liveness-dependency`,
    /// `fairness` and `environment` slots; the `fixed-point` slot is its own
    /// item.
    pub premises: Vec<CompositionSlot>,
}

impl CheckedComposition {
    /// The declared name `S`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The node `S` is emitted as (`correspondence`/`component_composition`).
    pub fn node(&self) -> &NodeKey {
        &self.key
    }

    /// The identity text of the checked package that carries `S`.
    pub fn package(&self) -> &str {
        &self.package
    }

    /// The selected rule.
    pub fn rule(&self) -> CompositionRule {
        self.rule
    }

    /// The spans of the declaration's members.
    pub fn spans(&self) -> CompositionSpans {
        self.spans
    }

    /// The `component` members in source order.
    pub fn components(&self) -> &[CheckedComponent] {
        &self.components
    }

    /// The `guarantee` members in source order.
    pub fn guarantees(&self) -> &[CompositionGuarantee] {
        &self.guarantees
    }

    /// The `rely` members in source order: the `relies` list.
    pub fn relies(&self) -> &[CompositionPremise] {
        &self.relies
    }

    /// The `depends` members in source order: the `liveness_dependencies`
    /// list.
    pub fn liveness_dependencies(&self) -> &[CompositionPremise] {
        &self.liveness_dependencies
    }

    /// The `supplies` members in source order.
    pub fn supplies(&self) -> &[CompositionSupply] {
        &self.supplies
    }

    /// The `environment` members in source order: the `environment` list.
    pub fn environment(&self) -> &[EnvironmentMember] {
        &self.environment
    }

    /// Every clause a member names, with its FR-363 form and fairness set.
    pub fn clauses(&self) -> &[CompositionClauseFacts] {
        &self.clauses
    }

    /// The dependency graph's cycles (strongly connected components of more
    /// than one node, and self edges).
    pub fn cycles(&self) -> &[CompositionCycle] {
        &self.cycles
    }

    /// The profile that declares FR-176's lattice, when the unit selects one.
    pub fn lattice(&self) -> Option<&str> {
        self.lattice.as_deref()
    }

    /// The composition item's scoped refusal, when the dependency graph has a
    /// cycle and no selected profile declares FR-176's lattice.
    pub fn item_refusal(&self) -> Option<&CompositionItemRefusal> {
        self.refusal.as_ref()
    }

    fn facts(&self, reference: &ClauseRef) -> Option<&CompositionClauseFacts> {
        self.clauses
            .iter()
            .find(|facts| facts.reference == *reference)
    }

    fn component(&self, name: &str) -> Option<&CheckedComponent> {
        self.components
            .iter()
            .find(|component| component.name == name)
    }

    /// The slot key of the `component-guarantee` slot of `reference`, with the
    /// FR-176 assumption set its `rely` and `depends` members give it.
    fn guarantee_key(&self, reference: &ClauseRef) -> Option<ComponentGuaranteeKey> {
        let facts = self.facts(reference)?;
        let component = self.component(&reference.component)?;
        let mut fairness: Vec<FairnessConstraint> = facts
            .fairness
            .iter()
            .map(|entry| entry.constraint.clone())
            .collect();
        fairness.sort();
        let assumptions: BTreeSet<NodeKey> = self
            .relies
            .iter()
            .chain(&self.liveness_dependencies)
            .filter(|premise| premise.guarantee == *reference)
            .map(|premise| premise.on.node.clone())
            .collect();
        Some(ComponentGuaranteeKey {
            key: SlotKey {
                node: reference.node.clone(),
                package: reference.node.package().to_owned(),
                ordinal: 0,
                subject: component.subject.to_string(),
                claim: reference.node.clone(),
                family: DeclarationFamily::TemporalTrace,
                kind: Capability::TemporalSatisfaction,
                item_kind: Some(ItemKind::Formula),
                fairness,
                stutter_closure: facts.is_liveness(),
                correspondence: None,
                extent: ExtentClass::Unbounded,
            },
            assumptions: assumptions.into_iter().collect(),
        })
    }

    /// The premise slots QSpec FR-491 "Derived item and slots" derives, in
    /// its order: one `component-guarantee` slot per declared guarantee, one
    /// `rely` slot per `rely`, one `liveness-dependency` slot per `depends`,
    /// one `fairness` slot per derived fairness premise (each constraint of
    /// the resolved fairness set of each declared guarantee and of each `H` a
    /// `rely` or `depends` member names), one `environment` slot per
    /// `environment` member, and one `fixed-point` slot per admitted cycle.
    pub fn slots(&self) -> Vec<CompositionSlot> {
        let mut slots: Vec<CompositionSlot> = Vec::new();
        let mut push = |slot: CompositionSlot| {
            let index = slots.len();
            slots.push(CompositionSlot { index, ..slot });
        };
        let slot = |role, backing, source| CompositionSlot {
            index: 0,
            role,
            backing,
            key: None,
            guarantee: None,
            on: None,
            constraint: None,
            environment: None,
            source,
        };
        for member in &self.guarantees {
            push(CompositionSlot {
                key: self
                    .guarantee_key(&member.reference)
                    .map(CompositionSlotKey::ComponentGuarantee),
                guarantee: Some(member.reference.clone()),
                ..slot(
                    SlotRole::ComponentGuarantee,
                    SlotBacking::Item,
                    member.source,
                )
            });
        }
        for (role, premises) in [
            (SlotRole::Rely, &self.relies),
            (SlotRole::LivenessDependency, &self.liveness_dependencies),
        ] {
            for premise in premises {
                push(CompositionSlot {
                    guarantee: Some(premise.guarantee.clone()),
                    on: Some(premise.on.clone()),
                    ..slot(role, SlotBacking::PlannerLocal, premise.source)
                });
            }
        }
        // The fairness premises: the declared guarantees first, then each
        // peer a `rely` and then a `depends` member names; a premise a clause
        // already derived is not derived again.
        let mut derived: BTreeSet<(&ClauseRef, &FairnessConstraint)> = BTreeSet::new();
        let owners = self
            .guarantees
            .iter()
            .map(|member| (&member.reference, member.source))
            .chain(
                self.relies
                    .iter()
                    .chain(&self.liveness_dependencies)
                    .map(|premise| (&premise.on, premise.source)),
            );
        for (reference, source) in owners {
            let Some(facts) = self.facts(reference) else {
                continue;
            };
            for entry in &facts.fairness {
                if derived.insert((reference, &entry.constraint)) {
                    push(CompositionSlot {
                        guarantee: Some(reference.clone()),
                        constraint: Some(entry.constraint.clone()),
                        ..slot(SlotRole::Fairness, SlotBacking::PlannerLocal, source)
                    });
                }
            }
        }
        for member in &self.environment {
            push(CompositionSlot {
                environment: Some(member.name.clone()),
                ..slot(
                    SlotRole::Environment,
                    SlotBacking::Environment,
                    member.source,
                )
            });
        }
        if let Some(profile) = &self.lattice {
            for cycle in &self.cycles {
                let premises: Vec<ComponentGuaranteeKey> = cycle
                    .guarantees
                    .iter()
                    .filter_map(|reference| self.guarantee_key(reference))
                    .collect();
                push(CompositionSlot {
                    key: Some(CompositionSlotKey::FixedPoint {
                        node: self.key.clone(),
                        package: self.package.clone(),
                        premises,
                        profile: profile.clone(),
                    }),
                    ..slot(SlotRole::FixedPoint, SlotBacking::Item, cycle.source)
                });
            }
        }
        slots
    }

    /// The item the declaration derives (QSpec FR-491 "Derived item and
    /// slots"): one planner-local item under `assume-guarantee`, one FR-331
    /// `composition` request item under the other three rules. An item whose
    /// [`Self::item_refusal`] is present is not settled.
    pub fn item(&self) -> CompositionItem {
        let placement = match self.rule {
            CompositionRule::AssumeGuarantee => ItemPlacement::PlannerLocal,
            CompositionRule::ProtocolRefinement
            | CompositionRule::RefinementTransfer
            | CompositionRule::Interface => ItemPlacement::Request,
        };
        let premises = self
            .slots()
            .into_iter()
            .filter(|slot| {
                placement == ItemPlacement::PlannerLocal || slot.role != SlotRole::FixedPoint
            })
            .collect();
        let mut subjects: Vec<&CheckedComponent> = self.components.iter().collect();
        subjects.sort_by(|left, right| left.name.cmp(&right.name));
        CompositionItem {
            placement,
            rule: self.rule,
            node: self.key.clone(),
            package: self.package.clone(),
            subjects: subjects
                .into_iter()
                .map(|component| component.subject.clone())
                .collect(),
            premises,
        }
    }

    /// The term the composition node's body is (QSpec FR-491 "Checked-package
    /// node"), its arrays in canonical order.
    pub fn term(&self) -> CompositionTerm {
        let clause = |reference: &ClauseRef| {
            ClauseEntry::new(reference.component.clone(), reference.node.clone())
        };
        let premise = |premise: &CompositionPremise| {
            PremiseEntry::new(clause(&premise.guarantee), clause(&premise.on))
        };
        CompositionTerm::new(
            self.rule,
            self.components
                .iter()
                .map(|component| {
                    ComponentEntry::new(
                        component.name.clone(),
                        component.subject.package.clone(),
                        component.subject_node.clone(),
                    )
                })
                .collect(),
            self.guarantees
                .iter()
                .map(|member| clause(&member.reference))
                .collect(),
            self.relies.iter().map(premise).collect(),
            self.liveness_dependencies.iter().map(premise).collect(),
            self.supplies
                .iter()
                .map(|supply| {
                    SupplyEntry::new(
                        kind_spelling(supply.kind).to_owned(),
                        granularity_spelling(supply.granularity).to_owned(),
                        supply.context.clone(),
                        supply.operation.as_str().to_owned(),
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
                                context: NodeRef::Admitted(context.clone()),
                                operation: operation.as_str().to_owned(),
                            },
                            Supplies::Assumption(node) => {
                                SuppliesEntry::Assumption(NodeRef::Admitted(node.clone()))
                            }
                        },
                    )
                })
                .collect(),
        )
    }
}

/// The `Origin::TypeDeclaration` name a `compose` declaration is located
/// under (QSpec FR-451's pattern, as the guarantee's).
pub(crate) fn declaration_origin(name: &str) -> String {
    format!("compose {name}")
}

/// What an imported package says about one component `M`.
struct ComponentFacts {
    subject: ComponentSubject,
    subject_node: NodeKey,
    operations: BTreeSet<SubjectOperation>,
}

/// A component with the graph it is read from.
struct Component<'g> {
    name: String,
    graph: &'g CheckedGraph,
    alias: String,
    facts: ComponentFacts,
    source: Span,
}

/// The facts of the model `alias` of the package `graph` (identity
/// `library`): its subject identity, its subject node and every operation of
/// the object types it exports.
fn component_facts(graph: &CheckedGraph, library: &str, alias: &str) -> Option<ComponentFacts> {
    let names = graph.model_names.get(alias)?;
    let mut subject_node: Option<NodeKey> = None;
    let mut operations = BTreeSet::new();
    for key in names.exports() {
        let (Some(node), Some(id)) = (graph.object_nodes.get(key), graph.object_types.get(key))
        else {
            continue;
        };
        subject_node.get_or_insert_with(|| node.clone());
        for (declaring, operation) in graph
            .scope
            .operations()
            .effective(graph.scope.types(), id.clone())
        {
            let (_, context) = graph.context_of(&declaring)?;
            operations.insert(SubjectOperation {
                context,
                operation: Identifier::new(operation.name()).ok()?,
            });
        }
    }
    Some(ComponentFacts {
        subject: ComponentSubject {
            package: library.to_owned(),
            model: names.package().to_owned(),
        },
        subject_node: subject_node?,
        operations,
    })
}

/// A fairness kind a node spells.
#[ix_trace_rs::string_edge]
fn kind_of(spelling: &str) -> Option<FairnessKind> {
    match spelling {
        "weak" => Some(FairnessKind::Weak),
        "strong" => Some(FairnessKind::Strong),
        _ => None,
    }
}

/// A fairness granularity a node spells.
#[ix_trace_rs::string_edge]
fn granularity_of(spelling: &str) -> Option<FairnessGranularity> {
    match spelling {
        "whole" => Some(FairnessGranularity::Whole),
        "each" => Some(FairnessGranularity::Each),
        _ => None,
    }
}

fn rule_of(form: CompositionRuleForm) -> CompositionRule {
    match form {
        CompositionRuleForm::AssumeGuarantee => CompositionRule::AssumeGuarantee,
        CompositionRuleForm::ProtocolRefinement => CompositionRule::ProtocolRefinement,
        CompositionRuleForm::RefinementTransfer => CompositionRule::RefinementTransfer,
        CompositionRuleForm::Interface => CompositionRule::Interface,
    }
}

/// The members check 3 and 4 read, in the same shape on the source path and
/// on the reader's.
struct Members {
    guarantees: Vec<CompositionGuarantee>,
    relies: Vec<CompositionPremise>,
    liveness_dependencies: Vec<CompositionPremise>,
    clauses: Vec<CompositionClauseFacts>,
}

impl Members {
    fn facts(&self, reference: &ClauseRef) -> Option<&CompositionClauseFacts> {
        self.clauses
            .iter()
            .find(|facts| facts.reference == *reference)
    }

    /// QSpec FR-491 "Static checks" 3 then 4 over every `rely` and `depends`
    /// member, in source order within each check.
    fn check_premises(&self) -> Result<(), CompositionRefusal> {
        // Each member with the word it is written with and whether it is a
        // `rely`, in source order within each list.
        let premises = || {
            self.relies
                .iter()
                .map(|premise| ("rely", true, premise))
                .chain(
                    self.liveness_dependencies
                        .iter()
                        .map(|premise| ("depends", false, premise)),
                )
        };
        let malformed = |reason, word: &str, premise: &CompositionPremise| {
            Err(CompositionRefusal::Malformed {
                reason,
                member: format!("{word} {} on {}", premise.guarantee, premise.on),
                span: premise.source,
            })
        };
        // Check 3.
        for (word, _, premise) in premises() {
            if !self
                .guarantees
                .iter()
                .any(|member| member.reference == premise.guarantee)
            {
                return malformed(MalformedMember::UndeclaredGuarantee, word, premise);
            }
            if premise.guarantee.component == premise.on.component {
                return malformed(MalformedMember::SelfComponent, word, premise);
            }
        }
        // Check 4.
        let liveness =
            |reference: &ClauseRef| self.facts(reference).is_some_and(|f| f.is_liveness());
        for (word, is_rely, premise) in premises() {
            if is_rely {
                if liveness(&premise.on) {
                    return malformed(MalformedMember::RelyOnLiveness, word, premise);
                }
            } else if !(liveness(&premise.guarantee) && liveness(&premise.on)) {
                return malformed(MalformedMember::DependsNotLiveness, word, premise);
            }
        }
        Ok(())
    }

    /// Two members that state one guarantee, `rely`, `depends`, `supplies` or
    /// `environment` name, as the first such pair.
    fn check_distinct(
        &self,
        supplies: &[CompositionSupply],
        environment: &[EnvironmentMember],
    ) -> Result<(), CompositionRefusal> {
        let conflict = |member: String, first: Span, second: Span| {
            Err(CompositionRefusal::ConflictingMember {
                member,
                first,
                second,
            })
        };
        let mut seen: BTreeMap<&ClauseRef, Span> = BTreeMap::new();
        for member in &self.guarantees {
            if let Some(first) = seen.insert(&member.reference, member.source) {
                return conflict(
                    format!("guarantee {}", member.reference),
                    first,
                    member.source,
                );
            }
        }
        for (word, premises) in [
            ("rely", &self.relies),
            ("depends", &self.liveness_dependencies),
        ] {
            let mut seen: BTreeMap<(&ClauseRef, &ClauseRef), Span> = BTreeMap::new();
            for premise in premises {
                if let Some(first) = seen.insert((&premise.guarantee, &premise.on), premise.source)
                {
                    return conflict(
                        format!("{word} {} on {}", premise.guarantee, premise.on),
                        first,
                        premise.source,
                    );
                }
            }
        }
        let mut seen: BTreeMap<(&NodeKey, &str, &str, &str), Span> = BTreeMap::new();
        for supply in supplies {
            let key = (
                &supply.context,
                supply.operation.as_str(),
                kind_spelling(supply.kind),
                granularity_spelling(supply.granularity),
            );
            if let Some(first) = seen.insert(key, supply.source) {
                return conflict(
                    format!("supplies {}", supply.operation.as_str()),
                    first,
                    supply.source,
                );
            }
        }
        let mut seen: BTreeMap<&str, Span> = BTreeMap::new();
        for member in environment {
            if let Some(first) = seen.insert(member.name.as_str(), member.source) {
                return conflict(
                    format!("environment {}", member.name.as_str()),
                    first,
                    member.source,
                );
            }
        }
        Ok(())
    }

    /// The dependency graph's cycles: the strongly connected components of
    /// more than one node, and the nodes with an edge to themselves. A node is
    /// a declared guarantee; an edge `C::G -> D::H` is a `rely` or `depends`
    /// member between two declared guarantees.
    fn cycles(&self) -> Vec<CompositionCycle> {
        let nodes: Vec<&ClauseRef> = self
            .guarantees
            .iter()
            .map(|member| &member.reference)
            .collect();
        let position = |reference: &ClauseRef| nodes.iter().position(|node| *node == reference);
        // The edges in member order: rely members, then depends members.
        let edges: Vec<(usize, usize, Span)> = self
            .relies
            .iter()
            .chain(&self.liveness_dependencies)
            .filter_map(|premise| {
                Some((
                    position(&premise.guarantee)?,
                    position(&premise.on)?,
                    premise.source,
                ))
            })
            .collect();
        let mut successors: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
        for (from, to, _) in &edges {
            if let Some(list) = successors.get_mut(*from) {
                list.push(*to);
            }
        }
        let mut cycles = Vec::new();
        for component in strongly_connected(&successors) {
            let internal: Vec<&(usize, usize, Span)> = edges
                .iter()
                .filter(|(from, to, _)| component.contains(from) && component.contains(to))
                .collect();
            if component.len() < 2 && internal.is_empty() {
                continue;
            }
            let mut tails: Vec<usize> = Vec::new();
            for (from, _, _) in &internal {
                if !tails.contains(from) {
                    tails.push(*from);
                }
            }
            let Some(source) = internal
                .iter()
                .map(|(_, _, span)| *span)
                .min_by_key(|s| s.start)
            else {
                continue;
            };
            cycles.push(CompositionCycle {
                guarantees: tails
                    .into_iter()
                    .filter_map(|index| nodes.get(index).map(|node| (*node).clone()))
                    .collect(),
                source,
            });
        }
        cycles.sort_by_key(|cycle| cycle.source.start);
        cycles
    }
}

/// The strongly connected components of the graph whose successor lists are
/// `successors` (Tarjan's algorithm, iterative so a long chain of members
/// cannot exhaust the stack).
fn strongly_connected(successors: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let count = successors.len();
    let mut index: Vec<Option<usize>> = vec![None; count];
    let mut low: Vec<usize> = vec![0; count];
    let mut on_stack: Vec<bool> = vec![false; count];
    let mut stack: Vec<usize> = Vec::new();
    let mut next_index = 0usize;
    let mut components: Vec<Vec<usize>> = Vec::new();
    for root in 0..count {
        if index.get(root).copied().flatten().is_some() {
            continue;
        }
        let mut work: Vec<(usize, usize)> = vec![(root, 0)];
        let visit = |node: usize,
                     next_index: &mut usize,
                     index: &mut Vec<Option<usize>>,
                     low: &mut Vec<usize>,
                     on_stack: &mut Vec<bool>,
                     stack: &mut Vec<usize>| {
            if let (Some(i), Some(l), Some(s)) = (
                index.get_mut(node),
                low.get_mut(node),
                on_stack.get_mut(node),
            ) {
                *i = Some(*next_index);
                *l = *next_index;
                *s = true;
            }
            *next_index += 1;
            stack.push(node);
        };
        visit(
            root,
            &mut next_index,
            &mut index,
            &mut low,
            &mut on_stack,
            &mut stack,
        );
        while let Some(&(node, edge)) = work.last() {
            let next = successors
                .get(node)
                .and_then(|list| list.get(edge))
                .copied();
            if let Some(next) = next {
                if let Some(top) = work.last_mut() {
                    top.1 += 1;
                }
                match index.get(next).copied().flatten() {
                    None => {
                        visit(
                            next,
                            &mut next_index,
                            &mut index,
                            &mut low,
                            &mut on_stack,
                            &mut stack,
                        );
                        work.push((next, 0));
                    }
                    Some(reached) if on_stack.get(next).copied().unwrap_or(false) => {
                        if let Some(l) = low.get_mut(node) {
                            *l = (*l).min(reached);
                        }
                    }
                    Some(_) => {}
                }
                continue;
            }
            work.pop();
            let node_low = low.get(node).copied().unwrap_or(0);
            if let Some(&(parent, _)) = work.last() {
                if let Some(l) = low.get_mut(parent) {
                    *l = (*l).min(node_low);
                }
            }
            if index.get(node).copied().flatten() == Some(node_low) {
                let mut component = Vec::new();
                while let Some(member) = stack.pop() {
                    if let Some(s) = on_stack.get_mut(member) {
                        *s = false;
                    }
                    component.push(member);
                    if member == node {
                        break;
                    }
                }
                component.sort_unstable();
                components.push(component);
            }
        }
    }
    components
}

impl CheckedGraph {
    /// QSpec FR-491: the unit's checked `compose` declarations, in
    /// declaration order.
    pub fn compositions(&self) -> &[CheckedComposition] {
        &self.compositions
    }

    /// The profile the unit selects that declares FR-176's lattice, initial
    /// element and convergence rule, when it selects one.
    pub fn composition_lattice(&self) -> Option<&str> {
        self.composition_lattice.as_deref()
    }

    /// Where a refusal in the `index`th `compose` declaration is located.
    pub(super) fn composition_location(&self, index: usize) -> Location {
        Location::root(Origin::TypeDeclaration {
            name: self
                .composition_forms
                .get(index)
                .map(|declaration| declaration_origin(&declaration.name.name))
                .unwrap_or_default(),
        })
    }

    /// QSpec FR-491 "Static checks" 2 to 5 over every `compose` declaration.
    /// A declaration's first refusal is reported with its index; a
    /// declaration that checks is produced.
    ///
    /// # Errors
    ///
    /// Each declaration's first refusal, in declaration order.
    pub(super) fn check_declared_compositions(
        &self,
    ) -> Result<Vec<CheckedComposition>, Vec<(usize, CompositionRefusal)>> {
        let mut checked = Vec::new();
        let mut refusals = Vec::new();
        for (index, form) in self.composition_forms.iter().enumerate() {
            match self.check_composition(form) {
                Ok(composition) => checked.push(composition),
                Err(refusal) => refusals.push((index, refusal)),
            }
        }
        if refusals.is_empty() {
            Ok(checked)
        } else {
            Err(refusals)
        }
    }

    /// The structural key of the node of the composition named `name`.
    pub(super) fn composition_node_key(&self, name: &str) -> Option<NodeKey> {
        super::derived_node_key(
            self.package_name(),
            super::node_key::NodeTag::Correspondence,
            "component_composition",
            vec![quire_exact::PathSegment::Text(name.to_owned())],
            0,
        )
        .ok()
    }

    fn check_composition(
        &self,
        form: &CompositionDeclarationForm,
    ) -> Result<CheckedComposition, CompositionRefusal> {
        let internal = |invariant| {
            CompositionRefusal::Internal(InternalFault::new("check.composition", invariant))
        };
        let missing = |what, name: &str, span| CompositionRefusal::MissingName {
            what,
            name: name.to_owned(),
            span,
        };
        // Check 2: the components.
        let mut components: Vec<Component<'_>> = Vec::new();
        for member in &form.components {
            let import = self.scope.imported(&member.import.name).ok_or_else(|| {
                missing(
                    CompositionName::Import,
                    &member.import.name,
                    member.import.span,
                )
            })?;
            let graph = import.graph.as_ref();
            let facts = component_facts(graph, import.identity.as_str(), &member.model.name)
                .ok_or_else(|| {
                    missing(
                        CompositionName::Model,
                        &member.model.name,
                        member.model.span,
                    )
                })?;
            if let Some(first) = components.iter().find(|c| c.name == member.name.name) {
                return Err(CompositionRefusal::ConflictingComponent {
                    name: member.name.name.clone(),
                    first: first.source,
                    second: member.span,
                });
            }
            if let Some(first) = components.iter().find(|c| c.facts.subject == facts.subject) {
                return Err(CompositionRefusal::ConflictingSubject {
                    subject: facts.subject.to_string(),
                    first_component: first.name.clone(),
                    second_component: member.name.name.clone(),
                    first: first.source,
                    second: member.span,
                });
            }
            components.push(Component {
                name: member.name.name.clone(),
                graph,
                alias: member.model.name.clone(),
                facts,
                source: member.span,
            });
        }
        // Check 2: the clauses the members name.
        let mut clauses: Vec<CompositionClauseFacts> = Vec::new();
        let mut resolve = |name: &ComponentClauseForm| -> Result<ClauseRef, CompositionRefusal> {
            let component = components
                .iter()
                .find(|component| component.name == name.component.name)
                .ok_or_else(|| {
                    missing(
                        CompositionName::Component,
                        &name.component.name,
                        name.component.span,
                    )
                })?;
            let absent = || missing(CompositionName::Clause, &name.clause.name, name.clause.span);
            let graph = component.graph;
            let node = graph
                .temporal_clauses
                .iter()
                .find(|node| node.clause().name() == name.clause.name)
                .ok_or_else(absent)?;
            if graph.temporal_subject_aliases.get(&name.clause.name) != Some(&component.alias) {
                return Err(absent());
            }
            let facts = graph
                .guarantee_clause(node, component.alias.clone())
                .ok_or_else(absent)?;
            let reference = ClauseRef {
                component: name.component.name.clone(),
                name: name.clause.name.clone(),
                node: facts.node.clone(),
            };
            if !clauses.iter().any(|known| known.reference == reference) {
                clauses.push(CompositionClauseFacts {
                    reference: reference.clone(),
                    form: facts.form,
                    fairness: facts.fairness,
                });
            }
            Ok(reference)
        };
        let mut guarantees = Vec::new();
        let mut relies = Vec::new();
        let mut liveness_dependencies = Vec::new();
        for member in &form.members {
            match member {
                CompositionMemberForm::Guarantee(member) => guarantees.push(CompositionGuarantee {
                    reference: resolve(&member.guarantee)?,
                    source: member.span,
                }),
                CompositionMemberForm::Rely(member) => {
                    relies.push(CompositionPremise {
                        guarantee: resolve(&member.guarantee)?,
                        on: resolve(&member.on)?,
                        source: member.span,
                    });
                }
                CompositionMemberForm::Depends(member) => {
                    liveness_dependencies.push(CompositionPremise {
                        guarantee: resolve(&member.guarantee)?,
                        on: resolve(&member.on)?,
                        source: member.span,
                    });
                }
                CompositionMemberForm::Supplies(_) | CompositionMemberForm::Environment(_) => {}
            }
        }
        let members = Members {
            guarantees,
            relies,
            liveness_dependencies,
            clauses,
        };
        // Checks 3 and 4.
        members.check_premises()?;
        // Check 5: the operations and predicates the members name.
        let mut supplies = Vec::new();
        let mut environment = Vec::new();
        for member in &form.members {
            match member {
                CompositionMemberForm::Supplies(member) => {
                    let (context, operation) =
                        resolve_operation(&components, &member.constraint.operation)?;
                    supplies.push(CompositionSupply {
                        kind: super::temporal_clause::checked_kind(&member.constraint),
                        granularity: super::temporal_clause::checked_granularity(
                            &member.constraint,
                        ),
                        context,
                        operation,
                        source: member.span,
                    });
                }
                CompositionMemberForm::Environment(member) => {
                    let name = Identifier::new(&member.name.name).map_err(|_| {
                        missing(
                            CompositionName::Operation,
                            &member.name.name,
                            member.name.span,
                        )
                    })?;
                    let supplies = match &member.supplies {
                        GuaranteeSuppliesForm::Fairness(constraint) => {
                            let (context, operation) =
                                resolve_operation(&components, &constraint.operation)?;
                            Supplies::Fairness {
                                kind: super::temporal_clause::checked_kind(constraint),
                                granularity: super::temporal_clause::checked_granularity(
                                    constraint,
                                ),
                                context,
                                operation,
                            }
                        }
                        GuaranteeSuppliesForm::Assumes(predicate) => Supplies::Assumption(
                            self.resolve_assumption(&member.name.name, predicate)?,
                        ),
                    };
                    environment.push(EnvironmentMember {
                        name,
                        supplier: member.supplier.text.clone(),
                        supplies,
                        source: member.span,
                    });
                }
                CompositionMemberForm::Guarantee(_)
                | CompositionMemberForm::Rely(_)
                | CompositionMemberForm::Depends(_) => {}
            }
        }
        members.check_distinct(&supplies, &environment)?;
        let key = self
            .composition_node_key(&form.name.name)
            .ok_or_else(|| internal("composition-node-is-keyed"))?;
        Ok(self.finish_composition(
            form.name.name.clone(),
            key,
            rule_of(form.rule),
            CompositionSpans {
                declaration: form.span,
                rule: form.rule_span,
            },
            components
                .iter()
                .map(|component| CheckedComponent {
                    name: component.name.clone(),
                    subject: component.facts.subject.clone(),
                    subject_node: component.facts.subject_node.clone(),
                    source: component.source,
                })
                .collect(),
            members,
            supplies,
            environment,
        ))
    }

    /// The composition, its dependency graph's cycles found and the lattice
    /// check applied.
    #[allow(
        clippy::too_many_arguments,
        reason = "the checked composition's members"
    )]
    fn finish_composition(
        &self,
        name: String,
        key: NodeKey,
        rule: CompositionRule,
        spans: CompositionSpans,
        components: Vec<CheckedComponent>,
        mut members: Members,
        supplies: Vec<CompositionSupply>,
        environment: Vec<EnvironmentMember>,
    ) -> CheckedComposition {
        // The order the clauses were found in depends on the path that found
        // them (the source's, or the node's), so they are kept by identity.
        members
            .clauses
            .sort_by(|left, right| left.reference.cmp(&right.reference));
        let cycles = members.cycles();
        let lattice = self.composition_lattice.clone();
        let refusal = (!cycles.is_empty() && lattice.is_none()).then(|| CompositionItemRefusal {
            cycles: cycles
                .iter()
                .map(|cycle| cycle.guarantees.clone())
                .collect(),
        });
        CheckedComposition {
            name,
            key,
            package: self.package_name().to_owned(),
            rule,
            spans,
            components,
            guarantees: members.guarantees,
            relies: members.relies,
            liveness_dependencies: members.liveness_dependencies,
            supplies,
            environment,
            clauses: members.clauses,
            cycles,
            lattice,
            refusal,
        }
    }

    /// Check 5 for an `assumes` name: the node of the `predicate` it names.
    fn resolve_assumption(
        &self,
        environment: &str,
        name: &NameForm,
    ) -> Result<NodeKey, CompositionRefusal> {
        let missing = || CompositionRefusal::MissingName {
            what: CompositionName::Assumption,
            name: name.name.clone(),
            span: name.span,
        };
        if self.predicate_names.contains(&name.name) {
            return self.function_identity(&name.name).ok_or_else(missing);
        }
        if self.declares_function(&name.name) || self.type_spans.contains_key(&name.name) {
            return Err(CompositionRefusal::Malformed {
                reason: MalformedMember::AssumptionNotPredicate,
                member: format!("environment {environment} assumes {}", name.name),
                span: name.span,
            });
        }
        Err(missing())
    }
}

/// Check 5 for an operation name `D::T::o`: it resolves, in a component whose
/// model alias (or name) is `D`, to an operation of that component's subject,
/// as its declaring object type node and name.
fn resolve_operation(
    components: &[Component<'_>],
    name: &NameForm,
) -> Result<(NodeKey, Identifier), CompositionRefusal> {
    let missing = || CompositionRefusal::MissingName {
        what: CompositionName::Operation,
        name: name.name.clone(),
        span: name.span,
    };
    let parts = name_parts(name, 3).ok_or_else(missing)?;
    let written = parts.first().ok_or_else(missing)?;
    for component in components {
        if component.alias != written.as_str() && component.name != written.as_str() {
            continue;
        }
        let graph = component.graph;
        let Some(declaring) = graph
            .model_names
            .get(component.alias.as_str())
            .zip(parts.get(1))
            .and_then(|(model, ty)| model.export(ty))
        else {
            continue;
        };
        let Some(owner) = graph.object_types.get(declaring) else {
            continue;
        };
        let Some(operation_name) = parts.get(2) else {
            continue;
        };
        let crate::model::operation::OperationLookup::Declared {
            declaring: resolved,
            operation,
        } = graph.scope.operations().resolve(
            graph.scope.types(),
            owner.clone(),
            operation_name.as_str(),
        )
        else {
            continue;
        };
        let Some((_, context)) = graph.context_of(&resolved) else {
            continue;
        };
        let Ok(operation) = Identifier::new(operation.name()) else {
            continue;
        };
        let wanted = SubjectOperation {
            context: context.clone(),
            operation: operation.clone(),
        };
        if component.facts.operations.contains(&wanted) {
            return Ok((context, operation));
        }
    }
    Err(missing())
}

/// One component of a composition node as a reader found it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadComponent {
    /// The component name.
    pub name: String,
    /// The identity of the component's package.
    pub package: String,
    /// The subject node, in the component's package.
    pub subject: NodeKey,
    /// The span of the member, from the node's source map.
    pub span: Span,
}

/// A `C::G` of a composition node as a reader found it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadClause {
    /// The component name.
    pub component: String,
    /// The clause node, in the component's package.
    pub clause: NodeKey,
}

/// A `guarantee` entry of a composition node as a reader found it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadGuaranteeMember {
    /// The guarantee.
    pub clause: ReadClause,
    /// The span of the member, from the node's source map.
    pub span: Span,
}

/// A `rely` or `depends` entry of a composition node as a reader found it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadPremise {
    /// The guarantee that carries the premise.
    pub guarantee: ReadClause,
    /// The peer guarantee.
    pub on: ReadClause,
    /// The span of the member, from the node's source map.
    pub span: Span,
}

/// A `supplies` entry of a composition node as a reader found it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadSupply {
    /// `weak` or `strong`.
    pub kind: String,
    /// `whole` or `each`.
    pub granularity: String,
    /// The operation's declaring object type node.
    pub context: NodeKey,
    /// The operation name.
    pub operation: String,
    /// The span of the member, from the node's source map.
    pub span: Span,
}

/// A composition node's body and source map as a reader found them, after the
/// reader's shape, identity and dependency checks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadComposition {
    /// The declared name, from the node's id.
    pub name: String,
    /// The rule.
    pub rule: CompositionRule,
    /// The `components` entries in the node's order.
    pub components: Vec<ReadComponent>,
    /// The `guarantees` entries in the node's order.
    pub guarantees: Vec<ReadGuaranteeMember>,
    /// The `relies` entries in the node's order.
    pub relies: Vec<ReadPremise>,
    /// The `liveness_dependencies` entries in the node's order.
    pub liveness_dependencies: Vec<ReadPremise>,
    /// The `supplies` entries in the node's order.
    pub supplies: Vec<ReadSupply>,
    /// The `environment` entries in the node's order.
    pub environment: Vec<ReadEnvironment>,
    /// The spans of the declaration's members.
    pub spans: CompositionSpans,
}

impl CheckedGraph {
    /// QSpec FR-491 "Checked-package node", the reader's steps after the
    /// shape, identity, term and canonical-order checks: each target is a node
    /// of the component package it names, in `dependency`'s resolution of the
    /// package identities (FR-322), then static checks 2 to 5 on the read
    /// values.
    ///
    /// # Errors
    ///
    /// The first defect, in the reader's order.
    pub fn read_composition<'g>(
        &self,
        dependency: &dyn Fn(&str) -> Option<&'g CheckedGraph>,
        read: &ReadComposition,
    ) -> Result<CheckedComposition, CompositionRefusal> {
        let internal = |invariant| {
            CompositionRefusal::Internal(InternalFault::new("check.composition-read", invariant))
        };
        let missing = |what, name: &str, span| CompositionRefusal::MissingName {
            what,
            name: name.to_owned(),
            span,
        };
        let key = self
            .composition_node_key(&read.name)
            .ok_or_else(|| internal("composition-node-is-keyed"))?;
        // Check 2: the components, whose subjects are nodes of their packages.
        let mut components: Vec<Component<'_>> = Vec::new();
        for entry in &read.components {
            let graph = dependency(&entry.package)
                .ok_or_else(|| missing(CompositionName::Import, &entry.package, entry.span))?;
            let (alias, facts) = graph
                .model_names
                .keys()
                .find_map(|alias| {
                    component_facts(graph, &entry.package, alias)
                        .filter(|facts| facts.subject_node == entry.subject)
                        .map(|facts| (alias.clone(), facts))
                })
                .ok_or_else(|| {
                    missing(
                        CompositionName::Model,
                        &entry.subject.to_string(),
                        entry.span,
                    )
                })?;
            if let Some(first) = components.iter().find(|c| c.name == entry.name) {
                return Err(CompositionRefusal::ConflictingComponent {
                    name: entry.name.clone(),
                    first: first.source,
                    second: entry.span,
                });
            }
            if let Some(first) = components.iter().find(|c| c.facts.subject == facts.subject) {
                return Err(CompositionRefusal::ConflictingSubject {
                    subject: facts.subject.to_string(),
                    first_component: first.name.clone(),
                    second_component: entry.name.clone(),
                    first: first.source,
                    second: entry.span,
                });
            }
            components.push(Component {
                name: entry.name.clone(),
                graph,
                alias,
                facts,
                source: entry.span,
            });
        }
        // Check 2: the clauses the entries name.
        let mut clauses: Vec<CompositionClauseFacts> = Vec::new();
        let mut resolve =
            |entry: &ReadClause, span: Span| -> Result<ClauseRef, CompositionRefusal> {
                let component = components
                    .iter()
                    .find(|component| component.name == entry.component)
                    .ok_or_else(|| missing(CompositionName::Component, &entry.component, span))?;
                let absent = || missing(CompositionName::Clause, &entry.clause.to_string(), span);
                let graph = component.graph;
                let node = graph
                    .temporal_clauses
                    .iter()
                    .find(|node| node.identity() == entry.clause)
                    .ok_or_else(absent)?;
                let name = node.clause().name().to_owned();
                if graph.temporal_subject_aliases.get(&name) != Some(&component.alias) {
                    return Err(absent());
                }
                let facts = graph
                    .guarantee_clause(node, component.alias.clone())
                    .ok_or_else(absent)?;
                let reference = ClauseRef {
                    component: entry.component.clone(),
                    name,
                    node: facts.node.clone(),
                };
                if !clauses.iter().any(|known| known.reference == reference) {
                    clauses.push(CompositionClauseFacts {
                        reference: reference.clone(),
                        form: facts.form,
                        fairness: facts.fairness,
                    });
                }
                Ok(reference)
            };
        let mut guarantees = Vec::new();
        for member in &read.guarantees {
            guarantees.push(CompositionGuarantee {
                reference: resolve(&member.clause, member.span)?,
                source: member.span,
            });
        }
        let mut premises =
            |entries: &[ReadPremise]| -> Result<Vec<CompositionPremise>, CompositionRefusal> {
                entries
                    .iter()
                    .map(|entry| {
                        Ok(CompositionPremise {
                            guarantee: resolve(&entry.guarantee, entry.span)?,
                            on: resolve(&entry.on, entry.span)?,
                            source: entry.span,
                        })
                    })
                    .collect()
            };
        let mut relies = premises(&read.relies)?;
        let mut liveness_dependencies = premises(&read.liveness_dependencies)?;
        // The source map gives source order back.
        guarantees.sort_by_key(|member| (member.source.start, member.source.end));
        relies.sort_by_key(|member| (member.source.start, member.source.end));
        liveness_dependencies.sort_by_key(|member| (member.source.start, member.source.end));
        let members = Members {
            guarantees,
            relies,
            liveness_dependencies,
            clauses,
        };
        // Checks 3 and 4.
        members.check_premises()?;
        // Check 5: every operation is one of a component subject's.
        let is_operation = |context: &NodeKey, operation: &Identifier| {
            let wanted = SubjectOperation {
                context: context.clone(),
                operation: operation.clone(),
            };
            components
                .iter()
                .any(|component| component.facts.operations.contains(&wanted))
        };
        let mut supplies = Vec::new();
        for entry in &read.supplies {
            let operation = Identifier::new(&entry.operation)
                .map_err(|_| missing(CompositionName::Operation, &entry.operation, entry.span))?;
            if !is_operation(&entry.context, &operation) {
                return Err(missing(
                    CompositionName::Operation,
                    &entry.operation,
                    entry.span,
                ));
            }
            supplies.push(CompositionSupply {
                kind: kind_of(&entry.kind).ok_or_else(|| internal("fairness-kind-is-spelled"))?,
                granularity: granularity_of(&entry.granularity)
                    .ok_or_else(|| internal("fairness-granularity-is-spelled"))?,
                context: entry.context.clone(),
                operation,
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
                    let identifier = Identifier::new(operation)
                        .map_err(|_| missing(CompositionName::Operation, operation, entry.span))?;
                    if !is_operation(&context, &identifier) {
                        return Err(missing(CompositionName::Operation, operation, entry.span));
                    }
                    Supplies::Fairness {
                        kind: kind_of(kind).ok_or_else(|| internal("fairness-kind-is-spelled"))?,
                        granularity: granularity_of(granularity)
                            .ok_or_else(|| internal("fairness-granularity-is-spelled"))?,
                        context,
                        operation: identifier,
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
                        return Err(CompositionRefusal::Malformed {
                            reason: MalformedMember::AssumptionNotPredicate,
                            member: format!("environment {} assumes {node}", entry.name),
                            span: entry.span,
                        });
                    } else {
                        return Err(missing(
                            CompositionName::Assumption,
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
        supplies.sort_by_key(|member: &CompositionSupply| (member.source.start, member.source.end));
        environment
            .sort_by_key(|member: &EnvironmentMember| (member.source.start, member.source.end));
        members.check_distinct(&supplies, &environment)?;
        Ok(self.finish_composition(
            read.name.clone(),
            key,
            read.rule,
            read.spans,
            {
                let mut checked: Vec<CheckedComponent> = components
                    .iter()
                    .map(|component| CheckedComponent {
                        name: component.name.clone(),
                        subject: component.facts.subject.clone(),
                        subject_node: component.facts.subject_node.clone(),
                        source: component.source,
                    })
                    .collect();
                checked.sort_by_key(|component| (component.source.start, component.source.end));
                checked
            },
            members,
            supplies,
            environment,
        ))
    }
}
