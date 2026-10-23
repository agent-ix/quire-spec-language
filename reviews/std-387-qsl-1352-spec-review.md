---
id: SR-16331
title: "spec-review of the constraint clause from S1 to S4, constraint advertisement, reduction charges and constraint-undefined replay (quire-spec-language PR 1352)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@a85170a3e; spec/functional/FR-102; FR-105; FR-128; FR-152; FR-158; FR-159; FR-161; TC-456/462/539/569/581/583/586"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-102
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-152
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-158
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-159
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-161
    type: references
---

# Spec-review of the constraint clause from S1 to S4, constraint advertisement, reduction charges and constraint-undefined replay (quire-spec-language PR 1352)

## Summary

Ticket: STD-387. One spec-review pass over `git diff origin/main...a85170a3e`, checked for the no-drop/no-narrowing/no-deferral rule, testable vectors (key vectors derived by hand), id uniqueness across all open PR heads, cross-spec names, and mergeability against main.

Examined, clean:

- FR-102-AC-8, FR-105-AC-10 and FR-158-AC-6 are consistent with each other and with QSpec #406 (node pair `state`/`state_clause`, clause member `constraint`, anchor and binding as an invariant). The FR-331 locus list on main already names `constraint`.
- FR-128-AC-8 was derived by hand: `6 mod (2 - self.v) >= 0` holds at v 0 (0 >= 0) and v 1, and is undefined at v 2, so the first reached state with a counter at 2 refutes with cause division-by-zero. The payload with its last step removed ends at v 1, where the constraint is defined, so `ReplayParity` follows.
- FR-161-AC-5: a concrete step charges two (one transition identity, one state), so the 3-step loop of AC-1 charges 6 and the 2-step lasso of AC-2 charges 4. The least-M argument is robust to whether the limit stops on reaching or on exceeding.
- G1 to G4 of STD-387 are all addressed in the PR; the QSL FR-159 `constraint` spelling agrees with QSpec's backend-provider schema enum.

Merge status: Conflicts with origin/main d0a756b2b (47 commits behind): FR-128 (main added the reading-order rule and AC-7), FR-158 (main added protocol subjects and AC-5), TC-539, TC-581. No conflict with the other open PRs touching these files that the PR body names. Resolve as main plus this PR's additions; FR-158-AC-6 stays AC-6.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | AC-5 asserts exact charges that the new Cost rule does not fix. The rule bounds comparisons by `n * ceil(log2 n)` ("at most"), so an implementation making 2 comparisons for the 2-key class `[b, c]`, or 5 for `[a, b, c]`, conforms to the SHALL and fails AC-5 ("charges exactly 2", "at most 4"). The `[[b, c]]` budget-1 vector depends on the same exact count. Fix: state the algorithm (the comparison count of a named sort) in the rule, or make the AC assert only the bound and the denial point it implies. | spec/functional/FR-152-canonicalise-a-model-state-by-sorting-its-symmetry-classes.md:108 | wrong-requirement |
| FND-002 | medium | The constraint-undefined replay rule is stated for model counterexamples only. Main's FR-158 now applies the constraint to protocol subjects (`ProtocolSystem`, STD-380), so a protocol run can refute with `UndefinedEvaluation` at a constraint locus, but FR-217 and FR-227 replay an `UndefinedEvaluation` only by "the claim's letters along the replayed prefix" and say nothing of a constraint locus. That counterexample has no replay rule. If protocol replay is outside G4, a new STD ticket must own it; as written the feature is half-specified. | spec/functional/FR-128-replay-a-model-counterexample.md:223 | missing-requirement |
| FND-003 | low | FR-290 step 4 still gives `state-constraint` as an example of a QSL outcome-document spelling outside FR-331. FR-159 now makes `constraint` the one spelling wherever a reduction is named, so the example names a spelling that no longer exists (qsl-replay `ReductionKind::as_str` is the code consumer). Pick another example or confirm `state-constraint` is a distinct spelling. | spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:149 | wrong-requirement |

## Verdict

**Not mergeable as it stands**: four content conflicts with main (FR-128, FR-158, TC-539, TC-581) and two medium findings. No high finding, and no deferral or narrowing language in the PR's added lines. Gate `QSPEC_DIR=<QSpec #406 head> make check-spec-validation`: exit 0, 19 warnings (the baseline count; the 2 on a touched file are at FR-104 line 43, not in the delta). Ids unique across 61 open heads (FR-158-AC-5 is main's, so AC-6 is free).

## Dispositions

| FND | Round | Outcome | Reason |
| --- | --- | --- | --- |
| FND-001 | 1 | fixed | Fixed in head 38d14082; the fix agrees with the finding and the vectors were derived by hand. |
| FND-002 | 1 | fixed | Fixed in head 38d14082; the fix agrees with the finding and the vectors were derived by hand. |
| FND-003 | 1 | fixed | Fixed in head 38d14082; the fix agrees with the finding and the vectors were derived by hand. |

Round 1 verdict: All three findings fixed, no new defect; mergeable (merge-tree with main clean; gate exit 0, 19 warnings).
