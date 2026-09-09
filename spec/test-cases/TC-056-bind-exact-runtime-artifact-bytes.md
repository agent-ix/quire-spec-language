---
id: TC-056
title: "Bind exact runtime artifact bytes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-018
    type: verifies
---
# TC-056: Bind exact runtime artifact bytes

## Description

Property, priority P1. Verifies FR-018-AC-3, FR-018-AC-4, FR-018-AC-7. Planned until actual Rust
execution. Setup uses qualified public model/checker APIs; unrelated setup
failure cannot stand in for the intended phase's outcome.

## Test Procedure

Generate a bounded family of ASCII/Unicode labels, scalar extrema, empty/nonempty vectors, field-order permutations and snapshot/invocation artifacts. Compare bytes to independently authored envelope expectations and SHA-256 to an independent known-vector check. Repeat identical requests and mutate each identity/body field independently; include empty labels and role-specific reference API compile checks.

## Expected Results

Repeated exact input reproduces bytes/digests. Altered labels and fields yield the expected changed bytes. Vector order remains byte-significant even when later semantic equality ignores it. Snapshot/Invocation refs cannot substitute for each other's Rust type; invalid labels produce a structured error with no digest invented.
