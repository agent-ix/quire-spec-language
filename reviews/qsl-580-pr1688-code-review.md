---
id: SR-1688001
title: "Code review of quire-spec-language PR #1688: finite record, enum and collection domains; Quantity snapshots; EN-6 fixes (580o)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@2f7c28cda; PR #1688 diff against main (50 files): qsl-analyze/src/zone_check/search.rs; qsl-eval/src/simulation/model{.rs,/domain.rs,/frames.rs,/timed.rs,/application.rs,/symbolic.rs}; qsl-eval/src/value/expression; qsl-semantics/src/model/{observation.rs,observation/document.rs,observation/encode.rs,observation/tree.rs,state.rs,intake.rs}; tests timed_subject, zone_clock_collections"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: reviews
---
# Code review of quire-spec-language PR #1688

## Summary

Ticket: QSL-580, slice 580o. The Rust review lane is folded into this review.
I reviewed only this PR's diff against `main` at 2f7c28cda. The specs are
FR-120 "value domains" with AC-22..26 and TC-1270 (spec PRs #1682 and #1685,
both merged), FR-106 "A Quantity value" with AC-59, FR-154 unit bounds, and
FR-230.

Gates: `cargo test -p qsl-semantics -p qsl-eval -p qsl-analyze --all-features`
passed at 2f7c28cda (EXIT=0, no failures). Clippy was not run.

The six soundness questions from the brief:

1. **`scaled_bounds` (EN-6 aLU).** No clock constant is missed. The walk now
   collects every guard part and every part of a quantifier family under it,
   recursively. It also collects time-invariant bounds, reset values (resets
   by selection included), the timed-claim automaton's constants and the
   horizon. `ClockConstraint` is single-clock, so there are no diagonal
   constants to miss. A family's template clock is the element `ClockId`, and
   `place.clock()` maps it to every slot place. OK.
2. **`uncovered`.** Exact. A local probe (not committed) built 3,000 random
   2-clock zones with 0 to 3 random covers. Some cases added a duplicate
   cover, some added a cover equal to the zone. Every integer point of a
   12 × 12 grid matched `zone ∧ ¬∪covers`: 432,000 points, no mismatch.
   - Dedup by `Dbm` equality can only miss deduplication, never coverage.
   - `includes` returning nothing and skipping a disjoint cover are both
     exact.
3. **Collection domains.** Counts and orders match AC-26: sequence `p^k`,
   set `C(p,k)`, bag `C(p+k-1,k)`. Unranking is exact, and `combinations`
   saturates. Sequence slots carry by position.
   - Set and bag matching is wrong outside the receiver's top-level fields
     (FND-001).
   - Matching is wrong when combined with resets by selection (FND-002).
   - EN-6 drops the carries that matching produces (FND-003).
4. **Records and enums.** OK.
   - `record_orders` takes declared positions from intake.
   - `Space::Composite` takes digits in declared order and writes them into
     unchanged (name-sorted) slots.
   - An order whose length differs from the field count falls back to slot
     order.
   - A union anywhere below a record, including inside a collection, stays
     unbounded through `holds_union`.
   - An ordered set stays unbounded.
5. **Frame-granted fields.** A granted finite field ranges over its domain.
   An unbounded one is a `domains()` root, so `explore_model` refuses with
   `RequiresBound` before expanding. The new `data_writes` path is outside
   the spec (FND-004).
6. **FR-106 Quantity.** The refusal order is FR-106's:
   - Envelope checks 1 to 5, with `magnitude` as the one object member.
   - Then the unit name (check 6).
   - Then the magnitude's own envelope and kind. A String magnitude gives
     `wrong-value-kind` naming `magnitude`.
   - Then the Rational/Decimal values (`invalid-value` at the path
     `magnitude`).

   Identity normalizes the magnitude, and the encoder writes a Rational
   magnitude back. Two small mismatches remain (FND-005).

## Verdict

**Changes requested.** FND-001 to FND-003 give wrong clock values. Explorer
and zone analysis can therefore decide timed claims over clock valuations
the model does not have. Two local probes confirm FND-001 and FND-002, and
FND-003 is confirmed in the code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-120-AC-26 matches set and bag elements by equal data, but `matched` only handles the receiver's top-level unordered fields. On another object of the state whose granted `Set<Timer>` changes, the clocks stay by slot. Probe: `c` and `d` each hold `{n1}`, delay 5, then step `work` on `c`. `d` becomes `{n0, n1}` with `n0` at x = 5 and `n1` at x = 0; the spec requires `n1` at 5 and `n0` fresh at 0. The receiver's own step is correct. The same gap applies to a set below a record field (`[Field r, Field s, Slot]`) and to clocks below a nested collection inside a moved element, which the doc comment admits. | qsl-eval/src/simulation/model/timed.rs:1221-1311 |
| FND-002 | high | A reset by selection pushes resets at pre-state slot places, and they are applied after the carries from `matched`. When the frame-granted set moves the selected element, the reset lands on the wrong or a dead slot. Probe: `g = {n1, n2}` at 3 ms, with `post forall(e in self.g: e.n > 1 implies e.x = 0 ms)`. The candidate `{n2}` gives `n2` x = 3 ms (carry (0,1), reset at dead place 1). `{n2, n3}` also gives `n2` x = 3 ms. In both, the selected element should read 0. | qsl-eval/src/simulation/model/timed.rs:1074-1116 |
| FND-003 | high | EN-6 never applies `GuardedStep::carries`: `successors` resets only. Its `CarriedClocks` refusal keys on `writes_collections()`, which sees only postcondition collection writes. A granted `Set`/`Bag` of clock records is now a finite domain whose steps carry clocks between slots. The zone search accepts it and drops those carries, so a moved element's clock reads its old slot's value or a stale dead slot. Extrapolation over a wrong valuation is unsound. `zone_clock_collections` tests only sequences. Fix: refuse whenever a step carries, or copy the DBM row/column for each carry. | qsl-analyze/src/zone_check/search.rs:379, 723-770 |
| FND-004 | medium | `data_writes` executes `self.f = E` for granted fields with no finite domain. It adds the cause `unsupported_construct`/`write-not-computed`, which is in neither FR-120 nor the diagnostics catalog. No test reaches any of its refusals. `explore_model`/`sample_model` refuse such a field with `RequiresBound` first, so the path runs only through raw `ModelSystem::successors`. That is how the "stepped quantity" FR-154 test runs: the bound is not shown end to end through exploration. It needs a spec ruling (FR-120 "Successors" plus the catalog entry) or removal. | qsl-eval/src/simulation/model/frames.rs:1220-1365, 1536 |
| FND-005 | low | Two mismatches with FR-106-AC-56. A Rational outside its declared domain refuses `invalid-value` with no fields; the AC requires naming the declared type and the normalized pair. `NativeKind::Decimal` is now a known kind, but no arm admits a Decimal object into a `Decimal` slot. Such an object now refuses `wrong-value-kind` (earlier it refused `invalid-value` naming `kind`). | qsl-semantics/src/model/observation/document.rs:1603-1610 |
| FND-006 | low | The doc comment "A Text object's `text` member" now sits above `quantity_identity`, so `text_member` has no doc and `quantity_identity` has two summaries. `#[string_edge]` is duplicated in that run. | qsl-semantics/src/model/observation.rs:1083-1097 |

## Dispositions

Round 1, fix commit 16d014311 (head fb774be88). `cargo test -p qsl-semantics -p qsl-eval -p qsl-analyze -p qsl-replay --all-features` passed at fb774be88 (EXIT=0, no failures). The run went through locked-build with jobs 4, debug info off, and QSPEC_DIR at QSpec e3d48d76. My probe_1688.diff probes were re-applied on that head and passed (not committed).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `matched` now walks every live slot place on every object. `source_slots` reads each collection along the clock's path in turn: a sequence slot keeps its position, and a set or bag element is matched one to one in ascending slot order by `values_equal`. Re-running my probe (`c` and `d` each `{n1}` at 5 ms, then `work` on `c`) now gives `d.n0` at 0 and `d.n1` at 5 (carry slot 0 to slot 1). Receiver `d` gives the same. New tests cover the non-receiver case, a bag, a record-nested set and a nested collection (`set_and_bag_elements_keep_their_clocks_on_every_object`, `matching_reads_every_collection_on_the_clock_path`). |
| FND-002 | fixed | Carries and fresh zeros are now computed first. A reset by selection reads the post-state element (`elements(post, …)` and `about_one(post, …)`) and writes the post-state place, so it is applied after the carries. Re-running my probe: `{n2}` gives `n2` at 0; `{n1, n2}` gives 3 and 0; `{n2, n3}` gives `n2` at 0; `{n0, n1}` carries `n1` at 3. New test: `a_selection_reset_writes_the_post_state_element`. |
| FND-003 | fixed | `carries_clocks()` is true for any whole-collection write, or for any clock whose path has an unordered `Slot`. EN-6, EN-1 timed explore and `check_zone_certificate` all refuse `CarriedClocks` on it. I looked for another way to get a carry through. `source_slots` returns `place.slots` unchanged for every ordered step, so `matched` can carry only through an unordered slot. `written` carries only on whole-collection writes. Selection resets, created objects and references produce no carries. A further probe over granted `Sequence<Timer>[0, 2]` from three pre-states checked 441 two-step successors each: every step's carries are empty, and `carries_clocks()` is false. I found no other route. Applying carries in the zone is QSL-1162. This also closes SR-1671001 FND-005 on EN-6, because a symmetric model with a clocked set or bag is refused before symmetry applies. |
| FND-004 | fixed | `data_writes`, `evaluate_write` and `write-not-computed` are deleted. `TimedSystem::with_max_clocks` runs `check_model_bounds` and refuses `NotSimulated(RequiresBound)`. `walk::expand` has `debug_assert!(system.bounded())`. EN-6's own bound check is gone, and it is redundant now that every `TimedSystem` is checked when built. `explore_model` and `sample_model` already check bounds. If a caller in a release build reaches raw `successors` on an unbounded system, the frame enumerator stops with `BROKEN`, because `Domain::of(..).ok_or_else(stop(BROKEN))` refuses rather than producing a value. No path expands an unbounded granted field. Dropping the FR-154 stepped quantity is right: a Quantity field has no finite domain, so a grant always settles `RequiresBound` and a field that is not granted never changes. Under FR-120 a stepped quantity therefore cannot happen. FR-154's check on supplied values (snapshots and invocations) is still tested. |
| FND-005 | fixed | An out-of-domain Rational refuses `invalid-value` with `expected` `Rational[-10, 10; 1, 12]` and `actual` set to the normalized pair (`22`/`2` gives `11/1`), as `a_rational_outside_its_domain_refuses_with_the_normalized_pair` tests. Admission of a Decimal object into a `Decimal` slot is recorded in QSL-1068 (Decimal and Float fields), which the PR body names. |
| FND-006 | fixed | The `text_member` doc comment and `#[string_edge]` are back in place, and `quantity_identity` has a single doc comment. |
