// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315 "Refinement queries" (TC-900) and FR-314 "Refinement certificates"
//! (TC-893, TC-1165): `encode_refinement_query` and
//! `check_refinement_smt_proof` over ADR-020 §8's `CasRefinesCounter`, with
//! hand-written Alethe refutations of its queries; and FR-146's settlement of
//! the per-step simulation records and the safety half from them (TC-551).

use ix_trace_rs::trace;
use qsl_eval::smt::{
    RefinementCertificateRequest, RefinementQueryKind, Refusal, SmtEncodingRefusal,
};
use qsl_replay::{
    certify_step_records, check_refinement_smt_proof, settle_step_records, CertificateCheckLimits,
    CertificateLocus, CertificateRejection, CertificateRule, Certification, IncompleteCause,
    InconclusiveCause, ItemLabel, ModelCheckLimit, OtherBackendResult, ProofBasis, ProofFormat,
    QueryPart, RefinementFailure, RefinementSmtCertificate, RefinementTraceSettlement,
    SettlementBasis, SmtProofCheck, StepCertification, StepRecordVerdict, StepRecordsSettlement,
    TerminalValue, UnavailabilityCause,
};
use quire_exact::Cancel;

use crate::refinement_fixture::{
    cas_refines_counter, cas_refines_counter_add, mutated, Variant, World,
};
use crate::refinement_proofs::{
    assertions, certificate, encode, operands, part, row_of, with_request,
};

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
             (assert (or (not true) (not (= s0.c.value 0))))\n\
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
                "(and (= s1.c.busyA s0.c.busyA) (= s1.c.busyB s0.c.busyB) \
                 (= s1.c.tmpA s0.c.tmpA) (= s1.c.tmpB s0.c.tmpB) (= s1.c.value s0.c.value) \
                 (or (not true) (not (= s1.c.value s0.c.value))))"
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
                "(and (and (and s0.c.busyA (= s0.c.tmpA s0.c.value)) (< s0.c.value 2)) \
                 (and (=> (< s0.c.tmpA 2) (and (<= 0 (+ s0.c.tmpA 1)) (<= (+ s0.c.tmpA 1) 2))) \
                 (and (and (< s0.c.tmpA 2) (= s1.c.value (+ s0.c.tmpA 1))) (not s1.c.busyA))) \
                 (= s1.c.busyB s0.c.busyB) (= s1.c.tmpA s0.c.tmpA) (= s1.c.tmpB s0.c.tmpB) \
                 (or (not true) (not (and (< s0.c.value 2) (and (=> (< s0.c.value 2) \
                 (and (<= 0 (+ s0.c.value 1)) (<= (+ s0.c.value 1) 2))) \
                 (and (< s0.c.value 2) (= s1.c.value (+ s0.c.value 1))))))))"
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
        hole.base.proof = with_command(
            &valid.base.proof,
            3,
            "(step t1 (cl (not true) (not (= s0.c.value 0))) :rule hole)",
        );
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

const AA_INV: &str = "invariant AaInv using inf on Impl::Counter at current { self.tmpA <= 2 }\n";
const CAS_INV: &str = "invariant CasInv using inf on Impl::Counter at current { \
     self.busyA or self.tmpA <= self.value }\n";
const ABS_INV: &str =
    "invariant AbsInv using inf on Spec::Counter at current { self.value <= 2 }\n";

/// TC-900's `CasRefinesCounter2`: two counters `c0` < `c1`, with `clauses`
/// after the refinement.
fn counter2(clauses: &str) -> World {
    World::with_keys(
        Variant::CAS,
        &format!("{}\n{clauses}", cas_refines_counter()),
        &["c0", "c1"],
    )
}

/// The command heads of `script`, in order.
fn heads(script: &str) -> Vec<&str> {
    script
        .lines()
        .map(|line| line[1..].split([' ', ')']).next().unwrap_or_default())
        .collect()
}

/// FR-315-AC-53 (TC-900): the refinement queries' normative layout over
/// `CasRefinesCounter2`, and each layout mutation rejected `QueryMismatch`.
#[trace("TC-900", "FR-315-AC-53")]
#[test]
fn refinement_queries_print_in_the_normative_order() {
    let world = counter2(&format!("{AA_INV}{CAS_INV}{ABS_INV}"));
    let commit = row_of(&world, "commitA");
    let (initial, step) = with_request(&world, |request| {
        (
            encode(request, RefinementQueryKind::Initial).expect("encodes"),
            encode(request, RefinementQueryKind::Step { row: commit }).expect("encodes"),
        )
    });
    let declarations = 10;
    let ranges = 6;
    let mut expected = vec!["set-logic"];
    expected.extend(std::iter::repeat_n("declare-const", declarations));
    expected.extend(std::iter::repeat_n("assert", ranges + 1 + 4 + 1));
    expected.push("check-sat");
    assert_eq!(heads(&initial), expected);
    let declared: Vec<&str> = initial
        .lines()
        .filter_map(|line| line.strip_prefix("(declare-const "))
        .map(|rest| rest.split(' ').next().unwrap_or_default())
        .collect();
    assert_eq!(
        declared,
        [
            "s0.c0.busyA",
            "s0.c0.busyB",
            "s0.c0.tmpA",
            "s0.c0.tmpB",
            "s0.c0.value",
            "s0.c1.busyA",
            "s0.c1.busyB",
            "s0.c1.tmpA",
            "s0.c1.tmpB",
            "s0.c1.value",
        ]
    );
    let hypotheses = [
        "(<= s0.c0.tmpA 2)",
        "(<= s0.c1.tmpA 2)",
        "(or s0.c0.busyA (<= s0.c0.tmpA s0.c0.value))",
        "(or s0.c1.busyA (<= s0.c1.tmpA s0.c1.value))",
    ];
    let asserted = assertions(&initial);
    assert_eq!(asserted[ranges + 1..ranges + 5], hypotheses);
    assert!(asserted[ranges + 5].starts_with("(or (not true) (not "));

    let step_asserted = assertions(&step);
    assert_eq!(step_asserted.len(), 2 * ranges + 4 + 1);
    assert_eq!(step_asserted[2 * ranges..2 * ranges + 4], hypotheses);
    assert!(
        !step.contains("(= s0.c0.busyA false)"),
        "no initial predicate"
    );
    let row = step_asserted[2 * ranges + 4];
    let applications = operands(row);
    assert_eq!(applications.len(), 3, "(or <c0> <c1>)");
    assert_eq!(applications[0], "or");
    assert!(applications[1].contains("s0.c0.busyA") && !applications[1].contains("s0.c1.busyA"));
    assert!(applications[2].contains("s0.c1.busyA"));
    for application in &applications[1..] {
        let conjuncts = operands(application);
        assert!(conjuncts
            .last()
            .is_some_and(|last| last.starts_with("(or (not true) (not ")));
    }
    with_request(&world, |request| {
        assert_eq!(
            encode(request, RefinementQueryKind::Initial).expect("encodes"),
            initial
        );
        assert_eq!(
            encode(request, RefinementQueryKind::Step { row: commit }).expect("encodes"),
            step
        );
    });

    // No digest-derived order: renaming moves the clause, source order
    // changes nothing.
    let renamed = counter2(&format!("{}{CAS_INV}", AA_INV.replace("AaInv", "ZzInv")));
    let renamed_initial = with_request(&renamed, |request| {
        encode(request, RefinementQueryKind::Initial)
    })
    .expect("encodes");
    assert_eq!(
        assertions(&renamed_initial)[ranges + 1..ranges + 5],
        [hypotheses[2], hypotheses[3], hypotheses[0], hypotheses[1]]
    );
    let reversed = counter2(&format!("{ABS_INV}{CAS_INV}{AA_INV}"));
    assert_eq!(
        with_request(&reversed, |request| encode(
            request,
            RefinementQueryKind::Initial
        ))
        .expect("encodes"),
        initial
    );

    // Each mutation is rejected `QueryMismatch` at the row's part.
    let replace = |from: &str, to: &str| step.replacen(from, to, 1);
    let line = |term: &str| format!("(assert {term})\n");
    let initial_predicate = assertions(&initial)[ranges];
    let mutations = [
        replace(
            row,
            &format!("(or {} {})", applications[2], applications[1]),
        ),
        replace(
            &format!("{}{}", line(hypotheses[1]), line(hypotheses[2])),
            &format!("{}{}", line(hypotheses[2]), line(hypotheses[1])),
        )
        .replacen(&line(hypotheses[0]), &line(hypotheses[2]), 1)
        .replacen(&line(hypotheses[3]), &line(hypotheses[1]), 1),
        replace(&line(hypotheses[0]), &line("(<= s1.c0.tmpA 2)")),
        replace(
            &line(hypotheses[0]),
            &format!("{}{}", line(initial_predicate), line(hypotheses[0])),
        ),
        replace(
            &line(row),
            &format!("{}{}", line(applications[1]), line(applications[2])),
        ),
        replace(
            &line(row),
            &format!(
                "{}{}",
                line(
                    &format!("(or {} {})", applications[1], applications[2])
                        .replace("(or (not true) (not ", "(and true (")
                ),
                line("(not (or R_c0 R_c1))")
            ),
        ),
    ];
    with_request(&world, |request| {
        let rows = u64::try_from(
            world
                .refinement()
                .v2_emission()
                .expect("emits")
                .step_sources
                .len(),
        )
        .expect("small");
        let honest = |row: u64| {
            part(
                encode(request, RefinementQueryKind::Step { row }).expect("encodes"),
                String::new(),
            )
        };
        for mutated in mutations {
            assert_ne!(mutated, step);
            let certificate = RefinementSmtCertificate {
                format: ProofFormat::Alethe,
                base: part(initial.clone(), String::new()),
                steps: (0..rows)
                    .map(|row| {
                        if row == commit {
                            part(mutated.clone(), String::new())
                        } else {
                            honest(row)
                        }
                    })
                    .collect(),
            };
            assert_eq!(
                check(request, &certificate),
                rejected(
                    CertificateRule::QueryMismatch,
                    CertificateLocus::Query {
                        part: step_part(commit)
                    }
                )
            );
        }
    });
}

/// FR-314: a refused initial query has no expected query, so the base part
/// is rejected `QueryMismatch`, never `Unencodable`.
#[trace("TC-893")]
#[test]
fn a_refused_initial_query_rejects_the_base_part() {
    let world = World::new(&mutated(
        &cas_refines_counter(),
        "value = self.value;",
        "value = if self.value * self.tmpA = 0 then self.value else self.tmpA;",
    ));
    with_request(&world, |request| {
        let certificate = RefinementSmtCertificate {
            format: ProofFormat::Alethe,
            base: part("(check-sat)\n".to_owned(), String::new()),
            steps: (0..7).map(|_| part(String::new(), String::new())).collect(),
        };
        assert_eq!(
            check(request, &certificate),
            rejected(
                CertificateRule::QueryMismatch,
                CertificateLocus::Query {
                    part: QueryPart::Base
                }
            )
        );
    });
}

fn proved(certification: Certification) -> TerminalValue {
    TerminalValue::Proved {
        basis: ProofBasis::Inductive { depth: 1 },
        certification,
    }
}

fn no_replay() -> Result<RefinementTraceSettlement, qsl_replay::ReplayRefusal> {
    panic!("only an initial counterexample is replayed")
}

/// The values of `settled`'s records.
fn values(settled: &StepRecordsSettlement) -> Vec<TerminalValue> {
    settled
        .records
        .iter()
        .map(|record| record.value.clone())
        .collect()
}

/// TC-551, the certificate part of FR-146-AC-4: every record of
/// `CasRefinesCounter` whose obligation holds settles `Certified` exactly when
/// FR-314 verifies the refinement's certificate, `Uncertified` with no
/// certificate, and every record of `CasRefinesCounterAdd`, whose `_` row
/// FR-315 refuses to encode, `Uncertified`.
#[trace("TC-551")]
#[test]
fn step_records_are_certified_only_by_a_verified_certificate() {
    let holds = vec![StepRecordVerdict::Holds; 8];
    let world = World::new(&cas_refines_counter());
    with_request(&world, |request| {
        let valid = certificate(request, 7);
        let limits = CertificateCheckLimits::default();
        let cancel = Cancel::new();
        for (certificate, certification) in [
            (Some(&valid), Certification::Certified),
            (None, Certification::Uncertified),
        ] {
            let checked = certify_step_records(request, certificate, 7, &limits, &cancel);
            let settled = settle_step_records(&holds, &checked, &[], no_replay);
            assert_eq!(values(&settled), vec![proved(certification); 8]);
            assert!(settled
                .records
                .iter()
                .all(|record| record.basis == SettlementBasis::DecisiveWitness));
            assert_eq!(settled.safety.value, proved(certification));
            assert_eq!(settled.safety.basis, SettlementBasis::DecisiveWitness);
        }
    });

    let add = World::with(Variant::ADD, &cas_refines_counter_add());
    let bump = usize::try_from(row_of(&add, "bump")).expect("small");
    with_request(&add, |request| {
        let mut unencodable = certificate(request, 8);
        unencodable.steps[bump] = part("anything".to_owned(), "(not a proof".to_owned());
        let checked = certify_step_records(
            request,
            Some(&unencodable),
            8,
            &CertificateCheckLimits::default(),
            &Cancel::new(),
        );
        let settled =
            settle_step_records(&vec![StepRecordVerdict::Holds; 9], &checked, &[], no_replay);
        assert_eq!(
            values(&settled),
            vec![proved(Certification::Uncertified); 9]
        );
        assert_eq!(settled.safety.value, proved(Certification::Uncertified));
    });
}

/// FR-146-AC-7 (TC-551): the settlement map fed FR-314's results directly.
/// A rejection at a part makes that part's record `CertificateRejected`
/// with FR-314's rule and locus, never `proved`, and every other record
/// `Uncertified`; an unverifiable certificate leaves every holding record
/// `Proved{Inductive{1}, Uncertified}`.
#[trace("TC-551", "FR-146-AC-7")]
#[test]
fn a_rejected_part_rejects_only_its_record() {
    let holds = vec![StepRecordVerdict::Holds; 8];
    let uncertified = proved(Certification::Uncertified);
    for (part, record) in [(QueryPart::Base, 0), (step_part(2), 3)] {
        let at = CertificateLocus::ProofStep { part, index: 3 };
        let rejection = CertificateRejection {
            rule: CertificateRule::ProofStepInvalid,
            at: at.clone(),
        };
        let checked = StepCertification::of_check(Some(Err(rejection)), 7);
        let settled = settle_step_records(&holds, &checked, &[], no_replay);
        let wrong = TerminalValue::Inconclusive(InconclusiveCause::CertificateRejected {
            rule: CertificateRule::ProofStepInvalid,
            at,
        });
        let mut expected = vec![uncertified.clone(); 8];
        expected[record] = wrong.clone();
        assert_eq!(values(&settled), expected, "rejected at {part:?}");
        assert_eq!(settled.records[record].basis, SettlementBasis::Unsettled);
        assert_eq!(settled.safety.value, wrong);
    }

    // A shape mismatch past the last row names no record: every record is
    // `Uncertified`, and the half is still rejected.
    let at = CertificateLocus::Query { part: step_part(7) };
    let checked = StepCertification::of_check(
        Some(Err(CertificateRejection {
            rule: CertificateRule::ShapeMismatch,
            at: at.clone(),
        })),
        7,
    );
    let settled = settle_step_records(&holds, &checked, &[], no_replay);
    assert_eq!(values(&settled), vec![uncertified.clone(); 8]);
    assert_eq!(
        settled.safety.value,
        TerminalValue::Inconclusive(InconclusiveCause::CertificateRejected {
            rule: CertificateRule::ShapeMismatch,
            at,
        })
    );

    for check in [
        SmtProofCheck::Unverifiable {
            part: step_part(1),
            step: 3,
        },
        SmtProofCheck::Unencodable { part: step_part(1) },
    ] {
        let checked = StepCertification::of_check(Some(Ok(check)), 7);
        let settled = settle_step_records(&holds, &checked, &[], no_replay);
        assert_eq!(values(&settled), vec![uncertified.clone(); 8]);
    }
}

/// TC-551, QSpec FR-468 "Settlement": the safety half takes the first rule
/// that applies, in the order initial counterexample, hypothesis item not
/// `proved`, step counterexample, certificate shown wrong, other backend
/// result, then the proof, `Certified` only when every record and every
/// hypothesis item is.
#[trace("TC-551")]
#[test]
fn the_safety_half_settles_by_the_first_rule_that_applies() {
    let hypothesis = quire_exact::NodeKey::from_digest([7; 32]);
    let certified = proved(Certification::Certified);
    let uncertified = proved(Certification::Uncertified);
    let verified = StepCertification::of_check(Some(Ok(SmtProofCheck::Verified)), 7);
    let rejected = StepCertification::of_check(
        Some(Err(CertificateRejection {
            rule: CertificateRule::QueryMismatch,
            at: CertificateLocus::Query { part: step_part(4) },
        })),
        7,
    );
    let holds = vec![StepRecordVerdict::Holds; 8];
    let mut step_failed = holds.clone();
    step_failed[5] = StepRecordVerdict::Counterexample;
    let mut initial_failed = holds.clone();
    initial_failed[0] = StepRecordVerdict::Counterexample;
    let mut absent = holds.clone();
    absent[2] = StepRecordVerdict::NoCandidate;
    let unproved = TerminalValue::Inconclusive(InconclusiveCause::InductionNotClosed { depth: 1 });
    let not_established =
        TerminalValue::Inconclusive(InconclusiveCause::HypothesesNotEstablished {
            hypothesis,
            result: ItemLabel::Inconclusive,
        });
    let induction = TerminalValue::Inconclusive(InconclusiveCause::InductionNotClosed { depth: 1 });
    let safety = |verdicts: &[StepRecordVerdict],
                  certification: &StepCertification,
                  item: &TerminalValue| {
        settle_step_records(
            verdicts,
            certification,
            &[(hypothesis, item.clone())],
            no_replay,
        )
        .safety
        .value
    };

    assert_eq!(safety(&holds, &verified, &certified), certified);
    assert_eq!(safety(&holds, &verified, &uncertified), uncertified);
    assert_eq!(safety(&holds, &verified, &unproved), not_established);
    assert_eq!(safety(&step_failed, &verified, &unproved), not_established);
    assert_eq!(safety(&step_failed, &verified, &certified), induction);
    assert_eq!(safety(&step_failed, &rejected, &certified), induction);
    assert_eq!(
        safety(&absent, &verified, &certified),
        TerminalValue::Unsupported(UnavailabilityCause::BackendAbsent)
    );
    assert!(matches!(
        safety(&holds, &rejected, &certified),
        TerminalValue::Inconclusive(InconclusiveCause::CertificateRejected { .. })
    ));

    // An initial counterexample comes first, and settles `refuted` only
    // through its replay.
    let reproduced = settle_step_records(
        &initial_failed,
        &verified,
        &[(hypothesis, unproved.clone())],
        || {
            Ok(RefinementTraceSettlement::Reproduced {
                failure: RefinementFailure::InitialNotAbstract { initial: 0 },
                mapped: Vec::new(),
            })
        },
    );
    assert_eq!(reproduced.safety.value, TerminalValue::Refuted);
    assert_eq!(
        reproduced.safety.basis,
        SettlementBasis::DecisiveCounterexample
    );
    // The `Initial` record itself never settles `refuted`.
    assert_eq!(reproduced.records[0].value, induction);
}

/// TC-551, QSpec FR-468 "Other backend results": a backend result that
/// neither holds nor fails settles its record as FR-331 records it, and the
/// safety half takes the first such record's result in record order, after
/// every rule before it.
#[trace("TC-551")]
#[test]
fn other_backend_results_settle_the_record_and_the_half() {
    let solver = TerminalValue::Inconclusive(InconclusiveCause::SolverInconclusive {
        trace: None,
        replay: None,
    });
    let timeout = TerminalValue::Incomplete(IncompleteCause::TimedOut);
    let mut verdicts = vec![StepRecordVerdict::Holds; 8];
    verdicts[2] = StepRecordVerdict::of_backend(&timeout).expect("an other result");
    verdicts[5] = StepRecordVerdict::of_backend(&solver).expect("an other result");
    verdicts[6] = StepRecordVerdict::NoCandidate;
    let verified = StepCertification::of_check(Some(Ok(SmtProofCheck::Verified)), 7);
    let settled = settle_step_records(&verdicts, &verified, &[], no_replay);
    assert_eq!(settled.records[2].value, timeout);
    assert_eq!(settled.records[2].basis, SettlementBasis::Unavailable);
    assert_eq!(settled.records[5].value, solver);
    assert_eq!(settled.records[5].basis, SettlementBasis::Unsettled);
    assert_eq!(
        settled.records[6].value,
        TerminalValue::Unsupported(UnavailabilityCause::BackendAbsent)
    );
    assert_eq!(settled.safety.value, timeout);

    // A step counterexample comes before any other result.
    verdicts[7] = StepRecordVerdict::Counterexample;
    assert_eq!(
        settle_step_records(&verdicts, &verified, &[], no_replay)
            .safety
            .value,
        TerminalValue::Inconclusive(InconclusiveCause::InductionNotClosed { depth: 1 })
    );

    assert_eq!(
        StepRecordVerdict::of_backend(&proved(Certification::Uncertified)),
        Some(StepRecordVerdict::Holds)
    );
    assert_eq!(
        StepRecordVerdict::of_backend(&TerminalValue::Refuted),
        Some(StepRecordVerdict::Counterexample)
    );
    assert_eq!(
        StepRecordVerdict::of_backend(&TerminalValue::Failed),
        Some(StepRecordVerdict::Other(OtherBackendResult::Failed))
    );
    assert_eq!(StepRecordVerdict::of_backend(&TerminalValue::Tested), None);
}
