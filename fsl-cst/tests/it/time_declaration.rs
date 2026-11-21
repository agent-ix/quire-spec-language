// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original syntactic near-neighbours of timed declarations; no S2/S3 admission oracle.

use fsl_cst::{CompleteCause, CompleteCode, Limits, Production};
use fsl_foundation::{SourceIdentity, Span};
use ix_trace_rs::trace;

pub(super) const PREFIX: &str = "language \"ix:native\" edition \"1-draft\";\nprofile p = \"filament.value.complete/v1\";\n// durée\n";

pub(super) fn accepts(declaration: &str, production: Production) {
    let text = format!("{PREFIX}{declaration}");
    let parsed = fsl_cst::parse(
        SourceIdentity::new("test", "test:temporal-near-misses"),
        "near-misses.native",
        text.as_bytes(),
        Limits::default(),
    )
    .expect("valid source identity and UTF-8");
    assert!(
        parsed.is_admissible(),
        "{declaration}: {:?}",
        parsed.diagnostics()
    );
    let node = parsed
        .cst()
        .nodes()
        .iter()
        .find(|node| {
            node.production() == production
                && node.span()
                    == Span {
                        start: PREFIX.len(),
                        end: text.len(),
                    }
        })
        .expect("whole accepted declaration node");
    assert_eq!(
        parsed.cst().render_node(node).expect("valid node"),
        declaration.as_bytes()
    );
    assert_eq!(parsed.cst().render(), text.as_bytes());
}

pub(super) fn refuses(declaration: &str, token: &str, expected: &str) {
    refuses_at(declaration, token, token.len(), expected);
}

pub(super) fn refuses_at(declaration: &str, token: &str, width: usize, expected: &str) {
    let text = format!("{PREFIX}{declaration}");
    let parsed = fsl_cst::parse(
        SourceIdentity::new("test", "test:temporal-near-misses"),
        "near-misses.native",
        text.as_bytes(),
        Limits::default(),
    )
    .expect("valid source identity and UTF-8");
    assert!(!parsed.is_admissible(), "must refuse {declaration}");
    let diagnostic = parsed
        .diagnostics()
        .first()
        .expect("located grammar refusal");
    let start = if token.is_empty() {
        text.len()
    } else {
        PREFIX.len() + declaration.rfind(token).expect("authored offending token")
    };
    assert_eq!(
        (diagnostic.code, diagnostic.cause),
        (
            CompleteCode::InvalidSyntax,
            if token.is_empty() {
                CompleteCause::UnexpectedEnd
            } else {
                CompleteCause::UnexpectedToken
            }
        ),
        "{declaration}: {diagnostic:?}"
    );
    assert_eq!(
        diagnostic.byte_span(),
        Some(Span {
            start,
            end: start + width
        }),
        "{declaration}: {diagnostic:?}"
    );
    assert_eq!(diagnostic.message, expected, "{declaration}");
    assert_eq!(parsed.cst().render(), text.as_bytes());
}

/// Provenance: QSL-2076; time-decl in the complete-V1 shared grammar.
#[test]
#[trace("FSpec-FR-339-AC-1")]
fn time_declaration_refuses_near_misses() {
    accepts("time Chamber dense unit ms;", Production::TimeDeclaration);
    accepts(
        "time Chamber tick Chamber::advance period 2 ms unit ms;",
        Production::TimeDeclaration,
    );
    for (bad, token, expected) in [
        ("time Chamber dense;", ";", "expected `unit`"),
        (
            "time Chamber tick Chamber::advance 2 ms unit ms;",
            "2",
            "expected `period`",
        ),
        (
            "time Chamber tick Chamber::advance period \"slow\" unit ms;",
            "\"slow\"",
            "expected `decimal` or `rational` or decimal literal or unsigned integer",
        ),
        ("time Chamber dense unit ms", "", "expected `;`"),
    ] {
        refuses(bad, token, expected);
    }
}

/// Provenance: QSL-2076; time-invariant, clock-bound and quantified-clock-bound.
#[test]
#[trace("FSpec-FR-339-AC-1")]
fn time_invariant_and_clock_bound_refuse_near_misses() {
    accepts("time invariant Bounded when { enabled } { self.age <= 7 ms and forall (part in fleet : part.age < 9 ms) }", Production::TimeInvariant);
    for (bad, token, expected) in [
        (
            "time invariant Bounded when { enabled } { self.age 7 ms }",
            "7",
            "expected `<=` or `<`",
        ),
        (
            "time invariant Bounded when { enabled } { self.age <= 7 ms or self.idle < 9 ms }",
            "or",
            "expected `}`",
        ),
        (
            "time invariant Bounded when { enabled } { forall () }",
            ")",
            "expected identifier",
        ),
        (
            "time invariant Bounded when { enabled } { forall (part fleet : part.age < 9 ms) }",
            "fleet",
            "expected `in`",
        ),
    ] {
        refuses(bad, token, expected);
    }
}

/// Provenance: QSL-2076; urgent-decl's target, guard and terminator are mandatory.
#[test]
#[trace("FSpec-FR-339-AC-1")]
fn urgent_declaration_refuses_near_misses() {
    accepts(
        "urgent Drain Chamber::flush;",
        Production::UrgentDeclaration,
    );
    accepts(
        "urgent Drain when { backlog > 0 };",
        Production::UrgentDeclaration,
    );
    for (bad, token, expected) in [
        ("urgent Drain;", ";", "expected `when` or identifier"),
        (
            "urgent Drain Chamber::flush afterwards;",
            "afterwards",
            "expected `;`",
        ),
        ("urgent Drain when backlog > 0;", "backlog", "expected `{`"),
        ("urgent Drain Chamber::flush", "", "expected `;`"),
    ] {
        refuses(bad, token, expected);
    }
}
