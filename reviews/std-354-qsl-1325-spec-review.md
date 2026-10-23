---
id: SR-16314
title: "spec-review of FR-345 witness negotiation is a caller-supplied function (quire-spec-language PR 1325)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@808d153754; spec/functional/FR-345-run-the-profile-layering-refinement-gate.md; spec/test-cases/TC-868-seeded-layering-regression-fails-the-gate.md; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-345
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-868
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-280
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-344
    type: references
---

# Spec review of FR-345 witness negotiation is a caller-supplied function (quire-spec-language PR 1325)

## Summary

Ticket: STD-354. One spec-review pass over `git diff origin/main...808d153754` (agent-ix/quire-spec-language#1325), checked for the no-drop/no-narrowing/no-deferral rule, testable vectors (derived by hand), id uniqueness across open PR heads, and mergeability. Gate: `QSPEC_DIR=<worktree of QSpec origin/main> make check-spec-validation` exit 0.

Examined, clean:

- Hand-derived TC-868 step 5/6 vectors: the sixth run (value-validity-only registry, no negotiation function) gives a `regression` naming the empty candidate set for the four layers whose `witness_request` kind is `operation-contract` (state core, state queries, state graph, complete model, per QSpec FR-453 Witness), and complete value (`value-validity`) holds, as the PR states.
- The no-CG-dependency design (`compare` public in the `xtask` library, `negotiation` an optional parameter, `REAL_CORPUS`, root dev-dependency on `xtask` already present on main) is internally consistent; `xtask` has no CG dependency on main.
- FR-345-AC-10's call over `REAL_CORPUS` with a test negotiation function settling every request `supported` gives verdict success; the no-function call returns the report `make refinement-layering` writes.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | scope-deferral: the PR moves the real-backend half of the gate out of QSL. After it, QSL's `make ci` no longer negotiates any layer's witness item (AC-9 weakens to "admitted with a non-empty candidate set"), and the check that each layer's witness settles `supported` under a real candidate is assigned in a new Overlap section to quire-integration. That section only describes the other repo's test; this PR specifies no requirement, AC or ticket in quire-integration that makes the run happen, so FR-345-AC-6 over real backends and the witness half of AC-9 have no owner and no verification. Standing rule: a check moved to another lane without its own spec is a narrowing, even when the reason (QSL may not depend on CG, FR-280-AC-3) is sound. Fix: land the quire-integration requirement and AC (or a new STD ticket and spec PR for it) with this PR, and cite it from FR-345. | spec/functional/FR-345-run-the-profile-layering-refinement-gate.md | missing-requirement |

## Verdict

Not mergeable: FND-001 (high) block the PR.

Conflicts with open head #1374 (STD-421): FR-345 and TC-868. Clean against origin/main.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1, head b40c200e. Over the real backend registry, with quire-contract-codegen's `negotiate_*` as the negotiation function, `compare` over `REAL_CORPUS` gives each of the five layers a `holds` witness result: its witness unit admits and its witness item, requested once per candidate by backend name, settles `supported |
