// SPDX-License-Identifier: AGPL-3.0-or-later
//! Temporal nodes share value leaves with the unit's existing expression parser.
use super::*;

/// Temporal binary operator and its precedence power, or `None` if `kind`
/// does not start one. `Implies` is lowest (right-associative and
/// unbounded); `Or`/`And` are left-associative; the four temporal relations
/// share one non-chaining precedence level (`relation_seen` below refuses a
/// second one without an intervening `and`).
fn temporal_binary_op(kind: &K) -> Option<(TemporalOp, u8)> {
    Some(match kind {
        K::Implies => (TemporalOp::Implies, 1),
        K::Or => (TemporalOp::Or, 2),
        K::And => (TemporalOp::And, 3),
        K::Until => (TemporalOp::Until, 4),
        K::Release => (TemporalOp::Release, 4),
        K::Since => (TemporalOp::Since, 4),
        K::Triggered => (TemporalOp::Triggered, 4),
        _ => return None,
    })
}

impl Parser {
    /// `implies`/`or`/`and`/relation chains are parsed with an explicit
    /// loop rather than by Rust recursion per operator, matching
    /// `Parser::binary` in the base expression grammar: NFR-001 gives these
    /// chains nesting depth 0, bounded only by the token/node ceilings, so
    /// a long chain must not grow the native call stack per element.
    pub(super) fn temporal_expression(
        &mut self,
        minimum: u8,
    ) -> Result<TemporalId, Box<Diagnostic>> {
        let mut left = self.temporal_unary()?;
        let mut relation_seen = false;
        while let Some((op, power)) = temporal_binary_op(&self.peek().kind) {
            if power < minimum {
                break;
            }
            if power == 4 && relation_seen {
                return Err(self.unexpected("a non-chained temporal relation"));
            }
            relation_seen = power == 4;
            if op == TemporalOp::Implies {
                left = self.temporal_implies_chain(left)?;
                continue;
            }
            let token = self.take();
            let interval = if power == 4 {
                Some(self.interval()?)
            } else {
                None
            };
            let right = self.temporal_expression(power + 1)?;
            let span = Span {
                start: self.temporal[left.0].span.start,
                end: self.temporal[right.0].span.end,
            };
            left = self.temporal_add(
                TemporalKind::Binary {
                    op: Spanned {
                        value: op,
                        span: token.span,
                    },
                    interval,
                    left,
                    right,
                },
                span,
            )?;
        }
        Ok(left)
    }

    /// Collect a right-associative temporal `implies` chain iteratively,
    /// then fold the operands from the right (see
    /// `Parser::implies_chain` in the base expression grammar for the same
    /// pattern and rationale).
    fn temporal_implies_chain(&mut self, first: TemporalId) -> Result<TemporalId, Box<Diagnostic>> {
        let mut operands = vec![first];
        let mut tokens = Vec::new();
        loop {
            tokens.push(self.take());
            operands.push(self.temporal_expression(2)?);
            if !matches!(
                temporal_binary_op(&self.peek().kind),
                Some((TemporalOp::Implies, _))
            ) {
                break;
            }
        }
        let mut acc = operands.pop().expect("at least one temporal operand");
        while let Some(left) = operands.pop() {
            let token = tokens.pop().expect("one token per additional operand");
            let span = Span {
                start: self.temporal[left.0].span.start,
                end: self.temporal[acc.0].span.end,
            };
            acc = self.temporal_add(
                TemporalKind::Binary {
                    op: Spanned {
                        value: TemporalOp::Implies,
                        span: token.span,
                    },
                    interval: None,
                    left,
                    right: acc,
                },
                span,
            )?;
        }
        Ok(acc)
    }

    fn temporal_add(
        &mut self,
        kind: TemporalKind,
        span: Span,
    ) -> Result<TemporalId, Box<Diagnostic>> {
        self.charge(span)?;
        let id = TemporalId(self.temporal.len());
        self.temporal.push(Temporal { kind, span });
        Ok(id)
    }

    fn temporal_unary(&mut self) -> Result<TemporalId, Box<Diagnostic>> {
        let mut operators = Vec::new();
        loop {
            let op = match self.peek().kind {
                K::Not => TemporalOp::Not,
                K::Eventually => TemporalOp::Eventually,
                K::Always => TemporalOp::Always,
                K::Once => TemporalOp::Once,
                K::Historically => TemporalOp::Historically,
                _ => break,
            };
            let token = self.take();
            let interval = if op == TemporalOp::Not {
                None
            } else {
                Some(self.interval()?)
            };
            operators.push((
                Spanned {
                    value: op,
                    span: token.span,
                },
                interval,
            ));
        }
        let token = self.take();
        let kind = match token.kind {
            K::True | K::False => TemporalKind::Constant(token.kind == K::True),
            K::Holds => {
                self.open(K::OpenParen)?;
                let value = self.expression()?;
                self.close(K::CloseParen)?;
                TemporalKind::Holds(value)
            }
            K::OpenParen => {
                self.open_taken(token.span)?;
                let formula = self.temporal_expression(0)?;
                self.close(K::CloseParen)?;
                TemporalKind::Group(formula)
            }
            _ => {
                return Err(self.failure(
                    Code::InvalidSyntax,
                    Phase::Parse,
                    token.span,
                    "expected temporal constant, group or holds(value)",
                ))
            }
        };
        let mut argument = self.temporal_add(kind, self.range_from(token.span.start))?;
        for (op, interval) in operators.into_iter().rev() {
            let span = Span {
                start: op.span.start,
                end: self.temporal[argument.0].span.end,
            };
            argument = self.temporal_add(
                TemporalKind::Unary {
                    op,
                    interval,
                    argument,
                },
                span,
            )?;
        }
        Ok(argument)
    }
}
