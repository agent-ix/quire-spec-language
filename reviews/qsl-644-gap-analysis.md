---
id: SR-1351
title: "Gap analysis of quire-spec-language PR #649: the compile_package facade (QSL-644)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@3d53d1dd4c57afaac44a9112bc8ae1a270fa969f; PR #649 diff against origin/main; FR-060-AC-5..AC-7; trace tags TC-908 (qsl-replay/tests/compile_package_facade.rs)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-060
    type: reviews
---
# Gap analysis of quire-spec-language PR #649

## Summary

Ticket: QSL-644. Planless gap analysis. Plan completion: not assessed.

Trace: each of FR-060-AC-5, AC-6 and AC-7 has a tagged test in qsl-replay/tests/compile_package_facade.rs, under `#[trace("TC-908", ...)]`. There are four tests over the three ACs. The new production code (`compile.rs`, `run_spine`) belongs to FR-060's "The facade's compile entry" statement. None of the tests is a stub, and none is tautological.

Oracle strength. Each test compares the facade with an independent spine run written out in the test (`spine_run`: parse, select, check, package under `SpineLimits::default()`).
- If the facade dropped the packages, or keyed them differently from `package_input`, AC-7's with-package byte comparison fails.
- If it skipped `check_unit_owner`, AC-6's owner test fails. That test asserts the `DependencyInput` variant, which no spine refusal maps to.
- If it ignored `text_input_bytes` or the reader ceiling, AC-6's limit test fails.
- If it changed a spine stage's arguments, AC-5's and AC-7's byte and `package_id` comparisons fail.
- If `run_spine` itself drifted (say, a non-default `LockEvidence`), replay and the facade would move together, and the test's own `spine_run` would still catch it.

Model selections (the plan lead's requirement): AC-7 covers them. The test compiles a unit whose `model M = "acme/orders" ... digest "sha256-jcs:<hex>"` names the fixture `tests/fixtures/spine-model.semantic-ir.json`. It asserts the bytes and `package_id` equal the spine's when the package is supplied, and `missing_import` when it is not.

Examined:
- FR-060-AC-5 (examined): "`qsl_replay::compile_package` compiles FR-092's recursive `List` ... and `Tree` ... from source through the facade alone, and for each the package bytes and `package_id` equal those of the spine's parse, select, check and package run over the same source."
- FR-060-AC-6 (examined): "`qsl_replay::compile_package` refuses as the spine does: a malformed source refuses `ReplayRefusal::Recompile` with the spine refusal's catalog code and stage; a library whose source has the unit's owner refuses `ReplayRefusal::DependencyInput`; an S1 `text_input_bytes` limit below the source's size refuses `stage_limit_exceeded`; and an S1 limit above the reader limit refuses `LimitAboveReader`."
- FR-060-AC-7 (examined): "... without the package refuses `missing_import` at the intake stage as the spine does."
- TC-908 (examined)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-060-AC-7 says that without the package the facade refuses `missing_import` "at the intake stage as the spine does". The test asserts only the code, against the spine and against `Code::MissingImport`, and never the stage. A facade that refused `missing_import` from another stage would pass. Add `assert_eq!(refusal.stage(), expected.stage())`, as the malformed-source test does at line 135. | qsl-replay/tests/compile_package_facade.rs:210-215 |

## Verdict

One low finding: AC-7's "at the intake stage" clause is not asserted. Every AC is traced and has a real, divergence-sensitive oracle. Mergeable once FND-001 is fixed in this PR.
