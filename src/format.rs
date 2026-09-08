// SPDX-License-Identifier: AGPL-3.0-only
//! Format validated source by rewriting whitespace, preserving every token and
//! comment. Syntax formatting does not need a second expression printer/grammar.
use crate::token::Kind;
use crate::{Code, Diagnostic, Limits, ParsedUnit, Phase};
use logos::Logos;

pub fn format(unit: &ParsedUnit) -> Result<String, Box<Diagnostic>> {
    let mut output = String::new();
    let mut previous: Option<Kind> = None;
    let mut indent = 0;
    for (token, range) in Kind::lexer(unit.source().text()).spanned() {
        let kind = token.expect("ParsedUnit contains validated source");
        let spelling = &unit.source().text()[range.clone()];
        if kind == Kind::CloseBrace {
            indent -= 1;
            newline(&mut output);
        }
        if output.ends_with('\n') && indent > 0 {
            output.push_str("  ");
        }
        let no_space = output.is_empty()
            || output.ends_with([' ', '\n'])
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
            output.push(' ');
        }
        output.push_str(spelling);
        match kind {
            Kind::OpenBrace => {
                indent += 1;
                newline(&mut output);
            }
            Kind::CloseBrace | Kind::Semicolon | Kind::Comment => newline(&mut output),
            _ => {}
        }
        if output.len() >= Limits::default().source_bytes {
            return Err(crate::diagnostic::error(
                unit.source(),
                Code::ResourceExhausted,
                Phase::Format,
                range.start,
                range.end,
                "formatted source byte budget exhausted",
            ));
        }
        previous = Some(kind);
    }
    newline(&mut output);
    Ok(output)
}

fn newline(output: &mut String) {
    if !output.ends_with('\n') {
        output.push('\n');
    }
}
