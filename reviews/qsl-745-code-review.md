---
id: SR-8730
title: "QSL-745 code review of PR #811"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@34a3c0a6e8ae0dbfd9cc8a68cd22c78c842a8df8; base impl/business-model-meanings@dea94c20c20c9b8416d9d57249b235bfe42b149a; qsl-replay/src/spine.rs; qsl-replay/src/spine/lifecycle.rs; qsl-replay/src/spine/native_result.rs; qsl-replay/src/spine/lifecycle/tests.rs; qsl-replay/src/spine/call/tests.rs; qsl-semantics/src/model/intake/unit.rs; qsl-semantics/src/model/refusal.rs; qsl-package/src/emit/tests.rs; consumers read: qsl-replay/src/outcome.rs, qsl-semantics/src/model/observation.rs, src/protocol_artifact/domain.rs, src/command.rs, tests/it/*.rs, tests/support/front_end.rs, qsl-eval/src/value/expression/causes.rs"
review_set: subset
---
## Summary

Ticket: QSL-745, parts (a) and (b). This is a code review with its Rust lane, run read-only over the PR diff at the reviewed sha. The PR stops replay `select` from turning intake refusals into stage limits. It gives `ancestor-steps`, `family-steps` and `intake-limit-exceeded` the fields `setting`, `bound` and `actual`. It routes compile-path fields through `CompileRefusal::fields()`.

The classification is correct. The removed `limit_of` Intake arm covered only `UnitIntakeCause::Limit`, which is always a `ModelNormalizationLimitsV1` denied charge, and the model causes for which `limit_exceeded()` is `Some`: `AncestorSteps` (with `model.*` or `admission.*` settings), `FamilySteps` and `IntakeLimitExceeded`. Every one is a caller budget in FR-255 Behavior 1's table. Every remaining `limit_of` arm carries a stage or reader limits type: S1 `SyntaxLimit`, S3 `CheckCause::ResourceExhausted`, identity, assembly, `DependencyLimits` imports and I2 import-view reads. No stage-limit type moved to `resource_exhausted`. The exit status stays 22, because both codes are `is_incomplete`.

`src/protocol_artifact/domain.rs:303` maps the intake limit to its own `Error::Incomplete(Exhaustion)`, a protocol-artifact code space rather than catalog codes. It already treats the limit as the caller's budget, so it agrees with the rule.

## Verdict

**FAIL**: the duplication rule records two in-repo duplications as high. Correctness, classification and the gate logs are sound.

Every changed test expectation would fail on the old code, judged by reading:

- `select_names_the_intake_limit_for_an_oversize_document`: the old `limit_of` returned `StageFailure::Limit`, so the `let-else` panics.
- `select_names_the_model_limit_field_it_reached`: the old code gave `Reached::Limit`, so `assert_budget_field` panics.
- The emit test: the old `UnitIntakeCause::code()` gave `StageLimitExceeded`.
- `tc_452_step_4`: the old `catalog_fields` gave `None`, which made the result `CallRefusal::Family`.
- `a_reached_model_ceiling_carries_setting_bound_and_actual`: red in `qsl745-sem-red.log` (assertion failed at refusal.rs:2098).

Gate logs read:

- clippy `--workspace --all-targets --all-features -D warnings` exits 0. The root crate compiled.
- sem-lib: 519 passed. sem-it: 606 passed.
- replay-all: 491 passed, plus 6, 1, 12, 9 and 13.
- pkg-eval: 60, 308, 124 and 12 passed.
- xtask: 182 passed. spec: exit 0.

The replay-1 log shows the `CatalogCoded` unused-import warning that the second commit removes. The clippy run, which postdates it, passed with `-D warnings`.

The root crate's tests were not run. A grep found no root test that pins `stage_limit_exceeded`, or a null cause, for a `model.*` or `intake.*` limit. `tests/it/compile_command.rs` checks intake `code` only, and every root `stage_limit_exceeded` assertion is an S1/syntax limit.

The cause of every Intake refusal is now non-null, through `CompileRefusal::cause()`. This is intended: a cause tag is now reported where the catalog has one. No test or consumer depends on the null.

`assert_budget_field` costs about 17 binary-search runs and 2 more per field, the same as `assert_field`. It is deterministic.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The denied-charge record for a `ModelNormalizationLimitsV1` ceiling is now built twice. The literals `stage`/`model-normalization`, `accounting_contract`/`ModelNormalizationLimitsV1`, `consumed`, `next_charge` and `charge_point`, plus `setting`/`bound`/`actual`, appear in both `UnitIntakeCause::limit_fields` and observation admission's `caller_budget_refusal` call. A third hand-built setting/bound/actual map is in `ModelRefusalCause::catalog_fields`. A field added to the FR-255 record must be edited in all of them. Searched: `ModelNormalizationLimitsV1"`, `"charge_point"` and `catalog_fields` across qsl-semantics. Give `NormalizationIncomplete` (or `LimitExceeded`) one caller-budget field method and call it from all three sites. | qsl-semantics/src/model/intake/unit.rs:145-172; qsl-semantics/src/model/observation.rs:868-881, 1066-1077; qsl-semantics/src/model/refusal.rs:1178-1187 |
| FND-002 | high | The test helper `assert_budget_field` copies `assert_field_with`'s binary search (high check, `low`/`top` loop, `counter > 0`, one-below and at-counter runs) and changes only the per-step match arms. A fix to the search must be made twice. Make one search generic over a classifier closure that returns the reached limit's setting, and keep the two final-step oracles separate. | qsl-replay/src/spine/lifecycle/tests.rs:1099-1139, 1144-1186 |
| FND-003 | low | `UnitIntakeCause::cause()` takes the cause from `refusals[0].cause.catalog_code()`, but `code()` still reads the native-v1 `refusals[0].code` field. ADR-013 R-09 says `ModelRefusal::catalog_code` reads the cause alone. The newly emitted (code, cause) pair therefore comes from two sources and can disagree for any producer whose `code` field differs from its cause's catalog code. Read both from `refusals[0].catalog_code()`. | qsl-semantics/src/model/intake/unit.rs:120-138 |
| FND-004 | low | Two comments are stale. The comment above the family cases says "with no row: `TypeMismatch`, `AncestorSteps`", but `AncestorSteps` now builds a record. The `catalog_fields` doc still says it returns a payload only "for a cause with a row in FR-096's key table". The three causes it now fills have no row there (see the spec-review artifact). | qsl-replay/src/spine/call/tests.rs:1810; qsl-semantics/src/model/refusal.rs:1159-1162 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | After this PR, `ModelRefusalCause::limit_exceeded()` and `Incomplete::limit_exceeded()` have no production caller. The removed `limit_of` Intake arm and the old `caller_budget_refusal` were their last callers; `AdmissionIncomplete::limit_exceeded()` is in the same state. Each turns a caller budget into a `LimitExceeded`, the stage-limit record (`stage_limit_exceeded`, `kind`, `bound`), which Ruling A and FR-255 Behavior 1 say a caller budget never produces. Only tests keep them alive, and four of those tests still say `.expect("a step ceiling is a stage limit")`. Delete the three methods. Point the tests at `denial()` / `catalog_fields()`, or at the variant fields. | qsl-semantics/src/model/refusal.rs:1127-1160; qsl-semantics/src/model/accounting.rs:379-389; qsl-semantics/src/model/population.rs:348-358; qsl-semantics/tests/it/model_conformance.rs:2352, 2427; qsl-semantics/tests/it/model_population.rs:1641, 1715, 3416, 3569; qsl-semantics/tests/it/model_dispatch.rs:265, 842; qsl-semantics/src/model/intake.rs:8359 |
| FND-006 | low | Two doc comments are stale. `CompileRefusal::fields` still says a caller-budget ceiling carries "`setting`, `bound` and `actual`". It now carries `limit` and the retained charge, or for intake the size record `kind`/`bound`/`actual`/`setting`. `caller_budget_refusal`'s doc names only `ModelNormalizationLimitsV1`, but document.rs also calls it for `admission.ancestor_steps` walks (`PopulationAdmissionLimitsV1`). | qsl-replay/src/spine.rs:246-249; qsl-semantics/src/model/observation.rs:1047-1052 |
| FND-007 | low | `BudgetDenial::charge` takes four adjacent `&'static str` parameters by position (`stage`, `accounting_contract`, `limit_kind`, `charge_point`) under `#[allow(clippy::too_many_arguments)]`, so a swap compiles. The owner pair `"model-normalization"`/`"ModelNormalizationLimitsV1"` is spelled at three call sites. `setting()`, `limit()` and `actual()` have no caller. Fix: a typed budget owner (one const per accounting contract holding stage and contract), and delete the unused accessors. | qsl-foundation/src/diagnostic/budget.rs:53-93; qsl-semantics/src/model/accounting.rs:396-406; qsl-semantics/src/model/refusal.rs:1166-1196 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | low | The rewritten `IntakeLimitExceeded` doc cites "ADR-014 section 1" for intake's byte limit being a caller budget that refuses `resource_exhausted`. ADR-014 never mentions intake limits: section 1 lists accounting-contract types under B-2 and reader limits under B-3. SR-8732 FND-004 removed the same citation from FR-277. The classification comes from FR-255 Behavior 1's intake row and FR-260 item 5 (FR-260-AC-4). Cite those. | qsl-semantics/src/model/refusal.rs:879-883 |

## Dispositions

Round 1, reviewed at a7be05cb81cb87eab1b83eee676595771d16b494.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | d060eb068: `qsl_foundation::diagnostic::BudgetDenial` builds both records once. `UnitIntakeCause::denial`, `Incomplete::denial`, `ModelRefusalCause::denial`/`catalog_fields`, observation `caller_budget_refusal` and the document.rs conformance walk all call it. A grep for `"accounting_contract"`, `"charge_point"`, `"next_charge"` and `"limit_kind"` outside tests finds only budget.rs |
| FND-002 | fixed | d060eb068: `smallest_admitted` is the one bisection. `assert_field_with` and `assert_budget_field` keep only their own final-step oracles |
| FND-003 | fixed | d060eb068: `UnitIntakeCause::catalog_code()` reads `refusals[0].catalog_code()`, which reads the cause alone (ADR-013 R-09). `code()` and `cause()` both derive from it |
| FND-004 | fixed | d060eb068: the call/tests.rs comment now names only `TypeMismatch` as recordless. The `catalog_fields` doc points the walk ceilings and intake at the FR-096 rows the PR adds |

Round 2, reviewed at bd42e758db3cd9f61c71f56bbcaff02d8551f8d2.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-005 | fixed | bd42e758d: `limit_exceeded()` is deleted from `ModelRefusalCause`, `Incomplete` and `AdmissionIncomplete`; no `limit_exceeded()` call remains in the workspace. `AdmissionIncomplete` gains `denial()` with the `ADMISSION_OWNER` charge record. The nine test call sites use `denial()` and now expect "a step ceiling is a denied charge" |
| FND-006 | fixed | bd42e758d: the `CompileRefusal::fields` doc names the intake size record and the denied charge's `setting`, `limit`, `actual` and retained charge. The `caller_budget_refusal` doc names both `ModelNormalizationLimitsV1` and `PopulationAdmissionLimitsV1` |
| FND-007 | fixed | bd42e758d: `BudgetDenial::charge(setting, limit, DeniedCharge)` takes a struct with named fields, so a swap no longer compiles. `BudgetOwner` is spelled once per contract, as `MODEL_NORMALIZATION_OWNER` (accounting.rs) and `ADMISSION_OWNER` (population.rs). The accessors `setting()`, `limit()` and `actual()` are now read by six tests; they are the public read API of a public type, so keeping them is legitimate |
