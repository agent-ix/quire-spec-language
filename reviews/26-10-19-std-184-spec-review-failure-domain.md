---
id: SR-12908
title: "Failure-domain review of STD-184 (PR #998): FR-381 commit dominance over join policies, choices and repeats"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@bc002a60e; spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md Compensation templates commit bullet and AC-11; spec/test-cases/TC-1058-s3-checks-compensation-registration-retry-commit-and-recovery.md rows 19-20a; agent-ix/quire-specification@ad2b37e spec/functional/protocol/FR-171, FR-426, spec/objects/interfaces/FR-464 Derived causal relation and FR-466 parallel layout and Binder scope"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-381
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1058
    type: reviews
---
# SR-12908: failure-domain review of STD-184 (PR #998)

## Summary

Ticket: STD-184. This review looks for executions the new commit-dominance rule classifies wrongly or leaves unstated.

Commit dominance, stated plainly: the author's reading is correct and the labels are not inverted. FR-381 refuses a commit that every execution has passed before the effect registers its obligation (reason `commit-precedes-effect`), because such a commit cuts the compensation off before it can exist. A commit in a branch of `join all [a, b]` with the effect after the `parallel` is passed by every execution that reaches the effect: QSpec FR-171 settles `all` only after every branch settles, FR-426's `join` step fires only when the policy holds over `completed` dispositions, and FR-466 makes the branches' `Out` readable after the parallel only for policy `all`. So TC-1058 row 19a refusing and row 19b (`join any`, an execution can leave through `b` first) checking is the intended behaviour, matching the ticket intent that such a commit SHOULD dominate. The old path definition was wrong because FR-464's `join` edges are drawn from every joined branch whatever the policy, so a path through `b` always skipped `a`'s commit; the new execution definition fixes that, and row 19 (attempt, commit, effect in sequence) is still refused although the `effect` edge bypasses the commit. The remaining defects are in the edge cases of the join classification and in the 19a/19b fixture, not in the direction of the rule.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The execution definition is right, but its join classification is incomplete in two places. (1) `join any [a]` (and a predicate join over `[a]`) is admitted by FR-218/FR-466 with a subset of labels, and FR-466's `joined` is 'the branch labels the join awaits', so it settles only when `a` completes: a commit in `a` is passed by every execution that reaches the effect after the parallel, yet FR-381's positive list names only `join all` and a full `quorum(n)`, and the next clause groups `join any` with joins that do not require the branch. An implementer following the list admits a dominating commit, the defect this ticket exists to close. (2) `quorum(n)` with `n` equal to the distinct labels joined may list a subset of branches (only `all` must list every branch), and 'a commit inside a branch of ... a `quorum(n)`' reads as any branch of the `parallel`, including an unjoined one that no execution must pass. State the rule once by the join's settle condition: a commit in a joined branch `L` is passed by every execution leaving the `parallel` exactly when every set of completed joined branches that satisfies the policy contains `L` (`all`; `quorum(n)` with `n` = |joined|; `any` with one joined label), a branch outside `joined` is never required, and say how a `predicate` join is classified (for example: never required, since its predicate is not evaluated statically). | spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md:230-238 |
| FND-002 | low | The FR text classifies a full `quorum(n)` (dominates), a smaller `quorum(n)` and a `join predicate` (do not), and a commit in a `repeat` body (does not), but AC-11 and TC-1058 test only `join all`, `join any`, a sibling branch and a `choice` case, so a checker that treats every `quorum` like `any`, or a `repeat` body like a sequence, passes. Add AC-11/TC-1058 cases for `quorum(2) [a, b]` (refuses), `quorum(1) [a, b]` (checks) and a commit in a `repeat` body with the effect after the `repeat` (checks). Also drop 'that may run zero times': QSpec FR-464 says every `repeat` 'can be skipped by a false guard', so the qualifier suggests a class of repeats that does not exist. | spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md:238,274 |

## Verdict

Fix FND-001 before merge; FND-002 should land in the same round. The rule's direction is correct and the labels in TC-1058 rows 19a and 19b are not inverted. A commit sequenced between an attempt and its effect (row 19), one in a `join all` branch before the effect (19a) and one before a `choice` that holds the effect in every case all dominate under the new text; a commit in one `case` (20a), in a sibling of the effect's branch (20), in a `join any` branch (19b) and after the effect do not.
