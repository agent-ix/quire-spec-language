---
id: SR-1685001
title: "spec-review of FR-120 max-declared Sequence finite value domain (quire-spec-language PR 1685)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-120-simulate-a-checked-package-s-state-family.md; spec/test-cases/TC-1270-a-record-or-tuple-of-finite-non-clock-components-is-a-finite-value-domain.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-215
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: references
---

# Spec review of STD-508 max-declared Sequence finite value domain (quire-spec-language PR 1685)

## Summary

Ticket: STD-508 (planner ruling e73d9279). One spec-review pass over `git diff origin/main...HEAD`
at head 2de9209b5ecf2f6f1595bf940c31e17965dc5e9a (FR-120 bullets + FR-120-AC-26, TC-1270 step 5).
Checked against FR-120 "Bounds", FR-215 (value-domain bullet, lines 164-176), FR-230 "Clocks and
resets", "Clock collections", "Collection-valued writes", FR-230-AC-14/19/20, FR-097-AC-2, TC-685.

Arithmetic, verified by hand: `Sequence<Cfg>[0,2]` 1+6+36 = 43; `Tick {x: Clock}` p=1 gives N+1 = 4 for
`[0,3]`, and `Sequence<Clock>[0,3]` 4; `Timer {x: Clock, n: Int[0,3]}` p=4 gives 1+4+16 = 21 for `[0,2]` and
1+4+16+64 = 85 for `[0,3]`; `(p^(N+1)-1)/(p-1)` gives 85 and 21 and `N+1` when p=1. All correct.
Reading vs the ruling: the ruling's "Sequence<Timer>[0,3] = 4" holds only for a pure-clock element
(the ruling's own formula, clock places add no factor, gives 85 for AC-24's Timer); the author's
reading is right and AC-26 uses `Tick` for 4. `Reference<Clock>` unbounded is consistent with FR-230
("Clock references": data, not a place). Set/Bag/Map unbounded departs from the ruling's literal
"(Sequence/Set/etc.)" and needs its own ruling (their sizes are not the sum of p^k; FR-230 gives
`Set<Clock>` at most one element, a Map has keys); the PR states the exception, it does not hide it.

Order "length first, then lexicographic, first element most significant": FR-215 states an order
only for records and delegates a field's domain to FR-120 ("A field's value domain is FR-120's for
its type"), and mentions no sequence order, so there is no contradiction; the "FR-215 gives" wording
in the order bullet covers only the record rule. No other prose in spec/, docs/ or sources calls
`Sequence<Timer>`/`Sequence<Clock>` unbounded (FR-230-AC-14/19/20, TC-685 only discuss
admission/places/provenance). Out of scope, noted: the qsl-eval `domains()` comments/code treat every
non-`enumerable` type as unbounded (see SR-1682001 implementation gap); the sequence-of-record path is
not implemented either.

`quire validate` on FR-120 and TC-1270: exit 0 before (main FR-120) and after (only the module
duplicate-archetype warnings). A scratch-scope `quire matrix` over FR-120 and TC-1270 lists
FR-120-AC-26 as its own row beside AC-22..25. `FR-120-AC-26` and `SR-1685001` are unused on main and
on all 11 open PR heads.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The clock-slot bullet says a candidate's clock slots "take the pre-state value or the reset that FR-230's slot provenance gives", but FR-230 gives provenance only for a place-preserving collection-valued write conjunct, and a candidate of a length different from the pre-state's has slots with no pre-state element. A granted `Sequence<Timer>[0, 2]` with no postcondition, from pre-state length 0, has candidates of length 2 whose slots have neither a pre-state value nor a provenance; the earlier STD-508 comment's "FR-230 silent" is not resolved for collections. Say what a new slot holds (for example the declared reset default, or a `Fresh` zero) or that a candidate changing the length requires a conjunct. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:461-466 | missing-requirement |
| FND-002 | medium | AC-26 and TC-1270 expected results assert `Sequence<Timer>[0, 3]` has 85 values, but step 5 grants only `sr: Sequence<Timer>[0, 2]` (21), so the 85 case and the planner's headline example have no procedure step and no field in any model; the expected result for it cannot be observed. Also step 5 says "a separate model" but uses `Cfg`, `Tick`, `Timer`, `Open` and the `time dense unit ms` declaration without saying the model declares them. Add `Sequence<Timer>[0, 3]` to the granted fields and name the types. | spec/test-cases/TC-1270-a-record-or-tuple-of-finite-non-clock-components-is-a-finite-value-domain.md:57-62 | missing-requirement |
| FND-003 | low | The element rule covers `Clock` and an alias but `Sequence<Option<Clock>>` (FR-230-AC-19's own `q`) is neither a "clock place" element in this bullet's wording nor sized: the record bullet treats `Option<Clock>` as a clock place with no factor, this bullet defines `p` only for a bare clock, a record/tuple, or a finite `E`. State whether presence of an `Option<Clock>` element adds a factor (it should not, per the record rule). | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:440-452 | missing-requirement |
| FND-004 | low | AC-26 has no order check for a sequence of a non-pure-clock record of a clock-bearing element beyond `Cfg` (no ordered case for `Sequence<Timer>`, whose order is by `n` and ignores `x`), and no negative for `Set<Cfg>[0, 1]`/`Bag`/`Map` that the new bullet names as unbounded, so the stated Set/Bag/Map exception is not pinned. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:711 | missing-requirement |
| FND-005 | low | The order bullet carries rationale ("the cheapest order, since it needs no comparison between lengths") and an unneeded FR-215 appeal inside a SHALL; the list sentence calls the new item "the clock-bearing sequence" although `Sequence<Cfg>` with no clock is included. Wording only. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:406-407,453-460 | missing-requirement |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-006 | medium | The new slot rule is stated for "the same position of a sequence" only, but `Set<E>` and `Bag<E>` of a clock-bearing non-pure-clock element (`Set<Timer>[0, 2]`, `Bag<Timer>[0, 2]`) are now finite by the same bullet and have clock slots (FR-230-AC-19 lists `Call.b.[slot]`). Which pre-state slot a candidate's element keeps (by data part, by ascending order, by count) is not stated, and AC-26/TC-1270 grant only `Set<Cfg>`, `Bag<Cfg>` and `Set<Clock>`, so no clock-bearing Set/Bag is pinned. State the matching rule (or that a clock-bearing Set/Bag element other than a bare clock stays unbounded until ruled) and add one case. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:462-470 | missing-requirement |
| FND-007 | low | The new-slot rule says the added slot holds "a newly created clock at 0 in its declared unit (FR-230 \"Clocks and resets\")", but FR-230 states no creation value or default for a clock slot; `Fresh` there carries the constants of a written literal. The rule is the author's own, so cite it as this section's (as the tuple/clock-place bullet does) rather than as FR-230's. Not a contradiction with FR-230 (its Carried/Fresh provenance concerns a written conjunct, which the new text defers to). | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:467-470 | wrong-requirement |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Same-position slots keep the pre-state value or the FR-230 provenance when the collection is written; an added slot is Fresh at 0; AC-26 and TC-1270 step 5 work the length-1 to length-2 and empty pre-state cases. A Set/Bag residue is FND-006. |
| FND-002 | fixed | Step 5 declares dense time and the types and grants sr21 and sr85; 21 and 85 are each observable. |
| FND-003 | fixed | Option<Clock> is a clock place with p = 1; AC-26 `so: Sequence<Option<Clock>>[0, 3]` has 4 and step 5 grants it. |
| FND-004 | fixed | Set/Bag are now finite with sizes pinned (7, 22, 28, 2), Map, ordered set and Set with no maximum are step 6 negatives, and sr21/sr85 order by n is stated. |
| FND-005 | fixed | Rationale moved out of the SHALL bullets; list sentence now reads "sequence of a finite or clock element". |
| FND-006 | still-open | medium: Set/Bag clock-slot matching undefined; see New findings. |
| FND-007 | still-open | low: citation of FR-230 for a rule FR-230 does not state. |

## Verdict

Round 1: changes requested, narrow. FND-001 to FND-005 are fixed. Arithmetic verified: Set<Cfg>[0,1] 1+6 = 7; Set<Cfg>[0,2] 1+6+C(6,2)=15 = 22; Bag<Cfg>[0,2] 1+6+C(7,2)=21 = 28; Set<Clock>[0,2] p=1 gives C(1,0)+C(1,1)+C(1,2) = 2, matching FR-230 "Set<Clock> holding at most one element"; Bag p=1 gives N+1, matching FR-230 "one for each occurrence". Set, Bag and OrderedSet are QSL types (FR-230 "Clock collections", FR-106 spelling, TC-1171). Map and ordered set left unbounded with a follow-up note is acceptable. Count-then-ascending order for Set/Bag is consistent. quire validate passes; the scratch matrix lists FR-120-AC-22..26 rows.
