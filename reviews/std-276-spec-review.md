---
id: SR-16020
title: "Spec review of quire-spec-language PR #1263: STD-276, the claim extent of RE-2 per-step records"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@ede069343ef3020a50594a6e00b4d37a6afb4b63; PR #1263 (git diff origin/main...ede069343): spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md, spec/test-cases/TC-551-per-step-simulation-records-are-written-and-settled.md; read for context: FR-104 (extent and domain keys, AC-5, AC-11), FR-097, FR-075 (candidates and unbounded items), ADR-012 §1.1, ADR-020 §8 and RM-2, FR-103, FR-380; agent-ix/quire-specification main 5ee0071fc FR-468, FR-290"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-146
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-551
    type: reviews
---
# SR-16020: Spec review of quire-spec-language PR #1263: STD-276

## Summary

Ticket: STD-276. Sub-analyses folded in: integrity, EARS, failure-domain. The head ede069343 is behind main 423ac6aa0 but merges cleanly. `QSPEC_DIR=/Users/peter/dev/quire-specification make check-spec-validation` on the merge result exits 0 with no diagnostic on FR-146 or TC-551. QSL FR-146-AC-10 is new: the `FR-146-AC-10` in ADR-012 and FR-318 is QSpec's FR-146. No other open PR touches FR-146 or TC-551. No status wording and no compat layer.

The deviation from the ticket's fixed `unbounded` is the right call. It classifies the records by FR-097 over their roots, as every other `operation-contract` record already is (FR-104-AC-5, FR-104-AC-11), and ADR-014's extent describes the domains, not the quantifier. The vectors check out. ADR-020 §8's `Counter` fields are all `Int[0, 2]` or `Boolean`, `Impl::counters` has no authored maximum, and there are 7 step rows plus the `Initial` record, so 8 records. Each is `Unbounded` with one `Population(None)` domain, `Bounded` under `bound CountersBound on Impl::counters [2]` (FR-380 spelling as in FR-104-AC-11), and a `peek(n: Integer)` parameter adds an `Integer` domain to that row only. Keying by the O-07 occurrence matches FR-146-AC-9.

## Verdict

**MERGEABLE after small fixes.** No major findings. Three medium findings: the root list leaves out the operation's result, the root list differs from FR-104 although the text says it is FR-104's, and the ticket's routing question is not answered. One low finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | [minor] A `Step` record's roots leave out the concrete operation's result type. FR-104 classifies a clause record over "the clause's `self`, `result` and parameter types", and QSL model operations have results (FR-103: "its result value type or none"; ConfigVersion's `attemptUpdate` returns `Boolean`). A concrete operation returning an unbounded `Integer` would get a `Bounded` step record even though the step's post-state ranges over that result. Add the result type to the roots, or say why a step record does not range over it. | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md:155-160 |
| FND-002 | medium | [minor] The rule says it classifies "as FR-104 does for a clause's record", but its root list is different. FR-104 takes the context population that covers `T` and each population a `reaches` walks. FR-146 takes "each population of the concrete model". For a concrete model with two populations, a `Step` record on `Impl::Counter` gets both population domains, where FR-104 would give one. That may be correct, because the step map reads the whole concrete state, but then the sentence should state the rule instead of claiming FR-104's, and AC-10 should include a second-population case. The current single-population fixture cannot tell the two readings apart. | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md:150-164,224 |
| FND-003 | medium | [minor] The ticket asks how negotiation routes these records to the SMT candidate. The PR adds only "routed by their kind and extent". Under FR-075 and ADR-012 §1.1, candidates are matched by kind alone ("Mode ... SHALL NOT affect which backends are candidates"), and extent is compared against the candidate's mode only at `negotiate_*`. An `Unbounded` record settles `requires-bound` or `unsupported` unless its candidate advertises `operation-contract` in unbounded mode. Nothing states that the EN-2 SMT candidate does. Since FR-146-AC-8 expects all eight `Unbounded` records proved, state the candidate's advertised mode and the resulting disposition, or the expected `requires-bound` with a `Cardinality` bound. Reword "routed by" so it does not read as extent affecting candidacy. | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md:168-170,222 |
| FND-004 | low | [minor] QSpec follow-up: the PR body says FR-468 or FR-290 need the root list "if QSpec wants the per-record extent normative on the IR side". That is a fair statement of the open question. Record it as a planner item (a QSpec ticket or an explicit no-change), so it does not stay only in a PR body. | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md:148-164 |

## New findings (disposition pass 1)

Round 1 checked head 26314aa42, which contains main 6fb751a31 and merges cleanly with main 879c3bf73. `make check-spec-validation` on the merge exits 0. Hand checks:
- The `audits` population is read by nothing, so it adds no domain.
- A `peek` result of `Integer` adds one domain.
- Under a bounded-only backend, an `Unbounded` record whose domains are all boundable settles `requires-bound`, as ADR-012 §1.1 states.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | [minor] The context population is "the one population that covers the receiver's type (FR-104)". FR-104 refuses `ambiguous_declaration`/`ambiguous-name` when two populations cover the type, but FR-146 does not say what a step record does in that case. A row for an operation with no contract clause would hit it with no clause-level refusal first. State the outcome, or say that the FR-104 refusal applies at the row. | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md:154-158 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | A `Step` record's roots now include the receiver, result and parameter types, and FR-146-AC-10 has an `Integer`-result variant. |
| FND-002 | fixed | The root list is now FR-104's: the context population plus the populations a `reaches` walks. FR-146-AC-10 has an unread `audits` population that adds no domain. |
| FND-003 | fixed | Routing now says candidates are chosen by kind alone, and that extent meets the candidate's mode only at negotiation. FR-146-AC-8 is conditioned on an unbounded-mode backend, and FR-146-AC-10 gives `requires-bound` under a bounded-only backend. |
| FND-004 | deferred | The QSpec FR-468/FR-290 extent follow-up stays a planner item (a QSpec ticket or an explicit no-change). It is outside this QSL PR. |

## Dispositions (round 2)

Round 2 checked head b08c24499, which contains main 4c3562eaa and merges cleanly. `make check-spec-validation` on the merge exits 0. No new findings. In the `spare` variant, the clauses on `Impl::Counter` also refuse under FR-104, and the AC does not claim its refusals are the only ones, so the vector holds.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-005 | fixed | When the receiver's type conforms to member types of two or more populations, FR-146 refuses `ambiguous_declaration`/`ambiguous-name` at the step row, names the populations in ascending `DeclarationKey` order and writes no record. FR-146-AC-10 and TC-551 step 11 have the `spare` vector. |
