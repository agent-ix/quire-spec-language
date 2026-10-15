---
id: TC-1029
title: "Snapshots and invocations carry UUID and Timestamp values"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: verifies
---
# TC-1029: Snapshots and invocations carry UUID and Timestamp values

## Description

Verify FR-106's UUID and Timestamp native payload objects: admission of the
canonical forms, each refusal with its code, cause, member and nested path,
declared-type fit, frame comparison and value counting.

Scope: FR-106-AC-18, FR-106-AC-19, FR-106-AC-20, FR-106-AC-21, FR-106-AC-22.

## Preconditions

- TC-465's `archive` package variant, with `ConfigVersion` gaining the fields
  `token: UUID`, `stamp: Timestamp`, `prior?: UUID` and `seen:
  Sequence<Timestamp>`, and an operation `touch(id: UUID, at: Timestamp):
  UUID` on `ConfigVersion` whose frame modifies `token` and `stamp`.
- The healthy-parent snapshot (FR-106) with `child` holding `token`
  `{"kind":"uuid","text":"00112233-4455-6677-8899-aabbccddeeff"}`, `stamp`
  `{"kind":"timestamp","nanoseconds":"0"}`, `prior` `{"absent":{}}` and
  `seen` `{"sequence":[]}`; `root` holds a different `token` and `stamp`.
  Digests are recomputed for every mutated document.

## Test Procedure

1. Admit the base snapshot for `ParentOrder` with `Current` selection
   (anchor `handler validate`, `self` `child`). Reorder the members of the
   UUID object and of the Timestamp object, and the whitespace of the
   document, and admit again.
2. Admit one mutation per row of `token` (UUID) values, each with its
   own digest: Nil `00000000-0000-0000-0000-000000000000`, Max
   `ffffffff-ffff-ffff-ffff-ffffffffffff`, and
   `00112233-4455-0677-8899-aabbccddeeff`. Admit one mutation per row of
   `stamp` (Timestamp) values: `"1"`, `"-1"`,
   `"170141183460469231731687303715884105727"` and
   `"-170141183460469231731687303715884105728"`. Admit `prior` as
   `{"present":{"kind":"uuid","text":"<canonical>"}}` and `seen` as
   `{"sequence":[{"kind":"timestamp","nanoseconds":"5"},{"kind":"timestamp","nanoseconds":"-5"}]}`.
3. Admit an invocation of `touch` (`id` a UUID object, `at` a Timestamp
   object, `result` a UUID object), selected for a postcondition, and the
   same invocation selected through `PreCall`.
4. Mutate `token` to each noncanonical UUID text, one per row (D1 to D10):
   D1 `00112233-4455-6677-8899-AABBCCDDEEFF`; D2
   `00112233-4455-6677-8899-aabbCCddeeff`; D3
   `00112233445566778899aabbccddeeff`; D4
   `{00112233-4455-6677-8899-aabbccddeeff}`; D5
   `urn:uuid:00112233-4455-6677-8899-aabbccddeeff`; D6
   `0011223-34455-6677-8899-aabbccddeeff`; D7
   `00112233-4455-6677-8899-aabbccddeefg`; D8
   `00112233-4455-6677-8899-aabbccddeef`; D9 `" 00112233-4455-6677-8899-aabbccddeeff"`;
   D10 with an Arabic-Indic digit in place of an ASCII `0` of the first group.
5. Mutate `stamp` to each noncanonical or out-of-domain text, one per row:
   `"-0"`, `"+1"`, `"01"`, `"-01"`, `""`, `" 1"`, `"1.0"`, `"1e3"`,
   `"2026-10-15T00:00:00Z"` (noncanonical); `"170141183460469231731687303715884105728"`
   and `"-170141183460469231731687303715884105729"` (out of domain);
   `"0170141183460469231731687303715884105728"` (noncanonical and out of
   domain).
6. Mutate `token` to each envelope defect, one per row: no `text`;
   `{"kind":"uuid","nanoseconds":"0"}`; an extra member `"extra":1`; `text`
   `null`; `text` `5`; `kind` `1`; `kind` `null`; `kind` `"string"`; `kind`
   `"UUID"`; `kind` `"uuidv4"`; `{"kind":"string","nanoseconds":"0"}`; an extra
   member together with a noncanonical `text`; `kind` `1` with no `text`.
   Repeat for `stamp` with no `nanoseconds` and `nanoseconds` the JSON
   number `0`.
7. Mutate for fit, one per row: a Timestamp object in `token`; a UUID object
   in `stamp`; `{"integer":"5"}` in `stamp`; `{"integer":"5"}` and
   `{"boolean":true}` in `token`; a UUID object in `versionNumber`; a UUID
   object as a `parent` value; a Timestamp object in `{"present":...}` of
   `prior`; a UUID object as an element of `seen` at index 1; a Timestamp object with `nanoseconds` `"01"` in `token`; a UUID object with `text` `"X"` in `stamp`. Repeat the
   first two as the `id` and `at` arguments of `touch`, and as its `result`.
8. Admit an invocation of `touch` changing `child.token` from the base UUID
   to another canonical UUID, and `child.stamp` from `"0"` to `"1"`, with the
   grant and without it. Admit one whose post `token` is the same payload
   with its members reordered.
9. Under `PostStateRange::Witness`, admit an invocation whose post snapshot
   holds a noncanonical `token` and one whose post `stamp` is out of
   domain.
10. Admit the base snapshot with `observation.values` set to its exact value
    count, then to one less.

## Expected Results

- Step 1 admits one observation in which `child.token` is a UUID value and
  `child.stamp` a Timestamp value of the written payloads; the reordered
  document gives the same typed observation and digest.
- Step 2 admits every row with the payload unchanged (version and variant
  bits retained); `prior` is present and `seen` holds `5`, `-5` in order.
- Step 3 admits `id`, `at` and `result` as UUID, Timestamp and UUID values in
  both selections, with the `PreCall` selection reading no result.
- Step 4 refuses every row `invalid_runtime_input`/`invalid-value` naming
  `child`, `token` and member `text`, kernel class `noncanonical-payload`.
- Step 5 refuses the nine noncanonical rows with `invalid-value` and class
  `noncanonical-payload`, the next two with `invalid-value` and class
  `payload-out-of-domain`, and the last with `noncanonical-payload`, all
  naming `child`, `stamp` and member `nanoseconds`.
- Step 6 refuses by the listed order: `kind` `1` and `null` (also `kind` `1`
  with no `text`) give `wrong-value-kind` naming `kind`; `kind` `"string"`,
  `"UUID"`, `"uuidv4"` and the `string`/`nanoseconds` object give
  `invalid-value` naming `kind`; the missing `text` rows (and the `uuid`
  object carrying only `nanoseconds`) give `missing-member` naming `text`, and
  the missing `nanoseconds` row naming `nanoseconds`; the extra member rows
  give `unknown-member` naming `extra`, including beside a noncanonical
  `text`; `text` `null` and `5`, and `nanoseconds` `0`, give `wrong-value-kind`
  naming the member.
- Step 7 refuses every row `invalid_runtime_input`/`wrong-value-kind` at the
  nested path (`prior` with its option-value step, `seen` with index 1, the
  `touch` binding root for arguments and result), even where the payload is
  also noncanonical.
- Step 8 admits the frame with the grant; without it refuses
  `frame_violation`/`unauthorized-change` naming `child` and `token` first
  (then `stamp` in its own run). The reordered-members post is no change.
- Step 9 refuses both with `invalid-value` and the step 4 and step 5 classes.
- Step 10 admits at the exact count; one less returns
  `stage_limit_exceeded`/`node-count-exceeded` naming `observation.values`.
- Assertions compare typed code, cause, input path, member and kernel
  class, never rendered text.

## Traceability

The expected results verify FR-106-AC-18 to FR-106-AC-22 and the FR-056
runtime-input adapter mapping (FR-056-AC-17) at the document edge.
