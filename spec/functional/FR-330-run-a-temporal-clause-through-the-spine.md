---
id: FR-330
title: "Run a selected temporal clause over a supplied trace through the spine"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-033
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-109
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-327
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-328
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-329
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-301
    type: depends_on
---
# FR-330: Run a selected temporal clause over a supplied trace through the spine

## Description

`qsl_replay::spine::run_clause` (FR-109) SHALL accept a `Temporal` selection
that names one temporal clause, a supplied trace and the clause's `over`
binding. It compiles the unit through the spine, admits each observation of
the trace (FR-106), evaluates the clause at S6a through the TemporalTrace
evaluator (FR-327, FR-328, FR-329), and returns one `ClauseRunReport`. With
it, every admitted clause kind executes through the spine, as the reference
runtime's exit asks (ADR-011 §5).

## Use case

An operator has recorded observations of a running system and a QSL unit
with temporal clauses. They run one clause by name over the recording, as
they already run a state clause, and get one report: held on this trace,
violated at a named position, pending, unsupported, or incomplete.

## Inputs

FR-109's `ClauseRunRequest`, with the selection
`Temporal { name, trace, over }`:

- `name`: a temporal clause's declared name;
- `trace`: `TraceInput::Finite(Vec<DocumentRef>)`,
  `TraceInput::Lasso{prefix: Vec<DocumentRef>, loop: Vec<DocumentRef>}`, or
  `TraceInput::Model{initial: DocumentRef, universes, prefix:
  Vec<ModelStep>, loop: Vec<ModelStep>}`, `TraceInput::Progressed{trace: Vec<DocumentRef>, progress, lower_boundary, binding}` (FR-504), the finite
  trace of an event-position or fixed-sample clause with its progress assertion
  (watermark, completeness, scope closure and surrounding-execution closure), its
  lower boundary beside the assertion and its `TraceBinding`,
  or `TraceInput::Timestamped{
  observations: Vec<{document: DocumentRef, stamp, order_key}>, watermark,
  completeness, scope, execution, lower_boundary, cover}` (FR-501), the trace of a clause under the
  timestamped-event finite-window profile, with an optional `ledger`
  (`ProgressLedger`, FR-502) on the selection; each `DocumentRef` names a
  snapshot in the request's snapshot provision by identity (FR-106);
- `execution`: the surrounding-execution closure (`Closed` or `Open`) of a
  `Timestamped` trace, the closure FR-501 lists beside `scope`, which is the
  decision-scope closure;
- `cover`: `Exact` (the default) or `Counter { uncertainty }`, how a `Timestamped`
  trace's stamps cover time (FR-501). The checked clause's clock configuration
  `TimestampedEvent` names only the timestamp unit, so the cover has no source
  in the package and the request supplies it, beside the stamps it describes;
- `binding`: a `TraceBinding` (FR-522), a required member of `Timestamped` and
  `Progressed`, which the entry compares with the clause's profile and clock rows
  before it evaluates; a `Timestamped` or `Progressed` request without it is
  refused at stage `admit`, `invalid_runtime_input`/`invalid-value`;
- `over`: `{"reference": {"population": ..., "key": ...}}`, resolved in the
  observation at position 0, when the clause has an `over` parameter.

## Outputs

FR-109's `ClauseRunReport`: the disposition, the compiled `package_id` and
the usage; for a `Timestamped` trace it also returns the updated `ledger`
(FR-502) and FR-501's result report (basis, decision support, premises and
result identity); a request refused by FR-502 returns the ledger it supplied,
unchanged. Its `category` adds `pending` (a pending result, with the pending clause ids in its body)
and `unsupported` (a missing fairness premise). Its `disposition` for a
`violation` (`Completed(false)` or an `Undefined` that is a violation)
carries the failing `TemporalPosition` and the refutation's
`TemporalCounterexample`, QSpec FR-364's observed arm, which the entry
builds from what it ran: one `{document}` step per represented position
from the identities of the trace's `DocumentRef`s (the `Finite`
trace as `prefix` with an empty `loop`, or the `Lasso`'s `prefix` and
`loop`; for an `Undefined` result, the positions 0 to the failing position
as `prefix` and an empty `loop`), the checked clause's fairness set, the
interval of its first interval-carrying operator in pre-order (`null` when
it has none, and for an `Undefined` result), the trace position, and for
an `Undefined` result the undefined-evaluation envelope whose `where`
carries the position, that position's document identity and the undefined
atom's locus. A refused condition's disposition carries its position. The
FR-267 writer writes these as the report holds them and derives none.

The report also carries `decisive` (`ClauseRunReport::decisive`), present only
for a temporal violation, a `Completed(false)` for which a decisive atom exists
("The decisive atom" below) and FR-107's clause evaluator decides it, and
absent for every other report. It holds
`{ position, quantifier, element, index, value_path }`: the `TemporalPosition`
of the decisive atom, the deciding collection quantifier's occurrence key
(absent when none decides), the deciding element, its index (absent when none
decides) and the FR-207 value path, all read by evaluating the atom over the
observation at `decisive.position`. When no collection quantifier decides it,
`quantifier` and `index` are absent, `element` is `false` and `value_path` is
the empty path at the observation's root. `decisive` is not a member of the
`TemporalCounterexample`, whose shape stays QSpec FR-364's, and FR-267 does
not write it. The report's violation position stays the failing activation's
position (FR-327); `decisive.position` may be later or earlier than it.

## Behavior

- The entry SHALL take a `Timestamped` trace's cover from the trace input.
- The entry SHALL treat a `Timestamped` input that omits the cover as `Exact`.
- The entry SHALL accept `TraceInput::Timestamped` from a library caller only.
- The request reader SHALL refuse a `timestamped` trace, because FR-312's request
  file has no member that carries a stamp, an order key, a progress assertion or
  a cover.
- The entry SHALL compile and report
  compile results exactly as FR-109 states.
- The entry SHALL resolve the name in the compiled package's temporal
  clauses. When it resolves to none, the entry SHALL report stage `select`,
  `missing_declaration`/`missing-name`.
- The entry SHALL admit every observation of the trace by FR-106 before
  evaluation, in position order, and report the first admission failure at
  stage `admit`. For `TraceInput::Model`, it SHALL admit the initial state
  and reconstruct the positions as FR-329 states for `Lasso::Model`.
- When `over` resolves to no object of the observation at position 0, the
  entry SHALL report stage `admit`, `invalid_runtime_input`/
  `wrong-role-mapping`.
- The entry SHALL evaluate exactly once: FR-327 for a bounded-profile
  clause over `Finite`; FR-328 for an infinite-trace clause over `Finite`;
  FR-329 for an infinite-trace clause over `Lasso` or `Model`. A
  bounded-profile clause over `Lasso` or `Model` SHALL report stage
  `admit`, `invalid_runtime_input`/`invalid-value`: a bounded profile reads
  a closed finite trace here; the zero-counted protocol lasso FR-329
  reads under a bounded profile is not an input of this entry.
- The entry SHALL evaluate a clause under the finite-window profile once
  through FR-501 over `TraceInput::Timestamped`.
- The entry SHALL evaluate an event-position or fixed-sample clause over
  `TraceInput::Progressed` once through FR-504, and SHALL refuse `Progressed`
  for any other profile or trace shape at stage `admit`,
  `invalid_runtime_input`/`invalid-value`.
- The entry SHALL report a `Finite`, `Lasso` or `Model` trace for that profile, and a
  `Timestamped` trace for any other profile, at stage `admit`,
  `invalid_runtime_input`/`invalid-value`. FR-501's `Pending` reports
  `pending` (the exit of FR-285's row for a supplied-trace clause pending when the trace ends) and its `Indeterminate` reports `indeterminate` (exit 11)
  with its reason as the cause, no `truth` and no counterexample. Both labels
  and exits are the shared catalog's `Category` values, with no local mapping
  (FR-285). A progress
  contradiction (FR-502) reports stage `evaluate`, category `refusal`, with its
  record.
- A refused `holds` condition (FR-327 "Refused condition") SHALL report
  stage `evaluate` with FR-100's `refused` outcome for its evaluation (the
  record's code, cause, fields and the atom's locus), its position, and
  the category and exit of the record's code; its kernel
  `Refusal::CheckedInvariant` reports `internal-failure`, as FR-100 maps
  it. It is not reported as an internal fault or as `undefined`.
- A refusal of FR-329 (malformed or unfair lasso) SHALL report stage
  `evaluate`, category `refusal`, with its record.
- The entry SHALL map `Completed(true)` to `success` (exit 0);
  `Completed(false)` to `violation` with the failing position; `Undefined`
  to `violation` with its position and its `UndefinedEvaluation` cause, as
  FR-109 reports a state clause that evaluates undefined;
  `InsufficientData` to `undefined` (exit 10, FR-285) with its position,
  `count` and `min_count`, no `truth` and no counterexample, as the trace
  lacks the data to decide the claim (QSpec FR-414);
  `Pending` to `pending` (exit 0, completed without violation) with the status and the pending clause ids in the report;
  `MissingFairnessPremise` to `unsupported` (exit 21) with cause
  `unsupported_projection`/`missing-fairness-premise`; and `Incomplete` to
  `incomplete` (exit 22), by QSpec FR-301's exit contract. Every
  `violation`, an undefined evaluation included, exits with the violation
  exit, 10.

### The decisive atom

- At a visited position `i`, the **eligible positions** of an operator with
  interval `[a,b]` are, for a future operator, the positions `j` with
  `i + a <= j <= i + b`, and for a past operator the positions `j` with
  `i - b <= j <= i - a`, restricted to the positions the profile reads
  (FR-327); an unbounded operator reads every position from `i + a`. Positions
  below `i + a` are not eligible, so a left operand of `until` is required only
  from the lower bound (FR-327-AC-9).
- The decisive atom of a `Completed(false)` SHALL be the `holds` atom reached by
  this descent, which starts at the failing activation's clause formula `φ`
  evaluated at the activation's position and always visits a subformula that
  evaluates false at the position it is visited at:
  - `holds(P)`: the atom, at that position.
  - `not φ'`: none; the clause then has no decisive atom.
  - `φ1 and φ2`: the descent continues into the first operand that is false.
  - `φ1 or φ2`: into `φ1`, both operands being false.
  - `φ1 implies φ2`: into `φ2`, the antecedent being true.
  - `always[a,b] φ'`: into `φ'` at the least eligible position where it is
    false. An unbounded `always` over a finite prefix or a lasso reads every
    position of the trace.
  - `eventually[a,b] φ'`: into `φ'` at the least eligible position, every
    eligible position being false.
  - `φ1 release φ2`: into `φ2` at the least eligible position where it is false.
  - `φ1 until φ2`: into `φ1` at the least eligible position `m` where `φ1` and
    `φ2` are both false when one exists, and otherwise into `φ2` at the least
    eligible position.
  - a past operator (`once`, `historically`, `since`, `triggered`): the mirror
    of its future counterpart over its eligible positions, taking the nearest
    eligible position first. The decisive position may therefore be earlier than
    the activation's.
  - a window aggregate atom (FR-194): the atom, at the position it is read.
- If the descent reaches a position before the first position of the trace, or,
  under a false-extension profile, a position after the last position (where
  every atom reads false and no observation exists), the clause SHALL have no
  decisive atom and the report SHALL carry no `decisive`.
- Over a `Lasso`, `decisive.position` is the position in the prefix-then-loop
  order of FR-329, the least such position being taken. Over a `Model` trace,
  positions are the states FR-329 reconstructs from the initial state and the
  steps, `decisive.position` is the position of the reconstructed state, and the
  atom is evaluated over that state as it is for a `Lasso`. Over `Progressed`,
  positions index the supplied `trace`, and a descent that reaches history before
  the supplied trace has no decisive atom. A `Timestamped` run carries no
  `decisive`: FR-501's decision support is its record.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-330-AC-1 | From source, the `Counter` unit's clause `Bounded` (`eventually[0,1] holds(c.value = 2)`, event-position false-extension) over `Finite` snapshots with `c.value` 0, 1, 2 reports stage `evaluate`, `violation`, position 0, exit 10. | Test (TC-840) |
| FR-330-AC-2 | The unit's infinite-trace clause `Reaches` (`eventually holds(c.value = 2)`) over the same `Finite` trace reports `pending`, exit 0, with no `truth` and the clause id in the report's pending ids; over `Lasso` with an empty prefix and loop 0, 1 it reports `violation` at position 0; over `Lasso` with loop 0, 1, 2 it reports `success`, exit 0. | Test (TC-840) |
| FR-330-AC-3 | A selection naming `Absent` reports stage `select`, `missing_declaration`/`missing-name`; `Bounded` over a `Lasso` reports stage `admit`, `invalid_runtime_input`/`invalid-value`; an `over` naming key `ghost` reports stage `admit`, `invalid_runtime_input`/`wrong-role-mapping`; a `Lasso` with an empty loop reports stage `evaluate`, `refusal`, `invalid_runtime_input`/`invalid-value`. | Test (TC-840) |
| FR-330-AC-4 | `Reaches` over the `Lasso` with loop 0, 1, 2 and a work budget of zero reports `incomplete`, exit 22; running any request of AC-1 to AC-3 twice gives equal reports. | Test (TC-840) |
| FR-330-AC-5 | From source, FR-327-AC-5's first clause, `always[0,2] holds(3 mod (2 - c.value) >= 0)` in the `Counter` unit, which compiles, over the `Finite` snapshots with `c.value` 0, 1, 2 reports `violation`, exit 10, position 2 and cause `UndefinedEvaluation` with `division-by-zero`. | Test (TC-847) |
| FR-330-AC-6 | From source, a clause `Fair` (`always eventually holds(c.value = 2)` under `fair weak inc`) over `Lasso` with an empty prefix and loop 0, 1, 2 reports stage `evaluate`, `unsupported`, exit 21, cause `unsupported_projection`/`missing-fairness-premise` naming `fair weak whole inc`. | Test (TC-848) |
| FR-330-AC-7 | From source, the `Counter` clause `always[0,1] holds(InUnit(c.value))` under event-position false-extension with `function InUnit using v(x: Int[0, 1]): Boolean pure { x >= 0 }`, whose `holds` condition completes for `c.value` 0 and, for `c.value` 2, refuses the `Coerce` of 2 into `Int[0, 1]` with kernel `integer_out_of_domain`/`outside-domain` and `expected` naming `Int[0, 1]` (FR-096-AC-8, TC-428), over `Finite` snapshots with `c.value` 0, 2 reports stage `evaluate`, `refusal`, exit 20, position 1, and an `outcome` `refused` with that code, cause, `expected` field and the condition's locus; it reports no `truth`, no `internal_fault` and no `UndefinedEvaluation`. | Test (TC-1160) |
| FR-330-AC-8 | From source, FR-327-AC-7's `always[0,2] A` `on origin` over `Finite` snapshots with `c.value` 0, 1, 2 reports stage `evaluate`, category `undefined`, exit 10, position 2, `count` 0 and `min_count` 1, and no `truth`, no counterexample and no `UndefinedEvaluation`. | Test (TC-847) |
| FR-330-AC-9 | From source, a clause under the finite-window profile run over `TraceInput::Timestamped` reports the outcomes of FR-501-AC-11 and never `unsupported_profile` or `ProfileNotEvaluated`; the same clause over `Finite` reports stage `admit`, `invalid_runtime_input`/`invalid-value`, and `Bounded` (event-position) over `Timestamped` reports the same. A `ledger` holding binding B at watermark 100 and a second request under B with watermark 60 reports stage `evaluate`, category `refusal`, `contradicted_dimension` `Watermark`. | Test (TC-1295, TC-1296) |
| FR-330-AC-10 | From source, an event-position clause and a fixed-sample clause run over `TraceInput::Progressed` report the outcomes of FR-504-AC-1 and AC-2, and return the updated `ledger`; a `Progressed` request for an infinite-trace clause, or over a `Lasso`, reports stage `admit`, `invalid_runtime_input`/`invalid-value`. | Test (TC-1299) |
| FR-330-AC-11 | From source, a `Timestamped` request whose `binding` asserts another clock name reports stage `admit`, `invalid_runtime_input`/`invalid-value` with `mismatched_dimension` `Clock`, a `Progressed` request whose `binding` asserts another profile reports `Profile`, and a `Timestamped` request with no `binding` reports stage `admit`, `invalid_runtime_input`/`invalid-value`. | Test (TC-1317) |
| FR-330-AC-12 | `Bounded`'s violation of AC-1 over `Finite` snapshots `c.value` 0, 1, 2 reports a `decisive` with `position` 0, no `quantifier` and no `index`, `element` `false` and the empty `value_path`; a clause `always holds(c.value <= 1)` `on origin` under the infinite-trace profile over the same snapshots reports the violation at position 0 and a `decisive` with `position` 2; a clause `always holds(forall i in self.items : i > 0)` `on origin` over snapshots whose positions 0 and 1 hold `items` 1, 2, 3 and whose position 2 holds `items` 3, 0, 5 reports the violation at position 0 and a `decisive` with `position` 2, the `forall` occurrence key as `quantifier`, `element` 0, `index` 1 and the FR-207 `value_path` to `items`; a pending, a successful and an `Undefined` report carry no `decisive`. | Test (TC-840) |
| FR-330-AC-13 | Over the `Counter` prefix `c.value` 0, 1, 2 under the infinite-trace profile, FR-328-AC-2's `always (holds(c.value = 1) implies eventually[0,1] holds(c.value = 3))` is violated at position 0 with a `decisive` of `position` 1 (the antecedent is true at position 1 and the consequent atom `c.value = 3` is false at positions 1 and 2, the least being 1); `holds(c.value <= 1) until holds(c.value = 9)` is violated at position 0 with a `decisive` of `position` 2 (`φ1` first fails there); FR-328-AC-3's `always (holds(c.value = 0) implies once[1,1] holds(c.value = 9))` is violated at position 0 and carries no `decisive` (the descent reaches the position before the trace); `always not holds(c.value = 2)` is violated at position 0 and carries no `decisive`. | Test (TC-840) |
| FR-330-AC-14 | Over `Lasso` with prefix `c.value` 0 and loop 1, 2, `always holds(c.value <= 1)` is violated at position 0 with a `decisive` of `position` 2. Over `Progressed` carrying `c.value` 0, 1, 2 under event-position false-extension, `always[0,2] holds(c.value <= 1)` `on origin` reports a violation at position 0 and a `decisive` of `position` 2; over `Timestamped` FR-501-AC-1's trace, `always[0,5] holds(p)` reports a violation at position 0 and no `decisive`. | Test (TC-840) |
| FR-330-AC-15 | Over the closed `Counter` trace `c.value` 0, 1, 2 under event-position false-extension, `always[0,3] holds(c.value <= 2)` `on origin` is violated at position 0 and carries no `decisive`, because it is false only at position 3, past the last position; `holds(c.value = 1) until[1,2] holds(c.value = 3)` `on origin` is violated at position 0 with a `decisive` of `position` 2 (the left operand is not required at the anchor, and is false at position 2 where the right operand is also false). | Test (TC-840) |
| FR-330-AC-16 | Over `Model` whose initial state and steps reconstruct `c.value` 0, then the loop states 1, 2, `always holds(c.value <= 1)` is violated at position 0 with a `decisive` of `position` 2, the atom evaluated over the reconstructed state of position 2. | Test (TC-840) |

## Dependencies

- ADR-011 §5 (the spine replacement for native clause execution); ADR-014
  §5 A-4.
- [FR-109](FR-109-run-a-state-clause-through-the-spine.md) (the run entry,
  its compile and report rules), [FR-106](FR-106-admit-snapshots-and-invocations.md),
  [FR-327](FR-327-evaluate-a-temporal-clause-over-a-finite-trace.md),
  [FR-328](FR-328-evaluate-an-infinite-trace-clause-over-a-finite-prefix.md),
  [FR-329](FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md).
- QSpec FR-301 (exit codes), FR-362 (the missing fairness premise).

## References

- Linear QSL-384 (spec ticket; the temporal share of the reference-runtime
  exit); QSL-43 (implementation).
