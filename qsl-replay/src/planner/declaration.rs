// SPDX-License-Identifier: AGPL-3.0-or-later
//! The planner's declaration input (QSL FR-529, QSpec FR-488, FR-490,
//! FR-491): plain data the front end fills from a checked package, and
//! nothing about how it was checked. The planner reads no source and no
//! package; it reads this and one settlement request.

use qsl_foundation::source::provenance::SourceRegion;
use quire_exact::NodeKey;
use quire_semantic_value::fairness::{FairnessConstraint, FairnessGranularity, FairnessKind};

use super::key::{OperationRef, SlotKey};

/// The role of the planner-local item a request settles (QSpec FR-492
/// `item.role`).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ItemRole {
    /// An FR-490 implementation guarantee.
    Guarantee,
    /// An FR-491 composition with rule `assume-guarantee`.
    Composition,
}

impl ItemRole {
    /// The request's `item.role` spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Guarantee => "guarantee",
            Self::Composition => "composition",
        }
    }

    /// The role `text` spells.
    #[ix_trace_rs::string_edge]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "guarantee" => Some(Self::Guarantee),
            "composition" => Some(Self::Composition),
            _ => None,
        }
    }
}

/// The closed role of a premise slot (QSpec FR-490 "Derived items", FR-491
/// "Derived item and slots").
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SlotRole {
    /// The clause proved over the model subject.
    ModelClaim,
    /// Every implementation initial state maps to a model initial state.
    Initialization,
    /// Every step maps to a model step or an allowed stutter.
    TransitionSimulation,
    /// No step changes abstract state outside the operation's frame.
    Frame,
    /// An environment premise, listed or required.
    Environment,
    /// The allowed stutters cannot continue forever.
    NoDivergence,
    /// A constraint of `F_P` implied by the supplied scheduling premises.
    FairnessCorrespondence,
    /// A component guarantee under its assumptions (composition).
    ComponentGuarantee,
    /// A `rely` premise (composition).
    Rely,
    /// A liveness dependency (composition).
    LivenessDependency,
    /// A derived fairness premise (composition).
    Fairness,
    /// A cycle's fixed-point item (composition).
    FixedPoint,
}

impl SlotRole {
    /// Every role, in the order QSpec FR-490 then FR-491 list them.
    pub const ALL: [Self; 12] = [
        Self::ModelClaim,
        Self::Initialization,
        Self::TransitionSimulation,
        Self::Frame,
        Self::Environment,
        Self::NoDivergence,
        Self::FairnessCorrespondence,
        Self::ComponentGuarantee,
        Self::Rely,
        Self::LivenessDependency,
        Self::Fairness,
        Self::FixedPoint,
    ];

    /// The role's wire spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ModelClaim => "model-claim",
            Self::Initialization => "initialization",
            Self::TransitionSimulation => "transition-simulation",
            Self::Frame => "frame",
            Self::Environment => "environment",
            Self::NoDivergence => "no-divergence",
            Self::FairnessCorrespondence => "fairness-correspondence",
            Self::ComponentGuarantee => "component-guarantee",
            Self::Rely => "rely",
            Self::LivenessDependency => "liveness-dependency",
            Self::Fairness => "fairness",
            Self::FixedPoint => "fixed-point",
        }
    }

    /// The role `text` spells.
    #[ix_trace_rs::string_edge]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|role| role.as_str() == text)
    }
}

/// Whether a clause is a liveness or a safety clause (QSpec FR-363).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClauseForm {
    /// A liveness clause: it needs the progress premises.
    Liveness,
    /// A safety clause: it needs none.
    Safety,
}

/// The facts of the declaration's temporal clause `P` the planner reads.
#[derive(Clone, Debug)]
pub struct ClauseFacts {
    /// The clause's claim node.
    pub node: NodeKey,
    /// Its form.
    pub form: ClauseForm,
    /// Its resolved fairness set `F_P`.
    pub fairness: Vec<FairnessConstraint>,
    /// The first operator, in clause order, outside QSpec FR-380's
    /// stuttering-invariant fragment (an interval operator nested under an
    /// unbounded one, a previous or strong previous operator); the front end
    /// computes it. `None` when the clause lies inside the fragment.
    pub fragment_operator: Option<String>,
    /// The source region of the clause text.
    pub clause_text_span: Option<SourceRegion>,
}

/// A scheduling premise an environment member supplies: the fairness the
/// implementation executes a model operation under.
#[derive(Clone, Debug)]
pub struct SchedulingPremise {
    /// The supplied constraint, over the model operation.
    pub constraint: FairnessConstraint,
    /// The operation as the member spells it (FR-490 `context`/`operation`).
    pub operation: OperationRef,
}

/// What an environment member supplies (QSpec FR-490 "Checked-package
/// node").
#[derive(Clone, Debug)]
pub enum EnvSupplies {
    /// `fair K Q M::T::o`: a scheduling premise.
    Fairness(SchedulingPremise),
    /// `assumes N`: the `predicate` node `N`.
    Assumption(NodeKey),
}

/// One `environment E by "s" ...` member of the declaration.
#[derive(Clone, Debug)]
pub struct EnvironmentMember {
    /// The member's name `E`.
    pub name: String,
    /// The supplier `s`.
    pub supplier: String,
    /// What it supplies.
    pub supplies: EnvSupplies,
}

/// What stands behind a derived slot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SlotBacking {
    /// A request item: a filled slot carries a premise result.
    Item,
    /// An environment slot; `member` names the declaration member a record
    /// fills, and is `None` for a required premise the declaration does not
    /// list (unfilled).
    Environment {
        /// The listed member's name.
        member: Option<String>,
    },
    /// Settled by the planner itself (`fairness-correspondence`).
    PlannerLocal,
    /// A slot the declaration derives with no item (an operation with no
    /// `progress` member): unfilled, settled `absent`.
    Unfilled,
    /// A composition slot discharged from the guarantee pool or a
    /// `discharges` entry; it carries neither a result nor a record.
    Pool,
}

/// One derived premise slot, in derived order.
#[derive(Clone, Debug)]
pub struct DerivedSlot {
    /// The slot's closed role.
    pub role: SlotRole,
    /// The slot key the declaration derives (QSpec FR-492 "Slot key").
    pub key: SlotKey,
    /// What stands behind the slot.
    pub backing: SlotBacking,
    /// The `F_P` constraint of a `fairness-correspondence` slot, or of an
    /// unfilled environment slot a required scheduling premise names.
    pub constraint: Option<FairnessConstraint>,
    /// The model operation of a per-operation slot, as FR-490 keys it.
    pub operation: Option<OperationRef>,
    /// The subject a `rely` or `liveness-dependency` slot's discharging
    /// premise must be about (QSpec FR-492 admission step 7), in the
    /// canonical text of the item's `subject` member.
    pub premise_subject: Option<String>,
    /// The peer guarantee clause node a `rely` or `liveness-dependency`
    /// slot's discharging premise must claim (QSpec FR-489 "Implication").
    /// Its FR-363 form is the clause's own, so a claim of this node has the
    /// slot's form.
    pub peer_clause: Option<NodeKey>,
    /// The accepted cycle (an index the declaration numbers) a `fixed-point`
    /// slot settles, and that each `rely` and `liveness-dependency` slot
    /// whose peer guarantee lies on it belongs to; `None` for every other
    /// slot. A cycle member is discharged only by its `fixed-point` slot's
    /// proved result, never from the pool (QSpec FR-489 "Cycles").
    pub cycle: Option<usize>,
    /// The source region of the declaration member that produced the slot
    /// (QSpec FR-322 source map), carried into the planner result.
    pub source: Option<SourceRegion>,
}

/// Everything the planner reads of one declaration. The front end builds it
/// from the checked package; the planner reads no source.
#[derive(Clone, Debug)]
pub struct PlannerDeclaration {
    /// Which kind of item the declaration is.
    pub(crate) role: ItemRole,
    /// The declaration node.
    pub(crate) node: NodeKey,
    /// The checked package's identity text.
    pub(crate) package: String,
    /// A composition's rule spelling (`assume-guarantee` is the only one the
    /// planner settles); `None` for a guarantee.
    pub(crate) rule: Option<String>,
    /// The temporal clause of a guarantee; `None` for a composition.
    pub(crate) clause: Option<ClauseFacts>,
    /// Every `RustPath` of the relation and the declaration: the
    /// implementation sources the conclusion record names.
    pub(crate) relation_sources: Vec<String>,
    /// The FR-353 relation's `abstraction_relation` node (a guarantee).
    pub(crate) relation_node: Option<NodeKey>,
    /// The declaration's environment members, in member order.
    pub(crate) environment: Vec<EnvironmentMember>,
    /// The derived premise slots, in derived order.
    pub(crate) slots: Vec<DerivedSlot>,
    /// The scheduling constraints a composition supplies (its `supplies`
    /// members): what discharges a `fairness` slot and what a supplied
    /// guarantee's fairness set must be implied by. Empty for a guarantee.
    pub(crate) supplies: Vec<FairnessConstraint>,
}

impl PlannerDeclaration {
    /// Which kind of item the declaration is.
    pub const fn role(&self) -> ItemRole {
        self.role
    }

    /// The declaration node.
    pub const fn node(&self) -> &NodeKey {
        &self.node
    }

    /// The checked package's identity text.
    pub fn package(&self) -> &str {
        &self.package
    }

    /// The declaration's environment members, in member order.
    pub fn environment(&self) -> &[EnvironmentMember] {
        &self.environment
    }

    /// The derived premise slots, in derived order.
    pub fn slots(&self) -> &[DerivedSlot] {
        &self.slots
    }
}

impl EnvSupplies {
    /// Whether this environment member supplies a scheduling premise that
    /// implies `needed` on `operation` under QSpec FR-380 "Fairness
    /// implication": the same operation, a `strong` supplied constraint
    /// implies a `weak` or `strong` one and a `weak` one only a `weak` one,
    /// and the granularity is equal or the needed constraint is the `whole`
    /// form of a supplied `each` constraint.
    pub(crate) fn implies(&self, operation: &OperationRef, needed: &FairnessConstraint) -> bool {
        let Self::Fairness(premise) = self else {
            return false;
        };
        let supplied = &premise.constraint;
        premise.operation == *operation
            && strength_implies(
                (supplied.kind(), supplied.granularity()),
                (needed.kind(), needed.granularity()),
            )
    }
}

/// QSpec FR-380 "Fairness implication", the kind and granularity half: a
/// `strong` supplied constraint implies a `weak` or `strong` one and a `weak`
/// one only a `weak` one, and the granularity is equal or the needed
/// constraint is the `whole` form of a supplied `each` constraint. The
/// operation match is the caller's.
pub(crate) fn strength_implies(
    (supplied_kind, supplied_granularity): (FairnessKind, FairnessGranularity),
    (needed_kind, needed_granularity): (FairnessKind, FairnessGranularity),
) -> bool {
    let kind_ok = supplied_kind == FairnessKind::Strong || needed_kind == FairnessKind::Weak;
    let granularity_ok = supplied_granularity == needed_granularity
        || (needed_granularity == FairnessGranularity::Whole
            && supplied_granularity == FairnessGranularity::Each);
    kind_ok && granularity_ok
}
