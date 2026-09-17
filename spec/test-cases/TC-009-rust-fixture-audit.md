---
id: TC-009
title: "CLI encoding and Rust-only execution"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-012
    type: verifies
---

## Description

CLI encoding and Rust-only execution. Scope: FR-012-AC-10, NFR-005-M-2. Type: E2E; priority P1.

## Test Procedure

Invoke the real binary with unknown/invalid OS arguments, and run self-test with external runtimes unavailable.

## Expected Results

Usage exits 2 without panic; Rust audits execute with no Python/Node.
