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

use std::collections::BTreeMap;

use qsl_forms::{
    ArgumentForm, ExpressionForm, FairnessConstraintForm, FairnessGranularity as FormGranularity,
    FairnessKind as FormKind, FairnessSide, RefinementForm, RefinementRowForm, StepTargetForm,
};
use qsl_foundation::selection::ProfileSelection;
use qsl_foundation::{Code, InternalFault, Span};
use quire_exact::{EffectiveId, IllTypedCause, Presence, ValueType};
use quire_semantic_value::declaration::TypeEnvironment;
use quire_semantic_value::fairness::{
    FairnessConstraint, FairnessGranularity, FairnessKind, FairnessOperation,
};
use quire_semantic_value::location::{Location, Origin};

use super::check::{bind_parameters, StateContext, Typer};
use super::ir::{CheckedBody, CheckedNode};
use super::lowering::AdmittedModel;
use super::profile::TemporalProfile;
use super::refusal::{CheckCause, CheckRefusal, KeyFault};
use super::state_clause::{ClauseDeclarations, ClauseOperation};
use crate::family::CheckContext;
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
            Self::Conflicting { .. } | Self::Malformed { .. } => Code::InvalidModelBinding,
            Self::Check(cause) => cause.code(),
        }
    }

    /// The closed cause tag.
    pub fn cause(&self) -> Option<&'static str> {
        match self {
            Self::WrongProfile { .. } => Some("wrong-selection-role"),
            Self::Unresolved { .. } | Self::MissingRow { .. } => Some("missing-name"),
            Self::Conflicting { .. } => Some("conflicting-binding"),
            Self::Malformed { .. } => Some("malformed-declaration"),
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
}

impl CheckedRow {
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

/// The concrete step a `step` row maps.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StepClass {
    /// A concrete operation.
    Operation(DeclarationKey),
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
    pub(crate) concrete_model: String,
    pub(crate) populations: Vec<PopulationRow>,
    pub(crate) objects: Vec<ObjectRow>,
    pub(crate) steps: Vec<StepRow>,
    pub(crate) concrete_fairness: Vec<FairnessConstraint>,
    pub(crate) abstract_fairness: Vec<FairnessConstraint>,
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
}

/// One model of the refinement and the alias the unit gives it.
struct Side<'a> {
    alias: &'a str,
    model: &'a AdmittedModel,
}

/// One population of a model, with the name a row spells it by.
struct Population<'a> {
    name: String,
    record: &'a PopulationRecord,
}

/// One object type of a model.
struct ObjectType {
    name: String,
    id: EffectiveId,
    key: DeclarationKey,
}

impl<'a> Side<'a> {
    fn populations(&self) -> Vec<Population<'a>> {
        self.model
            .population_domains()
            .filter_map(|(_, record, _)| {
                let segment = type_identity_segment(&record.key.package, &record.key.node)?;
                Some(Population {
                    name: format!("{}::{segment}", self.alias),
                    record,
                })
            })
            .collect()
    }

    fn object_types(&self, types: &TypeEnvironment) -> Vec<ObjectType> {
        self.model
            .object_types()
            .filter_map(|(key, id)| {
                Some(ObjectType {
                    name: types.object_type(id)?.name().to_owned(),
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
}

/// The object type name and member of `Alias::Type::member`.
fn split_member(name: &str) -> Option<(&str, &str)> {
    name.rsplit_once("::")
}

/// Whether a value of `actual` fits a slot of `required` (ADR-016 SC-2):
/// the same type, an integer in a containing integer range, an object
/// reference to a subtype, or an option of such.
fn conforms(types: &TypeEnvironment, actual: &ValueType, required: &ValueType) -> bool {
    match (actual, required) {
        (ValueType::Integer | ValueType::Int(_), ValueType::Integer) => true,
        (ValueType::Int(source), ValueType::Int(target)) => {
            target.lower() <= source.lower() && source.upper() <= target.upper()
        }
        (ValueType::Reference(sub), ValueType::Reference(sup)) => types.conforms(*sub, *sup),
        (ValueType::Option(sub), ValueType::Option(sup)) => conforms(types, sub, sup),
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

/// Checks the refinement `declaration` and returns it with the package's
/// running expression-node total after it, or every refusal of it.
pub(crate) fn check_refinement<'a, 'c>(
    declaration: &'c RefinementDeclaration,
    declarations: &'c ClauseDeclarations<'a>,
    cx: &mut CheckContext<'a, ClauseDeclarations<'a>>,
) -> Result<(CheckedRefinement, u64), (Vec<CheckRefusal>, u64)> {
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
    let (Some(abstract_side), Some(concrete_side)) =
        (side(&form.abstract_model), side(&form.concrete_model))
    else {
        for alias in [&form.abstract_model, &form.concrete_model] {
            if side(alias).is_none() {
                findings.unresolved(&alias.name, alias.span, None);
            }
        }
        return Err((findings.refusals, nodes));
    };
    let mut checker = Checker {
        declarations,
        abstract_side,
        concrete_side,
        findings,
        nodes: &mut nodes,
        lifting: BTreeMap::new(),
        maps_complete: true,
    };
    let populations = checker.populations(form);
    let objects = checker.objects(form, cx);
    let steps = checker.steps(form, cx);
    let concrete_fairness = checker.fairness(form, FairnessSide::Assume);
    let abstract_fairness = checker.fairness(form, FairnessSide::Ensure);
    let findings = checker.findings;
    if !findings.refusals.is_empty() {
        return Err((findings.refusals, nodes));
    }
    Ok((
        CheckedRefinement {
            name: form.name.name.clone(),
            profile: declaration.selection.clone(),
            abstract_model: form.abstract_model.name.clone(),
            concrete_model: form.concrete_model.name.clone(),
            populations,
            objects,
            steps,
            concrete_fairness,
            abstract_fairness,
        },
        nodes,
    ))
}

struct Checker<'c, 'a> {
    declarations: &'c ClauseDeclarations<'a>,
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
}

impl<'c, 'a> Checker<'c, 'a> {
    fn types(&self) -> &'a TypeEnvironment {
        self.declarations.scope.types()
    }

    /// FR-135 "Population map".
    fn populations(&mut self, form: &RefinementForm) -> Vec<PopulationRow> {
        let abstract_populations = self.abstract_side.populations();
        let concrete_populations = self.concrete_side.populations();
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
        let abstract_types = self.abstract_side.object_types(self.types());
        let concrete_types = self.concrete_side.object_types(self.types());
        let abstract_populations = self.abstract_side.populations();
        let concrete_populations = self.concrete_side.populations();
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

        self.maps_complete = self.findings.refusals.is_empty();
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
        for attribute in types.attributes(target.id)?.iter() {
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

    fn fault(&mut self, invariant: &'static str) {
        self.findings.refusals.push(CheckRefusal {
            location: Location::root(Origin::Expression),
            cause: CheckCause::InternalFault(Box::new(KeyFault::StageFault(InternalFault::new(
                "check.refinement",
                invariant,
            )))),
        });
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
        let declarations = self.declarations;
        let location = Location::root(Origin::Expression);
        let RowContext {
            parameters,
            kind,
            operation,
        } = context;
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
            .with_context(cx.reborrow(declarations.scope));
            bind_parameters(&mut typer, &parameters, &location)?;
            typer.enter_state_clause(StateContext {
                parent_projections: BTreeMap::new(),
                kind,
                operation,
                self_slot: 0,
                result_slot: None,
            });
            let body = typer.check_as(&expression.expression, required, &location)?;
            Ok::<_, CheckRefusal>((body, typer.slots()))
        })();
        match typed {
            Ok((body, slots)) => Some(CheckedRow {
                parameters,
                body,
                slots,
            }),
            Err(refusal) => {
                let span = super::region::span_at(&expression.spans, &refusal.location)
                    .or_else(|| expression.spans.at(&[]))
                    .unwrap_or(Span { start: 0, end: 0 });
                self.findings
                    .refuse(span, RefinementCause::Check(refusal.cause));
                None
            }
        }
    }

    /// FR-136: one `step` row per concrete operation, each right side
    /// checked.
    fn steps(
        &mut self,
        form: &RefinementForm,
        cx: &mut CheckContext<'a, ClauseDeclarations<'a>>,
    ) -> Vec<StepRow> {
        let scope = self.declarations.scope;
        let types = self.types();
        let abstract_types = self.abstract_side.object_types(types);
        let concrete_types = self.concrete_side.object_types(types);
        let mut first_rows: BTreeMap<(EffectiveId, String), Span> = BTreeMap::new();
        let mut steps = Vec::new();
        for row in &form.rows {
            let RefinementRowForm::Step(row) = row else {
                continue;
            };
            let Some((object, operation)) =
                self.operation_named(&row.concrete, &concrete_types, None)
            else {
                continue;
            };
            let OperationLookup::Declared {
                declaring,
                operation: concrete,
            } = operation
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
            let target = match &row.target {
                StepTargetForm::Stutter { .. } => Some(StepTarget::Stutter),
                StepTargetForm::Any { .. } => Some(StepTarget::Any),
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
                steps.push(StepRow {
                    class: StepClass::Operation(key.clone()),
                    target,
                });
            }
        }
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
        steps
    }

    /// The operation `name` (`Alias::Type::operation`) spells among the
    /// object types `types`, recording the refusal when there is none.
    fn operation_named<'t>(
        &mut self,
        name: &qsl_forms::NameForm,
        types: &'t [ObjectType],
        held_by: Option<&str>,
    ) -> Option<(&'t ObjectType, OperationLookup<'a>)> {
        let scope = self.declarations.scope;
        let found = split_member(&name.name).and_then(|(type_name, member)| {
            let object = types.iter().find(|object| object.name == type_name)?;
            match scope.operations().resolve(self.types(), object.id, member) {
                OperationLookup::Missing => None,
                lookup => Some((object, lookup)),
            }
        });
        if found.is_none() {
            self.findings.unresolved(&name.name, name.span, held_by);
        }
        found
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
        let (target_type, lookup) = self.operation_named(operation, abstract_types, None)?;
        let OperationLookup::Declared {
            declaring,
            operation: abstract_operation,
        } = lookup
        else {
            self.findings
                .unresolved(&operation.name, operation.span, None);
            return None;
        };
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
        let abstract_types = self.abstract_side.object_types(self.types());
        let concrete_types = self.concrete_side.object_types(self.types());
        let (own_types, other_types, other_alias) = match side {
            FairnessSide::Assume => (&concrete_types, &abstract_types, self.abstract_side.alias),
            FairnessSide::Ensure => (&abstract_types, &concrete_types, self.concrete_side.alias),
        };
        let mut set: Vec<FairnessConstraint> = Vec::new();
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
            }
        }
        set
    }
}

/// How a row expression is typed: its parameters, the clause kind it reads
/// as and the concrete operation whose arguments it may name.
struct RowContext {
    parameters: Vec<(String, ValueType)>,
    kind: qsl_forms::StateClauseKind,
    operation: Option<String>,
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
