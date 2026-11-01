---
id: SR-1988004
title: "Gap analysis — QSL-588 sample-step limit prerequisite"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; FR-255-AC-12, FR-255-AC-13, FR-254-AC-3; qsl-replay/src/model_check/limits.rs, qsl-replay/src/statistical.rs, TC-1161"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: references
---

## Summary

The changed limit-name row is backed by the exhaustive `every_limit_spells_its_name_in_snake_case` test and its direct FR-255-AC-12 trace. The full computed repository matrix did not finish within 120 seconds; the focused review found the new public value accessor cannot report a configured `max_sample_steps` value.

## Verdict

**CONDITIONAL** — focused trace coverage is present, but the new typed limit value path is incorrect and the full matrix audit was unavailable.

## Coverage

- FR-255-AC-12: examined. `TC-1161` is traced to this AC; `every_limit()` now enumerates `MaxSampleSteps`, and `row()` checks its expected limit and setting strings.
- FR-255-AC-13 and FR-254-AC-3: context only for this prerequisite PR. The PR adds limit naming but does not implement the timed sampler's end-to-end stop; that behavior belongs to the continuing QSL-588 implementation.
- Plan completion: not assessed.
- Full matrix: `quire matrix --scope <repo> --format json` and `quoin matrix --repo <repo> --json` each exceeded 120 seconds without producing a result. Manually checked the changed FR-255-AC-12 trace and the relevant FR-254/FR-255 requirements.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `StatisticalLimits::value(MaxSampleSteps)` returns the default constant instead of the caller-set `max_sample_steps`, so the named limit cannot reliably preserve the FR-254/FR-255 stop value. | qsl-replay/src/statistical.rs:216 |

## Verification

- Targeted static coverage: `every_limit_spells_its_name_in_snake_case` carries `TC-1161` and `FR-255-AC-12`, and `every_limit()` includes `MaxSampleSteps`.
- `quire matrix` and `quoin matrix` did not finish within 120 seconds; full repository coverage therefore remains unverified in this review pass.
