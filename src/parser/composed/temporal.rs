// SPDX-License-Identifier: AGPL-3.0-or-later
//! Temporal nodes share value leaves with the unit's existing expression parser.
use super::*;

impl Parser {
    pub(super) fn temporal_expression(
        &mut self,
        minimum: u8,
    ) -> Result<TemporalId, Box<Diagnostic>> {
        self.enter()?;
        let result = self.temporal_binary(minimum);
        self.depth -= 1;
        result
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

    fn temporal_binary(&mut self, minimum: u8) -> Result<TemporalId, Box<Diagnostic>> {
        let mut left = self.temporal_unary()?;
        let mut relation_seen = false;
        loop {
            let (op, power) = match self.peek().kind {
                K::Implies => (TemporalOp::Implies, 1),
                K::Or => (TemporalOp::Or, 2),
                K::And => (TemporalOp::And, 3),
                K::Until => (TemporalOp::Until, 4),
                K::Release => (TemporalOp::Release, 4),
                K::Since => (TemporalOp::Since, 4),
                K::Triggered => (TemporalOp::Triggered, 4),
                _ => break,
            };
            if power < minimum {
                break;
            }
            if power == 4 && relation_seen {
                return Err(self.unexpected("a non-chained temporal relation"));
            }
            relation_seen = power == 4;
            let token = self.take();
            let interval = if power == 4 {
                Some(self.interval()?)
            } else {
                None
            };
            let right = self.temporal_expression(power + u8::from(op != TemporalOp::Implies))?;
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
                self.expect(K::OpenParen)?;
                let value = self.expression()?;
                self.expect(K::CloseParen)?;
                TemporalKind::Holds(value)
            }
            K::OpenParen => {
                let formula = self.temporal_expression(0)?;
                self.expect(K::CloseParen)?;
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
