---
id: TC-197
title: "Empty candidate set carries the data an unsupported warning needs"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-076
    type: verifies
---
# TC-197: Empty candidate set carries the data an unsupported warning needs

## Description

Verify that when no registered backend advertises an item's matching
capability requirement, the registry returns an ordinary, well-formed empty
candidate set (no error, no panic) retaining its capability kind, its exact
evidence kind when probabilistic, and, when named, the requested backend's
identity. Scope: FR-076-AC-1, FR-076-AC-2.

Catches an implementation that raises a Rust panic or a `Result::Err` for
this case instead of returning a normal value, and an implementation that
returns a bare empty collection with no retained capability-kind,
evidence-kind or backend-identity data, forcing a caller to re-derive what
was missing from elsewhere (or silently warn with the wrong name).

## Test Procedure

1. Build a registry with one backend, Backend A, advertising only
   `value-validity`.
2. Compute the candidate set for an item requiring `temporal-satisfaction`
   with no named backend.
3. Compute the candidate set for an item requiring `temporal-satisfaction`,
   naming the registered `BackendId` "Backend A" (which does not advertise
   that kind).
4. Register backend P advertising `probabilistic-satisfaction` with
   `{statistical}` and compute candidates for an unnamed probabilistic item
   naming `exact`.
5. Compute candidates for probabilistic items with absent evidence and with
   an unknown evidence label, once with a backend advertising `{statistical}`
   and `{exact}` registered and once with no backend; pass each preserved item
   to negotiation. Neither malformed item matches either valid advertised
   evidence kind.
6. Inspect all results' runtime type/variant and any data they carry.

## Expected Results

Both steps 2 and 3 complete without panicking and without returning a
`Result::Err` or refusal-shaped value; both return an empty candidate set.
The registry's own returned output — not data the caller already held
before the call — exposes the requested capability kind
(`temporal-satisfaction`) in steps 2 and 3, and exposes "Backend A" in step 3
only. Step 4 returns an empty candidate set retaining both
`probabilistic-satisfaction` and `exact`; a test that inspects only the input
the caller passed in, rather than the value the registry returned, does not
satisfy this check.

Step 5 returns no candidates in all cases, preserves absent evidence as absent
and the unknown label verbatim in the candidate-stage output. Negotiation
settles each as `invalid-request`
with `absent-evidence-kind` or `unknown-evidence-kind`, respectively, not
`unsupported`, regardless of candidate availability.
