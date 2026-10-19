// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-172: the types of a `hyper` clause's atoms and `match` conjuncts and of
//! a `relation`'s body (ADR-023 HS-4, HS-5, HS-8).
//!
//! A body is typed in one forward pass over its expression arena, in which
//! every node follows its children, so no walk recurses. The types are the
//! ones a body can compare: Booleans, numbers, text, the operation and
//! receiver labels of a step, the `Option<T>` argument of a step and any other
//! model value, which compares only with its own type. The pass reads the
//! member `p` of an execution `x` as its parameter, `x.result` as its result
//! and `x.self.f` as a field of its receiver, and a member of an object
//! parameter as a field of the parameter's object type. A form outside that
//! set is refused as unsupported, not skipped.

use std::collections::BTreeMap;

use qsl_forms::{BinaryOperator, ExprNode, Expression, ExpressionSpans, StepLabelForm};
use qsl_foundation::Span;
use quire_exact::{EffectiveId, ValueType};

use super::hyper_clause::walk;
use super::Scope;

/// What a name at the root of a body denotes.
pub(crate) enum Root {
    /// An object parameter of a `hyper` clause.
    Object(EffectiveId),
    /// An execution variable of a `relation`.
    Execution(Box<ExecutionRoot>),
}

/// The operation an execution variable ranges over.
pub(crate) struct ExecutionRoot {
    /// The object type that declares the operation.
    pub(crate) declaring: EffectiveId,
    /// The operation's parameters.
    pub(crate) parameters: Vec<(String, ValueType)>,
    /// The operation's result.
    pub(crate) result: Option<ValueType>,
}

/// The type of a body expression.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Ty {
    Boolean,
    Numeric,
    Text,
    /// The operation of a step, or `stutter`.
    Label,
    /// A step's receiver key.
    Receiver,
    /// A step's argument, `Option<T>` for the parameter's `T`.
    Argument(ValueType),
    /// An execution variable.
    Execution(String),
    /// An execution's receiver.
    ExecutionSelf(String),
    /// Any other model value.
    Value(ValueType),
    /// A form already refused elsewhere; it compares with everything.
    Unknown,
}

impl Ty {
    /// The type a declared value type is compared as.
    pub(crate) fn of(value_type: &ValueType) -> Self {
        match value_type {
            ValueType::Boolean => Self::Boolean,
            ValueType::Integer
            | ValueType::Int(_)
            | ValueType::Rational(_)
            | ValueType::Decimal(_)
            | ValueType::Float(_)
            | ValueType::Quantity(_) => Self::Numeric,
            ValueType::Text(_) | ValueType::String => Self::Text,
            other => Self::Value(other.clone()),
        }
    }

    fn compares_with(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Unknown, _) | (_, Self::Unknown) => true,
            (Self::Argument(left), Self::Argument(right)) => {
                Self::of(left).compares_with(&Self::of(right))
            }
            (Self::Value(left), Self::Value(right)) => left == right,
            (left, right) => {
                std::mem::discriminant(left) == std::mem::discriminant(right)
                    && !matches!(left, Self::Execution(_) | Self::ExecutionSelf(_))
            }
        }
    }
}

/// Where a body is read.
pub(crate) enum Site {
    /// A `holds(...)` atom or a relation body.
    Body,
    /// A `match` conjunction: it reads step labels only, and any other read is
    /// refused by the `match` check.
    Match,
}

/// A refusal of a body, before it becomes a cause.
pub(crate) enum BodyFault {
    /// A name or member that is declared nowhere it is read.
    Missing { name: String, span: Span },
    /// An operand of the wrong type.
    IllTyped { span: Span },
    /// A form this typing does not cover.
    Unsupported { span: Span },
}

/// What a body is typed against.
pub(crate) struct Env<'a> {
    pub(crate) scope: &'a Scope,
    pub(crate) roots: BTreeMap<&'a str, Root>,
    pub(crate) site: Site,
    /// The type of a step label; `None` when the label is refused elsewhere.
    pub(crate) label: &'a dyn Fn(&str, &StepLabelForm) -> Option<Ty>,
}

/// The faults of `expression`, whose root must be Boolean.
pub(crate) fn type_condition(
    env: &Env<'_>,
    expression: &Expression,
    spans: &ExpressionSpans,
) -> Vec<BodyFault> {
    let mut located: Vec<Option<Span>> = vec![None; expression.len()];
    walk(expression, spans, |visit| {
        if let Some(slot) = located.get_mut(visit.node.id().index()) {
            *slot = visit.span;
        }
    });
    let span_of = |index: usize| {
        located
            .get(index)
            .copied()
            .flatten()
            .unwrap_or(Span { start: 0, end: 0 })
    };
    let mut faults = Vec::new();
    let mut types: Vec<Ty> = Vec::with_capacity(expression.len());
    for node in expression.iter() {
        let at = span_of(node.id().index());
        let child = |id: qsl_forms::ExprId| types.get(id.index()).cloned().unwrap_or(Ty::Unknown);
        let ill = |faults: &mut Vec<BodyFault>| {
            faults.push(BodyFault::IllTyped { span: at });
            Ty::Unknown
        };
        let missing = |faults: &mut Vec<BodyFault>, name: &str| {
            faults.push(BodyFault::Missing {
                name: name.to_owned(),
                span: at,
            });
            Ty::Unknown
        };
        let silent = matches!(env.site, Site::Match);
        let ty = match node.node() {
            ExprNode::Boolean(_) => Ty::Boolean,
            ExprNode::Integer(_) | ExprNode::Rational(..) => Ty::Numeric,
            ExprNode::String(_) => Ty::Text,
            ExprNode::Stutter => Ty::Label,
            ExprNode::StepLabel { trace, label } => {
                (env.label)(trace, label).unwrap_or(Ty::Unknown)
            }
            // Reads of state in a `match` are refused by the `match` check.
            ExprNode::Name(_)
            | ExprNode::Field { .. }
            | ExprNode::Indexed { .. }
            | ExprNode::Pre(_)
                if silent =>
            {
                Ty::Unknown
            }
            ExprNode::Name(name) => match env.roots.get(name.as_str()) {
                Some(Root::Object(object)) => Ty::Value(ValueType::Reference(*object)),
                Some(Root::Execution(_)) => Ty::Execution(name.clone()),
                None => missing(&mut faults, name),
            },
            ExprNode::Indexed { read, .. } | ExprNode::Pre(read) => child(*read),
            ExprNode::Field { operand, field } => match field_type(env, &child(*operand), field) {
                Field::Found(ty) => *ty,
                Field::Missing => missing(&mut faults, field),
                Field::IllTyped => ill(&mut faults),
                Field::Passed => Ty::Unknown,
            },
            ExprNode::Not(operand) => {
                if boolean(&child(*operand)) {
                    Ty::Boolean
                } else {
                    ill(&mut faults)
                }
            }
            ExprNode::Negate(operand) => {
                if numeric(&child(*operand)) {
                    Ty::Numeric
                } else {
                    ill(&mut faults)
                }
            }
            ExprNode::If {
                condition,
                then,
                otherwise,
            } => {
                let (then, otherwise) = (child(*then), child(*otherwise));
                if boolean(&child(*condition)) && then.compares_with(&otherwise) {
                    then
                } else {
                    ill(&mut faults)
                }
            }
            ExprNode::Binary {
                operator,
                left,
                right,
            } => {
                let (left, right) = (child(*left), child(*right));
                match operator {
                    BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Implies => {
                        if boolean(&left) && boolean(&right) {
                            Ty::Boolean
                        } else {
                            ill(&mut faults)
                        }
                    }
                    BinaryOperator::Equal | BinaryOperator::NotEqual => {
                        if left.compares_with(&right) {
                            Ty::Boolean
                        } else {
                            ill(&mut faults)
                        }
                    }
                    BinaryOperator::Less
                    | BinaryOperator::LessOrEqual
                    | BinaryOperator::Greater
                    | BinaryOperator::GreaterOrEqual => {
                        if numeric(&left) && numeric(&right) {
                            Ty::Boolean
                        } else {
                            ill(&mut faults)
                        }
                    }
                    BinaryOperator::Add
                    | BinaryOperator::Subtract
                    | BinaryOperator::Multiply
                    | BinaryOperator::Divide
                    | BinaryOperator::IntegerDiv
                    | BinaryOperator::IntegerRem
                    | BinaryOperator::IntegerMod => {
                        if numeric(&left) && numeric(&right) {
                            Ty::Numeric
                        } else {
                            ill(&mut faults)
                        }
                    }
                }
            }
            ExprNode::Case(_)
            | ExprNode::Let { .. }
            | ExprNode::Present(_)
            | ExprNode::Value(_)
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
            | ExprNode::SelfRef
            | ExprNode::Result
            | ExprNode::Reaches { .. }
            | ExprNode::ProtocolReference { .. } => {
                if !silent {
                    faults.push(BodyFault::Unsupported { span: at });
                }
                Ty::Unknown
            }
            #[cfg(seam_probe)]
            ExprNode::__SeamProbe => Ty::Unknown,
        };
        types.push(ty);
    }
    if !types.last().is_none_or(boolean) {
        faults.push(BodyFault::IllTyped {
            span: span_of(expression.root().id().index()),
        });
    }
    faults
}

fn boolean(ty: &Ty) -> bool {
    matches!(ty, Ty::Boolean | Ty::Unknown)
}

fn numeric(ty: &Ty) -> bool {
    matches!(ty, Ty::Numeric | Ty::Unknown)
}

enum Field {
    Found(Box<Ty>),
    Missing,
    IllTyped,
    Passed,
}

/// The member `field` of a value of type `operand`.
fn field_type(env: &Env<'_>, operand: &Ty, field: &str) -> Field {
    let attribute = |object: EffectiveId| {
        env.scope
            .types()
            .attribute(object, field)
            .map_or(Field::Missing, |attribute| {
                Field::Found(Box::new(Ty::of(attribute.field().value_type())))
            })
    };
    match operand {
        Ty::Unknown => Field::Passed,
        Ty::Value(ValueType::Reference(object)) => attribute(*object),
        Ty::Execution(name) => match env.roots.get(name.as_str()) {
            Some(Root::Execution(execution)) => match field {
                "self" => Field::Found(Box::new(Ty::ExecutionSelf(name.clone()))),
                "result" => execution.result.as_ref().map_or(Field::Missing, |result| {
                    Field::Found(Box::new(Ty::of(result)))
                }),
                _ => execution
                    .parameters
                    .iter()
                    .find(|(declared, _)| declared == field)
                    .map_or(Field::Missing, |(_, value_type)| {
                        Field::Found(Box::new(Ty::of(value_type)))
                    }),
            },
            _ => Field::IllTyped,
        },
        Ty::ExecutionSelf(name) => match env.roots.get(name.as_str()) {
            Some(Root::Execution(execution)) => attribute(execution.declaring),
            _ => Field::IllTyped,
        },
        _ => Field::IllTyped,
    }
}
