---
id: SR-1465001
title: "Code and Rust review of PR #2101: QSL-1465 analyze boxing"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-analyze/src/model_check/refine.rs; qsl-analyze/src/zone_check/refine.rs; qsl-analyze/tests/it; tests/it/emit_checked_v2.rs"
review_set: subset
date: '2026-11-02'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-375
    type: reviews
---

## Summary

Ticket: QSL-1465. Reviewed the diff origin/main...origin/peter/qsl-1465-analyze-boxing. Derefs match StepVerdict (Box post, Box taken, Box failure) at qsl-replay/src/refinement/failure.rs:352. Cargo not run.

## Verdict

**PASS (mergeable)** - no findings. Derefs are by-value unboxing at the consumer; Judgement::Fails re-boxes a tuple, a different type, so it is required. The emit_checked_v2 encode builds operation_effects with the same mapping as qsl-package/src/emit.rs:3982-3991. No allow attribute, no Cargo change. Merge-order note: PR #2093 changes the same CheckedPackageSidecarsRef literal in tests/it/emit_checked_v2.rs to operation_effects: None, so whichever merges second conflicts; resolve to this PR's version (preserves effects on re-encode).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
