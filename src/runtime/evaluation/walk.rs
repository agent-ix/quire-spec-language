// SPDX-License-Identifier: AGPL-3.0-only
//! FR-008: left-to-right native execution, lexical captures and finite graph walks.

use super::*;
use crate::syntax::{BinaryOp, Builtin, ExprKind, UnaryOp};
use std::collections::BTreeSet;

impl<'a, P: FnMut() -> bool> Evaluator<'a, '_, '_, P> {
    pub(super) fn visit(
        &mut self,
        mut id: ExprId,
        observation: ir::StateObservation,
        depth: usize,
        entry: Option<ImplicationEvent>,
    ) -> Result<Value<'a>> {
        // Groups preserve source lineage but consume neither fuel nor stack frames.
        while let ExprKind::Group { inner } = &self.node(id)?.kind {
            self.budget.poll(self.node(id)?.span)?;
            id = *inner;
        }
        let node = self.node(id)?;
        self.budget.enter(node.span, depth, entry)?;
        let span = node.span;
        match &node.kind {
            ExprKind::Group { .. } => {
                Err(self.invariant(span, "group traversal did not reach its child"))
            }
            ExprKind::Boolean(value) => Ok(Value::Boolean(*value)),
            ExprKind::Integer(text) => self.integer_result(id, text.parse().ok()),
            ExprKind::Text(value) => Ok(Value::Text(value)),
            ExprKind::Name(_) => self.declaration(id, observation),
            ExprKind::SelfValue => {
                let identity = match &self.context.selection().observation {
                    ObservationSelection::Current { self_object, .. } => self_object,
                    ObservationSelection::Invocation { .. } => {
                        &self
                            .context
                            .invocation()
                            .ok_or_else(|| {
                                self.invariant(span, "validated invocation self is unavailable")
                            })?
                            .draft()
                            .self_object
                    }
                };
                Ok(Value::Object {
                    identity,
                    observation,
                })
            }
            ExprKind::ResultValue => {
                let invocation = self
                    .context
                    .invocation()
                    .ok_or_else(|| {
                        self.invariant(span, "validated result invocation is unavailable")
                    })?
                    .draft();
                let result = invocation.result.ok_or_else(|| {
                    self.invariant(span, "validated invocation result is unavailable")
                })?;
                self.arena_value(
                    Arena {
                        nodes: &invocation.arena,
                        observation: ir::StateObservation::Post,
                    },
                    result,
                    span,
                )
            }
            ExprKind::EnumValue { variant, .. } => {
                let NativeType::Enumeration { model, declaration } = self.ty(id)? else {
                    return Err(self.invariant(span, "checked enum type is absent"));
                };
                Ok(Value::Enumeration {
                    owner: model.environment().owner(),
                    name: declaration.name(),
                    variant: &variant.value,
                })
            }
            ExprKind::Field { base, name } => {
                let base = self.visit(*base, observation, depth + 1, None)?;
                self.field(base, &name.value, span)
            }
            ExprKind::Unary { op, argument } => {
                let argument = self.visit(*argument, observation, depth + 1, None)?;
                match op {
                    UnaryOp::Not => Ok(Value::Boolean(!self.boolean(argument, span)?)),
                    UnaryOp::Negate => {
                        self.integer_result(id, self.integer(argument, span)?.checked_neg())
                    }
                }
            }
            ExprKind::Binary { op, left, right } => {
                self.binary(id, *op, *left, *right, observation, depth)
            }
            ExprKind::Call { builtin, argument } => {
                let selected = if *builtin == Builtin::Pre {
                    ir::StateObservation::Pre
                } else {
                    observation
                };
                let argument = self.visit(*argument, selected, depth + 1, None)?;
                match (builtin, argument) {
                    (Builtin::Pre, value) => Ok(value),
                    (Builtin::Present, Value::Option { value, .. }) => {
                        Ok(Value::Boolean(value.is_some()))
                    }
                    (
                        Builtin::Value,
                        Value::Option {
                            value: Some(value),
                            arena,
                        },
                    ) => self.arena_value(arena, value, span),
                    (
                        Builtin::Deref,
                        Value::Reference {
                            identity,
                            observation,
                        },
                    ) => {
                        if self.context.object(observation, identity).is_none() {
                            return Err(
                                self.invariant(span, "validated dereference target is absent")
                            );
                        }
                        Ok(Value::Object {
                            identity,
                            observation,
                        })
                    }
                    (Builtin::Size, Value::Sequence { values, .. }) => {
                        self.integer_result(id, i64::try_from(values.len()).ok())
                    }
                    _ => {
                        Err(self.invariant(span, "checked builtin argument or definedness failed"))
                    }
                }
            }
            ExprKind::Let { name, value, body } => {
                let value = self.visit(*value, observation, depth + 1, None)?;
                self.locals.insert(name.span.start, value);
                let result = self.visit(*body, observation, depth + 1, None);
                self.locals.remove(&name.span.start);
                result
            }
            ExprKind::If {
                condition,
                then_value,
                else_value,
            } => {
                let condition = self.visit(*condition, observation, depth + 1, None)?;
                let branch = if self.boolean(condition, span)? {
                    then_value
                } else {
                    else_value
                };
                self.visit(*branch, observation, depth + 1, None)
            }
            ExprKind::Quantifier {
                universal,
                name,
                domain,
                predicate,
            } => {
                let domain = self.visit(*domain, observation, depth + 1, None)?;
                let Value::Sequence { values, arena } = domain else {
                    return Err(self.invariant(span, "checked quantifier domain is not a sequence"));
                };
                for value in values {
                    self.budget.poll(span)?;
                    let value = self.arena_value(arena, *value, span)?;
                    self.locals.insert(name.span.start, value);
                    let result = self.visit(*predicate, observation, depth + 1, None);
                    self.locals.remove(&name.span.start);
                    let truth = self.boolean(result?, self.node(*predicate)?.span)?;
                    if truth != *universal {
                        return Ok(Value::Boolean(truth));
                    }
                }
                self.budget.poll(span)?;
                Ok(Value::Boolean(*universal))
            }
            ExprKind::Reaches {
                start,
                target,
                field,
            } => {
                let start = self.visit(*start, observation, depth + 1, None)?;
                let target = self.visit(*target, observation, depth + 1, None)?;
                self.reaches(start, target, &field.value, span)
                    .map(Value::Boolean)
            }
        }
    }

    fn event(
        &self,
        implication: ExprId,
        operand: ExprId,
        kind: ImplicationEventKind,
    ) -> Result<ImplicationEvent> {
        Ok(ImplicationEvent {
            implication,
            operand,
            span: self.node(operand)?.span,
            kind,
        })
    }

    fn binary(
        &mut self,
        id: ExprId,
        op: BinaryOp,
        left: ExprId,
        right: ExprId,
        observation: ir::StateObservation,
        depth: usize,
    ) -> Result<Value<'a>> {
        let span = self.node(id)?.span;
        let entry = if op == BinaryOp::Implies {
            Some(self.event(id, left, ImplicationEventKind::AntecedentEntered)?)
        } else {
            None
        };
        let a = self.visit(left, observation, depth + 1, entry)?;
        match op {
            BinaryOp::Implies | BinaryOp::And | BinaryOp::Or => {
                let truth = self.boolean(a, self.node(left)?.span)?;
                if op == BinaryOp::Implies {
                    self.budget.event(self.event(
                        id,
                        left,
                        ImplicationEventKind::AntecedentCompleted(truth),
                    )?)?;
                }
                if (op == BinaryOp::And && !truth) || (op == BinaryOp::Or && truth) {
                    return Ok(Value::Boolean(truth));
                }
                if op == BinaryOp::Implies && !truth {
                    return Ok(Value::Boolean(true));
                }
                let entry = if op == BinaryOp::Implies {
                    Some(self.event(id, right, ImplicationEventKind::ConsequentEntered)?)
                } else {
                    None
                };
                let b = self.visit(right, observation, depth + 1, entry)?;
                Ok(Value::Boolean(self.boolean(b, self.node(right)?.span)?))
            }
            _ => {
                let b = self.visit(right, observation, depth + 1, None)?;
                match op {
                    BinaryOp::Equal | BinaryOp::NotEqual => {
                        let equal = self.equal(self.ty(left)?, a, b, span, 1)?;
                        Ok(Value::Boolean(equal == (op == BinaryOp::Equal)))
                    }
                    BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual => {
                        let ordering = self.order(self.ty(left)?, a, b, span)?;
                        let truth = match op {
                            BinaryOp::Less => ordering.is_lt(),
                            BinaryOp::LessEqual => ordering.is_le(),
                            BinaryOp::Greater => ordering.is_gt(),
                            _ => ordering.is_ge(),
                        };
                        Ok(Value::Boolean(truth))
                    }
                    BinaryOp::Add
                    | BinaryOp::Subtract
                    | BinaryOp::Multiply
                    | BinaryOp::Divide
                    | BinaryOp::Remainder => {
                        let a = self.integer(a, span)?;
                        let b = self.integer(b, span)?;
                        let value = match op {
                            BinaryOp::Add => a.checked_add(b),
                            BinaryOp::Subtract => a.checked_sub(b),
                            BinaryOp::Multiply => a.checked_mul(b),
                            BinaryOp::Divide => a.checked_div(b),
                            _ => a.checked_rem(b),
                        };
                        self.integer_result(id, value)
                    }
                    BinaryOp::And | BinaryOp::Or | BinaryOp::Implies => {
                        Err(self.invariant(span, "Boolean dispatch invariant failed"))
                    }
                }
            }
        }
    }

    fn reaches(
        &mut self,
        start: Value<'a>,
        target: Value<'a>,
        field: &str,
        span: Span,
    ) -> Result<bool> {
        let (Some((mut current, observation)), Some((target, target_observation))) =
            (start.object_identity(), target.object_identity())
        else {
            return Err(self.invariant(span, "checked reaches arguments lack object identities"));
        };
        if observation != target_observation {
            return Err(self.invariant(span, "checked reaches observations differ"));
        }
        let mut visited = BTreeSet::new();
        loop {
            self.budget.poll(span)?;
            if visited.contains(current) {
                return Ok(false);
            }
            self.budget.graph(span)?;
            visited.insert(current);
            let edge = self.field(
                Value::Object {
                    identity: current,
                    observation,
                },
                field,
                span,
            )?;
            let Value::Option { value, arena } = edge else {
                return Err(self.invariant(span, "checked graph edge is not optional"));
            };
            let Some(value) = value else {
                return Ok(false);
            };
            let Value::Reference {
                identity,
                observation: edge_observation,
            } = self.arena_value(arena, value, span)?
            else {
                return Err(self.invariant(span, "checked graph edge is not a reference"));
            };
            if edge_observation != observation {
                return Err(self.invariant(span, "graph edge changed its observation"));
            }
            // Positive-length semantics: test the reached identity before suppressing a repeat.
            if identity == target {
                return Ok(true);
            }
            current = identity;
        }
    }
}
