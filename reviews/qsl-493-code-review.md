---
id: SR-12059
title: "QSL-493 PR #928 code review: ModelSystem over the FR-120 state seam"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@0001dc310de17b9d57f2d534bd3dd38aa070ebcc; qsl-eval/src/simulation/model.rs, qsl-eval/src/simulation/model/contracts.rs, qsl-eval/src/simulation/model/domain.rs, qsl-eval/src/simulation/model/encode.rs, qsl-eval/src/simulation/model/frames.rs, qsl-eval/src/simulation/model/walk.rs, qsl-eval/src/simulation/mod.rs, qsl-eval/tests/it/model_simulation.rs, qsl-eval/tests/it/state_model_family.rs, qsl-semantics/src/model/state.rs, qsl-semantics/src/model/observation.rs, qsl-semantics/src/model/observation/initial.rs, Cargo.lock"
review_set: subset
---

## Summary

Ticket: QSL-493. PR quire-spec-language#928 at head 0001dc310, diff `origin/main...HEAD` (14 files, +4267/-9). This is a code review with the rust-review lane folded in. Read-only: no build or test was run. As briefed, only major bugs are reported, and gates are not flagged: the author's `make ci` exited 0 on the f56ae2b9f merge, and the later main merge only touched `qsl-eval/tests/it/main.rs`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `results()` stops the expansion with `resource_exhausted` whenever the result domain is larger than the remaining budget plus one. It checks this before any pair is evaluated. But `complete()` returns at the first pair whose postcondition is true, and with no `post` clause that is the first pair. FR-120 counts only the pairs actually evaluated. So an operation such as `count(): Int[0, 100]` with no clause and `max_candidates` 50 stops the run, although the spec counts 2 per candidate and the expansion completes. The run reads as incomplete when it is exhaustive. | qsl-eval/src/simulation/model/frames.rs:713-735, qsl-eval/src/simulation/model/frames.rs:750-801 |
| FND-002 | medium | An argument vector is never charged when its effective precondition is `false` or undecided (`continue` at frames.rs:130 and :145). `classify_extent` accepts any `Int[lo, hi]` as bounded. So `setTo(n: Int[0, 10^15])` with `pre P { self.value = n }` runs 10^15 clause evaluations in one expansion, with no `max_candidates` stop and no poll. An undecided precondition also pushes one `ContractFinding` per vector, so memory grows without bound. FR-120 "Post-states" counts only candidates and pairs, so the spec has the same hole. ADR-016 G-4 means to bound one expansion's work. | qsl-eval/src/simulation/model/frames.rs:110-159 |
| FND-003 | medium | The new public `StateModel::field_is_granted` decides frame grants a second time, apart from `decide_frame`. ADR-016 FE-1 says `decide_frame` is the only function that decides whether a change stays inside a frame. The copies also differ. `field_is_granted` ORs `redefinition_reaches` over every selected view, unmetered. `decide_frame` walks only the population's own `frame.index`, through `redefinition_walk`. When they disagree, candidate construction builds a write that the decision refuses, and that surfaces as `runtime_invariant`/`established-invariant-broken` instead of a frame verdict. The predicate belongs on the same index walk `decide_frame` uses, or candidate construction should ask `decide_frame`. | qsl-semantics/src/model/state.rs:228-246, qsl-eval/src/simulation/model/frames.rs:428-434, qsl-semantics/src/model/population.rs:2097-2109 |

## Verdict

FIX. FND-001 gives a wrong outcome (`Stopped` where FR-120 gives `Exhaustive`). FND-002 lets one expansion of a model that passes the bound check run without limit. FND-003 is a second frame-grant decider that ADR-016 FE-1 rules out.

Checked and clean, with no finding:

- **Candidate generation is lazy.** Deletions, creations and field values are three odometers, and every value candidate is charged before it is built, including candidates the closure check drops. The AC-13 product is never materialized.
- **`decide_frame` is called per population** in ascending key order, with the created and deleted keys, the receiver, and the reference arguments scoped to that population. A refusal maps to `BROKEN`, as the spec says.
- **The hand-written `values_equal` callback** compares typed canonical `Enc` trees, and a value with no encoding equals nothing. That path cannot be reached: a state holding such a value already refuses `KeyEncoding` at its initial key.
- **Successor, finding and key order are deterministic.** All of it comes from BTreeMaps, ascending domain order, a sorted result domain, contract findings sorted by (transition bytes, candidate bytes, clause), and FR-101's own sort.
- **The `simulation-state` envelope** matches FR-181/FR-120: the population key as `text`, the reference triple, the record of fields in declaration order, and five empty maps.
- **Transition identity** keeps the receiver apart from the arguments and builds through QSV `ModelTransition::new`.
- **`sample_model`** replays the trace, then re-expands each step to get its effects.
- **Panics:** no `unwrap`, `expect`, `panic!` or unchecked index on a production path. Odometer indexing stays within its radices, and the casts widen.

On the new StateModel accessors (`populations`, `type_identity`, `type_key`, `is_abstract`, `fields`, `object_reference`, `field_is_granted`, `StateField`, `StatePopulation`):

- They expose only types that are already public (`DeclarationKey`, `EffectiveId`, `ObjectReference`, `ValueType`, `OperationEffect`) and leak no view internals.
- Apart from `field_is_granted` (FND-003), they do not duplicate the existing `population_index`/`ancestor_steps` surface.
- FR-120's seam listing does not state them; SR-12060 records that.

Cargo.lock, recorded as a note and not a finding:

- quire-semantic-value moved e622ca74 -> 74bd0afc42, which carries `ModelTransition`.
- quire-exact moved collaterally 526f5bd2 -> 21349174, because QSV follows quire-exact at branch=main.

The ticket's "Touches" list names qsl-eval and QSV but not qsl-semantics. That is a planner question.

The author-recorded gaps are planner questions and are not counted as defects here. They are: the AC-9 fixture, the AC-11 child process, the clause spellings, the `?` field as `{type: option}`, unenumerable roots listed as unbounded, the types with no `KeyEncoding`, and the `result_large_err` allows.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | high | Regression from the FND-003 fix. `writable_fields` probes every field of every object in the state, granted or not, and calls `Domain::of(...).ok_or_else(\|\| stop(BROKEN))?` before choosing a probe value. A field whose type is encodable but not enumerable (`Integer`, `String`, `Text`, `Rational`, `Float`, `Collection` without a bound), and that no frame grants, used to be kept as is. It now stops every expansion that has an enabled application with `runtime_invariant`/`established-invariant-broken`. Example: a `Counter` with `id: Integer` that no operation modifies. `domains()` lists only granted fields, so no `RequiresBound` refusal catches this first. The Counter fixture has only enumerable fields, so no test reaches the path. Fix: skip the probe when the field has no domain. A granted unenumerable field is already an unbounded root in `domains()`. | qsl-eval/src/simulation/model/frames.rs:663-718, qsl-eval/src/simulation/model/frames.rs:362 |
| FND-005 | medium | The state key has six members. QSpec FR-181 (current main) defines it as `{"type":"simulation-state","semantic":…,"control":…,"queues":…,"roles":…,"observations":…,"bounds":…,"memory":…}` and says that for a model subject `control`, `queues`, `roles`, `observations`, `bounds` and `memory` are empty maps. `model_key` writes no `memory` member. FR-120 "State, state key and anchor" and the AC-4 envelope list five empty maps and also omit it, so the key bytes and the golden test disagree with the normative source. Fix: add `("memory", empty_map())` and amend FR-120 and the AC-4 golden bytes. | qsl-eval/src/simulation/model/encode.rs:293-305, spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:186-192 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | c4ec15148 |
| FND-002 | fixed | c4ec15148 |
| FND-003 | fixed | c4ec15148 (the fix introduced FND-004) |
