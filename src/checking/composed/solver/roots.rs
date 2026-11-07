// SPDX-License-Identifier: AGPL-3.0-or-later
//! Value roots and nested profile selections from the original family ASTs.
use super::*;

impl Solver<'_, '_, '_, '_> {
    pub(super) fn roots(&mut self, syntax: &c::Declaration) -> Result<()> {
        match &syntax.kind {
            c::DeclarationKind::Predicate { body, .. } => {
                self.require(*body, Capability::Queries)?;
                self.boolean(*body)?;
            }
            c::DeclarationKind::State { body, .. } => {
                self.boolean(*body)?;
                self.obligation(*body, ObligationKind::InvocationContext)?;
            }
            c::DeclarationKind::Temporal {
                activation,
                captures,
                ..
            } => {
                self.activation(activation)?;
                self.captures(captures)?;
                let range = owned(
                    self.unit.temporal_nodes(),
                    syntax.span,
                    |node| node.span,
                    self.work,
                    self.declaration_site(),
                )?;
                for index in range {
                    self.work
                        .charge(D::Expressions, 1, self.declaration_site())?;
                    match &self.unit.temporal_nodes()[index].kind {
                        c::TemporalKind::Holds(expression) => self.boolean(*expression)?,
                        c::TemporalKind::Constant(_)
                        | c::TemporalKind::Group(_)
                        | c::TemporalKind::Unary { .. }
                        | c::TemporalKind::Binary { .. } => {}
                    }
                }
            }
        }
        Ok(())
    }
    fn activation(&mut self, activation: &c::Activation) -> Result<()> {
        match activation {
            c::Activation::Origin { .. } => {}
            c::Activation::Each { guard, .. } => {
                if let Some(guard) = guard {
                    self.boolean(*guard)?;
                }
            }
        }
        Ok(())
    }
    fn captures(&mut self, captures: &[c::Capture]) -> Result<()> {
        for capture in captures {
            for (index, binder) in self.scope.binders.iter().enumerate() {
                self.work
                    .charge(D::Constraints, 1, self.site(capture.value))?;
                if binder.span == capture.parameter.name.span {
                    self.work.charge(D::Records, 1, self.site(capture.value))?;
                    self.captures.insert(index, capture.value);
                    self.unify(
                        self.binder_var(index),
                        self.var(capture.value),
                        capture.value,
                    )?;
                    break;
                }
            }
            self.obligation(capture.value, ObligationKind::CaptureInput)?;
        }
        Ok(())
    }
}
