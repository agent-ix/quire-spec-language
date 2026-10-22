---
id: SR-15510
title: "integrity review of STD-264 application-occurrence replay mode (QSL PR #1221)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@4b52347fe6a7d07471189e0d4745c08ec078148b; spec/functional/FR-098-execute-a-replay-request.md, spec/functional/FR-071-implement-typed-replay-request.md, spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md, spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md, spec/test-cases/TC-444-the-replay-executor-recompiles-selects-calls-and-refuses.md, spec/test-cases/TC-774-plugin-results-settle-as-typed-records-and-plugin-proofs-carry.md, spec/test-cases/TC-185-replay-request-exact-members-round-trip.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: reviews
---
# SR-15510: integrity review of STD-264 application-occurrence replay mode (QSL PR #1221)

## Summary

Ticket: STD-264 (also QSL-979 spec check). Diff `origin/main...4b52347fe`, 8 spec files. Checked the
new FR-098 claim-selection section, FR-071's `occurrence_key` member and refusal, FR-290's application
items, the ADR-013 O-26 and ADR-029 PL-7 amendments, and the TC-444, TC-774 and TC-185 steps against
each other and against QSpec FR-323 (main 429c59d). QSpec FR-323 does define `occurrence_key` on a
`function` request (`node_id`, `role`, `ordinal`), and its decode rule 1 refuses the key on frame and
state-clause requests, as the PR says. The new AC ids (FR-098-AC-22 to AC-24, FR-071-AC-15,
FR-290-AC-15) are unique across the open QSL PRs. The gate
`QSPEC_DIR=/Users/peter/dev/quire-specification make check-spec-validation` exits 0 (quire 0.36.2,
quoin 0.28.3), and the changed files get no new warning.

The `r` fixture was tested against the real checker. A probe test in `qsl-semantics`
(`check::claims::tests`, the TC-160 harness, through S1, S2 and `check`) checked
`function r using v(x: Int[0, 9]): Int[0, 9] pure { x + 1 }` beside `p` and `q`. `check` refuses the
whole unit with `Unproved(Range { required: [0, 9], proved: [1, 10] })` at `r`'s body. The same
harness admits `function r using v(x: Int[0, 9]): Integer pure { 10 mod x }`, with one
`value-validity` record whose result bound is `Integer` and whose owner is `r`. It refuses
`n + 1` narrowed to `Int[0, 10]` over `n: Integer` (Unproved, unbounded), and a guarded `x + y`
narrowed to `Int[0, 9]` (Unproved, `[0, 18]`). So `check` proves every narrow it admits.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | major: the vectors built on `r` cannot run. `check` refuses `function r using v(x: Int[0, 9]): Int[0, 9] pure { x + 1 }` with `Unproved(Range { required: [0, 9], proved: [1, 10] })` (reproduced in the TC-160 harness at 4b52347fe). The refusal covers the whole unit, so `p` and `q` in the same unit cannot run either. That breaks FR-098-AC-22, AC-23 and AC-24, FR-290-AC-15, TC-444 step 10 and TC-774 step 11. `check` proves every narrow it admits, so no checked package reaches the "result outside the record's result bound" arm. Only the "undefined at its operands" arm can refute. Replacement: `function r using v(x: Int[0, 9]): Integer pure { 10 mod x }`. It checks, with one `value-validity` record (bound `Integer`, owner `r`). With `x = 0` it is undefined (division by zero, as in FR-141-AC-7 and `qsl-replay/tests/it/refinement_ring.rs`), so it refutes; with `x = 3` it gives `1`, which is `replay-parity`. For AC-23's "narrow with no record" case, use a separate `function inc using v(x: Int[0, 9]): Int[0, 10] pure { x + 1 }`. Also state that the bound arm is defensive, unreachable while `check` proves narrows. | spec/functional/FR-098-execute-a-replay-request.md:550-552, spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:492, spec/test-cases/TC-444-the-replay-executor-recompiles-selects-calls-and-refuses.md:76, spec/test-cases/TC-774-plugin-results-settle-as-typed-records-and-plugin-proofs-carry.md:29 |
| FND-002 | low | minor: ADR-013 O-26's "Validation and diagnostics" row does not list the two new refusals: a `function` request without `occurrence_key` (FR-071) and an occurrence key that selects no claim (`ClaimOccurrence`, FR-098). FR-098 says each O-26 refusal is a typed `ReplayRefusal` variant and lists `ClaimOccurrence` among them. The PR edits the Public type and Conversions rows of that table but not this row. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:1014 |
| FND-003 | low | minor: the `NotAPredicate` rule is now stated twice in different words: in the target-resolution bullet ("SHALL refuse a predicate claim whose function's declared result is not `Boolean` before any call") and in the new section's step 1. Keep one statement and point to it from the other. | spec/functional/FR-098-execute-a-replay-request.md:164-169,440-443 |

## Verdict

Not mergeable as written. FND-001 is major: every new execution vector uses a unit that `check`
refuses. The change itself is consistent: FR-071, FR-098, FR-290, ADR-013, ADR-029 and the three TCs
agree on the `occurrence_key` member, the claim selection and the refusal order. The QSL-979
conclusion holds: FR-057's text says "A `Value` function declaration carries no clause; each scalar
operation application in its body is one claim of its own", and FR-062 part 4 places requirements only
at scalar applications. Lead defaults 1, 5, 6 and 8 are sound as written.
