---
id: SR-1307
title: "Code review of quire-spec-language PR #636: stage limits are incomplete, execute's limit outcome carries FR-277 fields"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@6014d82773ad72641e9934b6d62c2963f7b870a7; git diff origin/main...HEAD (PR #636, base c8f0c2818): qsl-foundation/src/diagnostic.rs, qsl-replay/src/spine.rs, qsl-replay/src/spine/call.rs, qsl-replay/src/spine/call/tests.rs, qsl-replay/src/spine/clause/tests.rs, qsl-replay/src/execute/{frame,state_clause}.rs, src/command/output.rs, tests/it/{cli,composed_namespace,config_version_spine,parser,source_map}.rs, Cargo.lock"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-285
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-277
    type: reviews
---
# Code review of quire-spec-language PR #636

## Summary

Ticket: QSL-489 (LC1), reopened for the defect IR-608 reported. PR: quire-spec-language#636.
Rust lane (rust-review) folded in.

Checked and clean:
- The category fix is at the single source. `Code::is_incomplete` gains
  `StageLimitExceeded`; `Code::category` reads it; `RunRefusal::category`,
  `CallRefusal::category`, the CLI `status` (src/main.rs:52,
  src/command/output.rs:317) and every `is_incomplete()` wrapper read
  `Code`. No second exit or status map for stage limits was left behind.
- Nothing that should stay a refusal moved. B-1 authored bounds keep
  `cardinality_out_of_bound` (refusal); B-2 read-only ceilings keep
  `resource_exhausted` (unchanged by this PR); `resource_exhausted` stays
  separate from `stage_limit_exceeded` by the limits type (ADR-014 §1).
  Every producer of `stage_limit_exceeded` (S1 `SyntaxLimit`, `CheckingLimits`,
  type-environment limits, intake, the I2 and replay readers,
  `CompileRefusal::Limit`) is a B-3 stage limit, which ADR-029 CB-4 and
  FR-010 put at incomplete (22).
- The five integration tests moved from 20/refused to 22/incomplete
  (cli.rs, composed_namespace.rs, parser.rs x3, source_map.rs x2) agree with
  ADR-029 CB-4 and FR-010 ("An exhausted parser request is incomplete and
  exits 22").
- `CallIncomplete` carries the kernel `Incomplete` record (limit kind,
  configured `limit`, `consumed`, denied `next_charge`, charge point), an
  `Option<Location>`, `limits_field()` and `counter()`. That covers FR-277
  Outputs. `run` never shares its `Cancel`, so a kernel cancellation record
  (`limit == consumed`) cannot be misread as a work_units limit here.
- `counter()` is consumed + denied charge for the cumulative kinds
  (`WorkUnits`, `ResultUnits`) and the denied size for high-water kinds,
  matching quire-exact's `next_charge` doc ("a size for high-water
  counters, an addition for cumulative counters").
- Cargo.lock: one `quire-exact` package, moved 1098e44e -> a270df3c. That
  range is one commit (quire-exact #3) that adds spec files only, no code.
  `quire-semantic-value` moves to 660a126f, which has `with_nodes`.
- `CheckingLimits` is re-exported from `qsl_replay::spine`; the test changes
  `s3.nodes` alone and checks the other two fields keep their defaults.
- Focused run: `cargo test -p qsl-replay --lib -- spine::call::tests
  spine::clause::tests::run_clause_reports_incomplete`: 19 passed. make ci
  exit 0 per the supplied log.

## Verdict

Approve. The fix is correct and sits at the single source. Three low findings,
none blocking: a test oracle that restates its implementation, and two
judgement calls about codes that the single-source change now also moves.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The FR-277-AC-3 test's last assertion `counter() == consumed + next_charge` restates `CallIncomplete::counter`'s cumulative branch, so it cannot fail independently. A stronger external oracle exists: `needed` is the least passing bound, so the denied charge's counter is exactly `needed` (it is > needed-1 and <= the total under bound `needed`). Assert `incomplete.counter() == Integer::from(needed)`. The high-water branch of `counter()` (returns `next_charge`) has no test. | qsl-replay/src/spine/call/tests.rs:279-313; qsl-replay/src/spine/call.rs:188-196 |
| FND-002 | low | `ReplayRefusal::LimitAboveReader` (the request's own S1 `text_input_bytes` is above the reader limit) codes `stage_limit_exceeded`. With this change its category is incomplete (22). No limit was reached, and raising one cannot help: the request asks for more than the reader permits, which reads as invalid input (20). No exit is derived from `ReplayRefusal` yet, so nothing is wrong today. Either give it a request code, or have ADR-013 O-26 state that it is incomplete. | qsl-replay/src/execute.rs:83-89; qsl-replay/src/execute.rs:239 |
| FND-003 | low | The refusal-record map `CATALOG_CATEGORIES` still gives `stage_limit_exceeded` category refusal, and `limit_exceeded_reports_stage_limit_exceeded_per_kind` pins that. The same table puts `cancelled` at incomplete because O-16's incomplete row names it, and ADR-029 CB-4 now names `StageFailure::Limit` there too. Nothing reads the record map for a stage limit today, but a refusal record for a stage limit would report refusal beside exit 22. | qsl-foundation/src/diagnostic.rs:925; qsl-foundation/src/diagnostic/stage.rs:337 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | `CATALOG_CATEGORIES` gives `incomplete_population` and `unavailable_observation` category refusal. The spec has no refusal-record case for either code: FR-106 admission returns `Incomplete` with them, ADR-012's admission table and ADR-016 SC-5 call both incomplete, and FR-109-AC-3 exits 22. The map's doc justifies the refusal row for `resource_exhausted` only (ADR-014 B-2: refusal record versus stopped outcome), which is spec-intended. It says nothing for these two. `explore::stop_category` reads this map for every stop cause other than `resource_exhausted`, so an expansion stopped with either code would report refusal instead of incomplete. Give both rows `Category::Incomplete`, and pin them in the category test. This predates the PR, but it is the same map this PR edits for the same reason. | qsl-foundation/src/diagnostic.rs:917-918; qsl-eval/src/simulation/explore.rs:178-184 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | a04276c53700556ae1699ec78b947a01d0e33d5b |
| FND-002 | fixed | a04276c53700556ae1699ec78b947a01d0e33d5b |
| FND-003 | fixed | a04276c53700556ae1699ec78b947a01d0e33d5b |
| FND-004 | fixed | 48d4ddd59bcb4036db7ffc6857743ea48760f7e7 |
