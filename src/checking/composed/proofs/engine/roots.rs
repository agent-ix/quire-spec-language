// SPDX-License-Identifier: AGPL-3.0-only
//! Original declaration-local arena roots and explicitly authored captures.

use super::*;

impl Builder<'_, '_> {
    pub(super) fn roots(&mut self, syntax: &c::Declaration) -> Result<()> {
        let site = self.declaration_site();
        let range = c::arena::owned_range(
            self.unit.expressions(),
            syntax.span,
            |node| node.span,
            || self.work.charge(D::Expressions, 1, site),
        )?;
        // The parser owns contiguous declaration arenas; the type result must
        // retain exactly that window before proof-local indexing is permitted.
        if range.len() != self.typed.nodes().len() {
            return Err(upstream(site));
        }
        let start = range.start;
        self.work.charge(
            D::Records,
            self.typed.nodes().len(),
            self.declaration_site(),
        )?;
        let mut child = vec![false; self.typed.nodes().len()];
        for (index, node) in self.typed.nodes().iter().enumerate() {
            let at = node.expression;
            self.work.charge(D::Expressions, 1, self.site(at))?;
            let site = self.site(at);
            if at.0 != start + index {
                return Err(upstream(site));
            }
            let mut mark = |id: ExprId| -> Result<()> {
                self.work.charge(D::Expressions, 1, site)?;
                let index = id.0.checked_sub(start).ok_or_else(|| upstream(site))?;
                *child.get_mut(index).ok_or_else(|| upstream(site))? = true;
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
            let binder = *self
                .binders
                .get(&capture.parameter.name.span.start)
                .ok_or_else(|| upstream(self.site(capture.value)))?;
            output.insert(capture.value.0, binder);
        }
        Ok(())
    }
}
