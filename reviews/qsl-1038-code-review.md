---
id: SR-17009
title: "Code review of quire-spec-language PR #1439: EN-1 absent-key reads through the bound parameter and S3 admission of several covering populations"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@992556734f42370ee77d000f2fdac2152e55c725; PR #1439 (QSL-1038) diff origin/main...HEAD: qsl-eval/src/simulation/model/letters.rs, qsl-eval/src/value/expression/evaluate.rs, qsl-eval/src/value/expression/mod.rs, qsl-eval/src/value/mod.rs, qsl-semantics/src/check/mod.rs, qsl-semantics/src/check/state_clause.rs, qsl-semantics/src/check/temporal_clause.rs, qsl-semantics/src/check/temporal_clause/tests.rs, qsl-semantics/tests/it/{hyper_clauses,model_subject_bound,temporal_fairness,timed_intervals}.rs, qsl-analyze/tests/it/model_instances.rs"
review_set: subset
---
# Code review of quire-spec-language PR #1439

## Summary

Ticket: QSL-1038. Code review with the rust-review lane folded in. The root
crate was not built: origin/main does not compile there until PR #1438 lands.
The review covers qsl-semantics (with `test-support`), qsl-eval, qsl-analyze
and qsl-replay.

What changed:

- S3 no longer refuses `over (x: T)` when several populations cover `T`.
  `TemporalClaim::population()` is replaced by `populations()`. `requirements`
  takes `&[DomainKey]` and refuses (`population-domain-keyed`) unless it gets
  exactly one key per population. A new helper `populations_covering` lists
  the populations in `DeclarationKey` order.
- EN-1 `ClauseLetters`: at a key that holds no object conforming to the
  parameter's type, the atom goes through
  `evaluate_state_predicate_at_absent_key`. That entry seeds slot 0 with the
  parameter-typed reference under the key, and the `Machine` halts with
  `StateModelUndefined::AbsentKey` on any top-frame read of slot 0. The one
  exception is a read that is the direct reference operand of a `lookup`
  node, which is recorded by the operand's `Location` when the lookup is
  evaluated.

Consumers checked:

- The only caller of `TemporalClaim::population()` / `requirements` outside
  the tests is `PackageDeclarations` in `check/mod.rs`, and it is updated.
- The root crate and qsl-replay call neither.
- `Requirements` is a domain map, so package emit and the CG consumers take
  several `Population` domains without change.
- `TemporalCause::AmbiguousPopulation` is now never produced. qsl-replay's
  spine still formats it, and it compiles.
- Hyper clauses, operation contracts and state-clause reachability still use
  `population_of` and still refuse on ambiguity. FR-125 does not change them.

What counts as a read of `x`: every top-frame `Local(0)` evaluation that is
not a lookup's direct operand. That includes:

- a comparison (`x = r`)
- `contains(..., x)`
- navigation or an attribute (`x.age`)
- a function or imported-call argument, which is evaluated in the caller's
  frame
- a `let` initializer
- a quantifier or fold body, because binders bind in the same frame.

Halts are terminal, and no operator catches an undefined halt, so an atom
that reads `x` cannot settle `true` or `false` at an absent key. An atom with
no read of `x` evaluates normally. The one way a property can hold at such a
key is a false guard, which is what FR-125 prescribes. No wrong
proved/vacuous settlement was found.

Checks run by the reviewer at the head, in their own target directory, one
cargo job at a time:

- `cargo test --locked -p qsl-analyze -p qsl-eval`: pass.
- `cargo test --locked -p qsl-semantics --features test-support`: 763 lib,
  1198 it and 30 other tests pass.
- `cargo clippy --locked -p qsl-eval -p qsl-semantics -p qsl-analyze -p qsl-replay --all-targets --features qsl-semantics/test-support -- -D warnings`:
  clean.
- `cargo fmt --all --check`: clean.
- `cargo run -p xtask -- string-edge`: clean.

Mutations run by the reviewer:

- M1 (guard exemption removed) is killed by
  `a_key_holding_no_object_of_the_type_reads_absent_key`.
- M2 (every atom absent-key at an absent key, the old behaviour, so an atom
  that does not read `x` is no longer evaluated) is killed by the same test.
- M3 (the production record in `check/mod.rs` keys every population as the
  first one) survives all qsl-semantics tests. This is recorded in SR-17010
  FND-001.

## Verdict

**CONDITIONAL.** The S3 change and the EN-1 evaluation match the FR-125
Instancing ruling merged in #1435. These match:

- the guard exception
- "an atom that does not read `x` is evaluated normally"
- absent-key for every other read
- the guard's per-population falsity

The API break is confined to the crates the PR updates. There are no
high-severity findings.

FND-001 is a semantic gap that predates this PR. The ruling makes it more
visible. It needs a follow-up ticket or a spec clarification, not
necessarily a change here. The rest are low.

Mergeable once SR-17010 FND-001 (the untested production record wiring) is
fixed or explicitly deferred, and FND-001 here is ticketed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The requirement record has a population domain only for populations that cover `T` (member type `T` or a supertype). FR-125 Instancing, as now ruled, has instancing and reads use every population that overlaps `T`, including one whose member type is a proper subtype of `T`. An unbounded subtype population (no authored maximum) that contributes instances is therefore no domain of the clause's record. FR-336 says a temporal clause over a population with no declared maximum is `Unbounded` with that population's domain. This predates the PR (main used covering only), and the PR body states it. Fix: add overlapping unbounded populations to `populations()`, or have the spec owner rule that FR-336's "over a population" means covering only, and ticket it. | qsl-semantics/src/check/state_clause.rs:534-559; qsl-semantics/src/check/lowering/model.rs:710-731; qsl-semantics/src/check/temporal_clause.rs:933-941 |
| FND-002 | low | `TemporalCause::AmbiguousPopulation` is now unreachable. S3 never builds it, but its code and tag mapping in refusal.rs and its qsl-replay text (spine.rs:1210) remain. Dead refusal surface invites a reader to think S3 still refuses. Fix: remove the variant and the replay arm, or file a follow-up and mark the variant as retained for wire compatibility. | qsl-semantics/src/check/refusal.rs:1179; qsl-replay/src/spine.rs:1210 |
| FND-003 | low | `ClauseLetters::key_reference` adds a panic ("the parameter's type is an object type") on `type_key`/`object_reference` failure. `object_reference` refuses an empty key. The `# Panics` section of `ModelLetters::atom` does not list this case. Unreachable in practice (admitted keys are non-empty, and S3 guarantees an object type). Fix: name it in the `# Panics` doc. | qsl-eval/src/simulation/model/letters.rs:129-144; qsl-eval/src/simulation/model/letters.rs:147-152 |
| FND-004 | low | The ticket's item (1) is only partly delivered. The evaluator's `Attribute` arm still faults `attribute-slot-unresolved` for a reference to an object the state does not hold. `ClauseLetters` turns that into a panic ("the evaluator does not fault"). A claim's `x` no longer reaches the arm, so AC-12 is unaffected. Still open: whether any other dangling reference (a reference-typed field to a deleted object) is reachable. If it is, FR-125's "evaluate a holds atom over an object that does not exist at a position as FR-107 evaluates a reference to an absent object" is a panic rather than absent-key. Fix: a follow-up ticket that either shows deletion leaves no dangling reference or maps the arm to absent-key. | qsl-eval/src/value/expression/evaluate.rs:1864; qsl-eval/src/simulation/model/letters.rs:183 |
| FND-005 | low | "Reads x" is implemented as every top-frame use of slot 0 other than a lookup's direct operand. A guard written through a `let` alias, or inside a reusable predicate taking `x`, is therefore not the guard: the alias initializer or argument reads `x`, and the property refutes at a non-conforming key. FR-125 does not say whether such forms are reads, so another implementer could read them differently. Fix: the spec owner states it in FR-125 Instancing (suggested: any use of `x` other than as `lookup`'s literal reference operand is a read), and a test pins it. | qsl-eval/src/value/expression/evaluate.rs:1333-1364; spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:168-191 |
| FND-006 | low | The guard is recognized by the operand's source `Location`, inserted when the enclosing `Lookup` is evaluated, not by node identity. It is correct today: one source occurrence has one location, and only top-frame slot-0 reads are checked. It would silently exempt a non-guard read if lowering ever gave a synthesized `Local` the same location as a guard operand. Fix: key `guard_operands` on the checked node's id. | qsl-eval/src/value/expression/evaluate.rs:637-641; qsl-eval/src/value/expression/evaluate.rs:1333-1338 |

## Dispositions

Round 1. Reviewed at `9c2ae662e392b056a6d1d02acff55577ac1213f6`. Checks rerun:

- qsl-analyze, qsl-eval, qsl-replay and qsl-semantics (`test-support`) tests pass.
- clippy `-D warnings`, fmt and string-edge are clean.
- `cargo check --locked --workspace --all-targets` is clean on a local merge of the head with origin/main (c6e69e8c3, after #1438).
- M1 (guard exemption removed) and M2 (every atom absent-key) were rerun on this tree and both are killed.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | deferred | Filed as QSL-1051, which covers unbounded populations whose member type is a subtype of `T`. The `TemporalClaim::populations()` doc now states the gap. It names "a QSL-1038 review follow-up ticket" rather than QSL-1051, a non-blocking nit. |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | deferred | Filed as QSL-1052, for a dangling reference that faults `attribute-slot-unresolved` and becomes a panic. Accepted without a code comment, because a claim's `x` never reaches the arm. The PR body does not cite QSL-1052 at this head, a non-blocking nit. |
| FND-005 | deferred | This is a spec clarification for the FR-125 owner (whether a guard through a `let` alias or a helper is a read), outside this PR's code. The chosen reading is now documented at `guard_operands`. No ticket is filed yet, so the lead should file one against FR-125. |
| FND-006 | fixed | - |
