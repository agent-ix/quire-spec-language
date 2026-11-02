---
id: SR-202611021417
title: "Gap analysis — QSL-1467: a budget stop while classifying is a resource stop"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR #2118 head 354364ecf; qsl-eval/src/simulation/protocol/access.rs; qsl-analyze/tests/it/weak_memory.rs; spec/functional/FR-120-simulate-a-checked-package-s-state-family.md; spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md; spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md; spec/test-cases/TC-665-access-orderings-and-classification.md"
review_set: subset
---

## Summary

Ticket: QSL-1467. PR: quire-spec-language#2118, head 354364ecf, against origin/main 4a7cef38e. This is a planless gap analysis (plan completion was not assessed), scoped to the PR diff. It checks the PR's behaviour against the requirements it cites (FR-120, FR-220), the model-check requirement that governs an expansion stop (FR-126), and the two new tests, including mutation checks.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-120 "Post-states": a count over `max_candidates` stops the expansion with `resource_exhausted`/`insufficient-next-charge`, naming `MaxCandidates` | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:631-643 | examined |
| FR-126: an expansion that stops on the meter or on `max_candidates` SHALL return `Stopped{ResourceExhausted, limit}` | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:306-313 | examined |
| FR-220 EN-1 classification (AC-11 to AC-15) | spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md | examined |
| TC-665 (verifies FR-220) | spec/test-cases/TC-665-access-orderings-and-classification.md | examined |
| a_budget_stop_is_a_stop_and_a_broken_one_a_fault | qsl-eval/src/simulation/protocol/access.rs:1404-1449 | examined |
| a_candidate_bound_reached_while_classifying_is_a_resource_stop | qsl-analyze/tests/it/weak_memory.rs:2631-2657 | examined |
| ProtocolSystem::new classify order (Enabled, then Potential) | qsl-eval/src/simulation/protocol.rs:1028-1047 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The Potential-mode site is not covered. `ProtocolSystem::new` classifies in Enabled mode first, so with `max_candidates` 0 the Enabled pass stops construction before Potential runs. Mutation M3 changes only the Potential `map_err` (`attempt_identities`) back to `fault("attempt-identities-list")`, and both new tests still pass. Potential mode counts every attempt identity regardless of guards, so a bound that Enabled fits within and Potential exceeds is reachable and untested. The same holds for the `cell_of` site (M4 survives), though `cell_of` only walks rows and charges no candidates. | qsl-eval/src/simulation/protocol/access.rs:816-818; qsl-eval/src/simulation/protocol.rs:1038-1047; qsl-analyze/tests/it/weak_memory.rs:2638 |
| FND-002 | medium | No acceptance criterion states what EN-1's pre-check does when classification reaches a budget. FR-220 has no budget or stop clause. FR-120 "Post-states" defines the stop for one expansion, not for the pre-check. FR-126 requires a stopped expansion to settle as `Stopped{ResourceExhausted, limit}`, while the PR settles it as a `ProtocolRefusal` (code-review SR-202611021416 FND-001). Both tests carry `#[trace("TC-665", "FR-120", "FR-220")]`, but TC-665 verifies only FR-220, its procedure has no budget step, and neither test names an AC. The behaviour is unspecified and the trace claims coverage that no criterion backs. | qsl-analyze/tests/it/weak_memory.rs:2636; qsl-eval/src/simulation/protocol/access.rs:1415; spec/test-cases/TC-665-access-orderings-and-classification.md:1-8 |

## Verdict

Not mergeable until FND-001 and FND-002 are addressed, together with code-review FND-001. The eval-level split is otherwise tested well:

- Baseline: both new tests pass (qsl-eval lib 1 passed; qsl-analyze `it` 1 passed).
- M1, `classification_stop` always a fault: both tests fail.
- M2, always `InitialStop`: the unit test fails on the broken-invariant case. The integration test passes, as expected, because it covers only the budget case.
- M3, Potential site always a fault: both pass. The mutant survives (FND-001).
- M4, `cell_of` site always a fault: both pass. The mutant survives; `cell_of` charges no budget, so this is near-equivalent.

The integration test runs under both `tso` and `ra` and confirms that the default bound builds. Its assertions name `MaxCandidates` and `resource_exhausted`/`insufficient-next-charge`, so they are not tautological. There are no `#[allow]`, no caps and no compatibility layer.
