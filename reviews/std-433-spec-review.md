---
id: SR-16259
title: "spec-review of FR-337 citing QSpec FR-370-AC-13 for the over parameter (quire-spec-language PR 1367)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@1bd5eb46; spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md; spec/test-cases/TC-1079-a-temporal-clause-node-carries-its-root-and-over-node.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: references
---

# Spec review of FR-337 citing QSpec FR-370-AC-13 (quire-spec-language PR 1367)

## Summary

Ticket: STD-433. One spec-review pass over `git diff origin/main...1bd5eb46`.
The PR deletes FR-337's "Remaining work" section and cites QSpec
FR-370-AC-13 under References as the owner of the rule that FR-337-AC-5 and
TC-1079 step 3 apply.

The oracle was QSpec FR-370-AC-13 at QSpec main d2fab1b0: a `temporal` clause
with no `over` parameter refuses `invalid_syntax`/`unexpected-token` at the
token standing where `over` is required and produces no node. FR-337's
Behavior (lines 68-73), FR-337-AC-5 and TC-1079 step 3 already state this
rule. The vector `temporal T using inf clock "c" on origin { eventually
holds(true) }` refuses at `clock`. That is correct, because QSL's clause
syntax is `temporal N using <profile> over (x: T) clock ...` (ADR-018 §6), so
`clock` is the token that stands where `over` belongs. QSpec TC-180 uses the
same vector and the same token.

Nothing is dropped. The deleted section was a deferral note with no
behaviour left to specify. Gates: `make check-spec-validation` at 1bd5eb46
exits 0, with no warning on a touched file. The PR merges cleanly with
origin/main and with #1332, #1310, #1311, #1343, #1346, #1327 and #1356.
origin/main is one commit ahead and is not an ancestor of the head. The PR
adds no new ids.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Mergeable. The citation is accurate, the vector agrees with QSpec TC-180, and
no deferral language is left in the touched files.
