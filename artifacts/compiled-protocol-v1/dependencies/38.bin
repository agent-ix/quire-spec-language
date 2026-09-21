---
id: FR-046
title: "Validate state invocation inputs before clause evaluation"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-005
    type: implements
---
## Description

When accepting a state assessment request, the validator SHALL establish the exact anchor, observations, context and applicable invocation/frame correspondence required by its selected clause before evaluating it.

## Inputs

Operation/invocation/context/universe identities, immutable parameter/result values, exact pre/post snapshots, completeness statements and selected authored frame.

## Outputs

A validated anchored request or a typed binding/frame refusal or unavailable-input result.

## Behavior

Preconditions use pre, postconditions post, and result is post-only. Parameters come from one declared invocation. Complete pre/post populations determine actual create/delete/change inventories; caller-supplied lists must match and stay within the selected frame. Missing observation is not empty population or a frame violation. A complete post snapshot lacking required self is a context-binding refusal even when its deletion was permitted. Callers cannot select a more permissive frame or substitute another operation's observation.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-046-AC-1 | Correctly bound pre/post inputs and parameters from one invocation supply the expected distinct self reads and immutable parameter values. | Test (TC-046) |
| FR-046-AC-2 | Foreign operation/invocation/universe, wrong parameter/result type or a substituted frame refuses before evaluation. | Test (TC-046) |
| FR-046-AC-3 | For complete observations, incorrect creation/deletion inventories and changes beyond the selected frame refuse with distinct typed causes. | Test (TC-046) |
| FR-046-AC-4 | Missing pre/post/completeness input remains unavailable/incomplete, without fabricating a delta, violation or Boolean. | Test (TC-046) |
| FR-046-AC-5 | Allowed deletion of required post self still refuses the post-context binding; an independently qualified pre reference remains a pre reference. | Test (TC-046) |
| FR-046-AC-6 | An invariant binds its named initialization/handler observation; a bare context type or unrelated current snapshot refuses as a substitute. | Test (TC-046) |

## Dependencies

- [Detailed draft contract](../../proposals/quire-v1/state-contract.md).
- [Shared grammar](../../proposals/quire-v1/shared-grammar.md).
- [Planned acceptance matrix](../composed-foundation/tests.md).

Proposed composed-v1 obligation. Historical definitions and their acceptance
remain separate; this artifact does not establish implementation coverage.
