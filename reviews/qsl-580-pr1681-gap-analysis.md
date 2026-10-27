---
id: SR-1681002
title: "Gap analysis of quire-spec-language PR #1681: min/max bounds on clocks and units (580n part 1)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@18ba20b8d; PR #1681 against FR-230-AC-25, FR-230-AC-26 and TC-685; QSpec FR-154 Unit bounds and FR-154-AC-28, AC-29, AC-31 (origin/main, context)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
---
# Gap analysis of quire-spec-language PR #1681

## Summary

Ticket: QSL-580, slice 580n part 1. Plan completion: not assessed.

**AC-25.** `a_max_constraint_on_a_clock_alias_bounds_its_places` covers:
- the alias bound on `w` only;
- the `Clock` bound on every place;
- `min` refusing `time-invariant-shape`;
- `minLength` and a negative operand refusing `malformed-declaration`.

`a_max_constraint_on_a_clock_alias_bounds_the_live_places` covers the live
slots at qsl-eval.

**FR-154-AC-29 and the scope of the run-time half of AC-26.**
`unit_bounds_bound_the_members_of_their_unit_or_alias` covers:
- the bad operands `1/0`, `0.5`, `2/4` and `""`;
- crossed bounds on one type;
- an alias bound scoped to its alias;
- a unit bound reaching alias members;
- `s` left unbounded;
- `MemberBound` used directly.

**Correctly parked.**
- The static half (`Quantity<u>[min, max]`, FR-154-AC-31's literal refusal) is blocked on IR-1051 (quire-exact #91).
- The end-to-end AC-28 checks are blocked because no state can hold a quantity yet.
- Map values are IR-1046.
- The `Option<Timeout>` presence guard waits on `Option<Clock>` places, later in this stack.

## Verdict

**CONDITIONAL.** FND-001 is medium; the rest are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-25's last clause is neither built nor tested: "the same constraint under a `time tick` or untimed unit refuses `clock-without-dense-time`". The test doc calls it "Not covered". A clock-alias bound is refused anyway when a clock place exists. But a `max` with `appliesTo` `ix://quire/native/Clock` on an object type with no clock field, under `time tick`, appears to reach no refusing path (I have not probed it), and FR-230 requires the refusal. AC-25's `format` keyword and a non-rational operand such as `"abc"` are also untested; the code refuses both at `keyword` and `operands`. | FR-230-AC-25; qsl-semantics/tests/it/clock_places.rs |
| FND-002 | low | No test exercises a unit bound on a quantity inside a record (SR-1681001 FND-001), or crossed bounds declared on two different type nodes with the same `appliesTo` (SR-1681001 FND-003). | qsl-semantics/tests/it/unit_typed_members.rs |
| FND-003 | low | Intake refuses an alias of an alias of `Clock` or of a unit (580m's "not a clock or a unit this reader aliases"). Yet FR-154 scopes an alias bound to members "typed by that alias or an alias of it", which assumes chains can be declared. That is a QSL intake gap against QSpec FR-154, and no ticket names it. Track it with 580m's alias intake. | qsl-semantics/src/model/intake.rs (read_type_node alias branch); QSpec FR-154 Unit bounds |
