---
id: SR-12818
title: "Base review of PR #976: FR-220-AC-2 relaxed fence as an S1 parse refusal"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@cfcc4a707; spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md, spec/test-cases/TC-665-access-orderings-and-classification.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-220
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-665
    type: reviews
---

## Summary

Ticket: STD-167. PR quire-spec-language#976, one commit, diff `origin/main...HEAD`
(FR-220 Description, Outputs, Behavior, AC-2; TC-665 step 2 and its expected
result). Base checklist with the integrity analysis run separately (SR-12819).
Read only; no cargo or make ci.

The QSL text is right about the grammar: QSpec's shared grammar
`fence-order = 'acquire' | 'release' | 'acq_rel' | 'seq_cst'`
(proposals/quire-v1/shared-grammar.md:543) has no `relaxed`, so the fence is a
syntax error. The S3 `ill_typed`/`operator-ineligible` refusal now names only
attempts, in Description, Outputs, Behavior and AC-2 alike.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-220-AC-2 now mixes an S1 parse outcome with S3 check outcomes. PR #974's parse test `a_relaxed_fence_is_a_parse_refusal_at_the_relaxed_word` already checks the S1 half, but it cannot carry the FR-220-AC-2 tag without also claiming the S3 halves, which are not built yet. That is the inflation pattern in #974's FND-001. Moving the fence clause into its own AC would let that test trace exactly. | spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md:113 |
| FND-002 | low | Reflow nits. FR-220 line 31 and TC-665 lines 21 and 33 run well past the file's wrap width. | spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md:31; spec/test-cases/TC-665-access-orderings-and-classification.md:21,33 |

## Verdict

PASS for the QSL text. The two findings are low and do not block. The major
cross-spec issue is recorded in SR-12819.

Checked and clean:

- TC-665 oracle strength. Step 2 expects `invalid_syntax` at S1, at the
  `relaxed` token, with no S3 result. An implementation that let the fence
  reach S3 and refused it `ill_typed` would fail on the code, on the stage and
  on the token. An implementation that accepted the fence would fail too. The
  four admitted fences still have to check.
- Convention. "`invalid_syntax` at the offending token" matches TC-1060 rows
  2 to 5 (S1), TC-811 steps 3 and 4, and TC-406 (`RecoveringCst` maps to
  `invalid_syntax`).
- No other QSL artifact still says a relaxed fence is refused at S3 with
  `ill_typed`. Every `fence ... relaxed` hit in spec/ is FR-220, TC-665 or
  FR-309-AC-3. The FR-222, FR-223 and FR-226 hits are relaxed accesses beside
  ordered fences.
- FR-220-AC-4 (`SB` with every ordering `relaxed`) has no fences, so the
  change does not affect it.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1, cf22f23a1: the fence clause moved to a new FR-220-AC-6 ("`fence F ordering relaxed;` refuses at S1 with `invalid_syntax` at the `relaxed` token and never reaches S3."). AC-2 now holds only S3 outcomes, TC-665's scope is AC-1 to AC-6, and step 2 names AC-6. |
| FND-002 | fixed | Round 1, cf22f23a1: TC-665 lines 21 and 33 rewrapped, and FR-220 line 31 shortened. Lines of 86-87 characters remain, which is within this file's existing tolerance. |
