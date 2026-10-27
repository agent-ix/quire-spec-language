---
id: SR-1682001
title: "spec-review of FR-120 finite record and tuple value domains (quire-spec-language PR 1682)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-120-simulate-a-checked-package-s-state-family.md; spec/functional/FR-215-implement-protocolsystem-as-a-transitionsystem.md; spec/test-cases/TC-1270-a-record-or-tuple-of-finite-non-clock-components-is-a-finite-value-domain.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-215
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: references
---

# Spec review of STD-508 finite record and tuple value domains (quire-spec-language PR 1682)

## Summary

Ticket: STD-508 (and QSL-580o under QSL-580). One spec-review pass over
`git diff origin/main...HEAD` of the PR branch. Checked against ADR-027 (ST-11, TS-5), STD-493
item 21, FR-215, FR-230 ("Clocks and resets", "Records, tuples and unions"),
FR-097-AC-2, FR-120 "Bounds" and "Post-states", FR-120-AC-16, TC-846, and the
`qsl-eval` simulation domain code (`model/domain.rs`, `model.rs` `domains()`,
`model/frames.rs`, `model/application.rs`).

Hand check of the arithmetic: Cfg 2 x 3 = 6, Pr 2 x 2 = 4, Wrap 6 x 3 = 18,
put 6 x 4 x 18 = 432, `Option<Cfg>` and `Sequence<Cfg>[0, 1]` 1 + 6 = 7 each,
Timer 4, Stopwatch 1, Mix 4 x 2 = 8, and every listed order (first component
most significant, `false` before `true`, absent first), are correct.
`quire validate` on the three changed documents passes (exit 0; only module
duplicate-archetype warnings). AC-22 to AC-24 are appended after AC-21 and TC-1270
is unused on main and on every open PR branch.

ADR-027 ST-11 says only that the attempt record ranges over "`T`'s finite record
domain as ST-2's records range over theirs"; TS-5 says an unbounded root returns
`requires-bound`. Clock components, tuples, and the first-field-most-significant
order are stated by FR-215 and FR-230, not by the ADR. STD-493 item 21 asks for
"a record or tuple of finite components ... at least for a clock-bearing record";
the STD-508 ticket asks for a record or tuple, so the tuple rule is in the ask.
The enum addition is not in either.

Enum: in `model.rs` `domains()`, a type for which `domain::enumerable` is false
(`ValueType::Enum` and `Composite` included) is listed as the unbounded `Integer`
stand-in, so today a bare enum parameter, result or granted field returns
`RequiresBound`. No test in `qsl-eval/tests`, no AC and no TC expects that for an
enum (grep of FR-120, FR-097, FR-146, TC-471..473, TC-846, TC-1195, ADR-014/016),
and FR-120-AC-16's enum field is in a state that no frame grants. The enum
addition is consistent with FR-215 and is needed for a record with an enum field,
but it changes the bare-enum outcome with no criterion that pins it.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-120 gains `depends_on` FR-215 while FR-215 already `depends_on` FR-120: a two-node dependency cycle. FR-215's value-domain bullet also delegates a field's domain to FR-120 while the new FR-120 text delegates the enum and the record rule back to FR-215, so each requirement defines the rule by pointing at the other. Make one the owner (FR-120 for model value domains, FR-215 citing it) and use `references`, not `depends_on`, for the back edge; FR-215 also lacks a FR-230 edge for its new Clock sentence. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:24 | wrong-requirement |
| FND-002 | medium | The enum member added to FR-120's finite value-domain list changes a bare enum-typed parameter, result or granted field from `RequiresBound` (what `domains()` returns today) to finite, and no criterion or test case pins it: AC-22 to AC-24 and TC-1270 use no enum at all. It is not in the ticket or STD-493 item 21. It is justified (FR-215 already enumerates enum fields, and a record with an enum field must be finite for one record type to have one domain), but add an enum field to `Cfg` in AC-22 and a bare enum parameter case, or split the enum change into its own ticket and AC. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:406 | missing-requirement |
| FND-003 | medium | AC-23 and TC-1270 step 2 give no frame that grants `ck` or `sw` and no `Dense` time source. `domains()` lists a root only for a field a `modifies` entry grants (FR-120 "Bounds"), so "`domains()` lists no unbounded root for `ck`" is vacuous, and `self.ck = Timer { x: 0 ms, n: 3 }` writes a field outside `put`'s frame (`frame_violation`). FR-230 "Clocks and resets" refuses a `Clock` field in a model whose source is not `Dense` (`invalid_timed_model`/`clock-without-dense-time`) and the `0 ms` literal needs a unit. State the frame (`modifies ck`), the `time dense unit ms` declaration and which operation holds the postcondition. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:662 | wrong-requirement |
| FND-004 | medium | The clock-place rule names only `Clock` and `Option<Clock>`. FR-230 also has an alias of `Clock`, an optional clock field, clock collections (`Sequence<Clock>`, `Sequence<Timer>`, FR-230-AC-16 and AC-19 to AC-24, with `Carried`/`Fresh` slot provenance) and `Reference<Clock>` (data, not a clock). By the new text a `Sequence<Clock>[0, 2]` component has no finite domain (its element is not in the list) and a `Sequence<Timer>[0, 2]` component is finite over `Timer`'s non-clock domain with its clock slots unspecified. A record component declared `note?: T` (grammar form in FR-120-AC-16's `Tag`) has no stated domain; `qsl-eval` `Space` treats it as absent then each value, one extra value. Say which of these are clock places or data, and what the optional component and the sequence-of-clock-bearing cases contribute. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:413 | missing-requirement |
| FND-005 | medium | The text says an enum rule makes one record type have one domain "in a model and in a protocol", but a union is stated unbounded in `domains()` while `qsl-eval` `enumerable_in`/`Space` already enumerates a union of finite members for an attempt type (ADR-027 ST-11 test `a_union_attempt_type_enumerates_every_member`), so a record with a union field is finite in a protocol and unbounded in a model. Either state the union rule for both (and enumerate it in FR-120) or narrow the "one domain" sentence and name the union and optional-field exceptions. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:407 | wrong-requirement |
| FND-006 | low | AC-22's last sentence, "A record or tuple with `Option<Cfg>` or `Sequence<Cfg>[0, 1]` as a component has 7 values", is true only when that is the sole component (a record with another component multiplies), and TC-1270 step 1 tests a field of that type, not a record or tuple. Step 1 also adds those fields to the model that gave 432, so the 432 and the 7 cannot both hold of one model. Word it as a field or a one-component record and state a separate model. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:661 | wrong-requirement |
| FND-007 | low | AC-24 names `Wrap2 { o: Open }` as unbounded but gives no field of that type, so it is no root and TC-1270 step 3 cannot name its `WireNodeId`. `Tm { x: Clock, label: Text }` relies on the FR-120 list that makes `Text` unbounded, but FR-120 "Bounds" says RequiresBound comes from `classify_extent`, and FR-097-AC-2 gives `Text` no unbounded domain ("every other type adds none"). Use an `Integer` component for the `Tm` case or state the mechanism. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:663 | wrong-requirement |
| FND-008 | low | The clock pre-state is left implicit: "The explorer does not enumerate or write a clock place as a value; FR-230's reset of it is stated by the postcondition". FR-230 "Clocks and resets" already says a clock the operation does not reset keeps its value and the snapshot carries no clock value (FR-230-AC-11: the checked `Timer` node lists `n` only). Cite that sentence so a candidate over a record with no postcondition is unambiguously the pre-state clock. The empty record and clock-only edge ("exactly one value") is stated in FR-120 and AC-23's `Stopwatch` but a record with no component at all is not tested. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:417 | missing-requirement |
| FND-009 | low | The new value-domain text is one run-on sentence of about fifteen lines with no SHALL, mixing the list, the order, the size, the clock rule, the unbounded rule and a nested-clock-record rule; split it into one statement per rule. The FR-215 edit also adds a Clock rule to protocol binders that STD-508 (FR-120 only) and ADR-027 do not state; say so in the PR or ticket. | spec/functional/FR-215-implement-protocolsystem-as-a-transitionsystem.md:167 | wrong-requirement |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-010 | low | FR-120's last new bullet says a record or tuple has the same domain and order here and in FR-215 for every component type except a union, "which FR-215 enumerates for a protocol attempt type". FR-215 as it stands on main states neither: it has no clock-place rule, no tuple, no optional-field rule and does not mention unions (only the qsl-eval code and ADR-027 ST-11's test do). Reword to what FR-215 says (record fields: finite-field records, enum members, nested records) or restore an FR-215 sentence. The enum bullet and AC-23 also carry status prose ("before this criterion a bare enum is listed as unbounded", "QSL-580o's implementation gap"), which belongs in the ticket. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:441 | wrong-requirement |
| FND-011 | low | AC-24 and TC-1270 step 3 read the domain of `Mix { t: Timer, b: Boolean }` but declare no field of that type, and `domains()` lists fields, so `Mix` has no root to read; make it a granted field. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:683 | wrong-requirement |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-120's edges to FR-215 and FR-230 are now `references`; FR-215 is back to main, so no cycle and no FR-215 edge is owed. |
| FND-002 | fixed | The enum rule is its own SHALL bullet and FR-120-AC-23 and TC-1270 step 2 pin a bare parameter, result, granted field, and a record with an enum; Color 3, Pick 3 x 2 = 6 are correct. |
| FND-003 | fixed | AC-24 and step 3 declare `time dense unit ms`, grant `ck, sw` in `retime()` with the postcondition, and say a clock keeps its pre-state value. |
| FND-004 | fixed | The clock-place bullet names Clock, an alias, `Option<Clock>` and a `?` field of Clock; `Sequence<Clock>`, `Sequence<Timer>` and `Reference<Clock>` are stated unbounded; a presence-optional field is absent then each value (`Note` 3 x 2 = 6). |
| FND-005 | fixed | The "one domain" claim now excepts a union. Its remaining wording about FR-215 is FND-010. |
| FND-006 | fixed | AC-22 gives the 7-value fields in a separate model and a two-component `R1` of 6 x 2 = 12. |
| FND-007 | fixed | AC-25 grants `wr: Wrap2` as a field and uses `k: Integer` in `Tm`, so it no longer relies on `Text`. The FR-097-AC-2 vs FR-120 Text difference remains pre-existing and is not worsened (Text is no longer in the list). |
| FND-008 | fixed | The bullet cites FR-230 "Records, tuples and unions" for the pre-state; `Stopwatch` and the no-component rule remain. |
| FND-009 | fixed | The value-domain text is split into one SHALL bullet per rule; the FR-215 edit is gone. |
| FND-010 | still-open: low, wording only; fix in this PR or accept. | The FR-215 comparison and status prose remain. |
| FND-011 | still-open: low, wording only. | `Mix` has no granted field. |

## Verdict

Round 2: PASS. FND-001 to FND-009 are fixed, quire validate passes on FR-120, TC-1270 and this file, ids AC-1 to AC-21 are untouched and AC-22 to AC-25 are new (nothing on main cites them), and the new vectors' arithmetic is right. FND-010 and FND-011 (low) remain and do not block.

Round 1 verdict (superseded): changes requested: not mergeable until FND-001 to FND-005 (medium) are fixed; the
low findings should be fixed in the same round. The core rule (finite non-clock
components enumerate, first component most significant, clock places add no factor,
any unbounded non-clock component stays `RequiresBound`) matches STD-508, STD-493
item 21, FR-215 and FR-230.

Implementation gap for QSL-580o (not fixed here). `qsl-eval/src/simulation/model/domain.rs`:
`enumerable()` and `Domain::of()` return false/`None` for `Enum` and `Composite`, so
`model.rs` `domains()` lists them as the unbounded `Integer` stand-in and
`model/frames.rs` `writable_fields()` skips the field. A `Space`
enumerator (enum, record, tuple, optional field absent first, union; first field
most significant) exists but is used only for protocol attempt types through
`application.rs` `values_of`/`enumerates` and `enumerable_in`; the model path
(`domains()`, `writable_fields`, post-state candidate generation, creatable-type
field domains, result domains) must use it, skipping clock components, keeping unions
and any other type unbounded as FR-120 says, and counting candidates against
`max_candidates`. No test covers a model field of record, tuple or enum type with a
granting frame: TC-1270 / FR-120-AC-22 to AC-24 have none.
