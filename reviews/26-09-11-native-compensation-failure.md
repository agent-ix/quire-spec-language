---
id: SR-350
title: "Failure-domain review of compensation identity, capture, retry and recovery"
type: SpecReview
analysis: failure-domain
scope: "spec/functional/FR-042-publish-compiled-protocol-artifacts.md; spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md; docs/compiled-protocol-v1.md; src/protocol_artifact/native/runtime/compensations.rs; src/protocol_artifact/validate/control.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

Failure-domain recheck of the compensation obligation at the correction commit
`b7aafe1` against the reviewed `c7275f5`, across the same four declared phases —
registration identity, capture staging, bounded retry, recovery — plus the
payload-free effect requirement. Scope is the correction diff and the refusal
paths it changes. The two escapes recorded at `c7275f5` are closed and both were
reproduced red before the fix. The failure mode that replaces them points the
other way: the reader now reconstructs the expected recovery inventory itself, by
a different algorithm from the one that built it, so the residual risk is
refusing a genuine package rather than admitting a forged one.

## Verdict

**CONDITIONAL** — two mediums, three lows, no high. Effect-evidence
substitution and cross-obligation recovery substitution are both refused and
mutation-tested; the remaining exposure is a duplicated attribution rule and a
set of newly load-bearing refusals with no adverse case.

## Findings

| ID      | Severity | Summary                                                                                              | Refs                                                                                 | Escape Cause                        |
| ------- | -------- | ------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------- | ----------------------------------- |
| FND-001 | medium   | New failure mode: the reader recomputes the expected `recovery_bindings` set from the value graph and demands exact equality with an emitter set built by span containment, so a divergence refuses a genuine package | src/protocol_artifact/validate/control.rs:1011; src/protocol_artifact/native/runtime/compensations.rs:462 | missing-requirement                 |
| FND-002 | medium   | The clock's activation edge and temporal-profile authority, the snapshot's type/model and anchor back-reference, and the effect's retry-anchor checks are newly load-bearing with no adverse case | src/protocol_artifact/validate/control.rs:908; src/protocol_artifact/validate/control.rs:686 | correct-requirement-no-evidence     |
| FND-003 | low      | The signed-64 attempt ceiling is enforced only by the generic `Integer` decoder; no compensation-specific check or work dimension is charged against the value | src/protocol_artifact/validate.rs:143; src/protocol_artifact/native/runtime/compensations.rs:250 | missing-requirement                 |
| FND-004 | low      | Compensation registration/activation spans extend only forwards from the binder, so a capture authored before it would fall outside its phase scope (unchanged) | src/protocol_artifact/native/layout.rs:807                                               | missing-requirement                 |
| FND-005 | low      | Capture-stage confusion is caught by the type/scope stage, never by the emitter, so an emitter regression has no independent detector (unchanged) | tests/native_compensation_emission.rs:361                                                | correct-requirement-no-evidence     |

## Dispositions of the SR-350 findings recorded at c7275f5

| Prior   | Disposition                                                                                                                                   |
| ------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| FND-001 | **Resolved.** See "Identity" below. Reproduced red at `/tmp/quire-native-compensation-review-red.log`.                                          |
| FND-002 | **Resolved.** See "Recovery" below. The clock swap was reproduced red in the same log.                                                          |
| FND-003 | **Resolved by specification, with a residual.** `1..=9223372036854775807` is now the stated authored bound in FR-042 and the wire contract, and `compensation_attempt_bound_preserves_signed64_maximum_and_refuses_one_beyond` shows `i64::MAX` surviving source→emit→independent read and `9223372036854775808` refusing as `NumberError::ComponentOutOfRange { component: Decimal }`. Residual carried above as FND-003: the ceiling is the decoder's, not a compensation check, and no work dimension is charged against the value. |
| FND-004 | **Open**, unchanged. Carried above as FND-004.                                                                                                  |
| FND-005 | **Open**, unchanged. Carried above as FND-005.                                                                                                  |

## Failure domains examined

### Identity — which obligation an observation belongs to

Enforced in both lanes and tested. `validate.rs:620` now dispatches on
`kind == CompensationEffect` alone. `compensation_effect_identity` requires, in
this order: the binding's subject is a compensation of this declaration; that
compensation's `effect_instance` is *this* binding slot; the binding model is
exactly that compensation's `operation` and resolves as an `Operation` export;
the attempt binding shares the anchor, the subject and that same operation
model; the anchor is a `Retry` anchor owned by this compensation and pointing
back at the attempt binding; and the effect binding itself passes the full
compensation subject/anchor/`requires == [attempt_instance]`/ObservationBinding
contract check. Only after all of that does a non-null `value_type` return
`Unsupported::Export`.

That ordering is what makes the refusal honest. A foreign operation with a type
inserted still fails `Invalid::Binding` on the model comparison, never reaching
the unsupported result, and the test asserts both outcomes separately. No
operation result, attempt record or recovery view is invented as effect
evidence, and no typed observation is reinterpreted as untyped: the typed lane
is refused for a stated missing interface, not admitted under a weaker contract.

The chain is checked twice — once binding-first from `validate.rs:620`, once
obligation-first from `compensation_bindings`. The binding-first path is the
only one reached by a `CompensationEffect` binding that no compensation names,
so the duplication buys orphan coverage rather than being redundant.

### Capture staging — reading a value that does not exist yet

Enforced upstream. The four staging violations (registration capture reading the
recovery binder, guard reading the forward binder, retry reading the trigger,
recovery reading an attempt binder) are all refused at the scope/type stage with
`ScopeIssue::OutOfScope` on the exact name and span, and `native::admit` then
returns `Unsupported::FamilyProof` — static admission never sees them. That is
the right layering. FND-005 is the consequence: the emitter's own phase-scope
construction (`compensation_scopes`, layout.rs:799) has no adverse test of its
own, so a regression that widened a phase region would be caught only if it also
widened the checker's scopes, which it would not.

### Retry — bounded, and non-fabricating

`attempts` parses to `i64` with an explicit `NumericDomain` refusal, rejects
`<= 0` in the compiler and again at `validate.rs:143`; the interval refuses
`[31,30]`; `earlier`/`later` must be distinct binders of the declared attempt
record type, and the attempt-type mismatch refuses `Invalid::Type`. The retry
relation's arithmetic safety is a proof obligation, and the adverse case shows
`true and later.tally = earlier.tally + 1` refusing with an unproved
`CheckedRange` at the original span — retry cannot borrow runtime attempt
evidence to discharge itself.

The upper bound is now stated rather than open: `1..=9223372036854775807` is the
authored range in FR-042 and the wire contract, consumer execution budgets are
separated from it explicitly, and the attempt-bound test carries `i64::MAX`
through `admit` → `emit` → independent `read` with `read.package() == package`
and digest equality, comparing the emitted value against the original authored
token and its source slice. Resealing `9223372036854775808` refuses at the
decoder. No arbitrary feature ceiling and no numeric engine were added. FND-003
is what remains: the ceiling is a property of the shared `Integer` decoder, not
of any compensation check, so a future wider numeric representation would move
the bound silently, and nothing is charged against the value as work.

Note on evidence class: that test is not in the red log and is not a regression
test — `i64::MAX` was already accepted at `c7275f5`. It characterises the bound
and exercises an existing refusal path.

### Recovery — completeness premises and their owners

Emission is unchanged and exact. Reading is now exact too. `compensation_recovery`
(control.rs:898) pins, per obligation: the clock's compensation subject,
activation anchor, `requires == [activation]`, null type and model, and the
contract and authority of `definitions[value.profile]` — the same temporal
profile already family-checked by `compensations()`; the recovery binder's
anchor kind, owner and back-reference to the snapshot; the snapshot's subject,
anchor, activation prerequisite, ObservationBinding contract, and `value_type`
and `model` equal to the recovery binder's `Record`/`Object` export; exactly one
Progress and one Closure at the recovery anchor, each under the `Progress`
registered definition and each requiring the sorted clock/effect/snapshot
triple; and `recovery_bindings` equal, as a set, to those three plus the
declaration-owned population/closure pairs. Every one of those runtime
requirement records is additionally pinned to declaration scope index 0, which
`Runtime::add` assigns unconditionally, so the recovery binder's own lexical
scope cannot be substituted for it.

The previous FND-002 scenarios are all refused and asserted: the two clock
handles exchanged and `Full` taking `Partial`'s clock singly; `Full`'s
recovery inventory listing `Partial`'s Snapshot, Progress or Closure; each of
the seven prerequisite edges severed singly; each genuine `Full` population and
closure member removed; and the genuine `Partial`-only pair inserted sorted and
unique. Each mutation is preceded by an assertion that the original edge or
member exists exactly once, so no case rewrites nothing.

`models/populations.rs:138-242` still owns nominal pair content and set equality
against binders and anchored values; `compensation_recovery` adds only the
attribution of those pairs to a particular recovery, and no second closure
algorithm was introduced.

**FND-001 scenario — the new exposure.** The exactness is computed, not carried.
The emitter builds the inventory by walking `typed.nodes()` filtered by source-span
containment inside the `recover` expression regions, admitting a `Closure`
member whose model is non-null and whose single `requires` names a `Population`
binding. The reader rebuilds it by walking the validated `value_edges`
operand/initializer/origin graph, admitting a `Closure` member whose model's
export kind is `Population`, with no `requires` constraint. Consider a `recover`
predicate that reads a `let`-bound value whose typed node lies outside every
expression region the emitter walks but which is an operand edge in
`value_edges`: the reader collects that value's capture anchor, requires its
population pair, finds `recovery_bindings` one member short, and returns
`Invalid::Binding` on a package this compiler just emitted. The mirror case —
a `Closure` the emitter admits and the reader does not — produces a surplus
member and the same refusal.

Derived from the code, not executed: reproducing it needs a new source fixture
and a new test, and this recheck makes no code, spec or test edits. The single
authored `RecoveryFlow` fixture is the only shape under which the two
constructions are known to agree, and `recovery_bindings` exactness is not a
safety property that needs two independent derivations — one owner would remove
the mode entirely.

### What is correctly *not* claimed

Static admission fabricates no future observation anywhere in this increment:
the effect binding is an identity requirement with no payload, the activation is
an `Observation` requirement rather than an observed trigger, commit/never is
recorded without ordering, and progress/closure are Progress-contract
authorities rather than assertions of completion. The correction does not change
that: the typed effect lane is an explicit missing-interface refusal, and F still
owes the actual effect subject and its typed signal/value mapping separately.
The await test's five causal edges are branch alternatives, labelled as such in
the test source. Runtime
commit/recovery ordering remains the consumer's, consistent with the withdrawn
global one-compensator restriction and the withdrawn blanket
commit-before-recovery prohibition — multiple obligations per forward effect are
emitted and tested, and neither the compiler nor the reader re-imposes either
rule.
