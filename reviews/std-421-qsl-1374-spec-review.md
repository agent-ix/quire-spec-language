---
id: SR-16302
title: "Spec review of quire-spec-language PR 1374: STD-421"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@bfce55fb; spec/functional/FR-110-resolve-header-profile-selections-at-e3.md; spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md; spec/functional/FR-345-run-the-profile-layering-refinement-gate.md; spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md; spec/test-cases/TC-1017-the-model-profile-is-a-header-selection-only.md; spec/test-cases/TC-490-e3-resolves-header-profiles-against-the-definition-lock.md; spec/test-cases/TC-685-s3-checks-time-declarations-and-clock-constraints.md; spec/test-cases/TC-868-seeded-layering-regression-fails-the-gate.md"
review_set: subset
date: 2026-10-23
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-110
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-345
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-361
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-490
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-685
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-868
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1017
    type: references
---

# SR-16302: Spec review of quire-spec-language PR 1374 (STD-421)

## Summary

Ticket: STD-421. Head bfce55fb (PR 1374, contains 1360 head ffb65e4c; chain tip). Own delta: commit db72dc9c (FR-110, FR-230, FR-345, FR-361, TC-490, TC-685, TC-868, TC-1017). The selection rules (the layer requires `quire.model.complete/v1`, six `definition_selections` layers, complete-temporal lock rows, the unit selects the timed facet only through its header, a model-only unit gets FR-230-AC-7's refusals and keeps its model constructs) are consistent, the E6 edge and temporal witness follow QSpec FR-453, and `complete-temporal-lock.json` and `temporal-complete.md` exist in the QSpec stack (QSpec PR 417). The merged TC-685 numbering (steps 8 and 9 for the header cases, 10 to 21 clock cases) is coherent at the tip. Tip gate: exit 0, 20 warnings (19 on main plus FR-323 from PR 1357); `conformance_layer_rows_match_qspec_complete_value_lock` is expected red until QSL-1013 and was not counted.

Also checked:

- FR-110-AC-13, FR-361-AC-14, FR-361-AC-15 and FR-345-AC-11 are unique across the other open PR heads. Merge-tree with main clean; conflicts with open PRs 1335, 1333, 1325 (FR-345, TC-868), 1314, 1304, 706.
- The whole chain (1347 to 1374) was gated at each head: exit 0 at all eight, warnings 19 for 1347 to 1379 and 20 from 1357 on.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | FR-344-AC-6 and TC-866 step 4 still build a layering result set from 'the five layer results and the five edge results', while the gate now has six layers and six edges (FR-345-AC-4, AC-6, AC-7, AC-11). The ordering vector is still satisfiable but no longer covers the real set. Fix: say six layer results and six edge results. | spec/functional/FR-344-report-a-refinement-gate-verdict-and-exit.md:109 | missing-requirement |

## Verdict

**Mergeable.** Only low findings (FND-001).
