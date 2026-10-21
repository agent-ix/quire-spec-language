---
id: SR-13651
title: 'Spec review of PR #1138: QSpec consumer alignment, integrity'
type: SpecReview
analysis: integrity
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

Ticket: STD-197 (also STD-193, STD-194, STD-186, STD-217, QSL-877). PR quire-spec-language#1138, head fd5cd6d5e. Integrity lens: whether the edited FRs agree with each other, with QSL FR-136, FR-315 and FR-338, with the code on origin/main, and with QSpec main (FR-468, FR-375) and open PRs #313 and #314.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-146 safety half | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146 Certified pending | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146-AC-4 | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146-AC-7 | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146 row order | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146 protocol row order | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146-AC-5 | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-136 protocol order | spec/functional/FR-136-check-a-refinement-s-step-rows.md | context_only |
| FR-314 encode | spec/functional/FR-314-check-an-smt-proof-certificate.md | examined |
| FR-314-AC-3 | spec/functional/FR-314-check-an-smt-proof-certificate.md | examined |
| FR-314 locus | spec/functional/FR-314-check-an-smt-proof-certificate.md | examined |
| FR-314 refinement shape | spec/functional/FR-314-check-an-smt-proof-certificate.md | examined |
| FR-338 QueryPart | spec/functional/FR-338-check-an-en-1-closure-certificate.md | context_only |
| FR-125 one instance | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md | examined |
| FR-125 hyper instances | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md | examined |
| FR-134 admission rule | spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md | examined |
| FR-134 precedence | spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md | examined |
| FR-134-AC-3 | spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md | examined |
| TC-535 vectors | spec/test-cases/TC-535-en-1-advertises-fairness-kinds.md | examined |
| FR-057 registration causes | spec/functional/FR-057-admit-shared-capability-kinds.md | examined |
| FR-218 outputs requirement | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md | examined |
| FR-218 requirement binding | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md | examined |
| FR-218 wrong-export | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md | examined |
| FR-218-AC-14 | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md | examined |
| TC-1056 row 17 | spec/test-cases/TC-1056-s3-checks-channels-relationships-requirements-and-choice-visibility.md | examined |
| TC-1056 row 23 | spec/test-cases/TC-1056-s3-checks-channels-relationships-requirements-and-choice-visibility.md | examined |
| FR-337 over node | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md | examined |
| FR-337-AC-5 | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md | examined |
| TC-1079 step 3 | spec/test-cases/TC-1079-a-temporal-clause-node-carries-its-root-and-over-node.md | examined |
| FR-153-AC-3 | spec/functional/FR-153-decide-each-fairness-on-the-annotated-quotient.md | examined |
| FR-161-AC-2 | spec/functional/FR-161-concretise-a-reduced-counterexample.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-146's safety-half rule contradicts QSpec FR-468. FR-146 says the half is 'Certified only when the base part and every row's step part verify ... and every hypothesis item is proved; otherwise Uncertified, or Inconclusive with cause hypotheses-not-established'. QSpec FR-468 (main fd7a694) gives certified only 'when every part and every hypothesis item is certified', and FR-468-AC-3 says 'one hypothesis item uncertified also gives uncertified'; FR-146 certifies that case. 'otherwise Uncertified' also swallows the outcomes FR-468 orders before it (initial counterexample refuted InitialNotAbstract, certificate-rejected, induction-not-closed, solver-inconclusive / limit-reached / unsupported / failed) and FR-146's own CertificateRejected and InductionNotClosed bullets. Replace the bullet with a pointer ('QSL SHALL settle the safety half by QSpec FR-468 Settlement') or restate FR-468's ordered rules exactly. | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md:167-172 |
| FND-002 | high | FR-314-AC-3 needs FR-315 queries that FR-315 does not define and FR-146 says do not exist. FR-314-AC-3 expects the CasRefinesCounter certificate 'each refuting its FR-315 query' to be Verified, and the encode bullet now says the checker encodes 'for RefinementInductive the initial query for the base and each row's step query'. FR-315 has no refinement encoding (no initial or step query, no RE-2 content), and FR-146 keeps 'A Certified RE-2 settlement is pending FR-315 encoding the RE-2 per-step query, which it does not yet do', with FR-146-AC-4 and AC-7 saying a Certified settlement 'is not exercised until FR-315 encodes the RE-2 query'. AC-3's Verified and QueryMismatch vectors cannot be built until FR-315 states those encodings. Either specify the FR-315 initial and step query encodings in this PR, or keep FR-314-AC-3 to the shape vectors (which need no encoding) and leave Verified with FR-146's pending note. | spec/functional/FR-314-check-an-smt-proof-certificate.md:150; spec/functional/FR-314-check-an-smt-proof-certificate.md:103-108; spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md:164-166; spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md:181; spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md:184 |
| FND-003 | medium | FR-314 changes QueryPart::Step while FR-338, which defines it, still has a unit Step. FR-314 says '`QueryPart::Step` (FR-338) carries the row index for this variant and none for Inductive' and its ACs locate 'ShapeMismatch at Step row 3'. FR-338's Inputs block defines 'pub enum QueryPart { Unrolling, Base, Step }   // FR-314', and FR-314's own Inputs paragraph still says 'part being Unrolling, Base or Step'. Two FRs now give the same type two shapes, and neither states the wire spelling of the row inside certificate-rejected{rule, at} (the code serializes QueryPart as a lowercase unit string). Leaving FR-338 unedited is a contradiction, not a deferral: edit FR-338's block in this PR to 'Step { row: Option<u64> }' (or the chosen shape) with its wire form, and update FR-314's Inputs paragraph to match. | spec/functional/FR-314-check-an-smt-proof-certificate.md:100-102; spec/functional/FR-314-check-an-smt-proof-certificate.md:73-78; spec/functional/FR-338-check-an-en-1-closure-certificate.md:63; qsl-replay/src/certificate.rs:21-31 |
| FND-004 | medium | FR-146-AC-5 adopts FR-375's tie-breakers for model subjects only; the Behavior bullets and the protocol half keep the old order. AC-5 now says the model-subject order is FR-375's 'node-key digest of the first named declaration, then member name, then alias-free class spelling, rows naming no declaration last'. Two problems. (1) FR-146 Behavior still states only 'node-key digest ... then by member name', so the AC exceeds its SHALL; and the last two keys are the ones QSpec FR-375 (#311) gives for a protocol subject's steps, not a model subject's. (2) AC-5's protocol half and FR-146's protocol bullet still use FR-136's protocol key (clause digest for memory rows, then step keyword, then memory kind), while FR-375 now puts every memory row after every declaration-naming row and breaks ties by alias-free class spelling. A protocol with a memory row whose clause digest sorts low gets a different StepRowRef under the two rules. Align FR-136, FR-146 Behavior and AC-5 with FR-375, or restore AC-5 and open a follow-up for the protocol order. | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md:182; spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md:121-132; spec/functional/FR-136-check-a-refinement-s-step-rows.md:85-97 |
| FND-005 | medium | FR-125 gives a hyper clause with no over parameter one instance, which its hyper bullet and the grammar contradict. The new bullet reads 'QSL SHALL give a state-graph, probabilistic or hyper clause or claim with no over parameter one instance, with no bound object.' The preceding bullet instantiates a hyper clause 'with one instance per key of the universe of the one population each object parameter v: V::T resolves to'. The QSL grammar's HyperClause always writes 'over' followed by a behaviours or trace domain and has no optional over parameter. A reader cannot tell whether a hyper clause with object parameters gets one instance or the product. Drop 'hyper' from the list (or say which hyper clause has no object parameter). | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:161-162; spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:152-157; qsl-cst/src/grammar.rs:2252-2272 |
| FND-006 | low | FR-134 retains the value for not-a-list; QSpec FR-368 retains it only for the label defects. FR-134 retains 'for the last three, the received value or element', and FR-134-AC-3 and TC-535 V12 and V13 name the non-list member (`null`, `"weak"`). QSpec #313's FR-368 says the refusal retains the defect 'and, for the last two, the offending value', that is unknown-label and duplicate-label only, and FR-368-AC-8 names no value for the non-list forms. The FR-272 row ('the defect ... with the offending value') is looser. Retaining more is harmless inside QSL but makes the diagnostic differ from another FR-368 implementation. Either follow 'the last two' or ask STD-194 to make FR-368 say 'the last three'. | spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md:94-96; spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md:122; spec/test-cases/TC-535-en-1-advertises-fairness-kinds.md:43-44 |

## Verdict

Not mergeable as it stands. FND-001 and FND-002 are high: FR-146 restates QSpec FR-468's settlement wrongly, and FR-314-AC-3 depends on FR-315 encodings no spec defines and FR-146 says are pending. FND-003 answers the PR's open question: leaving FR-338 unedited is a contradiction, so edit FR-338 in this PR. FR-134/FR-057/TC-535 agree with QSpec #313 apart from FND-006, and FR-218/TC-1056 agree with the code and STD-197 items 2 and 3. FR-337, TC-1079, FR-153-AC-3 and FR-161-AC-2 agree with QSpec #314 and FR-370-AC-13; FR-125 does too, apart from FND-005.
