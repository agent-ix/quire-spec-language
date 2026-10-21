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
