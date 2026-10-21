---
id: SR-13621
title: 'Spec review of PR #1126: protocol refinement rows, EARS conformance'
type: SpecReview
analysis: ears-conformance
scope: 'agent-ix/quire-spec-language@74e87f43e; git diff origin/main...74e87f43e (PR #1126, branch qsl-563-spec, base main 6eceb158b): ADR-015, FR-027, FR-098, FR-099, FR-135, FR-136, FR-146, FR-147, FR-214, FR-310, TC-444, TC-446, TC-540, TC-541, TC-551, TC-659, TC-889; read against origin/main code (qsl-semantics check, qsl-cst grammar, qsl-replay request, src/command/wire.rs) and specs (FR-071, FR-141 to FR-148, ADR-015, ADR-027) and QSpec main 8736706 (FR-322, FR-323, FR-375, FR-432, shared grammar); ticket QSL-563'
review_set: subset
date: '2026-10-21'
relationships:
- target: ix://agent-ix/quire-spec-language/ADR-015
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-027
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-098
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-099
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-135
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-136
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-146
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-147
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-214
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-310
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-444
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-446
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-540
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-541
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-551
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-659
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-889
  type: reviews
---

## Summary

Ticket: QSL-563. PR quire-spec-language#1126, head 74e87f43e. EARS lens over every behaviour statement the diff adds or rewrites in FR-099, FR-098, FR-135, FR-136, FR-146, FR-147, FR-214 and FR-310. The ADR-015 paragraph is decision prose, not a requirement statement.

`make check-spec-validation` reports no grammar diagnostic on any touched file. The SHALL statements follow the ubiquitous, event or unwanted-behaviour pattern with S1, S2, S3, the checker, the compile, the input builder or the executor as subject. The findings below concern statements without SHALL and bundled bullets, not grammar.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-136 sort key | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 StepRowRef | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 select rows | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 one protocol | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 two protocols | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 unknown protocol | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 memory row | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 coverage inputs | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 order models | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 order protocol | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 order refused | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 unused operation | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-146 protocol order | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-147 order | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md | examined |
| FR-135 header | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| FR-135 order | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| FR-214 select | spec/functional/FR-214-require-an-explicit-refinement-row-for-every-protocol-step-class.md | examined |
| FR-310 header | spec/functional/FR-310-build-the-s2-form-of-a-refinement-declaration.md | examined |
| FR-310 syntax refusals | spec/functional/FR-310-build-the-s2-form-of-a-refinement-declaration.md | examined |
| FR-099 carry | spec/functional/FR-099-compile-against-supplied-libraries.md | examined |
| FR-099 no refuse | spec/functional/FR-099-compile-against-supplied-libraries.md | examined |
| FR-099 library defects | spec/functional/FR-099-compile-against-supplied-libraries.md | examined |
| FR-099 own value | spec/functional/FR-099-compile-against-supplied-libraries.md | examined |
| FR-099 replay | spec/functional/FR-099-compile-against-supplied-libraries.md | examined |
| FR-098 read internals | spec/functional/FR-098-execute-a-replay-request.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Several new behaviour bullets state the rule with no SHALL. Examples: 'Over a protocol subject the same rule applies with a sort key ...', 'A protocol subject's `StepRowRef` is the row's zero-based position in that order', FR-146's 'A protocol subject's rows are ordered by FR-136's protocol sort key', and 'S3 classifies a row's source by resolving the segment after `<C>::`'. The model-subject bullets above them use SHALL. Write each as 'S4 SHALL emit ...', 'The checker SHALL give ...' and 'S3 SHALL classify ...', so that each one is a requirement that can be traced. | spec/functional/FR-136-check-a-refinement-s-step-rows.md:85-100; spec/functional/FR-136-check-a-refinement-s-step-rows.md:101-103; spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md:127-131; spec/functional/FR-136-check-a-refinement-s-step-rows.md:107-115 |
| FND-002 | low | Two bullets mix requirements with rationale or facts. The order bullet holds a SHALL, an alternative ('equivalently, it defers ...') and a judgment ('a coverage check that runs before the clause is checked is a defect'). The one-protocol bullet holds two SHALLs and a grammar fact ('Every protocol clause declares exactly one `finish` node, whose row Coverage below requires, ...', with a stray capital on 'Coverage'). Keep one SHALL per bullet, and move the alternative, the judgment and the `finish` fact to prose (the fact can cite the shared grammar's `protocol-clause`). | spec/functional/FR-136-check-a-refinement-s-step-rows.md:147-153; spec/functional/FR-136-check-a-refinement-s-step-rows.md:116-122 |

## Verdict

Mergeable on this lens alone. Two low findings: statements without SHALL, and bundled bullets. Neither changes behaviour.
