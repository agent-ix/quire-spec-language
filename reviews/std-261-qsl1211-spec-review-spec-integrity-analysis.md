---
id: SR-15454
title: "integrity review of STD-261 FR-122 state_clause selection (QSL PR #1211)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@f2331483313b4bc808636afafed19f23eb301b77; spec/functional/FR-122-replay-a-state-clause-counterexample.md, spec/test-cases/TC-517-replay-a-state-clause-counterexample.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-122
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-517
    type: reviews
---
# SR-15454: integrity review of STD-261 FR-122 state_clause selection (QSL PR #1211)

## Summary

Ticket: STD-261. Diff reviewed: `git diff origin/main...f23314833` (FR-122, TC-517), read together
with QSpec #364 and QSL #1212.

Checks: FR-122 now selects by the request's `state_clause`, the payload has no `clause`, and resolution
uses FR-109's single name table, with `missing-name` naming the target. This matches FR-323 and AC-21 at
#364's head. FR-122-AC-8 and TC-517 step 8 are new, and the ids are unique among the open QSL PRs (#995's
"FR-122-AC-2" is a QSV id). The CG consumer (quire-contract-codegen FR-024 on main at 43d5a6f, lines 242
and AC-12) still sets the payload's `clause`. The PR body names this as CG follow-up. Option (b) is sound
(see SR-15450).

Merge order: FR-122 reads "its replay target (the packet's copy of the request's, FR-070)". On main,
FR-070 has no such member (it carries `selected_function`), so #1211 must merge after #1212.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | major: FR-122 depends on the envelope and the request carrying a `state_clause` replay target (FR-070, FR-071). At #1212's head, FR-070 and FR-071 type only two members ("a function packet's `function` ...; a frame packet's `frame_operation`" and "a function's `QualifiedName` ...; a frame's `{object, operation}`"). Neither FR-070-AC-18 nor FR-071-AC-14 round-trips a `state_clause` target or refuses a pair that includes `state_clause`. Once both PRs merge, no QSL requirement or AC makes the request or envelope type carry the member that FR-122 and FR-323 require. Extend FR-070, FR-071, AC-18, AC-14, TC-183 and TC-185 with `state_clause` (a state-clause packet's and request's `state_clause` QualifiedName, its round trip, and a pair refusing ConflictingTarget or `/replay/state_clause`). Do this in #1211 after rebasing on #1212, or in #1212. | spec/functional/FR-122-replay-a-state-clause-counterexample.md:89-96 |
| FND-002 | low | minor: removing the payload's `clause` breaks CG FR-024 (quire-contract-codegen main 43d5a6f, "The generator shall set the state-clause payload's `clause` ..." and FR-024-AC-12). The PR body calls this a CG-lane follow-up but names no ticket. Name or file the CG ticket, for example on IR-958's Q-6, so the consumer change is tracked. | spec/functional/FR-122-replay-a-state-clause-counterexample.md:56-60 |

## Verdict

Not mergeable: FND-001 (major). The FR-122 text itself agrees with QSpec #364. Merge after #1212.
