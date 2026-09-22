// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-002: bounded declarations and precedence parsing into flat located syntax.
use crate::lexer::{self, Kind, Token};
use crate::syntax::*;
use crate::token::Kind as K;
use crate::{Code, Diagnostic, Phase, Source, SourceIdentity, Span, Spanned};

mod composed;
use crate::syntax::composed as c;

/// Parse the historical `0-draft` grammar. Model imports stay unresolved here.
/// Use [`parse_native`] for explicit source-selected edition dispatch.
pub fn parse(
    identity: SourceIdentity,
    path: impl Into<String>,
    bytes: &[u8],
    limits: Limits,
) -> Result<ParsedUnit, Box<Diagnostic>> {
    let limits = limits.bounded();
    let source = Source::read(identity, path, bytes, limits.source_bytes)?;
    parse_source(source, limits)
}

/// Parse a historical source already loaded (and optionally digest-verified).
pub fn parse_source(source: Source, limits: Limits) -> Result<ParsedUnit, Box<Diagnostic>> {
    let limits = limits.bounded();
    if source.text().len() > limits.source_bytes {
        return Err(crate::diagnostic::error(
            &source,
            Code::ResourceExhausted,
            Phase::Source,
            0,
            0,
            "source byte budget exhausted",
        ));
    }
    let tokens = lexer::lex(&source, lexer_limits(limits))?;
    let mut parser = Parser::new(source, tokens, limits);
    parser.unit()
}

/// Convert this SEAM's own parse limits into the layer-1 lexer's equivalent
/// bounds. The two `Limits` types are independently defined (ADR-011 §6.1);
/// this is a field-by-field conversion at the SEAM/layer-1 boundary, not a
/// compatibility alias.
fn lexer_limits(limits: Limits) -> lexer::Limits {
    lexer::Limits {
        source_bytes: limits.source_bytes,
        tokens: limits.tokens,
        nodes: limits.nodes,
        nesting: limits.nesting,
    }
}

/// Parse the source-selected historical or composed edition, without semantic admission.
pub fn parse_native(
    identity: SourceIdentity,
    path: impl Into<String>,
    bytes: &[u8],
    limits: Limits,
) -> Result<c::NativeUnit, Box<Diagnostic>> {
    let limits = limits.bounded();
    let source = Source::read(identity, path, bytes, limits.source_bytes)?;
    parse_native_source(source, limits)
}

/// Edition-dispatched parsing of an immutable, optionally digest-verified source.
pub fn parse_native_source(
    source: Source,
    limits: Limits,
) -> Result<c::NativeUnit, Box<Diagnostic>> {
    let limits = limits.bounded();
    if source.text().len() > limits.source_bytes {
        return Err(crate::diagnostic::error(
            &source,
            Code::ResourceExhausted,
            Phase::Source,
            0,
            0,
            "source byte budget exhausted",
        ));
    }
    let tokens = lexer::recognize(&source, lexer_limits(limits))?;
    Parser::new(source, tokens, limits).native_unit()
}

struct Parser {
    source: Source,
    tokens: Vec<Token>,
    at: usize,
    nodes: Vec<Expr>,
    depth: usize,
    limits: Limits,
    composed: bool,
    values: Vec<c::Expression>,
    temporal: Vec<c::Temporal>,
    controls: Vec<c::Control>,
    syntax_nodes: usize,
}

impl Parser {
    fn new(source: Source, tokens: Vec<Token>, limits: Limits) -> Self {
        Self {
            source,
            tokens,
            at: 0,
            nodes: Vec::new(),
            depth: 0,
            limits,
            composed: false,
            values: Vec::new(),
            temporal: Vec::new(),
            controls: Vec::new(),
            syntax_nodes: 0,
        }
    }

    fn expression_span(&self, id: ExprId) -> Span {
        if self.composed {
            self.values[id.0].span
        } else {
            self.nodes[id.0].span
        }
    }

    fn add_operator(
        &mut self,
        kind: ExprKind,
        start: usize,
        end: usize,
        operator_span: Span,
    ) -> Result<ExprId, Box<Diagnostic>> {
        let id = self.add(kind, start, end)?;
        if self.composed {
            self.values[id.0].operator_span = Some(operator_span);
        }
        Ok(id)
    }
    fn peek(&self) -> &Token {
        &self.tokens[self.at]
    }
    fn take(&mut self) -> Token {
        let token = self.peek().clone();
        if token.kind != Kind::End {
            self.at += 1;
        }
        token
    }
    fn is(&self, expected: K) -> bool {
        self.peek().kind == expected
    }
    fn eat(&mut self, expected: K) -> bool {
        if self.is(expected) {
            self.take();
            true
        } else {
            false
        }
    }
    fn failure(
        &self,
        code: Code,
        phase: Phase,
        span: Span,
        message: impl Into<String>,
    ) -> Box<Diagnostic> {
        Box::new(Diagnostic {
            code,
            phase,
            source: self.source.identity().clone(),
            path: self.source.path().into(),
            span: self.source.locate(span).expect("parser span"),
            message: message.into(),
        })
    }
    fn unexpected(&self, expected: &str) -> Box<Diagnostic> {
        if !self.composed
            && matches!(
                self.peek().kind,
                Kind::Unsupported | Kind::Fractional | Kind::Slash | Kind::OpenBracket
            )
        {
            let name = self.source.slice(self.peek().span).expect("token span");
            self.failure(
                Code::UnsupportedConstruct,
                Phase::Profile,
                self.peek().span,
                format!("{name} is recognized but unsupported by {PROFILE}"),
            )
        } else {
            self.failure(
                Code::InvalidSyntax,
                Phase::Parse,
                self.peek().span,
                format!("expected {expected}"),
            )
        }
    }
    fn expect(&mut self, expected: K) -> Result<Token, Box<Diagnostic>> {
        if self.peek().kind == expected {
            Ok(self.take())
        } else {
            Err(self.unexpected(expected.description()))
        }
    }
    fn identifier(&mut self) -> Result<Spanned<String>, Box<Diagnostic>> {
        match &self.peek().kind {
            Kind::Identifier(word) => {
                let value = Spanned {
                    value: word.clone(),
                    span: self.peek().span,
                };
                self.take();
                Ok(value)
            }
            _ => Err(self.unexpected("identifier")),
        }
    }
    fn string(&mut self) -> Result<Spanned<String>, Box<Diagnostic>> {
        if let Kind::Text(value) = &self.peek().kind {
            let result = Spanned {
                value: value.clone(),
                span: self.peek().span,
            };
            self.take();
            Ok(result)
        } else {
            Err(self.unexpected("quoted string"))
        }
    }
    fn header(
        &mut self,
        keyword: K,
        expected: &str,
        code: Code,
    ) -> Result<Spanned<String>, Box<Diagnostic>> {
        self.expect(keyword.clone())?;
        let literal = self.string()?;
        if literal.value != expected {
            return Err(self.failure(
                code,
                Phase::Profile,
                literal.span,
                format!("supported {} is {expected}", keyword.description()),
            ));
        }
        Ok(literal)
    }
    fn unit(&mut self) -> Result<ParsedUnit, Box<Diagnostic>> {
        let language = self.header(K::Language, LANGUAGE, Code::UnknownLanguage)?;
        let edition = self.header(K::Edition, EDITION, Code::UnknownEdition)?;
        self.expect(K::Semicolon)?;
        self.header(K::Profile, PROFILE, Code::UnknownProfile)?;
        self.expect(K::Semicolon)?;
        let mut imports = vec![self.model()?];
        while self.is(K::Model) {
            imports.push(self.model()?);
        }
        let mut clauses = vec![self.clause()?];
        while self.peek().kind != Kind::End {
            clauses.push(self.clause()?);
        }
        Ok(ParsedUnit {
            source: self.source.clone(),
            language,
            edition,
            imports,
            clauses,
            expressions: std::mem::take(&mut self.nodes),
        })
    }
    fn model(&mut self) -> Result<ModelImport, Box<Diagnostic>> {
        self.import(K::Model)
    }
    fn import(&mut self, keyword: K) -> Result<ModelImport, Box<Diagnostic>> {
        if self.composed {
            self.charge(self.peek().span)?;
        }
        let start = self.expect(keyword)?.span.start;
        let alias = self.identifier()?;
        self.expect(K::Equal)?;
        let package = self.string()?;
        self.expect(K::Version)?;
        let version = self.string()?;
        self.expect(K::Digest)?;
        let digest = self.string()?;
        let end = self.expect(K::Semicolon)?.span.end;
        Ok(ModelImport {
            alias,
            package,
            version,
            digest,
            span: Span { start, end },
        })
    }
    fn clause(&mut self) -> Result<Clause, Box<Diagnostic>> {
        let start = self.peek().span.start;
        let kind = match self.peek().kind {
            K::Invariant => ClauseKind::Invariant,
            K::Pre => ClauseKind::Precondition,
            K::Post => ClauseKind::Postcondition,
            _ => return Err(self.unexpected("invariant, pre or post clause")),
        };
        self.take();
        let name = self.identifier()?;
        self.expect(K::On)?;
        let model = self.identifier()?;
        self.expect(K::Qualify)?;
        let context = self.identifier()?;
        let operation = if kind == ClauseKind::Invariant {
            self.expect(K::At)?;
            self.expect(K::Current)?;
            None
        } else {
            self.expect(K::Qualify)?;
            Some(self.identifier()?)
        };
        self.expect(K::OpenBrace)?;
        let expression = self.expression()?;
        let end = self.expect(K::CloseBrace)?.span.end;
        Ok(Clause {
            kind,
            name,
            model,
            context,
            operation,
            expression,
            span: Span { start, end },
        })
    }
    fn enter(&mut self) -> Result<(), Box<Diagnostic>> {
        if self.depth >= self.limits.nesting {
            return Err(self.failure(
                Code::ResourceExhausted,
                Phase::Parse,
                self.peek().span,
                "expression nesting budget exhausted",
            ));
        }
        self.depth += 1;
        Ok(())
    }
    fn add(&mut self, kind: ExprKind, start: usize, end: usize) -> Result<ExprId, Box<Diagnostic>> {
        if self.composed {
            return self.add_value(c::ValueKind::Shared(kind), Span { start, end });
        }
        if self.nodes.len() >= self.limits.nodes {
            return Err(self.failure(
                Code::ResourceExhausted,
                Phase::Parse,
                Span { start, end },
                "syntax node budget exhausted",
            ));
        }
        let id = ExprId(self.nodes.len());
        self.nodes.push(Expr {
            kind,
            span: Span { start, end },
        });
        Ok(id)
    }
    fn expression(&mut self) -> Result<ExprId, Box<Diagnostic>> {
        self.enter()?;
        let result = self.expression_inner();
        self.depth -= 1;
        result
    }
    fn expression_inner(&mut self) -> Result<ExprId, Box<Diagnostic>> {
        let start = self.peek().span.start;
        if self.eat(K::Let) {
            let name = self.identifier()?;
            self.expect(K::Equal)?;
            let value = self.expression()?;
            self.expect(K::In)?;
            let body = self.expression()?;
            return self.add(
                ExprKind::Let { name, value, body },
                start,
                self.expression_span(body).end,
            );
        }
        if self.eat(K::If) {
            let condition = self.expression()?;
            self.expect(K::Then)?;
            let then_value = self.expression()?;
            self.expect(K::Else)?;
            let else_value = self.expression()?;
            return self.add(
                ExprKind::If {
                    condition,
                    then_value,
                    else_value,
                },
                start,
                self.expression_span(else_value).end,
            );
        }
        self.binary(0)
    }
    fn binary(&mut self, minimum: u8) -> Result<ExprId, Box<Diagnostic>> {
        self.enter()?;
        let result = self.binary_inner(minimum);
        self.depth -= 1;
        result
    }
    fn binary_inner(&mut self, minimum: u8) -> Result<ExprId, Box<Diagnostic>> {
        let mut left = self.unary()?;
        let mut comparison_seen = false;
        while let Some(op) = binary_op(&self.peek().kind).or_else(|| {
            (self.composed && matches!(self.peek().kind, K::Slash | K::Mod))
                .then_some(BinaryOp::Multiply)
        }) {
            if op.power() < minimum {
                break;
            }
            if op.comparison() && comparison_seen {
                return Err(self.unexpected("a non-chained comparison (use explicit parentheses)"));
            }
            // A conjunction starts a new comparison operand; a second relation
            // without it is deliberately invalid rather than a guessed chain.
            comparison_seen = op.comparison();
            let operator = self.take();
            let right_min = op.power() + u8::from(op != BinaryOp::Implies);
            let right = self.binary(right_min)?;
            let span = Span {
                start: self.expression_span(left).start,
                end: self.expression_span(right).end,
            };
            left = if self.composed && matches!(operator.kind, K::Slash | K::Mod) {
                self.add_value(
                    c::ValueKind::Product {
                        op: Spanned {
                            value: if operator.kind == K::Slash {
                                c::ProductOp::Slash
                            } else {
                                c::ProductOp::Mod
                            },
                            span: operator.span,
                        },
                        left,
                        right,
                    },
                    span,
                )?
            } else {
                self.add_operator(
                    ExprKind::Binary { op, left, right },
                    span.start,
                    span.end,
                    operator.span,
                )?
            };
        }
        Ok(left)
    }
    fn unary(&mut self) -> Result<ExprId, Box<Diagnostic>> {
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
        let mut argument = self.primary()?;
        while self.eat(K::Dot) {
            let name = self.member()?;
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
            argument = self.add_operator(
                ExprKind::Unary { op, argument },
                operator_span.start,
                self.expression_span(argument).end,
                operator_span,
            )?;
        }
        Ok(argument)
    }
    fn primary(&mut self) -> Result<ExprId, Box<Diagnostic>> {
        if self.composed {
            if let Some(value) = self.extended_primary()? {
                return Ok(value);
            }
        }
        let token = self.peek().clone();
        let start = token.span.start;
        let operator_span = matches!(
            token.kind,
            K::Present
                | K::Value
                | K::Deref
                | K::Size
                | K::Pre
                | K::Forall
                | K::Exists
                | K::Reaches
        )
        .then_some(token.span);
        let kind = match token.kind {
            Kind::Text(value) => {
                self.take();
                ExprKind::Text(value)
            }
            Kind::Integer(value) => {
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
                self.take();
                let inner = self.expression()?;
                self.expect(K::CloseParen)?;
                ExprKind::Group { inner }
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
                let argument = self.expression()?;
                self.expect(K::CloseParen)?;
                ExprKind::Call { builtin, argument }
            }
            keyword @ (K::Forall | K::Exists) => {
                self.take();
                self.expect(K::OpenParen)?;
                let name = self.identifier()?;
                self.expect(K::In)?;
                let domain = self.expression()?;
                self.expect(K::Colon)?;
                let predicate = self.expression()?;
                self.expect(K::CloseParen)?;
                ExprKind::Quantifier {
                    universal: keyword == K::Forall,
                    name,
                    domain,
                    predicate,
                }
            }
            K::Reaches => {
                self.take();
                self.expect(K::OpenParen)?;
                let first = self.expression()?;
                self.expect(K::Comma)?;
                let target = self.expression()?;
                self.expect(K::Comma)?;
                let field = self.member()?;
                self.expect(K::CloseParen)?;
                ExprKind::Reaches {
                    start: first,
                    target,
                    field,
                }
            }
            Kind::Identifier(model) => {
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
                    let name = self.member()?;
                    self.expect(K::Qualify)?;
                    let variant = self.member()?;
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
        let end = self.tokens[self.at - 1].span.end;
        let id = self.add(kind, start, end)?;
        if self.composed {
            self.values[id.0].operator_span = operator_span;
        }
        Ok(id)
    }
}

fn binary_op(kind: &Kind) -> Option<BinaryOp> {
    Some(match kind {
        K::Implies => BinaryOp::Implies,
        K::Or => BinaryOp::Or,
        K::And => BinaryOp::And,
        K::Equal => BinaryOp::Equal,
        K::NotEqual => BinaryOp::NotEqual,
        K::Less => BinaryOp::Less,
        K::LessEqual => BinaryOp::LessEqual,
        K::Greater => BinaryOp::Greater,
        K::GreaterEqual => BinaryOp::GreaterEqual,
        K::Plus => BinaryOp::Add,
        K::Minus => BinaryOp::Subtract,
        K::Star => BinaryOp::Multiply,
        K::Div => BinaryOp::Divide,
        K::Rem => BinaryOp::Remainder,
        _ => return None,
    })
}
