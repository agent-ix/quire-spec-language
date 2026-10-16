---
id: TC-1038
title: "Snapshots and invocations carry Text values"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: verifies
---
# TC-1038: Snapshots and invocations carry Text values

## Description

Verify FR-106's Text payload object: admission of a text under its declared
`Text[min, max; profile]`, the length bound measured on the retained sequence
under each profile, each object refusal with its code, cause, member and nested
path in the payload order, declared-type fit against String, frame comparison
by the retained sequence, value counting and the document-edge refusals that
come before any value is read.

Scope: FR-106-AC-33, FR-106-AC-34, FR-106-AC-35, FR-106-AC-36, FR-106-AC-37,
FR-106-AC-38.

## Preconditions

- Steps 1, 3 and 7 to 10 declare the Text value types below and so depend on
  the amended semantic-IR validator that FR-056's Text paragraph states.
- TC-1036's package, with Text value types, declared in the form of FR-056-AC-37, `Title` (`Text[1, 8; nfc]`), `Tag`
  (`Text[0, 4; unicode-scalars]`) and `Code` (`Text[0, 4; binary-utf8]`), and
  `ConfigVersion` also gaining, after its other fields and in this declared
  order, `title: Title`, `subtitle?: Tag`, `code: Code` and `aliases:
  Sequence<Tag>`, and an operation `retitle(name: Title): Title` on
  `ConfigVersion` whose frame modifies `title`.
- TC-1036's base snapshot, with `child` also holding `title`
  `{"kind":"text","text":"Ready"}`, `subtitle` `{"absent":{}}`, `code`
  `{"kind":"text","text":"abc"}` and `aliases` `{"sequence":[]}`, and `root`
  holding `title` `{"kind":"text","text":"Root"}`, `subtitle`
  `{"absent":{}}`, `code` `{"kind":"text","text":"r"}` and `aliases`
  `{"sequence":[]}`. Digests are recomputed for every mutated document, except
  where a step says the selected digest stays the base snapshot's.
- Scalars are named by code point. Where a row says `U+00E9` or a pair, the
  document holds the literal scalars or their JSON `\u` escape, as the step
  says; the digest is the same either way.

## Test Procedure

1. Admit the base snapshot for `ParentOrder` with `Current` selection
   (anchor `handler validate`, `self` `child`). Reorder the members of the
   `title` object and the whitespace of the document, and admit again.
2. Admit one mutation per row of `title` texts, each with its own digest:
   `"a"`; `"abcdefgh"`; `"a;b>>>c"`; T1 U+00E9; T2 U+0065 U+0301; eight
   pairs of U+0065 U+0301 (sixteen scalars); `"\u0000"`; the paired escape
   `"\ud83d\ude00"`, and separately the literal character U+1F600. Admit
   `subtitle` as `{"present":{"kind":"text","text":"abcd"}}` and as four
   U+00E9, and `aliases` as
   `{"sequence":[{"kind":"text","text":"a"},{"kind":"text","text":""}]}`.
   Admit `code` as `"abcd"`, as two U+00E9 and as one U+1F600.
3. Admit an invocation of `retitle` (`name` a Text object, `result` a Text
   object), selected for a postcondition on `retitle`, and the same invocation
   selected through `PreCall`.
4. Mutate `title` to each object defect, one per row: `kind` `1`; `kind`
   `null`; `kind` `{}`; `kind` `1` with no `text`; `kind` `"Text"`; `kind`
   `"TEXT"`; `{"kind":"blob","value":"x"}`; `{"kind":"blob","text":"x"}`;
   `{"kind":"text"}`; `{"kind":"text","nanoseconds":"0"}`; an extra member
   `"extra":1` beside a valid `text`; a valid `text` beside
   `"nanoseconds":"0"`; `text` `5`; `text` `null`; `text` `true`; `text`
   `["x"]`; `text` `{}`; an extra member `"extra":1` together with `text` `5`.
5. Mutate for fit, one per row: a String object in `title`; as a `subtitle`
   value (`{"present":<String object>}`); as an element of `aliases` at index 1
   (after a valid Text object at index 0). A Text object in `label`; in
   `token`; in `stamp`; in `versionNumber`; as a `parent` value
   (`{"present":<Text object>}`). A UUID object, a Timestamp object,
   `{"integer":"5"}`, `{"boolean":true}`, `{"absent":{}}` and the bare JSON
   string `"Ready"` in `title`. A Text object with text
   `00112233-4455-6677-8899-aabbccddeeff` in `token`. A Text object with a
   text of nine `a` in `label`. A Text object with `text` `5` in `token`, and
   one with an extra member `"extra":1` and a valid `text` in
   `versionNumber`. Repeat the String-object row as the `name` argument of
   `retitle`, and a UUID object as its `result`.
6. Mutate for length, one per row. `title`: `""`; `"abcdefghi"`; nine pairs of
   U+0065 U+0301; a text of 65,536 `a`. `subtitle`, each as `{"present":<Text object>}`: `"abcde"`; three pairs of
   U+0065 U+0301. `code`: three U+00E9; two U+1F600. Repeat the empty text as
   the `name` argument of `retitle` and as its `result`.
7. Admit an invocation of `retitle` changing `child.title` from `"Ready"` to
   `"Steady"`, with the grant and without it. Admit one changing `child.title`
   from T1 to T2, and one whose post `title` is `Ready` with the members of
   the object reordered, and one whose post `title` is written
   `"\u0052eady"` (ASCII backslash escape of R), each without the grant. Admit one changing `child.subtitle`
   from `{"present":<Text object of U+00E9>}` to
   `{"present":<Text object of U+0065 U+0301>}`, without a grant for
   `subtitle`.
8. Under `PostStateRange::Witness`, admit an invocation whose post snapshot
   holds a String object in `title`, one whose post `title` is `""`, and one
   whose post `title` has `text` `5`.
9. Mutate `child`'s `aliases` (`child` follows `root` in document order, so it
   is the last object walked) to `{"sequence":[{"kind":"text","text":"x"}]}` and admit
   the document with `observation.values` set to its exact value count, then
   to one less.
10. Select each of these documents under the base snapshot's digest (the
    selected digest stays the base's): a `title` text holding the escape
    `\ud800`; one holding the escape `\udc00`; one holding the raw byte
    `0xC0`; the first of them with a wrong-kind `token` (a Timestamp object) as
    well. Then admit a document whose `title` text has 1,048,576 `a`, so that
    the document exceeds 1 MiB.

## Expected Results

- Step 1 admits one observation in which `child.title` is a Text value of
  the retained sequence `Ready`; the reordered document gives the same typed
  observation and digest.
- Step 2 admits every row. Each row's retained sequence is the text under
  `nfc`: T1 and T2 retain one scalar U+00E9 and are equal values with two
  different digests, the sixteen scalars retain eight, and the paired escape
  and the literal U+1F600 give one value and one digest. `"a;b>>>c"` retains
  seven scalars. `subtitle` is present, and the four-U+00E9 `subtitle` and
  the three `code` rows retain their scalars and bytes unchanged. `aliases`
  holds `a` and the empty Text, in order; the empty text is a `Tag` value
  because `Tag`'s minimum is `0`.
- Step 3 admits `name` and `result` as Text values in both selections, with
  the `PreCall` selection reading no result.
- Step 4 refuses every row `invalid_runtime_input` naming `child` and `title`
  by the listed order: the `kind` `1`, `null`, `{}` rows and the `kind` `1`
  row with no `text` give `wrong-value-kind` naming `kind` (check 1); the
  `"Text"`, `"TEXT"`, `blob`/`value` and `blob`/`text` rows give
  `invalid-value` naming `kind` (check 2), their members unexamined;
  `{"kind":"text"}` and the `text`/`nanoseconds` object give `missing-member`
  naming `text` (check 3); the two extra-member rows give `unknown-member`
  naming `extra` and `nanoseconds` (check 4), and so does the row holding both
  an extra member and `text` `5`, naming `extra`; the `text` `5`, `null`,
  `true`, `["x"]` and `{}` rows give `wrong-value-kind` naming `text`
  (check 5).
- Step 5 refuses every row `invalid_runtime_input`/`wrong-value-kind` at the
  nested path (`subtitle`, `parent` with their option-value step, `aliases`
  with index 1, the `retitle` binding root for the argument and result) and
  names no member, except: the `text` `5` row in `token` names `text` (check 5
  comes before type fit), and the extra-member row in `versionNumber` refuses
  `unknown-member` naming `extra` (check 4 comes before type fit). The Text
  object whose text is a canonical UUID is `wrong-value-kind` in `token` and
  no UUID value is read; the nine-`a` Text object in `label` is
  `wrong-value-kind`, not `invalid-value`, because type fit comes before the
  length check.
- Step 6 refuses every row `invalid_runtime_input`/`invalid-value` naming
  `text`, the declared bounds, the profile and the measured length, at the
  field (`subtitle` with its option-value step; `name` and `result` at the `retitle` binding root): the lengths are
  0, 9, 9 retained scalars and 65,536 for `title`, 5 and 6 for `subtitle`, and
  6 and 8 bytes for `code`. No row is `input-bytes-exceeded`.
- Step 7 admits the first with the grant, and refuses it without as
  `frame_violation`/`unauthorized-change` naming `child` and `title`. The
  T1-to-T2 post, the reordered-members post and the `"\u0052eady"` post are no
  change under `nfc`, so each admits without the grant. The `subtitle` change
  is a change under `unicode-scalars` and refuses
  `frame_violation`/`unauthorized-change` naming `child` and `subtitle`.
- Step 8 refuses the String-object post `wrong-value-kind`, the empty-text post
  `invalid-value` naming `text`, and the `text` `5` post `wrong-value-kind`
  naming `text`; `Witness` exempts none of them.
- Step 9 admits at the exact count; one less returns
  `stage_limit_exceeded`/`node-count-exceeded` naming `observation.values`,
  the bound and the count the Text object inside `aliases` would have reached.
- Step 10 refuses each of the four documents
  `stale_dependency`/`byte-digest-mismatch` carrying the selected digest and
  the raw bytes' own digest (check 1.3), with no `wrong-value-kind` from
  `token`; the oversize document refuses `stage_limit_exceeded`/
  `input-bytes-exceeded` naming `observation.input_bytes` (check 1.2).
- Assertions compare typed code, cause, input path, member and bound, never
  rendered text.

## Traceability

The expected results verify FR-106-AC-33 to FR-106-AC-38 and the Text object's
profile semantics of QSpec FR-141 at the document edge.
