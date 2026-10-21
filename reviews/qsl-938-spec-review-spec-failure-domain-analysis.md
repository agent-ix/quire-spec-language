---
id: SR-13676
title: "failure-domain review of QSL-938 FR-395 owner rule and fault arm (PR #1147)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@4e97aeaad6d855b125b7cbd4bbcc6870007a6db2; spec/functional/FR-395-associate-the-selected-call-s-requirement-items-through-the-replay-facade.md, spec/test-cases/TC-1089-the-replay-facade-associates-the-selected-call-s-requirement-items.md, spec/functional/FR-062-implement-checked-family-contract.md, spec/functional/FR-075-compute-candidates-from-registered-backends.md, spec/functional/FR-100-run-a-named-function-through-the-spine.md, spec/spec.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-395
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1089
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: reviews
---
# SR-13676: failure-domain review of QSL-938 FR-395 owner rule and fault arm (PR #1147)

## Summary

Ticket: QSL-938. Failure modes, identity confusion and edge cases of the
owner rule and the fault arm, checked against the check stage on origin/main.

The check stage writes the requirement map from four sources
(qsl-semantics/src/check/mod.rs). These are value claims of each function
and predicate body (`key_claims`, 2714), state-clause and frame
`operation-contract` records (2761), `temporal-satisfaction` records of
temporal clauses (2890-2925), and attempt and compensation frame
`operation-contract` records (2984, 3047). Clause bodies produce no
`value-validity` record (`clause_records` writes only `operation-contract`).
So a claim site inside an `Expect`-style clause body, or inside a function
called from a clause, raises no ownership question: a function's own
records are owned by that function. Measure occurrences have no record
(RR-17), so a function with scalar operations only in `decreases` has an
empty list, as FR-395 says. Identity confusion is real: K's `x + 1` is one
node across `inc` and `sq`, and FR-062 correctly makes the owner a recorded
field, not something derived from the key.

Examined: FR-062 Owner paragraph, FR-062-AC-14, FR-395 Terms (Owner, Item of
the call), Association rule, Ambiguous-association fault, Outcome
independence, Arguments do not select items, AC-10.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The Owner paragraph gives an owner rule only for `value-validity` (one owner) and `operation-contract` (none) records. The same map also holds `temporal-satisfaction` records (FR-326/FR-337, inserted at qsl-semantics/src/check/mod.rs:2890-2925), and any further kinds FR-290's vocabulary admits. A temporal clause can name a function through `requires temporal` (FR-218; `function_kinds` at mod.rs:1355). So "owned by the function it names" is a plausible second reading, which would make such records members of that function's call. State that every record kind other than `value-validity` has no owner, and add a temporal record to FR-062-AC-14. | spec/functional/FR-062-implement-checked-family-contract.md:241-246 |
| FND-002 | medium | FR-395's fault arm (a record with no owner, two owners or an unknown owner refuses `Fault`, `runtime_invariant`, exit 30, with no partial list) has no AC at the entry. AC-10 restates FR-062-AC-14's check-stage invariant, and no test of it can fail on an entry that ignores the fault. Also, "two owners" cannot happen if the owner is one field. And the fault's `details` stage and invariant identifiers are unnamed, though FR-100 names one per fault (for example `spine-run`/`locus-source-supplied`). Name the invariant, drop or define the two-owner case, and add an entry-level AC through a test seam that feeds an ownerless `value-validity` record. Or state that the arm is defensive and verified by inspection. | spec/functional/FR-395-associate-the-selected-call-s-requirement-items-through-the-replay-facade.md:149 |
| FND-003 | low | The owner's identity is unspecified: a function name, a declaration index or the function's node key. "An owner that names no function of the checked package" and the match against the selected function (`select` resolves by one-segment name) depend on it. Say what the owner records, for example the declaration's name as `select` resolves it. | spec/functional/FR-062-implement-checked-family-contract.md:241-246 |

## Verdict

Not mergeable until FND-001 and FND-002 are settled. FND-001 leaves the
membership of temporal records open, and FND-002 leaves the fault arm
unverified with no stable invariant id. The rest of the failure domain is
sound: outcome independence, arguments not filtering, one compile for
key/index/package id, and no ownership question for clause bodies or
called functions.
