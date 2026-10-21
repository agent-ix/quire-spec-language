---
id: SR-13652
title: 'Spec review of PR #1138: QSpec consumer alignment, failure domain'
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@fd5cd6d5e; git diff origin/main...fd5cd6d5e (PR #1138, branch qsl-spec-std-consumers, base main 5c4a96692): FR-057, FR-125, FR-134, FR-146, FR-153, FR-161, FR-218, FR-314, FR-337, TC-535, TC-893, TC-1056, TC-1079; read against origin/main code (qsl-route/src/lib.rs, qsl-route/tests/it/fairness_kinds.rs, qsl-semantics/src/check/protocol_requirement.rs, protocol_relationship.rs, refusal.rs, qsl-cst/src/grammar.rs, qsl-replay/src/certificate.rs), QSL FR-136, FR-315, FR-338, and quire-specification main fd7a694 (FR-468, FR-375) and open PRs #313 (9f0d0c8) and #314 (7722857); tickets STD-197, STD-193, STD-194, STD-186, STD-217, QSL-877"
review_set: subset
date: '2026-10-21'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-057
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-125
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-134
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-146
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-153
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-161
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-218
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-314
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-337
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-535
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-893
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-1056
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-1079
  type: reviews
---

## Summary

Ticket: STD-197 (also STD-193, STD-194, STD-186, STD-217, QSL-877). PR quire-spec-language#1138, head fd5cd6d5e. Failure-domain lens: unstated outcomes, unrepresentable cases and check-order gaps in the new FR-314 certificate variant, the FR-218 requirement binding and the FR-134 precedence.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-314 description | spec/functional/FR-314-check-an-smt-proof-certificate.md | examined |
| FR-314 signature | spec/functional/FR-314-check-an-smt-proof-certificate.md | examined |
| FR-314 variant shape | spec/functional/FR-314-check-an-smt-proof-certificate.md | examined |
| FR-314 refinement shape | spec/functional/FR-314-check-an-smt-proof-certificate.md | examined |
| FR-314 verified | spec/functional/FR-314-check-an-smt-proof-certificate.md | examined |
| FR-314-AC-3 | spec/functional/FR-314-check-an-smt-proof-certificate.md | examined |
| TC-893 step 4 | spec/test-cases/TC-893-an-smt-proof-certificate-is-checked.md | examined |
| FR-218 requirement binding | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md | examined |
| FR-218 not emitted | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md | examined |
| FR-218-AC-16 | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md | examined |
| FR-134 precedence | spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md | examined |
| FR-134 non-temporal | spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md | examined |
| FR-337 over node | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md | examined |
| TC-1079 step 3 | spec/test-cases/TC-1079-a-temporal-clause-node-carries-its-root-and-over-node.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | No basis selects RefinementInductive, so the existing variant rule rejects every refinement certificate. check_smt_proof takes 'basis: &ProofBasis // BoundedComplete or Inductive, with depth' and one CertificateRequest for one item, and the first Behavior bullet rejects ShapeMismatch 'a certificate whose variant differs from the basis: BoundedComplete carries one unrolling query and Inductive carries a base query and a step query'. An RE-2 record settles with basis Inductive{depth: 1} (FR-146-AC-4), so a RefinementInductive certificate arriving under that basis is rejected ShapeMismatch at Base by the first bullet before the new bullet applies. The PR also does not say which request the eight-part certificate is checked against (the refinement item, or each RE-2 record). State how the checker knows it is checking a refinement's per-step obligations (a basis variant, or the request kind), amend the variant bullet, and say which CertificateRequest it receives. | spec/functional/FR-314-check-an-smt-proof-certificate.md:88-92; spec/functional/FR-314-check-an-smt-proof-certificate.md:61-65; spec/functional/FR-314-check-an-smt-proof-certificate.md:93-96; spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md:181 |
| FND-002 | high | FR-314-AC-3's ShapeMismatch loci cannot be derived from the stated type and rules. The variant is 'RefinementInductive { format, base: RefutedQuery, steps: Vec<RefutedQuery> }': base is not optional, so 'a certificate with no base part' cannot occur, and a part carries no row tag, so 'whose first part is a step part' is visible only by comparing base's query with the initial query, which the next bullet calls QueryMismatch, not ShapeMismatch. With seven rows, 'the fourth step part missing' is a Vec of six; a count check finds the first missing row at 6, not 3, and finding 3 needs a query comparison of each part, which again is QueryMismatch at Step row 3. The order of the shape and query checks is not stated. Two implementations will return different rules and rows for AC-3's three adverse vectors. Either tag each part with its row (and state the check order), or define the shape check as a count check and set AC-3 to ShapeMismatch at Step row 6 for the missing part, row 7 for the extra one, and QueryMismatch at Base for a step part first. | spec/functional/FR-314-check-an-smt-proof-certificate.md:150; spec/functional/FR-314-check-an-smt-proof-certificate.md:96-100; spec/functional/FR-314-check-an-smt-proof-certificate.md:58; spec/test-cases/TC-893-an-smt-proof-certificate-is-checked.md:33-36 |
| FND-003 | medium | FR-314's Verified and Description rules are not extended to RefinementInductive. 'The checker SHALL return Verified only when every carried proof refutes its query using checked rules alone, both the base and the step for Inductive' and the Description ('an SmtProofCertificate that comes with an SMT backend's BoundedComplete{depth} or Inductive{depth} proof') still cover only the two old variants. Nothing states that Verified for RefinementInductive needs the base and every step part, or which part and row an Unverifiable for it names. Add 'the base and every step part for RefinementInductive' and extend the Description. | spec/functional/FR-314-check-an-smt-proof-certificate.md:122-124; spec/functional/FR-314-check-an-smt-proof-certificate.md:21-29 |
| FND-004 | low | The order of a not-emitted refusal and requirement refusals in one unit is unstated. FR-218 says S3 decides every requirement refusal 'whether or not S4 emits the required clause's kind' and that a not-emitted clause 'refuses the unit at that clause'. A unit with both (requires temporal Nothing; and a not-emitted clause) yields two refusals, but FR-218 does not say whether both are reported or in which order (source order, or requirement refusals first). State the order, and use it in the TC-1056 row asked for in base FND-002. | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:265-269; spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:419 |

## Verdict

Not mergeable as it stands. FND-001 and FND-002 are high: as written, no RefinementInductive certificate reaches the new rules, and the AC-3 adverse loci depend on a check order the spec does not give. The fairness precedence (absent, not-a-list, unknown-label, duplicate-label; first offending element) is total and matches the code's unknown-before-repeat order. The non-temporal ignore rule matches QSpec #313.

## New findings (disposition pass 1)

Head 5e73cbeb1 (PR #1138 round 1).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | FR-338's CertificateRequest can carry neither a refinement item nor the abstract subject the refinement queries need. check_refinement_smt_proof takes 'the refinement item's CertificateRequest (the recompiled package and the refinement node)'. FR-338 defines CertificateRequest as the recompiled package, one model subject re-admitted by FR-120, and an item that is 'a TP-1, TP-2 or TP-3 clause, or the deadlock-freedom item'. It has no refinement item and no second subject. FR-315's encode_refinement_query(concrete, abstract_, refinement, kind) needs both the concrete and the abstract subject. As written, the request the new entry takes cannot be built. Extend FR-338's CertificateRequest with a refinement item and its abstract subject, or define a RefinementCertificateRequest in FR-314. | spec/functional/FR-314-check-an-smt-proof-certificate.md:79-82; spec/functional/FR-314-check-an-smt-proof-certificate.md:133-139; spec/functional/FR-338-check-an-en-1-closure-certificate.md:37-41; spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:113-121 |
| FND-006 | medium | An abstract-operation step query with `_` arguments needs a quantifier the QF_LIA script cannot print. For an abstract-operation row, the Step query asserts the negation of the abstract transition holding 'with `_` arguments existentially quantified over their domains'. Negating an existential gives a universal quantifier inside an asserted formula, but FR-315 prints every query as `(set-logic QF_LIA)` (lines 71 and 105). The out-of-fragment refusal names mapping rows, receivers and arguments, not this case. Two implementations will diverge: one expands a finite domain, one refuses, one drops the quantifier, which is unsound. FR-315-AC-6 tests only a `stutter` row, so no vector covers an abstract-operation row, `_` or not. State the encoding (finite-domain expansion, or `_` refuses SmtEncodingRefusal) and add an abstract-operation row and a `_` row to AC-6 and TC-900 step 6. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:129-139; spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:71; spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:105; spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:153 |
| FND-007 | low | FR-314-AC-4's ProofStepInvalid vector names no index. FR-314-AC-4 expects `ProofStepInvalid` at `ProofStep { part: Step { row: Some(4) }, index }` and TC-893 step 5 says 'at the fifth step part'. Neither gives the index value, so a checker that miscounts commands passes. This is the gap #672's review recorded for AC-2. Give the index (after #672's command-number definition lands) or fix the invalid step's position in the vector. | spec/functional/FR-314-check-an-smt-proof-certificate.md:183; spec/test-cases/TC-893-an-smt-proof-certificate-is-checked.md:50-55 |

## Dispositions

Round 1, head 5e73cbeb1, checked against origin/main 5c4a96692 code, QSL FR-315/FR-338, QSpec main fd7a694 and QSpec #313 648d2d0 / #314 13bd4aa.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Fixed in 5e73cbeb1: ### Refinement certificates  `check_refinement_smt_proof` is the entry for a refinement item whose safety half is settled from FR-146's records. It takes the item's `CertificateRequest` (the recompiled package and the refinement node) and no `ProofBasis`: each record still settles `Inductive{depth: 1}` (FR-146), and the basis variant rules above do not apply to this entry. Let `N` be the number of step rows of the recompiled refinement's `steps` array.  |
| FND-002 | fixed | Fixed in 5e73cbeb1: - The checker SHALL first reject with `ShapeMismatch` a certificate whose `steps` holds fewer or more than `N` parts: at `Step { row: Some(len) }` for fewer, `len` being the number of parts present, and at `Step { row: Some(N) }` for more. - The checker SHALL then encode the expected queries with FR-315's refinement queries (the initial query for `base`, the step query of row `r` for `steps[r]`) and SHALL reject with `QueryMismatch`, at `Base` or at `Step { row: Some(r) }`, the first part in the order `base`, then rows `0..N`, whose carried query differs byte for byte. A step part carried in t |
| FND-003 | fixed | Fixed in 5e73cbeb1: For a refinement's per-step simulation obligations (QSpec FR-468), `check_refinement_smt_proof` SHALL accept a `RefinementSmtCertificate` exactly when it holds one base part and one step part per step row, each the query FR-315 encodes for it, and every part refutes its query using only checked Alethe rules. / - The checker SHALL return `Verified` only when the base and every step part refute their queries using checked rules alone. |
| FND-004 | fixed | Fixed in 5e73cbeb1: The not-emitted refusal is removed from FR-218, so no unit has both: - S3 SHALL bind a requirement by position for every temporal clause, whatever its activation or captures, and SHALL decide the refusals below at S3 before S4 emits any node, so a unit with a requirement refusal reports it whatever its other clauses hold. FR-337 specifies the emission of every checked clause, from which S4 mints the target key. |
