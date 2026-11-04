---
id: SR-20261104153718
title: 'Code review of PR #2300: QSL-1645 xtask lib scan drift, code-review'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@6c7714f17; git diff origin/main...6c7714f17 (PR #2300, branch peter/qsl-1645-red-tests): qsl-replay/src/execute.rs, qsl-replay/src/value_check.rs, qsl-semantics/src/check/capability.rs, xtask/src/checked_input.rs, xtask/src/seam_probe.rs, xtask/src/typestate_scan.rs; ticket QSL-1645'
review_set: subset
date: '2026-11-04'
---

## Summary

Ticket: QSL-1645. PR: quire-spec-language#2300, head 6c7714f17. Method: code-review with the rust-review lane. The PR fixes 9 red `xtask --lib` architecture-scan tests. Each scan table change follows code that merged PRs moved or added. The checked-input gate gains a 5-name exemption for `quire-contract-model` addressing values.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| deny moved to replay_with_cancel_and_temporal_selections | qsl-replay/src/execute.rs:1142 | examined |
| deny moved to ValueTarget::evaluate_with_case_budget | qsl-replay/src/value_check.rs:364 | examined |
| Capability::applies_to_declaration hand-written match | qsl-semantics/src/check/capability.rs:196-225 | examined |
| ADDRESSING_TYPES and is_representation | xtask/src/checked_input.rs:81-95,393-401 | examined |
| AC-3 look-alike plants | xtask/src/checked_input.rs:1184-1192 | examined |
| seam location replay_with_cancel_and_temporal_selections | xtask/src/seam_probe.rs:245 | examined |
| NodeKind read from quire-checked | xtask/src/seam_probe.rs:1612-1648 | examined |
| STAGE_CONSTRUCTORS additions | xtask/src/typestate_scan.rs:714-797 | examined |
| RAW_WIRE_TYPE_FIELDS additions | xtask/src/typestate_scan.rs:1056-1094 | examined |
| quire-contract-model addressing types at the locked rev 64de9ec | quire-contract-model/src/checked_package/shared.rs, identity.rs, checked_package/v2/temporal_binding.rs | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The two new AC-3 plants show only that the exemption admits `CheckedNodeId` and `SourceIdentity`. No new plant shows that a node-bearing `quire-contract-model` type, imported by `use` in the same way, still fails. An example is a v2 package or a raw node next to `CheckedNodeId`. Today, only the unchanged AC-1 `ContractPackage` plant (full path) proves the remaining set still fires. Add one AC-1 plant such as `use quire_contract_model::{CheckedNodeId, ContractPackage}; pub fn plant(c: &CheckedNodeId, p: &ContractPackage) {}` that expects exactly one finding naming `ContractPackage`. | xtask/src/checked_input.rs:1184-1192 |

## Verdict

Mergeable. The one finding is a low test-strength gap.

- **Checked-input widening is justified.** At the locked `quire-contract-model` rev 64de9ec, each of the five types is identity or clock data. `CheckedNodeId` holds {package, path segments of name, nested id or index, ordinal}. `CheckedOccurrence` holds {role, ordinal}. `SourceIdentity` holds {document id, revision}. `ClockConfiguration` is a closed enum of clock parameters. `TemporalBinding` holds {declaration id, occurrence, clock name, clock}. None of them carries a checked node, term or package. The gate's intent is unchanged: a stage entry cannot receive executable IR that skipped the I2 reader. These types are already in shipped `qsl-eval` and `qsl-replay` signatures under FR-516, FR-522 and FR-514 (`evidence/request.rs`, `evidence/binding.rs`, `evidence/package.rs`, and `source_inventory.rs::qsl_inventory`). The alternative would be QSL newtypes that mirror IR keys, which would add nothing.
- **The exemption is an exact name match.** It checks the last segment of a path rooted at `quire_contract_model`. `use … as` renames resolve to the original name, so renaming a package to `CheckedNodeId` does not slip through. Generic arguments are collected as separate paths, so `Vec<(CheckedNodeId, ContractPackage)>` is still flagged.
- **The deny moves are true relocations.** The `FamilyOutcome` match (execute.rs:1252-1298) sits in `replay_with_cancel_and_temporal_selections`, and `replay_with_cancel` only delegates to it. The `ValueTarget` match (value_check.rs:527) sits in `evaluate_with_case_budget`. Its seam location was already correct, and only the attribute moved.
- **The hand-written `applies_to_declaration` is exhaustive.** It covers all 12 variants, has no wildcard, and is equal row for row to the old macro table. The existing FR-507 full-matrix test (capability.rs:330-378, 14 applicable pairs) still pins it. Under `--cfg seam_probe`, the match leaves `__SeamProbe` unhandled, as the S7 location requires.
- **NodeKind is read from `quire-checked` through cargo metadata.** It is the same mechanism the test already used for `WrongSnapshotCause`, and the check is not deleted.
- **TC-244 additions are real S3/S4 constructors.** They are four `PackageDeclarations` methods in `check/mod.rs` that return `CheckedGraph`, and two `Resolution` methods in `spine/lifecycle.rs` that return `CheckedPackage`.
- **TC-260 additions are real raw-wire lookups.** They are `TypeNodeReader.type_meanings` and `TypeNodeReader.rational_types` in `model/intake.rs`.
- **Tests:** `cargo test -p xtask --lib` and the qsl-semantics capability tests are reported in the reviewer's handback.
