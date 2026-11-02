---
id: TC-1063
title: "Protocol emission refusals, read-back equality and the reader's refusals"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: verifies
---
# TC-1063: Protocol emission refusals, read-back equality and the reader's refusals

## Description

Verify that S4 refuses a stale bound key and a missing temporal profile with existing catalog pairs and no package, that a protocol
with an S3 refusal emits no node of its own and no package for the unit while
the other protocols keep their results (TC-1297), that QSL's I2 reader returns the emitted
protocol equal to the in-process checked clause, that it refuses each mutated
emitted package with QSpec's code at QSpec's locus, and that emission has no
limit or depth of its own. Scope: FR-383-AC-17 to FR-383-AC-21 and FR-383-AC-32 to FR-383-AC-48.

Catches a stale key repaired by a name lookup, a protocol dropped silently
when its temporal profile is missing, a
partial package after a refusal, a read-back that differs from the checked
clause in set order or binder level, a mutation the reader admits, and an
emitter that refuses a deep or wide protocol.

Fixture **W** is TC-1061's.

## Test Procedure

Compile or read one unit or graph per row and assert the stated result.

### Refusals

| # | Unit or graph | Expected |
| --- | --- | --- |
| 1 | **W**'s checked graph with the required temporal clause's key re-keyed after binding | `missing_declaration`/`missing-name` at the `requires temporal` member, naming the bound key; no protocol node, no package |
| 2 | the same with the model relationship key, then with an attempt's operation anchor key | `missing_declaration`/`missing-name` at the relationship member, then at the attempt; no package |
| 3 | **W** with an `await`, then a unit with only a `compensate`, in a package that selects no `temporal_profile` | `missing_declaration`/`missing-selection` at the first such construct; no package |
| 4 | **W** in a unit that selects no protocol profile | no protocol node and no refusal for the declaration |
| 5 | a unit holding **W** and a protocol **X** whose join names a missing branch | **X** `Refused` with `missing_declaration`/`missing-name` at the label; **W** `Emitted` with nodes byte-identical to **W** alone; the aggregate `Refused` with no package and no `package_id`; **W** alone emits a package (FR-383-AC-18; the full procedure is TC-1297) |

### Read-back

| # | Unit | Expected |
| --- | --- | --- |
| 6 | **W** through QSL's I2 reader | admitted; each protocol node returned with the emitted key, tag, form, type, declaration, body, dependencies and occurrences |
| 7 | the protocol rebuilt from row 6's nodes against the in-process checked clause | equal keys, set order, ordered lists, binder levels, channel premises, queue bounds and derived causal relation |

### Reader refusals (the mutations of row 8 each recompute node and package identities unless stated)

| # | Edit to **W**'s package | Expected, from QSpec FR-464 to FR-466 |
| --- | --- | --- |
| 8a | `roles` out of ascending node-id order; a repeated `roles` entry | `invalid_semantic_graph` at the clause for the out-of-order set; `invalid_semantic_graph` at the repeated role member for the repeated entry |
| 8b | the queue of `Charges` removed; a second queue naming `Charges` | `invalid_semantic_graph` |
| 8c | the control `Finalized` removed from `Main`'s children but kept as a node | `invalid_semantic_graph` at the control |
| 8d | `Received` sequenced before `Requested` | `invalid_package`/`definition-cycle`, naming the controls on the cycle |
| 8e | a binding missing from `Charged`; two arguments of `Requested` swapped | `malformed_wire` |
| 8f | `Orders`' model target replaced with a record type node | `invalid_model_binding`/`malformed-declaration` at the target |
| 8g | a `Done` condition that reads `view` (model state) | `ill_typed`/`operator-ineligible` at the read |
| 8h | `Outcome`'s case list in descending label order | `invalid_semantic_graph` |
| 8i | the package without a `temporal_profile` while `Refund` stays | `missing_declaration`/`missing-selection` |

QSL's conformance run reads the QSpec protocol vectors file
`protocol-node-vectors.json` (under `proposals/checked-package-v2/fixtures`),
admits each positive package through QSL's I2 reader and refuses each
mutation with the recorded code and locus. It copies no vector. The test is
traced only once that file exists.

### Protocol term locations

| # | Unit or graph | Expected |
| --- | --- | --- |
| L1 | fixture E and protocol L (TC-1070) emitted and read through QSL's I2 reader | for every slot, the rows of its origin for an application number that application's visits in the slot walk (FR-383-AC-32) |
| L1a | row L1's package | no row of a slot's origin names an application the walk does not visit (AC-33) |
| L1b | row L1's package | `Refund.retry.relation` has one `earlier.tally` node with two retry-origin rows; `Partial.retry.relation` one `partialEarlier.tally` node with two of its own (AC-34) |
| L2 | row L1's package | each row's `Location` equals the in-process `Location` of the term at the position it numbers (AC-35) |
| L2a | row L1's package | the two `earlier.tally` rows differ, the first that of the term in `earlier.tally < 3` (AC-36) |
| L3 | a unit with no protocol slot expression | `protocol_locations` empty (AC-37) |
| L3a | a package with no checked-term data | `checked_term_data` absent (AC-38) |
| L4 | row L1's package and fixture E-C's | no two slots share an origin (AC-39) |
| L4a | Inspection of the emitter | no row derived from a source region, node order or operand position, and none defaulted (AC-40) |
| L5 | row L1's package with the highest-index retry-origin `earlier.tally` row removed | admitted by the reader (AC-41) |
| L6 | row L1's package with the ordinal-zero retry-origin `earlier.tally` row removed; with ordinals 0 and 2 only | refused `invalid_package`/`invalid-value` at `/checked_term_data/protocol_locations` (AC-42) |
| L6a | row L1's package with a retry-origin `earlier.tally` row duplicated | refused `invalid_package`/`conflicting-definition` at `/checked_term_data/protocol_locations/<index of the duplicate>` (AC-43) |
| L7 | fixture E-C emitted and read back | per-slot row counts equal the walk's visits, with `m.count` twice in `Composed.if`, `m.note` twice in `Composed.option`, `m.items` twice in `Composed.coll`, once in `Composed.let` (AC-44) |
| L7a | fixture E-C emitted and read back | each row's `Location` equals the checked clause's (AC-45) |
| L7b | fixture E-C emitted and read back | the unselected arms of `Composed.if` and `Composed.case` have rows with the checked clause's `Location` values (AC-46) |
| L8 | fixture E's package | the `earlier.tally` rows share one occurrence key with `term_ordinal` 0 and 1 (AC-47) |
| L8a | fixture E-C emitted | `m.count >= 1` is one node held by `Composed.shareA` and `Composed.shareB`; its group holds one row of each origin with `term_ordinal` consecutive from zero across both (AC-48) |

### Limits and nesting

| # | Unit | Expected |
| --- | --- | --- |
| 9 | a protocol whose controls outnumber `i2.nodes`, emitted with the limit raised | emitted; read under the default limit it returns `StageFailure::Limit` of kind node count, setting `i2.nodes`; read with the setting raised it verifies |
| 10 | a `parallel` inside a `repeat` inside a branch of another `parallel`; a `sequence` of 5,000 children | each emits with no limit or depth refusal at S4 |
| 11 | a protocol whose `over` input is `Boolean`, `Option<M::T>` or `Sequence<M::T>`; a clause with its `owner_model` removed, copied to a `role`, written as an external key, left out of `dependencies`, pointed at a Boolean scalar node | the first is refused `unsupported_construct`/`not-yet-implemented` naming `NoOwnerModel` and writes no clause; the reader refuses the rest `malformed_wire`, `malformed_wire`, `malformed_wire`, `invalid_semantic_graph`, and `ill_typed`/`operator-ineligible` |

## Expected Results

- Each row asserts code, cause, locus and retained members as fields, never
  message text.
- Rows 1 to 3 and 5 produce no package (row 5 still reports **W**'s result). Row 4 produces a package without a protocol
  node.
