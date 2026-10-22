---
id: SR-15890
title: "Spec review of STD-291 step-relation emission (QSL PR #1260)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@74f1b4a2ba327acdfce872abe21a00117103510c; spec/functional/FR-180-hand-over-a-step-relation-code-claim.md, spec/functional/FR-362-bind-a-checked-unit-to-its-source-by-digest.md, spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md, spec/test-cases/TC-1143-a-step-relation-emits-a-v2-step-relation-node.md, spec/test-cases/TC-1026-one-source-yields-both-and-a-checked-unit-is-bound-by-digest.md, spec/test-cases/TC-1015-forms-not-gated-raise-no-model-gate-refusal.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-180
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-362
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1143
    type: reviews
---
# SR-15890: Spec review of STD-291 step-relation emission (QSL PR #1260)

## Summary

Ticket: STD-291. The diff is `origin/main...74f1b4a2b`. Main has 4 commits not on the head; `git merge-tree` with main is clean. It is spec only. The reviewer checked it against QSpec FR-477 at PR #377's head 9e408ad86, QSpec FR-397, QSL FR-173, ADR-033's "Moved and unmoved families" bullet, and every other QSL file that names `NotEmitted`.

These claims hold:

- FR-180's new S4 bullets match FR-477's body (subjects, executions with alias, anchor, parameters, pre/post, implementation; invariants; body) and its "empty `implementation` when unbound" rule. The I2 reader bullet defers code and locus to FR-477; AC-4's refusal (`invalid_model_binding`/`malformed-declaration` at the `preconditions` aggregate) is FR-477 reader step 4.
- FR-362 item 8 and FR-362-AC-7 now emit a `relation` over model operations and refuse only a checked `hyper` clause `NotEmitted`; the finite-reading sentence for a `relation` over non-operations stays. FR-361-AC-4, TC-1015 and TC-1026 step 5 agree; TC-1026's procedure needs no change (its second read has neither clause).
- ADR-033's bullet says a checked clause is refused `NotEmitted` "until an S4 emission writes it", and names FR-362 as normative. This PR supplies that emission, so the decision record stays true without an edit (lead default accepted).
- TC-1143 is unused on main and on all 45 other open PR heads; no other open PR touches the edited files. No status wording, no compatibility layer.

Gate: `QSPEC_DIR=<#377 head worktree> make check-spec-validation` passes, with no diagnostic in a changed file.

The `code-review` method was skipped: the diff holds no code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | major: FR-180-AC-4 cannot catch the QSpec FR-477 defect (SR-15880 FND-001): it checks which premises each `executions` entry references, but not that `x` and `y` have distinct parameter nodes or that the emitted body still distinguishes them. An emitter that lowers `Det`'s body to a tautology over shared binders passes AC-4 and TC-1143. <br>Fix, after the QSpec decision: add to AC-4 and TC-1143 step 1 that `x`'s and `y`'s binders are distinct per FR-477's resolved rule and that the body reads `x`'s and `y`'s values through them. This PR cannot merge before #377's FND-001 is resolved, because it adopts FR-477's shape. | spec/functional/FR-180-hand-over-a-step-relation-code-claim.md:126, spec/test-cases/TC-1143-a-step-relation-emits-a-v2-step-relation-node.md:33-38 |
| FND-002 | low | minor: The I2 reader bullet requires refusal of every FR-477 rule, but AC-4 and TC-1143 exercise only the `preconditions` refusal. Reader conformance for the rest rests on QSpec TC-962 RN-07 to RN-12 with no QSL vector naming them. <br>Fix: have TC-1143 step 2 feed TC-962's RN-07 to RN-12 mutations to the I2 reader, or state in FR-180 that QSL's conformance run of TC-962 covers them. | spec/functional/FR-180-hand-over-a-step-relation-code-claim.md:89-90, spec/test-cases/TC-1143-a-step-relation-emits-a-v2-step-relation-node.md:26-27 |

## Verdict

The QSL text is consistent with FR-477 and with the FR-361/FR-362 refusal changes. It is not mergeable until #377's FND-001 is settled and FND-001 here is updated to match. Merge #377 first, then this PR (with main merged in).

## Dispositions

Head 7f8f6fbe2. Main has one commit not on the head (6fb751a31); `git merge-tree` is clean. The fix round adds no new finding.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-180's S4 bullet and AC-4 require distinct per-execution binders (`x` at 0 and 1, `y` at 2 and 3) and body reads through them, so an emitter that shares binders fails; TC-1143 step 1 compares the binder nodes. This agrees with QSpec FR-477 at 2ea9d5a7e and with QSL FR-092's positional levels. |
| FND-002 | fixed | FR-180 and TC-1143 step 2 feed TC-962's RN-07 to RN-12 and RN-14 to the I2 reader, re-keying each edit. |
