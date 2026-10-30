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

mod catalog;
mod history;
mod internal_list;
mod protocol;
mod rows;
mod step_label;

use std::collections::{BTreeMap, BTreeSet};

pub use catalog::{
    KeyedCatalogClass, KeyedClassCatalog, KeyedMemoryClass, RefinementCatalogClass,
    RefinementCatalogNode, RefinementClassCatalog,
};
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
use qsl_foundation::{Code, Span};
use quire_exact::{
    CardinalityBound, CollectionKind, CollectionType, EffectiveId, IllTypedCause, Integer,
    IntegerInterval, NodeKey, Presence, ValueType,
};
use quire_semantic_value::catalog::InternalFault;
use quire_semantic_value::declaration::{CompositeShape, FieldDeclaration, TypeEnvironment};
use quire_semantic_value::fairness::{
    FairnessConstraint, FairnessGranularity, FairnessKind, FairnessOperation,
};
use quire_semantic_value::location::{Location, Origin};
pub(crate) use rows::{
    KeyedFairnessRow, KeyedObjectRow, KeyedPopulationRow, KeyedRefinementRows, KeyedStepRow,
    KeyedStepTarget,
};
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
use super::state_clause::{observation_of, ClauseDeclarations, ClauseOperation, PopulationDomain};
use crate::family::CheckContext;
use crate::model::clause_observation::Observation;
use crate::model::domain_package::{DomainPackageRef, PopulationRecord};
use crate::model::intake::type_identity_segment;
use crate::model::operation::OperationLookup;
use crate::value::layer::HeaderLayer;
use quire_checked::key::DeclarationKey;

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
    /// A field row whose type nests a `Reference` in an option, record,
    /// tuple (a record by position), union or collection, other than a whole-population `Set<Reference<T>>`
    /// (FR-135): the lifting of ADR-020 RM-3 reaches no further than the
    /// reference itself.
    NestedReferenceRow,
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
            Self::NestedReferenceRow => "nested-reference-row",
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
    /// A row expression uses a form outside the value and model layers that
    /// the governing infinite-trace profile requires: `unsupported_construct`/
    /// `expression-form` (QSpec FR-375 "Row layers").
    UnsupportedRowForm {
        /// The identity of the layer that admits the expression form.
        layer: &'static str,
    },
    /// The abstract model declares relationships, which a refinement cannot
    /// map into its abstract state (FR-135-AC-7).
    AbstractRelationships {
        /// The unsupported declaration form.
        form: &'static str,
        /// The refinement whose abstract side declares it.
        refinement: String,
        /// The selected profile identity.
        profile: String,
    },
    /// The abstract model is timed: it declares a time source (QSpec FR-430
    /// "Timed refinement"): `unsupported_construct`/`declaration-form` at
    /// the refinement's abstract model reference.
    TimedAbstract {
        /// The abstract model's alias.
        model: String,
    },
    /// A row reads a clock field of the concrete model, so the mapped
    /// abstract state would not be a function of the discrete state alone
    /// (QSpec FR-430 "Timed refinement"): `unsupported_construct`/
    /// `expression-form` at the read.
    ClockRead,
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
    /// A node row's argument list whose length differs from the field count
    /// of the node's binder record: `ill_typed`/`operator-ineligible` at the
    /// node (FR-147 "Step rows and observations").
    NodeArgumentCount {
        /// The node, as written.
        node: String,
        /// The field count of the binder record.
        expected: usize,
        /// The number of arguments written.
        actual: usize,
    },
    /// A node row's argument whose type is not its binder field's type:
    /// `ill_typed`/`type-mismatch` at the argument (FR-147 "Step rows and
    /// observations").
    NodeArgumentType {
        /// The node, as written.
        node: String,
        /// The argument's position, from 0.
        position: usize,
        /// The field's type.
        expected: Box<ValueType>,
        /// The argument's type.
        actual: Box<ValueType>,
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
            Self::Unsupported { .. }
            | Self::UnsupportedRowForm { .. }
            | Self::AbstractRelationships { .. }
            | Self::TimedAbstract { .. }
            | Self::ClockRead => Code::UnsupportedConstruct,
            Self::PopulationBound { .. }
            | Self::NodeArgumentCount { .. }
            | Self::NodeArgumentType { .. } => Code::IllTyped,
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
            Self::UnsupportedRowForm { .. } | Self::ClockRead => Some("expression-form"),
            Self::AbstractRelationships { .. } | Self::TimedAbstract { .. } => {
                Some("declaration-form")
            }
            Self::PopulationBound { .. } | Self::NodeArgumentType { .. } => Some("type-mismatch"),
            Self::NodeArgumentCount { .. } => Some("operator-ineligible"),
            Self::Check(cause) => cause.cause(),
        }
    }
}

/// The checked body of one row expression: its parameters (`self`, then a
/// step's operation parameters), the body and the evaluation slots it needs.
pub struct CheckedRow {
    pub(crate) parameters: Vec<(String, ValueType)>,
    pub(crate) body: CheckedBody,
    pub(crate) slots: usize,
    pub(crate) uses: Vec<PopulationUse>,
    /// The observation each model read of the body takes, by the clause kind
    /// the row is typed as (FR-104). A history update reads its post-state
    /// fields at `post` and `pre(..)` operands at `pre`.
    pub(crate) observations: Observations,
    /// The name each body slot was bound under, indexed by slot.
    pub(crate) slot_names: Vec<String>,
    /// The declaration site the body's locations name: the row
    /// expression, whose span tree [`Self::spans`] is (see [`row_site`]).
    pub(crate) site: String,
    /// The row expression's span tree, which locates the body's nodes.
    pub(crate) spans: qsl_forms::ExpressionSpans,
}

/// The checked value, without the row's source site and span tree.
impl std::fmt::Debug for CheckedRow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CheckedRow")
            .field("parameters", &self.parameters)
            .field("body", &self.body)
            .field("slots", &self.slots)
            .field("uses", &self.uses)
            .field("observations", &self.observations)
            .field("slot_names", &self.slot_names)
            .finish_non_exhaustive()
    }
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
                ValueType::Reference(self.member.clone()),
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
    /// The typed V2 body and key of this refinement (QSpec FR-375): its
    /// `abstract` root, a protocol subject's `protocol` clause and internal
    /// list, its keyed rows in canonical order and its lowered class
    /// catalog, with, for each `steps` entry in that order, the position of
    /// the source row it was written as (its [`Self::step_sources`] entry).
    /// Construction remains in the check core; the package writer can
    /// serialize the checked body without depending on JSON or minting a
    /// semantic identity.
    pub fn v2_emission(&self) -> Result<RefinementV2Emission, RefinementV2EncodingError> {
        use quire_contract_model::{
            RefinementAssume, RefinementEnsure, RefinementField, RefinementKey, RefinementObject,
            RefinementOpen, RefinementPopulation, RefinementStep, RefinementStepArgument,
            RefinementStepTarget,
        };
        let abstract_subject = self
            .abstract_subject_node
            .as_ref()
            .ok_or(RefinementV2EncodingError::MissingSubject)?;
        let concrete_subject = self
            .concrete_subject_node
            .as_ref()
            .ok_or(RefinementV2EncodingError::MissingSubject)?;
        let rows = self
            .row_keys
            .as_ref()
            .ok_or(RefinementV2EncodingError::MissingRows)?;
        let local = |key: &NodeKey| RefinementKey::Local(v2_node_id(key));
        // A leaf's typed V2 term: its canonical bytes, read back as the
        // term they spell.
        let term = |leaf: &crate::check::node_key::LeafTerm| {
            let bytes = quire_canonical::to_vec(leaf, quire_canonical::Limits::new(u64::MAX))
                .map_err(RefinementV2EncodingError::Term)?;
            serde_json::from_slice::<quire_contract_model::CheckedTerm>(&bytes)
                .map_err(RefinementV2EncodingError::Body)
        };
        let arguments = |arguments: &[Option<crate::check::node_key::LeafTerm>]| {
            arguments
                .iter()
                .map(|argument| match argument {
                    Some(leaf) => Ok(RefinementStepArgument::Term(Box::new(term(leaf)?))),
                    None => Ok(RefinementStepArgument::Open(RefinementOpen {
                        term: "open".into(),
                    })),
                })
                .collect::<Result<Vec<_>, RefinementV2EncodingError>>()
        };
        let mut keyed_populations: Vec<&KeyedPopulationRow> = rows.populations.iter().collect();
        keyed_populations
            .sort_by(|left, right| left.abstract_population.cmp(&right.abstract_population));
        let populations = keyed_populations
            .into_iter()
            .map(|row| RefinementPopulation {
                r#abstract: local(&row.abstract_population),
                concrete: v2_node_id(&row.concrete_population),
            })
            .collect();
        let mut keyed_objects: Vec<&KeyedObjectRow> = rows.objects.iter().collect();
        keyed_objects.sort_by(|left, right| left.abstract_type.cmp(&right.abstract_type));
        let objects = keyed_objects
            .into_iter()
            .map(|row| {
                let mut fields = row
                    .fields
                    .iter()
                    .map(|(name, expression)| {
                        Ok(RefinementField {
                            name: name.as_str().into(),
                            expression: term(expression)?,
                        })
                    })
                    .collect::<Result<Vec<_>, RefinementV2EncodingError>>()?;
                fields.sort_by(|left, right| left.name.as_bytes().cmp(right.name.as_bytes()));
                Ok(RefinementObject {
                    abstract_type: local(&row.abstract_type),
                    concrete_type: v2_node_id(&row.concrete_type),
                    fields,
                })
            })
            .collect::<Result<Vec<_>, RefinementV2EncodingError>>()?;
        // FR-375: rows naming a declaration by its structural node id, then
        // the class spelling; a row naming none (a memory row) after them.
        let mut ordered: Vec<&KeyedStepRow> = rows.steps.iter().collect();
        ordered.sort_by(|left, right| {
            (
                left.source_node.is_none(),
                left.source_node.as_ref(),
                left.source_class.as_bytes(),
            )
                .cmp(&(
                    right.source_node.is_none(),
                    right.source_node.as_ref(),
                    right.source_class.as_bytes(),
                ))
        });
        let mut steps = Vec::with_capacity(ordered.len());
        let mut step_sources = Vec::with_capacity(ordered.len());
        for row in ordered {
            let target = match &row.target {
                KeyedStepTarget::Operation {
                    operation,
                    receiver,
                    arguments: written,
                } => RefinementStepTarget::Operation {
                    operation: local(operation),
                    receiver: Box::new(term(receiver)?),
                    arguments: arguments(written)?,
                },
                KeyedStepTarget::ProtocolStep {
                    class,
                    node,
                    arguments: written,
                } => RefinementStepTarget::ProtocolStep {
                    class: class.as_str().into(),
                    node: node.as_ref().map(local),
                    arguments: arguments(written)?,
                },
                KeyedStepTarget::Stutter => RefinementStepTarget::Stutter,
                KeyedStepTarget::Any => RefinementStepTarget::Any,
            };
            steps.push(RefinementStep {
                source_class: row.source_class.as_str().into(),
                source_node: row.source_node.as_ref().map(v2_node_id),
                target,
            });
            step_sources.push(row.source);
        }
        fn fairness(
            rows: &[KeyedFairnessRow],
        ) -> Vec<(
            &NodeKey,
            quire_contract_model::RefinementFairnessKind,
            quire_contract_model::RefinementGranularity,
        )> {
            let mut rows: Vec<_> = rows
                .iter()
                .map(|row| {
                    (
                        &row.operation,
                        v2_fairness_kind(row.kind),
                        v2_granularity(row.granularity),
                    )
                })
                .collect();
            rows.sort_by(|left, right| {
                (left.0, fairness_label(left.1), granularity_label(left.2)).cmp(&(
                    right.0,
                    fairness_label(right.1),
                    granularity_label(right.2),
                ))
            });
            rows
        }
        let assume = fairness(&rows.assume)
            .into_iter()
            .map(|(operation, kind, granularity)| RefinementAssume {
                operation: v2_node_id(operation),
                kind,
                granularity,
            })
            .collect();
        let ensure = fairness(&rows.ensure)
            .into_iter()
            .map(|(operation, kind, granularity)| RefinementEnsure {
                operation: local(operation),
                kind,
                granularity,
            })
            .collect();
        let body = quire_contract_model::CheckedTerm::Refinement {
            abstract_key: Box::new(local(abstract_subject)),
            profile: quire_contract_model::CheckedArtifactRef {
                authority: "agent-ix".into(),
                identity: self.profile.identity.as_str().into(),
            },
            populations,
            objects,
            steps,
            histories: Vec::new(),
            assume,
            ensure,
            protocol: self
                .abstract_protocol
                .as_ref()
                .map(|clause| Box::new(local(clause))),
            abstract_internal: (!self.abstract_internal.is_empty()).then(|| {
                self.abstract_internal
                    .iter()
                    .map(|class| class.spelling().into())
                    .collect()
            }),
            class_catalog: self.class_catalog_keys.as_ref().map(v2_class_catalog),
        };
        let identity = self.v2_identity(concrete_subject)?;
        let mut dependencies: Vec<&NodeKey> = std::iter::once(abstract_subject)
            .chain(self.abstract_protocol.as_ref())
            .chain(
                self.class_catalog_keys
                    .iter()
                    .flat_map(KeyedClassCatalog::node_keys),
            )
            .chain(rows.node_keys())
            .collect();
        dependencies.sort();
        dependencies.dedup();
        Ok(RefinementV2Emission {
            identity,
            body,
            dependencies: dependencies.into_iter().cloned().collect(),
            step_sources,
            proof_eligible: self.proof_eligible(),
        })
    }

    /// FR-375: the class catalog of a model-abstract refinement, rebuilt
    /// once the unit's protocol clauses are checked, so the protocol clauses
    /// over the concrete model contribute their classes even when no step
    /// row selects one. The refinement's own check ran before any clause
    /// was checked.
    pub(crate) fn catalog_concrete_protocols(
        &mut self,
        declarations: &ClauseDeclarations<'_>,
        protocols: &[ProtocolView<'_>],
    ) -> Result<(), InternalFault> {
        if self.abstract_side != AbstractSide::Model {
            return Ok(());
        }
        let model = |alias: &str| {
            declarations
                .models
                .iter()
                .find(|model| model.alias() == Some(alias))
                .ok_or_else(|| {
                    InternalFault::new(
                        "check.refinement-class-catalog",
                        "the-models-of-a-refinement-are-admitted",
                    )
                })
        };
        self.class_catalog = catalog::build(catalog::CatalogBuildInput {
            concrete_alias: &self.concrete_model,
            concrete_model: model(&self.concrete_model)?,
            abstract_alias: &self.abstract_model,
            abstract_model: model(&self.abstract_model)?,
            abstract_classes: None,
            declarations,
            protocols,
            has_abstract_protocol: false,
        })?;
        Ok(())
    }

    /// The refinement node's structural id (QSpec FR-322): the declaring
    /// package, which also owns the local concrete subject, and the
    /// declaration path, the refinement's name. A declaration has one node,
    /// so its ordinal is zero.
    fn v2_identity(
        &self,
        concrete_subject: &NodeKey,
    ) -> Result<NodeKey, RefinementV2EncodingError> {
        crate::check::declared_node_key(
            concrete_subject.package().to_owned(),
            vec![self.name.clone()],
            0,
        )
        .map_err(RefinementV2EncodingError::Identity)
    }

    /// Whether this declaration is in the current V2 emission slice: its
    /// rows and its class catalog, when it has one, lowered to node keys, a
    /// protocol subject's clause keyed, and no history row (whose fields
    /// are not written yet).
    pub fn v2_emittable(&self) -> bool {
        let subject = match &self.abstract_side {
            AbstractSide::Model => self.abstract_internal.is_empty(),
            AbstractSide::Protocol { .. } => self.abstract_protocol.is_some(),
        };
        subject
            && self.row_keys.is_some()
            && self.history.is_empty()
            && self.class_catalog.is_some() == self.class_catalog_keys.is_some()
            && self.abstract_subject_node.is_some()
            && self.concrete_subject_node.is_some()
    }

    /// Whether S5 states step-simulation obligations for this refinement
    /// (QSpec FR-322's proof-eligible owner): no history row, no `any` step
    /// and no hidden abstract field. Such an owner carries one `generated`
    /// occurrence per obligation: the initial one, then one per step row.
    pub fn proof_eligible(&self) -> bool {
        self.history.is_empty()
            && !self
                .steps
                .iter()
                .any(|row| matches!(row.target, StepTarget::Any))
            && self.objects.iter().all(|object| object.hidden.is_empty())
    }

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

/// A refinement's typed V2 node: its key, its body, its dependencies, the
/// source row of each step and whether it is proof-eligible.
#[derive(Debug)]
pub struct RefinementV2Emission {
    /// The refinement's structural node key.
    pub identity: NodeKey,
    /// The FR-375 body.
    pub body: quire_contract_model::CheckedTerm,
    /// Every local node the body names, ascending by structural id.
    pub dependencies: Vec<NodeKey>,
    /// For each `steps` entry in body order, its source row's position among
    /// the authored step rows.
    pub step_sources: Vec<usize>,
    /// See [`CheckedRefinement::proof_eligible`].
    pub proof_eligible: bool,
}

/// A refinement's checked V2 body could not be constructed.
#[derive(Debug)]
pub enum RefinementV2EncodingError {
    /// The checked declaration has no selected model subject.
    MissingSubject,
    /// The authored rows are not lowered to node keys.
    MissingRows,
    /// A row term's canonical bytes did not read back as a V2 term.
    Body(serde_json::Error),
    /// A row term could not be canonically encoded.
    Term(quire_canonical::Error),
    /// The refinement's structural node id is not a valid node key.
    Identity(quire_exact::InvalidNodeKey),
}

/// QSpec FR-375's `class_catalog` member: each declared class's node and
/// binder types as local node references (the abstract side's as
/// `RefinementKey::Local`, its model being selected by this package), and
/// each memory class as its closed memory entry.
fn v2_class_catalog(catalog: &KeyedClassCatalog) -> quire_contract_model::RefinementClassCatalog {
    use quire_contract_model::{
        RefinementAbstractClass, RefinementConcreteClass, RefinementDrain, RefinementKey,
        RefinementMemoryClass, RefinementMemoryModel, RefinementTsoModel,
    };
    let memory = |class: KeyedMemoryClass| match class {
        KeyedMemoryClass::Drain(model) => RefinementMemoryClass::Drain {
            memory_model: match model {
                qsl_foundation::MemoryModelName::Sc => RefinementMemoryModel::Sc,
                qsl_foundation::MemoryModelName::Tso => RefinementMemoryModel::Tso,
                qsl_foundation::MemoryModelName::Ra => RefinementMemoryModel::Ra,
            },
            drain: RefinementDrain::StoreBufferFlush,
        },
        KeyedMemoryClass::Flush => RefinementMemoryClass::Flush {
            memory_model: RefinementTsoModel::Tso,
        },
    };
    quire_contract_model::RefinementClassCatalog {
        concrete: catalog
            .concrete()
            .iter()
            .map(|class| match class {
                KeyedCatalogClass::Declared {
                    class,
                    node,
                    binder_types,
                } => RefinementConcreteClass::Declared {
                    class: class.as_str().into(),
                    node: v2_node_id(node),
                    binder_types: binder_types.iter().map(v2_node_id).collect(),
                },
                KeyedCatalogClass::Memory(class) => RefinementConcreteClass::Memory(memory(*class)),
            })
            .collect(),
        r#abstract: catalog
            .abstract_side()
            .iter()
            .map(|class| match class {
                KeyedCatalogClass::Declared {
                    class,
                    node,
                    binder_types,
                } => RefinementAbstractClass::Declared {
                    class: class.as_str().into(),
                    node: RefinementKey::Local(v2_node_id(node)),
                    binder_types: binder_types
                        .iter()
                        .map(|key| RefinementKey::Local(v2_node_id(key)))
                        .collect(),
                },
                KeyedCatalogClass::Memory(class) => RefinementAbstractClass::Memory(memory(*class)),
            })
            .collect(),
    }
}

fn v2_fairness_kind(kind: FairnessKind) -> quire_contract_model::RefinementFairnessKind {
    match kind {
        FairnessKind::Weak => quire_contract_model::RefinementFairnessKind::Weak,
        FairnessKind::Strong => quire_contract_model::RefinementFairnessKind::Strong,
    }
}

fn v2_granularity(granularity: FairnessGranularity) -> quire_contract_model::RefinementGranularity {
    match granularity {
        FairnessGranularity::Each => quire_contract_model::RefinementGranularity::Each,
        FairnessGranularity::Whole => quire_contract_model::RefinementGranularity::Whole,
    }
}

/// The wire spelling a fairness row orders by after its operation.
fn fairness_label(kind: quire_contract_model::RefinementFairnessKind) -> &'static str {
    match kind {
        quire_contract_model::RefinementFairnessKind::Weak => "weak",
        quire_contract_model::RefinementFairnessKind::Strong => "strong",
    }
}

/// The wire spelling a fairness row orders by after its kind.
fn granularity_label(granularity: quire_contract_model::RefinementGranularity) -> &'static str {
    match granularity {
        quire_contract_model::RefinementGranularity::Each => "each",
        quire_contract_model::RefinementGranularity::Whole => "whole",
    }
}

/// `key` as a V2 graph node id: the same package, path and ordinal. A
/// nested operand id is converted from an explicit stack, so a deep path
/// uses no call stack proportional to its depth.
fn v2_node_id(key: &NodeKey) -> quire_contract_model::CheckedNodeId {
    use quire_contract_model::{CheckedNodeId, CheckedNodePathSegment};
    use quire_exact::{NodeKeyView, NodePathIter, PathSegmentView};
    struct Frame<'a> {
        package: &'a str,
        ordinal: u64,
        segments: NodePathIter<'a>,
        path: Vec<CheckedNodePathSegment>,
    }
    fn frame(view: NodeKeyView<'_>) -> Frame<'_> {
        Frame {
            package: view.package(),
            ordinal: view.ordinal(),
            segments: view.path(),
            path: Vec::new(),
        }
    }
    let mut current = frame(key.view());
    let mut parents = Vec::new();
    loop {
        match current.segments.next() {
            Some(PathSegmentView::Text(text)) => {
                current.path.push(CheckedNodePathSegment::Name(text.into()));
            }
            Some(PathSegmentView::Node(nested)) => {
                parents.push(std::mem::replace(&mut current, frame(nested)));
            }
            None => {
                let id = CheckedNodeId {
                    package: current.package.into(),
                    path: std::mem::take(&mut current.path),
                    ordinal: current.ordinal,
                };
                let Some(mut parent) = parents.pop() else {
                    return id;
                };
                parent.path.push(CheckedNodePathSegment::Node(Box::new(id)));
                current = parent;
            }
        }
    }
}

/// `population A::p from C::q`, resolved.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PopulationRow {
    /// The abstract population.
    pub abstract_population: DeclarationKey,
    /// The population declaration's canonical model-node key. FR-140 reads
    /// population rows in this key order, independently of source names.
    pub order_key: NodeKey,
    /// The concrete population it is the key-preserving image of.
    pub concrete_population: DeclarationKey,
}

/// `field = e` of an object row, resolved against an object type or a
/// state-machine field set.
#[derive(Debug)]
pub struct FieldRow {
    /// The abstract field.
    pub field: DeclarationKey,
    /// Its definition over the concrete object.
    pub expression: CheckedRow,
}

/// `object A::T from C::U { ... }`, resolved for an object type or a
/// state-machine field set.
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
    /// The class's alias-free spelling (QSpec FR-375 `source_class`): its
    /// FR-432 spelling with the concrete alias removed, such as
    /// `Cell::increment`, `fork Flow::par` or `memory drain`.
    pub(crate) spelling: String,
    /// An operation row's FR-146 record roots; `None` for a protocol row.
    pub(crate) roots: Option<RowRoots>,
}

impl StepRow {
    /// The concrete operation of an operation row, as a record references
    /// it: its declaring type and name; `None` for a protocol row.
    pub fn concrete_operation(&self) -> Option<(EffectiveId, &str)> {
        self.roots
            .as_ref()
            .map(|roots| (roots.operation.0.clone(), roots.operation.1.as_str()))
    }
}

/// FR-146 "Claim extent": what an operation row's `Step` record ranges over
/// besides the populations its contract clauses and the state mapping walk.
#[derive(Clone)]
pub(crate) struct RowRoots {
    /// The concrete operation, as a record references it (ADR-017 AR-4).
    pub(crate) operation: (EffectiveId, String),
    /// The receiver's type, the result's and each parameter's, then the
    /// abstract parameter domain of each `_` argument.
    pub(crate) types: Vec<ValueType>,
    /// The one concrete population covering the receiver's type (FR-104),
    /// or every one covering it in ascending `DeclarationKey` order.
    pub(crate) context: Result<Option<PopulationDomain>, Vec<DeclarationKey>>,
    /// The row's operation name, where an ambiguous context refuses.
    pub(crate) name_span: Span,
}

/// The checked roots, without the row's source span, as [`CheckedRow`]'s
/// value omits its site.
impl std::fmt::Debug for RowRoots {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RowRoots")
            .field("operation", &self.operation)
            .field("types", &self.types)
            .field("context", &self.context)
            .finish_non_exhaustive()
    }
}

/// FR-146 "Claim extent": the populations a model subject's records range
/// over beyond each row's own roots.
#[derive(Clone, Debug, Default)]
pub(crate) struct SimulationRoots {
    /// The population covering each concrete object type the state mapping
    /// maps: the `Initial` record's roots.
    pub(crate) initial: BTreeSet<PopulationDomain>,
    /// Each population a `reaches` in the state mapping walks.
    pub(crate) mapping: BTreeSet<PopulationDomain>,
}

impl StepRow {
    /// The class's alias-free spelling.
    pub fn spelling(&self) -> &str {
        &self.spelling
    }
}

/// The declaration site of the row expression at `span`: one name per row
/// expression of the unit, under which its span tree locates its nodes.
pub(crate) fn row_site(span: Span) -> String {
    format!("refinement row {}..{}", span.start, span.end)
}

/// The original source region of one checked, authored step row (FR-136,
/// QSpec FR-375-AC-22). The containing [`CheckedRefinement`] is its owner;
/// `ordinal` is its position among that refinement's authored step rows,
/// before S4 reorders the semantic `steps` array.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RefinementStepSource {
    ordinal: usize,
    span: Span,
}

impl RefinementStepSource {
    /// The source-order ordinal among authored `step` rows.
    pub const fn ordinal(&self) -> usize {
        self.ordinal
    }

    /// The original checked source span of the entire row.
    pub const fn span(&self) -> Span {
        self.span
    }
}

/// The origin of a checked refinement fairness member (FR-137).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RefinementFairnessOrigin {
    /// Written as an `assume` or `ensure` row.
    Authored,
    /// Derived from default scheduler fairness over a concrete protocol.
    Scheduler,
}

/// A protocol thread target for a derived scheduler fairness member (FR-137).
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RefinementThreadTarget {
    /// The protocol's root thread.
    Root,
    /// One branch of a `parallel`, named by the parallel's static path and
    /// the branch label.
    Branch {
        /// The `parallel` node's static declaration path.
        parallel: String,
        /// The branch label.
        branch: String,
    },
    /// One compensation template's thread target.
    CompensationTemplate {
        /// The compensation template's name.
        name: String,
    },
}

/// One member of a checked refinement fairness set (FR-137).
///
/// Authored members retain FR-123's operation constraint. Scheduler members
/// name a protocol thread target and are always `weak whole`; keeping the two
/// forms distinct prevents a thread obligation from being treated as an
/// operation fairness constraint by model-refinement consumers.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RefinementFairness {
    /// An authored operation constraint.
    Authored(FairnessConstraint),
    /// A scheduler constraint over a protocol thread.
    Scheduler(RefinementThreadTarget),
}

impl RefinementFairness {
    /// The origin of this fairness member.
    pub const fn origin(&self) -> RefinementFairnessOrigin {
        match self {
            Self::Authored(_) => RefinementFairnessOrigin::Authored,
            Self::Scheduler(_) => RefinementFairnessOrigin::Scheduler,
        }
    }

    /// The fairness kind; scheduler members are always weak.
    pub fn kind(&self) -> FairnessKind {
        match self {
            Self::Authored(constraint) => constraint.kind(),
            Self::Scheduler(_) => FairnessKind::Weak,
        }
    }

    /// The fairness granularity; scheduler members are always whole.
    pub fn granularity(&self) -> FairnessGranularity {
        match self {
            Self::Authored(constraint) => constraint.granularity(),
            Self::Scheduler(_) => FairnessGranularity::Whole,
        }
    }

    /// The authored operation constraint, or `None` for a scheduler member.
    pub fn operation_constraint(&self) -> Option<&FairnessConstraint> {
        match self {
            Self::Authored(constraint) => Some(constraint),
            Self::Scheduler(_) => None,
        }
    }

    /// The scheduler thread target, or `None` for an authored member.
    pub fn thread_target(&self) -> Option<&RefinementThreadTarget> {
        match self {
            Self::Authored(_) => None,
            Self::Scheduler(target) => Some(target),
        }
    }
}

/// A refinement between two models that checked (ADR-020 RM-1 to RM-6).
#[derive(Debug)]
pub struct CheckedRefinement {
    pub(crate) name: String,
    pub(crate) profile: ProfileSelection,
    pub(crate) abstract_model: String,
    /// The canonical selected package for the abstract side; its identity
    /// is independent of the source alias (FR-135).
    pub(crate) abstract_model_selection: DomainPackageRef,
    pub(crate) abstract_side: AbstractSide,
    pub(crate) abstract_internal: Vec<AbstractStepClass>,
    pub(crate) concrete_model: String,
    /// The canonical selected package for the concrete side; its identity
    /// is independent of the source alias (FR-135).
    pub(crate) concrete_model_selection: DomainPackageRef,
    pub(crate) populations: Vec<PopulationRow>,
    pub(crate) objects: Vec<ObjectRow>,
    pub(crate) steps: Vec<StepRow>,
    /// Every authored step row's source provenance, including rows whose
    /// mapping makes this refinement ineligible for a proof. This remains
    /// attached to the checked owner independently of any S5 proof records.
    pub(crate) step_sources: Vec<RefinementStepSource>,
    pub(crate) history: Vec<HistoryRow>,
    pub(crate) concrete_fairness: Vec<RefinementFairness>,
    pub(crate) abstract_fairness: Vec<RefinementFairness>,
    pub(crate) concrete_object_types: Vec<EffectiveId>,
    /// The complete checked classes of protocol or memory subjects, derived
    /// independently of the authored step rows (FR-375, QSpec FR-464).
    pub(crate) class_catalog: Option<RefinementClassCatalog>,
    /// [`Self::class_catalog`] lowered to the package graph's node keys,
    /// once every class node is minted; absent while a contributing
    /// protocol clause has no node.
    pub(crate) class_catalog_keys: Option<KeyedClassCatalog>,
    /// The authored rows lowered to node keys, once every node they name is
    /// minted; absent while one is not (QSpec FR-375).
    pub(crate) row_keys: Option<KeyedRefinementRows>,
    /// The protocol clause the step rows select; absent for a model subject.
    /// The clause's node exists once the unit's protocol nodes are minted
    /// (FR-092), so the pipeline fills it after the check.
    pub(crate) concrete_protocol: Option<NodeKey>,
    /// The abstract protocol clause of a protocol subject, once the unit's
    /// protocol nodes are minted; absent for a model subject and for a
    /// clause that has no node.
    pub(crate) abstract_protocol: Option<NodeKey>,
    /// The selected local state-machine nodes, filled from the checked model
    /// selections after protocol refinement resolution.
    pub(crate) abstract_subject_node: Option<NodeKey>,
    pub(crate) concrete_subject_node: Option<NodeKey>,
    /// The full authored declaration region used by the V2 relation-node
    /// occurrence and source map.
    pub(crate) declaration_span: Span,
    /// The `declaration` occurrence of the refinement's V2 node, which keys
    /// its FR-144 requirement record; set when S3 writes the record.
    pub(crate) requirement: Option<qsl_foundation::source::provenance::OccurrenceKey>,
    /// FR-146: the populations a model subject's per-step simulation
    /// records range over; `None` when it writes none.
    pub(crate) simulation: Option<SimulationRoots>,
    /// FR-146: the per-step simulation records S3 wrote, `Initial` first and
    /// then one per row in `steps` order.
    pub(crate) simulations: Vec<super::step_simulation::StepSimulationEntry>,
    /// For each authored step row, in source order, its zero-based position
    /// in the V2 node's `steps` array (QSpec FR-375); empty for a
    /// refinement S4 does not emit.
    pub(crate) row_positions: Vec<quire_semantic_value::step_simulation::StepRowRef>,
}

impl CheckedRefinement {
    /// FR-144: the occurrence that keys the refinement's requirement record
    /// in [`super::CheckedGraph::requirements`], the `declaration`
    /// occurrence of its V2 node. `None` for a refinement S3 wrote no record
    /// for, which a package that emits never holds.
    pub fn requirement_occurrence(
        &self,
    ) -> Option<&qsl_foundation::source::provenance::OccurrenceKey> {
        self.requirement.as_ref()
    }

    /// FR-146: the per-step simulation records S3 wrote beside the
    /// refinement record, `Initial` first and then one `Step` per row in
    /// `steps` order, each with its ADR-013 O-07 occurrence. Empty for a
    /// refinement with a hidden field, a history field or an `any` row.
    ///
    /// Interim, withdrawn when the spec defines a protocol row's `Step`
    /// record extent (STD-493 comment c699936f): a protocol subject writes
    /// none yet, though its rows have their FR-375 positions.
    pub fn step_simulations(&self) -> &[super::step_simulation::StepSimulationEntry] {
        &self.simulations
    }

    /// FR-146 "Row order": the [`StepRowRef`] of the authored step row at
    /// `source` (its position in [`Self::steps`]): its position in the V2
    /// node's `steps` array, which QSpec FR-375 sorts. `None` for a
    /// refinement S4 does not emit.
    ///
    /// [`StepRowRef`]: quire_semantic_value::step_simulation::StepRowRef
    pub fn step_row_ref(
        &self,
        source: usize,
    ) -> Option<quire_semantic_value::step_simulation::StepRowRef> {
        self.row_positions.get(source).copied()
    }

    /// FR-144 (ADR-020 "Negotiation"): the TP-5 property form of the
    /// refinement's record, with a liveness half exactly when `F_A` is
    /// non-empty.
    pub fn property_form(&self) -> quire_semantic_value::fairness::PropertyForm {
        quire_semantic_value::fairness::PropertyForm::Refinement {
            liveness: !self.abstract_fairness.is_empty(),
        }
    }

    /// FR-144 "Requirement record": the requirements of the refinement's one
    /// record, keyed at its V2 node `node`: `temporal-satisfaction` over the
    /// concrete model subject, unbounded over the infinite-trace domain
    /// rooted at the node, with the fairness kinds of `F_C` and `F_A` and the
    /// TP-5 form.
    pub(crate) fn requirements(&self, node: NodeKey) -> crate::family::Requirements {
        let domains = std::collections::BTreeMap::from([(
            qsl_foundation::bound::DomainKey::Node {
                node,
                path: Vec::new(),
            },
            qsl_foundation::bound::DomainKind::InfiniteTrace,
        )]);
        crate::family::Requirements::new(
            super::Capability::TemporalSatisfaction,
            crate::family::ClaimExtent::from_domains(domains),
        )
        .over_model_subject()
        .with_fairness(
            self.concrete_fairness
                .iter()
                .chain(&self.abstract_fairness)
                .map(RefinementFairness::kind),
        )
        .with_property_form(self.property_form())
    }

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

    /// The selected package identity for the abstract model.
    pub fn abstract_model_selection(&self) -> &DomainPackageRef {
        &self.abstract_model_selection
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

    /// The selected package identity for the concrete model.
    pub fn concrete_model_selection(&self) -> &DomainPackageRef {
        &self.concrete_model_selection
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

    /// Every authored step row's source region in source-row order.
    pub fn step_sources(&self) -> &[RefinementStepSource] {
        &self.step_sources
    }

    /// The protocol clause the step rows select (FR-136 "Selecting the
    /// concrete protocol"); `None` for a model subject.
    pub fn concrete_protocol(&self) -> Option<&NodeKey> {
        self.concrete_protocol.as_ref()
    }

    /// The abstract model's `model_root` node (QSpec FR-375).
    pub fn abstract_subject_node(&self) -> Option<&NodeKey> {
        self.abstract_subject_node.as_ref()
    }

    /// The concrete model's `model_root` node (QSpec FR-375).
    pub fn concrete_subject_node(&self) -> Option<&NodeKey> {
        self.concrete_subject_node.as_ref()
    }

    /// The authored refinement declaration span.
    pub fn declaration_span(&self) -> Span {
        self.declaration_span
    }

    /// The complete checked class catalogs for a refinement whose subjects
    /// contribute protocol or memory step classes. Model-only refinements
    /// have no catalog.
    pub fn class_catalog(&self) -> Option<&RefinementClassCatalog> {
        self.class_catalog.as_ref()
    }

    /// The abstract protocol clause's node of a protocol subject, once it is
    /// minted.
    pub fn abstract_protocol(&self) -> Option<&NodeKey> {
        self.abstract_protocol.as_ref()
    }

    /// The class catalog lowered to node keys, when every class node of it
    /// is in the package graph.
    pub fn class_catalog_keys(&self) -> Option<&KeyedClassCatalog> {
        self.class_catalog_keys.as_ref()
    }

    /// Every row expression of the refinement: each field row, each step
    /// target's receiver and arguments, and each history update.
    pub(crate) fn rows(&self) -> impl Iterator<Item = &CheckedRow> {
        let fields = self
            .objects
            .iter()
            .flat_map(|object| object.fields.iter().map(|field| &field.expression));
        let steps = self.steps.iter().flat_map(|step| match &step.target {
            StepTarget::Abstract {
                receiver,
                arguments,
                ..
            } => std::iter::once(receiver)
                .chain(arguments.iter().flatten())
                .collect::<Vec<_>>(),
            StepTarget::Node { arguments, .. } => arguments.iter().flatten().collect(),
            StepTarget::Stutter | StepTarget::Any => Vec::new(),
        });
        let updates = self
            .history
            .iter()
            .flat_map(|history| history.updates.iter().map(|update| &update.expression));
        fields.chain(steps).chain(updates)
    }

    /// Whether every row expression has a node form: a whole population, a
    /// history read and a `seq` have none yet (FR-139, FR-138; FR-093
    /// gives no row for them), so a refinement with one is not emitted.
    pub(crate) fn rows_have_nodes(&self) -> bool {
        self.rows().all(|row| {
            row.body().descendants().into_iter().all(|node| {
                !matches!(
                    node.kind(),
                    NodeKind::PopulationObjects { .. }
                        | NodeKind::History { .. }
                        | NodeKind::Seq { .. }
                )
            })
        })
    }

    /// The history rows, in source order (FR-138).
    pub fn history(&self) -> &[HistoryRow] {
        &self.history
    }

    /// The concrete fairness set `F_C`, with the origin of each member.
    pub fn concrete_fairness(&self) -> &[RefinementFairness] {
        &self.concrete_fairness
    }

    /// The authored operation constraints of `F_C`.
    ///
    /// Scheduler members of a concrete protocol subject do not name model
    /// operations and are omitted. Model-refinement engines consume this
    /// iterator while protocol consumers retain the complete set through
    /// [`Self::concrete_fairness`].
    pub fn concrete_operation_fairness(&self) -> impl Iterator<Item = &FairnessConstraint> + '_ {
        self.concrete_fairness
            .iter()
            .filter_map(RefinementFairness::operation_constraint)
    }

    /// The abstract fairness set `F_A`, with the origin of each member.
    pub fn abstract_fairness(&self) -> &[RefinementFairness] {
        &self.abstract_fairness
    }

    /// The authored operation constraints of `F_A`.
    pub fn abstract_operation_fairness(&self) -> impl Iterator<Item = &FairnessConstraint> + '_ {
        self.abstract_fairness
            .iter()
            .filter_map(RefinementFairness::operation_constraint)
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

/// One field-bearing declaration a refinement can map.
struct ObjectType {
    name: String,
    id: EffectiveId,
    key: DeclarationKey,
    kind: RefinementTypeKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RefinementTypeKind {
    ObjectType,
    StateMachine,
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
                        .object_type(id.clone())
                        .ok_or("admitted-object-type-is-in-the-environment")?
                        .name()
                        .to_owned(),
                    id,
                    key: key.clone(),
                    kind: RefinementTypeKind::ObjectType,
                })
            })
            .collect()
    }

    /// Every object-type or state-machine field set a refinement can map.
    fn refinement_types(&self, types: &TypeEnvironment) -> Result<Vec<ObjectType>, &'static str> {
        let mut found = self.object_types(types)?;
        found.extend(
            self.model
                .state_machine_types()
                .map(|(key, id)| {
                    let name = types
                        .object_type(id.clone())
                        .ok_or("admitted-state-machine-is-in-the-type-environment")?
                        .name()
                        .to_owned();
                    Ok(ObjectType {
                        name,
                        id,
                        key: key.clone(),
                        kind: RefinementTypeKind::StateMachine,
                    })
                })
                .collect::<Result<Vec<_>, &'static str>>()?,
        );
        Ok(found)
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
        (ValueType::Reference(sub), ValueType::Reference(sup)) => {
            types.conforms(sub.clone(), sup.clone())
        }
        (ValueType::Composite(sub), ValueType::Composite(sup)) => {
            types.composite_conforms(sub.clone(), sup.clone())
        }
        _ => actual == required,
    }
}

/// Whether `value_type` holds a `Reference` below its top level (FR-135
/// "Refinement features the build does not check"). A plain `Reference<T>`
/// is lifted by the object map; a reference inside an option, record, tuple,
/// union or collection is not. A tuple is a record by position, so it is
/// caught with the record and the union.
fn nests_reference(types: &TypeEnvironment, value_type: &ValueType) -> Result<bool, &'static str> {
    match value_type {
        ValueType::Reference(_) => Ok(false),
        other => holds_reference(types, other, &mut Vec::new()),
    }
}

/// Whether `value_type` is or contains a `Reference`. `seen` holds the
/// composite declarations being walked, so a declaration that names itself
/// is walked once. A composite with no declaration is an internal fault.
fn holds_reference(
    types: &TypeEnvironment,
    value_type: &ValueType,
    seen: &mut Vec<NodeKey>,
) -> Result<bool, &'static str> {
    let any = |candidates: &mut dyn Iterator<Item = &ValueType>,
               seen: &mut Vec<NodeKey>|
     -> Result<bool, &'static str> {
        for candidate in candidates {
            if holds_reference(types, candidate, seen)? {
                return Ok(true);
            }
        }
        Ok(false)
    };
    match value_type {
        ValueType::Reference(_) => Ok(true),
        ValueType::Option(payload) => holds_reference(types, payload, seen),
        ValueType::Collection(collection) => holds_reference(types, collection.element(), seen),
        ValueType::Map(map) => holds_reference(types, map.values(), seen),
        ValueType::Composite(key) => {
            if seen.contains(key) {
                return Ok(false);
            }
            seen.push(key.clone());
            let declaration = types
                .composite(key.clone())
                .ok_or("composite-type-has-a-declaration")?;
            match declaration.shape() {
                CompositeShape::Record(fields) => {
                    any(&mut fields.iter().map(FieldDeclaration::value_type), seen)
                }
                CompositeShape::Tuple(positions) => any(&mut positions.iter(), seen),
                CompositeShape::Union(members) => any(
                    &mut members.iter().flat_map(|member| member.positions()),
                    seen,
                ),
            }
        }
        ValueType::Boolean
        | ValueType::Integer
        | ValueType::Int(_)
        | ValueType::Rational(_)
        | ValueType::Decimal(_)
        | ValueType::Float(_)
        | ValueType::Quantity(_)
        | ValueType::BoundedQuantity(_)
        | ValueType::Text(_)
        | ValueType::String
        | ValueType::Uuid
        | ValueType::Timestamp
        | ValueType::Enum(_)
        | ValueType::Population(_) => Ok(false),
    }
}

/// `value_type` with each reference to an abstract object type that an
/// object row maps replaced by a reference to the concrete type it is mapped
/// from (ADR-020 RM-3 reference lifting).
fn lift(lifting: &BTreeMap<EffectiveId, EffectiveId>, value_type: &ValueType) -> ValueType {
    match value_type {
        ValueType::Reference(object) => {
            ValueType::Reference(lifting.get(object).unwrap_or(object).clone())
        }
        ValueType::Option(payload) => ValueType::option(lift(lifting, payload)),
        ValueType::Collection(collection) => {
            ValueType::collection(quire_exact::CollectionType::new(
                collection.kind(),
                lift(lifting, collection.element()),
                collection.bound(),
            ))
        }
        ValueType::Map(map) => ValueType::Map(std::sync::Arc::new(quire_exact::MapType::new(
            lift(lifting, map.values()),
            map.bound(),
        ))),
        other @ (ValueType::Boolean
        | ValueType::Integer
        | ValueType::Int(_)
        | ValueType::Rational(_)
        | ValueType::Decimal(_)
        | ValueType::Float(_)
        | ValueType::Quantity(_)
        | ValueType::BoundedQuantity(_)
        | ValueType::Text(_)
        | ValueType::String
        | ValueType::Uuid
        | ValueType::Timestamp
        | ValueType::Enum(_)
        | ValueType::Composite(_)
        | ValueType::Population(_)) => other.clone(),
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
    env: &RefinementEnv<'c>,
    identity: qsl_foundation::IdentityLimits,
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
    if matches!(form.abstract_side, AbstractSideForm::Model(_))
        && abstract_side
            .model
            .relationship_declarations()
            .next()
            .is_some()
    {
        findings.refuse(
            abstract_alias.span,
            RefinementCause::AbstractRelationships {
                form: "relationship",
                refinement: form.name.name.clone(),
                profile: declaration.selection.identity.clone(),
            },
        );
        return (RefinementCheck::Refused(findings.refusals), nodes);
    }
    // QSpec FR-430: a timed concrete model refines an untimed abstract one;
    // a timed abstract model has no reading as a refinement's abstract side.
    if env.timed_models.contains(&abstract_side.alias) {
        findings.refuse(
            abstract_alias.span,
            RefinementCause::TimedAbstract {
                model: abstract_side.alias.to_owned(),
            },
        );
        return (RefinementCheck::Refused(findings.refusals), nodes);
    }
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
        clock_reading: env.clock_reading,
        protocols,
        abstract_side,
        concrete_side,
        identity,
        findings,
        nodes: &mut nodes,
        lifting: BTreeMap::new(),
        maps_complete: true,
        baseline: 0,
        profile: declaration.selection.identity.clone(),
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
    let simulation = (protocol.is_none() && history.is_empty())
        .then(|| checker.simulation_roots(&objects, &operations, form.name.span))
        .flatten();
    let concrete_object_types = checker
        .concrete_side
        .model
        .object_types()
        .map(|(_, id)| id)
        .collect();
    let abstract_model_selection = checker.abstract_side.model.selection().clone();
    let concrete_model_selection = checker.concrete_side.model.selection().clone();
    let class_catalog = match catalog::build(catalog::CatalogBuildInput {
        concrete_alias: checker.concrete_side.alias,
        concrete_model: checker.concrete_side.model,
        abstract_alias: checker.abstract_side.alias,
        abstract_model: checker.abstract_side.model,
        abstract_classes: checker.protocol.as_ref(),
        declarations,
        protocols: env.protocols,
        has_abstract_protocol: checker.protocol_subject && checker.protocol.is_some(),
    }) {
        Ok(catalog) => catalog,
        Err(fault) => {
            return (
                RefinementCheck::Refused(vec![CheckRefusal {
                    location: Location::root(Origin::Expression),
                    cause: CheckCause::InternalFault(Box::new(KeyFault::StageFault(fault))),
                }]),
                nodes,
            );
        }
    };
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
        abstract_model_selection,
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
        concrete_model_selection,
        populations,
        objects,
        steps: Vec::new(),
        step_sources: form
            .rows
            .iter()
            .filter_map(|row| match row {
                RefinementRowForm::Step(row) => Some(row.span),
                RefinementRowForm::Population(_)
                | RefinementRowForm::Object(_)
                | RefinementRowForm::History(_)
                | RefinementRowForm::Fairness(_) => None,
            })
            .enumerate()
            .map(|(ordinal, span)| RefinementStepSource { ordinal, span })
            .collect(),
        history,
        concrete_fairness,
        abstract_fairness,
        concrete_object_types,
        class_catalog,
        class_catalog_keys: None,
        row_keys: None,
        concrete_protocol: None,
        abstract_protocol: None,
        abstract_subject_node: None,
        concrete_subject_node: None,
        requirement: None,
        simulation,
        simulations: Vec::new(),
        row_positions: Vec::new(),
        declaration_span: form.span,
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
            identity,
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
    /// Whether an abstract protocol header is intentionally deferred because
    /// another unit refusal stops the check before protocol clauses run.
    pub(crate) defer_abstract_protocol_resolution: bool,
    /// The aliases of the unit's timed models: those a `time` declaration
    /// names (QSpec FR-430).
    pub(crate) timed_models: &'e [&'e str],
    /// The unit's clock-reading declarations, whose calls read clocks
    /// (FR-230).
    pub(crate) clock_reading: &'e [super::assemble::ClockReadingDeclaration],
}

/// One protocol clause of the unit and its check: `checked` is `None` when
/// the clause was refused by its own checks.
pub(crate) struct ProtocolView<'e> {
    pub(crate) form: &'e qsl_forms::ProtocolDeclarationForm,
    pub(crate) checked: Option<&'e super::protocol_clause::CheckedProtocol>,
    /// The attempts of the clause, as `protocol_clause::check` bound them.
    pub(crate) attempts: &'e [super::protocol_clause::BoundAttempt],
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
        if report && !env.defer_abstract_protocol_resolution {
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
                .as_ref()
                .is_some_and(|context| object_types.iter().any(|object| object.id == *context))
                || role.model_selection.as_deref()
                    == Some(abstract_side.model.selection().identity.as_str())
        })
    };
    if !acts_on_abstract_model {
        if report {
            findings.malformed(&clause.name, clause.span);
        }
        return ListSubject::Unresolved;
    }
    ListSubject::Protocol(Box::new(ProtocolClasses::of(
        view.form,
        checked,
        view.attempts,
        types,
    )))
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
    /// The unit's clock-reading declarations (FR-230).
    clock_reading: &'c [super::assemble::ClockReadingDeclaration],
    /// The unit's protocol declarations, which a step row may select.
    protocols: &'c [ProtocolDeclarationForm],
    abstract_side: Side<'c>,
    concrete_side: Side<'c>,
    identity: qsl_foundation::IdentityLimits,
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
    /// The identity of the profile the declaration selects.
    profile: String,
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

/// The first expression form in a refinement row that the governing
/// infinite-trace profile does not admit, with the token span and the layer
/// that admits it (QSpec FR-375 "Row layers").
fn unsupported_row_form(expression: &ExpressionForm) -> Option<(&'static str, Span)> {
    let required: std::collections::BTreeSet<_> = HeaderLayer::ValueComplete
        .admitted_forms()
        .into_iter()
        .chain(HeaderLayer::ModelComplete.admitted_forms())
        .collect();
    let predicates = std::collections::BTreeSet::new();
    let mut unsupported = None;
    super::layer::visit_located(
        &expression.expression,
        &expression.spans,
        expression
            .spans
            .at(&[])
            .unwrap_or(Span { start: 0, end: 0 }),
        |node, span| {
            let layer = match node {
                qsl_forms::ExprNode::ProtocolReference { .. } => Some("quire.protocol.complete/v1"),
                qsl_forms::ExprNode::StepLabel { .. } | qsl_forms::ExprNode::Stutter => {
                    Some("quire.temporal.complete/v1")
                }
                _ => super::layer::expression_form(node, &predicates).and_then(|form| {
                    if required.contains(&form) {
                        None
                    } else {
                        HeaderLayer::ALL
                            .into_iter()
                            .find(|layer| layer.admits(form))
                            .map(HeaderLayer::identity)
                    }
                }),
            };
            let layer = layer?;
            unsupported = Some((layer, span));
            Some(())
        },
    );
    unsupported
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
                        member: member.id.clone(),
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
                let Some(order_key) =
                    super::lowering::population_node(&target.record.key, self.identity)
                        .ok()
                        .flatten()
                        .map(|node| node.key())
                else {
                    self.fault("admitted-abstract-population-has-a-population-node-key");
                    continue;
                };
                rows.push(PopulationRow {
                    abstract_population: target.record.key.clone(),
                    order_key,
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
            .filter(|object| object.kind == RefinementTypeKind::ObjectType)
            .filter(|object| {
                members
                    .iter()
                    .any(|member| self.types().conforms(object.id.clone(), member.clone()))
            })
            .map(|object| object.id.clone())
            .collect()
    }

    /// FR-135 "Object map".
    fn objects(
        &mut self,
        form: &RefinementForm,
        cx: &mut CheckContext<'a, ClauseDeclarations<'a>>,
    ) -> Vec<ObjectRow> {
        let abstract_types = {
            let found = self.abstract_side.refinement_types(self.types());
            self.or_fault(found)
        };
        let concrete_types = {
            let found = self.concrete_side.refinement_types(self.types());
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
            if target.kind != source.kind {
                self.findings.malformed(&row.source.name, row.source.span);
                continue;
            }
            if target.kind == RefinementTypeKind::ObjectType {
                // An object-type mapping is admitted through the paired
                // populations; a state-machine field set is not population
                // bound (FR-135, QSpec FR-375).
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
                            .any(|member| self.types().conforms(source.id.clone(), member))
                    })
                {
                    self.findings.malformed(&row.source.name, row.source.span);
                    continue;
                }
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
            first_rows.insert(target.id.clone(), row.span);
            self.lifting.insert(target.id.clone(), source.id.clone());
            resolved.push((row, target, source));
        }
        let mut reported = std::collections::BTreeSet::new();
        for population in &named {
            for id in self.admitted(population, &abstract_types) {
                if !first_rows.contains_key(&id) && reported.insert(id.clone()) {
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
        for machine in abstract_types
            .iter()
            .filter(|ty| ty.kind == RefinementTypeKind::StateMachine)
        {
            if !first_rows.contains_key(&machine.id) && reported.insert(machine.id.clone()) {
                self.findings.refuse(
                    form.name.span,
                    RefinementCause::MissingRow {
                        class: RowClass::Object,
                        name: machine.name.clone(),
                    },
                );
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

    /// The refinable declaration of `types` that `name` spells; a name that
    /// spells one of `others` is malformed, and one that spells neither is
    /// missing.
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
            let Some(attribute) = types.attribute(target.id.clone(), written) else {
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
                    parameters: vec![("self".to_owned(), ValueType::Reference(source.id.clone()))],
                    kind: qsl_forms::StateClauseKind::Invariant,
                    operation: None,
                    label: format!("{}.{written}", target.name),
                    range_agnostic: false,
                    field_row: true,
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
        let Some(attributes) = types.attributes(target.id.clone()) else {
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
        if let Some((layer, span)) = unsupported_row_form(expression) {
            self.findings
                .refuse(span, RefinementCause::UnsupportedRowForm { layer });
            return None;
        }
        // QSpec FR-430: the row reads no clock place of the concrete
        // object, so the mapped state is a function of the discrete state.
        if let Some((_, ValueType::Reference(object))) = context.parameters.first() {
            if let Some(span) = super::timed_model::first_clock_read(
                expression,
                self.concrete_side.model,
                self.declarations.models,
                self.clock_reading,
                &object,
            ) {
                self.findings.refuse(span, RefinementCause::ClockRead);
                return None;
            }
        }
        let declarations = self.declarations;
        let Some(root_span) = expression.spans.at(&[]) else {
            self.fault("row-expression-has-a-span");
            return None;
        };
        let site = row_site(root_span);
        let location = Location::root(Origin::TypeDeclaration { name: site.clone() });
        let RowContext {
            parameters,
            kind,
            operation,
            label,
            range_agnostic,
            field_row,
        } = context;
        let populations = self.population_scope();
        let mut outcome: Option<Nested> = None;
        let typed = (|| {
            let mut typer = Typer::new(
                declarations.scope,
                declarations.signatures,
                declarations.checking_limits,
                &mut *self.nodes,
                match kind {
                    qsl_forms::StateClauseKind::Invariant
                    | qsl_forms::StateClauseKind::Constraint => qsl_forms::ClauseKind::Invariant,
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
            if field_row {
                let root = body.root();
                // A whole population is the one nesting a row may have.
                let whole_population = matches!(root.kind(), NodeKind::PopulationObjects { .. });
                if !whole_population {
                    match nests_reference(types, root.value_type()) {
                        Ok(false) => {}
                        Ok(true) => {
                            outcome = Some(Nested::Row);
                            return Err(CheckRefusal::ill_typed(
                                &location,
                                IllTypedCause::TypeMismatch,
                            ));
                        }
                        Err(invariant) => {
                            outcome = Some(Nested::Fault(invariant));
                            return Err(CheckRefusal::ill_typed(
                                &location,
                                IllTypedCause::TypeMismatch,
                            ));
                        }
                    }
                }
            }
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
            Ok::<_, CheckRefusal>((body, typer.slots(), typer.slot_names().to_vec(), uses))
        })();
        match typed {
            Ok((body, slots, slot_names, uses)) => {
                let observations =
                    Observations::of(body.root(), observation_of(kind), parameters.len());
                Some(CheckedRow {
                    parameters,
                    body,
                    slots,
                    uses,
                    observations,
                    slot_names,
                    site,
                    spans: expression.spans.clone(),
                })
            }
            Err(_) if outcome.is_some() => {
                match (outcome, expression.spans.at(&[])) {
                    (Some(Nested::Row), Some(span)) => self.findings.refuse(
                        span,
                        RefinementCause::Unsupported {
                            feature: UnsupportedFeature::NestedReferenceRow,
                            profile: self.profile.clone(),
                            subject: None,
                        },
                    ),
                    (Some(Nested::Fault(invariant)), _) => self.fault(invariant),
                    _ => self.fault("row-expression-has-a-span"),
                }
                None
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

    /// FR-146 "Claim extent": the populations a model subject's per-step
    /// simulation records range over, when it writes them (no hidden field
    /// and no `any` row; the caller checks history). An operation row whose
    /// receiver two concrete populations cover, or a state mapping
    /// reachability two cover, is refused `ambiguous_declaration`/
    /// `ambiguous-name` naming them, and no record is written.
    fn simulation_roots(
        &mut self,
        objects: &[ObjectRow],
        operations: &[(usize, StepRow)],
        name_span: Span,
    ) -> Option<SimulationRoots> {
        let writes = objects.iter().all(|object| object.hidden.is_empty())
            && !operations
                .iter()
                .any(|(_, row)| matches!(row.target, StepTarget::Any));
        if !writes {
            return None;
        }
        let scope = self.declarations.scope;
        let concrete = std::slice::from_ref(self.concrete_side.model);
        let mut roots = SimulationRoots::default();
        let mut ambiguous = false;
        for (_, row) in operations {
            if let Some(RowRoots {
                context: Err(keys),
                name_span: row_span,
                ..
            }) = &row.roots
            {
                self.ambiguous_populations(*row_span, row.spelling(), keys);
                ambiguous = true;
            }
        }
        for object in objects {
            let mapped = self
                .concrete_side
                .model
                .object_types()
                .find(|(key, _)| **key == object.concrete_type)
                .map(|(_, id)| id);
            if let Some(id) = mapped {
                match super::state_clause::population_of(concrete, id, scope.types()) {
                    Ok(domain) => roots.initial.extend(domain),
                    // A type with an operation row has refused at that row.
                    Err(keys) if !ambiguous => {
                        self.ambiguous_populations(name_span, &object.concrete_type.node, &keys);
                        ambiguous = true;
                    }
                    Err(_) => {}
                }
            }
            for field in &object.fields {
                match super::state_clause::walked_populations(
                    field.expression.body.root(),
                    concrete,
                    scope,
                ) {
                    Ok(walked) => roots.mapping.extend(walked),
                    Err(keys) => {
                        self.ambiguous_populations(name_span, &object.concrete_type.node, &keys);
                        ambiguous = true;
                    }
                }
            }
        }
        (!ambiguous).then_some(roots)
    }

    /// FR-146 "Claim extent" (FR-104's rule): `name` has no single context
    /// population; `populations` are the ones covering it, ascending.
    fn ambiguous_populations(&mut self, span: Span, name: &str, populations: &[DeclarationKey]) {
        let populations: Vec<&str> = populations.iter().map(|key| key.node.as_str()).collect();
        self.findings.refuse(
            span,
            RefinementCause::Ambiguous {
                name: format!("{name} ({})", populations.join(", ")),
            },
        );
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
                types.object_type(declaring.clone()).map_or(
                    "",
                    quire_semantic_value::declaration::ObjectTypeDeclaration::name
                ),
                concrete.name()
            );
            if let Some(first) = first_rows.get(&(declaring.clone(), concrete.name().to_owned())) {
                self.findings.refuse(
                    row.span,
                    RefinementCause::Conflicting {
                        name: display,
                        first: *first,
                    },
                );
                continue;
            }
            first_rows.insert((declaring.clone(), concrete.name().to_owned()), row.span);
            let clause_operation = ClauseOperation {
                declaring: declaring.clone(),
                declaration: concrete.clone(),
            };
            let Some(key) = self.concrete_side.model.operation_key(&clause_operation) else {
                self.fault("concrete-operation-has-a-declaration-key");
                continue;
            };
            written.push(key.clone());
            let target = match &row.target {
                StepTargetForm::Stutter { .. } => Some((StepTarget::Stutter, Vec::new())),
                StepTargetForm::Any { .. } => Some((StepTarget::Any, Vec::new())),
                StepTargetForm::Node {
                    node, arguments, ..
                } => self
                    .node_target(row.span, node, arguments, (object, concrete), cx)
                    .map(|target| (target, Vec::new())),
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
            // QSpec FR-375: the class is spelled by the object type the row
            // names, as the catalog spells its owner's operations.
            let Some(owner) = self
                .concrete_side
                .model
                .declaration_key(object.id.clone())
                .and_then(|owner| type_identity_segment(&owner.package, &owner.node))
            else {
                self.fault("concrete-object-has-alias-free-identity-segment");
                continue;
            };
            if let Some((target, open)) = target {
                // FR-146 "Claim extent": the receiver, result and parameter
                // types, each `_` argument's abstract parameter domain, and
                // the context population.
                let mut root_types = vec![ValueType::Reference(object.id.clone())];
                root_types.extend(concrete.result().cloned());
                root_types.extend(
                    concrete
                        .parameters()
                        .iter()
                        .map(|(_, value_type)| value_type.clone()),
                );
                root_types.extend(open);
                let context = super::state_clause::population_of(
                    std::slice::from_ref(self.concrete_side.model),
                    object.id.clone(),
                    types,
                );
                steps.push((
                    index,
                    StepRow {
                        class: StepClass::Operation(key.clone()),
                        target,
                        spelling: format!("{owner}::{}", concrete.name()),
                        roots: Some(RowRoots {
                            operation: (declaring, concrete.name().to_owned()),
                            types: root_types,
                            context,
                            name_span: source.span,
                        }),
                    },
                ));
            }
        }
        // Over a protocol subject an operation needs a row only where an
        // attempt of it has no node row, which coverage decides.
        if selection.is_none() {
            for object in &concrete_types {
                for operation in scope.operations().declared_by(object.id.clone()) {
                    if !first_rows.contains_key(&(object.id.clone(), operation.name().to_owned())) {
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
                scope
                    .operations()
                    .resolve(self.types(), object.id.clone(), member),
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
        let (protocol, member) = named.spelling().split_once("::")?;
        // QSpec FR-432 "The binder record of an abstract node".
        let record: Option<Vec<ValueType>> = classes.record(member).map(<[ValueType]>::to_vec);
        if let Some(record) = &record {
            if !arguments.is_empty() && arguments.len() != record.len() {
                self.findings.refuse(
                    node.span,
                    RefinementCause::NodeArgumentCount {
                        node: node.name.clone(),
                        expected: record.len(),
                        actual: arguments.len(),
                    },
                );
                return None;
            }
        }
        let mut parameters = vec![("self".to_owned(), ValueType::Reference(object.id.clone()))];
        parameters.extend(concrete.parameters().iter().cloned());
        let mut checked = Vec::new();
        let mut complete = true;
        for (position, argument) in arguments.iter().enumerate() {
            match argument {
                ArgumentForm::Open { .. } => checked.push(None),
                ArgumentForm::Expression(expression) => {
                    let context = RowContext {
                        parameters: parameters.clone(),
                        kind: qsl_forms::StateClauseKind::Postcondition,
                        operation: Some(concrete.name().to_owned()),
                        label: format!("step {}", concrete.name()),
                        range_agnostic: false,
                        field_row: false,
                    };
                    let Some(row) = self.check_row_as(expression, None, context, cx) else {
                        complete = false;
                        continue;
                    };
                    let field = record.as_ref().and_then(|record| record.get(position));
                    if let Some(field) = field {
                        let actual = row.body().value_type();
                        if !conforms(self.types(), actual, &lift(&self.lifting, field)) {
                            let cause = RefinementCause::NodeArgumentType {
                                node: node.name.clone(),
                                position,
                                expected: Box::new(field.clone()),
                                actual: Box::new(actual.clone()),
                            };
                            match expression.spans.at(&[]) {
                                Some(span) => self.findings.refuse(span, cause),
                                None => self.fault("row-expression-has-a-span"),
                            }
                            complete = false;
                            continue;
                        }
                    }
                    checked.push(Some(row));
                }
            }
        }
        // A list left out leaves every field of the record open.
        if arguments.is_empty() {
            checked.resize_with(record.map_or(0, |record| record.len()), || None);
        }
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
    ) -> Option<(StepTarget, Vec<ValueType>)> {
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
        let mut context_parameters =
            vec![("self".to_owned(), ValueType::Reference(object.id.clone()))];
        context_parameters.extend(concrete.parameters().iter().cloned());
        let context = |parameters: &Vec<(String, ValueType)>| RowContext {
            parameters: parameters.clone(),
            kind: qsl_forms::StateClauseKind::Postcondition,
            operation: Some(concrete.name().to_owned()),
            label: format!("step {}", concrete.name()),
            range_agnostic: false,
            field_row: false,
        };
        let mut complete = true;
        let receiver_type = lift(&self.lifting, &ValueType::Reference(target_type.id.clone()));
        let receiver = match &arguments[0] {
            ArgumentForm::Expression(expression) => {
                self.check_row(expression, &receiver_type, context(&context_parameters), cx)
            }
            ArgumentForm::Open { .. } => None,
        };
        complete &= receiver.is_some();
        let mut checked = Vec::new();
        let mut open = Vec::new();
        for (argument, (_, declared)) in arguments[1..].iter().zip(parameters) {
            match argument {
                ArgumentForm::Open { .. } => {
                    checked.push(None);
                    open.push(lift(&self.lifting, declared));
                }
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
        complete.then_some((
            StepTarget::Abstract {
                operation: key,
                receiver,
                arguments: checked,
                binds_result,
            },
            open,
        ))
    }

    /// FR-137: the rows of `side`, each constraint once, in source order.
    fn fairness(&mut self, form: &RefinementForm, side: FairnessSide) -> Vec<RefinementFairness> {
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
        let mut set: Vec<(RefinementFairness, Span)> = Vec::new();
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
                match scope
                    .operations()
                    .resolve(self.types(), object.id.clone(), member)
                {
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
            let member = RefinementFairness::Authored(constraint);
            if let Some((_, first)) = set.iter().find(|(earlier, _)| *earlier == member) {
                self.findings.refuse(
                    written.span,
                    RefinementCause::Conflicting {
                        name: written.operation.name.clone(),
                        first: *first,
                    },
                );
                continue;
            }
            set.push((member, written.span));
        }
        set.into_iter().map(|(constraint, _)| constraint).collect()
    }
}

/// What the nesting check of a field row found.
enum Nested {
    /// The row's type nests a reference.
    Row,
    /// A composite type of the row has no declaration.
    Fault(&'static str),
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
    /// Whether the row defines an abstract field (FR-135), which a type that
    /// nests a reference refuses `nested-reference-row` before any other
    /// check of the row's type.
    field_row: bool,
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
            };
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
        member: population.member.clone(),
        value,
        required: required.clone(),
    };
    // The part of the check that does not depend on `n`: the value's kind
    // and its bounds that hold for every `n`.
    let independent = match (&used.value, required) {
        (PopulationValue::Objects, ValueType::Collection(collection)) => {
            collection.kind() == CollectionKind::Set
                && matches!(collection.element(), ValueType::Reference(member) if *member == used.member)
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
    use quire_exact::{
        CollectionKind, CollectionType, NodeKey, OriginalDeclarationKey, PathSegment, Presence,
    };
    use quire_semantic_value::declaration::{
        CompositeDeclaration, CompositeShape, FieldDeclaration,
    };

    use super::*;

    fn node(index: u8) -> NodeKey {
        NodeKey::new("test", vec![PathSegment::Text(format!("Node{index}"))], 0)
            .expect("a valid structural key")
    }

    fn effective(index: u8) -> EffectiveId {
        EffectiveId::new(
            None,
            OriginalDeclarationKey::new("test", format!("Node{index}")),
        )
    }

    /// FR-359: a record conforms to the record it specializes and to no
    /// other; a parent does not conform to its child, and a collection of the
    /// child is not a collection of the parent, and an option of the child is not
    /// an option of the parent (FR-359).
    #[trace("TC-541")]
    #[test]
    fn a_record_conforms_to_its_ancestor_only() {
        let [money, taxed, fee] = [1, 2, 3].map(node);
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
                record(money.clone(), "Money", vec![]),
                record(taxed.clone(), "TaxedMoney", vec![money.clone()]),
                record(fee.clone(), "Fee", vec![]),
            ],
            [],
        )
        .expect("the records admit");
        let record = |key: &NodeKey| ValueType::Composite(key.clone());
        assert!(conforms(&types, &record(&taxed), &record(&money)));
        assert!(conforms(&types, &record(&money), &record(&money)));
        assert!(!conforms(&types, &record(&money), &record(&taxed)));
        assert!(!conforms(&types, &record(&fee), &record(&money)));
        let sequence_of = |element| {
            ValueType::collection(CollectionType::new(CollectionKind::Sequence, element, None))
        };
        assert!(!conforms(
            &types,
            &sequence_of(record(&taxed)),
            &sequence_of(record(&money))
        ));
        assert!(!conforms(
            &types,
            &ValueType::option(record(&taxed)),
            &ValueType::option(record(&money))
        ));
        assert!(conforms(
            &types,
            &ValueType::option(record(&money)),
            &ValueType::option(record(&money))
        ));
    }
    fn reference(byte: u8) -> ValueType {
        ValueType::Reference(effective(byte))
    }

    fn map_of(values: ValueType, bound: Option<quire_exact::CardinalityBound>) -> ValueType {
        ValueType::Map(std::sync::Arc::new(quire_exact::MapType::new(
            values, bound,
        )))
    }

    /// FR-135: a reference nested in a map's values is not lifted by the
    /// object map, so it is a refinement feature the build does not check,
    /// as one nested in an option or a collection is. A map of a value
    /// without a reference, and a bounded quantity, hold none. No QSL
    /// criterion pins the map arm itself.
    #[test]
    fn a_reference_in_a_map_value_is_a_nested_reference() {
        let types = TypeEnvironment::new([], []).expect("an empty environment");
        assert_eq!(
            nests_reference(&types, &map_of(reference(1), None)),
            Ok(true)
        );
        assert_eq!(
            nests_reference(&types, &map_of(map_of(reference(1), None), None)),
            Ok(true)
        );
        assert_eq!(
            nests_reference(&types, &map_of(ValueType::Boolean, None)),
            Ok(false)
        );
        let bounded = ValueType::BoundedQuantity(std::sync::Arc::new(
            quire_exact::BoundedQuantityType::new(
                quire_exact::UnitId::compound(vec![(node(5), Integer::one())]),
                None,
                None,
            )
            .expect("in the unit"),
        ));
        assert_eq!(nests_reference(&types, &bounded), Ok(false));
    }

    /// ADR-020 RM-3: the object map lifts a reference at any depth in a map's
    /// values, and the map keeps its bound; a bounded quantity is unchanged.
    #[test]
    fn lifting_reaches_a_reference_inside_a_map_value() {
        let abstract_type = effective(1);
        let concrete = effective(2);
        let lifting = BTreeMap::from([(abstract_type, concrete)]);
        let bound = quire_exact::CardinalityBound::new(1, 3).expect("a bound");
        assert_eq!(
            lift(&lifting, &map_of(reference(1), Some(bound))),
            map_of(reference(2), Some(bound))
        );
        assert_eq!(
            lift(&lifting, &map_of(map_of(reference(1), None), None)),
            map_of(map_of(reference(2), None), None)
        );
        assert_eq!(
            lift(&lifting, &map_of(ValueType::Boolean, None)),
            map_of(ValueType::Boolean, None)
        );
    }
}
