// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-003: format validated complete-V1 source over the S1 lossless CST
//! (ADR-011 §6.1 tool layer, depending on layer 1 and F only; §6.2 `format`
//! row). Formatting rewrites whitespace between the CST's own tokens and
//! keeps every token spelling and comment in order. It admits only a
//! `ParsedSource` that `is_admissible()`; a recovering or diagnosed parse
//! refuses with a typed cause and no output.
//!
//! `complete::editor::format_document` calls [`format_with_limit`].
use qsl_cst::{
    CompleteCause, CompleteCode, CompleteDiagnostic, CstToken, ParsedSource, TokenClass, TokenKind,
};
use qsl_foundation::source::MAX_SOURCE_BYTES;
use qsl_foundation::{Phase, Span};

/// The implementation's output-byte ceiling: 1 MiB. [`format()`] applies it,
/// and [`format_with_limit`] clamps every selected ceiling to it.
pub const OUTPUT_BYTE_CEILING: usize = MAX_SOURCE_BYTES;

/// Why `format` returned no output. Each arm holds the diagnostic that
/// carries its code, cause and source location.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FormatRefusal {
    /// The CST carries a recovery. The diagnostic is the input's first
    /// diagnostic, whose code the refusal keeps.
    RecoveringCst(Box<CompleteDiagnostic>),
    /// The parse carries a diagnostic and no recovery, for example a
    /// profile refusal added through `ParsedSource::prepend_diagnostic`.
    /// The diagnostic is the input's first diagnostic.
    DiagnosedSource(Box<CompleteDiagnostic>),
    /// The formatted text would exceed the selected output-byte ceiling
    /// (`resource_exhausted`).
    OutputBudgetExhausted(Box<CompleteDiagnostic>),
    /// An established S1 invariant does not hold: a recovery with no
    /// diagnostic, or a token span that is not a UTF-8 slice of the source
    /// (`runtime_invariant`).
    BrokenInvariant(Box<CompleteDiagnostic>),
}

impl FormatRefusal {
    /// The diagnostic this refusal carries.
    pub fn diagnostic(&self) -> &CompleteDiagnostic {
        match self {
            Self::RecoveringCst(diagnostic)
            | Self::DiagnosedSource(diagnostic)
            | Self::OutputBudgetExhausted(diagnostic)
            | Self::BrokenInvariant(diagnostic) => diagnostic,
        }
    }

    /// The refusal's catalog code.
    pub fn code(&self) -> CompleteCode {
        self.diagnostic().code
    }

    /// The diagnostic this refusal carries, by value.
    pub fn into_diagnostic(self) -> Box<CompleteDiagnostic> {
        match self {
            Self::RecoveringCst(diagnostic)
            | Self::DiagnosedSource(diagnostic)
            | Self::OutputBudgetExhausted(diagnostic)
            | Self::BrokenInvariant(diagnostic) => diagnostic,
        }
    }
}

impl std::fmt::Display for FormatRefusal {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.diagnostic().fmt(formatter)
    }
}

impl std::error::Error for FormatRefusal {}

/// Format admissible complete-V1 source under the 1 MiB output ceiling.
pub fn format(parsed: &ParsedSource) -> Result<String, FormatRefusal> {
    format_with_limit(parsed, OUTPUT_BYTE_CEILING)
}

/// Format admissible complete-V1 source under an inclusive selected
/// output-byte ceiling, clamped to [`OUTPUT_BYTE_CEILING`]. Every append is
/// checked before it grows the output, and a refusal returns no partial
/// string.
pub fn format_with_limit(
    parsed: &ParsedSource,
    output_bytes: usize,
) -> Result<String, FormatRefusal> {
    admit(parsed)?;
    let mut output = Output {
        parsed,
        text: String::new(),
        limit: output_bytes.min(OUTPUT_BYTE_CEILING),
    };
    let mut indent = 0_usize;
    let mut previous: Option<&CstToken> = None;
    let mut closed_declaration = false;
    for token in parsed
        .cst()
        .tokens()
        .iter()
        .filter(|token| token.class() != TokenClass::Whitespace)
    {
        let span = token.span();
        let text = output.token_text(token)?;
        if token.class() == TokenClass::Comment {
            output.begin_line(indent, span)?;
            // A comment never touches the previous token: `/` then `// c`
            // would otherwise lex as the comment `/// c`.
            if !output.text.is_empty() && !output.text.ends_with([' ', '\n']) {
                output.push(" ", span)?;
            }
            output.push(text, span)?;
            output.newline(span)?;
            previous = None;
            continue;
        }
        let spelling = token.spelling();
        if spelling == b"}" {
            indent = indent.saturating_sub(1);
            output.newline(span)?;
            output.begin_line(indent, span)?;
        } else if closed_declaration && !completes_block(spelling) {
            output.newline(span)?;
            output.begin_line(indent, span)?;
        } else if output.text.ends_with('\n') {
            output.begin_line(indent, span)?;
        }
        if needs_space(previous, token, &output.text) {
            output.push(" ", span)?;
        }
        output.push(text, span)?;
        if spelling == b"{" {
            indent = indent.saturating_add(1);
            output.newline(span)?;
        } else if spelling == b";" {
            output.newline(span)?;
        }
        closed_declaration = spelling == b"}" && indent == 0;
        previous = Some(token);
    }
    if !output.text.ends_with('\n') {
        let end = parsed.source().text().len();
        output.push("\n", Span { start: end, end })?;
    }
    Ok(output.text)
}

/// Admit only an unrecovered, diagnostic-free parse (FR-003 Behavior).
fn admit(parsed: &ParsedSource) -> Result<(), FormatRefusal> {
    let recovering = !parsed.cst().recoveries().is_empty();
    match (parsed.diagnostics().first(), recovering) {
        (None, false) => Ok(()),
        (Some(first), true) => Err(FormatRefusal::RecoveringCst(Box::new(first.clone()))),
        (Some(first), false) => Err(FormatRefusal::DiagnosedSource(Box::new(first.clone()))),
        (None, true) => Err(FormatRefusal::BrokenInvariant(whole_source_error(
            parsed,
            CompleteCode::RuntimeInvariant,
            CompleteCause::EstablishedInvariantBroken,
            "the CST carries a recovery with no diagnostic",
        ))),
    }
}

fn whole_source_error(
    parsed: &ParsedSource,
    code: CompleteCode,
    cause: CompleteCause,
    message: &str,
) -> Box<CompleteDiagnostic> {
    qsl_cst::diagnostic::error(
        parsed.source(),
        code,
        cause,
        Phase::Format,
        0,
        parsed.source().text().len(),
        message,
    )
}

/// Whether a token completes the braced construct before it and so stays
/// on the line of a declaration-level closing brace.
fn completes_block(current: &[u8]) -> bool {
    matches!(current, b";" | b"," | b")" | b"]")
}

/// Whether a space separates `current` from the previous significant token.
fn needs_space(previous: Option<&CstToken>, current: &CstToken, output: &str) -> bool {
    let Some(previous) = previous else {
        return false;
    };
    if output.ends_with([' ', '\n']) {
        return false;
    }
    if previous.span().end == current.span().start
        && matches!(
            (previous.kind(), current.kind()),
            (
                TokenKind::HexPrefix | TokenKind::HexDigit,
                TokenKind::HexDigit
            )
        )
    {
        return false;
    }
    if matches!(
        current.spelling(),
        b")" | b"]" | b"," | b";" | b"." | b"::" | b"?"
    ) {
        return false;
    }
    !matches!(previous.spelling(), b"(" | b"[" | b"." | b"::" | b"^")
}

/// The output under construction and its inclusive byte ceiling.
struct Output<'a> {
    parsed: &'a ParsedSource,
    text: String,
    limit: usize,
}

impl<'a> Output<'a> {
    /// A token's exact text, sliced from the source by the token's span.
    fn token_text(&self, token: &CstToken) -> Result<&'a str, FormatRefusal> {
        let span = token.span();
        self.parsed
            .source()
            .text()
            .get(span.start..span.end)
            .ok_or_else(|| {
                FormatRefusal::BrokenInvariant(whole_source_error(
                    self.parsed,
                    CompleteCode::RuntimeInvariant,
                    CompleteCause::EstablishedInvariantBroken,
                    "a lossless CST token span is not a UTF-8 slice of its source",
                ))
            })
    }

    fn push(&mut self, value: &str, span: Span) -> Result<(), FormatRefusal> {
        if self
            .text
            .len()
            .checked_add(value.len())
            .is_none_or(|length| length > self.limit)
        {
            return Err(FormatRefusal::OutputBudgetExhausted(
                qsl_cst::diagnostic::error(
                    self.parsed.source(),
                    CompleteCode::ResourceExhausted,
                    CompleteCause::InsufficientNextCharge,
                    Phase::Format,
                    span.start,
                    span.end,
                    "formatted output byte budget exhausted",
                ),
            ));
        }
        self.text.push_str(value);
        Ok(())
    }

    fn begin_line(&mut self, indent: usize, span: Span) -> Result<(), FormatRefusal> {
        if self.text.is_empty() || self.text.ends_with('\n') {
            for _ in 0..indent {
                self.push("  ", span)?;
            }
        }
        Ok(())
    }

    /// End the current line. A comment's own trailing spaces are part of its
    /// spelling and stay.
    fn newline(&mut self, span: Span) -> Result<(), FormatRefusal> {
        if !self.text.is_empty() && !self.text.ends_with('\n') {
            self.push("\n", span)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use qsl_foundation::SourceIdentity;

    fn parse(text: &str) -> ParsedSource {
        qsl_cst::parse(
            SourceIdentity {
                identity: "test:format".into(),
                revision: "1".into(),
            },
            "format.quire",
            text.as_bytes(),
            qsl_cst::Limits::default(),
        )
        .expect("the source reads")
    }

    fn significant(parsed: &ParsedSource) -> Vec<(TokenClass, Vec<u8>)> {
        parsed
            .cst()
            .tokens()
            .iter()
            .filter(|token| token.class() != TokenClass::Whitespace)
            .map(|token| (token.class(), token.spelling().to_vec()))
            .collect()
    }

    const HEADER: &str = concat!(
        "language \"ix:native\" edition \"1-draft\";\n",
        "profile v = \"quire.value.complete/v1\" version \"1\" digest \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n",
    );

    #[trace("TC-013", "FR-003-AC-1", "FR-003-AC-2", "FR-003-AC-3")]
    #[test]
    fn formatter_preserves_ordered_token_and_comment_spellings() {
        for declarations in [
            "function f using v(x: Int[0, 9]): Int[0, 10] pure { let y = x in if y > 0 then y + 1 else 0 }",
            "function g using v(xs: Sequence<Int[0, 9]>[0, 4]): Boolean pure { forall(x in xs: exists(y in xs: x = y)) }",
            "record Point { x: Int[0, 9]; y: Text[0, 4; nfc]?; }\ntuple Pair(Int[0, 9], Boolean);",
            "type Digit = Int[0, 9];\nfunction h using v(p: Point): Int[0, 9] pure { p.x }",
            "function k using v(): Boolean pure { not not (true // retained café comment  \n and false) }",
            "function m using v(): Rational[0, 1; 1, 4] pure { rational(1, 2) }",
            "function d using v(x: Rational[0, 9; 1, 9]): Rational[0, 9; 1, 9] pure { x / // divide\n rational(3, 1) }",
            "function c using v(): Boolean pure { true// c\n }",
        ] {
            let source = format!("// leading comment\n{HEADER}{declarations}\n");
            let parsed = parse(&source);
            assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
            let formatted = format(&parsed).expect("admissible source formats");
            let reparsed = parse(&formatted);
            assert!(reparsed.is_admissible(), "{formatted}");
            assert_eq!(significant(&parsed), significant(&reparsed), "{formatted}");
            assert!(formatted.contains("// leading comment"));
            assert_eq!(format(&reparsed).expect("formats again"), formatted);
        }
    }

    #[trace("TC-013", "FR-003-AC-2")]
    #[test]
    fn a_comment_never_joins_the_token_before_it() {
        let parsed = parse(&format!(
            "{HEADER}function d using v(x: Rational[0, 9; 1, 9]): Rational[0, 9; 1, 9] pure {{ x / // divide\n rational(3, 1) }}\nfunction c using v(): Boolean pure {{ true// c\n }}\n"
        ));
        let formatted = format(&parsed).expect("admissible source formats");
        assert!(formatted.contains("x / // divide\n"), "{formatted}");
        assert!(formatted.contains("true // c\n"), "{formatted}");
    }

    #[trace("TC-013", "FR-003-AC-3")]
    #[test]
    fn only_a_declaration_level_closing_brace_ends_its_line() {
        let parsed = parse(&format!(
            "{HEADER}record P {{ x: Int[0, 9]; }} function f using v(b: Boolean): Int[0, 9] pure {{ if b then P {{ x: 1 }}.x else 0 }}\n"
        ));
        let formatted = format(&parsed).expect("admissible source formats");
        let expected = format!(
            "{HEADER}record P {{\n  x : Int [0, 9];\n}}\nfunction f using v (b : Boolean) : Int [0, 9] pure {{\n  if b then P {{\n    x : 1\n  }}.x else 0\n}}\n"
        );
        assert_eq!(formatted, expected);
        assert_eq!(
            format(&parse(&formatted)).expect("formats again"),
            formatted
        );
    }
}
