---
id: TC-136
title: "Admit checked-term inputs and preserve typed outcomes"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-049, type: verifies }
  - { target: ix://agent-ix/quire-spec-language/FR-046, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-047, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-056, type: references }
---
# TC-136: Admit checked-term inputs and preserve typed outcomes

## Description

Scope: FR-049-AC-1 to FR-049-AC-6 and FR-049-AC-10 to FR-049-AC-11.

Verify the replacement boundary for the retired native `state` evaluator:
checked expression identity, typed environment and observation admission,
graph identity, authority selection and typed outcomes. The test invokes the
current checked-term/family evaluator and never constructs the retired
`EvaluationRequest`, `StateView` or artifact-value graph.

## Test Procedure

Build a checked package through the supported source pipeline and evaluate an
exact selected expression through its family entry point. Exercise one Boolean,
exact number, text, enum, record, option, ordered sequence, reference and object
value, including nested values. For each kind, mutate its checked type, unit,
bounds or exported member independently; offer a foreign expression handle and
an out-of-range handle. Attempt to enter using raw bytes and freely constructed
wire records without the checked package/session.

For an expression requiring model or observation input, use the checked clause
environment and selected FR-056/observation authority. Supply a duplicate,
surplus, foreign and wrong-type binder separately. For object inputs, offer a
duplicate complete storage key, then the same logical object at distinct
pre/post observation occurrences. Compare identity across observations while
performing a field read under each original full occurrence. With complete
membership omit a referenced object; repeat with membership or closure
unavailable. Mutate the selected static identity and digest one at a time using
FR-056's authority cases rather than defining a local digest encoding.

For each supplied aggregate, make one nested member unavailable. Evaluate once
with a reached missing member and once where ordered short-circuiting skips the
same member. Observe typed results and ensure no fabricated payload is returned.

## Expected Results

Only the exact checked expression enters evaluation; raw bytes, free wire
records, foreign handles and each single-axis type/authority mutation are
rejected at the owning admission boundary. Duplicate full storage keys refuse;
distinct observation occurrences remain distinct storage entries; identity
comparison projects only the occurrence when the checked operator permits it,
while reads retain the full key. A missing target in a complete domain is
refused, whereas unavailable membership/closure remains incomplete. Reached
unavailable values retain their typed cause, skipped unavailable values do not
change a decisive result, and no incomplete/refused value is exposed as a
completed result. Authority mutations use FR-056's typed refusal and locus.
