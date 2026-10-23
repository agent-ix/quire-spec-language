---
id: SR-12364
title: "Code review of quire-spec-language PR #1361: call_site selects process and state-machine operations (QSL-976)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@f25a87280; git diff origin/main(e5554bc9f)...f25a87280: qsl-replay/src/call_site.rs, qsl-replay/src/proof_result.rs, qsl-replay/src/spine/clause/tests.rs, qsl-replay/src/spine/clause/tests/call_site_owners.rs, qsl-semantics/src/check/lowering/model.rs, qsl-semantics/src/check/mod.rs; read for context: qsl-semantics/src/check/probabilistic/model.rs, qsl-replay/src/execute/frame.rs, qsl-replay/src/spine/clause.rs, FR-121, FR-116, FR-115, FR-185, TC-1162; agent-ix/quire-contract-codegen main src/replay/function.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-121
    type: reviews
---
# Code review of quire-spec-language PR #1361 (QSL-976)

## Summary

Ticket: QSL-976. Rust review folded in (`/rust-review` lane). The
`call_site` operation path is correct: `NotAnOperationOwner` is decided
before resolution, `AmbiguousOperation` after resolution and before the
frame lookup, codes and message prefixes match FR-121 and QSpec FR-323,
and `proof_result.rs` maps both new variants to `Declined`/`InvalidInput`.
The diamond `C` case and the inherited `ExpressLifecycle::place` both
resolve as FR-121-AC-26/27 require. Focused tests run at the head:
`cargo test -p qsl-replay --lib call_site` 30 passed.

The defect is in how the owner kinds were widened. The change edits the
shared `model_objects` / `object_types` tables (`model_object_tables` now
iterates `operation_owners()`). Those tables are read by more than the
operation path, so the widening also reaches field selection
(`field_domain`), the FR-185 probabilistic checks (`ProbabilisticInput`),
`object_type_key` and `population_domain_key`. Two of these were measured
with temporary local probe tests (reverted, not pushed).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `field_domain` now resolves a process or state-machine owner. FR-121 says a field selection resolves "an object type (not a process or a state machine)" and refuses `UnknownField` otherwise, keyed on a `model`/`object_type` node. Probe at the head: `FieldName` `Config::Flow.n`, `Config::OrderLifecycle.placed` and `Config::ExpressLifecycle.placed` each return `Ok(DomainKey::Node {..})` instead of `UnknownField`. Cause: `field_domain` reads `model_objects` and `object_types`, which now hold every operation owner. Fix: keep an object-type-only table for `field_domain` (and `object_type_key`, `population_domain_key`), and add a separate owner table read only by `resolve_operation`, `operation_frame`, `operation_is_ambiguous` and `declaring_object`. Add a test that `Config::Flow.n` refuses `UnknownField`. | qsl-semantics/src/check/mod.rs:3432-3447,3908-3920 |
| FND-002 | medium | FR-185 workloads change behaviour as a side effect. `ProbabilisticInput.model_objects` and `object_types` are the same widened tables, so `model_operations` now lists process and state-machine operations. Probe at the head: TC-620's `Steady` workload, over the `Service` package plus a process `Flow` declaring `go`, is refused `incomplete-workload` naming `Flow::go`. On main it checks, because the table held object types only. `weight`, `random` and `reward` on `Flow::go` now resolve too. FR-185 says only "every operation of a model" and does not name owner kinds. FR-187 step probabilities were never specified or tested for process or state-machine operations. The coder did not record this widening. Fix: keep `ProbabilisticInput` on object types (the narrower implementation from FND-001), or get a draco ruling that FR-185/FR-187 cover process and state-machine operations and add TC-620 cases. | qsl-semantics/src/check/mod.rs:2349-2350; qsl-semantics/src/check/probabilistic/model.rs:530-540,582-600,1055 |
| FND-003 | medium | Downstream break. `CallSiteRefusal` is not `#[non_exhaustive]` and gains two variants. CG main has an exhaustive match, `impl From<&CallSiteRefusal> for ReplaySettlement` at `src/replay/function.rs:935`, that lists every variant without a wildcard. CG depends on `qsl-replay` with `branch = "main"`, so CG stops compiling at its next lock update. The PR body says downstream matches need the two arms, but no CG ticket or PR is linked. Fix: open the CG adaptation (both arms map to `SetupRefused(refusal.code())`) and land it with this merge. | qsl-replay/src/call_site.rs:305-329 |
| FND-004 | low | Names and docs still say "object type" for tables that now hold every operation owner. These are `model_objects`, `object_types`, `names_non_object_declaration`, and the doc comments of `resolve_operation` ("the object type of that package"), `declaring_object` ("no object type of that package"), `object_type_key`, `model_object_tables` and `model_non_object_tables` ("not object types"). `resolve_frame`'s doc in `qsl-replay/src/spine/clause.rs:1061` says "one of its object types". A later reader would take `object_types` as object-only, which is how FND-001 and FND-002 happened. Fix: rename the owner table, or split the tables as in FND-001, and update the docs. | qsl-semantics/src/check/mod.rs:536-548,3849-3856,3880-3885,3975,3420-3424,3450; qsl-replay/src/spine/clause.rs:1061 |

## Verdict

**Not mergeable.** The new `call_site` refusal path and its ordering are
right. FND-001 is a spec claim the code violates. FND-002 is an unspecified
change to which programs check, and it needs a narrower implementation or a
draco ruling. FND-003 must be landed with CG. The fix for FND-001 also fixes
FND-002 and most of FND-004: keep object-type tables for field, population
and probabilistic readers, and give the operation path its own owner table.

Coder deviations, judged:
- AC-27 enum owner untested: needs a test (see SR-12365 FND-001).
- `field_domain` / `population_domain` resolve process and state-machine
  owners: the `field_domain` widening contradicts FR-121 (FND-001) and needs
  a narrower implementation. `population_domain` changes only the member-type
  lookup in `population_domain_key`, and FR-121 keys a population on a
  `model`/`object_type` node, so the same narrowing covers it.
- FR-116 `replay_frame` accepts process and state-machine targets: this
  conforms to the spec (FR-116 Inputs: "the object type, process or state
  machine that declares the operation"), but it is untested (SR-12365
  FND-005).
