---
id: SR-2265002
title: QSL-577 PR #2265 scoped gap analysis
type: SpecReview
analysis: gap-analysis
date: '2026-11-04'
scope: 'agent-ix/quire-spec-language; PR #2265 at 2421444fe429fce556931ab674d64426f666dbff; FR-204 subject-scoped delay admission regression'
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-204
    type: references
---

## Summary

Manual scoped check of the new selected-package delay regression against FR-204 and its test trace. The test builds a real `M` subject while selecting `Other`, whose operation declares `exponential(1)`, and requires an MDP product. This exercises the subject-scoped behavior that would be rejected by an unfiltered delay scan. The final head tags the regression with `FR-204-AC-3` and adds the subject-own-delay refusal case. This fixes the trace mismatch found on the initial PR head.

`quire matrix --scope . --format json` timed out after 90 seconds. The initial PR head had the trace mismatch; the final head fixes it. No production source changed, so no reverse code-to-spec gap was introduced. Plan completion: not assessed.

## Verdict

CONDITIONAL: the trace finding is fixed on the final head; the computed matrix was unavailable, so full repository trace coverage is not claimed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new regression uses `#[trace("FR-204", "TC-639")]` instead of an FR-204 acceptance-criterion id, so it does not bind to the computed Test Matrix. | FR-204-AC-3 |

## Coverage

Plan completion: not assessed.

Manual scoped mapping covered the added regression in `qsl-analyze/tests/it/digital_clocks.rs` against FR-204-AC-3, which specifies unsupported delay distributions for the checked subject, and the existing tagged FR-204-AC-3 cases in the same test module. The new fixture helper in `qsl-analyze/tests/it/timed_world.rs` only supplies additional admitted package input to the unit. The documented Quire matrix command timed out; no matrix result is claimed. Optional semantic review was not requested. No applicable AssuranceProfile was found.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Final-head trace tag now names the criterion. |

```yaml
dispositions:
  - fnd: FND-001
    outcome: fixed
    after_excerpt: |-
      #[trace("FR-204-AC-3", "TC-639")]
```

`delay_admission_is_scoped_to_the_selected_subject_model` also asserts that an exponential delay declared on M itself returns `DelayDistribution`, while the same distribution on selected model Other does not refuse M's product.
