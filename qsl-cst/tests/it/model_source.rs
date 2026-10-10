// SPDX-License-Identifier: AGPL-3.0-or-later
//! Model source syntax prerequisites for FR-300/301.

use ix_trace_rs::trace;
use qsl_cst::{CompleteCause, CompleteCode, Limits, ParsedSource, Production, TokenKind};
use qsl_foundation::{SourceIdentity, Span};

fn source(body: &str) -> String {
    format!("language \"ix:native\" edition \"1-draft\";\nprofile m = \"quire.model.complete/v1\";\npre P using m on M::T::op {{ {body} }}\n")
}

fn parse(text: &str, limits: Limits) -> ParsedSource {
    qsl_cst::parse(
        SourceIdentity::new("test", "model-syntax", "test", "1"),
        "model-syntax.native",
        text.as_bytes(),
        limits,
    )
    .expect("source metadata admitted")
}

#[trace("TC-790", "TC-792", "FR-300-AC-1", "FR-301-AC-1", "FR-301-AC-2")]
#[test]
fn model_forms_preserve_source_bytes_and_named_productions() {
    for body in [
        "deref(r).field",
        "allInstances<M::T>(p)",
        "lookup<M::T>(p, r) absent refused",
        "lookup<M::T>(p, r) absent empty",
        "lookup<M::T>(p, r) absent undefined",
        "reaches(r, s, parent)",
        "self.query()",
        "deref(r).query(1, lookup<M::T>(p, r) absent refused).field",
        "r.query().next(2, 3)",
        "r.exhausted()",
    ] {
        let text = source(body);
        let parsed = parse(&text, Limits::default());
        assert!(parsed.is_admissible(), "{body}: {:?}", parsed.diagnostics());
        assert_eq!(parsed.cst().render(), text.as_bytes());
        assert_eq!(parsed.source().identity().identity, "model-syntax");
        assert_eq!(parsed.effective_limits(), Limits::default());
        let start = text.find(body).unwrap();
        assert!(parsed.cst().nodes().iter().any(|node| node.production()
            == Production::Expression
            && node.span()
                == Span {
                    start,
                    end: start + body.len()
                }));
    }
}

#[trace("TC-790", "FR-300-AC-2")]
#[test]
fn invalid_model_productions_refuse_at_the_offending_token() {
    for (body, token) in [
        ("lookup<M::T>(p, r)", "}"),
        ("lookup<M::T>(p) absent refused", ")"),
        ("lookup<M::T>(p, r) absent bogus", "bogus"),
        ("lookup<M::T>(p, r, s) absent refused", ", s"),
        ("r.query(, 1)", ","),
        ("r.query(1,)", ")"),
        ("r.query(1 2)", "2"),
        ("r.3", "3"),
    ] {
        let text = source(body);
        let parsed = parse(&text, Limits::default());
        assert!(!parsed.is_admissible(), "{body}");
        let diagnostic = &parsed.diagnostics()[0];
        assert_eq!(diagnostic.code, CompleteCode::InvalidSyntax, "{body}");
        assert_eq!(diagnostic.cause, CompleteCause::UnexpectedToken, "{body}");
        assert_eq!(diagnostic.source.identity, "model-syntax");
        assert_eq!(diagnostic.path, "model-syntax.native");
        let start = text.rfind(token).unwrap();
        let region = diagnostic.region.as_ref().expect("located refusal");
        assert_eq!(region.start(), start, "{body}: {diagnostic:?}");
        assert_eq!(
            region.end(),
            start + token.split_whitespace().next().unwrap().len(),
            "{body}"
        );
        assert!(
            diagnostic.message.starts_with("expected "),
            "{diagnostic:?}"
        );
    }
}

#[trace("TC-790", "FR-300-AC-1")]
#[test]
fn absence_words_remain_identifiers_outside_the_lookup_mode() {
    let text = "language \"ix:native\" edition \"1-draft\"; profile v = \"quire.value.complete/v1\"; function empty using v(undefined: Boolean, refused: Boolean): Boolean pure { undefined and refused }";
    let parsed = parse(text, Limits::default());
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    for word in [b"empty".as_slice(), b"undefined", b"refused"] {
        assert!(parsed
            .cst()
            .tokens()
            .iter()
            .any(|token| token.spelling() == word && token.kind() == TokenKind::Identifier));
    }
}

#[trace("TC-790", "FR-300-AC-1")]
#[test]
fn model_sources_keep_the_existing_budget_refusals() {
    let text = source("r.query(lookup<M::T>(p, r) absent refused, 3)");
    for limits in [
        Limits::default().with_tokens(1),
        Limits::default().with_nodes(1),
        Limits::default().with_work_units(0),
    ] {
        let parsed = parse(&text, limits);
        assert!(!parsed.is_admissible());
        assert_eq!(
            parsed.diagnostics()[0].code,
            CompleteCode::StageLimitExceeded
        );
        assert!(parsed.diagnostics()[0].limit().is_some());
        assert_eq!(parsed.effective_limits(), limits);
    }
    let refusal = qsl_cst::parse(
        SourceIdentity::new("test", "model-syntax", "test", "1"),
        "model-syntax.native",
        text.as_bytes(),
        Limits::default().with_source_bytes(text.len() - 1),
    )
    .unwrap_err();
    assert_eq!(refusal.code, CompleteCode::StageLimitExceeded);
}
