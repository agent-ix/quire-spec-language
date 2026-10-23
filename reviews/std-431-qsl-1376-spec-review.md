---
id: SR-16273
title: "Spec-review of nested field selection in call_site (quire-spec-language PR 1376)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@7cc93876c8f68a7315b91fb24cff0865be35f3a5; spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md; spec/test-cases/TC-516-locate-a-function-call-site.md"
review_set: subset
date: 2026-10-23
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-121
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-516
    type: references
---

# Spec-review of nested field selection in call_site (quire-spec-language PR 1376)

## Summary

Ticket: STD-431. Head 7cc93876. origin/main is 5 commits ahead and is not an ancestor. The
merge is clean. The gate exits 0, and the only warnings on FR-121 are the three already on main.

Hand derivation against `classify_domains` in `qsl-semantics/src/family/requirements.rs`. That walk indexes
collection elements and option payloads at 0, record fields in declaration order and tuple positions by
position, and gives union positions as the member-ordered concatenation. It keys a re-entered composite
`Recursive` at the path where it was first entered.

- AC-28: by name ordinal, `Box`'s fields are note=0, span=1, tags=2. That gives span [1], [1,0] (lo) and
  [1,1] (hi); tags [2] and [2,0]; note [Payload, Field(hi)] = [0,0,1]. The refusals are correct.
- AC-29: `head` [0] enters `Node`. Field(next)=1 then Payload=0 reaches `Node` again, so the walk gives
  `Recursive` at [0]. [Field(value)] = [0,0]. A step past the re-entry refuses. In `Shape`, the concatenation
  is [r, w, h], so `Member(Rect, 1)` = [0,2], and `Member(Circle, 1)` refuses.

FR-121-AC-28/29 are unique across open PR heads, and no QSL-345 or remaining-work text is left. The lead
defaults (named steps; enum, scalar and reference steps refuse) match the extent walk, which has no child
positions for those types. They do not narrow anything.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | [ambiguous] The rule applies when a step reaches a declaration 'that an earlier step of the same selection already entered'. In AC-29, `Node` is first entered by the field `head`'s own type, with no step. Read literally, `[Field(next), Payload]` re-enters nothing, so it would descend into `Node` again. The AC, and the walk, treat that as a re-entry. Fix: 'that the field's type or an earlier step of the same selection already entered'. | spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md:270-276 | wrong-requirement |

## Verdict

Mergeable. The nested selection agrees exactly with the extent walk. The one low finding is a wording fix. Merge origin/main first (clean).

## Dispositions

Round 1, against head a358c2b1. Gate `make check-spec-validation` at a358c2b1: exit 0 (only the 3 main warnings on FR-121).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
