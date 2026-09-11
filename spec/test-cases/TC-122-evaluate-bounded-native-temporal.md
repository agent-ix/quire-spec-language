---
id: TC-122
title: "Evaluate bounded native temporal obligations under each selected profile"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-043
    type: verifies
---
# TC-122: Evaluate bounded native temporal obligations under each selected profile

## Description

Public Rust API controls for
[FR-043](../functional/FR-043-evaluate-bounded-native-temporal.md). Each case
compiles real native temporal source through the existing parser, linker, checker
and protocol-artifact emitter, then evaluates the emitted temporal body against an
independently constructed observation trace. Expected truths, bases, premises and
refusal causes come from the authored source and the profile definitions in
`resources/native-v1/proposals/quire-v1/definitions/`, never from the evaluator's
own output. Each numbered group corresponds to the matching acceptance criterion.

Controls execute in `tests/composed_temporal_evaluation.rs`. Traces are Rust
values; no JSON fixture, shell step or installed CLI participates.

## Test Procedure

1. Compile `always[0,1] holds(p)` and `always[0,1] true` from one unit under
   `quire.temporal.event-position.false-extension/v1`, then under
   `quire.temporal.fixed-sample.false-extension/v1`. Supply one closed, complete
   decision scope holding exactly position 0 with `p` true. Require the `holds`
   form false and the constant form true in both profiles, with the false result's
   decision support naming the out-of-scope offset 1. Compare the two emitted
   operation graphs and require the distinction to survive in the evaluated
   result, not only in the graph.
2. Evaluate one authored `always[0,1] holds(p)` against three traces that agree on
   every `holds` valuation and on the numeric interval, and differ only in the
   selected profile: an event-position sequence of one admitted position, a
   fixed-sample trace of one required sample, and a complete finite window whose
   only admitted instant is the anchor. Require three results retaining three
   distinct profile identities and clock premises, with the finite-window result
   true and both false-extension results false. Require no shared cached result
   between the three.
3. Independently mutate each trace dimension against a fixed declaration: profile
   discriminant, clock identity, declared sample period, epoch, timestamp unit and
   sequence authority. Require a refusal that names the affected clause and the
   mutated dimension before any position is visited, and require no default,
   nearest-compatible or backend-suggested selection to appear in the result.
4. Exercise all eight bounded operators over one authored trace: `eventually`,
   `always`, `until`, `release`, `once`, `historically`, `since` and `triggered`.
   Include the zero-width `[k,k]` interval for a unary and a binary operator, and
   `once[1,1]` as strong previous. Require a zero-width interval over an admitted
   position to remain distinct from the same interval over an absent position,
   which is incomplete rather than false. Include a nested composition whose inner
   and outer intervals both contribute offsets.
5. Build a trace whose two participating positions carry equal clock coordinates
   and no admitted order key, and require `until`, `release`, `since` and
   `triggered` to refuse. Supply the same positions with an admitted order key and
   require each operator to evaluate. Independently reverse the trace's insertion
   order while leaving the order keys unchanged and require an unchanged result,
   establishing that trace index is not consulted.
6. Evaluate an open decision scope in three shapes: a witnessed
   `eventually[0,30] holds(p)` that settles true on the observed prefix without
   the scope closing; an `always[0,5] holds(p)` with a counterexample inside the
   prefix that settles false; and an `always[0,5] holds(p)` whose observed prefix
   is all true but whose remaining offsets are unobserved, which must return
   `pending` with an unsettled basis. Require the settled cases to leave the scope
   open and to claim no execution closure.
7. Independently remove one required `holds` valuation, one fixed sample under an
   otherwise valid binding, and one interior history interval. Require each to
   report incomplete naming the exact position and the missing dimension. Require
   that none becomes false, an empty population or a settled truth, and that a
   sibling operand's established value stays inspectable beside the incomplete one.
8. Evaluate one declaration twice with identical traces and require equal result
   identities. Then independently change the clock binding, the selected profile
   and one evaluation ceiling, and require each change to produce a distinct result
   identity whose premises record the changed dimension. Require that the earlier
   result is not reused and is not rewritten.

## Expected Results

Every truth is tied to its declaration, selected profile, clock binding and trace
completeness assertion. `true`, `false`, `pending`, incomplete and refused remain
five distinct outcomes; no missing observation, closed boundary or unsupported
mapping collapses into a Boolean. The three profile interpretations remain three
meanings over identical predicate values and identical numeric intervals.

Activation is asserted separately in
[TC-123](./TC-123-activate-temporal-obligations.md) and resource behavior in
[TC-124](./TC-124-bound-temporal-evaluation.md); this case makes no claim about
either. Native-to-TL lowering is outside this case.
