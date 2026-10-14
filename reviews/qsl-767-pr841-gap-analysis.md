---
id: SR-9116
title: "Gap analysis of quire-spec-language PR #841: QSL source inventory envelope"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@78a09cb4470660f0a4becac0e921e993be45745a; QSL-767 contract items 1-4 and AC; src/checking/source_inventory.rs; tests/it/source_inventory.rs"
review_set: subset
---
# Gap analysis of quire-spec-language PR #841

## Summary

Ticket: QSL-767. This is a diff-scoped, planless check of the ticket's
contract and acceptance criteria against the code and tests. The authority
is Contract IR's FR-052 types at 1227928d. The ticket's "count and
completeness" wording predates that contract: IR's envelope carries no count
and no completeness flag.

Examined:
- Item 1 (one public function returns the envelope with owner, source and
  scope): `pub fn qsl_inventory` in the public `checking::source_inventory`
  module. It returns `SourceInventoryOwner::Qsl` and the
  `QslCheckedRequirementsAndPendingObligations` scope. Test 1 asserts owner,
  scope and source (met)
- Item 2 (exhaustion and unfinished declarations give a typed cause, never a
  silent partial): tests 3 and 4 assert `TypeWorkExhausted`, with its native
  fields, and `TypeAdmissionUnfinished { declaration }` (met)
- Item 3 (requirement and pending sets stay separate): test 2 (met)
- Item 4 (no tracking file, no copy of CG/IR types): IR types are used
  through the `quire-contract-model` git dependency. No file or version
  tracking is added (met)
- AC vectors (complete source, exhausted, unfinished, separate sets): one
  test per vector. Each can fail; see the mutation list in SR-9115 (met)
- Stubs and coverage inflation: none. The builder is live code and every
  test asserts typed fields (examined)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

**PASS.** Ticket items 1-4 and the four AC vectors are met by tests that can
fail.

Recorded, not reported:
- medium: the "complete source" vector is built from two unrelated fixtures,
  not from one source. See SR-9115.
- low: no QSL FR owns `qsl_inventory`, and the tests carry no criterion
  trace tag. The owning requirement is IR FR-052.
- The CG IR-740 adoption control lives in IR-740 and is not assessed here.
