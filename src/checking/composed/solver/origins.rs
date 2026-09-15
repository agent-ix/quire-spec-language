// SPDX-License-Identifier: AGPL-3.0-or-later
//! Immutable observation provenance follows initializer values and branch joins.
use super::*;

impl Solver<'_, '_, '_, '_> {
    pub(super) fn origins(&mut self) -> Result<()> {
        let mut reads = vec![false; self.range.len()];
        for index in self.range.clone() {
            let at = ExprId(index);
            self.work.charge(D::Constraints, 1, self.site(at))?;
            let mut children = Vec::new();
            let mut direct = None;
            let mut eligible = false;
            match &self.unit.expressions()[index].kind {
                c::ValueKind::Shared(kind) => match kind {
                    ExprKind::Boolean(_)
                    | ExprKind::Integer(_)
                    | ExprKind::Text(_)
                    | ExprKind::EnumValue { .. } => direct = Some(ObservationOrigin::Independent),
                    ExprKind::Name(_) | ExprKind::SelfValue | ExprKind::ResultValue => {
                        let node = &self.output.nodes[self.var(at)];
                        if let Some(id) = node.binder {
                            let binder = &self.scope.binders[id.index()];
                            direct = match &binder.ty {
                                scopes::BinderType::Initializer(value) => {
                                    self.output.nodes[self.var(*value)].origin
                                }
                                scopes::BinderType::ElementOf(domain) => {
                                    self.output.nodes[self.var(*domain)].origin
                                }
                                scopes::BinderType::Declared(_)
                                | scopes::BinderType::Context(_)
                                | scopes::BinderType::ModelValue(_) => {
                                    if let Some(value) = self.captures.get(&id.index()) {
                                        self.output.nodes[self.var(*value)].origin
                                    } else if matches!(
                                        binder.kind,
                                        scopes::BinderKind::Parameter
                                            | scopes::BinderKind::InvocationParameter
                                    ) && matches!(
                                        node.ty,
                                        Some(
                                            NativeType::Boolean
                                                | NativeType::Scalar { .. }
                                                | NativeType::Enumeration { .. }
                                        )
                                    ) {
                                        Some(ObservationOrigin::Independent)
                                    } else {
                                        Some(ObservationOrigin::Anchored(binder.anchor))
                                    }
                                }
                            };
                            if let Some(occurrence) = self.occurrence(at)? {
                                if binder.kind == scopes::BinderKind::SelfValue {
                                    direct = Some(ObservationOrigin::Anchored(
                                        occurrence.evaluation_anchor,
                                    ));
                                    eligible =
                                        occurrence.evaluation_anchor == Anchor::InvocationPre;
                                } else if occurrence.evaluation_anchor == Anchor::InvocationPre
                                    && !matches!(
                                        direct,
                                        Some(
                                            ObservationOrigin::Independent
                                                | ObservationOrigin::Anchored(
                                                    Anchor::InvocationPre
                                                )
                                        )
                                    )
                                {
                                    self.cause(at, CauseKind::InvalidPreSelection)?;
                                }
                            }
                        }
                    }
                    ExprKind::Group { inner } => children.push(*inner),
                    ExprKind::Field { base, .. } => {
                        children.push(*base);
                        eligible = self.output.nodes[self.var(*base)].origin
                            == Some(ObservationOrigin::Anchored(Anchor::InvocationPre));
                    }
                    ExprKind::Unary { argument, .. } | ExprKind::Call { argument, .. } => {
                        children.push(*argument)
                    }
                    ExprKind::Binary { left, right, .. } => children.extend([*left, *right]),
                    ExprKind::Let { value, body, .. } => {
                        // Executing an initializer contributes read eligibility;
                        // the body's value determines the result origin. Names
                        // never replay an initializer outside this selector.
                        eligible |= reads[self.var(*value)];
                        children.push(*body);
                    }
                    ExprKind::If {
                        condition,
                        then_value,
                        else_value,
                    } => children.extend([*condition, *then_value, *else_value]),
                    ExprKind::Quantifier {
                        domain, predicate, ..
                    } => children.extend([*domain, *predicate]),
                    ExprKind::Reaches { start, target, .. } => {
                        children.extend([*start, *target]);
                        if self.output.nodes[self.var(*start)].origin
                            != self.output.nodes[self.var(*target)].origin
                        {
                            self.cause(at, CauseKind::InvalidGraphEdge)?;
                        }
                    }
                },
                c::ValueKind::Invoke { arguments, .. } => {
                    children.extend(arguments.iter().copied())
                }
                c::ValueKind::Rational { .. } => direct = Some(ObservationOrigin::Independent),
                c::ValueKind::Product { left, right, .. } => children.extend([*left, *right]),
                c::ValueKind::Size { argument, .. } => children.push(*argument),
                c::ValueKind::Contains { collection, member } => {
                    children.extend([*collection, *member])
                }
                c::ValueKind::Query { domain, body, .. } => children.extend([*domain, *body]),
            }
            let mut origin = direct.unwrap_or(ObservationOrigin::Independent);
            for child in children {
                self.work.charge(D::Constraints, 1, self.site(at))?;
                eligible |= reads[self.var(child)];
                let incoming = self.output.nodes[self.var(child)]
                    .origin
                    .unwrap_or(ObservationOrigin::Independent);
                origin = match (origin, incoming) {
                    (ObservationOrigin::Independent, value)
                    | (value, ObservationOrigin::Independent) => value,
                    (left, right) if left == right => left,
                    _ => ObservationOrigin::Selected {
                        unit: self.output.unit,
                        expression: at,
                    },
                };
            }
            if matches!(
                self.unit.expressions()[index].kind,
                c::ValueKind::Shared(ExprKind::Call {
                    builtin: Builtin::Pre,
                    ..
                })
            ) && (!eligible
                || !matches!(
                    origin,
                    ObservationOrigin::Independent
                        | ObservationOrigin::Anchored(Anchor::InvocationPre)
                ))
            {
                self.cause(at, CauseKind::InvalidPreSelection)?;
            }
            let local = self.var(at);
            reads[local] = eligible;
            self.output.nodes[local].origin = Some(origin);
        }
        Ok(())
    }
}
