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
and refusal causes come from the authored source and the supplied observations.
Each numbered group corresponds to the matching acceptance criterion.

Controls execute in `tests/composed_temporal_activation.rs`. The timed-refund
shape from the shared specification is the primary declaration: a failed order
triggers an obligation that captures the paid amount and the payment reference.

## Test Procedure

1. Deliver two distinct semantic failed-order triggers whose captured amounts
   compare equal, and require two obligation instances with distinct trigger
   identities. Assert each instance's proposition against the other's captured
   payment reference and require no satisfaction; require the reported mismatch to
   name both instance identities rather than the equal amount.
2. Deliver one semantic trigger twice under different receipt identities. Require
   exactly one instance, and require the second receipt's provenance to be retained
   on that instance and separately inspectable from the first. Require the
   instance's capture values to be unchanged by the second delivery.
3. Activate one instance, then mutate the source observation's amount, its
   reference and the predicate valuation environment it was read from. Re-read the
   instance's captures and require the originally captured expression handle, type,
   value, source binding and anchor to be byte-identical to the activation-time
   record.
4. Independently supply a missing capture input, a null-valued input, a
   wrong-typed input, a stale input bound to an earlier anchor, and an input whose
   anchor does not match the authored activation anchor. Require each to report an
   incomplete or refused activation naming that capture, and require that no
   temporal position is evaluated for the affected instance. Require an unrelated
   healthy instance in the same evaluation to retain its own activation.
5. Drive four trigger-scope shapes against one declaration: closed and complete
   with no admitted trigger; open with no admitted trigger; trigger evidence
   missing; and trigger evidence refused. Require `inactive` only for the first,
   `unknown` with open completeness for the second, and `unknown` carrying the
   distinct incomplete and refused execution dispositions for the third and
   fourth. Require every one of the four to carry no temporal truth, and require
   the `inactive` result to make no claim that recovery behavior was exercised.
6. Author three captures whose initializers have observable evaluation order and
   require exactly one evaluation each, in authored source order, at the
   activation anchor. Independently author a capture whose initializer reads a
   later capture's name and require a refusal naming the forward read. Independently
   author a capture initializer reading an ambient `self`, operation `result` or
   `pre(expr)` and require each to refuse.
7. Compile a source-valid past-time declaration and a source-valid timestamped
   declaration, and request a mapping the current bridge reports as unsupported.
   Require the refusal to retain the native declaration subject, the selected
   profile identity and the activation record, and require that no substitute
   profile, clock or reduced native form appears in the result or the retained
   source.

## Expected Results

Instance identity follows the semantic trigger, never a timestamp, display name,
receipt or equal payload. Captures are evaluated once, at the authored anchor, in
source order, and are immutable afterwards. `inactive`, `unknown`, incomplete and
refused activation remain four distinct dispositions, and none of them is a
temporal truth.

Temporal truth itself is asserted in
[TC-122](./TC-122-evaluate-bounded-native-temporal.md) and resource behavior in
[TC-124](./TC-124-bound-temporal-evaluation.md). Observation transport, receipt
deduplication in the wire layer and protocol participation results are owned
elsewhere and are not claimed here.
