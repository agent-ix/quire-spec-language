---
id: SR-1937002
title: "gap analysis of PR #1937 (QSL-1338 slice b): narrowed ConfigVersion fixtures and the RA state-key test"
type: SpecReview
analysis: gap-analysis
date: 2026-10-31
scope: "agent-ix/quire-spec-language; PR #1937 git diff origin/main...fd15a17ef (FR-126-AC-1, FR-133-AC-2, FR-339-AC-5, FR-229-AC-1, FR-181-AC-1, FR-216-AC-1; qsl-analyze/tests/it/{example_units,strong_hint,component_certificate,release_acquire,hyper_single,protocol_footprints,units}.rs)"
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-126
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-133
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-339
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-229
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-181
  type: references
- target: ix://agent-ix/quire-spec-language/FR-216
  type: references
---
# SR-1937002: gap analysis of PR #1937 (QSL-1338 slice b)

## Summary

Ticket: QSL-1338. Plan completion: not assessed. The PR changes no spec and adds no criterion. I checked criteria against tests by hand over the diff, using its `#[trace]` tags. The checks were these. Do the narrowed `VersionNumber` domains still hold every literal and every reachable value the tagged ACs need? Is the new TC-674 test a real check of the encoder? Is each memoized engine path still covered by a test that fails if the memo returns a wrong expansion? All the focused tests pass at the head: 89 + 79 + 166 in `qsl-analyze --test it`. The new FR-229-AC-1 test also passes with `ra.rs` reverted to origin/main, so it pins the old encoder's output.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-126-AC-1 | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:530 | examined |
| FR-133-AC-2 | spec/functional/FR-133-name-the-strong-constraint-that-would-exclude-a-refutation.md:102 | examined |
| FR-339-AC-5 | spec/functional/FR-339-check-an-en-1-component-certificate.md:229 | examined |
| FR-229-AC-1 | spec/functional/FR-229-carry-the-memory-component-through-the-protocol-system.md:124 | examined |
| FR-181-AC-1 | spec/functional/FR-181-check-a-single-existential-claim-through-the-possible-family.md:163 | examined |
| FR-216-AC-1 | spec/functional/FR-216-give-every-protocol-step-a-static-footprint.md:154 | context_only |
| config_over / CYCLES / with_versions_up_to(2) | qsl-analyze/tests/it/example_units.rs:41-84 | examined |
| guarded_world / with_versions_up_to(7) | qsl-analyze/tests/it/component_certificate.rs:669-690,800-920 | examined |
| the_sc_graph_member_names_events_and_edges_by_depth | qsl-analyze/tests/it/release_acquire.rs:347-480 | examined |
| the_engine_agrees_with_a_brute_force_search_of_lassos | qsl-analyze/tests/it/hyper_single.rs:1166-1215 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

**Clean. No gaps and no orphan code.**

1. `Int[0,2]` (`config_over`, used by strong_hint and example_units). `CYCLES` reaches 0 to 2 only. The shipped unit is `VersionUnchanged`. Every initial pair is in {0,1,2}: (0,0), (0,1), (0,2) and (2,0). Every formula literal is 0 or 2, including `6 mod (2 - c.versionNumber)`. FR-126-AC-1's 30/15 product states and its lassos, and FR-133-AC-2's `{Strong, attemptUpdate, Each}` hint at `upd(a)`, are about reached states, which are unchanged. A candidate above 2 was always rejected by the post clause. The AC's `max_candidates` 1,000,000 is still not reached. No ConfigVersion test sets a `MaxCandidates` limit that a 1,001-wide domain could have tripped. The `MaxStates`, `MaxTransitions` and `EvaluationMeter` limits count states, transitions and per-clause meters, not domain width.
2. `Int[0,7]` (`guarded_world`, FR-339-AC-5). The AC's two conditions are `c.versionNumber = 1` and `c.versionNumber = 7`. 7 is the bound, so the never-met condition is still a well-typed value that is never reached. That is what the `MissingAcceptance{set: 0}` row needs. `with_versions_up_to` asserts that the old bound is `"1000"` before it rewrites it, so the narrowing cannot silently match something else.
3. FR-229-AC-1 (TC-674). The new test compares the SC graph, both views of `left`, and `x`'s first message against hand-written JSON literals, not against another encoder call, so it is not a tautology. It passes with origin/main's `ra.rs`, so it pins the pre-PR bytes that the refactor had to preserve. The existing `keys()` comparisons in `release_acquire` also still pass.
4. Memo coverage. FR-181-AC-1's brute-force oracle (`hyper_single`) expands through its own `Expansions` cache over `ModelSystem::successors`, not through `WalkMemo`. So the engine's memoized phase 0 and loop checks are compared against an independent expansion. Sharing the oracle's expansions across clauses is sound because the vault's transitions do not depend on the clause. The weak-memory and litmus suites (166 tests) exercise `apply_attempt` and `enabled_attempt` on probes and on per-message reads.

Fidelity note, not a defect: FR-126-AC-1 and FR-339-AC-5 name ADR-018 §6's example unit, and the fixtures now use a narrower `VersionNumber` type than that domain. Every observable the ACs state is a reached-state property, so the narrowing changes no outcome the ACs fix.
