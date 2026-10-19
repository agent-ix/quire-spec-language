---
id: SR-12899
title: "Base review of PR #991: FR-393 and TC-1078 time language S1/S2 forms"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@104da0711; spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md; spec/test-cases/TC-1078-s2-builds-time-declaration-quantity-and-timed-interval-forms.md; spec/spec.md FR-393 row and edge; agent-ix/quire-specification@ad2b37e proposals/quire-v1/shared-grammar.md lines 25-90, 123-149, 360-368, 775-894, 1135-1162"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-393
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1078
    type: reviews
---

## Summary

Ticket: QSL-580. PR quire-spec-language#991 (renumbered FR-392/TC-1077 to
FR-393/TC-1078). This base review checks FR-393's forms field by field
against QSpec's shared-grammar productions `time-decl`, `time-invariant`,
`clock-bound`, `urgent-decl`, `type-ref =/ 'Clock'`, `exact-value`,
`quantity-literal` and `timed-interval`, and checks the oracle strength of
AC-1 to AC-4 and TC-1078. Cross-document consistency (FR-325, the lexer
vocabulary, facet selection) is in the integrity review,
26-10-19-qsl-580-spec-review-integrity.md.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | AC-1's fixtures do not parse under the productions FR-393 cites, so a correct S1 fails AC-1. (a) `urgent timeout;`: `urgent-decl = 'urgent', ident, ( 'when', block \| qualified-name ), ';'` needs a name and a target, and `timeout` is a reserved word (QSpec common list; qsl-cst/src/token.rs `Timeout`), so it cannot be the `ident`. This also contradicts FR-393's own `UrgentForm{name, target}`. (b) `time tick send period 1/2 ms unit ms;`: `time-decl = 'time', ident, ('dense' \| 'tick', qualified-name, 'period', quantity-literal), 'unit', ...` has no model name here (`tick` is taken as the ident, and then `send` is neither `dense` nor `tick`). `send` is reserved, so it cannot start a `qualified-name` (`Call::send` would work). `1/2` is not an `exact-value` (`uint \| decimal-literal \| exact-number`; QSpec says `/` is never a rational literal), so it should be `rational(1,2) ms`. (c) The Use case's `time invariant inflight when self.inflight { ... }` leaves out the braces that `'when', block` needs (`when { self.inflight }`). ADR-026 §11 also has no `urgent` member and writes `self.x <= T`, which is not a quantity-literal, so "§11's Rpc unit, written in the shared grammar's productions" does not pin down a fixture. Result: either the coder writes a non-conforming S1 so AC-1 passes, or AC-1 fails against a correct one. Fix: spell each fixture in the productions, e.g. `urgent u Call::timeout;`, `time Rpc tick Call::send period rational(1,2) ms unit ms;`, and give the invariants names and braced `when` blocks with `T` replaced by a quantity. | spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md:40-41,118; spec/test-cases/TC-1078-s2-builds-time-declaration-quantity-and-timed-interval-forms.md:19,30 |
| FND-002 | medium | The `Clock` type form has no name and AC-1 gives no oracle for it. Outputs say only "the type builder maps `Clock` to a clock type form", and AC-1 asks only for "a `Clock`-typed field". No variant, span or equality is stated, so any type form, including a user-named type `Clock`, passes. `Clock` is one of the four construct families in the title. Fix: name the variant (e.g. `TypeForm::Clock{span}`), say how it differs from a named type spelled `Clock`, and assert it in AC-1. | spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md:89-90,118 |
| FND-003 | medium | QSL-580 does not cite FR-393 or TC-1078. Its Spec section lists "Implements: FR-230..FR-234, FR-236, FR-235 (types)", and its Acceptance says the tests back those FRs "and pass; nothing beyond them". As written, the ticket's acceptance rules out the S1/S2 work this PR specifies, and the matrix row for FR-393 traces to no ticket. Fix: add FR-393 (AC-1..AC-4) and TC-1078 to QSL-580's Spec and Acceptance. | Linear QSL-580 description; spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md:139 |
| FND-004 | low | The `QuantityForm.magnitude` comment lists "uint, decimal-literal or rational" and leaves out the `decimal(signed-int, uint)` alternative of `exact-number`. `ExactValue` is not defined in any QSL artifact (FR-311 only uses it). | spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md:79,128 |
| FND-005 | low | AC-4 and TC-1078 step 4 name fragments (`eventually[0 s, 0.003 s]`, `(2 ms, 2 ms]`, `[3 ms, 2 ms]`) with no operand, and these do not parse as claims. Spell the full `... holds(p)` claims. | spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md:121; spec/test-cases/TC-1078-s2-builds-time-declaration-quantity-and-timed-interval-forms.md:24 |
| FND-006 | low | Mixed bounds are not stated or tested. `[0, 3 ms]`, `[0 ms, 3]` and `[0 ms, *]` match neither production, and no AC gives their S1 diagnostic location. | spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md:100-105,121 |
| FND-007 | low | AC-1 says "every form carries the span of its source text", but `TimeSourceForm`, `TimeInvariantForm`, `UrgentForm` and `QuantityForm` have no whole-form `span` field. Only `ClockBoundForm` and `TimedIntervalForm` do. State which span carrier (`ParsedForm`/`DeclarationForm` wrapper or field) the test reads. | spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md:54-86,118 |
| FND-008 | low | The relationships do not match the text. FR-325 is linked as `traces_to`, but FR-393 changes FR-325's operator form (`depends_on` or an amendment fits better). FR-233 is cited in the Description but has no edge. | spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md:5-22,32 |

## Verdict

The fields of `time-decl` (model, `dense` or `tick` operation + period,
unit), `time-invariant` (name, `when` block, ordered `clock-bound`s with
`<`/`<=`), `urgent-decl` (name, `when` block or qualified-name) and
`timed-interval` (two quantity-literals, each end open or closed) are
present, and their optionality matches. The FR-311 sibling pattern is
followed: thin `LeadingTokenKind` dispatch, one form per production, and
meaning deferred to S3 with FR-230-AC-2 and FR-234-AC-2 cited correctly.
The rename is clean: the only `FR-392` strings in the tree are existing
references to QSpec FR-392. FR-393 and TC-1078 are free on main and in
every open PR (#990 holds FR-392/TC-1077, #992 touches FR-310/TC-889), and
the spec.md index row and edge are present. FND-001 blocks merge, together
with integrity FND-001.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | low | The fix creates a `depends_on` cycle. FR-325 now has `depends_on` FR-393 (for `OperatorInterval`/`TimedIntervalForm`), and FR-393 has `depends_on` FR-325. One of the two edges should be `traces_to`, or carry an amends relation if the schema has one. | spec/functional/FR-325-parse-temporal-operators-with-an-optional-interval.md:18-19; spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md:14-15 |
| FND-010 | low | The new rule "S1 SHALL refuse a unit declaration that would introduce an unqualified type named `Clock`" (and FR-393-AC-7) refuses source that QSpec's shared grammar admits. QSpec adds `type-ref =/ 'Clock'` but does not reserve `Clock` in any list. This is a defensible QSL disambiguation, but it is a grammar divergence QSpec should record (STD follow-up), or a QSpec conformance vector could fail. | spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md:123-127,163 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | AC-1 now uses grammatical fixtures: named invariants with braced `when` blocks, `urgent hurry Call::timeout;`, `urgent stalled when {...};`, and `time Rpc tick Call::send period rational(1,2) ms unit ms;`. The Use case is corrected as well. |
| FND-002 | fixed | `TypeForm::Clock{span}` is named in the Outputs and asserted in AC-1 and AC-7. AC-7 also says `M::Clock` is a named type. |
| FND-003 | still-open | QSL-580 now cites FR-393 and TC-1078, but its Implements line and Acceptance still say "FR-393-AC-1..AC-4 ... nothing beyond them". FR-393 now has AC-1..AC-7, so AC-5..AC-7 are excluded. This is a Linear edit only (change AC-4 to AC-7); the PR needs no change. |
| FND-004 | fixed | `ExactValue` is now defined: an exact rational from `uint`, `decimal-literal` or `exact-number` (`rational(n,d)` or `decimal(n,s)`). |
| FND-005 | fixed | AC-4 and TC-1078 step 4 now write full `eventually... holds(p)` claims. |
| FND-006 | fixed | A new Behavior bullet and AC-5 refuse mixed bounds (`[0, 3 ms]`, `[0 ms, 3]`, `[0 ms, *]`) at S1, at the upper bound's span. TC-1078 step 5 tests this. |
| FND-007 | fixed | Every form in the Outputs carries a whole-form `span`, and AC-1 asserts it. |
| FND-008 | fixed | FR-325 is now `depends_on` (amended) and FR-233 has an edge. The new cycle this creates is FND-009. |
