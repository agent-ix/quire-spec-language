---
id: SR-15459
title: "EARS review of STD-245 changed SHALL statements (QSL PR #1212)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@4fabd79f984475fc42ba12336433f44d432d6b91; spec/decisions/ADR-017-protocol-refinement-abstraction-boundaries.md, spec/functional/FR-070-implement-typed-counterexample-witness-envelope.md, spec/functional/FR-071-implement-typed-replay-request.md, spec/functional/FR-098-execute-a-replay-request.md, spec/functional/FR-116-replay-a-frame-counterexample.md, spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md, spec/test-cases/TC-183-witness-envelope-missing-member-refusal.md, spec/test-cases/TC-185-replay-request-exact-members-round-trip.md, spec/test-cases/TC-444-the-replay-executor-recompiles-selects-calls-and-refuses.md, spec/test-cases/TC-515-replay-a-frame-counterexample.md, spec/test-cases/TC-516-locate-a-function-call-site.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-116
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-121
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: reviews
---
# SR-15459: EARS review of STD-245 changed SHALL statements (QSL PR #1212)

## Summary

Ticket: STD-245. These are the new or changed statements: FR-070 "Reconstruction SHALL refuse ...";
FR-071 "The replay target SHALL hold typed names only ..." and "The reader SHALL refuse a request by QSpec
FR-323's decode rule 1 ..."; FR-098 "If the request's replay target is not `function`, then the executor
SHALL refuse ..."; FR-116's two new "If ..., then the executor SHALL refuse" bullets and its
`frame_operation` resolution bullet; FR-121 "For an operation selection, `call_site` SHALL return the
resolved operation's `frame_operation` ...". `make check-spec-validation` warns on FR-071 line 160 (the
closing clause of the typed-names bullet) and on FR-121 lines 140 and 184. The FR-121 warnings are on
unchanged text.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | minor: the rewritten bullet ends "... and no implicit string-to-`QualifiedName` or string-to-`Identifier` conversion SHALL exist". Quire flags it (line 160: ears:unclassifiable, ears:missing-subject) because the SHALL has no system subject. The pattern was already on main. Since the bullet is rewritten anyway, give the clause a subject ("the request type SHALL provide no implicit ... conversion"). | spec/functional/FR-071-implement-typed-replay-request.md:156-160 |

## Verdict

Mergeable for EARS. One minor finding.

## Dispositions

Round 1, reviewed QSL #1212 at 373ddf489.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | fixed at 373ddf489 (the quire ears warning on FR-071 is gone) |
