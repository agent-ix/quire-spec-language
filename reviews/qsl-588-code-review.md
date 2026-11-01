---
id: SR-1988005
title: "Code review — QSL-588 typed timed form refusal span"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-analyze/src/exact_probabilistic.rs, qsl-analyze/src/exact_probabilistic/build.rs, qsl-analyze/src/exact_probabilistic/plan.rs, qsl-analyze/src/statistical/model.rs, qsl-semantics/src/check/probabilistic/claim.rs; FR-204-AC-4, FR-254-AC-3"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/FR-204", type: references }
  - { target: "ix://agent-ix/quire-spec-language/FR-254", type: references }
---

## Summary

Reviewed the five-file Rust diff in PR #1996, including the Rust review lane and its code/test alignment. The shared elapsed variant, typed span forwarding, and compile-required statistical consumer arms stay within the authorized prerequisite scope; no code defects were found.

## Verdict

**PASS** — the changed Rust paths are narrow and preserve the source span through exact planning/build refusal mapping.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

- FR-204-AC-4 examined: unsupported timed form shape is represented by a typed `PlanFault`, mapped to `ProductRefusal`, and covered by a span-preservation unit test tagged `TC-639` / `FR-204-AC-4`.
- FR-254-AC-3 context only: the statistical consumer arms compile against the new variant; timed elapsed sampling behavior is outside this prerequisite PR.
- Changed files examined: all five Rust files listed in `scope`.
- Changed paths: exactly `qsl-analyze/src/exact_probabilistic.rs`, `qsl-analyze/src/exact_probabilistic/build.rs`, `qsl-analyze/src/exact_probabilistic/plan.rs`, `qsl-analyze/src/statistical/model.rs`, and `qsl-semantics/src/check/probabilistic/claim.rs`.
- Authorization check: no edits outside the five authorized plumbing/consumer paths; no unrelated exact-engine behavior or refactors, and no synthesized FR-331 locus.
- `git diff --check`: passed. Tests were not run.

## Bindings

- `TC-639` → `FR-204-AC-4`: correct; the added test directly asserts that `plan_refusal` preserves the typed source span.
