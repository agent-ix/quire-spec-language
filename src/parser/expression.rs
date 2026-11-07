// SPDX-License-Identifier: AGPL-3.0-or-later
//! The value-expression grammar shared by both native editions, run on an
//! explicit heap stack.
//!
//! The grammar is recursive: `let`/`if` operands, binary right operands,
//! prefix operators and bracketed sub-expressions (`(…)`, builtin and
//! extended calls, quantifiers) each contain another expression. Parsing it
//! by Rust recursion would spend native stack per chain element and per
//! bracket pair. Instead every place the recursive formulation would call a
//! sub-parse pushes a [`Frame`] recording how to continue, and the parse
//! loops: descend into a [`Goal`] until a value exists, then hand that value
//! to the innermost frame. Stack use is constant; the frame stack grows on
//! the heap, bounded by the token ceiling. Nodes are added in the order the
//! recursive formulation added them (every operand before its operator), so
//! the node arena is unchanged.
use super::{binary_op, Parser, K};
use crate::syntax::{BinaryOp, Builtin, ExprId, ExprKind, UnaryOp};
use qsl_foundation::{Code, Diagnostic, Phase, Span, Spanned};

/// What to parse next.
#[derive(Clone, Copy)]
enum Goal {
    /// `let`/`if` prefixes, then a binary expression.
    Expression,
    /// A binary expression whose operators bind at least this tightly.
    Binary(u8),
}

/// A partially parsed construct waiting for the sub-expression being parsed.
enum Frame {
    LetValue {
        start: usize,
        name: Spanned<String>,
    },
    LetBody {
        start: usize,
        name: Spanned<String>,
        value: ExprId,
    },
    IfCondition {
        start: usize,
    },
    IfThen {
        start: usize,
        condition: ExprId,
    },
    IfElse {
        start: usize,
        condition: ExprId,
        then_value: ExprId,
    },
    /// Waiting for the left operand (the unary expression) of a binary
    /// expression at `minimum`.
    BinaryLeft {
        minimum: u8,
    },
    /// Waiting for the right operand of `left operator …`.
    BinaryRight {
        minimum: u8,
        left: ExprId,
        comparison_seen: bool,
        op: BinaryOp,
    },
    /// Waiting for the operand of these prefix operators.
    UnaryArgument {
        operators: Vec<(UnaryOp, Span)>,
    },
    Group {
        start: usize,
    },
    Builtin {
        start: usize,
        builtin: Builtin,
    },
    QuantifierDomain {
        start: usize,
        universal: bool,
        name: Spanned<String>,
    },
    QuantifierPredicate {
        start: usize,
        universal: bool,
        name: Spanned<String>,
        domain: ExprId,
    },
    ReachesStart {
        start: usize,
    },
    ReachesTarget {
        start: usize,
        first: ExprId,
    },
}

/// The result of one descent or continuation.
enum Next {
    /// A complete value for the innermost frame.
    Value(ExprId),
    /// Frames were pushed; parse this next.
    Descend(Goal),
}

type Parsed<T> = Result<T, Box<Diagnostic>>;

impl Parser {
    /// Parse one value expression.
    pub(super) fn expression(&mut self) -> Parsed<ExprId> {
        let mut frames = Vec::new();
        let mut goal = Goal::Expression;
        'descend: loop {
            let mut next = match goal {
                Goal::Expression => self.start_expression(&mut frames)?,
                Goal::Binary(minimum) => self.start_binary(minimum, &mut frames)?,
            };
            loop {
                match next {
                    Next::Descend(inner) => {
                        goal = inner;
                        continue 'descend;
                    }
                    Next::Value(value) => {
                        let Some(frame) = frames.pop() else {
                            return Ok(value);
                        };
                        next = self.resume(frame, value, &mut frames)?;
                    }
                }
            }
        }
    }

    fn start_expression(&mut self, frames: &mut Vec<Frame>) -> Parsed<Next> {
        let start = self.peek().span.start;
        if self.eat(K::Let) {
            let name = self.identifier()?;
            self.expect(K::Equal)?;
            frames.push(Frame::LetValue { start, name });
            return Ok(Next::Descend(Goal::Expression));
        }
        if self.eat(K::If) {
            frames.push(Frame::IfCondition { start });
            return Ok(Next::Descend(Goal::Expression));
        }
        self.start_binary(0, frames)
    }

    /// A binary expression starts with a unary one: prefix operators, then
    /// a primary.
    fn start_binary(&mut self, minimum: u8, frames: &mut Vec<Frame>) -> Parsed<Next> {
        frames.push(Frame::BinaryLeft { minimum });
        let mut operators = Vec::new();
        while self.is(K::Not) || self.is(K::Minus) {
            let token = self.take();
            let op = if matches!(token.kind, K::Minus) {
                UnaryOp::Negate
            } else {
                UnaryOp::Not
            };
            operators.push((op, token.span));
        }
        frames.push(Frame::UnaryArgument { operators });
        self.start_primary(frames)
    }

    fn resume(&mut self, frame: Frame, value: ExprId, frames: &mut Vec<Frame>) -> Parsed<Next> {
        match frame {
            Frame::LetValue { start, name } => {
                self.expect(K::In)?;
                frames.push(Frame::LetBody { start, name, value });
                Ok(Next::Descend(Goal::Expression))
            }
            Frame::LetBody {
                start,
                name,
                value: bound,
            } => {
                let end = self.expression_span(value).end;
                let id = self.add(
                    ExprKind::Let {
                        name,
                        value: bound,
                        body: value,
                    },
                    start,
                    end,
                )?;
                Ok(Next::Value(id))
            }
            Frame::IfCondition { start } => {
                self.expect(K::Then)?;
                frames.push(Frame::IfThen {
                    start,
                    condition: value,
                });
                Ok(Next::Descend(Goal::Expression))
            }
            Frame::IfThen { start, condition } => {
                self.expect(K::Else)?;
                frames.push(Frame::IfElse {
                    start,
                    condition,
                    then_value: value,
                });
                Ok(Next::Descend(Goal::Expression))
            }
            Frame::IfElse {
                start,
                condition,
                then_value,
            } => {
                let end = self.expression_span(value).end;
                let id = self.add(
                    ExprKind::If {
                        condition,
                        then_value,
                        else_value: value,
                    },
                    start,
                    end,
                )?;
                Ok(Next::Value(id))
            }
            Frame::BinaryLeft { minimum } => self.continue_binary(minimum, value, false, frames),
            Frame::BinaryRight {
                minimum,
                left,
                comparison_seen,
                op,
            } => {
                let span = Span {
                    start: self.expression_span(left).start,
                    end: self.expression_span(value).end,
                };
                let combined = self.add(
                    ExprKind::Binary {
                        op,
                        left,
                        right: value,
                    },
                    span.start,
                    span.end,
                )?;
                self.continue_binary(minimum, combined, comparison_seen, frames)
            }
            Frame::UnaryArgument { operators } => {
                let mut argument = value;
                while self.eat(K::Dot) {
                    let name = self.identifier()?;
                    let end = self.tokens[self.at - 1].span.end;
                    argument = self.add(
                        ExprKind::Field {
                            base: argument,
                            name,
                        },
                        self.expression_span(argument).start,
                        end,
                    )?;
                }
                for (op, operator_span) in operators.into_iter().rev() {
                    argument = self.add(
                        ExprKind::Unary { op, argument },
                        operator_span.start,
                        self.expression_span(argument).end,
                    )?;
                }
                Ok(Next::Value(argument))
            }
            Frame::Group { start } => {
                self.expect(K::CloseParen)?;
                self.finish_primary(ExprKind::Group { inner: value }, start)
            }
            Frame::Builtin { start, builtin } => {
                self.expect(K::CloseParen)?;
                self.finish_primary(
                    ExprKind::Call {
                        builtin,
                        argument: value,
                    },
                    start,
                )
            }
            Frame::QuantifierDomain {
                start,
                universal,
                name,
            } => {
                self.expect(K::Colon)?;
                frames.push(Frame::QuantifierPredicate {
                    start,
                    universal,
                    name,
                    domain: value,
                });
                Ok(Next::Descend(Goal::Expression))
            }
            Frame::QuantifierPredicate {
                start,
                universal,
                name,
                domain,
            } => {
                self.expect(K::CloseParen)?;
                self.finish_primary(
                    ExprKind::Quantifier {
                        universal,
                        name,
                        domain,
                        predicate: value,
                    },
                    start,
                )
            }
            Frame::ReachesStart { start } => {
                self.expect(K::Comma)?;
                frames.push(Frame::ReachesTarget {
                    start,
                    first: value,
                });
                Ok(Next::Descend(Goal::Expression))
            }
            Frame::ReachesTarget { start, first } => {
                self.expect(K::Comma)?;
                let field = self.identifier()?;
                self.expect(K::CloseParen)?;
                self.finish_primary(
                    ExprKind::Reaches {
                        start: first,
                        target: value,
                        field,
                    },
                    start,
                )
            }
        }
    }

    /// The operator loop of a binary expression at `minimum` whose operand so
    /// far is `left`: return `left` when no operator binds tightly enough,
    /// otherwise descend into the right operand.
    fn continue_binary(
        &mut self,
        minimum: u8,
        left: ExprId,
        comparison_seen: bool,
        frames: &mut Vec<Frame>,
    ) -> Parsed<Next> {
        let Some(op) = binary_op(&self.peek().kind) else {
            return Ok(Next::Value(left));
        };
        if op.power() < minimum {
            return Ok(Next::Value(left));
        }
        if op.comparison() && comparison_seen {
            return Err(self.unexpected("a non-chained comparison (use explicit parentheses)"));
        }
        // A conjunction starts a new comparison operand; a second relation
        // without it is deliberately invalid rather than a guessed chain.
        self.take();
        // `implies` is right-associative: its right operand may itself be an
        // `implies` chain.
        let right_minimum = op.power() + u8::from(op != BinaryOp::Implies);
        frames.push(Frame::BinaryRight {
            minimum,
            left,
            comparison_seen: op.comparison(),
            op,
        });
        Ok(Next::Descend(Goal::Binary(right_minimum)))
    }

    /// Add a primary expression spanning from `start` to the last consumed
    /// token.
    fn finish_primary(&mut self, kind: ExprKind, start: usize) -> Parsed<Next> {
        let end = self.tokens[self.at - 1].span.end;
        let id = self.add(kind, start, end)?;
        Ok(Next::Value(id))
    }

    fn start_primary(&mut self, frames: &mut Vec<Frame>) -> Parsed<Next> {
        let token = self.peek().clone();
        let start = token.span.start;
        let kind = match token.kind {
            K::Text(value) => {
                self.take();
                ExprKind::Text(value)
            }
            K::Integer(value) => {
                self.take();
                ExprKind::Integer(value)
            }
            K::True | K::False => {
                self.take();
                ExprKind::Boolean(token.kind == K::True)
            }
            K::SelfValue => {
                self.take();
                ExprKind::SelfValue
            }
            K::ResultValue => {
                self.take();
                ExprKind::ResultValue
            }
            K::OpenParen => {
                self.expect(K::OpenParen)?;
                frames.push(Frame::Group { start });
                return Ok(Next::Descend(Goal::Expression));
            }
            keyword @ (K::Present | K::Value | K::Deref | K::Size | K::Pre) => {
                let builtin = match keyword {
                    K::Present => Builtin::Present,
                    K::Value => Builtin::Value,
                    K::Deref => Builtin::Deref,
                    K::Size => Builtin::Size,
                    _ => Builtin::Pre,
                };
                self.take();
                self.expect(K::OpenParen)?;
                frames.push(Frame::Builtin { start, builtin });
                return Ok(Next::Descend(Goal::Expression));
            }
            keyword @ (K::Forall | K::Exists) => {
                self.take();
                self.expect(K::OpenParen)?;
                let name = self.identifier()?;
                self.expect(K::In)?;
                frames.push(Frame::QuantifierDomain {
                    start,
                    universal: keyword == K::Forall,
                    name,
                });
                return Ok(Next::Descend(Goal::Expression));
            }
            K::Reaches => {
                self.take();
                self.expect(K::OpenParen)?;
                frames.push(Frame::ReachesStart { start });
                return Ok(Next::Descend(Goal::Expression));
            }
            K::Identifier(model) => {
                let model = Spanned {
                    value: model,
                    span: token.span,
                };
                self.take();
                if self.is(K::OpenParen) {
                    // Same concept as native_model/admission.rs's pure-function
                    // check, met here at parse time instead of model admission:
                    // the native profile does not admit user-defined functions
                    // at all, as a call form or as a declaration. That is a
                    // form this profile's package structure excludes outright,
                    // not a real, catalogued capability this build lacks, so
                    // both land on InvalidPackage rather than UnsupportedConstruct.
                    return Err(self.failure(
                        Code::InvalidPackage,
                        Phase::Profile,
                        token.span,
                        "user function calls are outside the native model profile",
                    ));
                }
                if self.eat(K::Qualify) {
                    let name = self.identifier()?;
                    self.expect(K::Qualify)?;
                    let variant = self.identifier()?;
                    ExprKind::EnumValue {
                        model,
                        name,
                        variant,
                    }
                } else {
                    ExprKind::Name(model)
                }
            }
            _ => return Err(self.unexpected("expression")),
        };
        self.finish_primary(kind, start)
    }
}
