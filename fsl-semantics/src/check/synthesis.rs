// SPDX-License-Identifier: AGPL-3.0-or-later
//! S3 finite-domain analysis for FR-754 synthesis declarations.

use filament_checked::node::NodeKind;
use filament_exact::{IllTypedCause, ValueType};
use filament_semantic_value::declaration::{CompositeShape, TypeEnvironment};
use filament_semantic_value::location::ExpressionLocation as Location;

use super::check::NamedScope;
use super::ir::CheckedBody;
pub use super::plan_value::{CheckedPlan, PlanOperation, PlanPopulation, PlanShape};
use super::{CheckCause, CheckRefusal, Scope, Signatures, Typer};
use crate::family::{CheckContext, DiagnosticSink, ScopeStack, StageLimits};

/// The checked source identity needed to name an emitted synthesis claim.
///
/// The package writer uses the authored name and complete declaration span
/// rather than reconstructing either from the graph node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedSynthesisClaim {
    node: filament_exact::NodeKey,
    name: String,
    span: fsl_foundation::Span,
}

impl CheckedSynthesisClaim {
    pub(crate) fn new(
        node: filament_exact::NodeKey,
        name: String,
        span: fsl_foundation::Span,
    ) -> Self {
        Self { node, name, span }
    }

    /// The structural node emitted for this declaration.
    pub fn node(&self) -> &filament_exact::NodeKey {
        &self.node
    }

    /// The authored synthesis declaration name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The complete authored declaration span.
    pub fn span(&self) -> fsl_foundation::Span {
        self.span
    }
}

/// One synthesis declaration after its names and expressions pass S3.
#[derive(Clone, Debug)]
pub struct CheckedSynthesis {
    /// The S2 declaration and its authored spans.
    pub(crate) form: fsl_forms::SynthesisForm,
    /// The resolved candidate grammar type.
    pub(crate) grammar: ValueType,
    /// The resolved finite candidate domain type.
    pub(crate) domain: ValueType,
    /// The checked `satisfies` block.
    pub(crate) satisfies: CheckedBody,
    /// The checked objective when the declaration has one.
    pub(crate) objective: Option<CheckedBody>,
    /// The candidate binder name shared by both checked expressions.
    pub(crate) slot_names: Vec<String>,
    /// The plan value of a plan synthesis (FSpec FR-800), whose grammar is a
    /// model object type.
    pub(crate) plan: Option<CheckedPlan>,
}

impl CheckedSynthesis {
    /// The authored declaration name used by checked-package claim selection.
    pub fn name(&self) -> &str {
        &self.form.name.name
    }

    /// The complete source span of this declaration.
    pub fn span(&self) -> fsl_foundation::Span {
        self.form.span
    }

    /// The plan value's declarations when this is a plan synthesis (FSpec
    /// FR-800), whose checked expressions read the plan through them.
    pub fn plan(&self) -> Option<&CheckedPlan> {
        self.plan.as_ref()
    }

    /// The checked finite domain type.
    pub fn domain(&self) -> &ValueType {
        &self.domain
    }

    /// The checked grammar type.
    pub fn grammar(&self) -> &ValueType {
        &self.grammar
    }

    /// The checked satisfaction expression.
    pub fn satisfies(&self) -> &CheckedBody {
        &self.satisfies
    }

    /// The checked objective expression, when authored.
    pub fn objective(&self) -> Option<&CheckedBody> {
        self.objective.as_ref()
    }

    /// Whether the objective is exactly the candidate binder, whose value
    /// order proves the first satisfying value optimal.
    pub fn objective_is_candidate(&self) -> bool {
        self.objective
            .as_ref()
            .is_some_and(|body| matches!(body.root().kind(), NodeKind::Local(_)))
    }

    /// Candidate binder names in evaluation frame order.
    pub fn slot_names(&self) -> &[String] {
        &self.slot_names
    }

    /// The objective's normalized text: its tokens joined by one space,
    /// comments dropped, as the emitted node's `text` binding carries it.
    pub fn objective_source(&self) -> Option<&str> {
        Some(self.form.objective.as_ref()?.text.as_str())
    }

    /// Objective direction, when the declaration has an objective.
    pub fn objective_direction(&self) -> Option<&'static str> {
        Some(match self.form.objective.as_ref()?.direction {
            fsl_forms::SynthesisDirectionForm::Minimize => "minimize",
            fsl_forms::SynthesisDirectionForm::Maximize => "maximize",
        })
    }
}

/// Resolve and type-check each source synthesis declaration (FR-754 S3).
pub(crate) fn check_forms(
    forms: &[fsl_forms::SynthesisForm],
    scope: &Scope,
    signatures: &Signatures,
    named: &NamedScope,
    source: &fsl_foundation::source::provenance::RawSourceRef,
    source_owners: &std::collections::BTreeMap<
        String,
        fsl_foundation::source::provenance::RawSourceRef,
    >,
    package_name: &str,
) -> Result<Vec<CheckedSynthesis>, Vec<CheckRefusal>> {
    let mut checked = Vec::with_capacity(forms.len());
    for (index, form) in forms.iter().enumerate() {
        let location = super::lowering::synthesis_location(index, &form.name.name);
        let satisfies_location =
            super::lowering::synthesis_satisfies_location(index, &form.name.name);
        let objective_location =
            super::lowering::synthesis_objective_location(index, &form.name.name);
        let Some(domain) = named_type(scope, &form.domain.name) else {
            return Err(vec![CheckRefusal {
                location,
                cause: CheckCause::MissingQualifiedName(form.domain.clone()),
            }]);
        };
        let Some(grammar) = named_type(scope, &form.grammar.name) else {
            return Err(vec![CheckRefusal {
                location,
                cause: CheckCause::MissingQualifiedName(form.grammar.clone()),
            }]);
        };

        // FSpec FR-800: a grammar that is a model object type selects plan
        // synthesis, whose candidate is the plan and whose domain is the
        // plan lengths, never negative.
        let plan = match &grammar {
            ValueType::Reference(object) => {
                let lengths =
                    matches!(&domain, ValueType::Int(lengths) if !lengths.lower().is_negative());
                if !lengths {
                    return Err(vec![CheckRefusal::ill_typed(
                        &location,
                        IllTypedCause::TypeMismatch,
                    )]);
                }
                Some(
                    super::plan_value::declare(
                        scope,
                        named,
                        form,
                        object,
                        source_owners.get(&form.name.name).unwrap_or(source),
                        package_name,
                        &location,
                    )
                    .map_err(|refusal| vec![refusal])?,
                )
            }
            _ => None,
        };
        let candidate_type = match &plan {
            Some(plan) => ValueType::Composite(plan.shape.plan.clone()),
            None => domain.clone(),
        };
        let scope = plan.as_ref().map_or(scope, |plan| &plan.scope);
        let parameters = vec![(form.name.name.clone(), candidate_type)];
        let (satisfies, slot_names) = check_expression(
            scope,
            signatures,
            &form.satisfies.expression,
            &parameters,
            Some(&ValueType::Boolean),
            &satisfies_location,
        )
        .map_err(|refusal| vec![refusal])?;
        let objective = if let Some(objective) = &form.objective {
            let (body, _) = check_expression(
                scope,
                signatures,
                &objective.expression.expression,
                &parameters,
                None,
                &objective_location,
            )
            .map_err(|refusal| vec![refusal])?;
            if !is_numeric(body.root().value_type()) {
                return Err(vec![CheckRefusal::ill_typed(
                    &objective_location,
                    IllTypedCause::TypeMismatch,
                )]);
            }
            Some(body)
        } else {
            None
        };
        checked.push(CheckedSynthesis {
            form: form.clone(),
            grammar,
            domain,
            satisfies,
            objective,
            slot_names,
            plan,
        });
    }
    Ok(checked)
}

fn named_type(scope: &Scope, name: &str) -> Option<ValueType> {
    let [value_type] = scope.named_types(None, name) else {
        return None;
    };
    Some(value_type.clone())
}

/// Whether `value_type` is a numeric type an objective may rank candidates by.
fn is_numeric(value_type: &ValueType) -> bool {
    matches!(
        value_type,
        ValueType::Integer
            | ValueType::Int(_)
            | ValueType::Rational(_)
            | ValueType::Decimal(_)
            | ValueType::Float(_)
    )
}

fn check_expression(
    scope: &Scope,
    signatures: &Signatures,
    expression: &fsl_forms::Expression,
    parameters: &[(String, ValueType)],
    expected: Option<&ValueType>,
    location: &Location,
) -> Result<(CheckedBody, Vec<String>), CheckRefusal> {
    let limits = super::CheckingLimits::default();
    let mut nodes = 0;
    let typer = Typer::new(
        scope,
        signatures,
        limits,
        &mut nodes,
        fsl_forms::ClauseKind::Body,
    );
    let mut meter = filament_exact::Meter::new(super::family::SCALAR_LIMITS_UNLIMITED);
    let mut diagnostics = DiagnosticSink::default();
    let mut scopes = ScopeStack::default();
    let mut typer = typer.with_context(CheckContext::new(
        scope,
        StageLimits {
            input_bytes: limits.input_bytes(),
            node_count: limits.nodes(),
        },
        &mut meter,
        &mut diagnostics,
        &mut scopes,
    ));
    super::bind_parameters(&mut typer, parameters, location)?;
    let body = typer.infer(expression, expected, location)?;
    if expected == Some(&ValueType::Boolean) && body.root().value_type() != &ValueType::Boolean {
        return Err(CheckRefusal {
            location: location.clone(),
            cause: CheckCause::NonBooleanRoot,
        });
    }
    Ok((body, typer.slot_names().to_vec()))
}

/// Return the first component of `value_type` whose value set is unbounded.
/// Composite traversal follows declared field/member order, and recursive
/// composites are refused at the first back edge.
pub(crate) fn first_unbounded_component(
    types: &TypeEnvironment,
    value_type: &ValueType,
) -> Option<String> {
    fn visit(
        types: &TypeEnvironment,
        value_type: &ValueType,
        path: &str,
        composites: &mut Vec<filament_exact::NodeKey>,
    ) -> Option<String> {
        match value_type {
            ValueType::Boolean
            | ValueType::Int(_)
            | ValueType::Decimal(_)
            | ValueType::Float(_)
            | ValueType::Text(_)
            | ValueType::Uuid
            | ValueType::Timestamp
            | ValueType::Enum(_) => None,
            ValueType::Integer => Some(component(path, "Integer")),
            ValueType::Rational(_) => Some(component(path, "Rational")),
            ValueType::Quantity(_) | ValueType::CarriedQuantity(_) => {
                Some(component(path, "Quantity"))
            }
            ValueType::BoundedQuantity(_) => Some(component(path, "Quantity")),
            ValueType::String => Some(component(path, "String")),
            ValueType::Bytes => Some(component(path, "Bytes")),
            ValueType::JsonObject => Some(component(path, "JsonObject")),
            ValueType::Reference(_) | ValueType::Population(None) => {
                Some(component(path, "unbounded population"))
            }
            ValueType::Population(Some(_)) => None,
            ValueType::Option(inner) => visit(types, inner, &component(path, "some"), composites),
            ValueType::Collection(collection) => {
                if collection.bound().is_none() {
                    return Some(component(path, "unbounded collection"));
                }
                visit(
                    types,
                    collection.element(),
                    &component(path, "element"),
                    composites,
                )
            }
            ValueType::Map(map) => {
                if map.bound().is_none() {
                    return Some(component(path, "unbounded map"));
                }
                visit(types, map.key(), &component(path, "key"), composites)
                    .or_else(|| visit(types, map.values(), &component(path, "value"), composites))
            }
            ValueType::Composite(key) => {
                if composites.contains(key) {
                    return Some(component(path, "recursive composite"));
                }
                let Some(declaration) = types.composite(key.clone()) else {
                    return Some(component(path, "unknown composite"));
                };
                composites.push(key.clone());
                let result = match declaration.shape() {
                    CompositeShape::Record(fields) => fields.iter().find_map(|field| {
                        visit(
                            types,
                            field.value_type(),
                            &component(path, field.name()),
                            composites,
                        )
                    }),
                    CompositeShape::Tuple(items) => {
                        items.iter().enumerate().find_map(|(index, ty)| {
                            visit(types, ty, &component(path, &format!("{index}")), composites)
                        })
                    }
                    CompositeShape::Union(members) => members.iter().find_map(|member| {
                        member
                            .positions()
                            .iter()
                            .enumerate()
                            .find_map(|(index, ty)| {
                                visit(
                                    types,
                                    ty,
                                    &component(path, &format!("{}[{index}]", member.identifier())),
                                    composites,
                                )
                            })
                    }),
                };
                composites.pop();
                result
            }
        }
    }

    visit(types, value_type, "domain", &mut Vec::new())
}

fn component(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_owned()
    } else {
        format!("{parent}.{name}")
    }
}

#[cfg(test)]
mod tests {
    use super::first_unbounded_component;
    use filament_exact::{Identifier, NodeKey, PathSegment, Presence, ValueType};
    use filament_semantic_value::declaration::TypeEnvironment;
    use filament_semantic_value::declaration::{
        CompositeDeclaration, CompositeShape, FieldDeclaration, UnionMemberDeclaration,
    };
    use ix_trace_rs::trace;

    #[test]
    #[trace("TC-2104", "FR-754-AC-2")]
    fn bounded_scalar_domains_are_finite_and_integer_is_not() {
        let types = TypeEnvironment::default();
        assert_eq!(first_unbounded_component(&types, &ValueType::Boolean), None);
        assert_eq!(
            first_unbounded_component(&types, &ValueType::Integer),
            Some("domain.Integer".to_owned())
        );
    }

    #[test]
    #[trace("TC-2104", "FR-754-AC-2")]
    fn reports_unbounded_record_component_and_recursive_union() {
        let record_key = key("Record");
        let recursive_key = key("Recursive");
        let types = TypeEnvironment::new(
            [
                CompositeDeclaration::new(
                    record_key.clone(),
                    "Record",
                    CompositeShape::Record(vec![FieldDeclaration::new(
                        "payload",
                        ValueType::String,
                        Presence::Required,
                    )]),
                ),
                CompositeDeclaration::new(
                    recursive_key.clone(),
                    "Recursive",
                    CompositeShape::Union(vec![UnionMemberDeclaration::resolved(
                        Identifier::new("Next").expect("a member identifier"),
                        vec![ValueType::option(ValueType::Composite(
                            recursive_key.clone(),
                        ))],
                    )]),
                ),
            ],
            [],
        )
        .expect("the fixture composites are valid");

        assert_eq!(
            first_unbounded_component(&types, &ValueType::Composite(record_key)),
            Some("domain.payload.String".to_owned())
        );
        assert_eq!(
            first_unbounded_component(&types, &ValueType::Composite(recursive_key)),
            Some("domain.Next[0].some.recursive composite".to_owned())
        );
    }

    fn key(name: &str) -> NodeKey {
        NodeKey::new(
            "synthesis-tests",
            vec![PathSegment::Text(name.to_owned())],
            0,
        )
        .expect("a fixture key is valid")
    }
}
