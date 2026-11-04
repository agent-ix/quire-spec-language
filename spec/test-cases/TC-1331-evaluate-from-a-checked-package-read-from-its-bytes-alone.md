---
id: TC-1331
title: "Evaluate from a checked package read from its bytes alone"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-531
    type: verifies
---
# TC-1331: Evaluate from a checked package read from its bytes alone

## Description

Verify the bytes-only entry. Scope: FR-531-AC-1 to FR-531-AC-12. **R** is the package of FR-517; the test crate depends on `quire-contract-model`, `qsl-package` and `qsl-eval` only.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1, 3 | in a child run given only **R**'s handoff directory: `read_checked_package`, `BarePackage::new`, one temporal call, one slot call | both return a result; the source unit is never compiled |
| 2 | 2 | compile-fail probes of a `BarePackage` from bytes, from a decoded graph and from a term; API-surface listing of the signatures | each fails to compile; no signature names `CheckedPackage`, `Emission` or `AdmittedPackageRead` |
| 3 | 4 | each clause form and each slot kind **R** holds; a non-temporal node; a non-protocol node | results; `clause-root-unresolved` and FR-390-AC-4's refusal, empty charge logs |
| 4 | 5 | slot call with no supply, foreign rows, unknown rows, a missing dependency; the temporal call with no supply | `MissingIdentityWitness`, `ForeignIdentityWitness`, `UnknownIdentityWitness`, `DependencyUnheld` as exit-21 `Admission`, meter untouched; the temporal call returns |
| 5 | 6 | a package without `temporal_bindings`; without a profile row; with an unknown profile definition; with an infinite-trace profile | `Unavailable` with the clock `Missing`; `clause-unreadable` `profile-selection`; `clause-unreadable` `profile-unknown`; `not-finite-window-profile` |
| 6 | 7 | encode **R**'s results; search for the clause names, `source:` labels and source map regions; read the progress binding | none present; the binding holds `package.name()` |
| 7 | 8 | **R**'s directory: read `identities.json`; compare with the encoding of the in-process rows; write twice; omit the file from the supply | slot call evaluates; bytes equal; two writes equal; `MissingIdentityWitness` for the slot call only |
| 8 | 9 | enumerate the public methods of `EvidencePackage` and `CheckedPackageEvaluation` against the inventory table | one row each; an unlisted method fails the test |
| 9 | 10 | charge logs of bare and in-process calls; `new`, `with_supply`, view creation | equal; no `Meter` taken; a refusing call leaves the meter identical |
| 10 | 12 | `ReadIdentityRows::read` over a non-JSON file, an unknown or repeated member, another `format`, a row the package does not hold, a form that does not match its identity | `Malformed`, `Malformed`, `Format`, `Unjoined`, `Unjoined`; no rows |
| 11 | 11 | read FR-515, FR-516, FR-520 | the out-of-scope sentence is gone; each has a "Consumer entry" section naming FR-531 |

Tag the test `#[trace("FR-531-AC-1", ..., "FR-531-AC-12", "TC-1331")]`.

## Expected Results

Each row's Expected column.
