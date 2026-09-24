// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-002: apply token budgets and delimiter checks to the generated recognizer.
// Kept crate-private: `token::Kind` is `qsl_cst::token::Kind`'s own public
// path; the root crate's base-grammar `parser` imports it from there
// directly, not through this re-export (QSL-178 review F6).
use crate::token::Kind;
use crate::token::LexError;
use logos::Logos;
use qsl_foundation::diagnostic::{error, resource_exhausted};
use qsl_foundation::{Code, Diagnostic, Phase, Source, Span, SyntaxLimit};

/// Layer-1 parse limits: the lexer's own recognizer bounds and the
/// complete-V1 CST bounds built on it. A caller-supplied ceiling is used as
/// given (an implementation ceiling is not a domain bound, NFR-001) --
/// [`Self::default`] is the fail-closed starting point a caller who supplies
/// none gets, never an upper clamp on what a caller may ask for (ADR-011
/// §7.3, QSL-199). The effective ceiling actually used for one parse is
/// recorded on its [`crate::ParsedSource::effective_limits`]. Equality
/// compares the requested capacities, so a resource-only configuration
/// change remains visible in retained build provenance.
///
/// `source_bytes` is the one exception: `qsl_foundation::source::MAX_SOURCE_BYTES`
/// is foundation layer's own hard ceiling on the byte read itself
/// (`Source::read_typed`), independent of this type and out of QSL-199's
/// scope, so a `source_bytes` raised past it has no effect on what the
/// actual read admits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Limits {
    /// Inclusive input-content ceiling. Defaults to 1 MiB; see this type's
    /// own doc for why raising it past `qsl_foundation::source::MAX_SOURCE_BYTES`
    /// has no effect on the underlying read.
    pub source_bytes: usize,
    /// Maximum tokens the lexer's recognizer admits, and the retained CST
    /// leaf ceiling for complete-V1 parsing. Defaults to 100,000.
    pub tokens: usize,
    /// Maximum CST nodes. Defaults to 50,000: complete-V1 parsing counts one
    /// node per matched grammar production.
    pub nodes: usize,
    /// Maximum bracket-pair nesting depth (NFR-001 "Nesting level"): one
    /// level is one `(…)`, `[…]`, `{…}` or type-argument `<…>` pair;
    /// operator, prefix, `let … in` and `if … else` chains add none. The
    /// default is NFR-001's 64. A ceiling the implementation imposes on a
    /// larger request is not a domain bound.
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
    /// The effective limits: exactly what the caller supplied. Kept as a
    /// named step (rather than removed outright) so every existing call site
    /// stays a one-line, self-describing "this is the ceiling actually in
    /// force" marker; it no longer clamps a caller-supplied ceiling down to
    /// [`Self::default`] (ADR-011 §7.3, QSL-199 -- an implementation ceiling
    /// is not a domain bound, NFR-001).
    pub fn bounded(self) -> Self {
        self
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
            return Err(resource_exhausted(
                source,
                Phase::Lex,
                span,
                SyntaxLimit::Tokens {
                    bound: limits.tokens,
                },
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
                    // The opening bracket of pair `nesting + 1`, at its own
                    // span. This check covers `(`/`[`/`{`; the native parser
                    // charges a composed type-argument `<` itself.
                    return Err(resource_exhausted(
                        source,
                        Phase::Lex,
                        span,
                        SyntaxLimit::NestingDepth {
                            bound: limits.nesting,
                        },
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
