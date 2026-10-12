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

#[trace("TC-790")]
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

#[trace("TC-790")]
#[test]
fn population_type_references_are_admitted_as_named_type_productions() {
    for spelling in [
        "Population<M::Order>[3]",
        "Population<M::Order>[0]",
        "Population<M::Order>[123456789012345678901234567890]",
        "Population<M::Order>",
    ] {
        let text = format!(
            "language \"ix:native\" edition \"1-draft\"; profile v = \"quire.value.complete/v1\"; function f using v(p: {spelling}): Boolean pure {{ true }}\n"
        );
        let parsed = parse(&text, Limits::default());
        assert!(
            parsed.is_admissible(),
            "{spelling}: {:?}",
            parsed.diagnostics()
        );
        assert_eq!(parsed.cst().render(), text.as_bytes());
        let start = text.find(spelling).unwrap();
        let population = parsed
            .cst()
            .nodes()
            .iter()
            .find(|node| {
                node.production() == Production::TypeReference && node.span().start == start
            })
            .expect("population type reference");
        assert_eq!(population.span().end, start + spelling.len());
    }
}

#[trace("TC-790")]
#[test]
fn population_type_references_reject_malformed_maxima_at_the_offending_token() {
    for (spelling, offending, cause) in [
        (
            "Population<M::Order>[]",
            "]",
            CompleteCause::UnexpectedToken,
        ),
        (
            "Population<M::Order>[-1]",
            "-",
            CompleteCause::UnexpectedToken,
        ),
        (
            "Population<M::Order>[03]",
            "03",
            CompleteCause::InvalidToken,
        ),
        (
            "Population<M::Order>[1.5]",
            "1.5",
            CompleteCause::UnexpectedToken,
        ),
        (
            "Population<M::Order>[1e3]",
            "1e3",
            CompleteCause::UnexpectedToken,
        ),
        (
            "Population<M::Order>[*]",
            "*",
            CompleteCause::UnexpectedToken,
        ),
        (
            "Population<M::Order>[0,3]",
            ",",
            CompleteCause::UnexpectedToken,
        ),
        (
            "Population<M::Order[3]",
            "[",
            CompleteCause::UnexpectedToken,
        ),
        (
            "Population<Boolean>[3]",
            "Boolean",
            CompleteCause::UnexpectedToken,
        ),
        (
            "population<M::Order>[3]",
            "<",
            CompleteCause::UnexpectedToken,
        ),
    ] {
        let text = format!(
            "language \"ix:native\" edition \"1-draft\"; profile v = \"quire.value.complete/v1\"; function f using v(p: {spelling}): Boolean pure {{ true }}\n"
        );
        let parsed = parse(&text, Limits::default());
        assert!(!parsed.is_admissible(), "{spelling}");
        let diagnostic = &parsed.diagnostics()[0];
        assert_eq!(diagnostic.code, CompleteCode::InvalidSyntax, "{spelling}");
        assert_eq!(diagnostic.cause, cause, "{spelling}");
        let spelling_start = text.find(spelling).unwrap();
        let start = spelling_start + text[spelling_start..].find(offending).unwrap();
        let region = diagnostic.region.as_ref().expect("located refusal");
        assert_eq!(
            usize::try_from(region.start()).unwrap(),
            start,
            "{spelling}: text={text:?}, diagnostic={diagnostic:?}"
        );
        assert_eq!(
            usize::try_from(region.end()).unwrap(),
            start + offending.len(),
            "{spelling}: {region:?}"
        );
    }
}

#[trace("TC-790")]
#[test]
fn population_type_reference_reports_missing_maximum_close_at_eof() {
    let spelling = "Population<M::Order>[3";
    let text = format!(
        "language \"ix:native\" edition \"1-draft\"; profile v = \"quire.value.complete/v1\"; function f using v(p: {spelling}"
    );
    let parsed = parse(&text, Limits::default());
    assert!(!parsed.is_admissible(), "{spelling}");
    let diagnostic = &parsed.diagnostics()[0];
    assert_eq!(diagnostic.code, CompleteCode::InvalidSyntax);
    assert_eq!(diagnostic.cause, CompleteCause::UnexpectedToken);
    let region = diagnostic.region.as_ref().expect("located EOF refusal");
    let eof = text.len();
    assert_eq!(usize::try_from(region.start()).unwrap(), eof);
    assert_eq!(usize::try_from(region.end()).unwrap(), eof);
}

#[trace("TC-790")]
#[test]
fn population_keyword_is_reserved_only_in_type_and_binder_positions() {
    let uppercase_text = "language \"ix:native\" edition \"1-draft\"; profile v = \"quire.value.complete/v1\"; predicate Population using v(): Boolean { true }\n";
    let uppercase = parse(uppercase_text, Limits::default());
    assert!(!uppercase.is_admissible());
    let region = uppercase.diagnostics()[0]
        .region
        .as_ref()
        .expect("located reserved binder refusal");
    let start = uppercase_text.find("Population").unwrap();
    assert_eq!(usize::try_from(region.start()).unwrap(), start);
    assert_eq!(
        usize::try_from(region.end()).unwrap(),
        start + "Population".len()
    );

    let lowercase_text = "language \"ix:native\" edition \"1-draft\"; profile v = \"quire.value.complete/v1\"; predicate population using v(): Boolean { true }\n";
    let lowercase = parse(lowercase_text, Limits::default());
    assert!(lowercase.is_admissible(), "{:?}", lowercase.diagnostics());

    for body in ["M::Population", "receiver.Population"] {
        let parsed = parse(&source(body), Limits::default());
        assert!(parsed.is_admissible(), "{body}: {:?}", parsed.diagnostics());
    }
}

#[trace("TC-790")]
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
        assert_eq!(
            usize::try_from(region.start()).unwrap(),
            start,
            "{body}: {diagnostic:?}"
        );
        assert_eq!(
            usize::try_from(region.end()).unwrap(),
            start + token.split_whitespace().next().unwrap().len(),
            "{body}"
        );
        assert!(
            diagnostic.message.starts_with("expected "),
            "{diagnostic:?}"
        );
    }
}

#[trace("TC-790")]
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

#[trace("TC-790")]
#[test]
fn model_sources_keep_the_existing_budget_refusals() {
    let text = source("r.query(lookup<M::T>(p, r) absent refused, 3)");
    for limits in [
        Limits::default().with_tokens(1),
        Limits::default().with_nodes(1),
        Limits::default().with_work_units(0),
    ] {
        let diagnostic = match qsl_cst::parse(
            SourceIdentity::new("test", "model-syntax", "test", "1"),
            "model-syntax.native",
            text.as_bytes(),
            limits,
        ) {
            Ok(parsed) => {
                assert!(!parsed.is_admissible());
                assert_eq!(parsed.effective_limits(), limits);
                parsed.diagnostics()[0].clone()
            }
            Err(diagnostic) => *diagnostic,
        };
        assert_eq!(diagnostic.code, CompleteCode::StageLimitExceeded);
        assert!(diagnostic.limit().is_some());
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
