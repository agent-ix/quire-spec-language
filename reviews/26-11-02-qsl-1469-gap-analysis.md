---
id: SR-202611021518
title: "Gap analysis — QSL-1469 alias-less operation-effect declaring nodes"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; qsl-semantics/src/check/lowering/model.rs; qsl-package/src/emit/tests/variant_types.rs; qsl-package/src/emit/tests/owners.rs; FR-496; FR-094; FR-360-AC-8; FR-093-AC-21"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-496
    type: reviews
---

## Summary

Ticket: QSL-1469. PR agent-ix/quire-spec-language#2119. This is a planless gap analysis scoped to the PR diff. Plan completion: not assessed.

The installed quoin is 0.29.1. I did not run `quoin matrix`. Instead I mapped the changed behavior to its tests by hand.

## Verdict

**PASS with one low gap.** The alias-less fix is exercised by a test that fails when the fix is reverted. That test is traced to FR-093-AC-21/TC-416, though, not to FR-496 or FR-322-AC-303. The spec-conformance problem in the restated variant_types test is recorded as FND-001 in SR-202611021517 and is not repeated here.

Examined:

- FR-496 behavior: every selected document gets rows, and `declaring` names a package node.
- FR-496-AC-5: carrier presence does not change the nodes.
- FR-360-AC-8.
- FR-094's reachability rule.
- FR-093-AC-21.

I found no stubs, no unowned production code and no skipped tests in the diff.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | low      | No test traced to FR-496 covers an alias-less (dispatch-bridge) model's rows naming retained declaring nodes. The regression is caught only incidentally, by `owners::model_declaration_nodes_and_clause_functions_carry_their_model_owner` (traced FR-093-AC-21/TC-416). A FR-496/TC-1286-traced assertion that the `Sub/size` row's `declaring` resolves would make the coverage visible in the matrix. | qsl-package/src/emit/tests/owners.rs:293-295; qsl-semantics/src/check/lowering/model.rs:2047-2066 |
