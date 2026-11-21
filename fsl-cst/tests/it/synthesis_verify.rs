// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original S1 synthesis and ordered verification-plan near-neighbours.
use super::time_declaration::{PREFIX, accepts, refuses};
use fsl_cst::{CompleteCause, CompleteCode, Limits, Production};
use fsl_foundation::{SourceIdentity, Span};
use ix_trace_rs::trace;

/// Provenance: QSL-2076; synthesis-decl mandatory grammar/domain/goal sequence.
#[test]
#[trace("FSpec-FR-339-AC-1")]
fn synthesis_declaration_refuses_near_misses() {
    accepts(
        "synthesis Controller using p grammar Candidate domain Inputs satisfies { valid };",
        Production::SynthesisDeclaration,
    );
    for (bad, token, expected) in [
        (
            "synthesis Controller using p grammar Candidate domain Inputs;",
            ";",
            "expected `satisfies`",
        ),
        (
            "synthesis Controller using p domain Inputs grammar Candidate satisfies { valid };",
            "domain",
            "expected `grammar`",
        ),
        (
            "synthesis Controller using p grammar Candidate domain { valid };",
            "{",
            "expected identifier",
        ),
        (
            "synthesis Controller using p grammar Candidate domain Inputs satisfies { valid }",
            "",
            "expected `;`",
        ),
    ] {
        refuses(bad, token, expected);
    }
}

/// Provenance: QSL-2076; verification-plan and verification-step ordered clauses.
#[test]
#[trace("FSpec-FR-339-AC-1")]
fn verification_plan_refuses_near_misses() {
    accepts("verify Safety using p { check Base claim Bounded method Exhaustive domain Inputs; check Next claim Stable method Induction domain Inputs depends [Base] bound 5; }", Production::VerificationPlan);
    for (bad, token, expected) in [
        ("verify Safety using p { check Base claim ; }", ";", "expected identifier"),
        ("verify Safety using p { check Base claim Bounded domain Inputs method Exhaustive; }", "domain", "expected `method`"),
        ("verify Safety using p { check Base claim Bounded method Exhaustive domain Inputs bound 5 depends [Earlier]; }", "depends", "expected `;`"),
        ("verify Safety using p { check Base claim Bounded method Exhaustive domain Inputs }", "}", "expected `;`"),
    ] {
        refuses(bad, token, expected);
    }
}

/// FSpec FR-802 "Classification": `Reach.steps.sum(s => c)`, optionally plus
/// a constant, is a plan objective, and the objective root is the one place
/// `=>` is read.
#[test]
#[trace("FSpec-FR-802-AC-1")]
fn step_sum_objective_reads_only_at_the_objective_root() {
    for objective in [
        "minimize Reach.steps.sum(s => 2)",
        "minimize Reach.steps.sum(s => if s.operation = M::Op::charge then 0.10 else 0.05)",
        "maximize Reach.steps.sum(s => s.after.cost) + 3",
        "minimize Reach.steps.sum(s => 1) + Reach.length * 2",
    ] {
        accepts(
            &format!("synthesis Reach using p grammar M::G domain M::D {objective} satisfies {{ true }};"),
            Production::SynthesisDeclaration,
        );
    }
    // The formatter writes the form back as it reads it.
    let source = "language \"ix:native\" edition \"1-draft\";\nprofile p = \"filament.value.complete/v1\";\nsynthesis Reach using p grammar M::G domain M::D minimize   Reach.steps.sum( s=>if s.index=0 then 0.10 else 0.05 )+1 satisfies { true };\n";
    let parsed = fsl_cst::parse(
        SourceIdentity::new("test", "test:step-sum-format"),
        "step-sum.native",
        source.as_bytes(),
        Limits::default(),
    )
    .expect("valid source identity and UTF-8");
    let formatted = fsl_cst::format::format(&parsed).expect("an admissible parse formats");
    let reparsed = fsl_cst::parse(
        SourceIdentity::new("test", "test:step-sum-format"),
        "step-sum.native",
        formatted.as_bytes(),
        Limits::default(),
    )
    .expect("valid source identity and UTF-8");
    assert!(reparsed.is_admissible(), "{:?}", reparsed.diagnostics());
    assert_eq!(fsl_cst::format::format(&reparsed).expect("formats"), formatted);
    for (bad, token) in [
        (
            "synthesis Reach using p grammar M::G domain M::D satisfies { Reach.steps.sum(s => 1) > 0 };",
            "=>",
        ),
        (
            "synthesis Reach using p grammar M::G domain M::D minimize 1 + Reach.steps.sum(s => 1) satisfies { true };",
            "=>",
        ),
        (
            "synthesis Reach using p grammar M::G domain M::D minimize Reach.steps.min(s => 1) satisfies { true };",
            "=>",
        ),
        (
            "synthesis Reach using p grammar M::G domain M::D minimize Reach.steps.sum(s => 1, 2) satisfies { true };",
            ",",
        ),
    ] {
        let text = format!("{PREFIX}{bad}");
        let parsed = fsl_cst::parse(
            SourceIdentity::new("test", "test:step-sum-refusal"),
            "step-sum.native",
            text.as_bytes(),
            Limits::default(),
        )
        .expect("valid source identity and UTF-8");
        let diagnostic = parsed.diagnostics().first().expect("a located refusal");
        let start = PREFIX.len() + bad.rfind(token).expect("the offending token");
        assert_eq!(
            (diagnostic.code, diagnostic.cause, diagnostic.byte_span()),
            (
                CompleteCode::InvalidSyntax,
                CompleteCause::UnexpectedToken,
                Some(Span { start, end: start + token.len() })
            ),
            "{bad}"
        );
    }
}
