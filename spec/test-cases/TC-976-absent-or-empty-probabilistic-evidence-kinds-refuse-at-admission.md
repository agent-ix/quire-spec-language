---
id: TC-976
title: "Absent or empty probabilistic evidence kinds refuse at admission"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: verifies
---
# TC-976: Absent or empty probabilistic evidence kinds refuse at admission

## Description

Verify `BackendDescriptor::admit` rejects a probabilistic registration with no
evidence-kind label before the registry can observe it. Scope: FR-075-AC-10.

## Test Procedure

1. Register a valid descriptor for backend `p` advertising
   `probabilistic-satisfaction` with `{statistical}`.
2. Submit a descriptor for the same backend with no evidence-kind set, then
   one with an empty evidence-kind set.
3. For each refusal, inspect the registry and query candidates for the kind.

## Expected Results

Both admissions refuse with `invalid_capability`/`unknown-evidence-kind`, keyed
by `BackendId`. Neither refused descriptor replaces or withdraws the valid
descriptor already held for `p`.
