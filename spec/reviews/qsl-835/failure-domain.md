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

The main failure risk is losing feature-gated extraction coverage when the root
feature and direct dependency disappear. FR-490-AC-3 explicitly carries the
existing config-version and spine-run extraction tests into the replay-owned
feature path; no other identity, purity, or topology failure applies to this
deletion.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The implementation must move or otherwise run the extraction test modules currently gated in `tests/it/config_version_spine.rs` and `tests/it/spine_run.rs`; simply removing the root feature would silently stop those tests. FR-490-AC-3 makes that preservation observable. | FR-490-AC-3, TC-484 |
