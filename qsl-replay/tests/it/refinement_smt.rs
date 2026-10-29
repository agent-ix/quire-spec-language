// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315 "Refinement queries" (TC-900) and FR-314 "Refinement certificates"
//! (TC-893, TC-1165): `encode_refinement_query` and
//! `check_refinement_smt_proof` over ADR-020 §8's `CasRefinesCounter`, with
//! hand-written Alethe refutations of its queries.

use ix_trace_rs::trace;
use qsl_eval::model_subject::ModelSubject;
use qsl_eval::smt::{Refusal, SmtEncodingRefusal};
use qsl_replay::{
    check_refinement_smt_proof, encode_refinement_query, CertificateCheckLimits, CertificateLocus,
    CertificateRejection, CertificateRule, IncompleteCause, ModelCheckLimit, ProofFormat,
    QueryPart, RefinementCertificateRequest, RefinementQueryKind, RefinementSmtCertificate,
    RefutedQuery, SmtProofCheck,
};
use quire_exact::Cancel;

use crate::refinement_fixture::{
    cas_refines_counter, cas_refines_counter_add, mutated, Variant, World, SPEC, UNLIMITED,
};

/// The subjects of `world`'s refinement.
fn subjects(world: &World) -> (ModelSubject<'_>, ModelSubject<'_>) {
    let concrete = ModelSubject {
        simulation: world.concrete_request(UNLIMITED),
        proof_bounds: Vec::new(),
    };
    let abstract_ = ModelSubject {
        simulation: world.request(SPEC, vec![world.abstract_initial()], UNLIMITED),
        proof_bounds: Vec::new(),
    };
    (concrete, abstract_)
}

/// Run `body` with the refinement request of `world`.
fn with_request<T>(world: &World, body: impl FnOnce(&RefinementCertificateRequest<'_>) -> T) -> T {
    let (concrete, abstract_) = subjects(world);
    let request = RefinementCertificateRequest {
        package: world.package(),
        refinement: world
            .refinement()
            .v2_emission()
            .expect("the refinement emits")
            .identity,
        concrete: &concrete,
        abstract_: &abstract_,
    };
    body(&request)
}

/// The position in the `steps` array of the row whose class is spelled
/// `spelling`.
fn row_of(world: &World, spelling: &str) -> u64 {
    let refinement = world.refinement();
    let sources = refinement.v2_emission().expect("emits").step_sources;
    let position = sources
        .iter()
        .position(|source| refinement.steps()[*source].spelling().ends_with(spelling))
        .expect("the row exists");
    u64::try_from(position).expect("a small position")
}

fn encode(
    request: &RefinementCertificateRequest<'_>,
    kind: RefinementQueryKind,
) -> Result<String, SmtEncodingRefusal> {
    encode_refinement_query(request, kind)
        .map(|bytes| String::from_utf8(bytes).expect("a script is UTF-8"))
}

/// The terms of `query`'s assertions, in order.
fn assertions(query: &str) -> Vec<&str> {
    query
        .lines()
        .filter_map(|line| line.strip_prefix("(assert ")?.strip_suffix(')'))
        .collect()
}

const RANGE: &str = "(and (<= 0 s0.c.value) (<= s0.c.value 2))";

/// A refutation of the `Initial` query: its initial predicate fixes
/// `s0.c.value` at 0, which the negation denies.
fn initial_proof(query: &str) -> String {
    let asserted = assertions(query);
    let [.., initial, negation] = asserted.as_slice() else {
        panic!("an initial query has two assertions after its ranges");
    };
    format!(
        "(assume a0 {initial})\n\
         (assume a1 {negation})\n\
         (step t0 (cl (= s0.c.value 0)) :rule and :premises (a0))\n\
         (step t1 (cl) :rule resolution :premises (t0 a1))\n"
    )
}

const UNCHANGED: &str = "(= s1.c.value s0.c.value)";

/// A refutation of a `stutter` row's query `(and X (not Q))`, `Q` the frame
/// equality of `value`, a conjunct of `X`.
fn stutter_proof(assertion: &str) -> String {
    let negated = format!(" (not {UNCHANGED}))");
    let step = assertion
        .strip_prefix("(and ")
        .and_then(|rest| rest.strip_suffix(&negated))
        .expect("a stutter query asserts (and X (not Q))");
    format!(
        "(assume a0 {assertion})\n\
         (step t0 (cl {step}) :rule and :premises (a0))\n\
         (step t1 (cl (not {UNCHANGED})) :rule and :premises (a0))\n\
         (step t2 (cl {UNCHANGED}) :rule and :premises (t0))\n\
         (step t3 (cl) :rule resolution :premises (t2 t1))\n"
    )
}

/// A refutation of worker `w`'s `commit` row query: the concrete commit
/// writes `value = tmp + 1` with `tmp = value`, so `inc` holds over the
/// mapped pair, by linear arithmetic over the range of `s0.c.value`.
fn commit_proof(assertion: &str, w: char) -> String {
    let rng = "(and (<= 0 (+ s0.c.value 1)) (<= (+ s0.c.value 1) 2))";
    let k = "(and (< s0.c.value 2) (= s1.c.value (+ s0.c.value 1)))";
    let implication = format!("(=> (< s0.c.value 2) {rng})");
    let m = format!("(and {implication} {k})");
    let n = format!("(and (< s0.c.value 2) {m})");
    let negated = format!(" (not {n}))");
    let c = assertion
        .strip_prefix("(and ")
        .and_then(|rest| rest.strip_suffix(&negated))
        .expect("a commit query asserts (and C (not N))");
    let tmp = format!("s0.c.tmp{w}");
    let busy = format!("s0.c.busy{w}");
    let pre = format!("(and (and {busy} (= {tmp} s0.c.value)) (< s0.c.value 2))");
    let written = format!("(and (and (< {tmp} 2) (= s1.c.value (+ {tmp} 1))) (not s1.c.busy{w}))");
    let post =
        format!("(and (=> (< {tmp} 2) (and (<= 0 (+ {tmp} 1)) (<= (+ {tmp} 1) 2))) {written})");
    let v1 = "s1.c.value";
    let v = "s0.c.value";
    [
        format!("(assume a0 {assertion})"),
        format!("(assume r {RANGE})"),
        format!("(step c0 (cl {c}) :rule and :premises (a0))"),
        format!("(step c1 (cl (not {n})) :rule and :premises (a0))"),
        format!("(step c2 (cl {pre}) :rule and :premises (c0))"),
        format!("(step c3 (cl {post}) :rule and :premises (c0))"),
        format!("(step c4 (cl (< {v} 2)) :rule and :premises (c2))"),
        format!("(step c5 (cl (and {busy} (= {tmp} {v}))) :rule and :premises (c2))"),
        format!("(step c6 (cl (= {tmp} {v})) :rule and :premises (c5))"),
        format!("(step c7 (cl {written}) :rule and :premises (c3))"),
        format!("(step c8 (cl (and (< {tmp} 2) (= {v1} (+ {tmp} 1)))) :rule and :premises (c7))"),
        format!("(step c9 (cl (= {v1} (+ {tmp} 1))) :rule and :premises (c8))"),
        format!("(step c10 (cl (<= 0 {v})) :rule and :premises (r))"),
        format!(
            "(step c11 (cl (= {v1} (+ {v} 1)) (not (<= {v1} (+ {v} 1))) (not (<= (+ {v} 1) {v1}))) :rule la_disequality)"
        ),
        format!(
            "(step c12 (cl (<= {v1} (+ {v} 1)) (not (= {v1} (+ {tmp} 1))) (not (= {tmp} {v}))) :rule la_generic :args (1 (- 1) (- 1)))"
        ),
        format!(
            "(step c13 (cl (<= (+ {v} 1) {v1}) (not (= {v1} (+ {tmp} 1))) (not (= {tmp} {v}))) :rule la_generic :args (1 1 1))"
        ),
        format!("(step c14 (cl (= {v1} (+ {v} 1))) :rule resolution :premises (c11 c12 c13 c9 c6))"),
        format!("(step c15 (cl (<= 0 (+ {v} 1)) (not (<= 0 {v}))) :rule la_generic :args (1 1))"),
        format!("(step c16 (cl (<= (+ {v} 1) 2) (not (< {v} 2))) :rule la_generic :args (1 1))"),
        format!("(step c17 (cl (<= 0 (+ {v} 1))) :rule resolution :premises (c15 c10))"),
        format!("(step c18 (cl (<= (+ {v} 1) 2)) :rule resolution :premises (c16 c4))"),
        format!(
            "(step c19 (cl {rng} (not (<= 0 (+ {v} 1))) (not (<= (+ {v} 1) 2))) :rule and_neg)"
        ),
        format!("(step c20 (cl {rng}) :rule resolution :premises (c19 c17 c18))"),
        format!("(step c21 (cl {implication} (not {rng})) :rule implies_neg2)"),
        format!("(step c22 (cl {implication}) :rule resolution :premises (c21 c20))"),
        format!("(step c23 (cl {k} (not (< {v} 2)) (not (= {v1} (+ {v} 1)))) :rule and_neg)"),
        format!("(step c24 (cl {k}) :rule resolution :premises (c23 c4 c14))"),
        format!("(step c25 (cl {m} (not {implication}) (not {k})) :rule and_neg)"),
        format!("(step c26 (cl {m}) :rule resolution :premises (c25 c22 c24))"),
        format!("(step c27 (cl {n} (not (< {v} 2)) (not {m})) :rule and_neg)"),
        format!("(step c28 (cl {n}) :rule resolution :premises (c27 c4 c26))"),
        "(step c29 (cl) :rule resolution :premises (c28 c1))".to_owned(),
    ]
    .join("\n")
}

/// A refutation of a step row's query, by the row's kind.
fn step_proof(query: &str) -> String {
    let assertion = *assertions(query).last().expect("a step query asserts");
    if assertion.contains("(= s1.c.value (+ s0.c.tmpA 1))") {
        commit_proof(assertion, 'A')
    } else if assertion.contains("(= s1.c.value (+ s0.c.tmpB 1))") {
        commit_proof(assertion, 'B')
    } else {
        stutter_proof(assertion)
    }
}

fn part(query: String, proof: String) -> RefutedQuery {
    RefutedQuery {
        query: query.into_bytes(),
        proof: proof.into_bytes(),
    }
}

/// The certificate of `request` with every part's FR-315 query and a
/// hand-written refutation; a part whose query FR-315 refuses carries
/// `refused`.
fn certificate(request: &RefinementCertificateRequest<'_>, rows: u64) -> RefinementSmtCertificate {
    let base = encode(request, RefinementQueryKind::Initial).expect("the initial query encodes");
    let steps = (0..rows)
        .map(
            |row| match encode(request, RefinementQueryKind::Step { row }) {
                Ok(query) => {
                    let proof = step_proof(&query);
                    part(query, proof)
                }
                Err(_) => part(String::new(), String::new()),
            },
        )
        .collect();
    RefinementSmtCertificate {
        format: ProofFormat::Alethe,
        base: part(base.clone(), initial_proof(&base)),
        steps,
    }
}

fn check(
    request: &RefinementCertificateRequest<'_>,
    certificate: &RefinementSmtCertificate,
) -> Result<SmtProofCheck, CertificateRejection> {
    check_refinement_smt_proof(
        request,
        certificate,
        &CertificateCheckLimits::default(),
        &Cancel::new(),
    )
}

fn rejected(
    rule: CertificateRule,
    at: CertificateLocus,
) -> Result<SmtProofCheck, CertificateRejection> {
    Err(CertificateRejection { rule, at })
}

fn step_part(row: u64) -> QueryPart {
    QueryPart::Step { row: Some(row) }
}

/// `proof` with its command `index` (one command per line) replaced by
/// `command`.
fn with_command(proof: &[u8], index: usize, command: &str) -> Vec<u8> {
    let text = String::from_utf8(proof.to_vec()).expect("UTF-8");
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    lines[index] = command.to_owned();
    lines.join("\n").into_bytes()
}

/// FR-315-AC-6 (TC-900): the `Initial` and `Step` queries of
/// `CasRefinesCounter`, their determinism, and the `any`, `_` and
/// out-of-fragment refusals.
#[trace("TC-900", "FR-315-AC-6")]
#[test]
fn refinement_queries_encode_the_initial_and_step_obligations() {
    let world = World::new(&cas_refines_counter());
    let peek = row_of(&world, "peek");
    let commit = row_of(&world, "commitA");
    with_request(&world, |request| {
        let initial = encode(request, RefinementQueryKind::Initial).expect("encodes");
        assert_eq!(
            initial,
            "(set-logic QF_LIA)\n\
             (declare-const s0.c.busyA Bool)\n\
             (declare-const s0.c.busyB Bool)\n\
             (declare-const s0.c.tmpA Int)\n\
             (declare-const s0.c.tmpB Int)\n\
             (declare-const s0.c.value Int)\n\
             (assert (and (<= 0 s0.c.tmpA) (<= s0.c.tmpA 2)))\n\
             (assert (and (<= 0 s0.c.tmpB) (<= s0.c.tmpB 2)))\n\
             (assert (and (<= 0 s0.c.value) (<= s0.c.value 2)))\n\
             (assert (and (= s0.c.busyA false) (= s0.c.busyB false) (= s0.c.tmpA 0) (= s0.c.tmpB 0) (= s0.c.value 0)))\n\
             (assert (not (= s0.c.value 0)))\n\
             (check-sat)\n"
        );
        assert_eq!(
            encode(request, RefinementQueryKind::Initial).expect("encodes"),
            initial
        );

        let stutter = encode(request, RefinementQueryKind::Step { row: peek }).expect("encodes");
        assert!(stutter.contains("(declare-const s1.c.value Int)\n"));
        assert_eq!(
            assertions(&stutter).last().copied(),
            Some(
                "(and (and (= s1.c.busyA s0.c.busyA) (= s1.c.busyB s0.c.busyB) \
                 (= s1.c.tmpA s0.c.tmpA) (= s1.c.tmpB s0.c.tmpB) (= s1.c.value s0.c.value)) \
                 (not (= s1.c.value s0.c.value)))"
            )
        );
        assert_eq!(
            encode(request, RefinementQueryKind::Step { row: peek }).expect("encodes"),
            stutter
        );

        let operation =
            encode(request, RefinementQueryKind::Step { row: commit }).expect("encodes");
        assert_eq!(
            assertions(&operation).last().copied(),
            Some(
                "(and (and (and (and s0.c.busyA (= s0.c.tmpA s0.c.value)) (< s0.c.value 2)) \
                 (and (=> (< s0.c.tmpA 2) (and (<= 0 (+ s0.c.tmpA 1)) (<= (+ s0.c.tmpA 1) 2))) \
                 (and (and (< s0.c.tmpA 2) (= s1.c.value (+ s0.c.tmpA 1))) (not s1.c.busyA))) \
                 (= s1.c.busyB s0.c.busyB) (= s1.c.tmpA s0.c.tmpA) (= s1.c.tmpB s0.c.tmpB)) \
                 (not (and (< s0.c.value 2) (and (=> (< s0.c.value 2) \
                 (and (<= 0 (+ s0.c.value 1)) (<= (+ s0.c.value 1) 2))) \
                 (and (< s0.c.value 2) (= s1.c.value (+ s0.c.value 1)))))))"
            )
        );
    });

    let any = World::new(&mutated(
        &cas_refines_counter(),
        "step Impl::Counter::commitB -> Spec::Counter::inc(self);",
        "step Impl::Counter::commitB -> any;",
    ));
    let commit_b = row_of(&any, "commitB");
    with_request(&any, |request| {
        assert_eq!(
            encode(request, RefinementQueryKind::Step { row: commit_b }),
            Err(Refusal::RefinementAnyRow { row: commit_b }.into())
        );
    });

    let add = World::with(Variant::ADD, &cas_refines_counter_add());
    let bump = row_of(&add, "bump");
    with_request(&add, |request| {
        assert_eq!(
            encode(request, RefinementQueryKind::Step { row: bump }),
            Err(Refusal::RefinementOpenArgument {
                row: bump,
                argument: 0
            }
            .into())
        );
    });

    let nonlinear = World::new(&mutated(
        &cas_refines_counter(),
        "value = self.value;",
        "value = if self.value * self.tmpA = 0 then self.value else self.tmpA;",
    ));
    with_request(&nonlinear, |request| {
        let refused = encode(request, RefinementQueryKind::Initial).expect_err("refuses");
        let SmtEncodingRefusal::Refused(Refusal::OutOfFragment { locus, .. }) = &refused else {
            panic!("out-of-fragment, not {refused:?}");
        };
        assert_eq!(refused.code(), Some("out-of-fragment"));
        assert_eq!(locus.clause, "c.value");
    });
}

/// FR-314-AC-4 (TC-893): `CasRefinesCounter`'s refinement certificate is
/// verified, and each of AC-4's tamperings is rejected, unverifiable or
/// unencodable at its part.
#[trace("TC-893", "FR-314-AC-4")]
#[test]
fn refinement_certificates_check_part_by_part() {
    let world = World::new(&cas_refines_counter());
    with_request(&world, |request| {
        let valid = certificate(request, 7);
        assert_eq!(check(request, &valid), Ok(SmtProofCheck::Verified));

        let mut short = valid.clone();
        short.steps.pop();
        assert_eq!(
            check(request, &short),
            rejected(
                CertificateRule::ShapeMismatch,
                CertificateLocus::Query { part: step_part(6) }
            )
        );
        let mut long = valid.clone();
        long.steps.push(valid.steps[0].clone());
        assert_eq!(
            check(request, &long),
            rejected(
                CertificateRule::ShapeMismatch,
                CertificateLocus::Query { part: step_part(7) }
            )
        );

        let mut swapped = valid.clone();
        std::mem::swap(&mut swapped.base, &mut swapped.steps[0]);
        assert_eq!(
            check(request, &swapped),
            rejected(
                CertificateRule::QueryMismatch,
                CertificateLocus::Query {
                    part: QueryPart::Base
                }
            )
        );

        let mut replaced = valid.clone();
        replaced.steps[2].query = valid.steps[3].query.clone();
        assert_eq!(
            check(request, &replaced),
            rejected(
                CertificateRule::QueryMismatch,
                CertificateLocus::Query { part: step_part(2) }
            )
        );

        let mut invalid = valid.clone();
        invalid.steps[4].proof = with_command(
            &valid.steps[4].proof,
            3,
            "(step t9 (cl false) :rule and :premises (a0))",
        );
        assert_eq!(
            check(request, &invalid),
            rejected(
                CertificateRule::ProofStepInvalid,
                CertificateLocus::ProofStep {
                    part: step_part(4),
                    index: 3
                }
            )
        );

        let mut hole = valid.clone();
        hole.base.proof = with_command(&valid.base.proof, 3, "(step t1 (cl) :rule hole)");
        assert_eq!(
            check(request, &hole),
            Ok(SmtProofCheck::Unverifiable {
                part: QueryPart::Base,
                step: 3
            })
        );
    });

    let add = World::with(Variant::ADD, &cas_refines_counter_add());
    let bump = row_of(&add, "bump");
    with_request(&add, |request| {
        let mut unencodable = certificate(request, 8);
        unencodable.steps[usize::try_from(bump).expect("small")] =
            part("anything".to_owned(), "(not a proof".to_owned());
        assert_eq!(
            check(request, &unencodable),
            Ok(SmtProofCheck::Unencodable {
                part: step_part(bump)
            })
        );
    });
}

/// Part of FR-314-AC-5 (TC-1165): `CasRefinesCounter`'s certificate checked
/// with `max_certificate_bytes` 1 stops naming `max_certificate_bytes`, and
/// twice under one budget gives one result; a cancelled handle stops it.
#[trace("TC-1165")]
#[test]
fn a_refinement_check_stops_at_its_byte_budget() {
    let world = World::new(&cas_refines_counter());
    with_request(&world, |request| {
        let certificate = certificate(request, 7);
        let limits = CertificateCheckLimits {
            max_certificate_bytes: 1,
            ..CertificateCheckLimits::default()
        };
        let stopped = Ok(SmtProofCheck::Stopped(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::CertificateMaxBytes,
            value: 1,
        }));
        for _ in 0..2 {
            assert_eq!(
                check_refinement_smt_proof(request, &certificate, &limits, &Cancel::new()),
                stopped
            );
        }
        let cancel = Cancel::new();
        cancel.cancel(quire_exact::CancelCause::Requested);
        assert_eq!(
            check_refinement_smt_proof(
                request,
                &certificate,
                &CertificateCheckLimits::default(),
                &cancel
            ),
            Ok(SmtProofCheck::Stopped(IncompleteCause::Cancelled {
                source: quire_exact::CancelCause::Requested
            }))
        );
    });
}
