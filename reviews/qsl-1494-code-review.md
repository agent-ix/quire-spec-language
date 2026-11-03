---
id: SR-2026110216374
title: "Code review — QSL-1494 progress ledger and replay integration"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-eval/src/temporal/ledger.rs, qsl-eval/src/temporal/mod.rs, qsl-foundation/src/setting.rs, qsl-replay/src/clause_evaluation.rs, qsl-replay/src/execute/frame.rs, qsl-replay/src/execute/state_clause.rs, qsl-replay/src/limits.rs, qsl-replay/src/spine.rs, qsl-replay/src/spine/clause.rs, qsl-replay/src/spine/clause/temporal.rs, qsl-replay/src/spine/clause/tests/refinement.rs, qsl-replay/src/spine/clause/tests/temporal.rs, qsl-replay/src/spine/native_result.rs, qsl-replay/src/spine/refinement.rs, qsl-replay/src/spine/request.rs, qsl-semantics/src/check/refusal.rs; FR-502 AC-1..8, FR-052 AC-13, FR-330 AC-9"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-502
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-330
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-052
    type: reviews
---

## Summary

Reviewed PR #2168's Rust progress-ledger implementation and replay API changes against FR-502, FR-052 AC-13, and FR-330 AC-9. The ledger's standalone contradiction rules are substantive, but the replay path never admits progress or returns an updated ledger, so the ticket's central route behavior is not implemented.

## Verdict

**FAIL** — one high-severity finding leaves progress-carrying temporal runs impossible; a second finding notes that binding identity correctness depends on caller-managed strings.

## Coverage

The changed Rust files and existing replay/evaluator callers were inspected. Rust review covered public API documentation, immutable state transitions, error mapping, resource bounds, and test assertions. The review also inspected the applicable acceptance criteria: FR-502 AC-1 through AC-8, FR-052 AC-13, and FR-330 AC-9. FR-501/FR-504 were read as dependency context; their evaluator ownership is outside this PR's finite-window implementation scope. The PR's own CI reports `cla / gate` passed. Local `cargo fmt --check` passed. `cargo deny check` could not acquire `/home/peter/.cargo/advisory-dbs/db.lock` because that path is read-only. The first local locked qsl-eval/qsl-replay library test run timed out after 300 seconds; a second cold-build attempt was stopped before producing test results. Local focused test results are unavailable.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The replay route cannot accept or retain progress. `TraceInput` has only `Finite` and `Lasso`, `carries_progress()` returns false for both, and `run_temporal` returns before calling the new `admit_progress` helper whenever a ledger is supplied. Consequently no accepted request can update `ClauseRunReport.ledger`, and no replay-level regression can produce the required evaluate-stage contradiction. Add the progress-carrying replay trace input and call the admission path so a successful run returns the updated ledger; test the route end to end. | qsl-replay/src/spine/clause/temporal.rs:40; qsl-replay/src/spine/clause/temporal.rs:59; qsl-replay/src/spine/clause/temporal.rs:250; qsl-replay/src/spine/clause/temporal.rs:349 | correct-requirement-no-evidence |
| FND-002 | medium | `LedgerBinding` represents checked package and clause identities as unrestricted strings and exposes a public constructor-by-fields; the ledger can therefore merge progress for distinct checked identities if a caller supplies display names or a lossy encoding. Use the repository's checked identity types or an unambiguous identity constructor so the whole-binding isolation required by FR-502 does not depend on caller convention. | qsl-eval/src/temporal/ledger.rs:23; qsl-eval/src/temporal/ledger.rs:224 | implementation-bug-despite-evidence |

## Rust Review

The immutable `admit` API supports atomic rejection, and the unit tests exercise watermark, completeness, closure, binding variation, covered positions, and capacity. The main defect is the missing connection from `run_temporal` to that API. The standalone ledger tests cannot establish report propagation or route-level contradiction classification. The binding API also lacks type-level protection for the checked identities specified by FR-502.

## Acceptance Coverage

- FR-502 AC-1..3 and AC-7..8 have ledger-unit-level assertions, but no evaluator/report-level coverage for earlier result stability or returned progress.
- FR-502 AC-4 tests string-key separation, but not isolation by actual checked package/clause identities or a `Pending` temporal outcome.
- FR-502 AC-5 tests refusal for ledger-bearing non-progress traces, but the accepted `Timestamped` and `Progressed` cases cannot exist in the changed `TraceInput`.
- FR-502 AC-6 tests immutable ledger transitions, not accepted `ClauseRunReport` propagation through consecutive requests.
- FR-052 AC-13 and FR-330 AC-9 are not implemented end to end by this diff.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `TraceInput` now has `Progressed` and `Timestamped` cases; `run_temporal` admits progress and returns the updated ledger, and a replay test verifies update then unchanged ledger on watermark regression. |
| FND-002 | fixed | `LedgerBinding.package` now uses `SourceIdentity` and `clause` uses `NodeKey`, preserving the checked identities in the key. |

## New findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | The covered-position set drops each position's stamp and compares only its observation identity. If the same observation appears at a new stamp at or before a retained complete-through value, `check_progress` treats it as already covered and accepts it instead of refusing the previously uncovered position. Retain the full position identity, including stamp, in the coverage set and test this case. | qsl-eval/src/temporal/ledger.rs:63; qsl-eval/src/temporal/ledger.rs:93; qsl-eval/src/temporal/ledger.rs:240; qsl-eval/src/temporal/ledger.rs:306 |


**Round 2 disposition.**

| FND | outcome | reason |
| --- | --- | --- |
| FND-003 | still-open | The fix commit still stores covered positions as identity strings only. A second position with the same observation identity and a different stamp is therefore treated as covered. |
