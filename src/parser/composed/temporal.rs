// SPDX-License-Identifier: AGPL-3.0-or-later
//! Temporal nodes share value leaves with the unit's existing expression parser.
//!
//! Temporal formulas run on an explicit heap stack like value expressions
//! (`parser/expression.rs`): a binary right operand, a prefix operator's
//! operand and a parenthesized group each push a [`Frame`] instead of
//! recursing, so neither a long `implies`/`not`/`always` chain nor deep
//! grouping spends native stack. `holds(…)` calls the value-expression parser,
//! which is itself iterative.
use super::*;

/// A partially parsed temporal construct waiting for a sub-formula.
enum Frame {
    /// Waiting for the left operand of a binary formula at `minimum`.
    BinaryLeft { minimum: u8 },
    /// Waiting for the right operand of `left op …`.
    BinaryRight {
        minimum: u8,
        left: TemporalId,
        relation_seen: bool,
        op: Spanned<TemporalOp>,
        interval: Option<Interval>,
    },
    /// Waiting for the formula inside `( … )`, which these prefix operators
    /// then wrap.
    Group {
        start: usize,
        operators: Vec<(Spanned<TemporalOp>, Option<Interval>)>,
    },
}

/// The result of one descent or continuation.
enum Next {
    /// A complete formula for the innermost frame.
    Formula(TemporalId),
    /// Frames were pushed; parse a binary formula at this minimum next.
    Descend(u8),
}

impl Parser {
    /// Parse one temporal formula whose binary operators bind at least as
    /// tightly as `minimum`.
    pub(super) fn temporal_expression(
        &mut self,
        minimum: u8,
    ) -> Result<TemporalId, Box<Diagnostic>> {
        let mut frames = Vec::new();
        let mut minimum = minimum;
        'descend: loop {
            frames.push(Frame::BinaryLeft { minimum });
            let mut next = self.temporal_unary(&mut frames)?;
            loop {
                match next {
                    Next::Descend(inner) => {
                        minimum = inner;
                        continue 'descend;
                    }
                    Next::Formula(formula) => {
                        let Some(frame) = frames.pop() else {
                            return Ok(formula);
                        };
                        next = self.temporal_resume(frame, formula, &mut frames)?;
                    }
                }
            }
        }
    }

    fn temporal_resume(
        &mut self,
        frame: Frame,
        formula: TemporalId,
        frames: &mut Vec<Frame>,
    ) -> Result<Next, Box<Diagnostic>> {
        match frame {
            Frame::BinaryLeft { minimum } => self.temporal_binary(minimum, formula, false, frames),
            Frame::BinaryRight {
                minimum,
                left,
                relation_seen,
                op,
                interval,
            } => {
                let span = Span {
                    start: self.temporal[left.0].span.start,
                    end: self.temporal[formula.0].span.end,
                };
                let combined = self.temporal_add(
                    TemporalKind::Binary {
                        op,
                        interval,
                        left,
                        right: formula,
                    },
                    span,
                )?;
                self.temporal_binary(minimum, combined, relation_seen, frames)
            }
            Frame::Group { start, operators } => {
                self.close(K::CloseParen)?;
                let group =
                    self.temporal_add(TemporalKind::Group(formula), self.range_from(start))?;
                self.temporal_wrap(group, operators).map(Next::Formula)
            }
        }
    }

    /// The operator loop of a binary formula at `minimum` whose operand so
    /// far is `left`.
    fn temporal_binary(
        &mut self,
        minimum: u8,
        left: TemporalId,
        relation_seen: bool,
        frames: &mut Vec<Frame>,
    ) -> Result<Next, Box<Diagnostic>> {
        let (op, power) = match self.peek().kind {
            K::Implies => (TemporalOp::Implies, 1),
            K::Or => (TemporalOp::Or, 2),
            K::And => (TemporalOp::And, 3),
            K::Until => (TemporalOp::Until, 4),
            K::Release => (TemporalOp::Release, 4),
            K::Since => (TemporalOp::Since, 4),
            K::Triggered => (TemporalOp::Triggered, 4),
            _ => return Ok(Next::Formula(left)),
        };
        if power < minimum {
            return Ok(Next::Formula(left));
        }
        if power == 4 && relation_seen {
            return Err(self.unexpected("a non-chained temporal relation"));
        }
        let token = self.take();
        let interval = if power == 4 {
            Some(self.interval()?)
        } else {
            None
        };
        frames.push(Frame::BinaryRight {
            minimum,
            left,
            relation_seen: power == 4,
            op: Spanned {
                value: op,
                span: token.span,
            },
            interval,
        });
        // `implies` is right-associative: its right operand may itself be
        // an `implies` chain.
        Ok(Next::Descend(power + u8::from(op != TemporalOp::Implies)))
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

    /// Prefix operators, then a constant, `holds(…)` or a group.
    fn temporal_unary(&mut self, frames: &mut Vec<Frame>) -> Result<Next, Box<Diagnostic>> {
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
                frames.push(Frame::Group {
                    start: token.span.start,
                    operators,
                });
                return Ok(Next::Descend(0));
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
        let argument = self.temporal_add(kind, self.range_from(token.span.start))?;
        self.temporal_wrap(argument, operators).map(Next::Formula)
    }

    /// Wrap `argument` in `operators`, innermost (rightmost) first.
    fn temporal_wrap(
        &mut self,
        mut argument: TemporalId,
        operators: Vec<(Spanned<TemporalOp>, Option<Interval>)>,
    ) -> Result<TemporalId, Box<Diagnostic>> {
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
