---
id: TC-1036
title: "Snapshots and invocations carry String values"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: verifies
---
# TC-1036: Snapshots and invocations carry String values

## Description

Verify FR-106's String native payload object: admission of any Unicode scalar
sequence unchanged, each refusal with its code, cause, member and nested path in
the native payload order, declared-type fit, frame comparison by exact scalar
sequence, value counting and the document-edge refusals that come before any
value is read.

Scope: FR-106-AC-28, FR-106-AC-29, FR-106-AC-30, FR-106-AC-31, FR-106-AC-32.

## Preconditions

- TC-1029's package, with `ConfigVersion` also gaining, after its other
  fields and in this declared order, `tags: Sequence<String>`, `label: String`
  and `note?: String`, and an operation `rename(name: String): String` on
  `ConfigVersion` whose frame modifies `label`.
- TC-1029's base snapshot, with `child` also holding `tags`
  `{"sequence":[]}`, `label` `{"kind":"string","text":"ready"}` and `note`
  `{"absent":{}}`, and `root` holding `tags` `{"sequence":[]}`, `label`
  `{"kind":"string","text":"root"}` and `note` `{"absent":{}}`. Digests are
  recomputed for every mutated document, except where a step says the
  selected digest stays the base snapshot's.

## Test Procedure

1. Admit the base snapshot for `ParentOrder` with `Current` selection
   (anchor `handler validate`, `self` `child`). Reorder the members of the
   `label` object and the whitespace of the document, and admit again.
2. Admit one mutation per row of `label` texts, each with its own digest:
   the empty text `""`; `"  ready  "`; `"READY"`; S1 `"é"` (U+00E9);
   S2 `"é"` (U+0065 U+0301); `"00112233-4455-6677-8899-aabbccddeeff"`;
   `"\u0000"`; `"a\u001fb"`; `"￾"`; `"a;b>>>c"`; a text of 65,536
   `a`; the paired escape `"😀"`, and separately the literal
   character U+1F600. Admit `note` as `{"present":{"kind":"string","text":"n"}}`
   and `tags` as `{"sequence":[{"kind":"string","text":"a"},{"kind":"string","text":""}]}`.
3. Admit an invocation of `rename` (`name` a String object, `result` a String
   object), selected for a postcondition on `rename`, and the same invocation
   selected through `PreCall`.
4. Mutate `label` to each object defect, one per row: `kind` `1`; `kind`
   `null`; `kind` `{}`; `kind` `1` with no `text`; `kind` `"String"`; `kind`
   `"STRING"`; `{"kind":"text","value":"x"}`; `{"kind":"string"}`;
   `{"kind":"string","nanoseconds":"0"}`; an extra member `"extra":1` beside a
   valid `text`; a valid `text` beside `"nanoseconds":"0"`; `text` `5`; `text`
   `null`; `text` `true`; `text` `["x"]`; `text` `{}`; an extra member
   `"extra":1` together with `text` `5`.
5. Mutate for fit, one per row: a String object in `token`; in `stamp`; in
   `versionNumber`; as a `parent` value (`{"present":<String object>}`); in
   `{"present":...}` of `prior`; as an element of `seen` at index 1 (after a valid Timestamp object at index 0). A UUID
   object, a Timestamp object, `{"integer":"5"}`, `{"boolean":true}`,
   `{"absent":{}}` and the bare JSON string `"ready"` in `label`. A String
   object with text `00112233-4455-6677-8899-aabbccddeeff` in `token`.
   A String object with `text` `5` in `token`, and one with an extra member
   `"extra":1` and a valid `text` in `versionNumber`. Repeat the
   Timestamp-object row as the `name` argument of `rename`, and a UUID object
   as its `result`.
6. Admit an invocation of `rename` changing `child.label` from `"ready"` to
   `"Ready"`, and one from S1 to S2, each with the grant and without it. Admit
   one whose post `label` is `ready` with the members of the object reordered,
   and one whose post `label` is written `"ready"`.
7. Under `PostStateRange::Witness`, admit an invocation whose post snapshot
   holds a UUID object in `label`, and one whose post `label` has `text` `5`.
8. Mutate `note` to `{"present":{"kind":"string","text":"n"}}` and admit the
   document with `observation.values` set to its exact value count, then to
   one less.
9. Select each of these documents under the base snapshot's digest (the
   selected digest stays the base's): a `label` text holding the escape
   `\ud800`; one holding the escape `\udc00`; one holding the raw byte `0xC0`;
   the first of them with a wrong-kind `token` (a Timestamp object) as well.
   Then admit a document whose `label` text has 1,048,576 `a`, so that the
   document exceeds 1 MiB.

## Expected Results

- Step 1 admits one observation in which `child.label` is a String value of
  the scalar sequence `ready`; the reordered document gives the same typed
  observation and digest.
- Step 2 admits every row with its scalar sequence unchanged, nothing trimmed,
  folded, normalized or escaped. The S1 and S2 observations hold two unequal
  `label` values with different digests. The paired escape and the literal
  U+1F600 give one value and one digest. `note` is present and `tags` holds
  `a` and the empty String, in order.
- Step 3 admits `name` and `result` as String values in both selections, with
  the `PreCall` selection reading no result.
- Step 4 refuses every row `invalid_runtime_input` naming `child` and `label`
  by the listed order: the `kind` `1`, `null`, `{}` rows and the `kind` `1`
  row with no `text` give `wrong-value-kind` naming `kind` (check 1); the
  `"String"`, `"STRING"` and `text`/`value` rows give `invalid-value` naming
  `kind` (check 2), their members unexamined; `{"kind":"string"}` and the
  `string`/`nanoseconds` object give `missing-member` naming `text` (check 3);
  the two extra-member rows give `unknown-member` naming `extra` and
  `nanoseconds` (check 4), and so does the row holding both an extra member
  and `text` `5`, naming `extra`; the `text` `5`, `null`, `true`, `["x"]` and
  `{}` rows give `wrong-value-kind` naming `text` (check 5).
- Step 5 refuses every row `invalid_runtime_input`/`wrong-value-kind` at the
  nested path (`parent` and `prior` with their option-value step, `seen` with
  index 1, the `rename` binding root for the argument and result) and names no
  member, except: the `text` `5` row in `token` names `text` (check 5 comes
  before type fit), and the extra-member row in `versionNumber` refuses
  `unknown-member` naming `extra` (check 4 comes before type fit). The String
  object whose text is a canonical UUID is `wrong-value-kind` in `token` and
  no UUID value is read.
- Step 6 admits both frames with the grant; without it each refuses
  `frame_violation`/`unauthorized-change` naming `child` and `label`. The
  reordered-members post and the `"ready"` post are no change, so both
  admit without the grant.
- Step 7 refuses both with `wrong-value-kind`, the second naming `text`.
- Step 8 admits at the exact count; one less returns
  `stage_limit_exceeded`/`node-count-exceeded` naming `observation.values`,
  the bound and the count the String object inside `note` would have reached.
- Step 9 refuses each of the four documents
  `stale_dependency`/`byte-digest-mismatch` carrying the selected digest and
  the raw bytes' own digest (check 1.3), with no `wrong-value-kind` from
  `token`; the oversize document refuses `stage_limit_exceeded`/
  `input-bytes-exceeded` naming `observation.input_bytes` (check 1.2).
- Assertions compare typed code, cause, input path, member and bound, never
  rendered text.

## Traceability

The expected results verify FR-106-AC-28 to FR-106-AC-32 and the FR-056
String wire object (FR-056-AC-23) at the document edge.
