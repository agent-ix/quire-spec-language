---
id: SR-16267
title: "Spec-review of record trigger domains and retire-predicate population reads (quire-spec-language PR 1324)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@f171629661ee384e5d99516e6372bc471db6dec5; spec/functional/FR-208-spawn-and-retire-replicated-role-instances.md; spec/functional/FR-209-start-a-protocol-instance-per-trigger-under-the-instance-budget.md; spec/functional/FR-215-implement-protocolsystem-as-a-transitionsystem.md; spec/functional/FR-216-give-every-protocol-step-a-static-footprint.md; spec/test-cases/TC-653-replicated-role-instances.md; spec/test-cases/TC-654-activation-on-each-and-instance-budget.md; spec/test-cases/TC-660-protocolsystem-transition-system.md"
review_set: subset
date: 2026-10-23
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-208
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-209
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-215
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-216
    type: references
---

# Spec-review of record trigger domains and retire-predicate population reads (quire-spec-language PR 1324)

## Summary

Tickets: STD-426 and STD-386. This reviews the whole PR at f1716296, both the
STD-386 commit f2006c02 (no earlier review on STD-386) and the STD-426 commit 856dcbf8.
origin/main is 7 commits ahead of the head and is not an ancestor. The merge is clean.

Gate `QSPEC_DIR=/Users/peter/dev/quire-specification make check-spec-validation`: exit 0, no warnings on the touched files.

Hand-derived vectors:

- FR-215-AC-5: `Msg {n: Int[0,1]; m: Mode; i: Inner}` gives 2 x 2 x 2 = 8 records. With the first
  field most significant, the order runs `{0,slow,{false}}`, `{0,slow,{true}}`, `{0,fast,{false}}`
  and so on to `{1,fast,{true}}`, as stated. `Wide.t: Integer` is unbounded, so `RequiresBound` is right.
- FR-209-AC-8: `Tick {n: Int[0,2]; up: Boolean}` with guard `x.n > 0` gives the 4 records n in {1,2} x up
  in {false,true}, in the stated order. The `lookup` guard refuses as a model-state read, and FR-218
  orders the `over`/model-state refusal before the out-of-scope one.
- FR-208-AC-6 and FR-216-AC-6 now name the population the same way, `lookup<M::Job>(M::jobs, j)`.

Ids FR-208-AC-6, FR-209-AC-8 and FR-215-AC-5 are unique across every open PR head. The deferral
notes in FR-209 and FR-216 are gone, and grep finds no QSL-968 or remaining-work text in the touched FRs.
The activation guard's bar on model state is an existing rule (QSL FR-381, QSpec FR-464). This PR restates
it and does not narrow anything. The PR conflicts with open #1368 (STD-434) in FR-209, because both make
the same two deletions. One of the two PRs is redundant.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | [ambiguous] The deleted note said that the tests of FR-209-AC-1 to AC-7 used the `Int` range trigger `Level`. Without it, those ACs run over the record `Tick`, and none of them declares `Tick`'s fields. Only AC-8 declares a `Tick` (`n: Int[0, 2]; up: Boolean`). The state counts and lassos of AC-5 to AC-7 depend on how many `activate(Tick)` steps there are. Fix: declare `Tick` once (for example `record Tick { n: Int[0, 1]; }`) in AC-1 and have AC-2 to AC-7 refer to it. | spec/functional/FR-209-start-a-protocol-instance-per-trigger-under-the-instance-budget.md:151-157 | wrong-requirement |

## Verdict

Mergeable. The record-domain rule, the retire-predicate scope and both deferral removals are complete and match the code paths they name. The one low finding can be fixed in place. Before merge: merge origin/main, which merges clean, and close #1368 (STD-434) or rebase it, since it duplicates this PR's FR-209 and FR-216 deletions and conflicts in FR-209.

## Dispositions

Round 1, against head 861ec646. Gate `make check-spec-validation` at 861ec646: exit 0.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
