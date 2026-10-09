---
id: SR-2453
title: "Failure-domain review of the QSL certificate-locus wire conformance (FR-314 step index) at 84fba37c8"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@84fba37c8a342218e99aa0f2cb66b89204e3e879; diff against origin/main b24dbda01d56843cd14d8cab9233ec3cff67535f: spec/functional/FR-314-check-an-smt-proof-certificate.md; context: spec/functional/FR-338-check-an-en-1-closure-certificate.md, spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md (PC-6), spec/test-cases/TC-893-an-smt-proof-certificate-is-checked.md; counterpart agent-ix/quire-specification@cff80d04fa88ecae0831be2ef2fb70adabed1c98 FR-331 and proposals/backend-provider-v1/schema.json"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-314
    type: reviews
---
# Failure-domain review of the FR-314 step-index locus

## Summary

Prepublication review, no PR and no ticket assigned yet. PR: pending. This
pass checks that the new `ProofStep{part, index}` locus is total. For every
certificate that FR-314 rejects at a step, exactly one index must exist, and
it must fit the wire.

Units examined:

- FR-314 Inputs, lines 69-80.
- FR-314 Behavior, lines 87-115: `ShapeMismatch`, `QueryMismatch`,
  `ProofStepInvalid`, `NotRefutation`, `Unverifiable` and `Verified`.
- FR-314-AC-2, TC-893, and ADR-018 PC-6.
- QSpec `U64String` and `CertificateLocus` (schema.json lines 58 and 651-656).

Checked and found clean:

- **`u64` bounds.** No proof that can be held in memory has more than 2^64
  commands, so the index never overflows.
- **Exactness.** `U64String` admits exactly the canonical `u64` spellings.
  `0` is the only spelling with a leading zero.
- **`Query{part}`.** It is total for `ShapeMismatch` and `QueryMismatch`,
  because those name a part, not a step.
- **Rejection and refusal.** A step rejection (`inconclusive`) and an
  unverifiable step (`proved`, `Uncertified`) share one index meaning and do
  not collide.

## Verdict

FAIL on two edge cases. The locus has no defined value for a proof with zero
commands. Nor is any rule or locus defined for a proof whose bytes are not
Alethe. Both are inputs an untrusted solver can send.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-314 rejects "with `NotRefutation` at the last step of a proof that does not conclude the empty clause". A carried `proof` with zero commands, such as empty bytes, does not conclude the empty clause and has no last step, so no `ProofStep{part, index}` exists for it. The new definition ("the zero-based position of its command") cannot produce one, and QSpec's closed `at` admits nothing else but `Query{part}`. Implementations will diverge: one invents index 0 for a command that does not exist, one uses `Query{part}`, and one panics or refuses. Fix: state the locus for an empty proof, for example "`NotRefutation` at `Query{part}` when the proof has no command", and add the vector to FR-314-AC-2 and TC-893. | spec/functional/FR-314-check-an-smt-proof-certificate.md:76-77; spec/functional/FR-314-check-an-smt-proof-certificate.md:106-107; spec/functional/FR-314-check-an-smt-proof-certificate.md:139 |
| FND-002 | medium | Indexing by "its command in that part's proof, in text order" assumes that the proof bytes parse into commands. FR-314 defines no rule and no locus for a `proof` that is not well-formed Alethe: unbalanced parentheses, an unknown command, invalid UTF-8, or a premise naming an `:id` defined twice. `CertificateRule` has no `Malformed` for SMT (FR-163 adds one only for hyper certificates), and the `Unverifiable`/`ProofStepInvalid` split needs a parsed step. The gap predates this commit, but the commit makes the locus depend on command structure, so the locus is undefined exactly where an untrusted solver's bytes are worst. Fix: say which rule and locus a non-Alethe proof rejects with (for example `ProofStepInvalid` at the first command that fails to parse, or `NotRefutation` at `Query{part}`), and add one vector. | spec/functional/FR-314-check-an-smt-proof-certificate.md:47-50; spec/functional/FR-314-check-an-smt-proof-certificate.md:76-77; spec/functional/FR-314-check-an-smt-proof-certificate.md:102-107; spec/functional/FR-338-check-an-en-1-closure-certificate.md:51-55 |
