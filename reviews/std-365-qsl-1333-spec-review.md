---
id: SR-16322
title: "spec-review of FR-093 text-to-number lowering (quire-spec-language PR 1333)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@16712db9; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md; spec/test-cases/TC-1196-a-text-to-number-convert-lowers-to-its-text-identity.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1196
    type: references
---

# Spec review of FR-093 text-to-number lowering (quire-spec-language PR 1333)

## Summary

Ticket: STD-365. One spec-review pass over `git diff origin/main...16712db9`, checked for the no-drop/no-narrowing/no-deferral rule, testable vectors (key vectors derived by hand), id uniqueness across all open PR heads, cross-repo names, mergeability and gates.

Gate: `QSPEC_DIR=<worktree of QSpec PR 395 head> make check-spec-validation`: exit 0, 19 warnings (the same 19 on every head in this batch; none in the touched files).

Examined, clean:

- The lowering row and AC-35 vectors match QSpec FR-478 (PR 395 head a3b9cd66): the identities `quire.op.text.to_integer` and `quire.op.text.to_decimal`, member `type_argument`, mode `text_profile`, the `Int[0, 99]` and `Decimal[0, 1000; 0, 2; exact]` spellings, and the refusal of `Rational`, `Text` and `String`-operand converts. The mode and law spelling match FR-093 rows already on main (`{"kind":"text_profile","value":"nfc"}`, `quire.value.text.unicode-17.0.0/v1`).
- TC-1196 steps 1 and 2 match FR-093-AC-35 one to one.
- Ids: FR-093-AC-35 and TC-1196 appear on no other open PR head or on main.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | Scope narrowing by lead default: `convert<T>(e)` over a `Text[..]` operand is admitted for integer and decimal `T` only. Every other target (`Rational`, floats, quantities) and every `String` operand is refused `ill_typed`/`operator-ineligible` permanently, and AC-35 pins those refusals. The PR body marks this a lead default, not an owner answer. Under the standing rule a lead default that removes a capability is a blocking scope-deferral. Fix: have Peter decide the target set, or specify the other targets, with vectors. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:328-338 | missing-requirement |
| FND-002 | medium | The existing "Converted equality operand" paragraph says an Equality operand written `convert<T>(e)` whose `e` has a type other than `T` is checked as a `ConvertScalar(T, e)` lowering by the `ConvertScalar` rows. The new "Text to number" paragraph says a `convert<T>(e)` over a `Text[..]` operand is a `ConvertText`, never a `ConvertScalar`. For `convert<Integer>(t) = 3` both rules apply and disagree, and AC-35 has no equality-position vector. Fix: scope the equality paragraph to numeric and quantity operands, and add an AC vector for `convert<Integer>(t) = 3`. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:348-356 | wrong-requirement |

## Verdict

Not mergeable: FND-001 (high) block the PR; FND-002 (medium) should be fixed in the same round.

Conflicts with main: none (merge-tree clean). FR-093 AC-table append conflicts with open #1347, #1351, #1355, #1357, #1360, #1369, #1374 and #1379 (same AC-35 position region); resolve by union in id order.
