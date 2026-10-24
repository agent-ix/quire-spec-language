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
with an S3 refusal emits nothing, that QSL's I2 reader returns the emitted
protocol equal to the in-process checked clause, that it refuses each mutated
emitted package with QSpec's code at QSpec's locus, and that emission has no
limit or depth of its own. Scope: FR-383-AC-17 to FR-383-AC-21.

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
| 3 | **W** with an `await`, then a unit with only a `compensate`, in a package whose lock selects no `temporal_profile` | `missing_declaration`/`missing-selection` at the first such construct; no package |
| 4 | **W** in a unit that selects no protocol profile | no protocol node and no refusal for the declaration |
| 5 | a unit holding **W** and a protocol whose join names a missing branch | the S3 refusal (`missing_declaration`/`missing-name` at the label); no protocol node of either protocol; **W** alone emits |

### Read-back

| # | Unit | Expected |
| --- | --- | --- |
| 6 | **W** through QSL's I2 reader | admitted; each protocol node returned with the emitted key, tag, form, type, declaration, body, dependencies and occurrences |
| 7 | the protocol rebuilt from row 6's nodes against the in-process checked clause | equal keys, set order, ordered lists, binder levels, channel premises, queue bounds and derived causal relation |

### Reader refusals (the mutations of row 8 each recompute node and package identities unless stated)

| # | Edit to **W**'s package | Expected, from QSpec FR-464 to FR-466 |
| --- | --- | --- |
| 8a | `roles` out of ascending digest order; a repeated `roles` entry | `invalid_semantic_graph` at the clause |
| 8b | the queue of `Charges` removed; a second queue naming `Charges` | `invalid_semantic_graph` |
| 8c | the control `Finalized` removed from `Main`'s children but kept as a node | `invalid_semantic_graph` at the control |
| 8d | `Received` sequenced before `Requested` | `invalid_package`/`definition-cycle`, naming the controls on the cycle |
| 8e | a binding missing from `Charged`; two arguments of `Requested` swapped | `malformed_wire` |
| 8f | `Orders`' model target replaced with a record type node | `invalid_model_binding`/`malformed-declaration` at the target |
| 8g | a `Done` condition that reads `view` (model state) | `ill_typed`/`operator-ineligible` at the read |
| 8h | `Outcome`'s case list in descending label order | `invalid_semantic_graph` |
| 8i | the package's lock without a `temporal_profile` while `Refund` stays | `missing_declaration`/`missing-selection` |

QSL's conformance run reads the QSpec protocol vectors file
`protocol-node-vectors.json` (under `proposals/checked-package-v2/fixtures`),
admits each positive package through QSL's I2 reader and refuses each
mutation with the recorded code and locus. It copies no vector. The test is
traced only once that file exists.

### Limits and nesting

| # | Unit | Expected |
| --- | --- | --- |
| 9 | a protocol whose controls outnumber `i2.nodes`, emitted with the limit raised | emitted; read under the default limit it returns `StageFailure::Limit` of kind node count, setting `i2.nodes`; read with the setting raised it verifies |
| 10 | a `parallel` inside a `repeat` inside a branch of another `parallel`; a `sequence` of 5,000 children | each emits with no limit or depth refusal at S4 |

## Expected Results

- Each row asserts code, cause, locus and retained members as fields, never
  message text.
- Rows 1 to 3 and 5 produce no package. Row 4 produces a package without a protocol
  node.
