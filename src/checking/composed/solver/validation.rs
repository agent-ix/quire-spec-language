// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native admissibility checks separate from the pending definedness obligations.
use super::*;

impl<'a> Solver<'_, 'a, '_, '_> {
    pub(super) fn model_bounds(&mut self) -> Result<()> {
        let site = self.declaration_site();
        for &input in self.model_bounds.imports(self.output.unit) {
            self.work.charge(D::Constraints, 1, site)?;
            if self.model_bounds.invalid(input) {
                self.work.charge(D::Records, 1, site)?;
                self.output.causes.push(TypeCause {
                    site,
                    kind: CauseKind::ModelDomain { input },
                    profile: None,
                });
            }
        }
        Ok(())
    }
    pub(super) fn empty_binders(&mut self) -> Result<()> {
        let site = self.declaration_site();
        for (index, binder) in self.scope.binders.iter().enumerate() {
            let ty = match &binder.ty {
                scopes::BinderType::Declared(c::ParameterType::Boolean(_)) => {
                    Some(NativeType::Boolean)
                }
                scopes::BinderType::Declared(c::ParameterType::Model(name))
                | scopes::BinderType::Context(name) => {
                    let span = Span {
                        start: name.model.span.start,
                        end: name.name.span.end,
                    };
                    let mut found = None;
                    for occurrence in
                        &self.binding.exports()[self.output.declaration.index()].occurrences
                    {
                        self.work.charge(D::Constraints, 1, site)?;
                        if occurrence.span == span {
                            if let ModelTarget::Type(bound) = &occurrence.target {
                                charge_type(bound.native(), self.work, site, 1)?;
                                found = Some(bound.native().clone());
                                break;
                            }
                        }
                    }
                    found
                }
                scopes::BinderType::ModelValue(_)
                | scopes::BinderType::Initializer(_)
                | scopes::BinderType::ElementOf(_) => None,
            };
            self.work.charge(D::Records, 1, site)?;
            self.output.binders.push(BinderType {
                binder: index,
                ty,
                anchor: binder.anchor,
            });
        }
        Ok(())
    }
    pub(super) fn validate(&mut self, at: ExprId) -> Result<()> {
        self.work.charge(D::Constraints, 1, self.site(at))?;
        if let Some(ty) = self.get(self.var(at), at)? {
            self.wrappers(&ty, at)?;
        }
        match &self.unit.expressions()[at.0].kind {
            c::ValueKind::Shared(kind) => match kind {
                ExprKind::Unary {
                    op: UnaryOp::Negate,
                    ..
                } => self.numeric(at, false, false)?,
                ExprKind::Binary { op, left, .. } => match op {
                    BinaryOp::Equal | BinaryOp::NotEqual => {
                        if let Some(ty) = self.get(self.var(*left), at)? {
                            self.equality(&ty, at)?;
                        }
                    }
                    BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual => {
                        if let Some(ty) = self.get(self.var(*left), at)? {
                            if !matches!(ty, NativeType::Scalar { .. }) {
                                self.cause(at, CauseKind::ForbiddenOperator)?;
                            }
                        }
                    }
                    BinaryOp::Add | BinaryOp::Subtract => self.numeric(at, false, false)?,
                    BinaryOp::Multiply => self.numeric(at, true, false)?,
                    BinaryOp::Implies
                    | BinaryOp::Or
                    | BinaryOp::And
                    | BinaryOp::Divide
                    | BinaryOp::Remainder => {}
                },
                ExprKind::Call {
                    builtin: Builtin::Size,
                    argument,
                } => self.count_domain(*argument, at)?,
                ExprKind::Reaches { start, field, .. } => self.graph(*start, field, at)?,
                ExprKind::Quantifier { domain, .. } => {
                    self.sequence(*domain, at)?;
                }
                ExprKind::Group { .. }
                | ExprKind::Boolean(_)
                | ExprKind::Integer(_)
                | ExprKind::Text(_)
                | ExprKind::Name(_)
                | ExprKind::SelfValue
                | ExprKind::ResultValue
                | ExprKind::EnumValue { .. }
                | ExprKind::Field { .. }
                | ExprKind::Unary {
                    op: UnaryOp::Not, ..
                }
                | ExprKind::Call {
                    builtin: Builtin::Present | Builtin::Value | Builtin::Deref | Builtin::Pre,
                    ..
                }
                | ExprKind::Let { .. }
                | ExprKind::If { .. } => {}
            },
            c::ValueKind::Product { op, .. } => match op.value {
                c::ProductOp::Slash => self.numeric(at, true, true)?,
                c::ProductOp::Mod => {}
            },
            c::ValueKind::Size { argument, .. } => self.count_domain(*argument, at)?,
            c::ValueKind::Contains { member, .. } => {
                if let Some(ty) = self.get(self.var(*member), at)? {
                    self.equality(&ty, at)?;
                }
            }
            c::ValueKind::Query {
                op, domain, body, ..
            } => match op.value {
                c::QueryOp::Filter | c::QueryOp::Map => {
                    self.sequence(*domain, at)?;
                }
                c::QueryOp::Count => self.count_domain(*domain, at)?,
                c::QueryOp::Sum => {
                    self.sequence(*domain, at)?;
                    let projection = self.get(self.var(*body), at)?;
                    let total = self.get(self.var(at), at)?;
                    if let (Some(projection), Some(total)) = (projection, total) {
                        let compatible = match (
                            projection.integer(),
                            total.integer(),
                            projection.rational(),
                            total.rational(),
                        ) {
                            (Some(value), Some(result), None, None) => {
                                result.minimum() <= 0
                                    && result.maximum() >= 0
                                    && result.minimum() <= value.minimum()
                                    && result.maximum() >= value.maximum()
                            }
                            (None, None, Some(value), Some(result)) => {
                                result.numerator_minimum() <= 0
                                    && result.numerator_maximum() >= 0
                                    && result.numerator_minimum() <= value.numerator_minimum()
                                    && result.numerator_maximum() >= value.numerator_maximum()
                                    && result.maximum_denominator() >= value.maximum_denominator()
                            }
                            _ => false,
                        };
                        if !compatible || projection.numeric_unit() != total.numeric_unit() {
                            self.cause(at, CauseKind::InvalidAggregateDomain)?;
                        }
                    }
                }
            },
            c::ValueKind::Invoke { .. } | c::ValueKind::Rational { .. } => {}
        }
        Ok(())
    }
    fn wrappers(&mut self, ty: &NativeType<'_>, at: ExprId) -> Result<()> {
        match ty {
            NativeType::Sequence { element, maximum } => {
                if *maximum > 10_000 {
                    self.cause(at, CauseKind::InvalidAggregateDomain)?;
                }
                self.wrappers(element, at)?;
            }
            NativeType::Option(value) => self.wrappers(value, at)?,
            NativeType::Boolean
            | NativeType::Scalar { .. }
            | NativeType::Enumeration { .. }
            | NativeType::Record { .. }
            | NativeType::Object { .. }
            | NativeType::Reference { .. } => {}
        }
        Ok(())
    }
    fn numeric(&mut self, at: ExprId, dimensionless: bool, rational: bool) -> Result<()> {
        if let Some(ty) = self.get(self.var(at), at)? {
            if ty.numeric_unit().is_none()
                || (rational && ty.rational().is_none())
                || (dimensionless && ty.numeric_unit() != Some(&Unit::Dimensionless))
            {
                self.cause(at, CauseKind::ForbiddenOperator)?;
            }
        }
        Ok(())
    }
    fn equality(&mut self, ty: &NativeType<'_>, at: ExprId) -> Result<()> {
        match ty {
            NativeType::Boolean | NativeType::Scalar { .. } | NativeType::Enumeration { .. } => {}
            NativeType::Object { .. } | NativeType::Reference { .. } => {
                self.require(at, Capability::Graph)?
            }
            NativeType::Record { .. } | NativeType::Option(_) | NativeType::Sequence { .. } => {
                self.cause(at, CauseKind::ForbiddenOperator)?
            }
        }
        Ok(())
    }
    fn sequence(&mut self, domain: ExprId, at: ExprId) -> Result<Option<u32>> {
        match self.get(self.var(domain), at)? {
            Some(NativeType::Sequence { maximum, .. }) => Ok(Some(maximum)),
            Some(_) => {
                self.cause(at, CauseKind::ExpectedSequence)?;
                Ok(None)
            }
            None => Ok(None),
        }
    }
    fn count_domain(&mut self, domain: ExprId, at: ExprId) -> Result<()> {
        let maximum = self.sequence(domain, at)?;
        if let (Some(maximum), Some(ty)) = (maximum, self.get(self.var(at), at)?) {
            if !ty.integer().is_some_and(|integer| {
                integer.minimum() <= 0 && integer.maximum() >= i64::from(maximum)
            }) || ty.numeric_unit() != Some(&Unit::Dimensionless)
            {
                self.cause(at, CauseKind::InvalidAggregateDomain)?;
            }
        }
        Ok(())
    }
    fn graph(
        &mut self,
        start: ExprId,
        name: &qsl_foundation::Spanned<String>,
        at: ExprId,
    ) -> Result<()> {
        let Some(endpoint) = self.get(self.var(start), at)? else {
            return Ok(());
        };
        let (model, role) = match &endpoint {
            NativeType::Object { model, role } | NativeType::Reference { model, role } => {
                (*model, *role)
            }
            NativeType::Boolean
            | NativeType::Scalar { .. }
            | NativeType::Enumeration { .. }
            | NativeType::Record { .. }
            | NativeType::Option(_)
            | NativeType::Sequence { .. } => {
                self.cause(at, CauseKind::InvalidGraphEdge)?;
                return Ok(());
            }
        };
        let object = NativeType::Object { model, role };
        let edge = self.field_inner(&object, name, at)?;
        let leaf = match edge.as_ref() {
            Some(NativeType::Option(value)) => Some(value.as_ref()),
            Some(NativeType::Sequence { element, .. }) => Some(element.as_ref()),
            other => other,
        };
        if !matches!(leaf,Some(NativeType::Reference{model:target,role:target_role}) if target.environment().owner()==model.environment().owner()&&target_role.record==role.record&&target_role.universe==role.universe)
        {
            self.cause(at, CauseKind::InvalidGraphEdge)?;
        }
        Ok(())
    }
}
