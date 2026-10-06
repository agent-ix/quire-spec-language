---
id: TC-905
title: "A witness value text decodes exactly for every composite and leaf family, and each malformed form refuses"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: verifies
---
# TC-905: A witness value text decodes exactly for every composite and leaf family, and each malformed form refuses

## Description

Verify that `Witness::decode` reads a `Canonical` binding's FR-070 witness
value text into a `WitnessValue` tree. The tree keeps every slot, option,
element, member, declaration identity and leaf exactly as written, beside an
unchanged scalar entry. Transcript escaping keeps text holding `;`, `>>>`,
`%` and `<` inside its entry. Each malformed entry refuses `Malformed`,
naming the parameter.

Scope: FR-070-AC-8, FR-070-AC-9, FR-070-AC-11, FR-070-AC-12.

## Test Procedure

Use parameter node ids `P` (bound `Canonical`) and `Q` (bound `I64`), and
declaration node ids `INNER`, `OUTER`, `SHAPE` and `COLOR`. Each value text
is its JCS encoding, escaped as FR-070 states, unless the step says
otherwise.

1. Decode a transcript whose `P` entry is an `OUTER` record with these
   fields, in declaration order:
   - `i`: an `INNER` record whose `a` is integer `3`, whose optional `b` is
     absent (`null`), and whose `c` slot is `{"type":"null"}`;
   - `o1`: a present option of integer `4`; `o2`: a `none` option;
   - `s`: a sequence of `2`, `1`, `2`;
   - `st`: a set of `1` and `2`; `bg`: a bag of `1`, `1` and `2`;
   - `os`: an ordered set of `2` and `1`;
   - `u1`: `SHAPE` member `Circle` with one component `5`; `u2`: `SHAPE`
     member `Empty` with no components.

   The `Q` entry is `-9`.
2. Decode one transcript per leaf family, each with one `P` entry:
   - the integer `-170141183460469231731687303715884105728`;
   - `COLOR` member `Red`;
   - the text `a;b=c<<<d>>>e%f`, whose entry holds
     `a%3Bb=c%3C%3C%3Cd%3E%3E%3Ee%25f`;
   - the rational `-3`/`4`;
   - the decimal with coefficient `1050` and scale `2`;
   - a `float32` with bits `7fc00001`, and a `float64` with bits
     `8000000000000000`;
   - a decimal quantity with coefficient `15`, scale `1` and a unit digest;
   - a reference.

   Then decode a sequence of the texts `x;y` and `p>q`.
3. For each malformed entry below, decode a transcript whose `P` entry
   holds it:
   - trailing bytes after the value; a space after a `:`; record members in
     the order `name`, `type`, `fields`;
   - the tag `map`; a record field entry with no `value` member;
   - an integer `"007"`; an integer as the JSON number `3`;
   - a rational `2`/`4`; a rational with denominator `-1`;
   - a `float64` with 15 hex digits; a `float64` in uppercase hex;
   - a reference `identity` of odd length;
   - `{"type":"null"}` as a sequence element;
   - a set of `1` and `1`; a set of `2` then `1`; an ordered set of `1` and
     `1`;
   - a raw `%` that does not begin an escape; the lowercase escape `%3b`;
   - a record field name written `a`.

4. Decode a transcript with a `Q` entry for node id `0a…` followed by a `Q`
   entry for node id `0b…`, then the same two entries in the other order.

Tag the tests `#[trace("TC-905", "FR-070-AC-8")]`,
`#[trace("TC-905", "FR-070-AC-9")]`, `#[trace("TC-905", "FR-070-AC-11")]`
and `#[trace("TC-905", "FR-070-AC-12")]`.

## Expected Results

- Step 1: two values in binding order. The first is the composite tree,
  with every slot state (present, absent, `null`), option state, element
  order (the set and the bag ascending, the sequence and the ordered set as
  written), union member identifier and declaration node id exactly as
  written. The second is `WitnessValue::Integer(-9)`.
- Step 2: each transcript parses as one assertion block, and each entry
  decodes to its leaf exactly: the integer beyond `i64`, the member `Red` of
  `COLOR`, the text `a;b=c<<<d>>>e%f`, the rational, the decimal's retained
  coefficient and scale, both bit patterns, the quantity's magnitude and
  unit, and the reference's three identities. The sequence decodes to its
  two texts, in order.
- Step 3: each refuses `DecodeRefusal::Malformed` naming `P`, and no value is
  returned.
- Step 4: the ascending transcript decodes. The other order refuses
  `DecodeRefusal::EntryOrder` naming the `0a…` entry, with no value.

## Status

Passed locally, `qsl-replay/src/witness/value_text/tests.rs`.
