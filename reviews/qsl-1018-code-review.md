---
id: SR-16358
title: "Code and Rust review of QSL-1018 named evaluation meter stops"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #1410 (branch qsl-1018-impl); qsl-replay/src/model_check/limits.rs, qsl-replay/src/statistical/settle.rs, qsl-analyze/tests/it/model_check.rs, qsl-analyze/tests/it/model_check_safety.rs, qsl-analyze/tests/it/model_settle.rs"
review_set: subset
---

## Summary

Ticket: QSL-1018. Review pass (round 0) by run f859fcbd-414c-44f6-9b9f-a7bb3332a988
at frozen head 6777c521f; this file was reconstructed from that run's posted
Linear comment, which carried the authoritative findings. Disposition round 1
checks fix commits e9a552614 and e3fbb05aa (head e3fbb05aa).

## Verdict

**APPROVED (round 2).** Round 2 at a608d9d22 fixes FND-001. Round 1: FND-002 is fixed. FND-001 is still open: the
cited limits.rs test no longer claims FR-255-AC-9, but three other stop-name
tests this PR adds still bind FR-255-AC-9 in `quire matrix`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | False FR-255-AC-9 trace tags claim settings refusal coverage in stop-name tests | qsl-replay/src/model_check/limits.rs:197 |
| FND-002 | medium | Statistical settlement test falsely claims FR-127-AC-16 model-check settlement coverage | qsl-replay/src/statistical/settle.rs:387 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | limits.rs test retagged to ("TC-521","FR-126-AC-19") (fixed), but `quire matrix` at e3fbb05aa still binds FR-255-AC-9 to three PR-added stop-name tests that never call the settings operation: qsl-analyze/tests/it/model_check.rs:561 `#[trace("TC-521","FR-126-AC-19","FR-255-AC-9")]` on limits_and_cancellation_stop_the_run_naming_what_stopped_it, and deprecated `/// Trace: ... FR-255-AC-9` doc lines at model_check.rs:761 and model_check_safety.rs:621. Only qsl-replay/src/limits.rs:465 (TC-921) genuinely covers FR-255-AC-9. |
| FND-002 | fixed | settle.rs:385 now `#[trace("TC-627", "FR-192-AC-1")]`. |
| FND-001 | fixed | Round 2 (a608d9d22): FR-255-AC-9 dropped from model_check.rs:561 attribute and both deprecated /// Trace: lines deleted; quire matrix binds FR-255-AC-9 only to qsl-replay/src/limits.rs:467. |
