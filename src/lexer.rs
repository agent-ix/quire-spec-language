// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-002: apply token budgets and delimiter checks to the generated recognizer.
use crate::diagnostic::error;
use crate::syntax::Limits;
pub(crate) use crate::token::Kind;
use crate::token::LexError;
use crate::{Code, Diagnostic, Phase, Source, Span};
use logos::Logos;

#[derive(Clone, Debug)]
pub(crate) struct Token {
    pub kind: Kind,
    pub span: Span,
}

pub(crate) fn lex(source: &Source, limits: Limits) -> Result<Vec<Token>, Box<Diagnostic>> {
    let mut tokens = recognize(source, limits)?;
    for token in &mut tokens {
        token.kind = token
            .kind
            .clone()
            .historical(source.slice(token.span).expect("token span"));
    }
    Ok(tokens)
}

pub(crate) fn recognize(source: &Source, limits: Limits) -> Result<Vec<Token>, Box<Diagnostic>> {
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
