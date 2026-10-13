// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-300: static model-expression decisions under the enclosing S3 context.

use qsl_forms::{ClauseKind, TypeForm};
use qsl_foundation::absence::AbsenceMode;
use qsl_foundation::diagnostic::{CatalogCode, InternalFault, StageFailure, Staged};
use quire_exact::{
    CardinalityBound, CollectionKind, CollectionType, IllTypedCause, Presence, ValueType,
};
use quire_semantic_value::location::Location;

use super::check::Scope;
use super::ir::{Node, NodeId, NodeKind};
use super::refusal::{CheckCause, CheckRefusal, KeyFault};
use crate::family::{CheckContext, CheckOutcome, FamilyContract, FamilyKind};

/// A located refusal attributed to the model form that produced it.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct StateModelRefusal {
    /// The original static question's typed cause, retaining its raw vocabulary.
    pub cause: CheckCause,
}

impl StateModelRefusal {
    /// The owning semantic family, independent of the enclosing body.
    pub const fn family(&self) -> FamilyKind {
        FamilyKind::StateModel
    }

    /// The published code and cause, attributed only when rendered.
    pub fn catalog_code(&self) -> Option<CatalogCode> {
        Some(
            CatalogCode::new(self.cause.code().as_str(), self.cause.cause()?)
                .with_prefix(self.family().catalog_code_prefix()),
        )
    }
}

/// One static question about a model form. Operands are already typed and
/// stored in the enclosing typer's arena; this hook never traverses syntax.
pub(crate) enum Form {
    Target(TypeForm),
    Attribute {
        reference: NodeId,
        value_type: ValueType,
        field: String,
        operand_location: Location,
        derefed: bool,
    },
    Reaches {
        source: NodeId,
        source_type: ValueType,
        target: NodeId,
        target_type: ValueType,
        edge: String,
    },
    ReachesContext(bool),
    AllInstances {
        target: ValueType,
        population: NodeId,
        population_type: ValueType,
        operand_location: Location,
    },
    Population {
        value_type: ValueType,
    },
    Lookup {
        target: ValueType,
        population: NodeId,
        reference: NodeId,
        reference_type: ValueType,
        reference_location: Location,
        absence: AbsenceMode,
    },
    DispatchContext(ClauseKind),
    DispatchOperation {
        receiver_type: ValueType,
        member: String,
        arity: usize,
    },
    Dispatch {
        receiver: NodeId,
        operation: usize,
        arguments: Vec<NodeId>,
    },
    Argument {
        actual: ValueType,
        required: ValueType,
    },
}

pub(crate) struct Request {
    pub(crate) form: Form,
    pub(crate) location: Location,
}

pub(crate) enum Checked {
    Node(Box<Node>),
    Type(ValueType),
    Operation(usize),
    Admitted,
}

pub(crate) fn fault(location: &Location) -> CheckRefusal {
    CheckRefusal {
        location: location.clone(),
        cause: CheckCause::InternalFault(Box::new(KeyFault::StageFault(InternalFault::new(
            "S3",
            "state-model-check-result-shape",
        )))),
    }
}

/// The sole StateModel S3 family hook. StateModel has no S6a entry.
pub(crate) struct StateModelFamily;

impl FamilyContract for StateModelFamily {
    type Form = Request;
    type Checked = Checked;
    type Cause = CheckRefusal;
    type Declarations<'a> = Scope;
    type Claim = ();

    #[deny(clippy::wildcard_enum_match_arm)]
    fn check<'a>(
        request: &Request,
        cx: &mut CheckContext<'a, Scope>,
    ) -> CheckOutcome<Checked, CheckRefusal> {
        let scope = cx.declarations();
        let location = &request.location;
        let refuse = |at: &Location, cause| CheckRefusal {
            location: at.clone(),
            cause: CheckCause::StateModel(Box::new(StateModelRefusal { cause })),
        };
        let mismatch =
            |at: &Location| refuse(at, CheckCause::IllTyped(IllTypedCause::TypeMismatch));
        let ineligible =
            |at: &Location| refuse(at, CheckCause::IllTyped(IllTypedCause::OperatorIneligible));
        let missing = |name: &str| refuse(location, CheckCause::MissingName(name.to_owned()));
        let make = |kind, value_type| {
            Checked::Node(Box::new(Node {
                kind,
                value_type,
                location: location.clone(),
            }))
        };
        let checked = (|| -> Result<Checked, CheckRefusal> {
            Ok(match &request.form {
                Form::Target(form) => {
                    let value_type = super::type_form::resolve_type_form(scope, form, location)
                        .map_err(|error| refuse(&error.location, error.cause))?;
                    if !matches!(value_type, ValueType::Reference(_)) {
                        return Err(mismatch(location));
                    }
                    Checked::Type(value_type)
                }
                Form::Attribute {
                    reference,
                    value_type,
                    field,
                    operand_location,
                    derefed,
                } => {
                    let ValueType::Reference(object) = value_type else {
                        return Err(mismatch(operand_location));
                    };
                    let attribute = scope
                        .types()
                        .attribute(*object, field)
                        .ok_or_else(|| mismatch(location))?;
                    let declared = attribute.field();
                    let optional = declared.presence() == Presence::Optional;
                    let value_type = if optional {
                        ValueType::option(declared.value_type().clone())
                    } else {
                        declared.value_type().clone()
                    };
                    make(
                        NodeKind::Attribute {
                            reference: *reference,
                            field: attribute.identity(),
                            optional,
                            derefed: *derefed,
                        },
                        value_type,
                    )
                }
                Form::Reaches {
                    source,
                    source_type,
                    target,
                    target_type,
                    edge,
                } => {
                    let (ValueType::Reference(object), ValueType::Reference(other)) =
                        (source_type, target_type)
                    else {
                        return Err(ineligible(location));
                    };
                    let attribute = scope
                        .types()
                        .attribute(*object, edge)
                        .ok_or_else(|| missing(edge))?;
                    let owner = attribute.owner();
                    let endpoints_admitted = if scope.complete_model {
                        scope.types().conforms(*object, owner)
                            && scope.types().conforms(*other, owner)
                    } else {
                        *object == owner && *other == owner
                    };
                    if !endpoints_admitted {
                        return Err(ineligible(location));
                    }
                    let declared = attribute.field();
                    let names_object = |ty: &ValueType| ty == &ValueType::Reference(owner);
                    let admitted = match (declared.presence(), declared.value_type()) {
                        (_, ValueType::Reference(_)) => names_object(declared.value_type()),
                        (Presence::Required, ValueType::Collection(collection)) => {
                            collection.kind() == CollectionKind::Sequence
                                && names_object(collection.element())
                        }
                        _ => false,
                    };
                    if !admitted {
                        return Err(ineligible(location));
                    }
                    make(
                        NodeKind::Reaches {
                            source: *source,
                            target: *target,
                            edge: attribute.identity(),
                        },
                        ValueType::Boolean,
                    )
                }
                Form::ReachesContext(in_state_clause) => {
                    if !in_state_clause {
                        return Err(ineligible(location));
                    }
                    Checked::Admitted
                }
                Form::AllInstances {
                    target,
                    population,
                    population_type,
                    operand_location,
                } => {
                    let ValueType::Population(maximum) = population_type else {
                        return Err(ineligible(operand_location));
                    };
                    let bound = maximum
                        .map(|maximum| {
                            CardinalityBound::new(0, maximum)
                                .map_err(|_| refuse(location, CheckCause::UnrepresentableBound))
                        })
                        .transpose()?;
                    make(
                        NodeKind::AllInstances {
                            population: *population,
                        },
                        ValueType::collection(CollectionType::new(
                            CollectionKind::Set,
                            target.clone(),
                            bound,
                        )),
                    )
                }
                Form::Population { value_type } => {
                    if !matches!(value_type, ValueType::Population(_)) {
                        return Err(ineligible(location));
                    }
                    Checked::Admitted
                }
                Form::Lookup {
                    target,
                    population,
                    reference,
                    reference_type,
                    reference_location,
                    absence,
                } => {
                    let (ValueType::Reference(s), ValueType::Reference(t)) =
                        (reference_type, target)
                    else {
                        return Err(mismatch(reference_location));
                    };
                    if !scope.types().conforms(*s, *t) {
                        return Err(mismatch(reference_location));
                    }
                    let value_type = match absence {
                        AbsenceMode::Undefined | AbsenceMode::Refused => target.clone(),
                        AbsenceMode::Empty => ValueType::option(target.clone()),
                    };
                    make(
                        NodeKind::Lookup {
                            population: *population,
                            reference: *reference,
                            absence: *absence,
                        },
                        value_type,
                    )
                }
                Form::DispatchContext(kind) => {
                    if !matches!(
                        kind,
                        ClauseKind::Invariant
                            | ClauseKind::Precondition
                            | ClauseKind::Postcondition
                    ) {
                        return Err(ineligible(location));
                    }
                    Checked::Admitted
                }
                Form::DispatchOperation {
                    receiver_type,
                    member,
                    arity,
                } => {
                    let ValueType::Reference(receiver_type) = receiver_type else {
                        return Err(ineligible(location));
                    };
                    let (index, operation) = scope
                        .dispatch_operations()
                        .iter()
                        .enumerate()
                        .find(|(_, operation)| {
                            operation.receiver_type == *receiver_type && operation.member == *member
                        })
                        .ok_or_else(|| missing(member))?;
                    if operation.parameters.len() != *arity {
                        return Err(mismatch(location));
                    }
                    Checked::Operation(index)
                }
                Form::Dispatch {
                    receiver,
                    operation,
                    arguments,
                } => {
                    let operation_decl = scope
                        .dispatch_operations()
                        .get(*operation)
                        .ok_or_else(|| fault(location))?;
                    make(
                        NodeKind::Dispatch {
                            receiver: *receiver,
                            table: operation_decl.table,
                            operation: *operation,
                            arguments: arguments.clone(),
                        },
                        operation_decl.result.clone(),
                    )
                }
                Form::Argument { actual, required } => {
                    // A reference upcast, or (FR-359 item 6) a record whose most-specific type conforms to the parameter's record type.
                    let admitted = actual == required
                        || match (actual, required) {
                            (ValueType::Reference(s), ValueType::Reference(t)) => {
                                scope.types().conforms(*s, *t)
                            }
                            (ValueType::Composite(s), ValueType::Composite(t)) => {
                                scope.types().composite_conforms(*s, *t)
                            }
                            _ => false,
                        };
                    if !admitted {
                        return Err(mismatch(location));
                    }
                    Checked::Type(required.clone())
                }
            })
        })()
        .map_err(StageFailure::Refused)?;
        cx.diagnostics.record(
            cx.scopes,
            format!(
                "state-model: checked form (node limit={}, meter admissions={})",
                cx.limits().node_count,
                cx.meter.admission_count()
            ),
        );
        Ok(Staged::new(checked))
    }

    fn requirements(_checked: &Checked) -> Vec<()> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::family::fixtures::{empty_scope, root_location};
    use crate::family::{DiagnosticSink, ScopeStack, StageLimits};
    use ix_trace_rs::trace;

    #[trace("FR-300-AC-1")]
    #[test]
    fn nested_model_hook_uses_the_enclosing_meter_sink_limits_and_scope() {
        let scope = empty_scope();
        let mut meter = quire_exact::Meter::new(crate::check::SCALAR_LIMITS_UNLIMITED);
        for _ in 0..3 {
            meter
                .charge(quire_exact::Charge::new(
                    quire_exact::ChargePoint::FunctionCall,
                ))
                .unwrap();
        }
        let mut diagnostics = DiagnosticSink::default();
        let mut scopes = ScopeStack::default();
        scopes.enter("enclosing");
        diagnostics.record(&scopes, "existing diagnostic");
        let mut enclosing = CheckContext::new(
            &(),
            StageLimits {
                input_bytes: 123,
                node_count: 37,
            },
            &mut meter,
            &mut diagnostics,
            &mut scopes,
        );
        {
            let mut nested = enclosing.reborrow(&scope);
            assert_eq!(nested.limits().input_bytes, 123);
            let result = StateModelFamily::check(
                &Request {
                    form: Form::Population {
                        value_type: ValueType::Population(Some(3)),
                    },
                    location: root_location(),
                },
                &mut nested,
            )
            .unwrap()
            .into_value();
            assert!(matches!(result, Checked::Admitted));
        }
        assert_eq!(
            meter.admission_count(),
            3,
            "no replacement meter or double charge"
        );
        let entries = diagnostics.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].message, "existing diagnostic");
        assert_eq!(entries[1].scope, "enclosing");
        assert!(entries[1]
            .message
            .contains("node limit=37, meter admissions=3"));
        assert_eq!(scopes.path(), "enclosing");
    }

    /// FR-359-AC-8 (TC-1001): a model-operation argument whose record type
    /// conforms to the parameter's record type is admitted and keeps the
    /// parameter's declared type; a parent at a child-typed parameter, an
    /// unrelated record, and a collection of the child at a collection of the
    /// parent are refused `ill_typed`/`type-mismatch`.
    #[trace("FR-359-AC-8", "TC-1001")]
    #[test]
    fn a_dispatch_argument_admits_a_conforming_record_only() {
        use quire_exact::{CollectionKind, CollectionType, NodeKey, Presence};
        use quire_semantic_value::declaration::{
            CompositeDeclaration, CompositeShape, FieldDeclaration, TypeEnvironment,
        };
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
        .unwrap();
        let scope = crate::check::family::fixtures::scope_with(types, Vec::new());
        let mut meter = quire_exact::Meter::new(crate::check::SCALAR_LIMITS_UNLIMITED);
        let mut diagnostics = DiagnosticSink::default();
        let mut scopes = ScopeStack::default();
        let mut enclosing = CheckContext::new(
            &(),
            StageLimits {
                input_bytes: 123,
                node_count: 37,
            },
            &mut meter,
            &mut diagnostics,
            &mut scopes,
        );
        let mut argument = |actual: ValueType, required: ValueType| {
            let mut nested = enclosing.reborrow(&scope);
            StateModelFamily::check(
                &Request {
                    form: Form::Argument { actual, required },
                    location: root_location(),
                },
                &mut nested,
            )
            .map(|staged| staged.into_value())
        };
        let composite = ValueType::Composite;
        let sequence_of = |element| {
            ValueType::collection(CollectionType::new(CollectionKind::Sequence, element, None))
        };
        for (actual, required) in [
            (composite(taxed), composite(money)),
            (composite(money), composite(money)),
        ] {
            match argument(actual, required.clone()) {
                Ok(Checked::Type(declared)) => assert_eq!(declared, required),
                other => panic!("admitted at the parameter's type, not {:?}", other.is_ok()),
            }
        }
        for (actual, required) in [
            (composite(money), composite(taxed)),
            (composite(fee), composite(money)),
            (sequence_of(composite(taxed)), sequence_of(composite(money))),
        ] {
            assert!(argument(actual, required).is_err());
        }
    }
}
