---
id: SR-15513
title: "gap analysis of STD-264 application-occurrence replay mode (QSL PR #1221)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@4b52347fe6a7d07471189e0d4745c08ec078148b; spec/functional/FR-098-execute-a-replay-request.md, spec/functional/FR-071-implement-typed-replay-request.md, spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md, spec/test-cases/TC-444-the-replay-executor-recompiles-selects-calls-and-refuses.md, spec/test-cases/TC-774-plugin-results-settle-as-typed-records-and-plugin-proofs-carry.md, spec/test-cases/TC-185-replay-request-exact-members-round-trip.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: reviews
---
# SR-15513: gap analysis of STD-264 application-occurrence replay mode (QSL PR #1221)

## Summary

Ticket: STD-264. This is a spec-only PR. Plan completion: not assessed. AC-to-TC trace:

- TC-185 scope adds FR-071-AC-15, and step 7 covers it.
- TC-444 scope adds FR-098-AC-22 to AC-24, and step 10 covers them.
- TC-774 scope is now FR-290-AC-1 to AC-15. Step 8 is rewritten for AC-12, and step 11 covers AC-15.

Each new step has an expected result. Nothing is implemented yet: on origin/main, `ReplayRequestWire`
(`qsl-replay/src/request.rs`) has no `occurrence_key` member and FR-098 has no claim selection. Code
belongs to the code lane. Merge order against open PRs:

- QSL #1219 (STD-265) edits FR-290's certification steps, diagnostics table and AC-7, with no hunk in
  common. `git merge-tree` of #1219 and #1221 over main is clean, so either can merge first.
- QSL #1216 (QSL-973) changes only FR-357 in the spec and does not conflict with this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | minor: `occurrence_key` is now required on every `function` request (default 2), and the spec does not say which key a value-parity request carries. FR-098 says only that "FR-357's two entries decode the occurrence key with the request and do not read it". Affected builders: <br>- CG's `src/replay/composite_builder.rs`, which builds the FR-357 requests, and `src/replay/function.rs:489`, which builds function-contract requests. Neither sets a key on quire-contract-codegen main be88468. No CG ticket is linked. <br>- About 20 `ReplayRequestWire` literals in QSL's `qsl-replay` and `qsl-analyze` tests. QSL #1216 (QSL-973, terra) reshapes the request's target in the same files (`request.rs`, `execute/*`, `spine/clause/tests/*`, `qsl-analyze/tests/it/*`), so the STD-264 code should start after #1216 merges. <br>Name the key an FR-357 request carries: the packet's own occurrence key, as ADR-013 C-12 copies it. Link a CG ticket for the two builders. | spec/functional/FR-098-execute-a-replay-request.md:464-465, spec/functional/FR-071-implement-typed-replay-request.md:167-175 |
| FND-002 | low | minor: some stated behaviour has no vector. Missing cases: <br>- a `NotReplayable` record owned by a synthesized function (`body` or `pre`), which the PR body itself lists; <br>- observation across binder-body iterations (default 4); <br>- `ClaimOccurrence` ordered against an argument-join refusal; <br>- the unbound-parameter and binder-root refusals (SR-15512 FND-002). <br>FR-290 also says QSL's request writer writes no `declaration` item, so AC-12's function-contract bullet uses a hand-built item; say so in the AC. | spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:292-307, spec/functional/FR-098-execute-a-replay-request.md:550-552 |
| FND-003 | low | minor: the PR says "No QSpec change is needed", but QSpec FR-323's decode rule 1 has no refusal for a `function` request that has no `occurrence_key`. QSL FR-071 adds `missing-member` at `/replay/occurrence_key` on its own authority. File a QSpec follow-up so FR-323 (the serialized authority, ADR-013) names that refusal and its pointer. | spec/functional/FR-071-implement-typed-replay-request.md:172-175 |

## Verdict

No major finding in this method. The majors are SR-15510 FND-001 (the `r` vectors fail `check`) and
SR-15512 FND-001 (matching an evaluation to its occurrence). The ids are unique, and the gate passes.
