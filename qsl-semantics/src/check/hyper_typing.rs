// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-172: the types of a `hyper` clause's atoms and `match` conjuncts and of
//! a `relation`'s body (ADR-023 HS-4, HS-5, HS-8).
//!
//! A body is typed in one forward pass over its expression arena, in which
//! every node follows its children, so no walk recurses. The types are the
//! ones a body can compare: Booleans, numbers of one kind (integers, rationals,
//! decimals, floats and quantities of one unit compare only with their own
//! kind), text, the operation and receiver labels of a step, the `Option<T>`
//! argument of a step and any other model value, which compares only with its
//! own type. The pass reads the member `p` of an execution `x` as its
//! parameter, `x.result` as its result and `x.self.f` as a field of its
//! receiver, a member of an object parameter as a field of the parameter's
//! object type, `E::m` as a member of a declared enum, `f(args)` as a call of
//! a declared function and `let n = v in b` as `b` with `n` bound to the type
//! of `v`. The value typer of the declarations (`check::check::typing`) binds
//! slots of declared types and has no execution, trace-read or step-label
//! root to bind, so this pass types the roots a hyper clause adds and
//! delegates nothing; any other form is refused as unsupported, not skipped.

use std::collections::BTreeMap;

use qsl_attrs::string_edge;
use qsl_forms::{BinaryOperator, ExprNode, Expression, ExpressionSpans, StepLabelForm};
use qsl_foundation::Span;
use quire_exact::{EffectiveId, UnitId, ValueType};

use super::check::Signatures;
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
    Integer,
    Rational,
    Decimal,
    Float,
    /// A quantity in exactly one unit.
    Quantity(UnitId),
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
            ValueType::Integer | ValueType::Int(_) => Self::Integer,
            ValueType::Rational(_) => Self::Rational,
            ValueType::Decimal(_) => Self::Decimal,
            ValueType::Float(_) => Self::Float,
            ValueType::Quantity(unit) => Self::Quantity(*unit),
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
            (Self::Quantity(left), Self::Quantity(right)) => left == right,
            (left, right) => {
                std::mem::discriminant(left) == std::mem::discriminant(right)
                    && !matches!(
                        left,
                        Self::Execution(_) | Self::ExecutionSelf(_) | Self::Quantity(_)
                    )
            }
        }
    }

    /// Whether this is a number of some kind.
    fn is_number(&self) -> bool {
        matches!(
            self,
            Self::Integer | Self::Rational | Self::Decimal | Self::Float | Self::Quantity(_)
        )
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
    /// A `let` binder named as another binder in scope or as a root.
    Shadows { name: String, span: Span },
}

/// What a body is typed against.
pub(crate) struct Env<'a> {
    pub(crate) scope: &'a Scope,
    /// The unit's declared functions, which a call resolves against.
    pub(crate) signatures: &'a Signatures,
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
    // Each node's subtree is the arena run `starts[i]..=i`, since a node
    // follows its children; a `let`'s body run is where its binder is in
    // scope.
    let mut starts: Vec<usize> = Vec::with_capacity(expression.len());
    let mut lets: Vec<Binding> = Vec::new();
    for node in expression.iter() {
        let index = node.id().index();
        let children = node.node().children();
        let start_of = |starts: &[usize], id: &qsl_forms::ExprId| {
            starts.get(id.index()).copied().unwrap_or(index)
        };
        if let ExprNode::Let { name, value, body } = node.node() {
            lets.push(Binding {
                name: name.clone(),
                value: value.index(),
                body_start: start_of(&starts, body),
                body: body.index(),
            });
        }
        let start = children
            .iter()
            .map(|id| start_of(&starts, id))
            .min()
            .unwrap_or(index);
        starts.push(start);
    }
    let mut faults = Vec::new();
    let mut types: Vec<Ty> = vec![Ty::Unknown; expression.len()];
    for id in evaluation_order(expression) {
        let node = expression.at(id);
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
            ExprNode::Integer(_) => Ty::Integer,
            ExprNode::Rational(..) => Ty::Rational,
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
            ExprNode::Name(name) => {
                let index = node.id().index();
                let binder = lets
                    .iter()
                    .filter(|bound| {
                        bound.name == *name && bound.body_start <= index && index <= bound.body
                    })
                    .max_by_key(|bound| bound.body_start);
                if let Some(bound) = binder {
                    types.get(bound.value).cloned().unwrap_or(Ty::Unknown)
                } else if let Some(root) = env.roots.get(name.as_str()) {
                    match root {
                        Root::Object(object) => Ty::Value(ValueType::Reference(*object)),
                        Root::Execution(_) => Ty::Execution(name.clone()),
                    }
                } else if let Some((binding, _)) = env.scope.enum_members_named(name).next() {
                    Ty::Value(ValueType::Enum(binding.shape()))
                } else {
                    missing(&mut faults, name)
                }
            }
            ExprNode::Let { name, body, .. } => {
                let index = node.id().index();
                let encloses = |bound: &Binding| {
                    bound.name == *name
                        && bound.body != body.index()
                        && bound.body_start <= index
                        && index <= bound.body
                };
                if env.roots.contains_key(name.as_str()) || lets.iter().any(encloses) {
                    faults.push(BodyFault::Shadows {
                        name: name.clone(),
                        span: at,
                    });
                }
                child(*body)
            }
            ExprNode::Call { name, arguments } if !silent => match env.signatures.callable(name) {
                None => missing(&mut faults, name),
                Some((_, signature)) => {
                    let accepted = signature.parameters.len() == arguments.len()
                        && signature.parameters.iter().zip(arguments).all(
                            |((_, declared), argument)| {
                                Ty::of(declared).compares_with(&child(*argument))
                            },
                        );
                    if accepted {
                        Ty::of(&signature.result)
                    } else {
                        ill(&mut faults)
                    }
                }
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
                let operand = child(*operand);
                if operand.is_number() || operand == Ty::Unknown {
                    operand
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
                        if numbers(&left, &right) {
                            Ty::Boolean
                        } else {
                            ill(&mut faults)
                        }
                    }
                    BinaryOperator::Add | BinaryOperator::Subtract => {
                        if numbers(&left, &right) {
                            known(left, right)
                        } else {
                            ill(&mut faults)
                        }
                    }
                    BinaryOperator::Multiply | BinaryOperator::Divide => {
                        if !numbers(&left, &right) {
                            ill(&mut faults)
                        } else if matches!(left, Ty::Quantity(_))
                            || matches!(right, Ty::Quantity(_))
                        {
                            // A product of units has no type here.
                            faults.push(BodyFault::Unsupported { span: at });
                            Ty::Unknown
                        } else {
                            known(left, right)
                        }
                    }
                    BinaryOperator::IntegerDiv
                    | BinaryOperator::IntegerRem
                    | BinaryOperator::IntegerMod => {
                        if [&left, &right]
                            .iter()
                            .all(|side| matches!(side, Ty::Integer | Ty::Unknown))
                        {
                            Ty::Integer
                        } else {
                            ill(&mut faults)
                        }
                    }
                }
            }
            ExprNode::Case(_)
            | ExprNode::Quantity(_)
            | ExprNode::Call { .. }
            | ExprNode::Present(_)
            | ExprNode::Value(_)
            | ExprNode::Deref(_)
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
        if let Some(slot) = types.get_mut(id.index()) {
            *slot = ty;
        }
    }
    // A body already refused for another fault is not refused again for the
    // type those faults left behind.
    if faults.is_empty()
        && !types
            .get(expression.root().id().index())
            .is_none_or(boolean)
    {
        faults.push(BodyFault::IllTyped {
            span: span_of(expression.root().id().index()),
        });
    }
    faults
}

/// The nodes of `expression` with every node after its operands and a
/// `let`'s value before its body, so a binder's type is known where its body
/// reads it. The arena order does neither: it stores a node's later operands
/// first.
fn evaluation_order(expression: &Expression) -> Vec<qsl_forms::ExprId> {
    let mut order = Vec::with_capacity(expression.len());
    let mut pending = vec![(expression.root().id(), false)];
    while let Some((id, expanded)) = pending.pop() {
        if expanded {
            order.push(id);
            continue;
        }
        pending.push((id, true));
        let node = expression.at(id);
        let mut operands = node.node().children();
        if let ExprNode::Let { value, body, .. } = node.node() {
            operands = vec![*value, *body];
        }
        // Popped last-first, so the first operand is typed first.
        pending.extend(operands.into_iter().rev().map(|operand| (operand, false)));
    }
    order
}

fn boolean(ty: &Ty) -> bool {
    matches!(ty, Ty::Boolean | Ty::Unknown)
}

/// Whether `left` and `right` are numbers of one kind.
fn numbers(left: &Ty, right: &Ty) -> bool {
    let number = |ty: &Ty| ty.is_number() || *ty == Ty::Unknown;
    number(left) && number(right) && left.compares_with(right)
}

/// The type of an operation over two numbers of one kind: the one that is
/// not unknown.
fn known(left: Ty, right: Ty) -> Ty {
    if left == Ty::Unknown {
        right
    } else {
        left
    }
}

/// A `let`'s binder, the arena index of its value and the arena run of its
/// body, in which the binder is in scope.
struct Binding {
    name: String,
    value: usize,
    body_start: usize,
    body: usize,
}

enum Field {
    Found(Box<Ty>),
    Missing,
    IllTyped,
    Passed,
}

/// The member `field` of a value of type `operand`. `#[string_edge]`: an
/// execution's reserved `self` and `result` members are read by their source
/// spelling, which is the language's own canonical text.
#[string_edge]
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A quantity compares only with a quantity of its own unit, never with a
    /// bare number.
    #[test]
    fn a_quantity_compares_only_with_its_own_unit() {
        let metre = Ty::Quantity(UnitId::compound([1; 32]));
        let second = Ty::Quantity(UnitId::compound([2; 32]));
        assert!(metre.compares_with(&metre.clone()));
        assert!(!metre.compares_with(&second));
        assert!(!metre.compares_with(&Ty::Integer));
        assert!(!Ty::Rational.compares_with(&Ty::Integer));
        assert!(Ty::Integer.compares_with(&Ty::Integer));
        assert!(!numbers(&metre, &Ty::Integer));
    }
}
