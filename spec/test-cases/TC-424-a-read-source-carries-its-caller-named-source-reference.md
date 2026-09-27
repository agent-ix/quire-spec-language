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

Scope: FR-001-AC-5 to FR-001-AC-11.

## Test Procedure

1. Admit bytes `b` as authority `agent-ix`, identity `specs/a.quire`,
   revision (`git`, `3f2a`). Admit them again at revision value `3f2b`.
2. Admit `b` four times, each with one of the four labels empty, four more
   times, each with one label a single space, and four more, each with one
   label U+3000 IDEOGRAPHIC SPACE. Admit `b` once with the authority U+200B
   ZERO WIDTH SPACE.
3. Check package declarations holding record `Point` with field
   `x: Int[0, 9]` under the reference of `b` as (`a`, `u`, `git`, `1`),
   under the reference of `b'` as (`a`, `u`, `git`, `2`), and under the
   reference of `b` as (`c`, `u`, `git`, `1`), and under the reference of `b`
   as (`a`, `v`, `git`, `1`).
4. Admit `a\xffb` and `ab\0c` as (`a`, `u`, `git`, `1`); admit `b` with an
   empty revision namespace; admit five bytes under a four-byte ceiling;
   admit `b` under verified intake with a different selected digest.
5. Render the region `[4, 7)` of admitted source `ab\ncdéf`.
6. Admit `b` with the revision namespace and the authority blank; with the
   identity blank and an empty path; and with all four labels non-blank and
   an empty path.

Tag the tests `#[trace("TC-424", "FR-001-AC-n")]` with the AC each backs.

## Expected Results

- Step 1: the reference reads the four labels exactly and the
  `quire.source.bytes/v1` digest of `b`; the second differs only in the
  revision value.
- Step 2: each blank-label admission refuses with
  `invalid_source_identity`, cause `blank-label`, field `label` naming the
  blank label (`authority`, `identity`, `revision_namespace` or
  `revision`). The U+200B authority admits.
- Step 3: `Point` has the same key under the first two references, and
  the third and fourth references each give a key different from it and
  from each other.
- Step 4: region `[1, 1)` under the `RawSourceRef` of `a\xffb`; region
  `[2, 3)` under that of `ab\0c`; the last three refuse with no region, not a
  region at byte 0. This checks the behaviour change FR-001 states.
- Step 5: start line 2, column 2; end line 2, column 4. The region holds
  only its reference, 4 and 7.
- Step 6: `blank-label` with `label` `authority`; `blank-label` with `label`
  `identity`, not `empty-path`; `invalid_source_identity`/`empty-path` with
  no field.

## Status

Partial. Steps 1 to 5 pass locally under ADR-013 §7 slice S-4b (QSL-233)
for the `invalid_source_identity` code. Step 2's U+3000 and U+200B cases,
its `blank-label` cause and `label` field, and step 6 (FR-001-AC-11), are
planned (QSL-245).
