---
id: TC-446
title: "An identical repeat registration is idempotent"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: verifies
  - target: ix://agent-ix/quire-specification/FR-290
    type: verifies
---
# TC-446: An identical repeat registration is idempotent

## Description

Verify that repeating a registration that is identical to one the registry
already holds -- same backend identity, manifest digest, tool and advertised
pairs -- is one registration and is never refused. Scope: FR-075-AC-7.

Mirrors quire-specification's TC-282 DB-01 and DB-02 vectors (FR-290-AC-9),
which state the same rule for QSpec's own registry model; this test case
verifies it against `qsl-route`'s real `Registry`.

## Test Procedure

1. Register backend `A` advertising `value-validity`.
2. Register an identical descriptor for `A` again: same identity, manifest
   digest, tool and advertised pairs.
3. Register a third, unrelated backend `B`.
4. Compute the candidate set for `value-validity` with no named backend.
5. Read the registry's registration refusals.

## Expected Results

Step 2 succeeds (`Ok`), not a refusal. Step 4's candidate set holds `A` once
and, after step 3, `B` once, each exactly as registered. Step 5 reports no
refusal for `A`. The result is identical however many times the identical
descriptor is repeated, and independent of where in the sequence `B` is
registered.
