// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-171: the shared grammar's `hyper-clause` over `behaviours` -- object
//! parameters, `of` quantifiers with fairness, `align skip`, `match` with step
//! labels and `stutter`, and trace-indexed reads -- parses to its productions,
//! and the finite trace domain keeps its meaning (TC-596).
use ix_trace_rs::trace;
use qsl_cst::{Limits, ParsedSource, Production};
use qsl_foundation::{SourceIdentity, Span};

fn source(declarations: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\nprofile inf = \"quire.temporal.infinite-trace/v1\";\n{declarations}"
    )
}

fn parse(text: &str) -> ParsedSource {
    qsl_cst::parse(
        SourceIdentity {
            authority: "test".into(),
            identity: "test:hyper-clause".into(),
            revision_namespace: "test".into(),
            revision: "1".into(),
        },
        "hyper.native",
        text.as_bytes(),
        Limits::default(),
    )
    .expect("S1 reads the unit")
}

fn texts<'t>(text: &'t str, parsed: &ParsedSource, production: Production) -> Vec<&'t str> {
    parsed
        .cst()
        .nodes()
        .iter()
        .filter(|node| node.production() == production)
        .map(|node| {
            let Span { start, end } = node.span();
            &text[start..end]
        })
        .collect()
}

/// FR-171-AC-1, AC-2: a clause over `behaviours` with an object parameter,
/// two quantifiers (one with a fairness constraint), an `align skip` list, a
/// `match` block of step labels and a body of indexed reads parses to one
/// node of each production.
#[trace("TC-596", "FR-171-AC-1", "FR-171-AC-2")]
#[test]
fn a_clause_over_behaviours_parses_to_its_productions() {
    let text = source(
        "hyper NonInterference using inf over behaviours (v: V::Vault)\n\
         forall trace a of V fair weak V::Vault::step;\n\
         forall trace b of V\n\
         align skip { V::Vault::mix }\n\
         match { a.step.op = b.step.op and a.step.V::Vault::step.i = b.step.V::Vault::step.i \
         and not (a.step.receiver = stutter) }\n\
         { always holds(v.l @ a = v.l @ b) }",
    );
    let parsed = parse(&text);
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    assert_eq!(
        texts(&text, &parsed, Production::BehavioursDomain),
        ["behaviours (v: V::Vault)"]
    );
    assert_eq!(
        texts(&text, &parsed, Production::Quantifier),
        [
            "forall trace a of V fair weak V::Vault::step;",
            "forall trace b of V"
        ]
    );
    assert_eq!(
        texts(&text, &parsed, Production::AlignSkip),
        ["align skip { V::Vault::mix }"]
    );
    assert_eq!(texts(&text, &parsed, Production::MatchBlock).len(), 1);
    assert_eq!(
        texts(&text, &parsed, Production::StepLabel),
        [
            "a.step.op",
            "b.step.op",
            "a.step.V::Vault::step.i",
            "b.step.V::Vault::step.i",
            "a.step.receiver"
        ]
    );
    assert_eq!(
        texts(&text, &parsed, Production::IndexedRead),
        ["v.l @ a", "v.l @ b"]
    );
}

/// FR-171 "Behavior": the finite trace domain `t in Q bounded N` keeps its
/// meaning, even when a trace variable is named `behaviours`.
#[trace("TC-596", "FR-171-AC-1")]
#[test]
fn a_finite_trace_domain_still_parses_as_a_trace_domain() {
    for domain in ["traces", "behaviours"] {
        let text = source(&format!(
            "hyper Secret using inf over {domain} in M::Execution bounded 2 forall trace a exists trace b {{ true }}"
        ));
        let parsed = parse(&text);
        assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
        assert_eq!(texts(&text, &parsed, Production::TraceDomain).len(), 1);
        assert!(texts(&text, &parsed, Production::BehavioursDomain).is_empty());
    }
}

/// An indexed read is a syntax of a hyper clause: in a function body, `@` is
/// not a token the grammar admits.
#[trace("TC-596", "FR-171-AC-1")]
#[test]
fn an_indexed_read_outside_a_hyper_or_temporal_condition_is_a_syntax_error() {
    let text = source("function f using inf(x: Integer): Integer pure { x @ a }");
    let parsed = parse(&text);
    assert!(!parsed.is_admissible());
    assert_eq!(
        parsed.diagnostics()[0].byte_span().expect("located").start,
        text.find('@').expect("the read")
    );
}
