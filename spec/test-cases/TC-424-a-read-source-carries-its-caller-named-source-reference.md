---
id: TC-424
title: "An admitted source carries the source reference its caller named, and its node keys ignore the revision"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-001
    type: verifies
---
# TC-424: An admitted source carries the source reference its caller named, and its node keys ignore the revision

## Description

Verify FR-001's source reference: admission keeps the caller's four labels
exactly, adds the `quire.source.bytes/v1` digest, refuses an empty or blank
label, and a declaration's key depends on the authority and identity but
not on the revision. This catches a defaulted or path-derived label, and a
revision that leaks into a node key.

Scope: FR-001-AC-5, FR-001-AC-6, FR-001-AC-7.

## Test Procedure

1. Admit bytes `b` as authority `agent-ix`, identity `specs/a.quire`,
   revision (`git`, `3f2a`). Admit them again at revision value `3f2b`.
2. Admit `b` four times, each with one of the four labels empty, and four
   more times, each with one label a single space.
3. Check package declarations holding record `Point` with field
   `x: Int[0, 9]` under the reference of `b` as (`a`, `u`, `git`, `1`),
   under the reference of `b'` as (`a`, `u`, `git`, `2`), and under the
   reference of `b` as (`c`, `u`, `git`, `1`), and under the reference of `b`
   as (`a`, `v`, `git`, `1`).

Tag the tests `#[trace("TC-424", "FR-001-AC-n")]` with the AC each backs.

## Expected Results

- Step 1: the reference reads the four labels exactly and the
  `quire.source.bytes/v1` digest of `b`; the second differs only in the
  revision value.
- Step 2: each admission refuses with `invalid_source_identity`.
- Step 3: `Point` has the same key under the first two references, and
  the third and fourth references each give a key different from it and
  from each other.

## Status

Planned. ADR-013 §7 slice S-4b (QSL-233).
