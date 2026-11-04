---
id: SR-2305001
title: "Code review of PR 2305: duration window bounds convert to the selected clock unit (QSL-1648), code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@6ac5a341f; git diff origin/main...6ac5a341f (PR 2305): qsl-semantics/src/check/{check.rs,mod.rs,refusal.rs,temporal_clause.rs,time_units.rs,hyper_clause.rs,lowering/temporal.rs}; qsl-package/src/emit.rs; qsl-replay/src/spine/lifecycle.rs; src/emit_checked_v2.rs; ticket QSL-1648"
review_set: subset
date: '2026-11-04'
---

## Summary

Ticket: QSL-1648. PR: quire-spec-language#2305, head 6ac5a341f. Method: code-review with the rust-review lane. Checked against QSpec FR-370 "Window aggregate atom" (clock unit node `["scalar_type","unit",T]`, both bounds converted exactly, step 5 unit-mismatch for a bound of another unit) and the accepted vector AG-07 / reader vectors AR-24, AR-25.

S3 now receives `ClockUnitSelection`s via `PackageDeclarations::with_clock_units`; the spine (`Resolution`, lifecycle.rs:3440) passes every timestamped-event selection of the unit's source, and the `emit_checked_v2` CLI pre-joins name-keyed selections to the parsed clause span. Every spine S3 entry (check_with_*, check_for_continuation, clause.rs, execute.rs, compile.rs, graph_replay recompile) funnels through that one Resolution site. `model_check/identity.rs`, `item.rs` and `certificate/closure.rs` only re-emit a caller-supplied CheckedPackage; they are not S3 entries.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| clock_units / with_clock_units call | qsl-replay/src/spine/lifecycle.rs:2907-2925, 3440-3443 | examined |
| ClockUnitSelection, DurationWindow, duration_window_model_nodes | qsl-semantics/src/check/temporal_clause.rs:108-120, 315-337, 777-801 | examined |
| clock target selection and window_bound_refusal | qsl-semantics/src/check/temporal_clause.rs:1160-1206, 1397-1430 | examined |
| clock_target, is_clock_unit_node, convert | qsl-semantics/src/check/time_units.rs:366-457 | examined |
| model-owned unit skip in lowering | qsl-semantics/src/check/lowering/temporal.rs:634-655 | examined |
| external set, write_time_nominal, duration model nodes loop | qsl-package/src/emit.rs:5314-5340, 5685-5760, 6150-6270 | examined |
| clause_clock_selections | src/emit_checked_v2.rs:447-506 | examined |
| identity.rs re-emit | qsl-replay/src/model_check/identity.rs:65-120 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The doc comment of `aggregate_duration_bound` still says the bound is converted "into the lower bound's time unit"; it now converts into the selected clock unit when one is given. | qsl-semantics/src/check/temporal_clause.rs:1160-1161 |
| FND-002 | low | `s3.rational_bits` is charged on the written magnitude only; the converted value (magnitude x scale x class size / clock reciprocal) is never checked against the ceiling, although the conversion target is now a third unit (the clock) that neither bound was written in. Growth is bounded by the package scale's bits, so no unbounded allocation, but the ceiling does not bound the emitted literal. Fix: check the converted rational's bits against the ceiling and refuse `LimitExceeded(S3RationalBits)` at the bound. | qsl-semantics/src/check/temporal_clause.rs:1167-1187; qsl-semantics/src/check/time_units.rs:417-438 |
| FND-003 | low | The re-emit limit named in the PR body (model_check identity / item / closure re-emit a caller-supplied CheckedPackage with selections but never re-run S3) has no ticket. Outcome is a spurious `ClauseRootRefusal::Package(InvalidTemporalClockValue)` for a package compiled without clock units whose bound unit differs from the clock, never a wrong verdict; graph_replay recompiles with selections and is unaffected. Fix: file a follow-up ticket (or document that the subject package must be compiled with the same selections). | qsl-replay/src/model_check/identity.rs:96-110; qsl-replay/src/model_check/item.rs:220-226 |

## Verdict

Mergeable; three low findings, no behaviour defect.

- Conversion is exact rational arithmetic (`TimeUnits::convert`, no float); `past[0 s, 300000 ms]` under an ms clock gives `0`/`300000` of the ms node, matching AG-07; `1 ms..1 s` under s gives `1/1000`/`1`.
- Refusal causes: `NotTimeDimension` and `OtherTimeDimension` map to `AggregateWindowKindMismatch` (ill_typed/unit-mismatch at the bound's unit), matching FR-370 step 5 and AR-24/AR-25 in spirit; unknown unit / no Time dimension stay missing_declaration/missing-name; `window_bound_refusal`'s match over `TimeUnitCause` is exhaustive with no wildcard. No enum gained a variant; every `AggregateWindow` match (temporal_clause.rs:783-797, lowering/temporal.rs:632-637) is updated and exhaustive.
- Model-owned unit nodes: lowering skips only a unit that is itself model-owned; the emitter writes the model-origin closure via the shared `write_time_nominal` (empty body, Model owner, generated occurrence at the clause), dedupes against nodes already present, and counts them as external so nothing is dropped as absent. Path match uses the shared `is_clock_unit_node`, same `["scalar_type","unit",T]` path as QSpec.
- Non-duration windows: `Positions` path unchanged; the only shared-code change (`source_artifact(region.source())`) is equal to `graph.source()` for every caller.
- Focused tests: qsl-semantics temporal_clause 17/17; qsl-package emit::tests::t 74/74 (QSPEC_DIR set); qsl-replay spine test 1/1.
