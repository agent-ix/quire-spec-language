// SPDX-License-Identifier: AGPL-3.0-only
use crate::lexer::{self, Kind, Token};
use crate::syntax::*;
use crate::token::Kind as K;
use crate::{Code, Diagnostic, LocatedSpan, Phase, Position, Source, SourceIdentity, Span};

/// Parse the selected native grammar. Model imports stay unresolved here.
pub fn parse(
    identity: SourceIdentity,
    path: impl Into<String>,
    bytes: &[u8],
    limits: Limits,
) -> Result<ParsedUnit, Box<Diagnostic>> {
    let path = path.into();
    let limits = limits.bounded();
    let point = Position {
        byte: 0,
        line: 1,
        column: 1,
    };
    let refusal = |code, message: &str| {
        Box::new(Diagnostic {
            phase: Phase::Source,
            code,
            source: identity.clone(),
            path: path.clone(),
            span: LocatedSpan {
                start: point,
                end: point,
            },
            message: message.into(),
        })
    };
    if identity.identity.trim().is_empty() || identity.revision.trim().is_empty() || path.is_empty()
    {
        return Err(refusal(
            Code::InvalidSourceIdentity,
            "source identity, revision and path must be explicit",
        ));
    }
    if bytes.len() > limits.source_bytes {
        return Err(refusal(
            Code::ResourceExhausted,
            "source byte budget exhausted",
        ));
    }
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => {
            let mut diagnostic = refusal(Code::InvalidUtf8, "source must be valid UTF-8");
            let prefix =
                std::str::from_utf8(&bytes[..error.valid_up_to()]).expect("UTF-8 valid prefix");
            let source = Source::new(identity.clone(), path.clone(), prefix);
            diagnostic.span = source
                .locate(Span {
                    start: prefix.len(),
                    end: prefix.len(),
                })
                .expect("prefix EOF");
            return Err(diagnostic);
        }
    };
    let source = Source::new(identity, path, text);
    if let Some(at) = text.find('\0') {
        return Err(lexer::error(
            &source,
            Code::InvalidSyntax,
            Phase::Source,
            at,
            at + 1,
            "NUL is forbidden in source bytes",
        ));
    }
    let tokens = lexer::lex(&source, limits)?;
    let mut parser = Parser {
        source,
        tokens,
        at: 0,
        nodes: Vec::new(),
        depth: 0,
        limits,
    };
    parser.unit()
}

struct Parser {
    source: Source,
    tokens: Vec<Token>,
    at: usize,
    nodes: Vec<Expr>,
    depth: usize,
    limits: Limits,
}

impl Parser {
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
        if matches!(
            self.peek().kind,
            Kind::Unsupported | Kind::Fractional | Kind::Slash | Kind::OpenBracket
        ) {
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
    fn identifier(&mut self) -> Result<String, Box<Diagnostic>> {
        match &self.peek().kind {
            Kind::Identifier(word) => {
                let value = word.clone();
                self.take();
                Ok(value)
            }
            _ => Err(self.unexpected("identifier")),
        }
    }
    fn string(&mut self) -> Result<(String, Span), Box<Diagnostic>> {
        if let Kind::Text(value) = &self.peek().kind {
            let result = (value.clone(), self.peek().span);
            self.take();
            Ok(result)
        } else {
            Err(self.unexpected("quoted string"))
        }
    }
    fn header(&mut self, keyword: K, expected: &str, code: Code) -> Result<(), Box<Diagnostic>> {
        self.expect(keyword.clone())?;
        let (value, span) = self.string()?;
        if value != expected {
            return Err(self.failure(
                code,
                Phase::Profile,
                span,
                format!("supported {} is {expected}", keyword.description()),
            ));
        }
        Ok(())
    }
    fn unit(&mut self) -> Result<ParsedUnit, Box<Diagnostic>> {
        self.header(K::Language, LANGUAGE, Code::UnknownLanguage)?;
        self.header(K::Edition, EDITION, Code::UnknownEdition)?;
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
            imports,
            clauses,
            expressions: std::mem::take(&mut self.nodes),
        })
    }
    fn model(&mut self) -> Result<ModelImport, Box<Diagnostic>> {
        let start = self.expect(K::Model)?.span.start;
        let alias = self.identifier()?;
        self.expect(K::Equal)?;
        let (package, _) = self.string()?;
        self.expect(K::Version)?;
        let (version, _) = self.string()?;
        self.expect(K::Digest)?;
        let (digest, _) = self.string()?;
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
                self.nodes[body.0].span.end,
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
                self.nodes[else_value.0].span.end,
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
        while let Some(op) = binary_op(&self.peek().kind) {
            if op.power() < minimum {
                break;
            }
            if op.comparison() && comparison_seen {
                return Err(self.unexpected("a non-chained comparison (use explicit parentheses)"));
            }
            // A conjunction starts a new comparison operand; a second relation
            // without it is deliberately invalid rather than a guessed chain.
            comparison_seen = op.comparison();
            self.take();
            let right_min = op.power() + u8::from(op != BinaryOp::Implies);
            let right = self.binary(right_min)?;
            left = self.add(
                ExprKind::Binary { op, left, right },
                self.nodes[left.0].span.start,
                self.nodes[right.0].span.end,
            )?;
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
            operators.push((op, token.span.start));
        }
        let mut argument = self.primary()?;
        while self.eat(K::Dot) {
            let name = self.identifier()?;
            let end = self.tokens[self.at - 1].span.end;
            argument = self.add(
                ExprKind::Field {
                    base: argument,
                    name,
                },
                self.nodes[argument.0].span.start,
                end,
            )?;
        }
        for (op, start) in operators.into_iter().rev() {
            argument = self.add(
                ExprKind::Unary { op, argument },
                start,
                self.nodes[argument.0].span.end,
            )?;
        }
        Ok(argument)
    }
    fn primary(&mut self) -> Result<ExprId, Box<Diagnostic>> {
        let token = self.peek().clone();
        let start = token.span.start;
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
                let field = self.identifier()?;
                self.expect(K::CloseParen)?;
                ExprKind::Reaches {
                    start: first,
                    target,
                    field,
                }
            }
            Kind::Identifier(model) => {
                self.take();
                if self.is(K::OpenParen) {
                    return Err(self.failure(
                        Code::UnsupportedConstruct,
                        Phase::Profile,
                        token.span,
                        "user helper calls are unsupported by this profile",
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
        let end = self.tokens[self.at - 1].span.end;
        self.add(kind, start, end)
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
