---
id: SR-11413
title: "Gap analysis — QSL-791/QSL-796 PR #879 native payload objects"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@84c9a9084; QSL-791; QSL-796; FR-106-AC-18..22; FR-106-AC-28..32; TC-1029; TC-1036; qsl-semantics/tests/it/native_snapshot_values.rs; qsl-semantics/tests/support/native_values.rs"
review_set: subset
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-106, type: references }
  - { target: ix://agent-ix/quire-spec-language/TC-1029, type: references }
  - { target: ix://agent-ix/quire-spec-language/TC-1036, type: references }
---

## Summary

Ticket: QSL-791, QSL-796. Plan completion: not assessed. I checked each of the 19 tests in `native_snapshot_values.rs` against the merged TC-1029 and TC-1036 tables and the AC text on `origin/main`. Red-to-green evidence: 18 of the 19 tests fail with the native branch disabled. The one that passes is the check 1.3/1.2 document-edge test, which exercises existing behaviour. All 23 pass green.

## Verdict

FAIL: one blocking finding. The TC-1029 vectors in steps 1 to 10 match the spec, including `kind` `1` with no `text`, the extra-member-plus-noncanonical row, the `"01"`/`"X"` fit-before-payload rows and `0170…728` as noncanonical. TC-1036 steps 1-3 and 5-9 match, including the `😀` escape and the `ready` frame rows, which are written as escapes, and `kind` `1` with no `text`. The TC-1036 step 4 test asserts the vectors as they stood before #876 merged, not the merged table.

Which test kills each mutant:
- Unknown-member before missing-member: killed by step 6 `{"kind":"uuid","nanoseconds":"0"}`.
- Payload type before unknown-member: killed by `{"kind":"string","extra":1,"text":5}`.
- Members examined before an unknown kind: killed by `{"kind":"String","nanoseconds":"0"}`.
- Fit before the envelope: killed by `text` `5` in `token` and an extra member in `versionNumber`.
- Payload check before fit: killed by `"01"` in `token`.
- Kernel class dropped or swapped: every step 4, 5 and 9 assertion compares `kernel`.
- Range check before the noncanonical check: killed by `0170…728`.
- An object counted as two values: killed by `values_used == 10` and `== 11`.

On step 10, asserting `actual = bound + 1` pins the refusal to the last value in walk order, which is all the AC asks.

## Examined Scope

```yaml
scope:
  - id: TC-1036
    path: spec/test-cases/TC-1036-snapshots-carry-string-values.md
    role: examined
    excerpt: "4. Mutate `label` to each object defect, one per row: ... `kind` `\"String\"`; `kind` `\"STRING\"`; `\"Text\"`; `{\"kind\":\"blob\",\"value\":\"x\"}`; `{\"kind\":\"blob\",\"text\":\"x\"}`; `{\"kind\":\"string\"}`; ..."
  - id: FR-106-AC-29
    path: spec/functional/FR-106-admit-snapshots-and-invocations.md
    role: examined
    excerpt: "`kind` `\"String\"`, `\"STRING\"` and `\"Text\"`, and `{\"kind\":\"blob\",\"value\":\"x\"}`, refuse `invalid-value` naming `kind`, the members unexamined"
  - id: FR-106-AC-18
    path: spec/functional/FR-106-admit-snapshots-and-invocations.md
    role: examined
    excerpt: "Nil ..., Max ..., a UUID whose version nibble is `0` ..., and Timestamps `1`, `-1`, ... admit unchanged ... each object counts as one value toward `observation.values`."
  - id: FR-106-AC-19
    path: spec/functional/FR-106-admit-snapshots-and-invocations.md
    role: examined
    excerpt: "`0170141183460469231731687303715884105728` refuses with `noncanonical-payload`, never the range class."
  - id: FR-106-AC-20
    path: spec/functional/FR-106-admit-snapshots-and-invocations.md
    role: examined
    excerpt: "`kind` as `1` or `null` refuses `wrong-value-kind` naming `kind`, even beside a missing payload member; ... An object with both an extra member and a noncanonical `text` refuses `unknown-member`."
  - id: FR-106-AC-21
    path: spec/functional/FR-106-admit-snapshots-and-invocations.md
    role: examined
    excerpt: "including a Timestamp object with `nanoseconds` `\"01\"` in `token` and a UUID object with `text` `\"X\"` in `stamp`, which are `wrong-value-kind`, never `invalid-value`."
  - id: FR-106-AC-22
    path: spec/functional/FR-106-admit-snapshots-and-invocations.md
    role: examined
    excerpt: "one less returns `stage_limit_exceeded`/`node-count-exceeded` at the last of those values in walk order."
  - id: FR-106-AC-28
    path: spec/functional/FR-106-admit-snapshots-and-invocations.md
    role: examined
    excerpt: "the paired escape `\"\\ud83d\\ude00\"`, which gives the value and digest of the literal U+1F600. U+00E9 and U+0065 U+0301 are two unequal values with different digests."
  - id: FR-106-AC-30
    path: spec/functional/FR-106-admit-snapshots-and-invocations.md
    role: examined
    excerpt: "`{\"kind\":\"string\",\"text\":5}` in `token` refuses `wrong-value-kind` naming `text`, and `{\"kind\":\"string\",\"text\":\"x\",\"extra\":1}` in `versionNumber` refuses `unknown-member`"
  - id: FR-106-AC-31
    path: spec/functional/FR-106-admit-snapshots-and-invocations.md
    role: examined
    excerpt: "the bound and the count the String object inside `note` would have reached."
  - id: FR-106-AC-32
    path: spec/functional/FR-106-admit-snapshots-and-invocations.md
    role: examined
    excerpt: "refuses `stale_dependency`/`byte-digest-mismatch` at check 1.3 carrying the selected digest and the raw bytes' own digest, before any value is read"
  - id: TC-1029
    path: spec/test-cases/TC-1029-snapshots-carry-uuid-and-timestamp-values.md
    role: examined
    excerpt: "10. Admit the base snapshot with `observation.values` set to its exact value count, then to one less."
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The TC-1036 step 4 test asserts `{"kind":"text","value":"x"}` and `{"kind":"text","text":"x"}` refuse `invalid-value` naming `kind`. Under merged FR-106 step 2, `text` is a known kind, so the first refuses `missing-member` and the second is a Text object refusing `wrong-value-kind` in `label` (AC-35). The test also leaves out the merged rows `"Text"`, `blob`/`value` and `blob`/`text`. | qsl-semantics/tests/it/native_snapshot_values.rs:1286-1287 |

## Review Notes

- Recorded, medium: three AC-28 clauses are not asserted: the same digest under reordering and whitespace, the S1 and S2 digests differing, and the paired escape giving the literal character's digest. The test checks only the value. AC-18's "same digest" clause is also unasserted.
- Recorded, low: AC-32's "carrying the selected digest and the raw bytes' own digest" is not asserted. Only the code and cause are.
- Recorded, low: TC-1029 step 3 matches `Datum::Uuid(_)` for `id`, `at` and `result` without checking the payload.
