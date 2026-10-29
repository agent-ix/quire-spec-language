---
id: SR-1786002
title: "PR #1786 (QSL-1205) gap analysis: FR-124, FR-232, FR-243, FR-244-AC-6, QSpec FR-368-AC-13..22, FR-331-AC-46, FR-418-AC-10"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; qsl-analyze/tests/it/derived_request.rs, refinement_request.rs, zone_certificate.rs, timed_refinement_certificate.rs; qsl-route request.rs tests at 75bf53dcd"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-124
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-232
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-243
    type: reviews
---

# PR #1786 gap analysis

## Summary

Ticket: QSL-1205. Plan completion: not assessed.

**Covered.**
- **The ticket's end-to-end ask:**
  - A dense `Rpc` request holds `[formula, deadlock-freedom, time-lock-freedom]`. Both derived items are anchored at the `model_root`, route to EN-6 alone, and settle `Proved` through `analyze` with FR-245's check.
  - An untimed `Counter` deadlock-freedom item routes to EN-1 and EN-6. On EN-1 it settles `Proved`, and `limit-reached` at `max_states` 1.
  - `terminal any` writes no derived item.
- **FR-368-AC-22:** two selections give no request item and an `unsupported` diagnostic.
- **FR-368-AC-19 (producer half):** the refinement item's `abstract_subject`.
- **FR-368-AC-20 (producer half):** the limit names.
- **FR-418-AC-10:** an empty `initials` is `malformed`.
- **FR-331-AC-46:** the 50 names, checked against the schema.
- **FR-244-AC-6:** `initials` of length 3.

## Verdict

Mergeable once the code review's FND-001 is fixed. Each gap below is low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | **No test covers a `time tick` subject's derived items.** No test writes these items' routing for a `time tick` subject, so the FR-243 deviation (SR-1786001 FND-001) goes unseen. The probe gives `["en-1", "en-6"]` for both. | FR-243-AC-1, FR-232-AC-1 |
| FND-002 | low | **No test covers a timed refinement subject's derived items through `ModelRequest::derived`.** The case is a timed refinement subject, with or without an authored item over it. It is where `derived` and `time_lock_freedom_items` disagree (SR-1786001 FND-003). | FR-232, QSpec FR-368-AC-17 |
| FND-003 | low | **No test covers a derived item's `max_clocks` reaching EN-6.** EN-6's caller-set `max_states` and `max_transitions` are not tested either. Only EN-1's `max_states` is. | QSpec FR-368-AC-20 |
