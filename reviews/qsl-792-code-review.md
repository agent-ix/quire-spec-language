---
id: SR-12051
title: "QSL-792 PR #925 code review: type-level invariant clauses (FR-366)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@ed3f27d1c4aeaa45ac3c5900387e50fe94bd59e9; qsl-eval/src/value/expression/value_invariants.rs, qsl-eval/src/value/expression/mod.rs, qsl-semantics/src/check/type_invariant.rs, qsl-semantics/src/check/assemble.rs, qsl-semantics/src/check/mod.rs, qsl-semantics/src/check/state_clause.rs, qsl-semantics/src/check/refusal.rs, qsl-semantics/src/model/type_invariant.rs, qsl-semantics/src/model/intake/unit.rs, qsl-semantics/src/model/object_environment.rs, qsl-semantics/src/model/observation/document.rs, qsl-replay/src/spine.rs, qsl-replay/src/spine/lifecycle.rs, qsl-eval/tests/it/type_invariants.rs, xtask/src/definition_scan.rs"
review_set: subset
---

## Summary

Ticket: QSL-792. PR quire-spec-language#925. Code review with the rust-review lane over `git diff origin/main...HEAD` (30 files, +2602/-25). Read only; no build was run. Author-recorded deferrals (FR-106-AC-48 ValueInvariants admission, the state-machine half of FR-366-AC-9, the TC-1051 step 6 fixture, step 1 history ordering, the xtask guard edit) are not re-reported.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A record or event invariant that reads through a reference (attribute read, `deref`, navigation, population quantifier) is admitted by the checker, but `evaluate` runs it over an empty `ObjectEnvironment`, so it returns `CallFailure::Fault` (`attribute-slot-unresolved` or `linked-type-invariant-completed-no-boolean`) where FR-366 requires a linked invariant to complete `true` or `false` | qsl-eval/src/value/expression/value_invariants.rs:225, qsl-eval/src/value/expression/value_invariants.rs:316-343, qsl-eval/src/value/expression/evaluate.rs:1603 |
| FND-002 | medium | `CheckCause::TypeInvariant` hides the inner cause from the spine's `limit_of` and `fault_of`, which match `refusal.cause` structurally. A stage limit or internal fault raised while checking a type invariant is reported as `StageFailure::Refused` instead of `Limit` or `Fault` | qsl-replay/src/spine/lifecycle.rs:180-235, qsl-replay/src/spine/lifecycle.rs:275-320, qsl-semantics/src/check/type_invariant.rs:72-81 |
| FND-003 | medium | A type invariant's check refusal keeps `Location::root(Origin::Expression)`. `TypeDefinition.clauses[k]` and the clause key are only in the `TypeInvariant` side payload, so the native result reports origin `Expression` and the message names no clause. Kept operation clauses use a dedicated `Origin::ModelOperationClause` for this | qsl-semantics/src/check/type_invariant.rs:84-87, qsl-replay/src/spine/native_result.rs:1793 |

## Verdict

FIX. FND-001 is a fault on input the checker admits. FND-002 and FND-003 are the same wrapper problem seen at the driver. The rest is sound:

- `TypeClauseRoles::of` is order-independent: roles in a `BTreeMap`, own clauses sorted, ancestors collected in a `BTreeSet`.
- A guard is matched by `(machine key, member name)`, so it is in no effective list.
- `with_parsed_type_invariants` counts `k` over every clause of the owner, including unevaluated ones. The parse refusal is located at `TypeDefinition.clauses[k]`, which the driver test and TC-1050 step 8 confirm.
- `CheckCause::TypeInvariant` delegates `code`, `cause` and `catalog_code` to the inner cause.
- `self` is typed as a composite for record and event owners (through `ModelTypes.handles`) and as a reference for object, state machine and process owners.
- `evaluate` sorts by `(path, clause)`, uses the one meter, returns the results so far with `Incomplete` on a denied charge, and does not follow `Value::Reference`.
- `ObjectEnvironment::with_references` is set on the one snapshot path (`finish_populations`) that feeds `AdmittedObservations`.
- No new `unsafe`, unchecked casts or panics in production code. The collection index uses `u64::try_from`.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | Since #911, a `Context` selection admits `current` as the invocation's post snapshot, holding the state-machine context. `collect` walks every object in `current.environment.references()`, so a state-machine invariant is evaluated at `(observation, object ctx)`, with the object as subject, instead of only at the invocation's receiver binding. Its pre and post sides are never reached, because that branch needs `pre` and `post` | qsl-eval/src/value/expression/value_invariants.rs:112-130, qsl-semantics/src/model/observation.rs:2815-2831 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | still-open | The denylist `check_type_invariant` added in ab8b210e3 (state_clause.rs:292-301) covers `Attribute`, `Navigate`, `AllInstances` and `Lookup`. It misses `Reaches` and `ReachesEnd`, which are relationship reads that the amended FR-366 rule names and that read the empty environment at evaluate. It also misses a `Call` whose callee reads objects through a passed reference. Any of these, checked inside a record or event invariant, still ends in `CallFailure::Fault` |
| FND-002 | fixed ab8b210e3 | `CheckCause::unwrapped()` is used by `limit_of`, `fault_of` and `CompileRefusal::fields`; tested by `a_limit_or_fault_inside_a_type_invariant_is_read_through_its_wrapper` |
| FND-003 | fixed ab8b210e3 | `CompileRefusal::fields` adds `clause` and `member`, which reach `RefusalWire.fields` in the native result, and `check_message` names the invariant and `TypeDefinition.clauses[k]`. The origin stays `Expression` because `Origin` is owned by quire-semantic-value; this is recorded as a gap |
