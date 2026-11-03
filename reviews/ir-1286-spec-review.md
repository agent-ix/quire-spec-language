---
id: SR-2177003
title: "Spec review - FR-091 String paragraph and FR-091-AC-69, IR-1286, PR 2177"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md; spec/test-cases/TC-1052-native-string-uuid-and-timestamp-literals.md; spec/functional/FR-310-build-the-s2-form-of-a-refinement-declaration.md"
review_set: subset
---

## Summary

Ticket: IR-1286. PR 2177 head 78b1556bb. The spec diff is one added sentence in the String paragraph and one extended clause in FR-091-AC-69 (single file). `quire validate --scope .` on FR-091: exit 0 (only the module duplicate-archetype warnings that main also prints). `quire matrix` lists FR-091-AC-69 tagged.
Accurate: the sentence matches the code (S2 holds decoded text plus quoted spelling; check, family node encoding and lowering read decoded text) and QSpec TC-186 T06b / FR-141 (source spelling stays provenance). It does not violate ADR-011 FB-01 as quoted in FR-091 ("no stage after S1 reads token text"): the spelling is read in S2 at the same place as the decode, which already read the token; checks read the form's field, not a token. FR-310 already declares `HistoryLiteralForm::Text(StringLiteral)`, so the new type matches it.
Not minimal or not single-claim: see findings.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | medium | FR-091-AC-69 was already a compound AC (S2 shape, type resolution, five refusals, the ambiguity refusal, the bounds cause) and the PR adds a ninth claim about payload bytes and provenance; the provenance behaviour is independent of the others and should be its own AC so one failure names one claim | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:1171 |
| FND-002 | medium | TC-1052 is named as the verifying test case but was not edited: its Description, Test Procedure step 8 and Expected Results step 8 do not mention raw versus escaped `Text("...")` or provenance, so the TC row is inconsistent with the AC it claims to verify | spec/test-cases/TC-1052-native-string-uuid-and-timestamp-literals.md:49-53 |
| FND-003 | low | The paragraph says the spelling "takes no part in the value's equality, node identity or lowering" without saying which equality; the form types derive structural equality that includes the spelling, so a reader cannot tell whether form equality is covered, and no AC states the identity claim | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:895-898 |
| FND-004 | low | FR-310-AC-4 (history row builds a `Text` literal form) does not state that the form now keeps the quoted spelling, though the type and the code carry it | spec/functional/FR-310-build-the-s2-form-of-a-refinement-declaration.md:138 |
