---
id: SR-1688002
title: "Gap analysis of quire-spec-language PR #1688: finite record, enum and collection domains; Quantity snapshots (580o)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@2f7c28cda; PR #1688 against FR-120-AC-22..26 and TC-1270, FR-106 A Quantity value and FR-106-AC-55/56/59, FR-230-AC-26 with QSpec FR-154 unit bounds"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: reviews
---
# Gap analysis of quire-spec-language PR #1688

## Summary

Ticket: QSL-580, slice 580o. Plan completion: not assessed.

**Covered.**
- **AC-22/23 domain sizes and orders.** `cfg` 6 with `on` first, `w` 18,
  `r1` 12, `col` 3, `pick` 6, `note` 6 with absent first, `oc` 7, `sq` 7.
- **AC-24 domain half.** `t` 4, `sw` 1, `mx` 8.
- **AC-25 `RequiresBound` through `explore_model`.** `op`, `tm` and `wr`.
- **AC-26 counts and orders.** 43, 21, 85, 4, 7, 22, 28, 11 and 15.
- **AC-26 bounds.** Unbounded `Sequence<Open>`, and unbounded unmaxed
  `Sequence<Cfg>` and `Set<Cfg>`.
- **AC-26 receiver matching.** The sequence position carry, and the set
  equal-`n` match on the receiver.
- **Enum parameter and result.**
- **Exploration of `Sequence<Timer>[0,3]` to 85 states.**
- **EN-6 over a granted sequence.**
- **FR-106-AC-59 subset.** The magnitude spellings, the `unit` mismatch,
  `denominator` 0 at the path `magnitude`, Decimal `scale` -1, and the `max`
  bound on a supplied and a stepped quantity.

The test doc comments name what is not covered: the tuple `Pr`, the union
`U`, `Set<Clock>`, `Sequence<Clock>`, `Reference<Clock>` and `Map`. The
package fixture cannot declare these.

## Verdict

**Gaps remain.** The matching tests stop at the receiver's own set. That is
why SR-1688001 FND-001 to FND-003 got through. Several AC halves have no
test.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-120-AC-26 set/bag matching has no test for a `Bag` with equal elements (matched in ascending order). It has none for a granted set on a non-receiver object, a set below a record field, a reset by selection in the same step, or EN-6 over a granted `Set`/`Bag` of clock records. These are the cases SR-1688001 FND-001 to FND-003 found broken. | qsl-eval/tests/it/timed_subject.rs:3285-3395; qsl-analyze/tests/it/zone_clock_collections.rs |
| FND-002 | medium | Several AC halves are untested. AC-22: `put`'s 6 × 4 × 18 = 432 candidates, each charged against `max_candidates`. AC-24: the successor half (`retime` gives `ck.n` 3 with `x` reset, `sw` keeps its clocks, and with no postcondition every clock keeps its value). AC-25/26: the `sample_model` `RequiresBound` half; only `explore_model` is asserted. | qsl-eval/tests/it/timed_subject.rs:2755-2975, 3195-3285 |
| FND-003 | medium | The FR-106 Quantity and Rational native admission is new in this PR, but only the subset above is tested. Not tested from FR-106-AC-59: `spans` `duplicate-member` across spellings, `loads` multiplicity 2 with its written form, a `retag` with no change across spellings versus a `frame_violation`, `missing-member` for `unit`/`magnitude`, `unknown-member` `bits`, a `magnitude` given as a String object, `{"integer"}` or a JSON string, and a Quantity in a Rational slot (and the reverse). Not tested from FR-106-AC-55/56: the Rational admission cases, including the out-of-domain naming that SR-1688001 FND-005 found missing. | qsl-semantics/src/model/observation/document.rs; qsl-eval/tests/it/timed_subject.rs:3075-3185 |
| FND-004 | low | TC-1270 steps 4 to 6 are only partly runnable: the tuple `Pr`, the union `U`, `Reference<Clock>`, `Map<Boolean, Boolean>[0, 1]`, and the `Set<Clock>` with and without a maximum. The tests record this as a fixture limit. The ticket should record the uncovered AC rows rather than leave them in test prose. | qsl-eval/tests/it/timed_subject.rs:2767, 2909, 3200 |

## Dispositions

Round 1, fix commit 16d014311. The checks are as in SR-1688001's dispositions.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | New tests: `set_and_bag_elements_keep_their_clocks_on_every_object` (non-receiver set, bag in ascending order), `matching_reads_every_collection_on_the_clock_path` (set below a record field, nested collection in a moved element), `a_selection_reset_writes_the_post_state_element` (reset with a moving element) and `the_zone_search_refuses_a_set_or_bag_of_clocked_elements` (EN-6 over Set/Bag). |
| FND-002 | fixed | `every_candidate_of_a_record_product_is_charged`: 6 × 4 × 18 = 432 successors. `t: Timer` (4 values) stands in for the tuple `Pr`, and `max_candidates` 431 stops with `MaxCandidates`. `a_granted_record_keeps_its_clocks_unless_reset` covers the AC-24 successors: `t.n` 3 with `t`'s clocks reset, and `sw`/`mx` clocks kept; with no postcondition, 32 successors with every clock kept. `a_record_with_an_unbounded_component_requires_a_bound` now also asserts `sample_model`'s `RequiresBound`. |
| FND-003 | fixed in part | `qsl-semantics` tests cover the following. Rational, through `admit_native`, because a domain package cannot declare `Rational[lo, hi; dmin, dmax]`: normalization, the boundaries, out-of-domain naming, every listed member defect, and a Decimal or Quantity object in a Rational slot (and a Rational object in a Float or Quantity slot). Quantity, through the replay harness with `reach: Quantity<ms>`: one value for both spellings, the frame comparison by exact magnitude, missing `unit`/`magnitude`, unknown `bits`, `magnitude` as a String object, `{"integer"}` or a JSON string, `unit` as a number or the wrong unit, and the magnitude's own defects at the path `magnitude`. Not reachable: `spans`/`loads` (and Rational `ratios`/`weights`/`lane`), because a set, bag or ordered set of Quantity, Rational, Decimal or Float is refused `unsupported-feature` (QSL-1068). The `float64` object case is also not reachable: there is no `float64` native kind yet, so it refuses `invalid-value` naming `kind` (QSL-1068). |
| FND-004 | recorded | The TC-1270 rows blocked by the fixture are listed in the PR body: the tuple `Pr`, the union `U`, `Reference<Clock>`, `Map<Boolean, Boolean>[0, 1]` and `Set<Clock>`. |
