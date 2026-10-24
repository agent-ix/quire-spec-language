---
id: SR-9091
title: "Code review of quire-spec-language PR #833: systems interfaces first and the S2 StateModel absence"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@b23386bb470bd029d311ccf7ed7f50b857589ea6; PR #833 diff origin/impl/business-model-meanings...feat/systems-interface-first-and-s2-arm: qsl-semantics/src/model/systems.rs, qsl-semantics/tests/it/{model_systems,model_intake,model_population}.rs, xtask/src/seam_probe.rs, with the rust-review lane folded in"
review_set: subset
---
# Code review of quire-spec-language PR #833

## Summary

Ticket: QSL-497. This review covers the code-review and rust-review lanes in one pass. It reports only defects that give a wrong result, a panic, a failed AC or a broken gate. The oracle is this repo's FR-086, FR-084 and FR-300 rows and TC-233, TC-234, TC-236, TC-241, TC-410 and TC-791. QSpec FR-152 is cited by id only.

What was checked and found correct:

- `classify` now has a first phase that charges one `systems.kind` unit per interface and fills `interface_types`. A port with a direction and a Part owner classifies `Port` only when its `value_type` is in that set. Otherwise it records wrong-export (required Interface) and no kind. QSpec FR-152 names one `systems.kind` charge point for every record, so a per-kind charge point would not be faithful.
- The order is load-bearing. Ports read `interface_types`, which only the first phase fills. A mutant that collects interfaces during the record scan but charges them after the endpoints, or that runs one loop in key order, refuses the TC-241 Port (`model.M.port` sorts before `model.Z.iface`).
- Consumers of `classify`: `admitted.rs` turns every classify refusal into an admission refusal. `composed_domain_models.rs` and the model_intake tests now declare `Flow` as an interface. The gate compiled and ran them.
- The fixture edits match the repo rows. Under FR-086-AC-5, y07's `model.Count` and the two intake tests' `object_type` Flow are not Interfaces, so their ports were non-conformant. y10 counts 3 charges because Flow is now an Interface.
- TC-410 recomputes the universe digest independently. `serde_json` has no `preserve_order` in this workspace, so `to_vec` writes sorted keys, which is JCS for these ASCII-only strings. The three universes are asserted different.
- `forms_s2_report` still fails with `SeamProbeMismatch` when a two-segment `state_model::f(..)` call arm appears in the dispatch match. The new test edits a real `Body` arm in a scratch copy and asserts the error. `state_model_arm_report` is still called by the probe's main path (seam_probe.rs:531).
- Gate qsl497b-gate4: fmt, clippy (all features, all targets), the per-crate and workspace tests, xtask lib tests and spec validation, EXIT 0. It finished 24 s before b23386bb4 was committed, and the worktree is clean.

## Verdict

PASS. There are no blocking findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Recorded

- medium, recorded: a port whose interface type names another endpoint, a relationship or an allocation gets `actual kind none` (or `Port` only when the named endpoint sorts earlier). Its own kind is not reported. The refusal itself is correct. qsl-semantics/src/model/systems.rs:297.
- low, recorded: the S2 check matches only two-segment direct-call arms, so `crate::state_model::f(..)` or a block-bodied arm would not be caught.
- low, recorded: the gate ran the xtask lib tests, not `make seam-probe`. The lib test runs `state_model_arm_report` over the real tree, which covers the changed function.
