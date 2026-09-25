---
id: TC-196
title: "A conflicting backend identity registration refuses both and withdraws the held registration"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: verifies
  - target: ix://agent-ix/quire-specification/FR-290
    type: verifies
---
# TC-196: A conflicting backend identity registration refuses both and withdraws the held registration

## Description

Verify that registering a `BackendId` already held by the registry, with a
descriptor that differs from the held one, is a conflict (quire-specification
FR-290 "Candidate set and negotiation"): both the held registration and the
new one are refused with `invalid_capability`/`duplicate-backend`, naming the
identity, and the held registration is withdrawn -- **not** left standing.
Scope: FR-075-AC-4.

**This test case is inverted from its earlier form.** It previously verified
that "the original stands": a repeated `BackendId` refused only the second
registration and left the first one in effect. FR-075's older rule is
superseded by quire-specification FR-290, which QSL now implements: two
unequal descriptors under one identity conflict outright, and neither
survives. Only a registration identical to the one already held is not
refused (FR-075-AC-7, [TC-446](TC-446-identical-repeat-registration-is-idempotent.md)).

Catches an implementation that silently overwrites the first registration
with the second (losing the original advertisement set), an implementation
that merges the two registrations' advertised-kind sets instead of refusing
outright, and an implementation that reverts to the older "original stands"
behavior this test case used to assert.

[TC-155](TC-155-keep-admission-backend-independent.md) (FR-057-AC-8, on
`main`) step 3 also registers a backend that repeats an already-registered
identity, as one case inside a broader admission-and-routing fixture scan.
This test case isolates the conflicting-identity rule at the registry
(FR-075) itself and additionally asserts the specific post-conflict
candidate-set contents (steps 4-5 below), which TC-155's scan does not
check.

## Test Procedure

1. Register Backend A advertising `value-validity` only.
2. Register a second, unequal descriptor under the same `BackendId`
   "Backend A", this time advertising `operation-contract` only.
3. Inspect the result of the second registration attempt.
4. Compute the candidate set for an item requiring `operation-contract`
   with no named backend.
5. Compute the candidate set for an item requiring `value-validity` with no
   named backend.
6. Compute the candidate set for an item requiring `value-validity`, naming
   "Backend A".
7. Repeat steps 1-6 with the two registrations in the opposite order.

## Expected Results

Step 3 is a refusal carrying both registrations' refusals, each
`invalid_capability`/`duplicate-backend`, naming "Backend A". Steps 4 and 5
each return an empty candidate set: neither the second registration's
`operation-contract` advertisement nor the first's `value-validity`
advertisement took effect -- the held registration is withdrawn, not left
standing. Step 6 returns the unknown-backend mark carrying "Backend A": the
identity is unregistered. Step 7 gives the identical result: which
descriptor arrived first does not change the outcome.
