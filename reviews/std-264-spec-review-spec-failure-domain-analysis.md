---
id: SR-15512
title: "failure-domain review of STD-264 claim selection, observation and refusal order (QSL PR #1221)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@4b52347fe6a7d07471189e0d4745c08ec078148b; spec/functional/FR-098-execute-a-replay-request.md, spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: reviews
---
# SR-15512: failure-domain review of STD-264 claim selection, observation and refusal order (QSL PR #1221)

## Summary

Ticket: STD-264. This review checks the application claim's meaning against FR-062 (claim sites, Key,
Extent, path condition, Owner), FR-057's claim-form table, FR-016's short circuit, FR-072's
settlement, and the S6a evaluator on origin/main (`qsl-eval/src/value/expression/evaluate.rs`: its
`Trail` is keyed by `Location`, and `on_claim_level` excludes nested frames). It also checks the
refusal order. Sound as written:

- A call that never evaluates the occurrence settles `success` (default 6).
- A recursive or nested call is not observed (default 4).
- A stop inside an operand gives `NoValue`.
- Short circuit keeps every observation on the path condition. S6a evaluates more lazily than FR-062
  records guards, which can only lose a reproduction; it cannot make a false one.
- `ClaimOccurrence` comes after the function resolves and before the argument join, so before
  `UnusedObjectEnvironment`, which runs after the join.
- Frame and state-clause requests never reach the selection.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | major: the spec does not say how an evaluation is matched to the requested occurrence. One canonical node can have several `expression` occurrences in one body, each with its own record, path condition and operand values: FR-062 RR-5 (`(x + 1) * (x + 1)`), RR-15 (two `t * 2`, one `Bounded` and one at `n`) and RR-16. FR-098 converts only the key's node to a `NodeKey` and says to observe evaluations "at the record's occurrence". An observer keyed on the node, the obvious way to build it, would record another occurrence's failure against the requested claim, and the item would settle `refuted` on a counterexample that does not reproduce. Example: `function s using v(x: Int[0, 9]): Integer pure { (if x > 0 then 10 mod x else 0) + 10 mod x }` at ordinal 0 with `x = 0`. Ordinal 0 is never evaluated (its guard is false), but ordinal 1 is undefined, so a node-keyed observer refutes. The correct result is `NoValue`. Fix: say the observation matches the claim site's `Location`, the one `check` paired with the occurrence key (FR-062 "Key"), and never the node alone. Add this two-occurrence vector to FR-098-AC-22. | spec/functional/FR-098-execute-a-replay-request.md:431-438,470-479 |
| FND-002 | medium | minor: default 7 refuses replay when an assignment names a binder root or leaves an owner parameter unbound. Under FR-062's Extent rule, only parameters read in the application's operands or guards are roots. So any application that does not read every owner parameter can never be refuted (for example, `10 mod x` in `f(x, y)`): it settles `inconclusive`/`replay-refused`, and nothing tells the user that this is a limitation rather than a refusal. No AC covers either refusal or its code. Filling the non-root parameters with any admitted value would be sound: the operands and guards read only roots, and an early stop elsewhere is `NoValue`, never `refuted`. Either adopt that, or state the limitation in FR-290 and add one vector for an unbound parameter and one for a binder root, each with its O-26 refusal and code. | spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:287-290 |
| FND-003 | low | minor: the same defect gets two codes, one on each side. A key whose node is not in the package refuses `ClaimOccurrence` (`stale_dependency`/`content-mismatch`) in `replay`, but `UnknownOccurrence` (`missing_declaration`/`missing-name`) in FR-290's `request` (AC-14). Default 3 cites FR-357's `ScalarIdentity`, so it is defensible. But `package_id` has already matched when the selection runs, so "another function's declaration" or "a record another function owns" is not staleness. Say why `content-mismatch` is right here, or align the two sides. | spec/functional/FR-098-execute-a-replay-request.md:450-455 |
| FND-004 | low | minor: the request's `obligation_identity` is not checked against the claim that `occurrence_key` selects. A function request whose identity was minted for the function contract but which carries an application key, or the reverse, is not refused. Its result then names an obligation it did not decide. FR-357 checks its identity against the claim (`ScalarIdentity` `Obligation`). Add a check, or state that the identity is carried and not verified on this path. | spec/functional/FR-098-execute-a-replay-request.md:431-438 |

## Verdict

Not mergeable as written. FND-001 is major: as specified, the observation allows an implementation that
refutes a claim on a counterexample that does not reproduce, which breaks the "refuted only when replay
reproduces" invariant (FR-289 item 4). Lead defaults: 4 is sound once FND-001 is fixed; 7 is sound but
costs coverage (FND-002); 3 is acceptable (FND-003); 6 is sound.
