---
id: TC-982
title: "Backend descriptor field differences have isolated registration outcomes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-288
    type: verifies
---
# TC-982: Backend descriptor field differences have isolated registration outcomes

## Description

Verify every `BackendDescriptor` member named by FR-075 and FR-288 in an
isolated second-registration control. Use one valid baseline descriptor and
change exactly the listed member in the second descriptor. Scope:
FR-075-AC-4/7 and FR-288-AC-5/6.

## Test Procedure

Register baseline descriptor A, then for each matrix row in a fresh registry
register the row's second descriptor. All values not named in that row are
identical to A. Use values admitted by the descriptor constructor. For an
origin or digest row, check every order of the two registrations.

| Field varied only | Second descriptor compared with baseline A | Expected registration result |
|---|---|---|
| Backend identity (`BackendId`) | Different identity; every other member equal | Both identities are held independently; no duplicate-backend conflict, because identity selects the registry key. |
| Provider origin | Same identity and manifest digest; `Linked` versus `Process`; other members equal | The identity conflicts, both registrations are withdrawn, and the conflict is independent of registration order. |
| Manifest digest | Same identity and origin; a different valid manifest digest; other members equal | The identity conflicts; refusal records follow FR-290's distinct-manifest-digest rule, independent of registration order. |
| Advertised pairs | Same identity and all other members; add exactly one valid `(Capability, mode)` pair | The identity conflicts and is withdrawn. |
| Retained `domains` | Same identity and all other members; change between two distinct admitted domain sets (or absent/present where both are valid for the advertised modes) | The identity conflicts and is withdrawn. |
| Evidence-kind set | Same identity and advertised probabilistic pair; `{statistical}` versus `{exact}`; all other members equal | The identity conflicts and is withdrawn, as TC-981 requires. |

## Expected Results

Every row varies one field only. Each descriptor-only difference produces the
specified conflict and withdrawal; a changed backend identity does not, because
it is the registry key rather than a value compared under an existing key.
Repeating A unchanged across every descriptor member remains idempotent under
TC-448. No case changes registration outcome because of candidate ordering.
