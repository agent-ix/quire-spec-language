---
id: SR-16100
title: "Spec review — quire-eval clause entry and clause environment contract (FR-406)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-406-evaluate-clause-families-through-a-quire-eval-clause-entry.md, spec/test-cases/TC-1180-the-quire-eval-clause-entry-matches-the-std-reference.md, spec/functional/FR-403-implement-the-no-std-evaluator-in-quire-eval.md, spec/functional/FR-405-retire-the-std-reference-evaluator-for-quire-eval.md, spec/test-cases/TC-1121-the-no-std-evaluator-in-quire-eval-matches-the-std-reference.md, spec/spec.md at 33c1a84"
review_set: subset
evaluated_revision: "33c1a84714c0300c4cec6ea97ec8494c4afec433"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-406
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1180
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-403
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-405
    type: reviews
---

## Summary

Ticket: STD-318 (blocks IR-593 S9 and IR-584). PR quire-spec-language#1277
adds FR-406 (the `quire-eval` clause entry, the C1-C7 clause environment
contract, the 11-event trail contract, the SW-13 separation orchestration,
cause placement, parity and the thumbv7em size budget) and TC-1180. It also
edits the FR-403 scope text, cause row and AC-9, TC-1121 step 9, the FR-405
prerequisite paragraph and its dependency edge, and the spec.md index.
Reviewed `git diff origin/main...HEAD` at 33c1a84, which has main as an
ancestor. The review checked the diff against FR-107, FR-265, FR-268,
FR-366, FR-391, ADR-031 SW-2..SW-13, FR-400..FR-405 and NFR-015, and against
the reference evaluator in QSL main: `qsl-eval/src/value/expression/evaluate.rs`
(`Machine`, `separate`, `named_all_instances`, `named_lookup`,
`population_objects`, `objects_for`, the receiver swap, `read_path`,
`log_membership_of`), `mod.rs` (`evaluate`, `evaluate_holds`,
`check_separation`, `evaluate_*`), `value_invariants.rs`,
`s6a/protocol_clause.rs`, `causes.rs`, and quire-exact `collection::form`.

Examined units: the FR-406 statement and each Behavior section (One
evaluator two entries, Clause input, the per-node-kind table, the C1-C7
contract, trail events, separation check, causes, what stays in qsl-eval,
parity, size), FR-406-AC-1 to AC-14, the TC-1180 procedure and expected
results 1 to 14, and the FR-403, FR-405 and TC-1121 edits.

The following checks found nothing wrong:
- The corpus claim holds. FR-402 lines 149-154 say a version-1 case carries
  no object environment or domain package, and a non-null
  `object_environment` refuses. Line 252 says it encodes only Boolean and
  Integer results, and every checked-in case is a `function` call. A clause
  input therefore cannot be a corpus case.
- The regime assignment matches the reference's constructors.
  `evaluate_closed` and `evaluate_reward` use `Machine::new`. The
  terminal-predicate, refinement-row, history-update, state-predicate,
  protocol-slot, value-invariant and separation paths use `with_pre`.
- The FR-107 `model.deref`/`model.navigate` charge rule, the `WrongAnchor`
  refusals, the `ProtocolClauseUnsupported` refusal in the Clause regime, the
  `HistoryUnresolved` and `NamedPopulationIncomplete` codes and fields, and
  the `population-members-unobserved` fault all match the reference.
- The cause placement matches `causes.rs`. `ProtocolClauseFrameRefusal`
  carries `AdmissionRecord`, and `ModelQueryRefusal` wraps `ModelRefusal`.
- Frame check 11 evaluates no checked term (`protocol_clause.rs`).
- The FR-403, FR-405 and TC-1121 edits agree with FR-406 and with FR-405's
  three constraints.
- Wording is behaviour-only apart from FND-009 and the change-description
  obligation noted in FND-005, which follows FR-405's precedent.
- The PR body flags lead defaults 1-7 as "not owner answers".
- FR-406 and TC-1180 appear in no other open PR head (all 24 checked) and
  in no file on main.
- `make check-spec-validation` exits 0 with no diagnostic on a touched file.

## Verdict

NOT MERGEABLE as written: two high and four medium findings. The temporal
`holds`-atom path (`evaluate_holds`, FR-327) is missing from FR-406's
regime list, its description and AC-11. It runs on `Machine::new` today and
can read a named population, so the natural IR-593 routing to the
`Value`-family entry would fault. FR-406-AC-5 asserts one `collection.bound`
for `allInstances<T>(M::P)`. The reference admits two, because it charges
one explicitly and `form` charges another, so the AC fails against the
reference. The NamedPopulation row's charge text has the same gap. The
separation-check section leaves recorder continuity and let-step
attribution implicit. The 2% size budget is a one-time description check,
and part of it cannot be met. The body-form graph rule is ambiguous. Apart
from these findings, IR-593 can be built from FR-406 without guessing.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-406 never assigns the FR-327 temporal `holds`-atom evaluation (`CheckedPackageEvaluation::evaluate_holds`, a node plus slots run on `Machine::new`) to an entry or regime. It is absent from the Description's list, the Value-regime list and AC-11. Its input is neither a callable application nor a checked expression, and a holds atom can name a population (`NamedPopulationIncomplete` doc; `named_population.rs` `a_holds_atom_naming_a_population_is_emitted`). Routing it to the `Value`-family entry would fault `outside-value-family`. Fix: assign it the body form in the `Value` regime, and add its tests (FR-327, FR-391 holds atoms) to AC-11. | spec/functional/FR-406-evaluate-clause-families-through-a-quire-eval-clause-entry.md:51-58,140-146,387 |
| FND-002 | medium | The NamedPopulation row gives incomplete or wrong charges and leaves the check order open. (1) `allInstances`: after the visits and the explicit `collection.bound`, the reference forms the result with `quire_exact::form`, which charges set coalescing, a second `collection.bound` and `collection.result-retain`. The row states only the first two. (2) `lookup`: "result-retain sized ... 1 otherwise" is wrong for an absent key under `undefined` or `refused`, which admit no retain charge. (3) Order: the coverage check and the type-mismatch refusal come before `lookup.key` and before the `complete` check, and the `complete` check comes after `lookup.key`. The row does not say this, yet the result for an incomplete population with a non-conforming `r` depends on it. | spec/functional/FR-406-evaluate-clause-families-through-a-quire-eval-clause-entry.md:161 |
| FND-003 | high | AC-5 asserts `allInstances<T>(M::P)` admits "three `population.visit` charges and one `collection.bound` sized 2". The reference (`named_all_instances`, evaluate.rs:2414-2425) charges `collection.bound` sized 2 explicitly and then calls `form(result_type, selected, meter)`. `form` charges coalescing comparisons, a second `collection.bound` sized 2 and `collection.result-retain`. The AC as written fails against the reference and contradicts its own "charge sequence equals the reference's". Fix: state the full sequence, or assert only equality with the reference plus the visit count. | spec/functional/FR-406-evaluate-clause-families-through-a-quire-eval-clause-entry.md:381 |
| FND-004 | medium | The separation-check steps leave two things to guess. (1) The reference (`Machine::separate`) keeps one `Trail` across the let, domain and body evaluations, so a domain bound by a `let` keeps its provenance (Arc identity). FR-406 says "with ... a recorder" but not that the same recorder carries across the Begins. A fresh recorder per Begin would lose the let-bound domain's provenance. (2) A failed let Begin is "the result FR-268 states for that step", but FR-268 has no let step. The reference returns `Separation::Stopped(SeparationStep::Domain, _)`. State that let Begins belong to step 2 and share one recorder. | spec/functional/FR-406-evaluate-clause-families-through-a-quire-eval-clause-entry.md:265-281 |
| FND-005 | medium | The 2% size budget cannot be fully met and does not act as a gate. (1) AC-14 and the Size section require both probe sizes at the change's base, but `probe-clause` cannot build at a base that has no clause entry. If the change also adds the probes, `probe-value` does not exist at the base either, and the spec does not say what to build there. (2) The gate only reports sizes. The 2% limit is checked once, by inspecting one change's description, so a later change can grow `probe-value` with no check. (3) The build profile (opt-level, LTO, codegen-units, panic strategy) and the measuring tool are unstated, so the numbers are not reproducible. Fix: make the gate compare `probe-value` against a stated reference build and fail above 2%, drop `probe-clause` at base, and fix the profile. | spec/functional/FR-406-evaluate-clause-families-through-a-quire-eval-clause-entry.md:349-366,390 |
| FND-006 | medium | The body form requires "a node of the registry's closure graph". FR-403's closure includes the imported graphs its scope view holds (FR-403 "The checked closure" item 2), yet AC-12 requires an imported library graph's node to fault `clause-root-outside-closure`. As written, a node of an imported graph is inside the closure, so the rule and AC-12 disagree. Fix: say the node must belong to the closure's own (root) graph, not an imported one. | spec/functional/FR-406-evaluate-clause-families-through-a-quire-eval-clause-entry.md:128-132,388 |
| FND-007 | low | C7 timing: the reference records a membership read before the query's charges for `Value`-regime `allInstances`/`lookup` and `PopulationObjects` (`log_membership_read` before `evaluate_all_instances`, `log_membership_of` before `form`). It records the read after the visits for named `allInstances`, and after `lookup.key` for named `lookup`. "After every population or membership read" does not fix the timing, so under a meter denial the reported reads can differ from the reference's. | spec/functional/FR-406-evaluate-clause-families-through-a-quire-eval-clause-entry.md:165-169 |
| FND-008 | low | The reference switches a `pre` receiver read to the pre-side reference only when a trail is attached (evaluate.rs:1795-1808). The terminal-predicate, refinement-row, history-update, state-predicate and protocol-slot paths run `with_pre` with no trail, so they do not switch. FR-406 ties the switch to the optional receiver correspondence but does not say which callers supply it. State that only the trail-carrying paths (state clause, clause set, separation) pass one. | spec/functional/FR-406-evaluate-clause-families-through-a-quire-eval-clause-entry.md:85-87,157 |
| FND-009 | low | Change-relative wording in the criteria: AC-1 "still meets FR-403-AC-1" and AC-2 "still returns". State the property directly. | spec/functional/FR-406-evaluate-clause-families-through-a-quire-eval-clause-entry.md:377-378 |
