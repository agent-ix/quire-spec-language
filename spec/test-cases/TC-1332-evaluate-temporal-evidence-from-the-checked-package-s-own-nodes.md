---
id: TC-1332
title: "Evaluate temporal evidence from the checked package's own nodes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-532
    type: verifies
---
# TC-1332: Evaluate temporal evidence from the checked package's own nodes

## Description

Verify temporal evidence evaluated from a bare package and its equality with the two source routes. Scope: FR-532-AC-1 to FR-532-AC-9. **R** is the package of FR-517.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1 | **R**'s four clauses over three packages; requests for `True`, `False`, `Pending`, each activation state and each FR-523 `Incomplete` counter; the three routes | equal results in field and canonical bytes, charge log, `usage`, ledgers and `result_identity` |
| 2 | 2 | one clock row, one node id, one instance-bound atom, one operation, one interval, one operand edge replaced in turn in a reader-admitted package | `from_package` results differ from the unmutated ones; the in-process routes refuse `clause-unreadable` for the formula or operation cases |
| 3 | 3 | one authored clause per operator of the temporal list over `examples/config-version`, a distinguishing trace | equal verdict, settlement, support and usage on all three routes |
| 4 | 4 | the FR-515-AC-22 clause; a clause with a non-pre-order arena | `{0, 3}` on all routes; valuations keyed by pre-order position; the arena-keyed request reads another atom |
| 5 | 5 | ledgers and result identities of the three routes; a ledger under the labels | `package.name()` component, byte-equal; the old ledger is another binding and is not read |
| 6 | 6 | duplicate `{ declaration, occurrence }` name rows | `AmbiguousName`, nothing charged; no other variant for any package |
| 7 | 7 | unbounded operator, timed interval, cycle, out-of-range operand, window aggregate, count mismatch; their readable counterparts; an overflowing tick sum | `clause-unreadable`, empty charge log; results; `history_span` `Incomplete` |
| 8 | 8 | two clauses of different nodes; a foreign `binding`; several occurrences with and without a selector; ordinals 7 then 3 | own rows; `binding-mismatch`; `occurrence-required`, `occurrence-unknown`; paired by key |
| 9 | 9 | search the evaluator and constructors for source activation, profile and clock reads and for `SourceIdentity` in `LedgerBinding` | none outside the cross-check |

Tag the test `#[trace("FR-532-AC-1", ..., "FR-532-AC-9", "TC-1332")]`; row 1 also carries FR-516-AC-18.

## Expected Results

Each row's Expected column.
