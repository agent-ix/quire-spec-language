// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-199: `lexer::Limits::bounded()` no longer clamps a caller-supplied
//! ceiling down to [`Limits::default`] (ADR-011 §7.3; NFR-001 "an
//! implementation ceiling is not a domain bound"). These tests exercise the
//! `tokens` field directly through [`recognize`], the one ceiling the lexer
//! itself enforces that is not also independently re-clamped elsewhere
//! (`source_bytes` is separately hard-clamped by
//! `qsl_foundation::source::MAX_SOURCE_BYTES`, out of this ticket's scope;
//! `nodes`/`nesting` are QSL-197's parser-nesting territory).
use ix_trace_rs::trace;
use qsl_cst::diagnostic::read_source;
use qsl_cst::lexer::{recognize, Limits};
use qsl_foundation::{Code, SourceIdentity};

fn identity(id: &str) -> SourceIdentity {
    SourceIdentity {
        identity: format!("test:{id}"),
        revision: "1".into(),
    }
}

/// `n` single-character identifier tokens, well-formed at the lexer level
/// (no delimiters, so nesting never enters into it), plus a trailing `Kind::End`
/// the lexer appends itself.
fn flat_tokens(n: usize) -> String {
    "x ".repeat(n)
}

#[trace("TC-012", "NFR-001-M-2")]
#[test]
fn a_caller_raised_token_ceiling_admits_a_source_the_default_refuses() {
    let text = flat_tokens(Limits::default().tokens + 50);
    let source = read_source(
        identity("raise"),
        "raise.native",
        text.as_bytes(),
        Limits::default().source_bytes,
    )
    .expect("well under the source-byte ceiling");

    assert!(
        recognize(&source, Limits::default()).is_err(),
        "the default token ceiling must refuse a source past it"
    );

    let raised = Limits {
        tokens: Limits::default().tokens + 100,
        ..Limits::default()
    };
    assert!(
        recognize(&source, raised).is_ok(),
        "a caller-raised token ceiling must admit what the default refuses"
    );
}

#[trace("TC-012", "NFR-001-M-2")]
#[test]
fn reaching_a_caller_raised_token_ceiling_refuses_naming_the_kind_and_bound() {
    let raised = Limits {
        tokens: Limits::default().tokens + 100,
        ..Limits::default()
    };
    let text = flat_tokens(raised.tokens + 10);
    let source = read_source(
        identity("reach"),
        "reach.native",
        text.as_bytes(),
        raised.source_bytes,
    )
    .expect("well under the source-byte ceiling");

    let refusal =
        recognize(&source, raised).expect_err("must refuse past the caller's own raised ceiling");
    assert_eq!(refusal.code, Code::ResourceExhausted);
    assert!(
        refusal.message.contains("token"),
        "refusal must name the exhausted resource kind, got {refusal:?}"
    );
    assert!(
        refusal.message.contains(&raised.tokens.to_string()),
        "refusal must name the configured bound, got {refusal:?}"
    );
}
