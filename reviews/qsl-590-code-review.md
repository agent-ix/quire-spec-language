---
id: SR-1250
title: "Code review of quire-spec-language PR #610: one exit function and severity fold (LC2)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@8464165c04450cc813f10637fdcdf80d42e94466; PR #610 diff against origin/main: qsl-foundation/src/diagnostic.rs, qsl-foundation/src/diagnostic/stage.rs, qsl-eval/src/value/expression/mod.rs, qsl-replay/src/{lib,proof_result,result,execute}.rs, qsl-replay/src/result/wire.rs, qsl-replay/src/execute/{frame,state_clause}.rs, qsl-replay/src/spine/{call,clause}.rs and their tests, src/{main,command}.rs, src/command/output.rs, tests/it/{spine_run,config_version_spine,native_boundaries,standalone}.rs"
review_set: subset
---
# Code review of quire-spec-language PR #610

## Summary

Ticket: QSL-590 (LC2). The PR makes `Category::exit_code` the one exit map,
adds `Category::most_severe` and `Category::trace_exit_code`, folds
`ProofCategory` into `Category`, adds `category()` to the outcome and failure
types, deletes every per-type `exit_code` and the output.rs helpers, and moves
undefined from exit 20 to exit 10.

Team-leader positions checked:
- **Admission exits (position 1): not confirmed.** `ClauseDisposition::category`
  maps every `Admit(AdmissionFailure::Refused(_))` to `Refusal`. On main the
  exit came from `record.code.exit_code()`. FR-106 check 6.2 refuses a
  set, bag or ordered-set field with `unknown_required_feature`/`unsupported-feature`
  (qsl-semantics/src/model/observation/document.rs:1003, :1142). That code is
  `is_unsupported`, so it exited 21 on main and exits 20 here. The deleted test
  `exit_code_maps_every_admission_record_code_to_its_own_exit_status` asserted
  the 21. The new test `admission_failure_exits_by_its_category` asserts 20
  for every code. This is a real output change (FND-001).
- **`trace_exit_code` (position 2): warranted.** FR-285's Behavior has its own
  bullet for the pending supplied-trace clause (0) and AC-1 has a row for it.
  `tc_769_exit_function_maps_every_table_row` asserts it, and asserts that it
  equals `exit_code` for every other category. It is small and has no
  production caller until FR-283 lands; that is acceptable here.
- **`VerdictWire` gains `undefined` (position 4).** The only writer and reader
  are `VerdictWire::of` and `VerdictWire::read` in wire.rs. No JSON fixture
  and no canonical identity digest covers this wire, so no fixture or digest
  changes. No production path builds `Verdict(Category::Undefined)`: all three
  replay paths map an undefined evaluation to `Inconclusive`. The reader now
  accepts a label nothing writes (FND-003).
- Every frontend exit in src/main.rs and src/command/output.rs now goes
  through `Category::exit_code` or `Category::most_severe`. No numeric exit
  literal is left, apart from the `RunResult.exit_code` field that holds the
  computed value.
- The spine-run, `RunCause`, compile and argument-refusal exits are the same
  as on main, apart from undefined (20 to 10). `RunRefusal::Fault` still exits
  30 through `Code::RuntimeInvariant`'s category.
- Rust lane (rust-review): no new panic, `unwrap` or `unsafe` on a production
  path. `Category::exit_code` and `trace_exit_code` are `const` exhaustive
  matches. Clippy `-D warnings` is clean on qsl-foundation, qsl-eval and
  qsl-replay at this head. The `tc_769` tests and
  `admission_failure_exits_by_its_category` pass with a worktree-local target
  dir.
- Test oracles: `tc_786_run_exit_statuses_come_from_the_exit_function` runs
  the real binary and `qsl_replay::spine::run` on the same input, and checks
  that both give the same code. That is a strong oracle. The `most_severe` test
  checks each set in both orders.

## Verdict

Changes requested: one high, one medium and one low finding. The
category-type fold, the deletion of the old helpers and the undefined-to-10
change are correct.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | An admission refusal now exits by the `AdmissionFailure` variant, not the record's code. A set, bag or ordered-set field (`unknown_required_feature`/`unsupported-feature`, FR-106 check 6.2) exited 21 on main and exits 20 here. Compile and argument refusals with the same code still exit 21. This goes against FR-285's unsupported row (21) and QSpec FR-301. FR-109's new wording, "FR-106's own split ... (refused 20, incomplete 22)", writes the change into the spec, but FR-106 assigns no exit or category. Fix: map `Admit(Refused(record))` to `record.code.category()` and keep `Admit(Incomplete(_))` as `Incomplete`. Reword FR-109 to say so. Change `admission_failure_exits_by_its_category` to expect 21 for `unknown_required_feature`. | qsl-replay/src/spine/clause.rs:339; qsl-replay/src/spine/clause/tests.rs (admission_failure_exits_by_its_category); spec/functional/FR-109-run-a-state-clause-through-the-spine.md:143-150 |
| FND-002 | medium | Two functions now map a catalog code to a `Category`, and they disagree. `Code::category` is documented as "this code's ADR-013 O-16 category" and gives `Unsupported` for `unsupported_construct`, `unknown_required_feature` and `unsupported_projection`, and `Incomplete` for `resource_exhausted`, `incomplete_population` and `unavailable_observation`. `catalog_category`/`category_of` give `Refusal` for all six. The `CATALOG_CATEGORIES` doc still says `is_incomplete`/`is_unsupported` are "the native-v1 exit-code ladder (FR-301), not the O-16 category". This conflicts with FR-285's "one category". Fix: reconcile the two maps, or say which map is the category and which is only the exit category, and update the `CATALOG_CATEGORIES` doc to match. | qsl-foundation/src/diagnostic.rs:306-320; qsl-foundation/src/diagnostic.rs:857-869 |
| FND-003 | low | `VerdictWire` restates `Category`'s kebab-case labels (a duplicate of `Category::as_str`). It now reads `undefined` into a proof or replay `Verdict`, which no writer produces and which FR-285 says no proof item carries. The new variant has no test. Fix: refuse `undefined` when reading a verdict, or derive the wire from `Category::as_str` and add a round-trip test. | qsl-replay/src/result/wire.rs:194-234 |

## New findings (disposition pass 1)

Found at e4f2770ceef6ddfaed0c9b087f4028723026a5b9.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | `ClauseDisposition::category` now returns `Unsupported` for a compile, argument, admission or evaluate refusal whose code is unsupported. `unknown_required_feature` at admission is the FND-001 case. On main every one of these reported `refusal`. FR-109 still lists the disposition categories as "`success`, `violation`, `refusal`, `incomplete` or `internal-failure`", and its evaluate bullet still says "category `refusal` or `incomplete`". So the code reports a category that FR-109 rules out. The exits are correct (21). Fix: add `unsupported` to both lists in FR-109. This is a spec-only edit. | qsl-replay/src/spine/clause.rs:331-346; spec/functional/FR-109-run-a-state-clause-through-the-spine.md:74-76, 133-136 |

## New findings (disposition pass 2)

Found at db2fb6d116502873bb9fb1718a0ddb9e2758bee7.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | FR-109's undefined bullet now says category `undefined`, exit 10, which matches FR-285 and the code. But it still cites "(ADR-018 RU-5, QSpec FR-301)", and both sources say the opposite about the category. RU-5 settles an undefined claim as refuted (violation). QSpec FR-301 files "a claim whose evaluation is undefined" under violation, and gives `undefined` only to a value run. The citation supports only the exit 10. QSpec FR-301 now contradicts FR-109 on the clause-run label; before this round, the two agreed. The root of this is FR-285, which predates the PR and already names a clause run as undefined. Fix: cite FR-285 for the category, keep RU-5/FR-301 only for exit 10, and route the QSpec FR-301 clause-run label to its QSpec half (STD-141). | spec/functional/FR-109-run-a-state-clause-through-the-spine.md:129-132 |

## Dispositions

Round 1, reviewed at e4f2770ceef6ddfaed0c9b087f4028723026a5b9 (fix commits bc464d1d6 and e4f2770ce).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | bc464d1d6: `Admit(AdmissionFailure::Refused(record)) => record.code.category()`, so `unknown_required_feature` exits 21 again. `admission_failure_exits_by_its_record_code_category` asserts 21, 20, the code's own category for every `Code`, and 22 for every `Incomplete`. It passes at this head. FR-109's exit bullet now says the admit refusal record takes the category of its code. |
| FND-002 | fixed | bc464d1d6: the `Code::category` doc names it as the only code-to-category map an exit comes from. The `CATALOG_CATEGORIES`/`category_of` docs call that table the refusal-record map, never an exit source, and name the six codes where the two differ. No exit path in the repo reads `category_of`; its only production caller is `explore::Outcome::category`, and nothing takes an exit from that. |
| FND-003 | fixed | bc464d1d6: `VerdictWire(Category)` writes `Category::as_str` and refuses `undefined` and unknown labels on read. `verdict_labels_round_trip_and_undefined_is_refused` passes at this head. |


Round 2, reviewed at db2fb6d116502873bb9fb1718a0ddb9e2758bee7 (fix commit db2fb6d11).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-004 | fixed | db2fb6d11: FR-109's disposition category list now reads "`success`, `violation`, `undefined`, `refusal`, `unsupported`, `incomplete` or `internal-failure`". The per-category record list covers "an `evaluate` refusal, unsupported or incomplete", and the evaluate mapping says "category `refusal`, `unsupported` or `incomplete`". |

Round 3, reviewed at db8b11c19bc99fb74f328270a81f71c7fb0fd24a (fix commit db8b11c19).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-005 | fixed | db8b11c19: FR-109's undefined bullet now reads "category `undefined` (FR-285) with `UndefinedEvaluation{cause}`, and exit 10 (ADR-018 RU-5, QSpec FR-301)". FR-285 backs the category, and RU-5 and FR-301 back only the exit. QSpec FR-301's clause-run label is left to STD-141 in Linear. |
