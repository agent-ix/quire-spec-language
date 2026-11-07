---
id: SR-20261105101087
title: QSL-1739 spec-review/failure-domain
type: SpecReview
analysis: failure-domain
scope: agent-ix/quire-spec-language; QSL-1739 application observation diff, FR-090
  AC-23 through AC-26 and TC-1338
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-090
  type: reviews
---

## Summary

Ticket: QSL-1739. Source-only review of the frozen PR candidate. Runtime, replay and Clippy evidence is pending; this document does not claim an executable acceptance pass.

## Verdict

PASS. No defects found within this method’s examined scope.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

All changed production and test files were inspected; ordinary calls and legacy observer consumers were compared. Canonical Exact events and ChargeAmounts are reused. Whole-call capacity is reduced before re-enabling each interval; truncation stays sticky, admits whole events only, and leaves actual evaluation running. Final global counters remain separate from interval sum/max amounts. Caller event capture is detached and returned; existing charge accounting is untouched. No CI gates changed.

Overflow is resilient and typed, failures retain owned report metadata, unread prior capture is returned intact, and all returns disable capture. Ordered duplicate size entries preserve identity without deduplication. No callback, user side effect, asynchronous lifecycle or new graph traversal is introduced.

No applicable local AssuranceProfile found. Rust API, implementation, safety, testing and wire guidance were read; no new wire/persistence boundary or Tokio work applies. Cargo gates were intentionally not started during the higher-priority gate. The prior test-only compile failure is not runtime evidence; corrected tests have not yet been run.

### Examined units

- FR-090-AC-23 — examined: Given one checked function-body evaluation, `ApplicationObservation` exposes an `outcome` and ordered `losses` equal to that evaluation, along with its ten final Meter counter totals. Its charge ledger contains the fixture's expected requests admitted by `Meter::charge`, in call order, with each point and every requested counter amount. A completed evaluation and a typed refusal retain their full expected ledger; an incomplete evaluation retains its admitted prefix and omits the known refused request. The fixture's expected requests are the ledger oracle; the test does not read the Meter's optional diagnostic log. The observation adds no outcome or accounting semantics.
- FR-090-AC-24 — examined: Given explicit whole-call event and size-request capacities, an observing call that exceeds either capacity returns a typed admitted-prefix ledger with explicitly partial interval amounts; its evaluation and final global meter totals match an adequately sized capture of the same call.
- FR-090-AC-25 — examined: Given an unread caller capture, the observing report returns that capture intact as displaced evidence, including when it was truncated or the observing call fails.
- FR-090-AC-26 — examined: After any observing-call return, event capture is disabled until the caller explicitly enables a new capture.
- TC-1338 — examined: Failed operations retain their checked location even though they have no completed-application value.

## Focused runtime verification supplement

Latest scoped verdict: PASS. Both original code-review findings remain fixed; the independent nested-operation location oracle now passes at runtime. No new finding arose from the narrow fixture corrections. This is a focused review verdict, not final merge acceptance: the combined gate after rebase remains pending.

Read receipts: application_observation integration tests 11 passed; duplicate-request capture unit test 1 passed; legacy total_functions integration tests 17 passed. Replay all-targets cargo check and scoped qsl-eval/qsl-replay all-targets Clippy with warnings denied finished successfully. Evidence was read from /tmp/qsl1739-application-tests.log, /tmp/qsl1739-capture-unit.log, /tmp/qsl1739-total-functions.log, /tmp/qsl1739-replay-check.log and /tmp/qsl1739-clippy.log. Formatter, diff and specification validation passes were reported by the lead; this final pass did not rerun them.

The equality fixture now forms admitted typed Sequence arguments through the real Exact collection constructor, isolating actual EqualityPlan increment/reservation evidence from collection-literal construction. The family refusal fixture admits the K2 model before package assembly and uses its selected type and universe identities with the model profile. Its real dereference failure returns the owned family refusal; assertions inspect the typed catalog and dereference field, retain the expected FunctionCall-only ledger, and consume the actual owned evaluation. No mocked refusal or projected replacement cause was added.

The earlier six test-only Value equality compile errors and subsequent fixture setup failures precede these successful runs; they are not evidence of a production failure or an earlier runtime pass. Production observer semantics are unchanged by these fixture corrections. No new build, global matrix or broader audit was run in this verification pass.
