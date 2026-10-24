---
id: SR-13676
title: "failure-domain review of QSL-938 FR-395 owner rule and fault arm (PR #1147)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@952dcf836b98e1e6acaabc092bf2e5214e227793; spec/functional/FR-395-associate-the-selected-call-s-requirement-items-through-the-replay-facade.md, spec/test-cases/TC-1089-the-replay-facade-associates-the-selected-call-s-requirement-items.md, spec/functional/FR-062-implement-checked-family-contract.md, spec/functional/FR-075-compute-candidates-from-registered-backends.md, spec/functional/FR-100-run-a-named-function-through-the-spine.md, spec/spec.md"
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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | high | The temporal-satisfaction fixture FR-062-AC-14 and TC-1089 step 6 name, "TC-590 step 4's `possible Q` claim over its `Config` domain package", is `possible Q using v over (c: Config::ConfigVersion) { c.versionNumber = newVersion }`. TC-590's Expected Results for step 4 say "the parameter claim refuses `missing-name` at the name; no checked claim for any refusal". So the unit has no checked package and no record, and the AC's "has no owner" cannot be observed. Use a claim that checks and writes a `temporal-satisfaction` record: TC-590 step 1's `possible P` (Expected: records `temporal-satisfaction`, `Unbounded`), or step 4's no-`over` `possible Anything using v { true }`. | spec/functional/FR-062-implement-checked-family-contract.md:369 |


## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | Probe (temporary test in tests/it/temporal_negotiation.rs on 952dcf836, reverted): the exact clause `temporal Reaches using t over (c: Config::ConfigVersion) clock "model-steps" on origin { eventually holds(c.versionNumber = 2) }` checks under the default `check`. `graph.requirements()` then holds exactly one record, `(818c7c23..., claim, 0)` of kind `TemporalSatisfaction`, written by the temporal-clause path and not from a function body. So the fixture works. The clause is written out in full, but its unit is not. Alias `t` needs `profile t = "quire.temporal.infinite-trace/v1";`. The model needs `profile v = "quire.model.complete/v1";`, `model Config = "example/config-version" version "1.0.0" digest "sha256-jcs:<the package digest>";` and the ConfigVersion domain package. Neither TC-842's spec text nor TC-1089 states them. That package lives in the root crate's test support (`tests/it/support/config_version_spine`), not in qsl-replay/tests, where TC-1089 says it runs. State the unit header and package source in TC-1089 step 6 (tests/it/temporal_negotiation.rs:30-36 is the working text), or move that step to the root crate's tests. | spec/functional/FR-062-implement-checked-family-contract.md:369 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | 29f11fe9b: **Owner.** The owner of a record is the qualified name of the `Value` function declaration whose body holds its claim site, as the checked graph holds that name. The checker SHALL record the owner as a single field of each `value-validity` record, from the function it was checking when it found the site, so a consumer never recovers it from the occurrence key. The checker SHALL record no owner for an `operation-contract` record (FR-104) and none for a `temporal-satisfaction` record: a temporal clause is not a function body, and a function it names through a predicate does not hold the clause's |
| FND-002 | fixed | 29f11fe9b: - **Broken ownership is a fault.** The facade SHALL refuse with `Fault` (code `runtime_invariant`, stage `spine-run`, invariant `requirement-owner-known`, exit 30), and return no partial member list, when a `value-validity` record has no owner or an owner that names no function of the checked package. The owner is a single field, so two owners cannot arise and the facade never picks one. \| The association step, run directly on a checked package whose record map has a `value-validity` record with no owner, refuses `Fault` with stage `spine-run` and invariant `requirement-owner-known`; the same  |
| FND-003 | fixed | 29f11fe9b: **Owner.** The owner of a record is the qualified name of the `Value` function declaration whose body holds its claim site, as the checked graph holds that name. The checker SHALL record the owner as a single field of each `value-validity` record, from the function it was checking when it found the site, so a consumer never recovers it from the occurrence key. The checker SHALL record no owner for an `operation-contract` record (FR-104) and none for a `temporal-satisfaction` record: a temporal clause is not a function body, and a function it names through a predicate does not hold the clause's |
| FND-004 | still-open | Probe (temporary test in qsl-semantics/tests/it/state_graph_claims.rs on a4c01ebd3, reverted): TC-590 step 1's `possible P using v over (c: Config::ConfigVersion) { c.versionNumber = 2 }` gives no record in `CheckedGraph::requirements()`. Under the default `check` the unit refuses `unsupported_construct`/`not-yet-implemented` (FR-165: a checked state-graph claim is refused as not emitted, test `a_checked_claim_is_refused_as_not_emitted_by_default`), so there is no checked package. Under the test-only `with_unwritten_state_graph_claims()` hook it checks, but `graph.requirements().len()` is 0. Its `temporal-satisfaction` record lives only on `CheckedStateGraphClaim::claim()`, not in the map FR-395 reads. TC-1089 step 6's "which yields a `temporal-satisfaction` record" is false. Use a temporal clause, which the check writes into the map (qsl-semantics/src/check/mod.rs temporal_clause records): TC-842's `temporal Reaches using t over (c: Config::ConfigVersion) clock "model-steps" on origin { eventually holds(c.versionNumber = 2) }` (tests/it/temporal_negotiation.rs:35, which reads a TemporalSatisfaction record from `graph.requirements()`). |
| FND-004 | fixed | 952dcf836: For each fixture RR-1 to RR-17, every record has exactly one owner, the qualified name of the function that holds its claim site; for RR-5's three records the owner is `sq`. The `operation-contract` record of QSpec TC-198 L07 (`post Q using V on M::A::remove` over its F1 domain package) and the `temporal-satisfaction` record the check stage writes into the map for the temporal clause `temporal Reaches using t over (c: Config::ConfigVersion) clock "model-steps" on origin { eventually holds(c.versionNumber = 2) }` (TC-842, FR-332-AC-1) each have no owner; a state-graph claim's record, held on th |
