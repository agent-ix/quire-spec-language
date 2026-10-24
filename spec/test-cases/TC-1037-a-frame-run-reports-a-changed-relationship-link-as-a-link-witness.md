---
id: TC-1037
title: "A Frame run reports a changed relationship link as a link change of its frame witness"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-115
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-267
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-116
    type: verifies
---
# TC-1037: A Frame run reports a changed relationship link as a link change of its frame witness

## Description

Verify that `run_clause`'s `Frame` selection reports FR-106 check 11.5's
finding as the `Link` change of the FR-115 `FrameWitness`, in step 5's order and
after the per-population findings, that it writes and reads back as the
`link` change of `native-run-result/2`, and that replay settles it.

Scope: FR-115-AC-8, FR-115-AC-9, FR-115-AC-10, FR-267-AC-12 and
FR-116-AC-7.

## Test Procedure

Use TC-1035's **orders** fixture and its invocation **I** (operation `rehome`
on `Pump` with the precondition `RehomePre` and the postcondition
`RehomePost`, so the package holds the operation's frame node; `self`
`{Q, p1}`; `created []`, `deleted []`; frame `modifies []` unless a row sets
it). Its pre snapshot holds in complete `Q` the objects `s1`, `p1`, `p2`, `t1`
and `t2` and lists only `Pump-housing-Tank`, complete, with `p1`-`t1` and
`p2`-`t1`; its post snapshot holds the same objects and lists housing complete
with `p1`-`t2` and `p2`-`t1`. The unit selects the `orders` package as
`Orders`. Every row runs `Frame { operation: Orders::Pump::rehome,
invocation: I' }` where `I'` is I with the row's change, in memory, with the
provision digests recomputed for every changed document. Each row applies one
change to I unless it says otherwise. Tag the tests
`#[trace("TC-1037", "FR-115-AC-n")]`.

| # | Change to I | Expected |
| --- | --- | --- |
| 1 | `modifies [Pump-housing-Tank]` | `evaluate`, `success`, `truth: true`, exit 0 |
| 2 | I as written (`modifies []`) | `evaluate`, `violation`, `truth: false`, exit 10; the witness has `code` `frame_violation`, `cause` `unauthorized-change`, the invocation, pre and post references, `population` `Q`, and `change` `Link { relationship: Pump-housing-Tank, difference: removed, source: {Q, p1}, target: {Q, t1}, modifies: [] }`, the first differing instance in source-then-target order, before `p1`-`t2` `added` |
| 3 | `modifies [Pump.backup]` (a field entry) | as row 2, with `modifies` holding that one key |
| 4 | post `Q` also holds `p3` (a `Pump`); post housing `p1`-`t1`, `p2`-`t1`, `p3`-`t1`; pre housing as in I; `created [{Q, p3}]`; frame `creates [Pump]`, `modifies []` | `violation`, exit 10, `Link { Pump-housing-Tank, added, {Q, p3}, {Q, t1}, [] }`: 11.1 and 11.4 pass for the granted creation, so 11.5 is the first finding |
| 5 | row 4 with `modifies [Pump-housing-Tank]` | `evaluate`, `success`, exit 0: the relationship grant covers the link at the created object |
| 6 | both snapshots list `Sys-composition-Pump` complete, the pre with `s1`-`p1` and the post with no instance | `violation` with the `Link` of row 2: housing precedes composition in ascending `DeclarationKey` order, though both differ |
| 7 | pre housing `p2`-`t1` only; post housing `p1`-`t2` only | `violation` with `Link { Pump-housing-Tank, added, {Q, p1}, {Q, t2}, [] }`: the lower source first, though the other instance is `removed` and has the lower target |
| 8 | row 4 with frame `creates []` | `violation` whose `change` is `Created { object: p3, type_name: Pump, creates: [] }`, not a `Link`: 11.1 precedes 11.5 |
| 9 | row 4 with `created []` | `evaluate`, `refusal`, `population_delta_mismatch`/`delta-disagreement`, exit 20: 11.4 precedes 11.5, whose link `p3`-`t1` is never reported |
| 10 | post `Q` lacks `p2`; post housing `p1`-`t1` only; `deleted [{Q, p2}]`; frame `deletes [Pump]`, `modifies []` | `evaluate`, `success`, exit 0: 11.2 finds the deletion granted, 11.4 finds the delta equal, 11.5 leaves out `p2`-`t1` and finds `p1`-`t1` unchanged |
| 11 | housing listed in the pre snapshot only; then housing listed `"complete": false` in the post snapshot | `success`, exit 0, twice: the snapshots do not say whether housing changed |
| 12 | row 2 twice | equal reports, including usage |
| 13 | write row 2's report, and row 4's, as `native-run-result/2` and read each back | each reads back to the equal `FrameWitness`; the document carries a `link` change with `relationship`, `difference` (`removed`, then `added`), `source`, `target` and `modifies`, `violation`, `truth: false`, `basis: closed-scope`, no outcome and no FR-351 witness; the top-level `population` is `Q` |
| 14 | edit row 13's document independently: remove each `link` member; remove `population` from `source`, then `key` from `target`; add an unknown member to the change; set `difference` to `"moved"`, then `null`; give `source` a bare key | each refuses with a located wire diagnostic |
| 15 | hand-built `FrameCounterexample` envelopes for `Orders::Pump::rehome`, over the package whose frame modifies nothing, carrying row 2's invocation and `change` the link `p1`-`t1` `removed`; then `change` the link `p1`-`t2` `added`; then, over the package whose frame modifies housing, an envelope carrying row 1's invocation and the first `change` | `reproduced-with-evaluated-witness` holding the payload's change and the replay's `Link`; the same, holding both changes as found; `inconclusive`, `Verdicts` |

## Expected Results

Each finding is the one named for its row alone. A link finding is the `Link`
change of the witness, selected in step 5's order and reported only after every
population's steps 1 to 4 find nothing, and it carries the frame's checked
`modifies`. A link that step 5 leaves out produces no witness.
