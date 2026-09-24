// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-002: bounded declarations and precedence parsing into flat located syntax.
use crate::syntax::*;
use qsl_cst::lexer::{self, Token};
use qsl_cst::token::{Kind, Kind as K};
use qsl_foundation::{Code, Diagnostic, Phase, Source, SourceIdentity, Span, Spanned, SyntaxLimit};

mod composed;
mod expression;
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
        return Err(qsl_foundation::diagnostic::error(
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
        return Err(qsl_foundation::diagnostic::error(
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
            limit: None,
        })
    }
    /// Refuse at `span` because the next operation would exceed `limit`.
    fn exhausted(&self, span: Span, limit: SyntaxLimit) -> Box<Diagnostic> {
        qsl_foundation::diagnostic::resource_exhausted(&self.source, Phase::Parse, span, limit)
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
    /// Match a nesting-level-opening bracket -- `(`, `[`, `{`, or a
    /// type-argument `<` (composed.rs) -- charging nesting depth exactly
    /// once per bracket pair (NFR-001 "Nesting level"), never per
    /// `expression()`/`binary()` call. This parser is fully predictive (no
    /// PEG backtracking that could try, fail, and retry a bracket from the
    /// same position), so a plain mutable `self.depth` counter is sound.
    fn open(&mut self, expected: K) -> Result<Token, Box<Diagnostic>> {
        let token = self.expect(expected)?;
        self.open_taken(token.span)?;
        Ok(token)
    }
    /// Charge nesting depth for an opening bracket already consumed via
    /// `self.take()` before its kind was known (e.g. a lookahead `match` on
    /// a token fetched up front). `span` is that bracket's own span.
    fn open_taken(&mut self, span: Span) -> Result<(), Box<Diagnostic>> {
        if self.depth >= self.limits.nesting {
            return Err(self.exhausted(
                span,
                SyntaxLimit::NestingDepth {
                    bound: self.limits.nesting,
                },
            ));
        }
        self.depth += 1;
        Ok(())
    }
    /// The `close` counterpart of [`Self::open`].
    fn close(&mut self, expected: K) -> Result<Token, Box<Diagnostic>> {
        let token = self.expect(expected)?;
        self.depth = self.depth.saturating_sub(1);
        Ok(token)
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
        self.open(K::OpenBrace)?;
        let expression = self.expression()?;
        let end = self.close(K::CloseBrace)?.span.end;
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
    fn add(&mut self, kind: ExprKind, start: usize, end: usize) -> Result<ExprId, Box<Diagnostic>> {
        if self.composed {
            return self.add_value(c::ValueKind::Shared(kind), Span { start, end });
        }
        if self.nodes.len() >= self.limits.nodes {
            return Err(self.exhausted(
                Span { start, end },
                SyntaxLimit::Nodes {
                    bound: self.limits.nodes,
                },
            ));
        }
        let id = ExprId(self.nodes.len());
        self.nodes.push(Expr {
            kind,
            span: Span { start, end },
        });
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

#[cfg(test)]
mod tests {
    use super::*;

    fn source(text: &str) -> Source {
        Source::read(
            SourceIdentity {
                identity: "test:raised".into(),
                revision: "1".into(),
            },
            "raised.native",
            text.as_bytes(),
            qsl_foundation::source::MAX_SOURCE_BYTES,
        )
        .expect("test source")
    }

    // A caller may raise the nesting ceiling far past the default; brackets
    // nested that deep still never recurse. `Parser` here sits below the
    // public clamp in `Limits::bounded`.
    #[test]
    fn brackets_under_a_raised_nesting_ceiling_never_overflow_the_stack() {
        std::thread::Builder::new()
            .stack_size(512 * 1024)
            .spawn(|| {
                let limits = Limits {
                    nesting: 100_000,
                    ..Limits::default()
                };
                let depth = 30_000;
                let historical = format!(
                    "language \"ix:native\" edition \"0-draft\";\nprofile \"state-finite/0-draft\";\nmodel M = \"test/model\" version \"1\" digest \"unresolved\";\ninvariant T on M::Thing at current {{ {}1{} }}\n",
                    "(".repeat(depth),
                    ")".repeat(depth)
                );
                let text = source(&historical);
                let tokens = lexer::lex(&text, lexer_limits(limits)).expect("lexes");
                let unit = Parser::new(text, tokens, limits).unit().expect("parses");
                // One group per pair around the literal.
                assert_eq!(unit.expressions.len(), depth + 1);

                let composed = format!(
                    "language \"ix:native\" edition \"1-draft\";\nprofile T = \"quire.temporal.timestamped-event.finite-window/v1\" version \"t\" digest \"u\";\nmodel M = \"m\" version \"1\" digest \"u\";\ntemporal W using T over (v: M::V) clock \"c\" on origin {{ {}holds({}v{}){} }}\n",
                    "(".repeat(depth / 2),
                    "(".repeat(depth / 4),
                    ")".repeat(depth / 4),
                    ")".repeat(depth / 2)
                );
                let text = source(&composed);
                let tokens = lexer::recognize(&text, lexer_limits(limits)).expect("lexes");
                Parser::new(text, tokens, limits)
                    .native_unit()
                    .expect("parses");
            })
            .expect("spawn a 512 KiB thread")
            .join()
            .expect("the parser must not overflow a 512 KiB stack");
    }
}
