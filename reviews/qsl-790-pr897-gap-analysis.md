---
id: SR-11702
title: "Gap analysis of quire-spec-language PR #897: clause-set results and per-read runtime value paths"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@204cdaeee9999870e30d24ce7d672f56d0ba3032; qsl-replay/src/spine/clause/tests/clause_set.rs, qsl-replay/src/spine/clause/tests/clause_paths.rs, qsl-replay/src/spine/clause/tests/witness.rs, qsl-replay/src/result/wire.rs, qsl-eval/tests/it/tc235_business_meaning_runtime.rs, qsl-eval/src/value/expression/clause_set.rs, spec/functional/FR-107-evaluate-state-clauses-at-s6a.md, spec/test-cases/TC-1039-a-clause-set-reports-its-results-together.md, spec/test-cases/TC-1040-a-clause-evaluation-reports-its-receiver-and-read-paths.md"
review_set: subset
---
# Gap analysis of quire-spec-language PR #897

## Summary

Ticket: QSL-790. Manual check of acceptance criteria against tests over
`git diff origin/main...HEAD`. No builds were run and `quoin matrix` was not
run. Plan completion: not assessed. The ticket text was read as data.

Examined: FR-107-AC-7 to AC-18, the steps of TC-1039 and TC-1040, and the
TC-235 M16 run 3 and M22 tests in QSL. The QSpec TC-235 expectations were
checked against the brief and cited by id only.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-107-AC-16 is tagged as covered, but its duplicate-clause conjunct is not built, and the code does the opposite. `a_set_naming_an_unknown_clause_or_mismatched_observations_is_refused_whole` carries `#[trace("TC-1039", "FR-107-AC-16")]` and asserts only the `UnknownClause` and `ObservationsMismatch` refusals. Its doc comment admits the duplicate test is "in a separate test", which is not in this PR. So the matrix reports AC-16 as covered. Meanwhile `clause_set::evaluate` has no duplicate check: a slice naming `UpdateAllowed` twice evaluates it twice and returns two entries. FR-107 "Clause sets" says that set SHALL be refused `DuplicateClause` with no entry. `InputRefusal::DuplicateClause` does not exist on quire-semantic-value main (`ad6e975`), so the hold-out is a real cross-repo blocker. The tag still overclaims, and the code still gives a wrong result for that input. Fix: keep the behaviour in this PR once the QSV variant lands, or drop the AC-16 tag until the duplicate test is in, and keep QSL-790 open for it. | qsl-replay/src/spine/clause/tests/clause_set.rs:276-305, qsl-eval/src/value/expression/clause_set.rs:111-130 |

## Verdict

FAIL: 1 blocking finding (FND-001, a tag overclaim over unbuilt behaviour). See
also SR-11701 FND-002: FR-107-AC-17 / TC-1040 step 9 is buildable now that
FR-364 (#892) has merged, and it is in this ticket's per-read-path outcome.

Recorded, not blocking:
- medium: `tc235_m16_run3_a_clause_set_reports_one_result_for_a_false_precondition`
  is tagged `FR-107-AC-13` and `FR-107-AC-14`, but it uses the `Fulfilment`
  fixture. It asserts nothing about `UpdateNotMax` or an invariant in the set,
  which is AC-14's content. Both ACs are fully covered in `clause_set.rs`, so
  no coverage is invented. Its true tag is `QSpec-TC-235`.
- medium: the AC-15 zero-budget assertion checks that the result is
  `Incomplete`, but not the `resource_exhausted`/`insufficient-next-charge`
  code that TC-1039 step 4 states.

Rows not built, judged:
- TC-1039 step 6 / FR-107-AC-18 is a spec defect: the row cannot be built as
  written. It is not a missing evaluator feature. FR-091 says `/` never
  denotes integer division, and integer-operand promotion for rational `/`
  exists only under `quire.value.complete/v1`. So
  `100 / self.versionNumber > 0` on the `Config` unit is refused
  `AmbiguousLiteral`, which agrees with the author. The repo also records
  that a linked state clause must prove divisors nonzero (`witness.rs`
  `tc_744_*`), so no division-by-zero `Undefined` is reachable. The sibling
  row FR-109-AC-7 (`6 / (..) >= 0`) has the same defect and is also
  unbuilt. The `Undecided` path itself is generic: `combine` is
  unit-tested. AC-18 needs a spec amendment to a clause that checks and
  evaluates to a non-Boolean (for example a `Refused` arithmetic-domain
  clause), routed as a spec ticket.
- TC-1039 `UpdateRaises` is written as `self.versionNumber -
  pre(self.versionNumber) = 1`, because `= pre(..) + 1` is refused
  `TypeMismatch` between `Int[0,1000]` and `Int[1,1001]`. Over exact integers
  the two forms are equivalent (a = b + 1 iff a - b = 1), and TC-1039 asserts
  only outcomes. Accepted. The TC text should be amended to the form that
  checks.
- TC-1039 step 5, the duplicate refusal: see FND-001.

Clean (each test asserts what its row states):
- FR-107-AC-13 / TC-1039 step 1: over `up12`, two entries, both true,
  outcome `True`. Over `up34`, exactly one entry, `UpdateAllowed` false,
  outcome `False`. Reversing the slice changes nothing.
- FR-107-AC-14 / TC-1039 steps 2 and 3: the order is `UpdateAllowed`,
  `UpdateNotMax`, `UpdateRaises`. With `NoCycle`, the invariant comes first.
  `UpdateRaises` has no entry after a false precondition.
- FR-107-AC-15 / TC-1039 step 4: a zero budget gives one `Incomplete` entry.
  An empty set is `True`. Two runs give equal entries.
- FR-107-AC-7 to AC-10 / TC-1040 steps 1 to 4: the receiver paths, the four
  `ParentOrder` reads in order with their exact paths, `implies` skipping its
  right operand, a zero budget giving no reads, `VersionUnchanged` post then
  pre, the `ProbeOrdered` parameter binding with `[reference b, member
  versionNumber]`, and no path under `PreCall`.
- FR-107-AC-11 / TC-1040 steps 6 to 8 (M22): the receiver binding with
  `[field placed]` and `[field cancelled]`, the parameter binding with
  `[field placed]`, unequal and with the parameter ordered first, the
  `PlaceRequires` paths at `p1`, and none under `PreCall`. The
  invocation-binding wire forms are on main (`tc235_m22_*` in `wire.rs`).
- FR-107-AC-12 / TC-1040 step 5: the built-list subject with `[index 0,
  reference root, member versionNumber]` and `[index 1, reference mid,
  member versionNumber]`.
- TC-235 M16 run 3: 0 to 1 gives two results, both true, outcome `True`. 3 to
  4 gives exactly one result, `RestartRequires` false, outcome `False`, and
  no result for `RestartEnsures`.
- QSpec FR-207-AC-10, the `reference` step wire form: the round trip and the
  malformed-universe refusal.
