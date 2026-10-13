---
id: SR-8720
title: "Code review of quire-spec-language PR #808 (QSL-744): normalize-stage declaration refusals keep their node's origin"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@88cdadd279a4247945340f3bd8ae3b982c85e1cf; qsl-semantics/src/model/normalize.rs, qsl-semantics/src/model/intake.rs, qsl-semantics/src/model/admitted.rs, qsl-semantics/src/check/field_refinement.rs, qsl-semantics/tests/it/tc235_origin_retained.rs, qsl-semantics/tests/it/model_conformance.rs"
review_set: subset
---
# Code review of quire-spec-language PR #808

## Summary

Ticket: QSL-744. Method: code-review with its rust-review lane and a test-oracle check. PR #808 (base `impl/business-model-meanings`, merge base d5d81f8a8) routes every normalize-stage `malformed-declaration` through `malformed_at`, so the refusal carries the artifact and span of the refused node's FCD origin. It also changes the reader so a node that already has an intake refusal still records its origin, widens the population-member rule, and locates the scalar-domain refusal at the scalar type. Reviewer-only: no build or test run. The coder's gate logs were read.

Examined:
- the ten normalize.rs sites now calling `malformed_at` (normalize.rs:1120, 1171, 1188, 1297, 1312, 1690, 1766, 1797, 2042, 2402, 2414, 2438) and `malformed_at`/`origin_of` (normalize.rs:1860-1883) (examined)
- `read_domain_package`'s origin loop and its refusal suppression, intake.rs:4297-4451 (examined)
- `build`'s intake-refusal branch and `intake_state_machine_refusals`, normalize.rs:2649-2659 and 1720-1734 (examined)
- every consumer of `DomainPackage::original` (admitted.rs:386, normalize.rs:1880, qsl-bench model.rs) (examined)
- `identity_refusal`, admitted.rs:169-183 (examined)
- `field_domain_type` and its callers, field_refinement.rs:96-310 (examined)
- the population arm and `declares`, normalize.rs:1845 and 2433-2464; `RecordIndex` object-type flags, index.rs:397-446 (examined)
- `tc235_origin_retained.rs` (all 12 test functions) and `r08i_a_malformed_scalar_domain_refuses_rather_than_panicking` (examined)
- xtask TC-255 and TC-260 scans, xtask/src/typestate_scan.rs:1013-1061 and 1334-1404 (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The intake.rs change (a node with an intake refusal now records its origin; its own reader defects stay suppressed) is exercised by only one vector: the `operations: []` half of `a_state_machine_missing_required_members_keeps_its_origin`. That half asserts `found[0]` only, with no count and no check of the other refusals. That run reaches `build`'s intake branch (intake refusals for the now-dangling triggers plus `intake_state_machine_refusals`), which is exactly where a double report or a masked refusal from this change would appear, and the test could not see either. Reading the code, the reported set is unchanged: a refused node's reader errors were never run before and are all dropped now. But no test pins that. Fix: assert the full refusal list (code, cause, node, member) for that run, as the `transitions: []` half does with its count. | qsl-semantics/tests/it/tc235_origin_retained.rs:211-220; qsl-semantics/src/model/intake.rs:4326-4332, 4442-4447; qsl-semantics/src/model/normalize.rs:2649-2659 |

## Verdict

Everything else in the change is correct:
- **All ten normalize sites.** No production `IntakeMalformedDeclaration { .. artifact: None .. }` remains in normalize.rs, systems.rs, index.rs, admitted.rs or check/. The remaining `artifact: None` literals are in `#[cfg(test)]` modules (intake.rs after line 4699, normalize.rs after 4134) and in `refusal.rs`'s `fixtures::exhaustive_samples`. Intake's own `malformed_declaration(...)` calls with `None` name the package document (`node: "$"`), not a declaration. The only unit `MalformedDeclaration` constructors left are normalize.rs:1089 (empty selection) and 1992 (empty key), each with its stated reason.
- **Each refusal keeps the right node's origin.** The operation an event type declares is located at `operation.key`, not the event. The scalar refusal is located at the scalar key taken from `redefined.value_type.as_package()`. The relationship ends use `relationship.key`.
- **intake.rs behaviour change.** Before, a node in `refused_nodes` was skipped, so none of its reader errors were reported and no origin was recorded. Now the loop runs and records originals, and `!node_refused` drops every error it raises. A non-refused node's errors are filtered exactly as before. So the reported set is unchanged. Nothing masks a refusal, and nothing is reported twice. Recording originals for a refused node only changes the artifact and span of refusals that normalization adds for it. With intake refusals present, the only such refusals are `intake_state_machine_refusals`. Other consumers of `original()` cannot see such a package: `AdmittedPackage` comes after a normalization that refuses, and `qsl-bench` uses a clean package. The closure can stop partway and leave a partial set of member originals, which is harmless because a missing original only means `None`.
- **Population rule.** `!is_object_type && declares` sends a state machine, record value type, event type, scalar, persistence interface, namespace or member node to `malformed-declaration`, and an undeclared name to `missing_declaration`. A process still counts as an object type (index.rs:442-444), so TC-235 M10 run 9 (`[Fulfilment]`) is still admitted. `declares` is a linear scan, but it only runs for members that already fail.
- **Rust lane.** No panic surface, `unsafe`, integer conversion or new string comparison. `identity_refusal(&package, ..)` borrows an owned `DomainPackage`, so clippy's `needless_borrow` does not apply. Nothing new is feature-gated.
- **xtask scans (after #807).** The PR mints no `NodeKey` (TC-255) and adds no struct field under `qsl-semantics/src/model/` (TC-260). `ScalarDomain` is a type alias in `check/`. Neither scan can trip.
- **Overlap.** No file is shared with #805 (merged onto the base after this PR's merge base) or with open #807 (qsl-replay, xtask, FR-088, TC-260). GitHub reports the PR as MERGEABLE.

Test oracles: the red log shows 10 of 13 functions failing on the old code at the artifact assertion (line 100) or at the cause match (population). Three passed: the generated-origin negative, the old `a_process_without_identity_keeps_its_origin`, and the occurrence vector. The process vector was merged into `a_process_without_identity_or_process_supertype_keeps_its_origin`, whose sites (normalize.rs:1766, 1797) had `None` before. The current occurrence vector can only be produced by normalize.rs:1224, which had `None` before, so it fails on the old code as it stands now. The red run must have used an earlier body, which matches the coder's account. Every positive vector pins a distinct line per node, so a refusal using the wrong node's origin fails.

Gate logs (read, not re-run): fmt clean. Clippy finished with no warnings, but the log does not show the command, and `make ci` also runs `--all-features`. qsl-semantics lib: 518 passed, 1 ignored. it: 609 passed, 2 ignored. Spec validation: warnings only in untouched FRs, no errors, and the log records no exit status. `make ci` has not been run and is required before merge.

**Mergeable once FND-001 is fixed.**
