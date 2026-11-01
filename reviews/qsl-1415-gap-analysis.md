---
id: SR-202610310138
title: "Gap analysis — QSL-1415 checked temporal limits v2"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; NFR-008-AC-1..5 and the checked-package v2 temporal-limit successor"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-008
    type: reviews
---

## Summary

A focused manual traceability pass checked NFR-008-AC-1..5 against PR #2007's checked-package v2 tests and source. Every AC has a trace-tagged successor test, but AC-4's restoration-state identity branch is not exercised on the checked-v2 path; the repository-wide computed matrix could not be obtained because `quoin matrix --repo . --json` remained silent and running.

## Verdict

**CONDITIONAL** — the tagged successor tests cover the named scenarios, with one AC-4 branch still lacking v2 evidence.

## Coverage

- NFR-008-AC-1: two tagged tests cover future overflow and past-history underflow before valuation work.
- NFR-008-AC-2: tagged tests cover seven typed limit stops and preservation of settled siblings on visit exhaustion.
- NFR-008-AC-3: a tagged test covers the explicit eviction-list path returning typed incompleteness.
- NFR-008-AC-4: tagged tests cover changed limit identity and 100,000-deep evaluation on a 512 KiB stack. No checked-v2 test varies admitted restoration state and compares result identity.
- NFR-008-AC-5: a tagged test iterates zero, exact, one-short, raised ceilings, and retries across the seven dimensions.
- Reverse-gap and stub review was limited to changed temporal files; no new unowned behavior or live-path stubs were found in those files.
- Plan completion: not assessed.
- Semantic review: skipped; it was not requested.
- Computed matrix: unavailable; command remained silent and running for over 35 seconds, then was interrupted. Manual tag inspection was used.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The checked-v2 AC-4 successor does not test result identity when admitted restoration state changes, leaving that branch without successor evidence before legacy tests are eligible for deletion. | tests/it/checked_temporal_limits_v2.rs:495 | correct-requirement-no-evidence |

## Traceability

| Requirement | Test binding | Result |
| --- | --- | --- |
| NFR-008-AC-1 | `checked_v2_temporal_composition_refuses_i64_overflow_before_work`; `checked_v2_past_history_composition_refuses_i64_underflow_before_atoms` | Tagged and exercised |
| NFR-008-AC-2 | `checked_v2_limits_stop_all_seven_dimensions_without_truth`; `checked_v2_exhaustion_preserves_settled_sibling_assessment` | Tagged and exercised; sibling control uses visit exhaustion |
| NFR-008-AC-3 | `checked_v2_eviction_is_an_incomplete_result_not_a_narrower_window` | Tagged and exercises explicit eviction list |
| NFR-008-AC-4 | `checked_v2_effective_limits_participate_in_result_identity`; three 100,000-depth tests | Limit and stack cases tagged; restoration-state identity is not covered on this path |
| NFR-008-AC-5 | `checked_v2_retries_are_charge_exact_and_leave_inputs_unchanged` | Tagged and iterates all seven dimensions |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | still-open | The scan exhaustion tests do not exercise a retained instance or retry the same input after a visit refusal; add that successor case to establish AC-5 for the mutable ledger path. |
| FND-002 | fixed | - |
| FND-003 | still-open | The AC-5 retry test forces exhaustion during the eviction scan, not after the scan at the formula root. It therefore cannot detect receipt mutation on the later failure path. |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |

## Disposition evidence

The checked-v2 test `checked_v2_restored_capture_state_participates_in_result_identity` now seeds two distinct retained capture values, replays each restored state, verifies repeat identity, and asserts the two restored results differ.

## Disposition pass 2

No new gap-analysis finding required a disposition row in this round. FND-001's latest outcome remains `fixed` from round 1; the new scan and retention-accounting regression tests were reviewed against their NFR-008 bindings.


## New findings (disposition pass 2)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-002 | medium | The checked-v2 retry tests do not exercise a visit-limit refusal during retained-capture eviction scanning followed by retry, so the AC-5 unmutated-state guarantee is unverified for the mutable ledger path. | tests/it/checked_temporal_limits_v2.rs:435 | correct-requirement-no-evidence |

### FND-002 — retained-ledger retry after scan refusal lacks coverage

The new capture scan test starts with a fresh obligation and checks only the typed visit exhaustion and usage. It does not seed a `Ledger`, trigger the existing-instance scan, or retry after the scan refusal. Consequently, it does not detect the receipt mutation found in code review FND-005.


### Disposition pass 3 evidence

`checked_v2_retained_capture_scan_refusal_retries_without_duplicate_receipt` exercises the retained ledger path, confirms visit exhaustion, retries the unchanged input, and asserts the expected receipt list and restored capture.


## New findings (disposition pass 4)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-003 | medium | The checked-v2 retry suite does not test visit exhaustion after eviction scanning completes but before the formula's first visit, leaving the retained-ledger retry branch uncovered. | tests/it/checked_temporal_limits_v2.rs:470 | correct-requirement-no-evidence |

### FND-003 — post-scan formula refusal is not covered

The new regression asserts a visit limit of 2 with eight eviction records, so it stops inside the scan. It does not exercise a limit that permits the trigger and full scan, then refuses the formula root, and retries the same input.

### Disposition pass 4 evidence

- FND-001 and FND-002 remain fixed through the prior checked-v2 identity and retention-demand regressions.
- FND-003 remains open because the AC-5 retained-ledger test sets `visits: 2` with eight eviction entries and therefore never exercises formula-root exhaustion after the scan completes.

### Disposition pass 5 evidence

- FND-001 and FND-002 remain fixed: checked-v2 result identity covers retained restoration state and the scan-refusal retry regression tests the mutable ledger path.
- FND-003 is fixed: `checked_v2_formula_refusal_retries_without_duplicate_receipt` uses visits=2, enough to complete the one-entry eviction scan after trigger admission, then exhausts at formula evaluation. It retries the same ledger with visits=16 and asserts the receipt occurs exactly once and capture value `17` survives.
