---
id: TC-123
title: "Activate temporal obligations and retain immutable captures"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-044
    type: verifies
---
# TC-123: Activate temporal obligations and retain immutable captures

## Description

Public Rust API controls for
[FR-044](../functional/FR-044-activate-temporal-obligations.md). Each case
compiles a real native temporal declaration with a trigger binder, an activation
guard and ordered captures, then drives activation with independently constructed
trigger observations. Expected instance identities, capture values, dispositions
and refusal causes come from the authored source and the supplied observations,
never from the evaluator's own output. Each numbered group corresponds to the
matching acceptance criterion.

Controls execute in `tests/composed_temporal_activation.rs`. The timed-refund
shape from the shared specification is the primary declaration: a failed order
triggers an obligation that captures the paid amount and the payment reference.
Capture immutability is established by Rust ownership — a retained capture record
is not reachable for mutation through the public API — so group 3 pairs a
compile-fail control with the observation-mutation control rather than claiming a
runtime mutation attempt succeeded in reaching the record.

## Test Procedure

1. Deliver two distinct semantic failed-order triggers whose captured amounts
   compare equal, and require two obligation instances with distinct identities.
   Assert each instance's proposition against the other's captured payment
   reference and require no satisfaction; require the reported mismatch to name
   both instance identities rather than the equal amount.
2. Deliver one semantic trigger twice under different receipt identities. Require
   exactly one instance, require the second receipt's provenance retained on that
   instance and separately inspectable from the first, and require the instance's
   capture values unchanged. Then deliver the same semantic trigger identity twice
   with conflicting payloads and require a typed contradiction refusal naming that
   trigger, with no aliasing and no replaced capture.
3. Activate one instance, then mutate the source observation's amount, its
   reference and the predicate valuation environment it was read from. Re-read the
   instance's captures and require the originally captured expression handle,
   type, value, source binding and anchor byte-identical to the activation-time
   record. Pair this with a compile-fail doctest that pins its expected
   diagnostic — the borrow checker's immutable-borrow error naming the retained
   capture accessor — so the control cannot pass on an unrelated compile error.
4. Independently supply a missing capture input, a null-valued input, a
   wrong-typed input, a stale input bound to an earlier anchor, and an input whose
   anchor does not match the authored activation anchor. Require each to report an
   incomplete or refused activation naming that capture, and require reported
   `positions` usage zero for the affected instance. Require an unrelated healthy
   instance in the same evaluation to retain its own activation and its own
   captures.
5. Drive four trigger-scope shapes against one declaration: closed and complete
   with no admitted trigger; open with no admitted trigger; trigger evidence
   missing; and trigger evidence refused. Require `inactive` only for the first,
   `unknown` with open completeness for the second, and `unknown` carrying the
   distinct incomplete and refused execution dispositions for the third and
   fourth. Require every one of the four to carry no temporal truth and to
   evaluate no position, and require the `inactive` result to make no claim that
   recovery behavior was exercised.
6. Author three captures whose initializers read distinct observation fields, and
   require exactly one evaluation each, in authored source order, at the
   activation anchor, by inspecting the per-instance evaluation counter the
   evaluator reports. Independently author a capture whose initializer reads a
   later capture's name and require a refusal naming the forward read. Re-evaluate
   the same instance incrementally and under an admitted restoration, and require
   the per-instance evaluation counter to stay one.
7. Supply a trigger whose activation guard evaluates false and require no
   instance and no refusal. Supply a closed-complete scope whose every admitted
   trigger is guard-false and require `inactive`; supply an open scope in the same
   shape and require `unknown` with open completeness, establishing that
   guard-false is not a disposition of its own. Supply a guard whose input is
   missing and a guard whose input is wrongly typed, and require incomplete and
   refused activation respectively, each naming the guard, with neither treated as
   a false guard.
8. Compile a declaration activating on a whole-execution origin and drive it with
   one admitted execution origin; require exactly one instance keyed by the clause
   subject and that origin identity. Deliver the same origin identity twice with
   conflicting payloads and require a typed contradiction refusal. Drive it with
   two admitted executions and require two instances that do not alias and whose captures do not satisfy each
   other. Re-evaluate one execution incrementally and require its instance count
   to stay one.
9. Activate an instance that remains unsettled, inject the eviction of one
   retained capture through the trace's eviction list, and require an
   incomplete result naming the evicted capture. Require that no capture
   initializer is re-evaluated at the later anchor during that incremental
   re-evaluation, during an admitted restoration, and during replay, by inspecting
   the per-instance evaluation counter.

## Expected Results

Instance identity follows the admitted semantic trigger or execution origin,
never a timestamp, display name, receipt or equal payload. Captures are evaluated
once, at the authored anchor, in source order, and are neither replaced nor
recomputed afterwards. `inactive`, `unknown`, incomplete and refused remain four
distinct activation outcomes, and none of them is a temporal truth. Guard-false
is not a fifth: it creates no instance, so the scope's disposition follows from
whatever remains.

Temporal truth itself is asserted in
[TC-122](./TC-122-evaluate-bounded-native-temporal.md), resource behavior in
[TC-124](./TC-124-bound-temporal-evaluation.md). Observation transport, receipt
deduplication in the wire layer, and protocol participation results are owned
elsewhere and are not claimed here; a receipt identity is an opaque supplied
value in every group above.
