---
id: SR-2293001
title: 'Spec review of PR #2293: decimal constructors (FR-093 section, FR-104-AC-13, s3.decimal_literal_scale)'
type: SpecReview
analysis: base
scope: 'agent-ix/quire-spec-language@f7725595a; git diff origin/main...HEAD (PR #2293, branch qsl-spec-b14-decimal-constructors); spec/functional FR-091 FR-093 FR-104 FR-255 FR-516; spec/non-functional NFR-011; spec/test-cases TC-1084 TC-1232; behavior source PR #2277 (qsl-1619-decimal-source-forms@530aee5c8) and QSpec FR-140, FR-146, FR-495'
review_set: subset
---

## Summary

Ticket: QSL-1619. Method: spec-review with the EARS, integrity, dependency and criterion-strength sub-analyses run by hand in one pass (Jev criterion-strength client not used; AC strength judged by hand). Head f7725595a, one commit, 8 files. Every behavior claim was checked against the #2277 code at 530aee5c8 and QSpec b840f0985.

Verified against code or QSpec: the AC-38 checking order (no type, then non-decimal type, then scale limit, then membership) matches `decimal_literal`; 65534 and 65535 spell as `0.` plus zeros; the spelling table of AC-39 matches `LiteralValue::decimal`; `a / decimal(4, 0)` and `a / decimal(400, 2)` discharge, a zero coefficient, a parameter, `a / b` and an `if` of two literals do not; FR-104-AC-13 d, d2, e, f, g (the equality peer or the declared result supplies P in the code); FR-495-AC-3 d2 and d4; QSpec FR-146 line 324 ("a Decimal divisor satisfies the nonzero obligation only as a decimal(c,s) literal with c != 0"). By code reading only (no #2277 test): AC-39 `-decimal(5, 0)` lowers to one `quire.op.decimal.negate` node over the literal (check.rs Negate arm, lowering.rs:4015); AC-41 y6 refuses because facts.rs discharges only `NodeKind::Literal` and the negation is `NodeKind::DecimalNegate`. Not pinned by any #2277 test: the exact conversion of 2/3 refuses with no code asserted (`Outcome::Refused(_)`).

Scale-limit design: the vocabulary `stage_limit_exceeded`/`work-budget-exceeded` with limit kind work budget is the same as the existing `s3.decimal_scale` (FR-091 catalog row, FR-091-AC-67) and `s3.work_units`; the name follows FR-255's rule; the default 65535 is within u32; the limit before membership is a stage limit (NFR-011, ADR-030), not a QSpec membership refusal, so it does not contradict QSpec FR-140-AC-5/AC-8 and no TC-185 vector writes a scale above 65535. The decision is sound. The code's interim `ill_typed`/`type-mismatch` (check.rs `MAX_LITERAL_SCALE`, and the rows.rs test named `..._ill_typed_type_mismatch`) must change to it; that is a #2277 follow-up, not a spec defect.

FR-091-AC-58 versus code: at head 530aee5c8 the code matches the spec. `decimal(-0, 2)`, `decimal(-00, 2)` and `decimal(+1, 2)` refuse `invalid_syntax`/`invalid-token` at S1 (qsl-cst parser `decimal_constructor_numbers` and `malformed_decimal_argument`, tests in qsl-cst/tests/it/decimal_constructor.rs). The difference the #2277 gap analysis flagged was at the older head 79c8e075d and is fixed. The spec side is right (FR-091 "Decimal constructor" text, QSpec signed-int). AC-58 is not edited by this PR and needs no change for it; the one related gap (a scale above u32::MAX) is FND-006.

`quire validate --scope .` per changed file: exit 0 for all eight (FR-091, FR-093, FR-104, FR-255, FR-516, NFR-011, TC-1084, TC-1232); the output is the module's inline-data-schema and DuplicateArch warnings. FR-093-AC-38 to AC-42 were free on origin/main (TC-1232 and FR-091 cited AC-39 to AC-42 that did not exist); no other open id is touched. No ticket id (QSL-, STD-, #n) appears in any added line.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-093 "Decimal constructors" | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:321 | examined |
| FR-093-AC-38 to FR-093-AC-42 | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:912 | examined |
| FR-104 infix decimal arithmetic bullet | spec/functional/FR-104-check-state-clauses.md:213 | examined |
| FR-104-AC-13 | spec/functional/FR-104-check-state-clauses.md:398 | examined |
| FR-091 "Decimal constructor", FR-091-AC-57, FR-091-AC-58 | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:1028 | examined |
| FR-255 setting table row | spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:100 | examined |
| NFR-011 limit list, table row, counter bullet | spec/non-functional/NFR-011-bound-value-checking-work.md:37 | examined |
| FR-516-AC-41, FR-516-AC-45, FR-516-AC-46 | spec/functional/FR-516-carry-the-temporal-evidence-request-and-result-on-the-checked-package-route.md:443 | examined |
| TC-1232, TC-1084 | spec/test-cases | examined |
| FR-096 stage-limit locus table | spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md:182 | context_only |
| FR-094 division by a quantity literal | spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md:335 | context_only |
| QSpec FR-140, FR-146, FR-495-AC-3, TC-185 | quire-specification | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-104's new sentence "a quotient is typed whether or not its position supplies an expected type" is contradicted by the #2277 code: `Binary Divide` is `contextual` in check.rs and its test `sums_differences_and_products_of_one_typed_pair_need_no_expected_type` asserts `let s = a / b in s = a` refuses `ill_typed`/`ambiguous-literal`; the spec (QSpec FR-140 "operands and result have one named type") says it is typed and refuses `unproved-nonzero`. | spec/functional/FR-104-check-state-clauses.md:222 |
| FND-002 | medium | FR-104-AC-13 cannot fail on the claim it was rewritten to pin: f has a declared result P and g and e get P from the equality peer, so no example has a quotient with no expected type at all, and e is described as having none. The claim passes in the code that violates it (FND-001). | spec/functional/FR-104-check-state-clauses.md:398 |
| FND-003 | medium | FR-093-AC-42 says the exact conversion of 2/3 "returns a refusal and no value" without a code. QSpec FR-140 makes the Rational-to-Decimal conversion the same rounding step, whose exact refusal is `inexact_decimal`/`nonzero-discarded-digit`; any refusal passes (the #2277 test asserts only `Refused(_)`). | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:916 |
| FND-004 | medium | The default 65535 bounds one literal, not the check's output: a 1 MiB source (`s1.input_bytes`) can hold about 58000 distinct `decimal(0, N)` literals, each a distinct node (a scale is a distinct node), whose spellings sum to the order of a gigabyte (a few hundred MB to 1.7 GB depending on how many literals `s3.nodes` allows), while `s3.nodes` (100000) and the "before any charge" rule bound neither. NFR-011 states "64 KiB per literal" and negative-abuse-testing but no aggregate bound or per-byte charge. | spec/non-functional/NFR-011-bound-value-checking-work.md:82 |
| FND-005 | medium | FR-516-AC-41, AC-45 and AC-46 replace "the caller's ledgers unchanged" with "no result". For `Err(CallFailure::Admission)` there is never a result, so the new wording cannot fail, and the claim it replaced is no longer tested while FR-516's text (line 139) and FR-516-AC-17 still assert it. The edit is outside the decimal-constructor scope and its reason is in no commit message. | spec/functional/FR-516-carry-the-temporal-evidence-request-and-result-on-the-checked-package-route.md:443 |
| FND-006 | medium | FR-091 states S2 "refuses no scale, so decimal(1, 4294967295) builds", but S1 refuses a scale above u32::MAX (`decimal(1, 4294967296)` is `invalid_syntax`/`invalid-token` in the #2277 code and its test tagged FR-091-AC-58). No FR-091 text or AC states it; the sentence this PR edits is the place. | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:1028 |
| FND-007 | low | A negated literal "takes its operand's type", so `-decimal(5, 0)` at `Decimal[-9, 0; 0, 0; ..]` refuses `ill_typed`/`type-mismatch` (5 is not a member) although -5 is; the code does this (operand hint is the expected type). No AC fixes which way it goes; AC-39 uses a symmetric type. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:325 |
| FND-008 | low | The FR-096 stage-limit producer and locus table and its sentence naming the `CheckingLimits` ceilings (node count, declaration input bytes and work) are not extended with the new S3 literal-scale stop; FR-093 gives its locus ("at the literal") but FR-096 says each producer's locus is "the locus this FR gives its row". | spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md:182 |
| FND-009 | low | EARS: the new FR-093 "Decimal constructors" section has no SHALL (declarative "check types it", "refuses", "lowers"), three compound rules in one numbered list, and the added FR-104 sentences ("A quotient is typed ...", "Its divisor carries the nonzero obligation ...") state facts inside a SHALL bullet without their own obligation. Sibling FR-093 sections use SHALL. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:321 |
| FND-010 | low | Dependency: FR-093 cites the new S3 setting and QSpec FR-140 in its new section but declares no relationship to NFR-011, FR-255 or quire-specification FR-140/FR-146, and the section links none of them; FR-104 declares no edge to FR-093 for the nonzero rule it now delegates. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:321 |
| FND-011 | low | TC-1232 drops, with no replacement, the float-to-decimal conversion refusal, the "convert of a constructor builds no node" check, the oversize-coefficient constructor and the lowering of conversions to the listed operations; FR-093-AC-42 checks and evaluates only. These had no AC (AC-39 to AC-42 did not exist) but are behavior the code implements. | spec/test-cases/TC-1232-a-decimal-constructor-in-source-checks-lowers-and-evaluates.md:34 |
| FND-012 | low | Redundancy and weak form: FR-093-AC-40 a1 repeats FR-104-AC-16 e1 verbatim, and AC-38 (eight claims) and AC-41 (nine functions) are multi-claim rows whose single failure hides the others. NFR-011's rationale "65535 is the largest width a Rust format argument takes" is an implementation artifact; the #2277 code now spells without a format width. | spec/non-functional/NFR-011-bound-value-checking-work.md:82 |

## Verdict

Mergeable after FND-001 to FND-006 are addressed or explicitly accepted; none blocks the decision to use `stage_limit_exceeded`/`work-budget-exceeded` with `s3.decimal_literal_scale` (sound, consistent with FR-255, `s3.decimal_scale` and NFR-011). FND-001 and FND-002 are one decision: keep the FR-104 sentence (QSpec supports it) and add an example with no expected type, such as `let s = a / b in s = a` refusing `undefined_expression`/`unproved-nonzero`; the #2277 code and its test then change. FND-003 needs the code named. FND-004 needs an aggregate bound or an explicit acceptance. FND-005 should be dropped from this PR or reasoned (and AC-17 and line 139 aligned). FND-006 needs one sentence and an AC-58 entry. The rest are low.

Code follow-ups for #2277 (not spec findings): the interim scale refusal must become `stage_limit_exceeded`/`work-budget-exceeded` naming the setting and become a `CheckingLimits` field; the quotient typing in FND-001; facts.rs keeps an interval-excludes-zero arm for a Decimal divisor that QSpec FR-146 line 324 ("only as a decimal(c,s) literal with c != 0") and the new FR-093/FR-104 text do not allow; QSpec FR-146 admits a negated nonzero quantity literal but says nothing for a negated decimal literal, so AC-41 y6 refusing is consistent with the text but asymmetric (QSpec owner's call). Tests for AC-39 `-decimal(5, 0)` (negate node over the literal), AC-41 y6, AC-42 f and the 2/3 `exact` refusal code are absent in #2277.
