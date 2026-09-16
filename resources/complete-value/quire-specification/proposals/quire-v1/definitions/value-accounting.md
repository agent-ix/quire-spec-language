# Complete scalar accounting definition

Definition identity: `quire.value.accounting/v1`; revision: `1-draft.1`.
This is a leaf definition selected by `quire.value.complete/v1`; it has no
reverse dependency on that root.

## Limit object

Every complete-value request carries `ScalarLimitsV1` with exact unsigned
64-bit (`u64`) limits for `integer_bits`, `decimal_digits`, `scale_expansion`,
`text_input_bytes`, `text_scalars`, `normalized_scalars`, `unit_edges`,
`value_occurrences`, `work_units` and `result_units`. Omission is a refused
request. Every consumed counter is also an exact `u64`; a semantic size or next
charge greater than `u64::MAX` returns incomplete at that named counter before
narrowing, allocation or arithmetic. Zero is a real limit and never means
unlimited.

## Ordered charge points

The following identifiers and order are normative. A charge is made before the
named expansion, operation or retention. The first unavailable charge returns
`incomplete { limit_kind, limit, consumed, next_charge, charge_point }` and no
completed result. That record is closed; its members are defined under
[Incomplete record](#incomplete-record).

| Operation family | Ordered charge points |
| --- | --- |
| decimal | `decimal.operands`, `decimal.scale-expansion`, `decimal.arithmetic`, `decimal.rounding`, `decimal.result-retain` |
| text | `text.input-bytes`, `text.decode-scalars`, `text.normalize-input`, one `text.normalize-output` before each emitted scalar, `text.result-retain` |
| enum | `enum.identity-read`, `enum.result-retain` |
| unit | `unit.identity-read`, one `unit.edge` before each source/target edge, `unit.rational-arithmetic`, `unit.target-domain`, `unit.result-retain` |
| integer division | `integer-division.operands`, `integer-division.arithmetic`, `integer-division.domain-pair`, `integer-division.result-pair` |
| integer modulus (`mod`) | `integer-modulus.operands`, `integer-modulus.arithmetic`, `integer-modulus.domain`, `integer-modulus.result-retain` |
| IEEE | `ieee.operands`, conditional `ieee.exact-intermediate`, conditional `ieee.round`, `ieee.result-retain` |
| equality | `equality.plan-form`, `equality.plan`, one `equality.pair` for each planned semantic occurrence-path pair, then `equality.result-retain` |
| function call | one `function.call` before binding the evaluated arguments of each checked pure-function call |
| collection | one `collection.element` before each constructor element expression or one `collection.visit` before binding each visited occurrence, one `collection.member-walk` then one `collection.member-test` for each membership comparison, then `collection.bound` and `collection.result-retain`, in the FR-144/FR-145 order |
| composite construction | `composite.result-retain` after the last record field or tuple argument completes |
| integer arithmetic | `integer-arithmetic.operands`, `integer-arithmetic.arithmetic`, `integer-arithmetic.result-retain` for each `Integer` or `Int[..]` `+`, `-`, `*` and unary `-` |
| rational arithmetic | `rational-arithmetic.operands`, `rational-arithmetic.arithmetic`, `rational-arithmetic.normalize`, `rational-arithmetic.result-retain` for each `Rational[..]` `+`, `-`, `*`, `/` and unary `-`, and each `Integer` or `Int[..]` `/` producing a `Rational[..]` |
| numeric ordering | `ordering.operands`, `ordering.arithmetic`, `ordering.result-retain` for each `Integer`, `Int[..]`, `Rational[..]` or `Decimal[..]` `<`, `<=`, `>` and `>=` |
| Boolean connective | `boolean.result-retain` for each `and`, `or`, `implies` and `not` |

Each charge is the following exact `{ counter: amount }` vector. A semantic-size
counter records the high-water maximum, not a sum: admitting amount `n` changes
its consumed value to `max(consumed,n)`. `work_units` and `result_units` are
cumulative additions. Thus counter aggregation is independent of batching or an
implementation's internal algorithm.

Let `bits(x)` be the magnitude bit length of a mathematical integer (zero is
one), `digits(x)` its base-ten magnitude digit count (zero is one), `occ(v)` the
number of typed value occurrences including the outer value and every nested
scalar/composite occurrence (`none` counts one; a present option counts one plus
its payload; a record or tuple counts one plus its present field values, while an
`absent` or `null` slot counts zero; a collection counts one plus every
occurrence, a bag occurrence once per multiplicity; a reference counts one), and `maxparts(r)` the maximum `bits` of a reduced
rational's numerator and positive denominator.

| Charge point | Exact counter amount before adding one `work_unit` |
| --- | --- |
| `decimal.operands` | `integer_bits=max(bits(coeff_i))`, `decimal_digits=max(digits(coeff_i))`, `value_occurrences=operand_count` |
| `decimal.scale-expansion` | `scale_expansion=max decimal-place shift in this operation`, `integer_bits`/`decimal_digits` of every expanded coefficient |
| `decimal.arithmetic` | `integer_bits=maxparts(exact intermediate)` and `decimal_digits=max(digits(numerator),digits(denominator))` |
| `decimal.rounding` | `integer_bits=bits(rounded coefficient)`, `decimal_digits=digits(rounded coefficient)`; charged only under the decimal applicability rule below |
| `decimal.result-retain` | semantic sizes of the completed decimal's retained representation (`v × 10^T`, `T`), `value_occurrences=1`, `result_units += 1` |
| `text.input-bytes` | `text_input_bytes=sum UTF-8 bytes of all operands`, `value_occurrences=operand_count` |
| `text.decode-scalars` | `text_scalars=sum decoded scalars of all operands` |
| `text.normalize-input` | no size-counter change |
| kth `text.normalize-output` | `normalized_scalars=k` across all result-side normalized operand sequences |
| `text.result-retain` | `result_units += 1` for a text or Boolean result |
| each `enum.identity-read` | `value_occurrences=number of enum operands read so far` |
| `enum.result-retain` | `result_units += 1` |
| each `unit.identity-read` | `value_occurrences=number of quantity operands read so far`, `integer_bits=maxparts` over their exact rational values |
| kth `unit.edge` | `unit_edges=k` across all source/target root paths |
| each `unit.rational-arithmetic` | `integer_bits=maxparts` over the exact rational result of the scheduled operation |
| `unit.target-domain` | for a decimal target, `integer_bits` and `decimal_digits` of the retained coefficient; for an integer target, only `integer_bits=bits(rounded integer)`; zero for an unbounded rational target |
| `unit.result-retain` | `value_occurrences=occ(result)`, `result_units += occ(result)` |
| `integer-division.operands` | `integer_bits=max(bits(a),bits(b))`, `value_occurrences=2` |
| `integer-division.arithmetic` | `integer_bits=max(bits(a),bits(b),bits(q),bits(r))` |
| `integer-division.domain-pair` | `value_occurrences=2` |
| `integer-division.result-pair` | `result_units += 2` atomically |
| `integer-modulus.operands` | `integer_bits=max(bits(a),bits(b))`, `value_occurrences=2` |
| `integer-modulus.arithmetic` | `integer_bits=max(bits(a),bits(b),bits(q),bits(r))` for the Euclidean quotient `q` and remainder `r` |
| `integer-modulus.domain` | `value_occurrences=1` |
| `integer-modulus.result-retain` | `result_units += 1` |
| `ieee.operands` | `integer_bits=selected width`, `value_occurrences=arity`; for conversions, the source width or the exact source size in the IEEE conversion paragraph |
| `ieee.exact-intermediate` | `integer_bits=selected width`, or for conversions the target width or IEEE-to-exact `maxparts` in the IEEE conversion paragraph; this is the fixed semantic allowance for one exact-real operation at binary32/binary64, not a claim that an irrational square root is materialized as a rational |
| `ieee.round` | `integer_bits=selected width`, or the target width for conversions |
| `ieee.result-retain` | `result_units += 1` for bits/flags or Boolean |
| `equality.plan-form` | `value_occurrences=max(occ(left),occ(right))`; `work_units += occ(left) + occ(right)` instead of one; charged before plan formation walks either operand |
| `equality.plan` | `value_occurrences=exact total planned pair events`; without changing consumed counters, atomically require remaining capacity for `pair_events + 2` work units and one result unit, then consume the plan's one work unit |
| each `equality.pair` | no size-counter change; `work_units += 1` |
| `equality.result-retain` | `result_units += 1` |
| each `function.call` | no size-counter change; `work_units += 1` |
| each `collection.element` | no size-counter change; `work_units += 1` |
| each `collection.visit` | no size-counter change; `work_units += 1` |
| each `collection.member-walk` | `value_occurrences=max(occ(c),occ(m))` for candidate `c` and member `m`; `work_units += occ(c) + occ(m)` instead of one; charged before that comparison's plan formation |
| each `collection.member-test` | `value_occurrences=p`, where `p` is the FR-149 planned pair count of that one comparison; `work_units += p` instead of one |
| `collection.bound` | `value_occurrences=n`, the FR-144 bound count of the formed collection |
| `collection.result-retain` | `value_occurrences=occ(result)`, `result_units += occ(result)`; `result_units += 1` for a scalar query result |
| `composite.result-retain` | `value_occurrences=occ(result)`, `result_units += occ(result)` |
| `integer-arithmetic.operands` | `integer_bits=max(bits(operand_i))`, `value_occurrences=operand_count` |
| `integer-arithmetic.arithmetic` | `integer_bits=bits(exact result)`, derived before the result is materialized |
| `integer-arithmetic.result-retain` | `result_units += 1` |
| `rational-arithmetic.operands` | `integer_bits=max(maxparts(operand_i))`, an integer operand `n` taken as `n/1`, `value_occurrences=operand_count` |
| `rational-arithmetic.arithmetic` | `integer_bits=max(bits(N),bits(D))` of the unreduced exact intermediate `N/D` for operands `a/b` and `c/d`: `N=a×d±c×b`, `D=b×d` for `+` and `-`; `N=a×c`, `D=b×d` for `*`; `N=a×d`, `D=b×c` for `/`; `N=-a`, `D=b` for unary `-` |
| `rational-arithmetic.normalize` | `integer_bits=maxparts(reduced result)` |
| `rational-arithmetic.result-retain` | `result_units += 1` |
| `ordering.operands` | `integer_bits=max` over both operands of `bits` for an integer, `maxparts` for a rational and `bits(coefficient)` for a decimal; `decimal_digits=max(digits(coefficient))` for decimals, each decimal measured in its retained representation `(coefficient, scale)` as retained and never normalized; `value_occurrences=2` |
| `ordering.arithmetic` | for integers, `integer_bits=max(bits(a),bits(b))`; for rationals `a/b` and `c/d`, `integer_bits=max(bits(a×d),bits(c×b))`; for decimals in their retained representations `(c1,s1)` and `(c2,s2)`, never normalized, aligned to `s=max(s1,s2)`, `scale_expansion=|s1-s2|` and the `integer_bits` and `decimal_digits` of both aligned coefficients |
| `ordering.result-retain` | `result_units += 1` |
| `boolean.result-retain` | `result_units += 1` |

For each non-repeated row, `work_units += 1`; repeated identity, edge, rational,
normalization-output, equality-pair, function-call, collection-element and
collection-visit rows add one per event, and `equality.plan-form`,
`collection.member-walk` and `collection.member-test` add their stated amounts.
Retaining a paired quotient/remainder costs two result units; a composite retained value costs
`occ(result)`. Checked amount derivation and counter arithmetic precede
allocation or semantic arithmetic. A counter amount greater than `u64::MAX`
returns incomplete at that point. No unnamed precision, shift, host-backend cost
or implementation-work counter may change an outcome.

Unit rational events are representation-independent. Traversing one affine
unit edge schedules exactly two events: multiply then add in the declared
direction, or subtract then divide in the reverse direction, including identity
multipliers and zero offsets. After all required conversions, quantity add,
subtract, multiply or divide schedules one event for its exact mathematical
result; integer power schedules one event for the exact powered result,
independent of the exponentiation algorithm. Dimension/compound-unit map
normalization schedules no rational event. Thus U10 has one identity read, one
edge, two rational events, target admission and result retention: six work
units. Equality preflight is only an availability check: the subsequent pair
and result-retention charges are the sole mutations for those reserved units,
so E21, whose operands each have `occ = 16`, consumes exactly
`32 + 1 + 17 + 1 = 51` work units and one result unit.

FR-149 selects the equality schedule for a top-level Boolean, `Integer`,
`Int[..]`, `Rational[..]`, `Decimal[..]` or `Reference<T>` comparison as a
one-pair plan: `equality.plan-form` with two work units, `equality.plan` with
`value_occurrences=1`, one `equality.pair` and `equality.result-retain`, which
is five work units and one result unit, and no `decimal.*` or `integer-*`
charge. Every comparison that uses the FR-149 equality schedule, top-level or
nested, charges `equality.plan-form` before plan formation walks either
operand; a top-level text, enumeration or quantity comparison uses its own
FR-141 or FR-142 schedule and charges no `equality.*` point. Plan formation then makes its
checks, including the `foreign_reference` universe check, without a further
charge, and precedes `equality.plan`; a refused check stops after
`equality.plan-form`. Of the FR-149 equality conversions, an FR-142 unit
conversion charges its `unit.*` schedule and a `Decimal`-to-`Decimal` conversion
charges the decimal conversion schedule below. An `Int[..]` or
`Rational[..;d,1]` source converted to `Decimal[c1,c2;s1,s2;m]` charges the
same four decimal points on the integer `n`: `decimal.operands` with
`integer_bits=bits(n)`, `decimal_digits=digits(n)` and `value_occurrences=1`;
`decimal.scale-expansion` with `scale_expansion=s1` and the `integer_bits` and
`decimal_digits` of `n × 10^s1`; `decimal.arithmetic` with those same amounts;
and `decimal.result-retain` for the retained `(n × 10^s1, s1)`. A `Decimal`
source `(c, s)` converted to `Rational[..]` charges `decimal.operands` on
`(c, s)`; `decimal.scale-expansion` with `scale_expansion=s` and
`integer_bits=bits(10^s)`; `decimal.arithmetic` with `integer_bits=maxparts`
and `decimal_digits=max(digits(numerator),digits(denominator))` of the reduced
`c/10^s`; and `decimal.result-retain` with `integer_bits=maxparts` of that
rational, `value_occurrences=1` and `result_units += 1`. Every amount is derived
analytically before materialization, and none of these conversions has a
rounding step. Every other admitted equality conversion keeps the source
magnitude and has no charge point. Record and tuple construction charges
`composite.result-retain` after its last field or argument, and its field
expressions and calls charge their own points. Integer `+`, `-`, `*` and unary
`-` charge `integer-arithmetic.operands`, then `integer-arithmetic.arithmetic`,
then decide any FR-044 bounded-result membership without a charge, then
`integer-arithmetic.result-retain`. Boolean `and`, `or` and `not` charge
`boolean.result-retain` once their result is decided, and `implies` does the
same. `Rational[..]` `+`, `-`, `*`, `/` and unary `-`, and an `Integer` or
`Int[..]` `/` whose result is `Rational[..]`, charge
`rational-arithmetic.operands`, then `rational-arithmetic.arithmetic`, then
`rational-arithmetic.normalize`, then decide any FR-044 result-bound membership
without a charge, then `rational-arithmetic.result-retain`; a zero divisor met
by direct kernel evaluation is undefined after `rational-arithmetic.operands`.
`Integer`, `Int[..]`, `Rational[..]` and `Decimal[..]` ordering `<`, `<=`, `>`
and `>=` charges `ordering.operands`, `ordering.arithmetic` and
`ordering.result-retain`: three work units and one result unit. Quantity
ordering keeps its FR-142 unit schedule. Every amount is derived analytically
before any product or aligned coefficient is materialized. FR-144 collection construction and every FR-145 query and conversion
charge the collection family in the order those requirements define: a
constructor element or visited occurrence charges before it is evaluated or
bound, each membership comparison charges `collection.member-walk` before its
plan formation and `collection.member-test` before its pair comparisons, and
`collection.bound` precedes the bound check, which makes no charge. A membership
comparison charges no `equality.*` point, and an FR-149 equality count measures
only the equality expression over completed operands. `function.call` is the execution
fuel of FR-146: it is charged after the call's arguments are evaluated left to
right and before they are bound, so a denied call enters no callee.

Text applicability is profile-dependent. `nfc`, `nfd`, `nfkc` and `nfkd`
charge every text row in order. The non-normalizing `unicode-scalars` and
`binary-utf8` profiles charge only `text.input-bytes`, `text.decode-scalars`
and `text.result-retain`; they charge no `text.normalize-input` or
`text.normalize-output` point and leave `normalized_scalars` unchanged. The
length-bound refusal of an admission follows the charge that measures the
bounded length: after the last `text.normalize-output` for a normalizing profile, after
`text.decode-scalars` for `unicode-scalars` and after `text.input-bytes` for
`binary-utf8`; it always precedes `text.result-retain`, and an exhausted counter
at an earlier charge returns incomplete instead. An
unordered-enumeration ordering request or cross-declaration enum comparison is
refused by type checking before any `enum.*` charge.

Every unit operation follows one schedule. Incompatible dimensions,
affine-unit arithmetic and distinct units where the identical unit is required
are FR-142 type-time `ill_typed` refusals and make no charge. Evaluation charges
one `unit.identity-read` per operand in operand order. It then decides the
runtime conditions in order, first match wins, without a charge: a zero divisor,
then a zero base under a negative integer exponent; each is undefined. It then
charges one `unit.edge` for every edge the operation traverses, all before any
rational event: conversion traverses the full source-to-root path and then the
full root-to-target path with no common-ancestor shortcut, and a compound side
contributes no edge; multiplication, division, integer power, equality and
ordering traverse the left and then the right operand's root path; addition and
subtraction of the identical unit traverse no edge. It then schedules the two
rational events of each edge in that same order, followed by the operation's
own event; conversion, equality and ordering have no own event. It then charges
`unit.target-domain`, except for equality and ordering, and finally
`unit.result-retain`. Every quantity addition, subtraction, multiplication,
division and integer power, and every conversion to an exact rational target,
has an unbounded exact rational result, so its `unit.target-domain` charges no
size counter: that result's size is already charged by its final
`unit.rational-arithmetic` event, or by the identity read for a conversion
without edges. For a decimal target, FR-140's rounding step and the bits and
digits of the retained coefficient `v × 10^T` (rounded when a rounding step
occurs) are decided analytically, without materializing that coefficient;
strict `exact` at a rounding step refuses before `unit.target-domain`; that
charge carries those `integer_bits` and `decimal_digits` amounts; the
coefficient is materialized only after it; and FR-140 membership, which charges
nothing, refuses after it and before `unit.result-retain`. An integer target is
a decimal target at scale zero: its rounding step, strict `exact` refusal and
`unit.target-domain` sizing occur at the same positions, except that its
`unit.target-domain` carries only `integer_bits = bits(rounded integer)` and no
`decimal_digits` amount, and integer-domain
membership, which charges nothing, refuses after `unit.target-domain` and
before `unit.result-retain`. Decimal and integer targets charge no `decimal.*`
or `integer-*` point.

A top-level quantity `=`, `!=`, `<`, `<=`, `>` or `>=` expression between two
operands of the identical unit uses that schedule with no operation event and
no `unit.target-domain`, then `unit.result-retain` with `occ(result) = 1`, and
compares exact root values; it charges no `equality.*` point. A quantity leaf
inside an FR-149 equality plan is instead compared by exact value in its
identical unit and charges only its planned `equality.pair`, like every other
scalar leaf, with no `unit.*` read, edge or event. The FR-142 order among the
static quantity refusals is informative only: every static cause returns the
same `refused { code: ill_typed }` with no charge, so the order has no
observable outcome difference.

IEEE applicability is also representation-independent. Equality, `totalOrder`
and bit identity charge only `ieee.operands` and `ieee.result-retain`; every
IEEE operation or comparison with mixed widths or an exact operand is a
type-time `ill_typed` refusal with no charge. An arithmetic operation charges
only those two points exactly when a NaN operand, an infinite operand, an
invalid operation or a division by zero determines its result; it constructs no
finite exact real and performs no rounding. Every other add, subtract, multiply,
divide and FMA, whose operands are all finite with zeros of either sign
included, plus square root of a finite operand that is a zero of either sign or
positive, additionally charges `ieee.exact-intermediate` and `ieee.round` in
that order. A negative nonzero finite square-root operand follows the
classified-invalid path. Strict `exact` refuses after `ieee.round` and before
`ieee.result-retain`, because inexactness at an IEEE width is decided by that
fixed-width rounding step; decimal and unit targets decide inexactness
analytically before their sizing charge. This applies even when the exact real
is irrational or the rounded result is exact. The fixed width amount is total
for every path and prevents an implementation's rational, symbolic-root or
hardware representation from changing accounting. Thus a comparison or
classified exceptional operation consumes two work units, while a finite
arithmetic operation consumes four.

Explicit IEEE conversions reuse the four IEEE points with arity one. A width
conversion charges `ieee.operands` with `integer_bits=source width`. For a NaN
source it then decides the payload refusal, which makes no charge, and for a NaN
or infinite source it then charges only `ieee.result-retain`. For a finite
source, zeros included, it charges `ieee.exact-intermediate` and `ieee.round`
with `integer_bits=target width`, then decides any strict `exact` refusal, then
charges `ieee.result-retain`. An IEEE-to-exact conversion charges
`ieee.operands` with `integer_bits=source width`; a NaN or infinity is then
undefined; a finite value, either zero included, then charges
`ieee.exact-intermediate` with `integer_bits=maxparts` of the exact result
rational, sized analytically from the decoded exponent and mantissa before the
rational is materialized, then decides `Rational[..]` target membership at
evaluation, exempt from FR-044's static result-bound proof, which makes no
charge and returns `refused { code: ieee_rational_out_of_domain }` for a
non-member before `ieee.result-retain`, then `ieee.result-retain`. FR-148
defines no conversion from an IEEE value to `Decimal`, `Integer` or `Int[..]`; a
direct IEEE conversion to those types is refused `ill_typed` with no charge, and
an IEEE value reaches exact arithmetic only as `Rational[..]`. Conversion of a
`Rational[..]` quantity into a decimal or integer target remains governed by
FR-142 and is unaffected. An exact-to-IEEE conversion charges `ieee.operands`
with `integer_bits` measured on the source: `bits(n)` for an integer `n`,
`maxparts` for a rational, and `max(bits(coefficient), bits(10^scale))` of the
retained representation for a decimal; it then charges `ieee.exact-intermediate`
and `ieee.round` with `integer_bits=target width`, then decides any strict
`exact` refusal, then charges `ieee.result-retain`.

Decimal accounting is independent of implementation storage but not of
retained scale. Every decimal amount is measured on each operand's retained
representation: a literal's coefficient/scale as written, or a computed
result's (`v × 10^T`, `T`) at its target scale `T`; a normalized form is never
substituted. FR-140 membership makes no charge. Decimal addition,
subtraction, multiplication, division, unary negation and explicit conversion
each charge `decimal.operands`, `decimal.scale-expansion` and
`decimal.arithmetic` in that order unless an earlier outcome stops evaluation.
The `decimal.scale-expansion` shift is `abs(left scale - right scale)` for
addition and subtraction, `abs(T + divisor scale - dividend scale)` for division
into target scale `T`, and zero for multiplication, negation and conversion; a
zero shift is still charged, with `scale_expansion=0` and no expanded
coefficient. The `decimal.arithmetic` intermediate is the exact result
coefficient as `c/1` for addition, subtraction, multiplication, negation and
conversion, and the reduced FR-140 `N/D` for division. `decimal.rounding` is
charged exactly when FR-140 defines a rounding step (at least one nonzero
discarded digit) and the selected mode is not `exact`; all-zero discarded
digits are not a rounding step and make no `decimal.rounding` charge. A
divisor whose normalized coefficient is zero is undefined after
`decimal.operands` and before `decimal.scale-expansion`. Strict `exact` at a
rounding step is refused after `decimal.arithmetic` and before
`decimal.rounding`. A nonmember result is refused after
`decimal.rounding`, or after `decimal.arithmetic` when no rounding step occurs,
and before `decimal.result-retain`. An undefined or refused outcome makes no
later charge.

Integer `div`/`rem` and `mod` charge their domain point on every evaluation,
including an unbounded mathematical-integer result. A zero divisor is undefined
after `integer-division.operands` or `integer-modulus.operands` and before the
corresponding arithmetic point, and makes no later charge. A bounded-consumer
membership refusal follows `integer-division.domain-pair` or
`integer-modulus.domain` and precedes result retention. `mod` never charges an
`integer-division.*` point, and `div`/`rem` never charges an
`integer-modulus.*` point.

## Incomplete record

The normative conformance record is exactly
`incomplete { limit_kind, limit, consumed, next_charge, charge_point }`.
Implementations must not expose any additional member in that record, including
a denial cause, fault-injection marker, retry hint or internal partial state.
`charge_point` is the named point whose charge was unavailable. `limit_kind` is
the first `ScalarLimitsV1` counter, in field order, whose amount is unavailable:
a semantic-size counter whose amount exceeds its limit, or a cumulative counter
whose consumed value plus its addition exceeds its limit. `limit` (`u64`) is
that counter's limit, `consumed` (`u64`) is its consumed value before the
denied charge, and `next_charge` (a nonnegative mathematical integer that may
exceed `u64::MAX`) is its semantic-size amount or cumulative addition. For the
`equality.plan` availability check, `next_charge` is the unavailable
reservation: `pair_events + 2` for `work_units` or one for `result_units`;
its canonical vector is TC-194 E23. A
denied charge changes no counter. An injected denial is not a counter
exhaustion: it is reported as defined under
[Qualification seam](#qualification-seam), as if the `work_units` limit were
`w`, so `limit = consumed = w` regardless of the configured limits. Because
decimal scales are `u32` and integer-division operands are materialized values,
no decimal or integer-division amount exceeds `u64::MAX`; a `next_charge`
greater than `u64::MAX` arises from unit integer power (TC-187 U13).

## Qualification seam

NFR-071 fault injection may deny the exact next charge at any named point. An
injected denial at point `P` returns exactly
`incomplete { limit_kind: work_units, limit: w, consumed: w, next_charge: n,
charge_point: P }`, where `w` is the work units consumed before `P` and `n` is
the `work_units` addition `P` requires (one, the `equality.plan`
reservation, or the stated amount of `equality.plan-form`,
`collection.member-walk` or `collection.member-test`). A canonical exact-bound run permits every charge through result
retention; its one-less partner denies the final named charge. Implementations
may use a more efficient internal algorithm, but cannot omit, reorder or change
normative charges or expose internal partial state.
