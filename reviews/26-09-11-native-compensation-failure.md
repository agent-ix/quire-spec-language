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

Failure-domain analysis of the compensation obligation across its four declared
phases — registration identity, capture staging, bounded retry, recovery — and
of the narrow payload-free effect requirement. The interesting question for this
increment is which refusals are load-bearing: four of the emitted prerequisite
chains are enforced and mutation-tested, the recovery half is emitted but not
enforced, and the effect identity gate has one open bypass.

## Verdict

**FAIL** — FND-001 is high: the one guarantee this increment exists to make, that
no operation result, attempt record or recovery view can pass as effect
evidence, is enforced only when the effect's `type` is null.

## Findings

| ID      | Severity | Summary                                                                                              | Refs                                                                                 | Escape Cause                        |
| ------- | -------- | ------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------- | ----------------------------------- |
| FND-001 | high     | Effect-evidence substitution is refused only in the null-type lane; adding a value type admits any model on the effect binding | src/protocol_artifact/validate.rs:620; docs/compiled-protocol-v1.md:419                   | implementation-bug-despite-evidence |
| FND-002 | medium   | Cross-obligation recovery failure is undetected: snapshot, progress, closure and clock are accepted on kind alone | src/protocol_artifact/validate/control.rs:863; src/protocol_artifact/validate/control.rs:911 | correct-requirement-no-evidence     |
| FND-003 | medium   | `maximum_attempts` has a lower bound and no upper bound in either the compiler or the reader          | src/protocol_artifact/native/runtime/compensations.rs:250; src/protocol_artifact/validate.rs:143 | missing-requirement                 |
| FND-004 | low      | Compensation registration/activation spans extend only forwards from the binder, so a capture authored before it would fall outside its phase scope | src/protocol_artifact/native/layout.rs:807                                               | missing-requirement                 |
| FND-005 | low      | Capture-stage confusion is caught by the type/scope stage, never by the emitter, so an emitter regression has no independent detector | tests/native_compensation_emission.rs:361                                                | correct-requirement-no-evidence     |

## Failure domains examined

### Identity — which obligation an observation belongs to

Enforced and tested. Each obligation's registration, activation, attempt and
effect bindings carry `Subject::Compensation` with the compensation's own
handle, its own registration/activation/retry anchors with matching
`AnchorKind`, `owner` and `binding` back-references, and the registered
ObservationBinding contract resolved against the offered dependency
(`control.rs:693-838`). Two obligations sharing one forward effect keep disjoint
instance indices, asserted by dedup at
`tests/native_compensation_emission.rs:338`. Swapping subject, retry anchor or
attempt prerequisite between `Full` and `Partial` is refused `Invalid::Binding`;
a foreign declaration handle is refused `Invalid::Owner`. The one escape is
FND-001.

**FND-001 scenario.** Reseal the emitted `RecoveryFlow` package with
`bindings[Full.effect_instance].value_type = Some(full.attempt_type)` and
`.model = Some(<Node object export>)`. `compensation_effect_identity` is skipped
because it is gated on `value_type.is_none()`; the generic branch only requires
both fields non-null; `compensation_bindings` checks subject, anchor, `requires`
and contract but not the model. The package is accepted with the attempt
record's own model standing as the effect's authority — the exact substitution
`docs/compiled-protocol-v1.md:419` forbids. The suite's `wrong_export` case
proves the gate works in the null-type lane only.

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
evidence to discharge itself. FND-003 is the residual: nothing caps
`maximum_attempts` above, so an artifact may declare `attempts 9223372036854775807`.
Bounded-by-declaration is arguably correct here since the runtime consumer owns
enforcement, but FR-042 line 142 says "bounded retries" without saying bounded by
what, and no work dimension or limit is charged against the value.

### Recovery — completeness premises and their owners

Emission is exact: snapshot requires the activation; progress and closure each
require (clock, effect, snapshot), sorted; the authored `recover` predicate's
population/closure pairs are attached at their original recovery and capture
origins, and the N=5 exact-sum full recovery and the `let`-bound partial
recovery keep distinct value roots. Reading is not. `recovery_bindings` accepts
any binding of six kinds (control.rs:911) and the compensation clock only has to
be of kind `Clock` (control.rs:863).

**FND-002 scenario.** Reseal with `Full.recovery_bindings` listing `Partial`'s
snapshot index, or with `Full`'s snapshot `requires` emptied of the activation,
or with the two obligations' `clock` handles exchanged. Each is accepted. The
mitigation is partial and worth stating precisely: `models/populations.rs:138-242`
validates every Population/Closure binding globally — pairing, model/type/anchor
agreement and set-equality against the declaration's binders and anchored values
— so a *fabricated* population cannot enter through `recovery_bindings`, only a
mis-attributed Snapshot/Progress/Closure or a mis-attributed clock. Controls
already have the analogous check (`timeout_authority`, control.rs:641), which
makes the omission look like an oversight rather than a decision.

### What is correctly *not* claimed

Static admission fabricates no future observation anywhere in this increment:
the effect binding is an identity requirement with no payload, the activation is
an `Observation` requirement rather than an observed trigger, commit/never is
recorded without ordering, and progress/closure are Progress-contract
authorities rather than assertions of completion. The seventh test's five causal
edges are branch alternatives, labelled as such in the test source. Runtime
commit/recovery ordering remains the consumer's, consistent with the withdrawn
global one-compensator restriction and the withdrawn blanket
commit-before-recovery prohibition — multiple obligations per forward effect are
emitted and tested, and neither the compiler nor the reader re-imposes either
rule.
