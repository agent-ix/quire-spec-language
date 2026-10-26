---
id: SR-1646002
title: "Gap analysis of quire-spec-language PR #1646: clock places through package records (580h)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@790e70f3dbcb6d5f8b90de0e38a47f4be6df0ff0; PR #1646 against FR-230 (Records, tuples and unions; Places; Parameters; AC-11, AC-14 parts; AC-24, AC-26 deferrals), FR-120 typed value forms (AC-16) and TC-685; QSpec FR-181 typed canonical form (context); FCD semantic-ir 50c00e9 rules and schema (context)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: reviews
---
# Gap analysis of quire-spec-language PR #1646

## Summary

Ticket: QSL-580, slice 580h. Plan completion: not assessed.

What the tests cover:
- **Part of FR-230-AC-11.** The places `t.x`, `u.x` and `x` and their paths, the guard on `t.x`, the reset from the record literal, the data read and write of `n`, the two refusals, and the `Timer` node listing `n` only. In qsl-eval, nine places over three objects and the record digest.
- **Part of FR-230-AC-14.** The tick and untimed refusals.
- **Record parameter paths.** `inner(self.t)` and `fresh(self.u)`, a non-place argument, and a whole-record read in a body.

I checked the coder's claims and deferrals against the code and the FCD IR (semantic-ir 50c00e9):
- **No-scale package unit cannot occur (580g `None if unit.package`): verified.** The IR schema requires `scale` and `offset`. `unit_graph::decide` refuses a non-canonical or out-of-i128 rational, a zero scale, a missing target and a unit cycle. Intake refuses any Error diagnostic. So `package_unit_scale` cannot return `None` for an admitted package. The branch is a non-panicking guard and should stay.
- **Tuple field `p: (Clock, Int)`: verified.** The IR schema has no tuple type, so a package cannot declare one (STD-493 item 16).
- **Clock only as a required single field: verified.** `CLOCK_OUTSIDE_FIELD` (rules.rs:1207) and `INVALID_CLOCK_FIELD` (multiplicity `[1,1]`, presence `required`, not nullable). IR-1036 (Backlog) is the ticket for the other positions. AC-12, AC-13 and the Clock parts of AC-16/17/22/23/25 are correctly blocked on it.
- **A record-typed field can still be optional or a collection.** The PR refuses that `TimedNotChecked`. See FND-002.
- **AC-14 "at its span".** A package field has an `origin` locus in the package's own source, not a span in the unit. Placing the refusal at the time declaration or model declaration and routing the span to STD-493 is reasonable.
- **Clock atom (FR-231-AC-4) and `peer_clock` (AC-10) move to 580i.** `ReferencedClock` stays an honest not-checked refusal on the parameter path.
- **AC-24 and AC-26:** see FND-005.

## Verdict

**CONDITIONAL.** There are two medium coverage gaps, both on behaviour this
slice builds, and three low items. The code-review findings in SR-1646001
(FND-001 high) decide whether the PR can merge.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FR-181 typed forms this slice adds to the state key have one test: a record's data field changes the digest. There is no byte-level test of the `record`, `tuple` or `union` form. There is no test of nesting either: a record in a record, `Option<Timer>`, a set of records (with element order), or a union payload. FR-120-AC-16 asks for exactly these bytes for `tag`, `pair` and `shape` in the state key. TC-1195 (`encode.rs`) still tests only the UUID, timestamp and quantity families. Add unit tests in `encode.rs` (TC-1195, FR-120-AC-16) covering the record, tuple and nullary union bytes, plus one nested case through an option and a set. | qsl-eval/src/simulation/model/encode.rs:261-300, 317-430, 570-690 |
| FND-002 | medium | A new refusal has no test. Under a dense source, a model with a clock place under an optional member or in a collection (for example `ts: Sequence<Timer>` or `o?: Timer`) refuses `timed-not-checked` (`TimedConstruct::Clock`) at the time source. This is reachable: the IR admits an optional or multi-valued record-typed field. Nothing exercises it. FR-230-AC-14's `ts: Sequence<Timer>` under tick and untimed also takes this slice's code path and has no test; the test doc says it belongs to the collections slice. Add one test per branch. | qsl-semantics/src/check/timed_model.rs:1604-1618; qsl-semantics/tests/it/clock_places.rs:198-221 |
| FND-003 | low | Separation is tested only partly. The digest test shifts every clock by the same delay and changes `n`. No test shows that two states with the same values in different places (`t.x = 1, t.y = 0` against `t.x = 0, t.y = 1`) get different digests. That is the case where the new `path` member matters. Separately, AC-11's "a write of `n`" after the reset cut is stated in the test doc but not asserted (only the reset is). | qsl-eval/tests/it/timed_subject.rs:1140-1206; qsl-semantics/tests/it/clock_places.rs:84-150 |
| FND-004 | low | Spec conflict, not a code defect. QSL FR-120 writes a record, tuple or union `name` as the checked node id in 64 hex digits. It states that "FR-181's `<qualified-name>` leaves the text form to the implementation". QSpec FR-181-AC-19, however, fixes the bytes `"name":"Shop::Item"`, `"Shop::Pair"` and `"Shop::Shape"`, so a QSL key can never reproduce AC-19. QSL also uses two different name forms: an object's `record` name is the declaration identity text, while a record value's name is the hex node id. Either QSpec FR-181 should say the name text is implementation-defined and make AC-19's names placeholders, or QSL should adopt the qualified name. File this on STD-493. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:140-148, 225-237; quire-specification FR-181-AC-19 |
| FND-005 | low | Deferral routing does not agree, and AC-26's blocker has no ticket. The PR body sends AC-10/24, record collections and AC-26 to 580i. The coordinator brief says AC-24/26 are blocked by FCD IR rules (IR-1036) and that record collections (AC-19..21, 23) go to 580j. IR-1036 covers `Clock` in non-field positions only. That fits AC-24 (`Reference<Clock>` is a non-field Clock position) but not AC-26, which has no Clock in it. AC-26's alias of `ix://quire/unit/ms` falls under the IR's quantity-position rule instead ("a quantity type reference is permitted only on a field, parameter or result", rules.rs:1209). Fix: route AC-26 to its own IR ticket or widen IR-1036, and make the 580i/580j routing the same in the PR body and on QSL-580. | FR-230-AC-24, FR-230-AC-26; filament-core-data crates/semantic-ir/src/rules.rs:1196-1212 |
