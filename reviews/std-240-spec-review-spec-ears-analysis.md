---
id: SR-14091
title: "EARS review of STD-240 FR-222 and FR-229 statements (PR #1195, QSpec PR #334)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@f53be97e54425d2f247a90925a66fb4b5e3bc0a5; spec/functional/FR-222-explore-the-release-acquire-memory-model.md, spec/functional/FR-229-carry-the-memory-component-through-the-protocol-system.md; agent-ix/quire-specification@40cc2f8b08f639c790fba9791b155a3810d59b40; spec/functional/memory/FR-435-classify-memory-accesses-and-orderings.md, spec/functional/memory/FR-437-explore-release-acquire-memory.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-222
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-229
    type: reviews
---
# SR-14091: EARS review of STD-240 (PR #1195, QSpec PR #334)

## Summary

Ticket: STD-240. The changed requirement statements are the new FR-222
bullet ("`Ra` SHALL remove a message once it is earlier in mo than the
message every running thread's current view points at, and SHALL NOT let …
keep one; a view pointing at a removed message SHALL read as pointing at its
location's earliest live message") and the new FR-229 bullet ("Under `ra`,
the value the model gives the acting thread at a shared location SHALL be …
A store's precondition, and a local access's, SHALL therefore read the
current-view message"). The QSpec FR-435 and FR-437 paragraphs are prose
with no SHALL, as QSpec writes them. `make check-spec-validation` reports no
EARS warning on either file.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The FR-222 bullet joins three obligations (remove, do not keep, read as). Its "than the message every running thread's current view points at" reads as one message common to all threads; the intended text is "the earliest message any running thread's current view points at", as RA-7 says. | spec/functional/FR-222-explore-the-release-acquire-memory-model.md:59 |
| FND-002 | low | The FR-229 bullet has two SHALLs, and the second ("SHALL therefore read") states a consequence of the first as a separate obligation. | spec/functional/FR-229-carry-the-memory-component-through-the-protocol-system.md:77 |

## Verdict

Minor. Both statements are ubiquitous with the subject `Ra` or "the value
the model gives". Splitting each bullet into one SHALL per obligation, and
writing "the earliest message any running thread's current view points at",
would remove the ambiguity.
