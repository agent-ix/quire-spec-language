// SPDX-License-Identifier: AGPL-3.0-only
//! Shared value and temporal trees, with lexical environments carried explicitly.
use super::super::models::{ModelError, ModelErrorKind};
use super::*;
use crate::syntax::{Builtin, ExprKind};

impl Resolver<'_, '_> {
    fn value_occurrence(
        &mut self,
        expression: ExprId,
        span: Span,
        target: BinderId,
        env: Environment,
    ) -> Result<(), Exhaustion> {
        self.work.charge(Dimension::Bindings, 1)?;
        self.output.values.push(ValueOccurrence {
            unit: self.output.unit,
            expression,
            span,
            target,
            evaluation_anchor: env.anchor,
        });
        Ok(())
    }
    fn lookup_value(
        &mut self,
        expression: ExprId,
        name: &Spanned<String>,
        env: Environment,
    ) -> Result<(), Exhaustion> {
        self.work.charge(Dimension::References, 1)?;
        let mut cursor = env.frame;
        while let Some(index) = cursor {
            self.work.charge(Dimension::Edges, 1)?;
            let frame = &self.frames[index];
            if self.output.binders[frame.binder.0].name.as_deref() == Some(name.value.as_str()) {
                return self.value_occurrence(expression, name.span, frame.binder, env);
            }
            cursor = frame.parent;
        }
        self.issue(ScopeIssue::MissingValue {
            expression,
            span: name.span,
            name: name.value.clone(),
        })
    }
    fn child(
        &mut self,
        pending: &mut Vec<(ExprId, Environment)>,
        id: ExprId,
        env: Environment,
    ) -> Result<(), Exhaustion> {
        self.work.charge(Dimension::Edges, 1)?;
        pending.push((id, env));
        Ok(())
    }
    pub(super) fn expression(
        &mut self,
        root: ExprId,
        environment: Environment,
    ) -> Result<(), Exhaustion> {
        let mut pending = vec![(root, environment)];
        while let Some((id, env)) = pending.pop() {
            self.work.charge(Dimension::References, 1)?;
            let expression = self.unit.expression(id).expect("parser expression handle");
            match &expression.kind {
                c::ValueKind::Shared(kind) => match kind {
                    ExprKind::Name(name) => self.lookup_value(id, name, env)?,
                    ExprKind::SelfValue | ExprKind::ResultValue => {
                        let target = if matches!(kind, ExprKind::SelfValue) {
                            env.self_value
                        } else {
                            env.result
                        };
                        if let Some(target) = target {
                            self.value_occurrence(id, expression.span, target, env)?;
                        } else {
                            self.issue(ScopeIssue::AmbientUnavailable {
                                expression: id,
                                span: expression.span,
                            })?;
                        }
                    }
                    ExprKind::Let { name, value, body } => {
                        let binder = self.binder(Binder {
                            name: Some(name.value.clone()),
                            span: name.span,
                            kind: BinderKind::Let,
                            anchor: env.anchor,
                            ty: BinderType::Initializer(*value),
                        })?;
                        let body_env = self.extend(env, binder)?;
                        self.child(&mut pending, *body, body_env)?;
                        self.child(&mut pending, *value, env)?;
                    }
                    ExprKind::Quantifier {
                        name,
                        domain,
                        predicate,
                        ..
                    } => {
                        let binder = self.binder(Binder {
                            name: Some(name.value.clone()),
                            span: name.span,
                            kind: BinderKind::Query,
                            anchor: env.anchor,
                            ty: BinderType::ElementOf(*domain),
                        })?;
                        let body_env = self.extend(env, binder)?;
                        self.child(&mut pending, *predicate, body_env)?;
                        self.child(&mut pending, *domain, env)?;
                    }
                    ExprKind::Call {
                        builtin: Builtin::Pre,
                        argument,
                    } => {
                        if let Some(pre_self) = env.pre_self {
                            self.child(
                                &mut pending,
                                *argument,
                                Environment {
                                    anchor: Anchor::InvocationPre,
                                    self_value: Some(pre_self),
                                    result: None,
                                    ..env
                                },
                            )?;
                        } else {
                            self.issue(ScopeIssue::AmbientUnavailable {
                                expression: id,
                                span: expression.operator_span.unwrap_or(expression.span),
                            })?;
                            self.child(&mut pending, *argument, env)?;
                        }
                    }
                    ExprKind::Group { inner } => self.child(&mut pending, *inner, env)?,
                    ExprKind::Field { base, .. } => self.child(&mut pending, *base, env)?,
                    ExprKind::Unary { argument, .. } | ExprKind::Call { argument, .. } => {
                        self.child(&mut pending, *argument, env)?
                    }
                    ExprKind::Binary { left, right, .. } => {
                        self.child(&mut pending, *right, env)?;
                        self.child(&mut pending, *left, env)?;
                    }
                    ExprKind::If {
                        condition,
                        then_value,
                        else_value,
                    } => {
                        self.child(&mut pending, *else_value, env)?;
                        self.child(&mut pending, *then_value, env)?;
                        self.child(&mut pending, *condition, env)?;
                    }
                    ExprKind::Reaches { start, target, .. } => {
                        self.child(&mut pending, *target, env)?;
                        self.child(&mut pending, *start, env)?;
                    }
                    ExprKind::Boolean(_)
                    | ExprKind::Integer(_)
                    | ExprKind::Text(_)
                    | ExprKind::EnumValue { .. } => {}
                },
                c::ValueKind::Invoke { arguments, .. } => {
                    for argument in arguments.iter().rev() {
                        self.child(&mut pending, *argument, env)?;
                    }
                }
                c::ValueKind::Rational { .. } => {}
                c::ValueKind::Product { left, right, .. } => {
                    self.child(&mut pending, *right, env)?;
                    self.child(&mut pending, *left, env)?;
                }
                c::ValueKind::Size { argument, .. } => self.child(&mut pending, *argument, env)?,
                c::ValueKind::Contains { collection, member } => {
                    self.child(&mut pending, *member, env)?;
                    self.child(&mut pending, *collection, env)?;
                }
                c::ValueKind::Query {
                    binder,
                    domain,
                    body,
                    ..
                } => {
                    let id = self.binder(Binder {
                        name: Some(binder.value.clone()),
                        span: binder.span,
                        kind: BinderKind::Query,
                        anchor: env.anchor,
                        ty: BinderType::ElementOf(*domain),
                    })?;
                    let body_env = self.extend(env, id)?;
                    self.child(&mut pending, *body, body_env)?;
                    self.child(&mut pending, *domain, env)?;
                }
            }
        }
        Ok(())
    }
    pub(super) fn temporal(
        &mut self,
        root: c::TemporalId,
        env: Environment,
    ) -> Result<(), Exhaustion> {
        let mut pending = vec![root];
        while let Some(id) = pending.pop() {
            self.work.charge(Dimension::References, 1)?;
            match &self.unit.temporal(id).expect("parser temporal handle").kind {
                c::TemporalKind::Constant(_) => {}
                c::TemporalKind::Holds(value) => self.expression(*value, env)?,
                c::TemporalKind::Group(inner)
                | c::TemporalKind::Unary {
                    argument: inner, ..
                } => {
                    self.work.charge(Dimension::Edges, 1)?;
                    pending.push(*inner);
                }
                c::TemporalKind::Binary { left, right, .. } => {
                    self.work.charge(Dimension::Edges, 1)?;
                    pending.push(*right);
                    self.work.charge(Dimension::Edges, 1)?;
                    pending.push(*left);
                }
            }
        }
        Ok(())
    }
    pub(super) fn declaration(
        &mut self,
        declaration: &c::Declaration,
        models: &ModelBindings<'_>,
    ) -> Result<(), Exhaustion> {
        self.work.charge(Dimension::References, 1)?;
        match &declaration.kind {
            c::DeclarationKind::Predicate {
                parameters, body, ..
            } => {
                let mut env = Environment::new(Anchor::Predicate);
                for parameter in parameters {
                    let binder =
                        self.parameter(parameter, BinderKind::Parameter, Anchor::Predicate)?;
                    env = self.extend(env, binder)?;
                }
                self.expression(*body, env)
            }
            c::DeclarationKind::State {
                kind,
                context,
                operation,
                body,
            } => self.state(*kind, context, operation.as_ref(), *body, models),
            c::DeclarationKind::Temporal {
                input,
                activation,
                captures,
                formula,
                ..
            } => {
                let mut env = Environment::new(Anchor::TemporalInstant);
                let binder = self.parameter(input, BinderKind::Input, Anchor::TemporalInstant)?;
                env = self.extend(env, binder)?;
                env = self.activation(activation, env, captures)?;
                self.temporal(*formula, env)
            }
            c::DeclarationKind::Protocol(protocol) => self.protocol(protocol),
        }
    }
    fn state(
        &mut self,
        kind: ClauseKind,
        context: &QualifiedName,
        operation: Option<&Spanned<String>>,
        body: ExprId,
        models: &ModelBindings<'_>,
    ) -> Result<(), Exhaustion> {
        let anchor = match kind {
            ClauseKind::Invariant => Anchor::Current,
            ClauseKind::Precondition => Anchor::InvocationPre,
            ClauseKind::Postcondition => Anchor::InvocationPost,
        };
        let mut env = Environment::new(anchor);
        env.self_value = Some(self.binder(Binder {
            name: None,
            span: context.name.span,
            kind: BinderKind::SelfValue,
            anchor,
            ty: BinderType::Context(context.clone()),
        })?);
        if kind == ClauseKind::Postcondition {
            env.pre_self = Some(self.binder(Binder {
                name: None,
                span: context.name.span,
                kind: BinderKind::SelfValue,
                anchor: Anchor::InvocationPre,
                ty: BinderType::Context(context.clone()),
            })?);
        }
        if let Some(name) = operation {
            let operation = c::Operation {
                context: context.clone(),
                name: name.clone(),
            };
            let resolved = match models.resolve_operation(self.output.unit, &operation, self.work) {
                Ok(operation) => operation,
                Err(ModelError {
                    kind: ModelErrorKind::ResourceExhausted(exhaustion),
                    ..
                }) => return Err(exhaustion),
                Err(_) => {
                    if let Some(exhaustion) = models.exhaustion() {
                        return Err(*exhaustion);
                    }
                    self.issue(ScopeIssue::ModelOperationUnavailable { span: name.span })?;
                    // Missing operation inputs cannot be diagnosed as absent lexical names.
                    return Ok(());
                }
            };
            for parameter in &resolved.role().parameters {
                match models.value(
                    self.output.unit,
                    &resolved,
                    &Spanned {
                        value: parameter.as_str().into(),
                        span: name.span,
                    },
                    self.work,
                ) {
                    Ok(value) => {
                        let binder = self.binder(Binder {
                            name: Some(parameter.as_str().into()),
                            span: name.span,
                            kind: BinderKind::InvocationParameter,
                            anchor: Anchor::InvocationInput,
                            ty: BinderType::ModelValue(value.location().clone()),
                        })?;
                        env = self.extend(env, binder)?;
                    }
                    Err(ModelError {
                        kind: ModelErrorKind::ResourceExhausted(exhaustion),
                        ..
                    }) => return Err(exhaustion),
                    Err(_) => {
                        self.issue(ScopeIssue::ModelOperationUnavailable { span: name.span })?;
                        return Ok(());
                    }
                }
            }
            if kind == ClauseKind::Postcondition {
                if let Some(result) = &resolved.role().result {
                    match models.value(
                        self.output.unit,
                        &resolved,
                        &Spanned {
                            value: result.as_str().into(),
                            span: name.span,
                        },
                        self.work,
                    ) {
                        Ok(value) => {
                            env.result = Some(self.binder(Binder {
                                name: None,
                                span: name.span,
                                kind: BinderKind::ResultValue,
                                anchor: Anchor::InvocationPost,
                                ty: BinderType::ModelValue(value.location().clone()),
                            })?)
                        }
                        Err(ModelError {
                            kind: ModelErrorKind::ResourceExhausted(exhaustion),
                            ..
                        }) => return Err(exhaustion),
                        Err(_) => {
                            self.issue(ScopeIssue::ModelOperationUnavailable { span: name.span })?;
                            return Ok(());
                        }
                    }
                }
            }
        }
        self.expression(body, env)
    }
}
