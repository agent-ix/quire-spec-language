---
id: TC-437
title: "The extent rule names each unbounded type position once, by node and path, under a node-count ceiling"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-097
    type: verifies
---
# TC-437: The extent rule names each unbounded type position once, by node and path, under a node-count ceiling

## Description

Verify `classify_extent` against the ADR-014 §4 extent rule. Scope: FR-097-AC-2.

## Test Procedure

1. Classify `Integer`, `Sequence<Int[0,9]>` with no bound, a bounded
   sequence of `Integer`, an unbounded sequence of `Integer`,
   `Option<Integer>`, `Population(None)`, a record with one `Integer` field,
   a recursive record, and a quantity.
2. Classify `Int[0,9]`, `Boolean`, `Population(Some(3))` and bounded
   collections and options of `Int[0,9]`.
3. Classify two roots twice.
4. Classify a three-position record with ceiling 2, then 3.
5. Classify a composite missing from the environment.
6. Check a test claim family whose `check` classifies its argument types.

## Expected Results

- Step 1: each unbounded position is one domain at its node and path with
  its kind; the quantity's domain has no finite kind.
- Step 2: `Bounded`.
- Step 3: each root keys its own domains; the two results are equal.
- Step 4: node-count limit, bound 2, actual 3; ceiling 3 admits.
- Step 5: an internal fault, never `Bounded`.
- Step 6: its requirements carry the classified extent.

## Status

Backed: `qsl-semantics/src/family/requirements.rs`, `tests` module.
