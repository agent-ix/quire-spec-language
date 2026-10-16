---
id: SR-11696
title: "Code review of quire-spec-language PR #893: Text value types at intake and typed frame comparison"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@1aa51904f8d83a9d2524d78b233677764da392b1; qsl-semantics/src/model/intake.rs, qsl-semantics/src/model/domain_package.rs, qsl-semantics/src/model/index.rs, qsl-semantics/src/check/assemble.rs, qsl-semantics/src/model/observation.rs, qsl-semantics/src/model/observation/document.rs, qsl-semantics/src/model/observation/frame.rs, qsl-semantics/tests/it/native_snapshot_values.rs, qsl-semantics/tests/it/model_operations.rs, qsl-semantics/tests/support/native_values.rs"
review_set: subset
---
# Code review of quire-spec-language PR #893

## Summary

Ticket: QSL-806 (and QSL-798). One pass of code-review with the rust-review
lane over `git diff origin/main...HEAD` (7 commits). No builds were run, by
brief.

Oracles: FR-056's Text paragraph (the six-step reader order, the validator
rows, the native-String `textProfile` row, the retained-sequence domain),
FR-056-AC-11, AC-37, AC-38; FR-106-AC-33, AC-36, AC-37, AC-38, AC-40;
TC-1038 steps 1, 3, 7 to 10; TC-1044 steps 1 and 2; TC-897 step 3. The pinned
semantic-IR validator (filament-core-data `fd261ca`) was read to confirm which
TC-1044 step 2 rows it decides.

Examined: `read_text_value_type`, the `textProfile` refusal in
`read_value_type_ref`, `ScalarValueDomain::Text` and its arms in `assemble.rs`
and `index.rs`, `SnapshotValue::equals_by`, `retained_text`, `text_member`,
`admit_text`, `TypedRaw` and `typed_values_equal`, and every new test.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The Text reader refuses rows (b), (c) and (d) at the constraint node, not at the value type. FR-056's Text paragraph says every reader refusal is "at the value type", and FR-056-AC-38 says each is "at `Title`". The unknown-keyword and non-profile-`format` refusals call `unsupported_at(constraint, &constraint_at, ..)`, and the duplicate-keyword refusal calls `constraint_ctx.malformed_in(..)`. Both take the node from `node_identity_label(constraint, ..)`. A real constraint carries an `identity` (TC-1044's `ix://P/constraint/Title/pattern`), so the refusal's `node` is the constraint, not `Title`. The test cannot see this. The `title()` fixture writes constraints with no `identity`, so the node falls back to the path. The assertion only checks `detail.starts_with("$.types[0]")`, and `$.types[0].constraints[3]` passes that. Fix: build these three refusals over the value type's `ctx` (node `Title`), keep the constraint location as the member path, and assert `node` in the test with constraint identities in the fixture. | qsl-semantics/src/model/intake.rs:3403-3429, qsl-semantics/src/model/intake.rs:6693-6792 |

## Verdict

FAIL: 1 blocking finding (FND-001, a refusal location that contradicts the
spec, with a test too weak to catch it).

Clean:
- The reader order matches the six steps. All unknown keywords and non-profile
  formats are checked in constraint order before any duplicate. Then come the
  duplicates in constraint order, missing `format`, missing `minLength` then
  `maxLength`, the profile outside the six, then the bounds. Row (k), profile
  before bounds, holds.
- Codes match the paragraph: `declaration-form` for steps 1, 3, 5 and 6, and
  `malformed-declaration` for steps 2 and 4.
- TC-1044 step 2 rows (a), (l), (m) and (n) are owned by the validator. The
  pinned validator's schema refuses a `textProfile` on any definition whose
  `kind` is not the string `scalar` (a value type's `kind` is an object). It
  refuses a length operand that is not a non-negative integer, and a `format`
  name with an empty part after the colon (`is_namespaced_name`). `NFC` and
  `email:rfc` pass that pattern and reach the reader, as rows (c) and (i)
  require. No QSL reader code owns rows a/l/m/n. If the reader ever saw
  `-1` or `1.5` it would still refuse `malformed-declaration`.
- The pinned validator already has the applicability change: a value type
  carrying `scalar` is decided as that scalar. So the fixture's admitted Text
  rows are real, not a workaround.
- The native-String `textProfile` row refuses `declaration-form` at the member
  `textProfile`, naming `textProfile`.
- Typed equality brings no regression for other forms. `PartialEq` is
  `equals_by(None)`, which is the old walk. With a declared type, only a pair
  of Text natives under a declared `Text` changes behaviour. Integer, UUID,
  Timestamp, String, Reference and Boolean still compare as written.
  `Present` passes the bare field type through: fields are never `Option`, per
  `model_value_type`. `Sequence` takes the collection element type, which
  `model_value_type` builds for any multiplicity other than `[1,1]`.
- `retained_text` is the one normalization, shared by admission and
  comparison. Under `nfc`, U+00E9 equals U+0065 U+0301. Under
  `unicode-scalars` and `binary-utf8` the payload is compared as is. Digests
  are untouched: they are still over the raw bytes, as TC-1038 step 2 wants
  (equal values, different digests).
- No `unwrap`, `expect` or panic in non-test code. No ceremony, vendoring or
  compatibility layer. The changed old row (`scalar:"string"` to `boolean` as
  the unsupported example) is correct, since `string` now has a reader.

Recorded (not blocking):
- medium: `equals_by` pushes `None` for `Record` and `Variant` members. A
  record value type with a Text field (FR-056-AC-21's `Address { street:
  Text[..; nfc] }`) would compare its Text raw in check 11. Neither AC-40 nor
  this ticket lists records, and reachability through snapshot admission was
  not verified.
- medium: the TC-1038 step 7 tests use the single-object fixture `Item.i1`.
  They assert the field (`title`, `subtitle`) but not the object, where the
  spec rows name `child`.
- low: the refusal `what` strings (`value-type/v1:constraints:<kw>`,
  `:profile`, `:profile=<name>`, `:bounds=<min>..<max>`) name what each row
  names, and follow the integer reader's convention.
