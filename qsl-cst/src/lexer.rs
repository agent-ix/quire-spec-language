// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-002: apply token budgets and delimiter checks to the generated recognizer.
// Widened to `pub`: the root crate's own base-grammar `parser` imports
// `Kind` through this re-export across the crate boundary (ADR-011 §7.3
// X-3).
pub use crate::token::Kind;
use crate::token::LexError;
use logos::Logos;
use qsl_foundation::diagnostic::error;
use qsl_foundation::{Code, Diagnostic, Phase, Source, Span};

/// Layer-1 parse limits: the lexer's own recognizer bounds and the
/// complete-V1 CST bounds built on it. Caller limits may lower these
/// ceilings, never disable them. Equality compares the requested capacities,
/// so a resource-only configuration change remains visible in retained build
/// provenance.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Limits {
    /// Inclusive input-content ceiling, clamped to 1 MiB.
    pub source_bytes: usize,
    /// Maximum tokens the lexer's recognizer admits, and the retained CST
    /// leaf ceiling for complete-V1 parsing. Clamped to 100,000.
    pub tokens: usize,
    /// Maximum CST nodes, clamped to 50,000: complete-V1 parsing counts one
    /// node per matched grammar production.
    pub nodes: usize,
    /// Maximum delimiter or recursive parser nesting, clamped to 64.
    pub nesting: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            source_bytes: qsl_foundation::source::MAX_SOURCE_BYTES,
            tokens: 100_000,
            nodes: 50_000,
            nesting: 64,
        }
    }
}

impl Limits {
    /// Clamp every field to its hard ceiling, never raising it.
    // Widened to `pub`: the root crate's `complete::parse_with_catalog` calls
    // it across the crate boundary (ADR-011 §7.3 X-3).
    pub fn bounded(self) -> Self {
        let hard = Self::default();
        Self {
            source_bytes: self.source_bytes.min(hard.source_bytes),
            tokens: self.tokens.min(hard.tokens),
            nodes: self.nodes.min(hard.nodes),
            nesting: self.nesting.min(hard.nesting),
        }
    }
}

// `Token` and `lex`/`recognize` are widened to `pub`: the root crate's own
// base-grammar `parser` calls all three across the crate boundary
// (ADR-011 §7.3 X-3).
/// One recognized token and its exact source span.
#[derive(Clone, Debug)]
pub struct Token {
    /// Recognized token kind.
    pub kind: Kind,
    /// Exact half-open source span.
    pub span: Span,
}

/// Recognize `source` and reclassify each token under the historical
/// `0-draft` grammar's reservation set.
pub fn lex(source: &Source, limits: Limits) -> Result<Vec<Token>, Box<Diagnostic>> {
    let mut tokens = recognize(source, limits)?;
    for token in &mut tokens {
        token.kind = token
            .kind
            .clone()
            .historical(source.slice(token.span).expect("token span"));
    }
    Ok(tokens)
}

/// Recognize `source` into tokens under `limits`' recognizer bounds, without
/// the historical edition's reclassification.
pub fn recognize(source: &Source, limits: Limits) -> Result<Vec<Token>, Box<Diagnostic>> {
    let mut tokens = Vec::new();
    let mut delimiters = Vec::new();
    for (result, range) in Kind::lexer(source.text()).spanned() {
        let span = Span {
            start: range.start,
            end: range.end,
        };
        let kind = result.map_err(|reason| {
            error(
                source,
                Code::InvalidSyntax,
                Phase::Lex,
                span.start,
                span.end,
                match reason {
                    LexError::Character => "unexpected source character or unterminated string",
                    LexError::LeadingZero => "integer literals cannot have leading zeros",
                    LexError::String => "invalid JSON string",
                },
            )
        })?;
        // These lexical forms exist only in the separately selected complete
        // grammar. The historical/composed recognizer must retain its prior
        // invalid-syntax result rather than reclassifying them as unsupported.
        if matches!(
            kind,
            Kind::Caret | Kind::Question | Kind::Apostrophe | Kind::Hex(_)
        ) {
            return Err(error(
                source,
                Code::InvalidSyntax,
                Phase::Lex,
                span.start,
                span.end,
                "unexpected source character",
            ));
        }
        if kind == Kind::Comment {
            continue;
        }
        if tokens.len() >= limits.tokens {
            return Err(error(
                source,
                Code::ResourceExhausted,
                Phase::Lex,
                span.start,
                span.end,
                "token budget exhausted",
            ));
        }
        if kind == Kind::BadExponent {
            return Err(error(
                source,
                Code::InvalidSyntax,
                Phase::Lex,
                span.start,
                span.end,
                "numeric exponent has no digits",
            ));
        }
        // This profile distinguishes malformed delimiters from balanced reserved
        // forms, so check structure before the parser reports unsupported syntax.
        match kind {
            Kind::OpenParen | Kind::OpenBrace | Kind::OpenBracket => {
                if delimiters.len() >= limits.nesting {
                    return Err(error(
                        source,
                        Code::ResourceExhausted,
                        Phase::Lex,
                        span.start,
                        span.end,
                        "delimiter nesting budget exhausted",
                    ));
                }
                delimiters.push((kind.clone(), span));
            }
            Kind::CloseParen | Kind::CloseBrace | Kind::CloseBracket => {
                let expected = match kind {
                    Kind::CloseParen => Kind::OpenParen,
                    Kind::CloseBrace => Kind::OpenBrace,
                    _ => Kind::OpenBracket,
                };
                if delimiters.pop().is_none_or(|(open, _)| open != expected) {
                    return Err(error(
                        source,
                        Code::InvalidSyntax,
                        Phase::Lex,
                        span.start,
                        span.end,
                        "mismatched closing delimiter",
                    ));
                }
            }
            _ => {}
        }
        tokens.push(Token { kind, span });
    }
    if let Some((_, span)) = delimiters.pop() {
        return Err(error(
            source,
            Code::InvalidSyntax,
            Phase::Lex,
            span.start,
            span.end,
            "unclosed delimiter",
        ));
    }
    let end = source.text().len();
    tokens.push(Token {
        kind: Kind::End,
        span: Span { start: end, end },
    });
    Ok(tokens)
}
