---
id: SR-646
title: "QSL-271 base checklist review of spine run"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@3c7c0a8bb365b9ce460dfda97a037682a5c94100; spec/functional/FR-100-run-a-named-function-through-the-spine.md; spec/test-cases/TC-450-cli-run-routes-a-program-by-its-declared-edition.md; spec/test-cases/TC-451-spine-run-binds-arguments-and-maps-outcomes-to-exit-codes.md; spec/test-cases/TC-452-spine-run-entry-lives-in-qsl-replay.md; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md; spec/spec.md; spec/tests.md; checker evidence qsl-semantics/src/check/check.rs, qsl-semantics/src/check/facts.rs, qsl-eval/tests/it/total_functions.rs, quire-exact/src/outcome.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: reviews
---

## Summary

Ticket: QSL-271 (PR agent-ix/quire-spec-language#458, spec only). This is the
base checklist over the seven changed files. It covers ID formats, AC
testability, the six coverage rules, and TC oracles against the coverage
intents: routing, request form, argument encoding, outcome JSON, FR-301 exit
codes, the `qsl_replay` entry and the layering constraint.

Clean: the IDs are well formed (FR-100, FR-100-AC-1 to AC-7, TC-450 to
TC-452). Every AC maps to a TC, and the tests.md rows and the spec.md index row
are present. `quire validate` over the changed files exits 0. Routing, the
request form, argument encoding, the FR-301 exit codes for completed, refused
before S6a, and incomplete, and the `qsl_replay` entry each have a concrete
oracle. The `work_units` 0 incomplete case can be reached, because
`CheckedPackage::call` charges `function.call` before the body
(qsl-eval/src/value/expression/mod.rs:338-347).

Verdict: changes requested. One high finding: the AC-6 undefined case cannot
be reached through the checker.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-100-AC-6's undefined case cannot be tested as written. TC-451 step 7's `inv(x: Digit): Boolean { 1 / x > 0 }` does not pass the S3 checker, so `inv(0)` never reaches S6a. Integer `/` needs a Rational expected type. Without one, the checker refuses `ill_typed`, with cause `AmbiguousLiteral` when there is no hint and a mismatch when the hint is not Rational (qsl-semantics/src/check/check.rs:1398-1411). With a Rational context, the facts pass refuses `undefined_expression`/`unproved-nonzero`, because `x: Int[0, 9]` does not exclude zero (qsl-semantics/src/check/facts.rs:770-781, pinned by TC-191 `p04_division_presence_and_reduction_need_static_proofs`, qsl-eval/tests/it/total_functions.rs:431-441). Failure scenario: the TC-451 step 7 test gets a stage `check` refusal on stderr with empty stdout, not a `spine-run-result/1` `undefined` outcome, so AC-6 cannot pass. No other construct fits either. Kernel `EmptyReduction` and `NoneValue` are "only direct kernel evaluation of an unlinked expression" (quire-exact/src/outcome.rs:99-104). `IeeeNotFinite` needs a Float value, but 1-draft float literals are unrepresented (qsl-forms/src/value.rs:1102) and FR-100 admits only integer and Boolean arguments. The family `PreconditionFalse` (FR-151 dispatch) and `AbsentKey` (FR-153 `lookup`) each need a model object reference and a population, which run does not supply. Fix: verify the undefined row below the CLI. Unit-test the outcome renderer and exit mapping over a constructed `Outcome::Undefined` and `FamilyResult::Undefined`. Say in AC-6 that FR-146 totality makes the undefined outcome unreachable through CLI `run` for integer and Boolean 1-draft programs at this revision, and drop `inv` from TC-451. | spec/functional/FR-100-run-a-named-function-through-the-spine.md:166; spec/test-cases/TC-451-spine-run-binds-arguments-and-maps-outcomes-to-exit-codes.md:27, 36, 61-63 |
| FND-002 | medium | The `refused` outcome kind (`{"kind": "refused", "code": ...}`, exit 20 or 21) has no AC or TC step. AC-4 and AC-5 cover only refusals before S6a, which go to stderr. Failure scenario: an implementation that never emits `refused` in `spine-run-result/1`, or that maps it to the wrong exit status, passes TC-450 to TC-452. If, like `undefined`, the kind cannot be reached through the CLI for integer and Boolean programs, cover it the same way as FND-001 (a renderer and exit-map unit test) and say so. | spec/functional/FR-100-run-a-named-function-through-the-spine.md:89, 94-95, 160-167 |
| FND-003 | low | The layering Behavior statement "The types `qsl_replay::spine::run` takes and returns shall be `qsl_replay`, `qsl_foundation`, `qsl_semantics` or `quire_exact` types" has no AC. AC-7 and TC-452 step 2 check only that the root crate names no `qsl-eval` dependency (TC-390). Failure scenario: `qsl_replay` adds `pub use qsl_eval::value::Evaluation` and returns it from `spine::run`. The root crate still compiles with no `qsl-eval` edge, TC-390 passes, and the statement is broken. Add an AC clause and a TC-452 step, for example an arch-lint or public-API check that `spine::run`'s signature names no `qsl_eval` path. | spec/functional/FR-100-run-a-named-function-through-the-spine.md:154-155, 167; spec/test-cases/TC-452-spine-run-entry-lives-in-qsl-replay.md:25-26 |
| FND-004 | low | Some AC oracles leave out stated outputs. AC-1 and TC-450 step 1 do not assert `request_digest`, `format` or the `source` authored path. AC-5 tests only the `source` stage (a syntax error) of the five spine refusal stages FR-027-AC-8 lists, although run reuses compile's refusal path. A `check`-stage case is the one that exercises a check cause code's exit mapping. The `inv` source from FND-001 is a ready fixture for it (`undefined_expression` or `ill_typed` at stage `check`). Failure scenario: a wrong `request_digest`, or a check refusal reported at the wrong stage, passes TC-450 and TC-451. | spec/functional/FR-100-run-a-named-function-through-the-spine.md:76-84, 161, 165; spec/test-cases/TC-450-cli-run-routes-a-program-by-its-declared-edition.md:49-54; spec/test-cases/TC-451-spine-run-binds-arguments-and-maps-outcomes-to-exit-codes.md:35, 59 |
| FND-005 | low | The oracles say "naming the name" and "naming parameter position 0" without saying which member of the command-error envelope carries it: `message` or `details` (schemas/native-run-result-1.schema.json:331-356 requires both). Failure scenario: two implementations that both pass disagree on where a consumer reads the parameter position, and TC-452's "parameter the CLI renders" comparison has no fixed field to compare. Name the member, as FR-027-AC-7 does with "a message naming the file". | spec/functional/FR-100-run-a-named-function-through-the-spine.md:164-165; spec/test-cases/TC-451-spine-run-binds-arguments-and-maps-outcomes-to-exit-codes.md:48-57 |
| FND-006 | low | Disposition pass, at 8c786c48. FR-100-AC-5 and TC-451 step 6 expect `inv(x: Int[0, 9]): Boolean { 1 / x > 0 }` to refuse at stage `check` "with the check stage's cause code" but do not name the code, although the checker's own code is known. It was measured by compiling exactly this unit through `qsl_replay::spine::compile` (a temporary probe test in qsl-replay's `execute/tests.rs` harness, since reverted). The result is stage `Check`, code `ill_typed`, cause `IllTyped(AmbiguousLiteral)`, at the body path `[0]`. That is the integer `/` arm with no Rational expected type (qsl-semantics/src/check/check.rs:1398-1411). It is not `undefined_expression`: the nonzero obligation never runs, because typing refuses first. Failure scenario: a later checker change moves the refusal to `undefined_expression`/`unproved-nonzero`, or to another stage code, and TC-451 step 6 still passes, so the exit mapping of the check stage is not pinned. Pin `ill_typed` (exit 20) in AC-5 and in TC-451 step 6. | spec/functional/FR-100-run-a-named-function-through-the-spine.md:238; spec/test-cases/TC-451-spine-run-binds-arguments-and-maps-outcomes-to-exit-codes.md:62-64 |

## Dispositions

Disposition pass at `agent-ix/quire-spec-language@8c786c48fcd7b6fdf4bc56799b4bf9db1fa2ce48` (fix commit `8c786c48`, "QSL-271 spec: fix SR-646 to SR-649 findings on spine run"). Each outcome was re-checked against the spec at that head, not taken from the commit message.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed 8c786c48 | `inv` is gone from the undefined case: TC-451 step 7 is now `seven` with `work_units` 0 only, and AC-6 has no undefined clause. The undefined row is covered below the CLI by the total mapping table (FR-100:109-116) and by AC-9 and TC-452 step 4, which construct each `Outcome::Undefined` and `FamilyResult::Undefined`. `inv` is kept as a check-stage fixture; see new FND-006 for its unpinned code. |
| FND-002 | fixed 8c786c48 | The `refused` outcome now has mapping rows for kernel and family refusals with exit statuses (FR-100:113-114), and AC-9 and TC-452 step 4 construct each one. The kernel spelling table this relies on is incomplete; see SR-649 FND-004. |
| FND-003 | fixed 8c786c48 | FR-100-AC-8 and TC-452 step 3 add an arch-lint API-surface check over `qsl_replay::spine` public items and re-exports, including a failing probe of `pub use qsl_eval::value::Evaluation`. `tools/arch-lint/api_surface.rs` exists, and `qsl_replay` has no `qsl_eval` re-export today (qsl-replay/src/lib.rs:38-63). |
| FND-004 | fixed 8c786c48 | AC-1 and TC-450 step 1 now assert `format`, `request_digest`, the `source` authored path and empty stderr. AC-5 and TC-451 step 6 add a check-stage case (`inv`). Its code is left open; see new FND-006. |
| FND-005 | fixed 8c786c48 | The refusal table fixes the `details` keys: `{"function"}`, `{"parameter"}` and `{"position"}` (FR-100:152-155). AC-4, AC-5 and TC-451 steps 2, 3 and 5 assert them. |
| FND-006 | open | New in this pass. |
