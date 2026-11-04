---
id: SR-2298001
title: 'Spec review of PR #2298: decimal literal limits live in the assembly limits (follow-up to #2293)'
type: SpecReview
analysis: base
scope: 'agent-ix/quire-spec-language@f8bda279a; git diff origin/main...HEAD (PR #2298, branch qsl-spec-b14b-limits-home); spec/functional FR-093 FR-096 FR-255; spec/non-functional NFR-011; behavior source PR #2277 head 0b062ccd and QSpec FR-140'
review_set: subset
---

## Summary

Ticket: QSL-1619. Confined review of four files, one commit at head f8bda279a. Verified against #2277 head 0b062ccd. The settings home matches the code: `AssemblyLimits.decimal_literal: DecimalLiteralLimits { scale, total }` (assemble.rs), its `SettingLimits` impl maps `Setting::S3DecimalLiteralScale` and `Setting::S3DecimalLiteralScaleTotal` (qsl-foundation setting.rs, both "S3", WorkBudget), `PackageDeclarations.decimal_literal_limits` is set from `limits.decimal_literal` at assembly and passed to the checker's scope (`with_decimal_literal_limits`), and `Typer::decimal_literal` applies the per-literal limit, then the per-package running total, then membership. The FR-255 rows follow the neighbouring `identity.input_bytes` row's form. No `CheckingLimits` mention of the two limits remains in FR-093, FR-096, FR-255, NFR-011 or TC-1232 except NFR-011's deliberate "rather than held in `CheckingLimits`". The AC-57 `-500, scale 2` clause matches the #2277 test (`a_negated_literal_is_a_member_when_its_operand_is`, assert `(int(-500), 2)`) and QSpec FR-140 (a computed result is retained as (v x 10^T, T) at the target scale T = smax; unary negation is a computed result). AC-39's negate remark ("lowering-only; the evaluated value is AC-57's") is accurate and removes an overlap. `quire validate --scope .` exit 0 for all four files; no ticket id in any added line.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-255 rows s3.decimal_literal_scale and s3.decimal_literal_scale_total | spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:100 | examined |
| FR-096 CheckingLimits sentence and S3 Typer locus row | spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md:190 | examined |
| NFR-011 settings sentence | spec/non-functional/NFR-011-bound-value-checking-work.md:37 | examined |
| FR-093-AC-39 | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md | examined |
| FR-093-AC-57 | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | NFR-011 still says "The ceilings a check ran under are recorded with its result (`CheckedGraph::effective_limits`, `CheckedExpression::effective_limits`)", which returns `CheckingLimits`; the two decimal literal limits are no longer in it (the #2277 test compares `effective_limits()` with `CheckingLimits` only), so the sentence is now false for them. They are recorded on `PackageDeclarations.decimal_literal_limits`. | spec/non-functional/NFR-011-bound-value-checking-work.md:43 |

## Verdict

MERGEABLE. One low, non-blocking finding (add "except the two decimal literal limits, held on the package declarations" to the recording sentence). QSpec has no negation vector in TC-185; the -500 claim rests on FR-140's prose rule for computed results, which the code follows; a TC-185 negation vector would pin it (QSpec owner's call, not required here).
