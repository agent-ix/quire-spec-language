---
id: SR-11265
title: "Spec review of quire-spec-language PR #869: String values in snapshots and invocations"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@8da55f52cc4547499ca553aed106e5093fdca55f; spec/functional/FR-106-admit-snapshots-and-invocations.md, spec/functional/FR-070-implement-typed-counterexample-witness-envelope.md, spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md, spec/test-cases/TC-1036-snapshots-carry-string-values.md, spec/test-cases/TC-1029-snapshots-carry-uuid-and-timestamp-values.md, spec/test-cases/TC-905-composite-witness-value-text-decodes-and-refuses.md"
review_set: subset
---
# Spec review of quire-spec-language PR #869

## Summary

Ticket: QSL-796. One pass of spec-review with the contradiction, ambiguity,
completeness, testability and spec-integrity sub-analyses over the PR diff.
Spec-only PR, so no code review. Only major issues block. Mediums and lows
are recorded in the Verdict.

Oracles: FR-056 (K2 String row, FR-056-AC-23, the reader parse rule), FR-106
checks 1.1 to 11 in order (check 1.3's shared-reader rule, check 6.5 and the
merged native payload order), FR-261, FR-259, FR-255 (`observation.input_bytes`
1048576), FR-070 transcript escaping and decode rules, QSpec FR-149, the
quire-exact native String value and the quire-canonical reader.

Examined:
- FR-106 closed value grammar sentence, native payload value paragraph, fit paragraph and native order steps 1 to 7 (examined)
- FR-106 check 6.5 sentence and the post-state Witness sentence (examined)
- FR-106-AC-20 and FR-106-AC-28 to FR-106-AC-32 (examined)
- TC-1036 preconditions, steps 1 to 9 and expected results (examined)
- TC-1029 step 6 and its expected result (examined)
- FR-360 item 9 (examined)
- FR-070 String row, FR-070-AC-11 and TC-905 step 2 (examined)
- FR-106 check 1.1 to 1.3, FR-261, FR-259 shared reader (context_only)
- Open PR #865's FR-070 UUID and Timestamp rows (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | TC-1036 lost two of its JSON escapes, so two vectors are vacuous. In step 2 the row described as "the paired escape" holds the literal U+1F600 bytes (F0 9F 98 80), not `😀`, so it is the same document as the "literal character U+1F600" row. In step 6 the post `label` "written `"ready"`" is the base spelling, not `"ready"`, so it cannot show that escape spelling is no change. FR-106-AC-28's surrogate-pair claim and FR-106-AC-31's escape-spelling claim therefore have no vector that could fail. Write both as escapes in the TC text, as steps 2 and 9 already do for `\u0000` and `\ud800`. | spec/test-cases/TC-1036-snapshots-carry-string-values.md:45-46,71,91-92,115-116 |

## Verdict

**FAIL: 1 blocking finding (FND-001, a vacuous vector).**

Every TC-1036 row was traced through FR-106 checks 1 to 11 in order:
- Step 9: lone surrogate escapes and the byte `0xC0` are reader refusals (quire-canonical refuses non-UTF-8 and lone surrogates). Check 1.3 digests them raw, so they give `byte-digest-mismatch` under the base digest before check 6. The wrong-kind `token` row is pre-empted correctly. The 1,048,576-`a` document exceeds `observation.input_bytes` (1048576) at check 1.2.
- Step 4: `kind` checks 1 and 2 come before the member checks 3 to 5, so each expected result is reachable. `{"kind":"string","nanoseconds":"0"}` gives `missing-member` `text`. An extra member with `text` `5` gives `unknown-member`.
- Step 5: the String object with `text` `5` in `token` gives `wrong-value-kind` naming `text`, and the extra member in `versionNumber` gives `unknown-member`. Both follow the native order (steps 4 and 5 come before fit at step 6). The bare JSON string and `{"absent":{}}` in `label` give `wrong-value-kind` by check 6.5.
- Steps 1 to 3, 6 to 8: digest equality under JCS, frame comparison by exact scalar sequence, Witness still refusing a wrong kind and value counting all agree with the FR-106 text.

Other checks:
- No other row, AC or TC still uses `string` as an unknown kind. FR-106-AC-20 and TC-1029 now use `String`.
- No test parses FR-106, FR-070, FR-360, TC-905 or TC-1029. Only FR-092 is parsed by a test.
- The diff has no ticket ids and no tracking, hash or version ceremony.
- TC-1036 and FR-106-AC-28 to AC-32 collide with no open PR.

Author's choices:
1. No canonical grammar for String. This is consistent: FR-056's String row and the quire-exact `Value::String` ("an unbounded native Unicode scalar sequence") have no normalization, length or NUL rule.
2. Lone surrogates and invalid UTF-8 are refused at check 1.3. This is consistent with FR-056's parse rule, FR-261, FR-260 and the quire-canonical reader.
3. No String length limit. Consistent.
4. A bare JSON string is `wrong-value-kind` by check 6.5's fit rule. This was already implied. Consistent.
5. Text fields have no snapshot form. Consistent.
6. Identities are not String values. Consistent.
7. The FR-070 and TC-905 hunks are optional. They merge cleanly with the FR-070 rows of open PR #865 (checked with a three-way merge-file), and they follow the same "FR-056 closed wire object" pattern. The `a;b>>>c` and empty-String vectors are correct under FR-070's generic four-character escaping. They are not defects. However, they put a witness-decoder String arm into FR-070-AC-11, an AC that main already implements and trace-tags. That code is in the files #865 is rewriting, and the ticket's code slice (the reader and the admit_leaf arms) does not include it. Keep them only if the QSL-796 code slice takes the decoder arm after #865 lands. Otherwise drop them.

Recorded (medium/low, not blocking):
- medium: the FR-070 hunks widen the code slice into the witness decoder (choice 7 above).
- low: FR-070's witness paragraph still lists leaf families without String, and the String row is not next to #865's native rows.
- low: TC-1036 step 4 has no row for `kind` `"text"` with a `text` member. AC-29 lists one, and only `{"kind":"text","value":"x"}` is covered.
- low: the postcondition clause on `rename` and the "without the grant" frame are not named in the TC-1036 fixture. This matches TC-1029.
