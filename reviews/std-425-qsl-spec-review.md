---
id: SR-16251
title: "spec-review of the STD-425 status-wording cleanup (quire-spec-language PR 1363)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@8957558a; 22 files under docs/ and spec/ (ADR-011/013/014/029/030, FR-085/086/100/218/259/288/312, TC-125/239/245/255/429/521/539/906)"
review_set: subset
---

# Spec review of the STD-425 status-wording cleanup (quire-spec-language PR 1363)

## Summary

Ticket: STD-425. Review of `git diff origin/main...8957558a`. Every removed clause was read for behaviour. No behaviour, AC or table row was removed; removed text is status, ticket pointers and "remaining work" lines. No deferral wording is introduced. `make check-spec-validation` with QSPEC_DIR set exits 0 with no errors. origin/main is an ancestor of the head.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | TC-239 now ends a sentence mid-clause. The edit removed "but until an implementation PR adds a test ... known gap. Remaining work: #120." and left `(the result/effect checks are unconditional, outside the arity if/else), ` with a trailing comma and space, so the paragraph is ungrammatical and Part B's grounding sentence has no predicate. Fix: end the sentence with a period after the parenthesis. | spec/test-cases/TC-239-arity-mismatch-carves-out-parameter-axes.md:28-30 | wrong-requirement |
| FND-002 | low | FR-085 and FR-086 rewrites change statements about the code into unconditional present-tense claims. FR-085 now says the classify loop "refuses only the missing end" and "does not report the legitimately resolving end"; the removed text said the code reports both ends UnknownEndpoint and that FR-085-AC-1 does not ship. FR-086 now says the Part-signature field is "not part of the model". Both are acceptable as behaviour of the requirement, but they read as a claim about the shipped code in a bullet that cites a source path and function. Fix: word the FR-085 bullet as the requirement's behaviour of the relationship-classification step, without "is the only place ... resolved" as an observation about current code, or confirm that #120 has landed. | spec/functional/FR-085-resolve-relationship-end-references.md:102-111; spec/functional/FR-086-bind-systems-model-references.md:174-180 | wrong-requirement |

## Verdict

Not mergeable until FND-001 is fixed; FND-002 is a judgement call for the author. No scope-deferral and no removed behaviour found.
