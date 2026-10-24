---
id: SR-11266
title: "Spec review — QSL-797 FrameWitness Link change (PR 870)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@d8defb6b05250745959143b41372efee757d6d12; spec/functional/FR-106-admit-snapshots-and-invocations.md, spec/functional/FR-115-run-an-operation-frame-over-an-invocation.md, spec/functional/FR-116-replay-a-frame-counterexample.md, spec/functional/FR-267-write-run-results-as-native-run-result-2.md, spec/test-cases/TC-1037-a-frame-run-reports-a-changed-relationship-link-as-a-link-witness.md"
review_set: subset
---

## Summary

Ticket: QSL-797. One pass of spec-review with its contradiction, ambiguity, completeness, testability and integrity checks over the PR diff (`git diff origin/main...HEAD`). Oracles: merged FR-106 check 11 (steps 11.1 to 11.5 are the nested list items of check 11 on main), TC-1033/TC-1035's `orders` fixture, FR-364's reference-key order, ADR-012's `DeclarationKey` order (`package`, then `node`, as UTF-8 bytes), and QSpec FR-013, FR-340, FR-352 and TC-255, cited by id only.

Every TC-1037 row was traced through FR-106 checks 1 to 10 and 11.1 to 11.5 and FR-115's Frame routing in order. Each expected result is the first finding: rows 2/3/6/7 reach 11.5 with no earlier finding; row 4 passes 11.1 (granted creation) and 11.4 (delta equal); row 8 stops at 11.1 (`Created`); row 9 stops at 11.4 (exit 20); row 10 passes 11.2 and 11.4 and 11.5 leaves out `p2`-`t1`; row 11 is not compared; row 6's housing-before-composition holds because `relationship/Pump-housing-Tank` < `relationship/Sys-composition-Pump` bytewise; row 7's `p1` < `p2` and row 2's `t1` < `t2` hold in reference-key order; row 15's three settlements match FR-116's settlement rules, including "both stay in the result as found".

## Verdict

**PASS. No blocking finding.** The new rule set does not contradict FR-106, FR-364 or QSpec FR-013/FR-340: QSpec FR-013 already says an unauthorized added or removed instance receives `frame_violation`, and FR-340 names a relationship `modifies` entry by its node key, which is the `DeclarationKey` FR-115 now carries. The "steps 1 to 3, or step 5" routing contradicts no existing FR-115 statement; step 4 still routes to exit 20. The author's seven choices are internally consistent across FR-115, FR-116, FR-267 and TC-1037 (single top-level `population` = the source end's, stated in FR-115 and FR-267 and checked in row 13). AC-9 claims only relationship order before source order and row 6 can fail an implementation that walks composition first (it is declared first in the fixture), so it is honest and non-vacuous. No test parses the touched spec files (only FR-092, FR-093, FR-094 and FR-255 are read by tests), no code enumerates the frame-witness change tags yet, no ticket or PR id is added to spec text, no tracking ceremony is added, and the new ids collide with no open PR (862, 864 and 865 touch none of them). The five touched files pass `quire validate`.

QSpec: no change needed. FR-352 delegates the change variants to QSL FR-115 and their encoding to FR-267, and FR-352-AC-9's "change and checked permission" covers `link`. A TC-255 NR-10 link row is optional, not required, because FR-267-AC-12 / TC-1037 row 14 tests the link wire.

The three low findings below are recorded, not blocking.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-116's Description still defines a frame counterexample as an invocation that "writes, creates or deletes" outside its frame; it omits the link change that the Inputs now list. | spec/functional/FR-116-replay-a-frame-counterexample.md:25-26 |
| FND-002 | low | FR-115's "an instance whose count differs is named once, whatever the difference in count" has no vector: TC-1037 never uses the bag variant. | spec/functional/FR-115-run-an-operation-frame-over-an-invocation.md:129-130 |
| FND-003 | low | FR-115-AC-9 claims a step 1 to 3 finding "in any population" precedes a link finding; TC-1037 row 8 has one population only, so the cross-population case is untested. | spec/functional/FR-115-run-an-operation-frame-over-an-invocation.md:163 |
