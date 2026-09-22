// SPDX-License-Identifier: AGPL-3.0-or-later
//! Contextual constraints and closed native operator/profile permissions.
use super::*;

impl<'a> Solver<'_, 'a, '_, '_> {
    pub(super) fn expression(&mut self, at: ExprId) -> Result<()> {
        self.work.charge(D::Expressions, 1, self.site(at))?;
        let out = self.var(at);
        let mut children = Vec::new();
        match &self.unit.expressions()[at.0].kind {
            c::ValueKind::Shared(kind) => match kind {
                ExprKind::Boolean(_) => self.boolean(at)?,
                ExprKind::Integer(_) | ExprKind::Text(_) => {}
                ExprKind::Name(_) | ExprKind::SelfValue | ExprKind::ResultValue => {
                    let target = self.occurrence(at)?.map(|value| value.target);
                    if let Some(target) = target {
                        self.unify(out, self.binder_var(target.index()), at)?;
                    } else {
                        self.cause(at, CauseKind::UpstreamBinding)?;
                    }
                }
                ExprKind::EnumValue { variant, .. } => {
                    let mut selected = None;
                    for occurrence in
                        &self.binding.exports()[self.output.declaration.index()].occurrences
                    {
                        self.work.charge(D::Constraints, 1, self.site(at))?;
                        if occurrence.span.end == variant.span.end {
                            if let ModelTarget::Type(bound) = &occurrence.target {
                                selected = Some(bound.native());
                                break;
                            }
                        }
                    }
                    if let Some(ty) = selected {
                        charge_type(ty, self.work, self.site(at), 1)?;
                        self.assign(out, ty.clone(), at)?;
                    } else {
                        self.cause(at, CauseKind::UpstreamBinding)?;
                    }
                }
                ExprKind::Group { inner } => {
                    children.push(*inner);
                    self.unify(out, self.var(*inner), at)?;
                }
                ExprKind::Field { base, .. } => {
                    children.push(*base);
                    self.relation(
                        Relation::Field {
                            input: self.var(*base),
                            output: out,
                        },
                        at,
                    )?;
                }
                ExprKind::Unary { op, argument } => {
                    children.push(*argument);
                    match op {
                        UnaryOp::Not => {
                            self.boolean(at)?;
                            self.boolean(*argument)?;
                        }
                        UnaryOp::Negate => {
                            self.unify(out, self.var(*argument), at)?;
                            self.obligation(at, ObligationKind::ArithmeticRange)?;
                        }
                    }
                }
                ExprKind::Binary { op, left, right } => {
                    children.extend([*left, *right]);
                    match op {
                        BinaryOp::Implies | BinaryOp::Or | BinaryOp::And => {
                            self.boolean(at)?;
                            self.boolean(*left)?;
                            self.boolean(*right)?;
                        }
                        BinaryOp::Equal
                        | BinaryOp::NotEqual
                        | BinaryOp::Less
                        | BinaryOp::LessEqual
                        | BinaryOp::Greater
                        | BinaryOp::GreaterEqual => {
                            self.boolean(at)?;
                            self.unify(self.var(*left), self.var(*right), at)?;
                        }
                        BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply => {
                            self.unify(out, self.var(*left), at)?;
                            self.unify(out, self.var(*right), at)?;
                            self.obligation(at, ObligationKind::ArithmeticRange)?;
                        }
                        BinaryOp::Divide | BinaryOp::Remainder => {
                            self.cause(at, CauseKind::ForbiddenOperator)?;
                            self.unify(out, self.var(*left), at)?;
                            self.unify(out, self.var(*right), at)?;
                        }
                    }
                }
                ExprKind::Call { builtin, argument } => {
                    children.push(*argument);
                    match builtin {
                        Builtin::Present => {
                            self.boolean(at)?;
                            self.relation(
                                Relation::Option {
                                    input: self.var(*argument),
                                    output: None,
                                },
                                at,
                            )?;
                        }
                        Builtin::Value => {
                            self.relation(
                                Relation::Option {
                                    input: self.var(*argument),
                                    output: Some(out),
                                },
                                at,
                            )?;
                            self.obligation(at, ObligationKind::Presence)?;
                        }
                        Builtin::Deref => {
                            self.require(at, Capability::Graph)?;
                            self.relation(
                                Relation::Deref {
                                    input: self.var(*argument),
                                    output: out,
                                },
                                at,
                            )?;
                            self.obligation(at, ObligationKind::GraphClosure)?;
                        }
                        Builtin::Size => {}
                        Builtin::Pre => self.unify(out, self.var(*argument), at)?,
                    }
                }
                ExprKind::Let { value, body, .. } => {
                    children.extend([*value, *body]);
                    self.unify(out, self.var(*body), at)?;
                }
                ExprKind::If {
                    condition,
                    then_value,
                    else_value,
                } => {
                    children.extend([*condition, *then_value, *else_value]);
                    self.boolean(*condition)?;
                    self.unify(out, self.var(*then_value), at)?;
                    self.unify(out, self.var(*else_value), at)?;
                }
                ExprKind::Quantifier {
                    domain, predicate, ..
                } => {
                    children.extend([*domain, *predicate]);
                    self.boolean(at)?;
                    self.boolean(*predicate)?;
                }
                ExprKind::Reaches { start, target, .. } => {
                    children.extend([*start, *target]);
                    self.require(at, Capability::Graph)?;
                    self.boolean(at)?;
                    self.unify(self.var(*start), self.var(*target), at)?;
                    self.obligation(at, ObligationKind::GraphClosure)?;
                }
            },
            c::ValueKind::Invoke { arguments, .. } => {
                self.require(at, Capability::Queries)?;
                self.boolean(at)?;
                children.extend(arguments.iter().copied());
                let entry = self
                    .binding
                    .namespace()
                    .declaration(self.output.declaration)
                    .expect("bound declaration");
                let mut selected = None;
                for reference in entry.references() {
                    self.work.charge(D::Edges, 1, self.site(at))?;
                    if reference.kind == DependencyKind::PredicateCall
                        && reference.site == DependencySite::Expression(at)
                    {
                        selected = reference.target;
                        break;
                    }
                }
                if let Some(target) = selected {
                    let c::DeclarationKind::Predicate { parameters, .. } = &self
                        .binding
                        .namespace()
                        .syntax(target)
                        .expect("bound callee")
                        .kind
                    else {
                        self.cause(at, CauseKind::UpstreamBinding)?;
                        return Ok(());
                    };
                    if parameters.len() != arguments.len() {
                        self.cause(
                            at,
                            CauseKind::CallArity {
                                expected: parameters.len(),
                                actual: arguments.len(),
                            },
                        )?;
                    }
                    for (parameter, argument) in parameters.iter().zip(arguments) {
                        self.work.charge(D::Constraints, 1, self.site(at))?;
                        let ty = match &parameter.ty {
                            c::ParameterType::Boolean(_) => Some(NativeType::Boolean),
                            c::ParameterType::Model(name) => {
                                let span = Span {
                                    start: name.model.span.start,
                                    end: name.name.span.end,
                                };
                                let mut found = None;
                                if let Some(exports) = self.binding.exports().get(target.index()) {
                                    for occurrence in &exports.occurrences {
                                        self.work.charge(D::Constraints, 1, self.site(at))?;
                                        if occurrence.span == span {
                                            if let ModelTarget::Type(bound) = &occurrence.target {
                                                charge_type(
                                                    bound.native(),
                                                    self.work,
                                                    self.site(at),
                                                    1,
                                                )?;
                                                found = Some(bound.native().clone());
                                                break;
                                            }
                                        }
                                    }
                                }
                                found
                            }
                        };
                        if let Some(ty) = ty {
                            self.assign(self.var(*argument), ty, at)?;
                        }
                    }
                    self.obligation(at, ObligationKind::PredicateTotal { target })?;
                } else {
                    self.cause(at, CauseKind::UpstreamBinding)?;
                }
            }
            c::ValueKind::Rational { .. } => {}
            c::ValueKind::Product { op, left, right } => {
                children.extend([*left, *right]);
                self.unify(out, self.var(*left), at)?;
                self.unify(out, self.var(*right), at)?;
                match op.value {
                    c::ProductOp::Slash => {
                        self.obligation(at, ObligationKind::Nonzero)?;
                        self.obligation(at, ObligationKind::ArithmeticRange)?;
                    }
                    c::ProductOp::Mod => self.cause(at, CauseKind::ForbiddenOperator)?,
                }
            }
            c::ValueKind::Size { domain, argument } => {
                children.push(*argument);
                if let Some(ty) = self.qualified(domain, at)? {
                    self.assign(out, ty, at)?;
                }
            }
            c::ValueKind::Contains { collection, member } => {
                children.extend([*collection, *member]);
                self.boolean(at)?;
                self.relation(
                    Relation::Sequence {
                        input: self.var(*collection),
                        element: self.var(*member),
                    },
                    at,
                )?;
            }
            c::ValueKind::Query {
                op,
                result,
                domain,
                body,
                ..
            } => {
                children.extend([*domain, *body]);
                self.require(at, Capability::Queries)?;
                if let Some(name) = result {
                    if let Some(ty) = self.qualified(name, at)? {
                        self.assign(out, ty, at)?;
                    }
                }
                match op.value {
                    c::QueryOp::Filter => {
                        self.boolean(*body)?;
                        self.unify(out, self.var(*domain), at)?;
                    }
                    c::QueryOp::Map => self.relation(
                        Relation::Map {
                            input: self.var(*domain),
                            body: self.var(*body),
                            output: out,
                        },
                        at,
                    )?,
                    c::QueryOp::Count => self.boolean(*body)?,
                    c::QueryOp::Sum => {
                        self.obligation(at, ObligationKind::SumProjection)?;
                        self.obligation(at, ObligationKind::SumPrefixes)?;
                    }
                }
            }
        }
        let depth = children
            .iter()
            .map(|id| self.depths[self.var(*id)])
            .max()
            .unwrap_or(0)
            + 1;
        self.work.charge(D::Depth, depth, self.site(at))?;
        self.depths[out] = depth;
        Ok(())
    }

    pub(super) fn solve_relations(&mut self) -> Result<()> {
        // Every revisit is charged. Only new type information requests another pass.
        let mut changed = true;
        let mut settled = vec![false; self.relations.len()];
        while changed {
            changed = false;
            for (index, done) in settled.iter_mut().enumerate() {
                if *done {
                    continue;
                }
                let Pending { relation, at } = self.relations[index];
                self.work.charge(D::Constraints, 1, self.site(at))?;
                let (input, output) = match relation {
                    Relation::Option { input, output } => (input, output),
                    Relation::Sequence { input, element } => (input, Some(element)),
                    Relation::Field { input, output }
                    | Relation::Deref { input, output }
                    | Relation::Map { input, output, .. } => (input, Some(output)),
                };
                let Some(ty) = self.get(input, at)? else {
                    continue;
                };
                let result = match relation {
                    Relation::Option { .. } => match ty {
                        NativeType::Option(value) => Some(*value),
                        _ => {
                            self.cause(at, CauseKind::ExpectedOption)?;
                            *done = true;
                            None
                        }
                    },
                    Relation::Sequence { .. } => match ty {
                        NativeType::Sequence { element, .. } => Some(*element),
                        _ => {
                            self.cause(at, CauseKind::ExpectedSequence)?;
                            *done = true;
                            None
                        }
                    },
                    Relation::Deref { .. } => match ty {
                        NativeType::Reference { model, role } => {
                            Some(NativeType::Object { model, role })
                        }
                        _ => {
                            self.cause(at, CauseKind::ExpectedReference)?;
                            *done = true;
                            None
                        }
                    },
                    Relation::Field { .. } => {
                        let c::ValueKind::Shared(ExprKind::Field { name, .. }) =
                            &self.unit.expressions()[at.0].kind
                        else {
                            unreachable!("field relation")
                        };
                        self.field_inner(&ty, name, at)?
                    }
                    Relation::Map { body, .. } => match ty {
                        NativeType::Sequence { maximum, .. } => {
                            if let Some(element) = self.get(body, at)? {
                                Some(NativeType::Sequence {
                                    element: Box::new(element),
                                    maximum,
                                })
                            } else if let Some(NativeType::Sequence {
                                element,
                                maximum: expected,
                            }) = self.get(output.expect("map output"), at)?
                            {
                                if expected != maximum {
                                    self.cause(at, CauseKind::TypeMismatch)?;
                                    *done = true;
                                }
                                changed |= self.assign(body, *element, at)?;
                                None
                            } else {
                                None
                            }
                        }
                        _ => {
                            self.cause(at, CauseKind::ExpectedSequence)?;
                            *done = true;
                            None
                        }
                    },
                };
                if let (Some(output), Some(result)) = (output, result) {
                    changed |= self.assign(output, result, at)?;
                }
            }
        }
        Ok(())
    }
}

impl<'a> Solver<'_, 'a, '_, '_> {
    pub(super) fn field_inner(
        &mut self,
        receiver: &NativeType<'a>,
        name: &qsl_foundation::Spanned<String>,
        at: ExprId,
    ) -> Result<Option<NativeType<'a>>> {
        let (model, record) = match receiver {
            NativeType::Record { model, declaration } => (*model, declaration.name()),
            NativeType::Object { model, role } => (*model, &role.record),
            NativeType::Boolean
            | NativeType::Scalar { .. }
            | NativeType::Enumeration { .. }
            | NativeType::Reference { .. }
            | NativeType::Option(_)
            | NativeType::Sequence { .. } => {
                self.cause(at, CauseKind::InvalidField)?;
                return Ok(None);
            }
        };
        let Some(catalog) = self.catalog(model, at)? else {
            return Ok(None);
        };
        let Some(record_declaration) = catalog.records.get(record) else {
            self.cause(at, CauseKind::UpstreamBinding)?;
            return Ok(None);
        };
        for field in record_declaration.fields() {
            self.work.charge(D::Constraints, 1, self.site(at))?;
            self.work.charge(
                D::Bytes,
                name.value.len() + field.name().as_str().len(),
                self.site(at),
            )?;
            if field.name().as_str() == name.value {
                return self.formal_type(
                    catalog,
                    field.value_type(),
                    ScalarSite::Field {
                        record: record.clone(),
                        field: field.name().clone(),
                    },
                    at,
                );
            }
        }
        self.cause(at, CauseKind::InvalidField)?;
        Ok(None)
    }
}
