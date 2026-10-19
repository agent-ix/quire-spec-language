---
id: SR-13322
title: "Spec review of PR #1050: base review"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@c5407fabd; git diff origin/main...HEAD (PR #1050)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-146
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-541
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-551
    type: reviews
---

## Summary

Ticket: STD-212. Base spec review of `git diff origin/main...HEAD` at c5407fabd (FR-136, FR-146, TC-541, TC-551), compared with QSpec FR-375 (quire-specification origin/main 2c2918b), ADR-013 O-07/O-09, ADR-018 PC-2/PC-6, ADR-020 RM-1/RM-5/RS-2..RS-4/RE-2, quire-semantic-value FR-123 (main) and PR #1040 (qsl-548 at db972775). Measured: `make check-spec-validation` exit 0 with no warning on the four changed files, `tools/check-index-completeness.sh` exit 0, `quire validate` (quire 0.36.2) exit 0 on the four files; no existing AC renumbered; the new AC ids FR-136-AC-5 and FR-146-AC-5..8 collide with nothing on origin/main, #1040, #1048 or #1038 (#1038 is open, not merged, and touches FR-135/139/140/141, ADR-020, not these files). Every finding below is derived from reading spec and code text; nothing was run against the engine.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-136 Order of the checked rows bullet | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136-AC-5 | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| TC-541 step 5 | spec/test-cases/TC-541-s3-checks-a-refinement-s-step-rows.md | examined |
| FR-146 Inputs invariants bullet | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146 Outputs StepObligation comment | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146 hypotheses bullet | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146 Row order bullet | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146 Occurrence key bullet | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146 Obligation identity bullet | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146 settlement bullet | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146 rejected-certificate bullet | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146 Dependencies ADR list | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146-AC-1 | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146-AC-5 | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146-AC-6 | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146-AC-7 | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146-AC-8 | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| TC-551 steps 5-8 | spec/test-cases/TC-551-per-step-simulation-records-are-written-and-settled.md | examined |
| TC-551 expected steps 5-8 | spec/test-cases/TC-551-per-step-simulation-records-are-written-and-settled.md | examined |
| ADR-013 O-09 | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md | context_only |
| ADR-020 RE-2 | spec/decisions/ADR-020-refinement-mappings-between-qsl-models.md | context_only |
| ADR-018 PC-2 | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md | context_only |
| ADR-018 PC-6 | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md | context_only |
| FR-314-AC-2 | spec/functional/FR-314-check-an-smt-proof-certificate.md | context_only |
| QSpec FR-375 Checked-package node sort sentence | quire-specification:spec/functional/analysis/FR-375-declare-model-refinement.md | context_only |
| QSV FR-123 StepRowRef description | quire-semantic-value:spec/functional/FR-123-step-simulation-record-and-obligation.md | context_only |
| PR #1040 step_row_refs doc | quire-spec-language@db972775:qsl-semantics/src/check/step_simulation.rs | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-136 now requires `CheckedRefinement.steps` itself to be in FR-375's digest order, but PR #1040 keeps `steps` in source order and maps positions in `step_row_refs`, so the PR body's claim that #1040 'already sorts by digest then name' holds for `StepRowRef` only. #1040's doc: 'indexed by the row's position in CheckedRefinement::steps (source order)'. Against FR-136-AC-5 ('gives the same `steps`, each row at the same position') #1040's checker fails. Either #1040 (or a follow-up QSL ticket) sorts `steps` at S3, making `step_row_refs` the identity, or FR-136 states the order only for the emitted node's `steps` and for `StepRowRef`, and AC-5 drops 'the same `steps`'. #1040's `StepRowRef` values and its hypothesis sort (BTreeSet of NodeKey, deduplicated) do agree with FR-146. Derived from reading; not run. | spec/functional/FR-136-check-a-refinement-s-step-rows.md:77-81 |
| FND-002 | high | The obligation-identity preimage adds two members (`StepObligation`, `hypotheses`) and a fourth subject kind (a refinement node) to ADR-013 O-09, whose preimage has exactly four members and which CG computes, and no ADR-013 text changes. O-09 identifies an obligation by the digest of the clause or application or function node id, its occurrence key, the obligation kind and `arguments`, and says 'CG computes this digest'. A CG built to O-09 gives each RE-2 record a different identity from FR-146's. On the author's claim ('records reuse the refinement record's role; ordinals 0/1/2+StepRowRef'): this needs an ADR-013 edit in this repo, not only a planner ask. O-09 needs the refinement-record subject kind and its six-member preimage, and O-07 needs occurrences derived from one declaration node. It also needs a QSpec/CG ask, because O-09's serialized authority is QSpec (QC-14, `obligationIdentitySha256`). `StepObligation` repeats what the ordinal already carries (Initial = 1, Step = 2 + r), so only `hypotheses` is new. FR-144's extra members (the subjects' initial snapshots) show the same O-09 gap for the refinement record. | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md:140-148 |
| FND-003 | medium | The narrowed hypothesis set (invariants whose context is a concrete-model object type) contradicts ADR-020 RE-2, which still reads 'The concrete package's invariant clauses are the induction hypothesis'. The narrowing is right: RM-1 lets the declaring package hold the abstract model, and an abstract-model invariant over a concrete state is meaningless as a hypothesis. But the ADR this FR implements is not amended. Amend RE-2's sentence in this PR (ADR-020 is in this repo). Open PR #1038 also edits ADR-020, so coordinate the hunk. | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md:116-121 |
| FND-004 | medium | FR-146-AC-8 borrows 'FR-314-AC-2's vector', but that vector mutates FR-314-AC-1's `Counter` certificates, and FR-314 first requires each carried query to equal FR-315's query for the item, so on the `commitA` record it rejects `QueryMismatch`, not `ProofStepInvalid`. FR-315 encodes a model subject's transition relation for a temporal property. It has no form for an RE-2 per-step simulation query (`map`, RS-3/RS-4). PC-6's `Inductive` certificate carries a base and a step query, while an RE-2 `Step` record has no base. So no certificate for an RE-2 record can reach `ProofStepInvalid`, or the `Certified` that AC-4 already assumes, until FR-315 encodes that query. Either specify the RE-2 query and its certificate shape (FR-315/FR-314), or restate AC-8 against a certificate whose queries FR-315 produces, and name that certificate. Derived from FR-314 and FR-315 text. | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md:182 |

## Verdict

Findings: 1 high, 3 medium. Clean units: the rejected-certificate and Uncertified settlement text (FR-146 lines 158-169, AC-8's outcome values) agrees with ADR-018 PC-2/PC-6 and with #1040's `BackendCertificate::Rejected` arm (inconclusive, unsettled, never proved). The hypotheses rule and its order agree with #1040. FR-146-AC-5/6/7 and TC-551 steps 5-8 trace one-to-one, as do FR-136-AC-5 and TC-541 step 5. Not mergeable until FND-002 is resolved or explicitly deferred with an ADR-013 ticket.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | FR-146-AC-4 (on main) still requires `Proved{Inductive{depth: 1}, Certified}` 'with a certificate FR-314 verifies', while the new settlement bullet says a `Certified` RE-2 settlement is pending FR-315 and FR-146-AC-7 says `Certified` 'is not exercised until FR-315 encodes the RE-2 query'. The two ACs contradict each other, and AC-4's `Certified` half cannot be met. Restate that half the way AC-7 does: feed the settlement map the checker's `Verified` result directly, which is what #1040's `a_holding_record_settles_proved_with_the_certificates_standing` does. Or mark it pending with STD-217 item 5. AC-4 keeps its id. | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md:173 |

## Dispositions

Round 1, head 7cfb5530b (fix commit 7cfb5530b over merge 2b96e20e4).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
