---
id: SR-120
title: "Evidence-method review of native Boolean lowering"
type: SpecReview
analysis: evidence
scope: "PR #13; FR-009, TC-092–094, IT-008, Plan-008; code/test baseline d58ca7a with POC delivery amendment"
review_set: all
---

## Summary

Attempted quoin advise --repo . --json. It exited 2 while determining the Quire
version, although direct quire --version reports CLI 0.31.0, engine
0.46.0@ca7362d4. No deterministic recommendation is claimed. quoin catalog
methods --json succeeded; its integration-testing and negative-abuse-testing
entries belong to Test, with Integration evidence.

Author judgement retains the existing Test cells: real binder/codegen integration
for AC-1/3/4/5 and adverse/boundary integration for AC-2/6/7. The existing Rust
harness supplies those methods. The independent reviewer demonstrated mutation
detection for operator semantics and each hard-limit axis. Property generation,
fuzzing and broader mutation campaigns remain future assurance work; this pure
synchronous lowering introduces no concurrency obligation requiring Loom.

## Verdict

CONDITIONAL — method choices are explicit author judgement; activation evidence is deferred.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Advisor unavailable; retain Test using real binder integration. | FR-009-AC-1 |
| FND-002 | low | Advisor unavailable; retain Test using unsupported-feature controls. | FR-009-AC-2 |
| FND-003 | low | Advisor unavailable; retain Test using missing-binding rejection. | FR-009-AC-3 |
| FND-004 | low | Advisor unavailable; retain Test using paired identity changes. | FR-009-AC-4 |
| FND-005 | medium | Retain Test using actual compiled output; generated activation remains unverified and deferred by the owner. | FR-009-AC-5 |
| FND-006 | low | Advisor unavailable; retain Test using exact/zero/one-below limits and independent hard constants. | FR-009-AC-6 |
| FND-007 | low | Advisor unavailable; retain Test using later-clause and owner-population refusals. | FR-009-AC-7 |
