---
id: FR-043
title: "Evaluate bounded native temporal obligations under one selected profile"
type: FR
relationships:
  - { target: ix://agent-ix/quire-spec-language/US-003, type: implements }
  - { target: ix://agent-ix/quire-spec-language/FR-042, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-044, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-045, type: references }
  - { target: ix://agent-ix/quire-specification/FR-090, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-091, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-092, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-094, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-048, type: references }
---
# FR-043: Evaluate bounded native temporal obligations under one selected profile

## Description

When a caller supplies an admitted `quire.compiled-protocol/1` temporal body, its
exactly one selected temporal profile and an independently supplied observation
trace, the evaluator SHALL return the obligation's truth, settlement basis, exact
decision support and retained premises, or a located incomplete or refused result
that names the affected dimension.

## Semantic authority and boundary

Agent E owns native temporal meaning through
`ix://agent-ix/quire-specification/FR-090`, `FR-091`, `FR-092` and `FR-094`.
This requirement SHALL NOT define a native temporal interpretation of its own: it
specifies the compiler's evaluation of the artifact it emits, and every semantic
rule below is a local restatement of an owned shared rule for verification
purposes. If this evaluator's behavior and an owned shared rule diverge, then the
shared rule governs and this requirement is defective.

Agent F owns observation transport, storage, replay and completeness authority.
The trace is a caller-supplied input. Its completeness assertion, admitted order
and authoritative-origin claim are trusted as supplied and are not verified here;
they are retained as premises so a later contradiction can identify the results
that depended on them.

Agent B owns protocol activation, participation and result serialization. This
requirement produces no protocol result and no wire encoding of its own.

Native-to-TL mapping support is classified by
[FR-045](./FR-045-classify-temporal-mapping-support.md) and is not decided here.

## Inputs

The emitted temporal body — current-input binder, clock binding index,
activation, ordered captures and the closed temporal operation graph — together
with the declaration's admitted definition entry, which carries the selected
profile identity and revision.

One caller-constructed observation trace supplying:

- its own asserted profile identity and revision, and its clock binding name;
- the declared clock parameters the profile requires — sample period, epoch and
  unit for fixed-sample, timestamp unit for timestamped-event, sequence authority
  for event-position — as opaque retained premises;
- its admitted positions, each with a clock coordinate in the profile's domain,
  an optional admitted order key, and a valuation map from `holds` leaf handle to
  Boolean. A required leaf absent from a required position's map is a missing
  valuation, not a false one;
- its decision-scope closure, its surrounding-execution closure, its
  assessment-execution disposition, its input-completeness assertion, and whether
  its lower boundary is an authoritative execution origin or a mere history
  cutoff;
- a progress watermark in the profile's clock domain.

Caller-lowered ceilings arrive as explicit values under
[NFR-008](../non-functional/NFR-008-bound-temporal-evaluation.md). The evaluator
consumes no ambient clock, installed default profile, file ordering, ingestion
order or backend capability report.

## Outputs

Per activated obligation: a truth of `true`, `false` or `pending`; a settlement
basis of `closed-scope`, `decisive-witness`, `decisive-counterexample`,
`unsettled` or `unavailable`; the exact decision-support position set; and the
retained premises. Otherwise one located `Incomplete` or `Refused` result naming
the affected obligation, position and dimension.

## Behavior

### Profile and clock selection

The evaluator SHALL refuse a trace whose asserted profile identity or revision
differs from the declaration's admitted definition entry, and SHALL refuse a
trace whose clock binding name differs from the declaration's emitted clock
binding.

The evaluator SHALL NOT substitute, widen, default or nearest-match a profile,
clock, period, epoch, unit or sequence authority.

The evaluator SHALL retain the declared clock parameters in the result premises,
so that a changed period, epoch, unit or sequence authority yields a distinct
result identity and refuses reuse of an earlier result.

The evaluator SHALL interpret one interval tick as one admitted semantic-event
position under `quire.temporal.event-position.false-extension/v1`, one declared
sample under `quire.temporal.fixed-sample.false-extension/v1`, and one tick of
the declared timestamp unit under
`quire.temporal.timestamped-event.finite-window/v1`.

### Operators

The evaluator SHALL evaluate the eight bounded operators `eventually`, `always`,
`until`, `release`, `once`, `historically`, `since` and `triggered` over
inclusive integer intervals with `0 <= a <= b`.

The evaluator SHALL evaluate `not`, `and`, `or` and `implies` pointwise at each
offset as temporal Boolean connectives. They are not untimed: `not holds(p)` is
evaluated at every offset the enclosing operator ranges over, including offsets
where the selected profile's closed-boundary rule has made `holds(p)` false.

The evaluator SHALL require the left operand of `until` and `since` only between
the interval's lower bound and the witness, and SHALL evaluate `release` and
`triggered` as the Boolean duals of `until` and `since` respectively under that
same lower-bound convention.

The evaluator SHALL evaluate `once[1,1]` as strong previous, and SHALL admit no
separate weak-previous spelling or boundary.

The evaluator SHALL refuse `until`, `release`, `since` and `triggered` when two
participating positions carry equal clock coordinates and either lacks an
admitted order key, and SHALL NOT substitute ingestion order, arrival order or
trace index. An order key from a different admitted order authority SHALL refuse.

### Atoms, constants and boundaries

The evaluator SHALL treat `holds(expr)` as an atomic temporal valuation distinct
from a source constant, and SHALL preserve that distinction through evaluation
and through any grouping node.

While the decision scope is closed and its input is complete, under the two
false-extension profiles the evaluator SHALL evaluate an atomic valuation as
false at integer offsets outside the scope, and SHALL leave a temporal constant
at its authored value.

Under the finite-window profile the evaluator SHALL range quantification only
over admitted instants whose clock distance from the anchor lies in the inclusive
interval, and SHALL add no synthetic atom after closure. An instant exactly at
the inclusive upper endpoint SHALL participate before settlement.

If a finite window contains no admitted instant, then the evaluator SHALL emit
its empty-existential `false` or empty-universal `true` only where the trace
asserts completeness through the inclusive upper endpoint; otherwise the result
SHALL be pending with an `unsettled` basis, or incomplete where a required
valuation is missing.

If the input is incomplete, then the evaluator SHALL NOT apply false extension
and SHALL NOT apply finite-window empty truth, even where the decision scope is
labelled closed.

For a past operator, the evaluator SHALL require history through its computed
lower boundary, or an authoritative execution origin at the trace's lower
boundary. A mere history cutoff SHALL produce a missing-history incomplete result
rather than a Boolean.

### Progress, closure and settlement

The evaluator SHALL keep decision-scope closure, surrounding-execution closure,
assessment execution and input completeness as four independent dimensions.
Closing or completing one SHALL NOT close or complete another.

While the decision scope is closed and complete, the evaluator SHALL settle with
basis `closed-scope`.

While the decision scope is open, the evaluator SHALL settle only with basis
`decisive-witness` or `decisive-counterexample`, and only where every admitted
continuation preserves the Boolean over complete exact decision support. Every
other open future SHALL be `pending` with basis `unsettled`.

If a fact inside the completed decision-support set is missing, then the
evaluator SHALL report basis `unavailable` with no truth. A fact missing outside
that set SHALL remain a completeness gap and SHALL NOT falsify, delay or erase a
settled truth.

The evaluator SHALL settle a deadline at offset `D` from a progress watermark `W`
only while `W >= D` and completeness covers every required valuation through `D`.
A fixed-sample or timestamped progress assertion SHALL advance the clock through
`D` with no business event, settling the applicable bounded obligation. An
event-position clock SHALL NOT advance during silence.

If a progress watermark regresses under one binding, or a completeness assertion
is revised in conflict under one binding, then the evaluator SHALL return a typed
contradiction refusal, and SHALL NOT roll back progress, restamp closure or
rewrite an earlier result.

The evaluator SHALL accept a settlement basis only with its valid truth and
closure combination, and SHALL refuse any one-axis substitution.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-043-AC-1 | On one closed-complete position with `p` true, `always[0,1] holds(p)` is false with basis `closed-scope` and `always[0,1] true` is true under both false-extension profiles; the same distinction survives a grouping node and an enclosing connective, and neither the emitted graph nor evaluation folds the two. | Test (TC-122), Inspection |
| FR-043-AC-2 | The same valuations and the same numeric interval under event-position, fixed-sample and timestamped-event selections yield three results retaining three distinct profile identities, revisions and clock premises, the finite-window case true where the false-extension cases are false; no two of the three share a result identity. | Test (TC-122), Analysis |
| FR-043-AC-3 | A trace whose asserted profile identity, profile revision or clock binding name differs from the declaration's admitted selection refuses before any position is visited and names that dimension; no default or nearest-compatible selection is inserted. | Test (TC-122), Inspection |
| FR-043-AC-4 | All eight bounded operators evaluate over inclusive intervals including the zero-width `[k,k]` form and a nonzero lower bound; `once[1,1]` is strong previous; `p until[1,2] q` is true with `p` false at the anchor and `q` true at offset one; `release` and `triggered` agree with the Boolean duals of `until` and `since` on the same traces. | Test (TC-122) |
| FR-043-AC-5 | `until`, `release`, `since` and `triggered` refuse when participating positions share a clock coordinate and either lacks an admitted order key, or when two order keys come from different authorities; the same positions with one admitted order evaluate, and reversing insertion order changes nothing. | Test (TC-122) |
| FR-043-AC-6 | `not`, `and`, `or` and `implies` evaluate pointwise: with `p` true at the single closed-complete position, `always[0,1] not holds(p)` is false and `eventually[0,1] not holds(p)` is true under false extension, distinguishing pointwise negation from an untimed reading. | Test (TC-122) |
| FR-043-AC-7 | A closed-complete scope settles with basis `closed-scope`; an open scope settles only with `decisive-witness` or `decisive-counterexample` and otherwise returns pending with `unsettled`; a fact missing inside completed decision support returns basis `unavailable` with no truth, while a fact missing outside it leaves the settled truth intact. | Test (TC-122) |
| FR-043-AC-8 | An incomplete input applies neither false extension nor finite-window empty truth even where the scope is labelled closed; an empty finite window emits empty-existential or empty-universal truth only under completeness through the inclusive upper endpoint. | Test (TC-122) |
| FR-043-AC-9 | A past operator whose computed lower boundary is the authoritative execution origin yields a Boolean, while the identical visible suffix under a mere history cutoff yields missing-history incomplete; an omitted interior history interval is incomplete at its exact position and never false padding. | Test (TC-122) |
| FR-043-AC-10 | Decision-scope closure, surrounding-execution closure, assessment execution and input completeness remain four independently represented dimensions; closing one cannot close or complete another, and every one-axis substitution of a settlement basis, truth or closure combination is refused. | Test (TC-122) |
| FR-043-AC-11 | A fixed-sample or timestamped watermark reaching an inclusive deadline with complete valuations settles a silent obligation, an instant exactly at the deadline participates before settlement, an instant after it does not, and an event-position clock does not advance during silence. | Test (TC-122) |
| FR-043-AC-12 | A regressing watermark or a conflicting completeness revision under one binding returns a typed contradiction refusal without rolling back progress, restamping closure or rewriting an earlier result; a foreign clock, subject or binding cannot settle the obligation. | Test (TC-122) |
| FR-043-AC-13 | Repeated evaluation with identical inputs reproduces one result identity; a changed profile, clock binding, declared period, epoch, unit, sequence authority or ceiling yields a distinct result identity and the earlier result is neither reused nor rewritten. | Test (TC-122), Analysis |

## Dependencies

- **Upstream**: [FR-042](./FR-042-publish-compiled-protocol-artifacts.md) emits the
  temporal body, clock binding requirement and closed operation graph consumed here.
- **Upstream**: [US-003](../usecase/US-003-evaluate-bounded-state.md) drives honest
  bounded outcomes.
- **Peer**: [FR-044](./FR-044-activate-temporal-obligations.md) owns activation and
  immutable captures; this requirement evaluates only an activated obligation.
- **Peer**: [FR-045](./FR-045-classify-temporal-mapping-support.md) owns
  native-to-TL mapping support classification.
- **Constrained by**:
  [NFR-008](../non-functional/NFR-008-bound-temporal-evaluation.md).
- **Known limitation, recorded as remaining work on compiler #38**: the emitted
  temporal body carries a clock binding name and the selected definition entry,
  but no declared sample period, epoch, timestamp unit or sequence-authority
  value. Those are therefore retained trace premises that participate in result
  identity, and are not independently checked against the artifact. Checking them
  requires an FR-042 wire extension outside this scope.
