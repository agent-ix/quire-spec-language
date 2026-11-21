---
id: SR-202611210022442313144108431
title: "Gap analysis — QSL-2128 PR #3093 hyper checks through analyze (dc59305f00)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #3093 at dc59305f00: QSL-2128 acceptance criteria and lead rulings against fsl-analyze/tests/it/{hyper_wire,hyper,closure_certificate,analyze_family_cli,hyper_closed}.rs, fsl-semantics/tests/it/hyper_clauses.rs; FR-631, FR-163, FR-173-AC-6, FR-940-AC-18"
review_set: subset
evaluated_revision: "dc59305f00"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-940
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-631
    type: references
---

## Summary

Ticket: QSL-2128. Plan completion: not assessed. This is a manual check of each AC against its tests; no full-repo `quoin matrix` run was made. Each tagged test was read and judged on whether it can fail. The PR's tests were run name-filtered at the PR head. Two probes ran in a throwaway worktree:
- the one ignored test, run with `--ignored`;
- an event-position HP-5 analyze request.

Squash check: all 13 test files have the same content as the pre-rebase head f1692df174, apart from import order and the new `settle_hyper_check` arguments. No `#[ignore]` was added by the rebase, and no assertion was bent. The one `#[ignore]` (hyper_wire.rs:260) was already in the original PR.

## Verdict

**NOT MERGEABLE.**
- AC-1, AC-4 and the AC-7 tags are covered.
- The lead ruling on this ticket (FR-940 rulings 1-6, merged as spec #3113) is contradicted by a new test (FND-001).
- AC-5 is unmet and its ignore reason is stale (FND-002).
- AC-6's finite-trace-set half has no test (FND-003).

## Coverage

| Criterion | Test | Result |
| --- | --- | --- |
| AC-1 request item (`claim` only, per the readiness ruling) | hyper_wire.rs, all items | covered |
| AC-2/AC-3 driver end-to-end over the rover | filament-driver `fsl-cli/tests/frontend_cli.rs` | not in this repo; not assessed |
| AC-4 forall-exists, projection-aligned and single-existential, each holding and refuted | `analyze_hyper_forall_exists_holds_and_refutes`, `analyze_hyper_projected_holds_and_refutes`, `analyze_hyper_single_existential_holds_and_refutes` | covered (single-model, infinite-trace only; see code-review FND-001/FND-002) |
| AC-5 step relation through analyze (QSL-2238 is Done) | `hyper_step_relation_through_analyze`, ignored | unmet (FND-002) |
| AC-6 finite trace set is typed unsupported; copy budget is incomplete | `analyze_hyper_outside_the_admitted_forms_is_a_typed_refusal` (HP-4, not a trace set); `analyze_hyper_witness_budget_is_incomplete` | budget half covered; trace-set half missing (FND-003) |
| AC-7 tags FR-176-AC-1 / FR-173-AC-2 / FR-163 | hyper.rs:49; hyper_clauses.rs:599; closure_certificate.rs | covered |
| Lead ruling / FR-940-AC-18 (package route settles `Equal` proved) | `a_package_hyper_item_refuses_without_falling_back_to_source` | contradicted (FND-001) |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A new test pins the refusal the merged spec now forbids. It asserts that the package route returns `unsupported_construct`/`not-yet-implemented` with `items` `[]`, and it is tagged TC-2958/FR-940. FR-940-AC-18 (spec #3113, merged as the ruling on this ticket) requires that same request to settle `Equal` `proved`, and ruling item 6 forbids `not-yet-implemented`. The PR has no FR-940 package-form implementation and no ticket tracks one | fsl-analyze/tests/it/hyper_wire.rs:84-123 |
| FND-002 | medium | AC-5 is unmet and its ignore reason is stale. `hyper_step_relation_through_analyze` is `#[ignore = "blocked on QSL-2238: ... engine does not exist"]`, but QSL-2238 is Done. Run with `--ignored` at dc59305f00, it fails with "an item with a terminal record: null", so step relations still do not reach `fsl analyze` | fsl-analyze/tests/it/hyper_wire.rs:255-276 |
| FND-003 | medium | AC-6's "hyperproperties over an explicit finite trace set return the typed unsupported result" has no test. The test cited for it is an HP-4 `exists`/`forall` clause, which is a different refusal path | fsl-analyze/tests/it/hyper_wire.rs:228-253 |

## Findings detail

- **FND-001.** Either implement the FR-940 package hyper form in this PR, as ruled, or drop the FR-940/TC-2958 tag and the forbidden assertion and file a ticket for the remaining work. A test should not certify an outcome that FR-940-AC-18 forbids.
- **FND-002.** Either wire FR-179 through analyze now that QSL-2238 has landed, or re-point the ignore at a real open ticket and say what is still missing.
- **FND-003.** Add one analyze request over an explicit finite trace set (QSL-2239's form) asserting the typed unsupported code and cause, or record the trace-set half as deferred to QSL-2239.
