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
limit or depth of its own. Scope: FR-383-AC-17 to FR-383-AC-21 and FR-383-AC-32 to FR-383-AC-61 and FR-383-AC-63.

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

QSL's conformance run reads the QSpec protocol-node mutation manifest
`protocol-node-mutation-manifest.json` (version
`quire.protocol-node-mutation-manifest/v1`, under
`proposals/checked-package-v2/fixtures`) and the `protocol-node-layout-rows.json`
rows it defers to, in place, and copies no vector, base fixture or edit into
QSL. For every row of the manifest's `rows` array and then of the layout file's
`rows` array, in file order, the run applies the row's RFC 6902 `edit` (a list
of operations; every executable row uses this form) to the row's `base` fixture
and reads the result through QSL's I2 reader:

- an `admit` row is admitted;
- a `refuse` row is refused with the row's `code`, `cause`, `stage` and `locus`
  exactly, the locus read by the row's `locus_basis`: `node`, `target`,
  `field`, `dependencies_list` and `schema_error_path` as the row states, with
  `document_pointer` where the row gives one, and `not_applicable` for the rows
  that have no locus; a `step_alone` row is read at its step alone and a
  `retains` row also asserts what the refusal retains;
- a `property` row's `assertions` hold.

The reader's deliberate order is preserved (QSpec FR-464 "Reader order"): a row
expecting a later semantic refusal is not satisfied by an earlier wire or
schema refusal, and a row asserts only the first defect in reader order. The
executed set is the manifest rows plus the layout rows (25 `admit`, 90 `refuse`
and 4 `property` rows at QSpec main); a `pending` entry is executed through its
same-id layout row, and `CH14a` through `CH14a-1` to `CH14a-8`. The run records
the executed count for each of `admit`, `refuse` and `property`, fails when
any is zero or when an executable row was not executed, and reports a `pending`
or `later` entry with no executable row as not executed with its count, never
as passing. The test is traced `TC-1063` and `FR-383-AC-20`.

### Protocol term locations

| # | Unit or graph | Expected |
| --- | --- | --- |
| L1 | **W**, fixture E and protocol L (TC-1070) emitted and read through QSL's I2 reader | for every slot, the rows under its site name for an application number that application's visits in the slot's walk (FR-383-AC-32) |
| L1a | row L1's package | no row under a slot's site name names an application the slot's walk does not visit (AC-33) |
| L1b | row L1's package | the retry origin of `Refund` holds one `earlier.tally` node with two rows; the retry origin of `Partial` one `partialEarlier.tally` node with two of its own (AC-34) |
| L2 | row L1's package | each row's `Location` equals the checked clause's `Location` of the term at the position it numbers (AC-35) |
| L2a | row L1's package | the two `earlier.tally` rows differ, the first that of the term in `earlier.tally < 3` (AC-36) |
| L3 | a unit with no protocol slot expression | `protocol_locations` empty (AC-37) |
| L3a | a package with no checked-term data | `checked_term_data` absent (AC-38) |
| L4 | fixtures E, L and E-C emitted | every row of every slot that holds an application is under a type-declaration origin whose `name` is that slot's site name (AC-39) |
| L4b | fixtures E, L and E-C emitted | no two slots have the same site name (AC-40) |
| L4c | fixtures E, L and E-C emitted | no row has a `protocol_attempt` origin (AC-41) |
| L4d | fixture L emitted; its variant whose `Got` ends call one identity function | case guards under `... Decide case yes guard` and `... case no guard`, `visible[0]` under `... decision 2641..2650` and no other `decision` origin; the variant's ends under `... Got related 0 from` and `... related 0 to` (AC-63) |
| L4a | Inspection of the emitter | no row derived from a source region, node order or operand position, and none defaulted (AC-42) |
| L5 | row L1's package with the retry-origin `earlier.tally` row of `Refund` keyed to occurrence 1 removed | admitted by the reader (AC-43) |
| L6 | row L1's package with the occurrence-1 retry-origin `earlier.tally` row's `term_ordinal` changed from 0 to 1 | refused `invalid_package`/`invalid-value` at `/checked_term_data/protocol_locations` (AC-44) |
| L6a | row L1's package with the occurrence-1 retry-origin `earlier.tally` row rekeyed to occurrence 0 at `term_ordinal` 2 | refused `invalid_package`/`invalid-value` at `/checked_term_data/protocol_locations` (AC-45) |
| L6b | row L1's package with a retry-origin `earlier.tally` row duplicated next to the original | refused `invalid_package`/`conflicting-definition` at `/checked_term_data/protocol_locations/<index of the duplicate>` (AC-46) |
| L7 | fixture E-C emitted and read back | per-slot row counts equal the walk's visits, with `m.count` twice in `Composed.if`, `m.note` twice in `Composed.option`, `m.items` twice in `Composed.coll`, once in `Composed.let` (AC-47) |
| L7a | fixture E-C emitted and read back | each row's `Location` equals the checked clause's (AC-48) |
| L7b | fixture E-C emitted and read back | the unselected arms of `Composed.if` and `Composed.case` have rows with the checked clause's `Location` values (AC-49) |
| L8 | fixture E's package | the `earlier.tally` rows are keyed to occurrences 0 and 1 in visit order, each with `term_ordinal` 0 (AC-50) |
| L8h | fixture E checked; fixture E-C's `Tried` and `Picked` controls checked | `earlier.tally` has exactly two `expression` occurrences, 0 in `earlier.tally < 3` and 1 in `earlier.tally + 1`; `target >= 0` exactly two, 0 in the trigger guard and 1 in the retry relation; E-C's `order >= 0` exactly two, 0 at `Tried` and 1 at `Picked` (AC-61) |
| L8a | fixture E emitted | `target >= 0` is one node held by `Refund`'s retry relation and trigger guard (AC-51) |
| L8b | fixture E emitted | its trigger-guard row is keyed to occurrence 0 and its retry row to occurrence 1, one row per group (AC-52) |
| L8c | fixture E emitted | both rows have `term_ordinal` 0 and distinct occurrences (AC-53) |
| L8d | fixture E emitted | each of the two rows carries the location of the term in its own slot (AC-54) |
| L8e | fixture E-C emitted | `order >= 0` is one node held by the constraints of `Tried` and `Picked` (AC-55) |
| L8f | fixture E-C emitted | its `protocol Composed run Main Tried body` row is keyed to occurrence 0 and its `protocol Composed run Main Picked body` row to occurrence 1, one row per group (AC-56) |
| L8g | fixture E-C emitted | both rows have `term_ordinal` 0 (AC-57) |
| L7c | fixture E-C emitted | no row names a `Local` read, a literal, a `let`, a conditional or a case node (AC-59) |
| L7d | fixture E-C emitted | each arm of `Composed.case` holds exactly one row (AC-60) |
| L9 | **W**, fixtures E, L and E-C emitted | for every application and slot, the (occurrence ordinal, `term_ordinal`) pairs along the slot walk's visits strictly increase (AC-58) |

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
