---
id: TC-424
title: "An admitted source carries the source reference its caller named, and its node keys ignore the content digest"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-001
    type: verifies
---
# TC-424: An admitted source carries the source reference its caller named, and its node keys ignore the content digest

## Description

Verify FR-001's source reference: admission keeps the caller's two labels
exactly, adds the `quire.source.bytes/v1` digest, refuses an empty or blank
label, and a declaration's key depends on the authority and identity but
not on the digest. This catches a defaulted or path-derived label, a
reference that carries any other member, and a digest that leaks into a
node key.

Scope: FR-001-AC-5 to FR-001-AC-12.

## Test Procedure

1. Admit bytes `b` as authority `agent-ix`, identity `specs/a.quire`.
   Admit bytes `b'` under the same labels.
2. Admit `b` twice, each with one of the two labels empty, twice more,
   each with one label a single space, and twice more, each with one
   label U+3000 IDEOGRAPHIC SPACE. Admit `b` once with the authority U+200B
   ZERO WIDTH SPACE.
3. Check package declarations holding record `Point` with field
   `x: Int[0, 9]` under the reference of `b` as (`a`, `u`), under the
   reference of `b'` as (`a`, `u`), under the reference of `b` as
   (`c`, `u`), and under the reference of `b` as (`a`, `v`).
4. Admit `a\xffb` and `ab\0c` as (`a`, `u`); admit `b` with an empty
   identity; admit five bytes under a four-byte ceiling;
   admit `b` under verified intake with a different selected digest.
5. Render the region `[4, 7)` of admitted source `ab\ncdéf`.
6. Admit `b` with the identity and the authority blank; with the identity
   blank and an empty path; and with both labels non-blank and an empty
   path.
7. Apply an incremental edit whose expected predecessor digest is not the
   source's digest; render a CST node against a parsed source it does not
   belong to; and run an editor request bound to another source digest.

Tag the tests `#[trace("TC-424", "FR-001-AC-n")]` with the AC each backs.

## Expected Results

- Step 1: the reference reads the two labels exactly and the
  `quire.source.bytes/v1` digest of `b`, and has no other member; the
  second differs only in the digest.
- Step 2: each blank-label admission refuses with
  `invalid_source_identity`, cause `blank-label`, field `label` naming the
  blank label (`authority` or `identity`). The U+200B authority admits.
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
- Step 7: each refuses with `invalid_source_map`, host cause
  `EditPredecessor`, `ForeignNode` and `RequestRevision` respectively, and
  no region; none is `invalid_source_identity`.

## Status

Partial. Passed locally (steps 1 to 7) step 2's
U+3000 and U+200B cases and its `blank-label` cause and `label`
(`qsl-foundation/src/source.rs`, `a_blank_label_refuses_naming_it_and_admits_nothing`),
step 6 (`label_order_precedes_the_path`, and the complete reader's
`the_reader_reports_blank_label_before_empty_path`) and step 7
(`a_stale_edit_predecessor_refuses_as_a_source_map_with_no_region`,
`rendering_a_foreign_cst_node_is_a_typed_refusal`,
`stale_profile_and_cancelled_editor_requests_are_typed`) under the
four-label reader. Steps 1 to 4, 6 and 7 as stated here wait on the
two-label reader and digest-based edit and request checks (FR-001 Status).
