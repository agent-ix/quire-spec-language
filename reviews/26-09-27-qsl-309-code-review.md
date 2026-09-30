---
id: SR-770
title: "QSL-309 code review of PR 510 (FR-114 protocol attempt-to-frame binding)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-forms/src/lib.rs; qsl-forms/src/protocol_clause.rs; qsl-forms/src/syntax.rs; qsl-forms/tests/it/protocol_clause_forms.rs; qsl-package/src/emit/tests.rs; qsl-replay/src/spine.rs; qsl-semantics/src/check/assemble.rs; qsl-semantics/src/check/assemble/tests.rs; qsl-semantics/src/check/check.rs; qsl-semantics/src/check/lowering.rs; qsl-semantics/src/check/lowering/state.rs; qsl-semantics/src/check/mod.rs; qsl-semantics/src/check/protocol_clause.rs; qsl-semantics/src/check/refusal.rs; qsl-semantics/src/check/region.rs; qsl-semantics/src/check/state_clause.rs; src/command/output.rs; src/command/output/types.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-114
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-113
    type: reviews
---
## Summary

Ticket: QSL-309. PR: quire-spec-language#510. This
file covers code-review with the rust-review lane folded in.

The parts that work:

- **S2 capture.** Every attempt passes through `event_node_anchors`,
  including attempts nested in sequence, choice/case, parallel/branch, repeat
  and an `await` match template. The grammar nests `Parameter`, `Related` and
  `Block`, so the event node's own identifier tokens are exactly: name, role,
  then the contracts entries. `get(2..)` is therefore correct against today's
  grammar.
- **Assembler resolution.** `protocol_attempts` repeats `state_clauses`'
  context lookup (`UnresolvedTypeName`), its operation lookup
  (`UnresolvedOperation` and `AmbiguousOperation` at the operation span) and
  its declaring-type `population_of`. Arity and type checks do not apply: an
  attempt names its operation and passes no arguments.
- **S3/S4 reuse.** `Lowering::protocol_attempt` calls the same private
  `operation_anchor`, `frame_node` and `frame_occurrence` as
  `Lowering::state_clause`. The diff adds no new `NodeTag` or node kind.
  Attempt anchors join `register_frame_occurrences`.
- **Contracts.** A missing entry refuses `MissingContract`. An invariant, or
  a clause on a different operation, refuses `WrongContractAnchor`. Neither
  case is skipped silently.
- **Gate.** A fresh `make ci`, in its own
  `CARGO_TARGET_DIR=target-309`, exited 0: 93 `test result: ok`, 0 FAILED,
  `ci-docs` clean under `-D warnings`. Every added intra-doc link is either on
  an undocumented private/`pub(crate)` item or names a public target.

Reviewer probes: temporary tests added to `qsl-package/src/emit/tests.rs` and
then reverted, never committed. They show two high defects that the gate
misses because no committed test exercises them (FND-001 and FND-002).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The `protocol_clause::unimplemented` refusal is deleted outright, not conditioned. A protocol with garbage content now checks and compiles, and it is silently missing from the emitted package. Only anchors, binders and `contracts` are checked, and the pipeline comment itself says roles, channels, requirements and bodies "still ha[ve] no checker". Probe: a unit with `function f ... { true }` plus a protocol with `over (input: Nope::Input)`, `role R on Nope::Actor`, `send S via Nope as (x: Undeclared) { 1 + true }` and `finish ... { 1 + true }` passes check and `emit_checked`, and the wire never names `Flow`. This reintroduces SR-753 FND-002 / SR-761 FND-001. The replay regression test was repointed so that it now refuses only because the attempt's model is undeclared (an assembly refusal), so it no longer guards body or binder-type garbage. FR-113 Status keeps the refusal until protocol checking and emission are complete, and emission of the protocol itself is not done. | qsl-semantics/src/check/mod.rs:1056-1076; qsl-replay/src/spine.rs:1237-1272 |
| FND-002 | high | An operation named only by an attempt (FR-114-AC-2: no clause, `contracts []`) checks but fails `emit_checked` with `UnlocatedOccurrence { role: Generated, ordinal: 0 }`. So QSL-299's "emit.rs needs zero new code" claim is false for that case, and the PR's emission test cannot see it because it always includes a `post` clause. Cause, part 1: `enclosing_declarations` places a generated occurrence only under `Origin::Body`, `Measure` or `StateClause`, and has no `Origin::ProtocolAttempt` arm. Adding that arm locally moved the failure to a second node, likely the `Boolean` type node that `protocol_attempt` mints with no recorded occurrence. So the fix also needs that node placed or given an occurrence. Probe C (a valid attempt plus garbage content) fails only because of this bug, and passes check. | qsl-semantics/src/check/lowering.rs:3981-3984; qsl-semantics/src/check/lowering/state.rs:233-250 |
| FND-003 | medium | The attempt's frame `operation-contract` record hand-copies `clause_records`' `ClaimSubject::Frame` arm (receiver/result/parameter roots, `Some(frame)` prefix, the `ClassifyFailure` mapping) instead of sharing one helper. The dedup arm `Occupied(existing) if *existing.get() == record` depends on the two copies staying byte-identical. Any drift turns an attempt plus a clause on one operation into an `UnkeyableRequirements` internal fault. Extract one `frame_record(operation, frame, populations, boolean, ...)` function used by both. | qsl-semantics/src/check/mod.rs:1396-1466; qsl-semantics/src/check/state_clause.rs:642-668 |
| FND-004 | medium | Anchor occurrence ordinals are not in source order. All clauses' `Anchor` occurrences are recorded first (the `lowered_clauses` loop) and all attempts' afterwards (the `lowered_attempts` loop). FR-114 Behavior requires "one `anchor` occurrence per clause or attempt that names the operation, ordinals in source order". A protocol written above the `post` clause gets the later ordinal. Found by reading the code; not measured on the wire. | qsl-semantics/src/check/mod.rs:1229-1286 |
| FND-005 | medium | `bind_attempts` binds a `contracts` entry to `indices[0]` when two state clauses share a name. The doc calls this "the same convention FR-113's own `by_scope_name` lookup follows", but FR-113 says "No source order or first match picks one" and refuses the ambiguity. Nothing in the assembler or the refusal catalog refuses duplicate state-clause names, so `contracts [X]` can bind the wrong clause identity silently. Refuse `ambiguous_declaration`/`ambiguous-name` at the entry when `indices.len() > 1`. | qsl-semantics/src/check/protocol_clause.rs:283-321 |
| FND-006 | low | The `state_clauses(...)?` early return means attempt assembly errors are never gathered into the same `AssemblyRefusal` as clause errors. A unit with one bad clause and one bad attempt reports only the clause. | qsl-semantics/src/check/assemble.rs:1547-1555 |
| FND-007 | low | Panic surface. (a) `lowered_clause_keys[index]` and `checked_protocols[protocol_index]` are only in bounds because of early returns about 180 lines above (mod.rs:1079, 1303). (b) `DeclarationRegions` construction `.expect("every protocol attempt names its own declaration")` sits in library code. Prefer `.get()` mapped to an `InternalFault`. | qsl-semantics/src/check/mod.rs:1484-1489; qsl-semantics/src/check/region.rs:201 |
| FND-008 | low | `contracts` capture is positional (`identifiers.get(2..)`, the event node's own identifier tokens after name and role). It is correct for today's grammar, but a future direct identifier in the attempt production silently becomes a contract entry. Read the tokens between `contracts [` and `]` instead. | qsl-forms/src/protocol_clause.rs:717-731 |

## Verdict

Request changes. FND-001 and FND-002 are high and block merge. With the
refusal lifted, protocols with unchecked content compile silently, and the
AC-2 attempt-only case cannot emit. FND-003 to FND-005 should be fixed in
the same round. The rest is low.

Note on FND-001: the fix is either (a) keep an `unimplemented` refusal for
protocols until protocol content checking and protocol emission exist (FR-113
Status), keeping FR-114's binding, or (b) a ruling from the ticket owner that
the lift is intended now, with FR-113/FR-114 Status updated to say so. The
QSL-309 ticket text asks for the lift, but the spec condition it relies on is
not met.

## Dispositions

| FND | Outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | (plus the string-edge gate fix): `protocol_clause::content` refuses every uncovered node kind, construct and non-literal body, and checks the grammar-mandatory rest. The split was verified complete against the grammar (SR-772) |
| FND-002 | fixed | the `Origin::ProtocolAttempt` arm and the `Boolean` `Type` record. Reverting either reproduces `UnlocatedOccurrence` (SR-772 FND-001 covers the population record) |
| FND-003 | fixed | one `state_clause::frame_record` used by both the clause and the attempt |
| FND-004 | fixed | one source-ordered lowering pass, tested by `anchor_occurrence_ordinals_follow_source_order` |
| FND-005 | fixed | `let [clause_index] = indices[..] else` refuses `Ambiguous`, reached directly in `a_contract_entry_naming_two_clauses_of_one_name_refuses` |
| FND-006 | fixed | clause and attempt assembly errors gathered into one refusal |
| FND-007 | fixed | zips and `.get()` with an `InternalFault` for an empty slot; `attempt_spans` is `Option<Span>` |
| FND-008 | fixed | `contracts` read between `contracts [` and `]`, role read after `by` |
