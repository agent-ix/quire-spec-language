---
id: SR-819
title: "QSL-329 gap analysis of PR 537 (FR-105-AC-3, FR-108-AC-6, TC-463, TC-469 vs tests)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@5a08d72f78773410fac8c7b62978d814edd86f5a; spec/functional/FR-105-emit-state-nodes.md (AC-3); spec/functional/FR-108-run-the-configversion-spine-corpus.md (AC-6); spec/test-cases/TC-463-s4-state-package-reads-back-and-is-stable.md; spec/test-cases/TC-469-configversion-spine-corpus-matches-native.md; qsl-replay/src/spine/clause/tests.rs; tests/it/config_version_spine.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-108
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-463
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-469
    type: reviews
---
## Summary

Ticket: QSL-329. PR: quire-spec-language#537 at 5a08d72f.

Ticket items, measured at head:

- FR-105-AC-3 (I2 read of the emitted package, frame step, recomputed
  package_id equals emitted). Backed by `s4_state_package_reads_back_through_i2`,
  tagged `#[trace("TC-463", "FR-105-AC-3")]`, no longer ignored. Passes at
  head (exit 0); fails at the old lock with OperatorIneligible at node 25.
- FR-108-AC-6 (package_id agreement across cases; I04 read admits the
  bytes). The package_id half is `tc_469_step_6_package_id_is_pinned_across_every_case`;
  the I04 half is `tc_469_step_6_the_emitted_package_admits_via_i04`, both
  tagged `#[trace("TC-469", "FR-108-AC-6")]`. The I04 test is no longer
  ignored and passes at head; it fails at the old lock at node 8.
- No `#[ignore]` remains in either test file.

Test-to-AC bindings checked: all correct. Matrix and test-case status text
that did not follow the code is recorded in the spec review (SR-820), not
here.

## Verdict

Clean. Both ACs are backed by tests that run, pass and can fail.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
