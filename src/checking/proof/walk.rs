// SPDX-License-Identifier: AGPL-3.0-only
//! FR-016: native evaluation order and symbolic guard construction.

use super::*;

impl<'u, 'a> Builder<'u, 'a> {
    fn boolean_outcomes(
        &mut self,
        op: BinaryOp,
        left: &Outcomes,
        right: &Outcomes,
        native: ExprId,
    ) -> Result<Outcomes> {
        let span = self.span(native);
        let (yes, no) = match op {
            BinaryOp::And => {
                let yes = facts::sequential(&left.yes, &right.yes, &mut self.meter, span)?;
                let right_false = facts::sequential(&left.yes, &right.no, &mut self.meter, span)?;
                let no = facts::alternative(&left.no, &right_false, &mut self.meter, span)?;
                (yes, no)
            }
            BinaryOp::Or => {
                let right_true = facts::sequential(&left.no, &right.yes, &mut self.meter, span)?;
                let yes = facts::alternative(&left.yes, &right_true, &mut self.meter, span)?;
                let no = facts::sequential(&left.no, &right.no, &mut self.meter, span)?;
                (yes, no)
            }
            BinaryOp::Implies => {
                let right_true = facts::sequential(&left.yes, &right.yes, &mut self.meter, span)?;
                let yes = facts::alternative(&left.no, &right_true, &mut self.meter, span)?;
                let no = facts::sequential(&left.yes, &right.no, &mut self.meter, span)?;
                (yes, no)
            }
            _ => {
                return Err(failure(
                    self.meter.source,
                    Code::InvalidModelBinding,
                    span,
                    "expected a native Boolean operator",
                ))
            }
        };
        let mut outcomes = Outcomes { yes, no };
        outcomes.rebase(native, &mut self.meter, span)?;
        Ok(outcomes)
    }

    pub(super) fn visit(&mut self, id: ExprId, path: &Path) -> Result<Value> {
        let ty = &self.ty(id)?.ty;
        if path.facts.is_none() {
            return self.symbolic(Key::Expression(id.0), id, ty, false);
        }
        let kind = &self.linked.unit().expressions()[id.0].kind;
        let mut value = match kind {
            ExprKind::Group { inner } => self.visit(*inner, path)?,
            ExprKind::Boolean(value) => {
                let graph = self.node(Kind::Boolean(*value), id)?;
                self.expression_value(graph, id, true, Outcomes::constant(*value))
            }
            ExprKind::Integer(text) => {
                let integer = ty.integer().ok_or_else(|| {
                    failure(
                        self.meter.source,
                        Code::InvalidModelBinding,
                        self.span(id),
                        "integer proof has no native integer type",
                    )
                })?;
                let value = text.parse().map_err(|_| {
                    failure(
                        self.meter.source,
                        Code::InvalidModelBinding,
                        self.span(id),
                        "checked integer literal cannot be represented",
                    )
                })?;
                let graph = self.node(Kind::Integer(value, integer), id)?;
                self.expression_value(graph, id, true, Outcomes::unknown())
            }
            ExprKind::Text(_) | ExprKind::EnumValue { .. } => {
                self.symbolic(Key::Expression(id.0), id, ty, true)?
            }
            ExprKind::SelfValue => {
                let NativeType::Object { model, role } = ty else {
                    return Err(failure(
                        self.meter.source,
                        Code::InvalidModelBinding,
                        self.span(id),
                        "self has no native object type",
                    ));
                };
                self.symbolic(
                    Key::Context(
                        model.environment().owner(),
                        &role.record,
                        self.observed(id)?,
                    ),
                    id,
                    ty,
                    true,
                )?
            }
            ExprKind::ResultValue => self.declaration(id)?,
            ExprKind::Name(name) => {
                if let Some((_, value)) = self
                    .locals
                    .iter()
                    .rev()
                    .find(|(local, _)| *local == name.value)
                {
                    value.clone()
                } else {
                    self.declaration(id)?
                }
            }
            ExprKind::Field { base, .. } => {
                let base = self.visit(*base, path)?;
                let (catalog, location) = self.catalog(id)?;
                let DeclarationKey::Field { record, field } = &location.identity.key else {
                    return Err(failure(
                        self.meter.source,
                        Code::InvalidModelBinding,
                        self.span(id),
                        "field proof has no field identity",
                    ));
                };
                let declaration = catalog.records.get(record).copied().ok_or_else(|| {
                    failure(
                        self.meter.source,
                        Code::InvalidModelBinding,
                        self.span(id),
                        "field receiver declaration is absent",
                    )
                })?;
                let field = declaration
                    .fields()
                    .iter()
                    .find(|value| value.name() == field)
                    .ok_or_else(|| {
                        failure(
                            self.meter.source,
                            Code::InvalidModelBinding,
                            self.span(id),
                            "field declaration is absent",
                        )
                    })?;
                let key = Key::Path(
                    base.key,
                    Step::Field(
                        catalog.model.environment().owner(),
                        declaration.name(),
                        field.name(),
                    ),
                );
                self.symbolic(key, id, ty, base.stable)?
            }
            ExprKind::Unary { op, argument } => {
                let argument = self.visit(*argument, path)?;
                match op {
                    UnaryOp::Not => {
                        let graph = self.node(Kind::Not(argument.graph), id)?;
                        self.meter.facts(
                            argument.outcomes.yes.as_ref().map_or(0, BTreeMap::len)
                                + argument.outcomes.no.as_ref().map_or(0, BTreeMap::len),
                            self.span(id),
                        )?;
                        let mut outcomes = Outcomes {
                            yes: argument.outcomes.no.clone(),
                            no: argument.outcomes.yes.clone(),
                        };
                        let span = self.span(id);
                        outcomes.rebase(id, &mut self.meter, span)?;
                        self.expression_value(graph, id, false, outcomes)
                    }
                    UnaryOp::Negate => {
                        let graph = self.node(Kind::Negate(argument.graph), id)?;
                        self.goal(id, graph, path)?;
                        self.expression_value(graph, id, false, Outcomes::unknown())
                    }
                }
            }
            ExprKind::Binary { op, left, right } => {
                let left_value = self.visit(*left, path)?;
                let right_path = match op {
                    BinaryOp::And | BinaryOp::Implies => {
                        self.assume(path, &left_value, true, *left)?
                    }
                    BinaryOp::Or => self.assume(path, &left_value, false, *left)?,
                    _ => path.clone(),
                };
                let right_value = self.visit(*right, &right_path)?;
                match op {
                    BinaryOp::And | BinaryOp::Or | BinaryOp::Implies => {
                        let operator = match op {
                            BinaryOp::And => ir::BooleanOperator::ShortCircuitAnd,
                            BinaryOp::Or => ir::BooleanOperator::ShortCircuitOr,
                            _ => ir::BooleanOperator::Implication,
                        };
                        let graph = self.node(
                            Kind::BooleanOp(operator, left_value.graph, right_value.graph),
                            id,
                        )?;
                        let outcomes = self.boolean_outcomes(
                            *op,
                            &left_value.outcomes,
                            &right_value.outcomes,
                            id,
                        )?;
                        self.expression_value(graph, id, false, outcomes)
                    }
                    BinaryOp::Add
                    | BinaryOp::Subtract
                    | BinaryOp::Multiply
                    | BinaryOp::Divide
                    | BinaryOp::Remainder => {
                        let operator = match op {
                            BinaryOp::Add => ir::NumericOperator::Add,
                            BinaryOp::Subtract => ir::NumericOperator::Subtract,
                            BinaryOp::Multiply => ir::NumericOperator::Multiply,
                            BinaryOp::Divide => ir::NumericOperator::Divide,
                            _ => ir::NumericOperator::Remainder,
                        };
                        let graph = self.node(
                            Kind::Numeric(operator, left_value.graph, right_value.graph),
                            id,
                        )?;
                        self.goal(id, graph, path)?;
                        self.expression_value(graph, id, false, Outcomes::unknown())
                    }
                    BinaryOp::Equal
                    | BinaryOp::NotEqual
                    | BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual => {
                        if self.ty(*left)?.ty.integer().is_some() {
                            let operator = match op {
                                BinaryOp::Equal => ir::ComparisonOperator::Equal,
                                BinaryOp::NotEqual => ir::ComparisonOperator::NotEqual,
                                BinaryOp::Less => ir::ComparisonOperator::Less,
                                BinaryOp::LessEqual => ir::ComparisonOperator::LessEqual,
                                BinaryOp::Greater => ir::ComparisonOperator::Greater,
                                _ => ir::ComparisonOperator::GreaterEqual,
                            };
                            let graph = self.node(
                                Kind::Compare(operator, left_value.graph, right_value.graph),
                                id,
                            )?;
                            self.expression_value(graph, id, false, Outcomes::unknown())
                        } else {
                            self.symbolic(Key::Expression(id.0), id, &NativeType::Boolean, false)?
                        }
                    }
                }
            }
            ExprKind::Call { builtin, argument } => {
                let argument = self.visit(*argument, path)?;
                match builtin {
                    Builtin::Pre => argument,
                    Builtin::Present => {
                        self.meter.facts(2, self.span(id))?;
                        let graph = self.node(Kind::Present(argument.graph), id)?;
                        self.expression_value(
                            graph,
                            id,
                            false,
                            Outcomes::presence(argument.key, id),
                        )
                    }
                    Builtin::Value => {
                        let key = self.key(Key::Path(argument.key, Step::Unwrap), id);
                        let graph = self.node(Kind::Unwrap(argument.graph), id)?;
                        self.key_info[key.0].graph.get_or_insert(graph);
                        self.goal(id, graph, path)?;
                        Value {
                            key,
                            graph,
                            stable: argument.stable,
                            outcomes: Rc::new(Outcomes::unknown()),
                        }
                    }
                    Builtin::Deref => self.symbolic(
                        Key::Path(argument.key, Step::Deref),
                        id,
                        ty,
                        argument.stable,
                    )?,
                    Builtin::Size => self.symbolic(Key::Expression(id.0), id, ty, false)?,
                }
            }
            ExprKind::Let { name, value, body } => {
                let mut captured = self.visit(*value, path)?;
                let initializer = &self.ty(*value)?.ty;
                if !captured.stable && initializer != &NativeType::Boolean {
                    captured =
                        self.symbolic(Key::Local(name.span.start), *value, initializer, true)?;
                }
                self.locals.push((&name.value, captured));
                let result = self.visit(*body, path)?;
                self.locals.pop();
                result
            }
            ExprKind::If {
                condition,
                then_value,
                else_value,
            } => {
                let condition_value = self.visit(*condition, path)?;
                let then_path = self.assume(path, &condition_value, true, *condition)?;
                let then_result = self.visit(*then_value, &then_path)?;
                let else_path = self.assume(path, &condition_value, false, *condition)?;
                let else_result = self.visit(*else_value, &else_path)?;
                if ty == &NativeType::Boolean {
                    let not_condition = self.node(Kind::Not(condition_value.graph), *condition)?;
                    let then_graph = self.node(
                        Kind::BooleanOp(
                            ir::BooleanOperator::ShortCircuitAnd,
                            condition_value.graph,
                            then_result.graph,
                        ),
                        id,
                    )?;
                    let else_graph = self.node(
                        Kind::BooleanOp(
                            ir::BooleanOperator::ShortCircuitAnd,
                            not_condition,
                            else_result.graph,
                        ),
                        id,
                    )?;
                    let graph = self.node(
                        Kind::BooleanOp(
                            ir::BooleanOperator::ShortCircuitOr,
                            then_graph,
                            else_graph,
                        ),
                        id,
                    )?;
                    let span = self.span(id);
                    let then_yes = facts::sequential(
                        &condition_value.outcomes.yes,
                        &then_result.outcomes.yes,
                        &mut self.meter,
                        span,
                    )?;
                    let else_yes = facts::sequential(
                        &condition_value.outcomes.no,
                        &else_result.outcomes.yes,
                        &mut self.meter,
                        span,
                    )?;
                    let yes = facts::alternative(&then_yes, &else_yes, &mut self.meter, span)?;
                    let then_no = facts::sequential(
                        &condition_value.outcomes.yes,
                        &then_result.outcomes.no,
                        &mut self.meter,
                        span,
                    )?;
                    let else_no = facts::sequential(
                        &condition_value.outcomes.no,
                        &else_result.outcomes.no,
                        &mut self.meter,
                        span,
                    )?;
                    let no = facts::alternative(&then_no, &else_no, &mut self.meter, span)?;
                    let mut outcomes = Outcomes { yes, no };
                    outcomes.rebase(id, &mut self.meter, span)?;
                    self.expression_value(graph, id, false, outcomes)
                } else {
                    self.symbolic(Key::Expression(id.0), id, ty, false)?
                }
            }
            ExprKind::Quantifier {
                name,
                domain,
                predicate,
                ..
            } => {
                self.visit(*domain, path)?;
                let NativeType::Sequence { element, .. } = &self.ty(*domain)?.ty else {
                    return Err(failure(
                        self.meter.source,
                        Code::InvalidModelBinding,
                        self.span(id),
                        "checked quantifier has no sequence domain",
                    ));
                };
                let element = self.symbolic(Key::Local(name.span.start), *domain, element, true)?;
                self.locals.push((&name.value, element));
                self.visit(*predicate, path)?;
                self.locals.pop();
                self.symbolic(Key::Expression(id.0), id, &NativeType::Boolean, false)?
            }
            ExprKind::Reaches { start, target, .. } => {
                self.visit(*start, path)?;
                self.visit(*target, path)?;
                self.symbolic(Key::Expression(id.0), id, &NativeType::Boolean, false)?
            }
        };
        if ty == &NativeType::Boolean {
            value.graph = self.protect(
                value.graph,
                &path.facts,
                id,
                ir::BooleanOperator::ShortCircuitAnd,
            )?;
        }
        Ok(value)
    }
}
