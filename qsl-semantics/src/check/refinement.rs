// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-135, FR-136, FR-137: the S3 check of a refinement declaration between
//! two state models (ADR-020 §1 RM-1 to RM-6).
//!
//! The assembler resolves a declaration's `using` alias into a
//! [`RefinementDeclaration`]; [`check_refinement`] resolves everything else
//! against the unit's admitted models: both model aliases, the population
//! map, the object map with its field rows, one step row per concrete
//! operation and the two fairness sets. It reports every refusal of the
//! declaration, in source order within each part, and yields a
//! [`CheckedRefinement`] only when there is none.
//!
//! A row expression is typed as a state-clause body is, with `self` bound to
//! the concrete object, and checked against the abstract type it defines
//! after reference lifting: a reference to an abstract object type that an
//! object row maps denotes the concrete object of the same key, so the
//! abstract type is read as a reference to the concrete type it is mapped
//! from.
//!
//! A row may also read a concrete population whole (FR-139, ADR-020 RM-8):
//! the population's name, and the `count`, `sum`, `exists`, `all`, `only` and
//! `seq` forms over it. S3 types them with the population's universe size `n`
//! symbolic. A row whose value is a population name, a `count` or a `sum`
//! records the bound that depends on `n` ([`PopulationUse`]), and
//! [`CheckedRefinement::admit_universes`] checks it against the type of the
//! row's use once a subject supplies the universes. A bound that depends on
//! `n` through any other form is refused as not yet checked.

mod history;
mod internal_list;
mod protocol;
mod step_label;

use std::collections::BTreeMap;

pub use history::{HistoryRow, HistoryUpdate};
pub use internal_list::{AbstractStepClass, AbstractStepKind, ListDefect, ListDefectKind};
pub(crate) use internal_list::{NodeClass, ParsedLists, ProtocolClasses};
pub use protocol::{ChannelStepKind, MemoryStepKind, ProtocolStepKind, RoleStepKind};
pub(crate) use protocol::{ProtocolClause, ProtocolRefinement, SelectedProtocol};
use qsl_forms::{
    AbstractSideForm, ArgumentForm, ExpressionForm, FairnessConstraintForm,
    FairnessGranularity as FormGranularity, FairnessKind as FormKind, FairnessSide,
    ProtocolDeclarationForm, ProtocolNodeKind, RefinementForm, RefinementRowForm, StepRowForm,
    StepSourceForm, StepTargetForm,
};
use qsl_foundation::selection::ProfileSelection;
use qsl_foundation::{Code, InternalFault, Span};
use quire_exact::{
    CardinalityBound, CollectionKind, CollectionType, EffectiveId, IllTypedCause, Integer,
    IntegerInterval, NodeKey, Presence, ValueType,
};
use quire_semantic_value::declaration::TypeEnvironment;
use quire_semantic_value::fairness::{
    FairnessConstraint, FairnessGranularity, FairnessKind, FairnessOperation,
};
use quire_semantic_value::location::{Location, Origin};
pub use step_label::{observation, LabelStep, RowObservation, StepLabel};

use super::check::{
    bind_parameters, PopulationScope, ScopedHistory, ScopedPopulation, StateContext, Typer,
};
use super::ir::{static_integer_range, CheckedBody, CheckedNode, NodeKind, Visit};
use super::lowering::AdmittedModel;
use super::observation::Observations;
use super::profile::TemporalProfile;
use super::protocol_clause::ProtocolNodeId;
use super::refusal::{CheckCause, CheckRefusal, KeyFault};
use super::state_clause::{observation_of, ClauseDeclarations, ClauseOperation};
use crate::family::CheckContext;
use crate::model::clause_observation::Observation;
use crate::model::domain_package::PopulationRecord;
use crate::model::intake::type_identity_segment;
use crate::model::key::DeclarationKey;
use crate::model::operation::OperationLookup;

/// One `refinement` declaration the assembler resolved: its form and the
/// profile selection its `using` alias resolved to.
#[derive(Clone, Debug)]
pub struct RefinementDeclaration {
    /// The declaration as S2 built it.
    pub form: RefinementForm,
    /// The profile selection the `using` alias resolved to.
    pub selection: ProfileSelection,
}

/// Which kind of row a missing row would have been.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RowClass {
    /// A `population` row for an abstract population.
    Population,
    /// An `object` row for an abstract object type.
    Object,
    /// A `step` row for a concrete operation.
    Step,
    /// A `step` row for a protocol node: an `event`, `send`, `receive`,
    /// `timeout` or `finish` node, an attempt node no operation row covers,
    /// or a `parallel`'s `fork` or `join` (FR-136).
    Node,
    /// A `step` row for a compensation template's `cattempt` or `cend`.
    Template,
    /// A `step` row for a channel's `duplicate` or `lose`.
    Channel,
    /// A `step` row for a replicated role's `spawn` or `retire`.
    Role,
    /// A `step` row for the `activation on each` trigger.
    Trigger,
    /// A `step` row for a memory step kind.
    Memory,
}

/// A spelling of a refinement declaration that S3 has no checker for yet.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum UnsupportedFeature {
    /// An abstract protocol side (FR-147, QSpec FR-375's
    /// `protocol-abstract-side`).
    ProtocolAbstractSide,
    /// A non-empty resolved internal list (FR-147, QSpec FR-375's
    /// `refinement-internal-list`).
    RefinementInternalList,
    /// A history row whose declared type has values with no FR-181 typed
    /// canonical form (an enum or a decimal), which a product state could not
    /// key (FR-138).
    HistoryRow,
    /// An abstract-operation application as the right side of a protocol
    /// step row: FR-136 binds `self` to the concrete receiver, and a
    /// protocol step has none that the requirement names.
    ProtocolStepApplication,
}

impl UnsupportedFeature {
    /// The feature's name in a refusal.
    pub fn name(self) -> &'static str {
        match self {
            Self::ProtocolAbstractSide => "protocol-abstract-side",
            Self::RefinementInternalList => "refinement-internal-list",
            Self::HistoryRow => "history-row",
            Self::ProtocolStepApplication => "protocol-step-application",
        }
    }
}

/// The refinement features this build has built (FR-135 "Refinement
/// features the build does not check"): a build capability and not a request
/// parameter. A declaration that uses a feature outside the set is refused
/// `unsupported_construct`/`not-yet-implemented`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RefinementSupport {
    /// `protocol-abstract-side`: `abstract <A>::<protocol>` (FR-147).
    pub protocol_abstract_side: bool,
    /// `refinement-internal-list`: a non-empty resolved internal list
    /// (FR-147).
    pub refinement_internal_list: bool,
}

impl RefinementSupport {
    /// What this build has built: both features of FR-147's S3 half.
    pub const BUILD: Self = Self {
        protocol_abstract_side: true,
        refinement_internal_list: true,
    };
}

impl Default for RefinementSupport {
    fn default() -> Self {
        Self::BUILD
    }
}

/// Why a refinement declaration was refused.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum RefinementCause {
    /// The `using` alias does not select the infinite-trace profile:
    /// `unknown_profile`/`wrong-selection-role`.
    WrongProfile {
        /// The identity the alias selected.
        selected: String,
    },
    /// A name resolves to no declaration of the model it must name:
    /// `missing_declaration`/`missing-name`.
    Unresolved {
        /// The name as written.
        name: String,
        /// The model the name belongs to, when it names a declaration of
        /// the other model of the refinement.
        held_by: Option<String>,
    },
    /// A name resolves to several operations, none of whose declaring types
    /// is more derived than every other: `ambiguous_declaration`/`ambiguous-name`.
    Ambiguous {
        /// The name as written.
        name: String,
    },
    /// A population, object type or concrete operation has no row:
    /// `missing_declaration`/`missing-name`.
    MissingRow {
        /// The kind of row.
        class: RowClass,
        /// The population, object type or operation, as qualified names
        /// spell it.
        name: String,
    },
    /// Two rows define one population, object type, field or operation:
    /// `invalid_model_binding`/`conflicting-binding`.
    Conflicting {
        /// The doubly defined name.
        name: String,
        /// The earlier row.
        first: Span,
    },
    /// A row names a declaration that cannot take its place:
    /// `invalid_model_binding`/`malformed-declaration`.
    Malformed {
        /// The name as written.
        name: String,
    },
    /// A row or header spelling the S2 form carries but S3 has no checker
    /// for yet: `unsupported_construct`/`not-yet-implemented` (FR-341-AC-5,
    /// which the spine classifies `unsupported`). A declaration holding one
    /// is refused rather than checked with the row ignored.
    Unsupported {
        /// The feature the declaration spells.
        feature: UnsupportedFeature,
        /// The identity of the profile the declaration selects.
        profile: String,
        /// The kind of type or form the feature is refused for, when the
        /// feature alone does not say.
        subject: Option<&'static str>,
    },
    /// A defect of the unit's `refinement_internals` value, located by the
    /// JSON Pointer inside it and not by a source span (FR-147 "Defects").
    InternalList(ListDefect),
    /// A class the internal list names that a step row's right side also
    /// names: `invalid_model_binding`/`conflicting-binding`, at the row and
    /// naming the entry (FR-147 "One class, one decision").
    ListEntryConflict {
        /// The class, by its alias-free spelling.
        class: String,
        /// The pointer of the list entry that names it.
        entry: String,
    },
    /// A population-valued row whose bound does not fit the type it is used
    /// as once the subject's universe fixes the population's size:
    /// `ill_typed`/`type-mismatch` (FR-139).
    PopulationBound {
        /// The row.
        row: String,
        /// The population, as the row writes it.
        population: String,
        /// The population's universe size.
        size: u64,
    },
    /// A row's expression or type refused.
    Check(CheckCause),
}

/// A refusal of a refinement declaration, at the span of the row or name it
/// is about.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RefinementRefusal {
    /// The declared name of the refinement.
    pub refinement: String,
    /// The span of the row, name or expression the refusal is about.
    pub span: Span,
    /// Why.
    pub cause: RefinementCause,
}

impl RefinementCause {
    /// The refusal code.
    pub fn code(&self) -> Code {
        match self {
            Self::WrongProfile { .. } => Code::UnknownProfile,
            Self::Unresolved { .. } | Self::MissingRow { .. } => Code::MissingDeclaration,
            Self::Ambiguous { .. } => Code::AmbiguousDeclaration,
            Self::Conflicting { .. } | Self::Malformed { .. } | Self::ListEntryConflict { .. } => {
                Code::InvalidModelBinding
            }
            Self::InternalList(defect) => match defect.kind {
                ListDefectKind::Missing => Code::MissingDeclaration,
                ListDefectKind::Malformed | ListDefectKind::Conflicting => {
                    Code::InvalidModelBinding
                }
            },
            Self::Unsupported { .. } => Code::UnsupportedConstruct,
            Self::PopulationBound { .. } => Code::IllTyped,
            Self::Check(cause) => cause.code(),
        }
    }

    /// The closed cause tag.
    pub fn cause(&self) -> Option<&'static str> {
        match self {
            Self::WrongProfile { .. } => Some("wrong-selection-role"),
            Self::Unresolved { .. } | Self::MissingRow { .. } => Some("missing-name"),
            Self::Ambiguous { .. } => Some("ambiguous-name"),
            Self::Conflicting { .. } | Self::ListEntryConflict { .. } => {
                Some("conflicting-binding")
            }
            Self::InternalList(defect) => Some(match defect.kind {
                ListDefectKind::Missing => "missing-name",
                ListDefectKind::Malformed => "malformed-declaration",
                ListDefectKind::Conflicting => "conflicting-binding",
            }),
            Self::Malformed { .. } => Some("malformed-declaration"),
            Self::Unsupported { .. } => Some("not-yet-implemented"),
            Self::PopulationBound { .. } => Some("type-mismatch"),
            Self::Check(cause) => cause.cause(),
        }
    }
}

/// The checked body of one row expression: its parameters (`self`, then a
/// step's operation parameters), the body and the evaluation slots it needs.
#[derive(Debug)]
pub struct CheckedRow {
    pub(crate) parameters: Vec<(String, ValueType)>,
    pub(crate) body: CheckedBody,
    pub(crate) slots: usize,
    pub(crate) uses: Vec<PopulationUse>,
    /// The observation each model read of the body takes, by the clause kind
    /// the row is typed as (FR-104). A history update reads its post-state
    /// fields at `post` and `pre(..)` operands at `pre`.
    pub(crate) observations: Observations,
}

impl CheckedRow {
    /// Every model read of the body and the observation it takes, by the
    /// read's location.
    pub fn reads(&self) -> impl Iterator<Item = (&Location, Observation)> {
        self.observations.iter()
    }

    /// The parameters the body is checked over, `self` first.
    pub fn parameters(&self) -> &[(String, ValueType)] {
        &self.parameters
    }

    /// The checked body.
    pub fn body(&self) -> CheckedNode<'_> {
        self.body.root()
    }

    /// The number of evaluation slots of the body.
    pub fn slots(&self) -> usize {
        self.slots
    }

    /// The bounds of this row that depend on a population's universe size,
    /// which a subject's admission checks (FR-139).
    pub fn population_uses(&self) -> &[PopulationUse] {
        &self.uses
    }
}

/// The population-size-dependent value a row is (FR-139, ADR-020 RM-8).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PopulationValue {
    /// A concrete population name: `Set<Reference<T>>[0, n]`.
    Objects,
    /// `count(x in P where c)`: `Int[0, n]`.
    Count,
    /// `sum(x in P where c: e)`: the sum of `e`, which has this range, over
    /// at most `n` objects, and 0 over none.
    Sum {
        /// The range of `e`.
        summand: IntegerInterval,
    },
}

/// A row whose result is a population-size-dependent value (FR-139).
/// S3 checks every typing rule that does not depend on the universe size
/// `n` and records the bound that does, against the type of the row's use;
/// [`CheckedRefinement::admit_universes`] checks it once a subject supplies
/// the universes.
#[derive(Clone, Debug)]
pub struct PopulationUse {
    pub(crate) row: String,
    pub(crate) span: Span,
    pub(crate) population: String,
    pub(crate) key: DeclarationKey,
    pub(crate) member: EffectiveId,
    pub(crate) value: PopulationValue,
    pub(crate) required: ValueType,
}

impl PopulationUse {
    /// The row, as `Type.field` or `step operation`.
    pub fn row(&self) -> &str {
        &self.row
    }

    /// The population, as the row writes it.
    pub fn population(&self) -> &str {
        &self.population
    }

    /// The population's declaration.
    pub fn population_key(&self) -> &DeclarationKey {
        &self.key
    }

    /// Which value the row is.
    pub fn value(&self) -> &PopulationValue {
        &self.value
    }

    /// The type the row's value is used as.
    pub fn required(&self) -> &ValueType {
        &self.required
    }

    /// The row value's type once the population's universe has `size`
    /// objects, or `None` when that bound is not representable.
    pub fn admitted_type(&self, size: u64) -> Option<ValueType> {
        let count = Integer::from(size);
        match &self.value {
            PopulationValue::Objects => Some(ValueType::collection(CollectionType::new(
                CollectionKind::Set,
                ValueType::Reference(self.member),
                Some(CardinalityBound::new(0, size).ok()?),
            ))),
            PopulationValue::Count => Some(ValueType::Int(
                IntegerInterval::new(Integer::zero(), count).ok()?,
            )),
            PopulationValue::Sum { summand } => {
                let lower = summand.lower().mul(&count);
                let upper = summand.upper().mul(&count);
                let zero = Integer::zero();
                Some(ValueType::Int(
                    IntegerInterval::new(
                        if lower < zero { lower } else { zero.clone() },
                        if upper > zero { upper } else { zero },
                    )
                    .ok()?,
                ))
            }
        }
    }

    /// Whether the row's value fits the type it is used as when the
    /// population has `size` objects.
    fn fits_at(&self, size: u64) -> bool {
        let Some(actual) = self.admitted_type(size) else {
            return false;
        };
        match (&actual, &self.required) {
            (ValueType::Int(_), ValueType::Integer) => true,
            (ValueType::Int(source), ValueType::Int(target)) => {
                target.lower() <= source.lower() && source.upper() <= target.upper()
            }
            // `Set<Reference<T>>[0, n]` fits a set of the same element whose
            // bound contains `[0, n]`, or has none.
            (ValueType::Collection(source), ValueType::Collection(target)) => {
                source.kind() == target.kind()
                    && source.element() == target.element()
                    && match (source.bound(), target.bound()) {
                        (_, None) => true,
                        (Some(source), Some(target)) => {
                            target.minimum() <= source.minimum()
                                && source.maximum() <= target.maximum()
                        }
                        (None, Some(_)) => false,
                    }
            }
            _ => actual == self.required,
        }
    }
}

impl CheckedRefinement {
    /// FR-139: checks the bounds that depend on a population's universe
    /// size, once a request admits the concrete subject's `universes` and
    /// before exploration. Each `universes` entry is the number of objects of
    /// one concrete population. A row whose value, with that size
    /// substituted for `n`, does not fit the type it is used as is refused
    /// `ill_typed`/`type-mismatch`, naming the row, the population and its
    /// size; a population with no entry is refused `missing_declaration`/
    /// `missing-name`.
    ///
    /// # Errors
    ///
    /// Every refusal, in row order.
    pub fn admit_universes(
        &self,
        universes: &BTreeMap<DeclarationKey, u64>,
    ) -> Result<(), Vec<RefinementRefusal>> {
        let rows = self
            .objects
            .iter()
            .flat_map(|object| object.fields.iter().map(|field| &field.expression))
            .chain(self.steps.iter().flat_map(|step| {
                match &step.target {
                    StepTarget::Abstract {
                        receiver,
                        arguments,
                        ..
                    } => std::iter::once(receiver)
                        .chain(arguments.iter().flatten())
                        .collect::<Vec<_>>(),
                    StepTarget::Node { arguments, .. } => {
                        arguments.iter().flatten().collect::<Vec<_>>()
                    }
                    StepTarget::Stutter | StepTarget::Any => Vec::new(),
                }
            }));
        let mut refusals = Vec::new();
        for row in rows {
            for used in &row.uses {
                let cause = match universes.get(&used.key) {
                    None => RefinementCause::Unresolved {
                        name: used.population.clone(),
                        held_by: None,
                    },
                    Some(size) if used.fits_at(*size) => continue,
                    Some(size) => RefinementCause::PopulationBound {
                        row: used.row.clone(),
                        population: used.population.clone(),
                        size: *size,
                    },
                };
                refusals.push(RefinementRefusal {
                    refinement: self.name.clone(),
                    span: used.span,
                    cause,
                });
            }
        }
        if refusals.is_empty() {
            Ok(())
        } else {
            Err(refusals)
        }
    }
}

/// `population A::p from C::q`, resolved.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PopulationRow {
    /// The abstract population.
    pub abstract_population: DeclarationKey,
    /// The concrete population it is the key-preserving image of.
    pub concrete_population: DeclarationKey,
}

/// `field = e` of an object row, resolved.
#[derive(Debug)]
pub struct FieldRow {
    /// The abstract field.
    pub field: DeclarationKey,
    /// Its definition over the concrete object.
    pub expression: CheckedRow,
}

/// `object A::T from C::U { ... }`, resolved.
#[derive(Debug)]
pub struct ObjectRow {
    /// The abstract object type.
    pub abstract_type: DeclarationKey,
    /// The concrete object type it is mapped from.
    pub concrete_type: DeclarationKey,
    /// The visible fields, in source order.
    pub fields: Vec<FieldRow>,
    /// The abstract fields no row defines (ADR-020 AX-2).
    pub hidden: Vec<DeclarationKey>,
}

/// The concrete step a `step` row maps (FR-136 Outputs).
///
/// FR-383's minting of the protocol nodes' keys is not built, so a class
/// names a node by its identity in the clause, and a template, channel or
/// role by its declared name, where FR-136 has `NodeKey`s. They become
/// `NodeKey`s with the sort key (FR-136-AC-7, FR-146-AC-5).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StepClass {
    /// A concrete operation.
    Operation(DeclarationKey),
    /// An `attempt` node, whose row takes precedence over its operation's.
    AttemptNode(ProtocolNodeId),
    /// A compensation template's `cattempt`, written as the template's name.
    CompensationAttempt(String),
    /// A compensation template's `cend`.
    CompensationEnd(String),
    /// A protocol node's step.
    ProtocolNode {
        /// Which step of the node.
        kind: ProtocolStepKind,
        /// The node.
        node: ProtocolNodeId,
    },
    /// A channel's `duplicate` or `lose`.
    Channel {
        /// Which step.
        kind: ChannelStepKind,
        /// The channel's declared name.
        channel: String,
    },
    /// A replicated role's `spawn` or `retire`.
    Role {
        /// Which step.
        kind: RoleStepKind,
        /// The role's declared name.
        role: String,
    },
    /// The `activation on each` trigger's `activate`.
    Trigger,
    /// A step of the memory model of an outermost `parallel`.
    MemoryStep(MemoryStepKind),
}

/// The right side of a `step` row, resolved.
#[derive(Debug)]
pub enum StepTarget {
    /// An abstract operation applied to a receiver and its arguments.
    Abstract {
        /// The abstract operation.
        operation: DeclarationKey,
        /// The receiver, over the concrete step.
        receiver: CheckedRow,
        /// One entry per abstract parameter; `None` is `_`.
        arguments: Vec<Option<CheckedRow>>,
        /// Whether both operations declare a result, which the abstract
        /// result is then bound to the concrete one for.
        binds_result: bool,
    },
    /// `node <A>::<protocol>::<node>(argument, ...)`: an `attempt`, `event`,
    /// `send`, `receive` or `finish` node of the abstract protocol, with one
    /// entry per argument written (FR-147 "Step rows and observations").
    Node {
        /// The abstract protocol clause.
        protocol: String,
        /// The abstract node.
        node: String,
        /// The expressions over the concrete step, in source order; `None`
        /// is `_`.
        arguments: Vec<Option<CheckedRow>>,
    },
    /// `stutter`.
    Stutter,
    /// `any`.
    Any,
}

/// One `step` row, resolved.
#[derive(Debug)]
pub struct StepRow {
    /// The concrete step.
    pub class: StepClass,
    /// What it maps to.
    pub target: StepTarget,
}

/// A refinement between two models that checked (ADR-020 RM-1 to RM-6).
#[derive(Debug)]
pub struct CheckedRefinement {
    pub(crate) name: String,
    pub(crate) profile: ProfileSelection,
    pub(crate) abstract_model: String,
    pub(crate) abstract_side: AbstractSide,
    pub(crate) abstract_internal: Vec<AbstractStepClass>,
    pub(crate) concrete_model: String,
    pub(crate) populations: Vec<PopulationRow>,
    pub(crate) objects: Vec<ObjectRow>,
    pub(crate) steps: Vec<StepRow>,
    pub(crate) history: Vec<HistoryRow>,
    pub(crate) concrete_fairness: Vec<FairnessConstraint>,
    pub(crate) abstract_fairness: Vec<FairnessConstraint>,
    pub(crate) concrete_object_types: Vec<EffectiveId>,
    /// The protocol clause the step rows select; absent for a model subject.
    /// The clause's node exists once the unit's protocol nodes are minted
    /// (FR-092), so the pipeline fills it after the check.
    pub(crate) concrete_protocol: Option<NodeKey>,
}

impl CheckedRefinement {
    /// The declared name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The infinite-trace profile selection.
    pub fn profile(&self) -> &ProfileSelection {
        &self.profile
    }

    /// The abstract model's alias.
    pub fn abstract_model(&self) -> &str {
        &self.abstract_model
    }

    /// The abstract subject: the model, or a protocol clause of it (FR-147).
    pub fn abstract_side(&self) -> &AbstractSide {
        &self.abstract_side
    }

    /// The abstract step classes the refinement's internal list makes
    /// internal, in ascending UTF-8 byte order of their alias-free
    /// spellings; empty for a model subject and for an absent list
    /// (FR-147 "Outputs").
    pub fn abstract_internal(&self) -> &[AbstractStepClass] {
        &self.abstract_internal
    }

    /// The concrete model's alias, the declaring package's.
    pub fn concrete_model(&self) -> &str {
        &self.concrete_model
    }

    /// The population rows, in source order.
    pub fn populations(&self) -> &[PopulationRow] {
        &self.populations
    }

    /// The object rows, in source order.
    pub fn objects(&self) -> &[ObjectRow] {
        &self.objects
    }

    /// The step rows, in source order.
    pub fn steps(&self) -> &[StepRow] {
        &self.steps
    }

    /// The protocol clause the step rows select (FR-136 "Selecting the
    /// concrete protocol"); `None` for a model subject.
    pub fn concrete_protocol(&self) -> Option<NodeKey> {
        self.concrete_protocol
    }

    /// The history rows, in source order (FR-138).
    pub fn history(&self) -> &[HistoryRow] {
        &self.history
    }

    /// The concrete fairness set `F_C`: every `assume` row once, in source
    /// order.
    pub fn concrete_fairness(&self) -> &[FairnessConstraint] {
        &self.concrete_fairness
    }

    /// The abstract fairness set `F_A`: every `ensure` row once, in source
    /// order.
    pub fn abstract_fairness(&self) -> &[FairnessConstraint] {
        &self.abstract_fairness
    }

    /// Every object type of the concrete model, mapped by an object row or
    /// not: the contexts of the invariant clauses the concrete package
    /// holds (FR-146).
    pub fn concrete_object_types(&self) -> &[EffectiveId] {
        &self.concrete_object_types
    }
}

/// One model of the refinement and the alias the unit gives it.
struct Side<'a> {
    alias: &'a str,
    model: &'a AdmittedModel,
}

/// The abstract subject of a checked refinement (FR-135, FR-147).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AbstractSide {
    /// `abstract A`: the abstract model.
    Model,
    /// `abstract A::protocol`: a checked protocol clause of the abstract
    /// model.
    Protocol {
        /// The protocol clause's declared name.
        clause: String,
    },
}

/// One population of a model, with the name a row spells it by.
struct Population<'a> {
    name: String,
    record: &'a PopulationRecord,
}

/// The names a row spells the concrete populations of `models` by, whether or
/// not the clause they are written in is a refinement's (FR-139).
pub(crate) fn population_names(models: &[AdmittedModel]) -> Vec<String> {
    models
        .iter()
        .filter_map(|model| {
            let alias = model.alias()?;
            Some(
                model
                    .population_domains()
                    .filter_map(move |(_, record, _)| {
                        let segment = type_identity_segment(&record.key.package, &record.key.node)?;
                        Some(format!("{alias}::{segment}"))
                    }),
            )
        })
        .flatten()
        .collect()
}

/// The names of the history fields `declarations` write (FR-138), which no
/// clause but the declaring refinement's rows may read.
pub(crate) fn history_names(declarations: &[RefinementDeclaration]) -> Vec<String> {
    declarations
        .iter()
        .flat_map(|declaration| &declaration.form.rows)
        .filter_map(|row| match row {
            RefinementRowForm::History(row) => Some(row.field.name.clone()),
            _ => None,
        })
        .collect()
}

/// One object type of a model.
struct ObjectType {
    name: String,
    id: EffectiveId,
    key: DeclarationKey,
}

impl<'a> Side<'a> {
    /// Every population of the model, or the invariant an admitted
    /// population breaks by having no artifact name.
    fn populations(&self) -> Result<Vec<Population<'a>>, &'static str> {
        self.model
            .population_domains()
            .map(|(_, record, _)| {
                let segment = type_identity_segment(&record.key.package, &record.key.node)
                    .ok_or("population-identity-is-an-artifact-name")?;
                Ok(Population {
                    name: format!("{}::{segment}", self.alias),
                    record,
                })
            })
            .collect()
    }

    /// Every object type of the model, or the invariant an admitted object
    /// type breaks by being absent from the type environment.
    fn object_types(&self, types: &TypeEnvironment) -> Result<Vec<ObjectType>, &'static str> {
        self.model
            .object_types()
            .map(|(key, id)| {
                Ok(ObjectType {
                    name: types
                        .object_type(id)
                        .ok_or("admitted-object-type-is-in-the-environment")?
                        .name()
                        .to_owned(),
                    id,
                    key: key.clone(),
                })
            })
            .collect()
    }
}

/// The refusals found in one declaration, in the order they were found.
struct Findings {
    refinement: String,
    refusals: Vec<CheckRefusal>,
}

impl Findings {
    fn refuse(&mut self, span: Span, cause: RefinementCause) {
        self.refusals.push(CheckRefusal {
            location: Location::root(Origin::Expression),
            cause: CheckCause::Refinement(Box::new(RefinementRefusal {
                refinement: self.refinement.clone(),
                span,
                cause,
            })),
        });
    }

    /// A defect of the internal list, located inside its value and not by a
    /// source span.
    fn list_defect(&mut self, defect: ListDefect) {
        self.refuse(
            Span { start: 0, end: 0 },
            RefinementCause::InternalList(defect),
        );
    }

    fn unresolved(&mut self, name: &str, span: Span, held_by: Option<&str>) {
        self.refuse(
            span,
            RefinementCause::Unresolved {
                name: name.to_owned(),
                held_by: held_by.map(str::to_owned),
            },
        );
    }

    fn malformed(&mut self, name: &str, span: Span) {
        self.refuse(
            span,
            RefinementCause::Malformed {
                name: name.to_owned(),
            },
        );
    }

    fn fault(&mut self, invariant: &'static str) {
        self.refusals.push(CheckRefusal {
            location: Location::root(Origin::Expression),
            cause: CheckCause::InternalFault(Box::new(KeyFault::StageFault(InternalFault::new(
                "check.refinement",
                invariant,
            )))),
        });
    }
}

/// A qualified name split at its `::` separators, with the span of each
/// segment.
struct NameSegments<'n> {
    parts: Vec<&'n str>,
    span: Span,
    /// Whether the name's span covers exactly its spelling, so that a
    /// segment's span follows from its offset; with trivia between the
    /// tokens a segment is located by the whole name.
    exact: bool,
}

impl<'n> NameSegments<'n> {
    fn of(name: &'n qsl_forms::NameForm) -> Self {
        Self {
            parts: name.name.split("::").collect(),
            span: name.span,
            exact: name.span.end - name.span.start == name.name.len(),
        }
    }

    fn parts(&self) -> &[&'n str] {
        &self.parts
    }

    /// The span of the segment at `index`.
    fn span(&self, index: usize) -> Span {
        if !self.exact {
            return self.span;
        }
        let offset: usize = self.parts[..index].iter().map(|part| part.len() + 2).sum();
        Span {
            start: self.span.start + offset,
            end: self.span.start + offset + self.parts[index].len(),
        }
    }
}

/// The object type name and member of `Alias::Type::member`.
fn split_member(name: &str) -> Option<(&str, &str)> {
    name.rsplit_once("::")
}

/// Whether a value of `actual` fits a slot of `required` (ADR-016 SC-2): the
/// same type, an integer in a containing integer range, an object reference
/// to a subtype, or a record to a record it specializes (FR-359). An `Option`
/// or collection type is invariant in its payload, as FR-359 states: a type's
/// identity includes its element type, so `Option<S>` or `Sequence<S>` is not
/// an `Option<P>` or `Sequence<P>` for a child `S` of `P`.
fn conforms(types: &TypeEnvironment, actual: &ValueType, required: &ValueType) -> bool {
    match (actual, required) {
        (ValueType::Integer | ValueType::Int(_), ValueType::Integer) => true,
        (ValueType::Int(source), ValueType::Int(target)) => {
            target.lower() <= source.lower() && source.upper() <= target.upper()
        }
        (ValueType::Reference(sub), ValueType::Reference(sup)) => types.conforms(*sub, *sup),
        (ValueType::Composite(sub), ValueType::Composite(sup)) => {
            types.composite_conforms(*sub, *sup)
        }
        _ => actual == required,
    }
}

/// `value_type` with each reference to an abstract object type that an
/// object row maps replaced by a reference to the concrete type it is mapped
/// from (ADR-020 RM-3 reference lifting).
fn lift(lifting: &BTreeMap<EffectiveId, EffectiveId>, value_type: &ValueType) -> ValueType {
    match value_type {
        ValueType::Reference(object) => {
            ValueType::Reference(lifting.get(object).copied().unwrap_or(*object))
        }
        ValueType::Option(payload) => ValueType::option(lift(lifting, payload)),
        ValueType::Collection(collection) => {
            ValueType::collection(quire_exact::CollectionType::new(
                collection.kind(),
                lift(lifting, collection.element()),
                collection.bound(),
            ))
        }
        other => other.clone(),
    }
}

/// What the check of one refinement declaration found.
pub(crate) enum RefinementCheck<'f> {
    /// Every part checked.
    Checked(Box<CheckedRefinement>),
    /// The rows select a protocol clause: everything that depends on the
    /// models only is checked, and the rest waits for the checked clause
    /// (FR-136 "Order of the checks").
    Protocol(Box<ProtocolRefinement<'f>>),
    /// Every refusal of the declaration.
    Refused(Vec<CheckRefusal>),
}

/// Checks the refinement `declaration` against the unit's models and the
/// names of its `protocols`, and returns what it found with the package's
/// running expression-node total after it.
pub(crate) fn check_refinement<'f: 'c, 'a, 'c>(
    declaration: &'f RefinementDeclaration,
    protocols: &'c [ProtocolDeclarationForm],
    declarations: &'c ClauseDeclarations<'a>,
    cx: &mut CheckContext<'a, ClauseDeclarations<'a>>,
    env: &RefinementEnv<'_>,
) -> (RefinementCheck<'f>, u64) {
    let form = &declaration.form;
    let mut findings = Findings {
        refinement: form.name.name.clone(),
        refusals: Vec::new(),
    };
    let mut nodes = declarations.nodes_used;
    if TemporalProfile::from_identity(&declaration.selection.identity)
        != Some(TemporalProfile::InfiniteTrace)
    {
        findings.refuse(
            form.profile.span,
            RefinementCause::WrongProfile {
                selected: declaration.selection.identity.clone(),
            },
        );
    }
    let abstract_alias = match &form.abstract_side {
        AbstractSideForm::Model(alias) => alias,
        AbstractSideForm::Protocol { alias, clause } => {
            if !env.support.protocol_abstract_side {
                // FR-147: a build that has not built the protocol side
                // refuses it before any check of the side, and still checks
                // the rest of the declaration.
                findings.refuse(
                    clause.span,
                    RefinementCause::Unsupported {
                        feature: UnsupportedFeature::ProtocolAbstractSide,
                        profile: declaration.selection.identity.clone(),
                        subject: None,
                    },
                );
            }
            alias
        }
    };
    let side = |alias: &'c qsl_forms::DeclaredName| {
        declarations
            .models
            .iter()
            .find(|model| model.alias() == Some(alias.name.as_str()))
            .map(|model| Side {
                alias: &alias.name,
                model,
            })
    };
    let (Some(abstract_side), Some(concrete_side)) = (side(abstract_alias), side(&form.concrete))
    else {
        for alias in [abstract_alias, &form.concrete] {
            if side(alias).is_none() {
                findings.unresolved(&alias.name, alias.span, None);
            }
        }
        return (RefinementCheck::Refused(findings.refusals), nodes);
    };
    let subject = match &form.abstract_side {
        AbstractSideForm::Model(_) => ListSubject::Model,
        AbstractSideForm::Protocol { clause, .. } => resolve_protocol(
            &mut findings,
            env,
            clause,
            &abstract_side,
            declarations.scope.types(),
        ),
    };
    let internal = internal_list(
        &mut findings,
        env,
        form,
        &declaration.selection.identity,
        &subject,
    );
    let clause_refused = matches!(subject, ListSubject::Refused);
    let mut checker = Checker {
        declarations,
        protocols,
        abstract_side,
        concrete_side,
        findings,
        nodes: &mut nodes,
        lifting: BTreeMap::new(),
        maps_complete: true,
        baseline: 0,
        populations: None,
        history: Vec::new(),
        protocol_subject: matches!(form.abstract_side, AbstractSideForm::Protocol { .. }),
        protocol: match subject {
            ListSubject::Protocol(classes) => Some(*classes),
            _ => None,
        },
        internal,
    };
    checker.baseline = checker.findings.refusals.len();
    let headers = checker.history_headers(form, &declaration.selection.identity);
    let populations = checker.populations(form);
    let objects = checker.objects(form, cx);
    let StepRows {
        operations,
        written,
        protocol,
    } = checker.steps(form, cx);
    let history = checker.history_rows(headers, cx);
    let concrete_fairness = checker.fairness(form, FairnessSide::Assume);
    let abstract_fairness = checker.fairness(form, FairnessSide::Ensure);
    let concrete_object_types = checker
        .concrete_side
        .model
        .object_types()
        .map(|(_, id)| id)
        .collect();
    let Checker {
        findings,
        lifting,
        maps_complete,
        history: scoped_history,
        internal,
        protocol_subject,
        protocol: abstract_classes,
        ..
    } = checker;
    let checked = CheckedRefinement {
        name: form.name.name.clone(),
        profile: declaration.selection.clone(),
        abstract_model: abstract_alias.name.clone(),
        abstract_side: match &form.abstract_side {
            AbstractSideForm::Model(_) => AbstractSide::Model,
            AbstractSideForm::Protocol { clause, .. } => AbstractSide::Protocol {
                clause: clause.name.clone(),
            },
        },
        abstract_internal: {
            let mut classes: Vec<AbstractStepClass> =
                internal.iter().map(|(class, _)| class.clone()).collect();
            classes.sort();
            classes
        },
        concrete_model: form.concrete.name.clone(),
        populations,
        objects,
        steps: Vec::new(),
        history,
        concrete_fairness,
        abstract_fairness,
        concrete_object_types,
        concrete_protocol: None,
    };
    // A protocol abstract side refused by its own clause checks reported
    // those refusals (FR-147 "Abstract side"); the refinement gives no check.
    let check = match protocol {
        _ if clause_refused => RefinementCheck::Refused(findings.refusals),
        // A protocol subject keeps every refusal found so far beside the
        // clause's, whichever part they come from: the clause check does
        // not wait on them (FR-136 "Order of the checks").
        Some((selection, rows)) => RefinementCheck::Protocol(Box::new(ProtocolRefinement {
            checked,
            operations,
            written,
            selection,
            rows,
            profile: declaration.selection.identity.clone(),
            refusals: findings.refusals,
            name_span: form.name.span,
            typing: protocol::Typing {
                lifting,
                maps_complete,
                history: scoped_history,
                protocol_subject,
                abstract_classes,
                internal: internal.clone(),
            },
        })),
        None if findings.refusals.is_empty() => {
            RefinementCheck::Checked(Box::new(CheckedRefinement {
                steps: operations.into_iter().map(|(_, row)| row).collect(),
                ..checked
            }))
        }
        None => RefinementCheck::Refused(findings.refusals),
    };
    (check, nodes)
}

/// The rows of a declaration after its operation rows are checked.
struct StepRows<'f> {
    /// The operation rows that checked, each with its position among the
    /// declaration's rows.
    operations: Vec<(usize, StepRow)>,
    /// The operation of every operation row written, whether or not its
    /// right side checked: a refused right side is that row's refusal, and
    /// the operation still has its row.
    written: Vec<DeclarationKey>,
    /// The selected protocol and the rows that wait for its checked clause.
    protocol: Option<(protocol::Selection, Vec<(usize, &'f StepRowForm)>)>,
}

/// What the check of a refinement reads beyond the unit's models.
pub(crate) struct RefinementEnv<'e> {
    /// The refinement features this build has built.
    pub(crate) support: RefinementSupport,
    /// The unit's `refinement_internals` value, parsed.
    pub(crate) lists: &'e ParsedLists,
    /// The unit's protocol clauses, which a protocol subject selects among.
    pub(crate) protocols: &'e [ProtocolView<'e>],
}

/// One protocol clause of the unit and its check: `checked` is `None` when
/// the clause was refused by its own checks.
pub(crate) struct ProtocolView<'e> {
    pub(crate) form: &'e qsl_forms::ProtocolDeclarationForm,
    pub(crate) checked: Option<&'e super::protocol_clause::CheckedProtocol>,
}

/// How a refinement's abstract subject resolved, which decides what its
/// internal list is resolved against.
enum ListSubject {
    /// A model subject: a list is never resolved against a protocol.
    Model,
    /// A protocol subject whose clause resolved.
    Protocol(Box<ProtocolClasses>),
    /// A protocol subject whose clause did not resolve, or whose side the
    /// build has not built: it reported what it reports and its entries
    /// resolve against nothing.
    Unresolved,
    /// A protocol subject whose clause was refused by its own checks, which
    /// reported its refusals (FR-147 "Abstract side").
    Refused,
}

/// FR-147 "Abstract side": the checked clause `clause` names, among the
/// unit's protocol clauses whose roles act on the abstract model.
fn resolve_protocol(
    findings: &mut Findings,
    env: &RefinementEnv<'_>,
    clause: &qsl_forms::DeclaredName,
    abstract_side: &Side<'_>,
    types: &TypeEnvironment,
) -> ListSubject {
    // A build that has not built the side refused it already and reports
    // nothing else about it.
    let report = env.support.protocol_abstract_side;
    let Some(view) = env
        .protocols
        .iter()
        .find(|view| view.form.name.name == clause.name)
    else {
        if report {
            findings.unresolved(&clause.name, clause.span, None);
        }
        return ListSubject::Unresolved;
    };
    let Some(checked) = view.checked else {
        return ListSubject::Refused;
    };
    let acts_on_abstract_model = {
        let object_types = abstract_side.object_types(types).unwrap_or_default();
        checked.roles.iter().any(|role| {
            role.context
                .is_some_and(|context| object_types.iter().any(|object| object.id == context))
        })
    };
    if !acts_on_abstract_model {
        if report {
            findings.malformed(&clause.name, clause.span);
        }
        return ListSubject::Unresolved;
    }
    ListSubject::Protocol(Box::new(ProtocolClasses::of(view.form, checked)))
}

/// FR-147 "The refinement's internal list": the classes `form`'s list makes
/// internal, each with the pointer of its entry, and the refusals the list
/// gives for this refinement. A defect found while parsing the value has
/// been reported with the value; a refinement with one has no list.
fn internal_list(
    findings: &mut Findings,
    env: &RefinementEnv<'_>,
    form: &RefinementForm,
    profile: &str,
    subject: &ListSubject,
) -> Vec<(AbstractStepClass, String)> {
    let name = &form.name.name;
    let (AbstractSideForm::Model(alias) | AbstractSideForm::Protocol { alias, .. }) =
        &form.abstract_side;
    let Some(object) = env.lists.object_of(name) else {
        return Vec::new();
    };
    if env.lists.is_defective(name) {
        return Vec::new();
    }
    let not_built = |findings: &mut Findings| {
        findings.refuse(
            alias.span,
            RefinementCause::Unsupported {
                feature: UnsupportedFeature::RefinementInternalList,
                profile: profile.to_owned(),
                subject: None,
            },
        );
    };
    match subject {
        ListSubject::Protocol(classes) => {
            let (resolved, missing) = classes.resolve_list(&alias.name, object);
            for pointer in &missing {
                findings.list_defect(ListDefect {
                    kind: ListDefectKind::Missing,
                    pointer: pointer.clone(),
                    first: None,
                });
            }
            if !missing.is_empty() {
                return Vec::new();
            }
            if !resolved.is_empty() && !env.support.refinement_internal_list {
                not_built(findings);
            }
            resolved
        }
        ListSubject::Model => {
            // A model subject selects no protocol to resolve an entry
            // against: a well-formed non-empty list is not yet built, or
            // malformed once it is.
            if object.non_empty {
                if env.support.refinement_internal_list {
                    findings.list_defect(ListDefect {
                        kind: ListDefectKind::Malformed,
                        pointer: object.pointer.clone(),
                        first: None,
                    });
                } else {
                    not_built(findings);
                }
            }
            Vec::new()
        }
        ListSubject::Unresolved | ListSubject::Refused => Vec::new(),
    }
}

struct Checker<'c, 'a> {
    declarations: &'c ClauseDeclarations<'a>,
    /// The unit's protocol declarations, which a step row may select.
    protocols: &'c [ProtocolDeclarationForm],
    abstract_side: Side<'c>,
    concrete_side: Side<'c>,
    findings: Findings,
    nodes: &'c mut u64,
    /// Each abstract object type an object row maps, with the concrete type
    /// it is mapped from.
    lifting: BTreeMap<EffectiveId, EffectiveId>,
    /// Whether the population and object maps checked without a refusal.
    /// Typing a step's receiver and arguments is meaningful only against a
    /// complete map: a type no row maps cannot be lifted.
    maps_complete: bool,
    /// How many refusals the header already holds, which say nothing of the
    /// maps.
    baseline: usize,
    /// The concrete populations a row may name, resolved on first use.
    populations: Option<PopulationScope>,
    /// The history fields a row may read (FR-138), in scope once their rows'
    /// headers check.
    history: Vec<ScopedHistory>,
    /// Whether the abstract side is written `abstract A::protocol`.
    protocol_subject: bool,
    /// The step classes of the selected abstract protocol clause, when the
    /// side is a protocol subject that resolved.
    protocol: Option<ProtocolClasses>,
    /// The classes the internal list makes internal, each with the pointer
    /// of the entry that names it.
    internal: Vec<(AbstractStepClass, String)>,
}

impl<'c, 'a> Checker<'c, 'a> {
    /// The concrete populations a row may name (FR-139): the populations of
    /// the concrete model whose members are of one object type, by the name
    /// a row spells them. A population of several member types names no
    /// single `Set<Reference<T>>` and is not offered.
    fn population_scope(&mut self) -> PopulationScope {
        if let Some(scope) = &self.populations {
            return scope.clone();
        }
        let scope = self.resolve_population_scope();
        self.populations = Some(scope.clone());
        scope
    }

    fn resolve_population_scope(&mut self) -> PopulationScope {
        let types = self.types();
        let populations = self.concrete_side.populations();
        let object_types = self.concrete_side.object_types(types);
        let (Ok(populations), Ok(object_types)) = (populations, object_types) else {
            self.fault("concrete-populations-and-object-types-are-admitted");
            return PopulationScope::default();
        };
        PopulationScope {
            populations: populations
                .into_iter()
                .filter_map(|population| {
                    let [member] = &population.record.member_types[..] else {
                        return None;
                    };
                    let member = object_types.iter().find(|object| &object.key == member)?;
                    Some(ScopedPopulation {
                        name: population.name,
                        key: population.record.key.clone(),
                        member: member.id,
                    })
                })
                .collect(),
        }
    }

    fn types(&self) -> &'a TypeEnvironment {
        self.declarations.scope.types()
    }

    /// FR-135 "Population map".
    fn populations(&mut self, form: &RefinementForm) -> Vec<PopulationRow> {
        let abstract_populations = {
            let found = self.abstract_side.populations();
            self.or_fault(found)
        };
        let concrete_populations = {
            let found = self.concrete_side.populations();
            self.or_fault(found)
        };
        let mut rows: Vec<PopulationRow> = Vec::new();
        // The abstract populations a row names on its left, and the row.
        let mut named: BTreeMap<&DeclarationKey, Span> = BTreeMap::new();
        for row in &form.rows {
            let RefinementRowForm::Population(row) = row else {
                continue;
            };
            let target = abstract_populations
                .iter()
                .find(|population| population.name == row.target.name);
            let source = concrete_populations
                .iter()
                .find(|population| population.name == row.source.name);
            if target.is_none() || source.is_none() {
                // A name that spells a population of the other model is the
                // wrong side of the row; one that spells none is missing.
                let wrong_side = |written: &qsl_forms::NameForm, others: &[Population<'_>]| {
                    others
                        .iter()
                        .any(|population| population.name == written.name)
                };
                let sides = [
                    (target.is_none(), &row.target, &concrete_populations),
                    (source.is_none(), &row.source, &abstract_populations),
                ];
                let mut reported_malformed = false;
                for (unresolved, written, others) in sides {
                    if !unresolved {
                        continue;
                    }
                    if wrong_side(written, others) {
                        // One refusal for a row with its sides swapped.
                        if !reported_malformed {
                            self.findings.malformed(&written.name, row.span);
                            reported_malformed = true;
                        }
                    } else {
                        self.findings.unresolved(&written.name, written.span, None);
                    }
                }
            }
            let Some(target) = target else {
                // A row with its sides swapped still names the abstract
                // population it was written for.
                if let Some(swapped) = abstract_populations
                    .iter()
                    .find(|population| population.name == row.source.name)
                {
                    named.entry(&swapped.record.key).or_insert(row.span);
                }
                continue;
            };
            match named.get(&target.record.key) {
                Some(first) => {
                    self.findings.refuse(
                        row.span,
                        RefinementCause::Conflicting {
                            name: target.name.clone(),
                            first: *first,
                        },
                    );
                    continue;
                }
                None => {
                    named.insert(&target.record.key, row.span);
                }
            }
            if let Some(source) = source {
                rows.push(PopulationRow {
                    abstract_population: target.record.key.clone(),
                    concrete_population: source.record.key.clone(),
                });
            }
        }
        for population in &abstract_populations {
            if !named.contains_key(&population.record.key) {
                self.findings.refuse(
                    form.name.span,
                    RefinementCause::MissingRow {
                        class: RowClass::Population,
                        name: population.name.clone(),
                    },
                );
            }
        }
        rows
    }

    /// The abstract object types the abstract population `population`
    /// admits: its member types and each object type of the model that
    /// conforms to one.
    fn admitted(&self, population: &Population<'_>, types: &[ObjectType]) -> Vec<EffectiveId> {
        let members: Vec<EffectiveId> = population
            .record
            .member_types
            .iter()
            .filter_map(|key| self.abstract_side.model.effective_id(key))
            .collect();
        types
            .iter()
            .filter(|object| {
                members
                    .iter()
                    .any(|member| self.types().conforms(object.id, *member))
            })
            .map(|object| object.id)
            .collect()
    }

    /// FR-135 "Object map".
    fn objects(
        &mut self,
        form: &RefinementForm,
        cx: &mut CheckContext<'a, ClauseDeclarations<'a>>,
    ) -> Vec<ObjectRow> {
        let abstract_types = {
            let found = self.abstract_side.object_types(self.types());
            self.or_fault(found)
        };
        let concrete_types = {
            let found = self.concrete_side.object_types(self.types());
            self.or_fault(found)
        };
        let abstract_populations = {
            let found = self.abstract_side.populations();
            self.or_fault(found)
        };
        let concrete_populations = {
            let found = self.concrete_side.populations();
            self.or_fault(found)
        };
        // The population rows that resolved, as (abstract, concrete).
        let mapped: Vec<(&Population<'_>, &Population<'_>)> = form
            .rows
            .iter()
            .filter_map(|row| match row {
                RefinementRowForm::Population(row) => Some((
                    abstract_populations
                        .iter()
                        .find(|population| population.name == row.target.name)?,
                    concrete_populations
                        .iter()
                        .find(|population| population.name == row.source.name)?,
                )),
                _ => None,
            })
            .collect();

        // The abstract populations some population row names on its left.
        let named: Vec<&Population<'_>> = abstract_populations
            .iter()
            .filter(|population| {
                form.rows.iter().any(|row| {
                    matches!(
                        row,
                        RefinementRowForm::Population(row)
                            if row.target.name == population.name
                                || row.source.name == population.name
                    )
                })
            })
            .collect();

        // The abstract types each object row maps, so every field expression
        // is lifted against the whole map.
        let mut first_rows: BTreeMap<EffectiveId, Span> = BTreeMap::new();
        let mut resolved: Vec<(&qsl_forms::ObjectRowForm, &ObjectType, &ObjectType)> = Vec::new();
        for row in &form.rows {
            let RefinementRowForm::Object(row) = row else {
                continue;
            };
            let target = self.type_named(&row.target, &abstract_types, &concrete_types);
            let source = self.type_named(&row.source, &concrete_types, &abstract_types);
            let (Some(target), Some(source)) = (target, source) else {
                continue;
            };
            // The abstract type must belong to an abstract population, and
            // the concrete type to that population's concrete source.
            let admitting: Vec<&Population<'_>> = abstract_populations
                .iter()
                .filter(|population| {
                    self.admitted(population, &abstract_types)
                        .contains(&target.id)
                })
                .collect();
            if admitting.is_empty() {
                self.findings.malformed(&row.target.name, row.target.span);
                continue;
            }
            let sources: Vec<&Population<'_>> = mapped
                .iter()
                .filter(|(abstract_population, _)| {
                    admitting
                        .iter()
                        .any(|admitting| admitting.record.key == abstract_population.record.key)
                })
                .map(|(_, concrete_population)| *concrete_population)
                .collect();
            if !sources.is_empty()
                && !sources.iter().any(|population| {
                    population
                        .record
                        .member_types
                        .iter()
                        .filter_map(|key| self.concrete_side.model.effective_id(key))
                        .any(|member| self.types().conforms(source.id, member))
                })
            {
                self.findings.malformed(&row.source.name, row.source.span);
                continue;
            }
            if let Some(first) = first_rows.get(&target.id) {
                self.findings.refuse(
                    row.span,
                    RefinementCause::Conflicting {
                        name: target.name.clone(),
                        first: *first,
                    },
                );
                continue;
            }
            first_rows.insert(target.id, row.span);
            self.lifting.insert(target.id, source.id);
            resolved.push((row, target, source));
        }
        let mut reported = std::collections::BTreeSet::new();
        for population in &named {
            for id in self.admitted(population, &abstract_types) {
                if !first_rows.contains_key(&id) && reported.insert(id) {
                    let name = abstract_types
                        .iter()
                        .find(|object| object.id == id)
                        .map_or_else(String::new, |object| object.name.clone());
                    self.findings.refuse(
                        form.name.span,
                        RefinementCause::MissingRow {
                            class: RowClass::Object,
                            name,
                        },
                    );
                }
            }
        }

        self.maps_complete = self.findings.refusals.len() == self.baseline;
        let mut objects = Vec::new();
        for (row, target, source) in resolved {
            if let Some(object) = self.object_row(row, target, source, cx) {
                objects.push(object);
            }
        }
        objects
    }

    /// The object type of `types` that `name` spells; a name that spells
    /// one of `others` is malformed, and one that spells neither is missing.
    fn type_named<'t>(
        &mut self,
        name: &qsl_forms::NameForm,
        types: &'t [ObjectType],
        others: &[ObjectType],
    ) -> Option<&'t ObjectType> {
        if let Some(found) = types.iter().find(|object| object.name == name.name) {
            return Some(found);
        }
        if others.iter().any(|object| object.name == name.name) {
            self.findings.malformed(&name.name, name.span);
        } else {
            self.findings.unresolved(&name.name, name.span, None);
        }
        None
    }

    /// One object row's fields, and the abstract fields it leaves hidden.
    fn object_row(
        &mut self,
        row: &qsl_forms::ObjectRowForm,
        target: &ObjectType,
        source: &ObjectType,
        cx: &mut CheckContext<'a, ClauseDeclarations<'a>>,
    ) -> Option<ObjectRow> {
        let types = self.types();
        let mut defined: BTreeMap<&str, Span> = BTreeMap::new();
        let mut fields = Vec::new();
        let mut complete = true;
        for field in &row.fields {
            let written = field.field.name.as_str();
            let Some(attribute) = types.attribute(target.id, written) else {
                // The protocol's state takes no row (FR-147 "Abstract side").
                if self
                    .protocol
                    .as_ref()
                    .is_some_and(|classes| classes.has_state(written))
                {
                    self.findings.malformed(written, field.field.span);
                    complete = false;
                    continue;
                }
                self.findings.unresolved(
                    &format!("{}.{written}", target.name),
                    field.field.span,
                    None,
                );
                complete = false;
                continue;
            };
            if let Some(first) = defined.get(written) {
                self.findings.refuse(
                    field.span,
                    RefinementCause::Conflicting {
                        name: format!("{}.{written}", target.name),
                        first: *first,
                    },
                );
                complete = false;
                continue;
            }
            defined.insert(written, field.span);
            let declared = attribute.field();
            let required = lift(
                &self.lifting,
                &if declared.presence() == Presence::Optional {
                    ValueType::option(declared.value_type().clone())
                } else {
                    declared.value_type().clone()
                },
            );
            let expression = self.check_row(
                &field.value,
                &required,
                RowContext {
                    parameters: vec![("self".to_owned(), ValueType::Reference(source.id))],
                    kind: qsl_forms::StateClauseKind::Invariant,
                    operation: None,
                    label: format!("{}.{written}", target.name),
                    range_agnostic: false,
                },
                cx,
            );
            let key = self.abstract_side.model.field_key(&attribute.identity());
            match (expression, key) {
                (Some(expression), Some(key)) => fields.push(FieldRow {
                    field: key.clone(),
                    expression,
                }),
                (_, None) => {
                    self.fault("abstract-field-has-a-declaration-key");
                    complete = false;
                }
                (None, Some(_)) => complete = false,
            }
        }
        let mut hidden = Vec::new();
        let Some(attributes) = types.attributes(target.id) else {
            self.fault("abstract-object-type-has-attributes");
            return None;
        };
        for attribute in attributes.iter() {
            if defined.contains_key(attribute.field().name()) {
                continue;
            }
            match self.abstract_side.model.field_key(&attribute.identity()) {
                Some(key) => hidden.push(key.clone()),
                None => {
                    self.fault("abstract-field-has-a-declaration-key");
                    complete = false;
                }
            }
        }
        complete.then(|| ObjectRow {
            abstract_type: target.key.clone(),
            concrete_type: source.key.clone(),
            fields,
            hidden,
        })
    }

    /// `found`, or an empty list and the internal fault of the invariant it
    /// reports broken.
    fn or_fault<T>(&mut self, found: Result<Vec<T>, &'static str>) -> Vec<T> {
        found.unwrap_or_else(|invariant| {
            self.fault(invariant);
            Vec::new()
        })
    }

    fn fault(&mut self, invariant: &'static str) {
        self.findings.fault(invariant);
    }

    /// Types `expression` where `required` is expected, over `context`, and
    /// records its refusal at the expression's span.
    fn check_row(
        &mut self,
        expression: &ExpressionForm,
        required: &ValueType,
        context: RowContext,
        cx: &mut CheckContext<'a, ClauseDeclarations<'a>>,
    ) -> Option<CheckedRow> {
        self.check_row_as(expression, Some(required), context, cx)
    }

    /// [`Self::check_row`] where the type the row defines is `required`, or
    /// any type when there is none.
    fn check_row_as(
        &mut self,
        expression: &ExpressionForm,
        required: Option<&ValueType>,
        context: RowContext,
        cx: &mut CheckContext<'a, ClauseDeclarations<'a>>,
    ) -> Option<CheckedRow> {
        let declarations = self.declarations;
        let location = Location::root(Origin::Expression);
        let RowContext {
            parameters,
            kind,
            operation,
            label,
            range_agnostic,
        } = context;
        let populations = self.population_scope();
        let typed = (|| {
            let mut typer = Typer::new(
                declarations.scope,
                declarations.signatures,
                declarations.checking_limits,
                &mut *self.nodes,
                match kind {
                    qsl_forms::StateClauseKind::Invariant => qsl_forms::ClauseKind::Invariant,
                    qsl_forms::StateClauseKind::Precondition => qsl_forms::ClauseKind::Precondition,
                    qsl_forms::StateClauseKind::Postcondition => {
                        qsl_forms::ClauseKind::Postcondition
                    }
                },
            )
            .with_cancel(declarations.cancel)
            .with_context(cx.reborrow(declarations.scope))
            .with_populations(populations.clone())
            .with_history(self.history.clone());
            bind_parameters(&mut typer, &parameters, &location)?;
            typer.enter_state_clause(StateContext {
                parent_projections: BTreeMap::new(),
                kind,
                operation,
                self_slot: 0,
                result_slot: None,
            });
            // One rule for every row: the expression's own type conforms to
            // the type it defines (ADR-016 SC-2), never a range-checked
            // coercion into it.
            let body = typer.infer(&expression.expression, required, &location)?;
            let types = declarations.scope.types();
            let uses = match required {
                None => Vec::new(),
                Some(required)
                    if range_agnostic
                        && relaxed_conforms(types, body.root().value_type(), required) =>
                {
                    Vec::new()
                }
                Some(required) => population_uses(
                    types,
                    &body,
                    required,
                    (&label, &populations),
                    expression
                        .spans
                        .at(&[])
                        .unwrap_or(Span { start: 0, end: 0 }),
                )
                .map_err(|cause| match cause {
                    Some(cause) => refuse_with(&location, cause),
                    None => CheckRefusal::ill_typed(&location, IllTypedCause::TypeMismatch),
                })?,
            };
            Ok::<_, CheckRefusal>((body, typer.slots(), uses))
        })();
        match typed {
            Ok((body, slots, uses)) => {
                let observations =
                    Observations::of(body.root(), observation_of(kind), parameters.len());
                Some(CheckedRow {
                    parameters,
                    body,
                    slots,
                    uses,
                    observations,
                })
            }
            Err(refusal) => {
                match super::region::span_at(&expression.spans, &refusal.location)
                    .or_else(|| expression.spans.at(&[]))
                {
                    Some(span) => self
                        .findings
                        .refuse(span, RefinementCause::Check(refusal.cause)),
                    None => self.fault("row-expression-has-a-span"),
                }
                None
            }
        }
    }

    /// FR-136: the operation rows, each right side checked, and, over a
    /// model subject, one row per concrete operation. The rows that select
    /// a protocol clause are classified here and resolved once the clause is
    /// checked (`ProtocolRefinement::complete`).
    fn steps<'f>(
        &mut self,
        form: &'f RefinementForm,
        cx: &mut CheckContext<'a, ClauseDeclarations<'a>>,
    ) -> StepRows<'f> {
        let scope = self.declarations.scope;
        let types = self.types();
        let abstract_types = {
            let found = self.abstract_side.object_types(types);
            self.or_fault(found)
        };
        let concrete_types = {
            let found = self.concrete_side.object_types(types);
            self.or_fault(found)
        };
        let protocol::Classified { rows, selection } =
            protocol::classify(form, self.protocols, &concrete_types, &mut self.findings);
        let mut first_rows: BTreeMap<(EffectiveId, String), Span> = BTreeMap::new();
        let mut steps = Vec::new();
        let mut written = Vec::new();
        let mut deferred = Vec::new();
        for (index, row, sort) in rows {
            let source = match (&row.source, sort) {
                (_, protocol::RowSort::Refused) => continue,
                (_, protocol::RowSort::Unresolved) => {
                    self.fault("an-unresolved-row-is-settled-by-classify");
                    continue;
                }
                (_, protocol::RowSort::Protocol) => {
                    deferred.push((index, row));
                    continue;
                }
                // A `memory <kind>` row names a kind of the selected
                // protocol's memory models, and with no protocol selected
                // it names none.
                (StepSourceForm::Memory(_), _) if selection.is_some() => {
                    deferred.push((index, row));
                    continue;
                }
                (StepSourceForm::Memory(kind), _) => {
                    self.findings.unresolved(&kind.name, kind.span, None);
                    continue;
                }
                (StepSourceForm::Named { kind: None, name }, _) => name,
                (StepSourceForm::Named { kind: Some(_), .. }, _) => {
                    self.fault("a-keyworded-row-names-a-protocol");
                    continue;
                }
            };
            let Some((object, declaring, concrete)) =
                self.operation_named(source, &concrete_types, None)
            else {
                continue;
            };
            let display = format!(
                "{}::{}",
                types.object_type(declaring).map_or(
                    "",
                    quire_semantic_value::declaration::ObjectTypeDeclaration::name
                ),
                concrete.name()
            );
            if let Some(first) = first_rows.get(&(declaring, concrete.name().to_owned())) {
                self.findings.refuse(
                    row.span,
                    RefinementCause::Conflicting {
                        name: display,
                        first: *first,
                    },
                );
                continue;
            }
            first_rows.insert((declaring, concrete.name().to_owned()), row.span);
            let clause_operation = ClauseOperation {
                declaring,
                declaration: concrete.clone(),
            };
            let Some(key) = self.concrete_side.model.operation_key(&clause_operation) else {
                self.fault("concrete-operation-has-a-declaration-key");
                continue;
            };
            written.push(key.clone());
            let target = match &row.target {
                StepTargetForm::Stutter { .. } => Some(StepTarget::Stutter),
                StepTargetForm::Any { .. } => Some(StepTarget::Any),
                StepTargetForm::Node {
                    node, arguments, ..
                } => self.node_target(row.span, node, arguments, (object, concrete), cx),
                StepTargetForm::Apply {
                    operation,
                    arguments,
                    ..
                } => self.application(
                    row.span,
                    operation,
                    arguments,
                    (object, concrete),
                    &abstract_types,
                    cx,
                ),
            };
            if let Some(target) = target {
                steps.push((
                    index,
                    StepRow {
                        class: StepClass::Operation(key.clone()),
                        target,
                    },
                ));
            }
        }
        // Over a protocol subject an operation needs a row only where an
        // attempt of it has no node row, which coverage decides.
        if selection.is_none() {
            for object in &concrete_types {
                for operation in scope.operations().declared_by(object.id) {
                    if !first_rows.contains_key(&(object.id, operation.name().to_owned())) {
                        self.findings.refuse(
                            form.name.span,
                            RefinementCause::MissingRow {
                                class: RowClass::Step,
                                name: format!("{}::{}", object.name, operation.name()),
                            },
                        );
                    }
                }
            }
        }
        StepRows {
            operations: steps,
            written,
            protocol: selection.map(|selection| (selection, deferred)),
        }
    }

    /// The operation `name` (`Alias::Type::operation`) spells among the
    /// object types `types`, recording the refusal when there is none.
    fn operation_named<'t>(
        &mut self,
        name: &qsl_forms::NameForm,
        types: &'t [ObjectType],
        held_by: Option<&str>,
    ) -> Option<(
        &'t ObjectType,
        EffectiveId,
        &'a crate::model::operation::OperationDeclaration,
    )> {
        let scope = self.declarations.scope;
        let lookup = split_member(&name.name).and_then(|(type_name, member)| {
            let object = types.iter().find(|object| object.name == type_name)?;
            Some((
                object,
                scope.operations().resolve(self.types(), object.id, member),
            ))
        });
        match lookup {
            Some((
                object,
                OperationLookup::Declared {
                    declaring,
                    operation,
                },
            )) => Some((object, declaring, operation)),
            Some((_, OperationLookup::Ambiguous(_))) => {
                self.findings.refuse(
                    name.span,
                    RefinementCause::Ambiguous {
                        name: name.name.clone(),
                    },
                );
                None
            }
            Some((_, OperationLookup::Missing)) | None => {
                self.findings.unresolved(&name.name, name.span, held_by);
                None
            }
        }
    }

    /// A step row whose right side is `node <A>::<protocol>::<node>(...)`
    /// (FR-147 "Step rows and observations").
    fn node_target(
        &mut self,
        row_span: Span,
        node: &qsl_forms::NameForm,
        arguments: &[ArgumentForm],
        (object, concrete): (&ObjectType, &crate::model::operation::OperationDeclaration),
        cx: &mut CheckContext<'a, ClauseDeclarations<'a>>,
    ) -> Option<StepTarget> {
        if !self.protocol_subject {
            // While the abstract side is a model subject, a right side
            // naming a protocol node is malformed.
            self.findings.malformed(&node.name, node.span);
            return None;
        }
        // A protocol subject that did not resolve reported why.
        let classes = self.protocol.as_ref()?;
        let named = match node.name.split("::").collect::<Vec<_>>()[..] {
            [alias, protocol, member]
                if alias == self.abstract_side.alias
                    && protocol == classes.protocol
                    && matches!(
                        classes.node(member),
                        Some(NodeClass::Bare(
                            ProtocolNodeKind::Attempt
                                | ProtocolNodeKind::Effect
                                | ProtocolNodeKind::Event
                                | ProtocolNodeKind::Commit
                                | ProtocolNodeKind::Send
                                | ProtocolNodeKind::Receive
                                | ProtocolNodeKind::Finish
                        ))
                    ) =>
            {
                AbstractStepClass::node(protocol, member)
            }
            _ => {
                self.findings.unresolved(&node.name, node.span, None);
                return None;
            }
        };
        if let Some((_, entry)) = self.internal.iter().find(|(class, _)| *class == named) {
            let conflict = RefinementCause::ListEntryConflict {
                class: named.spelling().to_owned(),
                entry: entry.clone(),
            };
            self.findings.refuse(row_span, conflict);
            return None;
        }
        let mut parameters = vec![("self".to_owned(), ValueType::Reference(object.id))];
        parameters.extend(concrete.parameters().iter().cloned());
        let mut checked = Vec::new();
        let mut complete = true;
        for argument in arguments {
            match argument {
                ArgumentForm::Open { .. } => checked.push(None),
                ArgumentForm::Expression(expression) => {
                    let context = RowContext {
                        parameters: parameters.clone(),
                        kind: qsl_forms::StateClauseKind::Postcondition,
                        operation: Some(concrete.name().to_owned()),
                        label: format!("step {}", concrete.name()),
                        range_agnostic: false,
                    };
                    match self.check_row_as(expression, None, context, cx) {
                        Some(row) => checked.push(Some(row)),
                        None => complete = false,
                    }
                }
            }
        }
        let (protocol, member) = named.spelling().split_once("::")?;
        complete.then(|| StepTarget::Node {
            protocol: protocol.to_owned(),
            node: member.to_owned(),
            arguments: checked,
        })
    }

    /// A step row whose right side applies an abstract operation.
    fn application(
        &mut self,
        row_span: Span,
        operation: &qsl_forms::NameForm,
        arguments: &[ArgumentForm],
        (object, concrete): (&ObjectType, &crate::model::operation::OperationDeclaration),
        abstract_types: &[ObjectType],
        cx: &mut CheckContext<'a, ClauseDeclarations<'a>>,
    ) -> Option<StepTarget> {
        let types = self.types();
        let (target_type, declaring, abstract_operation) =
            self.operation_named(operation, abstract_types, None)?;
        let parameters = abstract_operation.parameters();
        if arguments.len() != parameters.len() + 1 {
            self.findings.refuse(
                row_span,
                RefinementCause::Check(CheckCause::IllTyped(IllTypedCause::OperatorIneligible)),
            );
            return None;
        }
        if !self.maps_complete {
            return None;
        }
        let key = self.abstract_side.model.operation_key(&ClauseOperation {
            declaring,
            declaration: abstract_operation.clone(),
        });
        let Some(key) = key.cloned() else {
            self.fault("abstract-operation-has-a-declaration-key");
            return None;
        };
        let mut context_parameters = vec![("self".to_owned(), ValueType::Reference(object.id))];
        context_parameters.extend(concrete.parameters().iter().cloned());
        let context = |parameters: &Vec<(String, ValueType)>| RowContext {
            parameters: parameters.clone(),
            kind: qsl_forms::StateClauseKind::Postcondition,
            operation: Some(concrete.name().to_owned()),
            label: format!("step {}", concrete.name()),
            range_agnostic: false,
        };
        let mut complete = true;
        let receiver_type = lift(&self.lifting, &ValueType::Reference(target_type.id));
        let receiver = match &arguments[0] {
            ArgumentForm::Expression(expression) => {
                self.check_row(expression, &receiver_type, context(&context_parameters), cx)
            }
            ArgumentForm::Open { .. } => None,
        };
        complete &= receiver.is_some();
        let mut checked = Vec::new();
        for (argument, (_, declared)) in arguments[1..].iter().zip(parameters) {
            match argument {
                ArgumentForm::Open { .. } => checked.push(None),
                ArgumentForm::Expression(expression) => {
                    let required = lift(&self.lifting, declared);
                    match self.check_row(expression, &required, context(&context_parameters), cx) {
                        Some(row) => checked.push(Some(row)),
                        None => complete = false,
                    }
                }
            }
        }
        let binds_result = match (concrete.result(), abstract_operation.result()) {
            (Some(actual), Some(required))
                if conforms(types, actual, &lift(&self.lifting, required)) =>
            {
                true
            }
            (Some(_), Some(_)) => {
                self.findings.refuse(
                    row_span,
                    RefinementCause::Check(CheckCause::IllTyped(IllTypedCause::TypeMismatch)),
                );
                complete = false;
                false
            }
            _ => false,
        };
        let receiver = receiver?;
        complete.then_some(StepTarget::Abstract {
            operation: key,
            receiver,
            arguments: checked,
            binds_result,
        })
    }

    /// FR-137: the rows of `side`, each constraint once, in source order.
    fn fairness(&mut self, form: &RefinementForm, side: FairnessSide) -> Vec<FairnessConstraint> {
        let scope = self.declarations.scope;
        let abstract_types = {
            let found = self.abstract_side.object_types(self.types());
            self.or_fault(found)
        };
        let concrete_types = {
            let found = self.concrete_side.object_types(self.types());
            self.or_fault(found)
        };
        let (own_types, other_types, other_alias) = match side {
            FairnessSide::Assume => (&concrete_types, &abstract_types, self.abstract_side.alias),
            FairnessSide::Ensure => (&abstract_types, &concrete_types, self.concrete_side.alias),
        };
        let mut set: Vec<(FairnessConstraint, Span)> = Vec::new();
        for row in &form.rows {
            let RefinementRowForm::Fairness(row) = row else {
                continue;
            };
            if row.side != side {
                continue;
            }
            let written: &FairnessConstraintForm = &row.constraint;
            let resolve = |types: &[ObjectType]| {
                let (type_name, member) = split_member(&written.operation.name)?;
                let object = types.iter().find(|object| object.name == type_name)?;
                match scope.operations().resolve(self.types(), object.id, member) {
                    OperationLookup::Declared {
                        declaring,
                        operation,
                    } => Some(FairnessOperation::new(
                        declaring,
                        operation.name().to_owned(),
                    )),
                    OperationLookup::Missing | OperationLookup::Ambiguous(_) => None,
                }
            };
            let Some(operation) = resolve(own_types) else {
                let held_by = resolve(other_types).map(|_| other_alias);
                self.findings
                    .unresolved(&written.operation.name, written.operation.span, held_by);
                continue;
            };
            // A refinement row writes its kind (S1 refuses a kindless row,
            // QSpec FR-375); an unwritten granularity reads as `whole`.
            let kind = match written.kind {
                Some(FormKind::Weak) => FairnessKind::Weak,
                Some(FormKind::Strong) => FairnessKind::Strong,
                None => {
                    self.fault("a-refinement-fairness-row-writes-its-kind");
                    continue;
                }
            };
            let constraint = FairnessConstraint::new(
                kind,
                operation,
                match written.granularity {
                    None | Some(FormGranularity::Whole) => FairnessGranularity::Whole,
                    Some(FormGranularity::Each) => FairnessGranularity::Each,
                },
            );
            // FR-137: two rows of one set with one kind, operation and
            // granularity conflict; neither is dropped.
            if let Some((_, first)) = set.iter().find(|(earlier, _)| *earlier == constraint) {
                self.findings.refuse(
                    written.span,
                    RefinementCause::Conflicting {
                        name: written.operation.name.clone(),
                        first: *first,
                    },
                );
                continue;
            }
            set.push((constraint, written.span));
        }
        set.into_iter().map(|(constraint, _)| constraint).collect()
    }
}

/// How a row expression is typed: its parameters, the clause kind it reads
/// as and the concrete operation whose arguments it may name.
struct RowContext {
    parameters: Vec<(String, ValueType)>,
    kind: qsl_forms::StateClauseKind,
    operation: Option<String>,
    /// The row, as a refusal names it: `Type.field` or `step operation`.
    label: String,
    /// Whether the row's type conforms to the type it defines when only the
    /// native types are compared and not the integer ranges, which are left
    /// to run time: a history update (FR-138), whose value is computed and
    /// not a literal.
    range_agnostic: bool,
}

fn refuse_with(location: &Location, cause: CheckCause) -> CheckRefusal {
    CheckRefusal {
        location: location.clone(),
        cause,
    }
}

/// The population-size-dependent bounds of a row `body` used as `required`
/// (FR-139): empty when the row's type already conforms. S3 checks every
/// typing rule that does not depend on the universe size `n` and records the
/// bound that does. `Err(None)` is a mismatch that holds whatever `n` is;
/// `Err(Some(_))` is a row whose bound depends on `n` through a form S3 does
/// not read yet.
fn population_uses(
    types: &TypeEnvironment,
    body: &CheckedBody,
    required: &ValueType,
    (label, scope): (&str, &PopulationScope),
    span: Span,
) -> Result<Vec<PopulationUse>, Option<CheckCause>> {
    let root = body.root();
    if conforms(types, root.value_type(), required) {
        return Ok(Vec::new());
    }
    let reads_population = root
        .descendants()
        .iter()
        .any(|node| matches!(node.kind(), NodeKind::PopulationObjects { .. }));
    if !reads_population {
        return Err(None);
    }
    let source_population = |node: CheckedNode<'_>| {
        let mut node = node;
        loop {
            match node.kind() {
                NodeKind::PopulationObjects { population } => return Some(population.clone()),
                NodeKind::Query {
                    visit: Visit::Filter,
                    source,
                    ..
                } => node = node.at(*source),
                _ => return None,
            }
        }
    };
    let (key, value) = match root.kind() {
        NodeKind::PopulationObjects { population } => {
            (population.clone(), PopulationValue::Objects)
        }
        NodeKind::Query {
            visit: Visit::Count,
            source,
            ..
        } => match source_population(root.at(*source)) {
            Some(key) => (key, PopulationValue::Count),
            None => return Err(Some(depends_on_size())),
        },
        NodeKind::Query {
            visit: Visit::Sum,
            source,
            body: summand,
            ..
        } => {
            let summand = root.at(*summand);
            let range = static_integer_range((summand.kind(), summand.value_type()), &|id| {
                let found = body.node(id);
                Some((found.kind(), found.value_type()))
            })
            .and_then(|(lower, upper)| IntegerInterval::new(lower, upper).ok());
            match (source_population(root.at(*source)), range) {
                (Some(key), Some(summand)) => (key, PopulationValue::Sum { summand }),
                // An unbounded summand gives an unbounded sum, whatever `n`
                // is.
                (Some(_), None) => return Err(None),
                (None, _) => return Err(Some(depends_on_size())),
            }
        }
        _ => {
            return if relaxed_conforms(types, root.value_type(), required) {
                Err(Some(depends_on_size()))
            } else {
                Err(None)
            }
        }
    };
    let Some(population) = scope.populations.iter().find(|found| found.key == key) else {
        return Err(None);
    };
    let used = PopulationUse {
        row: label.to_owned(),
        span,
        population: population.name.clone(),
        key,
        member: population.member,
        value,
        required: required.clone(),
    };
    // The part of the check that does not depend on `n`: the value's kind
    // and its bounds that hold for every `n`.
    let independent = match (&used.value, required) {
        (PopulationValue::Objects, ValueType::Collection(collection)) => {
            collection.kind() == CollectionKind::Set
                && collection.element() == &ValueType::Reference(used.member)
                // `n` may be 0, so a minimum above 0 is never met.
                && collection.bound().is_none_or(|bound| bound.minimum() == 0)
        }
        (PopulationValue::Count | PopulationValue::Sum { .. }, ValueType::Integer) => true,
        (PopulationValue::Count | PopulationValue::Sum { .. }, ValueType::Int(target)) => {
            // `n` is at least 0, and a sum over no object is 0.
            (target.lower().is_negative() || target.lower().is_zero())
                && !target.upper().is_negative()
        }
        _ => false,
    };
    if independent {
        Ok(vec![used])
    } else {
        Err(None)
    }
}

fn depends_on_size() -> CheckCause {
    CheckCause::PopulationFormNotChecked {
        what: "a bound that depends on a population's size, through a form other than the row's whole value",
    }
}

/// Whether `actual` would fit `required` if the integer and collection
/// bounds were ignored: the shape of a row that only a universe size can
/// settle.
fn relaxed_conforms(types: &TypeEnvironment, actual: &ValueType, required: &ValueType) -> bool {
    match (actual, required) {
        (ValueType::Integer | ValueType::Int(_), ValueType::Integer | ValueType::Int(_)) => true,
        (ValueType::Collection(actual), ValueType::Collection(required)) => {
            actual.kind() == required.kind()
                && relaxed_conforms(types, actual.element(), required.element())
        }
        (ValueType::Option(actual), ValueType::Option(required)) => {
            relaxed_conforms(types, actual, required)
        }
        _ => conforms(types, actual, required),
    }
}

/// The refusal for a `refinement` declaration that checks but that no S4
/// emission writes yet (FR-135): refused rather than compiled with an
/// identity equal to the same package without it.
pub(crate) fn not_emitted(name: String, span: Span) -> CheckRefusal {
    CheckRefusal {
        location: Location::root(Origin::Expression),
        cause: CheckCause::RefinementNotEmitted { name, span },
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use quire_exact::{CollectionKind, CollectionType, NodeKey, Presence};
    use quire_semantic_value::declaration::{
        CompositeDeclaration, CompositeShape, FieldDeclaration,
    };

    use super::*;

    /// FR-359: a record conforms to the record it specializes and to no
    /// other; a parent does not conform to its child, and a collection of the
    /// child is not a collection of the parent, and an option of the child is not
    /// an option of the parent (FR-359).
    #[trace("TC-541")]
    #[test]
    fn a_record_conforms_to_its_ancestor_only() {
        let [money, taxed, fee] = [1, 2, 3].map(|byte| NodeKey::from_digest([byte; 32]));
        let record = |key, name: &str, supertypes: Vec<NodeKey>| {
            CompositeDeclaration::new(
                key,
                name,
                CompositeShape::Record(vec![FieldDeclaration::new(
                    "amount",
                    ValueType::Integer,
                    Presence::Required,
                )]),
            )
            .with_supertypes(supertypes)
        };
        let types = TypeEnvironment::new(
            [
                record(money, "Money", vec![]),
                record(taxed, "TaxedMoney", vec![money]),
                record(fee, "Fee", vec![]),
            ],
            [],
        )
        .expect("the records admit");
        let record = ValueType::Composite;
        assert!(conforms(&types, &record(taxed), &record(money)));
        assert!(conforms(&types, &record(money), &record(money)));
        assert!(!conforms(&types, &record(money), &record(taxed)));
        assert!(!conforms(&types, &record(fee), &record(money)));
        let sequence_of = |element| {
            ValueType::collection(CollectionType::new(CollectionKind::Sequence, element, None))
        };
        assert!(!conforms(
            &types,
            &sequence_of(record(taxed)),
            &sequence_of(record(money))
        ));
        assert!(!conforms(
            &types,
            &ValueType::option(record(taxed)),
            &ValueType::option(record(money))
        ));
        assert!(conforms(
            &types,
            &ValueType::option(record(money)),
            &ValueType::option(record(money))
        ));
    }
}
