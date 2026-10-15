---
id: SR-11260
title: "Spec review of quire-spec-language PR #866: UUID and Timestamp values in snapshots and invocations"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@e0de2ddec5c3862f63d139cde72839356bb7f811; spec/functional/FR-106-admit-snapshots-and-invocations.md, spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md, spec/test-cases/TC-1029-snapshots-carry-uuid-and-timestamp-values.md"
review_set: subset
---
# Spec review of quire-spec-language PR #866

## Summary

Ticket: QSL-791. One pass of spec-review with the contradiction, ambiguity,
completeness, testability and spec-integrity sub-analyses over the PR diff.
Spec-only PR, so no code review. Only major issues are reported. Mediums and
lows are recorded in the Verdict.

Oracles: FR-056 (K2 canonical native payloads, the runtime-input adapter
mapping and the FR-056-AC-17 controls), FR-106 check 1.3 and check 6, FR-360
item 9, QSpec FR-149 (UUID and Timestamp equality rows), and quire-exact
FR-370 (canonical UUID and Timestamp text).

Examined:
- FR-106 closed value grammar sentence and the new native payload value paragraph (examined)
- FR-106 declared-type fit paragraph and refusal order steps 1 to 7 (examined)
- FR-106 check 6.5 addition and the post-state Witness sentence (examined)
- FR-106-AC-18, FR-106-AC-19, FR-106-AC-20, FR-106-AC-21, FR-106-AC-22 (examined)
- FR-360 item 9 corrected sentence (examined)
- TC-1029 preconditions, steps 1 to 10 and expected results (examined)
- FR-106 check 1.3 shared-reader rule and the code that implements it (context_only)
- Open PR #865's FR-070 witness rows and PR #867's new test cases (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Step 1 `duplicate-member` cannot be reached. FR-106 check 1.3 says the shared reader (FR-261) refuses a document with a repeated member name. Such bytes are digested raw and refuse at check 1 before any value is read: `byte-digest-mismatch` against the canonical digest, and `unknown_wire`/`unsupported-wire` in the current reader when the raw digest matches. So AC-20's "a repeated `text` refuses `duplicate-member`" and TC-1029 step 6's two repeated-member rows expect a result the document edge can never give. Delete step 1 and those rows, or state that check 1.3 settles a repeated member. Neither the record form nor FR-360 item 9 has a duplicate step. | spec/functional/FR-106-admit-snapshots-and-invocations.md:184; :510; spec/test-cases/TC-1029-snapshots-carry-uuid-and-timestamp-values.md:66-71 |
| FND-002 | high | The order checks the kind-dependent member set before it validates `kind`. Step 2 (missing payload member, `text` or `nanoseconds`) and step 3 (unknown member) both need to know the kind, but `kind` is first checked at step 4 (not a string) and step 5 (unknown string). No rule says which members are expected when `kind` is `1`, `"string"`, `"UUID"` or `"uuidv4"`. A literal reader could refuse AC-20's vectors `missing-member` or `unknown-member` instead of the stated `wrong-value-kind` or `invalid-value`, and `{"kind":"string","nanoseconds":"0"}` has no defined result at all. Check `kind` first: a non-string kind is `wrong-value-kind` and an unknown kind is `invalid-value`, both before the member-set checks. FR-360 item 9 (a) checks the tag first, and PR #865's witness decoder already dispatches on `kind` before it checks members. AC-20's case of an extra member plus noncanonical `text` stays `unknown-member`. | spec/functional/FR-106-admit-snapshots-and-invocations.md:184-198; :510 |
| FND-003 | medium | AC-21's clause "including when the object's payload is also noncanonical", and TC-1029 step 7's matching "even where the payload is also noncanonical", have no vector. Every step 7 row holds a canonical payload. This clause is the only test of the chosen order (type fit, step 6, before the payload check, step 7), so as written it is vacuous. Add a row such as a Timestamp object with `nanoseconds` `"01"` in `token`, expecting `wrong-value-kind`. | spec/functional/FR-106-admit-snapshots-and-invocations.md:511; spec/test-cases/TC-1029-snapshots-carry-uuid-and-timestamp-values.md:72-77,108-112 |
| FND-004 | high | The id TC-1029 collides with open PR #867, which adds `TC-1029-r-name-over-a-relationship-end-types-by-the-destination-multiplicity.md`. Whichever PR merges second leaves two artifacts with one id on main, and spec validation goes red. If #867 lands first, renumber this TC and update FR-106-AC-18 to AC-22's `Test (TC-1029)` cells and the TC Scope line. | spec/test-cases/TC-1029-snapshots-carry-uuid-and-timestamp-values.md:2 |

## Verdict

**FAIL: 4 blocking findings (FND-001, FND-002 and FND-004 high; FND-003 the vacuous-vector rule).**

Verified clean against the oracles:
- All 10 UUID texts in step 4 are noncanonical (D6 has 32 hex digits with a moved hyphen; D8 has 31).
- The 12 Timestamp texts in step 5 match their classes: 9 noncanonical; 2^127 and -2^127-1 out of domain; the leading-zero exterior value noncanonical first. This agrees with FR-056, FR-056-AC-17 and quire-exact FR-370.
- Every admit vector is canonical, and both signed-128 endpoints are correct.
- Equality and the frame check compare by canonical payload, which agrees with QSpec FR-149.
- Each object counts as one value.
- The Witness mode exempts only out-of-range integers and still refuses native payload defects, which agrees with check 6.5.
- No test parses FR-106 or TC tables. Only code comments cite FR-106 line numbers.
- The diff has no ticket ids and no tracking, hash or version ceremony.

Author's choices:
1. A `kind` member marks a native object. This is consistent, since no existing tag is `kind`.
2. Type fit before the payload check. This is consistent with FR-106 check 6.5 and FR-360 (d) before (e), but it has no vector (FND-003).
3. An unknown kind string is `invalid-value`. This follows FR-056's "invalid tag" mapping.
4. `duplicate-member` at admission is wrong (FND-001).
5. `{"text":..}` with no `kind` is not ambiguous: in a UUID or Timestamp slot, step 6 and "no other form fits" make it `wrong-value-kind`.
6. The FR-360 correction is right.

Cross-PR: PR #865 at 2db8acc already makes FR-070's UUID and Timestamp witness rows reuse FR-056's closed `kind` object. The #866 spec text needs no rewording and FR-070 needs no note from #866. Only #866's PR body is stale, where it says FR-070 is a separate `{"type":"uuid","value":..}` wire.

Recorded, not reported:
- medium: AC-18 and TC step 2 say `00112233-4455-0677-0899-aabbccddeeff` has non-RFC 9562 version and variant bits. The variant nibble `0` (0b0xxx, NCS) is an RFC 9562 variant value; only the version nibble `0` is outside the table. The vector still admits correctly.
- low: an unknown native kind is `invalid-value`, while FR-360-AC-10 makes an unknown single tag such as `{"string":..}` `wrong-value-kind`. Each follows its own owner rule.
- low: the TC precondition "TC-465's `archive` package variant" names the fixture loosely.
- low: code comments cite FR-106 line numbers, which this insertion shifts (observation.rs:1711, state_clauses.rs:4357).
