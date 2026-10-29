---
id: SR-60052
title: "Failure-domain review for QSL-835 mapped-surface retirement"
type: SpecReview
analysis: failure-domain
scope: "Root API and feature deletion with the FR-030/FR-031 extraction path retained"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-490
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-484
    type: reviews
---

## Summary

The main failure risk is losing requirement-tagged extraction coverage when
the root feature and direct dependency disappear. FR-490-AC-3 requires passing
tests owned by `qsl-replay` or `qsl-source` tagged for FR-030-AC-1,
FR-030-AC-2 and FR-031-AC-2, as well as retaining the existing `qsl-source`
tests for FR-030-AC-3/4/5. The code change must move the root-owned cases
before deleting them. No other identity, purity, or topology failure applies
to this deletion.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The implementation must move the root-owned cases in `tests/it/config_version_spine.rs` and `tests/it/spine_run.rs` into `qsl-replay` or `qsl-source` tests tagged for FR-030-AC-1, FR-030-AC-2 and FR-031-AC-2 before deleting them. Before removing the root feature, run `cargo test -p quire-spec-language --features quire-extraction,qsl-replay/quire-extraction --test config_version_spine --test spine_run`; after the move, run `cargo test -p qsl-replay --features quire-extraction` and `cargo test -p qsl-source --features quire-extraction`. Merely dropping the root feature would orphan those acceptance criteria. | FR-490-AC-3, TC-484 |
