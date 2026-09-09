---
id: SR-124
title: "Plan-008 gaps at proof-of-concept engineering delivery"
type: SpecReview
analysis: gap-analysis
scope: "plan/Plan-008-native-lowering/; spec/native-lowering/tests.md; src/lowering.rs and wire.rs; tests/native_lowering.rs and native_backend.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/Plan-008
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TM-006
    type: references
---

## Summary

The implemented compiler slice is reviewable and tested; Plan-008 remains
incomplete because generated activation has not been qualified. The owner
explicitly permits proof-of-concept engineering delivery while retaining that
assurance work, so this report does not impose a dependency on further LC05 work.

## Verdict

**FAIL for complete Plan-008 acceptance** — Task-020 is still in progress.
Engineering delivery is permitted by the owner's 2026-09-09 direction; no claim
of full backend parity, production safety or complete LC04 acceptance follows.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Generated activation remains unverified; Task-020 is incomplete and retained for the later assurance effort. | Task-020; FR-009-AC-5; TC-094 |
| FND-002 | low | The installed matrix grammar requires Coverage Status while the coverage selector expects Status; retain the valid grammar and inspect this column manually. | spec/native-lowering/tests.md |

## Coverage

Reconciliation: quire coverage --scope . --json, CLI 0.31.0, engine
0.46.0@ca7362d4. The scoped TM-006 test-case group is 3/3 backed; all seven
FR-009 criteria have actual trace attributes. There are no scoped unbacked rows
or untracked tests. Source presence does not establish execution success: the
generated activation lane remains explicitly ignored in the ordinary suite.
Manual status inspection confirms that AC-5/TC-094 remain partial.

Tasks done: 1/2. Global reconciliation is 247/255 backed, with 20 pre-existing
untracked NFR-007 metric tags and additional catalog diagnostics outside this
scope; this is not a whole-repository assurance pass. The structural grammar
and status selector disagree as recorded above, so an empty status_lies list
is not used to prove the affected functional table's status correctness.

Reverse-gap inspection maps eight behavior groups to FR-009: Boolean translation,
owner census, source mapping, read/observation correspondence, native/IR identity,
atomic refusals, resource/retry accounting, and strict wire binding/accessors.
Untraced behaviors: 0; source stubs: 0; test stubs: 0. Optional semantic review
was skipped as requested. SR-114/115 retain the code/Rust review and execution
evidence at d58ca7a, including six passing targeted tests, the full ordinary
suite and detected operator/limit mutations. Production and tests are unchanged
by this delivery amendment; no Cargo phase was repeated.
