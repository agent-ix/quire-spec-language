---
id: SR-20261104020302
title: "Gap analysis — QSL-1609 restore the root it suite on main (PR #2241)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@ada7a08c1 trial-merged on origin/main 37b4a8ffb; PR #2241 diff; FR-501, FR-043, FR-044, FR-322-AC-294, FR-484-AC-11"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-501
    type: reviews
---

## Summary

Ticket: QSL-1609. Gap analysis of the PR diff against the specs. The product change in `src/temporal.rs` restores behavior the existing TC-1278/TC-1279/TC-1295 tests in `checked_temporal_successors` and `compiled_protocol_v2` already assert; those tests fail on origin/main and pass with the fix (4 of 58 selected tests fail on the old code). No tag was added or removed. Test-side edits restate fixtures to FR-322-AC-294 (clock selection required), the #2184 protocol-profile kind rule, FR-322 "Operand index segments" and ADR-011 layering, without changing an assertion's strength. Trial merge on origin/main 37b4a8ffb: fmt, check, clippy (default and no-default-features), arch-lint qualified-core and api-surface --qsl-only all exit 0; the root it suite is 878 passed, 0 failed, 6 ignored (none added by this diff); root lib tests 35 passed.

## Verdict

**MERGEABLE.** One low gap.

## Scope

- FR-501 timestamped-window tick intervals and FR-043 tick semantics: examined against `checked_interval`.
- FR-322-AC-294 (a configured temporal clause with no selection is refused): examined; the helper supplies one selection per configured clause and the emitter refusal itself is still covered by its own tests, which call `emitted` without selections.
- FR-484-AC-11 (lock selects profiles, empty selection list, replay): examined; assertions unchanged.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No requirement or test names the checked-to-wire route's handling of an open or non-integer Timed interval (a refusal in `checked_interval` versus a fault in the lowering's `temporal_interval`). The two layers disagree on the kind of outcome for the same invariant break. Fix: have the spec lane state which one owns the case, then tag a test. | src/temporal.rs:1299 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | deferred | Owning requirement for open or non-integer Timed intervals filed as QSL-1611 (spec lane). |
