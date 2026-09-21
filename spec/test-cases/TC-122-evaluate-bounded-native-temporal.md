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
independently constructed observation trace. Expected truths, bases, support sets,
premises and refusal causes come from the authored source and from the registered
temporal profile definitions (`src/linking/composed/definition_source.rs`), never
from the evaluator's own output or reported usage. Each numbered group
corresponds to the matching acceptance criterion.

Controls execute in `tests/composed_temporal_evaluation.rs`. Traces are Rust
values; no JSON fixture, shell step or installed CLI participates. A profile is
selected per declaration by its authored `profile` alias, so a case comparing
profiles compiles one declaration per profile from identical clause text.

## Test Procedure

1. Compile `always[0,1] holds(p)` and `always[0,1] true` under the event-position
   profile, then the same two clauses under the fixed-sample profile. Supply one
   closed, complete decision scope holding exactly position 0 with `p` true.
   Require the `holds` form false with basis `closed-scope` and the constant form
   true, in both profiles, with the false result's support naming the
   out-of-scope offset 1. Repeat both clauses wrapped in a grouping node and
   inside `true and (...)`, and require the same two outcomes. Inspect the two
   emitted operation graphs and confirm the `Constant` and `Holds` nodes remain
   distinct in the artifact as well as in the result.
2. Compile the same authored `always[0,1] holds(p)` clause three times, once per
   profile alias, and evaluate each against a trace that agrees on every `holds`
   valuation and on the numeric interval: an event-position sequence of one
   admitted position, a fixed-sample trace of one required sample, and a complete
   finite window whose only admitted instant is the anchor. Require the
   finite-window result true and both false-extension results false, and require
   the three results to retain three distinct profile identities, revisions and
   clock premises. Compare the three result identities pairwise and require no
   two to be equal.
3. Against one fixed declaration, independently mutate the trace's asserted
   profile identity, its asserted profile revision, and its clock binding name.
   Require each to refuse naming that dimension with reported `positions` usage
   zero, establishing that the refusal preceded any position visit. Inspect the
   refusal value and require no substituted profile, clock or nearest-compatible
   selection to appear in it.
4. Exercise all eight bounded operators under each of the three profiles on
   authored traces: `eventually`, `always`, `until`, `release`, `once`,
   `historically`, `since` and `triggered`. Include the zero-width `[k,k]`
   interval and a nonzero lower bound for one unary and one binary operator, and
   `once[1,1]` as strong previous. Require `p until[1,2] q` true where `p` is
   false at the anchor and `q` true at offset one. Independently evaluate
   `p release[1,2] q` against `not ((not p) until[1,2] (not q))` and
   `p triggered[1,2] q` against `not ((not p) since[1,2] (not q))` over the same
   traces, and require agreement. Include a nested composition whose inner and
   outer intervals both contribute offsets.
5. Build a trace whose two participating positions carry equal clock coordinates
   with no admitted order key, and require `until`, `release`, `since` and
   `triggered` to refuse. Repeat with only one of the two positions keyed, and
   with two keys drawn from different admitted order authorities; require both to
   refuse. Supply one admitted order covering both positions and require each
   operator to evaluate. Reverse the trace's insertion order leaving the order
   keys unchanged and require an unchanged result.
6. With `p` true at the single closed-complete position under each
   false-extension profile, require `always[0,1] not holds(p)` false and
   `eventually[0,1] not holds(p)` true, and require `always[0,1] not true` false.
   Evaluate `holds(p) implies holds(q)` and `holds(p) or holds(q)` at an
   out-of-scope offset and require the pointwise reading rather than a single
   untimed valuation.
7. Evaluate one clause in five shapes: closed-complete, requiring basis
   `closed-scope`; open with a witness that every admitted continuation preserves,
   requiring `decisive-witness` and true; open with such a counterexample,
   requiring `decisive-counterexample` and false; open with an all-true observed
   prefix and unobserved remaining offsets, requiring pending with `unsettled`;
   and open with a fact missing inside the completed decision-support set,
   requiring basis `unavailable` with no truth. Then remove a fact outside that
   support set and require the previously settled truth, basis and support to be
   unchanged.
8. Label a decision scope closed while asserting incomplete input, and require
   that neither false extension nor finite-window empty truth is applied and that
   the result is incomplete. Evaluate an empty finite window twice: once with
   completeness asserted through the inclusive upper endpoint, requiring
   `eventually` false and `always` true; once without it, requiring pending with
   `unsettled`.
9. Evaluate `once[0,2] holds(p)` and `historically[0,2] holds(p)` over one visible
   suffix twice: with its lower boundary declared the authoritative execution
   origin, requiring a Boolean; and with the identical suffix declared a history
   cutoff, requiring missing-history incomplete. Separately omit one interior
   history interval and require incomplete naming that exact position, never false
   padding.
10. Drive the four axes independently against one declaration: closed decision
    scope with open surrounding execution, open decision scope with closed
    surrounding execution, completed and failed assessment execution, and
    complete and incomplete input. Require each axis to be reported as supplied
   and require closing one never to close or complete another. Inspect the public
   evaluator input and require that it accepts no caller-supplied settlement
   basis or truth; inspect the closed result construction and require each
   result to derive those fields from the evaluated trace.
11. Under the fixed-sample and timestamped profiles, advance a watermark to an
    inclusive deadline with complete valuations and no business event, and require
    the applicable bounded obligation to settle. Place one admitted instant
    exactly at the deadline and require it to participate; move it one tick past
    and require it not to. Under the event-position profile, advance wall-clock
    and sample progress with no admitted position and require no settlement.
12. Submit a watermark that regresses under one binding, then a completeness
    assertion revised in conflict under one binding, and require a typed
    contradiction refusal in each case. Re-read the prior result and require its
    progress, closure, truth and bytes unchanged. Separately supply progress for a
    foreign clock, subject and binding and require no settlement.
13. Evaluate one declaration twice with identical inputs and require equal result
    identities. Then independently change the clock binding name, the selected
    profile, the declared sample period, the declared epoch, the declared
    timestamp unit, the declared sequence authority and one ceiling; require each
    to produce a distinct result identity whose premises record the changed
    dimension. Require the earlier result to be neither reused nor rewritten.

## Expected Results

Every truth is tied to its declaration, selected profile identity and revision,
clock binding and trace premises. `true`, `false`, `pending`, incomplete and
refused remain five distinct outcomes, and `closed-scope`, `decisive-witness`,
`decisive-counterexample`, `unsettled` and `unavailable` remain five distinct
bases; no missing observation, closed boundary, contradiction or resource stop
collapses into a Boolean. The three profile interpretations remain three meanings
over identical predicate values and identical numeric intervals.

Activation is asserted separately in
[TC-123](./TC-123-activate-temporal-obligations.md), resource behavior in
[TC-124](./TC-124-bound-temporal-evaluation.md) and mapping support in
[TC-125](./TC-125-classify-temporal-mapping-support.md); this case makes no claim
about any of the three. The emitted temporal body carries no declared sample
period, epoch, timestamp unit or sequence-authority value, so group 13 establishes
that changing one changes result identity, not that the artifact authenticates it.
