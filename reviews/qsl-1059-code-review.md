---
id: SR-17054
title: "Code review of PR #1486: ModelCheckLimit and statistical limit names in snake_case (QSL-1059), code-review with rust-review lane"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@99c6297208619ecedaedff5becabddbd98292d0b; git diff origin/main...99c629720 (PR #1486, branch qsl-1059-limit-snake-case): qsl-replay/src/model_check/limits.rs, qsl-replay/src/statistical.rs, qsl-replay/src/statistical/settle.rs, qsl-replay/src/hybrid.rs; context qsl-replay/src/outcome.rs, qsl-replay/src/statistical/witness.rs, qsl-replay/src/proof_result/plugin_read.rs, qsl-replay/src/limits.rs, qsl-analyze/tests/it/model_settle.rs"
review_set: subset
date: '2026-10-25'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-250
    type: reviews
---
## Summary

Ticket: QSL-1059. PR: quire-spec-language#1486. The rust-review lane is folded
into this file. The diff renames every kebab `ModelCheckLimit::as_str()` and
`StatisticalLimit::as_str()` spelling to snake_case, updates two pinned tests
and two doc comments, and adds `every_limit_spells_its_name_in_snake_case`.

**Spellings against the spec.** Each new name was checked against FR-255's
engine table (spec/functional/FR-255-...md:182-214), the setting `setting()`
returns for it, and the source FR:

- `max_states`, `max_transitions`, `max_automaton_states`: FR-255 rows 182-184
  (and the EN-5 rows 193-195, already snake). Match.
- `witness_samples` (not `max_witness_samples`): FR-255 row 185, FR-166,
  ADR-022 GE-2/QS-7 all name the member `witness_samples`. Match.
- `max_walk_steps`, `max_witness_set`, `max_relation_tuples`: rows 186-188,
  FR-166, FR-184. Match.
- `SolverSteps -> max_solver_steps` (not `solver_steps`): FR-255 row 214
  (`hybrid.max_solver_steps` / `max_solver_steps`), FR-250:246 and FR-250-AC-3
  (`LimitReached{limit: max_solver_steps, value: 1000, setting:
  hybrid.max_solver_steps}`), QSpec FR-193:359. `solver_steps` appears nowhere
  in either spec. Match. (FR-182-AC-14 and FR-255-AC-14 name `max_solver_steps`
  for the SMT engine's `smt.max_solver_steps`; no `ModelCheckLimit` variant
  exists for it yet, so nothing in this diff applies.)
- `max_samples`, `max_draws`, `max_cycle_steps`: rows 202-204, FR-191:40-42,
  ADR-024 ST-7/ST-8, FR-190-AC-2. Match.
- The meter names and `max_candidates` were already snake. Every
  `(as_str, setting)` pair now equals its engine-table row.

**Kebab strings left alone.** `CatalogCode::new("resource_exhausted",
"max-automaton-states")` (statistical/witness.rs:165) is a catalog reason, not
a limit name; every catalog reason in the repo is kebab (`caller-cancelled`,
`not-markov`, `intake-limit-exceeded`), and no spec gives it a snake spelling.
`solver-inconclusive`, `bounded-solver`, `limit-reached` are cause and basis
kinds, which FR-331 spells in kebab. Correctly unchanged. No variant that
should have changed was missed: `git grep` over the whole tree for every old
kebab limit string finds only that catalog reason and historical
`reviews/*.md` text.

**Consumers.** `as_str()` reaches the wire at outcome.rs:605 (the `limit`
member next to `setting`), the only producer; no reader in qsl-replay,
qsl-analyze, qsl-semantics, qsl-package, qsl-route, xtask, tests/ or schemas
parses a `limit` string back to `ModelCheckLimit` (the only `limit` string
matchers, plugin_read.rs:694/706, read the FR-331 plugin method bounds
`max_store_buffer`/`max_messages`/`max_live_instances`, already snake).
`schemas/*.json`, golden JSON and conformance vectors pin no limit name. Only
qsl-replay and qsl-analyze reference `ModelCheckLimit`; qsl-semantics'
`limit.as_str()` (model/intake.rs:416) and request.rs:470 are `IntakeLimit`,
unrelated. qsl-analyze's `a_meter_or_candidate_stop_settles_naming_its_setting`
asserts `limit.as_str()` only for already-snake names.

**Other repos.** quire-contract-codegen, quire-contract-ir,
quire-contract-runtime, quire-exact, quire-verification(-contracts),
quire-integration, quire-analyze, quire-corpus, quire-driver, quire-protocol:
no code use of any old kebab limit string (contract-runtime mentions
`LimitReached` only in reviews/). quire-specification's backend-provider-v1
schema already requires `LimitName` `^[a-z][a-z0-9_]*$` (schema.json:78, 724),
so the old spellings were schema-invalid on that wire and the change aligns
QSL with it; its `temporal-limits-solver-steps-zero` hits are vector names.
No follow-up needed downstream.

**New test.** Not a tautology: every expected value is a literal, never
derived from `as_str()`. Reviewer mutations in an exported copy of the head
tree, each with `cargo test -p qsl-replay --lib
every_limit_spells_its_name_in_snake_case`: `WitnessSamples ->
"witness-samples"` killed (left/right mismatch); `MaxDraws -> "max_samples"`
(a snake but wrong name, which the pattern half alone would pass) killed. The
coder's `solver-steps` mutation is also reported killed. Its weakness is
exhaustiveness (FND-001) and its missing tag (FND-002).

**Gates run by the reviewer** at 99c629720 with a private CARGO_TARGET_DIR,
each under `flock /tmp/qsl-cargo.lock`: `cargo test -p qsl-replay --lib` 853
passed; `cargo test -p qsl-analyze` 78 + 765 passed, 0 failed; `cargo clippy
-p qsl-replay --all-targets -- -D warnings` clean. qsl-route and qsl-semantics
do not reference `ModelCheckLimit` or `StatisticalLimit`, so their suites were
not run.

## Verdict

Approve. Every spelling matches FR-255's engine table and its source FRs,
`max_solver_steps` included, no consumer of the old strings exists in this
repo or downstream, cause and catalog spellings were correctly kept kebab, and
the affected suites pass. Two low findings on the new test, neither blocking.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `every_limit_spells_its_name_in_snake_case` lists its rows by hand, so it is not exhaustive: a new `ModelCheckLimit` or `StatisticalLimit` variant (or `LimitKind` counter) added later with a kebab or wrong name compiles and passes, because nothing forces it into `rows`/`meters`. `as_str()`'s own `match` is exhaustive, the test's table is not. Fix: build the row list through a helper with an exhaustive `match` on the variant (no `_` arm) that returns the expected name, so adding a variant fails to compile until the test names it. | qsl-replay/src/model_check/limits.rs:274-336 |
| FND-002 | low | The test carries no `#[trace]`, although it is the limit-name half of FR-255-AC-12 ("Every limit name of the engine table matches `^[a-z][a-z0-9_]*$` ... its mapping from each limit member to (limit name, setting name) equals its engine-table rows") and TC-1161 step 2 for `ModelCheckLimit`. The PR body declines the tag because the test does not assert every AC-12 clause, but the repo already tags the settings half of AC-12 that way (`the_settings_operation_and_a_request_take_every_setting`, limits.rs:513, `#[trace("TC-721", "FR-255-AC-4", "FR-255-AC-12", "FR-255-AC-13")]`), so the matrix credits AC-12 to a test that checks no limit name while the one that does is invisible. Fix: tag it `#[trace("TC-1161", "FR-255-AC-12")]` with a doc line naming the clause it covers (the limit name of each `ModelCheckLimit` variant and its pattern; adding a `setting()` assertion per row would cover the pair as well), and give it a doc comment like its neighbours. | qsl-replay/src/model_check/limits.rs:274-275 |
