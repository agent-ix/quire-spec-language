---
id: SR-16337
title: "QSL-656 PR 1395 gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR 1395 changed Rust files in qsl-foundation, qsl-replay, qsl-eval, xtask, and related callers and tests"
review_set: subset
---

## Summary

Ticket: QSL-656. Independent review of PR #1395's accounting setting rename and ci-docs correction. This follow-up has new PR-specific artifacts; the existing QSL-656 artifacts concern PR #665 and are preserved.

## Verdict

**CONDITIONAL** — The changed refinement report assertion contradicts FR-344-AC-4 and no longer names an accepted key in the reported accounting member.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The changed refinement report oracle now expects name=accounting.work_units with member=accounting, but FR-344-AC-4 requires the bare work_units counter. limit_reports forwards ReachedLimit.name unchanged (report.rs:279), while Limits::of accepts only bare counter keys under case.accounting (versioning.rs:536). Consequently the report's name/member pair cannot be applied to the named case member. Preserve the bare counter in this report, or explicitly revise its contract and distinguish a setting name from the case-member key. | xtask/tests/refinement_versioning.rs:733 |

## Coverage

Plan completion: not assessed

Diff reviewed: origin/main...HEAD at the dispatched candidate. Read repository AGENTS.md, CLAUDE.md, README.md, LICENSE-DECISION.md, CONTRIBUTING.md, Cargo and deny configuration; current FR-255, related criteria below, QSL-656 and newest comments, QSL-661 and newest comments. No spec or CI workflow change in the diff; no spec-review method required. No applicable AssuranceProfile found. No new unsafe, concurrency, narrowing conversions, dependency copies, or gate weakening in this change.

Manual affected-criterion-to-test review ran, including production routing through SettingName, parse_operands, StageLimits, effective_accounting_limits, CallIncomplete, OutcomeDocument, refinement reporting and case-file parsing. All ten setting names and bare counter conversions have executable assertions; zero/max, unknown, duplicate and overflow operands, replay precedence and identity consistency are exercised. Existing FR-255-AC-11 tests compare sent and reported identities and successful/incomplete results. No whole-repository coverage claim is made.

quire matrix --scope . --format json emitted module warnings, then no result after several minutes; it was interrupted. Matrix evidence is unavailable, and the permitted manual AC-to-test fallback was used. The optional repository-wide semantic audit was not run. quoin 0.28.3 supplied the installed SpecReview schema.

Cargo verification did not execute: df reported 247 GiB available; the mandatory locked-build.sh wrapper failed before cargo fmt --check with 'Failed to connect to bus: No data available'. Tests, clippy, docs and cargo-deny were therefore not independently run. The parent owns the existing aggregate gate; none was started here. The integration failure is established by the parser branch and test operand, not claimed as an observed test execution.

## Examined scope and bindings

```yaml
{
  "clean": false,
  "scope": [
    {
      "id": "FR-255-AC-3",
      "path": "spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md",
      "role": "examined",
      "excerpt": "Each stage's and operation's limits type, including `FormatLimits`, and the call's accounting limits, maps every one of its fields to exactly one setting name"
    },
    {
      "id": "FR-255-AC-7",
      "path": "spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md",
      "role": "examined",
      "excerpt": "FR-100's `seven` (`tests/fixtures/spine-compile.native`), called under the accounting limits the settings operation returns for the operand `accounting.work_units=0`, settles `Incomplete` naming counter `work_units`, bound 0, count 1 and setting `accounting.work_units`; its outcome names setting `accounting.work_units`. Under the operand `accounting.work_units=1000000` the same call completes with 7."
    },
    {
      "id": "FR-255-AC-8",
      "path": "spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md",
      "role": "examined",
      "excerpt": "For each of the ten rows of the accounting table, the settings operation given `accounting.<counter>=123456789` returns accounting limits with that counter at 123456789 and the other nine at their accounting-table defaults; given no operand, it returns accounting limits equal to the accounting-table defaults; given `accounting.work_units=18446744073709551615`, it returns `work_units` at `u64::MAX`."
    },
    {
      "id": "FR-255-AC-9",
      "path": "spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md",
      "role": "examined",
      "excerpt": "The settings operation given `depth=1`, the bare counter `work_units=1` and `accounting=1` each returns a usage refusal with cause unknown setting; given `accounting.work_units=ten`, `accounting.work_units=-1` and `accounting.work_units=18446744073709551616` each returns cause not an integer; given `accounting.work_units=5` and `accounting.work_units=6` together returns cause repeated naming `accounting.work_units=6`. Each refusal names its operand, and no stage runs."
    },
    {
      "id": "FR-255-AC-10",
      "path": "spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md",
      "role": "examined",
      "excerpt": "A replay request whose `stage_limits` holds the entry `accounting.work_units` at `B` decodes, and its call runs with `work_units` at `B` whatever the request's accounting limits give `work_units`, and with every other counter at the request's accounting-limits value."
    },
    {
      "id": "FR-255-AC-11",
      "path": "spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md",
      "role": "examined",
      "excerpt": "Two replay requests that differ only in how they reach the same effective accounting limits, one carrying every counter in its accounting limits and no accounting entry in `stage_limits`, the other carrying `accounting.work_units` and `accounting.value_occurrences` only as `stage_limits` entries over accounting limits holding other values for them, give the same function-level value-parity claim identity (`claim()`, FR-357) on every outcome, settle the same, and charge the same."
    },
    {
      "id": "FR-344-AC-4",
      "path": "spec/functional/FR-344-report-a-refinement-gate-verdict-and-exit.md",
      "role": "examined",
      "excerpt": "A case whose superseding run exceeds its `accounting` work budget names, in its entry, `work_units`, the budget's value and the member `accounting`."
    },
    {
      "id": "FR-340-AC-4",
      "path": "spec/functional/FR-340-run-the-spec-versioning-refinement-gate-over-a-corpus.md",
      "role": "examined",
      "excerpt": "A case stating no limit sets runs with each set's published default; the same case stating `accounting` with a work budget of one unit runs both revisions with that budget: both classes are `incomplete` and the case is `unresolved (incomplete)`."
    },
    {
      "id": "FR-343-AC-5",
      "path": "spec/functional/FR-343-compare-prior-and-superseding-classes-per-case.md",
      "role": "examined",
      "excerpt": "A pair whose superseding compile refuses `unsupported_construct`/`not-yet-implemented` gives `unresolved (unsupported)`; a case whose `accounting` work budget the prior run fits and the superseding run exceeds gives `unresolved (incomplete)`; a case whose prior run exceeds its work budget gives `unresolved (incomplete)`, not `not applicable`; a pair whose prior compile refuses `unsupported_construct`/`not-yet-implemented` gives `unresolved (unsupported)`, not `tool failure`. None gives `holds`."
    },
    {
      "id": "FR-404-AC-6",
      "path": "spec/functional/FR-404-carry-the-call-s-plugin-replay-context-through-the-replay-facade.md",
      "role": "examined",
      "excerpt": "Over R, `q` with `x = 3` under `CallerLimits` whose `s1.tokens` is 200000, whose `library.definitions` is 9000 and whose accounting `work_units` is 5000 completes, and the request for its member holds `s1.tokens` 200000 and `library.definitions` 9000 in `stage_limits` and `work_units` 5000 in `accounting_limits`. The same call under `CallerLimits` whose `s1.tokens` is 1 refuses `Compile` at the S1 token limit with no context, while `run_with_requirements` over the same inputs with `SpineLimits::default()` completes."
    },
    {
      "id": "qsl-eval/src/smt/mod.rs",
      "path": "qsl-eval/src/smt/mod.rs",
      "role": "examined",
      "excerpt": "//! - `query` assembles a script for an item and a [`SmtQueryKind`]."
    }
  ],
  "findings": [
    {
      "fnd": "FND-001",
      "severity": "medium",
      "confidence": "high",
      "method": "gap-analysis",
      "check": "test-intent",
      "artifact_id": "FR-344-AC-4",
      "related_ids": [
        "FR-255-AC-7"
      ],
      "path": "xtask/tests/refinement_versioning.rs",
      "lines": "733",
      "excerpt": "A case whose superseding run exceeds its `accounting` work budget names, in its entry, `work_units`, the budget's value and the member `accounting`.",
      "finding": "The changed refinement report oracle now expects name=accounting.work_units with member=accounting, but FR-344-AC-4 requires the bare work_units counter. limit_reports forwards ReachedLimit.name unchanged (report.rs:279), while Limits::of accepts only bare counter keys under case.accounting (versioning.rs:536). Consequently the report's name/member pair cannot be applied to the named case member. Preserve the bare counter in this report, or explicitly revise its contract and distinguish a setting name from the case-member key."
    }
  ],
  "bindings": [
    {
      "test_id": "accounting_names_are_prefixed_counters_in_neither_stage_table_nor_bare",
      "ac_id": "FR-255-AC-3",
      "trace": "correct"
    },
    {
      "test_id": "each_limits_field_maps_to_one_distinct_setting_and_sets_only_itself",
      "ac_id": "FR-255-AC-3",
      "trace": "correct"
    },
    {
      "test_id": "a_reached_budget_set_by_operand_names_its_setting",
      "ac_id": "FR-255-AC-7",
      "trace": "correct"
    },
    {
      "test_id": "each_accounting_setting_sets_its_counter_alone",
      "ac_id": "FR-255-AC-8",
      "trace": "correct"
    },
    {
      "test_id": "a_malformed_accounting_operand_is_a_usage_refusal_naming_it",
      "ac_id": "FR-255-AC-9",
      "trace": "correct"
    },
    {
      "test_id": "tc_921_a_stage_limits_counter_entry_sets_the_budget",
      "ac_id": "FR-255-AC-10",
      "trace": "correct"
    },
    {
      "test_id": "tc_921_a_split_accounting_limit_gives_the_same_claim_settlement_and_charges",
      "ac_id": "FR-255-AC-11",
      "trace": "correct"
    },
    {
      "test_id": "a_stated_budget_reaches_both_runs_and_a_stated_nothing_uses_defaults",
      "ac_id": "FR-340-AC-4",
      "trace": "correct"
    }
  ]
}
```

