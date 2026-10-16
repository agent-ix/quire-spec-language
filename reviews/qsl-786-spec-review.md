---
id: SR-11749
title: "Spec review — QSL-786 native String, UUID and Timestamp literals (PR #908)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@c1f411d215e2c2cf205d4b54b8fb12f974824fd8; PR #908; spec/functional/FR-056, FR-091, FR-092, FR-093, FR-360; spec/test-cases/TC-396, TC-1007, TC-1028, TC-1052"
review_set: base
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
---

## Summary

Ticket: QSL-786. One-pass spec review (base checklist with integrity and
code-consistency checks folded in) of the diff `origin/main...HEAD`. Checked
against the merged lexer (`qsl-cst/src/token.rs`, `qsl-cst/src/parser.rs`),
the CST diagnostic catalog (`qsl-cst/src/diagnostic.rs`), the grammar
transcription (`qsl-cst/src/grammar.rs`), S2's text-token refusal
(`qsl-forms/src/value.rs:1370`), the `IntegerOutsideI128` precedent
(`qsl-semantics/src/check/check/typing.rs:734`,
`qsl-semantics/src/check/refusal.rs:1088`), and QSpec FR-322 "Scalar literal
values" (read-only, by id). No build, no edit.

## Verdict

**CHANGES REQUESTED — 2 blocking findings.** The refusal codes are all in the
catalog (`invalid-escape`, `invalid-token`, `unexpected-token` exist;
`MalformedUuidLiteral`/`TimestampOutsideI128` follow the `IntegerOutsideI128`
`ill_typed`/`type-mismatch` precedent). The String escape rules describe the
JSON lexer truthfully (regex excludes raw controls, serde_json rejects bad
escapes and lone surrogates). FR-092's `string`/`uuid`/`timestamp` spellings
agree with QSpec FR-322. FR-360 item 5, AC-5 and TC-1007 consistently move the
quoted literal from S2 `UnrepresentedConstruct` to a checker `type-mismatch`.
Every new AC has a TC-1052 row. No ticket ids, roadmap language or quoted QSpec
text in the diff.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `timestamp(007)` is stated to check and equal `timestamp(7)`, but the merged lexer refuses any integer with a leading zero as `invalid_syntax`/`invalid-token` (`LexError::LeadingZero`), and the QSpec shared grammar makes leading zeros a syntax error. The FR-091 Timestamp paragraph and FR-091-AC-41 assert an unreachable outcome. Fix: drop the `007` example, or state that `timestamp(007)` refuses `invalid_syntax`/`invalid-token` and add it to FR-091-AC-42. | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:806, :950; qsl-cst/src/token.rs:20-26; qsl-cst/src/parser.rs:637-640 |
| FND-002 | high | The new `uuid(...)` and `timestamp(...)` productions and the reservation of `uuid`/`timestamp` are not in the complete-V1 grammar, which FR-091 itself names as the authority for the syntax it maps (QSpec's shared grammar, transcribed by `qsl-cst/src/grammar.rs`; its `primary` and `exact-number` hold only `rational`/`decimal`). The diff states the syntax as QSL's without naming the out-of-repo grammar change it needs, so FR-091 contradicts its own authority paragraph. Fix: either state the QSpec shared-grammar change as an explicit upstream dependency of this section, or use the type-named form `UUID("...")`/`Timestamp(n)`, which the existing qualified-call `primary` already parses and check can classify, with no grammar change and no new reserved word. | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:129-131, :816-826; qsl-cst/src/grammar.rs:705-725 |

Recorded, not blocking:

- medium: FR-056-AC-35 now lists literals as operand sources, but its only row TC-1028 says "TC-1052 covers the literal forms", so the literal clause of AC-35 has no covering row (AC-40/TC-1052 cover it in substance; drop "or literals" from AC-35 or name TC-1052 on it).
- medium: FR-056-AC-40 says the functions replay through the real spine with settlement in each row, but TC-1052 step 6 replays only `s`/`ns`; the UUID, Timestamp, call, `let` and field-read rows are only "checked and evaluated" in step 4.
- low: FR-016 says text literals take their type from constraints; the new rule gives a quoted literal no expected type. Scopes differ (native clause checker vs unit language) but the two read as conflicting.
- low: FR-092's "QSpec FR-322 ... fixes these three spellings;" line is indented as a continuation of the `timestamp` bullet.
- low: TC-1052 declares `verifies` FR-098, but no FR-098 AC names TC-1052.
- low: whether `uuid`/`timestamp` are reserved in the complete-V1 vocabulary only (like `decimal`) or also in base/historical editions (like `rational`) is unstated.

## New findings (disposition pass 1)

Reviewed at agent-ix/quire-spec-language@8a3303bd064d9691510514f52a7f00952a145eef.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | high | FR-091-AC-43 expects `Timestamp(-170141183460469231731687303715884105729)` to refuse `IntegerOutsideI128` with limit `i128::MIN`. S2 folds only the directly negated 2^127 (`qsl-forms/src/value.rs:1236-1243`); any other `-n` is `Negate` over the positive literal, so check refuses the literal `170141183460469231731687303715884105729` with limit `i128::MAX` (`qsl-semantics/src/check/type_form.rs:90-97`), as merged FR-091-AC-38 states for the same spelling. The FR-091 Timestamp paragraph defers to that refusal. Fix: expect limit `i128::MAX` for both rows (value as the positive literal), or specify a new fold. | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:806-810, :951, :955 |

Recorded, not blocking (pass 1):

- medium: the "Name resolution" paragraph says `UUID`/`Timestamp` calls resolve "through the same name resolution as any call" where "exactly one candidate is required". Today's `Application::resolve` (`qsl-semantics/src/check/family.rs:869-930`) is precedence: a declared function wins, then a type name, and a native type head refuses `operator-ineligible`. The `function UUID` ambiguity is new behavior. The AC states it explicitly, so an implementer has the target, but the paragraph presents it as existing.
- low: FR-091-AC-42 places the `Timestamp(1, 2)` arity refusal "at the argument". Today's arity mismatch is located at the call.

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 8a3303bd064d9691510514f52a7f00952a145eef |
| FND-002 | fixed | 8a3303bd064d9691510514f52a7f00952a145eef |
| FND-003 | fixed | ca79032ef291dcbb71d52e650f955c73cd7f4f08 |
