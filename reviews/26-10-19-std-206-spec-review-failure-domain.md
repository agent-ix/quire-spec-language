---
id: SR-13270
title: "Spec review of PR #1038: failure-domain analysis"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@651a86903; git diff origin/main...HEAD (PR #1038)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-140
    type: reviews
---

## Summary

Ticket: STD-206. Unstated failure modes and identity confusion in the new FR-140 rules at 651a86903. Clean: the guard keeps the abstract `inc` from value 2 undefined-free; FR-139's sum bound covers the empty population.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-140 Inputs (two systems, over_model) | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md | examined |
| FR-140 relationship extent bullet | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md | examined |
| FR-140-AC-10 | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md | examined |
| FR-140 optional field bullet | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md | examined |
| FR-140-AC-11 | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md | examined |
| FR-140 UnmappedType object bullet | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md | examined |
| FR-140 refused/incomplete/OutOfType/Stopped bullet | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md | examined |
| FR-140-AC-8 | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md | examined |
| FR-139 sum bound bullet | spec/functional/FR-139-type-and-evaluate-population-valued-expressions.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | map_state 'takes the two systems as given and checks no more than that': swapped systems, or a system over a third selected model, give a silently wrong mapped key. over_model guards only against an unselected model. Both systems are over selected models, so passing the abstract system as concrete_system (or the reverse) evaluates rows over the wrong observation and keys the result with the wrong key function, with no failure. Each system knows its model, so map_state can cheaply check concrete_system against the refinement's concrete model and abstract_system against its abstract side and refuse with a typed cause. State it, with an AC. | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md:47-58 |
| FND-002 | medium | 'a link of the abstract model never holds in a mapped state' turns every abstract relationship into a silently empty extent; abstract initial states, frames and contracts that use links then refute or vacuously pass. ADR-020 and QSpec FR-375/377 are silent. If the abstract model declares a relationship and its initial snapshot or an operation's postcondition has a link, RS-2/RS-4 compare against mapped states that can never contain it, giving spurious refutations. A precondition that needs a link is never enabled, giving vacuous passes. AC-10 codifies the case as passing. Have S3 (FR-135) refuse a refinement whose abstract model declares a relationship, as unsupported_construct/not-yet-implemented with a support-set feature, or define link mapping in ADR-020. Add the QSpec ask. | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md:136-138 |
| FND-003 | medium | The optional-field rule maps None to 'unset' but QSpec separates absent from explicit null (FR-149 presence row); a nullable optional field's mapping is unstated. For a field declared optional and nullable, say whether the row's `none` and `null` map to absent and null, and whether a required-but-nullable field takes Option<T> rows. Otherwise two builds can disagree on the mapped state key. | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md:131-135 |
| FND-004 | medium | With the new Object-row failure there are now several failures per state, but FR-140 does not say which one map_state returns, or whether an Undefined row outranks an Undetermined one (refuted vs inconclusive). A state with a Stray object and an undefined `only` row could return Undetermined(UnmappedType) or Undefined. One makes the state undetermined and the other refutes it. FR-140's use case ('a verdict and its replay never disagree') needs a fixed order: object rows before field rows, rows in declaration order, objects in key order, and Undefined vs Undetermined precedence. | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md:125-130 |

## Verdict

Findings: 4 medium.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | The first-Undefined-wins order is sound and fixes replay, but it keeps evaluating after a Stopped (broken invariant), QSpec FR-377 states no precedence and STD-211 has no ask, and AC-9's 'original items row' case names no state. An undefined row refutes a reachable state whatever other rows give, so letting Undefined outrank Undetermined is sound. Two things are left open: whether rows are still evaluated after an evaluator Stopped (an internal fault), and the QSpec FR-377 counterpart. FR-140-AC-9's last clause needs its state (head, size) for the original row to be defined. | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md:155-163 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | The precedence bullet ('SHALL return the first Undefined ... if any row is undefined') and the new Stopped bullet ('at once ... even when an earlier row was undefined') are two SHALLs that contradict each other. Returning Undetermined on a stop is sound: a stop means the evaluator's invariants broke, so an Undefined seen earlier over the same observation is no more trustworthy than the rows after it, and dropping it costs completeness only, never a false refutation. But the precedence bullet must carry the exception, for example: 'otherwise the first Undetermined; a Stopped returns at once and outranks an earlier Undefined'. Otherwise one build returns the earlier Undefined and another the Stopped. | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md:160-168 |
| FND-007 | medium | A `null` row value is reported as EvalFailure::Stopped, the broken-invariant arm, so under the new stop rule an unsupported but trustworthy value cuts evaluation short and drops a genuine earlier refutation. A null row is not an internal fault: the observation is sound and only the mapping rule is missing. Reporting it as Stopped (whose catalog code is otherwise a broken invariant) makes the short-circuit discard an Undefined that should refute. Use EvalFailure::Refused with an unsupported_construct/not-yet-implemented refusal record, which follows normal precedence, or add a dedicated arm. | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md:147-151 |

## Dispositions

Round 1, head 72d04b1dc (fix commits d561b72e7, 72d04b1dc).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | still-open | The rule now separates `T?` (absent on None) from `Option<T>` (explicit None), but a nullable field and a `null` row value (the grammar has `null` as a primary, distinct from `none` under QSpec FR-149) are still unmapped; one sentence closes it. |
| FND-004 | fixed | - |
| FND-003 | fixed | - |
| FND-005 | fixed | - |

Disposition pass 1 (head 72d04b1dc): FND-001, FND-002 and FND-004 fixed; FND-003 still open (nullable fields). New: FND-005 (low). Not mergeable while FND-003 is open.

Round 2, head f0b08808e (fix commit f0b08808e): rows after round 1's.

Disposition pass 2 (head f0b08808e): FND-003 and FND-005 fixed. New: FND-006 (medium), FND-007 (medium). Not mergeable.
