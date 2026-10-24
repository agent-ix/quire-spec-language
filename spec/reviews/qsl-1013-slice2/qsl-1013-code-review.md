---
id: SR-16362
title: "Code review of QSL-1013 slice 2 at 77276554c"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; git diff origin/main...77276554c518595877ff2728e341865a8bfab905: qsl-package/src/emit.rs, qsl-package/src/emit/tests/timed_source.rs, qsl-replay/src/spine.rs, qsl-semantics/src/check/{assemble,check,mod,refusal,time_units}.rs, qsl-semantics/src/check/hybrid/cause.rs, qsl-semantics/src/check/lowering/model.rs, qsl-semantics/tests/it/timed_declarations.rs, qsl-forms/tests/it/time_forms.rs"
review_set: subset
---

## Summary

Ticket: QSL-1013. PR agent-ix/quire-spec-language#1403, reviewed SHA 77276554c518595877ff2728e341865a8bfab905, based on main. Code review with the rust-review lane folded in. Covers the S2 retention of timed forms, the S3 facet gate and time-source check, tick operation resolution, the new TemporalCause variants and their codes, the S4 `TimedModelNotEmitted` guard, and the merge-fix commits (NoTimeDimension struct-variant patterns in hybrid/cause.rs and qsl-replay/src/spine.rs, seven new spine.rs message arms).

Prior review SR-16358 reviewed fba5bbcf1, before the merge of main that brought QSL-1022 (typed timed-model nodes, `emit_timed_models`) and the FR-230 AC-29..37 rewrite. It does not cover this head.

## Verdict

**CHANGES REQUESTED.** The S3 time-source checker is sound for what it covers. Its refusal codes and tags match FR-230 AC-2/3/5/7: duplicate-time-source, missing-name, malformed-declaration, type-mismatch, and declaration-form for the facet gate. The rational-bit ceiling is kept, and resolving to a declaration identity is a real check, not a retained spelling. The merge-fix commits are correct: the `NoTimeDimension { .. }` patterns keep the old code and tag mapping, and the seven new spine.rs arms give specific, accurate messages, not placeholders. The S4 guard fails closed rather than silently dropping a time source, which is better than silent loss. But its stated rationale is false at head, and it is the exact refusal FR-230 now forbids (FND-001). The new ambiguous-operation refusal path is also untested (FND-002).

Commands run, each in its own target dir and one at a time: `cargo test -p qsl-semantics --features test-support --test it timed_declarations:: -- --include-ignored` gave 24 passed, plus the 2 ignored tests failing as described in SR-16363. `cargo test -p qsl-package --lib emit::tests` gave 172 passed. `cargo test -p qsl-forms --test it time_forms::` gave 10 passed. Clippy and the aggregate gate were not run; the code lead owns the gate.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | S4 refuses every checked graph with a time source as `TimedModelNotEmitted`. FR-230 says "S4 SHALL not refuse a successfully checked timed model as `TimedModelNotEmitted`" (AC-32). The guard's doc and message ("has no checked-package/v2 node", "cannot be written to a checked-package/v2 graph yet") are false at head: QSL-1022 (#1412), IR-992 and STD-447 are Done and `emit_timed_models`/`CheckedTimedModel` exist. No follow-up ticket was found that maps `CheckedGraph::time_sources` to timed_model nodes and removes the guard. | qsl-package/src/emit.rs:205-211, qsl-package/src/emit.rs:1438-1442 |
| FND-002 | medium | `TemporalCause::AmbiguousTickOperation` and the override filter of `AdmittedModel::tick_operation_keys` (the `owners.len() != 1` / `reaches_owner` exclusion) have no test. No vector writes an unqualified or owner-qualified operation that two unrelated owners declare. | qsl-semantics/src/check/lowering/model.rs:546-605, qsl-semantics/src/check/mod.rs:1042 |
| FND-003 | low | `units.target` returning `Ok(None)` (a zero-scale time unit, whose reciprocal is undefined) is refused as `TimedNotChecked{TimeSource}` (`unsupported_construct`/`not-yet-implemented`). That misreports a malformed unit as unimplemented. | qsl-semantics/src/check/mod.rs:1001-1008 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | The guard is kept as a tracked fail-closed interim. Its doc, error message and inline comment now say so, and cite FR-230-AC-32, TC-685 step 26 and emit_timed_models. QSL-1032 (Backlog, QSL V1 Architecture Spine) owns mapping CheckedGraph::time_sources into timed_model nodes and removing the guard. |
| FND-002 | fixed | Verified at c643f88da. |
| FND-003 | fixed | Verified at c643f88da. |
