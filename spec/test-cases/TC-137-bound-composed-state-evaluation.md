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
under test. Compute each semantic-size counter independently from every
charge point reached by the fixture, then take its specified high-water
maximum. For arithmetic, use the shared charge formulas for the intermediate
size allowance (for example, integer addition charges
`max(bits(a), bits(b)) + 1`, and decimal multiplication charges
`digits(a) + digits(b)` after any operand-derived alignment). For collection
construction, include both the per-element charges and the retained value's
occurrences: a three-element collection has `max(3, occ(result)) = 4` because
`occ` includes the outer collection. `result_units` separately sums retained
output occurrences; temporary intermediate values are not retained results.

### Per-counter boundaries

For each of the ten counters of FR-255's accounting table, build one fixture
whose evaluation reaches the applicable charge points in the second column
(the points are `quire-exact`'s named `ChargePoint`s). Derive each fixture's
expected amount independently using the formulas in the third column. Then run
the fixture with only that counter set
to zero (when the fixture requires work), to its counted quantity and to one
below it, holding the other nine sufficient. A high-water counter
(`integer_bits`, `decimal_digits`, `scale_expansion`, `text_input_bytes`,
`text_scalars`, `normalized_scalars`, `unit_edges`, `value_occurrences`) is
compared with the largest single charge; a cumulative counter (`work_units`,
`result_units`) is compared with the sum.

| Counter | Charge point the fixture must reach | Independently counted amount |
| --- | --- | --- |
| `integer_bits` | `integer-arithmetic.operands` and `integer-arithmetic.arithmetic` | the maximum of the operand size charge and arithmetic allowance: `max(bits(a), bits(b)) + 1` for addition/subtraction, `bits(a) + bits(b)` for multiplication, or `bits(a)` for unary negation |
| `decimal_digits` | `decimal.operands`, `decimal.scale-expansion`, `decimal.arithmetic`, applicable `decimal.rounding` and `decimal.result-retain` | the maximum amount from the full schedule: operand digits, operand-derived aligned digits, the arithmetic formula (addition/subtraction `max(G1,G2)+1`; multiplication `G1+G2`; division `max(G1,G2)`) and the retained coefficient's `sdigits(c,k)`; rounding reuses the arithmetic amount |
| `scale_expansion` | `decimal.scale-expansion` and `decimal.result-retain` | the high-water maximum of the operand-derived alignment shift and any retained-coefficient upscaling shift; use a fixture with output scale equal to the aligned operand scale so the latter is zero |
| `text_input_bytes` | `text.input-bytes` | the UTF-8 byte length of a text operand |
| `text_scalars` | `text.decode-scalars` | the Unicode scalar count of the same operand |
| `normalized_scalars` | `text.normalize-output` | the scalar count of a text operand whose normal form is longer than its input |
| `unit_edges` | `unit.edge` | the edges of a unit conversion chain |
| `value_occurrences` | `collection.element`, `collection.bound` and `collection.result-retain` | the high-water maximum of element, bound and retained-result charges; a collection with three elements has four occurrences including its outer value |
| `work_units` | `function.call` | the calls of a nested predicate chain |
| `result_units` | `collection.result-retain` | `occ(result)` of the completed retained sequence, including the outer collection and every nested occurrence; temporary intermediate sequences do not count as retained results |

The meter's admitted-charge log must hold every named charge point for the
fixture; a fixture whose log lacks one does not stress that schedule. Derive
the expected counter amount from the authored operands, alignment, arithmetic
formula, and retained result shape above, then compare it with the successful
meter usage; never use the meter reading itself as the oracle. Include a
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
| C2 | a meter built with an injected denial of the first `function.call` charge (a stand-in for an allocator refusal, which denies a charge for a reason other than a configured limit) | `Incomplete` with `limit_kind` `work_units` (an injected denial is always recorded against `work_units`), `charge_point` `function.call`, `limit` equal to `consumed`; no completed value |

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
