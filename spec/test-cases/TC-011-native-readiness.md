---
id: TC-011
title: "Exact native source intake"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-001
    type: verifies
---

## Description

Exact native source intake. Type: Integration; priority P1. Traces: FR-001-AC-1, FR-001-AC-2, FR-001-AC-3, FR-001-AC-4.

## Test Procedure

Run the existing source/digest and parser-source tests over original Unicode/CRLF bytes, selected and changed digests, invalid UTF-8 and a lowered source ceiling.

## Expected Results

Accepted sources retain exact bytes and coordinates; changed digest, invalid encoding and excess bytes return their distinct diagnostics.

