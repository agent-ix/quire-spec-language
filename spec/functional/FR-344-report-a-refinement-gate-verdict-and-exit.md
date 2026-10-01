---
id: FR-344
title: "Report every refinement case result with one verdict and its QSpec FR-301 exit code"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-034
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-343
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-345
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-301
    type: depends_on
---
# FR-344: Report every refinement case result with one verdict and its QSpec FR-301 exit code

## Description

When a refinement gate (FR-340, FR-345) has a result for every case and
every malformed pair, it SHALL write one report listing every result, give
one verdict for the run, and exit with the QSpec FR-301 exit code for that
verdict.

## Inputs

The case results of FR-343 or FR-345, and FR-340's pair-level tool-failure
results.

## Outputs

- A JSON report on stdout with, in this member order: `cases`, a list of
  every result, each holding the case's name (for a versioning case, the
  prior and superseding `RawSourceRef`s and the selection; for a layering
  case, FR-345's name), each side's class, stage and codes, any limit
  reached, and the result; then `verdict`; then `exit`.
- The process exit code.

## Behavior

- The report SHALL list every result, ordered by the case's name: its
  members compared in the order the Outputs give them, each as its bytes.
  A pair-level tool-failure result sorts by its `pair.json` path.
- The gate SHALL write byte-equal reports for two runs over one corpus.
- If either side's class of a case is `incomplete`, then the report SHALL
  name, in that case's entry, the limit reached, its value and the case member that raises
  it (`limits`, `observation_limits`, `model_limits` or `accounting`).
- The verdict SHALL be the highest-severity result present under QSpec
  FR-301's order, and the exit code QSpec FR-301's code for it:

| Results present | Verdict | Exit |
| --- | --- | --- |
| any `tool failure` | tool failure | 30 |
| any `unresolved (unsupported)` | unsupported | 21 |
| any `unresolved (incomplete)` | incomplete | 22 |
| any `regression` | violation | 10 |
| otherwise | success | 0 |

- The report SHALL list every `regression` whatever the verdict, so an
  unsupported, incomplete or tool-failure result never hides a regression.
- The report SHALL keep `holds` and `not applicable` as distinct results.
- The report SHALL never list an unresolved result as `holds`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-344-AC-1 | For result sets containing, respectively, only `holds`; `holds` and a `regression`; a `regression` and an `unresolved (incomplete)`; that plus an `unresolved (unsupported)`; and that plus a `tool failure`, the verdicts are success, violation, incomplete, unsupported and tool failure, with exits 0, 10, 22, 21 and 30. | Test (TC-866) |
| FR-344-AC-2 | In the result set whose verdict is unsupported and which holds one `regression`, the report lists that `regression` with its name, both classes and the superseding codes. | Test (TC-866) |
| FR-344-AC-3 | A corpus of three pairs, created on disk in an order other than their names' order, yields a report ordered by case name; two runs give byte-equal reports. | Test (TC-866) |
| FR-344-AC-4 | A case whose superseding run exceeds its `accounting` work budget names, in its entry, `work_units`, the budget's value and the member `accounting`. | Test (TC-865) |
| FR-344-AC-5 | The seeded-regression test corpus of TC-864 reports verdict violation, exit 10, and exactly one `regression`, naming the seeded case. | Test (TC-864) |

## Dependencies

- FR-343 and FR-345 (case results), FR-340 (pair-level results).
- QSpec FR-301 (exit codes and severity order).

## References

- ADR-017 §2 RF-4 (amended 2026-10-01: the reached limit of an incomplete
  case), RF-5.
- Specification tickets QSL-386, QSL-387; implementation tickets QSL-40,
  QSL-39.

## Status

Specified; not yet implemented -- TC-864 to TC-866 planned.
