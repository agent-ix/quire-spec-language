---
id: SR-12181
title: "Code review (with Rust lane) of QSL-886 FR-125 instancing from the model subject's universes (PR #1019)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@7ff160b37351dece5d249aa5852ccf80c7568e7d; qsl-eval/src/simulation/model.rs, qsl-eval/src/model_subject.rs, qsl-replay/src/model_check/item.rs, qsl-replay/src/model_check/counterexample.rs, qsl-replay/src/certificate/closure.rs, qsl-analyze/src/model_check/check.rs, qsl-analyze/tests/it/{model_instances,model_check,counter,closure_certificate,main}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: reviews
---
# SR-12181: Code review (with Rust lane) of QSL-886 FR-125 instancing (PR #1019)

## Summary

Ticket: QSL-886. PR quire-spec-language#1019 at head 7ff160b37. Scope is
`git diff origin/main...HEAD`. The rust-review lane is folded into this file.
This was a static review. No cargo build or test ran, per the brief.
`make check-spec-validation` was run and passed.

What I checked:

- `ModelSystem::instances(over)`. Admission by member type `T`, subtype or
  supertype, through `TypeEnvironment::conforms` in both directions. One
  instance per key, whatever number of member types are comparable: the
  `any` over member types decides admission once per population, then each
  universe key is emitted once. Order: `sort_by` on the JCS declaration
  identity text, which is stable. It is also total, because
  `StateModel::populations` holds one entry per `DeclarationKey` and the
  text is injective over (package, node). Keys follow universe-list order,
  and `ModelSystem::new` refuses duplicate keys in a universe. Two
  populations that hold the same key give two distinct instances, as FR-125
  requires.
- RequiresBound for an admitting population with no universe. The root id
  is the sha256 of `declaration_enc(population)`, the same derivation
  `domains()` uses. `Population(None)` goes through `check_requires_bound`,
  the classifier the pre-check uses. It fires before any instance is listed,
  even when no initial state holds the population.
- NoInstance. A non-reference `over`, no admitting population, and admitting
  populations with only empty universes all give `Ok(vec![])`. `check_model`
  then returns `Undecided(NoInstance)` and `check_closure` returns
  `ClosureRefusal::NoInstance`.
- Order of pre-checks. `check_model`: form refusal, admission, the bounds
  pre-check, then instances (RequiresBound/Extent mapped through
  `not_simulated`, other `ItemRefusal`s as `Item`), then NoInitialState, then
  NoInstance. `check_closure`: admission, bounds, instances, NoInstance, then
  NoInitialState. Before this PR, closure instancing and NoInstance ran
  ahead of admission. No spec fixes that order, and moving it is what lets
  instancing read the universes. The NoInitialState/NoInstance order differs
  between engine and checker, but it did on main too.
- Removal of `ModelLetters::bindings` and `ModelCheckRefusal::TooManyBindings`.
  No code caller, doc or spec text refers to either now. The only mentions
  are historical SR files under `reviews/`. `ItemRefusal::TooManyBindings`
  still bounds the count at `MAX_ATOMS`.
- Rust lane: no new `unwrap`, `expect` or panic in production code, no
  `unsafe`, and no unchecked integer casts. Encode errors map to
  `NotSimulated::Extent(Fault)`. `ModelInstance` is a plain value type.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | An invalid universe key is reported as an internal fault. `ModelSystem::new` checks universe keys only for duplicates. An empty key `""` reaches `ModelSystem::bound_reference`, and `StateModel::object_reference` refuses it there (`ObjectId::new("")` is `EmptyObjectIdentity`, mapped to `invalid_runtime_input`/`invalid-value`). `bound_reference` turns that refusal into `None` with `.ok()`, and `ModelCheckItem::instances` turns `None` into `ItemRefusal::Fault("model-check-item", "over-type-declared")`. A caller's bad universe key (untrusted input, per the SR-750 note on `object_reference`) is then reported as a broken QSL invariant, in both `check_model` and `check_closure`. Fix: return the `AdmissionFailure` (or refuse empty keys in `ModelSystem::new`) and keep `Fault` for an undeclared `over` type only. No test covers this case. | qsl-eval/src/simulation/model.rs:776-783; qsl-replay/src/model_check/item.rs:120-131 |
| FND-002 | low | The module doc of `item.rs` still says each instance "has its own key context and its own translation". That is false where two admitting populations share a key, which FR-125's new Remaining-work bullet admits. The context binds `bound_reference(over, key)`, which does not depend on the population. The edited line is also not rewrapped (about 100 columns). | qsl-replay/src/model_check/item.rs:6-10 |
| FND-003 | low | `Check::atom` in the closure checker still names its parameter `object`, and its doc says "for `object`", but the value is now a `ModelInstance` (population and key, no object). | qsl-replay/src/certificate/closure.rs:541-547 |

## Verdict

Mergeable. No high finding. The instancing logic matches FR-125: admission
in both conformance directions, one instance per key, a stable and total
canonical order, RequiresBound before instancing and NoInstance from real
universes. The pre-check order is sound, and nothing stale is left from
the removed `bindings` and `TooManyBindings`. FND-001 is a real
misclassification on an untrusted-input edge and should be fixed in this
PR. FND-002 and FND-003 are wording.

Declared and not raised: claims over a subtype of a mixed population are
refuted at non-conforming keys until STD-196 rules. The FR-338 key context
binds the typed reference of the `over` type. For TP-1, a shared key context
across populations does not make the closure check unsound: the automaton
states of a holding run are the same whatever the instance, and a shared
context can at worst make the checker reject. AC-7, AC-9, AC-11 and AC-12
are untagged. #1017 overlaps this PR.

## Dispositions

Round 1, reviewed at d38c0c83b. The fix round is merge commit 2e6feb746
(main plus #1017, and the fixes), followed by d38c0c83b. The #1017
behaviour is kept. `ModelCheckItem::instances(&subject, &system, cancel)`
still resolves `ClauseIdentity` from the subject's package. A cancelled
handle still gives `Stopped{Cancelled, None}` in `check_model` and
`Stopped(Cancelled)` in `check_closure`. The resolution now runs after
admission and the bounds pre-check, and the #1017 tests are unchanged
apart from the `Letters` constructor. No conflict markers remain, and
`make check-spec-validation` passes.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 2e6feb746: `ModelSystem::new` refuses an empty universe key `invalid_runtime_input`/`invalid-value` (FR-120 check 1 updated). `bound_reference` returns `Result<_, InternalFault>`, not `.ok()`. `an_empty_universe_key_is_refused_as_invalid_input_not_a_fault` covers `check_model` and `check_closure`. The remaining `ItemRefusal::Fault` path (no selected model declares `over`, or an admitted key has no reference) is unreachable through the public API: S3 resolves `over` to a model type, and admission refuses the only key `object_reference` refuses. So it stays untested, which is accepted. |
| FND-002 | fixed | 2e6feb746: the `item.rs` module doc now says two admitting populations that hold the same key give two instances with one key context, and it is rewrapped. |
| FND-003 | fixed | 2e6feb746: the `Check::atom` parameter is `instance: &ModelInstance`, its doc says "the bound `instance`", and the caller's local is `bound`. |
