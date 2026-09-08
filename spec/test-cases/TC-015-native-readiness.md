---
id: TC-015
title: "Native CLI outcomes and digest"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-010
    type: verifies
---

## Description

Native CLI outcomes and digest. Type: E2E; priority P1. Traces: FR-010-AC-1, FR-010-AC-2, FR-010-AC-3, FR-010-AC-4, FR-010-AC-5, FR-010-AC-7.

## Test Procedure

Invoke the real compiler binary for parse/format, empty identity, invalid/missing/extra command arguments, a missing source file and an oversized temporary source. Compare parse digest with the actual input bytes.

## Expected Results

Exit codes are 0/1/2/3 for success/refusal/usage-or-I/O/incomplete. Only successful parsing emits parsed, and no path claims logical evaluation.

