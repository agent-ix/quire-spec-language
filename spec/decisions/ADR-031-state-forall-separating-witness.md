---
id: ADR-031
title: "Separating witness for a state forall on native-run-result/2"
type: ADR
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-072
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-109
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-122
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-014
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-041
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-243
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-351
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-352
    type: depends_on
---
# ADR-031: Separating witness for a state forall on native-run-result/2

## Context

A state clause (FR-107) is evaluated at S6a over one admitted observation: the
current snapshot for an invariant, the pre-call snapshot and arguments for a
precondition, the invocation for a postcondition (FR-106). A `forall(x in c:
p)` in that clause stops at the first element whose body is false, and an
`exists` at the first element whose body is true (QSpec FR-041-AC-4). The S6a
evaluator in `qsl-eval` already stops there. It returns only the Boolean.

QSpec FR-351 defines the separating witness record: deciding element, index,
value path and trace position. One record shape serves every family, and a
family adds no shape of its own (ADR-012 §13, Q210-4). QSpec FR-352 mints
`native-run-result/2`: every `/1` member plus one closed member, the QSpec FR-351
record, present exactly when the settlement basis is `decisive-witness` or
`decisive-counterexample`. ADR-013 O-25 and O-27 place the record on the
replay result and on `native-run-result/2`. FR-070-AC-5 reserves a typed
extension point on the counterexample envelope for the state `forall` payload.

What is open is when a state clause has a separating witness, what each
component holds for a state `forall`, how replay checks it, and where it sits
on the result.

Today `qsl-replay` has a `SeparatingWitnessRecord` whose `deciding_element` is
a Boolean stand-in and whose `index` is always present. The predicate and
state-clause replays fill it with the result Boolean, index 0 and an empty
path. FR-122's `StateClauseCounterexample` names the clause and its
observation.

"QSpec FR-nnn" names a quire-specification requirement; a bare FR id is a QSL
requirement. Item ids `SW-` and `R-` are local to this record. Other artifacts
cite them as `ADR-031 SW-3`.

## Decision

### 1. When a clause has a separating witness

| ID | Rule |
| --- | --- |
| SW-1 | **Decisive occurrence.** A quantifier occurrence that stops before its last element is a decisive occurrence: a `forall` at the first element whose body is false, an `exists` at the first element whose body is true (QSpec FR-041-AC-4). It has a binder value, a position in the domain and a source binding. A quantifier that visits every element (`forall` true, `exists` false, or an empty domain) has no decisive occurrence. |
| SW-2 | **Decision path.** From the clause's claim root, the decision path follows the operand whose value its parent returned: the inner node of a group; the body of a `let`; the selected branch of an `if`; the operand of `not`, with the polarity flipped; the operand of `and` or `or` on which evaluation stopped (the first false operand of a false `and`, the first true operand of a true `or`, otherwise the right operand); for `implies`, the antecedent with the polarity flipped when it is false, otherwise the consequent. The path ends at the first node of any other kind. When that node is a decisive occurrence, it is the clause's decisive occurrence. The walk visits each node of the claim at most once, so its length is bounded by the claim's own size. |
| SW-3 | **Settlement basis.** A clause with a decisive occurrence settles with basis `decisive-counterexample` when its truth is false and `decisive-witness` when its truth is true (QSpec FR-243). The elements after the stop are never read, and the retained element fixes the truth whatever they hold; that unread remainder is the open part of the decision scope. A clause whose path ends anywhere else settles `closed-scope` and carries no record. This is the basis of one clause evaluated over one admitted observation. A model-check verdict settles its own basis by FR-127's map (ADR-018): a reproduced `Violated` trace is `decisive-counterexample` whatever basis the clause evaluation at its last state has. |
| SW-4 | **One record, the root-most occurrence.** The record names the decisive occurrence SW-2 reaches, which is the root-most one on the path. When the body was itself decided by an inner quantifier, that inner decision is re-established by the separation check (SW-13), not recorded. A clause therefore carries one record or none, as QSpec FR-351 requires. Because no quantifier above it on the path stopped early, the decisive occurrence's domain is evaluated in no binder of an enclosing quantifier, only in the clause's roots and `let` bindings. |

The common case, `forall(item in self.items: item < self.n)` false at the
third item, settles `violation`, truth false, basis
`decisive-counterexample`, with a record naming the third item. The guarded
form `p implies forall(...)` reaches the same record when `p` holds. `not
forall(...)` true and `exists(...)` true settle `success` with basis
`decisive-witness`. `forall(...) == q` and `forall(...)` true carry no record.

### 2. The payload

| ID | Rule |
| --- | --- |
| SW-5 | **Components.** The record is the QSpec FR-351 record with the deciding-quantifier component QSpec adds (R-4). **Deciding quantifier**: the occurrence key (O-07) of the decisive quantifier node, which holds that node's identity and tells apart two occurrences of one node. **Deciding element**: the kernel `Value` bound to the quantifier's binder at the stop; it compares under ADR-013 O-13 semantic equality. **Index**: the element's zero-based position in the domain, in the domain's own order (QSpec FR-041 keeps order observable); always present for this family. **Value path**: the QSpec FR-207 runtime value path of the deciding element's location; its subjects, steps and wire spelling are QSpec's (R-2; QSpec FR-207-AC-9 to AC-11). **Trace position**: absent, because a state clause reads one observation and keeps no trace. |
| SW-6 | **Computed domains.** When the domain is computed by `filter` or `map`, or built inside the claim, the value path and index are the ones QSpec's value-path vocabulary gives that element (R-2; QSpec FR-207-AC-9 to AC-11). The deciding element stays the value the body was evaluated on, which for `map` is the mapped value. |
| SW-7 | **Rust shape.** `qsl-replay`'s `SeparatingWitnessRecord` becomes the QSpec FR-351 record: `quantifier` the decisive quantifier's occurrence key, `deciding_element` a kernel `Value`, `index: Option<u64>` (absent for a family that assigns no position), `value_path` a typed segment sequence per SW-5, and `trace_position: Option<TracePosition>`. A result with no decisive occurrence carries no record, so the predicate and state-clause replays stop building the result-Boolean record at index 0. `WitnessArmResult::settle` takes the record as its own `Option`, independent of whether the replay completed a value. |
| SW-8 | **Family payload.** FR-122's `StateClauseCounterexample` carries `witness: Option<SeparatingWitnessRecord>`, present exactly when the clause's basis is decisive (SW-3). It travels as `WitnessEnvelope<StateClauseCounterexample>` with no change to the envelope (FR-070-AC-5). The envelope's `occurrence_key` stays the clause's `claim` occurrence. The record names the decisive quantifier by its occurrence key (SW-5); that occurrence is also a function of the package, the observation and SW-2, so replay re-derives it and compares (SW-12). |
| SW-9 | **Producer.** The S6a evaluator reports each early stop to its caller beside the outcome, never inside a `Value`: the quantifier occurrence, the index, the binder value and the source binding. The FR-109 clause run applies SW-2 to those reports and builds the record. FR-122's replay calls the same function, so producer and replay derive the record one way. |

### 3. Place on native-run-result/2

| ID | Rule |
| --- | --- |
| SW-10 | **The member.** `native-run-result/2` is the library serialization, on the spine check route (`qsl_replay::spine::run_clause`), of an FR-109 clause run report, and the CLI `run` command-error envelope. It carries every `/1` member unchanged plus two members. `basis` is the QSpec FR-243 settlement basis label, carried on every clause-run result as every other verdict carries it (R-3): `decisive-counterexample`, `decisive-witness` or `closed-scope` by SW-3 for an `evaluate` `success` or `violation`, and `unavailable` for every other disposition. `witness` is the SW-5 record, present exactly when the basis is decisive. A refusal, an undefined result, an exhausted or cancelled run and an unsupported capability carry basis `unavailable`, no `witness` and no substitute (QSpec FR-351-AC-3, AC-4). The record encodes through ADR-013 §2's RFC 8785 encoder; the exact wire spelling of both members is QSpec's (R-2, R-3). |
| SW-11 | **Version.** From the change that lands FR-100's clause runner (FR-312's reader plus `run_clause`), which deletes native `run` (R-1), the `run` command produces `/2` only, and `WireFormat` names no `native-run-result/1` (ADR-013 OQ-1). A `/2` reader refuses a record missing a component, carrying a foreign shape, or defaulting an absent component (QSpec FR-351-AC-5), and refuses a `/1` document under FR-014's unsupported-version rule (QSpec FR-352-AC-2). |

### 4. Replay and the separation check

| ID | Rule |
| --- | --- |
| SW-12 | **Re-derivation.** FR-122's replay evaluates the clause once over the observation it admits from the byte provision, as it does today, derives its own record by SW-2 and SW-9, and compares it with the payload's record componentwise (QSpec FR-351 identity, the deciding quantifier included; the deciding element under O-13). Equal verdicts and equal records settle the arm's agreement: `reproduced-with-evaluated-witness` on the `Witness` arm, `reproduced-without-witness` on the `Input` arm. A record that differs, or is present on one side only, settles `inconclusive` with a new typed `DisagreementCause::Witness` carrying both records. |
| SW-13 | **Separation check.** Replay then checks the record on its own terms, without trusting the search that found it. It resolves the record's deciding quantifier in the recompiled package, requires it to be a `forall` or `exists` occurrence of the clause's claim, evaluates its domain expression in the admitted observation, reads the element at `index`, requires its source location to be the value path and its value to equal the deciding element, and evaluates the quantifier's body once with the binder bound to that element, in the clause's roots and the `let` bindings on the path. The body must return false for a `forall` and true for an `exists`. Any failed step settles `inconclusive` with `DisagreementCause::Witness` naming the step; a domain or body evaluation that ends `undefined` fails its step and refutes the witness with cause `UndefinedEvaluation { where, cause }`, and one that is refused fails its step with its refusal record. An exhausted meter settles `NoValue`. This shows that the element alone separates the clause from the other truth value. |
| SW-14 | **Bounds.** The record's encoded size counts against the envelope's configured reader bound (FR-070-AC-7) and the separation check is charged to the request's evaluation meter. The value path is as long as the observation's own nesting. |

### 5. Verdicts the record backs

| Clause result | Category, exit | Basis | Record | Replay settlement on agreement |
| --- | --- | --- | --- | --- |
| `forall` refuted at an element (SW-2 path, polarity unflipped) | `violation`, 10 | `decisive-counterexample` | present | `reproduced-with-evaluated-witness` (`Witness` arm) or `reproduced-without-witness` (`Input` arm) |
| `exists` satisfied, or `not forall` true | `success`, 0 | `decisive-witness` | present | as above |
| Truth decided elsewhere, or every element visited | `success` or `violation` | `closed-scope` | absent | verdict parity only |
| Refused, undefined, incomplete, unsupported | the FR-109 category | `unavailable` | absent | FR-072 as today |

On the proof side, a refuted clause maps to `TerminalValue::Refuted`, O-16
`violation`, with no change to O-16 or O-24.

## Rulings

Owner rulings, 2026-10-01, recorded on the owning ticket.

| ID | Question | Ruling |
| --- | --- | --- |
| R-1 | ADR-013 OQ-1 removes `/1` where `/2` lands, while ADR-011 §7.3 M-6c and ADR-012 §15.8 keep the native `run` command, which emits `/1`, for `0-draft` sources until §15.8 step 2. | `/2` lands in the change that retires native `run`, as one change with no compatibility path. OQ-1 holds as written; no build produces both versions (SW-11). Native `run` is retired only by the change that lands FR-100's clause runner (FR-312's reader plus `run_clause`) (ruling 2026-10-01), so that one change lands the clause runner, `/2` and the deletion of native `run` and `/1`. Until then FR-026 to FR-032 stand. |
| R-2 | QSpec FR-351 names the value path but not its segment vocabulary or wire spelling, nor a root for an element with no stored location. | QSpec gets a value-path segment vocabulary: SW-5's segment kinds, the filter and map source bindings of SW-6, and the building-expression root for an element built inside the claim. QSL cites it and restates none of it (SW-5, SW-6). |
| R-3 | QSpec FR-352 adds one member, so `/2` would carry no settlement-basis label. | `/2` carries a basis label consistent with the other verdicts: the QSpec FR-243 label on every clause-run result (SW-10). |
| R-4 | `/2` would not say which quantifier decided the clause. | The record names the deciding quantifier by its node key: the quantifier's occurrence key, which holds its node identity (SW-5). |

## Consequences

- A failing state `forall` names the element that refutes it, where it sits in
  the observation and why it refutes, and that statement is checked by replay
  rather than trusted.
- The record is produced by the reference evaluator with no solver and no
  routing, so it needs no backend.
- `SeparatingWitnessRecord` gains the kernel `Value` and an optional index;
  every current construction site of the placeholder record changes.
- The same channel serves `exists` with no extra rule, because SW-1 to SW-3 are
  stated over both quantifiers.

## Alternatives Considered

- **Record the innermost decisive occurrence.** Rejected. Its value path would
  have to pass through the outer binder's element, and the outer element is
  already enough to separate the clause; the separation check re-establishes
  the inner decision.
- **One record per decisive quantifier on the path.** Rejected. QSpec FR-351
  allows exactly one record or none.
- **A separate `StateForallWitness` family payload.** Rejected. A state
  `forall` refutation is a state-clause counterexample; a second payload type
  would duplicate FR-122's clause and observation members.
- **The record as a member of the common envelope.** Rejected. FR-070-AC-5
  keeps family content in the family payload.
- **A record for every false clause, with the clause's Boolean as the deciding
  element.** Rejected. It names no element, so it separates nothing, and QSpec
  QSpec FR-352 ties the record to a decisive basis.

## References

- Owning ticket: Linear QSL-389. Implementation: Linear QSL-45.
- QSpec half: QSpec FR-351 and QSpec FR-352 (quire-specification#114); the
  value-path segment vocabulary (FR-207 AC-9 to AC-11), the `/2` basis label
  and witness members (QSpec FR-352 AC-6 and AC-7) and the deciding quantifier
  component (QSpec FR-351-AC-7), R-2 to R-4: Linear STD-144. QSpec FR-352-AC-2 refuses
  `/1`.
- Requirements: FR-265 (derivation), FR-266 (the clause run's witness and
  basis), FR-267 (`native-run-result/2`), FR-268 (replay and the separation
  check), FR-269 (`DisagreementCause::Witness`).
- Replay consumer: agent-ix/quire-contract-codegen#50.
- Witness shapes this record is consistent with: ADR-018 CX-2 and ADR-022
  GX-1 (both drafts), which carry their own family payloads in
  `WitnessEnvelope` the same way SW-8 does.
