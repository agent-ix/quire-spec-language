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

/// A refutation of the `Initial` query: its initial predicate fixes
/// `s0.c.value` at 0, which `(not (= s0.c.value 0))` denies.
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
         (step t1 (cl) :rule resolution :premises (a1 t0))\n"
    )
}

const UNCHANGED: &str = "(= s1.c.value s0.c.value)";

/// A refutation of a `stutter` row's query `(and ... Q ... (not Q))`, `Q`
/// the frame equality of `value`.
pub(crate) fn stutter_proof(assertion: &str) -> String {
    let t0 = and_step("t0", UNCHANGED, "a0", assertion);
    let t1 = and_step("t1", &format!("(not {UNCHANGED})"), "a0", assertion);
    format!(
        "(assume a0 {assertion})\n\
         {t0}\n\
         {t1}\n\
         (step t2 (cl) :rule resolution :premises (t1 t0))\n"
    )
}

/// A refutation of worker `w`'s `commit` row query: the concrete commit
/// writes `value = tmp + 1` with `tmp = value`, so `inc` holds over the
/// mapped pair, by linear arithmetic over the range of `s0.c.value`.
pub(crate) fn commit_proof(assertion: &str, w: char) -> String {
    let v1 = "s1.c.value";
    let v = "s0.c.value";
    let tmp = format!("s0.c.tmp{w}");
    let rng = format!("(and (<= (- 1) {v}) (<= {v} 1))");
    let implication = format!("(or (not (< {v} 2)) {rng})");
    let next = format!("(= {v1} (+ {v} 1))");
    let n = format!("(and (< {v} 2) {implication} {next})");
    [
        format!("(assume a0 {assertion})"),
        format!("(assume r {RANGE})"),
        and_step("c0", &format!("(not {n})"), "a0", assertion),
        and_step("c1", &format!("(< {v} 2)"), "a0", assertion),
        and_step("c2", &format!("(= {tmp} {v})"), "a0", assertion),
        and_step("c3", &format!("(= {v1} (+ {tmp} 1))"), "a0", assertion),
        and_step("c4", &format!("(<= 0 {v})"), "r", RANGE),
        format!(
            "(step c5 (cl {next} (not (<= {v1} (+ {v} 1))) (not (<= (+ {v} 1) {v1}))) :rule la_disequality)"
        ),
        format!(
            "(step c6 (cl (<= {v1} (+ {v} 1)) (not (= {v1} (+ {tmp} 1))) (not (= {tmp} {v}))) :rule la_generic :args (1 (- 1) (- 1)))"
        ),
        format!(
            "(step c7 (cl (<= (+ {v} 1) {v1}) (not (= {v1} (+ {tmp} 1))) (not (= {tmp} {v}))) :rule la_generic :args (1 1 1))"
        ),
        format!("(step c8 (cl {next}) :rule resolution :premises (c5 c6 c7 c3 c2))"),
        format!("(step c9 (cl (<= (- 1) {v}) (not (<= 0 {v}))) :rule la_generic :args (1 1))"),
        format!("(step c10 (cl (<= {v} 1) (not (< {v} 2))) :rule la_generic :args (1 1))"),
        format!("(step c11 (cl (<= (- 1) {v})) :rule resolution :premises (c9 c4))"),
        format!("(step c12 (cl (<= {v} 1)) :rule resolution :premises (c10 c1))"),
        format!("(step c13 (cl {rng} (not (<= (- 1) {v})) (not (<= {v} 1))) :rule and_neg)"),
        format!("(step c14 (cl {rng}) :rule resolution :premises (c13 c11 c12))"),
        format!("(step c15 (cl {implication} (not {rng})) :rule or_neg :args (1))"),
        format!("(step c16 (cl {implication}) :rule resolution :premises (c15 c14))"),
        format!(
            "(step c17 (cl {n} (not (< {v} 2)) (not {implication}) (not {next})) :rule and_neg)"
        ),
        format!("(step c18 (cl {n}) :rule resolution :premises (c17 c1 c16 c8))"),
        "(step c19 (cl) :rule resolution :premises (c18 c0))".to_owned(),
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
