---
id: SR-16297
title: "Spec review of quire-spec-language PR 1379: STD-424"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@6270d77a; spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md; spec/functional/FR-092-key-type-parameter-and-declared-nodes.md; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md; spec/test-cases/TC-1232-a-decimal-constructor-in-source-checks-lowers-and-evaluates.md"
review_set: subset
date: 2026-10-23
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1232
    type: references
---

# SR-16297: Spec review of quire-spec-language PR 1379 (STD-424)

## Summary

Ticket: STD-424. Head 6270d77a (PR 1379, contains 1351 head b65f16ce). Own delta: commit 7d985e44 (FR-091, FR-092, FR-093, TC-1232, TC-396). The S1/S2 form, the typing by expected type, the nonzero-divisor rule (`decimal(c, s)` with c != 0, matching QSpec FR-146 in the stack), the FR-092 value spelling ("1.25", "0.05", "10", "0.00", "-0.5"), lowering and the TC-1232 steps were derived by hand against QSpec FR-140 and shared-grammar.md (`decimal(signed-int, uint)`); the AC-39 to AC-42 results (1.00 + 1.25 = 2.25, 6.00 / 4 = 1.50, 5/4 for 1.25 as a Rational, 1.25 to scale 1 under nearest-even = 1.2) hold. FR-091-AC-7 and TC-396 correctly drop `decimal(1, 2)` from the unrepresented list. No deferral or narrowing language in the delta.

Also checked:

- Ids FR-091-AC-57/58, FR-093-AC-39..42 and TC-1232 are unique across all open PR heads. Gate exit 0, 19 warnings (main: 19). Merge-tree with main clean; conflicts with open PRs 1335, 1333, 706.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The typing rule refuses a constructor whose written coefficient or scale lies outside the expected type's `[lo, hi]` and `[smin, smax]`, and FR-093-AC-40 pins `decimal(5, 0)` at `Decimal[0, 1000; 1, 2; nearest-even]` as `type-mismatch`. QSpec FR-140 decides membership on the mathematical value: the membership scale is max(s, smin) and the membership coefficient is v x 10^s*, so `decimal(5, 0)` is a member of that type (as 5.0, coefficient 50) and `decimal(10, 3)` is a member of a `smax` 2 type (0.01 normalizes to scale 2). The rule and the vector contradict the oracle the PR cites. Fix: state the check as membership of the normalized value (QSpec FR-140), keep the retained coefficient and scale as written, and replace the `decimal(5, 0)` vector (a member) with a real violation such as `decimal(1, 3)` at `smax` 2 or a coefficient past `hi`. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:316 | wrong-requirement |
| FND-002 | medium | FR-091-AC-58 assigns `decimal(1, -2)` to `invalid_syntax`/`unexpected-token` but `decimal(+1, 2)` and `decimal(1, 02)` to `invalid-token`, while the prose lists a signed, leading-zero argument and a negative scale together as 'a syntax error' without saying which group the negative scale belongs to. Both `-2` and `+1` are one stray sign before an integer, so the split is unexplained and an implementer cannot derive the vector. Fix: state the rule (for example, a sign that the argument grammar does not admit is an `invalid-token` of the argument) and align the vector. | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:1055 | missing-requirement |

## Verdict

**Not mergeable.** FND-001 (high) block the PR; FND-002 (medium) should be fixed in the same round.
