---
id: SR-1988006
title: "Gap analysis — QSL-588 elapsed form prerequisite"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; FR-204-AC-4, FR-254-AC-3, QSpec-FR-421-AC-4; qsl-analyze/src/exact_probabilistic.rs, qsl-analyze/src/exact_probabilistic/build.rs, qsl-analyze/src/exact_probabilistic/plan.rs, qsl-analyze/src/statistical/model.rs, qsl-semantics/src/check/probabilistic/claim.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/FR-204", type: references }
  - { target: "ix://agent-ix/quire-spec-language/FR-254", type: references }
---

## Summary

Focused gap analysis of the elapsed measure path against QSL FR-254-AC-3 and FR-204-AC-4, with QSpec FR-421-AC-4 used as cross-repository context. Exact timed-form refusal span plumbing is covered; timed statistical elapsed sampling remains incomplete and belongs to the continuing QSL-588 implementation.

## Verdict

**CONDITIONAL** — this prerequisite closes the exact-refusal span path, while FR-254-AC-3's elapsed sample measurement remains unimplemented outside this PR's authorized scope.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Statistical `elapsed` measures still have no reader: `measure_of` returns `None`, and statistical reduction turns that into an internal fault instead of sampling the time-stamp difference required by FR-254-AC-3. | FR-254-AC-3, qsl-analyze/src/statistical/model.rs:478, qsl-analyze/src/statistical/model.rs:520,539 |

## Coverage

- FR-204-AC-4 examined: `Plan::of` rejects elapsed measure forms as `PlanFault::TimedFormShape`; `build.rs` maps the fault and preserves the span in `ProductRefusal`. The added `TC-639` test checks the mapping function's span preservation.
- QSpec FR-421-AC-4 context only: the counterpart requires an unadmitted timed form to settle `unsupported` with `timed-form-shape`; it is in `agent-ix/quire-specification`, not this repository.
- FR-254-AC-3 examined: its elapsed behavior requires sample timestamp difference; `statistical/model.rs::measure_of` intentionally has no elapsed reader, so reduction follows the internal-failure path. This feature gap is acknowledged as continuing QSL-588 work, not a request to expand this prerequisite PR.
- Reconciliation: focused manual check of the stated criteria and changed call paths. `quoin matrix --repo /private/tmp/qsl-pr1996-review --json` produced no result during the review wait and was interrupted; the repository-wide matrix was unavailable. Focused source trace found the new `TC-639` tag for `FR-204-AC-4`.
- Plan completion: not assessed
- Criteria counts: not computed; the requested scope is a focused subset, and the repository matrix did not finish.
- Untraced behaviors / stubs: one in-scope outstanding behavior (`FR-254-AC-3` elapsed sample reader).
- Semantic review: skipped; not requested.
