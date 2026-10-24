---
id: SR-16634
title: "Scoped gap analysis of QSL-1012 Quantity<ms> encoding"
type: SpecReview
analysis: gap-analysis
scope: "QSL-1012 PR #1431; selected FR-120-AC-16 and FR-189-AC-15; scoped inputs under /tmp/qsl-1012-scope"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-189
    type: reviews
---

## Summary

Planless, scoped audit of the two acceptance criteria changed by QSL-1012. `quire matrix --scope /tmp/qsl-1012-scope --format json` read only FR-120, FR-189, TC-1195, TC-624 and the touched test sources. The filtered matrix reports FR-120-AC-16 and FR-189-AC-15 as `tagged`. The new tests exercise the production encoder and full probabilistic model path, including rational equivalence, unit conversion and refusal before sampling. Reverse review found no new behavior without an owning requirement, no test stub, and no coverage inflation.

Plan completion: not assessed

## Verdict

PASS for the two selected criteria and the code surface listed in scope. This is not a claim about all criteria in FR-120 or FR-189.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No gap found in the scoped FR-120-AC-16 and FR-189-AC-15 implementation/test/code chain. | FR-120-AC-16; FR-189-AC-15 |

## Coverage

- FR-120-AC-16: tagged by `qsl-eval/src/simulation/model/encode.rs` test `a_quantity_encodes_its_rational_magnitude_and_unit`.
- FR-189-AC-15: tagged by the exact quantity integration test and the mismatched-support refusal test in `qsl-analyze/tests/it/statistical_model.rs`.
- Reverse gap: no introduced code behavior outside the two owning requirements was found in the selected files.
- Stubs and hollow evidence: none found in the selected tests or implementation.
- Plan completion: not assessed.
