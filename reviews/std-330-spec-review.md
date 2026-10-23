---
id: SR-16202
title: "Spec review of quire-spec-language PR #1296: STD-330, FR-106 Decimal value object and normalized set/bag elements"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@5556f7bd6e5f1936b2bee980d10f2baff1b10cc6; PR #1296 (git diff origin/main...5556f7bd6): spec/functional/FR-106-admit-snapshots-and-invocations.md, spec/test-cases/TC-1171-snapshots-carry-set-bag-and-ordered-set-values.md; read for context: QSL FR-038, FR-056, FR-070, FR-092; agent-ix/quire-specification main b3f3c803b FR-140, FR-144, FR-149, FR-181"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1171
    type: reviews
---
# SR-16202: Spec review of quire-spec-language PR #1296: STD-330

## Summary

Ticket: STD-330 (STD-285 item 3). Sub-analyses folded in: integrity, EARS, object review, criterion strength (manual, no Jev), cross-repo fidelity. `make check-spec-validation` exits 0 at the head; main 5ba013c75 is an ancestor of the head. FR-106-AC-53 and FR-106-AC-54 are unique across all 21 open PR heads, and no other open PR touches FR-106 or TC-1171.

Fidelity, checked against QSpec main:

- FR-181's `decimal` form has members `coefficient` and `scale` and keeps the retained representation. A set or bag element is written in the FR-140 normalized form (FR-181-AC-20), and only there. The PR matches both.
- FR-140: the representation is (`Integer`, `u32`), so the `4294967295` scale cap is correct. Membership uses `s* = max(s, smin)`, `c* = v × 10^s*`, `s* <= smax`, `lo <= c* <= hi`. Every AC example checks out against that rule: `1.000` is admitted, `0.001` is refused (s* = 3 > 2), `100.01` is refused (c* = 10001), `-1` is refused, and `2` is admitted as `2.00`.
- FR-149: equality is by mathematical value. FR-144: set and ordered-set duplicates are decided by FR-149 equality, a bag groups equal elements, and a value outside the declared domain is `invalid_runtime_input`/`invalid-value`. The PR matches all three.
- The written order of `rates` (`15`/`1` before `2`/`0`) is the same under FR-106's `kind` spelling and FR-181's `type` spelling.

The four decisions (adopt normalization, the `kind` payload object, the scalar `Decimal` slot, and refusal under `PostStateRange::Witness`) are flagged in the PR body as lead defaults, not owner answers. Refusing a Decimal under `Witness` matches FR-106's existing treatment of a Text outside its length.

## Verdict

**NOT MERGEABLE as is.** The Decimal object, its check order and its membership rule are correct and well tested. Two medium defects need a fix round first. The existing native-kind equality sentence now contradicts by-value Decimal equality (FND-001). The normalization scope contradicts itself for a Decimal nested inside a set element, and it omits variants and tuples (FND-002). FND-003 is low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The existing sentence "Two admitted values of one native kind are equal exactly when their canonical payloads are equal (QSpec FR-149 ...), so the frame check compares a String, UUID or Timestamp field by payload" now covers the Decimal payload object as well. A Decimal's admitted payload is its retained coefficient and scale, so under this sentence `150`/`2` and `15`/`1` are unequal. The new Decimal paragraph says they are one value, so the two statements conflict. Scope the sentence to String, UUID, Timestamp and Text, or say that a Decimal compares by value as stated below. | spec/functional/FR-106-admit-snapshots-and-invocations.md:318-321 |
| FND-002 | medium | Two problems with the normalization scope. (1) It contradicts itself: "A Decimal element of a set or bag, at any depth of nesting through options, collections and records, SHALL be written with its normalized coefficient and scale", but "A Decimal element of a sequence or ordered set ... keeps its supplied coefficient and scale". For `Set<Sequence<Decimal>>`, or a record with a sequence field inside a set, both rules apply to the same value. QSpec FR-181 normalizes recursively throughout a set or bag element. (2) The depth list omits FR-360 variants and tuples, which FR-181 includes ("options, collections, records, tuples and unions"). Fix: normalize every Decimal anywhere inside a set or bag element, including variant payloads; every other Decimal keeps its supplied representation. Add one nested case to AC-53, for example a `Set<Option<Decimal>>` or a record element. | spec/functional/FR-106-admit-snapshots-and-invocations.md:252-258,1005 |
| FND-003 | low | AC-53 asserts that `tallies` has one member, `15`/`1`, with multiplicity 2, but not its written bytes. QSpec FR-144-AC-21 requires the bag to be written as two identical canonical elements. A writer that groups by value but writes the supplied `150`/`2` for the first occurrence would pass. Add the written array `[{"coefficient":"15","kind":"decimal","scale":"1"},{"coefficient":"15","kind":"decimal","scale":"1"}]` to AC-53 and to TC-1171 step 4. | spec/functional/FR-106-admit-snapshots-and-invocations.md:1005; spec/test-cases/TC-1171-snapshots-carry-set-bag-and-ordered-set-values.md:36 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | [minor] AC-53's lists lost their joining words when `opts` and `runs` were added. The fixture reads "`tallies: ...` and `steps: ...`, `opts: ...` and `runs: ...`", and the snapshot list has no separator between the `tallies` value and "`steps`". The same fixture sentence is in TC-1171's Decimal fixture. Fix: "`tallies`, `steps`, `opts` and `runs`" in both lists, and add a comma or "and" before `steps` in the snapshot list. | spec/functional/FR-106-admit-snapshots-and-invocations.md:1010; spec/test-cases/TC-1171-snapshots-carry-set-bag-and-ordered-set-values.md:21 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | d86a41c99: the equality sentence is scoped to "Two admitted String, UUID or Timestamp values of one kind", and adds "A Decimal is not compared by its payload: two Decimals are equal by value, and a Text by its retained sequence, as stated below". This matches the Text paragraph (retained sequence under the profile) and the Decimal paragraph (by value). |
| FND-002 | fixed | d86a41c99: one rule now covers it, "Every Decimal inside an element of a set or bag SHALL be written with its normalized coefficient and scale ... at any depth and through every nesting the grammar has: options, sequences, sets, bags, ordered sets, records and FR-360 variant payloads". It explicitly includes a sequence or ordered set inside a set or bag element, and every other Decimal keeps its supplied form. AC-53 and TC-1171 add `opts: Set<Option<Decimal>>`, written `[{"absent":{}},{"present":{"coefficient":"15",...}}]`, and `runs: Bag<Sequence<Decimal>>`, written with its sequence elements normalized to 15/1 and 1/0 and in sequence order. Both are correct by JCS byte order and FR-140 normalization. |
| FND-003 | fixed | d86a41c99: AC-53 and TC-1171 step 4 state that `tallies` is written `[{"coefficient":"15","kind":"decimal","scale":"1"},{"coefficient":"15","kind":"decimal","scale":"1"}]`, as QSpec FR-144-AC-21 requires. |
