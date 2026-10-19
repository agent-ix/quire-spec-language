---
id: SR-12915
title: "Failure-domain review of STD-189 (PR #1002): edge cases of FR-381 control-tree commit dominance"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@2e8dd2460; spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md Compensation templates commit bullet and AC-11; spec/test-cases/TC-1058-s3-checks-compensation-registration-retry-commit-and-recovery.md rows 19-20b; agent-ix/quire-specification@ad2b37e spec/functional/protocol/FR-171, FR-426"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-381
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1058
    type: reviews
---
# SR-12915: failure-domain review of STD-189 (PR #1002)

## Summary

Ticket: STD-189. This review looks for control shapes that the control-tree rule decides wrongly, that is, a refusal of a commit that does not dominate (unsound), or for shapes it leaves undecided.

Every refusal the rule produces is a true dominance. A `sequence` starts a later child only after the earlier child completes. A completed `sequence` has completed every child. A `parallel` leaves only when its policy holds over completed branches (QSpec FR-171, FR-426), so a required branch has completed. Every shape not covered by the recursion falls to "not passed", which can only admit: a `choice`, a `repeat`, an `await` arm, an unlisted branch, a `join predicate`, and a smaller `quorum`/`any`. The shapes checked were these:

- An effect nested inside a later `choice`, `repeat` or `parallel` child: refused, which is correct.
- Commit and effect in the same `repeat` body or `case`: refused by the local `sequence`, which is correct.
- Nested parallels inside a required branch: passed through the recursion.
- A duplicated join label: one label, per FR-218.
- Cross-branch send/receive: admitted, as the FR states.

Admitting is conservative because the refusal is a static `declaration-form` error, and FR-057's runtime ordering violations still catch registration after a commit. No shape is left undecided.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean. The rule is sound for refusals and total over the control constructs. The admitted cases where it is incomplete (cross-branch causal edges, predicate joins) are stated in FR-381 as a later extension.
