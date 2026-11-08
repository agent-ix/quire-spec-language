---
id: SR-164206
title: QSL-591 gap-analysis PR2576 execution seam
type: SpecReview
analysis: gap-analysis
review_set: subset
scope: "agent-ix/quire-spec-language; QSL-591 PR2576 @883d75ea5 FR-279, FR-294, FR-296 (function half) against qsl-eval/src/execution.rs and qsl-replay/src/spine/call/execution_tests.rs"
relationships:
- type: references
  target: ix://agent-ix/quire-spec-language/FR-279
- type: references
  target: ix://agent-ix/quire-spec-language/FR-294
- type: references
  target: ix://agent-ix/quire-spec-language/FR-296
---

## Summary

Ticket: QSL-591. Planless gap analysis of FR-279, FR-294 and FR-296 for the function path. Trace tags were collected by grep over `#[trace(...)]`, because a repo-wide `quoin matrix --json` did not finish inside the review window. Thirteen tests are tagged, covering FR-279-AC-1/3/4, FR-294-AC-1/3/4/5/6/7 and FR-296-AC-1/2/3/4. FR-294-AC-2 has no tag. Out of scope, as known and pending: the clause half (FR-279-AC-2, FR-294-AC-1's `evaluate` clause, `EntrySelection::Clause`), FR-296-AC-4's `ExecutionBackendChoice::default()` clause (waiting on the quire-eval leaf change) and FR-294-AC-2's AOT `generate` clause (driver repo).

## Verdict

CONDITIONAL. No production stub was found. The tests for FR-279-AC-1/4, FR-294-AC-1/3/4/5 and FR-296-AC-1/2/3/4 assert real behavior through the real `execute` and the spine. Four tagged or owned ACs are only partly evidenced, so their coverage is overstated. Changed behaviors without a test: the native-result fields of the new refusal records. Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-294-AC-2 has a single `compile_fail` doctest, for package bytes. The ParsedSource and unchecked-expression cases the AC lists are missing. No test is traced to FR-294-AC-2, so the AC shows uncovered. Add the two doctests and a traced control test, following the FR-275-AC-2 convention. | qsl-eval/src/execution.rs:18-40 |
| FND-002 | medium | The test tagged FR-294-AC-6 calls a free function, `translate`, with five synthetic nodes and two blocks. It does not run a backend's `prepare` over FR-262-AC-1's `count`. It does not check that each charge comes before its node or block, and it does not check that no prepared value is returned. Only the meter arithmetic and the cancel poll are evidenced. | qsl-replay/src/spine/call/execution_tests.rs:557-646 |
| FND-003 | medium | The test tagged FR-294-AC-7 runs a `find` over construct lists the test builds by hand, then checks the payload rendering. No backend `prepare` runs, and all three entries share one `location()`. The AC requires "each `location` that construct's", and nothing asserts it. | qsl-replay/src/spine/call/execution_tests.rs:648-718 |
| FND-004 | medium | The test tagged FR-279-AC-3 covers FR-100-AC-4 (`lt`, `id`) and AC-6 (`seven` at 0 work units) but leaves out FR-100-AC-1's request (`seven` completes with 7) and `flag`. It compares the Debug output of the in-memory `CallOutcome`, not the `outcome` member of the `spine-run-result/1` document the AC names. | qsl-replay/src/spine/call/execution_tests.rs:273-326 |
| FND-005 | low | The native-result disposition arms for `RunRefusal::Unsupported` (`backend`, `construct`, `construct_name`) and `RunRefusal::PrepareLimit` (`setting`, `bound`, `actual`) have no test, so the rendered fields are unverified. | qsl-replay/src/spine/native_result.rs:2378-2398 |

## Coverage

```yaml
scope:
- {id: FR-279-AC-1, path: spec/functional/FR-279-execute-a-checked-entry-as-a-library-operation.md, role: examined, excerpt: "`execute` with the interpreter backend over FR-100-AC-4's unit returns, for `lt(5, 3)`, `flag(1)` and `id(4)`, the `Evaluation` `CheckedPackage::call` returns for the same arguments, and for `id(12)` the same `CallFailure::Input`."}
- {id: FR-279-AC-3, path: spec/functional/FR-279-execute-a-checked-entry-as-a-library-operation.md, role: examined, excerpt: "For each FR-100-AC-1, AC-4 and AC-6 request, the `outcome` member of FR-100's `spine-run-result/1` document equals the rendering of `execute`'s result for the same source, function, arguments and accounting limits."}
- {id: FR-279-AC-4, path: spec/functional/FR-279-execute-a-checked-entry-as-a-library-operation.md, role: examined, excerpt: "`execute` choosing a supplied test backend standing in for `aot` whose `prepare` returns `PrepareFailure::Limit` naming `prepare.code_bytes` with configured value 1 and counter 2 returns `CallFailure::Limit(PrepareLimitReached { ... })`"}
- {id: FR-294-AC-1, path: spec/functional/FR-294-run-every-execution-backend-behind-one-seam.md, role: examined, excerpt: "The interpreter backend, prepared over FR-100-AC-4's unit with every function selected, returns for each AC-4 call the `Evaluation` or `CallFailure` that `CheckedPackage::call` returns"}
- {id: FR-294-AC-2, path: spec/functional/FR-294-run-every-execution-backend-behind-one-seam.md, role: examined, excerpt: "Passing package bytes, a `ParsedSource` or an unchecked expression to `prepare`, and package bytes read from a file to AOT's `generate` step, each fails to compile"}
- {id: FR-294-AC-3, path: spec/functional/FR-294-run-every-execution-backend-behind-one-seam.md, role: examined, excerpt: "A test backend standing in for `jit` that cannot run the third of three selected entries returns `Ok` from `prepare`"}
- {id: FR-294-AC-4, path: spec/functional/FR-294-run-every-execution-backend-behind-one-seam.md, role: examined, excerpt: "`prepare` and `call` with a handle already cancelled return `PrepareFailure::Cancelled` and `CallFailure::Cancelled`."}
- {id: FR-294-AC-5, path: spec/functional/FR-294-run-every-execution-backend-behind-one-seam.md, role: examined, excerpt: "`PrepareLimits::default()` has `compile_work_units` 16777216 and `code_bytes` 67108864."}
- {id: FR-294-AC-6, path: spec/functional/FR-294-run-every-execution-backend-behind-one-seam.md, role: examined, excerpt: "A test backend that follows \"Prepare charges\", prepared over FR-262-AC-1's `count` alone, records one `compile_work_units` charge per checked body node it translates, each before that node"}
- {id: FR-294-AC-7, path: spec/functional/FR-294-run-every-execution-backend-behind-one-seam.md, role: examined, excerpt: "A test backend standing in for `jit` that does not translate `let`, has no representation for `Decimal` and does not implement one checked operation ... each `location` that construct's"}
- {id: FR-296-AC-1, path: spec/functional/FR-296-select-the-execution-backend-in-the-request.md, role: examined, excerpt: "An `execute` request that names no backend runs `seven` on the interpreter and completes with 7."}
- {id: FR-296-AC-2, path: spec/functional/FR-296-select-the-execution-backend-in-the-request.md, role: examined, excerpt: "An `execute` request choosing `jit` with `SuppliedBackends::default()` returns `CallFailure::Unsupported(UnsupportedEntry { backend: Jit, cause: NotSupplied })`, category unsupported (exit 21), and no stage runs."}
- {id: FR-296-AC-3, path: spec/functional/FR-296-select-the-execution-backend-in-the-request.md, role: examined, excerpt: "With a test JIT that lists one entry `unsupported` with cause `Untranslatable`, `execute` of that entry returns `CallFailure::Unsupported` with that `UnsupportedEntry`, category unsupported (exit 21), and the interpreter is never called."}
- {id: FR-296-AC-4, path: spec/functional/FR-296-select-the-execution-backend-in-the-request.md, role: examined, excerpt: "`ExecutionBackendChoice::default()` is `Interpreter`; a test JIT passed as `SuppliedBackends { aot: None, jit: Some(&jit) }` through the blanket `SuppliedBackend` implementation runs `seven` when `jit` is chosen and is the only backend whose `prepare` is called"}
- {id: FR-279-AC-2, path: spec/functional/FR-279-execute-a-checked-entry-as-a-library-operation.md, role: context_only, excerpt: "`execute` with a clause selection over FR-109's ConfigVersion unit returns the `Evaluation` `qsl_replay::spine::run_clause` reports for the same clause and input."}
```

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | low | The new unchecked-expression `compile_fail,E0308` doctest imports `qsl_forms::syntax::Expression`, but `syntax` is a private module (`mod syntax;`). The import fails with E0603 for a reason unrelated to the seam. The doctest passes only because rustc also reports E0308 once privacy fails. Import the public re-export, `qsl_forms::Expression`. | qsl-eval/src/execution.rs:37-45 (import at :40) |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | 9d3a3f2af: ParsedSource and unchecked-expression `compile_fail` doctests were added, plus `prepare_admits_a_checked_package`, the traced FR-294-AC-2 control. The AOT `generate` clause stays with the driver repo. The import path of the expression doctest is FND-006. |
| FND-002 | fixed | 9d3a3f2af: a `Translator` backend walks the checked body of `tally`, which is FR-262-AC-1's `count` body under another name because `count` is a grammar keyword, so the checked node shape is the same. It logs each charge followed by its node or block, checks that the refused charge translated nothing (nodes-1), and checks one below and at each count, no prepared value on a limit, and the cancel from a second thread. |
| FND-003 | fixed | 9d3a3f2af: a real `prepare` over four checked entries lists the first untranslatable construct of each in node order (let, Decimal, an application of `id`, and an application before a `let`). Each location is the refused checked node's own `node.location()`, carried verbatim through `unsupported()` and `payload()`. The test checks that the locations are pairwise distinct rather than comparing each with an independently looked-up node location. That is acceptable, because the seam's duty is to carry the location and the test backend's choice of node is scaffolding. |
| FND-004 | fixed | 9d3a3f2af: the FR-100-AC-1 (`seven` to 7), AC-4 (`lt` with b before a, `flag`, `id`) and AC-6 requests are covered, and the run's outcome and `execute`'s rendering are compared as `spine-run-result/1` outcome JSON against FR-100's literal values. The JSON comes from a test-local mirror, because the document writer lives in the root crate, which qsl-replay cannot depend on. tests/it/spine_run.rs covers the real writer. |
| FND-005 | fixed | 9d3a3f2af: `the_native_result_renders_the_unsupported_and_prepare_limit_refusals` checks backend, construct, construct_name, location, setting, bound and actual in the written clause-run document. |
