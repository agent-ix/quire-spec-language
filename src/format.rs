// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-003: format validated source by rewriting whitespace, preserving every token and
//! comment. Syntax formatting does not need a second expression printer/grammar.
use crate::{Limits, ParsedUnit};
use logos::Logos;
use qsl_cst::token::Kind;
use qsl_foundation::{Code, Diagnostic, Phase, Source, Span};

/// Format tokens/comments using the default 1 MiB output-byte ceiling.
pub fn format(unit: &ParsedUnit) -> Result<String, Box<Diagnostic>> {
    format_with_limit(unit, Limits::default().source_bytes)
}

/// Format with an inclusive selected byte ceiling, clamped to 1 MiB.
/// Every append is checked before growth; failure returns no partial string.
pub fn format_with_limit(
    unit: &ParsedUnit,
    output_bytes: usize,
) -> Result<String, Box<Diagnostic>> {
    let mut output = Output {
        text: String::new(),
        source: unit.source(),
        limit: output_bytes.min(Limits::default().source_bytes),
    };
    let mut previous: Option<Kind> = None;
    let mut indent = 0;
    for (token, range) in Kind::lexer(unit.source().text()).spanned() {
        let kind = token.expect("ParsedUnit contains validated source");
        let spelling = &unit.source().text()[range.clone()];
        let span = Span {
            start: range.start,
            end: range.end,
        };
        if kind == Kind::CloseBrace {
            indent -= 1;
            output.newline(span)?;
        }
        if output.text.ends_with('\n') && indent > 0 {
            output.push("  ", span)?;
        }
        let no_space = output.text.is_empty()
            || output.text.ends_with([' ', '\n'])
            || matches!(
                kind,
                Kind::Dot | Kind::Qualify | Kind::CloseParen | Kind::Comma | Kind::Semicolon
            )
            || matches!(previous, Some(Kind::Dot | Kind::Qualify | Kind::OpenParen))
            || (kind == Kind::OpenParen
                && matches!(
                    previous,
                    Some(
                        Kind::Present
                            | Kind::Value
                            | Kind::Deref
                            | Kind::Size
                            | Kind::Pre
                            | Kind::Forall
                            | Kind::Exists
                            | Kind::Reaches
                    )
                ));
        if !no_space {
            output.push(" ", span)?;
        }
        output.push(spelling, span)?;
        match kind {
            Kind::OpenBrace => {
                indent += 1;
                output.newline(span)?;
            }
            Kind::CloseBrace | Kind::Semicolon | Kind::Comment => output.newline(span)?,
            _ => {}
        }
        previous = Some(kind);
    }
    let end = unit.source().text().len();
    output.newline(Span { start: end, end })?;
    Ok(output.text)
}

struct Output<'a> {
    text: String,
    source: &'a Source,
    limit: usize,
}

impl Output<'_> {
    fn push(&mut self, text: &str, span: Span) -> Result<(), Box<Diagnostic>> {
        if self
            .text
            .len()
            .checked_add(text.len())
            .is_none_or(|length| length > self.limit)
        {
            return Err(qsl_foundation::diagnostic::error(
                self.source,
                Code::ResourceExhausted,
                Phase::Format,
                span.start,
                span.end,
                "formatted source byte budget exhausted",
            ));
        }
        self.text.push_str(text);
        Ok(())
    }

    fn newline(&mut self, span: Span) -> Result<(), Box<Diagnostic>> {
        if !self.text.ends_with('\n') {
            self.push("\n", span)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn spellings(text: &str) -> Vec<&str> {
        Kind::lexer(text)
            .spanned()
            .map(|(token, span)| {
                token.unwrap();
                &text[span]
            })
            .collect()
    }

    #[trace("TC-013", "FR-003-AC-1", "FR-003-AC-2", "FR-003-AC-3")]
    #[test]
    fn formatter_preserves_ordered_token_and_comment_spellings() {
        for expression in [
            "let p = self.parent in if present(p) then deref(value(p)).n > 0 else true",
            "forall(x in self.items: exists(y in self.items: x = y))",
            "not reaches(self, self, parent)",
            "M::Color::Red = M::Color::Blue",
            "-7 rem 3 = -1 and 7 div 3 = 2",
            "result = pre(self.n)",
            "\"caf\\u00e9\\n\\uD83D\\uDE00\" = \"café\\n😀\"",
            "not not false",
            "(if true then 2 else 3) + (let x = 4 in x)",
            "true // retained café comment  \n and false",
        ] {
            let source = format!(
                "language \"ix:native\" edition \"0-draft\";\nprofile \"state-finite/0-draft\";\nmodel M = \"test/model\" version \"1\" digest \"unresolved\";\ninvariant Test on M::Thing at current {{ {expression} }}\n"
            );
            let unit = crate::parse(
                qsl_foundation::SourceIdentity {
                    identity: "test:format".into(),
                    revision: "1".into(),
                },
                "format.native",
                source.as_bytes(),
                Limits::default(),
            )
            .unwrap();
            let formatted = format(&unit).unwrap();
            assert_eq!(spellings(&source), spellings(&formatted));
            let reparsed = crate::parse(
                unit.source().identity().clone(),
                "format.native",
                formatted.as_bytes(),
                Limits::default(),
            )
            .unwrap();
            assert_eq!(format(&reparsed).unwrap(), formatted);
        }
    }
}
