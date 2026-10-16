---
id: SR-11407
title: "Spec review of quire-spec-language PR #877: clause-set evaluation and per-read runtime value paths"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@a25dd52b4895b0434d61438c4becd036a58535bc; spec/functional/FR-106-admit-snapshots-and-invocations.md, spec/functional/FR-107-evaluate-state-clauses-at-s6a.md, spec/functional/FR-120-simulate-a-checked-package-s-state-family.md, spec/test-cases/TC-1039-a-clause-set-reports-its-results-together.md, spec/test-cases/TC-1040-a-clause-evaluation-reports-its-receiver-and-read-paths.md"
review_set: subset
---
# Spec review of quire-spec-language PR #877

## Summary

Ticket: QSL-790. One pass of spec-review with the contradiction, ambiguity,
completeness, testability and spec-integrity sub-analyses over the PR diff.
Spec-only PR, so no code review. Only major issues block. Mediums and lows
are recorded in the Verdict.

Oracles: FR-104, FR-106 (selection forms, check 2, Outputs), FR-108 corpus
and `cases.rs`, FR-120 contract clauses, FR-265 criteria fixture, FR-363,
FR-364, TC-1033 as merged; ADR-016 FE-3; qsl-eval
`value/expression/mod.rs` and `s6a/separation.rs` and qsl-semantics
`model/observation.rs` on the reviewed head (read-only); quire-specification
FR-207, FR-208 (State-machine context anchors, Locations, AC-10, AC-13),
FR-047-AC-9 and TC-235 M16 and M22 on main (cited by id only).

Examined:
- FR-106 Outputs invocation sentence (examined)
- FR-107 Outputs, the receiver path, read paths and clause sets sections,
  FR-107-AC-7 to FR-107-AC-16 (examined)
- FR-120 conjunction sentence (examined)
- TC-1039 steps 1 to 5 and Expected Results, each traced (examined)
- TC-1040 steps 1 to 9 and Expected Results, each traced (examined)
- FR-106 check 2 and `ClauseSelection`; FR-104 Typing and Observations of
  reads; FR-265 criteria fixture; FR-364 Description; TC-1033 fixture
  (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A state-machine invariant over an invocation is unreachable through FR-106, but FR-106 Outputs, FR-107-AC-7, FR-107-AC-11 and TC-1040 steps 6 and 7 depend on it. The new FR-106 Outputs sentence says the admitted value names the selected invocation "for the invariant of a state machine, whose context is the invocation's". FR-106 gives an invariant only the `Current { snapshot, anchor, self }` selection, which carries no invocation. Check 2 refuses `Invocation` selecting an invariant with `wrong_snapshot`/`wrong-observation`. TC-1040 says "Admit each clause's observations by FR-106" and then asks for `Consistent` at `q1` "with `i1` selected". No FR-106 selection produces that value, so the expected receiver path `(q1, invocation i1, receiver, [])` and the `[field placed]`/`[field cancelled]` reads at `i1`'s receiver binding cannot be reached. Today's TC-235 M15 test reaches it only by building `AdmittedObservations` by hand. QSpec FR-208 binds the invariant to an invocation's pre and post contexts, and TC-235 M22 needs this path. The fix is to give FR-106 a selection for a state-machine invariant at an invocation's pre or post observation, with check 2 amended and an AC. Alternatively, name the model-layer entry that builds such an observation and have TC-1040 steps 6 and 7 use it. | spec/functional/FR-106-admit-snapshots-and-invocations.md:78-84, spec/test-cases/TC-1040-a-clause-evaluation-reports-its-receiver-and-read-paths.md:64 |

## Verdict

FAIL, one blocking finding (FND-001).

Traced and correct: every TC-1039 row (up12 and up34 outcomes, ascending
name order with invariants first, postconditions skipped after a false
precondition, the zero-budget stop, the empty set, and the three refusals).
TC-1040 steps 1 to 5, 8 and 9: the four ParentOrder reads and paths, the
short-circuit, the zero budget, the pre/post roots of VersionUnchanged
(changed-version is child 2 to 3 in `cases.rs`), ProbeOrdered's parameter
root and absent pre-call path, GuardedAll's built-list index and reference
steps with the stop at index 1, and PlaceRequires' receiver path. Step 7's
comparison holds on its own: parameter orders before receiver under FR-207
and FR-047-AC-9. It inherits FND-001 only through the `Consistent` path it
compares against.

Code against spec: `evaluate_clause` returns `ClauseEvaluation` with
`receiver_path()`, so the FR-107 Outputs correction matches the code. Making
`receiver_path()` optional and naming the invocation for object receivers
are target-design changes. Today's code fabricates a pre-call context's
invocation from the pre snapshot identity. The spec's "none" follows FR-207's
absence rule. The set API, the fresh meter per clause (ADR-016 FE-3), the
FR-120 conjunction (ascending name order, enabled only when every
precondition is `Completed(true)`) and the merged FR-363 and FR-364 all
agree. The text uses `member` for object selections, which differs from
FR-208's `field` wording. The text says so openly and does not rely on it
silently.

No test parses the touched tables. There is no ticket id or ceremony in the
spec text. No id collides with the open PRs: #876 owns TC-1038 and
FR-106-AC-33 to AC-38, and #875 owns TC-1041.

Author's admitted gaps:
- TC-1040 step 9 can be tested as specified. FR-364 specifies navigation
  evaluation and TC-1034 covers it; FR-363 leaves evaluation out only for its
  own scope. Its implementation waits on the navigation-evaluation ticket, so
  that needs a blocked-by edge, not a spec change.
- `Undecided` has no vector: recorded below.

Recorded (not blocking):
- medium: `Undecided` has no AC or TC vector. A precondition that is
  `Undefined` (for example a division by a field holding 0) would cover it
  and the skipped postcondition.
- medium: "`Undefined` or `Refused`" in the outcome rule should say that it
  covers both the kernel `Outcome` and the `FamilyResult` arms. Read as
  kernel-only, "`True` otherwise" would count a family refusal as true,
  against FR-120's "true when every clause is `Completed(true)`".
- medium: TC-1040's "`Orders` fixture" for the state machine does not name
  its source or the frame of `place`. Admission refuses the `i1` post state
  unless the frame modifies both `placed` and `cancelled`.
- medium: the root-subject list does not cover a `let` binder or a called
  function's parameter as operand (`let s = self in s.versionNumber`).
- medium: AC-12 mixes a half that can be built now with a navigation half
  that waits on navigation evaluation.
- low: FR-107 Inputs still lists only "A `Meter`", while the set takes
  `ScalarLimits`.
- low: TC-1039 step 5 does not say which observations go with
  `sameIdentity`. TC-1040 step 4 does not give `i1`'s post snapshot.
- low: the FR-106 Outputs edit touches the same lines as #875 (merge
  conflict).
- low: qsl-eval comments cite FR-107 by line number (`:92-93`, `:102-105`),
  and this PR shifts those lines. Line-number citations are ceremony.
