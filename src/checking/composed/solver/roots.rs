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
            c::DeclarationKind::Protocol(protocol) => {
                self.activation(&protocol.activation)?;
                self.captures(&protocol.captures)?;
                for channel in &protocol.channels {
                    self.work
                        .charge(D::Constraints, 1, self.declaration_site())?;
                    match &channel.ordering {
                        c::Ordering::Unordered(_) => {}
                        c::Ordering::Fifo { key, .. } => {
                            self.obligation(*key, ObligationKind::CaptureInput)?
                        }
                    }
                }
                for requirement in &protocol.requirements {
                    self.work
                        .charge(D::Constraints, 1, self.declaration_site())?;
                    match requirement {
                        c::ProtocolRequirement::Temporal { .. } => {}
                        c::ProtocolRequirement::Compensation(compensation) => {
                            self.captures(&compensation.registration_captures)?;
                            self.captures(&compensation.activation_captures)?;
                            self.boolean(compensation.guard)?;
                            self.boolean(compensation.retry)?;
                            self.boolean(compensation.recover)?;
                        }
                    }
                }
                let range = owned(
                    self.unit.controls(),
                    syntax.span,
                    |node| node.span,
                    self.work,
                    self.declaration_site(),
                )?;
                for index in range {
                    self.work
                        .charge(D::Expressions, 1, self.declaration_site())?;
                    match &self.unit.controls()[index].kind {
                        c::ControlKind::Sequence(_)
                        | c::ControlKind::Parallel { .. }
                        | c::ControlKind::Await { .. } => {}
                        c::ControlKind::Choice { cases, .. } => {
                            for case in cases {
                                self.boolean(case.guard)?;
                            }
                        }
                        c::ControlKind::Repeat { guard, .. } => self.boolean(*guard)?,
                        c::ControlKind::Event(event) => self.boolean(event.constraint)?,
                        c::ControlKind::Commit { constraint, .. } => self.boolean(*constraint)?,
                        c::ControlKind::Check {
                            profile,
                            expression,
                        } => {
                            let uses = &self
                                .binding
                                .definitions()
                                .expect("profile report")
                                .declarations[self.output.declaration.index()]
                            .uses;
                            let mut selected = false;
                            for (profile_index, usage) in uses.iter().enumerate() {
                                self.work
                                    .charge(D::Constraints, 1, self.site(*expression))?;
                                if usage.alias.span == profile.span {
                                    selected = true;
                                    let owner = self.unit.expressions()[expression.0].span;
                                    for index in self.range.clone() {
                                        self.work.charge(
                                            D::Expressions,
                                            1,
                                            self.site(ExprId(index)),
                                        )?;
                                        let span = self.unit.expressions()[index].span;
                                        if span.start >= owner.start && span.end <= owner.end {
                                            self.profiles[index - self.range.start] = profile_index;
                                        }
                                    }
                                    break;
                                }
                            }
                            if !selected {
                                self.cause(*expression, CauseKind::UpstreamBinding)?;
                            }
                            self.boolean(*expression)?;
                        }
                    }
                }
                self.boolean(protocol.finish.constraint)?;
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
