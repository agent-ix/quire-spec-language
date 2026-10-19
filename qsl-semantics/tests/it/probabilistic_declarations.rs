// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-185 and FR-186 (TC-620, TC-621): the S3 check of the `Probabilistic`
//! family's declarations over TC-458's `ConfigVersion` fixture package
//! (`crate::model_operations`).
//!
//! S3 checks only what the forms alone let it refuse (`random` weights and
//! repeats, `workload` weights and repeats, a reward's reads, a claim's
//! thresholds, confidence parameters and event window) and refuses everything
//! else `unsupported_construct`/`not-yet-implemented`. These tests cover those
//! checks and that refusal; they tag the TC ids and no acceptance criterion,
//! because each FR-185 and FR-186 criterion also needs name resolution,
//! typing, units or the checked claim, which are not built yet.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_foundation::Span;
use qsl_semantics::check::CheckRefusal;
use quire_semantic_value::checking::CheckingLimits;
use serde_json::json;

use crate::model_operations::{
    admit_and_assemble_with_body, attempt_update_modifies_version_number, config_unit_with_body,
    config_version_document,
};

fn document() -> Vec<u8> {
    config_version_document(
        attempt_update_modifies_version_number(),
        Vec::new(),
        Vec::new(),
        json!([]),
    )
}

fn refusals_of(body: &str) -> Vec<CheckRefusal> {
    admit_and_assemble_with_body(&document(), body)
        .unwrap_or_else(|refusal| panic!("{body}: the unit assembles: {refusal:?}"))
        .check(CheckingLimits::default())
        .expect_err("the unit refuses")
}

/// Each refusal's code, catalog cause and the source text its region names.
fn located(body: &str) -> Vec<(Code, Option<&'static str>, String)> {
    let declarations = admit_and_assemble_with_body(&document(), body).expect("assembles");
    let regions = declarations.regions();
    let (text, _) = config_unit_with_body(&document(), body);
    let refusals = declarations
        .check(CheckingLimits::default())
        .expect_err("the unit refuses");
    refusals
        .iter()
        .map(|refusal| {
            let region = regions
                .refusal_region(refusal)
                .expect("the refusal names a region of the unit");
            (
                refusal.cause.code(),
                refusal.cause.cause(),
                text[region.start() as usize..region.end() as usize].to_owned(),
            )
        })
        .collect()
}

fn invalid(cause: &'static str, at: &str) -> (Code, Option<&'static str>, String) {
    (Code::InvalidProbabilisticModel, Some(cause), at.to_owned())
}

fn not_yet(at: &str) -> (Code, Option<&'static str>, String) {
    (
        Code::UnsupportedConstruct,
        Some("not-yet-implemented"),
        at.to_owned(),
    )
}

const SERVICE: [&str; 3] = [
    "random Server::attempt(d) ~ { 1 ms: 90, 3 ms: 7, 12 ms: 3 };",
    "reward duration on Server::attempt = d;",
    "workload Steady on Config { weight Server::request = 1; weight Server::attempt = 1; }",
];

const NO_FAULT: &str = "probabilistic NoFault using v on Config under Steady on origin \
     with alpha 0.01, beta 0.01, indifference 0.0005 \
     { probability >= 0.999 [ always[0,1000] holds(n.healthy) ] }";

/// Declarations that pass every form check refuse `not-yet-implemented`, one
/// refusal each, at their own span: nothing probabilistic compiles unchecked.
#[trace("TC-620", "TC-621")]
#[test]
fn a_declaration_that_passes_the_form_checks_refuses_not_yet_implemented() {
    let body = format!("{}\n{}\n{}\n{NO_FAULT}", SERVICE[0], SERVICE[1], SERVICE[2]);
    assert_eq!(
        located(&body),
        [
            not_yet(SERVICE[0]),
            not_yet(SERVICE[1]),
            not_yet(SERVICE[2]),
            not_yet(NO_FAULT),
        ]
    );
}

/// FR-185 "Random parameters": a weight that is not positive refuses
/// `invalid-weight` at the weight, a value listed twice refuses
/// `repeated-support-value` at the second, and a second distribution of one
/// parameter refuses `duplicate-distribution` at the second declaration. A
/// rational with denominator 0 is `undefined_expression`.
#[trace("TC-620")]
#[test]
fn random_weights_repeats_and_duplicate_distributions_refuse() {
    assert_eq!(
        located("random Server::attempt(d) ~ { 1 ms: 0, 3 ms: 7 };"),
        [invalid("invalid-weight", "0")]
    );
    assert_eq!(
        located("random Server::attempt(d) ~ { 1 ms: 9, 1 ms: 1 };"),
        [invalid("repeated-support-value", "1 ms")]
    );
    assert_eq!(
        located("random Server::attempt(outcome) ~ { Ok: 98, Fail: 1, Ok: 1 };"),
        [invalid("repeated-support-value", "Ok")]
    );
    let first = "random Server::attempt(d) ~ { 1 ms: 1 };";
    let second = "random Server::attempt(d) ~ { 3 ms: 1 };";
    assert_eq!(
        located(&format!("{first}\n{second}")),
        [not_yet(first), invalid("duplicate-distribution", second)]
    );
    assert_eq!(
        located("random Server::attempt(d) ~ { 1 ms: rational(1, 0) };"),
        [(
            Code::UndefinedExpression,
            Some("unproved-nonzero"),
            "rational(1, 0)".to_owned()
        )]
    );
    // Another parameter of the operation, and the same parameter of another
    // operation, are other distributions.
    let other =
        "random Server::attempt(outcome) ~ { Ok: 1 };\nrandom Server::reset(d) ~ { 1 ms: 1 };";
    assert!(located(other)
        .iter()
        .all(|(code, ..)| *code == Code::UnsupportedConstruct));
}

/// FR-185 "Workloads": a weight that is not positive refuses `invalid-weight`
/// and an operation weighted twice refuses `duplicate-weight`, each at the
/// weight.
#[trace("TC-620")]
#[test]
fn workload_weights_and_repeated_operations_refuse() {
    assert_eq!(
        located(
            "workload Steady on Config { weight Server::request = 0; weight Server::reset = 1; }"
        ),
        [invalid("invalid-weight", "0")]
    );
    assert_eq!(
        located(
            "workload Steady on Config { weight Server::request = 1; weight Server::request = 2; }"
        ),
        [invalid("duplicate-weight", "weight Server::request = 2;")]
    );
}

/// FR-185 "Rewards": a reward that reads `self`, a pre-state or `result`
/// refuses `state-read-in-reward` at the read; one that reads a parameter
/// and constants does not.
#[trace("TC-620")]
#[test]
fn a_reward_that_reads_state_refuses() {
    assert_eq!(
        located("reward duration on Server::attempt = self.attempts;"),
        [invalid("state-read-in-reward", "self")]
    );
    assert_eq!(
        located("reward cost on Server::request = pre(self.attempts) + 1;"),
        [invalid("state-read-in-reward", "pre(self.attempts)")]
    );
    assert_eq!(
        located("reward cost on Server::request = result;"),
        [invalid("state-read-in-reward", "result")]
    );
    let free = "reward cost on Server::request = 2 * d + 1;";
    assert_eq!(located(free), [not_yet(free)]);
}

fn claim(body: &str) -> String {
    format!("probabilistic C using v on Config under Steady on origin {body}")
}

/// FR-186 "Thresholds": a probability threshold, quantile level, mean
/// threshold or long-run threshold of 0 or 1 refuses `qualitative-threshold`
/// at the threshold, in every notation an exact value is written in.
#[trace("TC-621")]
#[test]
fn a_threshold_of_zero_or_one_refuses_qualitative() {
    for (body, at) in [
        ("{ probability >= 1 [ holds(true) ] }", "1"),
        ("{ probability >= 0 [ holds(true) ] }", "0"),
        ("{ probability <= 1.0 [ holds(true) ] }", "1.0"),
        (
            "{ probability >= rational(3, 3) [ holds(true) ] }",
            "rational(3, 3)",
        ),
        (
            "{ probability >= decimal(10, 1) [ holds(true) ] }",
            "decimal(10, 1)",
        ),
        (
            "{ quantile 1 of (steps from holds(a) until holds(b) within 5) <= 3 }",
            "1",
        ),
        ("{ mean of (fraction holds(a) over [0, 5]) >= 0 }", "0"),
        ("{ long_run_fraction holds(a) >= 1 }", "1"),
    ] {
        let source = claim(body);
        assert_eq!(
            located(&source),
            [invalid("qualitative-threshold", at)],
            "{body}"
        );
    }
    let interior = claim("{ probability >= 0.999 [ holds(true) ] }");
    assert_eq!(located(&interior), [not_yet(&interior)]);
}

/// FR-186 "Confidence parameters": a clause that states one or two of the
/// three, or one twice, refuses `partial-confidence`; a clause that states
/// none, or all three once with an indifference region, is not refused here.
#[trace("TC-621")]
#[test]
fn a_confidence_clause_states_all_three_parameters_once_or_none() {
    let lone = claim("with alpha 0.01 { probability >= 0.999 [ holds(true) ] }");
    assert_eq!(
        located(&lone),
        [
            invalid("partial-confidence", "alpha 0.01"),
            invalid("partial-confidence", "alpha 0.01"),
        ]
    );
    let repeated = claim(
        "with alpha 0.01, alpha 0.05, beta 0.01, indifference 0.0005 \
         { probability >= 0.999 [ holds(true) ] }",
    );
    assert_eq!(located(&repeated), [invalid("partial-confidence", "alpha")]);
    let refusals = refusals_of(&repeated);
    let span_of = |needle: &str| {
        let (text, _) = config_unit_with_body(&document(), &repeated);
        let start = text.rfind(needle).expect("in the unit");
        Span {
            start,
            end: start + needle.len(),
        }
    };
    let regions = admit_and_assemble_with_body(&document(), &repeated)
        .expect("assembles")
        .regions();
    let region = regions.refusal_region(&refusals[0]).expect("a region");
    // The second `alpha`, not the first.
    assert_eq!(
        (region.start() as usize, region.end() as usize),
        (
            span_of("alpha 0.05").start,
            span_of("alpha 0.05").start + "alpha".len()
        )
    );

    for ok in [
        "{ probability >= 0.999 [ holds(true) ] }",
        "with alpha 0.01, beta 0.01, indifference 0.0005 { probability >= 0.999 [ holds(true) ] }",
        "with indifference 0.0005, beta 0.01, alpha 0.01 { probability >= 0.999 [ holds(true) ] }",
    ] {
        let source = claim(ok);
        assert_eq!(located(&source), [not_yet(&source)], "{ok}");
    }
}

/// FR-186 "Confidence parameters", `0 < α < 1`, `0 < β < 1`, `0 < ι`,
/// `θ − ι > 0` and `θ + ι < 1`: a parameter that breaks its range refuses
/// `invalid-confidence` at the parameter.
#[trace("TC-621")]
#[test]
fn confidence_parameters_keep_their_ranges() {
    for (clause, at) in [
        ("alpha 0.01, beta 0.01, indifference 0.002", "indifference"),
        ("alpha 0.01, beta 0.01, indifference 0.999", "indifference"),
        ("alpha 0.01, beta 0.01, indifference 0", "indifference"),
        ("alpha 1, beta 0.01, indifference 0.0005", "alpha"),
        ("alpha 0.01, beta 0, indifference 0.0005", "beta"),
        (
            "alpha rational(3, 2), beta 0.01, indifference 0.0005",
            "alpha",
        ),
    ] {
        let source = claim(&format!(
            "with {clause} {{ probability >= 0.999 [ holds(true) ] }}"
        ));
        assert_eq!(
            located(&source),
            [invalid("invalid-confidence", at)],
            "{clause}"
        );
    }
    // θ − ι = 0 is not above zero: θ = 0.0005, ι = 0.0005.
    let source = claim(
        "with alpha 0.01, beta 0.01, indifference 0.0005 { probability >= 0.0005 [ holds(true) ] }",
    );
    assert_eq!(
        located(&source),
        [invalid("invalid-confidence", "indifference")]
    );
    // θ + ι = 0.9995 + 0.0004 < 1 is fine.
    let source = claim(
        "with alpha 0.01, beta 0.01, indifference 0.0004 { probability >= 0.9995 [ holds(true) ] }",
    );
    assert_eq!(located(&source), [not_yet(&source)]);
}

/// FR-186 "Events and measures": an event's window is not checked yet, so a
/// claim with an operator that has no closed interval refuses
/// `not-yet-implemented` like any other claim, not for its window: QSpec
/// FR-407 admits `[always holds(P)]` and the like under the infinite-trace
/// profile, and profiles do not resolve here yet.
#[trace("TC-621")]
#[test]
fn an_event_window_is_not_refused_before_profiles_resolve() {
    for event in [
        "always holds(n.healthy)",
        "eventually[0,*] holds(n.healthy)",
        "eventually[0,100] always[0,10] holds(n.healthy)",
        "holds(a) and not holds(b)",
    ] {
        let source = claim(&format!("{{ probability >= 0.99 [ {event} ] }}"));
        assert_eq!(located(&source), [not_yet(&source)], "{event}");
    }
}

/// A quantity literal in an ordinary expression refuses `unsupported_construct`/
/// `not-yet-implemented`: it parses and builds, and no checker reads one yet.
#[trace("TC-620")]
#[test]
fn a_quantity_literal_in_a_function_body_refuses_not_yet_implemented() {
    let refusals = refusals_of("function lag using v(): Boolean pure { 5 ms }\n");
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_eq!(refusals[0].cause.code(), Code::UnsupportedConstruct);
    assert_eq!(refusals[0].cause.cause(), Some("not-yet-implemented"));
}
