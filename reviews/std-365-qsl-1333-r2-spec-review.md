---
id: SR-16351
title: "spec-review of FR-093 text-to-number lowering (quire-spec-language PR 1333, fix round)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@5f0c2dde; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md; spec/test-cases/TC-1196-a-text-to-number-convert-lowers-to-its-text-identity.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1196
    type: references
---

# Spec review of FR-093 text-to-number lowering (quire-spec-language PR 1333, fix round)

## Summary

Ticket: STD-365. One spec-review pass over `git diff origin/main...5f0c2dde`, checked against QSpec FR-478 at quire-specification#395 head d92ea803.

Gates: `QSPEC_DIR=<#395 head worktree> make check-spec-validation`: exit 0, 19 warnings, none in FR-093 or TC-1196. `cargo test -p qsl-package --lib checked_v2` with `QSPEC_DIR` at the #395 head: 49 passed. Merge-tree against current origin/main (33824583, two commits ahead of the branch's merge) is clean.

Examined, clean:

- The `ConvertText` rule and lowering row cover all six targets for both `Text[..]` and `String` operands, with the same identities, laws (`text_profile` for text only, then `ieee_profile` for IEEE), modes (target-pinned `rounding` for decimal, IEEE and quantity; none for integer and rational) and members as FR-478's catalog table. No target is narrowed.
- FR-093-AC-35's modes match FR-478 and FR-322 `type_pinned_modes` (`Float64` with no mode pins `exact`). AC-47 resolves SR-16322 FND-002: the "Converted equality operand" paragraph is scoped to exact numeric and quantity operands, and `convert<Integer>(t) = 3` lowers to `quire.op.integer.eq` over `quire.op.text.to_integer`. AC-47's refusals (`Boolean`, another `Text[..]`, `String`) and the same-type no-node case match FR-478.
- TC-1196 steps cover AC-35 and AC-47 one to one.
- Ids: FR-093-AC-35, FR-093-AC-47 and TC-1196 are on no other open PR head and not on main (other open heads use FR-093-AC-36 to AC-46).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - | correct-requirement-no-evidence |

## Verdict

Clean. SR-16322 FND-001 and FND-002 are fixed. Mergeable together with quire-specification#395 once that PR's findings (SR-16350) are resolved, since FR-093 cites FR-478.
