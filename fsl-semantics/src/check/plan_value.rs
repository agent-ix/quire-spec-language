// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-800 "The plan value": the S3 types of the plan a plan
//! synthesis's `satisfies` block and objective read.
//!
//! The declaration's name denotes a record with `length`, `final`, `initial`
//! and `steps`. A model state (`final`, `initial`, a step's `before` and
//! `after`) is a record of one collection per population of the model, each
//! element a record of that object type's attributes, and the attributes of
//! the grammar's object type itself, which read like the model object of the
//! grammar when the state holds exactly one. A step record carries `index`,
//! `operation` (a value of the model's operation enum `Alias::Op`),
//! `arguments`, `before` and `after`.
//!
//! Every type is declared in a copy of the unit's scope that only the plan
//! synthesis reads, so no declaration of the checked package changes.

use std::collections::{BTreeMap, BTreeSet};

use filament_checked::key::DeclarationKey;
use filament_exact::{
    CollectionKind, CollectionType, EffectiveId, NodeKey, PathSegment, Presence, ValueType,
};
use filament_semantic_value::declaration::{
    CompositeDeclaration, CompositeShape, FieldDeclaration,
};
use filament_semantic_value::location::ExpressionLocation as Location;

use super::check::{EnumBinding, NamedScope};
use super::{CheckCause, CheckRefusal, Scope};
use crate::value::enumeration::{
    AdmittedEnumDeclaration, EnumDeclarationPreimage, EnumMemberPreimage,
};
use crate::value::semantic_node::{NodeOwner, OwnerSelection, OwnerSubject};

/// The package component of every node key of the plan value's types.
const PLAN_PACKAGE: &str = "filament.plan-value";

/// One operation a plan step may invoke.
#[derive(Clone, Debug)]
pub struct PlanOperation {
    /// The object type declaring the operation.
    pub declaring: EffectiveId,
    /// The operation's name.
    pub name: String,
    /// Every declared parameter, in declaration order; those whose type a
    /// step record's `arguments` can carry are [`Self::argument_fields`].
    pub parameters: Vec<(String, ValueType)>,
}

/// One population of the plan's model: the `state` record's collection field
/// that lists its objects.
#[derive(Clone, Debug)]
pub struct PlanPopulation {
    /// The field of the state record: the population's own name.
    pub field: String,
    /// The population's declaration key, which a model state keys it by.
    pub key: DeclarationKey,
    /// The canonical member object type of the population.
    pub object: EffectiveId,
}

/// The declarations of one plan synthesis's plan value.
#[derive(Clone, Debug)]
pub struct PlanShape {
    /// The plan record: `length`, `final`, `initial`, `steps`.
    pub plan: NodeKey,
    /// The model state record.
    pub state: NodeKey,
    /// The step record: `index`, `operation`, `arguments`, `before`, `after`.
    pub step: NodeKey,
    /// The `arguments` record of a step.
    pub arguments: NodeKey,
    /// The grammar's object type, whose attributes a state record carries
    /// directly.
    pub grammar: EffectiveId,
    /// The attribute names of the grammar's object type a state record
    /// carries directly.
    pub inline: Vec<String>,
    /// The record of each object type a population lists.
    pub objects: BTreeMap<EffectiveId, NodeKey>,
    /// The populations of the model, in the order the state record lists
    /// them.
    pub populations: Vec<PlanPopulation>,
    /// The operations a step may invoke, in declaration order (FSpec
    /// FR-804).
    pub operations: Vec<PlanOperation>,
    /// The `Alias::Op` enum: one case per operation name.
    pub operation_enum: EnumBinding,
}

impl PlanShape {
    /// The record fields of `arguments` and the type each carries: the
    /// parameters of every operation that name one type everywhere.
    pub fn argument_fields(&self) -> Vec<(String, ValueType)> {
        argument_fields(&self.operations)
    }
}

/// A plan synthesis's plan value and the scope that declares it.
#[derive(Clone, Debug)]
pub struct CheckedPlan {
    pub(crate) shape: PlanShape,
    pub(crate) scope: Scope,
}

impl CheckedPlan {
    /// The plan value's declarations.
    pub fn shape(&self) -> &PlanShape {
        &self.shape
    }

    /// The unit's scope with the plan value's types declared: the scope the
    /// checked expressions evaluate under.
    pub fn scope(&self) -> &Scope {
        &self.scope
    }
}

fn unsupported(location: &Location) -> CheckRefusal {
    CheckRefusal {
        location: location.clone(),
        cause: CheckCause::UnsupportedFeature {
            loci: vec![location.clone()],
        },
    }
}

fn key(path: &[&str]) -> Option<NodeKey> {
    NodeKey::new(
        PLAN_PACKAGE,
        path.iter()
            .map(|segment| PathSegment::Text((*segment).to_owned()))
            .collect(),
        0,
    )
    .ok()
}

/// Whether a step's `arguments` record can carry a parameter of
/// `value_type`: a value a step's canonical argument text reads back to
/// without an object environment.
fn carried(value_type: &ValueType) -> bool {
    matches!(
        value_type,
        ValueType::Boolean | ValueType::Integer | ValueType::Int(_)
    )
}

fn argument_fields(operations: &[PlanOperation]) -> Vec<(String, ValueType)> {
    let mut seen: BTreeMap<&str, Option<&ValueType>> = BTreeMap::new();
    let mut order: Vec<&str> = Vec::new();
    for operation in operations {
        for (name, value_type) in &operation.parameters {
            match seen.get(name.as_str()) {
                None => {
                    seen.insert(name, carried(value_type).then_some(value_type));
                    order.push(name);
                }
                Some(Some(first)) if *first != value_type => {
                    seen.insert(name, None);
                }
                Some(_) => {}
            }
        }
    }
    order
        .into_iter()
        .filter_map(|name| Some((name.to_owned(), seen.get(name).copied().flatten()?.clone())))
        .collect()
}

/// Declare the plan value of the synthesis `form` over the object type
/// `grammar` in a copy of `scope`.
///
/// # Errors
/// [`CheckCause::UnsupportedFeature`] at `location` when the unit's type
/// environment cannot take the declarations.
pub(crate) fn declare(
    scope: &Scope,
    named: &NamedScope,
    form: &fsl_forms::SynthesisForm,
    grammar: &EffectiveId,
    source: &fsl_foundation::source::provenance::RawSourceRef,
    package_name: &str,
    location: &Location,
) -> Result<CheckedPlan, CheckRefusal> {
    let types = scope.types();
    let synthesis = form.name.name.as_str();
    let alias = form
        .grammar
        .name
        .rsplit_once("::")
        .map(|(alias, _)| alias.to_owned());

    let populations: Vec<PlanPopulation> = alias
        .as_deref()
        .map(|alias| {
            named
                .populations
                .iter()
                .filter_map(|population| {
                    let field = population
                        .name
                        .strip_prefix(alias)?
                        .strip_prefix("::")?
                        .to_owned();
                    Some(PlanPopulation {
                        field,
                        key: population.key.clone(),
                        object: population.domain.object.clone(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let mut composites = Vec::new();
    let mut objects = BTreeMap::new();
    let mut needed: BTreeSet<EffectiveId> = populations
        .iter()
        .map(|population| population.object.clone())
        .collect();
    needed.insert(grammar.clone());
    for object in &needed {
        let declaration = types
            .object_type(object.clone())
            .ok_or_else(|| unsupported(location))?;
        let attributes = types
            .attributes(object.clone())
            .ok_or_else(|| unsupported(location))?;
        let record =
            key(&[synthesis, "object", declaration.name()]).ok_or_else(|| unsupported(location))?;
        let fields = attributes
            .iter()
            .map(|attribute| {
                FieldDeclaration::new(
                    attribute.field().name(),
                    attribute.field().value_type().clone(),
                    attribute.field().presence(),
                )
            })
            .collect();
        composites.push(CompositeDeclaration::new(
            record.clone(),
            declaration.name(),
            CompositeShape::Record(fields),
        ));
        objects.insert(object.clone(), record);
    }

    // The state record: one collection per population, then the grammar's
    // own attributes.
    let mut state_fields = Vec::new();
    let mut taken = BTreeSet::new();
    for population in &populations {
        let record = objects
            .get(&population.object)
            .ok_or_else(|| unsupported(location))?;
        if !taken.insert(population.field.clone()) {
            continue;
        }
        state_fields.push(FieldDeclaration::new(
            population.field.clone(),
            ValueType::collection(CollectionType::new(
                CollectionKind::Sequence,
                ValueType::Composite(record.clone()),
                None,
            )),
            Presence::Required,
        ));
    }
    let mut inline = Vec::new();
    if let Some(attributes) = types.attributes(grammar.clone()) {
        for attribute in attributes.iter() {
            let field = attribute.field();
            if taken.insert(field.name().to_owned()) {
                inline.push(field.name().to_owned());
                state_fields.push(FieldDeclaration::new(
                    field.name(),
                    field.value_type().clone(),
                    field.presence(),
                ));
            }
        }
    }
    let state = key(&[synthesis, "state"]).ok_or_else(|| unsupported(location))?;
    composites.push(CompositeDeclaration::new(
        state.clone(),
        "state",
        CompositeShape::Record(state_fields),
    ));

    // The operations, in declaration order of the object types' key order.
    let mut operations = Vec::new();
    for object in types.object_types() {
        for operation in scope.operations().declared_by(object.key()) {
            operations.push(PlanOperation {
                declaring: object.key(),
                name: operation.name().to_owned(),
                parameters: operation.parameters().to_vec(),
            });
        }
    }
    let cases: BTreeSet<String> = operations
        .iter()
        .map(|operation| operation.name.clone())
        .collect();
    let enum_name = match &alias {
        Some(alias) => format!("{alias}::Op"),
        None => "Op".to_owned(),
    };
    let operation_enum = operation_enum(
        &enum_name,
        synthesis,
        cases.into_iter().collect(),
        source,
        package_name,
    )
    .ok_or_else(|| unsupported(location))?;

    let arguments = key(&[synthesis, "arguments"]).ok_or_else(|| unsupported(location))?;
    composites.push(CompositeDeclaration::new(
        arguments.clone(),
        "arguments",
        CompositeShape::Record(
            argument_fields(&operations)
                .into_iter()
                .map(|(name, value_type)| {
                    FieldDeclaration::new(name, value_type, Presence::Optional)
                })
                .collect(),
        ),
    ));
    let step = key(&[synthesis, "step"]).ok_or_else(|| unsupported(location))?;
    composites.push(CompositeDeclaration::new(
        step.clone(),
        "step",
        CompositeShape::Record(vec![
            FieldDeclaration::new("index", ValueType::Integer, Presence::Required),
            FieldDeclaration::new(
                "operation",
                ValueType::Enum(operation_enum.shape()),
                Presence::Required,
            ),
            FieldDeclaration::new(
                "arguments",
                ValueType::Composite(arguments.clone()),
                Presence::Required,
            ),
            FieldDeclaration::new(
                "before",
                ValueType::Composite(state.clone()),
                Presence::Required,
            ),
            FieldDeclaration::new(
                "after",
                ValueType::Composite(state.clone()),
                Presence::Required,
            ),
        ]),
    ));
    let plan = key(&[synthesis, "plan"]).ok_or_else(|| unsupported(location))?;
    composites.push(CompositeDeclaration::new(
        plan.clone(),
        "plan",
        CompositeShape::Record(vec![
            FieldDeclaration::new("length", ValueType::Integer, Presence::Required),
            FieldDeclaration::new(
                "final",
                ValueType::Composite(state.clone()),
                Presence::Required,
            ),
            FieldDeclaration::new(
                "initial",
                ValueType::Composite(state.clone()),
                Presence::Required,
            ),
            FieldDeclaration::new(
                "steps",
                ValueType::collection(CollectionType::new(
                    CollectionKind::Sequence,
                    ValueType::Composite(step.clone()),
                    None,
                )),
                Presence::Required,
            ),
        ]),
    ));

    let scope = scope
        .with_plan_types(composites, operation_enum.clone())
        .ok_or_else(|| unsupported(location))?;
    Ok(CheckedPlan {
        shape: PlanShape {
            plan,
            state,
            step,
            arguments,
            grammar: grammar.clone(),
            inline,
            objects,
            populations,
            operations,
            operation_enum,
        },
        scope,
    })
}

/// The unordered enum `name` over `cases`, admitted under the authored
/// source owner and qualified by the synthesis declaration.
fn operation_enum(
    name: &str,
    synthesis: &str,
    cases: Vec<String>,
    source: &fsl_foundation::source::provenance::RawSourceRef,
    package_name: &str,
) -> Option<EnumBinding> {
    let subject = OwnerSubject::new(source.authority(), source.identity(), package_name);
    let owners = OwnerSelection::new([NodeOwner::Source(subject.clone())]);
    let preimage = EnumDeclarationPreimage::new(
        NodeOwner::Source(subject),
        std::iter::once(synthesis)
            .chain(name.split("::"))
            .map(str::to_owned)
            .collect(),
        false,
        cases.clone(),
    )
    .ok()?;
    let key = preimage.key().ok()?;
    let declaration = AdmittedEnumDeclaration::admit(preimage, key.clone(), &owners).ok()?;
    let mut members = Vec::with_capacity(cases.len());
    for case in cases {
        let member = EnumMemberPreimage::new(key.clone(), case).ok()?;
        let member_key = member.key().ok()?;
        members.push(declaration.admit_member(&member, member_key).ok()?);
    }
    Some(EnumBinding {
        name: name.to_owned(),
        declaration,
        members,
    })
}
