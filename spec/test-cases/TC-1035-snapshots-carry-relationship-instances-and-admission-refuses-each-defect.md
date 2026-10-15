---
id: TC-1035
title: "Snapshots carry relationship instances and admission refuses each defect in check order"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-364
    type: verifies
---
# TC-1035: Snapshots carry relationship instances and admission refuses each defect in check order

## Description

Verify that a snapshot document carries relationship instances in FR-106's
`relationships` member, that the admitted observation holds them as extents,
that each of checks 6.6 to 6.9 refuses its defect alone, how completeness and
closure treat an instance end, that the frame compares instances, and that
the document limits and a precondition's skipped `post` apply to them.

Scope: FR-106-AC-23, FR-106-AC-24, FR-106-AC-25, FR-106-AC-26, FR-106-AC-27.

## Test Procedure

Use TC-1033's fixture **orders** and variants, with two additions: the
invariant `SysOk` on `Sys` (always true) for the `Current` rows, and the
operation `rehome` on `Pump` (no parameters, no result, `modifies` empty
unless a row says otherwise) for the `Invocation` rows. Documents are written
as FR-106 shows them, held in in-memory provisions keyed by `sha256-jcs`
digest, and the provision digest is recomputed for every mutated document.

Base snapshot **B**: `observation` `current`, anchor `{handler, validate}`,
population `Q` complete holding `s1`, `s2`, `p1`, `p2`, `p3`, `t1` (all
fields as declared, `backup` absent), `relationships`:
`Sys-composition-Pump` complete with instances `s1`-`p2` then `s1`-`p1`, and
`Pump-housing-Tank` complete with `p1`-`t1` then `p2`-`t1`. Selection:
`Current { snapshot: B, anchor: {handler, validate}, self: {Q, s1} }` for
`SysOk`. Each row applies one mutation to B unless it says otherwise. Tag the
tests `#[trace("TC-1035", "FR-106-AC-n")]`.

| # | Check | Mutation | Expected |
| --- | --- | --- | --- |
| 1 | admit | B as written | one current observation; composition extent complete with `targets(s1)` `[p1, p2]` and `sources(p1)` `[s1]`; housing extent complete with `sources(t1)` `[p1, p2]` |
| 2 | admit | B with the entries, the instances and the JSON members reordered | the digest and the admitted value of row 1 |
| 3 | admit | `relationships` removed | admitted; `complete` false for both relationships |
| 4 | admit | composition entry `{"complete": true, "instances": []}` | admitted; composition `complete` true and empty; housing as row 1 |
| 5 | admit | composition `s1`-`p1`, `s1`-`p2`, `s1`-`p3` and `s1`-`p4` with `p4` added to `Q`; and housing listing no instance for `p3` | admitted; no multiplicity comparison |
| 6 | 6.6 | composition entry without `instances` | `invalid_runtime_input`/`missing-member` naming composition |
| 7 | 6.6 | composition entry with an extra member `note` | `invalid_runtime_input`/`unknown-member` |
| 8 | 6.6 | composition `complete` `"yes"` | `invalid_runtime_input`/`wrong-value-kind` |
| 9 | 6.7 | `relationship` the identity of Connection `pipe`; then `relationship/Nope-x-Pump` | `invalid_runtime_input`/`wrong-role-mapping`, twice |
| 10 | 6.7 | a second composition entry | `invalid_runtime_input`/`conflicting-identity` at the second entry |
| 11 | 6.8 | composition instance 0 without `target`; then with an extra member `note`; then with `key` the number `1` | `invalid_runtime_input`/`missing-member`; `unknown-member`; `wrong-value-kind`; each naming composition and instance 0 |
| 12 | 6.8 | composition instance 0 `source` population `ix://test/orders/other` | `invalid_runtime_input`/`wrong-role-mapping` naming composition, instance 0 and end `source` |
| 13 | 6.8 | composition instance 0 `source` `t1`; then its `target` `t1` | `invalid_runtime_input`/`wrong-value-kind` naming composition, instance 0 and end `source`; then end `target` |
| 14 | 6.9 | composition `s1`-`p2`, `s1`-`p1`, `s1`-`p2` | `invalid_runtime_input`/`conflicting-identity` naming composition and instance 2 |
| 15 | 6.9 | row 14 over variant **bag**; then over variant **bag-target** | admitted with `targets(s1)` `[p1, p2, p2]`; then the row 14 refusal |
| 16 | 6.8 over 6.9 and walk order | composition instance 0 `target` `t1` (wrong type), instance 1 `population` `ix://test/orders/other` | `wrong-value-kind` at instance 0 |
| 17 | 8 | housing `p2`-`t9`, `t9` listed nowhere | `dangling_reference`/`absent-target-in-complete-population` naming housing, instance 1, end `target`, `t9` and `Q` |
| 18 | 7 over 8 | row 17 with `Q` `"complete": false` | `Incomplete`, `incomplete_population`/`incomplete-scope` naming `Q`, no dangling record |
| 19 | 8 order | row 17 plus `p3.backup` present naming `p9`, absent from `Q` | `dangling_reference` naming `p9` and `Q` (the `reference` value first) |
| 20 | 7 | variant **Tanks**: population `Tanks` (member type `Tank`, closed) holding `t1`, marked `"complete": false`, `Q` over `[Sys, Pump]`; housing `p1`-`t1` | `Incomplete`, `incomplete_population`/`incomplete-scope` naming `Tanks` |
| 21 | 7 | row 20 with the housing entry removed | admitted: no instance names `Tanks` |
| 22 | 7 | composition entry `"complete": false` holding `s1`-`p1` | admitted; composition `complete` false |
| 23 | 11.5 | invocation `rehome` on `p1` with `modifies [Pump-housing-Tank]`; pre housing `p1`-`t1`, post housing `p1`-`t2` (`t2` in `Q` on both sides), both complete | admitted |
| 24 | 11.5 | row 23 with `modifies []` | `frame_violation`/`unauthorized-change` naming housing, `p1`-`t1` and `removed` |
| 25 | 11.5 | pre housing `p1`-`t1`, `p2`-`t1`; post housing `p1`-`t1` and `p2` deleted from `Q`, `deleted [{Q, p2}]`, frame `deletes [Pump]`, `modifies []` | admitted: the `p2`-`t1` instance goes with `p2` |
| 26 | 11.5 | pre housing `p1`-`t1`; post housing `p1`-`t1`, `p3`-`t1` with `p3` created, `created [{Q, p3}]`, frame `creates [Pump]`, `modifies []` | `frame_violation`/`unauthorized-change` naming housing, `p3`-`t1` and `added` |
| 27 | 11.5 | row 24 with housing listed in the pre snapshot only | admitted: the snapshots do not say whether it changed |
| 28 | limit | `ObservationLimits` values-per-document set to the value count FR-106 gives B; then B plus one more composition instance | admitted; `stage_limit_exceeded`/`node-count-exceeded` naming `observation.values`, the bound and the count reached |
| 29 | skipped post | the invocation of row 23 selected for a precondition on `rehome`, with its `post` snapshot's housing entry holding `instances` the string `"none"` | admitted; the verdict equals that of `PreCall` over the same pre snapshot, `self` and parameters |

## Expected Results

Each refusal is the one named for its check alone, with the relationship, the
zero-based instance index and the end it names, and with no observation
admitted. Admission compares no instance count with a multiplicity bound.
An instance end the document does not list is decided only by checks 7 and 8;
a frame violation over instances is reported after the per-population frame
checks.
