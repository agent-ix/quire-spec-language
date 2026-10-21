---
id: SR-12271
title: "Code review of quire-spec-language PR #1156: own precondition clauses conjoined in ascending name with short-circuit (QSL-935)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@707401ee7eecfcb844c89593d2d7d3639948b9e1; PR #1156 diff against origin/main: qsl-eval/src/value/expression/clause_set.rs; qsl-eval/src/value/expression/mod.rs; qsl-eval/tests/it/model_simulation.rs; qsl-replay/src/spine/clause/tests/clause_set.rs; qsl-replay/tests/dispatch_link_compile.rs; qsl-semantics/src/check/checked_dispatch.rs; qsl-semantics/src/check/dispatch_link.rs; spec/functional/FR-107-evaluate-state-clauses-at-s6a.md; spec/functional/FR-120-simulate-a-checked-package-s-state-family.md"
review_set: subset
---
# Code review of quire-spec-language PR #1156

## Summary

Ticket: QSL-935. One code-review pass with the rust-review lane folded in,
over `git diff origin/main...HEAD` (9 files, +180/-41, one commit). QSpec
FR-151 (AC-17 to AC-19) and TC-196 D09 to D11 were read at QSpec's current
main and are cited by id. One focused run:
`cargo test -p qsl-eval --test it model_simulation`.

Checked and clean:

- `clause_set::run_group` with `short_circuit` returns `Ok(false)` on the
  first precondition whose verdict is not `True`, including when that clause
  is the last one, so postconditions run only when every precondition was
  evaluated and is `Completed(true)`. Each clause keeps its own fresh meter,
  so a clause that is not reached has no entry and no charge. Invariants and
  postconditions still run with `short_circuit` false and keep the
  set-of-outcomes rule; an `Incomplete` clause still ends any group. The
  removed `preconditions_true` scan is subsumed by the early return.
- The simulator caller `contracts::conjunction` (used by `frames.rs` for
  enabling and by `frames/judge.rs` for FR-141) needs no change: a false
  first clause gives `ClauseSetOutcome::False` (not enabled, no finding); an
  undefined or refused first clause gives `Undecided` with exactly one entry,
  so one `ContractUndetermined` with `candidate` none. `judge.rs` shares the
  same `conjunction`, so FR-141's "the two never disagree" holds.
- Other callers of the old behaviour: `evaluate_clause_set` is called only by
  `contracts.rs`, the replay clause-set test and `tc235_business_meaning_runtime.rs`
  (one precondition, unaffected). `view_clauses` has one caller
  (`link_dispatch`). No qsl-route or spine production path combines
  precondition clauses itself.
- `view_clauses` sorts `StateClauseDeclaration::name` (`String`) with
  `str::cmp`, which is UTF-8 byte order, not locale or `char` collation. The
  conjunction is built left-nested with `BinaryOperator::And`, so FR-006's
  short-circuit decides which clause is reached.
- The `checked_dispatch.rs` module doc now states `own(m) or eff(r)` with
  `own(m)` in ascending clause name, matching QSpec FR-151's statement.
- Spec edits: the FR-107 bullet, FR-107-AC-14 (3 to 4 now gives only
  `UpdateAllowed` `Completed(false)`), and FR-120-AC-9 (`pre Aaa { false }`)
  match the code and QSpec FR-151-AC-19.
- The changed existing test `an_undecided_precondition_is_a_contract_finding`
  (`Never` renamed `Aaa`) still tests its claim: a false clause that sorts
  before the undefined one hides it. The case it used to cover (false sorts
  after undefined) now gives a finding and is asserted by the new test's
  second loop.
- Test oracles: the new simulator test's second loop fails on the old
  set-of-outcomes code (a later false clause gave `False` and no finding).
  It also proves the later clause is not evaluated: if it were, `combine`
  would give `False` and no finding. The first loop also passes on the old
  code; for the false-first case, non-evaluation is proven at the clause-set
  level by the FR-107-AC-14 replay test (no `UpdateNotMax` entry). Charges
  are not observable in the simulator, and none exist to assert, because
  each clause has its own meter. The dispatch test compares outcome and
  work-unit charges across the two source orders. Without the sort, the
  swapped order would evaluate the undefined clause first, so its outcome
  would name `other` instead of `size` and the test fails.
- Rust lane: there is no `unsafe`, and production code has no new `unwrap`,
  `expect`, index or integer conversion. The `panic!`s are in tests.

## Verdict

PASS with fixes in-PR. There is no major finding. The behaviour change is
correct and minimal on both paths. Two trace and AC wording defects need
fixing in this PR: FND-001 (medium) and FND-002 (low).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The dispatch-path test carries `#[trace("TC-196", "QSpec-FR-151-AC-19", "FR-120-AC-9")]`. A bare `TC-196` binds QSL's own TC-196, `duplicate-backend-registration-refused` (FR-075), which is unrelated. The repo convention for QSpec's case is `QSpec-TC-196`, as in `dispatch_calls.rs` and `inherited_attributes.rs`. `FR-120-AC-9` is the simulator criterion over the `test/orders` fixture, and this test does not exercise it. The matrix therefore credits two QSL criteria that this test does not check. The fix is `#[trace("QSpec-TC-196", "QSpec-FR-151-AC-19")]`, plus a QSL criterion for the dispatch-path order if one is added (see FND-002). | qsl-replay/tests/dispatch_link_compile.rs:1225 |
| FND-002 | low | FR-120-AC-9 now names only the false-sorts-first case (`pre Aaa { false }` records no finding). FR-120's statement also gives the reverse: an undefined clause that sorts before a false one gives the undecided outcome and one finding. The new test asserts that case in both source orders, but no criterion states it, and no QSL criterion states the dispatch path's name order. The fix is to add both to AC-9 (or a new AC): the undefined clause sorting first records its one finding and the false clause after it is not reached, and the dispatch-path conjunction follows clause name whatever the source order. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:572 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The dispatch-path test is tagged `QSpec-TC-196`, `QSpec-FR-151-AC-19` and the new `FR-120-AC-15`; it no longer carries bare `TC-196` or `FR-120-AC-9`. |
| FND-002 | fixed | New FR-120-AC-15 states the reverse order (undefined clause first gives one finding, the false clause is not reached) and the dispatch path's name order in both source orders; the simulator test is tagged to it. |
