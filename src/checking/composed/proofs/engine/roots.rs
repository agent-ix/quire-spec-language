// SPDX-License-Identifier: AGPL-3.0-only
//! Original declaration-local arena roots and explicitly authored captures.

use super::*;

impl Builder<'_, '_> {
    pub(super) fn roots(&mut self, syntax: &c::Declaration) -> Result<()> {
        let Some(first) = self.typed.nodes().first() else {
            return Ok(());
        };
        let start = first.expression.0;
        self.work.charge(
            D::Records,
            self.typed.nodes().len(),
            self.declaration_site(),
        )?;
        let mut child = vec![false; self.typed.nodes().len()];
        for node in self.typed.nodes() {
            let at = node.expression;
            self.work.charge(D::Expressions, 1, self.site(at))?;
            let site = self.site(at);
            let mut mark = |id: ExprId| -> Result<()> {
                self.work.charge(D::Expressions, 1, site)?;
                child[id.0 - start] = true;
                Ok(())
            };
            match &self.unit.expressions()[at.0].kind {
                c::ValueKind::Shared(kind) => match kind {
                    ExprKind::Group { inner } => mark(*inner)?,
                    ExprKind::Field { base, .. } => mark(*base)?,
                    ExprKind::Unary { argument, .. } | ExprKind::Call { argument, .. } => {
                        mark(*argument)?
                    }
                    ExprKind::Binary { left, right, .. } => {
                        mark(*left)?;
                        mark(*right)?;
                    }
                    ExprKind::Let { value, body, .. } => {
                        mark(*value)?;
                        mark(*body)?;
                    }
                    ExprKind::If {
                        condition,
                        then_value,
                        else_value,
                    } => {
                        mark(*condition)?;
                        mark(*then_value)?;
                        mark(*else_value)?;
                    }
                    ExprKind::Quantifier {
                        domain, predicate, ..
                    } => {
                        mark(*domain)?;
                        mark(*predicate)?;
                    }
                    ExprKind::Reaches { start, target, .. } => {
                        mark(*start)?;
                        mark(*target)?;
                    }
                    ExprKind::Boolean(_)
                    | ExprKind::Integer(_)
                    | ExprKind::Text(_)
                    | ExprKind::Name(_)
                    | ExprKind::SelfValue
                    | ExprKind::ResultValue
                    | ExprKind::EnumValue { .. } => {}
                },
                c::ValueKind::Invoke { arguments, .. } => {
                    for argument in arguments {
                        mark(*argument)?;
                    }
                }
                c::ValueKind::Rational { .. } => {}
                c::ValueKind::Product { left, right, .. } => {
                    mark(*left)?;
                    mark(*right)?;
                }
                c::ValueKind::Size { argument, .. } => mark(*argument)?,
                c::ValueKind::Contains { collection, member } => {
                    mark(*collection)?;
                    mark(*member)?;
                }
                c::ValueKind::Query { domain, body, .. } => {
                    mark(*domain)?;
                    mark(*body)?;
                }
            }
        }
        let mut captures = BTreeMap::new();
        match &syntax.kind {
            c::DeclarationKind::Predicate { .. } | c::DeclarationKind::State { .. } => {}
            c::DeclarationKind::Temporal {
                captures: authored, ..
            } => self.captures(authored, &mut captures)?,
            c::DeclarationKind::Protocol(protocol) => {
                self.captures(&protocol.captures, &mut captures)?;
                for requirement in &protocol.requirements {
                    self.work
                        .charge(D::Expressions, 1, self.declaration_site())?;
                    match requirement {
                        c::ProtocolRequirement::Temporal { .. } => {}
                        c::ProtocolRequirement::Compensation(compensation) => {
                            self.captures(&compensation.registration_captures, &mut captures)?;
                            self.captures(&compensation.activation_captures, &mut captures)?;
                        }
                    }
                }
            }
        }
        // Top-level shared expressions live at independent family evaluation
        // points. No monitor/control truth is imported as a value guard. Captures
        // preserve their initialized key; later reads never replay the initializer.
        for (index, is_child) in child.into_iter().enumerate() {
            if !is_child {
                let at = ExprId(start + index);
                self.work.charge(D::Types, 1, self.site(at))?;
                let path = Path::empty();
                let value = if let Some(&binder) = captures.get(&at.0) {
                    self.capture(binder, at, &path, 1)?
                } else {
                    self.visit(at, &path, 1)?
                };
                self.goal(at, value.graph, &path)?;
            }
        }
        Ok(())
    }
    fn captures(
        &mut self,
        authored: &[c::Capture],
        output: &mut BTreeMap<usize, usize>,
    ) -> Result<()> {
        for capture in authored {
            self.work.charge(D::Types, 1, self.site(capture.value))?;
            self.work.charge(D::Records, 1, self.site(capture.value))?;
            let binder = self.binders[&capture.parameter.name.span.start];
            output.insert(capture.value.0, binder);
        }
        Ok(())
    }
}
