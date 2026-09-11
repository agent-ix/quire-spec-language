// SPDX-License-Identifier: AGPL-3.0-only
//! Original composed values, preserving evaluation order and immutable aliases.

use super::*;

impl<'s, 'a> Builder<'s, 'a> {
    fn outcomes(
        &mut self,
        op: BinaryOp,
        left: &Outcomes,
        right: &Outcomes,
        at: ExprId,
    ) -> Result<Outcomes> {
        let span = self.site(at).span;
        let (yes, no) = match op {
            BinaryOp::And => {
                let yes = facts::sequential(&left.yes, &right.yes, self, span)?;
                let tail = facts::sequential(&left.yes, &right.no, self, span)?;
                (yes, facts::alternative(&left.no, &tail, self, span)?)
            }
            BinaryOp::Or => {
                let tail = facts::sequential(&left.no, &right.yes, self, span)?;
                let yes = facts::alternative(&left.yes, &tail, self, span)?;
                (yes, facts::sequential(&left.no, &right.no, self, span)?)
            }
            BinaryOp::Implies => {
                let tail = facts::sequential(&left.yes, &right.yes, self, span)?;
                let yes = facts::alternative(&left.no, &tail, self, span)?;
                (yes, facts::sequential(&left.yes, &right.no, self, span)?)
            }
            BinaryOp::Add
            | BinaryOp::Subtract
            | BinaryOp::Multiply
            | BinaryOp::Divide
            | BinaryOp::Remainder
            | BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::LessEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterEqual => unreachable!("Boolean outcome operator"),
        };
        let mut result = Outcomes { yes, no };
        result.rebase(at, self, span)?;
        Ok(result)
    }
    fn read(&mut self, at: ExprId) -> Result<Value> {
        let node = self.typed.node(at).ok_or_else(|| upstream(self.site(at)))?;
        let binder = node.binder.ok_or_else(|| upstream(self.site(at)))?;
        self.work.charge(D::Types, 1, self.site(at))?;
        if let Some(value) = self.locals.get(&binder.index()) {
            return Ok(value.clone());
        }
        let observation = if self
            .scope
            .binders
            .get(binder.index())
            .ok_or_else(|| upstream(self.site(at)))?
            .kind
            == scopes::BinderKind::SelfValue
        {
            match node.origin {
                Some(ObservationOrigin::Anchored(Anchor::InvocationPre)) => 1,
                Some(ObservationOrigin::Anchored(Anchor::InvocationPost)) => 2,
                Some(ObservationOrigin::Anchored(Anchor::Current)) => 0,
                Some(ObservationOrigin::Independent | ObservationOrigin::Selected { .. })
                | Some(ObservationOrigin::Anchored(_))
                | None => return Err(self.unsupported(at, Unsupported::ValueRepresentation)),
            }
        } else {
            0
        };
        self.symbolic(
            Key::Binder(binder.index(), observation),
            at,
            self.ty(at)?,
            true,
        )
    }
    pub(super) fn capture(
        &mut self,
        binder: usize,
        at: ExprId,
        path: &Path,
        depth: usize,
    ) -> Result<Value> {
        let mut value = self.visit(at, path, depth)?;
        if !value.stable && self.ty(at)? != &NativeType::Boolean {
            value = self.symbolic(Key::Capture(binder), at, self.ty(at)?, true)?;
        }
        self.work.charge(D::Records, 1, self.site(at))?;
        self.locals.insert(binder, value.clone());
        Ok(value)
    }
    pub(super) fn visit(&mut self, at: ExprId, path: &Path, depth: usize) -> Result<Value> {
        self.work.charge(D::Depth, depth, self.site(at))?;
        self.work.charge(D::Expressions, 1, self.site(at))?;
        if path.facts.is_none() {
            return self.symbolic(Key::Expression(at.0), at, self.ty(at)?, false);
        }
        let mut value = match &self
            .unit
            .expression(at)
            .ok_or_else(|| upstream(self.site(at)))?
            .kind
        {
            c::ValueKind::Shared(kind) => match kind {
                ExprKind::Group { inner } => self.visit(*inner, path, depth + 1)?,
                ExprKind::Boolean(value) => {
                    let graph = self.node(Kind::Boolean(*value), at)?;
                    self.expression(graph, at, true, Outcomes::constant(*value))?
                }
                ExprKind::Integer(text) => {
                    self.work.charge(D::Bytes, text.len(), self.site(at))?;
                    let value = text
                        .parse::<i64>()
                        .map_err(|_| self.unsupported(at, Unsupported::ValueRepresentation))?;
                    let ty = self
                        .ty(at)?
                        .integer()
                        .ok_or_else(|| upstream(self.site(at)))?;
                    let graph = self.node(Kind::Integer(value, ty), at)?;
                    self.expression(graph, at, true, Outcomes::unknown())?
                }
                ExprKind::Text(_) | ExprKind::EnumValue { .. } => {
                    self.symbolic(Key::Expression(at.0), at, self.ty(at)?, true)?
                }
                ExprKind::Name(_) | ExprKind::SelfValue | ExprKind::ResultValue => self.read(at)?,
                ExprKind::Field { base, name: field } => {
                    let value = self.visit(*base, path, depth + 1)?;
                    let (model, record) = match self.ty(*base)? {
                        NativeType::Record { model, declaration } => {
                            (*model, declaration.name().as_str())
                        }
                        NativeType::Object { model, role } => (*model, role.record.as_str()),
                        NativeType::Boolean
                        | NativeType::Scalar { .. }
                        | NativeType::Enumeration { .. }
                        | NativeType::Reference { .. }
                        | NativeType::Option(_)
                        | NativeType::Sequence { .. } => {
                            return Err(self.unsupported(at, Unsupported::ValueRepresentation))
                        }
                    };
                    self.work.charge(
                        D::Bytes,
                        field.value.len()
                            + record.len()
                            + model.environment().owner().package().as_str().len()
                            + model.environment().owner().requirement().as_str().len(),
                        self.site(at),
                    )?;
                    self.symbolic(
                        Key::Field(
                            value.key,
                            model.environment().owner(),
                            record,
                            field.value.clone(),
                        ),
                        at,
                        self.ty(at)?,
                        value.stable,
                    )?
                }
                ExprKind::Unary { op, argument } => {
                    // The authored signed minimum is one checked literal, not a
                    // separately evaluated out-of-domain positive intermediate.
                    if *op == UnaryOp::Negate {
                        if let c::ValueKind::Shared(ExprKind::Integer(text)) =
                            &self.unit.expressions()[argument.0].kind
                        {
                            self.work.charge(D::Bytes, text.len(), self.site(at))?;
                            if let Ok(raw) = text.parse::<i128>() {
                                if let Some(value) = raw
                                    .checked_neg()
                                    .and_then(|value| i64::try_from(value).ok())
                                {
                                    self.work.charge(D::Expressions, 1, self.site(*argument))?;
                                    let graph = self.node(
                                        Kind::Integer(
                                            value,
                                            self.ty(at)?
                                                .integer()
                                                .ok_or_else(|| upstream(self.site(at)))?,
                                        ),
                                        at,
                                    )?;
                                    return self.expression(graph, at, true, Outcomes::unknown());
                                }
                            }
                        }
                    }
                    let value = self.visit(*argument, path, depth + 1)?;
                    match op {
                        UnaryOp::Not => {
                            let graph = self.node(Kind::Not(value.graph), at)?;
                            self.work.charge(
                                D::Facts,
                                value.outcomes.yes.as_ref().map_or(0, BTreeMap::len)
                                    + value.outcomes.no.as_ref().map_or(0, BTreeMap::len),
                                self.site(at),
                            )?;
                            let mut outcomes = Outcomes {
                                yes: value.outcomes.no.clone(),
                                no: value.outcomes.yes.clone(),
                            };
                            outcomes.rebase(at, self, self.site(at).span)?;
                            self.expression(graph, at, false, outcomes)?
                        }
                        UnaryOp::Negate => {
                            let graph = self.node(Kind::Negate(value.graph), at)?;
                            self.goal(at, graph, path)?;
                            self.expression(graph, at, false, Outcomes::unknown())?
                        }
                    }
                }
                ExprKind::Binary { op, left, right } => {
                    let left_value = self.visit(*left, path, depth + 1)?;
                    let right_path = match op {
                        BinaryOp::And | BinaryOp::Implies => {
                            Some(self.assume(path, &left_value, true, *left)?)
                        }
                        BinaryOp::Or => Some(self.assume(path, &left_value, false, *left)?),
                        BinaryOp::Add
                        | BinaryOp::Subtract
                        | BinaryOp::Multiply
                        | BinaryOp::Divide
                        | BinaryOp::Remainder
                        | BinaryOp::Equal
                        | BinaryOp::NotEqual
                        | BinaryOp::Less
                        | BinaryOp::LessEqual
                        | BinaryOp::Greater
                        | BinaryOp::GreaterEqual => None,
                    };
                    let right_value =
                        self.visit(*right, right_path.as_ref().unwrap_or(path), depth + 1)?;
                    match op {
                        BinaryOp::And | BinaryOp::Or | BinaryOp::Implies => {
                            let operator = match op {
                                BinaryOp::And => ir::BooleanOperator::ShortCircuitAnd,
                                BinaryOp::Or => ir::BooleanOperator::ShortCircuitOr,
                                BinaryOp::Implies => ir::BooleanOperator::Implication,
                                BinaryOp::Add
                                | BinaryOp::Subtract
                                | BinaryOp::Multiply
                                | BinaryOp::Divide
                                | BinaryOp::Remainder
                                | BinaryOp::Equal
                                | BinaryOp::NotEqual
                                | BinaryOp::Less
                                | BinaryOp::LessEqual
                                | BinaryOp::Greater
                                | BinaryOp::GreaterEqual => unreachable!("Boolean operator"),
                            };
                            let graph = self.node(
                                Kind::BooleanOp(operator, left_value.graph, right_value.graph),
                                at,
                            )?;
                            let outcomes = self.outcomes(
                                *op,
                                &left_value.outcomes,
                                &right_value.outcomes,
                                at,
                            )?;
                            self.expression(graph, at, false, outcomes)?
                        }
                        BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply => {
                            let operator = match op {
                                BinaryOp::Add => ir::NumericOperator::Add,
                                BinaryOp::Subtract => ir::NumericOperator::Subtract,
                                BinaryOp::Multiply => ir::NumericOperator::Multiply,
                                BinaryOp::And
                                | BinaryOp::Or
                                | BinaryOp::Implies
                                | BinaryOp::Divide
                                | BinaryOp::Remainder
                                | BinaryOp::Equal
                                | BinaryOp::NotEqual
                                | BinaryOp::Less
                                | BinaryOp::LessEqual
                                | BinaryOp::Greater
                                | BinaryOp::GreaterEqual => unreachable!("typed numeric operator"),
                            };
                            let graph = self.node(
                                Kind::Numeric(operator, left_value.graph, right_value.graph),
                                at,
                            )?;
                            self.goal(at, graph, path)?;
                            self.expression(graph, at, false, Outcomes::unknown())?
                        }
                        BinaryOp::Divide | BinaryOp::Remainder => {
                            return Err(self.unsupported(at, Unsupported::ValueRepresentation))
                        }
                        BinaryOp::Equal
                        | BinaryOp::NotEqual
                        | BinaryOp::Less
                        | BinaryOp::LessEqual
                        | BinaryOp::Greater
                        | BinaryOp::GreaterEqual => {
                            if self.ty(*left)?.integer().is_some()
                                || self.ty(*left)?.rational().is_some()
                                || self.ty(*left)? == &NativeType::Boolean
                            {
                                let operator = match op {
                                    BinaryOp::Equal => ir::ComparisonOperator::Equal,
                                    BinaryOp::NotEqual => ir::ComparisonOperator::NotEqual,
                                    BinaryOp::Less => ir::ComparisonOperator::Less,
                                    BinaryOp::LessEqual => ir::ComparisonOperator::LessEqual,
                                    BinaryOp::Greater => ir::ComparisonOperator::Greater,
                                    BinaryOp::GreaterEqual => ir::ComparisonOperator::GreaterEqual,
                                    BinaryOp::Add
                                    | BinaryOp::Subtract
                                    | BinaryOp::Multiply
                                    | BinaryOp::Divide
                                    | BinaryOp::Remainder
                                    | BinaryOp::And
                                    | BinaryOp::Or
                                    | BinaryOp::Implies => unreachable!("comparison"),
                                };
                                let graph = self.node(
                                    Kind::Compare(operator, left_value.graph, right_value.graph),
                                    at,
                                )?;
                                self.expression(graph, at, false, Outcomes::unknown())?
                            } else {
                                self.symbolic(
                                    Key::Expression(at.0),
                                    at,
                                    &NativeType::Boolean,
                                    false,
                                )?
                            }
                        }
                    }
                }
                ExprKind::Call { builtin, argument } => {
                    if *builtin == Builtin::Size {
                        return Err(self.unsupported(at, Unsupported::OrderedQuery));
                    }
                    let value = self.visit(*argument, path, depth + 1)?;
                    match builtin {
                        Builtin::Pre => value,
                        Builtin::Present => {
                            self.work.charge(D::Facts, 2, self.site(at))?;
                            self.work.charge(D::Records, 2, self.site(at))?;
                            let graph = self.node(Kind::Present(value.graph), at)?;
                            self.expression(graph, at, false, Outcomes::presence(value.key, at))?
                        }
                        Builtin::Value => {
                            let key = self.key(Key::Unwrap(value.key), at)?;
                            let graph = self.node(Kind::Unwrap(value.graph), at)?;
                            self.key_info[key.0].graph.get_or_insert(graph);
                            self.goal(at, graph, path)?;
                            Value {
                                key,
                                graph,
                                stable: value.stable,
                                outcomes: Rc::new(Outcomes::unknown()),
                            }
                        }
                        Builtin::Deref => {
                            self.symbolic(Key::Deref(value.key), at, self.ty(at)?, value.stable)?
                        }
                        Builtin::Size => unreachable!("unsupported query handled before its body"),
                    }
                }
                ExprKind::Let { name, value, body } => {
                    self.work.charge(D::Types, 1, self.site(at))?;
                    let binder = *self
                        .binders
                        .get(&name.span.start)
                        .ok_or_else(|| upstream(self.site(at)))?;
                    self.capture(binder, *value, path, depth + 1)?;
                    let result = self.visit(*body, path, depth + 1)?;
                    self.locals.remove(&binder);
                    result
                }
                ExprKind::If {
                    condition,
                    then_value,
                    else_value,
                } => {
                    let condition_value = self.visit(*condition, path, depth + 1)?;
                    let yes_path = self.assume(path, &condition_value, true, *condition)?;
                    let yes_value = self.visit(*then_value, &yes_path, depth + 1)?;
                    let no_path = self.assume(path, &condition_value, false, *condition)?;
                    let no_value = self.visit(*else_value, &no_path, depth + 1)?;
                    if self.ty(at)? == &NativeType::Boolean {
                        let negated = self.node(Kind::Not(condition_value.graph), *condition)?;
                        let yes = self.node(
                            Kind::BooleanOp(
                                ir::BooleanOperator::ShortCircuitAnd,
                                condition_value.graph,
                                yes_value.graph,
                            ),
                            at,
                        )?;
                        let no = self.node(
                            Kind::BooleanOp(
                                ir::BooleanOperator::ShortCircuitAnd,
                                negated,
                                no_value.graph,
                            ),
                            at,
                        )?;
                        let graph = self.node(
                            Kind::BooleanOp(ir::BooleanOperator::ShortCircuitOr, yes, no),
                            at,
                        )?;
                        let span = self.site(at).span;
                        let yes_yes = facts::sequential(
                            &condition_value.outcomes.yes,
                            &yes_value.outcomes.yes,
                            self,
                            span,
                        )?;
                        let no_yes = facts::sequential(
                            &condition_value.outcomes.no,
                            &no_value.outcomes.yes,
                            self,
                            span,
                        )?;
                        let yes = facts::alternative(&yes_yes, &no_yes, self, span)?;
                        let yes_no = facts::sequential(
                            &condition_value.outcomes.yes,
                            &yes_value.outcomes.no,
                            self,
                            span,
                        )?;
                        let no_no = facts::sequential(
                            &condition_value.outcomes.no,
                            &no_value.outcomes.no,
                            self,
                            span,
                        )?;
                        let no = facts::alternative(&yes_no, &no_no, self, span)?;
                        let mut outcomes = Outcomes { yes, no };
                        outcomes.rebase(at, self, span)?;
                        self.expression(graph, at, false, outcomes)?
                    } else {
                        self.symbolic(Key::Expression(at.0), at, self.ty(at)?, false)?
                    }
                }
                ExprKind::Quantifier { .. } => {
                    return Err(self.unsupported(at, Unsupported::OrderedQuery))
                }
                ExprKind::Reaches { start, target, .. } => {
                    self.visit(*start, path, depth + 1)?;
                    self.visit(*target, path, depth + 1)?;
                    self.symbolic(Key::Expression(at.0), at, &NativeType::Boolean, false)?
                }
            },
            c::ValueKind::Rational { .. } => {
                let (numerator, denominator) = self
                    .typed
                    .node(at)
                    .and_then(|node| node.normalized_rational)
                    .ok_or_else(|| upstream(self.site(at)))?;
                let graph = self.node(
                    Kind::Rational(
                        numerator,
                        denominator,
                        self.ty(at)?
                            .rational()
                            .ok_or_else(|| upstream(self.site(at)))?,
                    ),
                    at,
                )?;
                self.expression(graph, at, true, Outcomes::unknown())?
            }
            c::ValueKind::Product { op, left, right } => match op.value {
                c::ProductOp::Slash => {
                    let left = self.visit(*left, path, depth + 1)?;
                    let right = self.visit(*right, path, depth + 1)?;
                    let graph = self.node(
                        Kind::Numeric(ir::NumericOperator::Divide, left.graph, right.graph),
                        at,
                    )?;
                    self.goal(at, graph, path)?;
                    self.expression(graph, at, false, Outcomes::unknown())?
                }
                c::ProductOp::Mod => {
                    return Err(self.unsupported(at, Unsupported::ValueRepresentation))
                }
            },
            c::ValueKind::Invoke { arguments, .. } => {
                // Arguments are evaluated here; the separate native dependency
                // gate requires the callee's totality over its own full domains.
                for argument in arguments {
                    self.visit(*argument, path, depth + 1)?;
                }
                self.symbolic(Key::Expression(at.0), at, &NativeType::Boolean, false)?
            }
            c::ValueKind::Size { .. }
            | c::ValueKind::Contains { .. }
            | c::ValueKind::Query { .. } => {
                return Err(self.unsupported(at, Unsupported::OrderedQuery))
            }
        };
        if self.ty(at)? == &NativeType::Boolean {
            value.graph = self.protect(
                value.graph,
                &path.facts,
                at,
                ir::BooleanOperator::ShortCircuitAnd,
            )?;
        }
        Ok(value)
    }
}
