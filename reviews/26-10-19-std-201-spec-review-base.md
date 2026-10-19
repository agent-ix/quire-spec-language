---
id: SR-13073
title: "Base spec review of PR #1022: FR-175-AC-2 and TC-600 step 2 first difference at the fourth joint-loop position"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@dadd30aa0; git diff origin/main...HEAD: FR-175-AC-2, TC-600 Expected Results step 2"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-175
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-600
    type: reviews
---

## Summary

Ticket: STD-201. PR quire-spec-language#1022 at head dadd30aa0, one commit,
two changed lines (FR-175-AC-2, TC-600 Expected Results step 2). Review set:
subset (base, integrity). `quire validate` exits 0 on both changed files.

The math was checked independently. With loop lengths 2 and 3 aligned at the
start of the joint loop, equal outputs at offset t give `a[t mod 2] = b[t mod 3]`:
offset 0 gives a0 = b0, offset 1 gives a1 = b1, offset 2 gives a0 = b2,
offset 3 gives a1 = b0. Offsets 0 to 3 together make all five observations
equal, so offsets 4 (a0 = b1) and 5 (a1 = b2) follow and cannot hold the first
difference. Offset 3 is reachable, so it is the latest possible first
difference. The four equalities at offsets 0 to 3 connect all five
observations for any phase of the two loops (the same holds under the
Chinese remainder theorem for any stem offsets), and a brute-force check over
every phase pair and every binary assignment gives 3 as the latest first
difference. The example a = (0, 1), b = (0, 1, 0) reads (0,0), (1,1), (0,0),
(1,0) at offsets 0 to 3: equal at 0 to 2, different at 3 with `a` reading 1
and `b` reading 0. The added sentence is true. "Fifth or sixth position" is
offsets 4 and 5, consistent with "fourth = offset 3".

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The worked example reads `a` = (0, 1) and `b` = (0, 1, 0) from the first loop observation of each at joint-loop offset 0, which holds only when the two stems are equal (or differ by a multiple of both loop lengths). FR-175's Tuple of lassos rule puts the joint stem at the longest component stem, so with unequal stems the same loops are read with a phase shift: with `a`'s stem 1 and `b`'s stem 0, offset 0 already reads a0 = 0 against b1 = 1 and the first difference is at offset 0. Neither the AC nor TC-600's fixture line ("a pair of lassos with loop lengths 2 and 3") states the stems. State equal stems (for example, both loops entered at position 0) in the example and in the TC-600 fixture. The latest-first-difference claim itself holds for any stems. | spec/functional/FR-175-evaluate-a-body-over-a-tuple-of-behaviours.md:84; spec/test-cases/TC-600-evaluate-a-body-over-a-tuple-of-behaviours.md:19,30 |
| FND-002 | low | "equal outputs at offsets 0 to 3 fix every loop observation of both components" reads as if the observations' values were determined. The equalities force all five observations equal to one common value, not to particular values. Say "force all five loop observations of both components to be equal". | spec/functional/FR-175-evaluate-a-body-over-a-tuple-of-behaviours.md:84 |

## Verdict

Mergeable. The correction is right: a first difference at the fifth position
is unreachable for loops of length 2 and 3, the fourth position (offset 3) is
the latest reachable one, the example is correct, and the added sentence is
true. The two low findings are wording: the example's implicit equal-stems
assumption and the word "fix". Neither makes the AC wrong for the natural
fixture (both loops entered at position 0). The rest of FR-175 and TC-600 is
unchanged and does not restate the ordinal.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 67acb405d: FR-175-AC-2's example now reads "loops `a` = (0, 1) and `b` = (0, 1, 0), both entered at position 0 (equal stems)", and the latest-difference sentence adds "whatever the stems", which is true (checked for every phase pair). TC-600's fixture line states the same stems and values, so a fixture with unequal stems is no longer a valid reading. |
| FND-002 | fixed | 67acb405d: "fix every loop observation" is now "force all five loop observations of both components to be equal". |
