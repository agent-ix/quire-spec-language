---
id: TC-137
title: "Bound checked-term evaluation and retry immutable inputs"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-049, type: verifies }
  - { target: ix://agent-ix/quire-spec-language/NFR-009, type: verifies }
---
# TC-137: Bound checked-term evaluation and retry immutable inputs

## Description

Scope: FR-049-AC-7 to FR-049-AC-8 and NFR-009-AC-3.

Verify caller-owned accounting across the checked-term replacement entries,
including zero/no-work, exact and one-short boundaries, charge-before-work,
no partial result and a fresh-meter retry. Protocol-slot shared-meter
accumulation is exercised in TC-1073.

## Test Procedure

Use public checked-term/family entry points over admitted checked packages and
immutable inputs, driving each through a `quire-exact::Meter` built with the
`quire-exact` `test-support` feature, so that the meter's admitted-charge log
and `consumed` counters give an oracle that does not come from the evaluator
under test.

### Per-counter boundaries

For each of the ten counters of FR-255's accounting table, build one fixture
whose evaluation charges that counter at the charge point the second column
names (the points are `quire-exact`'s named `ChargePoint`s). Count the fixture's
quantity independently from its construction (the bit length of the operand,
the digits of the literal, the UTF-8 bytes of the text, the number of elements)
and compare it with the metered usage from a successful run under a sufficient
meter; the two must be equal. Then run the fixture with only that counter set
to zero (when the fixture requires work), to its counted quantity and to one
below it, holding the other nine sufficient. A high-water counter
(`integer_bits`, `decimal_digits`, `scale_expansion`, `text_input_bytes`,
`text_scalars`, `normalized_scalars`, `unit_edges`, `value_occurrences`) is
compared with the largest single charge; a cumulative counter (`work_units`,
`result_units`) is compared with the sum.

| Counter | Charge point the fixture must reach | Fixture quantity |
| --- | --- | --- |
| `integer_bits` | `integer-arithmetic.operands` | the bit length of the larger operand of an integer addition |
| `decimal_digits` | `decimal.operands` | the digit count of a decimal operand |
| `scale_expansion` | `decimal.scale-expansion` | the decimal-place shift of an addition of two decimals with different scales |
| `text_input_bytes` | `text.input-bytes` | the UTF-8 byte length of a text operand |
| `text_scalars` | `text.decode-scalars` | the Unicode scalar count of the same operand |
| `normalized_scalars` | `text.normalize-output` | the scalar count of a text operand whose normal form is longer than its input |
| `unit_edges` | `unit.edge` | the edges of a unit conversion chain |
| `value_occurrences` | `collection.element` | the elements of a sequence |
| `work_units` | `function.call` | the calls of a nested predicate chain |
| `result_units` | `collection.result-retain` | the units of a retained sequence result |

The meter's admitted-charge log must hold the named charge point for the
fixture; a fixture whose log lacks it does not stress its counter. Include a
no-work case at zero, and raise a caller-set counter above its default to
complete work that exceeds the default. A counter that no checked-term entry
point charges is not applicable under FR-049-AC-7; the case records, from the
admitted-charge logs of every fixture of this test case, that the counter is
never charged, and names no counter as inapplicable without that record.

### Retry, stack depth and typed causes

After each resource stop, retry the same immutable checked package and inputs
using a fresh sufficient meter and compare the result and usage with the
sufficient run. On a 512 KiB stack, evaluate a 100,000-record-deep input, a
10,000-call nested predicate chain and a 10,000-object graph chain.

For NFR-009-AC-3, run two controls through a checked-term call and compare the
`Incomplete` record the call returns:

| # | Control | Expected |
| --- | --- | --- |
| C1 | a meter whose `work_units` limit is `u64::MAX` and whose `work_units` consumption has been raised to `u64::MAX` by a prior charge, then a call that needs one more unit | `Incomplete` naming `work_units`, with `consumed` `u64::MAX` and the denied amount; no completed value, no wrapped or saturated count |
| C2 | a meter built with an injected denial of the first `function.call` charge (a stand-in for an allocator refusal, which denies a charge for a reason other than a configured limit) | `Incomplete` naming the counter of that charge point, `charge_point` `function.call`, `limit` equal to `consumed`; no completed value |

Both controls classify the result from the record's fields, never from
diagnostic text, and a retry of C1 and C2 with a fresh meter completes.

## Expected Results

No-work at zero and each exact-usage run complete. A one-short run stops before
the denied operation, identifies the counter, and returns no partial completed
value. Fresh-meter retries complete with the same value and leave checked
packages and runtime inputs unchanged. The deep-body and graph cases complete
without native-stack overflow. Raising a caller-set counter above its default
allows the fixture to complete work beyond that default. Each charged counter
and charge point has an independent oracle (the counted fixture quantity and
the admitted-charge log); omitting one charge makes its corresponding mutation
control fail. C1 and C2 return the meter's typed incomplete record with the
affected counter and charge point and expose no completed value. TC-1073 separately proves accumulation
across sequential protocol-slot calls on one shared meter.

The protocol-slot boundary trace is
`qsl-eval/tests/it/protocol_slots.rs::protocol_slot_calls_share_meter_usage_and_obey_work_boundaries`.
It measures a successful result, completes at the exact work-unit limit,
refuses one unit short without charging the denied unit, refuses at zero, and
repeats the immutable call with a fresh meter to verify the same result and
usage. Its trace tags include TC-137, TC-1073, FR-049-AC-7/8 and NFR-009-AC-1/2/4.
