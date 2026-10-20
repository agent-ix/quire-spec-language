---
id: SR-13451
title: "Gap analysis of QSL-551 E14 PR #1092 against FR-176 (AC-1..AC-4), FR-183 (AC-1..AC-4), TC-601 and TC-608"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@41db278b3; spec/functional/FR-176-*.md, spec/functional/FR-183-*.md, spec/test-cases/TC-601-*.md, spec/test-cases/TC-608-*.md; qsl-analyze/tests/it/hyper.rs, qsl-analyze/tests/it/hyper_replay.rs, qsl-analyze/src/model_check/hyper.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-176
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-183
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-601
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-608
    type: reviews
---
# SR-13451: Gap analysis of QSL-551 E14 PR #1092

## Summary

Ticket: QSL-551 (E14, part 2). Tools: `quire matrix --scope . --format tsv`
(quire 0.36.2, engine 0.50.2), exit 0. Plan completion: not assessed.

Matrix at 41db278b3:

| AC | Status | Bound by | Honest? |
| --- | --- | --- | --- |
| FR-176-AC-1 | untagged | - | yes; 10 vs 14 states is a spec follow-up |
| FR-176-AC-2 | tagged | hyper.rs:183 (doc citation only) | no (FND-002) |
| FR-176-AC-3 | tagged | hyper.rs:214, :294 | yes |
| FR-176-AC-4 | tagged | hyper.rs:328, :361, :405 | yes (FND-006 low) |
| FR-183-AC-1 | tagged | hyper_replay.rs:82 (doc citation only) | no, partial (FND-003) |
| FR-183-AC-2 | untagged | - | yes, not built |
| FR-183-AC-3 | tagged | hyper_replay.rs:102, :158 (doc citations) | no, partial (FND-004) |
| FR-183-AC-4 | tagged | hyper_replay.rs:191 (doc citation) | no, partial (FND-005) |
| FR-183-AC-5, AC-6 | untagged | - | yes (TC-610, TC-602) |
| FR-182-AC-1..AC-9 | untagged | - | yes, settlement not built |

The tests that carry an explicit `FR-176-AC-n` tag are truthful. AC-3's tests
cover the per-variable fairness liveness clause and stutter pairing. AC-4's
cover the pre-check stop, the undefined `match` and determinism. The PR body's
"not claimed" list is honest.

The problem is the matrix. It also binds an AC that a test's doc comment cites
in prose, so four ACs the PR calls unclaimed or partial read as covered.

Out of this PR's scope, as stated in its body:

- FR-163's certificate, so `Holds` settles no `proved`
- FR-182's settlement
- the FR-183 facade, envelope and reader
- FR-174's reductions
- `max_depth`
- the HP-1, HP-3 and HP-6 engines

QSL-551 stays open for them.

Spec follow-ups (re-measured; spec questions, not code bugs):

- Leaky product 10 vs 14 states (FR-176-AC-1, ADR-023 §8.1). The test asserts 10 and 2 accepting SCCs. By hand with the shipped tableau, which guards the `q0` stay on the letter: 2 x 2 equal-secret states, plus 3 per unequal pair ((0,0) q0, (0,1) q1, (1,0) q1), gives 10. The ADR's 14 assumes a `q0` that also stays on a differing letter. The two automata accept the same language, and ADR-023 says its §8 counts are hand enumeration. The AC needs restating; the PR rightly leaves it unclaimed.
- FR-176-AC-2's spelling `a.step.op = V::Vault::step`. Reproduced: S3 refuses it as `Hyper(MatchReadsState)`, from qsl-semantics/src/check/hyper_clause.rs:1470 on main. FR-172 types `t.step.op` but names no operation-constant operand. Either FR-172 adds one or the AC is respelled. This is not this PR's code.
- Vacuity across instances. HM-7 is per clause, and FR-176 does not say how vacuity reads when one instance is vacuous and another is not. The PR rules vacuity item-wide: VacuousMatch only when no instance has a matched fair cycle. That is the permissive reading, since a vacuous instance can sit under `Holds`. A spec ruling is needed.
- `max_transitions` counts joint steps (FR-176, FR-184). The engine counts retained product edges, so one joint step with two automaton successors counts twice. FR-126 charges each followed product edge the same way, so the engine is consistent with the liveness phase. FR-184's wording should say which it means.

## Verdict

**Not mergeable as is.**

- **FND-001 (medium).** No test runs a co-safety body, which is how SR-13450
  FND-001 got through the suite.
- **FND-002 to FND-005 (medium).** Prose AC citations make the matrix
  overclaim four ACs.
- **FND-006 and FND-007 (low).**

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test runs an HP-2 body whose negation has no acceptance set (`eventually p`, a bare state atom); every TC-601 fixture is `always`/`always eventually`, which is why SR-13450 FND-001 (an internal fault on such bodies) passes the suite. FR-176 admits any infinite-trace body; a regression test needs one co-safety body that holds and one that is violated. | qsl-analyze/tests/it/hyper.rs |
| FND-002 | medium | `quire matrix` binds FR-176-AC-2 to `without_match_the_secure_vault_is_refuted_and_an_unsatisfiable_match_is_vacuous` (hyper.rs:183) through the doc comment's leading `FR-176-AC-2:` citation, although the test carries only `#[trace("TC-601")]`, runs a respelled clause, and the PR states the AC is not claimed. The matrix is the coverage record; prose citations count as binders here (FR-176-AC-1, whose test docs cite no AC id, stays untagged). Drop the AC id from the doc or reword it, or claim it once the spelling is resolved. | qsl-analyze/tests/it/hyper.rs:173-183 |
| FND-003 | medium | `quire matrix` binds FR-183-AC-1 to `the_leaky_lockstep_counterexample_replays_to_its_evaluated_witness` (hyper_replay.rs:82) via its doc citation; the test covers only the Lockstep half, and the AC's WitnessExhausted half is not built. PR body says AC-1's WitnessExhausted half is not covered. | qsl-analyze/tests/it/hyper_replay.rs:76-82 |
| FND-004 | medium | `quire matrix` binds FR-183-AC-3 to hyper_replay.rs:102 and :158 via doc citations; the AC's `Projected` refusal (a `step` marked skipped) has no arm or test. Partial-AC binding; the repo precedent leaves a partially covered AC untagged. | qsl-analyze/tests/it/hyper_replay.rs:96-158 |
| FND-005 | medium | `quire matrix` binds FR-183-AC-4 to hyper_replay.rs:191 via its doc citation; the test covers the first sentence only (no WitnessExhausted envelope, no `max_witness_set` 0 replay). Partial-AC binding. | qsl-analyze/tests/it/hyper_replay.rs:186-191 |
| FND-006 | low | FR-176-AC-4 asks for byte-equal counterexamples; `the_leaky_request_run_twice_gives_equal_counterexamples` compares `PartialEq` and `Debug` text, not encoded bytes. HyperCounterexample has no wire encoding yet, so the test cannot check bytes; say so in the doc or encode once the wire exists. | qsl-analyze/tests/it/hyper.rs:401-416 |
| FND-007 | low | Behaviour in FR-176's text with no test: `max_transitions` on the product, more than one instance (ranking and the vacuity reading across instances), `each` granularity and strong constraints in the component-scoped filter, and a vacuity verdict decided by a terminal stutter loop. The random oracle in SR-13450 covered weak whole constraints only. | qsl-analyze/tests/it/hyper.rs |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | medium | `instances_are_ranked_by_depth_and_a_finding_ends_the_later_ones` never reaches the FR-126 cap: its violations are second-phase lassos, so `best` stays `None` (instrumented `cap=None` for both instances) and the tight-budget run passes trivially. Mutating the cap to `None`, removing the depth-0 `break`, or ranking a `match` hit at `depth` all leave every `hyper` test green (SR-13450 FND-002/FND-003 fixes are untested). | qsl-analyze/tests/it/hyper.rs:681-714 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | New test runs `eventually p` and a bare atom, holding and violated, with replay; it fails when the FND-001 fix is mutated away. |
| FND-002 | fixed | `quire matrix` at b6c72979e: FR-176-AC-2 untagged (doc no longer cites the AC id). |
| FND-003 | fixed | `quire matrix` at b6c72979e: FR-183-AC-1 untagged. |
| FND-004 | fixed | `quire matrix` at b6c72979e: FR-183-AC-3 untagged. |
| FND-005 | fixed | `quire matrix` at b6c72979e: FR-183-AC-4 untagged. |
| FND-006 | fixed | Doc states equality is `PartialEq` plus `Debug` text because the payload has no wire encoding yet. |
| FND-007 | fixed | Tests added for `max_transitions`, several instances, vacuity across instances, `each`/strong constraints and stutter-loop vacuity. The several-instances test is weak (FND-008). |
