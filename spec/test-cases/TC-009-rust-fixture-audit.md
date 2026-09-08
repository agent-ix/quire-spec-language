---
id: TC-009
title: "CLI encoding and producer refusal"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-012
    type: verifies
---

## Description

CLI encoding and producer refusal. Scope: FR-012-AC-10, FR-012-AC-11, NFR-005-M-2, NFR-005-M-3. Type: E2E; priority P1.

## Test Procedure

Invoke the real binary with unknown/invalid OS arguments, self-test/model-bytes with external runtimes unavailable, and model-producer.

## Expected Results

Usage exits 2 without panic; Rust audits execute with no Python/Node; producer mode exits 3 with producer-language-unapproved.
