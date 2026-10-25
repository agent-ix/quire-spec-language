---
id: SR-17091
title: "Gap analysis of PR #1490: ModelLetters fault channel and observation metering criteria (QSL-1039)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@8508046d5213213188fbcc01d9f417908e3dc739; PR #1490 (branch qsl-1039-letters-fault); spec/functional/FR-125 (Behavior, AC-1..AC-17), FR-281 (Engine refusals, AC-11), FR-380 (AC-7), FR-203 (Build refusals table, AC-6, AC-7), FR-127 (settlement map), FR-192 (PredicateRefused rows), ADR-014 TR-5; spec/test-cases/TC-520; qsl-eval/src/simulation/model/letters.rs, qsl-eval/src/model_subject.rs, qsl-analyze/src/analyze/items.rs, qsl-analyze/tests/it/model_instances.rs"
review_set: subset
date: '2026-10-25'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-520
    type: reviews
---
## Summary

Ticket: QSL-1039. PR: quire-spec-language#1490, reviewed at 8508046d5. The head 9426259936b2 only merges main, and the PR is merged as dd0fb3119, so these findings apply to main. No plan bundle; plan
completion was not assessed. No spec file changed, so spec-review was skipped.

The matrix was read with `quoin matrix --json` (quoin 0.29.1) at the head. For
the criteria this diff touches, FR-125-AC-1..6, 8, 10, 11, 12 and 17,
FR-281-AC-11, FR-380-AC-7 and FR-127-AC-1 are `tagged`. FR-125-AC-7, 9 and
13..16 are `untagged`; that is pre-existing and not changed by this diff. All
report `no run evidence`, as on main.

The four new tests carry `#[trace("TC-520")]` with no AC. That follows the
file's existing precedent: lines 583, 649 and others tag TC-520 alone.

- The tests do test something. Reviewer mutations M1 (cache ignores the
  digest) and M3 (refusal read as `False`) were re-run, and so was the
  coder's set.
- No AC owns this behaviour. A search of FR-125's ACs, FR-126 and FR-127
  found none for: a refused, unobservable or faulted `holds` read; its
  settlement; observing a state once; or charging the observation.
- The behaviour is spec-defined only indirectly:
  - FR-125 says a `holds` atom is evaluated "through the one clause evaluator
    (FR-107) over the observation at its position".
  - FR-380-AC-7 says an observation over `N + 1` objects of a bounded
    population refuses `cardinality_out_of_bound`/`above-maximum`, "as a
    resource limit never does".
  - FR-281's catch-all row says such a refusal settles "failed with its
    FR-272 diagnostic".

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No acceptance criterion covers what QSL-1039 implements. That is: a refused, unobservable or faulted `holds` read at EN-1 (refusal, not panic or reading); how it settles in `analyze`, replay and the closure checker; and observing each state once and charging the observation. The four new tests therefore bind to TC-520 only, and the coder's settlement and metering choices have nothing to verify against. This needs a spec ruling: add an FR-125 (or FR-126) AC, and an FR-281-AC-11 clause for `AtomRefused`. | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:100-103; qsl-analyze/tests/it/model_instances.rs:1093-1255 |
| FND-002 | medium | FR-281's "Any other engine refusal with no settlement-map row: `failed` with its FR-272 diagnostic" is not asserted for the new `AtomRefused` and `StateUnobservable` arms. `model_refusals_are_unsupported_only_for_unsupported_codes` (FR-281-AC-11) was not extended, and no test in the tree references either variant through `model_refusal`. So swapping the arms, or settling `FamilyRefused` as `BROKEN`, fails no test. That gap is how SR-17090 FND-001 went unnoticed. | qsl-analyze/src/analyze/items.rs:150-157,290-330 |
| FND-003 | low | `a_refused_holds_evaluation_is_a_refusal_of_the_atom_and_of_the_check` asserts FR-380-AC-7's refusal (`CardinalityOutOfBound`, count `N + 1` = 2 under `[1]`) at a new site, the EN-1 letters. It could also carry `FR-380-AC-7`, as `relationship_reachability.rs:1341` does for the evaluator site. Judgement call: the AC is phrased per observation, not per engine. | qsl-analyze/tests/it/model_instances.rs:1126-1186 |

## Verdict

Gaps found. The implementation matches the spec text that exists, with one
exception: SR-17090 FND-001, the settlement of a kernel refusal. The behaviour
the PR adds has no acceptance criterion of its own (FND-001), and its
settlement arms are untested (FND-002).

Resolve FND-002 in a follow-up by extending the FR-281-AC-11 test with
`AtomRefused(KernelRefused(CardinalityOutOfBound))`, `AtomRefused(FamilyRefused)`,
`AtomRefused(NotBoolean)` and `StateUnobservable`. FND-001 needs a spec ruling
from the spec owner; until it lands, TC-520-only tags are acceptable under the
existing precedent.

## Bindings

| Test | Claimed | Trace |
| --- | --- | --- |
| an_atom_that_is_no_holds_atom_of_the_clause_is_a_fault | TC-520 | correct (no AC exists) |
| a_refused_holds_evaluation_is_a_refusal_of_the_atom_and_of_the_check | TC-520 | correct; FR-380-AC-7 could be added |
| the_state_observation_is_charged_to_the_meter_on_every_read | TC-520 | correct (no AC exists) |
| a_state_is_observed_once_for_all_the_reads_of_it | TC-520 | correct (no AC exists) |
