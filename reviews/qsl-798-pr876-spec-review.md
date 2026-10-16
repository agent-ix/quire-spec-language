---
id: SR-11406
title: "Spec review of quire-spec-language PR #876: Text snapshot form and String and Text witness rows"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@9fa5a84229a4f1ab6582d46b3b4b12c5868f86a5; spec/functional/FR-070-implement-typed-counterexample-witness-envelope.md, spec/functional/FR-098-execute-a-replay-request.md, spec/functional/FR-106-admit-snapshots-and-invocations.md, spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md, spec/test-cases/TC-1028-unit-equality-on-native-string-uuid-and-timestamp.md, spec/test-cases/TC-1036-snapshots-carry-string-values.md, spec/test-cases/TC-1038-snapshots-carry-text-values.md, spec/test-cases/TC-905-composite-witness-value-text-decodes-and-refuses.md"
review_set: subset
---
# Spec review of quire-spec-language PR #876

## Summary

Ticket: QSL-798. One pass of spec-review with the contradiction, ambiguity,
completeness, testability and spec-integrity sub-analyses over the PR diff.
Spec-only PR, so no code review. Only major issues block. Mediums and lows
are recorded in the Verdict.

Oracles: FR-106 (payload rule 1 to 7, checks 1.2, 1.3, 6, 10 and 11, the
`PostStateRange::Witness` rule, the `sha256-jcs` digest rule), FR-070 (decode
rules, transcript escaping), FR-098, FR-096, FR-056, FR-360, TC-1028,
TC-1029 and TC-1036 as merged, and QSpec FR-141, FR-149 and FR-181 on main
(cited by id only).

Examined:
- FR-070 witness value text paragraph, the text and String rows, the new
  `kind` decode bullet, FR-070-AC-14, FR-070-AC-15 (examined)
- FR-098 leaf list, kind-mismatch bullet, conversion rule, FR-098-AC-13
  (examined)
- FR-106 value grammar list, payload object paragraph, Text value paragraph,
  type-fit paragraph, payload rule 2, 3, 4, 6 and 7, the record and
  `PostStateRange::Witness` edits, FR-106-AC-29, FR-106-AC-33 to
  FR-106-AC-38 (examined)
- FR-360 item 9 edit (examined)
- TC-1028 steps 6 and 7 with W1 to W5 and their expected results (examined)
- TC-1036 step 4 and expected-result edits (examined)
- TC-1038 all steps and expected results (examined)
- TC-905 steps 7 and 8 and their expected results (examined)
- FR-096 TextLengthOutOfDomain row, FR-098 replay settlement (proved verdict
  `violation`, FR-098-AC-2, FR-098-AC-5), QSpec FR-141 profiles and length,
  QSpec FR-149 Text row, QSpec FR-181 text form (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-070-AC-15's extra-member vector `{"kind":"string","text":"x","extra":"y"}` (repeated in TC-905 step 8) is not in JCS member order: JCS sorts `extra` before `kind` and `text`. FR-070 decode already refuses an entry that "is not its own JCS encoding (... reordered members ...)", so this row refuses `Malformed` for the member order whether or not an extra member is checked. It duplicates the `{"text":"x","kind":"string"}` member-order row and cannot fail when the extra-member check is missing, so the "(an extra member)" condition the AC claims is never tested. Use the JCS-ordered spelling `{"extra":"y","kind":"string","text":"x"}` in both FR-070-AC-15 and TC-905 step 8. | spec/functional/FR-070-implement-typed-counterexample-witness-envelope.md:254, spec/test-cases/TC-905-composite-witness-value-text-decodes-and-refuses.md:105 |

## Verdict

**FAIL: 1 blocking finding (FND-001).**

Every new vector was traced through the checks:

- FR-070-AC-14 and TC-905 step 7: `a;b>>>c` gives `a%3Bb%3E%3E%3Ec`, `100%<`
  gives `100%25%3C`, and `say "hi"\x;` gives JCS `say \"hi\"\\x;`, escaped to
  `say \"hi\"\\x%3B`. All match the four-escape rule. U+00E9 and U+0065
  U+0301 are spelled as literal scalars, so they are two distinct documents.
  The `é` refusal row is spelled out as ASCII and is distinct from the
  literal row. A `grep` for non-ASCII bytes in the added lines finds none.
- The other FR-070-AC-15 rows each reach `Malformed` for their stated reason.
  The positive objects are in JCS order (`kind` < `nanoseconds` < `text`,
  `type` < `value`).
- FR-106-AC-33 to AC-38 and TC-1038: every length was recomputed. Under
  `nfc`, eight or nine e+U+0301 pairs give 8 or 9. Under `unicode-scalars`,
  four U+00E9 give 4, `abcde` gives 5 and three pairs give 6. Under
  `binary-utf8`, two U+00E9 give 4 bytes, U+1F600 gives 4, three U+00E9
  give 6 and two U+1F600 give 8. `""` gives 0 against the minimum of 1, and
  65,536 `a` stays under 1 MiB. Payload rule 1 to 5 are reached before type
  fit (6) and length (7). A nine-`a` Text in `label` is `wrong-value-kind`.
  `\ud800`, `\udc00` and 0xC0 settle at check 1.3. 1,048,576 `a` exceeds the
  1 MiB default at check 1.2.
- Digest: `sha256-jcs` keeps the supplied scalars, so U+00E9 and U+0065
  U+0301 have two digests and, under `nfc`, one retained value. This is
  consistent with FR-106-AC-28 for String and with QSpec FR-141 and FR-149
  (equality and length on the profile's retained sequence, bytes under
  `binary-utf8`).
- TC-1028 W1 to W5 each reach FR-098's new kind-mismatch bullet in parameter
  order, from both the `Input` and the `Witness` arm.

Author choices, judged for consistency:
1. In FR-070, Text has one spelling, QSpec FR-181's type/value. String has
   one spelling, FR-056's kind/text, which is the pattern the merged UUID and
   Timestamp rows use. FR-106 snapshots are a separate grammar (`{"integer":..}`
   and so on), so the kind/text Text object there is not a second FR-070
   spelling. This is consistent with the one-spelling-per-table rule.
2. Consistent, as traced above.
3. The `invalid-value` mapping matches FR-106's admission precedent
   (integers) and FR-056's adapter.
4. Stated explicitly. The exemption covers integers only (FR-106-AC-12).
5 to 8. No conflict.

No id collisions: TC-1038, FR-106-AC-33 to AC-38, FR-070-AC-14 and AC-15 and
FR-098-AC-13 are not on main. Open #877 uses TC-1039 and TC-1040, and #875
uses TC-1041 and FR-106-AC-39. No ticket or PR ids and no tracking or hash
ceremony appear in the added spec text. No test includes or parses FR-070,
FR-098, FR-106, TC-905, TC-1028, TC-1036 or TC-1038; the only parsed spec
files are FR-092, FR-093, FR-094 and FR-255, all untouched. No row elsewhere
still uses `text` as an unknown kind.

Recorded (non-blocking):
- medium: FR-098-AC-13 and TC-1028 step 6 do not name the FR-072 settlement.
  FR-098 makes the proved verdict `violation`, so the `true` rows (E1, E2, E5
  and the negated `nes` rows) settle `inconclusive`/`Verdicts`, and the `false`
  rows settle `reproduced-without-witness`. "Completes without a refusal" is
  true but leaves this unstated.
- low: TC-1038 step 9 does not say which object's `aliases` (child or root)
  holds the last value in walk order. The same wording is in TC-1036.
- low: the `subtitle` rows in TC-1038 step 6 omit the `{"present":...}`
  wrapper and the option-value path step.
- low: FR-106-AC-36 names the refusal's bounds, profile and length, but not
  their field keys (FR-096 spells `expected`).
