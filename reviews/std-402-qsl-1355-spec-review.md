---
id: SR-16300
title: "Spec review of quire-spec-language PR 1355: STD-402"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@f74f95ac; spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md; spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md; spec/test-cases/TC-483-assembler-admits-dimensions-and-units.md; spec/test-cases/TC-685-s3-checks-time-declarations-and-clock-constraints.md"
review_set: subset
date: 2026-10-23
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-483
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-685
    type: references
---

# SR-16300: Spec review of quire-spec-language PR 1355 (STD-402)

## Summary

Ticket: STD-402. Head f74f95ac (PR 1355, contains 1369 head 9412adb0). Own delta: commit d2eb394a (FR-091, FR-230, TC-483, TC-685). The new rules (a package dimension is reached as `M::Name`, a bare name reaches source dimensions only, two `Time` nodes are two dimensions, the model's time dimension is the dimension of its `time` unit) match QSpec FR-142 "Namespaces" and FR-142-AC-19/21 in the stack (QSpec PR 407). FR-230-AC-18's three-package vectors hold by hand. The refusal of same-named dimensions is removed in FR-230 and FR-230-AC-5 but not in FR-091 and TC-483/TC-406 (FND-001).

Also checked:

- FR-091-AC-50 and FR-230-AC-18 are unique across the other open PR heads. Gate exit 0, 20 warnings (no new). Merge-tree with main clean; conflicts with open PRs 1335, 1333, 1314, 706.
- Decision 2 in the PR body (no explicit cross-package dimension equivalence construct) is a design remark, not a deferral in the delta: no spec text promises or postpones one. If Peter wants an equivalence declaration it needs its own STD ticket.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The PR makes same-named dimensions in a source and a package, or in two packages, distinct and refusing nothing (FR-091-AC-50, FR-230-AC-5, FR-230-AC-18; QSpec FR-142-AC-19 now refuses a bare `Rate`). It leaves the opposite rules in place: the FR-091 Catalog codes row 'dimension name shared with a domain package -> conflicting_declaration/duplicate-declaration' (line 988), FR-091-AC-21's matching cause (line 1047), FR-091-AC-55's 'Dimension-name collisions' clause that refuses `dimension Time;` beside `acme/rpc`'s `Time` and two packages' `Rate`, and its claim that `unit tick : Rate = 1/2 * M::us` with a package dimension is admitted (QSpec now makes that `missing-name`), TC-406 line 46, and TC-483 step 8 with its expected collision refusals (lines 54 and 99). The same input is therefore both refused and checked, and TC-483 step 8 fails against AC-50. Fix: delete the collision cause row and clause, qualify the `tick` unit as `M::Rate`, and update TC-406 and TC-483 step 8. | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:1077 | wrong-requirement |
| FND-002 | medium | FR-230-AC-18 says `time dense unit F::sec` checks 'with the `acme/flow` `Time` as the time dimension, and so does the source `time dense unit s`'. The source `s` is typed by the source `Time`, not the `acme/flow` one, and the AC also requires three different package identities. Fix: say the source `time dense unit s` checks with the source `Time` as the time dimension. | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:294 | wrong-requirement |

## Verdict

**Not mergeable.** FND-001 (high) block the PR; FND-002 (medium) should be fixed in the same round.
