---
id: TC-297
title: "PopulationId is an opaque digest identity that pairs, compares and orders by content alone"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-089
    type: verifies
---
# TC-297: PopulationId is an opaque digest identity that pairs, compares and orders by content alone

## Description

Verify the kernel-satisfiable half of FR-089-AC-5 and the general opacity
FR-089-AC-2 requires: `Value::Population(population_id)` behaves, inside
`quire-exact`, exactly like the other opaque digest leaves
(`Value::Enum(VariantId)`) already do --

- `ValueType::Population(maximum)` admits any `Value::Population(_)`
  regardless of `maximum`, and refuses every other `Value` kind; no other
  `ValueType` admits a `Value::Population(_)`. This is the kernel's
  *necessary* half of FR-089-AC-5's pairing: the kernel has no `model`
  correspondence to resolve `population_id` through, so it cannot compare
  `maximum` against a resolved binding's own declared maximum -- that
  *sufficient* half (the numeric comparison) is FR-089-AC-5's remaining
  scope, still `🚧 Planned` as TC-295, blocked on QSL `model` minting and
  the evaluator's resolution step (QSL-131's other half).
- two `Value::Population` values with equal `PopulationId` digest bytes
  compare `=`-equal under `crate::equality::plan_pairs`, and differing
  digests compare unequal, with no reference to any binding.
- two `Value::Population` values key-order under `crate::key::compare_keys`
  by raw digest bytes, with no declaration-aware ordering, matching
  `Value::Enum`'s own documented ordering rule.

Scope: FR-089-AC-2 (opaque payload, no resolution reachable from this
crate), FR-089-AC-5 (kernel-necessary pairing condition only).

Catches an implementation that has `ValueType::Population` admit every
`Value` unconditionally (not just `Value::Population`), one that has some
other `ValueType` also admit a `Value::Population`, one that compares
`Value::Population` values by anything other than the identity's own bytes
(for example by insertion order or a hidden counter), and one that gives
`Value::Population` no key at all (silently excluding it from set/bag
canonical ordering the way `Value::Float` is deliberately excluded).

## Test Procedure

1. Construct `ValueType::Population(5)` and `ValueType::Population(0)`, and
   a `Value::Population(id)` for one `PopulationId`. Check `admits` for
   both types against this value, against `Value::Boolean(true)`, and check
   `ValueType::Boolean.admits(&Value::Population(id))`.
2. Construct two `Value::Population` values sharing one `PopulationId`
   digest and a third with a distinct digest. Run `plan_pairs` on the
   sharing pair and on a sharing-versus-distinct pair.
3. Construct two `Value::Population` values with distinct digests, one
   numerically less than the other. Run `compare_keys` on the pair and on a
   value against itself.

## Expected Results

Step 1: both `ValueType::Population` types admit the `Value::Population`
value; neither admits `Value::Boolean(true)`; `ValueType::Boolean` does not
admit the `Value::Population` value. Step 2: the sharing pair's plan is
equal; the sharing-versus-distinct pair's plan is not equal. Step 3: the
pair orders `Less`, matching digest-byte order; a value against itself
orders `Equal`. A mutant that admits `Value::Population` under
`ValueType::Boolean`, or that treats two distinct-digest `Value::Population`
values as always equal, fails one of these assertions.
