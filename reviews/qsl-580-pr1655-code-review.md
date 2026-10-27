---
id: SR-1655001
title: "Code review of quire-spec-language PR #1655: record clock collections (580j)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@2f321e12ed2e001ac34909a5edd91208f78903ef; PR #1655 diff against qsl-580i-clock-references (d10f8147c): qsl-eval/src/simulation/model/{encode,timed}.rs; qsl-semantics/src/check/{lowering,mod,timed_model}.rs, lowering/model.rs, timed_model/reading.rs; qsl-semantics/tests/it/clock_places.rs; qsl-eval/tests/it/timed_subject.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
---
# Code review of quire-spec-language PR #1655

## Summary

Ticket: QSL-580, slice 580j, stacked on #1651. The Rust review lane is folded
into this review. I reviewed only this PR's diff against
`qsl-580i-clock-references`.

**Slot places.** A `Timer` member whose multiplicity is not `[1,1]` adds
`Slot(CollectionShape{ordered, unique, min, max})` to the path, so its place
is `ts.[slot].x`. `record_at` and `through_references` stop at a slot.
`RecordWrite` treats a slot as no omitted component. A `Path` argument with a
slot refuses `not-a-place`. Optional members still refuse `TimedNotChecked`
(STD-493 item 17).

**Clock dimension.** `ClockField::bound` is the product of the declared
maxima along the path (`checked_mul`), and is `None` when any maximum is
missing. `TimedModel::dimension` sums the bounds; see FND-003.

**Element identity at S3.** `element_path` maps `e.g…` under a traversal
binder to the collection's path, then an open slot, then `g…`.
`reaches`/`same_place` compare paths with any slot matching any slot. `e.n`
is a data read, while `e.x` and `e` refuse `clock-read-outside-constraint`.
`size(self.ts)` and the traversal source are exempt from the read-whole
rule. Probes I ran (local tests, never committed):
- `self.ts != self.ts` refuses at `self.ts`.
- `exists(e in self.ts: e.x >= 1 ms)` refuses at `e.x`, which is right until 580k.
- `exists(e in self.ts: e.n > 1)` checks.
- `size(filter(e in self.ts: e.n > 1)) = size(self.ts)` checks.
- `filter(…) = filter(…)` checks wrongly (FND-001).

**qsl-eval.**
- Slot places are listed per object in ascending lexicographic slot vectors up to the maxima.
- `live` walks the path through record components (`Typing::component`) and collection elements up to the innermost slot.
- A slot that becomes live is reset to 0.
- The digest names each live slot by its index and leaves out slots that are not live.
- `UnboundedCollection` refuses a collection with no maximum.
- `ClockPlace::present` now means only that the object is held; see FND-002.

Gates: `cargo test -p qsl-semantics -p qsl-eval -p qsl-analyze --all-features`
ran at 2f321e12e through `locked-build.sh` (jobs 4, `QSPEC_DIR` at QSpec
bd73a062) and passed (exit 0).

## Verdict

**CHANGES REQUESTED.**
- FND-001 is medium: a value comparison of a clock collection is wrongly admitted.
- FND-002 is medium: zone counterexamples list clocks of slots that are not live.
- FND-003 and FND-004 are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | A comparison of clock collections through a traversal result is admitted. Probe: `pre E ... { filter(e in self.ts: e.n > 1) = filter(e in self.ts: e.n > 2) }` checks with no refusal. FR-230 "Element identity" requires refusing `=` and `!=` over a clock collection "and any other query that compares elements as values" with `clock-read-outside-constraint`. The `filter` result is a collection of clock-bearing `Timer` elements, but `traversed()` exempts `self.ts` as the traversal source, and nothing marks the traversal node itself as a clock-bearing value. So the `=` compares whole elements. The same gap lets a `filter` result reach `contains`, `convert` or a data parameter. Fix: treat a traversal over a clock collection whose result holds its elements (`filter`, `sort`, `collect` of `e`) as clock-bearing, so that reading it whole outside `size` or another traversal's source refuses. Add the probe as a test. | qsl-semantics/src/check/timed_model.rs:778-790, 896-910 |
| FND-002 | medium | qsl-analyze is a consumer this PR left unchanged. The zone counterexample's clock vector filters places by `place.present(&state)` (zone_check/search.rs:1061). After this PR `present` means only that the object is held; liveness of a slot is `TimedSystem::live`. A counterexample from a state where `ts` holds fewer elements than its maximum therefore writes values for slots that are not live, while the state digest and STD-493's byte form leave them out. Fix: filter with `timed.live(place, &state)`, and consider renaming `present` (or making it private) so callers cannot mix it up with liveness. | qsl-analyze/src/zone_check/search.rs:1055-1063; qsl-eval/src/simulation/model/timed.rs:65-75, 497-551 |
| FND-003 | low | `TimedModel::dimension(owner)` keeps only places whose `owner == owner`, so a subtype's dimension leaves out the places it inherits from its ancestors, though FR-230 defines the dimension "for each object". Use `conforms`, or document that the argument is the declaring type. `fold(0, u64::saturating_add)` also caps an overflowing dimension silently; `checked_add` with `None` would match `bound`. | qsl-semantics/src/check/timed_model.rs:455-466 |
| FND-004 | low | For a `Set`, `Bag` or `OrderedSet` the slot index is the element's position in canonical key order, which is not element identity: inserting an element with a smaller key shifts every later slot, so clock values would move between elements and the shifted-in slot would be reset to 0. This cannot happen in this slice, because a write of `self.ts` refuses at S3 and FR-120 gives a record element no value domain, so exploration never writes `ts`. It has to be settled with 580l's provenance (`Carried`/`Fresh`). Record it there so slot liveness is not reused unchanged. `live` also re-walks the path and looks up record declarations for every place on every call; that is fine at these sizes. | qsl-eval/src/simulation/model/timed.rs:404-420, 497-551 |
