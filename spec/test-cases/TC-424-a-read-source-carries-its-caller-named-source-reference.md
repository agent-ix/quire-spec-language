---
id: TC-424
title: "An admitted source carries the source reference its caller named, and its node keys follow its labels"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-001
    type: verifies
---
# TC-424: An admitted source carries the source reference its caller named, and its node keys follow its labels

## Description

Verify FR-001's source reference: admission keeps the caller's two labels
exactly, refuses an empty or blank label, and a declaration's key depends
on the authority and identity but not on the source bytes. This catches a
defaulted or path-derived label, a reference that carries any other member,
and source content that leaks into a node key.

Scope: FR-001-AC-5 to FR-001-AC-15.

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
   identity; admit five bytes under a four-byte ceiling.
5. Render the region `[4, 7)` of admitted source `ab\ncdéf`.
6. Admit `b` with the identity and the authority blank; with the identity
   blank and an empty path; and with both labels non-blank and an empty
   path.
7. Apply an incremental edit with a stale expected source identity and send a
   complete-editor request bound to another source revision.
8. From a caller outside CST, use `SourceIdentity::first_blank_label` for
   the blank-label combinations in steps 2 and 6. Construct the existing
   typed refusal from its public fields, with no region, and project it
   through `qsl_cst::diagnostic::source_refusal` before offering bytes or
   admitting a source. Also project an empty-path refusal.
9. Project each `SourceReadCause`, retaining distinctive source labels,
   path and message. Use no region for blank-label, empty-path,
   reference-invariant and byte-budget refusals; use the original offered
   source's region for invalid UTF-8, BOM and NUL refusals.
10. Repeat projection with ceilings below and above 1 MiB. For source-read
    refusals reachable through `Source::read_typed`, compare direct
    projection with `read_source` under the same inputs and ceiling.

Tag the tests `#[trace("TC-424", "FR-001-AC-n")]` with the AC each backs.

## Expected Results

- Step 1: the reference reads the two labels exactly and has no other
  member; the second is equal to it.
- Step 2: each blank-label admission refuses with
  `invalid_source_identity`, cause `blank-label`, field `label` naming the
  blank label (`authority` or `identity`). The U+200B authority admits.
- Step 3: `Point` has the same key under the first two references, and
  the third and fourth references each give a key different from it and
  from each other.
- Step 4: region `[1, 1)` under the `RawSourceRef` of `a\xffb`; region
  `[2, 3)` under that of `ab\0c`; the last two refuse with no region, not a
  region at byte 0. This checks the behaviour change FR-001 states.
- Step 5: start line 2, column 2; end line 2, column 4. The region holds
  only its reference, 4 and 7.
- Step 6: `blank-label` with `label` `authority`; `blank-label` with `label`
  `identity`, not `empty-path`; `invalid_source_identity`/`empty-path` with
  no field.
- Step 7: the edit and editor request each refuse with `invalid_source_map`
  and no region, with host causes `EditPredecessor` and `RequestRevision`
  respectively; neither is `invalid_source_identity`.
- Step 8: no bytes, admitted source or filesystem operation is needed.
  The projected cause's `identity_fields()` names `blank-label` and exactly
  the first blank label, authority before identity. U+200B is not blank.
  Empty-path has cause `empty-path` and no label. Every absent region stays
  absent, never byte zero.
- Step 9: the code and complete cause match FR-001's projection table;
  phase is `Source`, related regions are empty, and the identity, path,
  message and optional region equal the supplied values exactly.
- Step 10: only byte-budget refusals carry a syntax limit, with precisely
  the supplied bound. Direct projection and `read_source` agree on every
  diagnostic field, including the limit and optional region.
