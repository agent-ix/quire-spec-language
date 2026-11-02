---
id: SR-202611021401
title: "Code and Rust review of PR #2113: QSL-1465 root test literal"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; tests/it/emit_checked_v2.rs"
review_set: subset
date: '2026-11-02'
---

## Summary

Ticket: QSL-1465. Reviewed origin/main...origin/peter/qsl-1465-root-test-literal (head a701aa88d). The test helper encode now builds schedulability_claims from read.schedulability_claims() with rows {declaration, occurrence.ordinal} and passes them only when non-empty. This is the same mapping as qsl-package/src/emit.rs (CheckedSchedulabilityClaimRow { declaration: id, occurrence: ordinal }, passed as (!rows.is_empty()).then_some(..)). operation_effects is unchanged. Cargo not run.

## Verdict

**PASS (mergeable)** - the new literal field compiles main again and re-encodes claims losslessly (Some when claims exist; None only when there are none, as the emitter does). No allow attribute.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Re-encode still passes checked_term_data: None and temporal_bindings: None, and no fixture in this test carries schedulability claims, so the new mapping is not exercised | tests/it/emit_checked_v2.rs:450-455 |
