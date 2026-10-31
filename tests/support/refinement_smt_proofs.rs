// SPDX-License-Identifier: AGPL-3.0-or-later
//! Hand-written Alethe refutations of ADR-020 §8's `CasRefinesCounter`
//! FR-315 refinement queries, and the FR-314 certificate they make: shared
//! by the replay crate's checker tests and the analyze crate's settlement
//! tests.
#![allow(dead_code, reason = "each including test crate uses a subset")]

use qsl_eval::model_subject::ModelSubject;
use qsl_eval::smt::{
    encode_refinement_query, RefinementCertificateRequest, RefinementQueryKind, SmtEncodingRefusal,
};
use qsl_replay::{ProofFormat, RefinementSmtCertificate, RefutedQuery};

use crate::refinement_fixture::{World, SPEC, UNLIMITED};

/// The subjects of `world`'s refinement.
pub(crate) fn subjects(world: &World) -> (ModelSubject<'_>, ModelSubject<'_>) {
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
pub(crate) fn with_request<T>(
    world: &World,
    body: impl FnOnce(&RefinementCertificateRequest<'_>) -> T,
) -> T {
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

/// The declaration position of the step row whose class is spelled
/// `spelling`.
pub(crate) fn row_of(world: &World, spelling: &str) -> u64 {
    let position = world
        .refinement()
        .steps()
        .iter()
        .position(|row| row.spelling().ends_with(spelling))
        .expect("the row exists");
    u64::try_from(position).expect("a small position")
}

pub(crate) fn encode(
    request: &RefinementCertificateRequest<'_>,
    kind: RefinementQueryKind,
) -> Result<String, SmtEncodingRefusal> {
    encode_refinement_query(request, kind)
        .map(|bytes| String::from_utf8(bytes).expect("a script is UTF-8"))
}

/// The terms of `query`'s assertions, in order.
pub(crate) fn assertions(query: &str) -> Vec<&str> {
    query
        .lines()
        .filter_map(|line| line.strip_prefix("(assert ")?.strip_suffix(')'))
        .collect()
}

/// `(step id (cl conjunct) :rule and :premises (premise) :args (k))`,
/// `k` the position of `conjunct` among the operands of `whole`.
pub(crate) fn and_step(id: &str, conjunct: &str, premise: &str, whole: &str) -> String {
    let k = operands(whole)[1..]
        .iter()
        .position(|operand| *operand == conjunct)
        .unwrap_or_else(|| panic!("{conjunct} is a conjunct of {whole}"));
    format!("(step {id} (cl {conjunct}) :rule and :premises ({premise}) :args ({k}))")
}

const RANGE: &str = "(and (<= 0 s0.c.value) (<= s0.c.value 2))";

/// The steps that refute the `(not true)` of a refuted `(or (not D) (not
/// R))` whose `D` is `true`: `tautology` over `not_not`'s clause, whose two
/// literals are complementary.
const TRUE: &str = "(step u0 (cl (not (not (not true))) true) :rule not_not)\n\
     (step u1 (cl true) :rule tautology :premises (u0))\n";

/// A refutation of the `Initial` query: its initial predicate fixes
/// `s0.c.value` at 0, which `(or (not true) (not (= s0.c.value 0)))` denies.
pub(crate) fn initial_proof(query: &str) -> String {
    let asserted = assertions(query);
    let [.., initial, negation] = asserted.as_slice() else {
        panic!("an initial query has two assertions after its ranges");
    };
    let t0 = and_step("t0", "(= s0.c.value 0)", "a0", initial);
    format!(
        "(assume a0 {initial})\n\
         (assume a1 {negation})\n\
         {t0}\n\
         (step t1 (cl (not true) (not (= s0.c.value 0))) :rule or :premises (a1))\n\
         {TRUE}\
         (step t2 (cl) :rule resolution :premises (t1 u1 t0))\n"
    )
}

const UNCHANGED: &str = "(= s1.c.value s0.c.value)";

/// A refutation of a `stutter` row's query `(and ... Q ... (or (not true)
/// (not Q)))`, `Q` the frame equality of `value`.
pub(crate) fn stutter_proof(assertion: &str) -> String {
    let t0 = and_step("t0", UNCHANGED, "a0", assertion);
    let refuted = format!("(or (not true) (not {UNCHANGED}))");
    let t1 = and_step("t1", &refuted, "a0", assertion);
    format!(
        "(assume a0 {assertion})\n\
         {t0}\n\
         {t1}\n\
         (step t2 (cl (not true) (not {UNCHANGED})) :rule or :premises (t1))\n\
         {TRUE}\
         (step t3 (cl) :rule resolution :premises (t2 u1 t0))\n"
    )
}

/// A refutation of worker `w`'s `commit` row query: the concrete commit
/// writes `value = tmp + 1` with `tmp = value`, so `inc` holds over the
/// mapped pair, by linear arithmetic over the range of `s0.c.value`.
pub(crate) fn commit_proof(assertion: &str, w: char) -> String {
    let rng = "(and (<= 0 (+ s0.c.value 1)) (<= (+ s0.c.value 1) 2))";
    let k = "(and (< s0.c.value 2) (= s1.c.value (+ s0.c.value 1)))";
    let implication = format!("(=> (< s0.c.value 2) {rng})");
    let m = format!("(and {implication} {k})");
    let n = format!("(and (< s0.c.value 2) {m})");
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
        and_step("c0", &format!("(or (not true) (not {n}))"), "a0", assertion),
        format!("(step c1 (cl (not true) (not {n})) :rule or :premises (c0))"),
        and_step("c2", &pre, "a0", assertion),
        and_step("c3", &post, "a0", assertion),
        and_step("c4", &format!("(< {v} 2)"), "c2", &pre),
        and_step("c5", &format!("(and {busy} (= {tmp} {v}))"), "c2", &pre),
        and_step("c6", &format!("(= {tmp} {v})"), "c5", &format!("(and {busy} (= {tmp} {v}))")),
        and_step("c7", &written, "c3", &post),
        and_step("c8", &format!("(and (< {tmp} 2) (= {v1} (+ {tmp} 1)))"), "c7", &written),
        and_step("c9", &format!("(= {v1} (+ {tmp} 1))"), "c8", &format!("(and (< {tmp} 2) (= {v1} (+ {tmp} 1)))")),
        and_step("c10", &format!("(<= 0 {v})"), "r", RANGE),
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
        TRUE.trim_end().to_owned(),
        "(step c29 (cl) :rule resolution :premises (c28 c1 u1))".to_owned(),
    ]
    .join("\n")
}

/// A refutation of a step row's query, by the row's kind.
pub(crate) fn step_proof(query: &str) -> String {
    let assertion = *assertions(query).last().expect("a step query asserts");
    if assertion.contains("(= s1.c.value (+ s0.c.tmpA 1))") {
        commit_proof(assertion, 'A')
    } else if assertion.contains("(= s1.c.value (+ s0.c.tmpB 1))") {
        commit_proof(assertion, 'B')
    } else {
        stutter_proof(assertion)
    }
}

pub(crate) fn part(query: String, proof: String) -> RefutedQuery {
    RefutedQuery {
        query: query.into_bytes(),
        proof: proof.into_bytes(),
    }
}

/// The certificate of `request` with every part's FR-315 query and a
/// hand-written refutation; a part whose query FR-315 refuses carries
/// `refused`.
pub(crate) fn certificate(
    request: &RefinementCertificateRequest<'_>,
    rows: u64,
) -> RefinementSmtCertificate {
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

/// The top-level operands of the S-expression `(head a b ...)`.
pub(crate) fn operands(term: &str) -> Vec<&str> {
    let inner = &term[1..term.len() - 1];
    let mut operands = Vec::new();
    let (mut depth, mut start) = (0_usize, None);
    for (at, character) in inner.char_indices() {
        match character {
            '(' => {
                if depth == 0 && start.is_none() {
                    start = Some(at);
                }
                depth += 1;
            }
            ')' => depth -= 1,
            ' ' if depth == 0 => {
                if let Some(from) = start.take() {
                    operands.push(&inner[from..at]);
                }
            }
            _ => {
                if start.is_none() {
                    start = Some(at);
                }
            }
        }
    }
    if let Some(from) = start {
        operands.push(&inner[from..]);
    }
    operands
}
