---
id: SR-2026111921282
title: "Hyper product closure evidence gaps"
type: SpecReview
analysis: gap-analysis
scope: "FR-163 and the QSL-2128 closure-order change; fsl-replay/src/certificate/product.rs; fsl-analyze/tests/it/closure_certificate.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-163
    type: references
---

## Summary

The computed repository matrix was inspected for FR-163, with source and test
inspection limited to the requested closure slice. The new behavior is owned by
FR-163's rule-order requirement. No plan was supplied.

## Verdict

**FAIL for complete FR-163 coverage.** The scoped correction does not establish
all FR-163 acceptance vectors or complete the partial PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-6 and AC-9 have no criterion binders in the computed matrix. Tagged AC-1, AC-2 and AC-3 also contain vectors beyond the added tests; tags alone do not establish complete coverage. The leaky fixture reaches 10 unreduced states rather than the requirement's historical 14-state vector. | FR-163-AC-1; FR-163-AC-2; FR-163-AC-3; FR-163-AC-6; FR-163-AC-9 |
| FND-002 | low | The matrix reports AC-8 untagged, although the requirement explicitly retires it. It is not a live implementation obligation. | FR-163-AC-8 |
| FND-003 | medium | The HP-5 undefined-letter fixture named by AC-3 cannot be authored through current admission: `eventually always holds(1 mod (1 - v.l @ b) = 0 and v.l @ b = 0)` is refused by `check_hyper_clauses` with `Hyper(BodyUnproved { obligation: Nonzero })`. The attempted new certificate test was removed after that fixture-construction failure; admission was not weakened and no alternative semantics were invented. | FR-163-AC-3; FR-181-AC-5; fsl-analyze/tests/it/hyper_world.rs |
| FND-004 | medium | The existing package-refusal regression fails during handoff fixture compilation, before reaching package hyper dispatch: `ClauseRequest { member: "handoff" }`, admitted package differs from checked unit on `package.nodes`. It therefore provides no fresh passing evidence for the package refusal. | fsl-analyze/tests/it/hyper_wire.rs::a_package_hyper_item_refuses_without_falling_back_to_source; fsl-analyze/tests/it/analyze_cli_support.rs::package_selection |

## Coverage

- Reconciliation: `quire matrix --scope . --format json`, quire 0.52.0,
  engine 0.62.2, exit 0; no run evidence read. The inventory emitted symbol
  collision and marker warnings elsewhere in the repository. The FR-163 tags
  in the changed tests were read and appear as binders.
- FR-163 criteria: tagged 6; untagged 3, including the retired AC-8.
- Plan completion: not assessed
- Untraced behaviors / stubs introduced by this correction: none found.
- Semantic review: skipped.
- Targeted results through `qsl-slot`, jobs 5, this worktree's target:
  `closure_certificate::hyper_product` 6/6 passed; `hyper_wire::` 7/8 passed,
  with the existing handoff fixture failure above. The initial closure run
  passed 6/7 before the inadmissible new HP-5 fixture was removed; the final
  closure filter passed in full. No aggregate gate was run.
