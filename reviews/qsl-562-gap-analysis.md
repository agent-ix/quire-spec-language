---
id: SR-12232
title: "Gap analysis of quire-spec-language PR #1108 against FR-216 and TC-661"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@74d8f669d5b36bf7817124dfe8a8efcce474238d; PR #1108: qsl-eval/src/simulation/protocol/footprint.rs, qsl-analyze/tests/it/protocol_footprints.rs; FR-216-AC-1, FR-216-AC-2, FR-216-AC-3, FR-216-AC-5, TC-661, ADR-027 FT-1 to FT-5, ADR-021 POR-5"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-216
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-661
    type: reviews
---
# Gap analysis of quire-spec-language PR #1108

## Summary

Ticket: QSL-562 (ticket text read as data). Planless and scoped to the PR
diff. Plan completion: not assessed. FR-216-AC-4 is out of scope: it needs
compensation (`reg`), which comes in a later PR.

Each in-scope AC has a test tagged `#[trace("TC-661", "FR-216-AC-n")]` in
`qsl-analyze/tests/it/protocol_footprints.rs`, and each test asserts the AC's
outcome:

- FR-216-AC-1 has
  `attempts_on_one_cell_are_dependent_and_on_two_cells_independent`. On one
  cell, both attempts write (`c`, `v`) and are dependent. On two cells, A on
  `c` and B on `d` are independent. For a claim over `d.v`, A is invisible and
  B is visible.
- FR-216-AC-2 has
  `control_steps_touch_only_protocol_locations_and_one_branch_is_dependent`.
  It asserts the exact `fork` write set (root, `left`, `right` `ctl` and
  `inst` of `Both`). It also checks that `join` reads `inst`, that `fork`,
  `join` and `finish` are invisible over (`c`, `v`), and that two steps of one
  branch are dependent.
- FR-216-AC-3 has
  `a_cancelling_join_writes_its_branches_and_a_queue_orders_its_steps`. It
  covers `cancel` against `continue` for branch `ctl`, the `receive` enabling
  queue, `send`/`receive` dependent, and `attempt(Done)` independent of
  `duplicate`.
- FR-216-AC-5 has
  `an_enabling_footprint_holds_exactly_what_enables_the_step`. It checks the
  exact enabling set (two fields, two memberships, binder `a`, own `ctl`) and
  the exact `duplicate` enabling set (`queue(ch)` of instance 0).
- There are two extra TC-661 tests: the branch-end disposition write, and
  clause-selected model reads.

The production code has an owner in FR-216 / ADR-027 §5. No stubs were found,
and no coverage was inflated.

## Verdict

Pass on AC coverage. The one high finding is code-review SR-12231 FND-001,
and it is not repeated here. A medium gap below explains why the suite missed
it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test checks FT-4's semantic contract: steps reported independent must commute (POR-4). The tests check that particular pairs are independent or dependent, but never that each independent pair reaches the same state in both orders. A diamond check over reachable states (run both orders of every independent pair and compare the successor indices) would have caught SR-12231 FND-001. Add it over the §7 fixtures and over a `repeat` / `outstanding continue` fixture. | qsl-analyze/tests/it/protocol_footprints.rs:174-506 |
| FND-002 | low | The FR-216-AC-1 and AC-2 visibility checks build `ClaimReads` by hand, (`d`, `v`) and (`c`, `v`), instead of deriving them from the claims the ACs name (`always holds(k.v <= 3)`, a claim over `d.v`). The test covers `visible` over a given read set, but not that a claim's atoms give that set. | qsl-analyze/tests/it/protocol_footprints.rs:166-172 |
| FND-003 | low | FR-216-AC-3 names FR-212's `Chatter`, but the test builds its own `Chatter` from the fixture (one sender branch, one receiver branch, `overflow block`). FR-216-AC-5 names `bump(by)`, and the test uses `nudge(peer)`. Both keep the AC's structure, but they are not the named artifacts. | qsl-analyze/tests/it/protocol_footprints.rs:323-336; qsl-analyze/tests/it/protocol_footprints.rs:374-377 |
| FND-004 | low | Spec: TC-661's Description says "Scope: FR-216-AC-1 to FR-216-AC-4", but its procedure step 5 and the tests cover FR-216-AC-5 as well. | spec/test-cases/TC-661-protocol-step-footprints.md |

## Dispositions

| Finding | Outcome | Note |
| --- | --- | --- |
| FND-001 | fixed 64b082d85 | Added `every_independent_pair_of_steps_commutes`, a diamond check over the §7 fixtures, the join-policy fixtures, `Chatter`, a fixture shaped like AC-5 and the `repeat` / `outstanding continue` re-fork. |
| FND-002 | fixed 64b082d85 | `ProtocolSystem::claim_reads` derives the reads from the checked `holds` atoms. The AC-1 and AC-2 tests read `temporal Bounded … { always holds(k.v <= 3) }` for `d` and for `c`, and assert the derived set. |
| FND-003 | accepted-no-change | `Chatter` keeps FR-212's branches, channel and nodes over this fixture's `Cell`. `nudge(peer)` stands for `bump(by)`: `bump` is the fixture's operation with no parameter, and `by` is an attempt-form keyword. Both are stated in the test docs. |
| FND-004 | deferred: spec | The TC-661 scope text is a spec change, and the spec author owns it. |
