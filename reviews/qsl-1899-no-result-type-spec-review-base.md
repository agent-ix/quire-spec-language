---
id: SR-2694001
title: "base review of FR-576 no-result-type body refusal (PR 2694)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; PR #2694 diff origin/main...HEAD: spec/functional/FR-576-emit-the-operation-body-into-checked-packages-for-backend-consumers.md, spec/test-cases/TC-1011-unit-compile-checks-body-arity-and-type-against-the-operation.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-576
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1011
    type: reviews
---

## Summary

Ticket: QSL-1899. Base checklist over the PR's one commit. FR-576 gains a
Behavior bullet and AC-10, and TC-1011 gains step 5 and a `verifies` edge to
FR-576. The claim under review is that an `operation-body` on an operation
with no declared result type refuses the compile `ill_typed`/`type-mismatch`
at the body, with no package and no withheld or partial `operation_bodies`.

Read against FR-094, FR-102, FR-103, FR-302, FR-494, FR-576 AC-1 to AC-9 and
QSpec FR-322-AC-397 on quire-specification `origin/main`. The code paths
those requirements describe were also read
(`qsl-semantics/src/check/dispatch_link.rs`, `checked_dispatch.rs`). Review
set `subset`: base and ears-conformance. The change is a new requirement
statement plus one AC, so the other lenses do not apply. No AssuranceProfile
exists under `spec/`. `quire validate` (quire 0.44.0) on both files reports
no errors.

The refusal itself is sound and is the only behaviour QSpec allows. QSpec
FR-322's carrier says a producer emits a row "for every operation member,
built and checked, or refuses the compile". Its reader refuses `no_body` for
an authored body. So refusing is the only conformant producer outcome for a
body that has no body function. The cited code and cause exist:
`IllTypedCause::TypeMismatch` is already the FR-302 body refusal
(dispatch_link.rs:513), and QSpec uses `ill_typed`/`type-mismatch`. FR-102
and FR-103 do not conflict: the form is still built, and `has_body` is still
set independently of the check. FR-494 does not conflict either, because a
resultless operation is never a dispatch candidate. The defects are in how
the new rule sits against the existing `has_body` rule and FR-094, and in
AC-10's verifiability.

No hash, digest, pin or tracking-record antipattern.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-576 Behavior now has two overlapping state-driven rules with incompatible outcomes. Lines 83-88 say that while `has_body` is true, `check` SHALL build the FR-094 body function and the emitter SHALL set the row's `body` to reference it. Lines 92-99 say that for a resultless operation with a body there is no body function and the compile refuses. A resultless operation with a body has `has_body` true (FR-103 sets it independently of the check), so both rules apply. Lines 89-91 also say "the body check has no other new refusal", but the new bullet adds one. Before this change, an uncalled body on a resultless operation compiled: `checked_dispatch` builds bodies only for query operations (checked_dispatch.rs:1002). Fix: scope the `has_body` bullet to operations that declare a result, and name this refusal next to the `operator-ineligible` one at line 89-91. | spec/functional/FR-576-emit-the-operation-body-into-checked-packages-for-backend-consumers.md:83-99 |
| FND-002 | high | The new bullet's rationale "FR-094 builds a body function only against a declared result" is not in FR-094, and FR-094 says the opposite. FR-094 lines 267-268 say `check` "also builds the body function ... for every operation with `has_body` that is not dispatch-eligible (FR-576)". A resultless operation is not dispatch-eligible, because FR-151's eligibility needs a present result. So FR-094 as written requires a body function for exactly the operation FR-576 now says has none. Fix: amend FR-094:267-268 to "every operation with `has_body` that declares a result and is not dispatch-eligible", in this PR. | spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md:267-268; spec/functional/FR-576-emit-the-operation-body-into-checked-packages-for-backend-consumers.md:95-96 |
| FND-003 | medium | AC-10's "with ... a dispatched call reaching it" case cannot produce only the stated refusal. QSpec FR-151 allows dispatched calls only to query operations, which need a present result. QSL refuses a call to a non-query target `ill_typed`/`operator-ineligible` at the call (checked_dispatch.rs:1002-1003), and a resultless call also cannot type a Boolean `pre` clause. So the with-call compile carries at least one more refusal at the call site, while AC-10 and TC-1011's Expected Results name only `type-mismatch` at the body. Two implementers would assert different refusal sets. Fix: either drop the with-call variant, or state both refusals and their locations. | spec/functional/FR-576-emit-the-operation-body-into-checked-packages-for-backend-consumers.md:127; spec/test-cases/TC-1011-unit-compile-checks-body-arity-and-type-against-the-operation.md:30-38 |
| FND-004 | medium | TC-1011 step 5 does not give its starting unit. After step 4 the unit still holds the `tooMany` or `wrongType` body for `A.add`, so both step-5 compiles would also refuse for `A.add` and could not isolate the `A.touch` refusal. The step also does not say where the dispatched call to `A.touch` goes (a `pre` clause, as in step 1?). Fix: start step 5 from the `ok` body of `A.add`, and name the clause that makes the call. | spec/test-cases/TC-1011-unit-compile-checks-body-arity-and-type-against-the-operation.md:30-32 |
| FND-005 | medium | AC-10's second and third clauses have no procedure in the TC its Verification cell names. The clauses are: the body removed emits a `no_body` row for `A.touch`; a declared result with a matching body emits a row referencing the body function. AC-10 says `Test (TC-1011)`, but TC-1011 has no step that removes the body or declares a result for `A.touch`, and its Expected Results never mention `operation_bodies`. Fix: add those two compiles and their `operation_bodies` rows to TC-1011, or move them to the TC that covers FR-576-AC-3 and AC-1. | spec/functional/FR-576-emit-the-operation-body-into-checked-packages-for-backend-consumers.md:127; spec/test-cases/TC-1011-unit-compile-checks-body-arity-and-type-against-the-operation.md:34-38 |
| FND-006 | low | FR-576 line 94-95 attributes the rule to FR-302 ("as for a body whose type differs from the declared result (FR-302)"). FR-302 owns the body check, but it (lines 89-96, AC-8) and FR-103 (lines 116-121) state the check only "against the operation's declared result" and are silent on an operation with none. A reader of FR-302 alone does not learn this refusal. Consider adding the resultless case to FR-302's body-check paragraph. | spec/functional/FR-302-link-dispatch-during-unit-compile.md:89-96; spec/functional/FR-103-admit-model-operations-and-frames-on-the-spine.md:116-121 |
| FND-007 | low | AC-10's clause "so `operation_bodies` is never withheld or partial for it" follows from "emits no package" and cannot be observed separately, because there is no member to inspect. It is a redundant clause and can be removed. | spec/functional/FR-576-emit-the-operation-body-into-checked-packages-for-backend-consumers.md:127 |
| FND-008 | low | TC-1011's title and Description (lines 9, 15-17) still describe only a parameter-count or result-type mismatch. Now that the TC also verifies FR-576-AC-10, the Description should name the case of an operation that declares no result. | spec/test-cases/TC-1011-unit-compile-checks-body-arity-and-type-against-the-operation.md:9-17 |

## Verdict

FAIL. The rule is right and is the only behaviour QSpec FR-322-AC-397 allows.
The cited diagnostic exists and is the established FR-302 body refusal.
FR-102, FR-103 and FR-494 are not contradicted. However, FND-001 and FND-002
leave FR-576's own `has_body` rule and FR-094 requiring a body function that
the new bullet says does not exist. AC-10 is not fully verifiable through
TC-1011 (FND-003 to FND-005). Fix FND-001 to FND-005 in this PR.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | low | The Description and Dependencies prose of FR-576 still say FR-094 builds a body function "for every operation with `has_body`", and that this requirement extends FR-094 "to every operation with `has_body`". Both statements contradict the amended FR-094:267-270 and FR-576's own Behavior bullet at lines 93-100, which give a resultless operation with a body no body function. Fix: say "every operation with `has_body` that declares a result type". | spec/functional/FR-576-emit-the-operation-body-into-checked-packages-for-backend-consumers.md:35, 44-45, 133-134 |
| FND-010 | low | TC-1011's Description says that all three cases refuse "whether or not a dispatched call reaches its operation". The resultless case is now tested uncalled only, and a call to it cannot be made at all: FR-151 allows calls only to query operations. Fix: limit the "whether or not" clause to the two mismatch cases. | spec/test-cases/TC-1011-unit-compile-checks-body-arity-and-type-against-the-operation.md:15-18 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | e74a9feb0: the two `has_body`-true bullets are now split by whether the operation declares a result (FR-576:83-100), and line 91-92 names the resultless refusal as the body check's only other new refusal. |
| FND-002 | fixed | e74a9feb0: FR-094:267-270 builds the body function only for an operation that declares a result type, and states that a resultless operation with a body has none and refuses at the body (FR-576). |
| FND-003 | fixed | e74a9feb0: AC-10 and TC-1011 step 5 drop the with-call variant. Step 5 now declares no clause calling `touch`. |
| FND-004 | still-open | Step 5 still starts from the step-4 unit, whose `A.add` holds the `wrongType` (or `tooMany`) body, so the step-5 compile also refuses at `A.add`'s body and the `A.touch` refusal is not isolated. Only step 6 says "Start from the `ok` body". Move that phrase into step 5. The call-placement half of the finding is resolved. |
| FND-005 | fixed | e74a9feb0: TC-1011 steps 6 and 7, with their Expected Results, cover the `no_body` row and the referencing row for `A.touch`. |
| FND-006 | fixed | e74a9feb0: FR-302:95-96 and FR-103:120-121 now state that a body on an operation that declares no result refuses `ill_typed`/`type-mismatch` (FR-576). |
| FND-007 | fixed | e74a9feb0: the "never withheld or partial" clause is removed from AC-10. |
| FND-008 | fixed | e74a9feb0: TC-1011's Description names the case of an operation with no result type. The title still reads correctly, because it covers checking a body's type against its operation. |
