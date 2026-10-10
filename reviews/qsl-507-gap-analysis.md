---
id: SR-2450
title: "QSL-507 gap analysis of PR #669 (FR-325-AC-1 to AC-4, TC-835, QSpec FR-362 fairness grammar)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language app version 0.2.0; PR #669; gap-analysis; review pass and disposition passes 1 to 2; spec/functional/FR-325-parse-temporal-operators-with-an-optional-interval.md; spec/test-cases/TC-835-s2-parses-temporal-operators-with-an-optional-interval.md; context: spec/functional/FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md, spec/functional/FR-257-build-forms-at-any-depth.md, QSpec spec/functional/temporal/FR-362-apply-fairness-constraints.md (qspec-main-ro)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-325
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-835
    type: reviews
---
# QSL-507 gap analysis of PR #669

## Summary

Ticket: QSL-507. Planless. Plan completion: not assessed. Computed matrix:
`quoin matrix --json` (quoin 0.28.3, quire 0.36.2, engine 0.50.2) run on the
PR head.

- **FR-325-AC-1**: tagged.
  `an_unbounded_operator_over_a_bounded_one_keeps_each_operators_interval`
  (qsl-forms/tests/it/temporal_clause_forms.rs:114).
- **FR-325-AC-2**: tagged.
  `binary_and_unary_operators_build_none_closed_open_and_inverted_intervals`
  (:164) and `every_temporal_operator_takes_each_interval_shape_and_nests_freely`
  (:209).
- **FR-325-AC-3**: tagged. `a_bound_that_is_not_a_u64_fails_at_s1_at_the_bound`
  (:273).
- **FR-325-AC-4**: tagged.
  `the_clause_forms_are_the_same_under_every_profile_selection` (:309).

All four tests carry `#[trace("TC-835", "FR-325-AC-n")]`, as TC-835 asks.
There is no run evidence, because the reviewer ran no builds. The author
reports these tests passing, and the lead runs `make ci`.

**Each AC against its test:**

- **AC-1**: asserts `None` on the outer `always` and on `eventually`, and
  `Some{0, Finite(10)}` with the `[0,10]` span on the inner `always`. It
  also checks each operator's own span and operator span against the source
  text. Strong oracle.
- **AC-2**: asserts all four interval values from the AC, the `[a,b]` and
  `[a,*]` spans, and that the inverted `[5,3]` builds as written.
  `is_admissible()` is `diagnostics.is_empty() && recoveries.is_empty()`, so
  "no S1 diagnostic" is asserted too. The second test covers every one of
  the 8 operators with each of the 3 interval shapes, plus the nesting rule.
- **AC-3**: asserts that the first diagnostic's byte span is exactly `x`, or
  exactly the 20-digit too-large bound, and that `build_unit` refuses. Plus
  `u64::MAX` is accepted.
- **AC-4**: see SR-2449's verdict. The oracle holds for what S2 can see, and
  the AC's wording is the weak point (FND-003).

**Code with no owning requirement.**

- The S3 `Temporal` arm (qsl-semantics) is owned by no AC. Its behaviour
  change is SR-2449 FND-001 and is not repeated here.
- Fairness forms are owned by FR-325 "Outputs", but no AC covers them
  (FND-002).
- The capture refusal is owned by no FR (FND-001).

No stubs. No coverage inflation: each of the five tests asserts concrete
values.

## Verdict

FR-325's four ACs are each backed by a real, value-asserting test, and
nothing beyond FR-325's statement is built in S1/S2. The gaps are in the
spec. FR-325 leaves captures without an owner. Fairness forms are in its
"Outputs" but no AC covers them. AC-4 is worded so that it cannot be met
literally. The silent S3 acceptance is recorded in SR-2449 FND-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-325 "Outputs" lists the `TemporalClauseForm` fields (name, profile, model references, `over`, activation, fairness, formula) but no captures. QSpec FR-362's grammar (`{ fairness }, { capture }, temporal-expr`) and qsl-cst both accept `capture`. No QSL FR owns the S2 form of a capture, so the PR refuses it with `UnrepresentedConstruct`. Fix in spec: add captures to FR-325 "Outputs" with an AC, or name the FR or ticket that owns them and state the interim refusal. | spec/functional/FR-325-parse-temporal-operators-with-an-optional-interval.md, qsl-forms/src/temporal_clause.rs:34-35 |
| FND-002 | low | FR-325 "Outputs" requires "its fairness constraint forms (FR-123)", and the PR builds them: an unwritten kind becomes `Weak`, an unwritten granularity becomes `None`, and each form keeps its operation name and span. FR-325-AC-1 to AC-4 test none of this, so the fairness surface has no AC of its own. Only the AC-4 test asserts kind, granularity and operation, as a side effect. Fix in spec: add an FR-325 AC for the fairness form shape, matching FR-123 "Inputs" (kind, operation name, optional granularity). | spec/functional/FR-325-parse-temporal-operators-with-an-optional-interval.md |
| FND-003 | low | FR-325-AC-4 asks for "the same unit text compiled with the infinite-trace profile selected and with the event-position false-extension profile selected" to give "byte-equal S2 forms". In complete V1 the profile selection is part of the unit text, so one text cannot select two profiles. S2 forms also have no byte encoding. The test reads the AC as two units that differ only in the profile string, padded to equal length, compared by `PartialEq` and `Debug`. That is reasonable, but it is an interpretation. Fix in spec: say "two units that differ only in the profile selection, with the clause at the same offset" and "equal forms". | spec/functional/FR-325-parse-temporal-operators-with-an-optional-interval.md, qsl-forms/tests/it/temporal_clause_forms.rs:306-318 |

## New findings (disposition pass 1)

Disposition pass 1, agent-ix/quire-spec-language app version 0.2.0, PR #669. Computed matrix: `quoin matrix --json` (quoin 0.28.3, quire 0.36.2, engine 0.50.2) for this pass. FR-325-AC-1 to AC-6 are each bound to exactly one value-asserting TC-835 test, except AC-2, which has two.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | The new S3 refusal test `each_temporal_clause_refuses_as_not_yet_implemented` is tagged `#[trace("FR-091-AC-8", "TC-396")]`. FR-091-AC-8 covers `deref(x)`, `allInstances<M::T>(x)` and `pre(x)` refusing at check with their own family causes, and it says "None of these refusals has code `unsupported_construct`". The temporal refusal is `unsupported_construct`/`not-yet-implemented`, so the test asserts the opposite of the AC it claims. The computed matrix now lists it as a third FR-091-AC-8 binder, which inflates that AC's coverage. No AC states the interim temporal refusal. FR-113 states its own interim `not-yet-implemented` refusal (FR-113:165), and that is the precedent. Fix: state the interim refusal in FR-325, or in the FR that QSL-508 lands (FR-326/FR-123), with an AC, and retag the test to it. Or drop the `trace` attribute and leave the test untraced until QSL-508 deletes it. | qsl-semantics/src/check/assemble/tests.rs:306; spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:799 |

## Dispositions

Disposition pass 1, agent-ix/quire-spec-language app version 0.2.0, PR #669.

| FND | Outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Spec, code and test: FR-325 "Outputs" now lists "its capture forms" as `CaptureForm{parameter, value, span}`. "Behavior" adds "S2 SHALL build one capture form per `capture`, in source order", and new FR-325-AC-6 and TC-835 step 6 cover it. S2 builds the form, and `captures_build_their_type_value_and_span_beside_an_unchanged_formula` is the AC-6 binder. |
| FND-002 | fixed | Spec and test: new FR-325-AC-5 states three fairness forms (kind, granularity, operation, span) in source order, and TC-835 step 5 covers it. `fairness_constraints_build_their_kind_granularity_operation_and_span` asserts all four fields for each of the three constraints, with the span sliced from the source. Who resolves an unwritten kind is a separate defect, recorded as SR-2451 FND-001. |
| FND-003 | fixed | FR-325-AC-4 now reads "Two units that differ only in the selected profile identity ... padded so their headers have equal length and the clause starts at the same byte offset, build equal S2 forms for the clause, compared by `PartialEq` and by their `Debug` rendering". TC-835 step 4 matches. The test asserts the equal offset, `PartialEq` and `Debug` equality. |
| FND-004 | still-open | New this round. The test is still traced to FR-091-AC-8, and no AC states the interim temporal refusal. |
| FND-004 | fixed | Disposition pass 2: the `#[trace("FR-091-AC-8", "TC-396")]` attribute is removed from `each_temporal_clause_refuses_as_not_yet_implemented` (qsl-semantics/src/check/assemble/tests.rs:303-307). The test itself is kept unchanged, and still asserts both clauses' causes, spans, `UnsupportedConstruct` and `not-yet-implemented`. The test is now untraced, as the round-1 fix option allowed. It no longer inflates FR-091-AC-8's binders: those are back to the two FR-091-AC-8 tests. The repo has no rule requiring every test to carry a trace, and one other test in that file is untraced. QSL-508 deletes the refusal and this test together. |
