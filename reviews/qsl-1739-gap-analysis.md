---
id: SR-20261105101083
title: QSL-1739 gap-analysis
type: SpecReview
analysis: gap-analysis
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

CONDITIONAL. No defects found within this method’s examined scope.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

All changed production and test files were inspected; ordinary calls and legacy observer consumers were compared. Canonical Exact events and ChargeAmounts are reused. Whole-call capacity is reduced before re-enabling each interval; truncation stays sticky, admits whole events only, and leaves actual evaluation running. Final global counters remain separate from interval sum/max amounts. Caller event capture is detached and returned; existing charge accounting is untouched. No CI gates changed.

Plan completion: not assessed

Full repository computed matrix was not run: the dispatch explicitly prohibits an additional global matrix gate. The scoped manual trace index finds AC-23/24/25/26 on real nonignored tests. This is a restricted method run, not a repository-wide gap-analysis PASS. Optional whole-repository semantic review was not requested. No reverse ownership or stub gap found in the changed surface.

No applicable local AssuranceProfile found. Rust API, implementation, safety, testing and wire guidance were read; no new wire/persistence boundary or Tokio work applies. Cargo gates were intentionally not started during the higher-priority gate. The prior test-only compile failure is not runtime evidence; corrected tests have not yet been run.

### Examined units

- FR-090-AC-23 — examined: Given one checked function-body evaluation, `ApplicationObservation` exposes an `outcome` and ordered `losses` equal to that evaluation, along with its ten final Meter counter totals. Its charge ledger contains the fixture's expected requests admitted by `Meter::charge`, in call order, with each point and every requested counter amount. A completed evaluation and a typed refusal retain their full expected ledger; an incomplete evaluation retains its admitted prefix and omits the known refused request. The fixture's expected requests are the ledger oracle; the test does not read the Meter's optional diagnostic log. The observation adds no outcome or accounting semantics.
- FR-090-AC-24 — examined: Given explicit whole-call event and size-request capacities, an observing call that exceeds either capacity returns a typed admitted-prefix ledger with explicitly partial interval amounts; its evaluation and final global meter totals match an adequately sized capture of the same call.
- FR-090-AC-25 — examined: Given an unread caller capture, the observing report returns that capture intact as displaced evidence, including when it was truncated or the observing call fails.
- FR-090-AC-26 — examined: After any observing-call return, event capture is disabled until the caller explicitly enables a new capture.
- TC-1338 — examined: Failed operations retain their checked location even though they have no completed-application value.
