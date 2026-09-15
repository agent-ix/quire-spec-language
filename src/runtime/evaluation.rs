// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-008: independent execution of the retained native AST over validated inputs.

mod api;
mod budget;
mod comparison;
mod value;
mod walk;

use quire_contract_ir as ir;
use std::collections::BTreeMap;

use super::{ObservationSelection, QualifiedName, ValidatedContext};
use crate::checking::{Catalog, NativeType};
use crate::linking::{DeclarationKey, ResolutionTarget};
use crate::syntax::{ClauseKind, Expr, ExprId};
use crate::{Code, Diagnostic, Span};
use budget::Budget;
use value::{Arena, Value};

pub use api::{
    EvaluationLimits, EvaluationOutcome, EvaluationReport, EvaluationUsage, ImplicationEvent,
    ImplicationEventKind,
};

type Result<T> = std::result::Result<T, Box<Diagnostic>>;

struct Evaluator<'a, 'checked, 'model, P> {
    context: &'a ValidatedContext<'checked, 'model>,
    budget: Budget<'a, P>,
    occurrences: BTreeMap<usize, &'a ResolutionTarget>,
    locals: BTreeMap<usize, Value<'a>>,
}

/// Execute exactly one validated clause with fresh state and explicit limits.
///
/// Cancellation and exhausted budgets return Incomplete with no Boolean. A
/// violated established invariant returns Refused. The caller's poll is the
/// only extension point; its panic propagates normally and creates no report.
pub fn evaluate<'a, 'checked, 'model>(
    context: &'a ValidatedContext<'checked, 'model>,
    limits: EvaluationLimits,
    poll: impl FnMut() -> bool,
) -> EvaluationReport<'a, 'checked, 'model> {
    let mut evaluator = Evaluator {
        context,
        budget: Budget {
            source: context.checked().linked().unit().source(),
            limits: limits.bounded(),
            usage: EvaluationUsage::default(),
            events: Vec::new(),
            poll,
        },
        occurrences: BTreeMap::new(),
        locals: BTreeMap::new(),
    };
    let outcome = match evaluator.run() {
        Ok(value) => EvaluationOutcome::Completed(value),
        Err(diagnostic) if diagnostic.is_incomplete() => EvaluationOutcome::Incomplete(diagnostic),
        Err(diagnostic) => EvaluationOutcome::Refused(diagnostic),
    };
    EvaluationReport {
        context,
        outcome,
        usage: evaluator.budget.usage,
        events: evaluator.budget.events,
    }
}

impl<'a, 'checked, 'model, P: FnMut() -> bool> Evaluator<'a, 'checked, 'model, P> {
    fn run(&mut self) -> Result<bool> {
        let index = self.context.clause_index();
        let unit = self.context.checked().linked().unit();
        let clause = unit.clauses().get(index).ok_or_else(|| {
            self.invariant(Span { start: 0, end: 0 }, "validated clause is absent")
        })?;
        let linked = self
            .context
            .checked()
            .linked()
            .clauses()
            .get(index)
            .ok_or_else(|| self.invariant(clause.span, "linked clause is absent"))?;
        for occurrence in linked.occurrences() {
            self.budget.poll(occurrence.span)?;
            if let Some(id) = occurrence.expression {
                self.occurrences.insert(id.0, &occurrence.target);
            }
        }
        let observation = match clause.kind {
            ClauseKind::Invariant => ir::StateObservation::Current,
            ClauseKind::Precondition => ir::StateObservation::Pre,
            ClauseKind::Postcondition => ir::StateObservation::Post,
        };
        let value = self.visit(clause.expression, observation, 1, None)?;
        self.boolean(value, clause.span)
    }

    fn invariant(&self, span: Span, message: &'static str) -> Box<Diagnostic> {
        self.budget.error(Code::RuntimeInvariant, span, message)
    }

    fn node(&self, id: ExprId) -> Result<&'a Expr> {
        self.context
            .checked()
            .linked()
            .unit()
            .expression(id)
            .ok_or_else(|| {
                self.invariant(Span { start: 0, end: 0 }, "checked expression is absent")
            })
    }

    fn ty(&self, id: ExprId) -> Result<&'a NativeType<'model>> {
        self.context.clause().expression_type(id).ok_or_else(|| {
            self.invariant(
                self.node(id)
                    .map_or(Span { start: 0, end: 0 }, |node| node.span),
                "checked expression type is absent",
            )
        })
    }

    fn catalog(&self, owner: &ir::RequirementRef, span: Span) -> Result<&'a Catalog<'model>> {
        self.context
            .checked()
            .catalogs()
            .iter()
            .find(|catalog| catalog.model.environment().owner() == owner)
            .ok_or_else(|| self.invariant(span, "checked model catalog is absent"))
    }

    fn boolean(&self, value: Value<'_>, span: Span) -> Result<bool> {
        if let Value::Boolean(value) = value {
            Ok(value)
        } else {
            Err(self.invariant(span, "checked Boolean has a different runtime shape"))
        }
    }

    fn integer(&self, value: Value<'_>, span: Span) -> Result<i64> {
        if let Value::Integer(value) = value {
            Ok(value)
        } else {
            Err(self.invariant(span, "checked integer has a different runtime shape"))
        }
    }

    fn integer_result(&self, id: ExprId, value: Option<i64>) -> Result<Value<'a>> {
        let span = self.node(id)?.span;
        let NativeType::Scalar {
            representation: ir::ValueType::Integer { value: integer },
            ..
        } = self.ty(id)?
        else {
            return Err(self.invariant(span, "checked integer result type is absent"));
        };
        let value = value
            .filter(|value| *value >= integer.minimum() && *value <= integer.maximum())
            .ok_or_else(|| {
                self.invariant(
                    span,
                    "checked arithmetic or its exact native result interval failed",
                )
            })?;
        Ok(Value::Integer(value))
    }

    fn arena(&self, observation: ir::StateObservation, span: Span) -> Result<Arena<'a>> {
        let snapshot = self
            .context
            .snapshot(observation)
            .ok_or_else(|| self.invariant(span, "validated snapshot is unavailable"))?;
        Ok(Arena {
            nodes: &snapshot.draft().arena,
            observation,
        })
    }

    fn arena_value(&self, arena: Arena<'a>, id: super::ValueId, span: Span) -> Result<Value<'a>> {
        arena
            .value(id)
            .ok_or_else(|| self.invariant(span, "validated value index is unavailable"))
    }

    fn declaration(&mut self, id: ExprId, observation: ir::StateObservation) -> Result<Value<'a>> {
        let span = self.node(id)?.span;
        let target = self
            .occurrences
            .get(&id.0)
            .copied()
            .ok_or_else(|| self.invariant(span, "read has no linked declaration"))?;
        match target {
            ResolutionTarget::Local(binding) => self
                .locals
                .get(&binding.start)
                .copied()
                .ok_or_else(|| self.invariant(span, "linked local is not active")),
            ResolutionTarget::Formal(location) => {
                let DeclarationKey::Value(name) = &location.identity.key else {
                    return Err(self.invariant(span, "read has a different declaration namespace"));
                };
                let catalog = self.catalog(&location.identity.owner, span)?;
                let declaration = catalog
                    .values
                    .get(name)
                    .ok_or_else(|| self.invariant(span, "checked value declaration is absent"))?;
                if declaration.kind() == ir::ValueDeclarationKind::State {
                    let name = QualifiedName {
                        model: location.identity.owner.clone(),
                        name: name.clone(),
                    };
                    let value = self.context.state(observation, &name).ok_or_else(|| {
                        self.invariant(span, "validated State root is unavailable")
                    })?;
                    self.arena_value(self.arena(observation, span)?, value, span)
                } else {
                    let invocation = self
                        .context
                        .invocation()
                        .ok_or_else(|| self.invariant(span, "validated invocation is unavailable"))?
                        .draft();
                    for parameter in &invocation.parameters {
                        self.budget.poll(span)?;
                        if parameter.declaration.model == location.identity.owner
                            && parameter.declaration.name == *name
                        {
                            return self.arena_value(
                                Arena {
                                    nodes: &invocation.arena,
                                    observation: ir::StateObservation::Pre,
                                },
                                parameter.value,
                                span,
                            );
                        }
                    }
                    Err(self.invariant(span, "validated parameter is unavailable"))
                }
            }
        }
    }

    fn field(&mut self, base: Value<'a>, name: &str, span: Span) -> Result<Value<'a>> {
        let (fields, arena) = match base {
            Value::Record { fields, arena } => (fields, arena),
            Value::Object {
                identity,
                observation,
            } => {
                let object = self.context.object(observation, identity).ok_or_else(|| {
                    self.invariant(
                        span,
                        "validated object is unavailable at its captured observation",
                    )
                })?;
                (object.fields.as_slice(), self.arena(observation, span)?)
            }
            _ => {
                return Err(
                    self.invariant(span, "checked field receiver has a different runtime shape")
                )
            }
        };
        for field in fields {
            self.budget.poll(span)?;
            if field.name.as_str() == name {
                return self.arena_value(arena, field.value, span);
            }
        }
        Err(self.invariant(span, "validated field is unavailable"))
    }
}
