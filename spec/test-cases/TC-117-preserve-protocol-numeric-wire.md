---
id: TC-117
title: "Preserve exact compiled protocol numbers and reject alternate forms"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-038, type: verifies }
---
# TC-117: Preserve exact compiled protocol numbers and reject alternate forms

## Description

Exercise the public Rust numeric codec using independently fixed expected objects
and values, including the consumer's accepted safe-integer and rational controls.
These component tests do not substitute for the real compiler-to-B integration.

## Test Procedure

1. Encode 0, ±9007199254740991, ±9007199254740992, i64::MIN and i64::MAX.
   Compare each decimal string and kind with an independently authored expectation;
   decode it and compare the exact i64 value, without a floating-point conversion.
2. Encode 1/3, -1/2, 1/1, 0/1 and admitted extreme components. Inspect numerator,
   positive denominator and rational tag; distinguish integer 1 from rational 1/1.
3. Independently offer decimal strings -0, +1, 01, -01, 1.0, 1e3, non-ASCII digits
   and values outside signed-64. Offer raw JSON numbers in each component position,
   including safe endpoints, unsafe integers, floats and negative zero.
4. Offer denominator 0, a negative denominator, 2/4, -2/4 and 0/2. Require a typed
   refusal, not reduction/sign repair. Supply the corresponding canonical positive
   control independently, so a dead rejection guard cannot satisfy the test.
5. Independently omit, duplicate and add each member, change the tag, and substitute
   integer members for rational members. Require refusal of the closed shape.
6. Retain a valid wire integer outside a separately selected narrow model bound.
   Assert that numeric decoding supplies an exact value without constructing a
   model-admitted or checked protocol artifact. A source 2/4 normalization fixture
   is a frontend control; the wire 2/4 case must still refuse.

## Expected Results

All positive values retain their exact kind and components. Every negative case
fails an unconditional assertion on the typed result; missing errors or missing
fields cannot skip verification. Correct re-encoding alone is insufficient:
fixed expected values/object forms catch a codec that consistently changes meaning.
