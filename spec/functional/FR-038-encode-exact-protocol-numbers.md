---
id: FR-038
title: "Encode compiled protocol numbers without rounding"
type: FR
relationships:
  - { target: ix://agent-ix/quire-spec-language/US-004, type: implements }
  - { target: ix://agent-ix/quire-specification/FR-039, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-044, type: depends_on }
  - { target: ix://agent-ix/quire-protocol/FR-001, type: references }
---
# FR-038: Encode compiled protocol numbers without rounding

## Description

When encoding a compiled protocol numeric value, the compiler SHALL preserve its
integer or rational kind and exact normalized components in a unique tagged
decimal-string representation.

## Inputs

A signed-64 integer or a normalized rational whose numerator is signed-64 and
whose denominator is in 1..9223372036854775807. The selected native/model domain
and field-specific bounds remain separate admission requirements; fitting these
wire components does not admit a value under an arbitrary model type.

This chooses the representation required by compiler #40 and the accepted
[consumer numeric contract](https://github.com/agent-ix/quire-protocol/pull/2).
All native numeric values use strings, including integers inside the JSON safe
range. Structural counts/indices are separate fields with their own safe bounds.

## Outputs

The numeric profile is `quire.protocol.numeric/1`. Its two closed object shapes
are `{ "kind": "integer", "decimal": "-17" }` and
`{ "kind": "rational", "numerator": "1", "denominator": "3" }`.
Member order in these illustrative objects is not a canonicalization rule;
the artifact's selected canonical encoding determines object-member order.
No additional members, alternate tag, number token or omitted component is valid.

## Behavior

The numeric encoder SHALL render each signed component in ASCII decimal with no
leading plus, redundant zero, exponent, fractional point or negative zero.

The numeric encoder SHALL render a rational with a positive denominator and
coprime numerator/denominator, including the unique zero representation 0/1.

If an offered wire component is noncanonical, outside its component domain or
part of an unreduced rational, then the numeric reader SHALL refuse that value
without normalizing or rounding it.

The numeric reader SHALL distinguish integer 1 from rational 1/1.

The compiler SHALL validate the typed numeric representation before passing it
to the selected canonical JSON serializer.

Source literal normalization remains the frontend's responsibility under standard
FR-039: a source rational(2,4) may normalize to 1/2 before model-bound checks.
That rule does not authorize a wire reader to repair an offered 2/4 encoding.
No integer-to-rational conversion or floating-point intermediate is introduced.

Rust represents admitted numeric values through types that cannot contain an
invalid rational. Untrusted decoded fields are converted through explicit typed
validation before becoming those values. Numeric failure classifications do not
depend on parsing error-message strings. A JSON shape error may retain the
original Serde cause; it does not become an admitted native number.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-038-AC-1 | Zero, both JSON safe-integer endpoints, adjacent wider integers and signed-64 extrema encode as exact canonical decimal strings and recover their original integer values. | Test (TC-117) |
| FR-038-AC-2 | 1/3, -1/2, integer 1 versus rational 1/1, zero 0/1 and admitted component extrema retain their exact kind/components and independent fixed expected object forms. | Test (TC-117) |
| FR-038-AC-3 | Floats, bare numeric component tokens, negative zero, leading plus/zero, non-ASCII digits, exponent/fraction spellings, overflow, nonpositive denominators and unreduced rationals refuse without repaired values. | Test (TC-117) |
| FR-038-AC-4 | Unknown/duplicate/missing members or tags refuse; a safe integer supplied as a bare number is also refused because this profile selects the uniform string representation. | Test (TC-117) |
| FR-038-AC-5 | Source normalization and selected model/field-domain validation remain separate from wire validation; numeric codec success alone grants neither model admission nor source-to-artifact correspondence. | Test (TC-117) |

## Dependencies

[US-004](../usecase/US-004-reuse-existing-toolchain.md) owns the consumer need.
The enclosing compiled artifact, exact selectors, source correspondence and
producer/consumer integration remain the full scope of
[compiler #40](https://github.com/agent-ix/quire-spec-language/issues/40).
This numeric component alone does not complete that delivery.
