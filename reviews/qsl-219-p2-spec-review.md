---
id: SR-1286
title: "Spec review of PR #625: FR-056 and TC-145 amended for numbers with no finite double (QSL-219 part 2)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@25610e30b8e443aa7b665d3880145b1b9c2d858a; PR #625 diff against origin/main: spec/functional/FR-056-admit-domain-package-model-declarations.md, spec/test-cases/TC-145-admit-ir-2-domain-package.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: reviews
---
# Spec review of PR #625

## Summary

Ticket: QSL-219 (part 2). PR: quire-spec-language#625. Integrity and EARS
sub-checks, scoped to the changed FR-056 prose and AC-2, and to TC-145
step 3 and its expected result.

Examined:

- The FR-056 "does not parse" paragraph, with `1e400` removed. Consistent
  with the code.
- The new paragraph on numbers with no finite double. It matches the ruling
  for causes, pointer, and the ban on `stale_dependency`. Its last sentence
  is an honest description of the code's behaviour. But it contradicts the
  normative SHALL two paragraphs below it (FND-001).
- The FR-056 SHALL paragraph (lines 136-153): "the RFC 6901 pointer of the
  first such number in document order". `1e400` is "such a number" (a whole
  value beyond 2^53). In `{"x":[1e-400,1e400]}` the first such number is
  `/x/0`, yet the code and the PR's test name `/x/1`.
- FR-056-AC-2 as amended. It does not say "first". It does not conflict with
  the ordering caveat. Its claims match the tests, except that FR-154
  `admit` is untested (see SR-1285).
- FR-056-AC-13 ("at `/b` and then `/a/0` names `/b`"). It uses only numbers
  that parse, so it does not conflict.
- TC-145 step 3 and its expected result. They match AC-2. They do not
  record the ordering case that the test asserts (folded into FND-001).

## Verdict

Request changes. The new caveat is honest about the behaviour, but it sits
outside the SHALL it overrides. The spec now states two orders for one
refusal. Precedence against other reader faults is also left unstated
(FND-002).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-056's SHALL says the refusal's `document_pointer` is "the RFC 6901 pointer of the first such number in document order". The new paragraph says that a number with no finite double "is the one named when the document also holds an earlier inexact number". The code and test (`{"x":[1e-400,1e400]}` names `/x/1`) follow the new paragraph, so they break the SHALL. Fix: amend the SHALL sentence itself, for example "...of the first such number in document order, except that a number with no finite double is named ahead of any earlier one, since the reader refuses it before any tree exists". Add the ordering case to TC-145's step 3 and its expected result, since the test already asserts it. | spec/functional/FR-056-admit-domain-package-model-declarations.md:108-109, spec/functional/FR-056-admit-domain-package-model-declarations.md:142-144 |
| FND-002 | medium | Precedence between an out-of-range number and the other reader faults is unstated, and it does not follow byte order. The reader checks for duplicate names when an object closes. So `{"a":1,"a":2,"n":1e400}` refuses `noncanonical_wire`, but `{"a":1,"a":2,"n":1e-400}` "does not parse" and is digested raw. Truncated `[1e400` refuses `noncanonical_wire`, not as non-JSON. FR-056 says such a number is "never a parse failure, never digested raw", but it does not say which wins when a document is also unparseable. IR's FR-038-AC-93 parity depends on the answer. Fix: one sentence stating that the reader's first refusal decides, including that a repeated name is found only when its object closes. Or rule the order explicitly. | spec/functional/FR-056-admit-domain-package-model-declarations.md:86-91, spec/functional/FR-056-admit-domain-package-model-declarations.md:100-108 |
