---
id: TC-198
title: "Backend absence never settles as a refusal or a hold at the registry"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-076
    type: verifies
---
# TC-198: Backend absence never settles as a refusal or a hold at the registry

## Description

Verify that the registry's output shapes for "no registrant advertises this
kind" (an empty candidate set) and "this backend identity was already
registered" (a registration refusal) are structurally distinct types, and
that neither the registry's public API nor its return types offer any
"deferred"/"pending"/"hold" variant that a caller could use to wait for a
future registration. Scope: FR-076-AC-3.

Catches an implementation that reuses one shared `Result<CandidateSet,
RegistryError>` type for both candidate computation and registration, so
that an empty candidate set and a duplicate-backend registration refusal
are both `Err` variants of the same enum and a caller (or a future
maintainer) can no longer tell them apart without inspecting the payload;
and an implementation that adds a queued/pending state for candidate
computation, which this requirement forbids outright.

## Test Procedure

1. Inspect the registry's public type signatures for candidate computation
   (FR-075) and for registration (FR-075-AC-4).
2. Trigger the empty-candidate-set case (an item whose kind no registrant
   advertises) and record the concrete return type/variant.
3. Trigger the duplicate-backend registration refusal case and record its
   concrete return type/variant.
4. Search the registry's public API and return types for any variant,
   field, or documented behavior representing a deferred, pending, or
   held outcome for a candidate-set computation.

## Expected Results

Step 2's return type/variant is structurally distinct from step 3's (for
example, a plain collection versus a dedicated registration-outcome enum),
so a caller cannot mistake one for the other by pattern-matching alone.
Step 4 finds no deferred/pending/hold variant anywhere in the registry's
candidate-computation API.
